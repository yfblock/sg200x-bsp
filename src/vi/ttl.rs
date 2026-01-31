//! TTL 接口配置
//!
//! 本模块提供 TTL 视频接口的配置功能，支持：
//! - BT.656/BT.601/BT.1120 通过 TTL 引脚输入
//! - DC (Digital Camera) 接口
//! - 多种同步模式 (VHS/VDE/VSDE)
//!
//! ## TTL 接口特点
//!
//! TTL 接口支持以下数据位宽：
//! - 8-bit: 用于 BT.656、8-bit DC
//! - 10-bit: 用于 10-bit DC
//! - 12-bit: 用于 12-bit DC
//! - 16-bit: 用于 BT.1120、16-bit DC
//!
//! ## 同步模式
//!
//! ### VHS 模式 (VS + HS)
//! 使用帧同步信号 (VS) 和行同步信号 (HS) 进行同步。
//! 需要配置消隐参数 (back porch) 和图像尺寸。
//!
//! ```text
//! VS __|‾‾‾‾|___________________________________|‾‾‾‾|___
//! HS __|‾|__|‾|__|‾|__|‾|__|‾|__|‾|__|‾|__|‾|__|‾|__|‾|__
//!       ^-- vs_back_porch
//!          ^-- hs_back_porch
//! ```
//!
//! ### VDE 模式 (VDE + HDE)
//! 使用帧有效信号 (VDE) 和行有效信号 (HDE) 进行同步。
//! 不需要配置消隐参数，直接根据有效信号采集数据。
//!
//! ### VSDE 模式 (VS + HDE)
//! 使用帧同步信号 (VS) 和行有效信号 (HDE) 进行同步。
//! 混合模式，VS 用于帧同步，HDE 用于行数据采集。
//!
//! ## DC 接口
//!
//! DC (Digital Camera) 接口用于连接数字摄像头，支持：
//! - RAW 格式数据 (RAW8/RAW10/RAW12/RAW16)
//! - 带同步码或外部同步信号

#![allow(dead_code)]

use super::regs;
use super::types::*;
use super::Vi;
use tock_registers::interfaces::ReadWriteable;

// ============================================================================
// TTL 接口配置函数
// ============================================================================

/// 配置 TTL 接口
///
/// 根据提供的配置参数设置 TTL 接口寄存器
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `config` - TTL 接口配置
///
/// # Returns
/// * `Ok(())` - 配置成功
/// * `Err(ViError)` - 配置失败
///
/// # Example
/// ```rust,ignore
/// let config = TtlConfig {
///     enable: true,
///     bit_width: TtlBitWidth::Bit8,
///     fmt_in: TtlInputFormat::Bt656_9bit,
///     img_size: ImageSize::new(720, 480),
///     ..Default::default()
/// };
/// ttl::configure_ttl(&mut vi, &config)?;
/// ```
pub fn configure_ttl(vi: &mut Vi, config: &TtlConfig) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        // 配置 REG_10 - TTL 模式配置寄存器 0
        configure_ttl_mode(regs, config);

        // 配置 REG_14 - 消隐参数
        configure_ttl_blanking(regs, &config.blanking);

        // 配置 REG_18 - 图像尺寸
        configure_ttl_image_size(regs, &config.img_size);

        // 如果使用同步码模式，配置同步码
        if config.fmt_in.uses_sync_code() {
            configure_ttl_sync_code(regs);
        }

        // 设置传感器 MAC 模式为 TTL
        if config.enable {
            vi.set_mac_mode(SensorMacMode::Ttl)?;
        }
    }

    Ok(())
}

