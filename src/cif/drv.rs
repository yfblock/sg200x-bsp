//! CIF 底层驱动函数
//!
//! 本模块提供与 C 代码 `cif_drv.c` 对应的底层驱动函数。

use super::regs::*;
use super::types::*;

/// CIF MAC 寄存器块 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifMacBlkId {
    Top = 0,
    Slvds,
    Csi,
    Max,
}

/// CIF Wrap 寄存器块 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifWrapBlkId {
    Top = 0,
    Lane4,
    Lane2,
    Max,
}

/// MAC 寄存器块基地址偏移
pub const CIF_MAC_BLK_BA_TOP: usize = 0x0000;
pub const CIF_MAC_BLK_BA_SLVDS: usize = 0x0200;
pub const CIF_MAC_BLK_BA_CSI: usize = 0x0400;

/// Wrap 寄存器块基地址偏移
pub const CIF_WRAP_BLK_BA_TOP: usize = 0x0000;
pub const CIF_WRAP_BLK_BA_4L: usize = 0x0300;
pub const CIF_WRAP_BLK_BA_2L: usize = 0x0600;

/// CIF 驱动上下文
pub struct CifDrvCtx {
    /// MAC 寄存器基地址数组
    pub mac_regs: [usize; 3],
    /// Wrap 寄存器基地址数组
    pub wrap_regs: [usize; 3],
    /// MAC 编号
    pub mac_num: u16,
}

impl CifDrvCtx {
    /// 创建新的驱动上下文
    ///
    /// # Safety
    /// 调用者必须确保基地址有效
    pub unsafe fn new(mac_num: u16) -> Self {
        let mac_base = match mac_num {
            0 => SENSOR_MAC0_BASE,
            1 => SENSOR_MAC1_BASE,
            _ => SENSOR_MAC_VI_BASE,
        };

        Self {
            mac_regs: [
                mac_base + CIF_MAC_BLK_BA_TOP,
                mac_base + CIF_MAC_BLK_BA_SLVDS,
                mac_base + CIF_MAC_BLK_BA_CSI,
            ],
            wrap_regs: [
                DPHY_TOP_BASE + CIF_WRAP_BLK_BA_TOP,
                DPHY_TOP_BASE + CIF_WRAP_BLK_BA_4L,
                DPHY_TOP_BASE + CIF_WRAP_BLK_BA_2L,
            ],
            mac_num,
        }
    }

    /// 获取 MAC TOP 寄存器基地址
    pub fn mac_top(&self) -> usize {
        self.mac_regs[CifMacBlkId::Top as usize]
    }

    /// 获取 MAC SLVDS 寄存器基地址
    pub fn mac_slvds(&self) -> usize {
        self.mac_regs[CifMacBlkId::Slvds as usize]
    }

    /// 获取 MAC CSI 寄存器基地址
    pub fn mac_csi(&self) -> usize {
        self.mac_regs[CifMacBlkId::Csi as usize]
    }

    /// 获取 Wrap TOP 寄存器基地址
    pub fn wrap_top(&self) -> usize {
        self.wrap_regs[CifWrapBlkId::Top as usize]
    }

    /// 获取 Wrap 4L 寄存器基地址
    pub fn wrap_4l(&self) -> usize {
        self.wrap_regs[CifWrapBlkId::Lane4 as usize]
    }

    /// 获取 Wrap 2L 寄存器基地址
    pub fn wrap_2l(&self) -> usize {
        self.wrap_regs[CifWrapBlkId::Lane2 as usize]
    }
}

// ============================================================================
// MAC TOP 寄存器偏移
// ============================================================================
pub const REG_SENSOR_MAC_MODE: usize = 0x00;
pub const REG_SENSOR_MAC_10: usize = 0x10;
pub const REG_SENSOR_MAC_14: usize = 0x14;
pub const REG_SENSOR_MAC_18: usize = 0x18;
pub const REG_SENSOR_MAC_1C: usize = 0x1C;
pub const REG_SENSOR_MAC_20: usize = 0x20;
pub const REG_SENSOR_MAC_24: usize = 0x24;
pub const REG_SENSOR_MAC_28: usize = 0x28;
pub const REG_SENSOR_MAC_30: usize = 0x30;
pub const REG_SENSOR_MAC_40: usize = 0x40;
pub const REG_SENSOR_MAC_44: usize = 0x44;
pub const REG_SENSOR_MAC_48: usize = 0x48;
pub const REG_SENSOR_MAC_BC: usize = 0xBC;

