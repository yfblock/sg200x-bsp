//! # PWM 实例定义
//!
//! 本模块定义了 SG2002 芯片上所有 PWM 控制器实例的基地址。
//!
//! SG2002 芯片共有 4 个 PWM 控制器：
//! - PWM0: 包含 PWM[0], PWM[1], PWM[2], PWM[3]
//! - PWM1: 包含 PWM[4], PWM[5], PWM[6], PWM[7]
//! - PWM2: 包含 PWM[8], PWM[9], PWM[10], PWM[11]
//! - PWM3: 包含 PWM[12], PWM[13], PWM[14], PWM[15]

/// PWM0 控制器基地址
pub const PWM0_BASE: usize = 0x0306_0000;

/// PWM1 控制器基地址
pub const PWM1_BASE: usize = 0x0306_1000;

/// PWM2 控制器基地址
pub const PWM2_BASE: usize = 0x0306_2000;

/// PWM3 控制器基地址
pub const PWM3_BASE: usize = 0x0306_3000;

/// PWM 控制器实例标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PwmInstance {
    /// PWM0 控制器 (PWM[0]-PWM[3])
    Pwm0 = 0,
    /// PWM1 控制器 (PWM[4]-PWM[7])
    Pwm1 = 1,
    /// PWM2 控制器 (PWM[8]-PWM[11])
    Pwm2 = 2,
    /// PWM3 控制器 (PWM[12]-PWM[15])
    Pwm3 = 3,
}

impl PwmInstance {
    /// 获取 PWM 控制器的基地址
    pub const fn base_address(&self) -> usize {
        match self {
            PwmInstance::Pwm0 => PWM0_BASE,
            PwmInstance::Pwm1 => PWM1_BASE,
            PwmInstance::Pwm2 => PWM2_BASE,
            PwmInstance::Pwm3 => PWM3_BASE,
        }
    }

    /// 从索引创建 PWM 实例
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(PwmInstance::Pwm0),
            1 => Some(PwmInstance::Pwm1),
            2 => Some(PwmInstance::Pwm2),
            3 => Some(PwmInstance::Pwm3),
            _ => None,
        }
    }

    /// 获取实例索引
    pub const fn index(&self) -> u8 {
        *self as u8
    }

    /// 获取此控制器的全局 PWM 通道起始编号
    /// PWM0: 0-3, PWM1: 4-7, PWM2: 8-11, PWM3: 12-15
    pub const fn channel_offset(&self) -> u8 {
        (*self as u8) * 4
    }

    /// 从全局 PWM 通道号获取对应的控制器实例
    pub const fn from_global_channel(channel: u8) -> Option<Self> {
        match channel {
            0..=3 => Some(PwmInstance::Pwm0),
            4..=7 => Some(PwmInstance::Pwm1),
            8..=11 => Some(PwmInstance::Pwm2),
            12..=15 => Some(PwmInstance::Pwm3),
            _ => None,
        }
    }

    /// 从全局 PWM 通道号获取控制器内的本地通道号
    pub const fn local_channel(global_channel: u8) -> Option<u8> {
        if global_channel < 16 {
            Some(global_channel % 4)
        } else {
            None
        }
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

    /// 获取所属的 PWM 控制器实例
    pub const fn instance(&self) -> PwmInstance {
        match *self as u8 {
            0..=3 => PwmInstance::Pwm0,
            4..=7 => PwmInstance::Pwm1,
            8..=11 => PwmInstance::Pwm2,
            _ => PwmInstance::Pwm3,
        }
    }

    /// 获取控制器内的本地通道号 (0-3)
    pub const fn local_channel(&self) -> u8 {
        (*self as u8) % 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pwm_base_addresses() {
        assert_eq!(PwmInstance::Pwm0.base_address(), 0x0306_0000);
        assert_eq!(PwmInstance::Pwm1.base_address(), 0x0306_1000);
        assert_eq!(PwmInstance::Pwm2.base_address(), 0x0306_2000);
        assert_eq!(PwmInstance::Pwm3.base_address(), 0x0306_3000);
    }

    #[test]
    fn test_pwm_channel_offset() {
        assert_eq!(PwmInstance::Pwm0.channel_offset(), 0);
        assert_eq!(PwmInstance::Pwm1.channel_offset(), 4);
        assert_eq!(PwmInstance::Pwm2.channel_offset(), 8);
        assert_eq!(PwmInstance::Pwm3.channel_offset(), 12);
    }

    #[test]
    fn test_global_channel() {
        assert_eq!(GlobalPwmChannel::Pwm0.instance(), PwmInstance::Pwm0);
        assert_eq!(GlobalPwmChannel::Pwm5.instance(), PwmInstance::Pwm1);
        assert_eq!(GlobalPwmChannel::Pwm10.instance(), PwmInstance::Pwm2);
        assert_eq!(GlobalPwmChannel::Pwm15.instance(), PwmInstance::Pwm3);

        assert_eq!(GlobalPwmChannel::Pwm0.local_channel(), 0);
        assert_eq!(GlobalPwmChannel::Pwm5.local_channel(), 1);
        assert_eq!(GlobalPwmChannel::Pwm10.local_channel(), 2);
        assert_eq!(GlobalPwmChannel::Pwm15.local_channel(), 3);
    }
}
