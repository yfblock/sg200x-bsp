//! BT (BT.656/BT.601/BT.1120) 接口配置
//!
//! 本模块提供 BT 系列视频接口的配置功能，支持：
//! - BT.656: 8-bit 数据 + 嵌入式同步码
//! - BT.601: 8-bit/16-bit 数据 + 外部同步信号
//! - BT.1120: 16-bit 数据 + 嵌入式同步码
//!
//! ## BT.656 接口
//!
//! BT.656 使用 8-bit 数据线传输 YUV422 视频数据，同步信息嵌入在数据流中。
//!
//! ```text
//! 同步码格式: {0xFF, 0x00, 0x00, SAV/EAV}
//!
//! SAV_VLD (0x80): 有效行开始
//! EAV_VLD (0x9D): 有效行结束
//! SAV_BLK (0xAB): 消隐行开始
//! EAV_BLK (0xB6): 消隐行结束
//! ```
//!
//! ## BT.1120 接口
//!
//! BT.1120 使用 16-bit 数据线传输 YUV422 视频数据，Y 和 C 分开传输。
//! 同步码格式与 BT.656 相同，但同时应用于 Y 和 C 通道。
//!
//! ## BT.601 接口
//!
//! BT.601 支持三种同步模式：
//! - VHS 模式: 使用 VS + HS 同步信号
//! - VDE 模式: 使用 VDE + HDE 同步信号
//! - VSDE 模式: 使用 VS + HDE 同步信号
//!
//! ## 多通道融合 (Demux)
//!
//! 支持 1/2/4 通道的 BT 信号融合输入，每个通道使用不同的同步码标识。

#![allow(dead_code)]

use super::regs;
use super::types::*;
use super::Vi;
use tock_registers::interfaces::ReadWriteable;

// ============================================================================
// BT 接口配置函数
// ============================================================================

/// 配置 BT 接口
///
/// 根据提供的配置参数设置 BT 接口寄存器
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `config` - BT 接口配置
///
/// # Returns
/// * `Ok(())` - 配置成功
/// * `Err(ViError)` - 配置失败
///
/// # Example
/// ```rust,ignore
/// let config = BtConfig {
///     enable: true,
///     format: BtFormat::Bt656_9bit,
///     img_size: ImageSize::new(720, 480),
///     ..Default::default()
/// };
/// bt::configure_bt(&mut vi, &config)?;
/// ```
pub fn configure_bt(vi: &mut Vi, config: &BtConfig) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        // 配置 REG_80 - BT 路径配置寄存器 0
        configure_bt_path(regs, config);

        // 配置 REG_88 - 图像尺寸
        configure_bt_image_size(regs, &config.img_size);

        // 配置 REG_8C/REG_90 - 消隐参数
        configure_bt_blanking(regs, &config.blanking);

        // 配置 REG_94 - 同步码前缀
        configure_bt_sync_prefix(regs, &config.sync_code);

        // 配置 REG_98~REG_A4 - Demux 同步码
        configure_bt_demux_sync(regs, config.demux_ch, &config.sync_code);

        // 如果使能 BT Demux，配置 REG_00
        if config.demux_ch != BtDemuxChannel::None {
            regs.reg_00.modify(regs::REG_00::REG_BT_DEMUX_ENABLE::SET);
        } else {
            regs.reg_00
                .modify(regs::REG_00::REG_BT_DEMUX_ENABLE::CLEAR);
        }
    }

    Ok(())
}