/// 配置 TTL 模式参数 (REG_10)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `config` - TTL 接口配置
fn configure_ttl_mode(regs: &regs::ViRegs, config: &TtlConfig) {
    regs.reg_10.modify(
        // TTL 使能
        if config.enable {
            regs::REG_10::REG_TTL_IP_EN::SET
        } else {
            regs::REG_10::REG_TTL_IP_EN::CLEAR
        }
        // 位宽模式
        + regs::REG_10::REG_TTL_SENSOR_BIT.val(config.bit_width as u32)
        // 输出格式
        + regs::REG_10::REG_TTL_BT_FMT_OUT.val(config.fmt_out as u32)
        // 输入格式
        + regs::REG_10::REG_TTL_FMT_IN.val(config.fmt_in as u32)
        // 数据序列
        + regs::REG_10::REG_TTL_BT_DATA_SEQ.val(config.data_seq as u32)
        // VS 信号反相
        + if config.vs_inv {
            regs::REG_10::REG_TTL_VS_INV::SET
        } else {
            regs::REG_10::REG_TTL_VS_INV::CLEAR
        }
        // HS 信号反相
        + if config.hs_inv {
            regs::REG_10::REG_TTL_HS_INV::SET
        } else {
            regs::REG_10::REG_TTL_HS_INV::CLEAR
        },
    );
}

/// 配置 TTL 消隐参数 (REG_14)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `blanking` - 消隐配置
fn configure_ttl_blanking(regs: &regs::ViRegs, blanking: &BlankingConfig) {
    regs.reg_14.modify(
        regs::REG_14::REG_TTL_VS_BP.val(blanking.vs_back_porch as u32)
            + regs::REG_14::REG_TTL_HS_BP.val(blanking.hs_back_porch as u32),
    );
}

/// 配置 TTL 图像尺寸 (REG_18)
///
/// # Arguments
/// * `regs` - VI 寄存器引用
/// * `img_size` - 图像尺寸
fn configure_ttl_image_size(regs: &regs::ViRegs, img_size: &ImageSize) {
    regs.reg_18.modify(
        regs::REG_18::REG_TTL_IMG_WD.val(img_size.width as u32)
            + regs::REG_18::REG_TTL_IMG_HT.val(img_size.height as u32),
    );
}

/// 配置 TTL 同步码 (REG_1C~REG_28)
///
/// 配置标准 BT.656/BT.1120 同步码
///
/// # Arguments
/// * `regs` - VI 寄存器引用
fn configure_ttl_sync_code(regs: &regs::ViRegs) {
    // 配置同步码前缀 (REG_1C/REG_20)
    // 标准同步码: 0xFF, 0x00, 0x00
    regs.reg_1c.modify(
        regs::REG_1C::REG_TTL_SYNC_0.val(0x00FF) // 0xFF, 0x00
            + regs::REG_1C::REG_TTL_SYNC_1.val(0x0000), // 0x00, 0x00
    );

    regs.reg_20.modify(regs::REG_20::REG_TTL_SYNC_2.val(0x0000));

    // 配置 SAV/EAV 同步码 (REG_24/REG_28)
    // 标准 BT.656 同步码:
    // SAV_VLD: 0x80 (有效行开始)
    // EAV_VLD: 0x9D (有效行结束)
    // SAV_BLK: 0xAB (消隐行开始)
    // EAV_BLK: 0xB6 (消隐行结束)
    regs.reg_24.modify(
        regs::REG_24::REG_TTL_SAV_VLD.val(0x0080) + regs::REG_24::REG_TTL_SAV_BLK.val(0x00AB),
    );

    regs.reg_28.modify(
        regs::REG_28::REG_TTL_EAV_VLD.val(0x009D) + regs::REG_28::REG_TTL_EAV_BLK.val(0x00B6),
    );
}

