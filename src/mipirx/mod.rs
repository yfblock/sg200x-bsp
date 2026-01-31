//! # MIPI RX 驱动模块
//!
//! 本模块提供 SG2002 芯片 MIPI RX (Mobile Industry Processor Interface Receiver)
//! 的 Rust 驱动实现。
//!
//! ## 功能特性
//!
//! - 支持 MIPI D-PHY ver2.1
//! - 可同时支持 2 路 sensor 输入
//! - 单一 sensor 最大支持 5M (2688x1944, 2880x1620) @30fps HDR 或 @60fps 线性输入
//! - 双路 sensor 最大支持 FHD (1920x1080) @60fps HDR 或线性输入
//! - 单路最多支持 4-Lane MIPI D-PHY 接口，最大支持 1.5Gbps/Lane
//! - 单路最多支持 4-Lane Sub-LVDS/HiSPi 接口，最大支持 1.5Gbps/Lane
//! - 支持 RAW8/RAW10/RAW12/RAW16 数据类型的解析
//! - 支持 YUV422 8-bit / YUV422 10-bit 数据类型的解析
//! - 最多支持 2 帧 WDR，支持多种 WDR 时序
//! - 支持 Lane 数和 Lane 顺序可配置
//!
//! ## 模块结构
//!
//! - `regs`: 寄存器定义（使用 tock-registers）
//! - `types`: 类型定义和枚举
//! - `phy`: PHY 层配置
//! - `csi`: CSI 控制器配置
//!
//! ## 使用示例
//!
//! ### 基本 MIPI CSI 配置
//!
//! ```rust,ignore
//! use sg200x_bsp::mipirx::*;
//!
//! // 创建 MIPI RX 驱动实例
//! let mut mipirx = unsafe { MipiRx::new() };
//!
//! // 初始化
//! mipirx.init();
//!
//! // 配置设备属性
//! let attr = MipiRxDevAttr {
//!     devno: 0,
//!     sensor_mode: SensorMode::Csi,
//!     lane_mode: LaneMode::Lane4,
//!     data_type: RawDataType::Raw10,
//!     hdr_mode: HdrMode::None,
//!     lane_id: [0, 1, 2, 3],
//!     clk_lane_sel: 0,
//!     pn_swap: [false; 5],
//!     img_width: 1920,
//!     img_height: 1080,
//! };
//!
//! mipirx.configure(&attr).unwrap();
//!
//! // 使能接收
//! mipirx.enable(0);
//! ```
//!
//! ### HDR 模式配置
//!
//! ```rust,ignore
//! use sg200x_bsp::mipirx::*;
//!
//! let mut mipirx = unsafe { MipiRx::new() };
//! mipirx.init();
//!
//! // 配置 HDR VC 模式
//! let attr = MipiRxDevAttr {
//!     devno: 0,
//!     sensor_mode: SensorMode::Csi,
//!     lane_mode: LaneMode::Lane4,
//!     data_type: RawDataType::Raw10,
//!     hdr_mode: HdrMode::Vc,  // 使用 VC 模式区分长短曝光
//!     lane_id: [0, 1, 2, 3],
//!     clk_lane_sel: 0,
//!     pn_swap: [false; 5],
//!     img_width: 1920,
//!     img_height: 1080,
//! };
//!
//! mipirx.configure(&attr).unwrap();
//! ```
//!
//! ### 双路 Sensor 配置
//!
//! ```rust,ignore
//! use sg200x_bsp::mipirx::*;
//!
//! let mut mipirx = unsafe { MipiRx::new() };
//! mipirx.init();
//!
//! // 设置 PHY 为双端口模式
//! mipirx.set_phy_mode(PhyMode::Mode1C2D_1C2D);
//!
//! // 配置第一路 (2-Lane)
//! let attr0 = MipiRxDevAttr {
//!     devno: 0,
//!     sensor_mode: SensorMode::Csi,
//!     lane_mode: LaneMode::Lane2,
//!     data_type: RawDataType::Raw10,
//!     hdr_mode: HdrMode::None,
//!     lane_id: [0, 1, -1, -1],
//!     clk_lane_sel: 0,
//!     pn_swap: [false; 5],
//!     img_width: 1920,
//!     img_height: 1080,
//! };
//! mipirx.configure(&attr0).unwrap();
//!
//! // 配置第二路 (2-Lane)
//! let attr1 = MipiRxDevAttr {
//!     devno: 1,
//!     sensor_mode: SensorMode::Csi,
//!     lane_mode: LaneMode::Lane2,
//!     data_type: RawDataType::Raw10,
//!     hdr_mode: HdrMode::None,
//!     lane_id: [2, 3, -1, -1],
//!     clk_lane_sel: 1,
//!     pn_swap: [false; 5],
//!     img_width: 1280,
//!     img_height: 720,
//! };
//! mipirx.configure(&attr1).unwrap();
//! ```

