//! SG2002 GPIO 驱动模块
//!
//! 本模块提供 SG2002 芯片通用输入/输出 (GPIO) 功能的驱动程序。
//!
//! # 功能概述
//!
//! SG2002 芯片包含 5 组 GPIO 控制器:
//!
//! - **GPIO0 (GPIOA)**: Active Domain，[`crate::soc::GPIO0_BASE`]
//! - **GPIO1 (GPIOB)**: Active Domain，[`crate::soc::GPIO1_BASE`]
//! - **GPIO2 (GPIOC)**: Active Domain，[`crate::soc::GPIO2_BASE`]
//! - **GPIO3 (GPIOD)**: Active Domain，[`crate::soc::GPIO3_BASE`]
//! - **RTCSYS_GPIO**: No-die Domain，[`crate::soc::RTCSYS_GPIO_BASE`]
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
//! use sg200x_bsp::gpio::{GPIO, Direction, InterruptType};
//! use sg200x_bsp::soc::GPIO0_BASE;
//!
//! // 创建 GPIO0 实例
//! let gpio0 = unsafe { GPIO::new(GPIO0_BASE) };
//!
//! // 通过 pin() 获取单引脚句柄
//! let led = gpio0.pin(0);
//! led.into_output();
//! led.set(true);
//!
//! let button = gpio0.pin(1);
//! button.into_input();
//! let pressed = button.is_high();
//!
//! let irq_pin = gpio0.pin(2);
//! irq_pin.configure_interrupt(InterruptType::RisingEdge);
//! irq_pin.set_interrupt(true);
//! ```

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

pub use crate::soc::{
    GPIO0_BASE, GPIO1_BASE, GPIO2_BASE, GPIO3_BASE, RTCSYS_GPIO_BASE,
};

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

/// GPIO 控制器（一组最多 32 根线）。
///
/// 单引脚操作请通过 [`GPIO::pin`] 取得 [`GPIOPin`]。
pub struct GPIO {
    regs: &'static GpioRegisters,
}

impl GPIO {
    /// 创建新的 GPIO 驱动实例
    ///
    /// # 参数
    ///
    /// - `base`: GPIO 控制器 MMIO 基地址（见 [`crate::soc::GPIO0_BASE`] 等）
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
    /// use sg200x_bsp::soc::GPIO0_BASE;
    /// let gpio0 = unsafe { GPIO::new(GPIO0_BASE) };
    /// ```
    pub unsafe fn new(base: usize) -> Self {
        unsafe {
            Self {
                regs: &*(base as *const GpioRegisters),
            }
        }
    }

    /// 获取 GPIO 寄存器组的引用
    pub fn registers(&self) -> &GpioRegisters {
        self.regs
    }

