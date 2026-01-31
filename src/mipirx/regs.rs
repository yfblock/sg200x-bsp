//! MIPI RX 寄存器定义
//!
//! 使用 tock-registers 定义 MIPI RX 相关寄存器
//!
//! MIPI RX 模块包含三组寄存器：
//! - PHY 寄存器 (基址: 0x0A0D0000)
//! - 4-Lane DPHY 寄存器 (基址: 0x0A0D0300)
//! - 2-Lane DPHY 寄存器 (基址: 0x0A0D0600)
//! - CSI 控制器寄存器 (基址: 0x0A0C2400 / 0x0A0C4400)

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite, registers::ReadOnly};

// ============================================================================
// 寄存器基地址
// ============================================================================

/// MIPI RX PHY 顶层寄存器基地址
pub const MIPIRX_PHY_TOP_BASE: usize = 0x0A0D_0000;

/// MIPI RX 4-Lane DPHY 寄存器基地址
pub const MIPIRX_DPHY_4L_BASE: usize = 0x0A0D_0300;

/// MIPI RX 2-Lane DPHY 寄存器基地址
pub const MIPIRX_DPHY_2L_BASE: usize = 0x0A0D_0600;

/// MIPI RX CSI0 控制器寄存器基地址
pub const MIPIRX_CSI0_BASE: usize = 0x0A0C_2400;

/// MIPI RX CSI1 控制器寄存器基地址
pub const MIPIRX_CSI1_BASE: usize = 0x0A0C_4400;

// ============================================================================
// PHY 顶层寄存器位域定义
// ============================================================================

register_bitfields! [
    u32,

    /// REG_00 - PHY 电源控制寄存器
    pub PHY_REG_00 [
        /// Reserved
        RESERVED0 OFFSET(0) NUMBITS(14) [],
        /// Power down analog ibias
        PD_IBIAS OFFSET(14) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(15) NUMBITS(1) [],
        /// Power down analog RXLP (6-bit)
        PD_RXLP OFFSET(16) NUMBITS(6) [],
        /// Reserved
        RESERVED2 OFFSET(22) NUMBITS(10) []
    ],

    /// REG_04 - PHY 时钟选择寄存器
    pub PHY_REG_04 [
        /// Reserved
        RESERVED0 OFFSET(0) NUMBITS(16) [],
        /// Analog macro clock lane select (6-bit)
        SEL_CLK_CHANNEL OFFSET(16) NUMBITS(6) [],
        /// Reserved
        RESERVED1 OFFSET(22) NUMBITS(9) [],
        /// Gating test clock from mipimpll
        MIPIMPLL_CLK_CSI_EN OFFSET(31) NUMBITS(1) []
    ],

    /// REG_30 - PHY 模式选择寄存器
    pub PHY_REG_30 [
        /// Sensor PHY mode enable select
        /// 0: 1C4D (1 clock + 4 data lanes)
        /// 1: 1C2D + 1C2D (dual port mode)
        SENSOR_PHY_MODE OFFSET(0) NUMBITS(3) [],
        /// Reserved
        RESERVED OFFSET(3) NUMBITS(29) []
    ],

    /// REG_34~48 - 校准结果寄存器 (只读)
    pub PHY_CAL_RESULT [
        /// Analog lane calibration result
        CAL_RESULT OFFSET(0) NUMBITS(32) []
    ],

    /// REG_80 - 时钟反转控制寄存器
    pub PHY_REG_80 [
        /// AD clock lane0 inverse
        D0_CLK_INV OFFSET(0) NUMBITS(1) [],
        /// AD clock lane1 inverse
        D1_CLK_INV OFFSET(1) NUMBITS(1) [],
        /// AD clock lane2 inverse
        D2_CLK_INV OFFSET(2) NUMBITS(1) [],
        /// AD clock lane3 inverse
        D3_CLK_INV OFFSET(3) NUMBITS(1) [],
        /// AD clock lane4 inverse
        D4_CLK_INV OFFSET(4) NUMBITS(1) [],
        /// AD clock lane5 inverse
        D5_CLK_INV OFFSET(5) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(6) NUMBITS(26) []
    ],

    /// REG_A0 - CAM0 VTT/VS_STR 寄存器
    pub PHY_REG_A0 [
        /// CAM0 VTT
        CAM0_VTT OFFSET(0) NUMBITS(14) [],
        /// Reserved
        RESERVED0 OFFSET(14) NUMBITS(2) [],
        /// CAM0 VS start
        CAM0_VS_STR OFFSET(16) NUMBITS(14) [],
        /// Reserved
        RESERVED1 OFFSET(30) NUMBITS(2) []
    ],

    /// REG_A4 - CAM0 VS_STP/HTT 寄存器
    pub PHY_REG_A4 [
        /// CAM0 VS stop
        CAM0_VS_STP OFFSET(0) NUMBITS(14) [],
        /// Reserved
        RESERVED0 OFFSET(14) NUMBITS(2) [],
        /// CAM0 HTT
        CAM0_HTT OFFSET(16) NUMBITS(14) [],
        /// Reserved
        RESERVED1 OFFSET(30) NUMBITS(2) []
    ],

    /// REG_A8 - CAM0 HS_STR/HS_STP 寄存器
    pub PHY_REG_A8 [
        /// CAM0 HS start
        CAM0_HS_STR OFFSET(0) NUMBITS(14) [],
        /// Reserved
        RESERVED0 OFFSET(14) NUMBITS(2) [],
        /// CAM0 HS stop
        CAM0_HS_STP OFFSET(16) NUMBITS(14) [],
        /// Reserved
        RESERVED1 OFFSET(30) NUMBITS(2) []
    ],

    /// REG_AC - CAM0 极性和时序生成使能寄存器
    pub PHY_REG_AC [
        /// CAM0 VS polarity
        CAM0_VS_POL OFFSET(0) NUMBITS(1) [],
        /// CAM0 HS polarity
        CAM0_HS_POL OFFSET(1) NUMBITS(1) [],
        /// CAM0 timing generator enable
        CAM0_TGEN_EN OFFSET(2) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(3) NUMBITS(29) []
    ]
];

