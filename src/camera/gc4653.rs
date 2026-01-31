//! GC4653 传感器驱动 (线性 2560x1440@30fps)
//!
//! GC4653 是一款 4M (2560x1440) CMOS 图像传感器，支持 MIPI 接口。
//! 本驱动实现了线性模式下的基本功能。
//!
//! ## 主要功能
//!
//! - I2C 寄存器读写 (16-bit 地址, 8-bit 数据)
//! - 传感器探测 (Chip ID 验证)
//! - 线性模式初始化 (2560x1440@30fps, 10-bit RAW)
//! - 曝光控制 (行数设置)
//! - 增益控制 (模拟增益 + 数字增益)
//! - 帧率控制 (通过 VTS 调节)
//! - 镜像/翻转设置
//! - 休眠与重启
//!
//! ## 硬件特性
//!
//! - 分辨率: 2560x1440 (4M)
//! - 帧率: 30fps (可调)
//! - 数据格式: RAW10
//! - 接口: MIPI CSI-2 (2-lane)
//! - I2C 地址: 0x10 (默认) 或 0x29
//! - I2C 速度: 400KHz
//!
//! ## 使用示例
//!
//! ```no_run
//! use sg200x_bsp::camera::gc4653::{Gc4653, GC4653_DEFAULT_I2C_INSTANCE};
//!
//! // 创建驱动实例
//! let sensor = unsafe { Gc4653::new(GC4653_DEFAULT_I2C_INSTANCE) };
//!
//! // 探测传感器
//! sensor.probe().expect("传感器探测失败");
//!
//! // 初始化为 1440p@30fps
//! sensor.init_linear_1440p30().expect("初始化失败");
//!
//! // 设置曝光 (1000 行)
//! sensor.set_exposure_lines(1000).expect("设置曝光失败");
//!
//! // 设置增益 (2x 模拟增益)
//! sensor.set_gain(2048, 1024).expect("设置增益失败");
//! ```

use crate::i2c::{I2c, I2cError, I2cInstance, I2cSpeed};

/// 默认 I2C 地址 (另一种地址为 0x29)
pub const GC4653_I2C_ADDR_DEFAULT: u8 = 0x10;
pub const GC4653_I2C_ADDR_ALT: u8 = 0x29;

/// 默认 I2C 控制器实例 (与原 C 代码一致)
pub const GC4653_DEFAULT_I2C_INSTANCE: I2cInstance = I2cInstance::I2c3;

const GC4653_CHIP_ID: u16 = 0x4653;
const GC4653_CHIP_ID_ADDR_H: u16 = 0x03f0;
const GC4653_CHIP_ID_ADDR_L: u16 = 0x03f1;
const GC4653_FULL_LINES_MAX: u16 = 0x3fff;

const REG_EXP_H: u16 = 0x0202;
const REG_EXP_L: u16 = 0x0203;
const REG_AGAIN_L: u16 = 0x02b3;
const REG_AGAIN_H: u16 = 0x02b4;
const REG_COL_AGAIN_H: u16 = 0x02b8;
const REG_COL_AGAIN_L: u16 = 0x02b9;
const REG_DGAIN_H: u16 = 0x020e;
const REG_DGAIN_L: u16 = 0x020f;
const REG_AGAIN_MAG1: u16 = 0x0515;
const REG_AGAIN_MAG2: u16 = 0x0519;
const REG_AGAIN_MAG3: u16 = 0x02d9;
const REG_VTS_H: u16 = 0x0340;
const REG_VTS_L: u16 = 0x0341;
const REG_MIRROR_FLIP: u16 = 0x0101;
const REG_FRAME_BUF: u16 = 0x031d;

// ============================================================================
// 传感器模式参数
// ============================================================================

/// 传感器 ID
pub const GC4653_SENSOR_ID: u16 = 4653;

/// 默认 VTS (垂直总行数) - 1500 行 @ 30fps
pub const GC4653_VTS_DEFAULT: u16 = 1500;

/// 默认 HTS (水平总像素) - 2200
pub const GC4653_HTS_DEFAULT: u16 = 2200;

/// 图像宽度
pub const GC4653_WIDTH: u16 = 2560;

/// 图像高度
pub const GC4653_HEIGHT: u16 = 1440;

/// 最大帧率
pub const GC4653_MAX_FPS: f32 = 30.0;

/// 最小帧率 (1500 * 30 / 16383)
pub const GC4653_MIN_FPS: f32 = 2.75;

/// 最小曝光行数
pub const GC4653_EXP_MIN: u16 = 1;

/// 最大曝光行数
pub const GC4653_EXP_MAX: u16 = 0x3fff;

/// 默认曝光值
pub const GC4653_EXP_DEFAULT: u16 = 0x2000;

/// 最小模拟增益 (1x = 1024)
pub const GC4653_AGAIN_MIN: u32 = 1024;

/// 最大模拟增益 (~76x)
pub const GC4653_AGAIN_MAX: u32 = 77648;

/// 最小数字增益 (1x = 1024)
pub const GC4653_DGAIN_MIN: u32 = 1024;

/// 最大数字增益 (~10x)
pub const GC4653_DGAIN_MAX: u32 = 10240;

// ============================================================================
// MIPI 配置参数
// ============================================================================

