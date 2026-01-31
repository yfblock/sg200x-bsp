//! VIP 系统控制

use super::regs::*;

/// VIP 系统寄存器基地址
pub const VIP_SYS_BASE: usize = 0x0A0C_8000;

/// VIP 系统时钟基地址
pub const CV181X_SYS_CLOCK_BASE: usize = 0x0300_2000;

/// VIP 系统寄存器偏移
pub const VIP_SYS_VIP_RESETS: usize = 0x0;
pub const VIP_SYS_VIP_CLK_CTRL1: usize = 0x1c;
pub const REG_DIV_CLK_SRC_VIP_SYS_2: usize = 0x110;

/// VIP 系统复位位偏移
pub const VIP_SYS_REG_RST_CSI_MAC0_OFFSET: u32 = 12;
pub const VIP_SYS_REG_RST_CSI_MAC1_OFFSET: u32 = 13;
pub const VIP_SYS_REG_RST_CSI_MAC2_OFFSET: u32 = 16;

/// MAC 时钟控制偏移
pub const MAC0_CLK_CTRL1_OFFSET: u32 = 4;
pub const MAC1_CLK_CTRL1_OFFSET: u32 = 8;
pub const MAC2_CLK_CTRL1_OFFSET: u32 = 30;
pub const MAC_CLK_CTRL1_MASK: u32 = 0x3;

/// VIP_SYS_2 时钟源
pub const VIP_SYS_2_SRC_DISPPLL: u32 = 2 << 8; // 1188MHz
pub const VIP_SYS_2_SRC_FPLL: u32 = 3 << 8; // 1500MHz
pub const VIP_SYS_2_SRC_MASK: u32 = 3 << 8;

/// CAM PLL 寄存器字段
pub const REG_CAM_DIV_DIS: u32 = 4;
pub const REG_CAM_SRC: u32 = 8;
pub const REG_CAM_SRC_MASK: u32 = 3 << REG_CAM_SRC;
pub const REG_CAM_DIV: u32 = 16;
pub const REG_CAM_DIV_MASK: u32 = 0x3F << REG_CAM_DIV;

/// CAM PLL 配置
#[derive(Debug, Clone, Copy)]
pub struct CamPllSetting {
    pub cam_src: u8,
    pub clk_div: u8,
}

impl CamPllSetting {
    pub const FREQ_37P125M: Self = Self {
        cam_src: 0,
        clk_div: 32,
    };

    pub const FREQ_27M: Self = Self {
        cam_src: 0,
        clk_div: 44,
    };

    pub const FREQ_26M: Self = Self {
        cam_src: 0,
        clk_div: 46,
    };

    pub const FREQ_25M: Self = Self {
        cam_src: 3,
        clk_div: 12,
    };

    pub const FREQ_24M: Self = Self {
        cam_src: 2,
        clk_div: 33,
    };
}

/// VIP 系统寄存器写入（带掩码）
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn vip_sys_reg_write_mask(offset: usize, mask: u32, data: u32) {
    reg_modify(VIP_SYS_BASE + offset, mask, data);
}

/// 切换 VIP 复位
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn vip_toggle_reset(mask: u32) {
    let reset_base = VIP_SYS_BASE + VIP_SYS_VIP_RESETS;

    // 置位复位
    reg_setbits(reset_base, mask);

    // 延时 20us
    for _ in 0..2000 {
        core::hint::spin_loop();
    }

    // 清除复位
    reg_clrbits(reset_base, mask);
}

/// 设置 VIP_SYS_2 时钟分频
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn set_vip_sys_2_clk_div(src: u32, div: u32) {
    let value = if src == VIP_SYS_2_SRC_DISPPLL {
        (div << 16) | 0x209
    } else {
        (div << 16) | 0x309
    };
    reg_write(CV181X_SYS_CLOCK_BASE + REG_DIV_CLK_SRC_VIP_SYS_2, value);
}

