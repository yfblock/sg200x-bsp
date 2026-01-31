//! VI (Video Input) 寄存器定义
//!
//! 使用 tock-registers 定义 VI 相关寄存器
//!
//! VI 模块包含三组寄存器：
//! - VI0 寄存器 (基址: 0x0A0C2000)
//! - VI1 寄存器 (基址: 0x0A0C4000)
//! - VI2 寄存器 (基址: 0x0A0C6000) - 仅支持 BT 接口

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

// ============================================================================
// 寄存器基地址
// ============================================================================

/// VI0 寄存器基地址
/// 支持 MIPI CSI-2、Sub-LVDS、HiSPi、TTL 等接口
pub const VI0_BASE: usize = 0x0A0C_2000;

/// VI1 寄存器基地址
/// 支持 MIPI CSI-2、Sub-LVDS、HiSPi、TTL 等接口
pub const VI1_BASE: usize = 0x0A0C_4000;

/// VI2 寄存器基地址
/// 仅支持 BT 接口 (BT.656/BT.601/BT.1120)
pub const VI2_BASE: usize = 0x0A0C_6000;

// ============================================================================
// REG_00 - 模式控制寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_00 - 传感器 MAC 模式控制寄存器
    /// 
    /// 该寄存器用于配置 VI 的工作模式，包括：
    /// - 传感器接口模式选择 (CSI/Sub-LVDS/TTL)
    /// - BT Demux 使能
    /// - CSI/Sub-LVDS 控制器使能
    /// - 信号极性配置
    pub REG_00 [
        /// 传感器 MAC 模式选择
        /// 
        /// 3'b000: 禁用
        /// 3'b001: CSI 模式
        /// 3'b010: Sub-LVDS 模式
        /// 3'b011: TTL 模式
        REG_SENSOR_MAC_MODE OFFSET(0) NUMBITS(3) [
            /// 禁用
            Disable = 0b000,
            /// CSI 模式
            Csi = 0b001,
            /// Sub-LVDS 模式
            SubLvds = 0b010,
            /// TTL 模式
            Ttl = 0b011
        ],

        /// BT Demux 使能
        /// 
        /// 用于 BT.656/BT.1120 多通道融合输入
        REG_BT_DEMUX_ENABLE OFFSET(3) NUMBITS(1) [],

        /// CSI 控制器使能
        REG_CSI_CTRL_ENABLE OFFSET(4) NUMBITS(1) [],

        /// CSI VS (垂直同步) 信号反相
        REG_CSI_VS_INV OFFSET(5) NUMBITS(1) [],

        /// CSI HS (水平同步) 信号反相
        REG_CSI_HS_INV OFFSET(6) NUMBITS(1) [],

        /// Reserved bit 7
        RESERVED_7 OFFSET(7) NUMBITS(1) [],

        /// Sub-LVDS 控制器使能
        REG_SUBLVDS_CTRL_ENABLE OFFSET(8) NUMBITS(1) [],

        /// Sub-LVDS VS (垂直同步) 信号反相
        REG_SUBLVDS_VS_INV OFFSET(9) NUMBITS(1) [],

        /// Sub-LVDS HS (水平同步) 信号反相
        REG_SUBLVDS_HS_INV OFFSET(10) NUMBITS(1) [],

        /// Sub-LVDS HDR 信号反相
        REG_SUBLVDS_HDR_INV OFFSET(11) NUMBITS(1) []
    ]
];

