//! DWMAC normal-mode 收发描述符（每条 16 字节）。
//!
//! - **TX 描述符（TDES0..TDES3）**：normal mode 下只有 `OWN` (TDES0) 是
//!   主机可写的状态位；TDES1 同时承载控制位与缓冲长度；TDES2 是数据缓冲
//!   物理地址（32 bit）。
//! - **RX 描述符（RDES0..RDES3）**：DMA 写回 RDES0 的 `OWN`/`ES` 与帧长，
//!   RDES1 配置缓冲长度 + ring/chain 标志，RDES2 是数据缓冲物理地址。
//!
//! 描述符按 64 byte 对齐，避免与相邻数据共享 cache line（C906 维护按行）。
//!
//! 本驱动让 `DMA_BUS_MODE.DSL = 12 words`，即两条相邻描述符之间额外跳 48 字节，
//! 配合 16 字节本身形成 **64 byte stride**，正好等于 C906 的 cache line 长度。

#[repr(C, align(64))]
pub struct DmaDesc {
    pub des0: u32,
    pub des1: u32,
    pub des2: u32,
    pub des3: u32,
}

impl DmaDesc {
    pub const fn zero() -> Self {
        Self { des0: 0, des1: 0, des2: 0, des3: 0 }
    }
}

// ===== TDES0 =====
pub const TDES0_OWN: u32 = 1 << 31;

// ===== TDES1（normal mode：control + buffer-1 size）=====
pub const TDES1_IC:        u32 = 1 << 31; // Interrupt on Completion
pub const TDES1_LS:        u32 = 1 << 30; // Last Segment
pub const TDES1_FS:        u32 = 1 << 29; // First Segment
pub const TDES1_TER:       u32 = 1 << 25; // Transmit End-of-Ring
pub const TDES1_TBS1_MASK: u32 = 0x7FF;   // Buffer 1 size

// ===== RDES0 =====
pub const RDES0_OWN:      u32 = 1 << 31;
pub const RDES0_FL_MASK:  u32 = 0x3FFF << 16; // Frame Length (含 4 字节 FCS)
pub const RDES0_FL_SHIFT: u32 = 16;
pub const RDES0_ES:       u32 = 1 << 15;      // Error Summary

// ===== RDES1 =====
pub const RDES1_RBS1_MASK: u32 = 0x7FF;       // Receive Buffer 1 size
pub const RDES1_RER:       u32 = 1 << 25;     // Receive End-of-Ring

pub const TX_RING_SIZE: usize = 32;
pub const RX_RING_SIZE: usize = 32;
pub const BUF_SIZE:     usize = 2048;