    /// 返回指定引脚的操作句柄。
    ///
    /// # Panics
    ///
    /// `index >= 32` 时 panic。
    pub fn pin(&self, index: u8) -> GPIOPin<'_> {
        assert!(index < 32, "GPIO pin number must be less than 32");
        GPIOPin {
            gpio: self,
            index,
        }
    }

    // ========================================================================
    // 端口级 GPIO 操作
    // ========================================================================

    /// 设置多个引脚的方向
    ///
    /// # 参数
    ///
    /// - `dirs`: 要设置为输出的引脚掩码 (位为 1 表示输出，位为 0 表示输入)
    pub fn set_direction(&self, dirs: u32) {
        self.regs.swporta_ddr.set(dirs);
    }

    /// 获取所有引脚的方向
    ///
    /// # 返回
    ///
    /// 方向掩码 (位为 1 表示输出，位为 0 表示输入)
    pub fn get_direction(&self) -> u32 {
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
    // 端口级中断
    // ========================================================================

    /// 清除所有引脚的中断（写 EOI 全 1）。
    pub fn clear_all_interrupts(&self) {
        self.regs.porta_eoi.set(0xFFFF_FFFF);
    }

    /// 中断状态（屏蔽后）；位为 1 表示对应引脚有待处理中断。
    pub fn get_interrupt_status(&self) -> u32 {
        self.regs.intstatus.get()
    }

    /// 原始中断状态（屏蔽前）。
    pub fn get_raw_interrupt_status(&self) -> u32 {
        self.regs.raw_intstatus.get()
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
// GPIOPin 单引脚抽象
// ============================================================================

/// 单根 GPIO 线；由 [`GPIO::pin`] 创建，不可单独构造。
pub struct GPIOPin<'a> {
    gpio: &'a GPIO,
    index: u8,
}

impl<'a> GPIOPin<'a> {
    #[inline]
    fn mask(&self) -> u32 {
        1u32 << self.index
    }

    #[inline]
    fn regs(&self) -> &GpioRegisters {
        self.gpio.regs
    }

    /// 引脚编号 (0–31)。
    pub fn index(&self) -> u8 {
        self.index
    }

    /// 设置引脚方向。
    pub fn set_direction(&self, direction: Direction) {
        let mask = self.mask();
        let current = self.regs().swporta_ddr.get();
        let value = match direction {
            Direction::Input => current & !mask,
            Direction::Output => current | mask,
        };
        self.regs().swporta_ddr.set(value);
    }

    /// 获取引脚方向。
    pub fn get_direction(&self) -> Direction {
        if self.regs().swporta_ddr.get() & self.mask() != 0 {
            Direction::Output
        } else {
            Direction::Input
        }
    }

    /// 配置为输入模式。
    pub fn into_input(&self) {
        self.set_direction(Direction::Input);
    }

    /// 配置为输出模式。
    pub fn into_output(&self) {
        self.set_direction(Direction::Output);
    }

    /// 设置输出电平（须先配置为输出）。
    pub fn set(&self, high: bool) {
        let mask = self.mask();
        let current = self.regs().swporta_dr.get();
        if high {
            self.regs().swporta_dr.set(current | mask);
        } else {
            self.regs().swporta_dr.set(current & !mask);
        }
    }

    /// 翻转输出电平。
    pub fn toggle(&self) {
        let mask = self.mask();
        let current = self.regs().swporta_dr.get();
        self.regs().swporta_dr.set(current ^ mask);
    }

    /// 读取引脚电平（输入为 pad 实际电平）。
    pub fn read(&self) -> bool {
        self.regs().ext_porta.get() & self.mask() != 0
    }

    #[inline]
    pub fn is_high(&self) -> bool {
        self.read()
    }

    #[inline]
    pub fn is_low(&self) -> bool {
        !self.read()
    }

    /// 配置中断类型；之后调用 [`set_interrupt`](Self::set_interrupt) 启用。
    pub fn configure_interrupt(&self, int_type: InterruptType) {
        let mask = self.mask();
        let (is_edge, is_high_or_rising) = match int_type {
            InterruptType::LowLevel => (false, false),
            InterruptType::HighLevel => (false, true),
            InterruptType::FallingEdge => (true, false),
            InterruptType::RisingEdge => (true, true),
        };

        let current_type = self.regs().inttype_level.get();
        if is_edge {
            self.regs().inttype_level.set(current_type | mask);
        } else {
            self.regs().inttype_level.set(current_type & !mask);
        }

        let current_polarity = self.regs().int_polarity.get();
        if is_high_or_rising {
            self.regs().int_polarity.set(current_polarity | mask);
        } else {
            self.regs().int_polarity.set(current_polarity & !mask);
        }
    }

    /// 启用或禁用中断。
    pub fn set_interrupt(&self, enable: bool) {
        let mask = self.mask();
        let current = self.regs().inten.get();
        if enable {
            self.regs().inten.set(current | mask);
        } else {
            self.regs().inten.set(current & !mask);
        }
    }

    /// 屏蔽中断（原始状态仍会更新）。
    pub fn set_interrupt_mask(&self, masked: bool) {
        let mask = self.mask();
        let current = self.regs().intmask.get();
        if masked {
            self.regs().intmask.set(current | mask);
        } else {
            self.regs().intmask.set(current & !mask);
        }
    }

    /// 清除本引脚中断（边沿触发）。
    pub fn clear_interrupt(&self) {
        self.regs().porta_eoi.set(self.mask());
    }

    /// 本引脚是否有待处理中断。
    pub fn is_interrupt_pending(&self) -> bool {
        self.regs().intstatus.get() & self.mask() != 0
    }

    /// 启用或禁用输入去抖动。
    pub fn set_debounce(&self, enable: bool) {
        let mask = self.mask();
        let current = self.regs().debounce.get();
        if enable {
            self.regs().debounce.set(current | mask);
        } else {
            self.regs().debounce.set(current & !mask);
        }
    }
}

