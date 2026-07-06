//! SD 命令下发与完成/传输等待。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::Sdmmc;
use super::consts::*;
use crate::utils::delay;

impl Sdmmc {
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
            CommandType::CMD(17) => {
                // 单块读取: 显式设置块大小 512 字节、块数量 1，
                // 使 read_block_single 不依赖此前残留的寄存器值。
                self.regs.blk_size_and_cnt.write(
                    BLK_SIZE_AND_CNT::XFER_BLK_SIZE.val(BLOCK_SIZE as u32)
                        + BLK_SIZE_AND_CNT::BLK_CNT.val(1),
                );
                // 读取命令: 有数据，读方向
                xfer_mode +=
                    XFER_MODE_AND_CMD::DATA_PRESENT::SET + XFER_MODE_AND_CMD::DAT_XFER_DIR::Read;
            }
            CommandType::ACMD(51) => {
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
                // 单块写入: 显式设置块大小 512 字节、块数量 1。
                self.regs.blk_size_and_cnt.write(
                    BLK_SIZE_AND_CNT::XFER_BLK_SIZE.val(BLOCK_SIZE as u32)
                        + BLK_SIZE_AND_CNT::BLK_CNT.val(1),
                );
                // 写入命令: 有数据，写方向
                xfer_mode +=
                    XFER_MODE_AND_CMD::DATA_PRESENT::SET + XFER_MODE_AND_CMD::DAT_XFER_DIR::Write;
            }
            CommandType::CMD(25) => {
                // 多块写入 (WRITE_MULTIPLE_BLOCK): 块大小 512、块数量 blk_cnt。
                self.regs.blk_size_and_cnt.write(
                    BLK_SIZE_AND_CNT::XFER_BLK_SIZE.val(BLOCK_SIZE as u32)
                        + BLK_SIZE_AND_CNT::BLK_CNT.val(blk_cnt),
                );
                // 写入命令: 有数据，写方向，多块 + AUTO_CMD12 自动停止。
                xfer_mode += XFER_MODE_AND_CMD::DATA_PRESENT::SET
                    + XFER_MODE_AND_CMD::DAT_XFER_DIR::Write
                    + XFER_MODE_AND_CMD::BLK_CNT_EN::SET
                    + XFER_MODE_AND_CMD::AUTO_CMD_EN::AutoCmd12
                    + XFER_MODE_AND_CMD::MULTI_BLK_SEL::SET;
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
            | CommandType::CMD(25)
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
        log::debug!(
            "read cmd transfer: {:#x?}",
            self.regs.norm_and_err_int_sts_en.get()
        );

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
}