/// 配置 MAC 时钟
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn set_mac_clk(devno: u32, mac_clk: super::RxMacClk, max_mac_clk: u32) -> Result<(), ()> {
    let clk_val = (mac_clk as u32 + 99) / 100;

    // 选择 vip_sys2 源
    let (mask, data) = match devno {
        0 => (
            MAC_CLK_CTRL1_MASK << MAC0_CLK_CTRL1_OFFSET,
            0x2 << MAC0_CLK_CTRL1_OFFSET,
        ),
        1 => (
            MAC_CLK_CTRL1_MASK << MAC1_CLK_CTRL1_OFFSET,
            0x2 << MAC1_CLK_CTRL1_OFFSET,
        ),
        2 => (
            MAC_CLK_CTRL1_MASK << MAC2_CLK_CTRL1_OFFSET,
            0x2 << MAC2_CLK_CTRL1_OFFSET,
        ),
        _ => return Err(()),
    };

    vip_sys_reg_write_mask(VIP_SYS_VIP_CLK_CTRL1, mask, data);

    // 设置时钟分频
    match mac_clk {
        super::RxMacClk::Clk200M => {
            set_vip_sys_2_clk_div(VIP_SYS_2_SRC_DISPPLL, 6);
        }
        super::RxMacClk::Clk300M => {
            set_vip_sys_2_clk_div(VIP_SYS_2_SRC_DISPPLL, 4);
        }
        super::RxMacClk::Clk400M => {
            set_vip_sys_2_clk_div(VIP_SYS_2_SRC_DISPPLL, 3);
        }
        super::RxMacClk::Clk500M => {
            set_vip_sys_2_clk_div(VIP_SYS_2_SRC_FPLL, 3);
        }
        super::RxMacClk::Clk600M => {
            set_vip_sys_2_clk_div(VIP_SYS_2_SRC_DISPPLL, 2);
        }
    }

    // 延时 5us
    for _ in 0..500 {
        core::hint::spin_loop();
    }

    Ok(())
}

/// 使能传感器时钟
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn enable_sensor_clock(
    devno: u32,
    freq: super::CamPllFreq,
    enable: bool,
) -> Result<(), ()> {
    if freq == super::CamPllFreq::None {
        return Ok(());
    }

    let setting = match freq {
        super::CamPllFreq::Freq37P125M => CamPllSetting::FREQ_37P125M,
        super::CamPllFreq::Freq24M => CamPllSetting::FREQ_24M,
        super::CamPllFreq::Freq26M => CamPllSetting::FREQ_26M,
        super::CamPllFreq::Freq25M => CamPllSetting::FREQ_25M,
        super::CamPllFreq::Freq27M => CamPllSetting::FREQ_27M,
        _ => CamPllSetting::FREQ_27M,
    };

    if enable {
        // 配置 CAM0 时钟
        reg_modify(
            CLK_CAM0_SRC_DIV,
            REG_CAM_SRC_MASK,
            (setting.cam_src as u32) << REG_CAM_SRC,
        );
        reg_modify(
            CLK_CAM0_SRC_DIV,
            REG_CAM_DIV_MASK,
            (setting.clk_div as u32) << REG_CAM_DIV,
        );

        // 配置 CAM1 时钟
        reg_modify(
            CLK_CAM1_SRC_DIV,
            REG_CAM_SRC_MASK,
            (setting.cam_src as u32) << REG_CAM_SRC,
        );
        reg_modify(
            CLK_CAM1_SRC_DIV,
            REG_CAM_DIV_MASK,
            (setting.clk_div as u32) << REG_CAM_DIV,
        );

        // 延时 100us
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
    }

    Ok(())
}

/// 复位 MIPI
///
/// # Safety
/// 调用者必须确保地址有效
pub unsafe fn reset_mipi(devno: u32) {
    match devno {
        0 => {
            // 复位 PHY/MAC
            reg_clrbits(0x0300_3008, (1 << 6) | (1 << 7));
            for _ in 0..500 {
                core::hint::spin_loop();
            }
            reg_setbits(0x0300_3008, (1 << 6) | (1 << 7));

            // 软件复位 MAC
            vip_toggle_reset(1 << VIP_SYS_REG_RST_CSI_MAC0_OFFSET);
        }
        1 => {
            // 复位 PHY/MAC
            reg_clrbits(0x0300_3008, (1 << 8) | (1 << 9));
            for _ in 0..500 {
                core::hint::spin_loop();
            }
            reg_setbits(0x0300_3008, (1 << 8) | (1 << 9));

            // 软件复位 MAC
            vip_toggle_reset(1 << VIP_SYS_REG_RST_CSI_MAC1_OFFSET);
        }
        2 => {
            // 复位 PHY/MAC
            reg_clrbits(0x0300_3008, (1 << 8) | (1 << 9));
            for _ in 0..500 {
                core::hint::spin_loop();
            }
            reg_setbits(0x0300_3008, (1 << 8) | (1 << 9));

            // 软件复位 MAC
            vip_toggle_reset(1 << VIP_SYS_REG_RST_CSI_MAC2_OFFSET);
        }
        _ => {}
    }
}
