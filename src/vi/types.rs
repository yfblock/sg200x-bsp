//! VI (Video Input) 类型定义
//!
//! 本模块定义了 VI 驱动所需的各种类型和枚举
//!
//! VI 模块支持以下接口类型：
//! - MIPI CSI-2
//! - Sub-LVDS
//! - HiSPi
//! - BT.656/BT.601/BT.1120 (TTL 接口)
//! - DC (Digital Camera)

#![allow(dead_code)]
#![allow(non_camel_case_types)]

// ============================================================================
// 错误类型
// ============================================================================

/// VI 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViError {
    /// 无效的设备编号
    /// 
    /// VI 设备编号必须为 0、1 或 2
    InvalidDevno,

    /// 无效的配置参数
    InvalidConfig,

    /// 无效的输入模式
    InvalidInputMode,

    /// 无效的图像尺寸
    /// 
    /// 图像尺寸超出支持范围
    InvalidImageSize,

    /// 无效的同步码
    InvalidSyncCode,

    /// 设备忙
    /// 
    /// 设备正在处理中，无法执行新操作
    Busy,

    /// 超时
    Timeout,

    /// 同步丢失
    /// 
    /// 输入信号同步丢失
    SyncLost,

    /// FIFO 溢出
    FifoOverflow,

    /// 未初始化
    NotInitialized,
}

// ============================================================================
// VI 设备和模式
// ============================================================================

/// VI 设备编号
/// 
/// SG2002 芯片有三个 VI 设备：
/// - VI0: 支持所有接口类型
/// - VI1: 支持所有接口类型
/// - VI2: 仅支持 BT 接口
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ViDevno {
    /// VI0 设备
    #[default]
    Vi0 = 0,
    /// VI1 设备
    Vi1 = 1,
    /// VI2 设备 (仅支持 BT 接口)
    Vi2 = 2,
}

impl ViDevno {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Vi0),
            1 => Some(Self::Vi1),
            2 => Some(Self::Vi2),
            _ => None,
        }
    }

    /// 检查是否支持 MIPI/Sub-LVDS/HiSPi 接口
    /// 
    /// VI2 仅支持 BT 接口
    pub fn supports_mipi(&self) -> bool {
        matches!(self, Self::Vi0 | Self::Vi1)
    }
}

/// 传感器 MAC 模式
/// 
/// 配置 VI 的工作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SensorMacMode {
    /// 禁用
    #[default]
    Disable = 0b000,
    /// CSI 模式 (MIPI CSI-2)
    Csi = 0b001,
    /// Sub-LVDS 模式
    SubLvds = 0b010,
    /// TTL 模式 (BT.656/BT.601/BT.1120/DC)
    Ttl = 0b011,
}

impl SensorMacMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b000 => Some(Self::Disable),
            0b001 => Some(Self::Csi),
            0b010 => Some(Self::SubLvds),
            0b011 => Some(Self::Ttl),
            _ => None,
        }
    }
}

/// VI 输入模式选择
/// 
/// 用于 REG_30 的 reg_vi_sel 字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ViInputMode {
    /// RAW 模式
    #[default]
    Raw = 0x1,
    /// BT601 模式
    Bt601 = 0x2,
    /// BT656 模式
    Bt656 = 0x3,
    /// BT1120 模式
    Bt1120 = 0x4,
}

impl ViInputMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x1 => Some(Self::Raw),
            0x2 => Some(Self::Bt601),
            0x3 => Some(Self::Bt656),
            0x4 => Some(Self::Bt1120),
            _ => None,
        }
    }
}

/// VI 输入来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ViInputSource {
    /// 来自 VI0
    #[default]
    FromVi0 = 0,
    /// 来自 VI1
    FromVi1 = 1,
}

// ============================================================================
// TTL 接口配置
// ============================================================================

/// TTL 传感器位宽
/// 
/// 配置 TTL 接口的数据位宽
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TtlBitWidth {
    /// 8-bit 模式
    #[default]
    Bit8 = 0b00,
    /// 10-bit 模式
    Bit10 = 0b01,
    /// 12-bit 模式
    Bit12 = 0b10,
    /// 16-bit 模式
    Bit16 = 0b11,
}

impl TtlBitWidth {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::Bit8),
            0b01 => Some(Self::Bit10),
            0b10 => Some(Self::Bit12),
            0b11 => Some(Self::Bit16),
            _ => None,
        }
    }

    /// 获取实际位宽
    pub fn bits(&self) -> u8 {
        match self {
            Self::Bit8 => 8,
            Self::Bit10 => 10,
            Self::Bit12 => 12,
            Self::Bit16 => 16,
        }
    }
}

