//! USB 标准 SETUP 数据包（小端，8 字节）。
//!
//! 返回的数组顺序与总线上 **SETUP PID** 后紧跟的 8 字节一致：`bmRequestType`、`bRequest`、
//! `wValue`、`wIndex`、`wLength`（各字段小端）。
//!
//! 类专用 SETUP：Mass Storage 见 [`crate::usb::class::mass_storage`]，
//! UVC 见 [`crate::usb::class::uvc`]，Hub 见 [`crate::usb::class::hub`]。

/// 构造标准 `GET_DESCRIPTOR(Device)` 的 8 字节 SETUP。
///
/// # 参数
/// - `w_length`：数据阶段主机希望读取的字节数（常见先读 8 再读 18）。
#[inline]
pub fn get_descriptor_device(w_length: u16) -> [u8; 8] {
    [
        0x80, // bmRequestType: Dir IN, Type Standard, Recipient Device
        6,    // GET_DESCRIPTOR
        0x00,
        0x01, // wValue: DEVICE (high) index 0 (low)
        0x00,
        0x00, // wIndex
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// 构造 `SET_ADDRESS` 的 8 字节 SETUP。
///
/// # 参数
/// - `addr`：设备新地址，合法范围 **1..=127**（0 为默认地址）。
#[inline]
pub fn set_address(addr: u8) -> [u8; 8] {
    [
        0x00,
        5, // SET_ADDRESS
        addr,
        0,
        0,
        0,
        0,
        0,
    ]
}

/// 构造 `GET_CONFIGURATION`（数据阶段返回 1 字节 `bConfigurationValue`）。
#[inline]
pub fn get_configuration() -> [u8; 8] {
    [
        0x80,
        8, // GET_CONFIGURATION
        0x00,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
    ]
}

/// 构造 `SET_CONFIGURATION` 的 8 字节 SETUP。
///
/// # 参数
/// - `cfg`：要选中的配置值 `bConfigurationValue`（通常非 0 表示激活该配置）。
#[inline]
pub fn set_configuration(cfg: u8) -> [u8; 8] {
    [
        0x00,
        9, // SET_CONFIGURATION
        cfg,
        0,
        0,
        0,
        0,
        0,
    ]
}

/// `USB_DT_CONFIGURATION`（`GET_DESCRIPTOR` 高字节）。
pub const USB_DT_CONFIGURATION: u8 = 2;

/// 构造 `GET_DESCRIPTOR(Configuration)` — 对**已分配地址**的设备使用。
///
/// # 参数
/// - `cfg_index`：配置描述符索引（通常为 0）。
/// - `w_length`：希望读回的字节数（可先读 9 字节头再按 `wTotalLength` 读全）。
#[inline]
pub fn get_descriptor_configuration(cfg_index: u8, w_length: u16) -> [u8; 8] {
    [
        0x80,
        6, // GET_DESCRIPTOR
        cfg_index,
        USB_DT_CONFIGURATION,
        0x00,
        0x00,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// 构造 `SET_INTERFACE`（选择接口的备用设置，UVC 开流等场景常用）。
///
/// # 参数
/// - `alt`：备用设置编号 `bAlternateSetting`。
/// - `interface`：接口号 `bInterfaceNumber`。
#[inline]
pub fn set_interface(alt: u8, interface: u8) -> [u8; 8] {
    [
        0x01,
        0x0b, // SET_INTERFACE
        alt,
        0,
        interface,
        0,
        0,
        0,
    ]
}
