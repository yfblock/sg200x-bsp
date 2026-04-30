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
const USB_SUBCLASS_VIDEO_CONTROL: u8 = 0x01;

// VideoControl class-specific interface descriptor subtypes
const VC_HEADER: u8 = 0x01;
const VC_INPUT_TERMINAL: u8 = 0x02;
const VC_PROCESSING_UNIT: u8 = 0x05;

/// `wTerminalType = 0x0201` 表示 ITT_CAMERA（CameraTerminal）。
const ITT_CAMERA: u16 = 0x0201;

// ProcessingUnit selectors (wValue MSB)
#[allow(dead_code)] const PU_BACKLIGHT_COMPENSATION: u8 = 0x01;
#[allow(dead_code)] const PU_BRIGHTNESS_CONTROL: u8 = 0x02;
#[allow(dead_code)] const PU_CONTRAST_CONTROL: u8 = 0x03;
#[allow(dead_code)] const PU_GAIN_CONTROL: u8 = 0x04;
#[allow(dead_code)] const PU_HUE_CONTROL: u8 = 0x06;
#[allow(dead_code)] const PU_SATURATION_CONTROL: u8 = 0x07;
#[allow(dead_code)] const PU_SHARPNESS_CONTROL: u8 = 0x08;
const PU_WHITE_BALANCE_TEMPERATURE_CONTROL: u8 = 0x0A;
const PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL: u8 = 0x0B;
const PU_HUE_AUTO_CONTROL: u8 = 0x10;
const PU_POWER_LINE_FREQUENCY_CONTROL: u8 = 0x05;

// CameraTerminal selectors
const CT_AE_MODE_CONTROL: u8 = 0x02;
const CT_AE_PRIORITY_CONTROL: u8 = 0x03;
#[allow(dead_code)] const CT_EXPOSURE_TIME_ABSOLUTE_CONTROL: u8 = 0x04;
const CT_FOCUS_AUTO_CONTROL: u8 = 0x08;

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

/// UVC 类控制 `SET_CUR`（**VideoControl 接口**，`wIndex = (entity_id<<8) | interface`）。
#[inline]
fn uvc_set_cur_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0x21,
        0x01,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC 类控制 `GET_CUR`（VideoControl 接口）。
#[inline]
fn uvc_get_cur_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0xA1,
        0x81,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC 类控制 `GET_DEF`（VideoControl 接口）：读取摄像头出厂默认值。
#[inline]
fn uvc_get_def_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0xA1,
        0x87,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
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
    /// 同一个 ep_num 下的所有 Isoch alt 候选 `(alt, mps_raw)`，按 mps*total 升序。
    /// PROBE 协商后用 [`reselect_isoch_alt_for_payload`] 回选最匹配的 alt，避免出现
    /// "alt=1 但 negotiated_payload=3060" 这种带宽不够的 mismatch。
    pub isoch_alts_count: u8,
    pub isoch_alts: [(u8, u16); 8],
}

/// 通过 EP0 读取完整配置描述符（按首 9 字节里的 `wTotalLength`，最大 4096）。
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
    // 同一 ep_num 的所有 Isoch alt 候选，用于 PROBE 协商后回选合适带宽。
    let mut isoch_alts: [(u8, u16); 8] = [(0u8, 0u16); 8];
    let mut isoch_alts_count: usize = 0;

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
                    let area = w * h;
                    let pref_max =
                        PREFERRED_MAX_PIXELS.load(core::sync::atomic::Ordering::Relaxed) as i32;
                    if pref_max > 0 {
                        // 设了上限：area <= pref_max 时越接近越好；超过则按超出量倒扣分。
                        return if area <= pref_max {
                            area.saturating_add(1_000_000)
                        } else {
                            (-(area - pref_max)).saturating_sub(1_000)
                        };
                    }
                    if w == 1280 && h == 720 { return 1_000_000; }
                    if w == 640 && h == 480 { return 900_000; }
                    if w == 800 && h == 600 { return 800_000; }
                    if w == 1024 && h == 768 { return 750_000; }
                    if w == 320 && h == 240 { return 700_000; }
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
                // 沿用旧逻辑给 best_isoch 一个"初始猜测"（mult=1 优先），但真正的 alt
                // 由 PROBE 之后 [`reselect_isoch_alt_for_payload`] 重新选定。
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
                if isoch_alts_count < isoch_alts.len() {
                    isoch_alts[isoch_alts_count] = (cur_alt, mps_raw);
                    isoch_alts_count += 1;
                }
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
        isoch_alts_count: isoch_alts_count as u8,
        isoch_alts,
    })
}

