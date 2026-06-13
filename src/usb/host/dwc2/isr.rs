//! DWC2 主机中断：PLIC ISR、通道 5 isoch 完成信号，以及 `GINTMSK`/`HAINTMSK` 配置。

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tock_registers::LocalRegisterCopy;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::regs::{GINTMSK, GINTSTS, HAINT, HAINTMSK, HCINT, HCTSIZ};
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::{dwc2_channel, dwc2_regs};
use crate::utils::wait_for_irq_or_spin;

/// Bulk/Isoch 传输通道（与 Linux DWC2 HCD 动态分配一致）。
const CH_BULK: u32 = 5;

/// `HCINT` 快照（通道 halt 时读出的中断原因位，供上层区分 XFERCOMPL / NAK / STALL 等）。
#[allow(dead_code)]
pub type HcintSnapshot = LocalRegisterCopy<u32, HCINT::Register>;

/// HCINT 写 1 清除：清完整 11 位（含 ACK/NYET 等）。
pub(crate) const HCINT_ALL_W1C: u32 = 0x7FF;

/// 等时传输关注的 HCINT 位掩码（XFERCOMPL / CHHLTD / 错误类）。
const HCINT_ISOCH_MASK: u32 = 0x39f;

// ---------------------------------------------------------------------------
// 中断驱动 isoch 传输：ISR ↔ 主循环通信
// ---------------------------------------------------------------------------

/// ISR 信号：一次 isoch 传输已完成，主循环可处理数据。
pub(crate) static ISOCH_DONE: AtomicBool = AtomicBool::new(false);

/// ISR 写入的 HCINT 快照（主循环读取以判断 XFERCOMPL / 错误）。
pub(crate) static ISOCH_HCINT: AtomicU32 = AtomicU32::new(0);

/// ISR 写入的本次传输实际接收字节数（0 = 无数据 / 错误）。
pub(crate) static ISOCH_ACTUAL: AtomicU32 = AtomicU32::new(0);

/// 主循环 → ISR：本次传输的 `HCTSIZ` 原始值（用于计算 `actual = xfersize - XFERSIZE`）。
pub(crate) static ISOCH_HCTSIZ: AtomicU32 = AtomicU32::new(0);

/// DWC2 USB 中断号（SG200x DTS: `usb@04340000 { interrupts = <30 ...> }`）。
pub const DWC2_IRQ_NUM: usize = 30;

/// 屏蔽全局中断并写 1 清除全部 `GINTSTS` 挂起位（`dwc2_host_init` 开头调用）。
pub fn dwc2_host_irq_mask_and_clear() {
    let regs = dwc2_regs();
    regs.gintmsk.set(0);
    regs.gintsts.set(0xFFFF_FFFF);
}

/// 使能通道 5 主机通道中断汇总，并再次清除 `GINTSTS`（`dwc2_host_init` 末尾调用）。
pub fn dwc2_host_irq_enable() {
    let regs = dwc2_regs();
    regs.haintmsk.modify(HAINTMSK::CHINT.val(1 << CH_BULK));
    regs.gintmsk.modify(GINTMSK::HCHINT::SET);
    regs.gintsts.set(0xFFFF_FFFF);
}

/// DWC2 中断处理函数：在 CHHLTD 时读取结果并通知主循环。
///
/// 由 PLIC 调用（通过 `axhal::irq::register` 注册）。
pub fn dwc2_interrupt_handler() {
    let regs = dwc2_regs();

    if !regs.gintsts.is_set(GINTSTS::HCHINT) {
        return;
    }

    if regs.haint.read(HAINT::CHINT) & (1u32 << CH_BULK) == 0 {
        regs.gintsts.modify(GINTSTS::HCHINT::SET);
        return;
    }

    let c = dwc2_channel(CH_BULK);
    let hi = c.hcint.extract();
    c.hcint.set(hi.get());

    if hi.is_set(HCINT::CHHLTD) {
        let mut actual = 0u32;
        if hi.is_set(HCINT::XFERCOMPL) {
            let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
            let xfersize = ISOCH_HCTSIZ.load(Ordering::Acquire) & 0x7FFFF;
            actual = xfersize.saturating_sub(rem);
        }
        ISOCH_HCINT.store(hi.get(), Ordering::Release);
        ISOCH_ACTUAL.store(actual, Ordering::Release);
        ISOCH_DONE.store(true, Ordering::Release);
    }

    regs.gintsts.modify(GINTSTS::HCHINT::SET);
}

