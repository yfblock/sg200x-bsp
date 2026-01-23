//! SG2002 Pinmux 驱动模块
//!
//! 本模块提供 SG2002 芯片引脚复用(Pinmux)功能的驱动程序。
//!
//! # 功能概述
//!
//! SG2002 芯片的引脚复用系统包含两类寄存器:
//!
//! 1. **FMUX (Function Mux) 寄存器**: 用于选择引脚的功能模式
//!    - 基地址: 0x0300_1000
//!    - 每个引脚可以选择多种功能(如 GPIO、UART、SPI、I2C 等)
//!
//! 2. **IOBLK (IO Block) 寄存器**: 用于配置引脚的电气特性
//!    - 多个 IO 组(G1, G7, G10, G12, GRTC)
//!    - 配置上拉/下拉、驱动强度、施密特触发器等
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::pinmux::{Pinmux, PinFunction, PullConfig, DriveStrength};
//!
//! let pinmux = unsafe { Pinmux::new() };
//!
//! // 将 SD0_CLK 引脚配置为 GPIO 功能
//! pinmux.set_function(PinFunction::Sd0Clk, 3); // 3 = XGPIOA[7]
//!
//! // 配置引脚的上拉电阻
//! pinmux.set_pull(PinFunction::Sd0Clk, PullConfig::PullUp);
//!
//! // 设置驱动强度
//! pinmux.set_drive_strength(PinFunction::Sd0Clk, DriveStrength::Level2);
//! ```

mod fmux;
mod ioblk;

pub use fmux::*;
pub use ioblk::*;

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

/// FMUX 寄存器基地址
pub const FMUX_BASE: usize = 0x0300_1000;

/// IOBLK G1 组寄存器基地址
pub const IOBLK_G1_BASE: usize = 0x0300_1800;

/// IOBLK G7 组寄存器基地址
pub const IOBLK_G7_BASE: usize = 0x0300_1900;

/// IOBLK G10 组寄存器基地址
pub const IOBLK_G10_BASE: usize = 0x0300_1A00;

/// IOBLK G12 组寄存器基地址
pub const IOBLK_G12_BASE: usize = 0x0300_1C00;

/// IOBLK GRTC 组寄存器基地址
pub const IOBLK_GRTC_BASE: usize = 0x0502_7000;

/// 上拉/下拉配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullConfig {
    /// 无上拉/下拉
    None,
    /// 上拉使能
    PullUp,
    /// 下拉使能
    PullDown,
}

/// 驱动强度等级
///
/// 驱动强度由 DS0, DS1 (部分引脚还有 DS2) 位控制
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStrength {
    /// 等级 0 (最弱)
    Level0 = 0,
    /// 等级 1
    Level1 = 1,
    /// 等级 2 (默认)
    Level2 = 2,
    /// 等级 3
    Level3 = 3,
    /// 等级 4 (仅部分引脚支持)
    Level4 = 4,
    /// 等级 5 (仅部分引脚支持)
    Level5 = 5,
    /// 等级 6 (仅部分引脚支持)
    Level6 = 6,
    /// 等级 7 (最强, 仅部分引脚支持)
    Level7 = 7,
}

/// 施密特触发器等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchmittTriggerLevel {
    /// 等级 0 (禁用)
    Level0 = 0,
    /// 等级 1
    Level1 = 1,
    /// 等级 2
    Level2 = 2,
    /// 等级 3
    Level3 = 3,
}

/// Pinmux 驱动结构体
///
/// 提供对 SG2002 引脚复用系统的访问接口
pub struct Pinmux {
    /// FMUX 寄存器组
    pub fmux: &'static FmuxRegisters,
    /// IOBLK G1 组寄存器
    ioblk_g1: &'static IoblkG1Registers,
    /// IOBLK G7 组寄存器
    ioblk_g7: &'static IoblkG7Registers,
    /// IOBLK G10 组寄存器
    ioblk_g10: &'static IoblkG10Registers,
    /// IOBLK G12 组寄存器
    ioblk_g12: &'static IoblkG12Registers,
    /// IOBLK GRTC 组寄存器
    ioblk_grtc: &'static IoblkGrtcRegisters,
}

