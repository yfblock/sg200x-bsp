//! SG2002 GPIO 驱动模块
//!
//! 本模块提供 SG2002 芯片通用输入/输出 (GPIO) 功能的驱动程序。
//!
//! # 功能概述
//!
//! SG2002 芯片包含 5 组 GPIO 控制器:
//!
//! - **GPIO0 (GPIOA)**: Active Domain, 基地址 0x0302_0000
//! - **GPIO1 (GPIOB)**: Active Domain, 基地址 0x0302_1000
//! - **GPIO2 (GPIOC)**: Active Domain, 基地址 0x0302_2000
//! - **GPIO3 (GPIOD)**: Active Domain, 基地址 0x0302_3000
//! - **RTCSYS_GPIO**: No-die Domain, 基地址 0x0502_1000
//!
//! 每组 GPIO 最多支持 32 个引脚。
//!
//! # 功能特性
//!
//! - 可配置为输入或输出模式
//! - 支持高低电平读写
//! - 支持中断功能:
//!   - 电平触发 (高电平/低电平)
//!   - 边沿触发 (上升沿/下降沿)
//! - 支持去抖动 (Debounce) 功能
//! - 支持电平同步功能
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::gpio::{GPIO, GPIOPort, Direction, InterruptType};
//!
//! // 创建 GPIO0 实例
//! let gpio0 = unsafe { GPIO::new(GPIOPort::GPIO0) };
//!
//! // 配置引脚 0 为输出模式
//! gpio0.set_direction(0, Direction::Output);
//!
//! // 设置引脚 0 为高电平
//! gpio0.set(0, true);
//!
//! // 配置引脚 1 为输入模式
//! gpio0.set_direction(1, Direction::Input);
//!
//! // 读取引脚 1 的电平 (true = 高电平, false = 低电平)
//! let is_high = gpio0.read(1);
//!
//! // 配置引脚 2 的中断 (上升沿触发)
//! gpio0.configure_interrupt(2, InterruptType::RisingEdge);
//! gpio0.set_interrupt(2, true);
//! ```

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

// ============================================================================
// GPIO 基地址定义
// ============================================================================

/// GPIO0 (GPIOA) 基地址
pub const GPIO0_BASE: usize = 0x0302_0000;

/// GPIO1 (GPIOB) 基地址
pub const GPIO1_BASE: usize = 0x0302_1000;

/// GPIO2 (GPIOC) 基地址
pub const GPIO2_BASE: usize = 0x0302_2000;

/// GPIO3 (GPIOD) 基地址
pub const GPIO3_BASE: usize = 0x0302_3000;

/// RTCSYS_GPIO 基地址
pub const RTCSYS_GPIO_BASE: usize = 0x0502_1000;

// ============================================================================
// GPIO 寄存器位域定义
// ============================================================================

