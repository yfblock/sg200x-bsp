//! USB Video Class（UVC）：配置描述符解析、`PROBE`/`COMMIT`、**Bulk 或 Isoch IN** 抓一帧。
//!
//! 修复要点：视频端点在无数据时大量 **NAK**，须在主机侧重试
//! （见 [`crate::usb::host::dwc2::ep0::bulk_in`] / [`crate::usb::host::dwc2::ep0::isoch_in_uframe`]）。

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::ep0 as dwc2_ep0;
use crate::usb::host::dwc2::ep0::{DMA_OFF_UVC_BULK, UVC_BULK_DMA_CAP};
use crate::usb::log::usb_log_fmt;
use crate::usb::setup;

const VS_PROBE_CONTROL: u8 = 0x01;
const VS_COMMIT_CONTROL: u8 = 0x02;

const USB_DT_CONFIGURATION: u8 = 2;
const USB_DT_INTERFACE: u8 = 4;
const USB_DT_ENDPOINT: u8 = 5;
const CS_INTERFACE: u8 = 0x24;

const VS_FORMAT_MJPEG: u8 = 0x06;
const VS_FRAME_MJPEG: u8 = 0x07;
const VS_FORMAT_UNCOMPRESSED: u8 = 0x04;
const VS_FRAME_UNCOMPRESSED: u8 = 0x05;

const USB_CLASS_VIDEO: u8 = 0x0e;
const USB_SUBCLASS_VIDEO_STREAMING: u8 = 0x02;

const ENDPOINT_ATTR_ISOCH: u8 = 1;
const ENDPOINT_ATTR_BULK: u8 = 2;

const UVC_PROBE_COMMIT_LEN: usize = 34;

// ---------- UVC 类专用 SETUP（VS PROBE / COMMIT 控制） ----------

/// UVC：`SET_CUR`（Video Streaming 接口，`wValue = selector<<8`）。
#[inline]
fn uvc_set_cur_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0x21,
        0x01,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC：`GET_CUR`（VS 探测/提交等）。
#[inline]
fn uvc_get_cur_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0xA1,
        0x81,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC：`GET_MAX`（最大探测结构长度）。
#[inline]
fn uvc_get_max_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0xA1,
        0x83,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

// ---------- 视频流参数 ----------

/// 视频流传输类型（VS 接口上的端点）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvcXferKind {
    Bulk,
    Isoch,
}

/// 解析得到的 VS 流参数（Bulk 优先；否则 Isoch）。
#[derive(Clone, Debug)]
pub struct UvcStreamSelection {
    pub vs_interface: u8,
    pub alt_setting: u8,
    pub ep_num: u8,
    /// `wMaxPacketSize` 原始值（含 HS 带宽倍增位）。
    pub mps_raw: u16,
    pub xfer: UvcXferKind,
    pub format_index: u8,
    pub frame_index: u8,
    pub frame_interval: u32,
    /// 选定格式是否为 MJPEG（用于上层判断输出是否为 JPEG）。
    pub is_mjpeg: bool,
    pub frame_w: u16,
    pub frame_h: u16,
    /// PROBE/COMMIT 协商后设备使用的 `dwMaxPayloadTransferSize`（单微帧字节数）。
    /// 由 [`uvc_start_video_stream`] 在协商后填充，用于 capture 切包。
    pub negotiated_payload_size: u32,
    /// PROBE/COMMIT 协商后设备的 `dwMaxVideoFrameSize`（用于 Bulk 模式判 EOF 与缓冲规划）。
    pub negotiated_frame_size: u32,
}

/// 读取配置描述符（最大 4096 字节，`wTotalLength`）。
pub fn read_configuration_descriptor(dev: u32, ep0_mps: u32, cfg_index: u8) -> UsbResult<[u8; 4096]> {
    let mut hdr = [0u8; 9];
    dwc2_ep0::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(cfg_index, 9),
        ep0_mps,
        &mut hdr,
    )?;
    if hdr[1] != USB_DT_CONFIGURATION {
        return Err(UsbError::Protocol("not a configuration descriptor"));
    }
    let total = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
    if total > 4096 {
        return Err(UsbError::Protocol("configuration descriptor too large (>4096)"));
    }
    let mut buf = [0u8; 4096];
    dwc2_ep0::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(cfg_index, total as u16),
        ep0_mps,
        &mut buf[..total],
    )?;
    Ok(buf)
}

