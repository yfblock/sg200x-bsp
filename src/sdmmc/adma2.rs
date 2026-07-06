//! ADMA2（32 位）描述符定义与相关常量。
//!
//! ADMA2 是 SDHCI v3.00 定义的分散/聚集 DMA 机制：主机在内存中构建一张
//! 描述符表，控制器据此把数据在卡与内存之间搬运，无需 CPU 逐字参与。

/// ADMA2 描述符属性位：有效 (Valid)。
pub const ADMA2_ATTR_VALID: u16 = 1 << 0;
/// ADMA2 描述符属性位：末尾 (End)，标记描述符表结束。
pub const ADMA2_ATTR_END: u16 = 1 << 1;
/// ADMA2 描述符属性位：中断 (Int)，传输到该描述符时触发中断。
pub const ADMA2_ATTR_INT: u16 = 1 << 2;
/// ADMA2 描述符动作 `Tran`（数据搬运，act = 0b10）。
pub const ADMA2_ATTR_ACT_TRAN: u16 = 0b10 << 4;

/// 单个 ADMA2 描述符可搬运的最大字节数。
///
/// `length` 字段为 16 位，`0` 在协议里被解释为 64 KiB；此处保守取
/// `64 KiB - 8`（且为 8 的倍数），以兼容部分拒绝 `length == 0` 的控制器。
pub const ADMA2_MAX_PER_DESC: usize = 65_528;

/// 32 位 ADMA2 描述符 (SDHCI v3.00 §1.13)。
///
/// 小端布局：
/// ```text
///   偏移 0  attr  [15:0]  (Valid | End | Int | Act)
///   偏移 2  len   [15:0]  (0 表示 64 KiB)
///   偏移 4  addr  [31:0]  数据缓冲区物理地址
/// ```
///
/// 描述符表必须放在 DMA 可见（物理连续）且按 8 字节对齐的内存中。
#[repr(C, align(8))]
#[derive(Clone, Copy, Default)]
pub struct Adma2Desc {
    /// 属性位组合 (Valid/End/Int/Act)。
    pub attr: u16,
    /// 本段长度（字节），0 表示 64 KiB。
    pub len: u16,
    /// 数据缓冲区物理地址 (32 位)。
    pub addr: u32,
}
