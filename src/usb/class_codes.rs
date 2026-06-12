//! USB 标准类码：[`UsbClass`] 对应 `bDeviceClass` / `bInterfaceClass`（USB-IF base class）。

/// USB 设备或接口的 **base class**（`bDeviceClass` / `bInterfaceClass`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbClass {
    /// `0x00`：类在接口描述符中定义（复合设备常见）。
    DefinedAtInterface,
    /// `0x08`：Mass Storage。
    MassStorage,
    /// `0x09`：Hub。
    Hub,
    /// `0x0e`：Video（UVC 等）。
    Video,
    /// 规范未在本枚举中列出的类码。
    Unknown(u8),
}

impl UsbClass {
    /// 从描述符原始字节解析类码。
    #[inline]
    pub const fn from_raw(b: u8) -> Self {
        match b {
            0x00 => Self::DefinedAtInterface,
            0x08 => Self::MassStorage,
            0x09 => Self::Hub,
            0x0e => Self::Video,
            x => Self::Unknown(x),
        }
    }

    /// 写回描述符或日志用的原始类码字节。
    #[inline]
    pub const fn as_raw(self) -> u8 {
        match self {
            Self::DefinedAtInterface => 0x00,
            Self::MassStorage => 0x08,
            Self::Hub => 0x09,
            Self::Video => 0x0e,
            Self::Unknown(x) => x,
        }
    }
}
