//! Isoch IN 抓帧：UVC 包头解析、FID/EOF 组帧。

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::{self, DMA_OFF_UVC_BULK, UVC_BULK_DMA_CAP};

use super::parse::{UvcStreamSelection, max_packet_11};

pub const UVC_WORK_AREA_BYTES: usize = 65536;
pub const UVC_ASSEMBLED_JPEG_DMA_OFF: usize = DMA_OFF_UVC_BULK + UVC_WORK_AREA_BYTES;

#[derive(Clone, Copy, Debug, Default)]
pub struct UvcStreamRunStats {
    pub frames: u64,
    pub bytes: u64,
    pub total_uframes: u32,
    pub data_uframes: u32,
}

fn parse_uvc_packet(pkt: &[u8]) -> (bool, usize, Option<u8>) {
    if pkt.len() < 2 {
        return (false, 0, None);
    }
    let hlen = pkt[0] as usize;
    if hlen < 2 || hlen > pkt.len() {
        return (false, 0, None);
    }
    let info = pkt[1];
    let cur_fid = info & 0x01;
    let payload_len = pkt.len() - hlen;
    let eof = (info & 0x02) != 0;
    (eof, payload_len, Some(cur_fid))
}

fn payload_starts_soi(pkt: &[u8]) -> bool {
    if pkt.len() < 4 {
        return false;
    }
    let hlen = pkt[0] as usize;
    hlen <= pkt.len().saturating_sub(2) && pkt[hlen] == 0xff && pkt[hlen + 1] == 0xd8
}

fn assembled_eoi_len(jpeg_len: usize) -> Option<usize> {
    if jpeg_len < 2 {
        return None;
    }
    let tail = dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF + jpeg_len - 2, 2)?;
    if tail[0] == 0xff && tail[1] == 0xd9 {
        return Some(jpeg_len);
    }
    let data = dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF, jpeg_len)?;
    let mut i = data.len().saturating_sub(2);
    loop {
        if data.get(i) == Some(&0xff) && data.get(i + 1) == Some(&0xd9) {
            return Some(i + 2);
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

enum FrameState {
    WaitFirstSwitch { last_fid: Option<u8> },
    Capturing {
        frame_fid: u8,
        saw_data: bool,
    },
}

pub static LAST_EOF_FID: core::sync::atomic::AtomicU8 =
    core::sync::atomic::AtomicU8::new(0xFF);

#[inline]
pub fn reset_frame_continuity() {
    LAST_EOF_FID.store(0xFF, core::sync::atomic::Ordering::Relaxed);
}

struct CapturingPacket<'a> {
    pkt: &'a [u8],
    payload_len: usize,
    eof: bool,
    cur_fid: u8,
    jpeg_len: &'a mut usize,
    jpeg_cap: usize,
}

fn min_jpeg_bytes_for(_sel: &UvcStreamSelection) -> usize {
    4096
}

fn try_complete_video_frame(
    jpeg_len: &mut usize,
    frame_fid: u8,
    min_jpeg: usize,
    completed_len: &mut Option<usize>,
) -> bool {
    if !is_likely_video_jpeg(*jpeg_len, min_jpeg) {
        return false;
    }
    trim_eoi_in_place(jpeg_len);
    *completed_len = Some(*jpeg_len);
    LAST_EOF_FID.store(frame_fid, core::sync::atomic::Ordering::Relaxed);
    true
}

fn is_likely_video_jpeg(jpeg_len: usize, min_bytes: usize) -> bool {
    if jpeg_len < min_bytes {
        return false;
    }
    dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF, 2)
        .map(|s| s[0] == 0xff && s[1] == 0xd8)
        .unwrap_or(false)
        && assembled_eoi_len(jpeg_len).is_some()
}

fn trim_eoi_in_place(jpeg_len: &mut usize) {
    if let Some(n) = assembled_eoi_len(*jpeg_len) {
        *jpeg_len = n;
    }
}

