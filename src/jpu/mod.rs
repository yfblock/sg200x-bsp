//! SG2002 JPU（JPEG Processing Unit）纯 Rust 驱动。
//!
//! 对照 U-Boot CVitek 驱动实现（`drivers/jpeg/`），在裸机上以轮询方式完成
//! JPEG 硬件解码，输出带显式 plane/stride 布局的 planar YUV 或灰度帧。

mod decoder;
mod engine;
mod header;
mod layout;
mod mem;
pub mod regs;

pub use decoder::{DecodeResult, JpuDecodeError, JpuDecoder, JpuDmaToPhysFn, JpuMmio};
pub use layout::{Extent, FrameLayout, FrameLayoutError, JpuPixelFormat, JpuScale, PlaneLayout};