/// 根据 PROBE/COMMIT 协商出的 `payload_per_uframe`，从所有 Isoch alt 候选中挑出
/// **总带宽 ≥ payload** 且**最小**的那一个；找不到则取带宽最大的。
///
/// 找到后更新 `sel.alt_setting` 和 `sel.mps_raw`。
///
/// **DWC2 兼容性**：SG2002 等低端 DWC2 不可靠支持 HS 高带宽 Isoch（mult > 1），
/// 传输能完成但数据内容错误。因此只考虑 mult=1 的候选；若设备协商的 payload
/// 超过 mult=1 最大带宽，仍选最大 mult=1 alt——摄像头会自适应降低每微帧吞吐，
/// 帧传输耗时更长但数据正确。
fn reselect_isoch_alt_for_payload(sel: &mut UvcStreamSelection) {
    if sel.xfer != UvcXferKind::Isoch || sel.isoch_alts_count == 0 {
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
        usb_log_fmt(format_args!(
            "UVC: re-select Isoch alt {} (mps_raw={:#06x}, {} B/uframe) -> alt {} (mps_raw={:#06x}, {} B/uframe) for payload={} (mult>1 skipped)",
            sel.alt_setting, sel.mps_raw,
            u32::from(sel.mps_raw & 0x7FF) * (u32::from((sel.mps_raw >> 11) & 0x3) + 1),
            new_alt, new_mps_raw, new_total, need
        ));
        sel.alt_setting = new_alt;
        sel.mps_raw = new_mps_raw;
    }
}

/// `parse_uvc_control_entities` 的输出：UVC VideoControl 接口及其下的实体 ID/支持位。
#[derive(Clone, Debug, Default)]
pub struct UvcControlEntities {
    pub vc_interface: u8,
    /// CameraTerminal（输入终端，wTerminalType=0x0201）的 entity ID。
    pub camera_terminal_id: Option<u8>,
    /// CameraTerminal `bmControls` 位掩码（最多 24 位，UVC 1.5）。
    pub ct_controls: u32,
    /// ProcessingUnit 的 entity ID。
    pub processing_unit_id: Option<u8>,
    /// ProcessingUnit `bmControls` 位掩码（最多 24 位）。
    pub pu_controls: u32,
}

