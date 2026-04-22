//! USB 描述符常量 + 字符串描述符序列化 helper。
//!
//! 完整的 device / config / interface / endpoint descriptor 由具体 class
//! （如 [`super::class::cdc_acm`]）按 `&'static [u8]` 形式自行拼好返回；本文件
//! 仅提供常量与 string descriptor 编码工具。

#![allow(dead_code)]

/// USB 标准请求 `bRequest` 编码（USB 2.0 §9.4）。
pub const REQ_GET_STATUS: u8 = 0;
pub const REQ_CLEAR_FEATURE: u8 = 1;
pub const REQ_SET_FEATURE: u8 = 3;
pub const REQ_SET_ADDRESS: u8 = 5;
pub const REQ_GET_DESCRIPTOR: u8 = 6;
pub const REQ_SET_DESCRIPTOR: u8 = 7;
pub const REQ_GET_CONFIGURATION: u8 = 8;
pub const REQ_SET_CONFIGURATION: u8 = 9;
pub const REQ_GET_INTERFACE: u8 = 10;
pub const REQ_SET_INTERFACE: u8 = 11;
pub const REQ_SYNCH_FRAME: u8 = 12;

/// `bDescriptorType`（USB 2.0 §9.4）。
pub const DT_DEVICE: u8 = 1;
pub const DT_CONFIG: u8 = 2;
pub const DT_STRING: u8 = 3;
pub const DT_INTERFACE: u8 = 4;
pub const DT_ENDPOINT: u8 = 5;
pub const DT_DEVICE_QUALIFIER: u8 = 6;
pub const DT_OTHER_SPEED_CONFIG: u8 = 7;
pub const DT_INTERFACE_POWER: u8 = 8;
pub const DT_INTERFACE_ASSOC: u8 = 11;
/// CDC class 描述符类型（CDC 1.1 §5.2）。
pub const DT_CS_INTERFACE: u8 = 0x24;
pub const DT_CS_ENDPOINT: u8 = 0x25;

/// `bmRequestType` 高 3 位（Type）。
pub const REQ_TYPE_STANDARD: u8 = 0 << 5;
pub const REQ_TYPE_CLASS: u8 = 1 << 5;
pub const REQ_TYPE_VENDOR: u8 = 2 << 5;

/// `bmRequestType` 低 5 位（Recipient）。
pub const REQ_RCPT_DEVICE: u8 = 0;
pub const REQ_RCPT_INTERFACE: u8 = 1;
pub const REQ_RCPT_ENDPOINT: u8 = 2;
pub const REQ_RCPT_OTHER: u8 = 3;

/// 把 ASCII 串编码为 USB string descriptor（UTF-16LE，前缀 `bLength + DT_STRING`）。
///
/// 写入 `dst` 头部的字节数从返回值取（≤ `dst.len()`）。
///
/// # 参数
/// - `s`：要编码的 UTF-8 文本（仅 BMP 平面字符，超长则截断）。
/// - `dst`：输出缓冲区；前 2 字节写入后会被设为 `bLength` 与 `DT_STRING`。
///
/// # 返回值
/// 实际写入的字节数（含 2 字节头）。
pub fn encode_string_descriptor(s: &str, dst: &mut [u8]) -> usize {
    let mut n = 2usize;
    for ch in s.chars() {
        // 仅对 BMP 字符做简化处理（surrogate pair 这里不会出现，CDC 串够用）
        if n + 2 > dst.len() {
            break;
        }
        let code = ch as u32;
        let lo = (code & 0xff) as u8;
        let hi = ((code >> 8) & 0xff) as u8;
        dst[n] = lo;
        dst[n + 1] = hi;
        n += 2;
    }
    if dst.len() >= 2 {
        dst[0] = n as u8;
        dst[1] = DT_STRING;
    }
    n
}

/// `LANGID = English (US) 0x0409` 的 string descriptor 0。
pub const STRING0_EN_US: [u8; 4] = [4, DT_STRING, 0x09, 0x04];

/// 组合 USB 端点地址字节（设备描述符 / 端点描述符中的 `bEndpointAddress`）。
///
/// # 参数
/// - `num`：端点号，仅低 4 位有效（1..=15 等，视控制器而定）。
/// - `dir_in`：`true` 表示 IN 端点（主机读设备），地址 bit7 置 1；`false` 为 OUT。
#[inline]
pub const fn ep_addr(num: u8, dir_in: bool) -> u8 {
    (num & 0x0f) | if dir_in { 0x80 } else { 0 }
}

/// 端点属性 `bmAttributes`：bit0..1 = transfer type。
pub const EP_ATTR_CONTROL: u8 = 0;
pub const EP_ATTR_ISOCHRONOUS: u8 = 1;
pub const EP_ATTR_BULK: u8 = 2;
pub const EP_ATTR_INTERRUPT: u8 = 3;
