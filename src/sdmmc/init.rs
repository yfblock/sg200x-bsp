//! SD 卡初始化命令序列。

use tock_registers::interfaces::{ReadWriteable, Readable};

use super::consts::*;
use super::{Sdmmc, parse_sd_card_info};
use crate::utils::delay;

impl Sdmmc {
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

        // SD 时钟保持常开：便于初始化后立即读写，无需每次 clk_en(true)。
        self.clk_en(true);

        // 提速 SD 时钟：初始化用分频 4 (3.125 MHz)，数据传输提速到分频 2 (6.25 MHz)。
        // div=2 经测试稳定且 4KB 读取吞吐量比 div=4 提升 27%。
        // div=1 (12.5 MHz) 在此卡上信号不稳定，div=0 (25 MHz) 会导致错误重试。
        // 可通过编译环境变量 SG2002_SD_CLK_DIV 覆盖。
        let div: u8 = option_env!("SG2002_SD_CLK_DIV")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        self.set_clock(div);

        Ok(())
    }
}
