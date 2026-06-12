//! USB Hub 类 SETUP 包与端口状态位域（USB 2.0 hub spec §11.24.2）。

use tock_registers::register_bitfields;

register_bitfields![u16,
    /// Hub `GET_PORT_STATUS` 返回的 `wPortStatus`。
    pub HubPortStatus [
        /// 1=端口有设备连接，RO。
        CONNECTION OFFSET(0) NUMBITS(1) [],
        /// 1=端口已 enable（reset 成功后），RO。
        PORT_ENABLE OFFSET(1) NUMBITS(1) [],
        /// 1=端口正在 reset，RO。
        RESET OFFSET(4) NUMBITS(1) [],
        /// 1=端口已供电（`PORT_POWER` 后），RO。
        POWER OFFSET(5) NUMBITS(1) [],
        /// 1=低速设备，RO。
        LOW_SPEED OFFSET(9) NUMBITS(1) [],
        /// 1=高速设备，RO；与 `LOW_SPEED` 均为 0 表示 FS。
        HIGH_SPEED OFFSET(10) NUMBITS(1) [],
    ],

    /// Hub `GET_PORT_STATUS` 返回的 `wPortChange`（变化位，W1C 清除）。
    pub HubPortChange [
        /// 连接状态自上次读后发生变化。
        C_CONNECTION OFFSET(0) NUMBITS(1) [],
        /// enable 状态发生变化。
        C_ENABLE OFFSET(1) NUMBITS(1) [],
        /// reset 完成。
        C_RESET OFFSET(4) NUMBITS(1) [],
    ],
];

/// Hub `GET_PORT_STATUS` 数据阶段长度：`wPortStatus`(2) + `wPortChange`(2)。
pub const HUB_PORT_STATUS_LEN: u16 = 4;

/// Hub 类描述符类型（`GET_DESCRIPTOR` 高字节）。
pub const USB_DT_HUB: u8 = 0x29;

/// Hub 端口特性：`PORT_CONNECTION`。
pub const HUB_PORT_FEATURE_CONNECTION: u16 = 0;
/// Hub 端口特性：`PORT_ENABLE`。
pub const HUB_PORT_FEATURE_ENABLE: u16 = 1;
/// Hub 端口特性：`PORT_RESET`。
pub const HUB_PORT_FEATURE_RESET: u16 = 4;
/// Hub 端口特性：`PORT_POWER`（hub 上电后端口电源默认关闭，必须先打开）。
pub const HUB_PORT_FEATURE_POWER: u16 = 8;
/// Hub 端口特性：`C_PORT_CONNECTION`（端口连接变化位，CLEAR 用）。
pub const HUB_PORT_FEATURE_C_CONNECTION: u16 = 16;
/// Hub 端口特性：`C_PORT_ENABLE`。
pub const HUB_PORT_FEATURE_C_ENABLE: u16 = 17;
/// Hub 端口特性：`C_PORT_RESET`。
pub const HUB_PORT_FEATURE_C_RESET: u16 = 20;

/// 构造 Hub 端口类 SETUP（`wIndex` = 端口号，从 1 开始）。
#[inline]
fn hub_port_setup(bm_request: u8, b_request: u8, port: u16, w_value: u16, w_length: u16) -> [u8; 8] {
    [
        bm_request,
        b_request,
        w_value as u8,
        (w_value >> 8) as u8,
        port as u8,
        (port >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// 解析 `GET_PORT_STATUS` 4 字节响应为 `(wPortStatus, wPortChange)`。
#[inline]
pub fn parse_hub_port_status(buf: &[u8; 4]) -> (u16, u16) {
    (
        u16::from_le_bytes([buf[0], buf[1]]),
        u16::from_le_bytes([buf[2], buf[3]]),
    )
}

/// 构造 `GET_DESCRIPTOR(HUB)` — Hub **已 SET_CONFIGURATION** 后读取 Hub 描述符。
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

/// Hub：`SET_PORT_FEATURE`（`bmRequestType=0x23`，`bRequest=SET_FEATURE`）。
#[inline]
pub fn hub_set_port_feature(port: u16, feature: u16) -> [u8; 8] {
    hub_port_setup(0x23, 0x03, port, feature, 0)
}

/// Hub：`CLEAR_PORT_FEATURE`（`bmRequestType=0x23`，`bRequest=CLEAR_FEATURE`）。
#[inline]
pub fn hub_clear_port_feature(port: u16, feature: u16) -> [u8; 8] {
    hub_port_setup(0x23, 0x01, port, feature, 0)
}

/// Hub：`GET_PORT_STATUS`（`bmRequestType=0xA3`，数据阶段 [`HUB_PORT_STATUS_LEN`] 字节）。
#[inline]
pub fn hub_get_port_status(port: u16) -> [u8; 8] {
    hub_port_setup(0xA3, 0, port, 0, HUB_PORT_STATUS_LEN)
}
