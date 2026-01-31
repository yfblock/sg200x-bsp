//! LVDS/Sub-LVDS/HiSPI 接口配置

use super::*;

/// LVDS 配置错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LvdsError {
    InvalidLaneId,
    UnsupportedSyncMode,
    UnsupportedHdrMode,
}

/// 配置 Sub-LVDS 属性
pub fn set_sublvds_attr(
    ctx: &mut CifCtx,
    attr: &LvdsDevAttr,
    img_size: &ImgSize,
) -> Result<(), LvdsError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Sublvds;

    // 配置数据格式
    let fmt = match attr.raw_data_type {
        RawDataType::Raw8Bit => SublvdsFmt::Bit8,
        RawDataType::Raw10Bit => SublvdsFmt::Bit10,
        RawDataType::Raw12Bit => SublvdsFmt::Bit12,
        _ => return Err(LvdsError::UnsupportedSyncMode),
    };

    // 配置端序
    let (endian, wrap_endian) = match (attr.data_endian, attr.sync_code_endian) {
        (LvdsBitEndian::Big, LvdsBitEndian::Big) => (CifEndian::Msb, CifEndian::Msb),
        (LvdsBitEndian::Little, LvdsBitEndian::Big) => (CifEndian::Lsb, CifEndian::Msb),
        (LvdsBitEndian::Big, LvdsBitEndian::Little) => (CifEndian::Lsb, CifEndian::Lsb),
        (LvdsBitEndian::Little, LvdsBitEndian::Little) => (CifEndian::Msb, CifEndian::Lsb),
    };

    // 检查同步模式
    if attr.sync_mode != LvdsSyncMode::Sav {
        return Err(LvdsError::UnsupportedSyncMode);
    }

    // 配置 Lane ID
    let mut lane_count = 0;
    for (i, &lane_id) in attr.lane_id.iter().enumerate() {
        if lane_id < 0 {
            continue;
        }
        if lane_id >= 6 {
            return Err(LvdsError::InvalidLaneId);
        }
        lane_count += 1;
    }

    if lane_count == 0 {
        return Err(LvdsError::InvalidLaneId);
    }

    // 配置同步码
    let mut sync_code = SyncCode::default();
    let mut slvds_sync = SublvdsSyncCode::default();
    slvds_sync.n0_lef_sav = attr.sync_code[0][0][0] as u16;
    slvds_sync.n0_lef_eav = attr.sync_code[0][0][1] as u16;
    slvds_sync.n1_lef_sav = attr.sync_code[0][0][2] as u16;
    slvds_sync.n1_lef_eav = attr.sync_code[0][0][3] as u16;
    slvds_sync.n0_sef_sav = attr.sync_code[0][1][0] as u16;
    slvds_sync.n0_sef_eav = attr.sync_code[0][1][1] as u16;
    slvds_sync.n1_sef_sav = attr.sync_code[0][1][2] as u16;
    slvds_sync.n1_sef_eav = attr.sync_code[0][1][3] as u16;
    slvds_sync.n0_lsef_sav = attr.sync_code[0][2][0] as u16;
    slvds_sync.n0_lsef_eav = attr.sync_code[0][2][1] as u16;
    slvds_sync.n1_lsef_sav = attr.sync_code[0][2][2] as u16;
    slvds_sync.n1_lsef_eav = attr.sync_code[0][2][3] as u16;
    sync_code.slvds = Some(slvds_sync);

    // 配置 HDR 模式
    let (hdr_mode, hdr_en) = match attr.hdr_mode {
        HdrMode::None => (SublvdsHdr::Pat1, false),
        HdrMode::Dol2F | HdrMode::Dol3F => {
            // HDR 模式需要根据 vsync_type 配置
            match attr.vsync_type.sync_type {
                LvdsVsyncType::Normal => (SublvdsHdr::Pat1, true),
                LvdsVsyncType::Hconnect => (SublvdsHdr::Pat2, true),
                _ => return Err(LvdsError::UnsupportedHdrMode),
            }
        }
        _ => return Err(LvdsError::UnsupportedHdrMode),
    };

    // 构建 Sub-LVDS 参数
    let sublvds_param = ParamSublvds {
        v_front_porch: 0,
        lane_num: (lane_count - 1) as u16,
        hdr_hblank: [attr.vsync_type.hblank1, attr.vsync_type.hblank2],
        h_size: img_size.width as u16,
        hdr_mode,
        endian,
        wrap_endian,
        fmt,
        hdr_v_fp: 0,
        sync_code,
    };

    param.cfg = CifCfg::Sublvds(sublvds_param);
    param.hdr_en = hdr_en;

    ctx.set_config(param);

    Ok(())
}