// ============================================================================
// REG_10~REG_30 - TTL 模式配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_10 - TTL 模式配置寄存器 0
    /// 
    /// 配置 TTL 接口的基本参数，包括：
    /// - TTL 使能
    /// - 位宽模式
    /// - 输出格式
    /// - 输入格式
    pub REG_10 [
        /// TTL IP 使能
        REG_TTL_IP_EN OFFSET(0) NUMBITS(1) [],

        /// TTL 传感器位宽
        /// 
        /// 2'b00: 8-bit
        /// 2'b01: 10-bit
        /// 2'b10: 12-bit
        /// 2'b11: 16-bit
        REG_TTL_SENSOR_BIT OFFSET(1) NUMBITS(2) [
            /// 8-bit 模式
            Bit8 = 0b00,
            /// 10-bit 模式
            Bit10 = 0b01,
            /// 12-bit 模式
            Bit12 = 0b10,
            /// 16-bit 模式
            Bit16 = 0b11
        ],

        /// Reserved bit 3
        RESERVED_3 OFFSET(3) NUMBITS(1) [],

        /// TTL BT 输出格式
        /// 
        /// 2'b00: {Cb,Y},{Cr,Y}
        /// 2'b01: {Cr,Y},{Cb,Y}
        /// 2'b10: {Y,Cb},{Y,Cr}
        /// 2'b11: {Y,Cr},{Y,Cb}
        REG_TTL_BT_FMT_OUT OFFSET(4) NUMBITS(2) [
            /// {Cb,Y},{Cr,Y} 格式
            CbYCrY = 0b00,
            /// {Cr,Y},{Cb,Y} 格式
            CrYCbY = 0b01,
            /// {Y,Cb},{Y,Cr} 格式
            YCbYCr = 0b10,
            /// {Y,Cr},{Y,Cb} 格式
            YCrYCb = 0b11
        ],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// TTL 输入格式
        /// 
        /// 4'b0000: bt_2x with sync pattern, 9-bit BT656
        /// 4'b0001: bt_1x with sync pattern, 17-bit BT1120
        /// 4'b0010: bt_2x without sync pattern, 11-bit BT601 (vhs mode)
        /// 4'b0011: bt_1x without sync pattern, 19-bit BT601 (vhs mode)
        /// 4'b0100: bt_2x without sync pattern, 11-bit BT601 (vde mode)
        /// 4'b0101: bt_1x without sync pattern, 19-bit BT601 (vde mode)
        /// 4'b0110: bt_2x without sync pattern, 11-bit BT601 (vsde mode)
        /// 4'b0111: bt_1x without sync pattern, 19-bit BT601 (vsde mode)
        /// 4'b100x: sensor with sync pattern
        /// 4'b101x: sensor without sync pattern, use vs + hs (vhs mode)
        /// 4'b110x: sensor without sync pattern, use vde + hde (vde mode)
        /// 4'b111x: sensor without sync pattern, use vs + hde (vsde mode)
        REG_TTL_FMT_IN OFFSET(8) NUMBITS(4) [
            /// BT656 9-bit (带同步码)
            Bt656_9bit = 0b0000,
            /// BT1120 17-bit (带同步码)
            Bt1120_17bit = 0b0001,
            /// BT601 11-bit VHS 模式 (无同步码)
            Bt601_11bit_Vhs = 0b0010,
            /// BT601 19-bit VHS 模式 (无同步码)
            Bt601_19bit_Vhs = 0b0011,
            /// BT601 11-bit VDE 模式 (无同步码)
            Bt601_11bit_Vde = 0b0100,
            /// BT601 19-bit VDE 模式 (无同步码)
            Bt601_19bit_Vde = 0b0101,
            /// BT601 11-bit VSDE 模式 (无同步码)
            Bt601_11bit_Vsde = 0b0110,
            /// BT601 19-bit VSDE 模式 (无同步码)
            Bt601_19bit_Vsde = 0b0111,
            /// Sensor 带同步码
            SensorWithSync = 0b1000,
            /// Sensor VHS 模式 (无同步码)
            SensorVhs = 0b1010,
            /// Sensor VDE 模式 (无同步码)
            SensorVde = 0b1100,
            /// Sensor VSDE 模式 (无同步码)
            SensorVsde = 0b1110
        ],

        /// TTL BT 数据序列
        /// 
        /// 2'b00: Cb0-Y0-Cr0-Y1
        /// 2'b01: Cr0-Y0-Cb0-Y1
        /// 2'b10: Y0-Cb0-Y1-Cr0
        /// 2'b11: Y0-Cr0-Y1-Cb0
        REG_TTL_BT_DATA_SEQ OFFSET(12) NUMBITS(2) [
            /// Cb0-Y0-Cr0-Y1 序列
            Cb0Y0Cr0Y1 = 0b00,
            /// Cr0-Y0-Cb0-Y1 序列
            Cr0Y0Cb0Y1 = 0b01,
            /// Y0-Cb0-Y1-Cr0 序列
            Y0Cb0Y1Cr0 = 0b10,
            /// Y0-Cr0-Y1-Cb0 序列
            Y0Cr0Y1Cb0 = 0b11
        ],

        /// TTL VS (垂直同步) 信号反相
        REG_TTL_VS_INV OFFSET(14) NUMBITS(1) [],

        /// TTL HS (水平同步) 信号反相
        REG_TTL_HS_INV OFFSET(15) NUMBITS(1) []
    ],

    /// REG_14 - TTL 消隐配置寄存器
    /// 
    /// 配置 TTL 接口的消隐参数
    pub REG_14 [
        /// TTL 垂直同步后消隐 (VS Back Porch)
        /// 
        /// 帧同步信号后的消隐行数
        REG_TTL_VS_BP OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// TTL 水平同步后消隐 (HS Back Porch)
        /// 
        /// 行同步信号后的消隐像素数
        REG_TTL_HS_BP OFFSET(16) NUMBITS(12) []
    ],

    /// REG_18 - TTL 图像尺寸配置寄存器
    pub REG_18 [
        /// TTL 图像宽度
        REG_TTL_IMG_WD OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// TTL 图像高度
        REG_TTL_IMG_HT OFFSET(16) NUMBITS(12) []
    ],

    /// REG_1C - TTL 同步码配置寄存器 0
    pub REG_1C [
        /// TTL 同步码 0
        REG_TTL_SYNC_0 OFFSET(0) NUMBITS(16) [],

        /// TTL 同步码 1
        REG_TTL_SYNC_1 OFFSET(16) NUMBITS(16) []
    ],

    /// REG_20 - TTL 同步码配置寄存器 1
    pub REG_20 [
        /// TTL 同步码 2
        REG_TTL_SYNC_2 OFFSET(0) NUMBITS(16) []
    ],

    /// REG_24 - TTL SAV 同步码配置寄存器
    pub REG_24 [
        /// TTL 有效行 SAV (Start of Active Video)
        REG_TTL_SAV_VLD OFFSET(0) NUMBITS(16) [],

        /// TTL 消隐行 SAV
        REG_TTL_SAV_BLK OFFSET(16) NUMBITS(16) []
    ],

    /// REG_28 - TTL EAV 同步码配置寄存器
    pub REG_28 [
        /// TTL 有效行 EAV (End of Active Video)
        REG_TTL_EAV_VLD OFFSET(0) NUMBITS(16) [],

        /// TTL 消隐行 EAV
        REG_TTL_EAV_BLK OFFSET(16) NUMBITS(16) []
    ],

    /// REG_30 - VI 选择配置寄存器
    pub REG_30 [
        /// VI 输入模式选择
        /// 
        /// 3'h1: RAW
        /// 3'h2: BT601
        /// 3'h3: BT656
        /// 3'h4: BT1120
        REG_VI_SEL OFFSET(0) NUMBITS(3) [
            /// RAW 模式
            Raw = 0x1,
            /// BT601 模式
            Bt601 = 0x2,
            /// BT656 模式
            Bt656 = 0x3,
            /// BT1120 模式
            Bt1120 = 0x4
        ],

        /// VI 输入来源选择
        /// 
        /// 1'b0: 来自 VI0
        /// 1'b1: 来自 VI1
        REG_VI_FROM OFFSET(3) NUMBITS(1) [
            /// 来自 VI0
            FromVi0 = 0,
            /// 来自 VI1
            FromVi1 = 1
        ],

        /// VI 时钟反相
        REG_VI_CLK_INV OFFSET(4) NUMBITS(1) [],

        /// VS 信号选择
        /// 
        /// 1'b1: vs_in 信号作为 vs
        /// 1'b0: vs_in 信号作为 vde
        REG_VI_V_SEL_VS OFFSET(5) NUMBITS(1) [],

        /// VS 调试源选择
        REG_VI_VS_DBG OFFSET(6) NUMBITS(1) [],

        /// Reserved bit 7
        RESERVED_7 OFFSET(7) NUMBITS(1) [],

        /// VI0 时钟反相
        REG_PAD_VI0_CLK_INV OFFSET(8) NUMBITS(1) [],

        /// VI1 时钟反相
        REG_PAD_VI1_CLK_INV OFFSET(9) NUMBITS(1) [],

        /// VI2 时钟反相
        REG_PAD_VI2_CLK_INV OFFSET(10) NUMBITS(1) []
    ]
];

