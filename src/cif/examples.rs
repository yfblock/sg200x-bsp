//! CIF 使用示例
//!
//! 本模块提供 CIF 驱动的使用示例，包括：
//! - MIPI CSI-2 配置
//! - 视频帧捕获
//! - 错误处理

#![allow(dead_code)]

use super::*;
use super::frame::*;
use super::drv::*;
use super::regs::CIF_INT_STS_CRC_ERR_MASK;

/// 创建 MIPI CSI-2 配置示例（用于 GC4653）
pub fn create_mipi_attr_for_gc4653() -> ComboDevAttr {
    let mut attr = ComboDevAttr::default();
    
    attr.input_mode = InputMode::Mipi;
    attr.devno = 0;
    attr.mac_clk = RxMacClk::Clk400M;
    
    attr.img_size = ImgSize {
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };
    
    attr.mclk = MclkPll {
        cam: 0,
        freq: CamPllFreq::Freq27M,
    };
    
    // MIPI 特定配置
    attr.mipi_attr = Some(MipiDevAttr {
        raw_data_type: RawDataType::Raw10Bit,
        lane_id: [2, 1, 3, -1, -1], // CLK=2, D0=1, D1=3
        hdr_mode: MipiHdrMode::None,
        data_type: [0x2B, 0], // RAW10
        pn_swap: [1, 1, 1, 0, 0],
        dphy: Dphy {
            enable: false,
            hs_settle: 0,
        },
        demux: MipiDemuxInfo {
            demux_en: false,
            vc_mapping: [0, 1, 2, 3],
        },
    });
    
    attr
}

/// 初始化 CIF 并配置 MIPI 的完整流程
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn init_cif_for_mipi() -> Result<CifDev, CifError> {
    // 1. 创建 CIF 设备
    let mut cif_dev = unsafe { CifDev::new(0) };
    
    // 2. 初始化设备
    unsafe { cif_dev.init() };
    
    // 3. 创建配置
    let attr = create_mipi_attr_for_gc4653();
    
    // 4. 应用配置
    cif_dev.set_dev_attr(&attr)?;
    
    // 5. 使能传感器时钟
    cif_dev.enable_sensor_clock(0, true)?;
    
    Ok(cif_dev)
}