/// 配置 BT 路径参数 (REG_80)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `config` - BT 接口配置
fn configure_bt_path(regs: &regs::ViRegs, config: &BtConfig) {
    // 构建时钟门控使能值
    // [0]: 延迟控制时钟
    // [1]: 时序解复用器时钟
    // [2]: 时序生成器时钟
    // [3-6]: RX 解码器 0-3 时钟
    let clk_en = if config.enable {
        match config.demux_ch {
            BtDemuxChannel::None => 0b0001111,   // 单通道，使能解码器 0
            BtDemuxChannel::Demux2 => 0b0011111, // 2 通道，使能解码器 0-1
            BtDemuxChannel::Demux3 => 0b0111111, // 3 通道，使能解码器 0-2
            BtDemuxChannel::Demux4 => 0b1111111, // 4 通道，使能解码器 0-3
        }
    } else {
        0
    };

    regs.reg_80.modify(
        // BT 路径使能
        if config.enable {
            regs::REG_80::REG_BT_IP_EN::SET
        } else {
            regs::REG_80::REG_BT_IP_EN::CLEAR
        }
        // DDR 模式
        + if config.ddr_mode {
            regs::REG_80::REG_BT_DDR_MODE::SET
        } else {
            regs::REG_80::REG_BT_DDR_MODE::CLEAR
        }
        // VS 信号反相
        + if config.vs_inv {
            regs::REG_80::REG_BT_VS_INV::SET
        } else {
            regs::REG_80::REG_BT_VS_INV::CLEAR
        }
        // HS 信号反相
        + if config.hs_inv {
            regs::REG_80::REG_BT_HS_INV::SET
        } else {
            regs::REG_80::REG_BT_HS_INV::CLEAR
        }
        // VS 作为 VDE 使用
        + if config.vs_as_vde {
            regs::REG_80::REG_BT_VS_AS_VDE::SET
        } else {
            regs::REG_80::REG_BT_VS_AS_VDE::CLEAR
        }
        // HS 作为 HDE 使用
        + if config.hs_as_hde {
            regs::REG_80::REG_BT_HS_AS_HDE::SET
        } else {
            regs::REG_80::REG_BT_HS_AS_HDE::CLEAR
        }
        // 时钟门控使能
        + regs::REG_80::REG_BT_SW_EN_CLK.val(clk_en)
        // Demux 通道数
        + regs::REG_80::REG_BT_DEMUX_CH.val(config.demux_ch as u32)
        // BT 格式选择
        + regs::REG_80::REG_BT_FMT_SEL.val(config.format as u32),
    );
}

/// 配置 BT 图像尺寸 (REG_88)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `img_size` - 图像尺寸
fn configure_bt_image_size(regs: &regs::ViRegs, img_size: &ImageSize) {
    // 寄存器值为实际值 - 1
    let width_m1 = if img_size.width > 0 {
        (img_size.width - 1) as u32
    } else {
        0
    };
    let height_m1 = if img_size.height > 0 {
        (img_size.height - 1) as u32
    } else {
        0
    };

    regs.reg_88.modify(
        regs::REG_88::REG_BT_IMG_WD_M1.val(width_m1)
            + regs::REG_88::REG_BT_IMG_HT_M1.val(height_m1),
    );
}

/// 配置 BT 消隐参数 (REG_8C/REG_90)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `blanking` - 消隐配置
fn configure_bt_blanking(regs: &regs::ViRegs, blanking: &BlankingConfig) {
    // 后消隐 (Back Porch) - 寄存器值为实际值 - 1
    let vs_bp_m1 = if blanking.vs_back_porch > 0 {
        (blanking.vs_back_porch - 1) as u32
    } else {
        0
    };
    let hs_bp_m1 = if blanking.hs_back_porch > 0 {
        (blanking.hs_back_porch - 1) as u32
    } else {
        0
    };

    regs.reg_8c.modify(
        regs::REG_8C::REG_BT_VS_BP_M1.val(vs_bp_m1) + regs::REG_8C::REG_BT_HS_BP_M1.val(hs_bp_m1),
    );

    // 前消隐 (Front Porch) - 寄存器值为实际值 - 1
    let vs_fp_m1 = if blanking.vs_front_porch > 0 {
        (blanking.vs_front_porch - 1) as u32
    } else {
        0
    };
    let hs_fp_m1 = if blanking.hs_front_porch > 0 {
        (blanking.hs_front_porch - 1) as u32
    } else {
        0
    };

    regs.reg_90.modify(
        regs::REG_90::REG_BT_VS_FP_M1.val(vs_fp_m1) + regs::REG_90::REG_BT_HS_FP_M1.val(hs_fp_m1),
    );
}