// ============================================================================
// REG_40~REG_58 - HDR 和 BLC 模式配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_40 - HDR 模式配置寄存器 0
    /// 
    /// 配置传感器 MAC 的 HDR (高动态范围) 模式
    pub REG_40 [
        /// 传感器 MAC HDR 手动模式使能
        /// 
        /// Shadow: Yes
        /// Shadow Ctrl: up_1t
        REG_SENSOR_MAC_HDR_EN OFFSET(0) NUMBITS(1) [],

        /// 传感器 MAC VS 输出反相
        REG_SENSOR_MAC_HDR_VSINV OFFSET(1) NUMBITS(1) [],

        /// 传感器 MAC HS 输出反相
        REG_SENSOR_MAC_HDR_HSINV OFFSET(2) NUMBITS(1) [],

        /// 传感器 MAC DE 输出反相
        REG_SENSOR_MAC_HDR_DEINV OFFSET(3) NUMBITS(1) [],

        /// 传感器 MAC HDR[0] 输出反相
        REG_SENSOR_MAC_HDR_HDR0INV OFFSET(4) NUMBITS(1) [],

        /// 传感器 MAC HDR[1] 输出反相
        REG_SENSOR_MAC_HDR_HDR1INV OFFSET(5) NUMBITS(1) [],

        /// 传感器 MAC BLC 输出反相
        REG_SENSOR_MAC_HDR_BLCINV OFFSET(6) NUMBITS(1) [],

        /// Reserved bit 7
        RESERVED_7 OFFSET(7) NUMBITS(1) [],

        /// 传感器 MAC HDR 模式
        /// 
        /// 1'b1: HiSPi S-SP HDR 模式，移除 HDR 消隐行
        REG_SENSOR_MAC_HDR_MODE OFFSET(8) NUMBITS(1) []
    ],

    /// REG_44 - HDR 模式配置寄存器 1
    pub REG_44 [
        /// 传感器 MAC HDR 长曝光偏移
        /// 
        /// 第一个短曝光行之前的长曝光行数
        REG_SENSOR_MAC_HDR_SHIFT OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// 传感器 MAC HDR 垂直尺寸
        REG_SENSOR_MAC_HDR_VSIZE OFFSET(16) NUMBITS(13) []
    ],

    /// REG_48 - 信息行配置寄存器
    pub REG_48 [
        /// 信息行数量
        REG_SENSOR_MAC_INFO_LINE_NUM OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// 移除信息行
        REG_SENSOR_MAC_RM_INFO_LINE OFFSET(16) NUMBITS(1) []
    ],

    /// REG_50 - BLC 模式配置寄存器
    /// 
    /// BLC (Black Level Calibration) 黑电平校准配置
    pub REG_50 [
        /// BLC0 模式使能
        REG_SENSOR_MAC_BLC0_EN OFFSET(0) NUMBITS(1) [],

        /// BLC1 模式使能
        REG_SENSOR_MAC_BLC1_EN OFFSET(1) NUMBITS(1) []
    ],

    /// REG_54 - BLC0 配置寄存器
    pub REG_54 [
        /// BLC0 起始行号
        REG_SENSOR_MAC_BLC0_START OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// BLC0 行数
        REG_SENSOR_MAC_BLC0_SIZE OFFSET(16) NUMBITS(13) []
    ],

    /// REG_58 - BLC1 配置寄存器
    pub REG_58 [
        /// BLC1 起始行号
        REG_SENSOR_MAC_BLC1_START OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// BLC1 行数
        REG_SENSOR_MAC_BLC1_SIZE OFFSET(16) NUMBITS(13) []
    ]
];

// ============================================================================
// REG_60~REG_74 - VI 引脚复用配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_60 - VI 引脚选择寄存器 0
    /// 
    /// 配置 VS/HS/VDE/HDE 信号的引脚映射
    pub REG_60 [
        /// VI VS 引脚选择
        /// 
        /// [5]: 来自 VI1 或 VI0
        /// [4:0]: 来自哪个 VI pad
        REG_VI_VS_SEL OFFSET(0) NUMBITS(6) [],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// VI HS 引脚选择
        REG_VI_HS_SEL OFFSET(8) NUMBITS(6) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// VI VDE 引脚选择
        REG_VI_VDE_SEL OFFSET(16) NUMBITS(6) [],

        /// Reserved bits 22-23
        RESERVED_22_23 OFFSET(22) NUMBITS(2) [],

        /// VI HDE 引脚选择
        REG_VI_HDE_SEL OFFSET(24) NUMBITS(6) []
    ],

    /// REG_64 - VI 数据引脚选择寄存器 0
    /// 
    /// 配置数据引脚 D0~D3 的映射
    pub REG_64 [
        /// VI D0 引脚选择
        REG_VI_D0_SEL OFFSET(0) NUMBITS(6) [],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// VI D1 引脚选择
        REG_VI_D1_SEL OFFSET(8) NUMBITS(6) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// VI D2 引脚选择
        REG_VI_D2_SEL OFFSET(16) NUMBITS(6) [],

        /// Reserved bits 22-23
        RESERVED_22_23 OFFSET(22) NUMBITS(2) [],

        /// VI D3 引脚选择
        REG_VI_D3_SEL OFFSET(24) NUMBITS(6) []
    ],

    /// REG_68 - VI 数据引脚选择寄存器 1
    /// 
    /// 配置数据引脚 D4~D7 的映射
    pub REG_68 [
        /// VI D4 引脚选择
        REG_VI_D4_SEL OFFSET(0) NUMBITS(6) [],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// VI D5 引脚选择
        REG_VI_D5_SEL OFFSET(8) NUMBITS(6) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// VI D6 引脚选择
        REG_VI_D6_SEL OFFSET(16) NUMBITS(6) [],

        /// Reserved bits 22-23
        RESERVED_22_23 OFFSET(22) NUMBITS(2) [],

        /// VI D7 引脚选择
        REG_VI_D7_SEL OFFSET(24) NUMBITS(6) []
    ],

    /// REG_6C - VI 数据引脚选择寄存器 2
    /// 
    /// 配置数据引脚 D8~D11 的映射
    pub REG_6C [
        /// VI D8 引脚选择
        REG_VI_D8_SEL OFFSET(0) NUMBITS(6) [],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// VI D9 引脚选择
        REG_VI_D9_SEL OFFSET(8) NUMBITS(6) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// VI D10 引脚选择
        REG_VI_D10_SEL OFFSET(16) NUMBITS(6) [],

        /// Reserved bits 22-23
        RESERVED_22_23 OFFSET(22) NUMBITS(2) [],

        /// VI D11 引脚选择
        REG_VI_D11_SEL OFFSET(24) NUMBITS(6) []
    ],

    /// REG_70 - VI 数据引脚选择寄存器 3
    /// 
    /// 配置数据引脚 D12~D15 的映射
    pub REG_70 [
        /// VI D12 引脚选择
        REG_VI_D12_SEL OFFSET(0) NUMBITS(6) [],

        /// Reserved bits 6-7
        RESERVED_6_7 OFFSET(6) NUMBITS(2) [],

        /// VI D13 引脚选择
        REG_VI_D13_SEL OFFSET(8) NUMBITS(6) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// VI D14 引脚选择
        REG_VI_D14_SEL OFFSET(16) NUMBITS(6) [],

        /// Reserved bits 22-23
        RESERVED_22_23 OFFSET(22) NUMBITS(2) [],

        /// VI D15 引脚选择
        REG_VI_D15_SEL OFFSET(24) NUMBITS(6) []
    ],

    /// REG_74 - VI BT 数据引脚选择寄存器
    /// 
    /// 配置 BT 接口数据引脚 D0~D7 的映射 (仅用于 VI2)
    pub REG_74 [
        /// VI BT D0 引脚选择 (来自 VI2 pad)
        REG_VI_BT_D0_SEL OFFSET(0) NUMBITS(3) [],

        /// Reserved bit 3
        RESERVED_3 OFFSET(3) NUMBITS(1) [],

        /// VI BT D1 引脚选择
        REG_VI_BT_D1_SEL OFFSET(4) NUMBITS(3) [],

        /// Reserved bit 7
        RESERVED_7 OFFSET(7) NUMBITS(1) [],

        /// VI BT D2 引脚选择
        REG_VI_BT_D2_SEL OFFSET(8) NUMBITS(3) [],

        /// Reserved bit 11
        RESERVED_11 OFFSET(11) NUMBITS(1) [],

        /// VI BT D3 引脚选择
        REG_VI_BT_D3_SEL OFFSET(12) NUMBITS(3) [],

        /// Reserved bit 15
        RESERVED_15 OFFSET(15) NUMBITS(1) [],

        /// VI BT D4 引脚选择
        REG_VI_BT_D4_SEL OFFSET(16) NUMBITS(3) [],

        /// Reserved bit 19
        RESERVED_19 OFFSET(19) NUMBITS(1) [],

        /// VI BT D5 引脚选择
        REG_VI_BT_D5_SEL OFFSET(20) NUMBITS(3) [],

        /// Reserved bit 23
        RESERVED_23 OFFSET(23) NUMBITS(1) [],

        /// VI BT D6 引脚选择
        REG_VI_BT_D6_SEL OFFSET(24) NUMBITS(3) [],

        /// Reserved bit 27
        RESERVED_27 OFFSET(27) NUMBITS(1) [],

        /// VI BT D7 引脚选择
        REG_VI_BT_D7_SEL OFFSET(28) NUMBITS(3) []
    ]
];

