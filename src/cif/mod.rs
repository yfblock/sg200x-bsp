//! # CIF (Camera Interface) 驱动模块
//!
//! 本模块提供 SG2002 芯片的相机接口驱动，支持：
//! - MIPI CSI-2 接口
//! - LVDS/Sub-LVDS/HiSPI 接口
//! - DVP/BT656/BT601/BT1120 并行接口
//!
//! ## 模块结构
//!
//! - `types`: 类型定义和枚举
//! - `regs`: 寄存器定义（使用 tock-registers）
//! - `mipi`: MIPI CSI-2 配置
//! - `lvds`: LVDS/Sub-LVDS/HiSPI 配置
//! - `ttl`: TTL/DVP/BT 并行接口配置
//! - `phy`: PHY 层配置
//! - `vip_sys`: VIP 系统控制
//! - `frame`: 视频帧捕获
//!
//! ## 视频采集架构
//!
//! SG2002 的视频采集流程：
//! ```text
//! Sensor -> CIF (MIPI/LVDS/TTL) -> ISP -> VI -> Frame Buffer
//! ```
//!
//! CIF 是相机接口层，负责：
//! 1. 接收传感器的原始数据（通过 MIPI CSI-2、LVDS 或 TTL 接口）
//! 2. 解析数据包格式（如 MIPI CSI-2 的 Data Type）
//! 3. 将数据传递给 ISP 进行处理
//!
//! ## 使用示例
//!
//! ### 基本 MIPI 配置
//!
//! ```rust,ignore
//! use sg200x_bsp::cif::*;
//!
//! // 创建 CIF 设备
//! let mut cif_dev = unsafe { CifDev::new(0) };
//! cif_dev.init();
//!
//! // 配置 MIPI 属性
//! let mut attr = ComboDevAttr::default();
//! attr.input_mode = InputMode::Mipi;
//! attr.devno = 0;
//! attr.mac_clk = RxMacClk::Clk400M;
//! attr.img_size = ImgSize {
//!     x: 0,
//!     y: 0,
//!     width: 2560,
//!     height: 1440,
//! };
//!
//! // 配置 MIPI 特定参数
//! attr.mipi_attr = Some(MipiDevAttr {
//!     raw_data_type: RawDataType::Raw10Bit,
//!     lane_id: [0, 1, 2, 3, -1],
//!     hdr_mode: MipiHdrMode::None,
//!     data_type: [0x2B, 0],
//!     pn_swap: [0; 5],
//!     dphy: Dphy {
//!         enable: true,
//!         hs_settle: 0,
//!     },
//!     demux: MipiDemuxInfo {
//!         demux_en: false,
//!         vc_mapping: [0, 1, 2, 3],
//!     },
//! });
//!
//! // 应用配置
//! cif_dev.set_dev_attr(&attr).unwrap();
//!
//! // 使能传感器时钟
//! cif_dev.enable_sensor_clock(0, true).unwrap();
//! ```
//!
//! ### 获取视频帧
//!
//! ```rust,ignore
//! use sg200x_bsp::cif::frame::*;
//!
//! // 创建帧捕获器
//! let mut capture = unsafe { FrameCapture::new(0) };
//!
//! // 配置捕获参数
//! let config = CaptureConfig {
//!     width: 2560,
//!     height: 1440,
//!     format: PixelFormat::Raw10,
//!     buffer_count: 3,
//! };
//! capture.configure(&config)?;
//!
//! // 分配缓冲区（需要提供物理地址）
//! let phy_addrs = [0x8000_0000, 0x8100_0000, 0x8200_0000];
//! let sizes = [0x100_0000; 3]; // 每个 16MB
//! capture.allocate_buffers(&phy_addrs, &sizes)?;
//!
//! // 开始捕获
//! capture.start()?;
//!
//! // 获取一帧
//! let frame = capture.get_frame(1000)?; // 1000ms 超时
//!
//! // 处理帧数据...
//! // frame.phy_addr 是帧数据的物理地址
//! // frame.width, frame.height 是图像尺寸
//!
//! // 释放帧
//! capture.release_frame(&frame)?;
//!
//! // 停止捕获
//! capture.stop()?;
//! ```

