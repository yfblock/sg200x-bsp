//! # VI (Video Input) 驱动模块
//!
//! 本模块提供 SG2002 芯片的视频输入驱动，支持：
//! - MIPI CSI-2 接口
//! - Sub-LVDS/HiSPi 接口
//! - BT.656/BT.601/BT.1120 接口
//! - DC (Digital Camera) 接口
//!
//! ## 模块结构
//!
//! - `types`: 类型定义和枚举
//! - `regs`: 寄存器定义（使用 tock-registers）
//! - `bt`: BT 接口配置
//! - `ttl`: TTL 接口配置
//!
//! ## 视频采集架构
//!
//! SG2002 的视频采集流程：
//!
//! ```text
//! Sensor -> CIF (MIPI/LVDS/TTL) -> VI -> ISP -> Frame Buffer
//!            |                     |
//!            +-- MIPI Rx 模块 -----+-- VI Proc 模块
//! ```
//!
//! VI 模块由两个物理子模块组成：
//! 1. **MIPI Rx**: 接收处理不同的视频数据
//! 2. **VI Proc**: 将不同格式的视频信号统整为 ISP 所需的单一视频信号
//!
//! ## 支持的输入配置
//!
//! | 配置 | 分辨率 | 帧率 | 模式 |
//! |------|--------|------|------|
//! | 单路 5M | 2688x1944 / 2880x1620 | 60fps | HDR |
//! | 单路 5M | 2688x1944 / 2880x1620 | 30fps | 线性 |
//! | 双路 FHD | 1920x1080 | 60fps | HDR/线性 |
//! | 单路 5M + BT | 5M + BT.656/601/1120 | - | - |
//!
//! ## 使用示例
//!
//! ### 配置 BT.656 输入
//!
//! ```rust,ignore
//! use sg200x_bsp::vi::*;
//!
//! // 创建 VI 设备
//! let mut vi = unsafe { Vi::new(ViDevno::Vi2) };
//!
//! // 配置 BT.656 属性
//! let config = BtConfig {
//!     enable: true,
//!     format: BtFormat::Bt656_9bit,
//!     demux_ch: BtDemuxChannel::None,
//!     img_size: ImageSize::new(720, 480),
//!     ..Default::default()
//! };
//!
//! // 应用配置
//! vi.configure_bt(&config)?;
//!
//! // 启动 VI
//! vi.enable()?;
//! ```
//!
//! ### 配置 BT.1120 输入
//!
//! ```rust,ignore
//! use sg200x_bsp::vi::*;
//!
//! let mut vi = unsafe { Vi::new(ViDevno::Vi0) };
//!
//! // 配置 BT.1120 (16-bit YUV422)
//! let config = BtConfig {
//!     enable: true,
//!     format: BtFormat::Bt1120_17bit,
//!     img_size: ImageSize::new(1920, 1080),
//!     blanking: BlankingConfig {
//!         vs_back_porch: 41,
//!         hs_back_porch: 192,
//!         ..Default::default()
//!     },
//!     ..Default::default()
//! };
//!
//! vi.configure_bt(&config)?;
//! vi.enable()?;
//! ```

#![allow(dead_code)]

pub mod regs;
pub mod types;
pub mod bt;
pub mod ttl;

pub use types::*;

use tock_registers::interfaces::ReadWriteable;

// ============================================================================
// VI 设备结构体
// ============================================================================

/// VI (Video Input) 设备
///
/// 代表一个 VI 硬件设备实例
pub struct Vi {
    /// 设备编号
    devno: ViDevno,
    /// 寄存器基地址
    base_addr: usize,
    /// 当前配置
    config: Option<ViDevAttr>,
    /// 是否已初始化
    initialized: bool,
}

impl Vi {
    /// 创建新的 VI 设备实例
    ///
    /// # Arguments
    /// * `devno` - VI 设备编号 (0, 1, 2)
    ///
    /// # Safety
    /// 调用者必须确保：
    /// - 寄存器地址有效且可访问
    /// - 同一时间只有一个实例访问同一个 VI 设备
    ///
    /// # Example
    /// ```rust,ignore
    /// let vi = unsafe { Vi::new(ViDevno::Vi0) };
    /// ```
    pub unsafe fn new(devno: ViDevno) -> Self {
        let base_addr = regs::vi_base_addr(devno as usize);
        Self {
            devno,
            base_addr,
            config: None,
            initialized: false,
        }
    }