/// MIPI 数据类型: RAW10
pub const GC4653_MIPI_DATA_TYPE: u8 = 0x2B; // RAW10

/// MIPI Lane 数量
pub const GC4653_MIPI_LANES: u8 = 2;

/// RX MAC 时钟频率 (200MHz)
pub const GC4653_RX_MAC_CLK: u32 = 200_000_000;

/// 摄像头主时钟频率 (27MHz)
pub const GC4653_MCLK_FREQ: u32 = 27_000_000;

// ============================================================================
// MIPI RX 属性配置
// ============================================================================

/// MIPI 输入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipiInputMode {
    /// MIPI CSI-2 模式
    #[default]
    Mipi,
    /// 子 LVDS 模式
    SubLvds,
    /// HiSPi 模式
    HiSpi,
}

/// MIPI HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipiHdrMode {
    /// 无 HDR
    #[default]
    None,
    /// VC (Virtual Channel) HDR
    Vc,
    /// DT (Data Type) HDR
    Dt,
    /// DOL (Digital Overlap) HDR
    Dol,
}

/// RAW 数据位宽
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawDataType {
    /// RAW 8-bit
    Raw8,
    /// RAW 10-bit
    #[default]
    Raw10,
    /// RAW 12-bit
    Raw12,
}

impl RawDataType {
    /// 转换为 MIPI 数据类型代码
    pub fn to_mipi_dt(self) -> u8 {
        match self {
            RawDataType::Raw8 => 0x2A,
            RawDataType::Raw10 => 0x2B,
            RawDataType::Raw12 => 0x2C,
        }
    }
}

/// MIPI RX 属性配置
#[derive(Debug, Clone, Copy)]
pub struct MipiRxAttr {
    /// 输入模式
    pub input_mode: MipiInputMode,
    /// RAW 数据类型
    pub raw_data_type: RawDataType,
    /// Lane ID 配置 [lane0, lane1, lane2, lane3, clk_lane]
    /// -1 表示未使用
    pub lane_id: [i8; 5],
    /// PN 交换配置
    pub pn_swap: [bool; 5],
    /// HDR 模式
    pub hdr_mode: MipiHdrMode,
    /// 设备号
    pub devno: u8,
}

impl Default for MipiRxAttr {
    fn default() -> Self {
        Self {
            input_mode: MipiInputMode::Mipi,
            raw_data_type: RawDataType::Raw10,
            // 默认 lane 配置: lane2, lane1, lane3, 未使用, 未使用
            lane_id: [2, 1, 3, -1, -1],
            // 默认 PN 交换配置
            pn_swap: [true, true, true, false, false],
            hdr_mode: MipiHdrMode::None,
            devno: 0,
        }
    }
}

// ============================================================================
// ISP 校准参数
// ============================================================================

/// 噪声校准系数 (每个 ISO 级别的 BGGR 四通道斜率和截距)
/// 格式: [[B_slope, B_intercept], [Gb_slope, Gb_intercept], [Gr_slope, Gr_intercept], [R_slope, R_intercept]]
pub const GC4653_NOISE_CALIBRATION: [[[f32; 2]; 4]; 16] = [
    // ISO 100
    [
        [0.04006369, 5.73531675],
        [0.03064449, 8.56994438],
        [0.03068986, 8.82220364],
        [0.03361050, 7.20687675],
    ],
    // ISO 200
    [
        [0.05054308, 7.91628408],
        [0.03632997, 13.05291843],
        [0.03697144, 12.49540806],
        [0.04156460, 10.44298553],
    ],
    // ISO 400
    [
        [0.06805338, 11.55693245],
        [0.04390667, 18.59673882],
        [0.04630871, 17.90183258],
        [0.05292421, 15.17318439],
    ],
    // ISO 800
    [
        [0.08635756, 17.79124641],
        [0.05776865, 26.76541901],
        [0.05839700, 26.44194031],
        [0.06750740, 22.42554474],
    ],
    // ISO 1600
    [
        [0.12254384, 26.20610046],
        [0.07916856, 39.39273834],
        [0.07857896, 39.03499985],
        [0.09273355, 33.09597778],
    ],
    // ISO 3200
    [
        [0.18002017, 34.86975861],
        [0.10951708, 54.58878326],
        [0.10485322, 57.16654205],
        [0.13257602, 46.27093506],
    ],
    // ISO 6400
    [
        [0.24713688, 48.62341690],
        [0.14974891, 77.06428528],
        [0.14544390, 76.57913971],
        [0.19056234, 62.13500214],
    ],
    // ISO 12800
    [
        [0.37728110, 58.15543365],
        [0.20440577, 100.45700073],
        [0.20059910, 102.35488892],
        [0.27388775, 79.65499878],
    ],
    // ISO 25600
    [
        [0.36612421, 115.28938293],
        [0.22633623, 164.58416748],
        [0.21590474, 168.92042542],
        [0.33193347, 127.92090607],
    ],
    // ISO 51200
    [
        [0.48242909, 147.39015198],
        [0.28994381, 223.02711487],
        [0.29200506, 220.64030457],
        [0.42304891, 173.74638367],
    ],
    // ISO 102400
    [
        [0.62099910, 130.97862244],
        [0.39534107, 219.74490356],
        [0.39458695, 213.37374878],
        [0.55690110, 158.37773132],
    ],
    // ISO 204800
    [
        [0.75350416, 77.81707001],
        [0.52716732, 148.77879333],
        [0.51073730, 153.86495972],
        [0.68910605, 102.12422180],
    ],
    // ISO 409600
    [
        [0.90276730, 43.78258514],
        [0.62851423, 119.41429138],
        [0.64918900, 110.74241638],
        [0.80880594, 68.89983368],
    ],
    // ISO 819200 (与 409600 相同)
    [
        [0.90276730, 43.78258514],
        [0.62851423, 119.41429138],
        [0.64918900, 110.74241638],
        [0.80880594, 68.89983368],
    ],
    // ISO 1638400 (与 409600 相同)
    [
        [0.90276730, 43.78258514],
        [0.62851423, 119.41429138],
        [0.64918900, 110.74241638],
        [0.80880594, 68.89983368],
    ],
    // ISO 3276800 (与 409600 相同)
    [
        [0.90276730, 43.78258514],
        [0.62851423, 119.41429138],
        [0.64918900, 110.74241638],
        [0.80880594, 68.89983368],
    ],
];