/// 配置 BT 同步码前缀 (REG_94)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `sync_code` - 同步码配置
fn configure_bt_sync_prefix(regs: &regs::ViRegs, sync_code: &BtSyncCode) {
    regs.reg_94.modify(
        regs::REG_94::REG_BT_SYNC_0.val(sync_code.sync_0 as u32)
            + regs::REG_94::REG_BT_SYNC_1.val(sync_code.sync_1 as u32)
            + regs::REG_94::REG_BT_SYNC_2.val(sync_code.sync_2 as u32),
    );
}

/// 配置 BT Demux 同步码 (REG_98~REG_A4)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `demux_ch` - Demux 通道数
/// * `sync_code` - 同步码配置
fn configure_bt_demux_sync(
    regs: &regs::ViRegs,
    demux_ch: BtDemuxChannel,
    sync_code: &BtSyncCode,
) {
    // 配置 Demux 0 同步码 (REG_98)
    regs.reg_98.modify(
        regs::REG_98::REG_BT_SAV_VLD_0.val(sync_code.sav_vld as u32)
            + regs::REG_98::REG_BT_SAV_BLK_0.val(sync_code.sav_blk as u32)
            + regs::REG_98::REG_BT_EAV_VLD_0.val(sync_code.eav_vld as u32)
            + regs::REG_98::REG_BT_EAV_BLK_0.val(sync_code.eav_blk as u32),
    );

    // 如果使用多通道 Demux，配置其他通道的同步码
    // 注意：多通道时，每个通道需要不同的同步码来区分
    // 这里使用简单的偏移方案，实际应用中可能需要根据具体传感器调整

    if demux_ch as u8 >= BtDemuxChannel::Demux2 as u8 {
        // 配置 Demux 1 同步码 (REG_9C)
        // 使用不同的同步码值来区分通道
        regs.reg_9c.modify(
            regs::REG_9C::REG_BT_SAV_VLD_1.val((sync_code.sav_vld.wrapping_add(0x10)) as u32)
                + regs::REG_9C::REG_BT_SAV_BLK_1.val((sync_code.sav_blk.wrapping_add(0x10)) as u32)
                + regs::REG_9C::REG_BT_EAV_VLD_1.val((sync_code.eav_vld.wrapping_add(0x10)) as u32)
                + regs::REG_9C::REG_BT_EAV_BLK_1.val((sync_code.eav_blk.wrapping_add(0x10)) as u32),
        );
    }

    if demux_ch as u8 >= BtDemuxChannel::Demux3 as u8 {
        // 配置 Demux 2 同步码 (REG_A0)
        regs.reg_a0.modify(
            regs::REG_A0::REG_BT_SAV_VLD_2.val((sync_code.sav_vld.wrapping_add(0x20)) as u32)
                + regs::REG_A0::REG_BT_SAV_BLK_2.val((sync_code.sav_blk.wrapping_add(0x20)) as u32)
                + regs::REG_A0::REG_BT_EAV_VLD_2.val((sync_code.eav_vld.wrapping_add(0x20)) as u32)
                + regs::REG_A0::REG_BT_EAV_BLK_2.val((sync_code.eav_blk.wrapping_add(0x20)) as u32),
        );
    }

    if demux_ch == BtDemuxChannel::Demux4 {
        // 配置 Demux 3 同步码 (REG_A4)
        regs.reg_a4.modify(
            regs::REG_A4::REG_BT_SAV_VLD_3.val((sync_code.sav_vld.wrapping_add(0x30)) as u32)
                + regs::REG_A4::REG_BT_SAV_BLK_3.val((sync_code.sav_blk.wrapping_add(0x30)) as u32)
                + regs::REG_A4::REG_BT_EAV_VLD_3.val((sync_code.eav_vld.wrapping_add(0x30)) as u32)
                + regs::REG_A4::REG_BT_EAV_BLK_3.val((sync_code.eav_blk.wrapping_add(0x30)) as u32),
        );
    }
}

