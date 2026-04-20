//! USB 设备 **类协议** 层（class drivers）：构建在 [`crate::usb::host::dwc2`] 通道之上。
//!
//! 当前已实现：
//! - [`uvc`]：USB Video Class（PROBE/COMMIT 协商 + Bulk/Isoch 抓帧组装 MJPEG）。
//! - [`mass_storage`]：USB Mass Storage / Bulk-Only Transport 类协议
//!   （SETUP 包构造、`Mass Storage Reset` / `GET_MAX_LUN`）。

pub mod uvc;
pub mod mass_storage;