/// 黑电平校准值 (手动模式, BGGR 四通道)
pub const GC4653_BLACK_LEVEL_MANUAL: [u16; 4] = [256, 256, 256, 256];

/// 黑电平校准值 (自动模式, 每个 ISO 级别的 BGGR 四通道)
pub const GC4653_BLACK_LEVEL_AUTO: [[u16; 4]; 16] = [
    [255, 255, 255, 255], // ISO 100
    [256, 256, 256, 256], // ISO 200
    [256, 256, 256, 256], // ISO 400
    [257, 257, 257, 257], // ISO 800
    [260, 260, 260, 260], // ISO 1600
    [264, 264, 264, 264], // ISO 3200
    [272, 272, 272, 272], // ISO 6400
    [289, 289, 289, 290], // ISO 12800
    [324, 325, 325, 325], // ISO 25600
    [400, 404, 401, 404], // ISO 51200
    [468, 469, 468, 471], // ISO 102400
    [474, 476, 477, 477], // ISO 204800
    [475, 477, 476, 478], // ISO 409600
    [476, 477, 477, 478], // ISO 819200
    [466, 467, 467, 468], // ISO 1638400
    [496, 493, 495, 493], // ISO 3276800
];

// ============================================================================
// 枚举类型定义
// ============================================================================

// ============================================================================
// 线性模式寄存器索引枚举
// ============================================================================

/// 线性模式寄存器索引 (与 C 代码 enum gc4653_linear_regs_e 对应)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LinearRegs {
    /// 曝光高字节 (0x0202 bit[13:8])
    ExpH = 0,
    /// 曝光低字节 (0x0203)
    ExpL = 1,
    /// 模拟增益低字节 (0x02b3)
    AgainL = 2,
    /// 模拟增益高字节 (0x02b4 bit[10:8])
    AgainH = 3,
    /// 列增益高字节 (0x02b8 bit[13:8])
    ColAgainH = 4,
    /// 列增益低字节 (0x02b9)
    ColAgainL = 5,
    /// 模拟增益幅度1 (0x0515)
    AgainMag1 = 6,
    /// 模拟增益幅度2 (0x0519)
    AgainMag2 = 7,
    /// 模拟增益幅度3 (0x02d9)
    AgainMag3 = 8,
    /// 数字增益高字节 (0x020e bit[9:6])
    DgainH = 9,
    /// 数字增益低字节 (0x020f bit[5:0])
    DgainL = 10,
    /// VTS 高字节 (0x0340 bit[13:8])
    VtsH = 11,
    /// VTS 低字节 (0x0341)
    VtsL = 12,
    /// 帧缓冲开启 (0x031D)
    FrameBufOn = 13,
    /// 镜像翻转 (0x0101)
    FlipMirror = 14,
    /// 帧缓冲关闭 (0x031D)
    FrameBufOff = 15,
}

impl LinearRegs {
    /// 获取寄存器地址
    pub fn addr(self) -> u16 {
        match self {
            LinearRegs::ExpH => 0x0202,
            LinearRegs::ExpL => 0x0203,
            LinearRegs::AgainL => 0x02b3,
            LinearRegs::AgainH => 0x02b4,
            LinearRegs::ColAgainH => 0x02b8,
            LinearRegs::ColAgainL => 0x02b9,
            LinearRegs::AgainMag1 => 0x0515,
            LinearRegs::AgainMag2 => 0x0519,
            LinearRegs::AgainMag3 => 0x02d9,
            LinearRegs::DgainH => 0x020e,
            LinearRegs::DgainL => 0x020f,
            LinearRegs::VtsH => 0x0340,
            LinearRegs::VtsL => 0x0341,
            LinearRegs::FrameBufOn | LinearRegs::FrameBufOff => 0x031d,
            LinearRegs::FlipMirror => 0x0101,
        }
    }
}

/// 线性模式寄存器数量
pub const LINEAR_REGS_NUM: usize = 16;

// ============================================================================
// 枚举类型定义
// ============================================================================

/// 镜像翻转模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MirrorFlip {
    /// 正常模式 (无镜像/翻转)
    #[default]
    Normal,
    /// 水平镜像
    Mirror,
    /// 垂直翻转
    Flip,
    /// 水平镜像 + 垂直翻转
    MirrorFlip,
}

