//! MIPI RX CSI 控制器模块
//!
//! 本模块提供 MIPI CSI-2 控制器的配置和控制功能

use super::regs::*;
use super::types::*;
use tock_registers::interfaces::{ReadWriteable, Readable};

/// CSI 控制器驱动
pub struct MipiRxCsi {
    /// CSI 寄存器
    pub regs: &'static MipiRxCsiRegs,
    /// 设备编号
    devno: u8,
}

impl MipiRxCsi {
    /// 创建新的 CSI 控制器驱动实例
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效且可访问
    ///
    /// # 参数
    /// - `devno`: 设备编号 (0 或 1)
    pub fn new(devno: u8) -> Option<Self> {
        let regs = unsafe { csi_regs(devno as usize)? };
        Some(Self { regs, devno })
    }

    /// 从指定基地址创建 CSI 控制器驱动实例
    ///
    /// # Safety
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize, devno: u8) -> Self {
        Self {
            regs: &*(base as *const MipiRxCsiRegs),
            devno,
        }
    }

    /// 获取设备编号
    pub fn devno(&self) -> u8 {
        self.devno
    }

    // ========================================================================
    // Lane 配置
    // ========================================================================

    /// 设置 Lane 模式
    ///
    /// # 参数
    /// - `mode`: Lane 模式 (1/2/4/8 lane)
    pub fn set_lane_mode(&self, mode: LaneMode) {
        self.regs
            .reg_00
            .modify(CSI_REG_00::LANE_MODE.val(mode as u32));
    }

    /// 获取 Lane 模式
    pub fn get_lane_mode(&self) -> LaneMode {
        let val = self.regs.reg_00.read(CSI_REG_00::LANE_MODE) as u8;
        LaneMode::from_u8(val).unwrap_or(LaneMode::Lane2)
    }

    // ========================================================================
    // ECC/VC 配置
    // ========================================================================

    /// 设置是否忽略 ECC 错误
    ///
    /// # 参数
    /// - `ignore`: true = 即使 ECC 错误也继续处理
    pub fn set_ignore_ecc(&self, ignore: bool) {
        self.regs
            .reg_00
            .modify(CSI_REG_00::IGNORE_ECC.val(if ignore { 1 } else { 0 }));
    }

    /// 使能 VC 检查
    ///
    /// # 参数
    /// - `enable`: 是否使能 VC 检查
    /// - `vc_set`: VC 设置值 (仅当 enable=true 时有效)
    pub fn set_vc_check(&self, enable: bool, vc_set: u8) {
        self.regs.reg_00.modify(
            CSI_REG_00::VC_CHECK.val(if enable { 1 } else { 0 })
                + CSI_REG_00::VC_SET.val(vc_set as u32),
        );
    }

    /// 设置 Line Start 发送模式
    ///
    /// # 参数
    /// - `use_ls_le`: true = 使用 LS/LE 包创建 hsync, false = 由控制器创建
    pub fn set_line_start_sent(&self, use_ls_le: bool) {
        self.regs
            .reg_00
            .modify(CSI_REG_00::LINE_START_SENT.val(if use_ls_le { 1 } else { 0 }));
    }

    // ========================================================================
    // 中断配置
    // ========================================================================

    /// 设置中断掩码
    ///
    /// # 参数
    /// - `mask`: 中断掩码 (bit 对应各中断类型)
    pub fn set_interrupt_mask(&self, mask: u8) {
        self.regs
            .reg_04
            .modify(CSI_REG_04::INTR_MASK.val(mask as u32));
    }

    /// 清除中断
    ///
    /// # 参数
    /// - `mask`: 要清除的中断掩码
    pub fn clear_interrupt(&self, mask: u8) {
        self.regs
            .reg_04
            .modify(CSI_REG_04::INTR_CLR.val(mask as u32));
    }

    /// 清除所有中断
    pub fn clear_all_interrupts(&self) {
        self.regs.reg_04.modify(CSI_REG_04::INTR_CLR.val(0xFF));
    }

    /// 获取中断状态
    pub fn get_interrupt_status(&self) -> CsiInterruptStatus {
        let status = self.regs.reg_60.read(CSI_REG_60::INTR_STATUS) as u8;
        CsiInterruptStatus::from(status)
    }

    // ========================================================================
    // HDR 配置
    // ========================================================================

    /// 使能 HDR 模式
    ///
    /// # 参数
    /// - `enable`: 是否使能 HDR
    /// - `id_mode`: true = ID 模式, false = VC 模式
    pub fn set_hdr_mode(&self, enable: bool, id_mode: bool) {
        self.regs.reg_04.modify(
            CSI_REG_04::HDR_EN.val(if enable { 1 } else { 0 })
                + CSI_REG_04::HDR_MODE.val(if id_mode { 1 } else { 0 }),
        );
    }

    /// 设置 HDR ID 移除选项
    ///
    /// # 参数
    /// - `remove_else`: 移除未识别的 ID 行
    /// - `remove_ob`: 移除 OB 行
    pub fn set_hdr_id_remove(&self, remove_else: bool, remove_ob: bool) {
        self.regs.reg_04.modify(
            CSI_REG_04::ID_RM_ELSE.val(if remove_else { 1 } else { 0 })
                + CSI_REG_04::ID_RM_OB.val(if remove_ob { 1 } else { 0 }),
        );
    }

    /// 配置 HDR DT 模式
    ///
    /// # 参数
    /// - `enable`: 使能 DT 模式
    /// - `format`: DT 格式
    /// - `lef_dt`: LEF Data Type
    /// - `sef_dt`: SEF Data Type
    pub fn configure_hdr_dt(&self, enable: bool, format: u8, lef_dt: u8, sef_dt: u8) {
        self.regs.reg_74.modify(
            CSI_REG_74::HDR_DT_MODE.val(if enable { 1 } else { 0 })
                + CSI_REG_74::HDR_DT_FORMAT.val(format as u32)
                + CSI_REG_74::HDR_DT_LEF.val(lef_dt as u32)
                + CSI_REG_74::HDR_DT_SEF.val(sef_dt as u32),
        );
    }

    // ========================================================================
    // HDR ID 配置
    // ========================================================================

    /// 配置 HDR OB ID (n0)
    ///
    /// # 参数
    /// - `lef_id`: LEF OB ID
    /// - `sef_id`: SEF OB ID
    pub fn configure_hdr_ob_id_n0(&self, lef_id: u16, sef_id: u16) {
        self.regs.reg_08.modify(
            CSI_REG_08::N0_OB_LEF.val(lef_id as u32) + CSI_REG_08::N0_OB_SEF.val(sef_id as u32),
        );
    }

    /// 配置 HDR Active ID (n0)
    ///
    /// # 参数
    /// - `lef_id`: LEF Active ID
    /// - `sef_id`: SEF Active ID
    pub fn configure_hdr_active_id_n0(&self, lef_id: u16, sef_id: u16) {
        self.regs
            .reg_0c
            .modify(CSI_REG_0C::N0_LEF.val(lef_id as u32));
        self.regs
            .reg_1c
            .modify(CSI_REG_1C::N0_SEF.val(sef_id as u32));
    }

    /// 配置 HDR OB ID (n1)
    ///
    /// # 参数
    /// - `lef_id`: LEF OB ID
    /// - `sef_id`: SEF OB ID
    pub fn configure_hdr_ob_id_n1(&self, lef_id: u16, sef_id: u16) {
        self.regs
            .reg_0c
            .modify(CSI_REG_0C::N1_OB_LEF.val(lef_id as u32));
        self.regs
            .reg_10
            .modify(CSI_REG_10::N1_OB_SEF.val(sef_id as u32));
    }

    /// 配置 HDR Active ID (n1)
    ///
    /// # 参数
    /// - `lef_id`: LEF Active ID
    /// - `sef_id`: SEF Active ID
    pub fn configure_hdr_active_id_n1(&self, lef_id: u16, sef_id: u16) {
        self.regs
            .reg_10
            .modify(CSI_REG_10::N1_LEF.val(lef_id as u32));
        self.regs
            .reg_1c
            .modify(CSI_REG_1C::N1_SEF.val(sef_id as u32));
    }

    /// 配置 HDR SEF2 ID
    ///
    /// # 参数
    /// - `n0_active`: n0 SEF2 Active ID
    /// - `n1_active`: n1 SEF2 Active ID
    /// - `n0_ob`: n0 SEF2 OB ID
    /// - `n1_ob`: n1 SEF2 OB ID
    pub fn configure_hdr_sef2_id(&self, n0_active: u16, n1_active: u16, n0_ob: u16, n1_ob: u16) {
        self.regs.reg_20.modify(
            CSI_REG_20::N0_SEF2.val(n0_active as u32) + CSI_REG_20::N1_SEF2.val(n1_active as u32),
        );
        self.regs.reg_24.modify(
            CSI_REG_24::N0_OB_SEF2.val(n0_ob as u32) + CSI_REG_24::N1_OB_SEF2.val(n1_ob as u32),
        );
    }

    // ========================================================================
    // BLC 配置
    // ========================================================================

    /// 配置 BLC (Black Level Calibration)
    ///
    /// # 参数
    /// - `enable`: 使能 BLC
    /// - `data_type`: 数据类型
    /// - `format`: 数据格式
    pub fn configure_blc(&self, enable: bool, data_type: u8, format: RawDataType) {
        self.regs.reg_14.modify(
            CSI_REG_14::BLC_EN.val(if enable { 1 } else { 0 })
                + CSI_REG_14::BLC_DT.val(data_type as u32)
                + CSI_REG_14::BLC_FORMAT_SET.val(format.blc_format() as u32),
        );
    }

    // ========================================================================
    // VC 映射配置
    // ========================================================================

    /// 配置 VC 映射
    ///
    /// # 参数
    /// - `mapping`: VC 映射配置
    pub fn configure_vc_mapping(&self, mapping: &VcMapping) {
        self.regs.reg_18.modify(
            CSI_REG_18::VC_MAP_CH00.val(mapping.ch00 as u32)
                + CSI_REG_18::VC_MAP_CH01.val(mapping.ch01 as u32)
                + CSI_REG_18::VC_MAP_CH10.val(mapping.ch10 as u32)
                + CSI_REG_18::VC_MAP_CH11.val(mapping.ch11 as u32),
        );
    }

    /// 设置默认 VC 映射 (直通)
    pub fn set_default_vc_mapping(&self) {
        self.configure_vc_mapping(&VcMapping::default_passthrough());
    }

    // ========================================================================
    // VS 生成配置
    // ========================================================================

    /// 设置 VS 生成模式
    ///
    /// # 参数
    /// - `mode`: VS 生成模式
    pub fn set_vs_gen_mode(&self, mode: VsGenMode) {
        self.regs
            .reg_70
            .modify(CSI_REG_70::VS_GEN_MODE.val(mode as u32));
    }

    /// 设置 VS 生成是否由指定 VC 触发
    ///
    /// # 参数
    /// - `by_vcset`: true = 仅由指定 VC 的短包触发, false = 由所有 VC 短包触发
    pub fn set_vs_gen_by_vcset(&self, by_vcset: bool) {
        self.regs
            .reg_70
            .modify(CSI_REG_70::VS_GEN_BY_VCSET.val(if by_vcset { 1 } else { 0 }));
    }

    // ========================================================================
    // 状态查询
    // ========================================================================

    /// 获取 CSI 状态
    pub fn get_status(&self) -> CsiStatus {
        let reg = self.regs.reg_40.get();
        CsiStatus {
            ecc_no_error: (reg & 0x01) != 0,
            ecc_corrected: (reg & 0x02) != 0,
            ecc_error: (reg & 0x04) != 0,
            crc_error: (reg & 0x10) != 0,
            wc_error: (reg & 0x20) != 0,
            fifo_full: (reg & 0x100) != 0,
            decode_format: ((reg >> 16) & 0x3F) as u8,
        }
    }

    /// 检查是否有错误
    pub fn has_error(&self) -> bool {
        self.get_status().has_error()
    }

    /// 获取解码格式
    pub fn get_decode_format(&self) -> u8 {
        self.regs.reg_40.read(CSI_REG_40::DECODE_FORMAT) as u8
    }

    // ========================================================================
    // 高级配置
    // ========================================================================

    /// 应用 MIPI RX 设备属性配置
    ///
    /// # 参数
    /// - `attr`: 设备属性
    pub fn apply_dev_attr(&self, attr: &MipiRxDevAttr) -> Result<(), MipiRxError> {
        // 设置 Lane 模式
        self.set_lane_mode(attr.lane_mode);

        // 配置 VC 映射为默认直通
        self.set_default_vc_mapping();

        // 配置 HDR 模式
        match attr.hdr_mode {
            HdrMode::None => {
                self.set_hdr_mode(false, false);
            }
            HdrMode::Vc => {
                self.set_hdr_mode(true, false);
            }
            HdrMode::Id => {
                self.set_hdr_mode(true, true);
            }
            HdrMode::Dt => {
                self.set_hdr_mode(true, false);
                self.configure_hdr_dt(
                    true,
                    attr.data_type.blc_format(),
                    attr.data_type.csi_data_type(),
                    attr.data_type.csi_data_type(),
                );
            }
            _ => {}
        }

        // 设置 VS 生成模式
        self.set_vs_gen_mode(VsGenMode::ByFsFe);

        // 清除所有中断
        self.clear_all_interrupts();

        Ok(())
    }

    /// 初始化 CSI 控制器
    pub fn init(&self) {
        // 设置默认配置
        self.set_lane_mode(LaneMode::Lane2);
        self.set_ignore_ecc(false);
        self.set_vc_check(false, 0);
        self.set_line_start_sent(false);
        self.set_hdr_mode(false, false);
        self.set_hdr_id_remove(true, true);
        self.set_default_vc_mapping();
        self.set_vs_gen_mode(VsGenMode::ByFsFe);
        self.clear_all_interrupts();
        self.set_interrupt_mask(0x1F); // 屏蔽所有中断
    }

    /// 复位 CSI 控制器
    pub fn reset(&self) {
        // 清除所有中断
        self.clear_all_interrupts();
        // 重新初始化
        self.init();
    }
}
