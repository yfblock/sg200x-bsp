//! USB 栈统一错误类型（host 与 class 通用）。

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbError {
    NotImplemented,
    Timeout,
    Hardware(&'static str),
    Protocol(&'static str),
    Stall,
    /// IN 传输设备返回 NAK（常见于视频端点尚无数据），上层应重试。
    Nak,
}

pub type UsbResult<T> = Result<T, UsbError>;
