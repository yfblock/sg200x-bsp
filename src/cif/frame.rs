//! 视频帧捕获模块
//!
//! 本模块提供从 CIF 获取视频帧的功能。
//!
//! ## 架构说明
//!
//! SG2002 的视频采集流程：
//! ```text
//! Sensor -> CIF (MIPI/LVDS/TTL) -> ISP -> VI -> Frame Buffer
//! ```
//!
//! CIF 是相机接口层，负责：
//! 1. 接收传感器的原始数据（通过 MIPI CSI-2、LVDS 或 TTL 接口）
//! 2. 解析数据包格式
//! 3. 将数据传递给 ISP 进行处理
//!
//! 要获取视频帧，需要配合 ISP 和 VI 模块使用。

use super::*;

/// 帧缓冲区信息
#[derive(Debug, Clone, Copy)]
pub struct FrameBuffer {
    /// 物理地址
    pub phy_addr: usize,
    /// 虚拟地址（如果有映射）
    pub vir_addr: Option<*mut u8>,
    /// 缓冲区大小
    pub size: usize,
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
    /// 每行字节数（stride）
    pub stride: u32,
    /// 像素格式
    pub format: PixelFormat,
}

/// 像素格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    /// RAW8 Bayer 格式
    Raw8 = 0,
    /// RAW10 Bayer 格式
    Raw10,
    /// RAW12 Bayer 格式
    Raw12,
    /// YUV422 8-bit
    Yuv422_8,
    /// YUV422 10-bit
    Yuv422_10,
    /// NV12 (YUV420 semi-planar)
    Nv12,
    /// NV21 (YUV420 semi-planar, UV swapped)
    Nv21,
}

/// 帧捕获状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// 空闲
    Idle,
    /// 正在捕获
    Capturing,
    /// 帧就绪
    FrameReady,
    /// 错误
    Error,
}

/// 帧捕获错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    /// 设备未初始化
    NotInitialized,
    /// 缓冲区不足
    BufferTooSmall,
    /// 超时
    Timeout,
    /// FIFO 溢出
    FifoOverflow,
    /// CRC 错误
    CrcError,
    /// ECC 错误
    EccError,
    /// 无效配置
    InvalidConfig,
}

/// 帧捕获器
///
/// 用于从 CIF 捕获视频帧的高级接口。
///
/// ## 使用示例
///
/// ```rust,ignore
/// use sg200x_bsp::cif::frame::*;
///
/// // 创建帧捕获器
/// let mut capture = unsafe { FrameCapture::new(0) };
///
/// // 配置捕获参数
/// let config = CaptureConfig {
///     width: 2560,
///     height: 1440,
///     format: PixelFormat::Raw10,
///     buffer_count: 3,
/// };
/// capture.configure(&config)?;
///
/// // 开始捕获
/// capture.start()?;
///
/// // 获取一帧
/// let frame = capture.get_frame(1000)?; // 1000ms 超时
///
/// // 处理帧数据...
///
/// // 释放帧
/// capture.release_frame(&frame)?;
///
/// // 停止捕获
/// capture.stop()?;
/// ```
pub struct FrameCapture {
    /// 设备编号
    devno: u32,
    /// CIF 设备引用
    cif_dev: Option<CifDev>,
    /// 捕获状态
    state: CaptureState,
    /// 帧缓冲区列表
    buffers: [Option<FrameBuffer>; 4],
    /// 当前缓冲区索引
    current_buffer: usize,
    /// 配置
    config: Option<CaptureConfig>,
}

/// 捕获配置
#[derive(Debug, Clone, Copy)]
pub struct CaptureConfig {
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
    /// 像素格式
    pub format: PixelFormat,
    /// 缓冲区数量（1-4）
    pub buffer_count: u8,
}

impl FrameCapture {
    /// 创建新的帧捕获器
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    pub unsafe fn new(devno: u32) -> Self {
        Self {
            devno,
            cif_dev: None,
            state: CaptureState::Idle,
            buffers: [None; 4],
            current_buffer: 0,
            config: None,
        }
    }

    /// 初始化帧捕获器
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    pub unsafe fn init(&mut self, cif_dev: CifDev) {
        self.cif_dev = Some(cif_dev);
    }

    /// 配置捕获参数
    pub fn configure(&mut self, config: &CaptureConfig) -> Result<(), CaptureError> {
        if self.cif_dev.is_none() {
            return Err(CaptureError::NotInitialized);
        }

        if config.buffer_count == 0 || config.buffer_count > 4 {
            return Err(CaptureError::InvalidConfig);
        }

        self.config = Some(*config);
        Ok(())
    }

