//! USB 栈统一错误类型（host 与 class 通用）。

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbError {
    /// 预留：某条代码路径尚未实现。
    NotImplemented,
    /// 控制器或通道在约定轮询次数内未就绪（含软复位、FIFO flush、传输完成等）。
    Timeout,
    /// 板级/硬件前提不满足（如未设置 MMIO 基址、DMA AHB 错误）。
    Hardware(&'static str),
    /// 协议或参数不合法（描述符解析失败、CSW 不匹配等）。
    Protocol(&'static str),
    /// 设备以 STALL 结束控制/批量事务。
    Stall,
    /// IN 传输设备返回 NAK（常见于视频端点尚无数据），上层应重试。
    Nak,
}

pub type UsbResult<T> = Result<T, UsbError>;