/// 配置自定义 TTL 同步码
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `sync_0` - 同步码 0
/// * `sync_1` - 同步码 1
/// * `sync_2` - 同步码 2
/// * `sav_vld` - 有效行 SAV
/// * `sav_blk` - 消隐行 SAV
/// * `eav_vld` - 有效行 EAV
/// * `eav_blk` - 消隐行 EAV
pub fn configure_ttl_custom_sync(
    vi: &mut Vi,
    sync_0: u16,
    sync_1: u16,
    sync_2: u16,
    sav_vld: u16,
    sav_blk: u16,
    eav_vld: u16,
    eav_blk: u16,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_1c.modify(
            regs::REG_1C::REG_TTL_SYNC_0.val(sync_0 as u32)
                + regs::REG_1C::REG_TTL_SYNC_1.val(sync_1 as u32),
        );

        regs.reg_20.modify(regs::REG_20::REG_TTL_SYNC_2.val(sync_2 as u32));

        regs.reg_24.modify(
            regs::REG_24::REG_TTL_SAV_VLD.val(sav_vld as u32)
                + regs::REG_24::REG_TTL_SAV_BLK.val(sav_blk as u32),
        );

        regs.reg_28.modify(
            regs::REG_28::REG_TTL_EAV_VLD.val(eav_vld as u32)
                + regs::REG_28::REG_TTL_EAV_BLK.val(eav_blk as u32),
        );
    }

    Ok(())
}

// ============================================================================
// TTL 引脚配置函数
// ============================================================================

/// 配置 VI 引脚映射
///
/// 配置 VS/HS/VDE/HDE 信号的引脚映射
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `vs_sel` - VS 引脚选择 ([5]: 来自 VI1/VI0, [4:0]: pad 编号)
/// * `hs_sel` - HS 引脚选择
/// * `vde_sel` - VDE 引脚选择
/// * `hde_sel` - HDE 引脚选择
pub fn configure_vi_sync_pins(
    vi: &mut Vi,
    vs_sel: u8,
    hs_sel: u8,
    vde_sel: u8,
    hde_sel: u8,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_60.modify(
            regs::REG_60::REG_VI_VS_SEL.val(vs_sel as u32)
                + regs::REG_60::REG_VI_HS_SEL.val(hs_sel as u32)
                + regs::REG_60::REG_VI_VDE_SEL.val(vde_sel as u32)
                + regs::REG_60::REG_VI_HDE_SEL.val(hde_sel as u32),
        );
    }

    Ok(())
}

/// 配置 VI 数据引脚映射 (D0~D3)
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `d0_sel` - D0 引脚选择
/// * `d1_sel` - D1 引脚选择
/// * `d2_sel` - D2 引脚选择
/// * `d3_sel` - D3 引脚选择
pub fn configure_vi_data_pins_0_3(
    vi: &mut Vi,
    d0_sel: u8,
    d1_sel: u8,
    d2_sel: u8,
    d3_sel: u8,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_64.modify(
            regs::REG_64::REG_VI_D0_SEL.val(d0_sel as u32)
                + regs::REG_64::REG_VI_D1_SEL.val(d1_sel as u32)
                + regs::REG_64::REG_VI_D2_SEL.val(d2_sel as u32)
                + regs::REG_64::REG_VI_D3_SEL.val(d3_sel as u32),
        );
    }

    Ok(())
}

/// 配置 VI 数据引脚映射 (D4~D7)
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `d4_sel` - D4 引脚选择
/// * `d5_sel` - D5 引脚选择
/// * `d6_sel` - D6 引脚选择
/// * `d7_sel` - D7 引脚选择
pub fn configure_vi_data_pins_4_7(
    vi: &mut Vi,
    d4_sel: u8,
    d5_sel: u8,
    d6_sel: u8,
    d7_sel: u8,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_68.modify(
            regs::REG_68::REG_VI_D4_SEL.val(d4_sel as u32)
                + regs::REG_68::REG_VI_D5_SEL.val(d5_sel as u32)
                + regs::REG_68::REG_VI_D6_SEL.val(d6_sel as u32)
                + regs::REG_68::REG_VI_D7_SEL.val(d7_sel as u32),
        );
    }

    Ok(())
}