impl MirrorFlip {
    /// 转换为寄存器值
    pub fn to_reg_value(self) -> u8 {
        match self {
            MirrorFlip::Normal => 0,
            MirrorFlip::Mirror => 1,
            MirrorFlip::Flip => 2,
            MirrorFlip::MirrorFlip => 3,
        }
    }

    /// 从寄存器值创建
    pub fn from_reg_value(val: u8) -> Self {
        match val & 0x03 {
            0 => MirrorFlip::Normal,
            1 => MirrorFlip::Mirror,
            2 => MirrorFlip::Flip,
            _ => MirrorFlip::MirrorFlip,
        }
    }
}

/// GC4653 驱动错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gc4653Error {
    /// I2C 通信错误
    I2c(I2cError),
    /// Chip ID 不匹配
    InvalidChipId { expected: u16, found: u16 },
    /// 参数超出范围
    OutOfRange,
    /// 不支持的模式
    UnsupportedMode,
}

impl From<I2cError> for Gc4653Error {
    fn from(err: I2cError) -> Self {
        Self::I2c(err)
    }
}

/// 传感器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SensorState {
    /// 未初始化
    #[default]
    Uninitialized,
    /// 待机模式
    Standby,
    /// 流式传输中
    Streaming,
}

/// GC4653 传感器模式信息
#[derive(Debug, Clone, Copy)]
pub struct Gc4653ModeInfo {
    /// 图像宽度
    pub width: u16,
    /// 图像高度
    pub height: u16,
    /// 最大帧率
    pub max_fps: f32,
    /// 最小帧率
    pub min_fps: f32,
    /// 默认 VTS
    pub vts_default: u16,
    /// 默认 HTS
    pub hts_default: u16,
}

impl Default for Gc4653ModeInfo {
    fn default() -> Self {
        Self {
            width: GC4653_WIDTH,
            height: GC4653_HEIGHT,
            max_fps: GC4653_MAX_FPS,
            min_fps: GC4653_MIN_FPS,
            vts_default: GC4653_VTS_DEFAULT,
            hts_default: GC4653_HTS_DEFAULT,
        }
    }
}

/// GC4653 驱动结构体
pub struct Gc4653 {
    i2c: I2c,
    i2c_addr: u8,
    state: SensorState,
    current_vts: u16,
    mirror_flip: MirrorFlip,
}

impl Gc4653 {
    /// 创建新的 GC4653 驱动实例 (使用默认 I2C 地址)
    ///
    /// # Safety
    /// 调用者必须确保 I2C 寄存器可访问且实例唯一。
    pub unsafe fn new(instance: I2cInstance) -> Self {
        unsafe { Self::new_with_addr(instance, GC4653_I2C_ADDR_DEFAULT) }
    }

    /// 创建新的 GC4653 驱动实例 (指定 I2C 地址)
    ///
    /// # Safety
    /// 调用者必须确保 I2C 寄存器可访问且实例唯一。
    pub unsafe fn new_with_addr(instance: I2cInstance, i2c_addr: u8) -> Self {
        let mut i2c = unsafe { I2c::new(instance) };
        i2c.init(I2cSpeed::Fast);
        Self {
            i2c,
            i2c_addr,
            state: SensorState::Uninitialized,
            current_vts: GC4653_VTS_DEFAULT,
            mirror_flip: MirrorFlip::Normal,
        }
    }

    /// 基于已创建的 I2C 驱动构造
    pub fn from_i2c(i2c: I2c, i2c_addr: u8) -> Self {
        Self {
            i2c,
            i2c_addr,
            state: SensorState::Uninitialized,
            current_vts: GC4653_VTS_DEFAULT,
            mirror_flip: MirrorFlip::Normal,
        }
    }

    /// 获取当前 I2C 地址
    pub fn i2c_addr(&self) -> u8 {
        self.i2c_addr
    }

    /// 设置 I2C 地址
    pub fn set_i2c_addr(&mut self, addr: u8) {
        self.i2c_addr = addr;
    }

    /// 获取当前传感器状态
    pub fn state(&self) -> SensorState {
        self.state
    }

    /// 获取当前模式信息
    pub fn mode_info(&self) -> Gc4653ModeInfo {
        Gc4653ModeInfo::default()
    }

    /// 获取当前 VTS 值
    pub fn current_vts(&self) -> u16 {
        self.current_vts
    }

    /// 获取当前镜像/翻转设置
    pub fn current_mirror_flip(&self) -> MirrorFlip {
        self.mirror_flip
    }

    /// 写寄存器 (16-bit 地址, 8-bit 数据)
    pub fn write_reg(&self, reg: u16, data: u8) -> Result<(), Gc4653Error> {
        let buf = [(reg >> 8) as u8, (reg & 0xff) as u8, data];
        self.i2c.write(self.i2c_addr, &buf)?;
        Ok(())
    }

    /// 读寄存器 (16-bit 地址, 8-bit 数据)
    pub fn read_reg(&self, reg: u16) -> Result<u8, Gc4653Error> {
        let addr = [(reg >> 8) as u8, (reg & 0xff) as u8];
        let mut out = [0u8];
        self.i2c.write_read(self.i2c_addr, &addr, &mut out)?;
        Ok(out[0])
    }

