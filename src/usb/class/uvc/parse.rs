//! VS 配置描述符解析与流参数选择。

use crate::usb::UsbClass;
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2;
use crate::usb::setup;

use super::consts::*;

/// 解析得到的 VS 流参数（Isoch IN）。
#[derive(Clone, Debug)]
pub struct UvcStreamSelection {
    pub vs_interface: u8,
    pub alt_setting: u8,
    pub ep_num: u8,
    /// `wMaxPacketSize` 原始值（含 HS 带宽倍增位）。
    pub mps_raw: u16,
    pub format_index: u8,
    pub frame_index: u8,
    pub frame_interval: u32,
    /// 选定格式是否为 MJPEG（用于上层判断输出是否为 JPEG）。
    pub is_mjpeg: bool,
    pub frame_w: u16,
    pub frame_h: u16,
    /// PROBE/COMMIT 协商后设备使用的 `dwMaxPayloadTransferSize`（单微帧字节数）。
    /// 由 [`super::uvc_start_video_stream`] 在协商后填充，用于 capture 切包。
    pub negotiated_payload_size: u32,
    /// PROBE/COMMIT 协商后设备的 `dwMaxVideoFrameSize`（缓冲规划与诊断）。
    pub negotiated_frame_size: u32,
    /// 同一个 ep_num 下的所有 Isoch alt 候选 `(alt, mps_raw)`，按 mps*total 升序。
    /// PROBE 协商后用 stream 模块回选最匹配的 alt，避免出现
    /// "alt=1 但 negotiated_payload=3060" 这种带宽不够的 mismatch。
    pub isoch_alts_count: u8,
    pub isoch_alts: [(u8, u16); 8],
}

/// 上层（如 helloworld）可设置一个"像素数上限"来引导 [`parse_uvc_video_stream`]
/// 优先选择尺寸 ≤ 此上限的 frame。0 表示不限制（沿用旧的"最大分辨率优先"行为）。
pub static PREFERRED_MAX_PIXELS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
pub static PREFERRED_FRAME_INTERVAL: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

/// 设置 [`PREFERRED_MAX_PIXELS`]。必须在 [`parse_uvc_video_stream`] 之前调用。
pub fn set_preferred_max_pixels(p: u32) {
    PREFERRED_MAX_PIXELS.store(p, core::sync::atomic::Ordering::Relaxed);
}

/// 设置期望的 frame interval（UVC 100ns 单位）。0 表示使用设备最快 interval。
pub fn set_preferred_frame_interval(interval: u32) {
    PREFERRED_FRAME_INTERVAL.store(interval, core::sync::atomic::Ordering::Relaxed);
}

/// 通过 EP0 读取完整配置描述符（按首 9 字节里的 `wTotalLength`，最大 4096）。
pub fn read_configuration_descriptor(
    dev: u32,
    ep0_mps: u32,
    cfg_index: u8,
) -> UsbResult<[u8; 4096]> {
    let mut hdr = [0u8; 9];
    dwc2::ep0_control_read(
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
        return Err(UsbError::Protocol(
            "configuration descriptor too large (>4096)",
        ));
    }
    let mut buf = [0u8; 4096];
    dwc2::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(cfg_index, total as u16),
        ep0_mps,
        &mut buf[..total],
    )?;
    Ok(buf)
}

#[inline]
pub(crate) fn max_packet_11(mps_raw: u16) -> u32 {
    u32::from(mps_raw & 0x7FF)
}

