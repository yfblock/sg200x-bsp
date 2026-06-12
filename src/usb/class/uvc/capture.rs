//! Isoch IN 抓帧：UVC 包头解析、FID/EOF 组帧。

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::{self, DMA_OFF_UVC_BULK, UVC_BULK_DMA_CAP};

use super::parse::{max_packet_11, UvcStreamSelection};

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

/// UVC 抓帧过程中单包解析上下文（`Capturing` 状态）。
struct CapturingPacket<'a> {
    pkt: &'a [u8],
    payload_len: usize,
    eof: bool,
    cur_fid: u8,
    info: u8,
    jpeg_len: &'a mut usize,
    jpeg_cap: usize,
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
        log::info!("UVC-pkt uf={} len={} hlen={} info={:#04x} fid={:?} eof={} payload={}",
            dwc2::current_uframe(),
            pkt.len(), if pkt.is_empty() { 0 } else { pkt[0] as usize },
            info, fid_opt, eof, payload_len);
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
                    let p = CapturingPacket { pkt, payload_len, eof, cur_fid, info, jpeg_len, jpeg_cap };
                    return process_packet_capturing(state, p);
                }
                _ => {}
            }
            Ok(false)
        }
        FrameState::Capturing { .. } => {
            let p = CapturingPacket { pkt, payload_len, eof, cur_fid, info, jpeg_len, jpeg_cap };
            process_packet_capturing(state, p)
        }
    }
}

fn process_packet_capturing(state: &mut FrameState, p: CapturingPacket<'_>) -> UsbResult<bool> {
    let FrameState::Capturing { frame_fid, saw_data } = state else {
        return Ok(false);
    };
    if p.cur_fid != *frame_fid {
        // 检查累积 JPEG 末尾是否真带 EOI(`ff d9`)：0c45:64ab 等廉价 webcam 会在正常帧之间
        // 插入 "元数据帧"——带 SOI 但**无** EOI，且 FID 也会翻转。仅看 saw_data 会让这种
        // 残帧（典型 1008 字节）被误判为帧结束，上抛后被 caller 校验失败、必须重试。
        // 这里要求 EOI 真实存在；不在则丢弃累积、用新 FID 重新开始帧，把当前 packet 当作
        // 新帧首包正常累积，**不**回调 caller 也不浪费一次完整的 capture。
        let has_eoi = *p.jpeg_len >= 2
            && dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF + *p.jpeg_len - 2, 2)
                .map(|t| t[0] == 0xff && t[1] == 0xd9)
                .unwrap_or(false);
        if FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::info!("UVC-trace FID-flip uf={} frame_fid={} new_fid={} saw_data={} jpeg_len={} has_eoi={}",
                dwc2::current_uframe(), *frame_fid, p.cur_fid, *saw_data, *p.jpeg_len, has_eoi);
        }
        if *saw_data && has_eoi {
            return Ok(true);
        }
        // 残帧（无 EOI）：丢弃，把当前 packet 作为新帧的首包，state 切到新 FID。
        *p.jpeg_len = 0;
        *frame_fid = p.cur_fid;
        *saw_data = false;
    }
    if p.payload_len > 0 {
        let hlen = p.pkt[0] as usize;
        let payload = &p.pkt[hlen..];
        // 第一次累积时校验 payload 必须以 SOI(`ff d8`) 开头：摄像头在帧间会插入 padding
        // packet（同一 FID 但 payload 不带 SOI），如果就这样累积下去会得到"首字节非 ff d8"
        // 的截断帧。这里跳过该 packet，等同 FID 内下一个真 SOI 开头的 packet 再开始累积。
        if !*saw_data && (payload.len() < 2 || payload[0] != 0xff || payload[1] != 0xd8) {
            return Ok(false);
        }
        if p.jpeg_len.checked_add(payload.len()).unwrap_or(usize::MAX) > p.jpeg_cap {
            return Err(UsbError::Hardware("video assemble overflow"));
        }
        dwc2::dma_write_at(UVC_ASSEMBLED_JPEG_DMA_OFF + *p.jpeg_len, payload)?;
        *p.jpeg_len += payload.len();
        *saw_data = true;
    }
    // EOF 时同样校验 EOI(`ff d9`)：0c45:64ab 的 metadata 帧带合法 EOF 标记但 jpeg 仅
    // 有 SOI，没有 EOI（典型 1008 字节）。仅看 EOF flag 会让这种残帧被上抛。
    let has_eoi_now = *p.jpeg_len >= 2
        && dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF + *p.jpeg_len - 2, 2)
            .map(|t| t[0] == 0xff && t[1] == 0xd9)
            .unwrap_or(false);
    let frame_done = p.eof && *saw_data && has_eoi_now;
    if p.eof && FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
        log::info!("UVC-trace EOF uf={} fid={} info={:#04x} payload={} saw_data={} jpeg_len={} has_eoi={} done={}",
            dwc2::current_uframe(), p.cur_fid, p.info, p.payload_len, *saw_data, *p.jpeg_len, has_eoi_now, frame_done);
    }
    if p.eof && *saw_data && !has_eoi_now {
        // 残帧（带 EOF 但无 EOI）：丢弃累积，回到 WaitFirstSwitch 等下一次 FID 翻转。
        // 不返回 false 让 caller 误以为"还在累积"——直接置 state 回 wait 让下一帧从干净状态开始。
        *p.jpeg_len = 0;
        *saw_data = false;
        *state = FrameState::WaitFirstSwitch { last_fid: Some(p.cur_fid) };
        return Ok(false);
    }
    Ok(frame_done)
}