    /// 传感器探测 (读取 Chip ID)
    pub fn probe(&self) -> Result<(), Gc4653Error> {
        let high = self.read_reg(GC4653_CHIP_ID_ADDR_H)? as u16;
        let low = self.read_reg(GC4653_CHIP_ID_ADDR_L)? as u16;
        let found = (high << 8) | low;
        if found != GC4653_CHIP_ID {
            return Err(Gc4653Error::InvalidChipId {
                expected: GC4653_CHIP_ID,
                found,
            });
        }
        Ok(())
    }

    /// 进入待机
    pub fn standby(&mut self) -> Result<(), Gc4653Error> {
        self.write_reg(0x0100, 0x00)?;
        self.write_reg(0x031c, 0xc7)?;
        self.write_reg(0x0317, 0x01)?;
        self.state = SensorState::Standby;
        Ok(())
    }

    /// 退出待机并重启
    pub fn restart(&mut self) -> Result<(), Gc4653Error> {
        self.write_reg(0x0317, 0x00)?;
        self.write_reg(0x031c, 0xc6)?;
        self.write_reg(0x0100, 0x09)?;
        self.state = SensorState::Streaming;
        Ok(())
    }

    /// 开始流式传输
    pub fn stream_on(&mut self) -> Result<(), Gc4653Error> {
        self.write_reg(0x0100, 0x09)?;
        self.state = SensorState::Streaming;
        Ok(())
    }

    /// 停止流式传输
    pub fn stream_off(&mut self) -> Result<(), Gc4653Error> {
        self.write_reg(0x0100, 0x00)?;
        self.state = SensorState::Standby;
        Ok(())
    }

    /// 初始化为 2560x1440@30fps 线性模式
    pub fn init_linear_1440p30(&mut self) -> Result<(), Gc4653Error> {
        for (reg, val) in GC4653_LINEAR_1440P30_INIT {
            self.write_reg(*reg, *val)?;
        }
        self.state = SensorState::Streaming;
        self.current_vts = GC4653_VTS_DEFAULT;
        Ok(())
    }

    /// 完整初始化流程 (探测 + 初始化)
    pub fn init(&mut self) -> Result<(), Gc4653Error> {
        self.probe()?;
        self.init_linear_1440p30()
    }

    /// 设置曝光 (单位: 行)
    pub fn set_exposure_lines(&self, lines: u16) -> Result<(), Gc4653Error> {
        let lines = lines.min(GC4653_FULL_LINES_MAX);
        let high = ((lines >> 8) & 0x3f) as u8;
        let low = (lines & 0xff) as u8;
        self.write_reg(REG_EXP_H, high)?;
        self.write_reg(REG_EXP_L, low)?;
        Ok(())
    }

    /// 设置帧长 VTS (用于调节帧率/曝光上限)
    pub fn set_vts(&mut self, vts: u16) -> Result<(), Gc4653Error> {
        let vts = vts.min(GC4653_FULL_LINES_MAX);
        self.write_reg(REG_VTS_H, ((vts >> 8) & 0x3f) as u8)?;
        self.write_reg(REG_VTS_L, (vts & 0xff) as u8)?;
        self.current_vts = vts;
        Ok(())
    }

    /// 设置帧率 (通过调整 VTS 实现)
    ///
    /// 帧率范围: 2.75fps ~ 30fps
    pub fn set_fps(&mut self, fps: f32) -> Result<(), Gc4653Error> {
        if fps < GC4653_MIN_FPS || fps > GC4653_MAX_FPS {
            return Err(Gc4653Error::OutOfRange);
        }
        let vts = ((GC4653_VTS_DEFAULT as f32) * GC4653_MAX_FPS / fps) as u16;
        self.set_vts(vts)
    }

    /// 获取当前帧率
    pub fn get_fps(&self) -> f32 {
        (GC4653_VTS_DEFAULT as f32) * GC4653_MAX_FPS / (self.current_vts as f32)
    }

    /// 获取最大曝光行数 (VTS - 8)
    pub fn get_max_exposure(&self) -> u16 {
        self.current_vts.saturating_sub(8)
    }

    /// 设置模拟增益与数字增益
    ///
    /// `again` 与 `dgain` 使用 1024 为 1x 的线性值。
    pub fn set_gain(&self, again: u32, dgain: u32) -> Result<(), Gc4653Error> {
        let mut index = GAIN_TABLE.len() - 1;
        let u32_dgain;

        if again < GAIN_TABLE[GAIN_TABLE.len() - 1] {
            for i in 1..GAIN_TABLE.len() {
                if again < GAIN_TABLE[i] {
                    index = i - 1;
                    break;
                }
            }
            u32_dgain = again * 64 / GAIN_TABLE[index];
        } else {
            index = GAIN_TABLE.len() - 1;
            u32_dgain = dgain * 64 / 1024;
        }

        let reg_vals = REG_VAL_TABLE[index];
        self.write_reg(REG_AGAIN_L, reg_vals[0])?;
        self.write_reg(REG_AGAIN_H, reg_vals[1])?;
        self.write_reg(REG_COL_AGAIN_H, reg_vals[2])?;
        self.write_reg(REG_COL_AGAIN_L, reg_vals[3])?;
        self.write_reg(REG_AGAIN_MAG1, reg_vals[4])?;
        self.write_reg(REG_AGAIN_MAG2, reg_vals[5])?;
        self.write_reg(REG_AGAIN_MAG3, reg_vals[6])?;
        self.write_reg(REG_DGAIN_H, (u32_dgain >> 6) as u8)?;
        self.write_reg(REG_DGAIN_L, ((u32_dgain & 0x3f) << 2) as u8)?;
        Ok(())
    }

