//! Isochronous IN 传输（通道 5）。

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::LocalRegisterCopy;

use crate::usb::error::{UsbError, UsbResult};
use crate::utils::{cache, spin_delay};
use super::channel::{
    self, hcchar_build, pktcnt_for, usb_bus_fence_before_dma, CH_BULK, PID_DATA0, PID_DATA1,
    PID_DATA2,
};
use super::dma::{self, DMA_OFF_UFRAME_BUF, UFRAME_BUF_SIZE, UVC_BULK_DMA_CAP};
use super::isr::HCINT_ALL_W1C;
use super::regs::{HCCHAR, HCINT, HCTSIZ, HFNUM};

use crate::usb::{dwc2_channel as channel_regs, dwc2_regs};

/// 当前 USB 微帧编号（`HFNUM` 低 16 位）；每 microframe (125µs) 递增并回绕。
#[inline]
pub fn current_uframe() -> u32 {
    dwc2_regs().hfnum.read(HFNUM::FRNUM)
}

/// Isoch IN 高带宽：在 **下一微帧** 启动一次通道，最多接收 `mult` 个 USB 事务（每个 ≤ `mps` 字节）。
pub fn isoch_in_uframe(dev: u32, ep: u32, mps_raw: u16, dma_off: usize) -> UsbResult<usize> {
    let mps = u32::from(mps_raw & 0x7ff);
    let mult = u32::from((mps_raw >> 11) & 0x3) + 1;
    if mps == 0 || mult == 0 || mult > 3 {
        return Err(UsbError::Protocol("bad isoch mps_raw"));
    }
    let xfersize = mps.saturating_mul(mult);
    if (xfersize as usize) > UVC_BULK_DMA_CAP {
        return Err(UsbError::Protocol("isoch xfer > dma cap"));
    }
    let pid = match mult {
        3 => PID_DATA2,
        2 => PID_DATA1,
        _ => PID_DATA0,
    };
    let pktcnt = mult;

    unsafe {
        let hc_base = hcchar_build(dev, ep, mps, HCCHAR::EPTYPE::Isochronous, true, mult.clamp(1, 3));
        let tsiz =
            (HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pktcnt) + HCTSIZ::XFERSIZE.val(xfersize)).value;

        let c = channel_regs(CH_BULK);
        channel::ch_wait_disabled(CH_BULK)?;
        channel::ch_halt(CH_BULK);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(tsiz);
        let dmap = dma::dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let mut oddfrm_reg = LocalRegisterCopy::<u32, HCCHAR::Register>::new(0);
        if dwc2_regs().hfnum.read(HFNUM::FRNUM) & 1 == 0 {
            oddfrm_reg.modify(HCCHAR::ODDFRM::SET);
        }
        let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc_base | oddfrm_reg.get());
        armed.modify(HCCHAR::CHENA::SET);
        c.hcchar.set(armed.get());

        let st = channel::ch_wait_halted(CH_BULK)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.is_set(HCINT::AHBERR) {
            return Err(UsbError::Hardware("AHBERR on isoch"));
        }
        if st.any_matching_bits_set(
            HCINT::FRMOVRN::SET
                + HCINT::XACTERR::SET
                + HCINT::BBLERR::SET
                + HCINT::DATATGLERR::SET
                + HCINT::NYET::SET
                + HCINT::NAK::SET,
        ) {
            return Ok(0);
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            return Ok(0);
        }
        let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
        let actual = xfersize.saturating_sub(rem) as usize;
        if actual > 0 {
            cache::dcache_invalidate_after_dma(dma::dma_ptr().add(dma_off), actual);
        }
        Ok(actual)
    }
}

