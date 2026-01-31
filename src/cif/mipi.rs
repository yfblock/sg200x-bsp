//! MIPI CSI-2 接口配置

use super::*;

/// MIPI 配置错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipiError {
    InvalidLaneId,
    InvalidDataType,
    UnsupportedMode,
}

/// 配置 MIPI 属性
///
/// 这是从 C 代码 `_cif_set_attr_mipi` 迁移的核心逻辑
pub fn set_mipi_attr(
    ctx: &mut CifCtx,
    attr: &MipiDevAttr,
    img_size: &ImgSize,
) -> Result<(), MipiError> {
    let mut param = CifParam::default();
    param.cif_type = CifType::Csi;

    // 配置数据格式
    let (fmt, decode_type) = match attr.raw_data_type {
        RawDataType::Raw8Bit => (CsiFmt::Raw8, 0x2A),
        RawDataType::Raw10Bit => (CsiFmt::Raw10, 0x2B),
        RawDataType::Raw12Bit => (CsiFmt::Raw12, 0x2C),
        RawDataType::Yuv422_8Bit => (CsiFmt::Yuv422_8b, 0x1E),
        RawDataType::Yuv422_10Bit => (CsiFmt::Yuv422_10b, 0x1F),
    };

    // 配置 Lane ID
    let mut lane_count = 0;
    for (i, &lane_id) in attr.lane_id.iter().enumerate() {
        if lane_id < 0 {
            continue;
        }
        if lane_id >= 6 {
            return Err(MipiError::InvalidLaneId);
        }
        // 这里应该调用底层的 lane 配置函数
        // cif_set_lane_id(ctx, i, lane_id, attr.pn_swap[i]);
        lane_count += 1;
    }

    if lane_count == 0 {
        return Err(MipiError::InvalidLaneId);
    }

    // 配置 HDR 模式
    let (hdr_mode, hdr_en) = match attr.hdr_mode {
        MipiHdrMode::None => (CsiHdrMode::Vc, false),
        MipiHdrMode::Dt => (CsiHdrMode::Dt, true),
        MipiHdrMode::Vc => (CsiHdrMode::Vc, true),
        MipiHdrMode::Dol => (CsiHdrMode::Dol, true),
        MipiHdrMode::Manual => {
            // 手动 HDR 模式需要额外配置
            (CsiHdrMode::Vc, true)
        }
    };

    // 配置 VC 映射
    let vc_mapping = if attr.demux.demux_en {
        attr.demux.vc_mapping
    } else {
        [0, 1, 2, 3]
    };

    // 构建 CSI 参数
    let csi_param = ParamCsi {
        lane_num: (lane_count - 1) as u16,
        fmt,
        vs_gen_mode: CsiVsGenMode::Fs,
        hdr_mode,
        data_type: [attr.data_type[0] as u16, attr.data_type[1] as u16],
        decode_type,
        vc_mapping,
    };

    param.cfg = CifCfg::Csi(csi_param);
    param.hdr_en = hdr_en;

    ctx.set_config(param);

    Ok(())
}

/// 配置 HS Settle 时间
pub fn set_hs_settle(ctx: &CifCtx, hs_settle: u8) {
    // 实际的寄存器写入操作
    // 这里需要根据具体的寄存器定义来实现
    log::debug!("Set HS settle to {}", hs_settle);
}

/// 检查 CSI 中断状态
pub fn check_csi_int_sts(ctx: &CifCtx, mask: u32) -> bool {
    // 读取中断状态寄存器并检查
    // 实际实现需要访问硬件寄存器
    false
}

/// 屏蔽 CSI 中断
pub fn mask_csi_int_sts(ctx: &CifCtx, mask: u32) {
    // 写入中断屏蔽寄存器
    log::debug!("Mask CSI interrupt: 0x{:x}", mask);
}

/// 取消屏蔽 CSI 中断
pub fn unmask_csi_int_sts(ctx: &CifCtx, mask: u32) {
    // 写入中断屏蔽寄存器
    log::debug!("Unmask CSI interrupt: 0x{:x}", mask);
}

/// 清除 CSI 中断状态
pub fn clear_csi_int_sts(ctx: &CifCtx) {
    // 清除中断状态寄存器
    log::debug!("Clear CSI interrupt status");
}
