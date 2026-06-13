//! PROBE/COMMIT 协商与视频流启停。

use crate::usb::error::UsbResult;
use crate::usb::host::dwc2;
use crate::usb::setup;

use super::capture::reset_frame_continuity;
use super::consts::*;
use super::parse::UvcStreamSelection;
use super::setup::{uvc_get_cur_vs, uvc_get_max_vs, uvc_set_cur_vs};

/// 根据协商 payload 重选 alt；SG2002 DWC2 仅使用 mult=1（alt=1）。
fn reselect_isoch_alt_for_payload(sel: &mut UvcStreamSelection) {
    if sel.isoch_alts_count == 0 || sel.negotiated_payload_size == 0 {
        return;
    }

    let need = sel.negotiated_payload_size;
    let alts = &sel.isoch_alts[..sel.isoch_alts_count as usize];
    let mut best_fit: Option<(u8, u16, u32)> = None;
    let mut best_max: Option<(u8, u16, u32)> = None;
    for &(alt, mps_raw) in alts {
        let mps = u32::from(mps_raw & 0x7ff);
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

    let (new_alt, new_mps_raw, _) = best_fit
        .or(best_max)
        .unwrap_or((sel.alt_setting, sel.mps_raw, 0));
    sel.alt_setting = new_alt;
    sel.mps_raw = new_mps_raw;
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
    b
}

pub fn uvc_start_video_stream(
    dev: u32,
    ep0_mps: u32,
    sel: &mut UvcStreamSelection,
) -> UsbResult<()> {
    reset_frame_continuity();
    let _ =
        dwc2::ep0_control_write_no_data(dev, setup::set_interface(0, sel.vs_interface), ep0_mps);

    let probe_init = build_probe_commit_payload(sel);
    dwc2::ep0_control_write(
        dev,
        uvc_set_cur_vs(
            sel.vs_interface,
            VS_PROBE_CONTROL,
            UVC_PROBE_COMMIT_LEN as u16,
        ),
        ep0_mps,
        &probe_init,
    )?;

    let mut probe_max = [0u8; UVC_PROBE_COMMIT_LEN];
    let _ = dwc2::ep0_control_read(
        dev,
        uvc_get_max_vs(
            sel.vs_interface,
            VS_PROBE_CONTROL,
            UVC_PROBE_COMMIT_LEN as u16,
        ),
        ep0_mps,
        &mut probe_max,
    );

    let mut probe = [0u8; UVC_PROBE_COMMIT_LEN];
    dwc2::ep0_control_read(
        dev,
        uvc_get_cur_vs(
            sel.vs_interface,
            VS_PROBE_CONTROL,
            UVC_PROBE_COMMIT_LEN as u16,
        ),
        ep0_mps,
        &mut probe,
    )?;

    let negotiated_interval = u32::from_le_bytes([probe[4], probe[5], probe[6], probe[7]]);
    if negotiated_interval != 0 {
        sel.frame_interval = negotiated_interval;
    }
    sel.negotiated_payload_size = u32::from_le_bytes([probe[22], probe[23], probe[24], probe[25]]);
    sel.negotiated_frame_size = u32::from_le_bytes([probe[18], probe[19], probe[20], probe[21]]);

    reselect_isoch_alt_for_payload(sel);

    let alt_mps = u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1);
    if alt_mps > 0 && alt_mps < sel.negotiated_payload_size {
        sel.negotiated_payload_size = alt_mps;
        probe[22..26].copy_from_slice(&alt_mps.to_le_bytes());
    }

    dwc2::ep0_control_write(
        dev,
        uvc_set_cur_vs(
            sel.vs_interface,
            VS_COMMIT_CONTROL,
            UVC_PROBE_COMMIT_LEN as u16,
        ),
        ep0_mps,
        &probe,
    )?;

    dwc2::ep0_control_write_no_data(
        dev,
        setup::set_interface(sel.alt_setting, sel.vs_interface),
        ep0_mps,
    )?;

    Ok(())
}

pub fn uvc_stop_streaming(dev: u32, ep0_mps: u32, vs_if: u8) -> UsbResult<()> {
    reset_frame_continuity();
    dwc2::ep0_control_write_no_data(dev, setup::set_interface(0, vs_if), ep0_mps)
}
