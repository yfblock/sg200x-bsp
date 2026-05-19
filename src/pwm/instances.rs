//! # PWM 实例定义
//!
//! SG2002 芯片共有 4 个 PWM 控制器（基址见 [`PWM0_BASE`] … [`PWM3_BASE`]）：
//! - PWM0: PWM[0]–PWM[3]
//! - PWM1: PWM[4]–PWM[7]
//! - PWM2: PWM[8]–PWM[11]
//! - PWM3: PWM[12]–PWM[15]

pub use crate::soc::{PWM0_BASE, PWM1_BASE, PWM2_BASE, PWM3_BASE};

/// 控制器 MMIO 基址（`controller_index` 为 0–3）
pub const fn pwm_controller_base(controller_index: u8) -> Option<usize> {
    match controller_index {
        0 => Some(PWM0_BASE),
        1 => Some(PWM1_BASE),
        2 => Some(PWM2_BASE),
        3 => Some(PWM3_BASE),
        _ => None,
    }
}

/// 全局 PWM 通道标识符 (0-15)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GlobalPwmChannel {
    /// PWM[0] (PWM0 控制器)
    Pwm0 = 0,
    /// PWM[1] (PWM0 控制器)
    Pwm1 = 1,
    /// PWM[2] (PWM0 控制器)
    Pwm2 = 2,
    /// PWM[3] (PWM0 控制器)
    Pwm3 = 3,
    /// PWM[4] (PWM1 控制器)
    Pwm4 = 4,
    /// PWM[5] (PWM1 控制器)
    Pwm5 = 5,
    /// PWM[6] (PWM1 控制器)
    Pwm6 = 6,
    /// PWM[7] (PWM1 控制器)
    Pwm7 = 7,
    /// PWM[8] (PWM2 控制器)
    Pwm8 = 8,
    /// PWM[9] (PWM2 控制器)
    Pwm9 = 9,
    /// PWM[10] (PWM2 控制器)
    Pwm10 = 10,
    /// PWM[11] (PWM2 控制器)
    Pwm11 = 11,
    /// PWM[12] (PWM3 控制器)
    Pwm12 = 12,
    /// PWM[13] (PWM3 控制器)
    Pwm13 = 13,
    /// PWM[14] (PWM3 控制器)
    Pwm14 = 14,
    /// PWM[15] (PWM3 控制器)
    Pwm15 = 15,
}

impl GlobalPwmChannel {
    /// 从 u8 值转换
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pwm0),
            1 => Some(Self::Pwm1),
            2 => Some(Self::Pwm2),
            3 => Some(Self::Pwm3),
            4 => Some(Self::Pwm4),
            5 => Some(Self::Pwm5),
            6 => Some(Self::Pwm6),
            7 => Some(Self::Pwm7),
            8 => Some(Self::Pwm8),
            9 => Some(Self::Pwm9),
            10 => Some(Self::Pwm10),
            11 => Some(Self::Pwm11),
            12 => Some(Self::Pwm12),
            13 => Some(Self::Pwm13),
            14 => Some(Self::Pwm14),
            15 => Some(Self::Pwm15),
            _ => None,
        }
    }

    /// 获取全局通道号
    pub const fn index(&self) -> u8 {
        *self as u8
    }

    /// 所属控制器索引 (0–3)
    pub const fn controller_index(&self) -> u8 {
        (*self as u8) / 4
    }

    /// 所属控制器的 MMIO 基址
    pub const fn controller_base(&self) -> usize {
        match self.controller_index() {
            0 => PWM0_BASE,
            1 => PWM1_BASE,
            2 => PWM2_BASE,
            _ => PWM3_BASE,
        }
    }

    /// 控制器内的本地通道号 (0-3)
    pub const fn local_channel(&self) -> u8 {
        (*self as u8) % 4
    }
}