/// 解析配置描述符，找出 VideoControl 接口下的 CameraTerminal/ProcessingUnit 实体 ID
/// 与各自的 `bmControls`，用于后续 SET_CUR 控制（自动白平衡 / 自动曝光等）。
pub fn parse_uvc_control_entities(cfg: &[u8], cfg_total: usize) -> Option<UvcControlEntities> {
    let len = cfg_total.min(cfg.len());
    if len < 12 {
        return None;
    }
    let mut i = usize::from(cfg[0]);
    if i >= len {
        return None;
    }

    let mut cur_ifc_class = 0u8;
    let mut cur_ifc_sub = 0u8;
    let mut cur_ifc_num = 0u8;
    let mut out = UvcControlEntities::default();
    let mut found_vc = false;

    while i + 2 <= len {
        let bl = cfg[i] as usize;
        if bl < 2 || i + bl > len {
            break;
        }
        let ty = cfg[i + 1];

        if ty == USB_DT_INTERFACE && bl >= 9 {
            cur_ifc_num = cfg[i + 2];
            cur_ifc_class = cfg[i + 5];
            cur_ifc_sub = cfg[i + 6];
            if cur_ifc_class == USB_CLASS_VIDEO && cur_ifc_sub == USB_SUBCLASS_VIDEO_CONTROL {
                out.vc_interface = cur_ifc_num;
                found_vc = true;
            }
        } else if ty == CS_INTERFACE
            && cur_ifc_class == USB_CLASS_VIDEO
            && cur_ifc_sub == USB_SUBCLASS_VIDEO_CONTROL
            && bl >= 3
        {
            let st = cfg[i + 2];
            match st {
                VC_HEADER => {}
                VC_INPUT_TERMINAL => {
                    // bLength=15+x，bUnitID@3, wTerminalType@4..6, bAssocTerm@6,
                    // 后续 wObjectiveFocalLengthMin/Max + wOcularFocalLength + bControlSize@14, bmControls@15..
                    if bl >= 15 {
                        let id = cfg[i + 3];
                        let tt = u16::from_le_bytes([cfg[i + 4], cfg[i + 5]]);
                        if tt == ITT_CAMERA {
                            out.camera_terminal_id = Some(id);
                            let csize = cfg[i + 14] as usize;
                            let cmax = csize.min(bl.saturating_sub(15)).min(4);
                            let mut bm = 0u32;
                            for k in 0..cmax {
                                bm |= u32::from(cfg[i + 15 + k]) << (8 * k);
                            }
                            out.ct_controls = bm;
                        }
                    }
                }
                VC_PROCESSING_UNIT => {
                    // bLength=10+n，bUnitID@3, bSourceID@4, wMaxMultiplier@5..7, bControlSize@7, bmControls@8..
                    if bl >= 9 {
                        let id = cfg[i + 3];
                        let csize = cfg[i + 7] as usize;
                        let cmax = csize.min(bl.saturating_sub(8)).min(4);
                        let mut bm = 0u32;
                        for k in 0..cmax {
                            bm |= u32::from(cfg[i + 8 + k]) << (8 * k);
                        }
                        out.processing_unit_id = Some(id);
                        out.pu_controls = bm;
                    }
                }
                _ => {}
            }
        }

        i += bl;
    }

    if found_vc { Some(out) } else { None }
}

/// 仅在控制传输出现 STALL 时返回 false，其它错误则当成"不支持"忽略。
fn try_set_cur_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
    value: u8,
) -> bool {
    let setup = uvc_set_cur_vc(vc_if, entity, selector, 1);
    let buf = [value];
    dwc2_ep0::ep0_control_write(dev, setup, ep0_mps, &buf).is_ok()
}

#[allow(dead_code)]
fn try_get_cur_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u8> {
    let setup = uvc_get_cur_vc(vc_if, entity, selector, 1);
    let mut buf = [0u8; 1];
    if dwc2_ep0::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(buf[0])
    } else {
        None
    }
}

#[allow(dead_code)]
fn try_get_cur_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u16> {
    let setup = uvc_get_cur_vc(vc_if, entity, selector, 2);
    let mut buf = [0u8; 2];
    if dwc2_ep0::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(u16::from_le_bytes(buf))
    } else {
        None
    }
}

fn try_get_def_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u8> {
    let setup = uvc_get_def_vc(vc_if, entity, selector, 1);
    let mut buf = [0u8; 1];
    if dwc2_ep0::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(buf[0])
    } else {
        None
    }
}

fn try_get_def_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u16> {
    let setup = uvc_get_def_vc(vc_if, entity, selector, 2);
    let mut buf = [0u8; 2];
    if dwc2_ep0::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(u16::from_le_bytes(buf))
    } else {
        None
    }
}

fn try_set_cur_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
    value: u16,
) -> bool {
    let setup = uvc_set_cur_vc(vc_if, entity, selector, 2);
    let buf = value.to_le_bytes();
    dwc2_ep0::ep0_control_write(dev, setup, ep0_mps, &buf).is_ok()
}