#![allow(dead_code)]

pub mod types;
pub mod regs;
pub mod mipi;
pub mod lvds;
pub mod ttl;
pub mod phy;
pub mod vip_sys;
pub mod frame;
pub mod drv;
pub mod examples;

pub use types::*;

/// CIF 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CifError {
    InvalidDevno,
    InvalidConfig,
    Mipi(mipi::MipiError),
    Lvds(lvds::LvdsError),
    Ttl(ttl::TtlError),
}

impl From<mipi::MipiError> for CifError {
    fn from(err: mipi::MipiError) -> Self {
        Self::Mipi(err)
    }
}

impl From<lvds::LvdsError> for CifError {
    fn from(err: lvds::LvdsError) -> Self {
        Self::Lvds(err)
    }
}

impl From<ttl::TtlError> for CifError {
    fn from(err: ttl::TtlError) -> Self {
        Self::Ttl(err)
    }
}

/// CIF 最大 CSI 数量
pub const CIF_MAX_CSI_NUM: usize = 2;
/// CIF 最大 MAC 数量
pub const CIF_MAX_MAC_NUM: usize = 3;
/// CIF 最大 Link 数量
pub const MAX_LINK_NUM: usize = 3;

/// PHY Lane 数量
pub const CIF_PHY_LANE_NUM: usize = 6;
/// Lane 数量（含时钟）
pub const CIF_LANE_NUM: usize = 5;

/// MIPI Lane 数量
pub const MIPI_LANE_NUM: usize = 4;

/// CIF 上下文
pub struct CifCtx {
    /// MAC 物理寄存器基地址
    pub mac_phys_regs: usize,
    /// Wrap 物理寄存器基地址
    pub wrap_phys_regs: usize,
    /// 当前配置
    pub cur_config: Option<CifParam>,
    /// MAC 编号
    pub mac_num: u16,
}

impl CifCtx {
    /// 创建新的 CIF 上下文
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    pub unsafe fn new(mac_base: usize, wrap_base: usize, mac_num: u16) -> Self {
        Self {
            mac_phys_regs: mac_base,
            wrap_phys_regs: wrap_base,
            cur_config: None,
            mac_num,
        }
    }

    /// 设置当前配置
    pub fn set_config(&mut self, config: CifParam) {
        self.cur_config = Some(config);
    }

    /// 获取当前配置的引用
    pub fn config(&self) -> Option<&CifParam> {
        self.cur_config.as_ref()
    }

    /// 获取当前配置的可变引用
    pub fn config_mut(&mut self) -> Option<&mut CifParam> {
        self.cur_config.as_mut()
    }
}

/// CIF 设备
pub struct CifDev {
    /// 设备编号
    pub devno: u32,
    /// Link 数组
    pub links: [CifLink; MAX_LINK_NUM],
    /// 最大 MAC 时钟（MHz）
    pub max_mac_clk: u32,
}

/// CIF Link
pub struct CifLink {
    /// CIF 上下文
    pub cif_ctx: CifCtx,
    /// 中断号
    pub irq_num: u32,
    /// 是否开启
    pub is_on: bool,
    /// CIF 参数
    pub param: CifParam,
    /// 组合设备属性
    pub attr: ComboDevAttr,
    /// 时钟边沿
    pub clk_edge: ClkEdge,
    /// 输出 MSB
    pub msb: OutputMsb,
    /// 裁剪顶部行数
    pub crop_top: u32,
    /// 帧前沿距离
    pub distance_fp: u32,
    /// 传感器复位引脚
    pub snsr_rst_pin: i32,
    /// 传感器复位极性
    pub snsr_rst_pol: GpioFlags,
    /// MAC 时钟
    pub mac_clk: RxMacClk,
    /// BT 格式输出
    pub bt_fmt_out: TtlBtFmtOut,
    /// CSI 状态
    pub sts_csi: CsiStatus,
}