/// 批量 Isoch IN：在一次调用中处理多个 uframe，减少函数调用开销。
pub fn isoch_in_uframe_batch<F>(
    dev: u32,
    ep: u32,
    mps_raw: u16,
    max_uframes: u32,
    mut callback: F,
) -> UsbResult<(u32, u32)>
where
    F: FnMut(u32, &[u8]) -> UsbResult<bool>,
{
    let mps = u32::from(mps_raw & 0x7ff);
    let mult = u32::from((mps_raw >> 11) & 0x3) + 1;
    if mps == 0 || mult == 0 || mult > 3 {
        return Err(UsbError::Protocol("bad isoch mps_raw"));
    }
    let xfersize = mps.saturating_mul(mult);
    if (xfersize as usize) > UFRAME_BUF_SIZE {
        return Err(UsbError::Protocol("isoch xfer > uframe buf size"));
    }
    let pid = match mult {
        3 => PID_DATA2,
        2 => PID_DATA1,
        _ => PID_DATA0,
    };
    let pktcnt = mult;

    let mut total_uframes = 0u32;
    let mut data_uframes = 0u32;
    let mut consecutive_empty = 0u32;

    unsafe {
        let hc_base = hcchar_build(dev, ep, mps, HCCHAR::EPTYPE::Isochronous, true, mult.clamp(1, 3));
        let tsiz =
            (HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pktcnt) + HCTSIZ::XFERSIZE.val(xfersize)).value;
        let c = channel_regs(CH_BULK);
        let dmap = dma::dma_phys(DMA_OFF_UFRAME_BUF);

        channel::ch_wait_disabled(CH_BULK)?;
        channel::ch_halt(CH_BULK);

        c.hcsplt.set(0);
        let mut oddfrm = {
            let mut r = LocalRegisterCopy::<u32, HCCHAR::Register>::new(0);
            if dwc2_regs().hfnum.read(HFNUM::FRNUM) & 1 == 0 {
                r.modify(HCCHAR::ODDFRM::SET);
            }
            r.get()
        };

        for uframe_idx in 0..max_uframes {
            total_uframes += 1;

            if consecutive_empty > 0 {
                spin_delay(32.min(consecutive_empty * 2));
            }

            c.hcint.set(HCINT_ALL_W1C);
            c.hctsiz.set(tsiz);
            usb_bus_fence_before_dma();
            c.hcdma.set(dmap);
            usb_bus_fence_before_dma();
            let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc_base | oddfrm);
            armed.modify(HCCHAR::CHENA::SET);
            c.hcchar.set(armed.get());

            let st = channel::ch_wait_halted(CH_BULK)?;
            oddfrm = {
                let mut r = LocalRegisterCopy::<u32, HCCHAR::Register>::new(0);
                if dwc2_regs().hfnum.read(HFNUM::FRNUM) & 1 == 0 {
                    r.modify(HCCHAR::ODDFRM::SET);
                }
                r.get()
            };

            if st.is_set(HCINT::XFERCOMPL) {
                let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
                let actual = xfersize.saturating_sub(rem) as usize;
                if actual > 0 {
                    cache::dcache_invalidate_after_dma(dma::dma_ptr().add(DMA_OFF_UFRAME_BUF), actual);
                    data_uframes += 1;
                    consecutive_empty = 0;
                    let slice = dma::dma_rx_slice(DMA_OFF_UFRAME_BUF, actual)
                        .ok_or(UsbError::Hardware("dma view"))?;
                    if callback(uframe_idx, slice)? {
                        break;
                    }
                } else {
                    consecutive_empty += 1;
                }
            } else if st.is_set(HCINT::STALL) {
                return Err(UsbError::Stall);
            } else if st.is_set(HCINT::AHBERR) {
                return Err(UsbError::Hardware("AHBERR on isoch"));
            } else {
                consecutive_empty += 1;
                if consecutive_empty > 0 {
                    spin_delay(32.min(consecutive_empty * 2));
                }
            }
        }

        channel::ch_halt(CH_BULK);
    }

    Ok((total_uframes, data_uframes))
}

/// 单包 Isoch IN（非高带宽）。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep`：Isoch IN 端点号。
/// - `mps`：单包最大字节数（与端点 `wMaxPacketSize` 一致）。
/// - `len`：本次读取长度，须 **≤ `mps`** 且非零。
/// - `dma_off`：DMA 窗口内接收偏移。
#[allow(dead_code)]
pub fn isoch_in(dev: u32, ep: u32, mps: u32, len: usize, dma_off: usize) -> UsbResult<usize> {
    if len == 0 || len > 0x7ffff || len > mps as usize {
        return Err(UsbError::Protocol("bad isoch in len"));
    }
    unsafe {
        let hc = hcchar_build(dev, ep, mps, HCCHAR::EPTYPE::Isochronous, true, 1);
        let pkts = pktcnt_for(mps, len as u32);
        let tsiz =
            (HCTSIZ::PID.val(PID_DATA0) + HCTSIZ::PKTCNT.val(pkts) + HCTSIZ::XFERSIZE.val(len as u32))
                .value;

        let c = channel_regs(CH_BULK);
        channel::ch_wait_disabled(CH_BULK)?;
        channel::ch_halt(CH_BULK);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(tsiz);
        let dmap = dma::dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let mut oddfrm_reg = LocalRegisterCopy::<u32, HCCHAR::Register>::new(0);
        if dwc2_regs().hfnum.read(HFNUM::FRNUM) & 1 == 0 {
            oddfrm_reg.modify(HCCHAR::ODDFRM::SET);
        }
        let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc | oddfrm_reg.get());
        armed.modify(HCCHAR::CHENA::SET);
        c.hcchar.set(armed.get());

        let st = channel::ch_wait_halted(CH_BULK)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.any_matching_bits_set(
            HCINT::FRMOVRN::SET
                + HCINT::XACTERR::SET
                + HCINT::BBLERR::SET
                + HCINT::NYET::SET,
        ) {
            return Ok(0);
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            return Ok(0);
        }
        let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
        let actual = (len as u32).saturating_sub(rem) as usize;
        if actual > 0 {
            cache::dcache_invalidate_after_dma(dma::dma_ptr().add(dma_off), actual);
        }
        Ok(actual)
    }
}