#[inline]
fn max_packet_11(mps_raw: u16) -> u32 {
    u32::from(mps_raw & 0x7FF)
}

/// HS 等时：每微帧总字节数 = `maxPacket * (mult+1)`；FS 通常 `mult=0`。
#[inline]
#[allow(dead_code)]
fn isoch_total_bytes_per_uframe(mps_raw: u16) -> usize {
    let mb = (mps_raw & 0x7FF) as usize;
    let mult = ((mps_raw >> 11) & 3) as usize + 1;
    mb.saturating_mul(mult)
}

/// 解析 VS 接口：优先选择 **MJPEG 格式**；若无 MJPEG 则使用未压缩 (YUY2/NV12) 格式。
/// 端点优先 **Bulk IN**；若无则 **Isoch IN**（取带宽最高的 alt）。
///
/// 同时把所有 VS 候选打到串口，便于诊断。
pub fn parse_uvc_video_stream(cfg: &[u8], cfg_total: usize) -> UsbResult<UvcStreamSelection> {
    let len = cfg_total.min(cfg.len());
    if len < 12 {
        return Err(UsbError::Protocol("cfg too short"));
    }

    let mut i = usize::from(cfg[0]);
    if i >= len {
        return Err(UsbError::Protocol("bad cfg bLength"));
    }

    let mut cur_ifc_class = 0u8;
    let mut cur_ifc_sub = 0u8;
    let mut cur_ifc_num = 0u8;
    let mut cur_alt = 0u8;
    let mut last_fmt_subtype = 0u8;
    let mut last_fmt_ix = 0u8;

    let mut best_bulk: Option<(u8, u8, u16, u8)> = None;
    let mut best_isoch: Option<(u8, u8, u16, u8)> = None;

    let mut mjpeg_pick: Option<(u8, u8, u16, u16, u32)> = None;
    let mut uncomp_pick: Option<(u8, u8, u16, u16, u32)> = None;
    let mut cur_fmt_ix_for_frame = 0u8;
    let mut cur_fmt_subtype_for_frame = 0u8;

    while i + 2 <= len {
        let bl = cfg[i] as usize;
        if bl < 2 || i + bl > len {
            break;
        }
        let ty = cfg[i + 1];

        if ty == USB_DT_INTERFACE && bl >= 9 {
            cur_ifc_num = cfg[i + 2];
            cur_alt = cfg[i + 3];
            cur_ifc_class = cfg[i + 5];
            cur_ifc_sub = cfg[i + 6];
        } else if ty == CS_INTERFACE
            && cur_ifc_class == USB_CLASS_VIDEO
            && cur_ifc_sub == USB_SUBCLASS_VIDEO_STREAMING
        {
            let st = cfg.get(i + 2).copied().unwrap_or(0);
            if (st == VS_FORMAT_MJPEG || st == VS_FORMAT_UNCOMPRESSED) && bl >= 4 {
                last_fmt_subtype = st;
                last_fmt_ix = cfg[i + 3];
                cur_fmt_subtype_for_frame = st;
                cur_fmt_ix_for_frame = cfg[i + 3];
                usb_log_fmt(format_args!(
                    "UVC: VS-fmt if={cur_ifc_num} alt={cur_alt} ix={} subtype={:#04x} ({})",
                    last_fmt_ix, st,
                    if st == VS_FORMAT_MJPEG { "MJPEG" } else { "Uncompressed" }
                ));
            }
            if (st == VS_FRAME_MJPEG || st == VS_FRAME_UNCOMPRESSED) && bl >= 26 {
                let frame_ix = cfg[i + 3];
                let w = u16::from_le_bytes([cfg[i + 5], cfg[i + 6]]);
                let h = u16::from_le_bytes([cfg[i + 7], cfg[i + 8]]);
                let dflt_ival = u32::from_le_bytes([cfg[i + 21], cfg[i + 22], cfg[i + 23], cfg[i + 24]]);
                let ival_type = cfg[i + 25];
                let mut min_ival = dflt_ival;
                if ival_type == 0 && bl >= 38 {
                    let dw_min = u32::from_le_bytes([cfg[i + 26], cfg[i + 27], cfg[i + 28], cfg[i + 29]]);
                    if dw_min > 0 { min_ival = dw_min; }
                } else if ival_type > 0 {
                    let n = ival_type as usize;
                    let mut p = i + 26;
                    for _ in 0..n {
                        if p + 4 > i + bl as usize { break; }
                        let v = u32::from_le_bytes([cfg[p], cfg[p + 1], cfg[p + 2], cfg[p + 3]]);
                        if v > 0 && v < min_ival { min_ival = v; }
                        p += 4;
                    }
                }
                let fps_x100 = if dflt_ival > 0 { 1_000_000_00u32 / dflt_ival.max(1) } else { 0 };
                let fps_min_x100 = if min_ival > 0 { 1_000_000_00u32 / min_ival.max(1) } else { 0 };
                usb_log_fmt(format_args!(
                    "UVC: VS-frame fmt_ix={cur_fmt_ix_for_frame} frame_ix={frame_ix} {}x{} iv_dflt={dflt_ival} ({}.{:02} fps) iv_min={min_ival} ({}.{:02} fps) ival_type={ival_type}",
                    w, h,
                    fps_x100 / 100, fps_x100 % 100,
                    fps_min_x100 / 100, fps_min_x100 % 100
                ));
                let dflt_ival = if min_ival > 0 { min_ival } else { dflt_ival };
                let pick = (cur_fmt_ix_for_frame, frame_ix, w, h, dflt_ival);
                let is_mjpeg = cur_fmt_subtype_for_frame == VS_FORMAT_MJPEG
                    || st == VS_FRAME_MJPEG;
                fn rank((_, _, pw, ph, _): (u8, u8, u16, u16, u32)) -> i32 {
                    let w = pw as i32;
                    let h = ph as i32;
                    if w == 1280 && h == 720 { return 1_000_000; }
                    if w == 640 && h == 480 { return 900_000; }
                    if w == 800 && h == 600 { return 800_000; }
                    if w == 1024 && h == 768 { return 750_000; }
                    if w == 320 && h == 240 { return 700_000; }
                    let area = w * h;
                    if area <= 1280 * 720 { 600_000 - (1280 * 720 - area) } else { 100_000 - (area - 1280 * 720) }
                }
                if is_mjpeg {
                    let pick_better = match mjpeg_pick {
                        None => true,
                        Some(prev) => rank(pick) > rank(prev),
                    };
                    if pick_better { mjpeg_pick = Some(pick); }
                } else {
                    let pick_better = match uncomp_pick {
                        None => true,
                        Some(prev) => rank(pick) > rank(prev),
                    };
                    if pick_better { uncomp_pick = Some(pick); }
                }
                let _ = last_fmt_subtype;
                let _ = last_fmt_ix;
            }
        } else if ty == USB_DT_ENDPOINT
            && cur_ifc_class == USB_CLASS_VIDEO
            && cur_ifc_sub == USB_SUBCLASS_VIDEO_STREAMING
        {
            let ep_addr = cfg[i + 2];
            let attr = cfg[i + 3];
            let mps_raw = u16::from_le_bytes([cfg[i + 4], cfg[i + 5]]);
            let mps = mps_raw & 0x7FF;
            let mult = ((mps_raw >> 11) & 0x3) + 1;
            let xfer = attr & 0x03;
            if (ep_addr & 0x80) == 0 {
                i += bl;
                continue;
            }
            let ep_num = ep_addr & 0x0F;
            let total = u32::from(mps) * u32::from(mult);
            usb_log_fmt(format_args!(
                "UVC: VS-cand if={cur_ifc_num} alt={cur_alt} ep={ep_num} kind={} mps={mps} mult={mult} total={total}/uframe mps_raw={mps_raw:#06x}",
                if xfer == ENDPOINT_ATTR_BULK { "Bulk" } else if xfer == ENDPOINT_ATTR_ISOCH { "Isoch" } else { "Other" }
            ));
            if xfer == ENDPOINT_ATTR_BULK {
                let tak = (cur_alt, ep_num, mps_raw, cur_ifc_num);
                best_bulk = Some(match best_bulk {
                    None => tak,
                    Some(b) => if mps > (b.2 & 0x7FF) { tak } else { b },
                });
            } else if xfer == ENDPOINT_ATTR_ISOCH {
                let tak = (cur_alt, ep_num, mps_raw, cur_ifc_num);
                let new_mult = mult;
                best_isoch = Some(match best_isoch {
                    None => tak,
                    Some(b) => {
                        let old_mps = b.2 & 0x7FF;
                        let old_mult = ((b.2 >> 11) & 0x3) + 1;
                        let new_score = if new_mult == 1 {
                            10_000_000u32 + u32::from(mps)
                        } else {
                            u32::from(mps) * u32::from(new_mult)
                        };
                        let old_score = if old_mult == 1 {
                            10_000_000u32 + u32::from(old_mps)
                        } else {
                            u32::from(old_mps) * u32::from(old_mult)
                        };
                        if new_score > old_score { tak } else { b }
                    }
                });
            }
        }

        i += bl;
    }

    let (alt, epn, mps_raw, vs_if, kind) = if let Some((a, e, m, v)) = best_bulk {
        (a, e, m, v, UvcXferKind::Bulk)
    } else if let Some((a, e, m, v)) = best_isoch {
        (a, e, m, v, UvcXferKind::Isoch)
    } else {
        return Err(UsbError::NotImplemented);
    };

    let (fmt_ix, frame_ix, frame_w, frame_h, interval, is_mjpeg) = match mjpeg_pick {
        Some((fi, frix, w, h, iv)) => (fi, frix, w, h, iv, true),
        None => match uncomp_pick {
            Some((fi, frix, w, h, iv)) => (fi, frix, w, h, iv, false),
            None => (1, 1, 0, 0, 333_333, false),
        },
    };

    usb_log_fmt(format_args!(
        "UVC: SEL if={vs_if} alt={alt} ep={epn} {:?} mps_raw={:#06x} fmt_ix={fmt_ix} frame_ix={frame_ix} {}x{} iv={interval} mjpeg={is_mjpeg}",
        kind, mps_raw, frame_w, frame_h
    ));

    Ok(UvcStreamSelection {
        vs_interface: vs_if,
        alt_setting: alt,
        ep_num: epn,
        mps_raw,
        xfer: kind,
        format_index: fmt_ix,
        frame_index: frame_ix,
        frame_interval: interval,
        is_mjpeg,
        frame_w,
        frame_h,
        negotiated_payload_size: 0,
        negotiated_frame_size: 0,
    })
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
    let est = if sel.is_mjpeg { w.saturating_mul(h) } else { w.saturating_mul(h).saturating_mul(2) };
    b[18..22].copy_from_slice(&est.to_le_bytes());
    let pkt_total = u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1);
    b[22..26].copy_from_slice(&pkt_total.to_le_bytes());
    b
}

