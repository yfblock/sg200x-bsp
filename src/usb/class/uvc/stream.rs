//! PROBE/COMMIT 协商与视频流启停。

use crate::usb::error::UsbResult;
use crate::usb::host::dwc2;
use crate::usb::setup;

use super::capture::reset_frame_continuity;
use super::consts::*;
use super::parse::UvcStreamSelection;
use super::setup::{uvc_get_cur_vs, uvc_get_max_vs, uvc_set_cur_vs};

/// 根据 PROBE/COMMIT 协商出的 `payload_per_uframe`，从所有 Isoch alt 候选中挑出
/// **总带宽 ≥ payload** 且**最小**的那一个；找不到则取带宽最大的。
///
/// **DWC2 兼容性**：SG2002 DWC2 在 mult>1 时性能严重下降，仅考虑 mult=1 候选。
fn reselect_isoch_alt_for_payload(sel: &mut UvcStreamSelection) {
    if sel.isoch_alts_count == 0 {
        return;
    }
    let need = sel.negotiated_payload_size;
    if need == 0 {
        return;
    }
    let alts = &sel.isoch_alts[..sel.isoch_alts_count as usize];
    let mut best_fit: Option<(u8, u16, u32)> = None;
    let mut best_max: Option<(u8, u16, u32)> = None;
    for &(alt, mps_raw) in alts {
        let mps = u32::from(mps_raw & 0x7FF);
        let mult = u32::from((mps_raw >> 11) & 0x3) + 1;
        if mult > 1 {
            continue;
        }
        let total = mps;
        if total >= need {
            let pick = (alt, mps_raw, total);
            best_fit = Some(match best_fit {
                None => pick,
                Some(p) if p.2 > total => pick,
                Some(p) => p,
            });
        }
        let pick = (alt, mps_raw, total);
        best_max = Some(match best_max {
            None => pick,
            Some(p) if p.2 < total => pick,
            Some(p) => p,
        });
    }
    let (new_alt, new_mps_raw, new_total) = best_fit
        .or(best_max)
        .unwrap_or((sel.alt_setting, sel.mps_raw, 0));
    if new_alt != sel.alt_setting || new_mps_raw != sel.mps_raw {
        log::info!("UVC: re-select Isoch alt {} (mps_raw={:#06x}, {} B/uframe) -> alt {} (mps_raw={:#06x}, {} B/uframe) for payload={}",
            sel.alt_setting, sel.mps_raw,
            u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1),
            new_alt, new_mps_raw, new_total, need);
        sel.alt_setting = new_alt;
        sel.mps_raw = new_mps_raw;
    }
}

fn build_probe_commit_payload(sel: &UvcStreamSelection) -> [u8; UVC_PROBE_COMMIT_LEN] {
    let mut b = [0u8; UVC_PROBE_COMMIT_LEN];
    b[0] = 0x01;
    b[1] = 0x00;
    b[2] = sel.format_index;
    b[3] = sel.frame_index;
    b[4..8].copy_from_slice(&sel.frame_interval.to_le_bytes());
    let w = u32::from(sel.frame_w.max(640));
    let h = u32::from(sel.frame_h.max(480));
    let est = if sel.is_mjpeg {
        w.saturating_mul(h)
    } else {
        w.saturating_mul(h).saturating_mul(2)
    };
    b[18..22].copy_from_slice(&est.to_le_bytes());
    let pkt_total = u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1);
    b[22..26].copy_from_slice(&pkt_total.to_le_bytes());
    b
}

