//! SG2002 板载以太网（DesignWare MAC 3.70a + 内部 EPHY）驱动。
//!
//! 子模块组织：
//!
//! | 模块                | 职责                                                  |
//! |---------------------|------------------------------------------------------|
//! | [`regs`]            | `tock-registers` GMAC + DMA 寄存器布局 / 位域定义      |
//! | [`desc`]            | DMA 描述符（TDES/RDES）结构体 + 位掩码                  |
//! | [`mdio`]            | MDIO clause 22 PHY 读写                                |
//! | [`nic`]             | [`CvitekEthNic`]：硬件初始化 + Net 驱动 trait 实现     |
//!
//! D-cache 维护（按行 clean / invalidate）由 [`crate::utils::cache`] 提供。
//!
//! 上层（`axdriver`）只需要 `use sg200x_bsp::ethernet::CvitekEthNic;`，
//! 然后 `CvitekEthNic::init(ETH_BASE)` 拿到一个实现了
//! [`axdriver_net::NetDriverOps`] 的网卡实例即可。

pub mod desc;
pub mod mdio;
pub mod nic;
pub mod regs;

pub use nic::{CvitekEthNic, ETH_BASE};
