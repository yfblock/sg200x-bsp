//! IOBLK (IO Block) 寄存器定义
//!
//! IOBLK 寄存器用于配置每个引脚的电气特性，包括:
//! - 上拉/下拉电阻 (PU/PD)
//! - 驱动强度 (DS0/DS1/DS2)
//! - 施密特触发器 (ST0/ST1)
//! - 总线保持器 (HE)
//! - 转换速率限制 (SL)
//!
//! 不同 IO 组的寄存器基地址:
//! - G1:   0x0300_1800
//! - G7:   0x0300_1900
//! - G10:  0x0300_1A00
//! - G12:  0x0300_1C00
//! - GRTC: 0x0502_7000

use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::ReadWrite,
};

// ============================================================================
// IOBLK 寄存器位域定义
// ============================================================================

register_bitfields! [
    u32,

    /// IOBLK 通用寄存器位域
    ///
    /// 适用于大多数 IO 引脚的配置寄存器
    pub IOBLK_REG [
        /// 上拉电阻使能
        /// - 0: 禁用上拉
        /// - 1: 使能上拉
        PU OFFSET(2) NUMBITS(1) [],

        /// 下拉电阻使能
        /// - 0: 禁用下拉
        /// - 1: 使能下拉
        PD OFFSET(3) NUMBITS(1) [],

        /// 输出驱动能力档位 bit 0
        DS0 OFFSET(5) NUMBITS(1) [],

        /// 输出驱动能力档位 bit 1
        DS1 OFFSET(6) NUMBITS(1) [],

        /// 输出驱动能力档位 bit 2 (部分引脚支持)
        DS2 OFFSET(7) NUMBITS(1) [],

        /// 输入施密特触发器强度控制 bit 0
        ST0 OFFSET(8) NUMBITS(1) [],

        /// 输入施密特触发器强度控制 bit 1
        ST1 OFFSET(9) NUMBITS(1) [],

        /// 弱电平维持器(Bus holder)使能
        /// - 0: 禁用
        /// - 1: 使能
        HE OFFSET(10) NUMBITS(1) [],

        /// 输出电平转换速率限制
        /// - 0: 禁用(较快)
        /// - 1: 使能(较慢)
        SL OFFSET(11) NUMBITS(1) []
    ],

    /// IOBLK 驱动强度字段 (2位)
    pub IOBLK_DS2 [
        /// 驱动强度 (2位: DS1:DS0)
        /// - 0b00: 最弱
        /// - 0b01: 较弱
        /// - 0b10: 较强 (默认)
        /// - 0b11: 最强
        DS OFFSET(5) NUMBITS(2) []
    ],

    /// IOBLK 驱动强度字段 (3位)
    pub IOBLK_DS3 [
        /// 驱动强度 (3位: DS2:DS1:DS0)
        /// - 0b000 ~ 0b111: 8级驱动强度
        DS OFFSET(5) NUMBITS(3) []
    ],

    /// IOBLK 施密特触发器字段
    pub IOBLK_ST [
        /// 施密特触发器等级 (2位: ST1:ST0)
        /// - 0b00: 禁用
        /// - 0b01 ~ 0b11: 不同触发等级
        ST OFFSET(8) NUMBITS(2) []
    ]
];

// ============================================================================
// IOBLK G1 组寄存器 (基地址: 0x0300_1800)
// ============================================================================

register_structs! {
    /// IOBLK G1 组寄存器
    ///
    /// 包含 PWM0_BUCK, ADC1, USB_VBUS_DET, PKG_TYPE0/1/2 等引脚的配置
    pub IoblkG1Registers {
        /// 保留 (偏移 0x00)
        (0x000 => _reserved0),

        /// PWM0_BUCK IO 配置寄存器 (偏移 0x04)
        ///
        /// 位域:
        /// - [2]: PU - 上拉使能
        /// - [3]: PD - 下拉使能
        /// - [5]: DS0 - 驱动强度 bit0
        /// - [6]: DS1 - 驱动强度 bit1
        /// - [8]: ST0 - 施密特触发器 bit0
        /// - [9]: ST1 - 施密特触发器 bit1
        /// - [10]: HE - 总线保持器使能
        /// - [11]: SL - 转换速率限制
        (0x004 => pub pwm0_buck: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x08-0x0C)
        (0x008 => _reserved1),

        /// ADC1 IO 配置寄存器 (偏移 0x10)
        (0x010 => pub adc1: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x14-0x18)
        (0x014 => _reserved2),

        /// PKG_TYPE0 IO 配置寄存器 (偏移 0x1C)
        (0x01C => pub pkg_type0: ReadWrite<u32, IOBLK_REG::Register>),

        /// USB_VBUS_DET IO 配置寄存器 (偏移 0x20)
        (0x020 => pub usb_vbus_det: ReadWrite<u32, IOBLK_REG::Register>),

        /// PKG_TYPE1 IO 配置寄存器 (偏移 0x24)
        (0x024 => pub pkg_type1: ReadWrite<u32, IOBLK_REG::Register>),

        /// PKG_TYPE2 IO 配置寄存器 (偏移 0x28)
        (0x028 => pub pkg_type2: ReadWrite<u32, IOBLK_REG::Register>),

        /// 结束标记
        (0x02C => @END),
    }
}

