//! Isochronous IN 传输（通道 5）。

use tock_registers::LocalRegisterCopy;
use tock_registers::interfaces::{Readable, Writeable};

use super::channel::{
    self, CH_BULK, PID_DATA0, PID_DATA1, PID_DATA2, hcchar_build, pktcnt_for,
    usb_bus_fence_before_dma,
};
use super::dma::{self, UVC_BULK_DMA_CAP};
use super::isr::{HCINT_ALL_W1C, enable_isoch_channel_irq, prepare_isoch_done, wait_isoch_done};
use super::regs::{HCCHAR, HCINT, HCTSIZ, HFNUM};
use crate::usb::error::{UsbError, UsbResult};
use crate::utils::cache;

use crate::usb::{dwc2_channel as channel_regs, dwc2_regs};

/// `HCCHAR.ODDFRM`（bit 29）：与 `HFNUM.FRNUM` 奇偶对齐，每微帧切换一次即可。
const HCCHAR_ODDFRM_BIT: u32 = 1 << 29;

#[inline(always)]
fn oddfrm_for_current_uframe() -> u32 {
    if dwc2_regs().hfnum.read(HFNUM::FRNUM) & 1 == 0 {
        HCCHAR_ODDFRM_BIT
    } else {
        0
    }
}

/// 当前 USB 微帧编号（`HFNUM` 低 16 位）；每 microframe (125µs) 递增并回绕。
#[inline(always)]
pub fn current_uframe() -> u32 {
    dwc2_regs().hfnum.read(HFNUM::FRNUM)
}

/// Isoch IN 高带宽：优化轮询模式。
///
/// 移除 spin_delay，纯寄存器轮询，最大化性能。
#[inline(never)] // 不内联，避免代码膨胀
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
        let hc_base = hcchar_build(
            dev,
            ep,
            mps,
            HCCHAR::EPTYPE::Isochronous,
            true,
            mult.saturating_sub(1),
        );
        let c = channel_regs(CH_BULK);

        // 快速路径：如果通道已经禁用，跳过等待
        if c.hcchar.is_set(HCCHAR::CHENA) {
            channel::ch_wait_disabled(CH_BULK)?;
            channel::ch_halt(CH_BULK);
        }

        // 设置传输参数
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.write(
            HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pktcnt) + HCTSIZ::XFERSIZE.val(xfersize),
        );
        let dmap = dma::dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();

        // 构建 HCCHAR 值
        let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc_base | oddfrm_for_current_uframe());
        armed.modify(HCCHAR::CHENA::SET);

        // 启动传输
        c.hcchar.set(armed.get());

        // 优化轮询：无 spin_delay，纯寄存器检查
        // 热路径：快速检查（大多数情况下会在前几次迭代完成）
        for _ in 0..1000u32 {
            let hi = c.hcint.extract();
            if hi.is_set(HCINT::CHHLTD) {
                c.hcint.set(hi.get());
                return process_hcint(hi, c, xfersize, dma_off);
            }
        }
        // 冷路径：完整等待
        for _ in 0..7_999_000u32 {
            let hi = c.hcint.extract();
            if hi.is_set(HCINT::CHHLTD) {
                c.hcint.set(hi.get());
                return process_hcint(hi, c, xfersize, dma_off);
            }
        }

        Err(UsbError::Timeout)
    }
}

/// 处理 HCINT 结果（内联热路径）
#[inline(always)]
unsafe fn process_hcint(
    hi: LocalRegisterCopy<u32, HCINT::Register>,
    c: &crate::usb::host::dwc2::regs::Dwc2HostChannel,
    xfersize: u32,
    dma_off: usize,
) -> UsbResult<usize> {
    if hi.is_set(HCINT::STALL) {
        return Err(UsbError::Stall);
    }
    if hi.is_set(HCINT::AHBERR) {
        return Err(UsbError::Hardware("AHBERR on isoch"));
    }
    if hi.any_matching_bits_set(
        HCINT::FRMOVRN::SET
            + HCINT::XACTERR::SET
            + HCINT::BBLERR::SET
            + HCINT::DATATGLERR::SET
            + HCINT::NYET::SET
            + HCINT::NAK::SET,
    ) {
        return Ok(0);
    }
    if !hi.is_set(HCINT::XFERCOMPL) {
        return Ok(0);
    }
    let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
    let actual = xfersize.saturating_sub(rem) as usize;
    if actual > 0 {
        cache::dcache_invalidate_after_dma(dma::dma_ptr().add(dma_off), actual);
    }
    Ok(actual)
}

