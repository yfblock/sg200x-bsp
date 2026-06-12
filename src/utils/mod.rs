//! SG2002 BSP 内部通用工具集合：与具体外设无关的底层 helper。
//!
//! 子模块：
//!
//! - [`cache`]：CPU D-cache 与 DMA 一致性维护（T-Head C906 / AArch64）。
//! - [`delay`]：忙等延时（NOP / `spin_loop` 自旋）。
//!
//! 外部使用者一般通过 `crate::utils::cache::*` 或 [`crate::utils::delay`] 直接拿到所需 API。

pub mod cache;
pub mod delay;

pub use delay::{delay, delay_long, delay_short, spin_delay};