// ============================================================================
// REG_80~REG_A4 - BT 路径配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_80 - BT 路径配置寄存器 0
    /// 
    /// 配置 BT 接口的基本参数
    pub REG_80 [
        /// 清除同步丢失信号 (写 1 清除)
        REG_BT_CLR_SYNC_LOST_1T OFFSET(0) NUMBITS(1) [],

        /// BT 路径使能
        REG_BT_IP_EN OFFSET(1) NUMBITS(1) [],

        /// BT DDR 模式
        /// 
        /// 双倍数据率模式，在时钟上升沿和下降沿都采样数据
        REG_BT_DDR_MODE OFFSET(2) NUMBITS(1) [],

        /// HS 由 VDE 门控
        REG_BT_HS_GATE_BY_VDE OFFSET(3) NUMBITS(1) [],

        /// VS (垂直同步) 信号反相
        REG_BT_VS_INV OFFSET(4) NUMBITS(1) [],

        /// HS (水平同步) 信号反相
        REG_BT_HS_INV OFFSET(5) NUMBITS(1) [],

        /// VS 作为 VDE 使用
        REG_BT_VS_AS_VDE OFFSET(6) NUMBITS(1) [],

        /// HS 作为 HDE 使用
        REG_BT_HS_AS_HDE OFFSET(7) NUMBITS(1) [],

        /// 时钟门控软件使能
        /// 
        /// [0]: 延迟控制时钟使能
        /// [1]: 时序解复用器时钟使能
        /// [2]: 时序生成器时钟使能
        /// [3]: RX 解码器 0 时钟使能
        /// [4]: RX 解码器 1 时钟使能
        /// [5]: RX 解码器 2 时钟使能
        /// [6]: RX 解码器 3 时钟使能
        REG_BT_SW_EN_CLK OFFSET(8) NUMBITS(7) [],

        /// Reserved bit 15
        RESERVED_15 OFFSET(15) NUMBITS(1) [],

        /// Demux 设置
        /// 
        /// 2'h0: 无 Demux
        /// 2'h1: Demux 2 通道
        /// 2'h2: Demux 3 通道
        /// 2'h3: Demux 4 通道
        REG_BT_DEMUX_CH OFFSET(16) NUMBITS(2) [
            /// 无 Demux
            None = 0x0,
            /// 2 通道 Demux
            Demux2 = 0x1,
            /// 3 通道 Demux
            Demux3 = 0x2,
            /// 4 通道 Demux
            Demux4 = 0x3
        ],

        /// Reserved bits 18-19
        RESERVED_18_19 OFFSET(18) NUMBITS(2) [],

        /// BT 格式选择
        /// 
        /// 3'b000: bt_2x with sync pattern, 9-bit BT656
        /// 3'b001: bt_1x with sync pattern, 17-bit BT1120
        /// 3'b010: bt_2x without sync pattern, 11-bit BT601 (vhs_mode)
        /// 3'b011: bt_1x without sync pattern, 19-bit BT601 (vhs_mode)
        /// 3'b100: bt_2x without sync pattern, 11-bit BT601 (vde_mode)
        /// 3'b101: bt_1x without sync pattern, 19-bit BT601 (vde_mode)
        /// 3'b110: bt_2x without sync pattern, 11-bit BT601 (vsde_mode)
        /// 3'b111: bt_1x without sync pattern, 19-bit BT601 (vsde_mode)
        REG_BT_FMT_SEL OFFSET(20) NUMBITS(3) [
            /// BT656 9-bit (带同步码)
            Bt656_9bit = 0b000,
            /// BT1120 17-bit (带同步码)
            Bt1120_17bit = 0b001,
            /// BT601 11-bit VHS 模式
            Bt601_11bit_Vhs = 0b010,
            /// BT601 19-bit VHS 模式
            Bt601_19bit_Vhs = 0b011,
            /// BT601 11-bit VDE 模式
            Bt601_11bit_Vde = 0b100,
            /// BT601 19-bit VDE 模式
            Bt601_19bit_Vde = 0b101,
            /// BT601 11-bit VSDE 模式
            Bt601_11bit_Vsde = 0b110,
            /// BT601 19-bit VSDE 模式
            Bt601_19bit_Vsde = 0b111
        ]
    ],

    /// REG_88 - BT 图像尺寸配置寄存器
    pub REG_88 [
        /// BT 图像宽度 (实际值 - 1)
        REG_BT_IMG_WD_M1 OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// BT 图像高度 (实际值 - 1)
        REG_BT_IMG_HT_M1 OFFSET(16) NUMBITS(12) []
    ],

    /// REG_8C - BT 消隐配置寄存器
    pub REG_8C [
        /// BT 垂直同步后消隐 (实际值 - 1)
        REG_BT_VS_BP_M1 OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// BT 水平同步后消隐 (实际值 - 1)
        REG_BT_HS_BP_M1 OFFSET(16) NUMBITS(12) []
    ],

    /// REG_90 - BT 前消隐配置寄存器
    pub REG_90 [
        /// BT 垂直同步前消隐 (实际值 - 1)
        REG_BT_VS_FP_M1 OFFSET(0) NUMBITS(8) [],

        /// BT 水平同步前消隐 (实际值 - 1)
        REG_BT_HS_FP_M1 OFFSET(8) NUMBITS(8) []
    ],

    /// REG_94 - BT 同步码配置寄存器
    pub REG_94 [
        /// BT 同步码字节 0
        REG_BT_SYNC_0 OFFSET(0) NUMBITS(8) [],

        /// BT 同步码字节 1
        REG_BT_SYNC_1 OFFSET(8) NUMBITS(8) [],

        /// BT 同步码字节 2
        REG_BT_SYNC_2 OFFSET(16) NUMBITS(8) []
    ],

    /// REG_98 - BT Demux 0 同步码配置寄存器
    pub REG_98 [
        /// BT Demux 0 有效行 SAV 同步码
        REG_BT_SAV_VLD_0 OFFSET(0) NUMBITS(8) [],

        /// BT Demux 0 消隐行 SAV 同步码
        REG_BT_SAV_BLK_0 OFFSET(8) NUMBITS(8) [],

        /// BT Demux 0 有效行 EAV 同步码
        REG_BT_EAV_VLD_0 OFFSET(16) NUMBITS(8) [],

        /// BT Demux 0 消隐行 EAV 同步码
        REG_BT_EAV_BLK_0 OFFSET(24) NUMBITS(8) []
    ],

    /// REG_9C - BT Demux 1 同步码配置寄存器
    pub REG_9C [
        /// BT Demux 1 有效行 SAV 同步码
        REG_BT_SAV_VLD_1 OFFSET(0) NUMBITS(8) [],

        /// BT Demux 1 消隐行 SAV 同步码
        REG_BT_SAV_BLK_1 OFFSET(8) NUMBITS(8) [],

        /// BT Demux 1 有效行 EAV 同步码
        REG_BT_EAV_VLD_1 OFFSET(16) NUMBITS(8) [],

        /// BT Demux 1 消隐行 EAV 同步码
        REG_BT_EAV_BLK_1 OFFSET(24) NUMBITS(8) []
    ],

    /// REG_A0 - BT Demux 2 同步码配置寄存器
    pub REG_A0 [
        /// BT Demux 2 有效行 SAV 同步码
        REG_BT_SAV_VLD_2 OFFSET(0) NUMBITS(8) [],

        /// BT Demux 2 消隐行 SAV 同步码
        REG_BT_SAV_BLK_2 OFFSET(8) NUMBITS(8) [],

        /// BT Demux 2 有效行 EAV 同步码
        REG_BT_EAV_VLD_2 OFFSET(16) NUMBITS(8) [],

        /// BT Demux 2 消隐行 EAV 同步码
        REG_BT_EAV_BLK_2 OFFSET(24) NUMBITS(8) []
    ],

    /// REG_A4 - BT Demux 3 同步码配置寄存器
    pub REG_A4 [
        /// BT Demux 3 有效行 SAV 同步码
        REG_BT_SAV_VLD_3 OFFSET(0) NUMBITS(8) [],

        /// BT Demux 3 消隐行 SAV 同步码
        REG_BT_SAV_BLK_3 OFFSET(8) NUMBITS(8) [],

        /// BT Demux 3 有效行 EAV 同步码
        REG_BT_EAV_VLD_3 OFFSET(16) NUMBITS(8) [],

        /// BT Demux 3 消隐行 EAV 同步码
        REG_BT_EAV_BLK_3 OFFSET(24) NUMBITS(8) []
    ]
];

