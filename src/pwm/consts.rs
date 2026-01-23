//! # PWM 驱动常量和寄存器定义
//!
//! 本模块定义了 SG2002 芯片 PWM 控制器相关的：
//! - 寄存器偏移地址
//! - 位域结构体 (使用 tock-registers)
//! - 模式枚举
//! - 错误类型

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

// ============================================================================
// 寄存器位域定义 (使用 tock-registers)
// ============================================================================

register_bitfields! [
    u32,

    /// PWM 低电平/高电平拍数寄存器 (偏移 0x000, 0x008, 0x010, 0x018)
    /// 当 POLARITY[n] 为 0 时，此值为低电平拍数
    /// 当 POLARITY[n] 为 1 时，此值为高电平拍数
    pub HLPERIOD [
        /// 低电平/高电平拍数 (bit0-29)
        /// 单位为 clk_pwm
        HLPERIOD OFFSET(0) NUMBITS(30) []
    ],

    /// PWM 方波周期拍数寄存器 (偏移 0x004, 0x00c, 0x014, 0x01c)
    /// 注意：PERIOD 值必须大于 HLPERIOD 值
    pub PERIOD [
        /// 方波周期拍数 (bit0-29)
        /// 单位为 clk_pwm
        PERIOD OFFSET(0) NUMBITS(30) []
    ],

    /// PWM 极性和模式配置寄存器 (偏移 0x040)
    pub POLARITY [
        /// PWM[0]~[3] 信号极性 (bit0-3)
        /// [n] = 0: PWM[n] 默认低电平输出
        /// [n] = 1: PWM[n] 默认高电平输出
        POLARITY OFFSET(0) NUMBITS(4) [],

        /// PWM[0]~[3] 工作模式 (bit8-11)
        /// [n+8] = 0: PWM[n] 连续输出模式
        /// [n+8] = 1: PWM[n] 固定脉冲数输出模式
        PWMMODE OFFSET(8) NUMBITS(4) [],

        /// PWM 同步相位输出模式使能 (bit16)
        /// 0 = PWM[0]~[3] 工作在普通模式
        /// 1 = PWM[0]~[3] 工作在 4 路同步输出模式
        SHIFTMODE OFFSET(16) NUMBITS(1) [],

        /// APB 时钟门控控制 (bit20)
        /// 0 = 使能 APB 时钟门控，空闲时自动关闭时钟
        /// 1 = APB 时钟保持常开
        PCLK_FORCE_EN OFFSET(20) NUMBITS(1) []
    ],

    /// PWM 启动寄存器 (偏移 0x044)
    pub PWMSTART [
        /// 使能 PWM[0]~[3] (bit0-3)
        /// [n] = 0: 停止 PWM[n]
        /// [n] = 1: 输出 PWM[n]
        /// 当 PWMMODE 为 0 时，写 bit n 为 0 再写 1 启动 PWM[n] 输出，
        /// 直到 bit n 写 0 停止输出。
        /// 当 PWMMODE 为 1 时，写 bit n 为 1 启动 PWM[n] 输出，
        /// 输出脉冲数等于 PCOUNTn 值后自动停止。
        /// 当 SHIFTMODE 为 1 时，PWMSTART[3:0] 为 PWM[0]~[3] 的输出使能，
        /// PWM 启动由 SHIFTSTART 控制。
        PWMSTART OFFSET(0) NUMBITS(4) []
    ],

    /// PWM 完成状态寄存器 (偏移 0x048)
    pub PWMDONE [
        /// PWM[0]~[3] 结束输出状态 (bit0-3)
        /// [n] = 1: PWMn 已关闭输出
        /// 当 PWMSTART[n] 从 0 设为 1 时，此寄存器值清零
        PWMDONE OFFSET(0) NUMBITS(4) []
    ],

    /// PWM 动态更新寄存器 (偏移 0x04c)
    pub PWMUPDATE [
        /// 动态加载 PWM 参数 (bit0-3)
        /// 当 PWMSTART 从 0 写 1 时，寄存器值 (HLPERIODn, PERIODn)
        /// 被暂存在 PWM 内部。如果要在 PWM 输出时动态改变波形，
        /// 先写新值到 HLPERIODn 和 PERIODn，然后写 1 到 PWMUPDATE[n]
        /// 再写 0 使新值生效。
        PWMUPDATE OFFSET(0) NUMBITS(4) []
    ],

    /// PWM 脉冲数寄存器 (偏移 0x050, 0x054, 0x058, 0x05c)
    /// 设置值必须大于 0
    /// 仅当 PWMMODE[n] 设为 1 时有效
    pub PCOUNT [
        /// PWM[n] 脉冲数 (bit0-23)
        PCOUNT OFFSET(0) NUMBITS(24) []
    ],

    /// PWM 输出脉冲计数状态寄存器 (偏移 0x060, 0x064, 0x068, 0x06c)
    pub PULSECOUNT [
        /// PWM[n] 输出脉冲数状态 (bit0-23)
        PULSECOUNT OFFSET(0) NUMBITS(24) []
    ],

    /// PWM 同步模式相位差寄存器 (偏移 0x080, 0x084, 0x088, 0x08c)
    /// 仅当 SHIFTMODE 设为 1 时有效
    pub SHIFTCOUNT [
        /// PWM[n] 第一个脉冲输出的相位差 (bit0-23)
        /// 单位为 clk_pwm
        SHIFTCOUNT OFFSET(0) NUMBITS(24) []
    ],

    /// PWM 同步模式启动寄存器 (偏移 0x090)
    pub SHIFTSTART [
        /// 同步模式下使能 PWM 输出 (bit0)
        /// 当 SHIFTMODE 设为 1 时，写 1 到此寄存器后开始输出 PWM[0]~[3]
        SHIFTSTART OFFSET(0) NUMBITS(1) []
    ],

    /// PWM IO 输出使能寄存器 (偏移 0x0d0)
    pub PWM_OE [
        /// PWM[0]~[3] IO 输出使能 (bit0-3)
        /// 1 = IO 为输出，0 = IO 为输入
        PWM_OE OFFSET(0) NUMBITS(4) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// PWM 控制器寄存器组
    /// 每个 PWM 控制器包含 4 路 PWM 输出
    pub PwmRegisters {
        /// PWM[0] 低电平拍数寄存器 (偏移 0x000)
        (0x000 => pub hlperiod0: ReadWrite<u32, HLPERIOD::Register>),

        /// PWM[0] 方波周期拍数寄存器 (偏移 0x004)
        (0x004 => pub period0: ReadWrite<u32, PERIOD::Register>),

        /// PWM[1] 低电平拍数寄存器 (偏移 0x008)
        (0x008 => pub hlperiod1: ReadWrite<u32, HLPERIOD::Register>),

        /// PWM[1] 方波周期拍数寄存器 (偏移 0x00c)
        (0x00c => pub period1: ReadWrite<u32, PERIOD::Register>),

        /// PWM[2] 低电平拍数寄存器 (偏移 0x010)
        (0x010 => pub hlperiod2: ReadWrite<u32, HLPERIOD::Register>),

        /// PWM[2] 方波周期拍数寄存器 (偏移 0x014)
        (0x014 => pub period2: ReadWrite<u32, PERIOD::Register>),

        /// PWM[3] 低电平拍数寄存器 (偏移 0x018)
        (0x018 => pub hlperiod3: ReadWrite<u32, HLPERIOD::Register>),

        /// PWM[3] 方波周期拍数寄存器 (偏移 0x01c)
        (0x01c => pub period3: ReadWrite<u32, PERIOD::Register>),

        /// 保留 (偏移 0x020-0x03c)
        (0x020 => _reserved0),

        /// PWM 极性和模式配置寄存器 (偏移 0x040)
        (0x040 => pub polarity: ReadWrite<u32, POLARITY::Register>),

        /// PWM 启动寄存器 (偏移 0x044)
        (0x044 => pub pwmstart: ReadWrite<u32, PWMSTART::Register>),

        /// PWM 完成状态寄存器 (偏移 0x048)
        (0x048 => pub pwmdone: ReadWrite<u32, PWMDONE::Register>),

        /// PWM 动态更新寄存器 (偏移 0x04c)
        (0x04c => pub pwmupdate: ReadWrite<u32, PWMUPDATE::Register>),

        /// PWM[0] 脉冲数寄存器 (偏移 0x050)
        (0x050 => pub pcount0: ReadWrite<u32, PCOUNT::Register>),

        /// PWM[1] 脉冲数寄存器 (偏移 0x054)
        (0x054 => pub pcount1: ReadWrite<u32, PCOUNT::Register>),

        /// PWM[2] 脉冲数寄存器 (偏移 0x058)
        (0x058 => pub pcount2: ReadWrite<u32, PCOUNT::Register>),

        /// PWM[3] 脉冲数寄存器 (偏移 0x05c)
        (0x05c => pub pcount3: ReadWrite<u32, PCOUNT::Register>),

        /// PWM[0] 输出脉冲计数状态寄存器 (偏移 0x060)
        (0x060 => pub pulsecount0: ReadWrite<u32, PULSECOUNT::Register>),

        /// PWM[1] 输出脉冲计数状态寄存器 (偏移 0x064)
        (0x064 => pub pulsecount1: ReadWrite<u32, PULSECOUNT::Register>),

        /// PWM[2] 输出脉冲计数状态寄存器 (偏移 0x068)
        (0x068 => pub pulsecount2: ReadWrite<u32, PULSECOUNT::Register>),

        /// PWM[3] 输出脉冲计数状态寄存器 (偏移 0x06c)
        (0x06c => pub pulsecount3: ReadWrite<u32, PULSECOUNT::Register>),

        /// 保留 (偏移 0x070-0x07c)
        (0x070 => _reserved1),

        /// PWM[0] 同步模式相位差寄存器 (偏移 0x080)
        (0x080 => pub shiftcount0: ReadWrite<u32, SHIFTCOUNT::Register>),

        /// PWM[1] 同步模式相位差寄存器 (偏移 0x084)
        (0x084 => pub shiftcount1: ReadWrite<u32, SHIFTCOUNT::Register>),

        /// PWM[2] 同步模式相位差寄存器 (偏移 0x088)
        (0x088 => pub shiftcount2: ReadWrite<u32, SHIFTCOUNT::Register>),

        /// PWM[3] 同步模式相位差寄存器 (偏移 0x08c)
        (0x08c => pub shiftcount3: ReadWrite<u32, SHIFTCOUNT::Register>),

        /// PWM 同步模式启动寄存器 (偏移 0x090)
        (0x090 => pub shiftstart: ReadWrite<u32, SHIFTSTART::Register>),

        /// 保留 (偏移 0x094-0x0cc)
        (0x094 => _reserved2),

        /// PWM IO 输出使能寄存器 (偏移 0x0d0)
        (0x0d0 => pub pwm_oe: ReadWrite<u32, PWM_OE::Register>),

        /// 结束标记
        (0x0d4 => @END),
    }
}

// ============================================================================
// 错误类型和枚举定义
// ============================================================================

/// PWM 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmError {
    /// 无效的通道号
    InvalidChannel,
    /// 无效的周期值
    InvalidPeriod,
    /// 无效的占空比
    InvalidDutyCycle,
    /// 无效的脉冲数
    InvalidPulseCount,
    /// PWM 正在运行
    Busy,
    /// 配置错误
    ConfigError,
}

