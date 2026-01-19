//! # SD 卡驱动工具函数
//!
//! 本模块提供了 SD 卡驱动所需的底层工具函数，包括：
//! - 延时函数

use core::arch::asm;

/// 短延时 (NOP 循环)
///
/// # 参数
/// - `count`: 循环次数
#[inline]
pub fn delay(count: usize) {
    for _ in 0..count {
        unsafe { asm!("nop") }
    }
}

/// 长延时 (用于电源稳定等场景)
#[inline]
pub fn delay_long() {
    delay(0x10_0000);
}

/// 短延时 (用于时钟稳定等场景)
#[inline]
pub fn delay_short() {
    delay(0x10);
}
