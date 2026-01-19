//! SG2002 BSP (Board Support Package)
//!
//! 本 crate 提供 SG2002 芯片的底层硬件抽象层驱动程序。
//!
//! # 模块
//!
//! - `pinmux`: 引脚复用控制驱动
//! - `gpio`: GPIO 控制驱动
//! - `sdmmc`: SD/MMC 控制驱动
//! - `tpu`: TPU (张量处理单元) 驱动

#![no_std]
#![recursion_limit = "512"]

pub mod gpio;
pub mod pinmux;
pub mod sdmmc;
pub mod tpu;