impl Pinmux {
    /// 创建新的 Pinmux 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new() -> Self {
        unsafe {
            Self {
                fmux: &*(FMUX_BASE as *const FmuxRegisters),
                ioblk_g1: &*(IOBLK_G1_BASE as *const IoblkG1Registers),
                ioblk_g7: &*(IOBLK_G7_BASE as *const IoblkG7Registers),
                ioblk_g10: &*(IOBLK_G10_BASE as *const IoblkG10Registers),
                ioblk_g12: &*(IOBLK_G12_BASE as *const IoblkG12Registers),
                ioblk_grtc: &*(IOBLK_GRTC_BASE as *const IoblkGrtcRegisters),
            }
        }
    }

    /// 从指定基地址创建 Pinmux 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保所有基地址有效且可访问
    pub unsafe fn from_base_addresses(
        fmux_base: usize,
        ioblk_g1_base: usize,
        ioblk_g7_base: usize,
        ioblk_g10_base: usize,
        ioblk_g12_base: usize,
        ioblk_grtc_base: usize,
    ) -> Self {
        unsafe {
            Self {
                fmux: &*(fmux_base as *const FmuxRegisters),
                ioblk_g1: &*(ioblk_g1_base as *const IoblkG1Registers),
                ioblk_g7: &*(ioblk_g7_base as *const IoblkG7Registers),
                ioblk_g10: &*(ioblk_g10_base as *const IoblkG10Registers),
                ioblk_g12: &*(ioblk_g12_base as *const IoblkG12Registers),
                ioblk_grtc: &*(ioblk_grtc_base as *const IoblkGrtcRegisters),
            }
        }
    }

    /// 获取 FMUX 寄存器组的引用
    pub fn fmux(&self) -> &FmuxRegisters {
        self.fmux
    }

    /// 获取 IOBLK G1 组寄存器的引用
    pub fn ioblk_g1(&self) -> &IoblkG1Registers {
        self.ioblk_g1
    }

    /// 获取 IOBLK G7 组寄存器的引用
    pub fn ioblk_g7(&self) -> &IoblkG7Registers {
        self.ioblk_g7
    }

    /// 获取 IOBLK G10 组寄存器的引用
    pub fn ioblk_g10(&self) -> &IoblkG10Registers {
        self.ioblk_g10
    }

    /// 获取 IOBLK G12 组寄存器的引用
    pub fn ioblk_g12(&self) -> &IoblkG12Registers {
        self.ioblk_g12
    }

    /// 获取 IOBLK GRTC 组寄存器的引用
    pub fn ioblk_grtc(&self) -> &IoblkGrtcRegisters {
        self.ioblk_grtc
    }
}

// ============================================================================
// SD0 引脚配置便捷方法
// ============================================================================

impl Pinmux {
    /// 设置 SD0_CLK 引脚功能 (类型安全版本)
    ///
    /// 使用枚举值设置功能，例如:
    /// ```ignore
    /// pinmux.set_sd0_clk_func(FMUX_SD0_CLK::FSEL::XGPIOA_7);
    /// ```
    pub fn set_sd0_clk_func(&self, func: FMUX_SD0_CLK::FSEL::Value) {
        self.fmux.sd0_clk.write(FMUX_SD0_CLK::FSEL.val(func as u32));
    }

    /// 获取 SD0_CLK 引脚当前功能
    pub fn get_sd0_clk_func(&self) -> u32 {
        self.fmux.sd0_clk.read(FMUX_SD0_CLK::FSEL)
    }

    /// 设置 SD0_CMD 引脚功能 (类型安全版本)
    pub fn set_sd0_cmd_func(&self, func: FMUX_SD0_CMD::FSEL::Value) {
        self.fmux.sd0_cmd.write(FMUX_SD0_CMD::FSEL.val(func as u32));
    }

    /// 获取 SD0_CMD 引脚当前功能
    pub fn get_sd0_cmd_func(&self) -> u32 {
        self.fmux.sd0_cmd.read(FMUX_SD0_CMD::FSEL)
    }

    /// 设置 SD0_D0 引脚功能 (类型安全版本)
    pub fn set_sd0_d0_func(&self, func: FMUX_SD0_D0::FSEL::Value) {
        self.fmux.sd0_d0.write(FMUX_SD0_D0::FSEL.val(func as u32));
    }

    /// 获取 SD0_D0 引脚当前功能
    pub fn get_sd0_d0_func(&self) -> u32 {
        self.fmux.sd0_d0.read(FMUX_SD0_D0::FSEL)
    }