fn dump_probe(prefix: &str, p: &[u8]) {
    if p.len() < 26 { return; }
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
    usb_log_fmt(format_args!(
        "UVC: {prefix} bmHint={bm_hint:#06x} fmt={fmt_ix} frame={frame_ix} iv={interval} keyFrm={key_frm} pFrm={pframe} compQ={comp_q} compW={comp_w} delay={delay} dwMaxVideoFrameSize={max_video} dwMaxPayloadTransferSize={max_pkt}"
    ));
}

/// `PROBE` → `GET_CUR` → `COMMIT` → `SET_INTERFACE`。
///
/// 协商后会更新 `sel.negotiated_payload_size` 与 `sel.negotiated_frame_size`，并依据
/// 协商出的 `dwMaxPayloadTransferSize` **重新选择最匹配的 alt setting**（避免 mps 切包错位）。
pub fn uvc_start_video_stream(dev: u32, ep0_mps: u32, sel: &mut UvcStreamSelection) -> UsbResult<()> {
    reset_frame_continuity();
    let _ = dwc2_ep0::ep0_control_write_no_data(
        dev,
        setup::set_interface(0, sel.vs_interface),
        ep0_mps,
    );

    let probe_init = build_probe_commit_payload(sel);
    dump_probe("PROBE.SET", &probe_init);

    dwc2_ep0::ep0_control_write(
        dev,
        uvc_set_cur_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &probe_init,
    )?;

    let mut probe_max = [0u8; UVC_PROBE_COMMIT_LEN];
    if dwc2_ep0::ep0_control_read(
        dev,
        uvc_get_max_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &mut probe_max,
    ).is_ok() {
        dump_probe("PROBE.MAX", &probe_max);
    }

    let mut probe = [0u8; UVC_PROBE_COMMIT_LEN];
    dwc2_ep0::ep0_control_read(
        dev,
        uvc_get_cur_vs(sel.vs_interface, VS_PROBE_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &mut probe,
    )?;
    dump_probe("PROBE.CUR", &probe);

    sel.negotiated_payload_size = u32::from_le_bytes([probe[22], probe[23], probe[24], probe[25]]);
    sel.negotiated_frame_size = u32::from_le_bytes([probe[18], probe[19], probe[20], probe[21]]);

    dwc2_ep0::ep0_control_write(
        dev,
        uvc_set_cur_vs(sel.vs_interface, VS_COMMIT_CONTROL, UVC_PROBE_COMMIT_LEN as u16),
        ep0_mps,
        &probe,
    )?;

    dwc2_ep0::ep0_control_write_no_data(
        dev,
        setup::set_interface(sel.alt_setting, sel.vs_interface),
        ep0_mps,
    )?;

    usb_log_fmt(format_args!(
        "UVC: streaming armed if={} alt={} negotiated_payload={} frame_size={}",
        sel.vs_interface, sel.alt_setting, sel.negotiated_payload_size, sel.negotiated_frame_size
    ));

    Ok(())
}

pub fn uvc_stop_streaming(dev: u32, ep0_mps: u32, vs_if: u8) -> UsbResult<()> {
    reset_frame_continuity();
    dwc2_ep0::ep0_control_write_no_data(dev, setup::set_interface(0, vs_if), ep0_mps)
}

#[inline]
fn uvc_payload_header_len(data: &[u8]) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let hlen = data[0] as usize;
    if hlen == 0 || hlen > data.len() {
        return None;
    }
    Some(hlen)
}

