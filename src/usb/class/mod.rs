//! USB 设备 **类协议** 层（class drivers）：构建在 [`crate::usb::host::dwc2`] 通道之上。
//!
//! 当前已实现：
//! - [`uvc`]：USB Video Class（PROBE/COMMIT 协商 + Bulk/Isoch 抓帧组装 MJPEG）。
//! - [`uvc_session`]：UVC 会话状态机（错误恢复 + 热拔插），构建在 [`uvc`] 之上。
//! - [`mass_storage`]：USB Mass Storage / Bulk-Only Transport 类协议
//!   （SETUP 包构造、`Mass Storage Reset` / `GET_MAX_LUN`）。

pub mod uvc;
pub mod uvc_session;
pub mod mass_storage;