    /// 设置 SD0_D1 引脚功能 (类型安全版本)
    pub fn set_sd0_d1_func(&self, func: FMUX_SD0_D1::FSEL::Value) {
        self.fmux.sd0_d1.write(FMUX_SD0_D1::FSEL.val(func as u32));
    }

    /// 获取 SD0_D1 引脚当前功能
    pub fn get_sd0_d1_func(&self) -> u32 {
        self.fmux.sd0_d1.read(FMUX_SD0_D1::FSEL)
    }

    /// 设置 SD0_D2 引脚功能 (类型安全版本)
    pub fn set_sd0_d2_func(&self, func: FMUX_SD0_D2::FSEL::Value) {
        self.fmux.sd0_d2.write(FMUX_SD0_D2::FSEL.val(func as u32));
    }

    /// 获取 SD0_D2 引脚当前功能
    pub fn get_sd0_d2_func(&self) -> u32 {
        self.fmux.sd0_d2.read(FMUX_SD0_D2::FSEL)
    }

    /// 设置 SD0_D3 引脚功能 (类型安全版本)
    pub fn set_sd0_d3_func(&self, func: FMUX_SD0_D3::FSEL::Value) {
        self.fmux.sd0_d3.write(FMUX_SD0_D3::FSEL.val(func as u32));
    }

    /// 获取 SD0_D3 引脚当前功能
    pub fn get_sd0_d3_func(&self) -> u32 {
        self.fmux.sd0_d3.read(FMUX_SD0_D3::FSEL)
    }

    pub fn get_aux0_input(&self) -> u32 {
        self.fmux.aux0.read(FMUX_AUX0::FSEL)
    }
}

// ============================================================================
// UART0 引脚配置便捷方法
// ============================================================================

impl Pinmux {
    /// 设置 UART0_TX 引脚功能 (类型安全版本)
    pub fn set_uart0_tx_func(&self, func: FMUX_UART0_TX::FSEL::Value) {
        self.fmux.uart0_tx.write(FMUX_UART0_TX::FSEL.val(func as u32));
    }

    /// 获取 UART0_TX 引脚当前功能
    pub fn get_uart0_tx_func(&self) -> u32 {
        self.fmux.uart0_tx.read(FMUX_UART0_TX::FSEL)
    }

    /// 设置 UART0_RX 引脚功能 (类型安全版本)
    pub fn set_uart0_rx_func(&self, func: FMUX_UART0_RX::FSEL::Value) {
        self.fmux.uart0_rx.write(FMUX_UART0_RX::FSEL.val(func as u32));
    }

    /// 获取 UART0_RX 引脚当前功能
    pub fn get_uart0_rx_func(&self) -> u32 {
        self.fmux.uart0_rx.read(FMUX_UART0_RX::FSEL)
    }

    /// 配置 UART0_TX 引脚的上拉/下拉
    pub fn set_uart0_tx_pull(&self, pull: PullConfig) {
        match pull {
            PullConfig::None => {
                self.ioblk_g7.uart0_tx.modify(IOBLK_REG::PU::CLEAR + IOBLK_REG::PD::CLEAR);
            }
            PullConfig::PullUp => {
                self.ioblk_g7.uart0_tx.modify(IOBLK_REG::PU::SET + IOBLK_REG::PD::CLEAR);
            }
            PullConfig::PullDown => {
                self.ioblk_g7.uart0_tx.modify(IOBLK_REG::PU::CLEAR + IOBLK_REG::PD::SET);
            }
        }
    }

    /// 配置 UART0_RX 引脚的上拉/下拉
    pub fn set_uart0_rx_pull(&self, pull: PullConfig) {
        match pull {
            PullConfig::None => {
                self.ioblk_g7.uart0_rx.modify(IOBLK_REG::PU::CLEAR + IOBLK_REG::PD::CLEAR);
            }
            PullConfig::PullUp => {
                self.ioblk_g7.uart0_rx.modify(IOBLK_REG::PU::SET + IOBLK_REG::PD::CLEAR);
            }
            PullConfig::PullDown => {
                self.ioblk_g7.uart0_rx.modify(IOBLK_REG::PU::CLEAR + IOBLK_REG::PD::SET);
            }
        }
    }
}

// ============================================================================
// I2C0 引脚配置便捷方法
// ============================================================================

