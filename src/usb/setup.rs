//! USB 标准 SETUP 数据包（小端，8 字节）。
//!
//! Class 专用 SETUP（如 Mass Storage 的 `Bulk-Only Reset` / `GET_MAX_LUN`）已搬到对应
//! `crate::usb::class::*` 模块。

/// `GET_DESCRIPTOR`（Device），`wLength` 为本次希望读回的字节数（常见先读 8）。
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

/// `SET_ADDRESS`（`addr` 1..127）。
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

/// `GET_CONFIGURATION`（返回 1 字节 `bConfigurationValue`）。
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

/// `SET_CONFIGURATION`。
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

/// Hub：`SET_PORT_FEATURE`（`bmRequestType=0x23` class+other，`bRequest=SET_FEATURE`）。
#[inline]
pub fn hub_set_port_feature(port: u16, feature: u16) -> [u8; 8] {
    [
        0x23,
        0x03, // USB_REQ_SET_FEATURE
        feature as u8,
        (feature >> 8) as u8,
        port as u8,
        (port >> 8) as u8,
        0,
        0,
    ]
}

/// Hub：`CLEAR_PORT_FEATURE`（`bmRequestType=0x23` class+other，`bRequest=CLEAR_FEATURE`）。
/// 用来清掉 `C_PORT_CONNECTION` / `C_PORT_RESET` 等"变化位"。
#[inline]
pub fn hub_clear_port_feature(port: u16, feature: u16) -> [u8; 8] {
    [
        0x23,
        0x01, // USB_REQ_CLEAR_FEATURE
        feature as u8,
        (feature >> 8) as u8,
        port as u8,
        (port >> 8) as u8,
        0,
        0,
    ]
}

/// Hub 端口特性：`PORT_CONNECTION`（USB 2.0 hub spec §11.24.2）。
pub const HUB_PORT_FEATURE_CONNECTION: u16 = 0;
/// Hub 端口特性：`PORT_ENABLE`。
pub const HUB_PORT_FEATURE_ENABLE: u16 = 1;
/// Hub 端口特性：`PORT_RESET`。
pub const HUB_PORT_FEATURE_RESET: u16 = 4;
/// Hub 端口特性：`PORT_POWER`（hub 上电后端口电源默认关闭，必须先打开）。
pub const HUB_PORT_FEATURE_POWER: u16 = 8;
/// Hub 端口特性：`C_PORT_CONNECTION`（端口"连接发生变化"位，CLEAR 用）。
pub const HUB_PORT_FEATURE_C_CONNECTION: u16 = 16;
/// Hub 端口特性：`C_PORT_ENABLE`。
pub const HUB_PORT_FEATURE_C_ENABLE: u16 = 17;
/// Hub 端口特性：`C_PORT_RESET`。
pub const HUB_PORT_FEATURE_C_RESET: u16 = 20;

/// `USB_DT_CONFIGURATION`（`GET_DESCRIPTOR` 高字节）。
pub const USB_DT_CONFIGURATION: u8 = 2;
/// Hub 类描述符类型（`GET_DESCRIPTOR` 高字节）。
pub const USB_DT_HUB: u8 = 0x29;

/// `GET_DESCRIPTOR(CONFIGURATION, index, wLength)` — 已寻址设备。
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

/// `GET_DESCRIPTOR(HUB)` — Hub 已配置后由 Hub 设备返回（`bmRequestType` Device+Class+IN）。
#[inline]
pub fn get_descriptor_hub(w_length: u16) -> [u8; 8] {
    [
        0xA0,
        6,
        0x00,
        USB_DT_HUB,
        0x00,
        0x00,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// Hub：`GET_PORT_STATUS`（`bmRequestType=0xA3` Class+IN+Other，`bRequest=GET_STATUS`）。
#[inline]
pub fn hub_get_port_status(port: u16) -> [u8; 8] {
    [
        0xA3,
        0, // GET_STATUS
        0,
        0,
        port as u8,
        (port >> 8) as u8,
        4,
        0,
    ]
}

/// `SET_INTERFACE`（选择备用设置，UVC 开流常用）。
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