/// 把 ProcessingUnit 的 1/2 字节控制项设到 `override_val`；为 `None` 则用 `GET_DEF`。
/// 只有 `bmControls` 标记支持的 selector 才会发送。
fn pu_apply_one(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    pu: u8,
    bm: u32,
    bit: u32,
    selector: u8,
    width: u8,
    name: &str,
    override_val: Option<u16>,
) {
    if (bm & (1u32 << bit)) == 0 {
        return;
    }
    let want_src = override_val.map(|_| "override").unwrap_or("def");
    match width {
        1 => {
            let cur = try_get_cur_u8(dev, ep0_mps, vc_if, pu, selector);
            let want = match override_val {
                Some(v) => Some(v as u8),
                None => try_get_def_u8(dev, ep0_mps, vc_if, pu, selector),
            };
            match (cur, want) {
                (Some(c), Some(d)) if c != d => {
                    let ok = try_set_cur_u8(dev, ep0_mps, vc_if, pu, selector, d);
                    usb_log_fmt(format_args!(
                        "UVC: PU.{name} {c} -> {d} ({want_src}, {})",
                        if ok { "ok" } else { "set 失败" }
                    ));
                }
                (None, _) => usb_log_fmt(format_args!("UVC: PU.{name} GET_CUR 失败")),
                _ => {}
            }
        }
        2 => {
            let cur = try_get_cur_u16(dev, ep0_mps, vc_if, pu, selector);
            let want = match override_val {
                Some(v) => Some(v),
                None => try_get_def_u16(dev, ep0_mps, vc_if, pu, selector),
            };
            match (cur, want) {
                (Some(c), Some(d)) if c != d => {
                    let ok = try_set_cur_u16(dev, ep0_mps, vc_if, pu, selector, d);
                    usb_log_fmt(format_args!(
                        "UVC: PU.{name} {c} -> {d} ({want_src}, {})",
                        if ok { "ok" } else { "set 失败" }
                    ));
                }
                (None, _) => usb_log_fmt(format_args!("UVC: PU.{name} GET_CUR 失败")),
                _ => {}
            }
        }
        _ => {}
    }
}

/// 上层可选的图像调节覆盖。每一项 `Some(v)` 表示用 `v` SET_CUR 覆盖摄像头出厂默认；
/// `None` 表示沿用 `GET_DEF` 出厂默认。
///
/// 使用例：把 `brightness=Some(96)` 让画面比 def 的 128 暗一些。
#[derive(Clone, Copy, Debug, Default)]
pub struct UvcImageTuning {
    pub brightness: Option<u16>,
    pub contrast: Option<u16>,
    pub hue: Option<u16>,
    pub saturation: Option<u16>,
    pub sharpness: Option<u16>,
    pub gamma: Option<u16>,
    pub backlight: Option<u16>,
    pub gain: Option<u16>,
    /// `Some(K)` = 关 Auto WB、用手动色温 K（典型 2800–6500）。`None` = 开 Auto WB。
    pub white_balance_temp_k: Option<u16>,
    /// 0=Disabled, 1=50Hz, 2=60Hz；`None` 时按 50Hz 设置。
    pub power_line_freq: Option<u8>,
}