// ============================================================================
// CSI 寄存器偏移
// ============================================================================
pub const REG_CSI_CTRL_00: usize = 0x00;
pub const REG_CSI_CTRL_04: usize = 0x04;
pub const REG_CSI_CTRL_18: usize = 0x18;
pub const REG_CSI_CTRL_40: usize = 0x40;
pub const REG_CSI_CTRL_60: usize = 0x60;
pub const REG_CSI_CTRL_70: usize = 0x70;
pub const REG_CSI_CTRL_74: usize = 0x74;

// ============================================================================
// PHY TOP 寄存器偏移
// ============================================================================
pub const REG_PHY_TOP_00: usize = 0x00;
pub const REG_PHY_TOP_04: usize = 0x04;
pub const REG_PHY_TOP_30: usize = 0x30;
pub const REG_PHY_TOP_70: usize = 0x70;
pub const REG_PHY_TOP_74: usize = 0x74;
pub const REG_PHY_TOP_80: usize = 0x80;
pub const REG_PHY_TOP_84: usize = 0x84;
pub const REG_PHY_TOP_88: usize = 0x88;

// ============================================================================
// PHY 4L 寄存器偏移
// ============================================================================
pub const REG_PHY_4L_00: usize = 0x00;
pub const REG_PHY_4L_04: usize = 0x04;
pub const REG_PHY_4L_08: usize = 0x08;
pub const REG_PHY_4L_0C: usize = 0x0C;
pub const REG_PHY_4L_10: usize = 0x10;
pub const REG_PHY_4L_20: usize = 0x20;

// ============================================================================
// 底层驱动函数
// ============================================================================

/// 配置 CSI 模式
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_config_csi(ctx: &CifDrvCtx, param: &ParamCsi) {
    let mac_top = ctx.mac_top();
    let mac_csi = ctx.mac_csi();
    let wrap_4l = ctx.wrap_4l();
    let wrap_2l = ctx.wrap_2l();

    // 配置传感器模式为 CSI
    let mut val = reg_read(mac_top + REG_SENSOR_MAC_MODE);
    val = (val & !0x3) | 1; // SENSOR_MAC_MODE = 1 (CSI)
    reg_write(mac_top + REG_SENSOR_MAC_MODE, val);

    // 反转 HS/VS
    val = reg_read(mac_top + REG_SENSOR_MAC_MODE);
    val |= (1 << 4) | (1 << 5); // CSI_VS_INV | CSI_HS_INV
    reg_write(mac_top + REG_SENSOR_MAC_MODE, val);

    // 使能 CSI 控制器
    val = reg_read(mac_top + REG_SENSOR_MAC_MODE);
    val |= 1 << 8; // CSI_CTRL_ENABLE
    reg_write(mac_top + REG_SENSOR_MAC_MODE, val);

    // 配置 Lane 数量
    val = reg_read(mac_csi + REG_CSI_CTRL_00);
    val = (val & !0x3) | ((param.lane_num as u32 - 1) & 0x3);
    reg_write(mac_csi + REG_CSI_CTRL_00, val);

    // 配置 VS 生成模式
    val = reg_read(mac_csi + REG_CSI_CTRL_70);
    val = (val & !0x3) | (param.vs_gen_mode as u32 & 0x3);
    reg_write(mac_csi + REG_CSI_CTRL_70, val);

    // 配置 DPHY
    if ctx.mac_num == 0 {
        // 禁用 auto_ignore 和 auto_sync
        val = reg_read(wrap_4l + REG_PHY_4L_10);
        val &= !(1 << 0); // AUTO_IGNORE
        val &= !(1 << 1); // AUTO_SYNC
        reg_write(wrap_4l + REG_PHY_4L_10, val);

        // 设置传感器模式为 CSI
        val = reg_read(wrap_4l + REG_PHY_4L_00);
        val &= !0x1; // SENSOR_MODE = 0 (CSI)
        reg_write(wrap_4l + REG_PHY_4L_00, val);
    } else {
        // 2L PHY
        val = reg_read(wrap_2l + REG_PHY_4L_10);
        val &= !(1 << 0);
        val &= !(1 << 1);
        reg_write(wrap_2l + REG_PHY_4L_10, val);

        val = reg_read(wrap_2l + REG_PHY_4L_00);
        val &= !0x1;
        reg_write(wrap_2l + REG_PHY_4L_00, val);
    }

    // 配置 VC 映射
    let vc_map = ((param.vc_mapping[0] as u32) << 0)
        | ((param.vc_mapping[1] as u32) << 4)
        | ((param.vc_mapping[2] as u32) << 8)
        | ((param.vc_mapping[3] as u32) << 12);
    reg_write(mac_csi + REG_CSI_CTRL_18, vc_map);
}