#![allow(dead_code)]

pub mod regs;
pub mod types;
pub mod phy;
pub mod csi;

pub use types::*;
pub use phy::MipiRxPhy;
pub use csi::MipiRxCsi;

/// MIPI RX 驱动
///
/// 提供对 MIPI RX 模块的统一访问接口
pub struct MipiRx {
    /// PHY 驱动
    phy: MipiRxPhy,
    /// CSI 控制器数组
    csi: [Option<MipiRxCsi>; MAX_CSI_NUM],
    /// 设备状态
    enabled: [bool; MAX_CSI_NUM],
}

impl MipiRx {
    /// 创建新的 MIPI RX 驱动实例
    ///
    /// # Safety
    /// 调用者必须确保寄存器地址有效且可访问
    pub unsafe fn new() -> Self {
        unsafe {
            Self {
                phy: MipiRxPhy::new(),
                csi: [MipiRxCsi::new(0), MipiRxCsi::new(1)],
                enabled: [false; MAX_CSI_NUM],
            }
        }
    }

    /// 初始化 MIPI RX 模块
    pub fn init(&mut self) {
        // 使能 PHY 电源
        self.phy.power_on();

        // 设置默认 PHY 模式为单端口 4-Lane
        self.phy.set_phy_mode(PhyMode::Mode1C4D);

        // 初始化所有 CSI 控制器
        for csi in self.csi.iter().flatten() {
            csi.init();
        }

        // 重置状态
        self.enabled = [false; MAX_CSI_NUM];
    }

    /// 获取 PHY 驱动引用
    pub fn phy(&self) -> &MipiRxPhy {
        &self.phy
    }

    /// 获取 PHY 驱动可变引用
    pub fn phy_mut(&mut self) -> &mut MipiRxPhy {
        &mut self.phy
    }

    /// 获取 CSI 控制器引用
    ///
    /// # 参数
    /// - `devno`: 设备编号 (0 或 1)
    pub fn csi(&self, devno: u8) -> Option<&MipiRxCsi> {
        self.csi.get(devno as usize).and_then(|c| c.as_ref())
    }

    /// 获取 CSI 控制器可变引用
    ///
    /// # 参数
    /// - `devno`: 设备编号 (0 或 1)
    pub fn csi_mut(&mut self, devno: u8) -> Option<&mut MipiRxCsi> {
        self.csi.get_mut(devno as usize).and_then(|c| c.as_mut())
    }

    /// 设置 PHY 模式
    ///
    /// # 参数
    /// - `mode`: PHY 模式
    pub fn set_phy_mode(&mut self, mode: PhyMode) {
        self.phy.set_phy_mode(mode);
    }

    /// 配置 MIPI RX 设备
    ///
    /// # 参数
    /// - `attr`: 设备属性配置
    pub fn configure(&mut self, attr: &MipiRxDevAttr) -> Result<(), MipiRxError> {
        let devno = attr.devno as usize;
        if devno >= MAX_CSI_NUM {
            return Err(MipiRxError::InvalidDevno);
        }

        // 获取端口号
        let port = if devno == 0 { 0 } else { 1 };

        // 配置 PHY
        self.configure_phy(port, attr)?;

        // 配置 CSI 控制器
        if let Some(csi) = self.csi_mut(attr.devno) {
            csi.apply_dev_attr(attr)?;
        } else {
            return Err(MipiRxError::InvalidDevno);
        }

        Ok(())
    }