/// 摄像头初始化常用控制：**自动白平衡**、**自动曝光**、**关闭手动 Hue**、**Power-line 50Hz**、
/// **Focus-Auto**。每一项都按 `bmControls` 决定是否发送，不支持就跳过；STALL 也只是日志，不致命。
///
/// `tune` 中 `Some(v)` 字段会用 `v` 覆盖出厂 def，`None` 字段则沿用 def。
///
/// `0c45:64ab` 等 SunplusIT/Sonix 摄像头出厂在某些场景下默认白平衡是手动模式，
/// 这是图像偏色（偏蓝/偏紫）的最常见原因。
pub fn uvc_init_camera_controls(
    dev: u32,
    ep0_mps: u32,
    ent: &UvcControlEntities,
    tune: &UvcImageTuning,
) -> UsbResult<()> {
    usb_log_fmt(format_args!(
        "UVC: VC if={} CT={:?} (bm={:#010x}) PU={:?} (bm={:#010x})",
        ent.vc_interface,
        ent.camera_terminal_id,
        ent.ct_controls,
        ent.processing_unit_id,
        ent.pu_controls
    ));

    if let Some(pu) = ent.processing_unit_id {
        let vc_if = ent.vc_interface;
        let bm = ent.pu_controls;

        // ① 图像调节参数：tune.* 为 Some 则覆盖；None 则用 GET_DEF。
        // PU bmControls 位定义（UVC 1.5）：
        //   D0=Brightness D1=Contrast D2=Hue D3=Saturation D4=Sharpness
        //   D5=Gamma D6=WB Temp D8=Backlight D9=Gain D10=PowerLineFreq
        //   D11=Hue Auto D12=WB Temp Auto
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 0, PU_BRIGHTNESS_CONTROL, 2, "Brightness", tune.brightness);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 1, PU_CONTRAST_CONTROL, 2, "Contrast", tune.contrast);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 2, PU_HUE_CONTROL, 2, "Hue", tune.hue);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 3, PU_SATURATION_CONTROL, 2, "Saturation", tune.saturation);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 4, PU_SHARPNESS_CONTROL, 2, "Sharpness", tune.sharpness);
        // PU_GAMMA selector = 0x09
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 5, 0x09, 2, "Gamma", tune.gamma);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 8, PU_BACKLIGHT_COMPENSATION, 2, "Backlight", tune.backlight);
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, 9, PU_GAIN_CONTROL, 2, "Gain", tune.gain);

        // ② 白平衡
        match tune.white_balance_temp_k {
            // 用户指定手动色温 → 关 Auto，写手动 K
            Some(k) if (bm & (1 << 6)) != 0 => {
                if (bm & (1 << 12)) != 0 {
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 0);
                }
                let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, k);
                usb_log_fmt(format_args!("UVC: PU.WB = {k}K (manual)"));
            }
            // 默认走 Auto WB（若支持 D12）
            _ => {
                if (bm & (1 << 12)) != 0 {
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 0);
                    if (bm & (1 << 6)) != 0 {
                        if let Some(d) = try_get_def_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL) {
                            let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, d);
                        }
                    }
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 1);
                    let cur_t = try_get_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL).unwrap_or(0);
                    usb_log_fmt(format_args!("UVC: PU.WB = Auto (cur {cur_t}K)"));
                } else if (bm & (1 << 6)) != 0 {
                    let val = try_get_def_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL).unwrap_or(4500);
                    let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, val);
                    usb_log_fmt(format_args!("UVC: PU.WB = {val}K (no-auto)"));
                }
            }
        }

        if (bm & (1 << 11)) != 0 {
            let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_HUE_AUTO_CONTROL, 1);
        }

        if (bm & (1 << 10)) != 0 {
            let plf = tune.power_line_freq.unwrap_or(1);
            let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_POWER_LINE_FREQUENCY_CONTROL, plf);
        }
    }

    if let Some(ct) = ent.camera_terminal_id {
        // CT bmControls：D1=AE Mode, D2=AE Priority, D17=Focus Auto
        if (ent.ct_controls & (1 << 1)) != 0 {
            // AE Mode 是位掩码（UVC 1.5）：0x01=Manual, 0x02=Auto,
            // 0x04=Shutter Priority, 0x08=Aperture Priority。
            // 廉价 webcam 多数只接受 0x08（光圈优先=自动曝光），先试 0x02 失败则降级。
            let mut applied = 0u8;
            for &mode in &[0x02u8, 0x08, 0x04] {
                if try_set_cur_u8(dev, ep0_mps, ent.vc_interface, ct, CT_AE_MODE_CONTROL, mode) {
                    applied = mode;
                    break;
                }
            }
            usb_log_fmt(format_args!("UVC: CT.AeMode = {applied:#04x}"));

            if (ent.ct_controls & (1 << 2)) != 0 {
                let _ = try_set_cur_u8(
                    dev,
                    ep0_mps,
                    ent.vc_interface,
                    ct,
                    CT_AE_PRIORITY_CONTROL,
                    1,
                );
            }
        }

        if (ent.ct_controls & (1 << 17)) != 0 {
            let _ = try_set_cur_u8(
                dev,
                ep0_mps,
                ent.vc_interface,
                ct,
                CT_FOCUS_AUTO_CONTROL,
                1,
            );
        }
    }

    Ok(())
}