/// 解析 VS 接口：优先选择 **MJPEG 格式**；若无 MJPEG 则使用未压缩 (YUY2/NV12) 格式。
/// 端点须为 **Isoch IN**（取带宽最高的 alt）。
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

    let mut best_isoch: Option<(u8, u8, u16, u8)> = None;
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
            && cur_ifc_class == UsbClass::Video.as_raw()
            && cur_ifc_sub == USB_SUBCLASS_VIDEO_STREAMING
        {
            let st = cfg.get(i + 2).copied().unwrap_or(0);
            if (st == VS_FORMAT_MJPEG || st == VS_FORMAT_UNCOMPRESSED) && bl >= 4 {
                last_fmt_subtype = st;
                last_fmt_ix = cfg[i + 3];
                cur_fmt_subtype_for_frame = st;
                cur_fmt_ix_for_frame = cfg[i + 3];
            }
            if (st == VS_FRAME_MJPEG || st == VS_FRAME_UNCOMPRESSED) && bl >= 26 {
                let frame_ix = cfg[i + 3];
                let w = u16::from_le_bytes([cfg[i + 5], cfg[i + 6]]);
                let h = u16::from_le_bytes([cfg[i + 7], cfg[i + 8]]);
                let dflt_ival =
                    u32::from_le_bytes([cfg[i + 21], cfg[i + 22], cfg[i + 23], cfg[i + 24]]);
                let ival_type = cfg[i + 25];
                let mut min_ival = dflt_ival;
                if ival_type == 0 && bl >= 38 {
                    let dw_min =
                        u32::from_le_bytes([cfg[i + 26], cfg[i + 27], cfg[i + 28], cfg[i + 29]]);
                    if dw_min > 0 {
                        min_ival = dw_min;
                    }
                } else if ival_type > 0 {
                    let n = ival_type as usize;
                    let mut p = i + 26;
                    for _ in 0..n {
                        if p + 4 > i + bl {
                            break;
                        }
                        let v = u32::from_le_bytes([cfg[p], cfg[p + 1], cfg[p + 2], cfg[p + 3]]);
                        if v > 0 && v < min_ival {
                            min_ival = v;
                        }
                        p += 4;
                    }
                }
                let preferred_ival =
                    PREFERRED_FRAME_INTERVAL.load(core::sync::atomic::Ordering::Relaxed);
                let selected_ival = if preferred_ival > 0 && ival_type > 0 {
                    let mut best = dflt_ival;
                    let mut best_delta = u32::MAX;
                    let mut p = i + 26;
                    for _ in 0..(ival_type as usize) {
                        if p + 4 > i + bl {
                            break;
                        }
                        let v = u32::from_le_bytes([cfg[p], cfg[p + 1], cfg[p + 2], cfg[p + 3]]);
                        if v > 0 {
                            let delta = v.abs_diff(preferred_ival);
                            if delta < best_delta {
                                best_delta = delta;
                                best = v;
                            }
                        }
                        p += 4;
                    }
                    best
                } else if preferred_ival > 0 && ival_type == 0 && bl >= 38 {
                    let dw_min =
                        u32::from_le_bytes([cfg[i + 26], cfg[i + 27], cfg[i + 28], cfg[i + 29]]);
                    let dw_max =
                        u32::from_le_bytes([cfg[i + 30], cfg[i + 31], cfg[i + 32], cfg[i + 33]]);
                    let dw_step =
                        u32::from_le_bytes([cfg[i + 34], cfg[i + 35], cfg[i + 36], cfg[i + 37]]);
                    if dw_min > 0 && dw_max >= dw_min && preferred_ival >= dw_min {
                        let clamped = preferred_ival.min(dw_max);
                        if dw_step > 0 {
                            dw_min + ((clamped - dw_min) / dw_step) * dw_step
                        } else {
                            clamped
                        }
                    } else if min_ival > 0 {
                        min_ival
                    } else {
                        dflt_ival
                    }
                } else if min_ival > 0 {
                    min_ival
                } else {
                    dflt_ival
                };
                let pick = (cur_fmt_ix_for_frame, frame_ix, w, h, selected_ival);
                let is_mjpeg = cur_fmt_subtype_for_frame == VS_FORMAT_MJPEG || st == VS_FRAME_MJPEG;
                fn rank((_, _, pw, ph, _): (u8, u8, u16, u16, u32)) -> i32 {
                    let w = pw as i32;
                    let h = ph as i32;
                    let area = w * h;
                    let pref_max =
                        PREFERRED_MAX_PIXELS.load(core::sync::atomic::Ordering::Relaxed) as i32;
                    if pref_max > 0 {
                        return if area <= pref_max {
                            area.saturating_add(1_000_000)
                        } else {
                            (-(area - pref_max)).saturating_sub(1_000)
                        };
                    }
                    match (w, h) {
                        (1280, 720) => 1_000_000,
                        (640, 480) => 900_000,
                        (800, 600) => 800_000,
                        (1024, 768) => 750_000,
                        (320, 240) => 700_000,
                        _ if area <= 1280 * 720 => 600_000 - (1280 * 720 - area),
                        _ => 100_000 - (area - 1280 * 720),
                    }
                }
                if is_mjpeg {
                    let pick_better = match mjpeg_pick {
                        None => true,
                        Some(prev) => rank(pick) > rank(prev),
                    };
                    if pick_better {
                        mjpeg_pick = Some(pick);
                    }
                } else {
                    let pick_better = match uncomp_pick {
                        None => true,
                        Some(prev) => rank(pick) > rank(prev),
                    };
                    if pick_better {
                        uncomp_pick = Some(pick);
                    }
                }
                let _ = last_fmt_subtype;
                let _ = last_fmt_ix;
            }
        } else if ty == USB_DT_ENDPOINT
            && cur_ifc_class == UsbClass::Video.as_raw()
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
            let _total = u32::from(mps) * u32::from(mult);
            if xfer != ENDPOINT_ATTR_ISOCH {
                i += bl;
                continue;
            }
            let tak = (cur_alt, ep_num, mps_raw, cur_ifc_num);
            let new_mult = mult;
            best_isoch = Some(match best_isoch {
                None => tak,
                Some(b) => {
                    let old_mps = b.2 & 0x7FF;
                    let old_mult = ((b.2 >> 11) & 0x3) + 1;
                    let new_score = u32::from(mps) * u32::from(new_mult);
                    let old_score = u32::from(old_mps) * u32::from(old_mult);
                    if new_score > old_score { tak } else { b }
                }
            });
            if isoch_alts_count < isoch_alts.len() {
                isoch_alts[isoch_alts_count] = (cur_alt, mps_raw);
                isoch_alts_count += 1;
            }
        }

        i += bl;
    }

    let (alt, epn, mps_raw, vs_if) = best_isoch.ok_or(UsbError::NotImplemented)?;

    let (fmt_ix, frame_ix, frame_w, frame_h, interval, is_mjpeg) = match mjpeg_pick {
        Some((fi, frix, w, h, iv)) => (fi, frix, w, h, iv, true),
        None => match uncomp_pick {
            Some((fi, frix, w, h, iv)) => (fi, frix, w, h, iv, false),
            None => (1, 1, 0, 0, 333_333, false),
        },
    };

    Ok(UvcStreamSelection {
        vs_interface: vs_if,
        alt_setting: alt,
        ep_num: epn,
        mps_raw,
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