// ============================================================================
// BT 接口辅助函数
// ============================================================================

/// 创建标准 BT.656 NTSC 配置 (720x480)
///
/// # Returns
/// BT.656 NTSC 配置
pub fn bt656_ntsc_config() -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt656_9bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(720, 480),
        blanking: BlankingConfig {
            vs_back_porch: 16,
            vs_front_porch: 9,
            hs_back_porch: 122,
            hs_front_porch: 16,
        },
        sync_code: BtSyncCode::default(),
    }
}

/// 创建标准 BT.656 PAL 配置 (720x576)
///
/// # Returns
/// BT.656 PAL 配置
pub fn bt656_pal_config() -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt656_9bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(720, 576),
        blanking: BlankingConfig {
            vs_back_porch: 22,
            vs_front_porch: 2,
            hs_back_porch: 132,
            hs_front_porch: 12,
        },
        sync_code: BtSyncCode::default(),
    }
}

/// 创建标准 BT.1120 720P 配置 (1280x720)
///
/// # Returns
/// BT.1120 720P 配置
pub fn bt1120_720p_config() -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt1120_17bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(1280, 720),
        blanking: BlankingConfig {
            vs_back_porch: 20,
            vs_front_porch: 5,
            hs_back_porch: 220,
            hs_front_porch: 110,
        },
        sync_code: BtSyncCode::default(),
    }
}

/// 创建标准 BT.1120 1080P 配置 (1920x1080)
///
/// # Returns
/// BT.1120 1080P 配置
pub fn bt1120_1080p_config() -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt1120_17bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(1920, 1080),
        blanking: BlankingConfig {
            vs_back_porch: 36,
            vs_front_porch: 4,
            hs_back_porch: 148,
            hs_front_porch: 88,
        },
        sync_code: BtSyncCode::default(),
    }
}

/// 创建 BT.601 VHS 模式配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
/// * `is_16bit` - 是否为 16-bit 模式
///
/// # Returns
/// BT.601 VHS 模式配置
pub fn bt601_vhs_config(width: u16, height: u16, is_16bit: bool) -> BtConfig {
    BtConfig {
        enable: true,
        format: if is_16bit {
            BtFormat::Bt601_19bit_Vhs
        } else {
            BtFormat::Bt601_11bit_Vhs
        },
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
        sync_code: BtSyncCode::default(),
    }
}

/// 创建 BT.601 VDE 模式配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
/// * `is_16bit` - 是否为 16-bit 模式
///
/// # Returns
/// BT.601 VDE 模式配置
pub fn bt601_vde_config(width: u16, height: u16, is_16bit: bool) -> BtConfig {
    BtConfig {
        enable: true,
        format: if is_16bit {
            BtFormat::Bt601_19bit_Vde
        } else {
            BtFormat::Bt601_11bit_Vde
        },
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: true,
        hs_as_hde: true,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
        sync_code: BtSyncCode::default(),
    }
}

/// 验证 BT 配置参数
///
/// # Arguments
/// * `config` - BT 接口配置
///
/// # Returns
/// * `Ok(())` - 配置有效
/// * `Err(ViError)` - 配置无效
pub fn validate_bt_config(config: &BtConfig) -> Result<(), ViError> {
    // 验证图像尺寸
    if !config.img_size.is_valid() {
        return Err(ViError::InvalidImageSize);
    }

    // 验证 Demux 配置
    // 多通道 Demux 仅支持带同步码的格式
    if config.demux_ch != BtDemuxChannel::None && !config.format.uses_sync_code() {
        return Err(ViError::InvalidConfig);
    }

    Ok(())
}