fn build_probe_commit_payload(sel: &UvcStreamSelection) -> [u8; UVC_PROBE_COMMIT_LEN] {
    let mut b = [0u8; UVC_PROBE_COMMIT_LEN];
    // 实测 0c45:64ab 等廉价 webcam 不论我们 wCompQuality 设几（1/47/10000），
    // 一旦 bmHint.D3=1 锁定 wCompQuality 就会切到"低质量量化表"（吐 ~21K），
    // 反而比不锁定（吐 ~26K）糟糕。所以这里只锁定 frame interval (D0=1)，
    // 不锁定 wCompQuality (D3=0)，让设备用出厂默认 quality。
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

    // 根据协商出的 dwMaxPayloadTransferSize 重新选 Isoch alt（跳过 mult>1）。
    reselect_isoch_alt_for_payload(sel);

    // 若 reselect 降级到了更低带宽的 alt（例如 mult=1），须把 COMMIT 中的
    // dwMaxPayloadTransferSize 压到该 alt 的实际每微帧吞吐，否则摄像头
    // 按 3060 B 分包、主机只收 1020 B/uframe 会导致数据截断。
    let alt_mps = u32::from(sel.mps_raw & 0x7FF)
        * (u32::from((sel.mps_raw >> 11) & 0x3) + 1);
    if alt_mps > 0 && alt_mps < sel.negotiated_payload_size {
        usb_log_fmt(format_args!(
            "UVC: clamping COMMIT dwMaxPayloadTransferSize {} -> {} to match alt bandwidth",
            sel.negotiated_payload_size, alt_mps
        ));
        sel.negotiated_payload_size = alt_mps;
        probe[22..26].copy_from_slice(&alt_mps.to_le_bytes());
    }

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

/// 将 VS 接口切回 `alt=0`，并清空抓帧连续性状态。
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
        // 检查累积 JPEG 末尾是否真带 EOI(`ff d9`)：0c45:64ab 等廉价 webcam 会在正常帧之间
        // 插入 "元数据帧"——带 SOI 但**无** EOI，且 FID 也会翻转。仅看 saw_data 会让这种
        // 残帧（典型 1008 字节）被误判为帧结束，上抛后被 caller 校验失败、必须重试。
        // 这里要求 EOI 真实存在；不在则丢弃累积、用新 FID 重新开始帧，把当前 packet 当作
        // 新帧首包正常累积，**不**回调 caller 也不浪费一次完整的 capture。
        let has_eoi = *jpeg_len >= 2
            && dwc2_ep0::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF + *jpeg_len - 2, 2)
                .map(|t| t[0] == 0xff && t[1] == 0xd9)
                .unwrap_or(false);
        if FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            usb_log_fmt(format_args!(
                "UVC-trace FID-flip uf={} frame_fid={} new_fid={} saw_data={} jpeg_len={} has_eoi={}",
                dwc2_ep0::current_uframe(), *frame_fid, cur_fid, *saw_data, *jpeg_len, has_eoi
            ));
        }
        if *saw_data && has_eoi {
            return Ok(true);
        }
        // 残帧（无 EOI）：丢弃，把当前 packet 作为新帧的首包，state 切到新 FID。
        *jpeg_len = 0;
        *frame_fid = cur_fid;
        *saw_data = false;
    }
    if payload_len > 0 {
        let hlen = pkt[0] as usize;
        let payload = &pkt[hlen..];
        // 第一次累积时校验 payload 必须以 SOI(`ff d8`) 开头：摄像头在帧间会插入 padding
        // packet（同一 FID 但 payload 不带 SOI），如果就这样累积下去会得到"首字节非 ff d8"
        // 的截断帧。这里跳过该 packet，等同 FID 内下一个真 SOI 开头的 packet 再开始累积。
        if !*saw_data && (payload.len() < 2 || payload[0] != 0xff || payload[1] != 0xd8) {
            return Ok(false);
        }
        if jpeg_len.checked_add(payload.len()).unwrap_or(usize::MAX) > jpeg_cap {
            return Err(UsbError::Hardware("video assemble overflow"));
        }
        dwc2_ep0::dma_write_at(UVC_ASSEMBLED_JPEG_DMA_OFF + *jpeg_len, payload)?;
        *jpeg_len += payload.len();
        *saw_data = true;
    }
    // EOF 时同样校验 EOI(`ff d9`)：0c45:64ab 的 metadata 帧带合法 EOF 标记但 jpeg 仅
    // 有 SOI，没有 EOI（典型 1008 字节）。仅看 EOF flag 会让这种残帧被上抛。
    let has_eoi_now = *jpeg_len >= 2
        && dwc2_ep0::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF + *jpeg_len - 2, 2)
            .map(|t| t[0] == 0xff && t[1] == 0xd9)
            .unwrap_or(false);
    let frame_done = eof && *saw_data && has_eoi_now;
    if eof && FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
        usb_log_fmt(format_args!(
            "UVC-trace EOF uf={} fid={} info={:#04x} payload={} saw_data={} jpeg_len={} has_eoi={} done={}",
            dwc2_ep0::current_uframe(), cur_fid, info, payload_len, *saw_data, *jpeg_len, has_eoi_now, frame_done
        ));
    }
    if eof && *saw_data && !has_eoi_now {
        // 残帧（带 EOF 但无 EOI）：丢弃累积，回到 WaitFirstSwitch 等下一次 FID 翻转。
        // 不返回 false 让 caller 误以为"还在累积"——直接置 state 回 wait 让下一帧从干净状态开始。
        *jpeg_len = 0;
        *saw_data = false;
        *state = FrameState::WaitFirstSwitch { last_fid: Some(cur_fid) };
        return Ok(false);
    }
    Ok(frame_done)
}

