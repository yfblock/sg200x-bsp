//! 块写入路径：CMD24 单块、CMD25 多块（PIO），以及 CMD25 + ADMA2（DMA）。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::consts::*;
use super::{
    ADMA2_ATTR_ACT_TRAN, ADMA2_ATTR_END, ADMA2_ATTR_VALID, ADMA2_MAX_PER_DESC, Adma2Desc, Sdmmc,
};
use crate::utils::cache::dcache_clean_range;
use crate::utils::delay;

impl Sdmmc {
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

    /// 使用 CMD24 写入**单个**数据块 (512 字节)。
    ///
    /// 语义与 [`Sdmmc::write_block`] 相同，命名与 [`Sdmmc::read_block_single`] 对齐。
    pub fn write_block_single(&self, block_id: u32, data: &[u8]) -> Result<(), CmdError> {
        assert_eq!(data.len(), BLOCK_SIZE);
        self.write_block(block_id, data)
    }

    /// 使用 CMD25 (WRITE_MULTIPLE_BLOCK) **一次性**写入多个连续数据块 (PIO)。
    ///
    /// 单条命令即可完成 `data.len() / BLOCK_SIZE` 个块的写入，控制器在传输
    /// 结束后通过 AUTO_CMD12 自动发送停止命令。CPU 逐字把数据写入数据端口。
    ///
    /// # 参数
    /// - `start_block`: 起始块号 (LBA)
    /// - `data`: 源缓冲区，长度须为 [`BLOCK_SIZE`] 的整数倍且非空
    pub fn write_blocks(&self, start_block: u32, data: &[u8]) -> Result<(), CmdError> {
        assert!(!data.is_empty(), "write_blocks: buffer must not be empty");
        assert!(
            data.len().is_multiple_of(BLOCK_SIZE),
            "write_blocks: buffer length must be a multiple of BLOCK_SIZE"
        );

        let blk_cnt = data.len() / BLOCK_SIZE;

        for attempt in 0..XFER_RETRY {
            if attempt > 0 {
                self.reset_dat_cmd_lines();
                log::warn!(
                    "sdmmc write retry {}/{}: block {}, {} blocks",
                    attempt,
                    XFER_RETRY,
                    start_block,
                    blk_cnt
                );
            }
            // 发送 CMD25 多块写入命令
            let r = self
                .cmd_transfer(CommandType::CMD(25), start_block, blk_cnt as _, false)
                .and_then(|_| self.write_buff(data))
                .and_then(|_| self.wait_for_xfer_done());
            // 清除中断状态寄存器
            self.regs
                .norm_and_err_int_sts
                .set(self.regs.norm_and_err_int_sts.get());
            match r {
                Ok(()) => return Ok(()),
                Err(CmdError::IntError) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(CmdError::IntError)
    }

    /// 使用 CMD25 + **ADMA2 (32 位)** 一次性写入多个连续数据块 (DMA)。
    ///
    /// 数据由 DMA 引擎直接从内存读出送往 SD 卡，CPU 不再逐字搬运。
    ///
    /// 职责划分与 [`Sdmmc::read_blocks_adma2`] 相同：调用方提供数据缓冲/描述符表
    /// 及其物理地址；cache 一致性由本函数处理（写方向仅需传输前 clean）。
    ///
    /// # Safety
    /// 物理地址须与切片指向同一段有效 DMA 内存。
    pub unsafe fn write_blocks_adma2(
        &self,
        start_block: u32,
        data: &[u8],
        data_paddr: u64,
        desc_table: &mut [Adma2Desc],
        desc_paddr: u64,
    ) -> Result<(), CmdError> {
        assert!(
            !data.is_empty(),
            "write_blocks_adma2: buffer must not be empty"
        );
        assert!(
            data.len().is_multiple_of(BLOCK_SIZE),
            "write_blocks_adma2: buffer length must be a multiple of BLOCK_SIZE"
        );

        let total_len = data.len();
        let blk_cnt = total_len / BLOCK_SIZE;

        // 构建 ADMA2 描述符表。
        let desc_needed = total_len.div_ceil(ADMA2_MAX_PER_DESC);
        assert!(
            desc_table.len() >= desc_needed,
            "write_blocks_adma2: descriptor table too small"
        );
        let mut offset = 0usize;
        for (i, desc) in desc_table.iter_mut().enumerate().take(desc_needed) {
            let remain = total_len - offset;
            let chunk = core::cmp::min(remain, ADMA2_MAX_PER_DESC);
            let mut attr = ADMA2_ATTR_VALID | ADMA2_ATTR_ACT_TRAN;
            if i == desc_needed - 1 {
                attr |= ADMA2_ATTR_END;
            }
            *desc = Adma2Desc {
                attr,
                len: chunk as u16,
                addr: (data_paddr + offset as u64) as u32,
            };
            offset += chunk;
        }

        // 写方向：把描述符表和源数据都 clean 到内存，保证 DMA 读到最新数据。
        let desc_bytes = desc_needed * core::mem::size_of::<Adma2Desc>();
        dcache_clean_range(desc_table.as_ptr() as usize, desc_bytes);
        dcache_clean_range(data.as_ptr() as usize, total_len);

        // 选择 ADMA2 并写入描述符表物理基址。
        self.regs
            .host_ctl1_pwr_bg_wup
            .modify(HOST_CTL1_PWR_BG_WUP::DMA_SEL::ADMA2);
        self.regs
            .adma_sys_addr_low
            .set((desc_paddr & 0xFFFF_FFFF) as u32);
        self.regs.adma_sys_addr_high.set(0);

        // 发送 CMD25（dma=true 置 DMA_EN），DMA 引擎负责后续数据搬运。
        self.cmd_transfer(CommandType::CMD(25), start_block, blk_cnt as _, true)?;

        // 等待传输完成（写方向无需 invalidate）。
        let res = self.wait_for_xfer_done();
        self.regs
            .norm_and_err_int_sts
            .set(self.regs.norm_and_err_int_sts.get());
        res
    }

    /// 向数据缓冲区写入数据（PIO）。
    ///
    /// 支持一个或多个块：`data` 长度须为 [`BLOCK_SIZE`] 的整数倍，
    /// 按块循环等待 `BUF_WRDY` 后逐 32 位写入数据端口。
    ///
    /// # 参数
    /// - `data`: 要写入的数据，长度须为 [`BLOCK_SIZE`] 的整数倍
    fn write_buff(&self, data: &[u8]) -> Result<(), CmdError> {
        assert!(data.len().is_multiple_of(BLOCK_SIZE));

        for block_buf in data.chunks_exact(BLOCK_SIZE) {
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
            for chunk in block_buf.chunks_exact(4) {
                let value = u32::from_le_bytes(chunk.try_into().unwrap());
                self.regs.buf_data_port.set(value);
                delay(1);
            }
        }
        Ok(())
    }
}