pub const UVC_WORK_AREA_BYTES: usize = 65536;
pub const UVC_ASSEMBLED_JPEG_DMA_OFF: usize = DMA_OFF_UVC_BULK + UVC_WORK_AREA_BYTES;

fn parse_uvc_packet(pkt: &[u8]) -> (bool, usize, Option<u8>, u8) {
    if pkt.len() < 2 {
        return (false, 0, None, 0);
    }
    let hlen = pkt[0] as usize;
    if hlen < 2 || hlen > pkt.len() {
        return (false, 0, None, 0);
    }
    let info = pkt[1];
    let cur_fid = info & 0x01;
    let payload_len = pkt.len() - hlen;
    let eof = (info & 0x02) != 0;
    (eof, payload_len, Some(cur_fid), info)
}

enum FrameState {
    /// 等待首次 FID 翻转（丢弃当前不完整帧的尾巴）。
    WaitFirstSwitch { last_fid: Option<u8> },
    /// 已锁定 frame_fid，开始累积；遇 EOF 或 fid 翻转都视为帧结束。
    Capturing { frame_fid: u8, saw_data: bool },
}

/// 跨 capture 持久化的「上次 EOF 帧的 FID」。
/// 0xFF = 还没抓过；其它值 = 0/1。后续 capture 直接以 `WaitFirstSwitch { last_fid: Some(..) }`
/// 开始，免去等到下一次完整翻转的 ~半~一个帧周期。
pub static LAST_EOF_FID: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0xFF);

