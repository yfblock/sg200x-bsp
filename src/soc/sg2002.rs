//! SG2002 / CV181x SoC 外设 MMIO **物理基址** 一览。
//!
//! 常量按物理地址升序排列。各驱动模块通过 `pub use crate::soc::…` 保持原有路径的兼容性；
//! 新代码请优先使用本模块中的常量。

// =============================================================================
// 0x020B_xxxx — 多核 / 安全子系统
// =============================================================================

/// 安全子系统（协处理器启动控制）
pub const SEC_SYS_BASE: usize = 0x020B_0000;

// =============================================================================
// 0x0300_xxxx — 系统 / 时钟 / 复位 / 引脚复用 / IOBLK / USB PHY
// =============================================================================

/// TOP 模块（系统顶层控制寄存器）
pub const TOP_BASE: usize = 0x0300_0000;

/// FMUX（引脚功能复用）
pub const FMUX_BASE: usize = 0x0300_1000;

/// IOBLK（Active Domain：G1 / G7 / G10 / G12 共用基址，组内见 [`IOBLK_G*_OFFSET`]）
pub const IOBLK_BASE: usize = 0x0300_1800;

/// IOBLK G1 组偏移（相对 [`IOBLK_BASE`]）
pub const IOBLK_G1_OFFSET: usize = 0x000;

/// IOBLK G7 组偏移
pub const IOBLK_G7_OFFSET: usize = 0x100;

/// IOBLK G10 组偏移
pub const IOBLK_G10_OFFSET: usize = 0x200;

/// IOBLK G12 组偏移
pub const IOBLK_G12_OFFSET: usize = 0x400;

/// 时钟发生器（`clock-controller`，`cvitek,cv181x-clk`）
pub const CLKGEN_BASE: usize = 0x0300_2000;

/// 复位控制器（含 SOFT_RSTN_0~3、SOFT_CPU_RSTN 等）
pub const RSTC_BASE: usize = 0x0300_3000;

/// `RSTC` 内 `SOFT_CPU_RSTN` 寄存器偏移
pub const RSTC_SOFT_CPU_RSTN_OFFSET: usize = 0x024;

/// CPU 软复位寄存器绝对地址（[`RSTC_BASE`] + [`RSTC_SOFT_CPU_RSTN_OFFSET`]）
pub const SOFT_CPU_RSTN_ADDR: usize = RSTC_BASE + RSTC_SOFT_CPU_RSTN_OFFSET;

/// CV182x 片内 USB2 PHY（DTS `usb@04340000` 第二段 `reg`，物理基址见本常量）
pub const CV182X_USB2_PHY_BASE: usize = 0x0300_6000;

// =============================================================================
// 0x0302_xxxx — GPIO
// =============================================================================

/// GPIO0 (GPIOA)，Active Domain
pub const GPIO0_BASE: usize = 0x0302_0000;

/// GPIO1 (GPIOB)，Active Domain
pub const GPIO1_BASE: usize = 0x0302_1000;

/// GPIO2 (GPIOC)，Active Domain
pub const GPIO2_BASE: usize = 0x0302_2000;

/// GPIO3 (GPIOD)，Active Domain
pub const GPIO3_BASE: usize = 0x0302_3000;

// =============================================================================
// 0x0306_xxxx — PWM
// =============================================================================

/// PWM0 控制器
pub const PWM0_BASE: usize = 0x0306_0000;

/// PWM1 控制器
pub const PWM1_BASE: usize = 0x0306_1000;

/// PWM2 控制器
pub const PWM2_BASE: usize = 0x0306_2000;

/// PWM3 控制器
pub const PWM3_BASE: usize = 0x0306_3000;

// =============================================================================
// 0x0400_xxxx — I2C / 以太网
// =============================================================================

/// I2C0，Active Domain
pub const I2C0_BASE: usize = 0x0400_0000;

/// I2C1，Active Domain
pub const I2C1_BASE: usize = 0x0401_0000;

/// I2C2，Active Domain
pub const I2C2_BASE: usize = 0x0402_0000;

/// I2C3，Active Domain
pub const I2C3_BASE: usize = 0x0403_0000;

/// I2C4，Active Domain
pub const I2C4_BASE: usize = 0x0404_0000;

/// 板载 GMAC（DTS `ethernet@4070000`，物理基址见本常量）
pub const ETH_BASE: usize = 0x0407_0000;

// =============================================================================
// 0x043x_xxxx — SD/MMC / USB (DWC2)
// =============================================================================

/// SDIO0 控制器
pub const SD_DRIVER_BASE: usize = 0x0431_0000;

/// DWC2 USB OTG 控制器物理基址（DTS `usb@04340000` 第一段 `reg`）
pub const DWC2_BASE: usize = 0x0434_0000;

// =============================================================================
// 0x0502_xxxx — No-die / RTC 域
// =============================================================================

/// RTCSYS_GPIO，No-die Domain
pub const RTCSYS_GPIO_BASE: usize = 0x0502_1000;

/// IOBLK GRTC 组（No-die / RTC 域）
pub const IOBLK_GRTC_BASE: usize = 0x0502_7000;

/// RTCSYS_I2C，No-die Domain
pub const RTCSYS_I2C_BASE: usize = 0x0502_B000;
