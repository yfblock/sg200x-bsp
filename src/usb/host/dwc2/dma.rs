//! 主机传输共用 DMA 窗口：EP0 工作区、UVC Bulk、Isoch uframe 缓冲。

use crate::usb;
use crate::usb::error::{UsbError, UsbResult};

/// EP0/小缓冲区 + UVC Bulk 大块 DMA（须物理连续；末段供 `bulk_in` 组装 MJPEG）。
#[repr(C, align(256))]
struct DmaBuf {
    bytes: [u8; 1024],
    uvc_bulk: [u8; 384 * 1024],
    /// Isoch uframe 接收工作区（单缓冲）。
    uframe_buf: [u8; 4096],
}

static mut DMA_BUF: DmaBuf = DmaBuf {
    bytes: [0; 1024],
    uvc_bulk: [0; 384 * 1024],
    uframe_buf: [0; 4096],
};

/// `bulk_in` / UVC 使用的 DMA 区起始偏移（紧跟在 1KiB EP0 工作区之后）。
pub const DMA_OFF_UVC_BULK: usize = 1024;
/// UVC 视频缓冲容量；前 `UVC_WORK_AREA_BYTES` 用作单微帧 RX 工作区，其余拼接 JPEG。
/// 720p MJPEG 单帧典型 100-300KB，需要 ≥320KB 的 JPEG 区。
pub const UVC_BULK_DMA_CAP: usize = 384 * 1024;

/// Isoch uframe 接收缓冲偏移（在 UVC Bulk 区之后）。
pub const DMA_OFF_UFRAME_BUF: usize = 1024 + 384 * 1024;
/// 单个 uframe 接收缓冲区大小。
pub const UFRAME_BUF_SIZE: usize = 4096;
/// 整个 `DmaBuf` 大小（供边界检查）。
const DMA_BUF_TOTAL: usize = 1024 + 384 * 1024 + 4096;
const _: () = assert!(DMA_BUF_TOTAL <= 1024 + 384 * 1024 + 4096);

/// EP0 SETUP/状态阶段工作区起始偏移。
pub(crate) const OFF_EP0: usize = 0;
/// EP0 小缓冲读（Hub 描述符、配置前缀、`GET_PORT_STATUS`），与 Bulk DMA 区错开。
pub(crate) const DMA_OFF_SMALL_IO: usize = 256;

/// MSC Command Block Wrapper（31 字节，对齐到 cache line）。
pub const DMA_OFF_CBW: usize = 320;
/// MSC Command Status Wrapper（13 字节，对齐到 cache line）。
pub const DMA_OFF_CSW: usize = 384;
/// MSC SCSI 数据区（与 UVC Bulk 区共享：MSC/UVC 互斥使用）。
pub const DMA_OFF_SECTOR: usize = DMA_OFF_UVC_BULK;
/// MSC SCSI 数据区最大字节数（与 UVC Bulk 区共享）。
pub const MSC_SECTOR_DMA_CAP: usize = UVC_BULK_DMA_CAP;

/// DMA 工作区基址（`static mut` 仅经裸指针访问，避免 `static_mut_refs`）。
#[inline]
pub(crate) fn dma_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(DMA_BUF).cast::<u8>()
}

pub(crate) fn dma_phys(off: usize) -> u32 {
    unsafe { usb::usb_dma_phys_for(dma_ptr().add(off)) }
}

/// 安全的只读视图，供 UVC 等解析刚完成的 `bulk_in` 数据（**仅**在 `bulk_in`/cache invalidate 之后调用）。
///
/// # 参数
/// - `off`：相对内部 DMA 窗口起始的字节偏移。
/// - `len`：要暴露的连续字节长度。
#[inline]
pub fn dma_rx_slice(off: usize, len: usize) -> Option<&'static [u8]> {
    if len == 0 || off.checked_add(len)? > DMA_BUF_TOTAL {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(dma_ptr().add(off), len) })
}

/// 将数据写入内部 DMA 窗口（CPU 写，供 UVC 组装 JPEG 等；写后需自行 `dcache_clean` 若要给 DMA 读）。
///
/// # 参数
/// - `off`：相对 DMA 窗口起始的偏移。
/// - `src`：要拷贝进去的源数据。
pub fn dma_write_at(off: usize, src: &[u8]) -> UsbResult<()> {
    let end = off
        .checked_add(src.len())
        .ok_or(UsbError::Protocol("dma write overflow"))?;
    if end > DMA_BUF_TOTAL {
        return Err(UsbError::Protocol("dma write out of buf"));
    }
    dma_append_unchecked(off, src);
    Ok(())
}

/// 热路径 payload 追加：调用方须已保证 `[off, off+src.len())` 在 DMA 窗口内。
#[inline(always)]
pub(crate) fn dma_append_unchecked(off: usize, src: &[u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dma_ptr().add(off), src.len());
    }
}

/// 从内部 DMA 窗口拷贝到 `dst`（**不**做 cache 维护；调用方须已 invalidate 或仅 CPU 写入区）。
///
/// # 参数
/// - `off`：源数据在 DMA 窗口内的起始字节偏移。
/// - `dst`：目标缓冲区；拷贝长度为 `dst.len()`。
pub fn dma_copy_out(off: usize, dst: &mut [u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(dma_ptr().add(off), dst.as_mut_ptr(), dst.len());
    }
}