/// 使能/禁用 CIF 流
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_streaming(ctx: &CifDrvCtx, on: bool, cif_type: CifType, lane_num: u16) {
    let mac_top = ctx.mac_top();
    let wrap_top = ctx.wrap_top();
    let wrap_4l = ctx.wrap_4l();
    let wrap_2l = ctx.wrap_2l();

    // 配置 PHY termination（仅串行接口）
    if cif_type == CifType::Csi || cif_type == CifType::Sublvds || cif_type == CifType::Hispi {
        let mut val = reg_read(wrap_top + REG_PHY_TOP_00);
        if on {
            val &= !(1 << 0); // MIPIRX_PD_IBIAS = 0
        } else {
            val |= 1 << 0; // MIPIRX_PD_IBIAS = 1
        }
        reg_write(wrap_top + REG_PHY_TOP_00, val);

        val = reg_read(wrap_top + REG_PHY_TOP_00);
        if on {
            val &= !(0x3F << 8); // MIPIRX_PD_RXLP = 0
        } else {
            val |= 0x3F << 8; // MIPIRX_PD_RXLP = 0x3F
        }
        reg_write(wrap_top + REG_PHY_TOP_00, val);
    }

    // 使能 MAC 调试
    let mut val = reg_read(mac_top + REG_SENSOR_MAC_BC);
    val |= 1 << 0; // SENSOR_MAC_DBG_EN
    reg_write(mac_top + REG_SENSOR_MAC_BC, val);

    match cif_type {
        CifType::Csi => {
            // 清除/设置 Lane 使能
            if ctx.mac_num == 0 {
                val = reg_read(wrap_4l + REG_PHY_4L_0C);
                if on {
                    // 延时
                    for _ in 0..2000 {
                        core::hint::spin_loop();
                    }
                    val |= (1 << lane_num) - 1; // DESKEW_LANE_EN
                } else {
                    val &= !0xF;
                }
                reg_write(wrap_4l + REG_PHY_4L_0C, val);
            } else {
                val = reg_read(wrap_2l + REG_PHY_4L_0C);
                if on {
                    for _ in 0..2000 {
                        core::hint::spin_loop();
                    }
                    val |= (1 << lane_num) - 1;
                } else {
                    val &= !0x3;
                }
                reg_write(wrap_2l + REG_PHY_4L_0C, val);
            }
        }
        CifType::Ttl => {
            // 使能 TTL
            val = reg_read(mac_top + REG_SENSOR_MAC_10);
            if on {
                val |= 1 << 0; // TTL_IP_EN
            } else {
                val &= !(1 << 0);
            }
            reg_write(mac_top + REG_SENSOR_MAC_10, val);
        }
        _ => {}
    }
}