/// 配置 VI 数据引脚映射 (D8~D11)
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `d8_sel` - D8 引脚选择
/// * `d9_sel` - D9 引脚选择
/// * `d10_sel` - D10 引脚选择
/// * `d11_sel` - D11 引脚选择
pub fn configure_vi_data_pins_8_11(
    vi: &mut Vi,
    d8_sel: u8,
    d9_sel: u8,
    d10_sel: u8,
    d11_sel: u8,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_6c.modify(
            regs::REG_6C::REG_VI_D8_SEL.val(d8_sel as u32)
                + regs::REG_6C::REG_VI_D9_SEL.val(d9_sel as u32)
                + regs::REG_6C::REG_VI_D10_SEL.val(d10_sel as u32)
                + regs::REG_6C::REG_VI_D11_SEL.val(d11_sel as u32),
        );
    }

    Ok(())
}

/// 配置 VI 数据引脚映射 (D12~D15)
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `d12_sel` - D12 引脚选择
/// * `d13_sel` - D13 引脚选择
/// * `d14_sel` - D14 引脚选择
/// * `d15_sel` - D15 引脚选择
pub fn configure_vi_data_pins_12_15(
    vi: &mut Vi,
    d12_sel: u8,
    d13_sel: u8,
    d14_sel: u8,
    d15_sel: u8,
) -> Result<(), ViError> {
    unsafe {
        let regs = vi.regs();

        regs.reg_70.modify(
            regs::REG_70::REG_VI_D12_SEL.val(d12_sel as u32)
                + regs::REG_70::REG_VI_D13_SEL.val(d13_sel as u32)
                + regs::REG_70::REG_VI_D14_SEL.val(d14_sel as u32)
                + regs::REG_70::REG_VI_D15_SEL.val(d15_sel as u32),
        );
    }

    Ok(())
}

/// 配置 VI2 BT 数据引脚映射 (D0~D7)
///
/// 仅用于 VI2，配置 BT 接口的数据引脚
///
/// # Arguments
/// * `vi` - VI 设备引用
/// * `d0_sel` - D0 引脚选择 (0~7)
/// * `d1_sel` - D1 引脚选择
/// * `d2_sel` - D2 引脚选择
/// * `d3_sel` - D3 引脚选择
/// * `d4_sel` - D4 引脚选择
/// * `d5_sel` - D5 引脚选择
/// * `d6_sel` - D6 引脚选择
/// * `d7_sel` - D7 引脚选择
pub fn configure_vi2_bt_data_pins(
    vi: &mut Vi,
    d0_sel: u8,
    d1_sel: u8,
    d2_sel: u8,
    d3_sel: u8,
    d4_sel: u8,
    d5_sel: u8,
    d6_sel: u8,
    d7_sel: u8,
) -> Result<(), ViError> {
    // 仅 VI2 支持此配置
    if vi.devno() != ViDevno::Vi2 {
        return Err(ViError::InvalidDevno);
    }

    unsafe {
        let regs = vi.regs();

        regs.reg_74.modify(
            regs::REG_74::REG_VI_BT_D0_SEL.val(d0_sel as u32)
                + regs::REG_74::REG_VI_BT_D1_SEL.val(d1_sel as u32)
                + regs::REG_74::REG_VI_BT_D2_SEL.val(d2_sel as u32)
                + regs::REG_74::REG_VI_BT_D3_SEL.val(d3_sel as u32)
                + regs::REG_74::REG_VI_BT_D4_SEL.val(d4_sel as u32)
                + regs::REG_74::REG_VI_BT_D5_SEL.val(d5_sel as u32)
                + regs::REG_74::REG_VI_BT_D6_SEL.val(d6_sel as u32)
                + regs::REG_74::REG_VI_BT_D7_SEL.val(d7_sel as u32),
        );
    }

    Ok(())
}

/// 配置默认的 VI2 BT 数据引脚映射
///
/// 使用默认的 1:1 映射 (D0->0, D1->1, ..., D7->7)
///
/// # Arguments
/// * `vi` - VI 设备引用
pub fn configure_vi2_bt_data_pins_default(vi: &mut Vi) -> Result<(), ViError> {
    configure_vi2_bt_data_pins(vi, 0, 1, 2, 3, 4, 5, 6, 7)
}