/// 重置跨 capture 的连续抓帧状态（停止 streaming/重新协商时调用）。
#[inline]
pub fn reset_frame_continuity() {
    LAST_EOF_FID.store(0xFF, core::sync::atomic::Ordering::Relaxed);
}

fn process_packet(
    pkt: &[u8],
    state: &mut FrameState,
    jpeg_len: &mut usize,
    jpeg_cap: usize,
    debug_remaining: &mut u32,
) -> UsbResult<bool> {
    let (eof, payload_len, fid_opt, info) = parse_uvc_packet(pkt);
    if *debug_remaining > 0 {
        usb_log_fmt(format_args!(
            "UVC-pkt uf={} len={} hlen={} info={:#04x} fid={:?} eof={} payload={}",
            dwc2_ep0::current_uframe(),
            pkt.len(), if pkt.is_empty() { 0 } else { pkt[0] as usize },
            info, fid_opt, eof, payload_len
        ));
        *debug_remaining -= 1;
    }
    let Some(cur_fid) = fid_opt else {
        return Ok(false);
    };

    match state {
        FrameState::WaitFirstSwitch { last_fid } => {
            match *last_fid {
                None => *last_fid = Some(cur_fid),
                Some(prev) if prev != cur_fid => {
                    *state = FrameState::Capturing { frame_fid: cur_fid, saw_data: false };
                    return process_packet_capturing(pkt, payload_len, eof, cur_fid, info, state, jpeg_len, jpeg_cap);
                }
                _ => {}
            }
            Ok(false)
        }
        FrameState::Capturing { .. } => process_packet_capturing(pkt, payload_len, eof, cur_fid, info, state, jpeg_len, jpeg_cap),
    }
}