impl Pinmux {
    /// 设置 IIC0_SCL 引脚功能 (类型安全版本)
    pub fn set_iic0_scl_func(&self, func: FMUX_IIC0_SCL::FSEL::Value) {
        self.fmux.iic0_scl.write(FMUX_IIC0_SCL::FSEL.val(func as u32));
    }

    /// 获取 IIC0_SCL 引脚当前功能
    pub fn get_iic0_scl_func(&self) -> u32 {
        self.fmux.iic0_scl.read(FMUX_IIC0_SCL::FSEL)
    }

    /// 设置 IIC0_SDA 引脚功能 (类型安全版本)
    pub fn set_iic0_sda_func(&self, func: FMUX_IIC0_SDA::FSEL::Value) {
        self.fmux.iic0_sda.write(FMUX_IIC0_SDA::FSEL.val(func as u32));
    }

    /// 获取 IIC0_SDA 引脚当前功能
    pub fn get_iic0_sda_func(&self) -> u32 {
        self.fmux.iic0_sda.read(FMUX_IIC0_SDA::FSEL)
    }
}

// ============================================================================
// PWR GPIO 引脚配置便捷方法
// ============================================================================

impl Pinmux {
    /// 设置 PWR_GPIO0 引脚功能 (类型安全版本)
    pub fn set_pwr_gpio0_func(&self, func: FMUX_PWR_GPIO0::FSEL::Value) {
        self.fmux.pwr_gpio0.write(FMUX_PWR_GPIO0::FSEL.val(func as u32));
    }

    /// 获取 PWR_GPIO0 引脚当前功能
    pub fn get_pwr_gpio0_func(&self) -> u32 {
        self.fmux.pwr_gpio0.read(FMUX_PWR_GPIO0::FSEL)
    }

    /// 设置 PWR_GPIO1 引脚功能 (类型安全版本)
    pub fn set_pwr_gpio1_func(&self, func: FMUX_PWR_GPIO1::FSEL::Value) {
        self.fmux.pwr_gpio1.write(FMUX_PWR_GPIO1::FSEL.val(func as u32));
    }

    /// 获取 PWR_GPIO1 引脚当前功能
    pub fn get_pwr_gpio1_func(&self) -> u32 {
        self.fmux.pwr_gpio1.read(FMUX_PWR_GPIO1::FSEL)
    }

    /// 设置 PWR_GPIO2 引脚功能 (类型安全版本)
    pub fn set_pwr_gpio2_func(&self, func: FMUX_PWR_GPIO2::FSEL::Value) {
        self.fmux.pwr_gpio2.write(FMUX_PWR_GPIO2::FSEL.val(func as u32));
    }

    /// 获取 PWR_GPIO2 引脚当前功能
    pub fn get_pwr_gpio2_func(&self) -> u32 {
        self.fmux.pwr_gpio2.read(FMUX_PWR_GPIO2::FSEL)
    }
}

// ============================================================================
// IIC3 引脚配置便捷方法 (通过 SD1_CMD/SD1_CLK 复用)
// ============================================================================

impl Pinmux {
    /// 配置 SD1_CMD 为 IIC3_SCL 功能
    ///
    /// 将 SD1_CMD (Pin 55) 引脚复用为 I2C3 的时钟线 (SCL)
    /// 功能选择值 = 2
    ///
    /// # 示例
    /// ```ignore
    /// let pinmux = unsafe { Pinmux::new() };
    /// pinmux.set_iic3_scl_on_sd1_cmd();
    /// ```
    pub fn set_iic3_scl_on_sd1_cmd(&self) {
        self.fmux.sd1_cmd.write(FMUX_SD1_CMD::FSEL.val(FMUX_SD1_CMD::FSEL::IIC3_SCL.into()));
    }

    /// 配置 SD1_CLK 为 IIC3_SDA 功能
    ///
    /// 将 SD1_CLK (Pin 56) 引脚复用为 I2C3 的数据线 (SDA)
    /// 功能选择值 = 2
    ///
    /// # 示例
    /// ```ignore
    /// let pinmux = unsafe { Pinmux::new() };
    /// pinmux.set_iic3_sda_on_sd1_clk();
    /// ```
    pub fn set_iic3_sda_on_sd1_clk(&self) {
        self.fmux.sd1_clk.write(FMUX_SD1_CLK::FSEL.val(FMUX_SD1_CLK::FSEL::IIC3_SDA.into()));
    }