/// 使能通道 5 等时传输相关 HCINT 中断（`isoch_in_uframe_batch` 每次 arm 前调用）。
#[inline(always)]
pub(crate) fn enable_isoch_channel_irq() {
    dwc2_channel(CH_BULK).hcintmsk.set(HCINT_ISOCH_MASK);
}

/// 主循环在 arm 通道前重置 ISR ↔ 主循环共享状态。
#[inline(always)]
pub(crate) fn prepare_isoch_done(hctsiz: u32) {
    ISOCH_HCTSIZ.store(hctsiz, Ordering::Release);
    ISOCH_HCINT.store(0, Ordering::Release);
    ISOCH_ACTUAL.store(0, Ordering::Release);
    ISOCH_DONE.store(false, Ordering::Release);
}

/// 读取并消费一次 ISR 完成的 isoch 传输结果。
fn consume_isoch_done() -> Option<(HcintSnapshot, u32)> {
    if !ISOCH_DONE.load(Ordering::Acquire) {
        return None;
    }
    let hcint = ISOCH_HCINT.load(Ordering::Acquire);
    let actual = ISOCH_ACTUAL.load(Ordering::Acquire);
    ISOCH_DONE.store(false, Ordering::Release);
    Some((
        LocalRegisterCopy::<u32, HCINT::Register>::new(hcint),
        actual,
    ))
}

/// 等待一次 isoch 传输完成：先快速轮询，再 `wfi` 等待 IRQ。
///
/// 返回 `(HCINT 快照, 实际接收字节数)`；超时返回 [`UsbError::Timeout`]。
pub(crate) fn wait_isoch_done(timeout_loops: u32) -> UsbResult<(HcintSnapshot, u32)> {
    const FAST_POLL_LOOPS: u32 = 256;

    let c = dwc2_channel(CH_BULK);

    // 阶段 1：短自旋轮询，覆盖 ISR 已就绪但原子标志尚未可见的窗口。
    for _ in 0..timeout_loops.min(FAST_POLL_LOOPS) {
        if let Some(done) = consume_isoch_done() {
            return Ok(done);
        }
        let hi = c.hcint.extract();
        if hi.is_set(HCINT::CHHLTD) {
            let mut actual = 0u32;
            if hi.is_set(HCINT::XFERCOMPL) {
                let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
                let xfersize = ISOCH_HCTSIZ.load(Ordering::Acquire) & 0x7FFFF;
                actual = xfersize.saturating_sub(rem);
            }
            c.hcint.set(hi.get());
            dwc2_regs().gintsts.modify(GINTSTS::HCHINT::SET);
            return Ok((hi, actual));
        }
        core::hint::spin_loop();
    }

    // 阶段 2：`wfi` 等 IRQ 唤醒；每 32 轮补一次寄存器轮询以防 IRQ 丢失。
    let mut wfi_round = 0u32;
    for _ in 0..timeout_loops {
        if let Some(done) = consume_isoch_done() {
            return Ok(done);
        }
        wfi_round = wfi_round.wrapping_add(1);
        if wfi_round & 0x1f == 0 {
            let hi = c.hcint.extract();
            if hi.is_set(HCINT::CHHLTD) {
                let mut actual = 0u32;
                if hi.is_set(HCINT::XFERCOMPL) {
                    let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
                    let xfersize = ISOCH_HCTSIZ.load(Ordering::Acquire) & 0x7FFFF;
                    actual = xfersize.saturating_sub(rem);
                }
                c.hcint.set(hi.get());
                dwc2_regs().gintsts.modify(GINTSTS::HCHINT::SET);
                return Ok((hi, actual));
            }
        }
        wait_for_irq_or_spin();
    }

    Err(UsbError::Timeout)
}