/// CSI 状态
#[derive(Debug, Clone, Copy, Default)]
pub struct CsiStatus {
    /// ECC 错误计数
    pub errcnt_ecc: u32,
    /// CRC 错误计数
    pub errcnt_crc: u32,
    /// 头部错误计数
    pub errcnt_hdr: u32,
    /// 字数错误计数
    pub errcnt_wc: u32,
    /// FIFO 满计数
    pub fifo_full: u32,
}

/// LVDS 状态
#[derive(Debug, Clone, Copy, Default)]
pub struct LvdsStatus {
    /// FIFO 满计数
    pub fifo_full: u32,
}

impl CifDev {
    /// 创建新的 CIF 设备
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    pub unsafe fn new(devno: u32) -> Self {
        Self {
            devno,
            links: core::array::from_fn(|_| CifLink::default()),
            max_mac_clk: 600, // 默认 600MHz
        }
    }

    /// 获取指定 Link
    pub fn link(&self, index: usize) -> Option<&CifLink> {
        self.links.get(index)
    }

    /// 获取指定 Link（可变）
    pub fn link_mut(&mut self, index: usize) -> Option<&mut CifLink> {
        self.links.get_mut(index)
    }
}

impl Default for CifLink {
    fn default() -> Self {
        Self {
            cif_ctx: unsafe { CifCtx::new(0, 0, 0) },
            irq_num: 0,
            is_on: false,
            param: CifParam::default(),
            attr: ComboDevAttr::default(),
            clk_edge: ClkEdge::Up,
            msb: OutputMsb::Normal,
            crop_top: 0,
            distance_fp: 0,
            snsr_rst_pin: -1,
            snsr_rst_pol: GpioFlags::ActiveLow,
            mac_clk: RxMacClk::Clk400M,
            bt_fmt_out: TtlBtFmtOut::Cbycry,
            sts_csi: CsiStatus::default(),
        }
    }
}

impl CifDev {
    /// 初始化 CIF 设备
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    pub unsafe fn init(&mut self) {
        // 初始化所有 Link 的寄存器基地址
        for i in 0..MAX_LINK_NUM {
            let mac_base = regs::get_mac_phys_reg_bases(i as u32);
            let wrap_base = regs::get_wrap_phys_reg_bases(i as u32);
            self.links[i].cif_ctx.mac_phys_regs = mac_base;
            self.links[i].cif_ctx.wrap_phys_regs = wrap_base;
            self.links[i].cif_ctx.mac_num = i as u16;
        }
    }

    /// 设置设备属性
    pub fn set_dev_attr(&mut self, attr: &ComboDevAttr) -> Result<(), CifError> {
        if attr.devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &mut self.links[attr.devno as usize];
        link.attr = *attr;

        // 设置 MAC 时钟
        unsafe {
            vip_sys::set_mac_clk(attr.devno, attr.mac_clk, self.max_mac_clk)
                .map_err(|_| CifError::InvalidConfig)?;
        }

        // 根据输入模式配置
        match attr.input_mode {
            InputMode::Mipi => {
                if let Some(mipi_attr) = &attr.mipi_attr {
                    mipi::set_mipi_attr(&mut link.cif_ctx, mipi_attr, &attr.img_size)?;
                }
            }
            InputMode::Sublvds => {
                if let Some(lvds_attr) = &attr.lvds_attr {
                    lvds::set_sublvds_attr(&mut link.cif_ctx, lvds_attr, &attr.img_size)?;
                }
            }
            InputMode::Hispi => {
                if let Some(lvds_attr) = &attr.lvds_attr {
                    lvds::set_hispi_attr(&mut link.cif_ctx, lvds_attr, &attr.img_size)?;
                }
            }
            InputMode::Cmos => {
                if let Some(ttl_attr) = &attr.ttl_attr {
                    ttl::set_cmos_attr(&mut link.cif_ctx, ttl_attr, &attr.img_size)?;
                }
            }
            InputMode::Bt1120 => {
                if let Some(ttl_attr) = &attr.ttl_attr {
                    ttl::set_bt1120_attr(
                        &mut link.cif_ctx,
                        ttl_attr,
                        &attr.img_size,
                        link.clk_edge,
                    )?;
                }
            }
            InputMode::Bt601_19bVhs => {
                if let Some(ttl_attr) = &attr.ttl_attr {
                    ttl::set_bt601_attr(
                        &mut link.cif_ctx,
                        ttl_attr,
                        &attr.img_size,
                        link.clk_edge,
                    )?;
                }
            }
            InputMode::Bt656_9b => {
                if let Some(ttl_attr) = &attr.ttl_attr {
                    ttl::set_bt656_attr(
                        &mut link.cif_ctx,
                        ttl_attr,
                        &attr.img_size,
                        link.clk_edge,
                    )?;
                }
            }
            InputMode::BtDemux => {
                if let Some(bt_demux_attr) = &attr.bt_demux_attr {
                    ttl::set_bt_demux_attr(
                        &mut link.cif_ctx,
                        bt_demux_attr,
                        &attr.img_size,
                        link.clk_edge,
                    )?;
                }
            }
            _ => return Err(CifError::InvalidConfig),
        }

        link.is_on = true;
        link.mac_clk = attr.mac_clk;

        Ok(())
    }

