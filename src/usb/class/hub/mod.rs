//! USB Hub 类协议：Hub 描述符、端口电源/reset、`GET_PORT_STATUS` 与下游端口递归枚举。
//!
//! 拓扑扫描在 [`crate::usb::host::topology`] 中识别 Hub 后，调用
//! [`enumerate_downstream_ports`] 遍历各下游端口；仅 **HS** 子设备会继续标准枚举。

mod device;
mod enumerate;
mod setup;

macro_rules! hub_log {
    ($depth:expr, $($tt:tt)*) => {
        ::log::info!(
            target: "sg200x_bsp::usb::hub",
            "{}{}",
            $crate::utils::log_indent($depth),
            format_args!($($tt)*)
        )
    };
}
pub(crate) use hub_log;

pub use device::{HubDescriptor, HubDevice, port_connected, port_enabled, port_speed_str};
pub use enumerate::enumerate_downstream_ports;
pub use setup::{
    parse_hub_port_status, HubPortChange, HubPortStatus, HUB_PORT_FEATURE_C_CONNECTION,
    HUB_PORT_FEATURE_C_ENABLE, HUB_PORT_FEATURE_C_RESET, HUB_PORT_FEATURE_CONNECTION,
    HUB_PORT_FEATURE_ENABLE, HUB_PORT_FEATURE_POWER, HUB_PORT_FEATURE_RESET,
    HUB_PORT_STATUS_LEN, USB_DT_HUB,
};

use crate::usb::UsbClass;

/// QEMU 默认 `usb-hub`（插在根口与首个外设之间）VID/PID。
const QEMU_USB_HUB_VID: u16 = 0x0409;
const QEMU_USB_HUB_PID: u16 = 0x55aa;

/// 根据设备描述符判断当前 @0 设备是否为 Hub。
#[inline]
pub fn is_hub_device(class: UsbClass, vid: u16, pid: u16) -> bool {
    class == UsbClass::Hub || (vid == QEMU_USB_HUB_VID && pid == QEMU_USB_HUB_PID)
}