    /// 设置镜像/翻转
    pub fn set_mirror_flip(&mut self, mode: MirrorFlip) -> Result<(), Gc4653Error> {
        let value = mode.to_reg_value();
        // 按原实现切换帧缓冲
        self.write_reg(REG_FRAME_BUF, 0x2d)?;
        self.write_reg(REG_MIRROR_FLIP, value)?;
        self.write_reg(REG_FRAME_BUF, 0x28)?;
        self.mirror_flip = mode;
        Ok(())
    }

    /// 读取当前镜像/翻转设置
    pub fn read_mirror_flip(&self) -> Result<MirrorFlip, Gc4653Error> {
        let val = self.read_reg(REG_MIRROR_FLIP)?;
        Ok(MirrorFlip::from_reg_value(val))
    }

    /// 计算模拟增益对应的增益表索引
    ///
    /// 返回 (索引, 预增益)
    pub fn calc_again_index(again: u32) -> (usize, u32) {
        let total = GAIN_TABLE.len();
        if again >= GAIN_TABLE[total - 1] {
            return (total - 1, 64);
        }
        for i in 1..total {
            if again < GAIN_TABLE[i] {
                let pregain = again * 64 / GAIN_TABLE[i - 1];
                return (i - 1, pregain);
            }
        }
        (total - 1, 64)
    }

    /// 获取增益表中指定索引的增益值
    pub fn get_gain_from_table(index: usize) -> Option<u32> {
        GAIN_TABLE.get(index).copied()
    }

    /// 获取增益表长度
    pub fn gain_table_len() -> usize {
        GAIN_TABLE.len()
    }

    /// 获取 AE 默认参数
    pub fn get_ae_default(&self) -> AeDefault {
        AeDefault {
            full_lines_std: self.current_vts,
            full_lines_max: GC4653_FULL_LINES_MAX,
            flicker_freq: 50 * 256, // 50Hz * 256
            max_again: GC4653_AGAIN_MAX,
            min_again: GC4653_AGAIN_MIN,
            max_dgain: GC4653_DGAIN_MAX,
            min_dgain: GC4653_DGAIN_MIN,
            max_int_time: self.current_vts.saturating_sub(8) as u32,
            min_int_time: GC4653_EXP_MIN as u32,
            init_exposure: GC4653_EXP_DEFAULT as u32,
            fps: GC4653_MAX_FPS,
            min_fps: GC4653_MIN_FPS,
        }
    }

    /// 批量写入寄存器
    pub fn write_regs(&self, regs: &[(u16, u8)]) -> Result<(), Gc4653Error> {
        for (reg, val) in regs {
            self.write_reg(*reg, *val)?;
        }
        Ok(())
    }

    /// 设置曝光和增益 (一次性更新，减少 I2C 通信次数)
    ///
    /// `exposure`: 曝光行数
    /// `again`: 模拟增益 (1024 = 1x)
    /// `dgain`: 数字增益 (1024 = 1x)
    pub fn set_exposure_gain(
        &self,
        exposure: u16,
        again: u32,
        dgain: u32,
    ) -> Result<(), Gc4653Error> {
        // 设置曝光
        let exp = exposure.min(GC4653_FULL_LINES_MAX);
        self.write_reg(REG_EXP_H, ((exp >> 8) & 0x3f) as u8)?;
        self.write_reg(REG_EXP_L, (exp & 0xff) as u8)?;

        // 设置增益
        self.set_gain(again, dgain)?;

        Ok(())
    }

    /// 软复位传感器
    pub fn soft_reset(&self) -> Result<(), Gc4653Error> {
        self.write_reg(0x03fe, 0xf0)?;
        // 延时应由调用者处理
        self.write_reg(0x03fe, 0x00)?;
        Ok(())
    }

    /// 读取当前曝光值 (行数)
    pub fn read_exposure(&self) -> Result<u16, Gc4653Error> {
        let high = self.read_reg(REG_EXP_H)? as u16;
        let low = self.read_reg(REG_EXP_L)? as u16;
        Ok(((high & 0x3f) << 8) | low)
    }

    /// 读取当前 VTS 值
    pub fn read_vts(&self) -> Result<u16, Gc4653Error> {
        let high = self.read_reg(REG_VTS_H)? as u16;
        let low = self.read_reg(REG_VTS_L)? as u16;
        Ok(((high & 0x3f) << 8) | low)
    }
}

// ============================================================================
// AE (自动曝光) 相关结构体
// ============================================================================