/// 设置 Lane ID
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_set_lane_id(
    ctx: &CifDrvCtx,
    lane: LaneId,
    select: u32,
    pn_swap: bool,
) {
    let wrap_top = ctx.wrap_top();
    let wrap_4l = ctx.wrap_4l();
    let wrap_2l = ctx.wrap_2l();

    match lane {
        LaneId::Clk => {
            if ctx.mac_num == 0 {
                // PHYA 时钟选择
                let mut val = reg_read(wrap_top + REG_PHY_TOP_04);
                val |= 1 << select; // MIPIRX_SEL_CLK_CHANNEL
                reg_write(wrap_top + REG_PHY_TOP_04, val);

                // PHYD 时钟选择
                val = reg_read(wrap_4l + REG_PHY_4L_08);
                val = (val & !0x7) | (select & 0x7); // CSI_LANE_CK_SEL
                if pn_swap {
                    val |= 1 << 4; // CSI_LANE_CK_PNSWAP
                } else {
                    val &= !(1 << 4);
                }
                reg_write(wrap_4l + REG_PHY_4L_08, val);
            } else {
                // 使能双模式
                let mut val = reg_read(wrap_top + REG_PHY_TOP_30);
                val |= 1 << 0; // SENSOR_PHY_MODE
                reg_write(wrap_top + REG_PHY_TOP_30, val);

                val = reg_read(wrap_top + REG_PHY_TOP_04);
                val |= 1 << select;
                reg_write(wrap_top + REG_PHY_TOP_04, val);

                val = reg_read(wrap_2l + REG_PHY_4L_08);
                val = (val & !0x7) | (select & 0x7);
                if pn_swap {
                    val |= 1 << 4;
                } else {
                    val &= !(1 << 4);
                }
                reg_write(wrap_2l + REG_PHY_4L_08, val);
            }
        }
        LaneId::Lane0 => {
            if ctx.mac_num == 0 {
                let mut val = reg_read(wrap_4l + REG_PHY_4L_04);
                val = (val & !0x7) | (select & 0x7); // CSI_LANE_D0_SEL
                reg_write(wrap_4l + REG_PHY_4L_04, val);

                val = reg_read(wrap_4l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 8; // CSI_LANE_D0_PNSWAP
                } else {
                    val &= !(1 << 8);
                }
                reg_write(wrap_4l + REG_PHY_4L_08, val);
            } else {
                let mut val = reg_read(wrap_2l + REG_PHY_4L_04);
                val = (val & !0x7) | ((select % 3) & 0x7);
                reg_write(wrap_2l + REG_PHY_4L_04, val);

                val = reg_read(wrap_2l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 8;
                } else {
                    val &= !(1 << 8);
                }
                reg_write(wrap_2l + REG_PHY_4L_08, val);
            }
        }
        LaneId::Lane1 => {
            if ctx.mac_num == 0 {
                let mut val = reg_read(wrap_4l + REG_PHY_4L_04);
                val = (val & !(0x7 << 4)) | ((select & 0x7) << 4); // CSI_LANE_D1_SEL
                reg_write(wrap_4l + REG_PHY_4L_04, val);

                val = reg_read(wrap_4l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 12; // CSI_LANE_D1_PNSWAP
                } else {
                    val &= !(1 << 12);
                }
                reg_write(wrap_4l + REG_PHY_4L_08, val);
            } else {
                let mut val = reg_read(wrap_2l + REG_PHY_4L_04);
                val = (val & !(0x7 << 4)) | (((select % 3) & 0x7) << 4);
                reg_write(wrap_2l + REG_PHY_4L_04, val);

                val = reg_read(wrap_2l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 12;
                } else {
                    val &= !(1 << 12);
                }
                reg_write(wrap_2l + REG_PHY_4L_08, val);
            }
        }
        LaneId::Lane2 => {
            if ctx.mac_num == 0 {
                let mut val = reg_read(wrap_4l + REG_PHY_4L_04);
                val = (val & !(0x7 << 8)) | ((select & 0x7) << 8); // CSI_LANE_D2_SEL
                reg_write(wrap_4l + REG_PHY_4L_04, val);

                val = reg_read(wrap_4l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 16; // CSI_LANE_D2_PNSWAP
                } else {
                    val &= !(1 << 16);
                }
                reg_write(wrap_4l + REG_PHY_4L_08, val);
            }
        }
        LaneId::Lane3 => {
            if ctx.mac_num == 0 {
                let mut val = reg_read(wrap_4l + REG_PHY_4L_04);
                val = (val & !(0x7 << 12)) | ((select & 0x7) << 12); // CSI_LANE_D3_SEL
                reg_write(wrap_4l + REG_PHY_4L_04, val);

                val = reg_read(wrap_4l + REG_PHY_4L_08);
                if pn_swap {
                    val |= 1 << 20; // CSI_LANE_D3_PNSWAP
                } else {
                    val &= !(1 << 20);
                }
                reg_write(wrap_4l + REG_PHY_4L_08, val);
            }
        }
    }
}