/// PWM 工作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PwmMode {
    /// 连续输出模式
    /// PWM 持续输出直到手动停止
    Continuous = 0,
    /// 固定脉冲数输出模式
    /// 输出指定数量的脉冲后自动停止
    PulseCount = 1,
}

impl PwmMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Continuous),
            1 => Some(Self::PulseCount),
            _ => None,
        }
    }
}

/// PWM 信号极性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PwmPolarity {
    /// 默认低电平输出
    /// HLPERIOD 值为低电平拍数
    ActiveHigh = 0,
    /// 默认高电平输出
    /// HLPERIOD 值为高电平拍数
    ActiveLow = 1,
}

impl PwmPolarity {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::ActiveHigh),
            1 => Some(Self::ActiveLow),
            _ => None,
        }
    }
}

/// PWM 时钟源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmClockSource {
    /// 100MHz 时钟源 (默认)
    Clk100MHz,
    /// 148.5MHz 时钟源
    Clk148_5MHz,
}

impl PwmClockSource {
    /// 获取时钟频率 (Hz)
    pub const fn frequency(&self) -> u32 {
        match self {
            Self::Clk100MHz => 100_000_000,
            Self::Clk148_5MHz => 148_500_000,
        }
    }
}

/// PWM 通道索引 (相对于控制器)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PwmChannel {
    /// 通道 0
    Channel0 = 0,
    /// 通道 1
    Channel1 = 1,
    /// 通道 2
    Channel2 = 2,
    /// 通道 3
    Channel3 = 3,
}

impl PwmChannel {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Channel0),
            1 => Some(Self::Channel1),
            2 => Some(Self::Channel2),
            3 => Some(Self::Channel3),
            _ => None,
        }
    }

    /// 获取通道索引
    pub const fn index(&self) -> u8 {
        *self as u8
    }

    /// 获取通道位掩码
    pub const fn mask(&self) -> u32 {
        1 << (*self as u32)
    }
}

/// PWM 最大周期值 (30 位)
pub const PWM_MAX_PERIOD: u32 = (1 << 30) - 1;

/// PWM 最大脉冲数 (24 位)
pub const PWM_MAX_PULSE_COUNT: u32 = (1 << 24) - 1;

/// PWM 最大相位差值 (24 位)
pub const PWM_MAX_SHIFT_COUNT: u32 = (1 << 24) - 1;

/// PWM 默认时钟频率 (100MHz)
pub const PWM_DEFAULT_CLK_FREQ: u32 = 100_000_000;
