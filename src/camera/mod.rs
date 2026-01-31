//! # 相机传感器驱动
//!
//! 本模块提供 CMOS 图像传感器的驱动支持。
//!
//! ## 支持的传感器
//!
//! - **GC4653**: 4M (2560x1440) CMOS 图像传感器
//!   - 分辨率: 2560x1440 @ 30fps
//!   - 数据格式: RAW10
//!   - 接口: MIPI CSI-2 (2-lane)
//!   - 支持功能: 曝光控制、增益控制、帧率控制、镜像/翻转
//!
//! ## 使用示例
//!
//! ```no_run
//! use sg200x_bsp::camera::gc4653::{Gc4653, GC4653_DEFAULT_I2C_INSTANCE};
//!
//! // 创建传感器驱动实例
//! let mut sensor = unsafe { Gc4653::new(GC4653_DEFAULT_I2C_INSTANCE) };
//!
//! // 初始化传感器
//! sensor.init().expect("传感器初始化失败");
//!
//! // 设置曝光和增益
//! sensor.set_exposure_lines(1000).expect("设置曝光失败");
//! sensor.set_gain(2048, 1024).expect("设置增益失败");
//! ```
//!
//! ## 架构说明
//!
//! 传感器驱动通过 I2C 接口与传感器通信，主要包含以下功能：
//!
//! 1. **传感器探测**: 通过读取 Chip ID 验证传感器型号
//! 2. **初始化**: 写入初始化寄存器序列配置传感器工作模式
//! 3. **曝光控制**: 设置曝光行数
//! 4. **增益控制**: 设置模拟增益和数字增益
//! 5. **帧率控制**: 通过调整 VTS (垂直总行数) 实现
//! 6. **镜像/翻转**: 支持水平镜像和垂直翻转
//!
//! ## MIPI 配置
//!
//! GC4653 使用 MIPI CSI-2 接口输出图像数据，默认配置：
//! - 2 个数据 Lane
//! - RAW10 数据格式
//! - 27MHz 主时钟

pub mod gc4653;

// 重新导出常用类型
pub use gc4653::{
    AeDefault, Gc4653, Gc4653Error, Gc4653ModeInfo, LinearRegs, MirrorFlip, MipiHdrMode,
    MipiInputMode, MipiRxAttr, RawDataType, SensorState, GC4653_DEFAULT_I2C_INSTANCE,
    GC4653_HEIGHT, GC4653_I2C_ADDR_ALT, GC4653_I2C_ADDR_DEFAULT, GC4653_MAX_FPS, GC4653_MIN_FPS,
    GC4653_WIDTH,
};
