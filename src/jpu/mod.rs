//! SG2002 JPU（JPEG Processing Unit）纯 Rust 驱动。
//!
//! 对照 U-Boot CVitek 驱动实现（`drivers/jpeg/`），在裸机上以轮询方式完成
//! Baseline JPEG 硬件解码，输出 YUV420 planar。

mod decoder;
mod header;
mod mem;
pub mod regs;

pub use decoder::{DecodeResult, JpuDecoder, JpuDmaToPhysFn, JpuMmio};
