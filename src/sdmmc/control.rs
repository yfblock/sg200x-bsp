//! 控制器配置：复位、电源、时钟、引脚 (PAD)，以及卡信息访问。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::consts::*;
use super::{SdCardInfo, Sdmmc, parse_sd_card_info};
use crate::pinmux::{
    DriveStrength, FMUX_SD0_CLK, FMUX_SD0_CMD, FMUX_SD0_D0, FMUX_SD0_D1, FMUX_SD0_D2, FMUX_SD0_D3,
    IoConfig,
};
use crate::utils::{delay, delay_long, delay_short};

impl Sdmmc {
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

    /// 复位数据/命令线状态机（不重置整卡、不影响卡选中状态）。
    ///
    /// 传输出错后数据线常卡在 `CMD_INHIBIT_DAT`，必须软复位才能下发下一条
    /// 命令。兼容两种 `SW_RST` 语义：先 `SET`（标准 SDHCI 写 1 复位、自清），
    /// 再 `CLEAR`（兼容 active-low 写 0 复位的实现）。
    pub fn reset_dat_cmd_lines(&self) {
        self.regs.clk_ctl.modify(
            CLK_CTL::SW_RST_DAT::SET + CLK_CTL::SW_RST_CMD::SET,
        );
        delay(0x1000);
        self.regs.clk_ctl.modify(
            CLK_CTL::SW_RST_DAT::CLEAR + CLK_CTL::SW_RST_CMD::CLEAR,
        );
        delay(0x1000);
        // 清除残留中断状态
        self.regs.norm_and_err_int_sts.set(0xF3FFFFFF);
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

    /// 使能传输完成 + 错误中断信号（用于 ADMA2 中断驱动完成）。
    pub fn enable_xfer_irq(&self) {
        // 先清除所有中断状态
        self.regs.norm_and_err_int_sts.set(0xF3FFFFFF);
        // 使能 XFER_CMPL 和 ERR_INT 的状态位
        self.regs.norm_and_err_int_sts_en.modify(
            NORM_AND_ERR_INT_STS_EN::XFER_CMPL_EN::SET,
        );
        // 使能中断信号输出到 CPU
        self.regs.norm_and_err_int_sig_en.modify(
            NORM_AND_ERR_INT_SIG_EN::XFER_CMPL_SIG_EN::SET,
        );
    }

    /// 禁用所有中断信号。
    pub fn disable_xfer_irq(&self) {
        self.regs.norm_and_err_int_sig_en.set(0);
    }

    /// 返回 SDMMC 寄存器基址（虚拟地址），供 IRQ handler 直接访问。
    pub fn regs_base(&self) -> usize {
        self.regs as *const _ as usize
    }
}
