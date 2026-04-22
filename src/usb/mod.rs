//! USB 子系统：基于 Synopsys **DWC2** 的 **主机**（[`host`]）栈；启用 feature `device-mode`
//! 时另有 **设备** 子模块 `device`；类协议在 [`class`]。
//!
//! # 使用顺序（主机）
//!
//! 1. [`platform::set_dwc2_base_virt`] 指向控制器 MMIO；若 VA≠PA，再注册 [`platform::set_usb_dma_to_phys_fn`]。
//! 2. 可选：[`log::set_usb_log_fn`] 把栈内日志接到串口等。
//! 3. 调 [`host::enumerate_root_port`] 或自行组合 `host::dwc2` + `host::topology`。
//!
//! # 公共子模块
//!
//! - [`error`]：[`error::UsbError`] / [`error::UsbResult`]。
//! - [`log`]：行缓冲日志（拓扑扫描结束可 [`log::usb_log_flush_residual`]）。
//! - [`platform`]：基址与 EP0 `HCDMA` 用的 VA→PA。
//! - [`setup`]：标准 SETUP 字节数组；**类专用** SETUP 见 [`class::uvc`]、[`class::mass_storage`]。
//!
//! DMA 与 CPU 视图一致性由 [`crate::utils::cache`] 的 clean / invalidate 辅助完成。

pub mod error;
pub mod log;
pub mod platform;
pub mod setup;

pub mod host;
pub mod class;
#[cfg(feature = "device-mode")]
pub mod device;

pub use error::{UsbError, UsbResult};
pub use log::{LineBufferedUsbLog, set_usb_log_fn};
pub use platform::{set_dwc2_base_virt, set_usb_dma_to_phys_fn};