/// 全局开关：为 `true` 时在抓帧路径上对 EOF / FID 翻转打印微帧级 trace。
pub static FRAME_DEBUG: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// 上层（如 helloworld）可设置一个"像素数上限"来引导 [`parse_uvc_video_stream`]
/// 优先选择尺寸 ≤ 此上限的 frame。0 表示不限制（沿用旧的"最大分辨率优先"行为）。
///
/// 适用场景：低端 webcam 硬件 JPEG 编码器吐 1280×720 时只有 ~26K 字节，每像素
/// 0.028B 必然出现块状/马赛克伪影。把上限设为 640×480 = 307_200 后，同样字节数
/// 的 JPEG 在更小分辨率上"每像素 byte 数"提升约 3 倍，主观质量显著改善。
pub static PREFERRED_MAX_PIXELS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

/// 设置 [`PREFERRED_MAX_PIXELS`]。必须在 [`parse_uvc_video_stream`] 之前调用。
pub fn set_preferred_max_pixels(p: u32) {
    PREFERRED_MAX_PIXELS.store(p, core::sync::atomic::Ordering::Relaxed);
}

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
    let frame_dbg = FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed);
    // 只有 FRAME_DEBUG=true 时才采集 uframe 时间戳，避免每帧多余的 mmio 读。
    let uf_start = if frame_dbg { dwc2_ep0::current_uframe() } else { 0 };
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
                    if frame_dbg {
                        usb_log_fmt(format_args!("UVC: frame {} bytes ({} bulk transfers)", jpeg_len, transfers));
                    }
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
                if frame_dbg && was_waiting && matches!(state, FrameState::Capturing { .. }) {
                    uf_first_switch = dwc2_ep0::current_uframe();
                    uframes_at_switch = transfers;
                }
                if eof {
                    if let FrameState::Capturing { frame_fid, .. } = state {
                        LAST_EOF_FID.store(frame_fid, core::sync::atomic::Ordering::Relaxed);
                    }
                    if frame_dbg {
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
                    }
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