// ============================================================================
// DPHY 寄存器位域定义 (4L/2L 共用)
// ============================================================================

register_bitfields! [
    u32,

    /// DPHY REG_00 - Sensor 模式选择寄存器
    pub DPHY_REG_00 [
        /// Sensor mode select
        /// 2'b00: CSI
        /// 2'b01: Sub-LVDS & HiSPi
        /// 2'b10: SLVSEC
        SENSOR_MODE OFFSET(0) NUMBITS(2) [],
        /// Reserved
        RESERVED OFFSET(2) NUMBITS(30) []
    ],

    /// DPHY REG_04 - CSI Lane 选择寄存器
    pub DPHY_REG_04 [
        /// Data lane 0 select
        CSI_LANE_D0_SEL OFFSET(0) NUMBITS(3) [],
        /// Reserved
        RESERVED0 OFFSET(3) NUMBITS(1) [],
        /// Data lane 1 select
        CSI_LANE_D1_SEL OFFSET(4) NUMBITS(3) [],
        /// Reserved
        RESERVED1 OFFSET(7) NUMBITS(1) [],
        /// Data lane 2 select
        CSI_LANE_D2_SEL OFFSET(8) NUMBITS(3) [],
        /// Reserved
        RESERVED2 OFFSET(11) NUMBITS(1) [],
        /// Data lane 3 select
        CSI_LANE_D3_SEL OFFSET(12) NUMBITS(3) [],
        /// Reserved
        RESERVED3 OFFSET(15) NUMBITS(17) []
    ],

    /// DPHY REG_08 - CSI Lane 配置寄存器
    pub DPHY_REG_08 [
        /// Clock lane select
        CSI_LANE_CK_SEL OFFSET(0) NUMBITS(3) [],
        /// Reserved
        RESERVED0 OFFSET(3) NUMBITS(1) [],
        /// Clock lane pn swap
        CSI_LANE_CK_PNSWAP OFFSET(4) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(5) NUMBITS(3) [],
        /// Data lane 0 pn swap
        CSI_LANE_D0_PNSWAP OFFSET(8) NUMBITS(1) [],
        /// Data lane 1 pn swap
        CSI_LANE_D1_PNSWAP OFFSET(9) NUMBITS(1) [],
        /// Data lane 2 pn swap
        CSI_LANE_D2_PNSWAP OFFSET(10) NUMBITS(1) [],
        /// Data lane 3 pn swap
        CSI_LANE_D3_PNSWAP OFFSET(11) NUMBITS(1) [],
        /// Reserved
        RESERVED2 OFFSET(12) NUMBITS(4) [],
        /// Clock lane phase
        CSI_CK_PHASE OFFSET(16) NUMBITS(8) [],
        /// Reserved
        RESERVED3 OFFSET(24) NUMBITS(8) []
    ],

    /// DPHY REG_0C - Deskew Lane 使能寄存器
    pub DPHY_REG_0C [
        /// Deskew lane enable
        /// 4'h0: No lane
        /// 4'h1: 1-lane
        /// 4'h3: 2-lane
        /// 4'hf: 4-lane
        DESKEW_LANE_EN OFFSET(0) NUMBITS(4) [],
        /// Reserved
        RESERVED OFFSET(4) NUMBITS(28) []
    ],

    /// DPHY REG_20 - Sub-LVDS 配置寄存器
    pub DPHY_REG_20 [
        /// Sub-LVDS bit reverse
        /// 1'b0: LSB first
        /// 1'b1: MSB first
        SLVDS_INV_EN OFFSET(0) NUMBITS(1) [],
        /// Reserved
        RESERVED0 OFFSET(1) NUMBITS(1) [],
        /// Sub-LVDS bit mode
        /// 2'b00: 8-bit
        /// 2'b01: 10-bit
        /// 2'b10: 12-bit
        SLVDS_BIT_MODE OFFSET(2) NUMBITS(2) [],
        /// Sub-LVDS lane enable
        SLVDS_LANE_EN OFFSET(4) NUMBITS(4) [],
        /// Reserved
        RESERVED1 OFFSET(8) NUMBITS(8) [],
        /// Sub-LVDS sync code 1st symbol
        SLVDS_SAV_1ST OFFSET(16) NUMBITS(12) [],
        /// Reserved
        RESERVED2 OFFSET(28) NUMBITS(4) []
    ],

    /// DPHY REG_24 - Sub-LVDS 同步码寄存器
    pub DPHY_REG_24 [
        /// Sub-LVDS sync code 2nd symbol
        SLVDS_SAV_2ND OFFSET(0) NUMBITS(12) [],
        /// Reserved
        RESERVED0 OFFSET(12) NUMBITS(4) [],
        /// Sub-LVDS sync code 3rd symbol
        SLVDS_SAV_3RD OFFSET(16) NUMBITS(12) [],
        /// Reserved
        RESERVED1 OFFSET(28) NUMBITS(4) []
    ],

    /// DPHY Lane 校准控制寄存器 0
    pub DPHY_LANE_CALIB_0 [
        /// Manual PRBS9 enable
        PRBS9_EN OFFSET(0) NUMBITS(1) [],
        /// PRBS9 clear error
        PRBS9_CLR_ERR OFFSET(1) NUMBITS(1) [],
        /// PRBS9 source select
        /// 1'b0: after sync code shift
        /// 1'b1: direct from input
        PRBS9_SOURCE OFFSET(2) NUMBITS(1) [],
        /// PRBS9 error count accumulation
        /// 1'b0: still count after test time done
        /// 1'b1: do not count after test time done
        PRBS9_STOP_WHEN_DONE OFFSET(3) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(4) NUMBITS(4) [],
        /// Calibration max step
        CALIB_MAX OFFSET(8) NUMBITS(8) [],
        /// Calibration one step value
        CALIB_STEP OFFSET(16) NUMBITS(8) [],
        /// Calibration golden pattern
        CALIB_PATTERN OFFSET(24) NUMBITS(8) []
    ],

    /// DPHY Lane 校准控制寄存器 1
    pub DPHY_LANE_CALIB_1 [
        /// Calibration software enable
        CALIB_EN OFFSET(0) NUMBITS(1) [],
        /// Calibration source
        /// 1'b0: normal position
        /// 1'b1: direct from analog
        CALIB_SOURCE OFFSET(1) NUMBITS(1) [],
        /// Calibration software mode
        /// 1'b0: use identical calibration pattern
        /// 1'b1: use PRBS9 pattern
        CALIB_MODE OFFSET(2) NUMBITS(1) [],
        /// Ignore calibration command
        CALIB_IGNORE OFFSET(3) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(4) NUMBITS(28) []
    ],

    /// DPHY Lane 校准结果寄存器
    pub DPHY_LANE_CALIB_RESULT [
        /// Calibration result
        RESULT OFFSET(0) NUMBITS(32) []
    ]
];