fn process_packet(
    pkt: &[u8],
    state: &mut FrameState,
    jpeg_len: &mut usize,
    jpeg_cap: usize,
    min_jpeg: usize,
    completed_len: &mut Option<usize>,
) -> UsbResult<bool> {
    let (eof, payload_len, fid_opt) = parse_uvc_packet(pkt);
    let Some(cur_fid) = fid_opt else {
        return Ok(false);
    };

    match state {
        FrameState::WaitFirstSwitch { last_fid } => {
            match *last_fid {
                None if payload_starts_soi(pkt) => {
                    *state = FrameState::Capturing {
                        frame_fid: cur_fid,
                        saw_data: false,
                    };
                    let p = CapturingPacket {
                        pkt,
                        payload_len,
                        eof,
                        cur_fid,
                        jpeg_len,
                        jpeg_cap,
                    };
                    return process_packet_capturing(state, p, min_jpeg, completed_len);
                }
                None => *last_fid = Some(cur_fid),
                Some(prev) if prev == cur_fid => {}
                Some(prev) if prev != cur_fid => {
                    *state = FrameState::Capturing {
                        frame_fid: cur_fid,
                        saw_data: false,
                    };
                    let p = CapturingPacket {
                        pkt,
                        payload_len,
                        eof,
                        cur_fid,
                        jpeg_len,
                        jpeg_cap,
                    };
                    return process_packet_capturing(state, p, min_jpeg, completed_len);
                }
                Some(_) if payload_starts_soi(pkt) => {
                    *state = FrameState::Capturing {
                        frame_fid: cur_fid,
                        saw_data: false,
                    };
                    let p = CapturingPacket {
                        pkt,
                        payload_len,
                        eof,
                        cur_fid,
                        jpeg_len,
                        jpeg_cap,
                    };
                    return process_packet_capturing(state, p, min_jpeg, completed_len);
                }
                _ => {}
            }
            Ok(false)
        }
        FrameState::Capturing { .. } => {
            let p = CapturingPacket {
                pkt,
                payload_len,
                eof,
                cur_fid,
                jpeg_len,
                jpeg_cap,
            };
            process_packet_capturing(state, p, min_jpeg, completed_len)
        }
    }
}

fn process_packet_capturing(
    state: &mut FrameState,
    p: CapturingPacket<'_>,
    min_jpeg: usize,
    completed_len: &mut Option<usize>,
) -> UsbResult<bool> {
    let (mut frame_fid, mut saw_data) = match *state {
        FrameState::Capturing {
            frame_fid,
            saw_data,
        } => (frame_fid, saw_data),
        FrameState::WaitFirstSwitch { .. } => return Ok(false),
    };

    if p.cur_fid != frame_fid {
        if saw_data && *p.jpeg_len > 0 {
            let _ = try_complete_video_frame(p.jpeg_len, frame_fid, min_jpeg, completed_len);
        }
        *p.jpeg_len = 0;
        frame_fid = p.cur_fid;
        saw_data = false;
    }
    if p.payload_len > 0 {
        let hlen = p.pkt[0] as usize;
        let payload = &p.pkt[hlen..];
        if !saw_data && (payload.len() < 2 || payload[0] != 0xff || payload[1] != 0xd8) {
            *state = FrameState::Capturing {
                frame_fid,
                saw_data,
            };
            return Ok(completed_len.is_some());
        }
        if p.jpeg_len.checked_add(payload.len()).unwrap_or(usize::MAX) > p.jpeg_cap {
            return Err(UsbError::Hardware("video assemble overflow"));
        }
        let dst_off = UVC_ASSEMBLED_JPEG_DMA_OFF + *p.jpeg_len;
        dwc2::dma_append_unchecked(dst_off, payload);
        *p.jpeg_len += payload.len();
        saw_data = true;
    }
    let mut next_state = FrameState::Capturing {
        frame_fid,
        saw_data,
    };
    if p.eof && saw_data {
        if try_complete_video_frame(p.jpeg_len, frame_fid, min_jpeg, completed_len) {
            *p.jpeg_len = 0;
            next_state = FrameState::Capturing {
                frame_fid: frame_fid ^ 1,
                saw_data: false,
            };
        } else if *p.jpeg_len > 0 {
            *p.jpeg_len = 0;
            next_state = FrameState::Capturing {
                frame_fid: frame_fid ^ 1,
                saw_data: false,
            };
        }
    }
    *state = next_state;
    Ok(completed_len.is_some())
}

fn process_uvc_slice(
    slice: &[u8],
    mult: usize,
    mps_low: usize,
    state: &mut FrameState,
    jpeg_len: &mut usize,
    jpeg_cap: usize,
    min_jpeg: usize,
    completed_len: &mut Option<usize>,
) -> UsbResult<bool> {
    if mult == 1 {
        return process_packet(
            slice,
            state,
            jpeg_len,
            jpeg_cap,
            min_jpeg,
            completed_len,
        );
    }
    let mut hit = false;
    let mut off = 0usize;
    while off < slice.len() {
        let end = if slice.len() - off >= mps_low {
            off + mps_low
        } else {
            slice.len()
        };
        let pkt = &slice[off..end];
        off = end;
        if process_packet(
            pkt,
            state,
            jpeg_len,
            jpeg_cap,
            min_jpeg,
            completed_len,
        )? {
            hit = true;
            break;
        }
    }
    Ok(hit)
}

