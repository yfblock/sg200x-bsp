//! USB 标准 SETUP 数据包（小端，8 字节）。
//!
//! 返回的数组顺序与总线上 **SETUP PID** 后紧跟的 8 字节一致：`bmRequestType`、`bRequest`、
//! `wValue`、`wIndex`、`wLength`（各字段小端）。
//!
//! 类专用 SETUP（如 Mass Storage 的 `Bulk-Only Reset` / `GET_MAX_LUN`）见
//! [`crate::usb::class::mass_storage`]；UVC 的 VS 控制见 [`crate::usb::class::uvc`]。

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

/// Hub：`SET_PORT_FEATURE`（`bmRequestType=0x23`，`bRequest=SET_FEATURE`）。
///
/// # 参数
/// - `port`：Hub 下游端口号，**从 1 开始**（与 USB Hub 规范一致）。
/// - `feature`：端口特性选择子，例如 [`HUB_PORT_FEATURE_POWER`]。
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

/// Hub：`CLEAR_PORT_FEATURE`（`bmRequestType=0x23`，`bRequest=CLEAR_FEATURE`）。
/// 用于清除 `C_PORT_CONNECTION` / `C_PORT_RESET` 等变化位。
///
/// # 参数
/// - `port`：Hub 下游端口号（从 1 开始）。
/// - `feature`：要清除的端口特性或 `C_PORT_*` 常量。
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

/// 构造 `GET_DESCRIPTOR(HUB)` — 在 Hub **已 SET_CONFIGURATION** 后向其读取 Hub 描述符。
///
/// # 参数
/// - `w_length`：数据阶段长度（常取 9 或更大以容纳 `bNbrPorts` 等字段）。
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

/// Hub：`GET_PORT_STATUS`（`bmRequestType=0xA3`，数据阶段固定 4 字节 `wPortStatus`/`wPortChange`）。
///
/// # 参数
/// - `port`：Hub 下游端口号（从 1 开始）。
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