// ============================================================================
// REG_B0~REG_B4 - 裁剪配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_B0 - 水平裁剪配置寄存器
    /// 
    /// 配置图像水平方向的裁剪参数
    pub REG_B0 [
        /// 裁剪起始 X 坐标
        /// 
        /// 在 reg_sensor_mac_crop_en 使能时，
        /// 每行中 X 坐标小于此值的像素将被裁剪
        REG_SENSOR_MAC_CROP_START_X OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// 裁剪结束 X 坐标
        /// 
        /// 在 reg_sensor_mac_crop_en 使能时，
        /// 每行中 X 坐标大于此值的像素将被裁剪
        REG_SENSOR_MAC_CROP_END_X OFFSET(16) NUMBITS(13) [],

        /// Reserved bits 29-30
        RESERVED_29_30 OFFSET(29) NUMBITS(2) [],

        /// 裁剪功能使能
        REG_SENSOR_MAC_CROP_EN OFFSET(31) NUMBITS(1) []
    ],

    /// REG_B4 - 垂直裁剪配置寄存器
    /// 
    /// 配置图像垂直方向的裁剪参数
    pub REG_B4 [
        /// 裁剪起始 Y 坐标
        /// 
        /// 在 reg_sensor_mac_crop_en 使能时，
        /// 每帧中 Y 坐标小于此值的行将被裁剪
        REG_SENSOR_MAC_CROP_START_Y OFFSET(0) NUMBITS(13) [],

        /// Reserved bits 13-15
        RESERVED_13_15 OFFSET(13) NUMBITS(3) [],

        /// 裁剪结束 Y 坐标
        /// 
        /// 在 reg_sensor_mac_crop_en 使能时，
        /// 每帧中 Y 坐标大于此值的行将被裁剪
        REG_SENSOR_MAC_CROP_END_Y OFFSET(16) NUMBITS(13) []
    ]
];