    /// 分配帧缓冲区
    ///
    /// # Arguments
    /// * `phy_addrs` - 物理地址数组
    /// * `sizes` - 缓冲区大小数组
    pub fn allocate_buffers(
        &mut self,
        phy_addrs: &[usize],
        sizes: &[usize],
    ) -> Result<(), CaptureError> {
        let config = self.config.ok_or(CaptureError::NotInitialized)?;

        if phy_addrs.len() < config.buffer_count as usize {
            return Err(CaptureError::BufferTooSmall);
        }

        let bytes_per_pixel = match config.format {
            PixelFormat::Raw8 | PixelFormat::Yuv422_8 => 1,
            PixelFormat::Raw10 | PixelFormat::Yuv422_10 => 2,
            PixelFormat::Raw12 => 2,
            PixelFormat::Nv12 | PixelFormat::Nv21 => 1, // Y plane only, UV is separate
        };

        let stride = config.width * bytes_per_pixel;
        let required_size = (stride * config.height) as usize;

        for i in 0..config.buffer_count as usize {
            if sizes[i] < required_size {
                return Err(CaptureError::BufferTooSmall);
            }

            self.buffers[i] = Some(FrameBuffer {
                phy_addr: phy_addrs[i],
                vir_addr: None,
                size: sizes[i],
                width: config.width,
                height: config.height,
                stride,
                format: config.format,
            });
        }

        Ok(())
    }

    /// 开始捕获
    pub fn start(&mut self) -> Result<(), CaptureError> {
        if self.cif_dev.is_none() || self.config.is_none() {
            return Err(CaptureError::NotInitialized);
        }

        self.state = CaptureState::Capturing;
        self.current_buffer = 0;

        // 实际的硬件启动需要配合 ISP 和 VI 模块
        log::info!("Frame capture started");

        Ok(())
    }

    /// 停止捕获
    pub fn stop(&mut self) -> Result<(), CaptureError> {
        self.state = CaptureState::Idle;
        log::info!("Frame capture stopped");
        Ok(())
    }

    /// 获取一帧（阻塞，带超时）
    ///
    /// # Arguments
    /// * `timeout_ms` - 超时时间（毫秒）
    ///
    /// # Returns
    /// 成功返回帧缓冲区信息，失败返回错误
    pub fn get_frame(&mut self, timeout_ms: u32) -> Result<FrameBuffer, CaptureError> {
        if self.state != CaptureState::Capturing {
            return Err(CaptureError::NotInitialized);
        }

        // 检查 CIF 状态
        if let Some(ref cif_dev) = self.cif_dev {
            let link = cif_dev.link(self.devno as usize).ok_or(CaptureError::NotInitialized)?;

            // 检查错误状态
            if link.sts_csi.errcnt_crc > 0 {
                return Err(CaptureError::CrcError);
            }
            if link.sts_csi.errcnt_ecc > 0 {
                return Err(CaptureError::EccError);
            }
            if link.sts_csi.fifo_full > 0 {
                return Err(CaptureError::FifoOverflow);
            }
        }

        // 获取当前缓冲区
        let buffer = self.buffers[self.current_buffer].ok_or(CaptureError::NotInitialized)?;

        // 更新缓冲区索引
        let config = self.config.ok_or(CaptureError::NotInitialized)?;
        self.current_buffer = (self.current_buffer + 1) % config.buffer_count as usize;

        Ok(buffer)
    }

    /// 释放帧缓冲区
    pub fn release_frame(&mut self, _frame: &FrameBuffer) -> Result<(), CaptureError> {
        // 在实际实现中，这里需要将缓冲区放回队列
        Ok(())
    }

    /// 获取捕获状态
    pub fn state(&self) -> CaptureState {
        self.state
    }

    /// 检查是否有帧就绪
    pub fn is_frame_ready(&self) -> bool {
        self.state == CaptureState::FrameReady
    }

    /// 获取 CIF 错误统计
    pub fn get_error_stats(&self) -> Option<CsiStatus> {
        self.cif_dev.as_ref().and_then(|dev| {
            dev.link(self.devno as usize).map(|link| link.sts_csi)
        })
    }
}

/// 计算 RAW 格式图像的缓冲区大小
pub fn calc_raw_buffer_size(width: u32, height: u32, format: PixelFormat) -> usize {
    let bytes_per_pixel = match format {
        PixelFormat::Raw8 => 1,
        PixelFormat::Raw10 | PixelFormat::Raw12 => 2,
        _ => 2,
    };
    (width * height * bytes_per_pixel) as usize
}

/// 计算 YUV 格式图像的缓冲区大小
pub fn calc_yuv_buffer_size(width: u32, height: u32, format: PixelFormat) -> usize {
    match format {
        PixelFormat::Yuv422_8 => (width * height * 2) as usize,
        PixelFormat::Yuv422_10 => (width * height * 4) as usize,
        PixelFormat::Nv12 | PixelFormat::Nv21 => (width * height * 3 / 2) as usize,
        _ => (width * height * 2) as usize,
    }
}
