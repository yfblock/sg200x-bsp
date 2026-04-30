//! # CV1811 SD 卡驱动库
//!
//! 本库提供了 CV1811 芯片 SD 卡控制器的驱动实现。
//! 支持 SD 卡的初始化、块读取和块写入操作。
//!
//! ## 功能特性
//! - SD 卡检测和初始化
//! - 单块读取 (CMD17)
//! - 单块写入 (CMD24)
//! - 支持 1.8V/3.0V/3.3V 电压切换
//! - 支持 4 位总线宽度
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::sdmmc::{Sdmmc, PowerLevel};
//!
//! // 创建 SDMMC 驱动实例
//! let mut sdmmc = unsafe { Sdmmc::new() };
//!
//! // 初始化 SD 卡
//! sdmmc.init()?;
//!
//! // 读取块
//! let mut buf = [0u8; 512];
//! sdmmc.read_block(0, &mut buf)?;
//!
//! // 写入块
//! sdmmc.write_block(0, &buf)?;
//! ```

mod consts;
mod utils;

pub use consts::{BLOCK_SIZE, CmdError, CommandType, PowerLevel, ResponseType};
pub use consts::{SD_DRIVER_BASE, SdmmcRegisters, TOP_BASE};

use consts::*;
use core::cell::Cell;
use utils::{delay, delay_long, delay_short};

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::pinmux::{
    DriveStrength, FMUX_SD0_CLK, FMUX_SD0_CMD, FMUX_SD0_D0, FMUX_SD0_D1, FMUX_SD0_D2, FMUX_SD0_D3,
    IoConfig, Pinmux,
};

extern crate alloc;

/// SD 卡基本信息（在 [`Sdmmc::init`] 之后填充，可由
/// [`Sdmmc::card_info`] 读取）。
#[derive(Debug, Clone, Copy, Default)]
pub struct SdCardInfo {
    /// 卡相对地址 (RCA)：CMD3 返回的高 16 位
    pub rca: u32,
    /// CSD 寄存器原始内容（按 SDHCI Response 寄存器排列：
    /// `csd_raw[0]` = response0 = CSD\[39:8\]，
    /// `csd_raw[1]` = response1 = CSD\[71:40\]，
    /// `csd_raw[2]` = response2 = CSD\[103:72\]，
    /// `csd_raw[3]` = response3 = CSD\[135:104\]，
    /// 高 8 位为 R2 起始位/保留位，CSD\[7:0\]（CRC+stop）已被硬件丢弃）
    pub csd_raw: [u32; 4],
    /// CSD_STRUCTURE 字段：0=v1.0(SDSC)，1=v2.0(SDHC/SDXC)，2=v3.0(SDUC)
    pub csd_structure: u8,
    /// SD 卡容量（字节）；解析失败或未初始化时为 0
    pub capacity_bytes: u64,
}

/// SDMMC 驱动结构体
///
/// 提供对 SD 卡控制器的访问接口
pub struct Sdmmc {
    /// SDMMC 寄存器组引用
    regs: &'static SdmmcRegisters,
    /// TOP 寄存器组引用
    top_regs: &'static TopRegisters,
    /// Pinmux 驱动 (可选)
    pinmux: Option<Pinmux>,
    /// 缓存 [`SdCardInfo`]：在 [`Sdmmc::init`] 中填充
    card_info: Cell<SdCardInfo>,
}

