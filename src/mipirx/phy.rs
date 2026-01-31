//! MIPI RX PHY 层配置
//!
//! 本模块提供 MIPI RX PHY 层的配置和控制功能

use super::regs::*;
use super::types::*;
use tock_registers::interfaces::{ReadWriteable, Readable};

/// MIPI RX PHY 驱动
pub struct MipiRxPhy {
    /// PHY 顶层寄存器
    phy_top: &'static MipiRxPhyTopRegs,
    /// 4-Lane DPHY 寄存器
    dphy_4l: &'static MipiRxDphyRegs,
    /// 2-Lane DPHY 寄存器
    dphy_2l: &'static MipiRxDphyRegs,
}

impl MipiRxPhy {
    /// 创建新的 PHY 驱动实例
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效且可访问
    pub unsafe fn new() -> Self {
        Self {
            phy_top: phy_top_regs(),
            dphy_4l: dphy_4l_regs(),
            dphy_2l: dphy_2l_regs(),
        }
    }

    /// 从指定基地址创建 PHY 驱动实例
    ///
    /// # Safety
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_addresses(
        phy_top_base: usize,
        dphy_4l_base: usize,
        dphy_2l_base: usize,
    ) -> Self {
        Self {
            phy_top: &*(phy_top_base as *const MipiRxPhyTopRegs),
            dphy_4l: &*(dphy_4l_base as *const MipiRxDphyRegs),
            dphy_2l: &*(dphy_2l_base as *const MipiRxDphyRegs),
        }
    }

    // ========================================================================
    // PHY 顶层配置
    // ========================================================================

    /// 设置 PHY 模式
    ///
    /// # 参数
    /// - `mode`: PHY 模式 (1C4D 或 1C2D+1C2D)
    pub fn set_phy_mode(&self, mode: PhyMode) {
        self.phy_top
            .reg_30
            .modify(PHY_REG_30::SENSOR_PHY_MODE.val(mode as u32));
    }

    /// 获取 PHY 模式
    pub fn get_phy_mode(&self) -> PhyMode {
        let val = self.phy_top.reg_30.read(PHY_REG_30::SENSOR_PHY_MODE) as u8;
        PhyMode::from_u8(val).unwrap_or(PhyMode::Mode1C4D)
    }

    /// 使能 PHY 电源
    pub fn power_on(&self) {
        // 清除 power down 位
        self.phy_top.reg_00.modify(PHY_REG_00::PD_IBIAS::CLEAR);
        self.phy_top.reg_00.modify(PHY_REG_00::PD_RXLP.val(0));
    }

    /// 关闭 PHY 电源
    pub fn power_off(&self) {
        // 设置 power down 位
        self.phy_top.reg_00.modify(PHY_REG_00::PD_IBIAS::SET);
        self.phy_top.reg_00.modify(PHY_REG_00::PD_RXLP.val(0x3F));
    }

    /// 设置时钟通道选择
    ///
    /// # 参数
    /// - `channel`: 时钟通道选择值 (6-bit)
    pub fn set_clk_channel(&self, channel: u8) {
        self.phy_top
            .reg_04
            .modify(PHY_REG_04::SEL_CLK_CHANNEL.val(channel as u32));
    }

    /// 使能 MIPI PLL 时钟到 CSI
    pub fn enable_mipimpll_clk(&self) {
        self.phy_top
            .reg_04
            .modify(PHY_REG_04::MIPIMPLL_CLK_CSI_EN::SET);
    }

    /// 禁用 MIPI PLL 时钟到 CSI
    pub fn disable_mipimpll_clk(&self) {
        self.phy_top
            .reg_04
            .modify(PHY_REG_04::MIPIMPLL_CLK_CSI_EN::CLEAR);
    }

