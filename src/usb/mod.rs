//! USB 子系统：DWC2 主机控制器（[`host`]）+ 类协议（[`class`]）。
//!
//! 公共支持：
//! - [`error`]：统一 [`error::UsbError`] / [`error::UsbResult`]。
//! - [`log`]：行缓冲日志，由板级注册回调到 `println!`。
//! - [`platform`]：DWC2 寄存器虚拟基址 + DMA VA→PA 转换回调。
//! - [`setup`]：USB 标准 SETUP 包构造器（class 专用包在 [`class::*`] 中）。
//!
//! DMA 缓存一致性（粗粒度 clean / invalidate）由 [`crate::utils::cache`] 提供。

pub mod error;
pub mod log;
pub mod platform;
pub mod setup;

pub mod host;
pub mod class;

pub use error::{UsbError, UsbResult};
pub use log::{LineBufferedUsbLog, set_usb_log_fn};
pub use platform::{set_dwc2_base_virt, set_usb_dma_to_phys_fn};