    /// 配置 IIC3 引脚 (SD1_CMD -> SCL, SD1_CLK -> SDA)
    ///
    /// 一次性配置 I2C3 的两个引脚:
    /// - SD1_CMD (Pin 55) -> IIC3_SCL
    /// - SD1_CLK (Pin 56) -> IIC3_SDA
    ///
    /// # 示例
    /// ```ignore
    /// let pinmux = unsafe { Pinmux::new() };
    /// pinmux.setup_iic3_pins();
    /// ```
    pub fn setup_iic3_pins(&self) {
        self.set_iic3_scl_on_sd1_cmd();
        self.set_iic3_sda_on_sd1_clk();
    }

    /// 设置 SD1_D3 引脚功能 (类型安全版本)
    pub fn set_sd1_d3_func(&self, func: FMUX_SD1_D3::FSEL::Value) {
        self.fmux.sd1_d3.write(FMUX_SD1_D3::FSEL.val(func as u32));
    }

    /// 获取 SD1_D3 引脚当前功能
    pub fn get_sd1_d3_func(&self) -> u32 {
        self.fmux.sd1_d3.read(FMUX_SD1_D3::FSEL)
    }

    /// 设置 SD1_D2 引脚功能 (类型安全版本)
    pub fn set_sd1_d2_func(&self, func: FMUX_SD1_D2::FSEL::Value) {
        self.fmux.sd1_d2.write(FMUX_SD1_D2::FSEL.val(func as u32));
    }

    /// 获取 SD1_D2 引脚当前功能
    pub fn get_sd1_d2_func(&self) -> u32 {
        self.fmux.sd1_d2.read(FMUX_SD1_D2::FSEL)
    }

    /// 设置 SD1_D1 引脚功能 (类型安全版本)
    pub fn set_sd1_d1_func(&self, func: FMUX_SD1_D1::FSEL::Value) {
        self.fmux.sd1_d1.write(FMUX_SD1_D1::FSEL.val(func as u32));
    }

    /// 获取 SD1_D1 引脚当前功能
    pub fn get_sd1_d1_func(&self) -> u32 {
        self.fmux.sd1_d1.read(FMUX_SD1_D1::FSEL)
    }

    /// 设置 SD1_D0 引脚功能 (类型安全版本)
    pub fn set_sd1_d0_func(&self, func: FMUX_SD1_D0::FSEL::Value) {
        self.fmux.sd1_d0.write(FMUX_SD1_D0::FSEL.val(func as u32));
    }

    /// 获取 SD1_D0 引脚当前功能
    pub fn get_sd1_d0_func(&self) -> u32 {
        self.fmux.sd1_d0.read(FMUX_SD1_D0::FSEL)
    }

    /// 设置 SD1_CMD 引脚功能 (类型安全版本)
    ///
    /// 注意: IIC3_SCL 在此引脚上 (功能选择 = 2)
    pub fn set_sd1_cmd_func(&self, func: FMUX_SD1_CMD::FSEL::Value) {
        self.fmux.sd1_cmd.write(FMUX_SD1_CMD::FSEL.val(func as u32));
    }

    /// 获取 SD1_CMD 引脚当前功能
    pub fn get_sd1_cmd_func(&self) -> u32 {
        self.fmux.sd1_cmd.read(FMUX_SD1_CMD::FSEL)
    }

    /// 设置 SD1_CLK 引脚功能 (类型安全版本)
    ///
    /// 注意: IIC3_SDA 在此引脚上 (功能选择 = 2)
    pub fn set_sd1_clk_func(&self, func: FMUX_SD1_CLK::FSEL::Value) {
        self.fmux.sd1_clk.write(FMUX_SD1_CLK::FSEL.val(func as u32));
    }

    /// 获取 SD1_CLK 引脚当前功能
    pub fn get_sd1_clk_func(&self) -> u32 {
        self.fmux.sd1_clk.read(FMUX_SD1_CLK::FSEL)
    }
}

impl Pinmux {
    pub fn set_iic3(&self) {
        self.fmux.sd1_cmd.write(FMUX_SD1_CMD::FSEL::IIC3_SCL);
        self.fmux.sd1_clk.write(FMUX_SD1_CLK::FSEL::IIC3_SDA);
    }

    pub fn set_pwm_7(&self) {
        self.fmux.jtag_cpu_tms.write(FMUX_JTAG_CPU_TMS::FSEL::PWM_7);
    }
}