/// TTL BT 输出格式
/// 
/// 配置 YUV422 数据的输出顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TtlBtFmtOut {
    /// {Cb,Y},{Cr,Y} 格式
    #[default]
    CbYCrY = 0b00,
    /// {Cr,Y},{Cb,Y} 格式
    CrYCbY = 0b01,
    /// {Y,Cb},{Y,Cr} 格式
    YCbYCr = 0b10,
    /// {Y,Cr},{Y,Cb} 格式
    YCrYCb = 0b11,
}

impl TtlBtFmtOut {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::CbYCrY),
            0b01 => Some(Self::CrYCbY),
            0b10 => Some(Self::YCbYCr),
            0b11 => Some(Self::YCrYCb),
            _ => None,
        }
    }
}

/// TTL 输入格式
/// 
/// 配置 TTL 接口的输入格式和同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TtlInputFormat {
    /// BT656 9-bit (带同步码)
    /// 
    /// 8-bit 数据 + 时钟，使用嵌入式同步码
    #[default]
    Bt656_9bit = 0b0000,

    /// BT1120 17-bit (带同步码)
    /// 
    /// 16-bit 数据 + 时钟，使用嵌入式同步码
    Bt1120_17bit = 0b0001,

    /// BT601 11-bit VHS 模式 (无同步码)
    /// 
    /// 8-bit 数据 + 时钟 + VS + HS
    Bt601_11bit_Vhs = 0b0010,

    /// BT601 19-bit VHS 模式 (无同步码)
    /// 
    /// 16-bit 数据 + 时钟 + VS + HS
    Bt601_19bit_Vhs = 0b0011,

    /// BT601 11-bit VDE 模式 (无同步码)
    /// 
    /// 8-bit 数据 + 时钟 + VDE + HDE
    Bt601_11bit_Vde = 0b0100,

    /// BT601 19-bit VDE 模式 (无同步码)
    /// 
    /// 16-bit 数据 + 时钟 + VDE + HDE
    Bt601_19bit_Vde = 0b0101,

    /// BT601 11-bit VSDE 模式 (无同步码)
    /// 
    /// 8-bit 数据 + 时钟 + VS + HDE
    Bt601_11bit_Vsde = 0b0110,

    /// BT601 19-bit VSDE 模式 (无同步码)
    /// 
    /// 16-bit 数据 + 时钟 + VS + HDE
    Bt601_19bit_Vsde = 0b0111,

    /// Sensor 带同步码
    SensorWithSync = 0b1000,

    /// Sensor VHS 模式 (无同步码)
    /// 
    /// 使用 VS + HS 同步
    SensorVhs = 0b1010,

    /// Sensor VDE 模式 (无同步码)
    /// 
    /// 使用 VDE + HDE 同步
    SensorVde = 0b1100,

    /// Sensor VSDE 模式 (无同步码)
    /// 
    /// 使用 VS + HDE 同步
    SensorVsde = 0b1110,
}

impl TtlInputFormat {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b0000 => Some(Self::Bt656_9bit),
            0b0001 => Some(Self::Bt1120_17bit),
            0b0010 => Some(Self::Bt601_11bit_Vhs),
            0b0011 => Some(Self::Bt601_19bit_Vhs),
            0b0100 => Some(Self::Bt601_11bit_Vde),
            0b0101 => Some(Self::Bt601_19bit_Vde),
            0b0110 => Some(Self::Bt601_11bit_Vsde),
            0b0111 => Some(Self::Bt601_19bit_Vsde),
            0b1000 | 0b1001 => Some(Self::SensorWithSync),
            0b1010 | 0b1011 => Some(Self::SensorVhs),
            0b1100 | 0b1101 => Some(Self::SensorVde),
            0b1110 | 0b1111 => Some(Self::SensorVsde),
            _ => None,
        }
    }

    /// 检查是否使用嵌入式同步码
    pub fn uses_sync_code(&self) -> bool {
        matches!(
            self,
            Self::Bt656_9bit | Self::Bt1120_17bit | Self::SensorWithSync
        )
    }

    /// 检查是否为 16-bit 模式
    pub fn is_16bit(&self) -> bool {
        matches!(
            self,
            Self::Bt1120_17bit
                | Self::Bt601_19bit_Vhs
                | Self::Bt601_19bit_Vde
                | Self::Bt601_19bit_Vsde
        )
    }
}