register_bitfields! [
    u32,

    /// GPIO 数据寄存器
    ///
    /// 当引脚配置为输出模式时，写入此寄存器的值将输出到对应引脚。
    /// 读取此寄存器返回最后写入的值。
    pub GPIO_SWPORTA_DR [
        /// Port A 数据位
        DR OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 数据方向寄存器
    ///
    /// 控制每个引脚的数据方向:
    /// - 0: 输入模式 (默认)
    /// - 1: 输出模式
    pub GPIO_SWPORTA_DDR [
        /// Port A 方向控制位
        DDR OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断使能寄存器
    ///
    /// 控制每个引脚是否启用中断功能:
    /// - 0: 作为普通 GPIO 信号 (默认)
    /// - 1: 配置为中断源
    pub GPIO_INTEN [
        /// 中断使能位
        INTEN OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断屏蔽寄存器
    ///
    /// 控制中断是否被屏蔽:
    /// - 0: 中断未屏蔽 (默认)
    /// - 1: 屏蔽中断
    pub GPIO_INTMASK [
        /// 中断屏蔽位
        INTMASK OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断类型寄存器
    ///
    /// 控制中断触发类型:
    /// - 0: 电平触发 (默认)
    /// - 1: 边沿触发
    pub GPIO_INTTYPE_LEVEL [
        /// 中断类型位
        INTTYPE OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断极性寄存器
    ///
    /// 控制中断触发极性:
    /// - 0: 低电平/下降沿触发 (默认)
    /// - 1: 高电平/上升沿触发
    pub GPIO_INT_POLARITY [
        /// 中断极性位
        POLARITY OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断状态寄存器 (屏蔽后)
    ///
    /// 显示经过屏蔽后的中断状态
    pub GPIO_INTSTATUS [
        /// 中断状态位
        INTSTATUS OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 原始中断状态寄存器 (屏蔽前)
    ///
    /// 显示屏蔽前的原始中断状态
    pub GPIO_RAW_INTSTATUS [
        /// 原始中断状态位
        RAW_INTSTATUS OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 去抖动寄存器
    ///
    /// 控制是否对输入信号进行去抖动处理:
    /// - 0: 不去抖动 (默认)
    /// - 1: 启用去抖动
    ///
    /// 启用后，信号需要在外部时钟的两个周期内保持稳定才会被处理。
    pub GPIO_DEBOUNCE [
        /// 去抖动使能位
        DEBOUNCE OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 中断清除寄存器
    ///
    /// 用于清除边沿触发类型的中断:
    /// - 写入 1: 清除对应中断
    /// - 写入 0: 无效果
    pub GPIO_PORTA_EOI [
        /// 中断清除位
        EOI OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 外部端口寄存器
    ///
    /// - 当配置为输入时: 读取引脚上的实际电平
    /// - 当配置为输出时: 读取数据寄存器的值
    pub GPIO_EXT_PORTA [
        /// 外部端口值
        EXT_PORTA OFFSET(0) NUMBITS(32) []
    ],

    /// GPIO 电平同步寄存器
    ///
    /// 控制电平敏感中断是否同步到 pclk_intr:
    /// - 0: 不同步 (默认)
    /// - 1: 同步到 pclk_intr
    pub GPIO_LS_SYNC [
        /// 同步使能位
        LS_SYNC OFFSET(0) NUMBITS(1) []
    ]
];

// ============================================================================
// GPIO 寄存器结构体定义
// ============================================================================

register_structs! {
    /// GPIO 寄存器组
    ///
    /// 包含 GPIO 控制器的所有寄存器
    pub GpioRegisters {
        /// 0x000 - Port A 数据寄存器
        (0x000 => pub swporta_dr: ReadWrite<u32, GPIO_SWPORTA_DR::Register>),
        /// 0x004 - Port A 数据方向寄存器
        (0x004 => pub swporta_ddr: ReadWrite<u32, GPIO_SWPORTA_DDR::Register>),
        /// 0x008 - 0x02C 保留
        (0x008 => _reserved0),
        /// 0x030 - 中断使能寄存器
        (0x030 => pub inten: ReadWrite<u32, GPIO_INTEN::Register>),
        /// 0x034 - 中断屏蔽寄存器
        (0x034 => pub intmask: ReadWrite<u32, GPIO_INTMASK::Register>),
        /// 0x038 - 中断类型寄存器
        (0x038 => pub inttype_level: ReadWrite<u32, GPIO_INTTYPE_LEVEL::Register>),
        /// 0x03C - 中断极性寄存器
        (0x03c => pub int_polarity: ReadWrite<u32, GPIO_INT_POLARITY::Register>),
        /// 0x040 - 中断状态寄存器
        (0x040 => pub intstatus: ReadOnly<u32, GPIO_INTSTATUS::Register>),
        /// 0x044 - 原始中断状态寄存器
        (0x044 => pub raw_intstatus: ReadOnly<u32, GPIO_RAW_INTSTATUS::Register>),
        /// 0x048 - 去抖动寄存器
        (0x048 => pub debounce: ReadWrite<u32, GPIO_DEBOUNCE::Register>),
        /// 0x04C - 中断清除寄存器
        (0x04c => pub porta_eoi: WriteOnly<u32, GPIO_PORTA_EOI::Register>),
        /// 0x050 - 外部端口寄存器
        (0x050 => pub ext_porta: ReadOnly<u32, GPIO_EXT_PORTA::Register>),
        /// 0x054 - 0x05C 保留
        (0x054 => _reserved1),
        /// 0x060 - 电平同步寄存器
        (0x060 => pub ls_sync: ReadWrite<u32, GPIO_LS_SYNC::Register>),
        /// 0x064 - 结束
        (0x064 => @END),
    }
}

// ============================================================================
// GPIO 枚举类型定义
// ============================================================================

/// GPIO 端口标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GPIOPort {
    /// GPIO0 (GPIOA) - Active Domain
    GPIO0,
    /// GPIO1 (GPIOB) - Active Domain
    GPIO1,
    /// GPIO2 (GPIOC) - Active Domain
    GPIO2,
    /// GPIO3 (GPIOD) - Active Domain
    GPIO3,
    /// RTCSYS_GPIO - No-die Domain
    RTCSysGPIO,
}

impl GPIOPort {
    /// 获取 GPIO 端口的基地址
    pub const fn base_address(self) -> usize {
        match self {
            GPIOPort::GPIO0 => GPIO0_BASE,
            GPIOPort::GPIO1 => GPIO1_BASE,
            GPIOPort::GPIO2 => GPIO2_BASE,
            GPIOPort::GPIO3 => GPIO3_BASE,
            GPIOPort::RTCSysGPIO => RTCSYS_GPIO_BASE,
        }
    }
}

/// GPIO 引脚方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 输入模式
    Input,
    /// 输出模式
    Output,
}

/// GPIO 中断触发类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// 低电平触发
    LowLevel,
    /// 高电平触发
    HighLevel,
    /// 下降沿触发
    FallingEdge,
    /// 上升沿触发
    RisingEdge,
}


// ============================================================================
// GPIO 驱动实现
// ============================================================================

/// GPIO 驱动结构体
///
/// 提供对单个 GPIO 端口的访问接口
#[derive(Clone)]
pub struct GPIO {
    /// GPIO 寄存器组
    regs: &'static GpioRegisters,
    /// GPIO 端口标识
    port: GPIOPort,
}

impl GPIO {
    /// 创建新的 GPIO 驱动实例
    ///
    /// # 参数
    ///
    /// - `port`: GPIO 端口标识
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个指向同一端口的实例导致数据竞争
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let gpio0 = unsafe { GPIO::new(GPIOPort::GPIO0) };
    /// ```
    pub fn new(port: GPIOPort) -> Self {
        unsafe {
            Self {
                regs: &*(port.base_address() as *const GpioRegisters),
                port,
            }
        }
    }

    /// 从指定基地址创建 GPIO 驱动实例
    ///
    /// # 参数
    ///
    /// - `base`: GPIO 寄存器基地址
    /// - `port`: GPIO 端口标识 (用于标识目的)
    ///
    /// # Safety
    ///
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize, port: GPIOPort) -> Self {
        unsafe {
            Self {
                regs: &*(base as *const GpioRegisters),
                port,
            }
        }
    }

    /// 获取 GPIO 端口标识
    pub fn port(&self) -> GPIOPort {
        self.port
    }

    /// 获取 GPIO 寄存器组的引用
    pub fn registers(&self) -> &GpioRegisters {
        self.regs
    }

    // ========================================================================
    // 基本 GPIO 操作
    // ========================================================================

    /// 设置引脚方向
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `direction`: 引脚方向
    ///
    /// # Panics
    ///
    /// 如果 `pin >= 32` 则 panic
    pub fn set_direction(&self, pin: u8, direction: Direction) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        match direction {
            Direction::Input => {
                let current = self.regs.swporta_ddr.get();
                self.regs.swporta_ddr.set(current & !mask);
            }
            Direction::Output => {
                let current = self.regs.swporta_ddr.get();
                self.regs.swporta_ddr.set(current | mask);
            }
        }
    }

    /// 获取引脚方向
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    ///
    /// # 返回
    ///
    /// 引脚当前的方向配置
    pub fn get_direction(&self, pin: u8) -> Direction {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        if (self.regs.swporta_ddr.get() & mask) != 0 {
            Direction::Output
        } else {
            Direction::Input
        }
    }

    /// 设置引脚输出电平
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `high`: true 为高电平，false 为低电平
    ///
    /// # 注意
    ///
    /// 引脚必须先配置为输出模式才能正常工作
    pub fn set(&self, pin: u8, high: bool) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        let current = self.regs.swporta_dr.get();
        if high {
            self.regs.swporta_dr.set(current | mask);
        } else {
            self.regs.swporta_dr.set(current & !mask);
        }
    }

    /// 翻转引脚电平
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    pub fn toggle(&self, pin: u8) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        let current = self.regs.swporta_dr.get();
        self.regs.swporta_dr.set(current ^ mask);
    }

    /// 读取引脚电平
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    ///
    /// # 返回
    ///
    /// - `true`: 高电平
    /// - `false`: 低电平
    /// - 输入模式: 返回引脚上的实际电平
    /// - 输出模式: 返回数据寄存器中的值
    pub fn read(&self, pin: u8) -> bool {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        (self.regs.ext_porta.get() & mask) != 0
    }

    /// 检查引脚是否为高电平
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    #[inline]
    pub fn is_high(&self, pin: u8) -> bool {
        self.read(pin)
    }

    /// 检查引脚是否为低电平
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    #[inline]
    pub fn is_low(&self, pin: u8) -> bool {
        !self.read(pin)
    }

    // ========================================================================
    // 批量 GPIO 操作
    // ========================================================================

    /// 设置多个引脚的方向
    ///
    /// # 参数
    ///
    /// - `mask`: 要设置为输出的引脚掩码 (位为 1 表示输出，位为 0 表示输入)
    pub fn set_direction_mask(&self, mask: u32) {
        self.regs.swporta_ddr.set(mask);
    }

    /// 获取所有引脚的方向
    ///
    /// # 返回
    ///
    /// 方向掩码 (位为 1 表示输出，位为 0 表示输入)
    pub fn get_direction_mask(&self) -> u32 {
        self.regs.swporta_ddr.get()
    }

    /// 设置多个引脚的输出值
    ///
    /// # 参数
    ///
    /// - `value`: 输出值掩码
    pub fn write_port(&self, value: u32) {
        self.regs.swporta_dr.set(value);
    }

    /// 读取整个端口的值
    ///
    /// # 返回
    ///
    /// 端口所有引脚的当前电平
    pub fn read_port(&self) -> u32 {
        self.regs.ext_porta.get()
    }

    // ========================================================================
    // 中断配置
    // ========================================================================

    /// 配置引脚中断类型
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `int_type`: 中断触发类型
    ///
    /// # 注意
    ///
    /// 配置中断类型后，还需要调用 `set_interrupt(pin, true)` 来启用中断
    pub fn configure_interrupt(&self, pin: u8, int_type: InterruptType) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;

        // 配置中断类型和极性
        let (is_edge, is_high_or_rising) = match int_type {
            InterruptType::LowLevel => (false, false),
            InterruptType::HighLevel => (false, true),
            InterruptType::FallingEdge => (true, false),
            InterruptType::RisingEdge => (true, true),
        };

        // 设置中断类型 (电平/边沿)
        let current_type = self.regs.inttype_level.get();
        if is_edge {
            self.regs.inttype_level.set(current_type | mask);
        } else {
            self.regs.inttype_level.set(current_type & !mask);
        }

        // 设置中断极性
        let current_polarity = self.regs.int_polarity.get();
        if is_high_or_rising {
            self.regs.int_polarity.set(current_polarity | mask);
        } else {
            self.regs.int_polarity.set(current_polarity & !mask);
        }
    }

    /// 设置引脚中断使能
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `enable`: true 启用中断，false 禁用中断
    pub fn set_interrupt(&self, pin: u8, enable: bool) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        let current = self.regs.inten.get();
        if enable {
            self.regs.inten.set(current | mask);
        } else {
            self.regs.inten.set(current & !mask);
        }
    }

    /// 设置引脚中断屏蔽
    ///
    /// 屏蔽后，中断不会传递到中断控制器，但原始中断状态仍会更新
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `masked`: true 屏蔽中断，false 取消屏蔽
    pub fn set_interrupt_mask(&self, pin: u8, masked: bool) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        let current = self.regs.intmask.get();
        if masked {
            self.regs.intmask.set(current | mask);
        } else {
            self.regs.intmask.set(current & !mask);
        }
    }

    /// 清除引脚中断
    ///
    /// 用于清除边沿触发类型的中断
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    pub fn clear_interrupt(&self, pin: u8) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        self.regs.porta_eoi.set(mask);
    }

    /// 清除所有中断
    pub fn clear_all_interrupts(&self) {
        self.regs.porta_eoi.set(0xFFFF_FFFF);
    }

    /// 获取中断状态 (屏蔽后)
    ///
    /// # 返回
    ///
    /// 中断状态掩码，位为 1 表示对应引脚有中断待处理
    pub fn get_interrupt_status(&self) -> u32 {
        self.regs.intstatus.get()
    }

    /// 获取原始中断状态 (屏蔽前)
    ///
    /// # 返回
    ///
    /// 原始中断状态掩码
    pub fn get_raw_interrupt_status(&self) -> u32 {
        self.regs.raw_intstatus.get()
    }

    /// 检查指定引脚是否有中断待处理
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    pub fn is_interrupt_pending(&self, pin: u8) -> bool {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        (self.regs.intstatus.get() & mask) != 0
    }

    // ========================================================================
    // 去抖动配置
    // ========================================================================

    /// 设置引脚去抖动
    ///
    /// 启用后，输入信号需要在外部时钟的两个周期内保持稳定才会被处理
    ///
    /// # 参数
    ///
    /// - `pin`: 引脚编号 (0-31)
    /// - `enable`: true 启用去抖动，false 禁用去抖动
    pub fn set_debounce(&self, pin: u8, enable: bool) {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        let mask = 1u32 << pin;
        let current = self.regs.debounce.get();
        if enable {
            self.regs.debounce.set(current | mask);
        } else {
            self.regs.debounce.set(current & !mask);
        }
    }

    // ========================================================================
    // 电平同步配置
    // ========================================================================

    /// 设置电平同步
    ///
    /// 启用后，所有电平敏感中断将同步到 pclk_intr
    ///
    /// # 参数
    ///
    /// - `enable`: true 启用电平同步，false 禁用电平同步
    pub fn set_level_sync(&self, enable: bool) {
        self.regs.ls_sync.set(if enable { 1 } else { 0 });
    }
}