fn dump_probe(prefix: &str, p: &[u8]) {
    if p.len() < 26 {
        return;
    }
    let bm_hint = u16::from_le_bytes([p[0], p[1]]);
    let fmt_ix = p[2];
    let frame_ix = p[3];
    let interval = u32::from_le_bytes([p[4], p[5], p[6], p[7]]);
    let key_frm = u16::from_le_bytes([p[8], p[9]]);
    let pframe = u16::from_le_bytes([p[10], p[11]]);
    let comp_q = u16::from_le_bytes([p[12], p[13]]);
    let comp_w = u16::from_le_bytes([p[14], p[15]]);
    let delay = u16::from_le_bytes([p[16], p[17]]);
    let max_video = u32::from_le_bytes([p[18], p[19], p[20], p[21]]);
    let max_pkt = u32::from_le_bytes([p[22], p[23], p[24], p[25]]);
    log::info!("UVC: {prefix} bmHint={bm_hint:#06x} fmt={fmt_ix} frame={frame_ix} iv={interval} keyFrm={key_frm} pFrm={pframe} compQ={comp_q} compW={comp_w} delay={delay} dwMaxVideoFrameSize={max_video} dwMaxPayloadTransferSize={max_pkt}");
}

/// `PROBE` → `GET_CUR` → `COMMIT` → `SET_INTERFACE`。
///
/// 协商后会更新 `sel.negotiated_payload_size` 与 `sel.negotiated_frame_size`，并依据
/// 协商出的 `dwMaxPayloadTransferSize` **重新选择最匹配的 alt setting**（避免 mps 切包错位）。
pub fn uvc_start_video_stream(dev: u32, ep0_mps: u32, sel: &mut UvcStreamSelection) -> UsbResult<()> {
    reset_frame_continuity();
    let _ = dwc2::ep0_control_write_no_data(
        dev,
        setup::set_interface(0, sel.vs_interface),
        ep0_mps,
    );

    let probe_init = build_probe_commit_payload(sel);
    dump_probe("PROBE.SET", &probe_init);

    dwc2::ep0_control_write(
        dev,
        uvc_set_cur_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &probe_init,
    )?;

    let mut probe_max = [0u8; UVC_PROBE_COMMIT_LEN];
    if dwc2::ep0_control_read(
        dev,
        uvc_get_max_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &mut probe_max,
    )
    .is_ok()
    {
        dump_probe("PROBE.MAX", &probe_max);
    }

    let mut probe = [0u8; UVC_PROBE_COMMIT_LEN];
    dwc2::ep0_control_read(
        dev,
        uvc_get_cur_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &mut probe,
    )?;
    dump_probe("PROBE.CUR", &probe);

    sel.negotiated_payload_size = u32::from_le_bytes([probe[22], probe[23], probe[24], probe[25]]);
    sel.negotiated_frame_size = u32::from_le_bytes([probe[18], probe[19], probe[20], probe[21]]);

    reselect_isoch_alt_for_payload(sel);

    let alt_mps = u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1);
    if alt_mps > 0 && alt_mps < sel.negotiated_payload_size {
        log::info!(
            "UVC: clamping COMMIT dwMaxPayloadTransferSize {} -> {} to match alt bandwidth",
            sel.negotiated_payload_size,
            alt_mps
        );
        sel.negotiated_payload_size = alt_mps;
        probe[22..26].copy_from_slice(&alt_mps.to_le_bytes());
    }

    dwc2::ep0_control_write(
        dev,
        uvc_set_cur_vs(sel.vs_interface, VS_COMMIT_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &probe,
    )?;

    dwc2::ep0_control_write_no_data(
        dev,
        setup::set_interface(sel.alt_setting, sel.vs_interface),
        ep0_mps,
    )?;

    log::info!(
        "UVC: streaming armed if={} alt={} negotiated_payload={} frame_size={}",
        sel.vs_interface,
        sel.alt_setting,
        sel.negotiated_payload_size,
        sel.negotiated_frame_size
    );

    Ok(())
}

/// 将 VS 接口切回 `alt=0`，并清空抓帧连续性状态。
pub fn uvc_stop_streaming(dev: u32, ep0_mps: u32, vs_if: u8) -> UsbResult<()> {
    reset_frame_continuity();
    dwc2::ep0_control_write_no_data(dev, setup::set_interface(0, vs_if), ep0_mps)
}