    /// 设置 Lane 时钟反转
    ///
    /// # 参数
    /// - `lane`: Lane ID
    /// - `invert`: 是否反转
    pub fn set_lane_clk_invert(&self, lane: LaneId, invert: bool) {
        let val = if invert { 1 } else { 0 };
        match lane {
            LaneId::Lane0 => self.phy_top.reg_80.modify(PHY_REG_80::D0_CLK_INV.val(val)),
            LaneId::Lane1 => self.phy_top.reg_80.modify(PHY_REG_80::D1_CLK_INV.val(val)),
            LaneId::Lane2 => self.phy_top.reg_80.modify(PHY_REG_80::D2_CLK_INV.val(val)),
            LaneId::Lane3 => self.phy_top.reg_80.modify(PHY_REG_80::D3_CLK_INV.val(val)),
            LaneId::Lane4 => self.phy_top.reg_80.modify(PHY_REG_80::D4_CLK_INV.val(val)),
            LaneId::Lane5 => self.phy_top.reg_80.modify(PHY_REG_80::D5_CLK_INV.val(val)),
        }
    }

    /// 获取 Lane 校准结果
    ///
    /// # 参数
    /// - `lane`: Lane ID
    pub fn get_lane_cal_result(&self, lane: LaneId) -> u32 {
        match lane {
            LaneId::Lane0 => self.phy_top.reg_34.read(PHY_CAL_RESULT::CAL_RESULT),
            LaneId::Lane1 => self.phy_top.reg_38.read(PHY_CAL_RESULT::CAL_RESULT),
            LaneId::Lane2 => self.phy_top.reg_3c.read(PHY_CAL_RESULT::CAL_RESULT),
            LaneId::Lane3 => self.phy_top.reg_40.read(PHY_CAL_RESULT::CAL_RESULT),
            LaneId::Lane4 => self.phy_top.reg_44.read(PHY_CAL_RESULT::CAL_RESULT),
            LaneId::Lane5 => self.phy_top.reg_48.read(PHY_CAL_RESULT::CAL_RESULT),
        }
    }

    // ========================================================================
    // CAM0 时序配置
    // ========================================================================

    /// 配置 CAM0 垂直时序
    ///
    /// # 参数
    /// - `vtt`: 垂直总行数
    /// - `vs_start`: VS 起始位置
    /// - `vs_stop`: VS 结束位置
    pub fn configure_cam0_vtiming(&self, vtt: u16, vs_start: u16, vs_stop: u16) {
        self.phy_top.reg_a0.modify(
            PHY_REG_A0::CAM0_VTT.val(vtt as u32)
                + PHY_REG_A0::CAM0_VS_STR.val(vs_start as u32),
        );
        self.phy_top
            .reg_a4
            .modify(PHY_REG_A4::CAM0_VS_STP.val(vs_stop as u32));
    }

    /// 配置 CAM0 水平时序
    ///
    /// # 参数
    /// - `htt`: 水平总像素数
    /// - `hs_start`: HS 起始位置
    /// - `hs_stop`: HS 结束位置
    pub fn configure_cam0_htiming(&self, htt: u16, hs_start: u16, hs_stop: u16) {
        self.phy_top
            .reg_a4
            .modify(PHY_REG_A4::CAM0_HTT.val(htt as u32));
        self.phy_top.reg_a8.modify(
            PHY_REG_A8::CAM0_HS_STR.val(hs_start as u32)
                + PHY_REG_A8::CAM0_HS_STP.val(hs_stop as u32),
        );
    }

    /// 配置 CAM0 极性
    ///
    /// # 参数
    /// - `vs_pol`: VS 极性 (true = 高有效)
    /// - `hs_pol`: HS 极性 (true = 高有效)
    pub fn configure_cam0_polarity(&self, vs_pol: bool, hs_pol: bool) {
        self.phy_top.reg_ac.modify(
            PHY_REG_AC::CAM0_VS_POL.val(if vs_pol { 1 } else { 0 })
                + PHY_REG_AC::CAM0_HS_POL.val(if hs_pol { 1 } else { 0 }),
        );
    }

    /// 使能 CAM0 时序生成器
    pub fn enable_cam0_tgen(&self) {
        self.phy_top.reg_ac.modify(PHY_REG_AC::CAM0_TGEN_EN::SET);
    }

    /// 禁用 CAM0 时序生成器
    pub fn disable_cam0_tgen(&self) {
        self.phy_top.reg_ac.modify(PHY_REG_AC::CAM0_TGEN_EN::CLEAR);
    }

    // ========================================================================
    // DPHY 配置
    // ========================================================================