// ============================================================================
// GpioPin 单引脚抽象
// ============================================================================

/// 单个 GPIO 引脚的抽象
///
/// 提供对单个引脚的便捷操作接口
pub struct GPIOPin {
    /// GPIO 控制器引用
    gpio: GPIO,
    /// 引脚编号
    pin: u8,
}

impl GPIOPin {
    /// 创建新的 GPIO 引脚实例
    ///
    /// # 参数
    ///
    /// - `gpio`: GPIO 控制器引用
    /// - `pin`: 引脚编号 (0-31)
    ///
    /// # Panics
    ///
    /// 如果 `pin >= 32` 则 panic
    pub fn new(gpio: &GPIO, pin: u8) -> Self {
        assert!(pin < 32, "GPIO pin number must be less than 32");
        Self { gpio: gpio.clone(), pin }
    }

    pub fn with_port(port: GPIOPort, pin: u8) -> Self {
        Self::new(&GPIO::new(port), pin)
    }

    /// 获取引脚编号
    pub fn pin(&self) -> u8 {
        self.pin
    }

    /// 设置引脚方向
    pub fn set_direction(&self, direction: Direction) {
        self.gpio.set_direction(self.pin, direction);
    }

    /// 获取引脚方向
    pub fn get_direction(&self) -> Direction {
        self.gpio.get_direction(self.pin)
    }