// ============================================================================
// CSI 控制器寄存器位域定义
// ============================================================================

register_bitfields! [
    u32,

    /// CSI REG_00 - Lane 模式和控制寄存器
    pub CSI_REG_00 [
        /// Lane mode
        /// 3'b000: 1-lane
        /// 3'b001: 2-lane
        /// 3'b011: 4-lane
        /// 3'b111: 8-lane
        LANE_MODE OFFSET(0) NUMBITS(3) [],
        /// Ignore ECC result
        /// 1'b0: normal
        /// 1'b1: still processing even ECC error
        IGNORE_ECC OFFSET(3) NUMBITS(1) [],
        /// VC check enable
        /// 1'b0: do not check VC
        /// 1'b1: only process packets that meet vc_set
        VC_CHECK OFFSET(4) NUMBITS(1) [],
        /// Reserved
        RESERVED0 OFFSET(5) NUMBITS(3) [],
        /// VC set (only used when VC check enabled)
        VC_SET OFFSET(8) NUMBITS(4) [],
        /// LS and LE packet sent
        /// 1'b0: create hsync signal by controller
        /// 1'b1: use LS and LE to create hsync signal
        LINE_START_SENT OFFSET(12) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(13) NUMBITS(19) []
    ],

    /// CSI REG_04 - 中断和 HDR 控制寄存器
    pub CSI_REG_04 [
        /// Interrupt mask control
        INTR_MASK OFFSET(0) NUMBITS(8) [],
        /// Interrupt clear (W1T)
        INTR_CLR OFFSET(8) NUMBITS(8) [],
        /// HDR mode enable
        HDR_EN OFFSET(16) NUMBITS(1) [],
        /// HDR mode selection
        /// 1'b0: HDR VC mode
        /// 1'b1: HDR ID mode
        HDR_MODE OFFSET(17) NUMBITS(1) [],
        /// Remove non recognized ID line
        /// 1'b0: don't remove
        /// 1'b1: remove
        ID_RM_ELSE OFFSET(18) NUMBITS(1) [],
        /// Remove OB line
        /// 1'b0: don't remove
        /// 1'b1: remove
        ID_RM_OB OFFSET(19) NUMBITS(1) [],
        /// Reserved
        RESERVED OFFSET(20) NUMBITS(12) []
    ],

    /// CSI REG_08 - OB ID 寄存器 0
    pub CSI_REG_08 [
        /// ID for LEF OB n0
        N0_OB_LEF OFFSET(0) NUMBITS(16) [],
        /// ID for SEF OB n0
        N0_OB_SEF OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_0C - LEF ID 寄存器
    pub CSI_REG_0C [
        /// ID for LEF active n0
        N0_LEF OFFSET(0) NUMBITS(16) [],
        /// ID for LEF OB n1
        N1_OB_LEF OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_10 - SEF ID 寄存器
    pub CSI_REG_10 [
        /// ID for SEF OB n1
        N1_OB_SEF OFFSET(0) NUMBITS(16) [],
        /// ID for LEF active n1
        N1_LEF OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_14 - BLC 配置寄存器
    pub CSI_REG_14 [
        /// Data type for optical black line
        BLC_DT OFFSET(0) NUMBITS(6) [],
        /// Reserved
        RESERVED0 OFFSET(6) NUMBITS(2) [],
        /// Optical black line mode enable
        BLC_EN OFFSET(8) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(9) NUMBITS(3) [],
        /// Optical black line data format set
        /// 3'd0: YUV422 8bit
        /// 3'd1: YUV422 10bit
        /// 3'd2: RAW8
        /// 3'd3: RAW10
        /// 3'd4: RAW12
        /// 3'd5: RAW16
        BLC_FORMAT_SET OFFSET(12) NUMBITS(3) [],
        /// Reserved
        RESERVED2 OFFSET(15) NUMBITS(17) []
    ],

    /// CSI REG_18 - VC 映射寄存器
    pub CSI_REG_18 [
        /// VC mapping to ISP channel 00
        VC_MAP_CH00 OFFSET(0) NUMBITS(4) [],
        /// VC mapping to ISP channel 01
        VC_MAP_CH01 OFFSET(4) NUMBITS(4) [],
        /// VC mapping to ISP channel 10
        VC_MAP_CH10 OFFSET(8) NUMBITS(4) [],
        /// VC mapping to ISP channel 11
        VC_MAP_CH11 OFFSET(12) NUMBITS(4) [],
        /// Reserved
        RESERVED OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_1C - SEF ID 寄存器 2
    pub CSI_REG_1C [
        /// ID for SEF active n0
        N0_SEF OFFSET(0) NUMBITS(16) [],
        /// ID for SEF active n1
        N1_SEF OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_20 - SEF2 ID 寄存器
    pub CSI_REG_20 [
        /// ID for SEF2 active n0
        N0_SEF2 OFFSET(0) NUMBITS(16) [],
        /// ID for SEF2 active n1
        N1_SEF2 OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_24 - SEF2 OB ID 寄存器
    pub CSI_REG_24 [
        /// ID for SEF2 OB n0
        N0_OB_SEF2 OFFSET(0) NUMBITS(16) [],
        /// ID for SEF2 OB n1
        N1_OB_SEF2 OFFSET(16) NUMBITS(16) []
    ],

    /// CSI REG_40 - 状态寄存器
    pub CSI_REG_40 [
        /// ECC no error
        ECC_NO_ERROR OFFSET(0) NUMBITS(1) [],
        /// ECC corrected error
        ECC_CORRECTED_ERROR OFFSET(1) NUMBITS(1) [],
        /// ECC error
        ECC_ERROR OFFSET(2) NUMBITS(1) [],
        /// Reserved
        RESERVED0 OFFSET(3) NUMBITS(1) [],
        /// CRC error
        CRC_ERROR OFFSET(4) NUMBITS(1) [],
        /// WC error
        WC_ERROR OFFSET(5) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(6) NUMBITS(2) [],
        /// CSI FIFO full
        FIFO_FULL OFFSET(8) NUMBITS(1) [],
        /// Reserved
        RESERVED2 OFFSET(9) NUMBITS(7) [],
        /// CSI decode format from header
        /// bit[0]: YUV422 8bit
        /// bit[1]: YUV422 10bit
        /// bit[2]: RAW8
        /// bit[3]: RAW10
        /// bit[4]: RAW12
        /// bit[5]: RAW16
        DECODE_FORMAT OFFSET(16) NUMBITS(6) [],
        /// Reserved
        RESERVED3 OFFSET(22) NUMBITS(10) []
    ],

    /// CSI REG_60 - 中断状态寄存器
    pub CSI_REG_60 [
        /// Interrupt status
        /// bit[0]: ECC error
        /// bit[1]: CRC error
        /// bit[2]: HDR ID error
        /// bit[3]: Word count error
        /// bit[4]: FIFO full
        INTR_STATUS OFFSET(0) NUMBITS(8) [],
        /// Reserved
        RESERVED OFFSET(8) NUMBITS(24) []
    ],

    /// CSI REG_70 - VS 生成模式寄存器
    pub CSI_REG_70 [
        /// VS generation mode
        /// 2'b00: vs gen by FS
        /// 2'b01: vs gen by FE
        /// else: vs gen by FS & FE
        VS_GEN_MODE OFFSET(0) NUMBITS(2) [],
        /// Reserved
        RESERVED0 OFFSET(2) NUMBITS(2) [],
        /// Vsync generation setting
        /// 1'b0: generated by all vc short packet
        /// 1'b1: only generated by indicated vc short packet
        VS_GEN_BY_VCSET OFFSET(4) NUMBITS(1) [],
        /// Reserved
        RESERVED1 OFFSET(5) NUMBITS(27) []
    ],

    /// CSI REG_74 - HDR DT 模式寄存器
    pub CSI_REG_74 [
        /// CSI HDR DT mode enable
        HDR_DT_MODE OFFSET(0) NUMBITS(1) [],
        /// Reserved
        RESERVED0 OFFSET(1) NUMBITS(3) [],
        /// CSI HDR DT format
        HDR_DT_FORMAT OFFSET(4) NUMBITS(6) [],
        /// Reserved
        RESERVED1 OFFSET(10) NUMBITS(2) [],
        /// CSI HDR DT LEF
        HDR_DT_LEF OFFSET(12) NUMBITS(6) [],
        /// Reserved
        RESERVED2 OFFSET(18) NUMBITS(2) [],
        /// CSI HDR DT SEF
        HDR_DT_SEF OFFSET(20) NUMBITS(6) [],
        /// Reserved
        RESERVED3 OFFSET(26) NUMBITS(6) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// MIPI RX PHY 顶层寄存器组
    pub MipiRxPhyTopRegs {
        /// REG_00 - PHY 电源控制寄存器
        (0x000 => pub reg_00: ReadWrite<u32, PHY_REG_00::Register>),
        /// REG_04 - PHY 时钟选择寄存器
        (0x004 => pub reg_04: ReadWrite<u32, PHY_REG_04::Register>),
        /// Reserved
        (0x008 => _reserved0),
        /// REG_30 - PHY 模式选择寄存器
        (0x030 => pub reg_30: ReadWrite<u32, PHY_REG_30::Register>),
        /// REG_34 - Lane 0 校准结果
        (0x034 => pub reg_34: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// REG_38 - Lane 1 校准结果
        (0x038 => pub reg_38: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// REG_3C - Lane 2 校准结果
        (0x03c => pub reg_3c: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// REG_40 - Lane 3 校准结果
        (0x040 => pub reg_40: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// REG_44 - Lane 4 校准结果
        (0x044 => pub reg_44: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// REG_48 - Lane 5 校准结果
        (0x048 => pub reg_48: ReadOnly<u32, PHY_CAL_RESULT::Register>),
        /// Reserved
        (0x04c => _reserved1),
        /// REG_80 - 时钟反转控制寄存器
        (0x080 => pub reg_80: ReadWrite<u32, PHY_REG_80::Register>),
        /// Reserved
        (0x084 => _reserved2),
        /// REG_A0 - CAM0 VTT/VS_STR 寄存器
        (0x0a0 => pub reg_a0: ReadWrite<u32, PHY_REG_A0::Register>),
        /// REG_A4 - CAM0 VS_STP/HTT 寄存器
        (0x0a4 => pub reg_a4: ReadWrite<u32, PHY_REG_A4::Register>),
        /// REG_A8 - CAM0 HS_STR/HS_STP 寄存器
        (0x0a8 => pub reg_a8: ReadWrite<u32, PHY_REG_A8::Register>),
        /// REG_AC - CAM0 极性和时序生成使能寄存器
        (0x0ac => pub reg_ac: ReadWrite<u32, PHY_REG_AC::Register>),
        /// 结束标记
        (0x0b0 => @END),
    }
}

register_structs! {
    /// MIPI RX DPHY 寄存器组 (4L/2L 共用结构)
    pub MipiRxDphyRegs {
        /// REG_00 - Sensor 模式选择寄存器
        (0x000 => pub reg_00: ReadWrite<u32, DPHY_REG_00::Register>),
        /// REG_04 - CSI Lane 选择寄存器
        (0x004 => pub reg_04: ReadWrite<u32, DPHY_REG_04::Register>),
        /// REG_08 - CSI Lane 配置寄存器
        (0x008 => pub reg_08: ReadWrite<u32, DPHY_REG_08::Register>),
        /// REG_0C - Deskew Lane 使能寄存器
        (0x00c => pub reg_0c: ReadWrite<u32, DPHY_REG_0C::Register>),
        /// Reserved
        (0x010 => _reserved0),
        /// REG_20 - Sub-LVDS 配置寄存器
        (0x020 => pub reg_20: ReadWrite<u32, DPHY_REG_20::Register>),
        /// REG_24 - Sub-LVDS 同步码寄存器
        (0x024 => pub reg_24: ReadWrite<u32, DPHY_REG_24::Register>),
        /// Reserved
        (0x028 => _reserved1),
        /// REG_D0_0 - Lane 0 校准控制寄存器 0
        (0x100 => pub d0_calib_0: ReadWrite<u32, DPHY_LANE_CALIB_0::Register>),
        /// REG_D0_1 - Lane 0 校准控制寄存器 1
        (0x104 => pub d0_calib_1: ReadWrite<u32, DPHY_LANE_CALIB_1::Register>),
        /// Reserved
        (0x108 => _reserved2),
        /// REG_D0_3~A - Lane 0 校准结果寄存器
        (0x10c => pub d0_calib_result: [ReadOnly<u32, DPHY_LANE_CALIB_RESULT::Register>; 8]),
        /// Reserved
        (0x12c => _reserved3),
        /// REG_D1_0 - Lane 1 校准控制寄存器 0
        (0x140 => pub d1_calib_0: ReadWrite<u32, DPHY_LANE_CALIB_0::Register>),
        /// REG_D1_1 - Lane 1 校准控制寄存器 1
        (0x144 => pub d1_calib_1: ReadWrite<u32, DPHY_LANE_CALIB_1::Register>),
        /// Reserved
        (0x148 => _reserved4),
        /// REG_D1_3~A - Lane 1 校准结果寄存器
        (0x14c => pub d1_calib_result: [ReadOnly<u32, DPHY_LANE_CALIB_RESULT::Register>; 8]),
        /// Reserved
        (0x16c => _reserved5),
        /// REG_D2_0 - Lane 2 校准控制寄存器 0
        (0x180 => pub d2_calib_0: ReadWrite<u32, DPHY_LANE_CALIB_0::Register>),
        /// REG_D2_1 - Lane 2 校准控制寄存器 1
        (0x184 => pub d2_calib_1: ReadWrite<u32, DPHY_LANE_CALIB_1::Register>),
        /// Reserved
        (0x188 => _reserved6),
        /// REG_D2_3~A - Lane 2 校准结果寄存器
        (0x18c => pub d2_calib_result: [ReadOnly<u32, DPHY_LANE_CALIB_RESULT::Register>; 8]),
        /// Reserved
        (0x1ac => _reserved7),
        /// REG_D3_0 - Lane 3 校准控制寄存器 0
        (0x1c0 => pub d3_calib_0: ReadWrite<u32, DPHY_LANE_CALIB_0::Register>),
        /// REG_D3_1 - Lane 3 校准控制寄存器 1
        (0x1c4 => pub d3_calib_1: ReadWrite<u32, DPHY_LANE_CALIB_1::Register>),
        /// Reserved
        (0x1c8 => _reserved8),
        /// REG_D3_3~A - Lane 3 校准结果寄存器
        (0x1cc => pub d3_calib_result: [ReadOnly<u32, DPHY_LANE_CALIB_RESULT::Register>; 8]),
        /// 结束标记
        (0x1ec => @END),
    }
}

register_structs! {
    /// MIPI RX CSI 控制器寄存器组
    pub MipiRxCsiRegs {
        /// REG_00 - Lane 模式和控制寄存器
        (0x000 => pub reg_00: ReadWrite<u32, CSI_REG_00::Register>),
        /// REG_04 - 中断和 HDR 控制寄存器
        (0x004 => pub reg_04: ReadWrite<u32, CSI_REG_04::Register>),
        /// REG_08 - OB ID 寄存器 0
        (0x008 => pub reg_08: ReadWrite<u32, CSI_REG_08::Register>),
        /// REG_0C - LEF ID 寄存器
        (0x00c => pub reg_0c: ReadWrite<u32, CSI_REG_0C::Register>),
        /// REG_10 - SEF ID 寄存器
        (0x010 => pub reg_10: ReadWrite<u32, CSI_REG_10::Register>),
        /// REG_14 - BLC 配置寄存器
        (0x014 => pub reg_14: ReadWrite<u32, CSI_REG_14::Register>),
        /// REG_18 - VC 映射寄存器
        (0x018 => pub reg_18: ReadWrite<u32, CSI_REG_18::Register>),
        /// REG_1C - SEF ID 寄存器 2
        (0x01c => pub reg_1c: ReadWrite<u32, CSI_REG_1C::Register>),
        /// REG_20 - SEF2 ID 寄存器
        (0x020 => pub reg_20: ReadWrite<u32, CSI_REG_20::Register>),
        /// REG_24 - SEF2 OB ID 寄存器
        (0x024 => pub reg_24: ReadWrite<u32, CSI_REG_24::Register>),
        /// Reserved
        (0x028 => _reserved0),
        /// REG_40 - 状态寄存器
        (0x040 => pub reg_40: ReadOnly<u32, CSI_REG_40::Register>),
        /// Reserved
        (0x044 => _reserved1),
        /// REG_60 - 中断状态寄存器
        (0x060 => pub reg_60: ReadOnly<u32, CSI_REG_60::Register>),
        /// Reserved
        (0x064 => _reserved2),
        /// REG_70 - VS 生成模式寄存器
        (0x070 => pub reg_70: ReadWrite<u32, CSI_REG_70::Register>),
        /// REG_74 - HDR DT 模式寄存器
        (0x074 => pub reg_74: ReadWrite<u32, CSI_REG_74::Register>),
        /// 结束标记
        (0x078 => @END),
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取 PHY 顶层寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn phy_top_regs() -> &'static MipiRxPhyTopRegs {
    &*(MIPIRX_PHY_TOP_BASE as *const MipiRxPhyTopRegs)
}

/// 获取 4-Lane DPHY 寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn dphy_4l_regs() -> &'static MipiRxDphyRegs {
    &*(MIPIRX_DPHY_4L_BASE as *const MipiRxDphyRegs)
}

/// 获取 2-Lane DPHY 寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn dphy_2l_regs() -> &'static MipiRxDphyRegs {
    &*(MIPIRX_DPHY_2L_BASE as *const MipiRxDphyRegs)
}

/// 获取 CSI0 控制器寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn csi0_regs() -> &'static MipiRxCsiRegs {
    &*(MIPIRX_CSI0_BASE as *const MipiRxCsiRegs)
}

/// 获取 CSI1 控制器寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn csi1_regs() -> &'static MipiRxCsiRegs {
    &*(MIPIRX_CSI1_BASE as *const MipiRxCsiRegs)
}

/// 根据索引获取 CSI 控制器寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn csi_regs(index: usize) -> Option<&'static MipiRxCsiRegs> {
    match index {
        0 => Some(csi0_regs()),
        1 => Some(csi1_regs()),
        _ => None,
    }
}

/// 根据索引获取 DPHY 寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn dphy_regs(index: usize) -> Option<&'static MipiRxDphyRegs> {
    match index {
        0 => Some(dphy_4l_regs()),
        1 => Some(dphy_2l_regs()),
        _ => None,
    }
}
