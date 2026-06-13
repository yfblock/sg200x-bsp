//! 忙等延时（NOP 循环），用于外设初始化、电源/时钟稳定等场景。

use core::arch::asm;

/// 短延时（NOP 循环）。
///
/// # 参数
/// - `count`：循环次数
#[inline]
pub fn delay(count: usize) {
    for _ in 0..count {
        unsafe { asm!("nop") }
    }
}

/// 长延时（用于电源稳定等场景）。
#[inline]
pub fn delay_long() {
    delay(0x10_0000);
}

/// 短延时（用于时钟稳定等场景）。
#[inline]
pub fn delay_short() {
    delay(0x10);
}

/// CPU 自旋忙等（`spin_loop`），用于 USB 轮询、NAK 退避等无定时器场景。
///
/// # 参数
/// - `iterations`：自旋循环次数（非精确时间，与 CPU 频率相关）。
#[inline]
pub fn spin_delay(iterations: u32) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

/// 等待中断或自旋：RISC-V 上执行 `wfi` 让出 CPU 直至 IRQ 唤醒，其它架构退化为 `spin_loop`。
#[inline(always)]
pub fn wait_for_irq_or_spin() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }

    #[cfg(not(target_arch = "riscv64"))]
    core::hint::spin_loop();
}
