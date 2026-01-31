//! MIPI RX 类型定义
//!
//! 本模块定义了 MIPI RX 驱动所需的各种类型和枚举

#![allow(dead_code)]

// ============================================================================
// 错误类型
// ============================================================================

/// MIPI RX 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipiRxError {
    /// 无效的设备编号
    InvalidDevno,
    /// 无效的 Lane 配置
    InvalidLaneConfig,
    /// 无效的数据类型
    InvalidDataType,
    /// 无效的 PHY 模式
    InvalidPhyMode,
    /// 配置错误
    ConfigError,
    /// ECC 错误
    EccError,
    /// CRC 错误
    CrcError,
    /// FIFO 满
    FifoFull,
    /// 超时
    Timeout,
    /// 设备忙
    Busy,
}

// ============================================================================
// PHY 模式
// ============================================================================

/// Sensor PHY 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum PhyMode {
    /// 1C4D 模式 (1 clock + 4 data lanes)
    #[default]
    Mode1C4D = 0,
    /// 1C2D + 1C2D 双端口模式
    Mode1C2D_1C2D = 1,
}

impl PhyMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Mode1C4D),
            1 => Some(Self::Mode1C2D_1C2D),
            _ => None,
        }
    }
}

/// Sensor 接口模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SensorMode {
    /// MIPI CSI-2 模式
    #[default]
    Csi = 0,
    /// Sub-LVDS / HiSPi 模式
    SubLvds = 1,
    /// SLVSEC 模式
    Slvsec = 2,
}

impl SensorMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Csi),
            1 => Some(Self::SubLvds),
            2 => Some(Self::Slvsec),
            _ => None,
        }
    }
}

// ============================================================================
// Lane 配置
// ============================================================================

/// CSI Lane 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LaneMode {
    /// 1-lane 模式
    Lane1 = 0b000,
    /// 2-lane 模式
    #[default]
    Lane2 = 0b001,
    /// 4-lane 模式
    Lane4 = 0b011,
    /// 8-lane 模式 (双 CSI)
    Lane8 = 0b111,
}

impl LaneMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b000 => Some(Self::Lane1),
            0b001 => Some(Self::Lane2),
            0b011 => Some(Self::Lane4),
            0b111 => Some(Self::Lane8),
            _ => None,
        }
    }

    /// 获取 Lane 数量
    pub fn lane_count(&self) -> u8 {
        match self {
            Self::Lane1 => 1,
            Self::Lane2 => 2,
            Self::Lane4 => 4,
            Self::Lane8 => 8,
        }
    }
}

/// Lane ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LaneId {
    Lane0 = 0,
    Lane1 = 1,
    Lane2 = 2,
    Lane3 = 3,
    Lane4 = 4,
    Lane5 = 5,
}

impl LaneId {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Lane0),
            1 => Some(Self::Lane1),
            2 => Some(Self::Lane2),
            3 => Some(Self::Lane3),
            4 => Some(Self::Lane4),
            5 => Some(Self::Lane5),
            _ => None,
        }
    }
}

/// Deskew Lane 使能配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DeskewLaneEnable {
    /// 无 Lane
    #[default]
    None = 0x0,
    /// 1-lane
    Lane1 = 0x1,
    /// 2-lane
    Lane2 = 0x3,
    /// 4-lane
    Lane4 = 0xf,
}

impl DeskewLaneEnable {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::Lane1),
            0x3 => Some(Self::Lane2),
            0xf => Some(Self::Lane4),
            _ => None,
        }
    }
}

// ============================================================================
// 数据格式
// ============================================================================

/// 原始数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RawDataType {
    /// RAW8
    Raw8 = 0,
    /// RAW10
    #[default]
    Raw10 = 1,
    /// RAW12
    Raw12 = 2,
    /// RAW16
    Raw16 = 3,
    /// YUV422 8-bit
    Yuv422_8bit = 4,
    /// YUV422 10-bit
    Yuv422_10bit = 5,
}