/// 创建 Sub-LVDS 配置示例
pub fn create_sublvds_attr() -> ComboDevAttr {
    let mut attr = ComboDevAttr::default();
    
    attr.input_mode = InputMode::Sublvds;
    attr.devno = 0;
    attr.mac_clk = RxMacClk::Clk400M;
    
    attr.img_size = ImgSize {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    
    // LVDS 特定配置
    attr.lvds_attr = Some(LvdsDevAttr {
        hdr_mode: HdrMode::None,
        sync_mode: LvdsSyncMode::Sav,
        raw_data_type: RawDataType::Raw10Bit,
        data_endian: LvdsBitEndian::Big,
        sync_code_endian: LvdsBitEndian::Big,
        lane_id: [0, 1, 2, 3, -1],
        sync_code: [[[0; 4]; 3]; 4],
        vsync_type: LvdsVsyncTypeS {
            sync_type: LvdsVsyncType::Normal,
            hblank1: 0,
            hblank2: 0,
        },
        fid_type: LvdsFidTypeS {
            fid: LvdsFidType::None,
        },
        pn_swap: [0; 5],
    });
    
    attr
}

// ============================================================================
// 视频帧捕获示例
// ============================================================================

/// 视频帧捕获完整示例
///
/// 这个示例展示了如何：
/// 1. 初始化 CIF 设备
/// 2. 配置 MIPI 接口
/// 3. 分配帧缓冲区
/// 4. 捕获视频帧
/// 5. 处理帧数据
///
/// # Safety
/// 调用者必须确保寄存器地址有效，且提供的物理地址可用
///
/// # 示例代码
///
/// ```rust,ignore
/// use sg200x_bsp::cif::examples::*;
///
/// // 帧缓冲区物理地址（需要从系统内存分配器获取）
/// let frame_buffers = [
///     0x8000_0000usize, // Buffer 0
///     0x8100_0000usize, // Buffer 1
///     0x8200_0000usize, // Buffer 2
/// ];
/// let buffer_sizes = [0x100_0000usize; 3]; // 每个 16MB
///
/// // 运行帧捕获
/// unsafe {
///     capture_frames_example(&frame_buffers, &buffer_sizes, 10)?;
/// }
/// ```
pub unsafe fn capture_frames_example(
    frame_buffer_addrs: &[usize],
    buffer_sizes: &[usize],
    frame_count: u32,
) -> Result<(), CaptureError> {
    // 1. 初始化 CIF 设备
    let cif_dev = init_cif_for_mipi().map_err(|_| CaptureError::NotInitialized)?;
    
    // 2. 创建帧捕获器
    let mut capture = FrameCapture::new(0);
    capture.init(cif_dev);
    
    // 3. 配置捕获参数
    let config = CaptureConfig {
        width: 2560,
        height: 1440,
        format: PixelFormat::Raw10,
        buffer_count: 3,
    };
    capture.configure(&config)?;
    
    // 4. 分配帧缓冲区
    capture.allocate_buffers(frame_buffer_addrs, buffer_sizes)?;
    
    // 5. 开始捕获
    capture.start()?;
    
    // 6. 捕获指定数量的帧
    for i in 0..frame_count {
        // 获取一帧（1000ms 超时）
        match capture.get_frame(1000) {
            Ok(frame) => {
                // 处理帧数据
                log::info!(
                    "Frame {}: addr=0x{:08x}, size={}, {}x{}",
                    i,
                    frame.phy_addr,
                    frame.size,
                    frame.width,
                    frame.height
                );
                
                // 这里可以添加实际的帧处理逻辑
                // 例如：保存到文件、发送到网络、进行图像处理等
                
                // 释放帧
                capture.release_frame(&frame)?;
            }
            Err(e) => {
                log::error!("Failed to get frame {}: {:?}", i, e);
                
                // 检查错误统计
                if let Some(stats) = capture.get_error_stats() {
                    log::error!(
                        "Error stats: ECC={}, CRC={}, FIFO={}",
                        stats.errcnt_ecc,
                        stats.errcnt_crc,
                        stats.fifo_full
                    );
                }
                
                return Err(e);
            }
        }
    }
    
    // 7. 停止捕获
    capture.stop()?;
    
    log::info!("Captured {} frames successfully", frame_count);
    
    Ok(())
}

// ============================================================================
// 底层驱动使用示例
// ============================================================================

/// 底层驱动使用示例
///
/// 这个示例展示了如何直接使用底层驱动函数来配置 CIF。
/// 适用于需要更精细控制的场景。
///
/// # Safety
/// 调用者必须确保寄存器地址有效
pub unsafe fn low_level_cif_example() {
    // 创建驱动上下文
    let ctx = CifDrvCtx::new(0);
    
    // 配置 CSI 参数
    let csi_param = ParamCsi {
        lane_num: 2,
        fmt: CsiFmt::Raw10,
        vs_gen_mode: CsiVsGenMode::Fs,
        hdr_mode: CsiHdrMode::Vc,
        data_type: [0x2B, 0],
        decode_type: 0x2B,
        vc_mapping: [0, 1, 2, 3],
    };
    
    // 配置 CSI
    cif_config_csi(&ctx, &csi_param);
    
    // 配置 Lane ID
    // CLK Lane
    cif_set_lane_id(&ctx, LaneId::Clk, 2, true);
    // Data Lane 0
    cif_set_lane_id(&ctx, LaneId::Lane0, 1, true);
    // Data Lane 1
    cif_set_lane_id(&ctx, LaneId::Lane1, 3, true);
    
    // 设置 HS Settle 时间（如果需要）
    // cif_set_hs_settle(&ctx, 0x10);
    
    // 取消屏蔽中断
    cif_unmask_csi_int_sts(&ctx, 0x1F);
    
    // 开始流
    cif_streaming(&ctx, true, CifType::Csi, 2);
    
    // ... 等待数据 ...
    
    // 检查中断状态
    if cif_check_csi_int_sts(&ctx, CIF_INT_STS_CRC_ERR_MASK) {
        log::error!("CRC error detected!");
    }
    
    if cif_check_csi_fifo_full(&ctx) {
        log::error!("FIFO overflow!");
    }
    
    // 获取解码格式
    let fmt = cif_get_csi_decode_fmt(&ctx);
    log::info!("Decode format: {:?}", fmt);
    
    // 停止流
    cif_streaming(&ctx, false, CifType::Csi, 2);
    
    // 屏蔽中断
    cif_mask_csi_int_sts(&ctx, 0x1F);
}

// ============================================================================
// 帧数据处理示例
// ============================================================================

/// 将 RAW10 数据转换为 RAW16
///
/// RAW10 数据是 10-bit 打包格式，每 4 个像素占用 5 个字节。
/// 这个函数将其转换为 16-bit 格式，方便后续处理。
///
/// # Arguments
/// * `src` - 源数据（RAW10 打包格式）
/// * `dst` - 目标缓冲区（RAW16 格式）
/// * `width` - 图像宽度
/// * `height` - 图像高度
pub fn convert_raw10_to_raw16(src: &[u8], dst: &mut [u16], width: u32, height: u32) {
    let pixels = (width * height) as usize;
    let src_stride = (width as usize * 10 + 7) / 8; // 每行字节数
    
    for y in 0..height as usize {
        let src_row = &src[y * src_stride..];
        let dst_row = &mut dst[y * width as usize..];
        
        for x in (0..width as usize).step_by(4) {
            // 每 4 个像素占用 5 个字节
            let byte_offset = x * 10 / 8;
            
            if byte_offset + 4 < src_row.len() && x + 3 < dst_row.len() {
                let b0 = src_row[byte_offset] as u16;
                let b1 = src_row[byte_offset + 1] as u16;
                let b2 = src_row[byte_offset + 2] as u16;
                let b3 = src_row[byte_offset + 3] as u16;
                let b4 = src_row[byte_offset + 4] as u16;
                
                // 解包 4 个 10-bit 像素
                dst_row[x] = (b0 << 2) | (b4 & 0x03);
                dst_row[x + 1] = (b1 << 2) | ((b4 >> 2) & 0x03);
                dst_row[x + 2] = (b2 << 2) | ((b4 >> 4) & 0x03);
                dst_row[x + 3] = (b3 << 2) | ((b4 >> 6) & 0x03);
            }
        }
    }
}

/// 计算图像的直方图
///
/// # Arguments
/// * `data` - 图像数据（16-bit）
/// * `width` - 图像宽度
/// * `height` - 图像高度
/// * `histogram` - 输出直方图（1024 个 bin，对应 10-bit 数据）
pub fn calculate_histogram(data: &[u16], width: u32, height: u32, histogram: &mut [u32; 1024]) {
    histogram.fill(0);
    
    for y in 0..height as usize {
        for x in 0..width as usize {
            let pixel = data[y * width as usize + x] as usize;
            if pixel < 1024 {
                histogram[pixel] += 1;
            }
        }
    }
}
