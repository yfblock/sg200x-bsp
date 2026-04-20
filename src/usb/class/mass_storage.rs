//! USB Mass Storage Class（MSC，Bulk-Only Transport / BBB）。
//!
//! 当前仅提供 **协议层 SETUP 包构造** 与 **EP0 控制传输辅助**：
//! - [`bulk_only_reset`]：BOT class-specific request `Mass Storage Reset`（`bRequest = 0xFF`）。
//! - [`get_max_lun`]：BOT class-specific request `Get Max LUN`（`bRequest = 0xFE`）。
//!
//! 真正的 **CBW/CSW + Bulk IN/OUT 数据阶段** 由调用方使用
//! [`crate::usb::host::dwc2::ep0::bulk_in`] / [`bulk_out`](crate::usb::host::dwc2::ep0::bulk_out)
//! 自行组装；本模块不做 SCSI 逻辑封装。
//!
//! 拓扑扫描期间，[`crate::usb::host::topology`] 仅记录 MSC 设备的 `(VID, PID, EP0 MPS, addr)` 四元组，
//! 并 **不** 自动发起 BOT reset；caller 在拿到设备后请显式调用 [`bulk_only_reset`]。

use crate::usb::error::UsbResult;
use crate::usb::host::dwc2::ep0;

/// USB MSC `Bulk-Only Mass Storage Reset`（`bmRequestType=0x21`，`bRequest=0xFF`）。
#[inline]
pub fn mass_storage_reset_setup(interface: u16) -> [u8; 8] {
    [
        0x21,
        0xFF,
        0x00,
        0x00,
        interface as u8,
        (interface >> 8) as u8,
        0x00,
        0x00,
    ]
}

/// `GET_MAX_LUN`（`bmRequestType=0xA1`，`bRequest=0xFE`，`wLength=1`）。
#[inline]
pub fn get_max_lun_setup(interface: u16) -> [u8; 8] {
    [
        0xA1,
        0xFE,
        0x00,
        0x00,
        interface as u8,
        (interface >> 8) as u8,
        0x01,
        0x00,
    ]
}

/// 对已寻址 MSC 设备发出 `Bulk-Only Mass Storage Reset`（无数据阶段）。
pub fn bulk_only_reset(dev: u32, interface: u16, ep0_mps: u32) -> UsbResult<()> {
    ep0::ep0_control_write_no_data(dev, mass_storage_reset_setup(interface), ep0_mps)
}

/// 对已寻址 MSC 设备读取 `GET_MAX_LUN`，返回 `bMaxLun`（多 LUN 设备使用）。
pub fn get_max_lun(dev: u32, interface: u16, ep0_mps: u32) -> UsbResult<u8> {
    ep0::ep0_control_read_one_byte(dev, get_max_lun_setup(interface), ep0_mps)
}