// ============================================================================
// REG_D0~REG_FC - Sub-LVDS 模式配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_D0 - Sub-LVDS 模式控制寄存器
    pub REG_D0 [
        /// Sub-LVDS Lane 使能
        REG_TTL_AS_SLVDS_ENABLE OFFSET(0) NUMBITS(1) [],

        /// Reserved bits 1-7
        RESERVED_1_7 OFFSET(1) NUMBITS(7) [],

        /// Sub-LVDS 位宽模式
        /// 
        /// 2'b00: 8-bit
        /// 2'b01: 10-bit
        /// 2'b10: 12-bit
        REG_TTL_AS_SLVDS_BIT_MODE OFFSET(8) NUMBITS(2) [
            /// 8-bit 模式
            Bit8 = 0b00,
            /// 10-bit 模式
            Bit10 = 0b01,
            /// 12-bit 模式
            Bit12 = 0b10
        ],

        /// Sub-LVDS 数据位反转
        REG_TTL_AS_SLVDS_DATA_REVERSE OFFSET(10) NUMBITS(1) [],

        /// Reserved bit 11
        RESERVED_11 OFFSET(11) NUMBITS(1) [],

        /// Sub-LVDS HDR 模式使能
        REG_TTL_AS_SLVDS_HDR_MODE OFFSET(12) NUMBITS(1) [],

        /// Sub-LVDS HDR 模式选择
        /// 
        /// 1'b0: pattern 1
        /// 1'b1: pattern 2
        REG_TTL_AS_SLVDS_HDR_PATTERN OFFSET(13) NUMBITS(1) []
    ],

    /// REG_D4 - Sub-LVDS 同步码配置寄存器 0
    pub REG_D4 [
        /// Sub-LVDS 同步码第一个字
        REG_TTL_AS_SLVDS_SYNC_1ST OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// Sub-LVDS 同步码第二个字
        REG_TTL_AS_SLVDS_SYNC_2ND OFFSET(16) NUMBITS(12) []
    ],

    /// REG_D8 - Sub-LVDS 同步码配置寄存器 1
    pub REG_D8 [
        /// Sub-LVDS 同步码第三个字
        REG_TTL_AS_SLVDS_SYNC_3RD OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// 普通模式消隐 SAV
        REG_TTL_AS_SLVDS_NORM_BK_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_DC - Sub-LVDS 普通模式同步码配置寄存器
    pub REG_DC [
        /// 普通模式消隐 EAV
        REG_TTL_AS_SLVDS_NORM_BK_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// 普通模式有效 SAV
        REG_TTL_AS_SLVDS_NORM_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_E0 - Sub-LVDS 普通模式/HDR N0 同步码配置寄存器
    pub REG_E0 [
        /// 普通模式有效 EAV
        REG_TTL_AS_SLVDS_NORM_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HDR 模式 N0 消隐 SAV
        REG_TTL_AS_SLVDS_N0_BK_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_E4 - Sub-LVDS HDR N0/N1 消隐同步码配置寄存器
    pub REG_E4 [
        /// HDR 模式 N0 消隐 EAV
        REG_TTL_AS_SLVDS_N0_BK_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HDR 模式 N1 消隐 SAV
        REG_TTL_AS_SLVDS_N1_BK_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_E8 - Sub-LVDS HDR N1/N0 LEF 同步码配置寄存器
    pub REG_E8 [
        /// HDR 模式 N1 消隐 EAV
        REG_TTL_AS_SLVDS_N1_BK_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// Sub-LVDS 模式: N0 长曝光 SAV
        /// HiSPi P-SP 模式: SOL T1
        REG_TTL_AS_SLVDS_N0_LEF_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_EC - Sub-LVDS HDR N0 LEF/SEF 同步码配置寄存器
    pub REG_EC [
        /// Sub-LVDS 模式: N0 长曝光 EAV
        /// HiSPi P-SP 模式: EOL T1
        REG_TTL_AS_SLVDS_N0_LEF_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// Sub-LVDS 模式: N0 短曝光 SAV
        /// HiSPi P-SP 模式: SOL T2
        REG_TTL_AS_SLVDS_N0_SEF_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_F0 - Sub-LVDS HDR N0 SEF/N1 LEF 同步码配置寄存器
    pub REG_F0 [
        /// Sub-LVDS 模式: N0 短曝光 EAV
        /// HiSPi P-SP 模式: EOL T2
        REG_TTL_AS_SLVDS_N0_SEF_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// Sub-LVDS 模式: N1 长曝光 SAV
        /// HiSPi P-SP 模式: SOF T1
        REG_TTL_AS_SLVDS_N1_LEF_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_F4 - Sub-LVDS HDR N1 LEF/SEF 同步码配置寄存器
    pub REG_F4 [
        /// Sub-LVDS 模式: N1 长曝光 EAV
        /// HiSPi P-SP 模式: EOF T1
        REG_TTL_AS_SLVDS_N1_LEF_EAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// Sub-LVDS 模式: N1 短曝光 SAV
        /// HiSPi P-SP 模式: SOF T2
        REG_TTL_AS_SLVDS_N1_SEF_SAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_F8 - Sub-LVDS HDR N1 SEF 同步码配置寄存器
    pub REG_F8 [
        /// Sub-LVDS 模式: N1 短曝光 EAV
        /// HiSPi P-SP 模式: EOF T2
        REG_TTL_AS_SLVDS_N1_SEF_EAV OFFSET(0) NUMBITS(12) []
    ],

    /// REG_FC - VS 生成配置寄存器
    pub REG_FC [
        /// VS 生成同步码值
        /// 
        /// 用于 HiSPi P-SP HDR 模式
        REG_TTL_AS_SLVDS_VS_GEN_SYNC_CODE OFFSET(0) NUMBITS(12) [],

        /// VS 由同步码生成
        /// 
        /// 用于 HiSPi P-SP HDR 模式
        REG_TTL_AS_SLVDS_VS_GEN_BY_SYNC_CODE OFFSET(12) NUMBITS(1) []
    ]
];

// ============================================================================
// REG_100~REG_124 - HDR Pattern 2 和 HiSPi 模式配置寄存器
// ============================================================================

register_bitfields! [
    u32,

    /// REG_100 - HDR Pattern 2 N0 同步码配置寄存器
    pub REG_100 [
        /// N0 长短曝光共存行 SAV (仅用于 pattern 2)
        REG_TTL_AS_SLVDS_N0_LSEF_SAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// N0 长短曝光共存行 EAV (仅用于 pattern 2)
        REG_TTL_AS_SLVDS_N0_LSEF_EAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_104 - HDR Pattern 2 N1 同步码配置寄存器
    pub REG_104 [
        /// N1 长短曝光共存行 SAV (仅用于 pattern 2)
        REG_TTL_AS_SLVDS_N1_LSEF_SAV OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// N1 长短曝光共存行 EAV (仅用于 pattern 2)
        REG_TTL_AS_SLVDS_N1_LSEF_EAV OFFSET(16) NUMBITS(12) []
    ],

    /// REG_108 - HDR Pattern 2 尺寸配置寄存器
    pub REG_108 [
        /// Pattern 2 水平尺寸
        REG_TTL_AS_SLVDS_HDR_P2_HSIZE OFFSET(0) NUMBITS(14) [],

        /// Reserved bits 14-15
        RESERVED_14_15 OFFSET(14) NUMBITS(2) [],

        /// Pattern 2 水平消隐尺寸
        REG_TTL_AS_SLVDS_HDR_P2_HBLANK OFFSET(16) NUMBITS(14) []
    ],

    /// REG_110 - HiSPi 模式控制寄存器 0
    pub REG_110 [
        /// HiSPi 模式使能
        /// 
        /// 1'b0: Sub-LVDS 模式
        /// 1'b1: HiSPi 模式
        REG_TTL_AS_HISPI_MODE OFFSET(0) NUMBITS(1) [],

        /// HiSPi DE 由寄存器计数取消断言
        REG_TTL_AS_HISPI_USE_HSIZE OFFSET(1) NUMBITS(1) [],

        /// Reserved bits 2-3
        RESERVED_2_3 OFFSET(2) NUMBITS(2) [],

        /// HiSPi P-SP HDR 模式使能
        REG_TTL_AS_HISPI_HDR_PSP_MODE OFFSET(4) NUMBITS(1) []
    ],

    /// REG_114 - HiSPi 普通模式同步码配置寄存器
    pub REG_114 [
        /// HiSPi SOF (Start of Frame) 同步码
        REG_TTL_AS_HISPI_NORM_SOF OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HiSPi EOF (End of Frame) 同步码
        REG_TTL_AS_HISPI_NORM_EOF OFFSET(16) NUMBITS(12) []
    ],

    /// REG_118 - HiSPi HDR T1 帧同步码配置寄存器
    pub REG_118 [
        /// HiSPi HDR T1 SOF
        REG_TTL_AS_HISPI_HDR_T1_SOF OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HiSPi HDR T1 EOF
        REG_TTL_AS_HISPI_HDR_T1_EOF OFFSET(16) NUMBITS(12) []
    ],

    /// REG_11C - HiSPi HDR T1 行同步码配置寄存器
    pub REG_11C [
        /// HiSPi HDR T1 SOL (Start of Line)
        REG_TTL_AS_HISPI_HDR_T1_SOL OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HiSPi HDR T1 EOL (End of Line)
        REG_TTL_AS_HISPI_HDR_T1_EOL OFFSET(16) NUMBITS(12) []
    ],

    /// REG_120 - HiSPi HDR T2 帧同步码配置寄存器
    pub REG_120 [
        /// HiSPi HDR T2 SOF
        REG_TTL_AS_HISPI_HDR_T2_SOF OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HiSPi HDR T2 EOF
        REG_TTL_AS_HISPI_HDR_T2_EOF OFFSET(16) NUMBITS(12) []
    ],

    /// REG_124 - HiSPi HDR T2 行同步码配置寄存器
    pub REG_124 [
        /// HiSPi HDR T2 SOL
        REG_TTL_AS_HISPI_HDR_T2_SOL OFFSET(0) NUMBITS(12) [],

        /// Reserved bits 12-15
        RESERVED_12_15 OFFSET(12) NUMBITS(4) [],

        /// HiSPi HDR T2 EOL
        REG_TTL_AS_HISPI_HDR_T2_EOL OFFSET(16) NUMBITS(12) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// VI 寄存器组
    /// 
    /// VI (Video Input) 模块的完整寄存器映射
    /// 基址: VI0 = 0x0A0C2000, VI1 = 0x0A0C4000, VI2 = 0x0A0C6000
    pub ViRegs {
        /// REG_00 - 模式控制寄存器
        /// 
        /// 配置传感器 MAC 模式、BT Demux、CSI/Sub-LVDS 控制器使能等
        (0x000 => pub reg_00: ReadWrite<u32, REG_00::Register>),

        /// Reserved 0x004 ~ 0x00C
        (0x004 => _reserved0),

        /// REG_10 - TTL 模式配置寄存器 0
        /// 
        /// 配置 TTL 使能、位宽、输出格式、输入格式等
        (0x010 => pub reg_10: ReadWrite<u32, REG_10::Register>),

        /// REG_14 - TTL 消隐配置寄存器
        /// 
        /// 配置 VS/HS 后消隐
        (0x014 => pub reg_14: ReadWrite<u32, REG_14::Register>),

        /// REG_18 - TTL 图像尺寸配置寄存器
        (0x018 => pub reg_18: ReadWrite<u32, REG_18::Register>),

        /// REG_1C - TTL 同步码配置寄存器 0
        (0x01c => pub reg_1c: ReadWrite<u32, REG_1C::Register>),

        /// REG_20 - TTL 同步码配置寄存器 1
        (0x020 => pub reg_20: ReadWrite<u32, REG_20::Register>),

        /// REG_24 - TTL SAV 同步码配置寄存器
        (0x024 => pub reg_24: ReadWrite<u32, REG_24::Register>),

        /// REG_28 - TTL EAV 同步码配置寄存器
        (0x028 => pub reg_28: ReadWrite<u32, REG_28::Register>),

        /// Reserved 0x02C
        (0x02c => _reserved1),

        /// REG_30 - VI 选择配置寄存器
        /// 
        /// 配置 VI 输入模式、来源、时钟反相等
        (0x030 => pub reg_30: ReadWrite<u32, REG_30::Register>),

        /// Reserved 0x034 ~ 0x03C
        (0x034 => _reserved2),

        /// REG_40 - HDR 模式配置寄存器 0
        (0x040 => pub reg_40: ReadWrite<u32, REG_40::Register>),

        /// REG_44 - HDR 模式配置寄存器 1
        (0x044 => pub reg_44: ReadWrite<u32, REG_44::Register>),

        /// REG_48 - 信息行配置寄存器
        (0x048 => pub reg_48: ReadWrite<u32, REG_48::Register>),

        /// Reserved 0x04C
        (0x04c => _reserved3),

        /// REG_50 - BLC 模式配置寄存器
        (0x050 => pub reg_50: ReadWrite<u32, REG_50::Register>),

        /// REG_54 - BLC0 配置寄存器
        (0x054 => pub reg_54: ReadWrite<u32, REG_54::Register>),

        /// REG_58 - BLC1 配置寄存器
        (0x058 => pub reg_58: ReadWrite<u32, REG_58::Register>),

        /// Reserved 0x05C
        (0x05c => _reserved4),

        /// REG_60 - VI 引脚选择寄存器 0 (VS/HS/VDE/HDE)
        (0x060 => pub reg_60: ReadWrite<u32, REG_60::Register>),

        /// REG_64 - VI 数据引脚选择寄存器 0 (D0~D3)
        (0x064 => pub reg_64: ReadWrite<u32, REG_64::Register>),

        /// REG_68 - VI 数据引脚选择寄存器 1 (D4~D7)
        (0x068 => pub reg_68: ReadWrite<u32, REG_68::Register>),

        /// REG_6C - VI 数据引脚选择寄存器 2 (D8~D11)
        (0x06c => pub reg_6c: ReadWrite<u32, REG_6C::Register>),

        /// REG_70 - VI 数据引脚选择寄存器 3 (D12~D15)
        (0x070 => pub reg_70: ReadWrite<u32, REG_70::Register>),

        /// REG_74 - VI BT 数据引脚选择寄存器 (D0~D7)
        (0x074 => pub reg_74: ReadWrite<u32, REG_74::Register>),

        /// Reserved 0x078 ~ 0x07C
        (0x078 => _reserved5),

        /// REG_80 - BT 路径配置寄存器 0
        (0x080 => pub reg_80: ReadWrite<u32, REG_80::Register>),

        /// Reserved 0x084
        (0x084 => _reserved6),

        /// REG_88 - BT 图像尺寸配置寄存器
        (0x088 => pub reg_88: ReadWrite<u32, REG_88::Register>),

        /// REG_8C - BT 消隐配置寄存器
        (0x08c => pub reg_8c: ReadWrite<u32, REG_8C::Register>),

        /// REG_90 - BT 前消隐配置寄存器
        (0x090 => pub reg_90: ReadWrite<u32, REG_90::Register>),

        /// REG_94 - BT 同步码配置寄存器
        (0x094 => pub reg_94: ReadWrite<u32, REG_94::Register>),

        /// REG_98 - BT Demux 0 同步码配置寄存器
        (0x098 => pub reg_98: ReadWrite<u32, REG_98::Register>),

        /// REG_9C - BT Demux 1 同步码配置寄存器
        (0x09c => pub reg_9c: ReadWrite<u32, REG_9C::Register>),

        /// REG_A0 - BT Demux 2 同步码配置寄存器
        (0x0a0 => pub reg_a0: ReadWrite<u32, REG_A0::Register>),

        /// REG_A4 - BT Demux 3 同步码配置寄存器
        (0x0a4 => pub reg_a4: ReadWrite<u32, REG_A4::Register>),

        /// Reserved 0x0A8 ~ 0x0AC
        (0x0a8 => _reserved7),

        /// REG_B0 - 水平裁剪配置寄存器
        (0x0b0 => pub reg_b0: ReadWrite<u32, REG_B0::Register>),

        /// REG_B4 - 垂直裁剪配置寄存器
        (0x0b4 => pub reg_b4: ReadWrite<u32, REG_B4::Register>),

        /// Reserved 0x0B8 ~ 0x0CC
        (0x0b8 => _reserved8),

        /// REG_D0 - Sub-LVDS 模式控制寄存器
        (0x0d0 => pub reg_d0: ReadWrite<u32, REG_D0::Register>),

        /// REG_D4 - Sub-LVDS 同步码配置寄存器 0
        (0x0d4 => pub reg_d4: ReadWrite<u32, REG_D4::Register>),

        /// REG_D8 - Sub-LVDS 同步码配置寄存器 1
        (0x0d8 => pub reg_d8: ReadWrite<u32, REG_D8::Register>),

        /// REG_DC - Sub-LVDS 普通模式同步码配置寄存器
        (0x0dc => pub reg_dc: ReadWrite<u32, REG_DC::Register>),

        /// REG_E0 - Sub-LVDS 普通模式/HDR N0 同步码配置寄存器
        (0x0e0 => pub reg_e0: ReadWrite<u32, REG_E0::Register>),

        /// REG_E4 - Sub-LVDS HDR N0/N1 消隐同步码配置寄存器
        (0x0e4 => pub reg_e4: ReadWrite<u32, REG_E4::Register>),

        /// REG_E8 - Sub-LVDS HDR N1/N0 LEF 同步码配置寄存器
        (0x0e8 => pub reg_e8: ReadWrite<u32, REG_E8::Register>),

        /// REG_EC - Sub-LVDS HDR N0 LEF/SEF 同步码配置寄存器
        (0x0ec => pub reg_ec: ReadWrite<u32, REG_EC::Register>),

        /// REG_F0 - Sub-LVDS HDR N0 SEF/N1 LEF 同步码配置寄存器
        (0x0f0 => pub reg_f0: ReadWrite<u32, REG_F0::Register>),

        /// REG_F4 - Sub-LVDS HDR N1 LEF/SEF 同步码配置寄存器
        (0x0f4 => pub reg_f4: ReadWrite<u32, REG_F4::Register>),

        /// REG_F8 - Sub-LVDS HDR N1 SEF 同步码配置寄存器
        (0x0f8 => pub reg_f8: ReadWrite<u32, REG_F8::Register>),

        /// REG_FC - VS 生成配置寄存器
        (0x0fc => pub reg_fc: ReadWrite<u32, REG_FC::Register>),

        /// REG_100 - HDR Pattern 2 N0 同步码配置寄存器
        (0x100 => pub reg_100: ReadWrite<u32, REG_100::Register>),

        /// REG_104 - HDR Pattern 2 N1 同步码配置寄存器
        (0x104 => pub reg_104: ReadWrite<u32, REG_104::Register>),

        /// REG_108 - HDR Pattern 2 尺寸配置寄存器
        (0x108 => pub reg_108: ReadWrite<u32, REG_108::Register>),

        /// Reserved 0x10C
        (0x10c => _reserved9),

        /// REG_110 - HiSPi 模式控制寄存器 0
        (0x110 => pub reg_110: ReadWrite<u32, REG_110::Register>),

        /// REG_114 - HiSPi 普通模式同步码配置寄存器
        (0x114 => pub reg_114: ReadWrite<u32, REG_114::Register>),

        /// REG_118 - HiSPi HDR T1 帧同步码配置寄存器
        (0x118 => pub reg_118: ReadWrite<u32, REG_118::Register>),

        /// REG_11C - HiSPi HDR T1 行同步码配置寄存器
        (0x11c => pub reg_11c: ReadWrite<u32, REG_11C::Register>),

        /// REG_120 - HiSPi HDR T2 帧同步码配置寄存器
        (0x120 => pub reg_120: ReadWrite<u32, REG_120::Register>),

        /// REG_124 - HiSPi HDR T2 行同步码配置寄存器
        (0x124 => pub reg_124: ReadWrite<u32, REG_124::Register>),

        /// 结束标记
        (0x128 => @END),
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取 VI0 寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn vi0_regs() -> &'static ViRegs {
    &*(VI0_BASE as *const ViRegs)
}

/// 获取 VI1 寄存器引用
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn vi1_regs() -> &'static ViRegs {
    &*(VI1_BASE as *const ViRegs)
}

/// 获取 VI2 寄存器引用 (仅支持 BT 接口)
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn vi2_regs() -> &'static ViRegs {
    &*(VI2_BASE as *const ViRegs)
}

/// 根据索引获取 VI 寄存器引用
///
/// # Arguments
/// * `index` - VI 索引 (0, 1, 2)
///
/// # Returns
/// * `Some(&'static ViRegs)` - 如果索引有效
/// * `None` - 如果索引无效
///
/// # Safety
/// 调用者必须确保寄存器地址有效且可访问
#[inline]
pub unsafe fn vi_regs(index: usize) -> Option<&'static ViRegs> {
    match index {
        0 => Some(vi0_regs()),
        1 => Some(vi1_regs()),
        2 => Some(vi2_regs()),
        _ => None,
    }
}

/// 获取 VI 寄存器基地址
///
/// # Arguments
/// * `index` - VI 索引 (0, 1, 2)
///
/// # Returns
/// * 寄存器基地址，如果索引无效则返回 0
#[inline]
pub const fn vi_base_addr(index: usize) -> usize {
    match index {
        0 => VI0_BASE,
        1 => VI1_BASE,
        2 => VI2_BASE,
        _ => 0,
    }
}