    /// 获取设备编号
    #[inline]
    pub fn devno(&self) -> ViDevno {
        self.devno
    }

    /// 获取寄存器基地址
    #[inline]
    pub fn base_addr(&self) -> usize {
        self.base_addr
    }

    /// 获取寄存器引用
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效
    #[inline]
    unsafe fn regs(&self) -> &regs::ViRegs {
        &*(self.base_addr as *const regs::ViRegs)
    }

    /// 初始化 VI 设备
    ///
    /// 执行基本的硬件初始化，包括：
    /// - 复位相关控制器
    /// - 设置默认配置
    ///
    /// # Returns
    /// * `Ok(())` - 初始化成功
    /// * `Err(ViError)` - 初始化失败
    pub fn init(&mut self) -> Result<(), ViError> {
        // 禁用所有控制器
        unsafe {
            let regs = self.regs();

            // 设置默认模式为禁用
            regs.reg_00.modify(
                regs::REG_00::REG_SENSOR_MAC_MODE::Disable
                    + regs::REG_00::REG_BT_DEMUX_ENABLE::CLEAR
                    + regs::REG_00::REG_CSI_CTRL_ENABLE::CLEAR
                    + regs::REG_00::REG_SUBLVDS_CTRL_ENABLE::CLEAR,
            );

            // 禁用 TTL
            regs.reg_10.modify(regs::REG_10::REG_TTL_IP_EN::CLEAR);

            // 禁用 BT 路径
            regs.reg_80.modify(regs::REG_80::REG_BT_IP_EN::CLEAR);
        }

        self.initialized = true;
        Ok(())
    }

