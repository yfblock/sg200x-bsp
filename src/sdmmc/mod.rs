//! # CV1811 SD 卡驱动库
//!
//! 本库提供了 CV1811 芯片 SD 卡控制器的驱动实现。
//! 支持 SD 卡的初始化、块读取和块写入操作。
//!
//! ## 功能特性
//! - SD 卡检测和初始化
//! - 单块读取 (CMD17)
//! - 多块一次性读取 (CMD18 + AUTO_CMD12)，PIO 与 ADMA2 (32 位 DMA) 两条路径
//! - 单块写入 (CMD24)
//! - 多块一次性写入 (CMD25 + AUTO_CMD12)，PIO 与 ADMA2 (32 位 DMA) 两条路径
//! - 支持 1.8V/3.0V/3.3V 电压切换
//! - 支持 4 位总线宽度
//!
//! ## 模块划分
//! - [`consts`]：寄存器/位域定义、命令与错误类型（`mod consts`，内部）
//! - `adma2`：ADMA2 描述符与常量
//! - `card_info`：SD 卡信息结构与 CSD 解析
//! - `command`：命令下发与完成/传输等待（`Sdmmc` 的 `cmd_transfer` 等）
//! - `read` / `write`：块读写路径（PIO 与 ADMA2 DMA）
//! - `control`：复位、电源、时钟、引脚、卡信息访问
//! - `init`：完整初始化命令序列
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::sdmmc::{Sdmmc, PowerLevel};
//!
//! use sg200x_bsp::soc::{SD_DRIVER_BASE, TOP_BASE};
//!
//! // 创建 SDMMC 驱动实例
//! let mut sdmmc = unsafe { Sdmmc::new(SD_DRIVER_BASE, TOP_BASE) };
//!
//! // 初始化 SD 卡
//! sdmmc.init()?;
//!
//! // 读取单块 (CMD17)
//! let mut buf = [0u8; 512];
//! sdmmc.read_block_single(0, &mut buf)?;
//!
//! // 一次性读取多块 (CMD18)：缓冲区长度须为 512 的整数倍
//! let mut multi = [0u8; 512 * 8];
//! sdmmc.read_blocks(0, &mut multi)?;
//!
//! // 写入块
//! sdmmc.write_block(0, &buf)?;
//! ```

mod adma2;
mod card_info;
mod command;
mod consts;
mod control;
mod init;
mod read;
mod write;

pub use adma2::{
    ADMA2_ATTR_ACT_TRAN, ADMA2_ATTR_END, ADMA2_ATTR_INT, ADMA2_ATTR_VALID, ADMA2_MAX_PER_DESC,
    Adma2Desc,
};
pub use card_info::{SdCardInfo, parse_sd_card_info};
pub use consts::{BLOCK_SIZE, CmdError, CommandType, PowerLevel, ResponseType};
pub use consts::{SD_DRIVER_BASE, SdmmcRegisters, TOP_BASE};

use consts::{PRESENT_STATE, TopRegisters};
use core::cell::Cell;

use tock_registers::interfaces::Readable;

use crate::pinmux::Pinmux;

/// SDMMC 驱动结构体
///
/// 提供对 SD 卡控制器的访问接口
pub struct Sdmmc {
    /// SDMMC 寄存器组引用
    regs: &'static SdmmcRegisters,
    /// TOP 寄存器组引用
    top_regs: &'static TopRegisters,
    /// Pinmux 驱动 (可选)
    pinmux: Option<Pinmux>,
    /// 缓存 [`SdCardInfo`]：在 [`Sdmmc::init`] 中填充
    card_info: Cell<SdCardInfo>,
}

impl Sdmmc {
    /// 创建新的 SDMMC 驱动实例
    ///
    /// # 参数
    ///
    /// - `sd_base`: SD/MMC 控制器 MMIO 基地址（见 [`SD_DRIVER_BASE`]）
    /// - `top_base`: TOP 模块 MMIO 基地址（见 [`TOP_BASE`]）
    ///
    /// Pinmux 默认未附加；初始化前可调用 [`Sdmmc::set_pinmux`] 配置引脚复用。
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new(sd_base: usize, top_base: usize) -> Self {
        unsafe {
            Self {
                regs: &*(sd_base as *const SdmmcRegisters),
                top_regs: &*(top_base as *const TopRegisters),
                pinmux: None,
                card_info: Cell::new(SdCardInfo::default()),
            }
        }
    }

    /// 设置 Pinmux 驱动
    pub fn set_pinmux(&mut self, pinmux: Pinmux) {
        self.pinmux = Some(pinmux);
    }

    /// 检测 SD 卡是否已插入
    pub fn is_card_inserted(&self) -> bool {
        self.regs.present_state.is_set(PRESENT_STATE::CARD_INSERTED)
    }
}

/// 创建 SDMMC 驱动、附加 Pinmux 并完成 SD 卡初始化
///
/// # 参数
///
/// - `sd_base` / `top_base`: SD 控制器与 TOP 模块 MMIO 基地址
/// - `fmux_base` / `ioblk_base` / `ioblk_grtc_base`: Pinmux 所需 FMUX 与 IOBLK 基地址
///
/// # Safety
///
/// 调用者必须确保所有 MMIO 基地址有效且可访问。
pub unsafe fn init(
    sd_base: usize,
    top_base: usize,
    fmux_base: usize,
    ioblk_base: usize,
    ioblk_grtc_base: usize,
) -> Result<Sdmmc, CmdError> {
    let mut sdmmc = unsafe { Sdmmc::new(sd_base, top_base) };
    sdmmc.set_pinmux(unsafe { Pinmux::new(fmux_base, ioblk_base, ioblk_grtc_base) });
    sdmmc.init()?;
    Ok(sdmmc)
}