fn process_packet_capturing(
    pkt: &[u8],
    payload_len: usize,
    eof: bool,
    cur_fid: u8,
    info: u8,
    state: &mut FrameState,
    jpeg_len: &mut usize,
    jpeg_cap: usize,
) -> UsbResult<bool> {
    let FrameState::Capturing { frame_fid, saw_data } = state else {
        return Ok(false);
    };
    if cur_fid != *frame_fid {
        if FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            usb_log_fmt(format_args!(
                "UVC-trace FID-flip uf={} frame_fid={} new_fid={} saw_data={} jpeg_len={}",
                dwc2_ep0::current_uframe(), *frame_fid, cur_fid, *saw_data, *jpeg_len
            ));
        }
        return Ok(*saw_data);
    }
    if payload_len > 0 {
        let hlen = pkt[0] as usize;
        let payload = &pkt[hlen..];
        if jpeg_len.checked_add(payload.len()).unwrap_or(usize::MAX) > jpeg_cap {
            return Err(UsbError::Hardware("video assemble overflow"));
        }
        dwc2_ep0::dma_write_at(UVC_ASSEMBLED_JPEG_DMA_OFF + *jpeg_len, payload)?;
        *jpeg_len += payload.len();
        *saw_data = true;
    }
    let frame_done = eof && *saw_data;
    if eof && FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
        usb_log_fmt(format_args!(
            "UVC-trace EOF uf={} fid={} info={:#04x} payload={} saw_data={} jpeg_len={}",
            dwc2_ep0::current_uframe(), cur_fid, info, payload_len, *saw_data, *jpeg_len
        ));
    }
    Ok(frame_done)
}