/// 批量 Isoch IN：单缓冲顺序收包（arm → wait → 回调），中断 + WFI 等待。
#[inline(never)]
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
    use super::dma::{DMA_OFF_UFRAME_BUF, UFRAME_BUF_SIZE};

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

    unsafe {
        let hc_base = hcchar_build(
            dev,
            ep,
            mps,
            HCCHAR::EPTYPE::Isochronous,
            true,
            mult.saturating_sub(1),
        );
        let c = channel_regs(CH_BULK);

        channel::ch_wait_disabled(CH_BULK)?;
        channel::ch_halt(CH_BULK);
        enable_isoch_channel_irq();

        c.hcsplt.set(0);

        let arm = |oddfrm: u32| {
            c.hcint.set(HCINT_ALL_W1C);
            c.hctsiz.write(
                HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pktcnt) + HCTSIZ::XFERSIZE.val(xfersize),
            );
            usb_bus_fence_before_dma();
            c.hcdma.set(dma::dma_phys(DMA_OFF_UFRAME_BUF));
            usb_bus_fence_before_dma();
            prepare_isoch_done(c.hctsiz.get());
            let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc_base | oddfrm);
            armed.modify(HCCHAR::CHENA::SET);
            c.hcchar.set(armed.get());
            armed.get()
        };

        let mut oddfrm = oddfrm_for_current_uframe();

        for uframe_idx in 0..max_uframes {
            total_uframes += 1;

            let _armed_hcchar = arm(oddfrm);
            let (st, actual) = match wait_isoch_done(16_000) {
                Ok(done) => done,
                Err(e) => {
                    channel::ch_halt(CH_BULK);
                    return Err(e);
                }
            };
            oddfrm ^= HCCHAR_ODDFRM_BIT;

            if st.is_set(HCINT::XFERCOMPL) {
                let actual = actual.min(xfersize) as usize;
                if actual > 0 {
                    cache::dcache_invalidate_after_dma(
                        dma::dma_ptr().add(DMA_OFF_UFRAME_BUF),
                        actual,
                    );
                    data_uframes += 1;
                    let slice = dma::dma_rx_slice(DMA_OFF_UFRAME_BUF, actual)
                        .ok_or(UsbError::Hardware("dma view"))?;
                    if callback(uframe_idx, slice)? {
                        break;
                    }
                }
            } else if st.is_set(HCINT::STALL) {
                return Err(UsbError::Stall);
            } else if st.is_set(HCINT::AHBERR) {
                return Err(UsbError::Hardware("AHBERR on isoch"));
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

        let c = channel_regs(CH_BULK);
        channel::ch_wait_disabled(CH_BULK)?;
        channel::ch_halt(CH_BULK);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.write(
            HCTSIZ::PID.val(PID_DATA0) + HCTSIZ::PKTCNT.val(pkts) + HCTSIZ::XFERSIZE.val(len as u32),
        );
        let dmap = dma::dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let mut armed =
            LocalRegisterCopy::<u32, HCCHAR::Register>::new(hc | oddfrm_for_current_uframe());
        armed.modify(HCCHAR::CHENA::SET);
        c.hcchar.set(armed.get());

        let st = channel::ch_wait_halted(CH_BULK)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.any_matching_bits_set(
            HCINT::FRMOVRN::SET + HCINT::XACTERR::SET + HCINT::BBLERR::SET + HCINT::NYET::SET,
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