/// 全局开关：为 `true` 时在抓帧路径上对 EOF / FID 翻转打印微帧级 trace。
pub static FRAME_DEBUG: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// 抓一帧（视频负载组装至 [`UVC_ASSEMBLED_JPEG_DMA_OFF`]）。
///
/// **关键**：等时模式下 `mult=1` 时，每次 `isoch_in_uframe` 返回的整个数据（最多 mps 字节）就是
/// **一个完整的 USB 包 = 一个 UVC 数据包**（带 12 字节头），**不可再切分**。
pub fn uvc_capture_one_frame(dev: u32, sel: &UvcStreamSelection) -> UsbResult<usize> {
    let ep = u32::from(sel.ep_num);
    let maxp = max_packet_11(sel.mps_raw).max(1);
    let mps_low = maxp as usize;
    let mult = (((sel.mps_raw >> 11) & 0x3) as u32 + 1).clamp(1, 3) as usize;
    let jpeg_cap = UVC_BULK_DMA_CAP.saturating_sub(UVC_WORK_AREA_BYTES);
    let mut jpeg_len = 0usize;
    let mut transfers = 0u32;
    let mut data_transfers = 0u32;
    let prev_eof_fid = LAST_EOF_FID.load(core::sync::atomic::Ordering::Relaxed);
    let mut state = FrameState::WaitFirstSwitch {
        last_fid: if prev_eof_fid <= 1 { Some(prev_eof_fid) } else { None },
    };
    let mut debug_remaining: u32 = 0;
    let frame_dbg = FRAME_DEBUG.load(core::sync::atomic::Ordering::Relaxed);
    // 只有 FRAME_DEBUG=true 时才采集 uframe 时间戳，避免每帧多余的 mmio 读。
    let uf_start = if frame_dbg { dwc2::current_uframe() } else { 0 };
    let mut uf_first_switch: u32 = uf_start;
    let mut uframes_at_switch: u32 = 0;

    const MAX_UFRAMES: u32 = 80_000;
    let mut eof_found = false;

    let (total_uf, data_uf) = dwc2::isoch_in_uframe_batch(
        dev,
        ep,
        sel.mps_raw,
        MAX_UFRAMES,
        |_uframe_idx, slice| {
            transfers = transfers.wrapping_add(1);
            data_transfers = data_transfers.wrapping_add(1);

            let was_waiting = matches!(state, FrameState::WaitFirstSwitch { .. });
            let eof = if mult == 1 {
                process_packet(slice, &mut state, &mut jpeg_len, jpeg_cap, &mut debug_remaining)?
            } else {
                let mut hit_eof = false;
                let mut off = 0usize;
                while off < slice.len() {
                    let end = if slice.len() - off >= mps_low {
                        off + mps_low
                    } else {
                        slice.len()
                    };
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
                uf_first_switch = dwc2::current_uframe();
                uframes_at_switch = transfers;
            }

            if eof {
                eof_found = true;
                if let FrameState::Capturing { frame_fid, .. } = state {
                    LAST_EOF_FID.store(frame_fid, core::sync::atomic::Ordering::Relaxed);
                }
                if frame_dbg {
                    let uf_end = dwc2::current_uframe();
                    let dwait = uf_first_switch.wrapping_sub(uf_start) & 0xffff;
                    let dcap = uf_end.wrapping_sub(uf_first_switch) & 0xffff;
                    log::info!("UVC: frame {} bytes ({} loops, {} data; HFNUM dwait={} uf ({}.{} ms / {} loops), dcap={} uf ({}.{} ms / {} loops), mult={})",
                        jpeg_len, transfers, data_transfers,
                        dwait, dwait / 8, (dwait % 8) * 125 / 10, uframes_at_switch,
                        dcap, dcap / 8, (dcap % 8) * 125 / 10, transfers - uframes_at_switch,
                        mult);
                }
                return Ok(true); // 停止批量处理
            }
            Ok(false) // 继续处理
        },
    )?;

    if eof_found {
        return Ok(jpeg_len);
    }

    log::info!("UVC: capture timeout after {} uframes ({} data; {} bytes assembled, mult={})",
        total_uf, data_uf, jpeg_len, mult);
    Err(UsbError::Timeout)
}