impl Sdmmc {
    /// 创建新的 SDMMC 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new() -> Self {
        unsafe {
            Self {
                regs: &*(SD_DRIVER_BASE as *const SdmmcRegisters),
                top_regs: &*(TOP_BASE as *const TopRegisters),
                pinmux: Some(Pinmux::new()),
                card_info: Cell::new(SdCardInfo::default()),
            }
        }
    }

    /// 从指定基地址创建 SDMMC 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保所有基地址有效且可访问
    pub unsafe fn from_base_addresses(sd_base: usize, top_base: usize) -> Self {
        unsafe {
            Self {
                regs: &*(sd_base as *const SdmmcRegisters),
                top_regs: &*(top_base as *const TopRegisters),
                pinmux: None,
                card_info: Cell::new(SdCardInfo::default()),
            }
        }
    }

    /// 设置 Pinmux 驱动
    pub fn set_pinmux(&mut self, pinmux: Pinmux) {
        self.pinmux = Some(pinmux);
    }

    /// 检测 SD 卡是否已插入
    pub fn is_card_inserted(&self) -> bool {
        self.regs.present_state.is_set(PRESENT_STATE::CARD_INSERTED)
    }

    pub fn read_block_single(&self, block_id: u32, buffer: &mut [u8]) -> Result<(), CmdError> {
        assert_eq!(buffer.len(), 512);
        // 发送 CMD17 读取单块命令
        self.cmd_transfer(CommandType::CMD(17), block_id, 1, false)?;

        // 从数据缓冲区读取数据
        self.read_buff(buffer)?;

        // 等待数据传输完成
        let res = self.wait_for_xfer_done();
        // 清除中断状态寄存器
        self.regs
            .norm_and_err_int_sts
            .set(self.regs.norm_and_err_int_sts.get());
        res
    }

    /// 从 SD 卡读取一个数据块 (512 字节)
    ///
    /// # 参数
    /// - `block_id`: 要读取的块号 (逻辑块地址 LBA)
    /// - `data`: 用于存储读取数据的缓冲区，必须为 512 字节
    ///
    /// # 返回值
    /// - `Ok(())`: 读取成功
    /// - `Err(CmdError)`: 读取失败，返回错误类型
    pub fn read_block(&self, block_id: u32, data: &mut [u8]) -> Result<(), CmdError> {
        let blk_cnt = data.len() / BLOCK_SIZE;
        log::debug!("reading {} blocks", blk_cnt);
        // 发送 CMD17 读取单块命令
        self.cmd_transfer(CommandType::CMD(18), block_id, blk_cnt as _, false)?;
        log::debug!(
            "blk cnt: {}",
            self.regs.blk_size_and_cnt.read(BLK_SIZE_AND_CNT::BLK_CNT)
        );
        // 从数据缓冲区读取数据
        self.read_buff(data)?;
        // 等待数据传输完成
        let res = self.wait_for_xfer_done();
        // 清除中断状态寄存器
        self.regs
            .norm_and_err_int_sts
            .set(self.regs.norm_and_err_int_sts.get());
        res
    }

    /// 向 SD 卡写入一个数据块 (512 字节)
    ///
    /// # 参数
    /// - `block_id`: 要写入的块号 (逻辑块地址 LBA)
    /// - `data`: 要写入的数据，必须为 512 字节
    ///
    /// # 返回值
    /// - `Ok(())`: 写入成功
    /// - `Err(CmdError)`: 写入失败，返回错误类型
    pub fn write_block(&self, block_id: u32, data: &[u8]) -> Result<(), CmdError> {
        // 发送 CMD24 写入单块命令
        self.cmd_transfer(CommandType::CMD(24), block_id, 1, false)?;
        // 将数据写入数据缓冲区
        self.write_buff(data)?;
        // 等待数据传输完成
        let res = self.wait_for_xfer_done();
        // 清除中断状态寄存器
        self.regs
            .norm_and_err_int_sts
            .set(self.regs.norm_and_err_int_sts.get());
        res
    }

    /// 重置 SD 卡控制器配置
    ///
    /// 执行以下操作：
    /// 1. 关闭总线电源
    /// 2. 软件复位控制器
    /// 3. 重新启用电源 (3.3V)
    /// 4. 配置数据传输宽度为 4 位
    pub fn reset_config(&self) {
        // 关闭总线电源
        self.power_config(PowerLevel::Close);

        // 软件复位: 清除 DAT 线、CMD 线和全部复位位
        self.regs.clk_ctl.modify(
            CLK_CTL::SW_RST_DAT::CLEAR + CLK_CTL::SW_RST_CMD::CLEAR + CLK_CTL::SW_RST_ALL::CLEAR,
        );

        // 等待复位完成
        delay(0x1000);

        // 重新启用电源，设置为 3.3V
        self.power_config(PowerLevel::V33);

        // 配置 4 位数据宽度
        self.regs
            .host_ctl1_pwr_bg_wup
            .modify(HOST_CTL1_PWR_BG_WUP::DAT_XFER_WIDTH::Width4Bit);
    }

    /// 等待命令完成
    ///
    /// 轮询中断状态寄存器，等待命令完成或错误发生
    pub fn wait_for_cmd_done(&self) -> Result<(), CmdError> {
        loop {
            let sts = self.regs.norm_and_err_int_sts.extract();
            // 检查是否发生错误中断
            if sts.is_set(NORM_AND_ERR_INT_STS::ERR_INT) {
                // 清除错误中断标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::ERR_INT::SET);
                return Err(CmdError::IntError);
            }
            // 检查命令是否完成
            if sts.is_set(NORM_AND_ERR_INT_STS::CMD_CMPL) {
                // 清除命令完成标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::CMD_CMPL::SET);
                return Ok(());
            }
            // 短暂延时，避免过度占用 CPU
            delay(1);
        }
    }

    /// 等待数据传输完成
    ///
    /// 轮询中断状态寄存器，等待数据传输完成或错误发生
    pub fn wait_for_xfer_done(&self) -> Result<(), CmdError> {
        loop {
            let sts = self.regs.norm_and_err_int_sts.extract();
            // 检查传输是否完成
            if sts.is_set(NORM_AND_ERR_INT_STS::XFER_CMPL) {
                // 清除传输完成标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::XFER_CMPL::SET);
                return Ok(());
            }
            // 检查是否发生错误中断
            if sts.is_set(NORM_AND_ERR_INT_STS::ERR_INT) {
                // 清除错误中断标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::ERR_INT::SET);
                return Err(CmdError::IntError);
            }
            // 短暂延时
            delay(1);
        }
    }

    /// 发送 SD 命令并等待响应
    ///
    /// # 参数
    /// - `cmd`: 命令类型 (CMD 或 ACMD)
    /// - `arg`: 命令参数 (32位)
    /// - `blk_cnt`: 数据块数量 (0 表示无数据传输)
    ///
    /// # 返回值
    /// - `Ok(())`: 命令执行成功
    /// - `Err(CmdError)`: 命令执行失败
    pub fn cmd_transfer(
        &self,
        cmd: CommandType,
        arg: u32,
        blk_cnt: u32,
        dma: bool,
    ) -> Result<(), CmdError> {
        // 等待命令线和数据线空闲
        while self.regs.present_state.any_matching_bits_set(
            PRESENT_STATE::CMD_INHIBIT::SET + PRESENT_STATE::CMD_INHIBIT_DAT::SET,
        ) {
            core::hint::spin_loop();
        }

        // 构建命令寄存器值
        let mut xfer_mode = XFER_MODE_AND_CMD::CMD_IDX.val(cmd.num() as u32);

        // 根据命令类型设置数据传输标志
        match cmd {
            CommandType::CMD(17) | CommandType::ACMD(51) => {
                // 读取命令: 有数据，读方向
                xfer_mode +=
                    XFER_MODE_AND_CMD::DATA_PRESENT::SET + XFER_MODE_AND_CMD::DAT_XFER_DIR::Read;
            }
            CommandType::CMD(18) => {
                // 块大小 0x200 = 512 字节
                self.regs.blk_size_and_cnt.write(
                    BLK_SIZE_AND_CNT::XFER_BLK_SIZE.val(BLOCK_SIZE as u32)
                        + BLK_SIZE_AND_CNT::BLK_CNT.val(blk_cnt),
                );
                // 读取命令: 有数据，读方向
                xfer_mode += XFER_MODE_AND_CMD::DATA_PRESENT::SET
                    + XFER_MODE_AND_CMD::DAT_XFER_DIR::Read
                    + XFER_MODE_AND_CMD::BLK_CNT_EN::SET
                    + XFER_MODE_AND_CMD::AUTO_CMD_EN::AutoCmd12
                    + XFER_MODE_AND_CMD::MULTI_BLK_SEL::SET;
            }
            CommandType::CMD(24) => {
                // 写入命令: 有数据，写方向
                xfer_mode +=
                    XFER_MODE_AND_CMD::DATA_PRESENT::SET + XFER_MODE_AND_CMD::DAT_XFER_DIR::Write;
            }
            _ => {}
        }

        log::debug!(
            "blk cnt: {}",
            self.regs.blk_size_and_cnt.read(BLK_SIZE_AND_CNT::BLK_CNT)
        );

        if dma {
            xfer_mode += XFER_MODE_AND_CMD::DMA_EN::SET;
        }

        // 根据命令类型设置响应格式和校验标志
        match cmd {
            // R1 响应: 48 位，带 CRC 和索引校验
            CommandType::CMD(7)
            | CommandType::CMD(8)
            | CommandType::CMD(16)
            | CommandType::CMD(17)
            | CommandType::CMD(18)
            | CommandType::CMD(24)
            | CommandType::ACMD(6)
            | CommandType::ACMD(42)
            | CommandType::ACMD(51) => {
                xfer_mode += XFER_MODE_AND_CMD::RESP_TYPE::Response48
                    + XFER_MODE_AND_CMD::CMD_CRC_CHK_EN::SET
                    + XFER_MODE_AND_CMD::CMD_IDX_CHK_EN::SET;
            }
            // R2 响应: 136 位，带 CRC 校验 (CID/CSD)
            CommandType::CMD(2) | CommandType::CMD(9) => {
                xfer_mode += XFER_MODE_AND_CMD::RESP_TYPE::Response136
                    + XFER_MODE_AND_CMD::CMD_CRC_CHK_EN::SET;
            }
            // R3 响应: 48 位，无校验 (OCR)
            CommandType::ACMD(41) | CommandType::CMD(58) => {
                xfer_mode += XFER_MODE_AND_CMD::RESP_TYPE::Response48;
            }
            // R6 响应: 48 位带忙，带 CRC 和索引校验 (RCA)
            CommandType::CMD(3) => {
                xfer_mode += XFER_MODE_AND_CMD::RESP_TYPE::Response48Busy
                    + XFER_MODE_AND_CMD::CMD_CRC_CHK_EN::SET
                    + XFER_MODE_AND_CMD::CMD_IDX_CHK_EN::SET;
            }
            _ => {}
        }

        // 设置超时时间 (0xe 表示最大超时)
        self.regs.clk_ctl.modify(CLK_CTL::TOUT_CNT::TMCLK2p27);

        // 清除所有中断状态
        self.regs.norm_and_err_int_sts.set(0xF3FFFFFF);
        log::debug!("read cmd transfer: {:#x?}", self.regs.norm_and_err_int_sts_en.get());

        // 写入命令参数
        self.regs.argument1.set(arg);

        // 写入命令和传输模式寄存器，触发命令发送
        self.regs.xfer_mode_and_cmd.write(xfer_mode);

        // 等待命令完成
        self.wait_for_cmd_done()?;

        // 读取响应寄存器 (必须读取，否则可能导致问题)
        let resp0 = self.regs.response0.get();
        let resp1 = self.regs.response1.get();
        let resp2 = self.regs.response2.get();
        let resp3 = self.regs.response3.get();

        log::trace!(
            "resp0: {:#x} resp1: {:#x} resp2: {:#x} resp3: {:#x}",
            resp0,
            resp1,
            resp2,
            resp3
        );

        Ok(())
    }

    /// 获取响应寄存器 0 的值
    pub fn get_response0(&self) -> u32 {
        self.regs.response0.get()
    }

    /// 返回 [`Sdmmc::init`] 期间缓存的 SD 卡基本信息（含 RCA / CSD / 容量）。
    ///
    /// 在 `init()` 成功调用之前，返回值为 [`SdCardInfo::default`]
    /// （`capacity_bytes == 0`）。
    pub fn card_info(&self) -> SdCardInfo {
        self.card_info.get()
    }

    /// 便捷接口：返回 SD 卡的容量（字节）。
    /// 若尚未初始化或 CSD 解析失败，则返回 0。
    pub fn card_capacity_bytes(&self) -> u64 {
        self.card_info.get().capacity_bytes
    }

    /// 便捷接口：返回 SD 卡的容量（块数，每块 [`BLOCK_SIZE`] 字节）。
    pub fn card_capacity_blocks(&self) -> u64 {
        self.card_info.get().capacity_bytes / BLOCK_SIZE as u64
    }

    /// 通过 CMD9 重新读取 CSD 寄存器并刷新缓存的 [`SdCardInfo`]。
    ///
    /// 仅当卡已经处于 stand-by 状态时才能直接发送 CMD9；
    /// 若卡当前在 transfer 状态（已被 CMD7 选中），调用方需要先发送
    /// `CMD7(arg=0)` 取消选中再调用本方法（本接口未做状态机切换，
    /// 主要用于诊断 / 在 [`Sdmmc::init`] 失败后再单独尝试一次）。
    pub fn refresh_csd(&self) -> Result<SdCardInfo, CmdError> {
        let rca = self.card_info.get().rca;
        self.cmd_transfer(CommandType::CMD(9), rca, 0, false)?;
        let csd_raw = [
            self.regs.response0.get(),
            self.regs.response1.get(),
            self.regs.response2.get(),
            self.regs.response3.get(),
        ];
        let info = parse_sd_card_info(rca, csd_raw);
        self.card_info.set(info);
        Ok(info)
    }

    /// 配置 SD 卡引脚 (PAD) 设置
    ///
    /// 初始化 SDIO0 接口的所有引脚配置
    pub fn pad_settings(&self) {
        // 配置 SD 电源开关控制寄存器
        self.top_regs
            .sd_pwrsw_ctrl
            .write(TOP_SD_PWRSW_CTRL::PWRSW_CTRL.val(0x9));

        if let Some(ref pinmux) = self.pinmux {
            // 配置引脚功能为 SDIO0
            pinmux.set_sd0_clk_func(FMUX_SD0_CLK::FSEL::Value::SDIO0_CLK);
            pinmux.set_sd0_cmd_func(FMUX_SD0_CMD::FSEL::Value::SDIO0_CMD);
            pinmux.set_sd0_d0_func(FMUX_SD0_D0::FSEL::Value::SDIO0_D0);
            pinmux.set_sd0_d1_func(FMUX_SD0_D1::FSEL::Value::SDIO0_D1);
            pinmux.set_sd0_d2_func(FMUX_SD0_D2::FSEL::Value::SDIO0_D2);
            pinmux.set_sd0_d3_func(FMUX_SD0_D3::FSEL::Value::SDIO0_D3);

            // 配置 IO 电气特性
            let ioblk_g10 = pinmux.ioblk_g10();

            // 配置驱动强度
            ioblk_g10
                .sd0_clk
                .set_drive_strength(DriveStrength::Level2 as u8);
            ioblk_g10
                .sd0_cmd
                .set_drive_strength(DriveStrength::Level2 as u8);
            ioblk_g10
                .sd0_d0
                .set_drive_strength(DriveStrength::Level2 as u8);
            ioblk_g10
                .sd0_d1
                .set_drive_strength(DriveStrength::Level2 as u8);
            ioblk_g10
                .sd0_d2
                .set_drive_strength(DriveStrength::Level2 as u8);
            ioblk_g10
                .sd0_d3
                .set_drive_strength(DriveStrength::Level2 as u8);

            // 配置上拉
            ioblk_g10.sd0_cmd.set_pull_up(true);
            ioblk_g10.sd0_d0.set_pull_up(true);
            ioblk_g10.sd0_d1.set_pull_up(true);
            ioblk_g10.sd0_d2.set_pull_up(true);
            ioblk_g10.sd0_d3.set_pull_up(true);
        }
    }

    /// 配置 SD 卡总线电源
    ///
    /// # 参数
    /// - `level`: 目标电压等级
    pub fn power_config(&self, level: PowerLevel) {
        match level {
            PowerLevel::V33 => {
                self.regs.host_ctl1_pwr_bg_wup.modify(
                    HOST_CTL1_PWR_BG_WUP::SD_BUS_VOL_SEL::V33
                        + HOST_CTL1_PWR_BG_WUP::SD_BUS_PWR::SET,
                );
                self.top_regs
                    .sd_pwrsw_ctrl
                    .write(TOP_SD_PWRSW_CTRL::PWRSW_CTRL.val(0x9));
            }
            PowerLevel::V30 => {
                self.regs.host_ctl1_pwr_bg_wup.modify(
                    HOST_CTL1_PWR_BG_WUP::SD_BUS_VOL_SEL::V30
                        + HOST_CTL1_PWR_BG_WUP::SD_BUS_PWR::SET,
                );
                self.top_regs
                    .sd_pwrsw_ctrl
                    .write(TOP_SD_PWRSW_CTRL::PWRSW_CTRL.val(0x9));
            }
            PowerLevel::V18 => {
                self.regs.host_ctl1_pwr_bg_wup.modify(
                    HOST_CTL1_PWR_BG_WUP::SD_BUS_VOL_SEL::V18
                        + HOST_CTL1_PWR_BG_WUP::SD_BUS_PWR::SET,
                );
                // 1.8V 模式需要额外配置
                self.top_regs
                    .sd_pwrsw_ctrl
                    .write(TOP_SD_PWRSW_CTRL::PWRSW_CTRL.val(0xd));

                // 配置时钟引脚 PAD 的驱动能力
                if let Some(ref pinmux) = self.pinmux {
                    pinmux
                        .ioblk_g10()
                        .sd0_clk
                        .set_drive_strength(DriveStrength::Level7 as u8);
                }
            }
            PowerLevel::Close => {
                self.regs
                    .host_ctl1_pwr_bg_wup
                    .modify(HOST_CTL1_PWR_BG_WUP::SD_BUS_PWR::CLEAR);
            }
        }

        // 等待电源稳定
        delay_long();
    }

    /// 设置 SD 卡时钟频率
    ///
    /// # 参数
    /// - `divider`: 时钟分频系数
    ///
    /// # 说明
    /// 输出时钟频率 = 内部时钟频率 / (2 × divider)
    pub fn set_clock(&self, divider: u8) {
        // 先禁用 SD 时钟输出
        self.regs.clk_ctl.modify(CLK_CTL::SD_CLK_EN::CLEAR);

        // 设置时钟分频系数
        self.regs
            .clk_ctl
            .modify(CLK_CTL::FREQ_SEL.val(divider as u32));

        // 使能内部时钟
        self.regs.clk_ctl.modify(CLK_CTL::INT_CLK_EN::SET);

        // 等待内部时钟稳定
        loop {
            if self.regs.clk_ctl.is_set(CLK_CTL::INT_CLK_STABLE) {
                break;
            }
            delay_short();
        }

        // 使能 SD 时钟输出
        self.regs.clk_ctl.modify(CLK_CTL::SD_CLK_EN::SET);

        // 等待时钟稳定
        delay_long();
    }

    /// 关闭 SD 卡时钟
    ///
    /// 在命令线和数据线都空闲时关闭时钟以节省功耗
    pub fn close_clock(&self) {
        // 检查命令线和数据线是否空闲
        if !self.regs.present_state.is_set(PRESENT_STATE::CMD_INHIBIT)
            && !self
                .regs
                .present_state
                .is_set(PRESENT_STATE::DAT_LINE_ACTIVE)
        {
            // 禁用 SD 时钟输出
            self.regs.clk_ctl.modify(CLK_CTL::SD_CLK_EN::CLEAR);
        }

        // 等待时钟关闭完成
        delay(0x100_0000);
    }

    /// 控制 SD 时钟使能
    ///
    /// # 参数
    /// - `en`: true 表示使能时钟，false 表示禁用时钟
    pub fn clk_en(&self, en: bool) {
        if en {
            self.regs.clk_ctl.modify(CLK_CTL::SD_CLK_EN::SET);
        } else {
            self.regs.clk_ctl.modify(CLK_CTL::SD_CLK_EN::CLEAR);
        }
    }

    /// 从数据缓冲区读取数据
    ///
    /// # 参数
    /// - `data`: 用于存储读取数据的缓冲区，必须为 512 字节
    fn read_buff(&self, data: &mut [u8]) -> Result<(), CmdError> {
        // assert!(data.len() == BLOCK_SIZE);
        assert!(data.len() % BLOCK_SIZE == 0);

        for block_buf in data.chunks_exact_mut(BLOCK_SIZE) {
            // 等待缓冲区读就绪
            loop {
                let sts = self.regs.norm_and_err_int_sts.extract();
                if sts.is_set(NORM_AND_ERR_INT_STS::BUF_RRDY) {
                    // 清除缓冲区读就绪标志
                    self.regs
                        .norm_and_err_int_sts
                        .write(NORM_AND_ERR_INT_STS::BUF_RRDY::SET);
                    break;
                }
                if sts.is_set(NORM_AND_ERR_INT_STS::ERR_INT) {
                    // 清除错误中断标志
                    self.regs
                        .norm_and_err_int_sts
                        .write(NORM_AND_ERR_INT_STS::ERR_INT::SET);
                    return Err(CmdError::IntError);
                }
                delay(1);
            }

            // 从数据端口寄存器读取数据
            // 每次读取 32 位 (4 字节)，共读取 128 次
            for chunk in block_buf.chunks_exact_mut(4) {
                let value = self.regs.buf_data_port.get();
                chunk.copy_from_slice(&value.to_le_bytes());
                delay(1);
            }
        }

        Ok(())
    }

    /// 向数据缓冲区写入数据
    ///
    /// # 参数
    /// - `data`: 要写入的数据，必须为 512 字节
    fn write_buff(&self, data: &[u8]) -> Result<(), CmdError> {
        assert!(data.len() == BLOCK_SIZE);

        // 等待缓冲区写就绪
        loop {
            let sts = self.regs.norm_and_err_int_sts.extract();
            if sts.is_set(NORM_AND_ERR_INT_STS::BUF_WRDY) {
                // 清除缓冲区写就绪标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::BUF_WRDY::SET);
                break;
            }
            if sts.is_set(NORM_AND_ERR_INT_STS::ERR_INT) {
                // 清除错误中断标志
                self.regs
                    .norm_and_err_int_sts
                    .write(NORM_AND_ERR_INT_STS::ERR_INT::SET);
                return Err(CmdError::IntError);
            }
            delay(1);
        }

        // 向数据端口寄存器写入数据
        // 每次写入 32 位 (4 字节)，共写入 128 次
        for chunk in data.chunks_exact(4) {
            let value = u32::from_le_bytes(chunk.try_into().unwrap());
            self.regs.buf_data_port.set(value);
            delay(1);
        }
        Ok(())
    }

    /// 初始化 SD 卡
    ///
    /// 执行完整的 SD 卡初始化流程：
    /// 1. 检测 SD 卡是否插入
    /// 2. 配置引脚和控制器
    /// 3. 设置电源和时钟
    /// 4. 执行 SD 卡初始化命令序列
    /// 5. 配置 4 位总线宽度
    ///
    /// # 返回值
    /// - `Ok(())`: 初始化成功
    /// - `Err(CmdError)`: 初始化失败
    pub fn init(&self) -> Result<(), CmdError> {
        // 检测 SD 卡是否插入
        if !self.is_card_inserted() {
            log::warn!("SD card not inserted");
            return Ok(());
        }

        // 重置控制器配置
        self.reset_config();

        // 设置电源为 1.8V (UHS-I 模式)
        self.power_config(PowerLevel::V18);

        // 设置时钟分频为 4 (低速初始化)
        self.set_clock(4);

        // SD 卡初始化命令序列
        // CMD0: GO_IDLE_STATE - 复位所有卡到空闲状态
        self.cmd_transfer(CommandType::CMD(0), 0, 0, false)?;

        // CMD8: SEND_IF_COND - 发送接口条件
        // 参数 0x1aa: VHS=0x1 (2.7-3.6V), check pattern=0xaa
        self.cmd_transfer(CommandType::CMD(8), 0x1aa, 0, false)?;
        // 循环发送 ACMD41，等待卡初始化完成
        loop {
            // CMD55: APP_CMD - 表示下一条命令是应用命令
            self.cmd_transfer(CommandType::CMD(55), 0, 0, false)?;

            // ACMD41: SD_SEND_OP_COND - 发送操作条件
            // 参数说明:
            // - 0x4000_0000: HCS (高容量支持) 位
            // - 0x0030_0000: 电压窗口 (3.2-3.4V)
            // - 0x1FF << 15: 电压窗口 (2.7-3.6V)
            self.cmd_transfer(
                CommandType::ACMD(41),
                0x4000_0000 | 0x0030_0000 | (0x1FF << 15),
                0,
                false,
            )?;

            // 检查响应的 bit31 (忙标志)
            // 当 bit31 = 1 时，卡初始化完成
            if self.get_response0() >> 31 == 1 {
                break;
            }

            // 等待一段时间后重试
            delay(0x100_0000);
        }

        // CMD2: ALL_SEND_CID - 获取卡识别号
        self.cmd_transfer(CommandType::CMD(2), 0, 0, false)?;

        // CMD3: SEND_RELATIVE_ADDR - 获取相对地址 (RCA)
        self.cmd_transfer(CommandType::CMD(3), 0, 0, false)?;

        // 从响应中提取 RCA (高 16 位)
        let rca = self.get_response0() & 0xffff0000;

        // CMD9: SEND_CSD - 获取卡特定数据
        self.cmd_transfer(CommandType::CMD(9), rca, 0, false)?;

        // 立即把 R2 响应保存到缓存：CMD7 之后再读 response 寄存器内容会被
        // 后续命令覆盖；此处保存的 [u32; 4] 即原始 CSD 数据（不含 CRC）。
        let csd_raw = [
            self.regs.response0.get(),
            self.regs.response1.get(),
            self.regs.response2.get(),
            self.regs.response3.get(),
        ];
        let info = parse_sd_card_info(rca, csd_raw);
        log::debug!(
            "sdmmc CSD raw: r0={:#010x} r1={:#010x} r2={:#010x} r3={:#010x}",
            csd_raw[0],
            csd_raw[1],
            csd_raw[2],
            csd_raw[3],
        );
        log::debug!(
            "sdmmc CSD struct=v{}.0 capacity={} bytes",
            info.csd_structure as u32 + 1,
            info.capacity_bytes
        );
        self.card_info.set(info);

        // CMD7: SELECT_CARD - 选择卡进入传输状态
        self.cmd_transfer(CommandType::CMD(7), rca, 0, false)?;

        // 切换到 4 位总线宽度
        // CMD55: 应用命令前缀
        self.cmd_transfer(CommandType::CMD(55), rca, 0, false)?;
        // ACMD6: SET_BUS_WIDTH - 参数 2 表示 4 位宽度
        self.cmd_transfer(CommandType::ACMD(6), 2, 0, false)?;

        // 配置主机控制器为 4 位模式
        self.regs
            .host_ctl1_pwr_bg_wup
            .modify(HOST_CTL1_PWR_BG_WUP::DAT_XFER_WIDTH::Width4Bit);

        log::debug!("sdmmc initialize done!");

        // 关闭时钟 (节省功耗，需要时再开启)
        self.clk_en(false);

        Ok(())
    }
}