// ============================================================================
// IOBLK G7 组寄存器 (基地址: 0x0300_1900)
// ============================================================================

register_structs! {
    /// IOBLK G7 组寄存器
    ///
    /// 包含 SD0_CD, SD0_PWR_EN, SPK_EN, UART0, EMMC, JTAG, IIC0, AUX0 等引脚的配置
    pub IoblkG7Registers {
        /// SD0_CD IO 配置寄存器 (偏移 0x00)
        ///
        /// 位域:
        /// - [2]: PU - 上拉使能 (默认: 1)
        /// - [3]: PD - 下拉使能
        /// - [5]: DS0 - 驱动强度 bit0
        /// - [6]: DS1 - 驱动强度 bit1
        /// - [7]: DS2 - 驱动强度 bit2
        /// - [8]: ST0 - 施密特触发器
        /// - [11]: SL - 转换速率限制
        (0x000 => pub sd0_cd: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_PWR_EN IO 配置寄存器 (偏移 0x04)
        (0x004 => pub sd0_pwr_en: ReadWrite<u32, IOBLK_REG::Register>),

        /// SPK_EN IO 配置寄存器 (偏移 0x08)
        (0x008 => pub spk_en: ReadWrite<u32, IOBLK_REG::Register>),

        /// UART0_TX IO 配置寄存器 (偏移 0x0C)
        ///
        /// 默认配置: 上拉使能
        (0x00C => pub uart0_tx: ReadWrite<u32, IOBLK_REG::Register>),

        /// UART0_RX IO 配置寄存器 (偏移 0x10)
        ///
        /// 默认配置: 上拉使能
        (0x010 => pub uart0_rx: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x14)
        (0x014 => _reserved0),

        /// EMMC_DAT2 IO 配置寄存器 (偏移 0x18)
        (0x018 => pub emmc_dat2: ReadWrite<u32, IOBLK_REG::Register>),

        /// EMMC_CLK IO 配置寄存器 (偏移 0x1C)
        (0x01C => pub emmc_clk: ReadWrite<u32, IOBLK_REG::Register>),

        /// EMMC_DAT0 IO 配置寄存器 (偏移 0x20)
        (0x020 => pub emmc_dat0: ReadWrite<u32, IOBLK_REG::Register>),

        /// EMMC_DAT3 IO 配置寄存器 (偏移 0x24)
        (0x024 => pub emmc_dat3: ReadWrite<u32, IOBLK_REG::Register>),

        /// EMMC_CMD IO 配置寄存器 (偏移 0x28)
        (0x028 => pub emmc_cmd: ReadWrite<u32, IOBLK_REG::Register>),

        /// EMMC_DAT1 IO 配置寄存器 (偏移 0x2C)
        (0x02C => pub emmc_dat1: ReadWrite<u32, IOBLK_REG::Register>),

        /// JTAG_CPU_TMS IO 配置寄存器 (偏移 0x30)
        (0x030 => pub jtag_cpu_tms: ReadWrite<u32, IOBLK_REG::Register>),

        /// JTAG_CPU_TCK IO 配置寄存器 (偏移 0x34)
        (0x034 => pub jtag_cpu_tck: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x38)
        (0x038 => _reserved1),

        /// IIC0_SCL IO 配置寄存器 (偏移 0x3C)
        (0x03C => pub iic0_scl: ReadWrite<u32, IOBLK_REG::Register>),

        /// IIC0_SDA IO 配置寄存器 (偏移 0x40)
        (0x040 => pub iic0_sda: ReadWrite<u32, IOBLK_REG::Register>),

        /// AUX0 IO 配置寄存器 (偏移 0x44)
        (0x044 => pub aux0: ReadWrite<u32, IOBLK_REG::Register>),

        /// 结束标记
        (0x048 => @END),
    }
}

// ============================================================================
// IOBLK G10 组寄存器 (基地址: 0x0300_1A00)
// ============================================================================

register_structs! {
    /// IOBLK G10 组寄存器
    ///
    /// 包含 SD0_CLK, SD0_CMD, SD0_D0~D3 等引脚的配置
    /// 这些引脚属于 VDDIO_SD0_EMMC 电源域
    pub IoblkG10Registers {
        /// SD0_CLK IO 配置寄存器 (偏移 0x00)
        ///
        /// 位域:
        /// - [2]: PU - 上拉使能
        /// - [3]: PD - 下拉使能 (默认: 1)
        /// - [5]: DS0 - 驱动强度 bit0
        /// - [6]: DS1 - 驱动强度 bit1
        /// - [7]: DS2 - 驱动强度 bit2
        /// - [8]: ST0 - 施密特触发器
        /// - [11]: SL - 转换速率限制
        (0x000 => pub sd0_clk: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_CMD IO 配置寄存器 (偏移 0x04)
        (0x004 => pub sd0_cmd: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_D0 IO 配置寄存器 (偏移 0x08)
        (0x008 => pub sd0_d0: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_D1 IO 配置寄存器 (偏移 0x0C)
        (0x00C => pub sd0_d1: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_D2 IO 配置寄存器 (偏移 0x10)
        (0x010 => pub sd0_d2: ReadWrite<u32, IOBLK_REG::Register>),

        /// SD0_D3 IO 配置寄存器 (偏移 0x14)
        (0x014 => pub sd0_d3: ReadWrite<u32, IOBLK_REG::Register>),

        /// 结束标记
        (0x018 => @END),
    }
}

// ============================================================================
// IOBLK G12 组寄存器 (基地址: 0x0300_1C00)
// ============================================================================

register_structs! {
    /// IOBLK G12 组寄存器
    ///
    /// 包含 MIPI RX/TX, GPIO_RTX 等引脚的配置
    /// 这些引脚属于 VDD18A_MIPI 电源域
    pub IoblkG12Registers {
        /// 保留 (偏移 0x00-0x34)
        (0x000 => _reserved0),

        /// PAD_MIPIRX4N IO 配置寄存器 (偏移 0x38)
        (0x038 => pub pad_mipirx4n: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX4P IO 配置寄存器 (偏移 0x3C)
        (0x03C => pub pad_mipirx4p: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX3N IO 配置寄存器 (偏移 0x40)
        (0x040 => pub pad_mipirx3n: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX3P IO 配置寄存器 (偏移 0x44)
        (0x044 => pub pad_mipirx3p: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX2N IO 配置寄存器 (偏移 0x48)
        (0x048 => pub pad_mipirx2n: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX2P IO 配置寄存器 (偏移 0x4C)
        (0x04C => pub pad_mipirx2p: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX1N IO 配置寄存器 (偏移 0x50)
        (0x050 => pub pad_mipirx1n: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX1P IO 配置寄存器 (偏移 0x54)
        (0x054 => pub pad_mipirx1p: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX0N IO 配置寄存器 (偏移 0x58)
        (0x058 => pub pad_mipirx0n: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPIRX0P IO 配置寄存器 (偏移 0x5C)
        (0x05C => pub pad_mipirx0p: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x60-0x6C)
        (0x060 => _reserved1),

        /// PAD_MIPI_TXM2 IO 配置寄存器 (偏移 0x70)
        (0x070 => pub pad_mipi_txm2: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPI_TXP2 IO 配置寄存器 (偏移 0x74)
        (0x074 => pub pad_mipi_txp2: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPI_TXM1 IO 配置寄存器 (偏移 0x78)
        (0x078 => pub pad_mipi_txm1: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPI_TXP1 IO 配置寄存器 (偏移 0x7C)
        (0x07C => pub pad_mipi_txp1: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPI_TXM0 IO 配置寄存器 (偏移 0x80)
        (0x080 => pub pad_mipi_txm0: ReadWrite<u32, IOBLK_REG::Register>),

        /// PAD_MIPI_TXP0 IO 配置寄存器 (偏移 0x84)
        (0x084 => pub pad_mipi_txp0: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x88)
        (0x088 => _reserved2),

        /// GPIO_RTX IO 配置寄存器 (偏移 0x8C)
        (0x08C => pub gpio_rtx: ReadWrite<u32, IOBLK_REG::Register>),

        /// 结束标记
        (0x090 => @END),
    }
}

// ============================================================================
// IOBLK GRTC 组寄存器 (基地址: 0x0502_7000)
// ============================================================================

register_structs! {
    /// IOBLK GRTC 组寄存器
    ///
    /// 包含 PWR_VBAT_DET, PWR_RSTN, PWR_SEQ1/2, PWR_WAKEUP0,
    /// PWR_BUTTON1, XTAL_XIN, PWR_GPIO0/1/2, GPIO_ZQ 等引脚的配置
    /// 这些引脚属于 VDDIO_RTC 电源域 (1.8V)
    pub IoblkGrtcRegisters {
        /// PWR_VBAT_DET IO 配置寄存器 (偏移 0x00)
        (0x000 => pub pwr_vbat_det: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_RSTN IO 配置寄存器 (偏移 0x04)
        (0x004 => pub pwr_rstn: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_SEQ1 IO 配置寄存器 (偏移 0x08)
        (0x008 => pub pwr_seq1: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_SEQ2 IO 配置寄存器 (偏移 0x0C)
        (0x00C => pub pwr_seq2: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x10-0x14)
        (0x010 => _reserved0),

        /// PWR_WAKEUP0 IO 配置寄存器 (偏移 0x18)
        (0x018 => pub pwr_wakeup0: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x1C)
        (0x01C => _reserved1),

        /// PWR_BUTTON1 IO 配置寄存器 (偏移 0x20)
        (0x020 => pub pwr_button1: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x24)
        (0x024 => _reserved2),

        /// XTAL_XIN IO 配置寄存器 (偏移 0x28)
        (0x028 => pub xtal_xin: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_GPIO0 IO 配置寄存器 (偏移 0x2C)
        (0x02C => pub pwr_gpio0: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_GPIO1 IO 配置寄存器 (偏移 0x30)
        (0x030 => pub pwr_gpio1: ReadWrite<u32, IOBLK_REG::Register>),

        /// PWR_GPIO2 IO 配置寄存器 (偏移 0x34)
        (0x034 => pub pwr_gpio2: ReadWrite<u32, IOBLK_REG::Register>),

        /// 保留 (偏移 0x38-0xDC)
        (0x038 => _reserved3),

        /// GPIO_ZQ IO 配置寄存器 (偏移 0xE0)
        (0x0E0 => pub gpio_zq: ReadWrite<u32, IOBLK_REG::Register>),

        /// 结束标记
        (0x0E4 => @END),
    }
}

// ============================================================================
// 辅助函数和 trait 实现
// ============================================================================

/// IO 配置 trait
///
/// 为所有 IOBLK 寄存器提供统一的配置接口
pub trait IoConfig {
    /// 设置上拉使能
    fn set_pull_up(&self, enable: bool);

    /// 设置下拉使能
    fn set_pull_down(&self, enable: bool);

    /// 设置驱动强度 (0-7)
    fn set_drive_strength(&self, strength: u8);

    /// 获取驱动强度
    fn get_drive_strength(&self) -> u8;

    /// 设置施密特触发器等级 (0-3)
    fn set_schmitt_trigger(&self, level: u8);

    /// 设置总线保持器使能
    fn set_bus_holder(&self, enable: bool);

    /// 设置转换速率限制
    fn set_slew_rate_limit(&self, enable: bool);
}

impl IoConfig for ReadWrite<u32, IOBLK_REG::Register> {
    fn set_pull_up(&self, enable: bool) {
        if enable {
            self.modify(IOBLK_REG::PU::SET);
        } else {
            self.modify(IOBLK_REG::PU::CLEAR);
        }
    }

    fn set_pull_down(&self, enable: bool) {
        if enable {
            self.modify(IOBLK_REG::PD::SET);
        } else {
            self.modify(IOBLK_REG::PD::CLEAR);
        }
    }

    fn set_drive_strength(&self, strength: u8) {
        let ds0 = (strength & 0x1) != 0;
        let ds1 = (strength & 0x2) != 0;
        let ds2 = (strength & 0x4) != 0;

        let mut val = self.get();
        if ds0 {
            val |= 1 << 5;
        } else {
            val &= !(1 << 5);
        }
        if ds1 {
            val |= 1 << 6;
        } else {
            val &= !(1 << 6);
        }
        if ds2 {
            val |= 1 << 7;
        } else {
            val &= !(1 << 7);
        }
        self.set(val);
    }

    fn get_drive_strength(&self) -> u8 {
        let val = self.get();
        let ds0 = ((val >> 5) & 1) as u8;
        let ds1 = ((val >> 6) & 1) as u8;
        let ds2 = ((val >> 7) & 1) as u8;
        ds0 | (ds1 << 1) | (ds2 << 2)
    }

    fn set_schmitt_trigger(&self, level: u8) {
        let st0 = (level & 0x1) != 0;
        let st1 = (level & 0x2) != 0;

        let mut val = self.get();
        if st0 {
            val |= 1 << 8;
        } else {
            val &= !(1 << 8);
        }
        if st1 {
            val |= 1 << 9;
        } else {
            val &= !(1 << 9);
        }
        self.set(val);
    }

    fn set_bus_holder(&self, enable: bool) {
        if enable {
            self.modify(IOBLK_REG::HE::SET);
        } else {
            self.modify(IOBLK_REG::HE::CLEAR);
        }
    }

    fn set_slew_rate_limit(&self, enable: bool) {
        if enable {
            self.modify(IOBLK_REG::SL::SET);
        } else {
            self.modify(IOBLK_REG::SL::CLEAR);
        }
    }
}