impl RawDataType {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Raw8),
            1 => Some(Self::Raw10),
            2 => Some(Self::Raw12),
            3 => Some(Self::Raw16),
            4 => Some(Self::Yuv422_8bit),
            5 => Some(Self::Yuv422_10bit),
            _ => None,
        }
    }

    /// 获取 MIPI CSI-2 Data Type 值
    pub fn csi_data_type(&self) -> u8 {
        match self {
            Self::Raw8 => 0x2A,
            Self::Raw10 => 0x2B,
            Self::Raw12 => 0x2C,
            Self::Raw16 => 0x2E,
            Self::Yuv422_8bit => 0x1E,
            Self::Yuv422_10bit => 0x1F,
        }
    }

    /// 获取 BLC 格式设置值
    pub fn blc_format(&self) -> u8 {
        match self {
            Self::Yuv422_8bit => 0,
            Self::Yuv422_10bit => 1,
            Self::Raw8 => 2,
            Self::Raw10 => 3,
            Self::Raw12 => 4,
            Self::Raw16 => 5,
        }
    }
}

/// Sub-LVDS 位宽模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SubLvdsBitMode {
    /// 8-bit 模式
    Bit8 = 0,
    /// 10-bit 模式
    #[default]
    Bit10 = 1,
    /// 12-bit 模式
    Bit12 = 2,
}

impl SubLvdsBitMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Bit8),
            1 => Some(Self::Bit10),
            2 => Some(Self::Bit12),
            _ => None,
        }
    }
}

// ============================================================================
// HDR 模式
// ============================================================================

/// HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum HdrMode {
    /// 无 HDR
    #[default]
    None = 0,
    /// VC (Virtual Channel) 模式
    Vc = 1,
    /// ID 模式
    Id = 2,
    /// DT (Data Type) 模式
    Dt = 3,
    /// DOL (Digital Overlap) 模式
    Dol = 4,
    /// 手动模式
    Manual = 5,
}

impl HdrMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Vc),
            2 => Some(Self::Id),
            3 => Some(Self::Dt),
            4 => Some(Self::Dol),
            5 => Some(Self::Manual),
            _ => None,
        }
    }
}

/// VS 生成模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum VsGenMode {
    /// 由 FS (Frame Start) 生成
    ByFs = 0,
    /// 由 FE (Frame End) 生成
    ByFe = 1,
    /// 由 FS 和 FE 生成
    #[default]
    ByFsFe = 2,
}

impl VsGenMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::ByFs),
            1 => Some(Self::ByFe),
            2 | 3 => Some(Self::ByFsFe),
            _ => None,
        }
    }
}

// ============================================================================
// 中断类型
// ============================================================================

/// CSI 中断类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsiInterrupt {
    /// ECC 错误
    EccError = 0,
    /// CRC 错误
    CrcError = 1,
    /// HDR ID 错误
    HdrIdError = 2,
    /// Word Count 错误
    WcError = 3,
    /// FIFO 满
    FifoFull = 4,
}

impl CsiInterrupt {
    /// 获取中断掩码
    pub fn mask(&self) -> u8 {
        1 << (*self as u8)
    }
}

/// CSI 中断状态
#[derive(Debug, Clone, Copy, Default)]
pub struct CsiInterruptStatus {
    /// ECC 错误
    pub ecc_error: bool,
    /// CRC 错误
    pub crc_error: bool,
    /// HDR ID 错误
    pub hdr_id_error: bool,
    /// Word Count 错误
    pub wc_error: bool,
    /// FIFO 满
    pub fifo_full: bool,
}

impl From<u8> for CsiInterruptStatus {
    fn from(value: u8) -> Self {
        Self {
            ecc_error: (value & 0x01) != 0,
            crc_error: (value & 0x02) != 0,
            hdr_id_error: (value & 0x04) != 0,
            wc_error: (value & 0x08) != 0,
            fifo_full: (value & 0x10) != 0,
        }
    }
}