/// 全局开关：打开后 [`process_packet_capturing`] 会对每次 EOF/FID-flip trace 出 microframe。
pub static FRAME_DEBUG: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// 抓一帧（视频负载组装至 [`UVC_ASSEMBLED_JPEG_DMA_OFF`]）。
///
/// **关键**：等时模式下 `mult=1` 时，每次 `isoch_in_uframe` 返回的整个数据（最多 mps 字节）就是
/// **一个完整的 USB 包 = 一个 UVC 数据包**（带 12 字节头），**不可再切分**。
pub fn uvc_capture_one_frame(dev: u32, ep0_mps: u32, sel: &UvcStreamSelection) -> UsbResult<usize> {
    let _ = ep0_mps;
    let _ = uvc_payload_header_len; // 仅在调试时使用，避免未引用警告
    let ep = u32::from(sel.ep_num);
    let maxp = max_packet_11(sel.mps_raw).max(1);
    let mps_low = maxp as usize;
    let mult = (((sel.mps_raw >> 11) & 0x3) as u32 + 1).clamp(1, 3) as usize;
    let jpeg_cap = UVC_BULK_DMA_CAP.saturating_sub(UVC_WORK_AREA_BYTES);
    let mut jpeg_len = 0usize;
    let mut transfers = 0u32;
    let mut data_transfers = 0u32;
    let work_off = DMA_OFF_UVC_BULK;
    let prev_eof_fid = LAST_EOF_FID.load(core::sync::atomic::Ordering::Relaxed);
    let mut state = FrameState::WaitFirstSwitch {
        last_fid: if prev_eof_fid <= 1 { Some(prev_eof_fid) } else { None },
    };
    let mut debug_remaining: u32 = 0;
    let uf_start = dwc2_ep0::current_uframe();
    let mut uf_first_switch: u32 = uf_start;
    let mut uframes_at_switch: u32 = 0;

    match sel.xfer {
        UvcXferKind::Bulk => {
            let chunk = (mps_low.saturating_mul(16)).min(UVC_WORK_AREA_BYTES);
            if chunk == 0 {
                return Err(UsbError::Protocol("bad chunk"));
            }
            let mut pid = dwc2_ep0::PID_DATA0;
            loop {
                let actual = dwc2_ep0::bulk_in(dev, ep, maxp, pid, chunk, work_off)?;
                if actual == 0 {
                    return Err(UsbError::Protocol("bulk IN 0 bytes"));
                }
                transfers = transfers.wrapping_add(1);
                pid = if pid == dwc2_ep0::PID_DATA0 {
                    dwc2_ep0::PID_DATA1
                } else {
                    dwc2_ep0::PID_DATA0
                };
                let slice = dwc2_ep0::dma_rx_slice(work_off, actual).ok_or(UsbError::Hardware("dma view"))?;
                let eof = process_packet(slice, &mut state, &mut jpeg_len, jpeg_cap, &mut debug_remaining)?;
                if eof {
                    usb_log_fmt(format_args!("UVC: frame {} bytes ({} bulk transfers)", jpeg_len, transfers));
                    return Ok(jpeg_len);
                }
                if transfers > 60_000 {
                    return Err(UsbError::Timeout);
                }
            }
        }
        UvcXferKind::Isoch => {
            const MAX_UFRAMES: u32 = 80_000;
            for _ in 0..MAX_UFRAMES {
                transfers = transfers.wrapping_add(1);
                let actual = dwc2_ep0::isoch_in_uframe(dev, ep, sel.mps_raw, work_off)?;
                if actual == 0 {
                    continue;
                }
                data_transfers = data_transfers.wrapping_add(1);
                let slice = dwc2_ep0::dma_rx_slice(work_off, actual).ok_or(UsbError::Hardware("dma view"))?;

                let was_waiting = matches!(state, FrameState::WaitFirstSwitch { .. });
                let eof = if mult == 1 {
                    process_packet(slice, &mut state, &mut jpeg_len, jpeg_cap, &mut debug_remaining)?
                } else {
                    let mut hit_eof = false;
                    let mut off = 0usize;
                    while off < slice.len() {
                        let end = if slice.len() - off >= mps_low { off + mps_low } else { slice.len() };
                        let pkt = &slice[off..end];
                        off = end;
                        if process_packet(pkt, &mut state, &mut jpeg_len, jpeg_cap, &mut debug_remaining)? {
                            hit_eof = true;
                            break;
                        }
                    }
                    hit_eof
                };
                if was_waiting && matches!(state, FrameState::Capturing { .. }) {
                    uf_first_switch = dwc2_ep0::current_uframe();
                    uframes_at_switch = transfers;
                }
                if eof {
                    if let FrameState::Capturing { frame_fid, .. } = state {
                        LAST_EOF_FID.store(frame_fid, core::sync::atomic::Ordering::Relaxed);
                    }
                    let uf_end = dwc2_ep0::current_uframe();
                    let dwait = uf_first_switch.wrapping_sub(uf_start) & 0xffff;
                    let dcap = uf_end.wrapping_sub(uf_first_switch) & 0xffff;
                    usb_log_fmt(format_args!(
                        "UVC: frame {} bytes ({} loops, {} data; HFNUM dwait={} uf ({}.{} ms / {} loops), dcap={} uf ({}.{} ms / {} loops), mult={})",
                        jpeg_len, transfers, data_transfers,
                        dwait, dwait / 8, (dwait % 8) * 125 / 10, uframes_at_switch,
                        dcap, dcap / 8, (dcap % 8) * 125 / 10, transfers - uframes_at_switch,
                        mult
                    ));
                    return Ok(jpeg_len);
                }
            }
            usb_log_fmt(format_args!(
                "UVC: capture timeout after {} uframes ({} data; {} bytes assembled, mult={})",
                transfers, data_transfers, jpeg_len, mult
            ));
            Err(UsbError::Timeout)
        }
    }
}