    /// 复位 MIPI
    pub fn reset_mipi(&mut self, devno: u32) -> Result<(), CifError> {
        if devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &mut self.links[devno as usize];

        // 屏蔽中断
        if link.is_on {
            mipi::mask_csi_int_sts(&link.cif_ctx, 0x1F);
        }

        // 执行硬件复位
        unsafe {
            vip_sys::reset_mipi(devno);
        }

        // 重置参数
        link.is_on = false;
        link.crop_top = 0;
        link.distance_fp = 0;
        link.sts_csi = CsiStatus::default();

        Ok(())
    }

    /// 设置输出时钟边沿
    pub fn set_output_clk_edge(&mut self, devno: u32, edge: ClkEdge) -> Result<(), CifError> {
        if devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &mut self.links[devno as usize];
        link.clk_edge = edge;

        let cif_edge = match edge {
            ClkEdge::Up => CifClkEdge::Rising,
            ClkEdge::Down => CifClkEdge::Falling,
        };

        // 设置所有 PHY Lane 的时钟边沿
        for i in 0..6 {
            phy::set_clk_edge(
                &link.cif_ctx,
                match i {
                    0 => PhyLaneId::Lane0,
                    1 => PhyLaneId::Lane1,
                    2 => PhyLaneId::Lane2,
                    3 => PhyLaneId::Lane3,
                    4 => PhyLaneId::Lane4,
                    5 => PhyLaneId::Lane5,
                    _ => unreachable!(),
                },
                cif_edge,
            );
        }

        Ok(())
    }

    /// 使能传感器时钟
    pub fn enable_sensor_clock(&mut self, devno: u32, enable: bool) -> Result<(), CifError> {
        if devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &self.links[devno as usize];
        unsafe {
            vip_sys::enable_sensor_clock(devno, link.attr.mclk.freq, enable)
                .map_err(|_| CifError::InvalidConfig)?;
        }

        Ok(())
    }

    /// 设置裁剪顶部行数
    pub fn set_crop_top(&mut self, devno: u32, crop_top: u32, update: bool) -> Result<(), CifError> {
        if devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &mut self.links[devno as usize];
        link.crop_top = crop_top;

        // 实际的裁剪配置需要调用底层函数
        log::debug!("Set crop top: devno={}, crop_top={}, update={}", devno, crop_top, update);

        Ok(())
    }

    /// 设置手动 HDR
    pub fn set_hdr_manual(&mut self, devno: u32, hdr_attr: &ManualHdrAttr) -> Result<(), CifError> {
        if devno >= MAX_LINK_NUM as u32 {
            return Err(CifError::InvalidDevno);
        }

        let link = &mut self.links[devno as usize];
        link.param.hdr_manual = hdr_attr.manual_en;
        link.param.hdr_shift = hdr_attr.l2s_distance;
        link.param.hdr_vsize = hdr_attr.lsef_length;
        link.param.hdr_rm_padding = if hdr_attr.discard_padding_lines { 1 } else { 0 };

        // 实际的 HDR 配置需要调用底层函数
        log::debug!("Set HDR manual: devno={}, en={}", devno, hdr_attr.manual_en);

        Ok(())
    }
}
