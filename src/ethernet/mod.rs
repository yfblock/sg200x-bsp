//! SG2002 板载以太网（DesignWare MAC 3.70a + 内部 EPHY）驱动 —— **纯硬件层**。
//!
//! 子模块组织：
//!
//! | 模块                | 职责                                                  |
//! |---------------------|------------------------------------------------------|
//! | [`regs`]            | `tock-registers` GMAC + DMA 寄存器布局 / 位域定义      |
//! | [`desc`]            | DMA 描述符（TDES/RDES）结构体 + 位掩码                  |
//! | [`mdio`]            | MDIO clause 22 PHY 读写                                |
//! | [`nic`]             | [`CvitekEthNic`]：硬件初始化 + 中性 TX/RX API          |
//!
//! D-cache 维护（按行 clean / invalidate）由 [`crate::utils::cache`] 提供。
//!
//! 本模块**不**依赖 ArceOS 的 `axdriver_*`/`NetBufPool` 等抽象，仅暴露中性的
//! [`CvitekEthNic::transmit`] / [`CvitekEthNic::receive`] 等接口。OS 适配层（例如
//! `sg2002-arceos/modules/axdriver/src/cvitek_eth.rs`）应在 BSP 之上自行包一个 wrapper
//! struct 去 `impl axdriver_net::NetDriverOps for CvitekEthDevice`。

pub mod desc;
pub mod mdio;
pub mod nic;
pub mod regs;

pub use nic::{CvitekEthNic, ETH_BASE, EthError, EthResult, RxToken};