/// TTL BT 数据序列
/// 
/// 配置 YUV422 数据的采样顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TtlBtDataSeq {
    /// Cb0-Y0-Cr0-Y1 序列
    #[default]
    Cb0Y0Cr0Y1 = 0b00,
    /// Cr0-Y0-Cb0-Y1 序列
    Cr0Y0Cb0Y1 = 0b01,
    /// Y0-Cb0-Y1-Cr0 序列
    Y0Cb0Y1Cr0 = 0b10,
    /// Y0-Cr0-Y1-Cb0 序列
    Y0Cr0Y1Cb0 = 0b11,
}

impl TtlBtDataSeq {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::Cb0Y0Cr0Y1),
            0b01 => Some(Self::Cr0Y0Cb0Y1),
            0b10 => Some(Self::Y0Cb0Y1Cr0),
            0b11 => Some(Self::Y0Cr0Y1Cb0),
            _ => None,
        }
    }
}

// ============================================================================
// BT 接口配置
// ============================================================================

/// BT 格式选择
/// 
/// 配置 BT 接口的格式和同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BtFormat {
    /// BT656 9-bit (带同步码)
    #[default]
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
    Bt601_19bit_Vsde = 0b111,
}

impl BtFormat {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b000 => Some(Self::Bt656_9bit),
            0b001 => Some(Self::Bt1120_17bit),
            0b010 => Some(Self::Bt601_11bit_Vhs),
            0b011 => Some(Self::Bt601_19bit_Vhs),
            0b100 => Some(Self::Bt601_11bit_Vde),
            0b101 => Some(Self::Bt601_19bit_Vde),
            0b110 => Some(Self::Bt601_11bit_Vsde),
            0b111 => Some(Self::Bt601_19bit_Vsde),
            _ => None,
        }
    }

    /// 检查是否使用嵌入式同步码
    pub fn uses_sync_code(&self) -> bool {
        matches!(self, Self::Bt656_9bit | Self::Bt1120_17bit)
    }

    /// 检查是否为 16-bit 模式
    pub fn is_16bit(&self) -> bool {
        matches!(
            self,
            Self::Bt1120_17bit
                | Self::Bt601_19bit_Vhs
                | Self::Bt601_19bit_Vde
                | Self::Bt601_19bit_Vsde
        )
    }
}

/// BT Demux 通道数
/// 
/// 配置 BT 多通道融合输入的通道数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BtDemuxChannel {
    /// 无 Demux (单通道)
    #[default]
    None = 0x0,
    /// 2 通道 Demux
    Demux2 = 0x1,
    /// 3 通道 Demux
    Demux3 = 0x2,
    /// 4 通道 Demux
    Demux4 = 0x3,
}

impl BtDemuxChannel {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::Demux2),
            0x2 => Some(Self::Demux3),
            0x3 => Some(Self::Demux4),
            _ => None,
        }
    }

    /// 获取通道数
    pub fn channel_count(&self) -> u8 {
        match self {
            Self::None => 1,
            Self::Demux2 => 2,
            Self::Demux3 => 3,
            Self::Demux4 => 4,
        }
    }
}

// ============================================================================
// Sub-LVDS 和 HiSPi 配置
// ============================================================================

/// Sub-LVDS 位宽模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SubLvdsBitMode {
    /// 8-bit 模式
    Bit8 = 0b00,
    /// 10-bit 模式
    #[default]
    Bit10 = 0b01,
    /// 12-bit 模式
    Bit12 = 0b10,
}

impl SubLvdsBitMode {
    /// 从 u8 值转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::Bit8),
            0b01 => Some(Self::Bit10),
            0b10 => Some(Self::Bit12),
            _ => None,
        }
    }

    /// 获取实际位宽
    pub fn bits(&self) -> u8 {
        match self {
            Self::Bit8 => 8,
            Self::Bit10 => 10,
            Self::Bit12 => 12,
        }
    }
}

/// Sub-LVDS HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SubLvdsHdrPattern {
    /// Pattern 1
    #[default]
    Pattern1 = 0,
    /// Pattern 2
    Pattern2 = 1,
}

// ============================================================================
// HDR 配置
// ============================================================================

/// HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum HdrMode {
    /// 无 HDR (线性模式)
    #[default]
    None = 0,
    /// 2 帧 HDR
    TwoFrame = 1,
    /// HiSPi S-SP HDR 模式
    HispiSsp = 2,
}

// ============================================================================
// 图像配置
// ============================================================================

/// 图像尺寸
#[derive(Debug, Clone, Copy, Default)]
pub struct ImageSize {
    /// 图像宽度 (像素)
    pub width: u16,
    /// 图像高度 (行)
    pub height: u16,
}

impl ImageSize {
    /// 创建新的图像尺寸
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// 检查尺寸是否有效
    /// 
    /// 最小尺寸: 64x64
    /// 最大尺寸: 2880x1944 (5M)
    pub fn is_valid(&self) -> bool {
        self.width >= 64 && self.width <= 2880 && self.height >= 64 && self.height <= 1944
    }
}

