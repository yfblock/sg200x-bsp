//! DWC2 主机中断：PLIC ISR、通道 5 isoch 完成信号，以及 `GINTMSK`/`HAINTMSK` 配置。

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::LocalRegisterCopy;

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::{dwc2_channel as channel, dwc2_regs};
use super::regs::{GINTMSK, GINTSTS, HAINT, HAINTMSK, HCCHAR, HCINT, HCTSIZ};

/// Bulk/Isoch 传输通道（与 Linux DWC2 HCD 动态分配一致）。
const CH_BULK: u32 = 5;

/// `HCINT` 快照（通道 halt 时读出的中断原因位，供上层区分 XFERCOMPL / NAK / STALL 等）。
#[allow(dead_code)]
pub type HcintSnapshot = LocalRegisterCopy<u32, HCINT::Register>;

/// HCINT 写 1 清除：清完整 11 位（含 ACK/NYET 等）。
pub(crate) const HCINT_ALL_W1C: u32 = 0x7FF;

// ---------------------------------------------------------------------------
// 中断驱动 isoch 传输：ISR ↔ 主循环通信
// ---------------------------------------------------------------------------

/// ISR 信号：一次 isoch 传输已完成，主循环可处理数据。
static ISOCH_DONE: AtomicBool = AtomicBool::new(false);

/// ISR 写入的 HCINT 快照（主循环读取以判断 XFERCOMPL / 错误）。
static ISOCH_HCINT: AtomicU32 = AtomicU32::new(0);

/// ISR 写入的本次传输实际接收字节数（0 = 无数据 / 错误）。
static ISOCH_ACTUAL: AtomicU32 = AtomicU32::new(0);

/// 主循环 → ISR：请求重新启动传输（ISR 在 CHHLTD 后自动重编程通道）。
static ISOCH_REARM: AtomicBool = AtomicBool::new(false);

/// ISR 重编程所需的寄存器值（由主循环在启动前写入）。
static ISOCH_HCCHAR: AtomicU32 = AtomicU32::new(0);
static ISOCH_HCTSIZ: AtomicU32 = AtomicU32::new(0);
static ISOCH_HCDMA: AtomicU32 = AtomicU32::new(0);

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

/// DWC2 中断处理函数：在 CHHLTD 时读取结果并重编程通道。
///
/// 由 PLIC 调用（通过 `axhal::irq::register` 注册）。
pub fn dwc2_interrupt_handler() {
    let regs = dwc2_regs();

    if !regs.gintsts.is_set(GINTSTS::HCHINT) {
        return;
    }
    log::info!("USB-ISR: gintsts={:#010x}", regs.gintsts.get());

    if regs.haint.read(HAINT::CHINT) & (1u32 << CH_BULK) == 0 {
        return;
    }

    let c = channel(CH_BULK);
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

        if ISOCH_REARM.load(Ordering::Acquire) {
            ISOCH_REARM.store(false, Ordering::Release);
            let hcchar = ISOCH_HCCHAR.load(Ordering::Acquire);
            let hctsiz = ISOCH_HCTSIZ.load(Ordering::Acquire);
            let hcdma = ISOCH_HCDMA.load(Ordering::Acquire);
            c.hcint.set(HCINT_ALL_W1C);
            c.hctsiz.set(hctsiz);
            unsafe { core::arch::asm!("fence rw, rw", options(nostack)); }
            c.hcdma.set(hcdma);
            unsafe { core::arch::asm!("fence rw, rw", options(nostack)); }
            let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hcchar);
            armed.modify(HCCHAR::CHENA::SET);
            c.hcchar.set(armed.get());
        }
    }

    regs.gintsts.modify(GINTSTS::HCHINT::SET);
}

/// 快速轮询等待 `GINTSTS.HCHINT` + 通道 `CHHLTD`：无 `spin_delay`。
///
/// 中断驱动替代方案——直接轮询 DWC2 全局中断状态，避免 PLIC 延迟。
#[inline]
#[allow(dead_code)]
pub fn ch_wait_halted_fast(ch: u32) -> UsbResult<HcintSnapshot> {
    let regs = dwc2_regs();
    let c = channel(ch);
    for _ in 0..400_000u32 {
        let hi = c.hcint.extract();
        if hi.is_set(HCINT::CHHLTD) {
            c.hcint.set(hi.get());
            regs.gintsts.modify(GINTSTS::HCHINT::SET);
            return Ok(hi);
        }
    }
    Err(UsbError::Timeout)
}
