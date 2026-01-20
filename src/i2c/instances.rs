//! # I2C 实例定义
//!
//! 本模块定义了 SG2002 芯片上所有 I2C 控制器实例的基地址。
//!
//! SG2002 芯片共有 6 个 I2C 控制器：
//! - I2C0-I2C4: 位于 Active Domain
//! - RTCSYS_I2C: 位于 No-die Domain (RTC 子系统)

/// I2C0 控制器基地址
pub const I2C0_BASE: usize = 0x0400_0000;

/// I2C1 控制器基地址
pub const I2C1_BASE: usize = 0x0401_0000;

/// I2C2 控制器基地址
pub const I2C2_BASE: usize = 0x0402_0000;

/// I2C3 控制器基地址
pub const I2C3_BASE: usize = 0x0403_0000;

/// I2C4 控制器基地址
pub const I2C4_BASE: usize = 0x0404_0000;

/// RTCSYS_I2C 控制器基地址 (No-die Domain)
pub const RTCSYS_I2C_BASE: usize = 0x0502_B000;

/// I2C 实例标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I2cInstance {
    /// I2C0 (Active Domain)
    I2c0 = 0,
    /// I2C1 (Active Domain)
    I2c1 = 1,
    /// I2C2 (Active Domain)
    I2c2 = 2,
    /// I2C3 (Active Domain)
    I2c3 = 3,
    /// I2C4 (Active Domain)
    I2c4 = 4,
    /// RTCSYS_I2C (No-die Domain)
    RtcsysI2c = 5,
}

impl I2cInstance {
    /// 获取 I2C 实例的基地址
    pub const fn base_address(&self) -> usize {
        match self {
            I2cInstance::I2c0 => I2C0_BASE,
            I2cInstance::I2c1 => I2C1_BASE,
            I2cInstance::I2c2 => I2C2_BASE,
            I2cInstance::I2c3 => I2C3_BASE,
            I2cInstance::I2c4 => I2C4_BASE,
            I2cInstance::RtcsysI2c => RTCSYS_I2C_BASE,
        }
    }

    /// 从索引创建 I2C 实例
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(I2cInstance::I2c0),
            1 => Some(I2cInstance::I2c1),
            2 => Some(I2cInstance::I2c2),
            3 => Some(I2cInstance::I2c3),
            4 => Some(I2cInstance::I2c4),
            5 => Some(I2cInstance::RtcsysI2c),
            _ => None,
        }
    }

    /// 获取实例索引
    pub const fn index(&self) -> u8 {
        *self as u8
    }

    /// 检查是否为 Active Domain 实例
    pub const fn is_active_domain(&self) -> bool {
        !matches!(self, I2cInstance::RtcsysI2c)
    }

    /// 检查是否为 RTC 子系统实例
    pub const fn is_rtc_domain(&self) -> bool {
        matches!(self, I2cInstance::RtcsysI2c)
    }
}

/// I2C 时钟配置
///
/// 根据 IIC_CLK 频率配置 SCL 时序参数
#[derive(Debug, Clone, Copy)]
pub struct I2cClockConfig {
    /// 标准模式 SCL 高电平计数
    pub ss_scl_hcnt: u16,
    /// 标准模式 SCL 低电平计数
    pub ss_scl_lcnt: u16,
    /// 快速模式 SCL 高电平计数
    pub fs_scl_hcnt: u16,
    /// 快速模式 SCL 低电平计数
    pub fs_scl_lcnt: u16,
    /// SDA 保持时间
    pub sda_hold: u16,
    /// SDA 建立时间
    pub sda_setup: u8,
    /// 毛刺抑制长度
    pub fs_spklen: u8,
}

impl I2cClockConfig {
    /// 25MHz IIC_CLK 时钟配置
    pub const CLK_25MHZ: Self = Self {
        ss_scl_hcnt: 115,
        ss_scl_lcnt: 135,
        fs_scl_hcnt: 21,
        fs_scl_lcnt: 42,
        sda_hold: 1,
        sda_setup: 6,
        fs_spklen: 2,
    };

    /// 100MHz IIC_CLK 时钟配置
    pub const CLK_100MHZ: Self = Self {
        ss_scl_hcnt: 460,
        ss_scl_lcnt: 540,
        fs_scl_hcnt: 90,
        fs_scl_lcnt: 160,
        sda_hold: 1,
        sda_setup: 25,
        fs_spklen: 5,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2c_base_addresses() {
        assert_eq!(I2cInstance::I2c0.base_address(), 0x0400_0000);
        assert_eq!(I2cInstance::I2c1.base_address(), 0x0401_0000);
        assert_eq!(I2cInstance::I2c2.base_address(), 0x0402_0000);
        assert_eq!(I2cInstance::I2c3.base_address(), 0x0403_0000);
        assert_eq!(I2cInstance::I2c4.base_address(), 0x0404_0000);
        assert_eq!(I2cInstance::RtcsysI2c.base_address(), 0x0502_B000);
    }

    #[test]
    fn test_i2c_domain() {
        assert!(I2cInstance::I2c0.is_active_domain());
        assert!(I2cInstance::I2c4.is_active_domain());
        assert!(!I2cInstance::RtcsysI2c.is_active_domain());
        assert!(I2cInstance::RtcsysI2c.is_rtc_domain());
    }
}
