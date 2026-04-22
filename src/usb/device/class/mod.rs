//! USB Device 模式下的 class 实现集合。
//!
//! 当前提供：
//!
//! - [`cdc_acm`]：CDC-ACM (USB 虚拟串口)，PC 端会出现 `/dev/ttyACM*`。
//!
//! 后续可加 HID / MSC Target / CDC-ECM / UVC 等，沿用 [`super::UsbDeviceClass`] trait。

#[cfg(feature = "device-cdc-acm")]
pub mod cdc_acm;

#[cfg(feature = "device-cdc-acm")]
pub use cdc_acm::CdcAcm;