/// 裁剪区域
#[derive(Debug, Clone, Copy, Default)]
pub struct CropRegion {
    /// 起始 X 坐标
    pub start_x: u16,
    /// 起始 Y 坐标
    pub start_y: u16,
    /// 结束 X 坐标
    pub end_x: u16,
    /// 结束 Y 坐标
    pub end_y: u16,
}

impl CropRegion {
    /// 创建新的裁剪区域
    pub const fn new(start_x: u16, start_y: u16, end_x: u16, end_y: u16) -> Self {
        Self {
            start_x,
            start_y,
            end_x,
            end_y,
        }
    }

    /// 获取裁剪后的宽度
    pub fn width(&self) -> u16 {
        if self.end_x > self.start_x {
            self.end_x - self.start_x + 1
        } else {
            0
        }
    }

    /// 获取裁剪后的高度
    pub fn height(&self) -> u16 {
        if self.end_y > self.start_y {
            self.end_y - self.start_y + 1
        } else {
            0
        }
    }
}

/// 消隐配置
#[derive(Debug, Clone, Copy, Default)]
pub struct BlankingConfig {
    /// 垂直同步后消隐 (行数)
    pub vs_back_porch: u16,
    /// 垂直同步前消隐 (行数)
    pub vs_front_porch: u16,
    /// 水平同步后消隐 (像素数)
    pub hs_back_porch: u16,
    /// 水平同步前消隐 (像素数)
    pub hs_front_porch: u16,
}

// ============================================================================
// BT 同步码配置
// ============================================================================

/// BT 同步码
/// 
/// BT.656/BT.1120 使用嵌入式同步码来标识帧和行的开始/结束
/// 同步码格式: {0xFF, 0x00, 0x00, SAV/EAV}
#[derive(Debug, Clone, Copy)]
pub struct BtSyncCode {
    /// 同步码字节 0 (通常为 0xFF)
    pub sync_0: u8,
    /// 同步码字节 1 (通常为 0x00)
    pub sync_1: u8,
    /// 同步码字节 2 (通常为 0x00)
    pub sync_2: u8,
    /// 有效行 SAV (Start of Active Video)
    pub sav_vld: u8,
    /// 消隐行 SAV
    pub sav_blk: u8,
    /// 有效行 EAV (End of Active Video)
    pub eav_vld: u8,
    /// 消隐行 EAV
    pub eav_blk: u8,
}

impl Default for BtSyncCode {
    /// 创建默认的 BT 同步码 (符合 BT.656/BT.1120 标准)
    fn default() -> Self {
        Self {
            sync_0: 0xFF,
            sync_1: 0x00,
            sync_2: 0x00,
            // 标准 BT.656 同步码:
            // SAV_VLD: 0x80 (有效行开始)
            // EAV_VLD: 0x9D (有效行结束)
            // SAV_BLK: 0xAB (消隐行开始)
            // EAV_BLK: 0xB6 (消隐行结束)
            sav_vld: 0x80,
            sav_blk: 0xAB,
            eav_vld: 0x9D,
            eav_blk: 0xB6,
        }
    }
}

// ============================================================================
// 配置结构体
// ============================================================================

/// TTL 接口配置
#[derive(Debug, Clone, Copy, Default)]
pub struct TtlConfig {
    /// 使能
    pub enable: bool,
    /// 位宽
    pub bit_width: TtlBitWidth,
    /// 输出格式
    pub fmt_out: TtlBtFmtOut,
    /// 输入格式
    pub fmt_in: TtlInputFormat,
    /// 数据序列
    pub data_seq: TtlBtDataSeq,
    /// VS 信号反相
    pub vs_inv: bool,
    /// HS 信号反相
    pub hs_inv: bool,
    /// 图像尺寸
    pub img_size: ImageSize,
    /// 消隐配置
    pub blanking: BlankingConfig,
}

/// BT 接口配置
#[derive(Debug, Clone, Copy, Default)]
pub struct BtConfig {
    /// 使能
    pub enable: bool,
    /// BT 格式
    pub format: BtFormat,
    /// Demux 通道数
    pub demux_ch: BtDemuxChannel,
    /// DDR 模式
    pub ddr_mode: bool,
    /// VS 信号反相
    pub vs_inv: bool,
    /// HS 信号反相
    pub hs_inv: bool,
    /// VS 作为 VDE 使用
    pub vs_as_vde: bool,
    /// HS 作为 HDE 使用
    pub hs_as_hde: bool,
    /// 图像尺寸
    pub img_size: ImageSize,
    /// 消隐配置
    pub blanking: BlankingConfig,
    /// 同步码配置
    pub sync_code: BtSyncCode,
}