// ============================================================================
// TTL 接口辅助函数
// ============================================================================

/// 创建 DC (Digital Camera) 配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
/// * `bit_width` - 数据位宽
/// * `use_sync_code` - 是否使用同步码
///
/// # Returns
/// DC 接口配置
pub fn dc_config(
    width: u16,
    height: u16,
    bit_width: TtlBitWidth,
    use_sync_code: bool,
) -> TtlConfig {
    TtlConfig {
        enable: true,
        bit_width,
        fmt_out: TtlBtFmtOut::default(),
        fmt_in: if use_sync_code {
            TtlInputFormat::SensorWithSync
        } else {
            TtlInputFormat::SensorVde
        },
        data_seq: TtlBtDataSeq::default(),
        vs_inv: false,
        hs_inv: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
    }
}

/// 创建 TTL BT.656 配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
///
/// # Returns
/// TTL BT.656 配置
pub fn ttl_bt656_config(width: u16, height: u16) -> TtlConfig {
    TtlConfig {
        enable: true,
        bit_width: TtlBitWidth::Bit8,
        fmt_out: TtlBtFmtOut::CbYCrY,
        fmt_in: TtlInputFormat::Bt656_9bit,
        data_seq: TtlBtDataSeq::Cb0Y0Cr0Y1,
        vs_inv: false,
        hs_inv: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
    }
}

/// 创建 TTL BT.1120 配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
///
/// # Returns
/// TTL BT.1120 配置
pub fn ttl_bt1120_config(width: u16, height: u16) -> TtlConfig {
    TtlConfig {
        enable: true,
        bit_width: TtlBitWidth::Bit16,
        fmt_out: TtlBtFmtOut::CbYCrY,
        fmt_in: TtlInputFormat::Bt1120_17bit,
        data_seq: TtlBtDataSeq::Cb0Y0Cr0Y1,
        vs_inv: false,
        hs_inv: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
    }
}

/// 创建 TTL BT.601 VHS 模式配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
/// * `is_16bit` - 是否为 16-bit 模式
/// * `blanking` - 消隐配置
///
/// # Returns
/// TTL BT.601 VHS 模式配置
pub fn ttl_bt601_vhs_config(
    width: u16,
    height: u16,
    is_16bit: bool,
    blanking: BlankingConfig,
) -> TtlConfig {
    TtlConfig {
        enable: true,
        bit_width: if is_16bit {
            TtlBitWidth::Bit16
        } else {
            TtlBitWidth::Bit8
        },
        fmt_out: TtlBtFmtOut::CbYCrY,
        fmt_in: if is_16bit {
            TtlInputFormat::Bt601_19bit_Vhs
        } else {
            TtlInputFormat::Bt601_11bit_Vhs
        },
        data_seq: TtlBtDataSeq::Cb0Y0Cr0Y1,
        vs_inv: false,
        hs_inv: false,
        img_size: ImageSize::new(width, height),
        blanking,
    }
}

/// 验证 TTL 配置参数
///
/// # Arguments
/// * `config` - TTL 接口配置
///
/// # Returns
/// * `Ok(())` - 配置有效
/// * `Err(ViError)` - 配置无效
pub fn validate_ttl_config(config: &TtlConfig) -> Result<(), ViError> {
    // 验证图像尺寸
    if !config.img_size.is_valid() {
        return Err(ViError::InvalidImageSize);
    }

    // 验证位宽与输入格式的匹配
    let is_16bit_format = config.fmt_in.is_16bit();
    let is_16bit_width = matches!(config.bit_width, TtlBitWidth::Bit16);

    if is_16bit_format && !is_16bit_width {
        return Err(ViError::InvalidConfig);
    }

    Ok(())
}