/// 连续抓帧直到 `should_stop()` 返回 true 或达到 `max_uframes` 上限。
pub fn uvc_stream_frames<F>(
    dev: u32,
    sel: &UvcStreamSelection,
    max_uframes: u32,
    mut should_stop: F,
) -> UsbResult<UvcStreamRunStats>
where
    F: FnMut() -> bool,
{
    let ep = u32::from(sel.ep_num);
    let mps_low = max_packet_11(sel.mps_raw).max(1) as usize;
    let mult = (((sel.mps_raw >> 11) & 0x3) as u32 + 1).clamp(1, 3) as usize;
    let jpeg_cap = UVC_BULK_DMA_CAP.saturating_sub(UVC_WORK_AREA_BYTES);
    let min_jpeg = min_jpeg_bytes_for(sel);
    let mut jpeg_len = 0usize;
    let prev_eof_fid = LAST_EOF_FID.load(core::sync::atomic::Ordering::Relaxed);
    let mut state = FrameState::WaitFirstSwitch {
        last_fid: if prev_eof_fid <= 1 {
            Some(prev_eof_fid)
        } else {
            None
        },
    };
    let mut stats = UvcStreamRunStats::default();

    let (total_uf, data_uf) = dwc2::isoch_in_uframe_batch(
        dev,
        ep,
        sel.mps_raw,
        max_uframes,
        |_uframe_idx, slice| {
            let mut frame_done_len = None;
            let eof = process_uvc_slice(
                slice,
                mult,
                mps_low,
                &mut state,
                &mut jpeg_len,
                jpeg_cap,
                min_jpeg,
                &mut frame_done_len,
            )?;

            if eof {
                if let Some(n) = frame_done_len {
                    stats.frames = stats.frames.saturating_add(1);
                    stats.bytes = stats.bytes.saturating_add(n as u64);
                }
                if matches!(state, FrameState::WaitFirstSwitch { .. }) {
                    jpeg_len = 0;
                }
            }

            Ok(should_stop())
        },
    )?;

    stats.total_uframes = total_uf;
    stats.data_uframes = data_uf;
    Ok(stats)
}

pub fn uvc_capture_one_frame(dev: u32, sel: &UvcStreamSelection) -> UsbResult<usize> {
    let ep = u32::from(sel.ep_num);
    let mps_low = max_packet_11(sel.mps_raw).max(1) as usize;
    let mult = (((sel.mps_raw >> 11) & 0x3) as u32 + 1).clamp(1, 3) as usize;
    let jpeg_cap = UVC_BULK_DMA_CAP.saturating_sub(UVC_WORK_AREA_BYTES);
    let min_jpeg = min_jpeg_bytes_for(sel);
    let mut jpeg_len = 0usize;
    let prev_eof_fid = LAST_EOF_FID.load(core::sync::atomic::Ordering::Relaxed);
    let mut state = FrameState::WaitFirstSwitch {
        last_fid: if prev_eof_fid <= 1 {
            Some(prev_eof_fid)
        } else {
            None
        },
    };

    const MAX_UFRAMES: u32 = 80_000;
    let mut captured_len = 0usize;

    let (total_uf, data_uf) = dwc2::isoch_in_uframe_batch(
        dev,
        ep,
        sel.mps_raw,
        MAX_UFRAMES,
        |_uframe_idx, slice| {
            let mut frame_done_len = None;
            let eof = process_uvc_slice(
                slice,
                mult,
                mps_low,
                &mut state,
                &mut jpeg_len,
                jpeg_cap,
                min_jpeg,
                &mut frame_done_len,
            )?;

            if eof {
                if let Some(n) = frame_done_len {
                    captured_len = n;
                    return Ok(true);
                }
                if matches!(state, FrameState::WaitFirstSwitch { .. }) {
                    jpeg_len = 0;
                }
            }
            Ok(false)
        },
    )?;

    if captured_len > 0 {
        return Ok(captured_len);
    }

    log::info!(
        "UVC: capture timeout after {} uframes ({} data; {} bytes assembled, mult={})",
        total_uf,
        data_uf,
        jpeg_len,
        mult
    );
    Err(UsbError::Timeout)
}
