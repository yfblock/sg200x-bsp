//! TTL/DVP/BT 并行接口配置

use super::*;

/// TTL 配置错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlError {
    InvalidPinFunc,
    UnsupportedMode,
}

/// 配置 TTL CMOS 属性
pub fn set_cmos_attr(
    ctx: &mut CifCtx,
    attr: &TtlDevAttr,
    img_size: &ImgSize,
) -> Result<(), TtlError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Ttl;

    if attr.vi == TtlViSrc::Vi2 {
        return Err(TtlError::UnsupportedMode);
    }

    // 配置引脚复用
    for (i, &func) in attr.func.iter().enumerate() {
        if func < 0 {
            continue;
        }
        if func >= 19 {
            return Err(TtlError::InvalidPinFunc);
        }
        // 实际的引脚复用配置需要调用底层函数
        // cif_set_ttl_pinmux(ctx, attr.vi, i, func);
    }

    // 构建 TTL 参数
    let ttl_param = ParamTtl {
        fmt: TtlFmt::VdeSensor,
        sensor_fmt: TtlSensorFmt::Bit12,
        fmt_out: TtlBtFmtOut::Cbycry,
        width: 0,
        height: 0,
        v_bp: attr.v_bp,
        h_bp: attr.h_bp,
        clk_inv: 0,
        vi_sel: 0,
        vi_from: attr.vi,
    };

    param.cfg = CifCfg::Ttl(ttl_param);
    param.hdr_en = false;

    ctx.set_config(param);

    Ok(())
}

/// 配置 BT1120 属性
pub fn set_bt1120_attr(
    ctx: &mut CifCtx,
    attr: &TtlDevAttr,
    img_size: &ImgSize,
    clk_edge: ClkEdge,
) -> Result<(), TtlError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Ttl;

    if attr.vi == TtlViSrc::Vi2 {
        return Err(TtlError::UnsupportedMode);
    }

    // 配置引脚复用
    for (i, &func) in attr.func.iter().enumerate() {
        if func < 0 {
            continue;
        }
        if func >= 19 {
            return Err(TtlError::InvalidPinFunc);
        }
    }

    // 构建 BT1120 参数
    let ttl_param = ParamTtl {
        fmt: TtlFmt::SyncPat17bBt1120,
        sensor_fmt: TtlSensorFmt::Bit12,
        fmt_out: TtlBtFmtOut::Cbycry,
        width: (img_size.width - 1) as u16,
        height: (img_size.height - 1) as u16,
        v_bp: if attr.v_bp == 0 { 9 } else { attr.v_bp },
        h_bp: if attr.h_bp == 0 { 8 } else { attr.h_bp },
        clk_inv: clk_edge as u32,
        vi_sel: TtlViMode::Bt1120 as u32,
        vi_from: attr.vi,
    };

    param.cfg = CifCfg::Ttl(ttl_param);
    param.hdr_en = false;

    ctx.set_config(param);

    Ok(())
}

/// 配置 BT601 属性
pub fn set_bt601_attr(
    ctx: &mut CifCtx,
    attr: &TtlDevAttr,
    img_size: &ImgSize,
    clk_edge: ClkEdge,
) -> Result<(), TtlError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Ttl;

    if attr.vi == TtlViSrc::Vi2 {
        return Err(TtlError::UnsupportedMode);
    }

    // 构建 BT601 参数
    let ttl_param = ParamTtl {
        fmt: TtlFmt::Vhs19bBt601,
        sensor_fmt: TtlSensorFmt::Bit12,
        fmt_out: TtlBtFmtOut::Cbycry,
        width: (img_size.width - 1) as u16,
        height: (img_size.height - 1) as u16,
        v_bp: if attr.v_bp == 0 { 0x23 } else { attr.v_bp },
        h_bp: if attr.h_bp == 0 { 0xbf } else { attr.h_bp },
        clk_inv: clk_edge as u32,
        vi_sel: TtlViMode::Bt601 as u32,
        vi_from: attr.vi,
    };

    param.cfg = CifCfg::Ttl(ttl_param);
    param.hdr_en = false;

    ctx.set_config(param);

    Ok(())
}

/// 配置 BT656 属性
pub fn set_bt656_attr(
    ctx: &mut CifCtx,
    attr: &TtlDevAttr,
    img_size: &ImgSize,
    clk_edge: ClkEdge,
) -> Result<(), TtlError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Ttl;

    // 构建 BT656 参数
    let ttl_param = ParamTtl {
        fmt: TtlFmt::SyncPat9bBt656,
        sensor_fmt: TtlSensorFmt::Bit12,
        fmt_out: TtlBtFmtOut::Cbycry,
        width: (img_size.width - 1) as u16,
        height: (img_size.height - 1) as u16,
        v_bp: if attr.v_bp == 0 { 0x0f } else { attr.v_bp },
        h_bp: if attr.h_bp == 0 { 0x0f } else { attr.h_bp },
        clk_inv: clk_edge as u32,
        vi_sel: TtlViMode::Bt656 as u32,
        vi_from: attr.vi,
    };

    param.cfg = CifCfg::Ttl(ttl_param);
    param.hdr_en = false;

    ctx.set_config(param);

    Ok(())
}

/// 配置 BT Demux 属性
pub fn set_bt_demux_attr(
    ctx: &mut CifCtx,
    attr: &BtDemuxAttr,
    img_size: &ImgSize,
    clk_edge: ClkEdge,
) -> Result<(), TtlError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::BtDmux;

    // 配置引脚复用
    for (i, &func) in attr.func.iter().enumerate().skip(4).take(8) {
        if func < 0 {
            continue;
        }
        if func >= 8 {
            return Err(TtlError::InvalidPinFunc);
        }
    }

    // 构建 BT Demux 参数
    let btdemux_param = ParamBtdemux {
        fmt: TtlFmt::SyncPat9bBt656,
        demux: attr.mode,
        width: (img_size.width - 1) as u16,
        height: (img_size.height - 1) as u16,
        v_fp: if attr.v_fp == 0 { 0x0f } else { attr.v_fp },
        h_fp: if attr.h_fp == 0 { 0x0f } else { attr.h_fp },
        v_bp: 0,
        h_bp: 0,
        clk_inv: clk_edge as u32,
        sync_code_part_a: attr.sync_code_part_a,
        sync_code_part_b: attr.sync_code_part_b,
        yc_exchg: attr.yc_exchg as u8,
    };

    param.cfg = CifCfg::Btdemux(btdemux_param);
    param.hdr_en = false;

    ctx.set_config(param);

    Ok(())
}

/// 设置 BT 格式输出
pub fn set_bt_fmt_out(ctx: &CifCtx, fmt_out: TtlBtFmtOut) {
    log::debug!("Set BT format output: {:?}", fmt_out);
}

/// 设置 TTL 引脚复用
pub fn set_ttl_pinmux(ctx: &CifCtx, vi: TtlViSrc, func: usize, pad: u32) {
    log::debug!("Set TTL pinmux: vi={:?}, func={}, pad={}", vi, func, pad);
}