    /// 获取 DPHY 寄存器引用
    fn get_dphy(&self, port: u8) -> &MipiRxDphyRegs {
        if port == 0 {
            self.dphy_4l
        } else {
            self.dphy_2l
        }
    }

    /// 设置 Sensor 模式
    ///
    /// # 参数
    /// - `port`: 端口号 (0: 4L, 1: 2L)
    /// - `mode`: Sensor 模式
    pub fn set_sensor_mode(&self, port: u8, mode: SensorMode) {
        let dphy = self.get_dphy(port);
        dphy.reg_00
            .modify(DPHY_REG_00::SENSOR_MODE.val(mode as u32));
    }

    /// 获取 Sensor 模式
    ///
    /// # 参数
    /// - `port`: 端口号
    pub fn get_sensor_mode(&self, port: u8) -> SensorMode {
        let dphy = self.get_dphy(port);
        let val = dphy.reg_00.read(DPHY_REG_00::SENSOR_MODE) as u8;
        SensorMode::from_u8(val).unwrap_or(SensorMode::Csi)
    }

    /// 配置 CSI Lane 选择
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `lane_sel`: Lane 选择数组 [D0, D1, D2, D3]
    pub fn configure_csi_lane_select(&self, port: u8, lane_sel: [u8; 4]) {
        let dphy = self.get_dphy(port);
        dphy.reg_04.modify(
            DPHY_REG_04::CSI_LANE_D0_SEL.val(lane_sel[0] as u32)
                + DPHY_REG_04::CSI_LANE_D1_SEL.val(lane_sel[1] as u32)
                + DPHY_REG_04::CSI_LANE_D2_SEL.val(lane_sel[2] as u32)
                + DPHY_REG_04::CSI_LANE_D3_SEL.val(lane_sel[3] as u32),
        );
    }

    /// 配置 CSI 时钟 Lane
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `clk_sel`: 时钟 Lane 选择
    /// - `pn_swap`: PN 交换
    /// - `phase`: 时钟相位
    pub fn configure_csi_clk_lane(&self, port: u8, clk_sel: u8, pn_swap: bool, phase: u8) {
        let dphy = self.get_dphy(port);
        dphy.reg_08.modify(
            DPHY_REG_08::CSI_LANE_CK_SEL.val(clk_sel as u32)
                + DPHY_REG_08::CSI_LANE_CK_PNSWAP.val(if pn_swap { 1 } else { 0 })
                + DPHY_REG_08::CSI_CK_PHASE.val(phase as u32),
        );
    }

    /// 配置 CSI 数据 Lane PN 交换
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `pn_swap`: PN 交换数组 [D0, D1, D2, D3]
    pub fn configure_csi_data_pn_swap(&self, port: u8, pn_swap: [bool; 4]) {
        let dphy = self.get_dphy(port);
        dphy.reg_08.modify(
            DPHY_REG_08::CSI_LANE_D0_PNSWAP.val(if pn_swap[0] { 1 } else { 0 })
                + DPHY_REG_08::CSI_LANE_D1_PNSWAP.val(if pn_swap[1] { 1 } else { 0 })
                + DPHY_REG_08::CSI_LANE_D2_PNSWAP.val(if pn_swap[2] { 1 } else { 0 })
                + DPHY_REG_08::CSI_LANE_D3_PNSWAP.val(if pn_swap[3] { 1 } else { 0 }),
        );
    }

    /// 设置 Deskew Lane 使能
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `enable`: Deskew Lane 使能配置
    pub fn set_deskew_lane_enable(&self, port: u8, enable: DeskewLaneEnable) {
        let dphy = self.get_dphy(port);
        dphy.reg_0c
            .modify(DPHY_REG_0C::DESKEW_LANE_EN.val(enable as u32));
    }

    // ========================================================================
    // Sub-LVDS 配置
    // ========================================================================