/// 配置 HiSPI 属性
pub fn set_hispi_attr(
    ctx: &mut CifCtx,
    attr: &LvdsDevAttr,
    img_size: &ImgSize,
) -> Result<(), LvdsError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Hispi;

    // 配置数据格式
    let fmt = match attr.raw_data_type {
        RawDataType::Raw8Bit => SublvdsFmt::Bit8,
        RawDataType::Raw10Bit => SublvdsFmt::Bit10,
        RawDataType::Raw12Bit => SublvdsFmt::Bit12,
        _ => return Err(LvdsError::UnsupportedSyncMode),
    };

    // 配置端序
    let (endian, wrap_endian) = match (attr.data_endian, attr.sync_code_endian) {
        (LvdsBitEndian::Big, LvdsBitEndian::Big) => (CifEndian::Msb, CifEndian::Msb),
        (LvdsBitEndian::Little, LvdsBitEndian::Big) => (CifEndian::Lsb, CifEndian::Msb),
        (LvdsBitEndian::Big, LvdsBitEndian::Little) => (CifEndian::Lsb, CifEndian::Lsb),
        (LvdsBitEndian::Little, LvdsBitEndian::Little) => (CifEndian::Msb, CifEndian::Lsb),
    };

    // 配置模式
    let mode = match attr.sync_mode {
        LvdsSyncMode::Sof => HispiMode::PktSp,
        LvdsSyncMode::Sav => HispiMode::StreamSp,
    };

    // 配置 Lane ID
    let mut lane_count = 0;
    for &lane_id in attr.lane_id.iter() {
        if lane_id < 0 {
            continue;
        }
        if lane_id >= 6 {
            return Err(LvdsError::InvalidLaneId);
        }
        lane_count += 1;
    }

    if lane_count == 0 {
        return Err(LvdsError::InvalidLaneId);
    }

    // 配置同步码
    let mut sync_code = SyncCode::default();
    let mut hispi_sync = HispiSyncCode::default();
    hispi_sync.t1_sol = attr.sync_code[0][0][0] as u16;
    hispi_sync.t1_eol = attr.sync_code[0][0][1] as u16;
    hispi_sync.t1_sof = attr.sync_code[0][0][2] as u16;
    hispi_sync.t1_eof = attr.sync_code[0][0][3] as u16;
    hispi_sync.t2_sol = attr.sync_code[0][1][0] as u16;
    hispi_sync.t2_eol = attr.sync_code[0][1][1] as u16;
    hispi_sync.t2_sof = attr.sync_code[0][1][2] as u16;
    hispi_sync.t2_eof = attr.sync_code[0][1][3] as u16;
    hispi_sync.vsync_gen = hispi_sync.t1_sof;
    sync_code.hispi = Some(hispi_sync);

    // 构建 HiSPI 参数
    let hispi_param = ParamHispi {
        lane_num: (lane_count - 1) as u16,
        h_size: img_size.width as u16,
        mode,
        endian,
        wrap_endian,
        fmt,
        sync_code,
    };

    param.cfg = CifCfg::Hispi(hispi_param);
    param.hdr_en = attr.hdr_mode != HdrMode::None;

    ctx.set_config(param);

    Ok(())
}

/// 设置 LVDS VSync 生成
pub fn set_lvds_vsync_gen(ctx: &CifCtx, fp: u32) {
    log::debug!("Set LVDS VSync gen: {}", fp);
}

/// 设置 LVDS 端序
pub fn set_lvds_endian(ctx: &CifCtx, mac: CifEndian, wrap: CifEndian) {
    log::debug!("Set LVDS endian: mac={:?}, wrap={:?}", mac, wrap);
}