/// 设置 HS Settle 时间
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_set_hs_settle(ctx: &CifDrvCtx, hs_settle: u8) {
    let wrap_4l = ctx.wrap_4l();
    let wrap_2l = ctx.wrap_2l();

    if ctx.mac_num == 0 {
        // 禁用 auto_ignore 和 auto_sync
        let mut val = reg_read(wrap_4l + REG_PHY_4L_10);
        val &= !(1 << 0); // AUTO_IGNORE
        val &= !(1 << 1); // AUTO_SYNC
        val = (val & !(0xFF << 8)) | ((hs_settle as u32) << 8); // T_HS_SETTLE
        reg_write(wrap_4l + REG_PHY_4L_10, val);
    } else {
        let mut val = reg_read(wrap_2l + REG_PHY_4L_10);
        val &= !(1 << 0);
        val &= !(1 << 1);
        val = (val & !(0xFF << 8)) | ((hs_settle as u32) << 8);
        reg_write(wrap_2l + REG_PHY_4L_10, val);
    }
}

/// 检查 CSI 中断状态
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_check_csi_int_sts(ctx: &CifDrvCtx, mask: u32) -> bool {
    let mac_csi = ctx.mac_csi();
    let val = reg_read(mac_csi + REG_CSI_CTRL_60);
    (val & mask) != 0
}

/// 清除 CSI 中断状态
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_clear_csi_int_sts(ctx: &CifDrvCtx) {
    let mac_csi = ctx.mac_csi();
    let mut val = reg_read(mac_csi + REG_CSI_CTRL_04);
    val |= 0xFF << 16; // CSI_INTR_CLR
    reg_write(mac_csi + REG_CSI_CTRL_04, val);
}

/// 屏蔽 CSI 中断
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_mask_csi_int_sts(ctx: &CifDrvCtx, mask: u32) {
    let mac_csi = ctx.mac_csi();
    let mut val = reg_read(mac_csi + REG_CSI_CTRL_04);
    val |= (mask & 0xFF) << 8; // CSI_INTR_MASK
    reg_write(mac_csi + REG_CSI_CTRL_04, val);
}

/// 取消屏蔽 CSI 中断
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_unmask_csi_int_sts(ctx: &CifDrvCtx, _mask: u32) {
    let mac_csi = ctx.mac_csi();
    let mut val = reg_read(mac_csi + REG_CSI_CTRL_04);
    val &= !(0xFF << 8); // CSI_INTR_MASK = 0
    reg_write(mac_csi + REG_CSI_CTRL_04, val);
}

/// 检查 CSI FIFO 是否满
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_check_csi_fifo_full(ctx: &CifDrvCtx) -> bool {
    let mac_csi = ctx.mac_csi();
    let val = reg_read(mac_csi + REG_CSI_CTRL_40);
    (val & (1 << 0)) != 0 // CSI_FIFO_FULL
}

/// 获取 CSI 解码格式
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn cif_get_csi_decode_fmt(ctx: &CifDrvCtx) -> CsiDecodeFmt {
    let mac_csi = ctx.mac_csi();
    let val = reg_read(mac_csi + REG_CSI_CTRL_40);
    let fmt = (val >> 8) & 0x1F; // CSI_DECODE_FORMAT

    match fmt.trailing_zeros() {
        0 => CsiDecodeFmt::Yuv422_8,
        1 => CsiDecodeFmt::Yuv422_10,
        2 => CsiDecodeFmt::Raw8,
        3 => CsiDecodeFmt::Raw10,
        4 => CsiDecodeFmt::Raw12,
        _ => CsiDecodeFmt::Raw10,
    }
}