/// AE 默认参数
#[derive(Debug, Clone, Copy)]
pub struct AeDefault {
    /// 标准帧行数
    pub full_lines_std: u16,
    /// 最大帧行数
    pub full_lines_max: u16,
    /// 闪烁频率 (Hz * 256)
    pub flicker_freq: u32,
    /// 最大模拟增益
    pub max_again: u32,
    /// 最小模拟增益
    pub min_again: u32,
    /// 最大数字增益
    pub max_dgain: u32,
    /// 最小数字增益
    pub min_dgain: u32,
    /// 最大曝光时间 (行)
    pub max_int_time: u32,
    /// 最小曝光时间 (行)
    pub min_int_time: u32,
    /// 初始曝光值
    pub init_exposure: u32,
    /// 帧率
    pub fps: f32,
    /// 最小帧率
    pub min_fps: f32,
}

impl Default for AeDefault {
    fn default() -> Self {
        Self {
            full_lines_std: GC4653_VTS_DEFAULT,
            full_lines_max: GC4653_FULL_LINES_MAX,
            flicker_freq: 50 * 256,
            max_again: GC4653_AGAIN_MAX,
            min_again: GC4653_AGAIN_MIN,
            max_dgain: GC4653_DGAIN_MAX,
            min_dgain: GC4653_DGAIN_MIN,
            max_int_time: (GC4653_VTS_DEFAULT - 8) as u32,
            min_int_time: GC4653_EXP_MIN as u32,
            init_exposure: GC4653_EXP_DEFAULT as u32,
            fps: GC4653_MAX_FPS,
            min_fps: GC4653_MIN_FPS,
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 验证 I2C 地址是否有效
pub fn is_valid_i2c_addr(addr: u8) -> bool {
    addr == GC4653_I2C_ADDR_DEFAULT || addr == GC4653_I2C_ADDR_ALT
}

/// 获取默认的 MIPI RX 属性配置
pub fn get_default_mipi_rx_attr() -> MipiRxAttr {
    MipiRxAttr::default()
}

/// 根据 ISO 值获取噪声校准系数索引
///
/// 返回最接近的 ISO 级别索引 (0-15)
pub fn get_noise_calibration_index(iso: u32) -> usize {
    let iso_levels: [u32; 16] = [
        100, 200, 400, 800, 1600, 3200, 6400, 12800, 25600, 51200, 102400, 204800, 409600, 819200,
        1638400, 3276800,
    ];

    for (i, &level) in iso_levels.iter().enumerate() {
        if iso <= level {
            return i;
        }
    }
    15 // 最大索引
}

/// 获取指定 ISO 级别的噪声校准系数
pub fn get_noise_calibration(iso: u32) -> [[f32; 2]; 4] {
    let index = get_noise_calibration_index(iso);
    GC4653_NOISE_CALIBRATION[index]
}

/// 获取指定 ISO 级别的黑电平值
pub fn get_black_level(iso: u32) -> [u16; 4] {
    let index = get_noise_calibration_index(iso);
    GC4653_BLACK_LEVEL_AUTO[index]
}

const REG_VAL_TABLE: [[u8; 7]; 26] = [
    [0x00, 0x00, 0x01, 0x00, 0x30, 0x1e, 0x5c],
    [0x20, 0x00, 0x01, 0x0b, 0x30, 0x1e, 0x5c],
    [0x01, 0x00, 0x01, 0x19, 0x30, 0x1d, 0x5b],
    [0x21, 0x00, 0x01, 0x2a, 0x30, 0x1e, 0x5c],
    [0x02, 0x00, 0x02, 0x00, 0x30, 0x1e, 0x5c],
    [0x22, 0x00, 0x02, 0x17, 0x30, 0x1d, 0x5b],
    [0x03, 0x00, 0x02, 0x33, 0x20, 0x16, 0x54],
    [0x23, 0x00, 0x03, 0x14, 0x20, 0x17, 0x55],
    [0x04, 0x00, 0x04, 0x00, 0x20, 0x17, 0x55],
    [0x24, 0x00, 0x04, 0x2f, 0x20, 0x19, 0x57],
    [0x05, 0x00, 0x05, 0x26, 0x20, 0x19, 0x57],
    [0x25, 0x00, 0x06, 0x28, 0x20, 0x1b, 0x59],
    [0x0c, 0x00, 0x08, 0x00, 0x20, 0x1d, 0x5b],
    [0x2c, 0x00, 0x09, 0x1e, 0x20, 0x1f, 0x5d],
    [0x0d, 0x00, 0x0b, 0x0c, 0x20, 0x21, 0x5f],
    [0x2d, 0x00, 0x0d, 0x11, 0x20, 0x24, 0x62],
    [0x1c, 0x00, 0x10, 0x00, 0x20, 0x26, 0x64],
    [0x3c, 0x00, 0x12, 0x3d, 0x18, 0x2a, 0x68],
    [0x5c, 0x00, 0x16, 0x19, 0x18, 0x2c, 0x6a],
    [0x7c, 0x00, 0x1a, 0x22, 0x18, 0x2e, 0x6c],
    [0x9c, 0x00, 0x20, 0x00, 0x18, 0x32, 0x70],
    [0xbc, 0x00, 0x25, 0x3a, 0x18, 0x35, 0x73],
    [0xdc, 0x00, 0x2c, 0x33, 0x10, 0x36, 0x74],
    [0xfc, 0x00, 0x35, 0x05, 0x10, 0x38, 0x76],
    [0x1c, 0x01, 0x40, 0x00, 0x10, 0x3c, 0x7a],
    [0x3c, 0x01, 0x4b, 0x35, 0x10, 0x42, 0x80],
];

const GAIN_TABLE: [u32; 26] = [
    1024, 1200, 1424, 1696, 2048, 2416, 2864, 3392, 4096, 4848, 5728, 6784, 8192, 9696,
    11456, 13584, 16384, 19408, 22928, 27168, 32768, 38816, 45872, 54352, 65536, 77648,
];

const GC4653_LINEAR_1440P30_INIT: &[(u16, u8)] = &[
    (0x03fe, 0xf0),
    (0x03fe, 0x00),
    (0x0317, 0x00),
    (0x0320, 0x77),
    (0x0324, 0xc8),
    (0x0325, 0x06),
    (0x0326, 0x60),
    (0x0327, 0x03),
    (0x0334, 0x40),
    (0x0336, 0x60),
    (0x0337, 0x82),
    (0x0315, 0x25),
    (0x031c, 0xc6),
    (0x0287, 0x18),
    (0x0084, 0x00),
    (0x0087, 0x50),
    (0x029d, 0x08),
    (0x0290, 0x00),
    (0x0340, 0x05),
    (0x0341, 0xdc),
    (0x0345, 0x06),
    (0x034b, 0xb0),
    (0x0352, 0x08),
    (0x0354, 0x08),
    (0x02d1, 0xe0),
    (0x0223, 0xf2),
    (0x0238, 0xa4),
    (0x02ce, 0x7f),
    (0x0232, 0xc4),
    (0x02d3, 0x05),
    (0x0243, 0x06),
    (0x02ee, 0x30),
    (0x026f, 0x70),
    (0x0257, 0x09),
    (0x0211, 0x02),
    (0x0219, 0x09),
    (0x023f, 0x2d),
    (0x0518, 0x00),
    (0x0519, 0x01),
    (0x0515, 0x08),
    (0x02d9, 0x3f),
    (0x02da, 0x02),
    (0x02db, 0xe8),
    (0x02e6, 0x20),
    (0x021b, 0x10),
    (0x0252, 0x22),
    (0x024e, 0x22),
    (0x02c4, 0x01),
    (0x021d, 0x17),
    (0x024a, 0x01),
    (0x02ca, 0x02),
    (0x0262, 0x10),
    (0x029a, 0x20),
    (0x021c, 0x0e),
    (0x0298, 0x03),
    (0x029c, 0x00),
    (0x027e, 0x14),
    (0x02c2, 0x10),
    (0x0540, 0x20),
    (0x0546, 0x01),
    (0x0548, 0x01),
    (0x0544, 0x01),
    (0x0242, 0x1b),
    (0x02c0, 0x1b),
    (0x02c3, 0x20),
    (0x02e4, 0x10),
    (0x022e, 0x00),
    (0x027b, 0x3f),
    (0x0269, 0x0f),
    (0x02d2, 0x40),
    (0x027c, 0x08),
    (0x023a, 0x2e),
    (0x0245, 0xce),
    (0x0530, 0x20),
    (0x0531, 0x02),
    (0x0228, 0x50),
    (0x02ab, 0x00),
    (0x0250, 0x00),
    (0x0221, 0x50),
    (0x02ac, 0x00),
    (0x02a5, 0x02),
    (0x0260, 0x0b),
    (0x0216, 0x04),
    (0x0299, 0x1c),
    (0x02bb, 0x0d),
    (0x02a3, 0x02),
    (0x02a4, 0x02),
    (0x021e, 0x02),
    (0x024f, 0x08),
    (0x028c, 0x08),
    (0x0532, 0x3f),
    (0x0533, 0x02),
    (0x0277, 0xc0),
    (0x0276, 0xc0),
    (0x0239, 0xc0),
    (0x0202, 0x05),
    (0x0203, 0x46),
    (0x0205, 0xc0),
    (0x02b0, 0x68),
    (0x0002, 0xa9),
    (0x0004, 0x01),
    (0x021a, 0x98),
    (0x0266, 0xa0),
    (0x0020, 0x01),
    (0x0021, 0x03),
    (0x0022, 0x00),
    (0x0023, 0x04),
    (0x0342, 0x06),
    (0x0343, 0x40),
    (0x03fe, 0x10),
    (0x03fe, 0x00),
    (0x0106, 0x78),
    (0x0108, 0x0c),
    (0x0114, 0x01),
    (0x0115, 0x12),
    (0x0180, 0x46),
    (0x0181, 0x30),
    (0x0182, 0x05),
    (0x0185, 0x01),
    (0x03fe, 0x10),
    (0x03fe, 0x00),
    (0x0100, 0x09),
    (0x0277, 0x38),
    (0x0276, 0xc0),
    (0x000f, 0x10),
    (0x0059, 0x00),
    (0x0080, 0x02),
    (0x0097, 0x0a),
    (0x0098, 0x10),
    (0x0099, 0x05),
    (0x009a, 0xb0),
    (0x0317, 0x08),
    (0x0a67, 0x80),
    (0x0a70, 0x03),
    (0x0a82, 0x00),
    (0x0a83, 0x10),
    (0x0a80, 0x2b),
    (0x05be, 0x00),
    (0x05a9, 0x01),
    (0x0313, 0x80),
    (0x05be, 0x01),
    (0x0317, 0x00),
    (0x0a67, 0x00),
];