/// Sub-LVDS 接口配置
#[derive(Debug, Clone, Copy, Default)]
pub struct SubLvdsConfig {
    /// 使能
    pub enable: bool,
    /// 位宽模式
    pub bit_mode: SubLvdsBitMode,
    /// 数据位反转
    pub data_reverse: bool,
    /// HDR 模式使能
    pub hdr_enable: bool,
    /// HDR 模式选择
    pub hdr_pattern: SubLvdsHdrPattern,
    /// 同步码第一个字
    pub sync_1st: u16,
    /// 同步码第二个字
    pub sync_2nd: u16,
    /// 同步码第三个字
    pub sync_3rd: u16,
}

/// HiSPi 接口配置
#[derive(Debug, Clone, Copy, Default)]
pub struct HispiConfig {
    /// HiSPi 模式使能 (false = Sub-LVDS, true = HiSPi)
    pub enable: bool,
    /// DE 由寄存器计数取消断言
    pub use_hsize: bool,
    /// P-SP HDR 模式使能
    pub hdr_psp_mode: bool,
    /// 普通模式 SOF 同步码
    pub norm_sof: u16,
    /// 普通模式 EOF 同步码
    pub norm_eof: u16,
}

/// HDR 配置
#[derive(Debug, Clone, Copy, Default)]
pub struct HdrConfig {
    /// HDR 手动模式使能
    pub enable: bool,
    /// VS 输出反相
    pub vs_inv: bool,
    /// HS 输出反相
    pub hs_inv: bool,
    /// DE 输出反相
    pub de_inv: bool,
    /// HDR 模式 (HiSPi S-SP HDR)
    pub mode: bool,
    /// 长曝光偏移 (第一个短曝光行之前的长曝光行数)
    pub shift: u16,
    /// HDR 垂直尺寸
    pub vsize: u16,
}

/// BLC (Black Level Calibration) 配置
#[derive(Debug, Clone, Copy, Default)]
pub struct BlcConfig {
    /// BLC0 使能
    pub blc0_enable: bool,
    /// BLC0 起始行号
    pub blc0_start: u16,
    /// BLC0 行数
    pub blc0_size: u16,
    /// BLC1 使能
    pub blc1_enable: bool,
    /// BLC1 起始行号
    pub blc1_start: u16,
    /// BLC1 行数
    pub blc1_size: u16,
}

/// VI 设备属性
/// 
/// 完整的 VI 配置参数
#[derive(Debug, Clone, Copy, Default)]
pub struct ViDevAttr {
    /// 设备编号
    pub devno: ViDevno,
    /// 传感器 MAC 模式
    pub mac_mode: SensorMacMode,
    /// VI 输入模式
    pub input_mode: ViInputMode,
    /// VI 输入来源
    pub input_source: ViInputSource,
    /// VI 时钟反相
    pub clk_inv: bool,
    /// TTL 配置 (当 mac_mode = Ttl 时使用)
    pub ttl_config: Option<TtlConfig>,
    /// BT 配置 (当使用 BT 接口时)
    pub bt_config: Option<BtConfig>,
    /// Sub-LVDS 配置 (当 mac_mode = SubLvds 时使用)
    pub sublvds_config: Option<SubLvdsConfig>,
    /// HiSPi 配置 (当使用 HiSPi 模式时)
    pub hispi_config: Option<HispiConfig>,
    /// HDR 配置
    pub hdr_config: Option<HdrConfig>,
    /// BLC 配置
    pub blc_config: Option<BlcConfig>,
    /// 裁剪区域
    pub crop_region: Option<CropRegion>,
}

// ============================================================================
// 常量定义
// ============================================================================

/// 最大 VI 设备数量
pub const MAX_VI_NUM: usize = 3;

/// 最大图像宽度
pub const MAX_IMAGE_WIDTH: u16 = 2880;

/// 最大图像高度
pub const MAX_IMAGE_HEIGHT: u16 = 1944;

/// 最小图像宽度
pub const MIN_IMAGE_WIDTH: u16 = 64;

/// 最小图像高度
pub const MIN_IMAGE_HEIGHT: u16 = 64;

/// 最大帧率 (5M HDR)
pub const MAX_FRAMERATE_5M_HDR: u8 = 60;

/// 最大帧率 (5M 线性)
pub const MAX_FRAMERATE_5M_LINEAR: u8 = 30;

/// 最大帧率 (FHD)
pub const MAX_FRAMERATE_FHD: u8 = 60;
