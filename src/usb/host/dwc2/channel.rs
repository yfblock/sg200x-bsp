//! 主机通道调度原语：启停通道、单次传输、HCCHAR 组装。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::LocalRegisterCopy;

use crate::usb::error::{UsbError, UsbResult};
use crate::utils::spin_delay;
use super::dma;
use super::isr::{HcintSnapshot, HCINT_ALL_W1C};
use super::regs::{HCCHAR, HCINT};

use crate::usb::dwc2_channel as channel;

/// EP0 控制传输固定用通道 0。
pub(crate) const CH_CTL: u32 = 0;
/// Bulk/Isoch 传输通道 5（与 Linux DWC2 HCD 动态分配一致）。
pub(crate) const CH_BULK: u32 = 5;

/// `HCTSIZ.PID` 编码：与 DesignWare 主机通道 `HCTSIZ` 字段一致（SETUP / DATA0/1/2）。
pub const PID_DATA0: u32 = 0;
pub const PID_DATA2: u32 = 1;
pub const PID_DATA1: u32 = 2;
pub const PID_SETUP: u32 = 3;

#[inline]
pub(crate) fn usb_bus_fence_before_dma() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence rw, rw", options(nostack));
    }
}

pub(crate) fn ch_wait_disabled(ch: u32) -> UsbResult<()> {
    let c = channel(ch);
    for _ in 0..2_000_000u32 {
        if !c.hcchar.is_set(HCCHAR::CHENA) {
            return Ok(());
        }
        spin_delay(2);
    }
    Err(UsbError::Timeout)
}

/// 若通道仍忙，按 Linux `dwc2_hc_halt` 同时置 `CHENA|CHDIS` 请求停止。
pub(crate) fn ch_halt(ch: u32) {
    let c = channel(ch);
    if !c.hcchar.is_set(HCCHAR::CHENA) {
        return;
    }
    c.hcchar.modify(HCCHAR::CHENA::SET + HCCHAR::CHDIS::SET);
    for _ in 0..500_000u32 {
        if !c.hcchar.is_set(HCCHAR::CHENA) {
            return;
        }
        spin_delay(2);
    }
}

pub(crate) fn ch_wait_halted(ch: u32) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    for _ in 0..8_000_000u32 {
        let hi = c.hcint.extract();
        if hi.is_set(HCINT::CHHLTD) {
            c.hcint.set(hi.get());
            return Ok(hi);
        }
        spin_delay(2);
    }
    Err(UsbError::Timeout)
}

/// 主机通道单次传输：清中断、写 `HCTSIZ`/`HCDMA`/`HCCHAR`，等待 `CHHLTD`。
/// EP0 上对 NAK / XACTERR 做有限次重试；STALL 立即返回。
pub(crate) unsafe fn ch_xfer(
    ch: u32,
    hcchar: u32,
    hctsiz: u32,
    dma_off: u32,
) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    let dmap = dma::dma_phys(dma_off as usize);

    const NAK_RETRIES: u32 = 64;
    const XACT_RETRIES: u32 = 8;
    let mut xact_left = XACT_RETRIES;
    for attempt in 0..=NAK_RETRIES {
        ch_wait_disabled(ch)?;
        ch_halt(ch);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(hctsiz);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hcchar);
        armed.modify(HCCHAR::CHENA::SET);
        c.hcchar.set(armed.get());
        let st = ch_wait_halted(ch)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.is_set(HCINT::XACTERR) {
            if xact_left == 0 {
                log::info!(
                    "USB-XACT EXHAUSTED ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                    ch,
                    hcchar,
                    hctsiz,
                    dmap,
                    st.get()
                );
                return Err(UsbError::Protocol("ch xfer error (XACT)"));
            }
            xact_left -= 1;
            spin_delay(2_000_000);
            continue;
        }
        if st.is_set(HCINT::NAK) {
            if attempt == NAK_RETRIES {
                log::info!(
                    "USB-NAK EXHAUSTED ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                    ch,
                    hcchar,
                    hctsiz,
                    dmap,
                    st.get()
                );
                return Err(UsbError::Protocol("ch xfer NAK exhausted"));
            }
            spin_delay(200_000);
            continue;
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            log::info!(
                "USB-CHHLTD-NO-XFER ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                ch,
                hcchar,
                hctsiz,
                dmap,
                st.get()
            );
            return Err(UsbError::Protocol("CHHLTD without XFERCOMPL"));
        }
        return Ok(st);
    }
    unreachable!()
}

/// 视频 Bulk/Isoch：**NAK 单独返回**（设备尚无帧数据时常见），调用方忙等重试。
pub(crate) unsafe fn ch_xfer_video_retryable(
    ch: u32,
    hcchar: u32,
    hctsiz: u32,
    dma_off: u32,
) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    ch_wait_disabled(ch)?;
    ch_halt(ch);
    c.hcsplt.set(0);
    c.hcint.set(HCINT_ALL_W1C);
    c.hctsiz.set(hctsiz);
    let dmap = dma::dma_phys(dma_off as usize);
    usb_bus_fence_before_dma();
    c.hcdma.set(dmap);
    usb_bus_fence_before_dma();
    let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hcchar);
    armed.modify(HCCHAR::CHENA::SET);
    c.hcchar.set(armed.get());
    let st = ch_wait_halted(ch)?;
    if st.is_set(HCINT::STALL) {
        return Err(UsbError::Stall);
    }
    if st.is_set(HCINT::XACTERR) {
        return Err(UsbError::Protocol("ch xfer XACTERR"));
    }
    if st.is_set(HCINT::NAK) {
        return Err(UsbError::Nak);
    }
    if !st.is_set(HCINT::XFERCOMPL) {
        return Err(UsbError::Protocol("CHHLTD without XFERCOMPL"));
    }
    Ok(st)
}

/// 组装 `HCCHAR` 通道描述字（不含 `CHENA`/`ODDFRM`）。
#[inline]
pub(crate) fn hcchar_build(
    dev: u32,
    ep: u32,
    mps: u32,
    eptype: tock_registers::fields::FieldValue<u32, HCCHAR::Register>,
    dir_in: bool,
    mc: u32,
) -> u32 {
    let mut h = LocalRegisterCopy::<u32, HCCHAR::Register>::new(0);
    let mut fields = HCCHAR::MPS.val(mps & 0x7ff)
        + HCCHAR::EPNUM.val(ep & 0xf)
        + eptype
        + HCCHAR::DEVADDR.val(dev & 0x7f);
    if mc != 0 {
        fields = fields + HCCHAR::MC.val(mc & 0x3);
    }
    if dir_in {
        fields = fields + HCCHAR::EPDIR::SET;
    }
    h.modify(fields);
    h.get()
}

/// 计算 `HCTSIZ.PKTCNT`：按 `mps` 分包后的包数（至少为 1）。
pub(crate) fn pktcnt_for(mps: u32, nbytes: u32) -> u32 {
    if mps == 0 {
        return 1;
    }
    nbytes.div_ceil(mps)
}