    /// 配置 PHY 层
    fn configure_phy(&mut self, port: u8, attr: &MipiRxDevAttr) -> Result<(), MipiRxError> {
        // 设置 Sensor 模式
        self.phy.set_sensor_mode(port, attr.sensor_mode);

        // 配置 Lane 选择
        let lane_sel = [
            if attr.lane_id[0] >= 0 { attr.lane_id[0] as u8 } else { 0 },
            if attr.lane_id[1] >= 0 { attr.lane_id[1] as u8 } else { 1 },
            if attr.lane_id[2] >= 0 { attr.lane_id[2] as u8 } else { 2 },
            if attr.lane_id[3] >= 0 { attr.lane_id[3] as u8 } else { 3 },
        ];
        self.phy.configure_csi_lane_select(port, lane_sel);

        // 配置时钟 Lane
        self.phy.configure_csi_clk_lane(
            port,
            attr.clk_lane_sel,
            attr.pn_swap[4],
            DEFAULT_CLK_PHASE,
        );

        // 配置数据 Lane PN 交换
        self.phy.configure_csi_data_pn_swap(
            port,
            [attr.pn_swap[0], attr.pn_swap[1], attr.pn_swap[2], attr.pn_swap[3]],
        );

        // 配置 Deskew Lane 使能
        let deskew_enable = match attr.lane_mode {
            LaneMode::Lane1 => DeskewLaneEnable::Lane1,
            LaneMode::Lane2 => DeskewLaneEnable::Lane2,
            LaneMode::Lane4 | LaneMode::Lane8 => DeskewLaneEnable::Lane4,
        };
        self.phy.set_deskew_lane_enable(port, deskew_enable);

        Ok(())
    }

    /// 配置 Sub-LVDS 设备
    ///
    /// # 参数
    /// - `attr`: Sub-LVDS 设备属性配置
    pub fn configure_sublvds(&mut self, attr: &SubLvdsDevAttr) -> Result<(), MipiRxError> {
        let devno = attr.devno as usize;
        if devno >= MAX_CSI_NUM {
            return Err(MipiRxError::InvalidDevno);
        }

        let port = if devno == 0 { 0 } else { 1 };

        // 设置 Sensor 模式为 Sub-LVDS
        self.phy.set_sensor_mode(port, SensorMode::SubLvds);

        // 配置 Sub-LVDS 参数
        self.phy.configure_sublvds(
            port,
            attr.bit_mode,
            attr.msb_first,
            attr.lane_enable,
        );

        // 配置同步码
        self.phy.configure_sublvds_sync_code(
            port,
            attr.sav_1st,
            attr.sav_2nd,
            attr.sav_3rd,
        );

        Ok(())
    }

    /// 使能 MIPI RX 设备
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn enable(&mut self, devno: u8) -> Result<(), MipiRxError> {
        let idx = devno as usize;
        if idx >= MAX_CSI_NUM {
            return Err(MipiRxError::InvalidDevno);
        }

        // 清除中断
        if let Some(csi) = self.csi(devno) {
            csi.clear_all_interrupts();
        }

        self.enabled[idx] = true;
        Ok(())
    }

    /// 禁用 MIPI RX 设备
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn disable(&mut self, devno: u8) -> Result<(), MipiRxError> {
        let idx = devno as usize;
        if idx >= MAX_CSI_NUM {
            return Err(MipiRxError::InvalidDevno);
        }

        self.enabled[idx] = false;
        Ok(())
    }

    /// 检查设备是否使能
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn is_enabled(&self, devno: u8) -> bool {
        self.enabled.get(devno as usize).copied().unwrap_or(false)
    }

    /// 获取 CSI 状态
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn get_status(&self, devno: u8) -> Option<CsiStatus> {
        self.csi(devno).map(|c| c.get_status())
    }

    /// 获取中断状态
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn get_interrupt_status(&self, devno: u8) -> Option<CsiInterruptStatus> {
        self.csi(devno).map(|c| c.get_interrupt_status())
    }

    /// 清除中断
    ///
    /// # 参数
    /// - `devno`: 设备编号
    /// - `mask`: 中断掩码
    pub fn clear_interrupt(&self, devno: u8, mask: u8) {
        if let Some(csi) = self.csi(devno) {
            csi.clear_interrupt(mask);
        }
    }

    /// 检查是否有错误
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn has_error(&self, devno: u8) -> bool {
        self.csi(devno).map(|c| c.has_error()).unwrap_or(false)
    }

    /// 复位 MIPI RX 设备
    ///
    /// # 参数
    /// - `devno`: 设备编号
    pub fn reset(&mut self, devno: u8) -> Result<(), MipiRxError> {
        let idx = devno as usize;
        if idx >= MAX_CSI_NUM {
            return Err(MipiRxError::InvalidDevno);
        }

        // 复位 CSI 控制器
        if let Some(csi) = self.csi(devno) {
            csi.reset();
        }

        self.enabled[idx] = false;
        Ok(())
    }

    /// 关闭 MIPI RX 模块
    pub fn shutdown(&mut self) {
        // 禁用所有设备
        for i in 0..MAX_CSI_NUM {
            let _ = self.disable(i as u8);
        }

        // 关闭 PHY 电源
        self.phy.power_off();
    }
}

impl Drop for MipiRx {
    fn drop(&mut self) {
        self.shutdown();
    }
}