    /// 配置为输入模式
    pub fn into_input(&self) {
        self.set_direction(Direction::Input);
    }

    /// 配置为输出模式
    pub fn into_output(&self) {
        self.set_direction(Direction::Output);
    }

    /// 设置引脚电平
    ///
    /// # 参数
    ///
    /// - `high`: true 为高电平，false 为低电平
    pub fn set(&self, high: bool) {
        self.gpio.set(self.pin, high);
    }

    /// 翻转电平
    pub fn toggle(&self) {
        self.gpio.toggle(self.pin);
    }

    /// 读取引脚电平
    ///
    /// # 返回
    ///
    /// - `true`: 高电平
    /// - `false`: 低电平
    pub fn read(&self) -> bool {
        self.gpio.read(self.pin)
    }

    /// 检查是否为高电平
    #[inline]
    pub fn is_high(&self) -> bool {
        self.gpio.is_high(self.pin)
    }

    /// 检查是否为低电平
    #[inline]
    pub fn is_low(&self) -> bool {
        self.gpio.is_low(self.pin)
    }

    /// 配置中断类型
    pub fn configure_interrupt(&self, int_type: InterruptType) {
        self.gpio.configure_interrupt(self.pin, int_type);
    }

    /// 设置中断使能
    ///
    /// # 参数
    ///
    /// - `enable`: true 启用中断，false 禁用中断
    pub fn set_interrupt(&self, enable: bool) {
        self.gpio.set_interrupt(self.pin, enable);
    }

    /// 设置中断屏蔽
    ///
    /// # 参数
    ///
    /// - `masked`: true 屏蔽中断，false 取消屏蔽
    pub fn set_interrupt_mask(&self, masked: bool) {
        self.gpio.set_interrupt_mask(self.pin, masked);
    }

    /// 清除中断
    pub fn clear_interrupt(&self) {
        self.gpio.clear_interrupt(self.pin);
    }

    /// 检查是否有中断待处理
    pub fn is_interrupt_pending(&self) -> bool {
        self.gpio.is_interrupt_pending(self.pin)
    }

    /// 设置去抖动
    ///
    /// # 参数
    ///
    /// - `enable`: true 启用去抖动，false 禁用去抖动
    pub fn set_debounce(&self, enable: bool) {
        self.gpio.set_debounce(self.pin, enable);
    }
}