impl CsiInterruptStatus {
    /// 检查是否有任何错误
    pub fn has_error(&self) -> bool {
        self.ecc_error || self.crc_error || self.hdr_id_error || self.wc_error || self.fifo_full
    }
}

// ============================================================================
// 配置结构体
// ============================================================================

/// MIPI RX 设备属性
#[derive(Debug, Clone, Copy, Default)]
pub struct MipiRxDevAttr {
    /// 设备编号 (0 或 1)
    pub devno: u8,
    /// Sensor 接口模式
    pub sensor_mode: SensorMode,
    /// Lane 模式
    pub lane_mode: LaneMode,
    /// 数据类型
    pub data_type: RawDataType,
    /// HDR 模式
    pub hdr_mode: HdrMode,
    /// Lane ID 配置 (最多 4 个数据 Lane)
    /// 值为 -1 表示不使用该 Lane
    pub lane_id: [i8; 4],
    /// 时钟 Lane 选择
    pub clk_lane_sel: u8,
    /// Lane PN 交换配置
    pub pn_swap: [bool; 5],
    /// 图像宽度
    pub img_width: u16,
    /// 图像高度
    pub img_height: u16,
}

/// Sub-LVDS 设备属性
#[derive(Debug, Clone, Copy, Default)]
pub struct SubLvdsDevAttr {
    /// 设备编号
    pub devno: u8,
    /// 位宽模式
    pub bit_mode: SubLvdsBitMode,
    /// Lane 使能 (位掩码)
    pub lane_enable: u8,
    /// MSB 优先
    pub msb_first: bool,
    /// 同步码第一个符号
    pub sav_1st: u16,
    /// 同步码第二个符号
    pub sav_2nd: u16,
    /// 同步码第三个符号
    pub sav_3rd: u16,
    /// 图像宽度
    pub img_width: u16,
    /// 图像高度
    pub img_height: u16,
}

/// VC 映射配置
#[derive(Debug, Clone, Copy, Default)]
pub struct VcMapping {
    /// VC 映射到 ISP 通道 00
    pub ch00: u8,
    /// VC 映射到 ISP 通道 01
    pub ch01: u8,
    /// VC 映射到 ISP 通道 10
    pub ch10: u8,
    /// VC 映射到 ISP 通道 11
    pub ch11: u8,
}

impl VcMapping {
    /// 创建默认映射 (直通)
    pub fn default_passthrough() -> Self {
        Self {
            ch00: 0,
            ch01: 1,
            ch10: 2,
            ch11: 3,
        }
    }
}

/// CSI 状态
#[derive(Debug, Clone, Copy, Default)]
pub struct CsiStatus {
    /// ECC 无错误
    pub ecc_no_error: bool,
    /// ECC 已纠正错误
    pub ecc_corrected: bool,
    /// ECC 错误
    pub ecc_error: bool,
    /// CRC 错误
    pub crc_error: bool,
    /// Word Count 错误
    pub wc_error: bool,
    /// FIFO 满
    pub fifo_full: bool,
    /// 解码格式
    pub decode_format: u8,
}

impl CsiStatus {
    /// 检查是否有任何错误
    pub fn has_error(&self) -> bool {
        self.ecc_error || self.crc_error || self.wc_error || self.fifo_full
    }
}

// ============================================================================
// 常量定义
// ============================================================================

/// 最大 CSI 设备数量
pub const MAX_CSI_NUM: usize = 2;

/// 最大 Lane 数量
pub const MAX_LANE_NUM: usize = 4;

/// 最大 PHY Lane 数量
pub const MAX_PHY_LANE_NUM: usize = 6;

/// 默认 HS Settle 时间
pub const DEFAULT_HS_SETTLE: u8 = 0x08;

/// 默认时钟相位
pub const DEFAULT_CLK_PHASE: u8 = 0x00;