    /// 配置 Sub-LVDS 模式
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `bit_mode`: 位宽模式
    /// - `msb_first`: MSB 优先
    /// - `lane_enable`: Lane 使能位掩码
    pub fn configure_sublvds(&self, port: u8, bit_mode: SubLvdsBitMode, msb_first: bool, lane_enable: u8) {
        let dphy = self.get_dphy(port);
        dphy.reg_20.modify(
            DPHY_REG_20::SLVDS_BIT_MODE.val(bit_mode as u32)
                + DPHY_REG_20::SLVDS_INV_EN.val(if msb_first { 1 } else { 0 })
                + DPHY_REG_20::SLVDS_LANE_EN.val(lane_enable as u32),
        );
    }

    /// 配置 Sub-LVDS 同步码
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `sav_1st`: 同步码第一个符号
    /// - `sav_2nd`: 同步码第二个符号
    /// - `sav_3rd`: 同步码第三个符号
    pub fn configure_sublvds_sync_code(&self, port: u8, sav_1st: u16, sav_2nd: u16, sav_3rd: u16) {
        let dphy = self.get_dphy(port);
        dphy.reg_20
            .modify(DPHY_REG_20::SLVDS_SAV_1ST.val(sav_1st as u32));
        dphy.reg_24.modify(
            DPHY_REG_24::SLVDS_SAV_2ND.val(sav_2nd as u32)
                + DPHY_REG_24::SLVDS_SAV_3RD.val(sav_3rd as u32),
        );
    }

    // ========================================================================
    // Lane 校准
    // ========================================================================

    /// 使能 Lane 校准
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `lane`: Lane 索引 (0-3)
    pub fn enable_lane_calibration(&self, port: u8, lane: u8) {
        let dphy = self.get_dphy(port);
        match lane {
            0 => dphy.d0_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::SET),
            1 => dphy.d1_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::SET),
            2 => dphy.d2_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::SET),
            3 => dphy.d3_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::SET),
            _ => {}
        }
    }

    /// 禁用 Lane 校准
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `lane`: Lane 索引 (0-3)
    pub fn disable_lane_calibration(&self, port: u8, lane: u8) {
        let dphy = self.get_dphy(port);
        match lane {
            0 => dphy.d0_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::CLEAR),
            1 => dphy.d1_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::CLEAR),
            2 => dphy.d2_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::CLEAR),
            3 => dphy.d3_calib_1.modify(DPHY_LANE_CALIB_1::CALIB_EN::CLEAR),
            _ => {}
        }
    }

    /// 配置 Lane 校准参数
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `lane`: Lane 索引 (0-3)
    /// - `max_step`: 最大步数
    /// - `step_value`: 步进值
    /// - `pattern`: 校准模式
    pub fn configure_lane_calibration(
        &self,
        port: u8,
        lane: u8,
        max_step: u8,
        step_value: u8,
        pattern: u8,
    ) {
        let dphy = self.get_dphy(port);
        let config = DPHY_LANE_CALIB_0::CALIB_MAX.val(max_step as u32)
            + DPHY_LANE_CALIB_0::CALIB_STEP.val(step_value as u32)
            + DPHY_LANE_CALIB_0::CALIB_PATTERN.val(pattern as u32);

        match lane {
            0 => dphy.d0_calib_0.modify(config),
            1 => dphy.d1_calib_0.modify(config),
            2 => dphy.d2_calib_0.modify(config),
            3 => dphy.d3_calib_0.modify(config),
            _ => {}
        }
    }

    /// 获取 Lane 校准结果
    ///
    /// # 参数
    /// - `port`: 端口号
    /// - `lane`: Lane 索引 (0-3)
    /// - `phase_group`: 相位组索引 (0-7)
    pub fn get_lane_calib_result(&self, port: u8, lane: u8, phase_group: usize) -> u32 {
        if phase_group >= 8 {
            return 0;
        }

        let dphy = self.get_dphy(port);
        match lane {
            0 => dphy.d0_calib_result[phase_group].read(DPHY_LANE_CALIB_RESULT::RESULT),
            1 => dphy.d1_calib_result[phase_group].read(DPHY_LANE_CALIB_RESULT::RESULT),
            2 => dphy.d2_calib_result[phase_group].read(DPHY_LANE_CALIB_RESULT::RESULT),
            3 => dphy.d3_calib_result[phase_group].read(DPHY_LANE_CALIB_RESULT::RESULT),
            _ => 0,
        }
    }
}