/// 初始化 SD 卡
pub fn init() -> Result<Sdmmc, CmdError> {
    let sdmmc = unsafe { Sdmmc::new() };
    sdmmc.init()?;
    Ok(sdmmc)
}

/// 根据 R2 响应寄存器（response0..response3）解析出 SD 卡的容量等基本信息。
///
/// SDHCI 把 R2 响应的 \[135:8\] 直接放进 RESP\_REG\[127:0\]：
///
/// ```text
/// response3[31:0] = R2[135:104] = (start+trans+rsv) | CSD[127:104]
/// response2[31:0] = R2[103:72]  = CSD[103:72]
/// response1[31:0] = R2[71:40]   = CSD[71:40]
/// response0[31:0] = R2[39:8]    = CSD[39:8]
/// ```
///
/// 即 `RESP_REG[i] = CSD[i + 8]`（CSD\[7:0\] 的 CRC + stop bit 已被硬件丢弃）。
/// 由此推出本函数中各字段的取位方式。
///
/// 三种 CSD 结构：
/// - **v1.0 (SDSC)**：容量 = `(C_SIZE + 1) * 2^(C_SIZE_MULT + 2) * 2^READ_BL_LEN`
/// - **v2.0 (SDHC/SDXC)**：容量 = `(C_SIZE + 1) * 512KiB`，C\_SIZE 为 22 bit
/// - **v3.0 (SDUC)**：容量 = `(C_SIZE + 1) * 512KiB`，C\_SIZE 为 28 bit
pub fn parse_sd_card_info(rca: u32, csd_raw: [u32; 4]) -> SdCardInfo {
    let r0 = csd_raw[0];
    let r1 = csd_raw[1];
    let r2 = csd_raw[2];
    let r3 = csd_raw[3];

    // CSD[127:126] —— RESP_REG[119:118] —— response3 bit[23:22]
    let csd_structure = ((r3 >> 22) & 0x3) as u8;

    let capacity_bytes = match csd_structure {
        0 => {
            // CSD v1.0 (SDSC)
            // READ_BL_LEN: CSD[83:80] = RESP_REG[75:72] = response2[11:8]
            let read_bl_len = (r2 >> 8) & 0xF;
            // C_SIZE: CSD[73:62] (12 bit)，跨 response2/response1
            //   高 2 bit  CSD[73:72] = response2[1:0]
            //   低 10 bit CSD[71:62] = response1[31:22]
            let c_size = ((r2 & 0x3) << 10) | ((r1 >> 22) & 0x3FF);
            // C_SIZE_MULT: CSD[49:47] = response1[9:7]
            let c_size_mult = (r1 >> 7) & 0x7;
            let mult = 1u64 << (c_size_mult + 2);
            let blocknr = (c_size as u64 + 1) * mult;
            let block_len = 1u64 << read_bl_len;
            blocknr * block_len
        }
        1 => {
            // CSD v2.0 (SDHC / SDXC)
            // C_SIZE: CSD[69:48] = response1[29:8]，22 bit
            let c_size = (r1 >> 8) & 0x3F_FFFF;
            (c_size as u64 + 1) * 512 * 1024
        }
        2 => {
            // CSD v3.0 (SDUC)
            // C_SIZE: CSD[75:48] (28 bit)
            //   高 4 bit  CSD[75:72] = response2[3:0]
            //   低 24 bit CSD[71:48] = response1[31:8]
            let c_size = ((r2 & 0xF) << 24) | ((r1 >> 8) & 0xFF_FFFF);
            (c_size as u64 + 1) * 512 * 1024
        }
        _ => 0,
    };

    let _ = r0; // 仅保留作完整存档；CSD v1/v2/v3 容量都不依赖 response0。

    SdCardInfo {
        rca,
        csd_raw,
        csd_structure,
        capacity_bytes,
    }
}
