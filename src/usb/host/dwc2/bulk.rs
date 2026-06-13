//! Bulk IN/OUT 传输（通道 5）。

use tock_registers::interfaces::Readable;

use super::channel::{CH_BULK, ch_xfer, ch_xfer_video_retryable, hcchar_build, pktcnt_for};
use super::dma;
use super::regs::{HCCHAR, HCTSIZ};
use crate::usb::error::{UsbError, UsbResult};
use crate::utils::{cache, spin_delay};

use crate::usb::dwc2_channel as channel_regs;

#[inline]
fn spin_short() {
    spin_delay(64);
}

/// Bulk OUT：将 `data` 写入内部 DMA 窗口后，经主机通道 5 发出。
pub fn bulk_out(
    dev: u32,
    ep: u32,
    mps: u32,
    pid: u32,
    data: &[u8],
    dma_off: usize,
) -> UsbResult<()> {
    if data.is_empty() || data.len() > 0x7ffff {
        return Err(UsbError::Protocol("bad bulk out len"));
    }
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), dma::dma_ptr().add(dma_off), data.len());
        cache::dcache_clean_for_dma(dma::dma_ptr().add(dma_off), data.len());
        let hc = hcchar_build(dev, ep, mps, HCCHAR::EPTYPE::Bulk, false, 0);
        let pkts = pktcnt_for(mps, data.len() as u32);
        ch_xfer(
            CH_BULK,
            hc,
            (HCTSIZ::PID.val(pid)
                + HCTSIZ::PKTCNT.val(pkts)
                + HCTSIZ::XFERSIZE.val(data.len() as u32))
            .value,
            dma_off as u32,
        )?;
        Ok(())
    }
}

/// Bulk IN；遇 NAK 自动重试（UVC 常见）。返回本事务实际收到的字节数。
pub fn bulk_in(
    dev: u32,
    ep: u32,
    mps: u32,
    pid: u32,
    len: usize,
    dma_off: usize,
) -> UsbResult<usize> {
    if len == 0 || len > 0x7ffff {
        return Err(UsbError::Protocol("bad bulk in len"));
    }
    unsafe {
        let hc = hcchar_build(dev, ep, mps, HCCHAR::EPTYPE::Bulk, true, 0);
        let pkts = pktcnt_for(mps, len as u32);
        let tsiz =
            (HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pkts) + HCTSIZ::XFERSIZE.val(len as u32))
                .value;

        for _ in 0..4_000_000u32 {
            match ch_xfer_video_retryable(CH_BULK, hc, tsiz, dma_off as u32) {
                Ok(_) => {}
                Err(UsbError::Nak) => {
                    spin_short();
                    continue;
                }
                Err(e) => return Err(e),
            }

            let rem = channel_regs(CH_BULK).hctsiz.read(HCTSIZ::XFERSIZE);
            let mut actual = (len as u32).saturating_sub(rem) as usize;
            actual = actual.min(len);
            if actual == 0 && len > 0 {
                actual = len;
            }
            if actual > 0 {
                cache::dcache_invalidate_after_dma(dma::dma_ptr().add(dma_off), actual);
            }
            return Ok(actual);
        }
        Err(UsbError::Timeout)
    }
}