    /// 配置传感器 MAC 模式
    ///
    /// # Arguments
    /// * `mode` - 传感器 MAC 模式
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn set_mac_mode(&mut self, mode: SensorMacMode) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            match mode {
                SensorMacMode::Disable => {
                    regs.reg_00
                        .modify(regs::REG_00::REG_SENSOR_MAC_MODE::Disable);
                }
                SensorMacMode::Csi => {
                    // VI2 不支持 CSI 模式
                    if self.devno == ViDevno::Vi2 {
                        return Err(ViError::InvalidInputMode);
                    }
                    regs.reg_00.modify(
                        regs::REG_00::REG_SENSOR_MAC_MODE::Csi
                            + regs::REG_00::REG_CSI_CTRL_ENABLE::SET,
                    );
                }
                SensorMacMode::SubLvds => {
                    // VI2 不支持 Sub-LVDS 模式
                    if self.devno == ViDevno::Vi2 {
                        return Err(ViError::InvalidInputMode);
                    }
                    regs.reg_00.modify(
                        regs::REG_00::REG_SENSOR_MAC_MODE::SubLvds
                            + regs::REG_00::REG_SUBLVDS_CTRL_ENABLE::SET,
                    );
                }
                SensorMacMode::Ttl => {
                    regs.reg_00
                        .modify(regs::REG_00::REG_SENSOR_MAC_MODE::Ttl);
                }
            }
        }

        Ok(())
    }

    /// 配置 VI 输入模式
    ///
    /// # Arguments
    /// * `mode` - VI 输入模式
    /// * `source` - VI 输入来源
    /// * `clk_inv` - 时钟反相
    pub fn set_input_mode(
        &mut self,
        mode: ViInputMode,
        source: ViInputSource,
        clk_inv: bool,
    ) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            // 设置输入模式
            let mode_val = mode as u32;
            let source_val = source as u32;

            regs.reg_30.modify(
                regs::REG_30::REG_VI_SEL.val(mode_val)
                    + regs::REG_30::REG_VI_FROM.val(source_val)
                    + if clk_inv {
                        regs::REG_30::REG_VI_CLK_INV::SET
                    } else {
                        regs::REG_30::REG_VI_CLK_INV::CLEAR
                    },
            );
        }

        Ok(())
    }

    /// 配置裁剪区域
    ///
    /// # Arguments
    /// * `region` - 裁剪区域配置，None 表示禁用裁剪
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn set_crop(&mut self, region: Option<CropRegion>) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            match region {
                Some(crop) => {
                    // 配置水平裁剪
                    regs.reg_b0.modify(
                        regs::REG_B0::REG_SENSOR_MAC_CROP_START_X.val(crop.start_x as u32)
                            + regs::REG_B0::REG_SENSOR_MAC_CROP_END_X.val(crop.end_x as u32)
                            + regs::REG_B0::REG_SENSOR_MAC_CROP_EN::SET,
                    );

                    // 配置垂直裁剪
                    regs.reg_b4.modify(
                        regs::REG_B4::REG_SENSOR_MAC_CROP_START_Y.val(crop.start_y as u32)
                            + regs::REG_B4::REG_SENSOR_MAC_CROP_END_Y.val(crop.end_y as u32),
                    );
                }
                None => {
                    // 禁用裁剪
                    regs.reg_b0
                        .modify(regs::REG_B0::REG_SENSOR_MAC_CROP_EN::CLEAR);
                }
            }
        }

        Ok(())
    }

    /// 配置 HDR 模式
    ///
    /// # Arguments
    /// * `config` - HDR 配置，None 表示禁用 HDR
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn set_hdr(&mut self, config: Option<HdrConfig>) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            match config {
                Some(hdr) => {
                    // 配置 HDR 模式
                    regs.reg_40.modify(
                        if hdr.enable {
                            regs::REG_40::REG_SENSOR_MAC_HDR_EN::SET
                        } else {
                            regs::REG_40::REG_SENSOR_MAC_HDR_EN::CLEAR
                        } + if hdr.vs_inv {
                            regs::REG_40::REG_SENSOR_MAC_HDR_VSINV::SET
                        } else {
                            regs::REG_40::REG_SENSOR_MAC_HDR_VSINV::CLEAR
                        } + if hdr.hs_inv {
                            regs::REG_40::REG_SENSOR_MAC_HDR_HSINV::SET
                        } else {
                            regs::REG_40::REG_SENSOR_MAC_HDR_HSINV::CLEAR
                        } + if hdr.de_inv {
                            regs::REG_40::REG_SENSOR_MAC_HDR_DEINV::SET
                        } else {
                            regs::REG_40::REG_SENSOR_MAC_HDR_DEINV::CLEAR
                        } + if hdr.mode {
                            regs::REG_40::REG_SENSOR_MAC_HDR_MODE::SET
                        } else {
                            regs::REG_40::REG_SENSOR_MAC_HDR_MODE::CLEAR
                        },
                    );

                    // 配置 HDR 参数
                    regs.reg_44.modify(
                        regs::REG_44::REG_SENSOR_MAC_HDR_SHIFT.val(hdr.shift as u32)
                            + regs::REG_44::REG_SENSOR_MAC_HDR_VSIZE.val(hdr.vsize as u32),
                    );
                }
                None => {
                    // 禁用 HDR
                    regs.reg_40
                        .modify(regs::REG_40::REG_SENSOR_MAC_HDR_EN::CLEAR);
                }
            }
        }

        Ok(())
    }

    /// 配置 BLC (Black Level Calibration)
    ///
    /// # Arguments
    /// * `config` - BLC 配置，None 表示禁用 BLC
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn set_blc(&mut self, config: Option<BlcConfig>) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            match config {
                Some(blc) => {
                    // 配置 BLC 使能
                    regs.reg_50.modify(
                        if blc.blc0_enable {
                            regs::REG_50::REG_SENSOR_MAC_BLC0_EN::SET
                        } else {
                            regs::REG_50::REG_SENSOR_MAC_BLC0_EN::CLEAR
                        } + if blc.blc1_enable {
                            regs::REG_50::REG_SENSOR_MAC_BLC1_EN::SET
                        } else {
                            regs::REG_50::REG_SENSOR_MAC_BLC1_EN::CLEAR
                        },
                    );

                    // 配置 BLC0 参数
                    if blc.blc0_enable {
                        regs.reg_54.modify(
                            regs::REG_54::REG_SENSOR_MAC_BLC0_START.val(blc.blc0_start as u32)
                                + regs::REG_54::REG_SENSOR_MAC_BLC0_SIZE.val(blc.blc0_size as u32),
                        );
                    }

                    // 配置 BLC1 参数
                    if blc.blc1_enable {
                        regs.reg_58.modify(
                            regs::REG_58::REG_SENSOR_MAC_BLC1_START.val(blc.blc1_start as u32)
                                + regs::REG_58::REG_SENSOR_MAC_BLC1_SIZE.val(blc.blc1_size as u32),
                        );
                    }
                }
                None => {
                    // 禁用 BLC
                    regs.reg_50.modify(
                        regs::REG_50::REG_SENSOR_MAC_BLC0_EN::CLEAR
                            + regs::REG_50::REG_SENSOR_MAC_BLC1_EN::CLEAR,
                    );
                }
            }
        }

        Ok(())
    }

    /// 配置 BT 接口
    ///
    /// # Arguments
    /// * `config` - BT 接口配置
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    ///
    /// # Example
    /// ```rust,ignore
    /// let config = BtConfig {
    ///     enable: true,
    ///     format: BtFormat::Bt656_9bit,
    ///     img_size: ImageSize::new(720, 480),
    ///     ..Default::default()
    /// };
    /// vi.configure_bt(&config)?;
    /// ```
    pub fn configure_bt(&mut self, config: &BtConfig) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // 验证图像尺寸
        if !config.img_size.is_valid() {
            return Err(ViError::InvalidImageSize);
        }

        bt::configure_bt(self, config)
    }

    /// 配置 TTL 接口
    ///
    /// # Arguments
    /// * `config` - TTL 接口配置
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn configure_ttl(&mut self, config: &TtlConfig) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // VI2 不支持 TTL 模式的某些功能
        if self.devno == ViDevno::Vi2 {
            // VI2 仅支持 BT 接口
            return Err(ViError::InvalidInputMode);
        }

        // 验证图像尺寸
        if !config.img_size.is_valid() {
            return Err(ViError::InvalidImageSize);
        }

        ttl::configure_ttl(self, config)
    }

    /// 配置 Sub-LVDS 接口
    ///
    /// # Arguments
    /// * `config` - Sub-LVDS 接口配置
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn configure_sublvds(&mut self, config: &SubLvdsConfig) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // VI2 不支持 Sub-LVDS 模式
        if self.devno == ViDevno::Vi2 {
            return Err(ViError::InvalidInputMode);
        }

        unsafe {
            let regs = self.regs();

            // 配置 Sub-LVDS 模式
            regs.reg_d0.modify(
                if config.enable {
                    regs::REG_D0::REG_TTL_AS_SLVDS_ENABLE::SET
                } else {
                    regs::REG_D0::REG_TTL_AS_SLVDS_ENABLE::CLEAR
                } + regs::REG_D0::REG_TTL_AS_SLVDS_BIT_MODE.val(config.bit_mode as u32)
                    + if config.data_reverse {
                        regs::REG_D0::REG_TTL_AS_SLVDS_DATA_REVERSE::SET
                    } else {
                        regs::REG_D0::REG_TTL_AS_SLVDS_DATA_REVERSE::CLEAR
                    }
                    + if config.hdr_enable {
                        regs::REG_D0::REG_TTL_AS_SLVDS_HDR_MODE::SET
                    } else {
                        regs::REG_D0::REG_TTL_AS_SLVDS_HDR_MODE::CLEAR
                    }
                    + regs::REG_D0::REG_TTL_AS_SLVDS_HDR_PATTERN
                        .val(config.hdr_pattern as u32),
            );

            // 配置同步码
            regs.reg_d4.modify(
                regs::REG_D4::REG_TTL_AS_SLVDS_SYNC_1ST.val(config.sync_1st as u32)
                    + regs::REG_D4::REG_TTL_AS_SLVDS_SYNC_2ND.val(config.sync_2nd as u32),
            );

            regs.reg_d8.modify(regs::REG_D8::REG_TTL_AS_SLVDS_SYNC_3RD.val(config.sync_3rd as u32));

            // 设置传感器 MAC 模式为 Sub-LVDS
            if config.enable {
                self.set_mac_mode(SensorMacMode::SubLvds)?;
            }
        }

        Ok(())
    }

    /// 配置 HiSPi 接口
    ///
    /// # Arguments
    /// * `config` - HiSPi 接口配置
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn configure_hispi(&mut self, config: &HispiConfig) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // VI2 不支持 HiSPi 模式
        if self.devno == ViDevno::Vi2 {
            return Err(ViError::InvalidInputMode);
        }

        unsafe {
            let regs = self.regs();

            // 配置 HiSPi 模式
            regs.reg_110.modify(
                if config.enable {
                    regs::REG_110::REG_TTL_AS_HISPI_MODE::SET
                } else {
                    regs::REG_110::REG_TTL_AS_HISPI_MODE::CLEAR
                } + if config.use_hsize {
                    regs::REG_110::REG_TTL_AS_HISPI_USE_HSIZE::SET
                } else {
                    regs::REG_110::REG_TTL_AS_HISPI_USE_HSIZE::CLEAR
                } + if config.hdr_psp_mode {
                    regs::REG_110::REG_TTL_AS_HISPI_HDR_PSP_MODE::SET
                } else {
                    regs::REG_110::REG_TTL_AS_HISPI_HDR_PSP_MODE::CLEAR
                },
            );

            // 配置普通模式同步码
            regs.reg_114.modify(
                regs::REG_114::REG_TTL_AS_HISPI_NORM_SOF.val(config.norm_sof as u32)
                    + regs::REG_114::REG_TTL_AS_HISPI_NORM_EOF.val(config.norm_eof as u32),
            );
        }

        Ok(())
    }

    /// 使能 VI 设备
    ///
    /// 根据当前配置使能相应的接口
    ///
    /// # Returns
    /// * `Ok(())` - 使能成功
    /// * `Err(ViError)` - 使能失败
    pub fn enable(&mut self) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // 根据当前配置使能相应的接口
        // 这里的具体实现取决于之前配置的接口类型

        Ok(())
    }

    /// 禁用 VI 设备
    ///
    /// 禁用所有接口
    ///
    /// # Returns
    /// * `Ok(())` - 禁用成功
    /// * `Err(ViError)` - 禁用失败
    pub fn disable(&mut self) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            // 禁用所有控制器
            regs.reg_00.modify(
                regs::REG_00::REG_SENSOR_MAC_MODE::Disable
                    + regs::REG_00::REG_BT_DEMUX_ENABLE::CLEAR
                    + regs::REG_00::REG_CSI_CTRL_ENABLE::CLEAR
                    + regs::REG_00::REG_SUBLVDS_CTRL_ENABLE::CLEAR,
            );

            // 禁用 TTL
            regs.reg_10.modify(regs::REG_10::REG_TTL_IP_EN::CLEAR);

            // 禁用 BT 路径
            regs.reg_80.modify(regs::REG_80::REG_BT_IP_EN::CLEAR);
        }

        Ok(())
    }

    /// 清除同步丢失状态
    ///
    /// 当检测到同步丢失时，调用此函数清除状态
    pub fn clear_sync_lost(&mut self) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();
            regs.reg_80
                .modify(regs::REG_80::REG_BT_CLR_SYNC_LOST_1T::SET);
        }

        Ok(())
    }

    /// 设置 VI 引脚时钟反相
    ///
    /// # Arguments
    /// * `vi0_inv` - VI0 时钟反相
    /// * `vi1_inv` - VI1 时钟反相
    /// * `vi2_inv` - VI2 时钟反相
    pub fn set_pad_clk_inv(
        &mut self,
        vi0_inv: bool,
        vi1_inv: bool,
        vi2_inv: bool,
    ) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        unsafe {
            let regs = self.regs();

            regs.reg_30.modify(
                if vi0_inv {
                    regs::REG_30::REG_PAD_VI0_CLK_INV::SET
                } else {
                    regs::REG_30::REG_PAD_VI0_CLK_INV::CLEAR
                } + if vi1_inv {
                    regs::REG_30::REG_PAD_VI1_CLK_INV::SET
                } else {
                    regs::REG_30::REG_PAD_VI1_CLK_INV::CLEAR
                } + if vi2_inv {
                    regs::REG_30::REG_PAD_VI2_CLK_INV::SET
                } else {
                    regs::REG_30::REG_PAD_VI2_CLK_INV::CLEAR
                },
            );
        }

        Ok(())
    }

    /// 配置 CSI 信号极性
    ///
    /// # Arguments
    /// * `vs_inv` - VS 信号反相
    /// * `hs_inv` - HS 信号反相
    pub fn set_csi_polarity(&mut self, vs_inv: bool, hs_inv: bool) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // VI2 不支持 CSI
        if self.devno == ViDevno::Vi2 {
            return Err(ViError::InvalidInputMode);
        }

        unsafe {
            let regs = self.regs();

            regs.reg_00.modify(
                if vs_inv {
                    regs::REG_00::REG_CSI_VS_INV::SET
                } else {
                    regs::REG_00::REG_CSI_VS_INV::CLEAR
                } + if hs_inv {
                    regs::REG_00::REG_CSI_HS_INV::SET
                } else {
                    regs::REG_00::REG_CSI_HS_INV::CLEAR
                },
            );
        }

        Ok(())
    }

    /// 配置 Sub-LVDS 信号极性
    ///
    /// # Arguments
    /// * `vs_inv` - VS 信号反相
    /// * `hs_inv` - HS 信号反相
    /// * `hdr_inv` - HDR 信号反相
    pub fn set_sublvds_polarity(
        &mut self,
        vs_inv: bool,
        hs_inv: bool,
        hdr_inv: bool,
    ) -> Result<(), ViError> {
        if !self.initialized {
            return Err(ViError::NotInitialized);
        }

        // VI2 不支持 Sub-LVDS
        if self.devno == ViDevno::Vi2 {
            return Err(ViError::InvalidInputMode);
        }

        unsafe {
            let regs = self.regs();

            regs.reg_00.modify(
                if vs_inv {
                    regs::REG_00::REG_SUBLVDS_VS_INV::SET
                } else {
                    regs::REG_00::REG_SUBLVDS_VS_INV::CLEAR
                } + if hs_inv {
                    regs::REG_00::REG_SUBLVDS_HS_INV::SET
                } else {
                    regs::REG_00::REG_SUBLVDS_HS_INV::CLEAR
                } + if hdr_inv {
                    regs::REG_00::REG_SUBLVDS_HDR_INV::SET
                } else {
                    regs::REG_00::REG_SUBLVDS_HDR_INV::CLEAR
                },
            );
        }

        Ok(())
    }

    /// 应用完整的设备属性配置
    ///
    /// # Arguments
    /// * `attr` - VI 设备属性
    ///
    /// # Returns
    /// * `Ok(())` - 配置成功
    /// * `Err(ViError)` - 配置失败
    pub fn set_dev_attr(&mut self, attr: &ViDevAttr) -> Result<(), ViError> {
        if !self.initialized {
            self.init()?;
        }

        // 验证设备编号
        if attr.devno != self.devno {
            return Err(ViError::InvalidDevno);
        }

        // 设置 MAC 模式
        self.set_mac_mode(attr.mac_mode)?;

        // 设置输入模式
        self.set_input_mode(attr.input_mode, attr.input_source, attr.clk_inv)?;

        // 配置 TTL 接口
        if let Some(ttl_config) = &attr.ttl_config {
            self.configure_ttl(ttl_config)?;
        }

        // 配置 BT 接口
        if let Some(bt_config) = &attr.bt_config {
            self.configure_bt(bt_config)?;
        }

        // 配置 Sub-LVDS 接口
        if let Some(sublvds_config) = &attr.sublvds_config {
            self.configure_sublvds(sublvds_config)?;
        }

        // 配置 HiSPi 接口
        if let Some(hispi_config) = &attr.hispi_config {
            self.configure_hispi(hispi_config)?;
        }

        // 配置 HDR
        self.set_hdr(attr.hdr_config)?;

        // 配置 BLC
        self.set_blc(attr.blc_config)?;

        // 配置裁剪
        self.set_crop(attr.crop_region)?;

        // 保存配置
        self.config = Some(*attr);

        Ok(())
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建默认的 BT.656 配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
///
/// # Returns
/// BT.656 配置
pub fn default_bt656_config(width: u16, height: u16) -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt656_9bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
        sync_code: BtSyncCode::default(),
    }
}

/// 创建默认的 BT.1120 配置
///
/// # Arguments
/// * `width` - 图像宽度
/// * `height` - 图像高度
///
/// # Returns
/// BT.1120 配置
pub fn default_bt1120_config(width: u16, height: u16) -> BtConfig {
    BtConfig {
        enable: true,
        format: BtFormat::Bt1120_17bit,
        demux_ch: BtDemuxChannel::None,
        ddr_mode: false,
        vs_inv: false,
        hs_inv: false,
        vs_as_vde: false,
        hs_as_hde: false,
        img_size: ImageSize::new(width, height),
        blanking: BlankingConfig::default(),
        sync_code: BtSyncCode::default(),
    }
}
