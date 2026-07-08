//! 块读取路径：CMD17 单块、CMD18 多块（PIO），以及 CMD18 + ADMA2（DMA）。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::consts::*;
use super::{
    ADMA2_ATTR_ACT_TRAN, ADMA2_ATTR_END, ADMA2_ATTR_VALID, ADMA2_MAX_PER_DESC, Adma2Desc, Sdmmc,
};
use crate::utils::cache::{dcache_clean_range, dcache_invalidate_range};
use crate::utils::delay;

impl Sdmmc {
    /// 使用 CMD17 从 SD 卡读取**单个**数据块 (512 字节)。
    ///
    /// 每次调用发送一条 CMD17 命令，仅传输一个块。适用于需要精确
    /// 控制单块访问的场景；批量读取请使用 [`Sdmmc::read_blocks`]。
    ///
    /// # 参数
    /// - `block_id`: 要读取的块号 (逻辑块地址 LBA)
    /// - `buffer`: 用于存储读取数据的缓冲区，必须为 [`BLOCK_SIZE`] (512) 字节
    pub fn read_block_single(&self, block_id: u32, buffer: &mut [u8]) -> Result<(), CmdError> {
        assert_eq!(buffer.len(), BLOCK_SIZE);
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

    /// 使用 CMD18 (READ_MULTIPLE_BLOCK) **一次性**从 SD 卡读取多个连续数据块。
    ///
    /// 单条命令即可完成 `data.len() / BLOCK_SIZE` 个块的传输，控制器在
    /// 传输结束后通过 AUTO_CMD12 自动发送停止命令 (CMD12)。相比循环调用
    /// [`Sdmmc::read_block_single`]，可显著降低每块的命令开销，提升吞吐。
    ///
    /// # 参数
    /// - `start_block`: 起始块号 (逻辑块地址 LBA)
    /// - `data`: 目标缓冲区，长度必须为 [`BLOCK_SIZE`] (512) 的整数倍，
    ///   且至少为一个块
    ///
    /// # Panics
    /// 当 `data` 长度不是 [`BLOCK_SIZE`] 的整数倍或为 0 时 panic。
    pub fn read_blocks(&self, start_block: u32, data: &mut [u8]) -> Result<(), CmdError> {
        assert!(!data.is_empty(), "read_blocks: buffer must not be empty");
        assert!(
            data.len().is_multiple_of(BLOCK_SIZE),
            "read_blocks: buffer length must be a multiple of BLOCK_SIZE"
        );

        let blk_cnt = data.len() / BLOCK_SIZE;
        log::debug!(
            "reading {} blocks (CMD18) from block {}",
            blk_cnt,
            start_block
        );

        // 对 SDHCI 错误中断（瞬时 data CRC / 命令线错误）重试：先软复位数据/命令线，
        // 再重新下发 CMD18。Linux 靠重试+DMA 容忍此类错误，此处对齐。
        for attempt in 0..XFER_RETRY {
            if attempt > 0 {
                self.reset_dat_cmd_lines();
                log::warn!(
                    "sdmmc read retry {}/{}: block {}, {} blocks",
                    attempt,
                    XFER_RETRY,
                    start_block,
                    blk_cnt
                );
            }
            // 发送 CMD18 多块读取命令 (块大小/块数量在 cmd_transfer 内部配置)
            let r = self
                .cmd_transfer(CommandType::CMD(18), start_block, blk_cnt as _, false)
                .and_then(|_| self.read_buff(data))
                .and_then(|_| self.wait_for_xfer_done());
            // 传输结束（无论成败）都清中断状态
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

    /// 从 SD 卡读取一个或多个数据块。
    ///
    /// 这是 [`Sdmmc::read_blocks`] 的兼容别名：`data` 长度为
    /// [`BLOCK_SIZE`] 时读取单块，为其整数倍时一次性读取多块。
    ///
    /// # 参数
    /// - `block_id`: 起始块号 (逻辑块地址 LBA)
    /// - `data`: 用于存储读取数据的缓冲区，长度须为 [`BLOCK_SIZE`] 的整数倍
    pub fn read_block(&self, block_id: u32, data: &mut [u8]) -> Result<(), CmdError> {
        self.read_blocks(block_id, data)
    }

    /// 使用 **ADMA2 (32 位)** 一次性读取多个连续数据块，数据由控制器 DMA
    /// 引擎直接写入内存，CPU 不再逐字从数据端口搬运。
    ///
    /// 相比 PIO 版本 [`Sdmmc::read_blocks`]，DMA 消除了 `buf_data_port`
    /// 轮询开销，通常可获得更高吞吐、更低 CPU 占用。
    ///
    /// # 职责划分
    /// 本 BSP 与具体 MMU / 分配器无关，因此**由调用方**负责：
    /// - 提供物理连续、且已知物理地址的数据缓冲区 `data` / `data_paddr`；
    /// - 提供物理连续、8 字节对齐的描述符表 `desc_table` / `desc_paddr`，
    ///   容量需 `>= ceil(data.len() / ADMA2_MAX_PER_DESC)`；
    /// - 保证两块内存均在 DMA 可见地址空间内。
    ///
    /// 缓存一致性（clean/invalidate）由本函数在内部按 C906/AArch64 规则处理。
    ///
    /// # 参数
    /// - `start_block`: 起始块号 (LBA)
    /// - `data`: 目标缓冲区（长度须为 [`BLOCK_SIZE`] 整数倍且非空）
    /// - `data_paddr`: `data` 的物理基址
    /// - `desc_table`: 供驱动填写的 ADMA2 描述符表暂存区
    /// - `desc_paddr`: `desc_table` 的物理基址（须 8 字节对齐）
    ///
    /// # Safety
    /// 调用者必须确保 `data_paddr` / `desc_paddr` 与对应切片指向同一段、
    /// 在 DMA 引擎视角下有效的物理内存，否则会造成内存破坏。
    pub unsafe fn read_blocks_adma2(
        &self,
        start_block: u32,
        data: &mut [u8],
        data_paddr: u64,
        desc_table: &mut [Adma2Desc],
        desc_paddr: u64,
    ) -> Result<(), CmdError> {
        // 发起 DMA 传输（配置描述符/寄存器并下发 CMD18）。
        unsafe {
            self.adma2_issue(start_block, data, data_paddr, desc_table, desc_paddr)?;
        }
        // 忙等待传输完成。
        let res = self.wait_for_xfer_done();
        // 收尾：清中断状态 + invalidate 数据缓冲。
        self.finish_dma_read(data);
        res
    }

    /// **非阻塞** 地发起一次 ADMA2 多块 DMA 读取：配置描述符表与寄存器、
    /// 下发 CMD18（等待命令完成 CMD_CMPL 后立即返回），数据由 DMA 引擎在
    /// 后台写入内存。调用方随后用 [`Sdmmc::poll_xfer_done`] 轮询是否结束，
    /// 结束后必须调用 [`Sdmmc::finish_dma_read`] 收尾。
    ///
    /// 该接口用于「传输期间让 CPU 去做其他工作」的场景（例如测量 CPU 可用率）。
    ///
    /// 参数含义同 [`Sdmmc::read_blocks_adma2`]。
    ///
    /// # Safety
    /// 同 [`Sdmmc::read_blocks_adma2`]：物理地址须与切片指向同一段有效 DMA 内存。
    pub unsafe fn read_blocks_adma2_start(
        &self,
        start_block: u32,
        data: &mut [u8],
        data_paddr: u64,
        desc_table: &mut [Adma2Desc],
        desc_paddr: u64,
    ) -> Result<(), CmdError> {
        unsafe { self.adma2_issue(start_block, data, data_paddr, desc_table, desc_paddr) }
    }

    /// 轮询 DMA 数据传输是否完成（**不清除** 完成标志，也不做 cache 维护）。
    ///
    /// - `Ok(true)`：传输完成，可调用 [`Sdmmc::finish_dma_read`] 收尾。
    /// - `Ok(false)`：仍在传输中。
    /// - `Err(_)`：发生错误中断（已清除错误标志）。
    pub fn poll_xfer_done(&self) -> Result<bool, CmdError> {
        let sts = self.regs.norm_and_err_int_sts.extract();
        if sts.is_set(NORM_AND_ERR_INT_STS::XFER_CMPL) {
            return Ok(true);
        }
        if sts.is_set(NORM_AND_ERR_INT_STS::ERR_INT) {
            self.regs
                .norm_and_err_int_sts
                .write(NORM_AND_ERR_INT_STS::ERR_INT::SET);
            return Err(CmdError::IntError);
        }
        Ok(false)
    }

    /// DMA 读取收尾：清除中断状态寄存器，并 invalidate 数据缓冲区，
    /// 保证 CPU 后续读到的是 DMA 写入内存的新数据。
    pub fn finish_dma_read(&self, data: &mut [u8]) {
        self.regs
            .norm_and_err_int_sts
            .set(self.regs.norm_and_err_int_sts.get());
        dcache_invalidate_range(data.as_ptr() as usize, data.len());
    }

    /// 内部：构建 ADMA2 描述符表、维护 cache、配置寄存器并下发 CMD18。
    ///
    /// 返回时命令已完成 (CMD_CMPL)，DMA 数据传输在后台进行。
    ///
    /// # Safety
    /// 见 [`Sdmmc::read_blocks_adma2`]。
    unsafe fn adma2_issue(
        &self,
        start_block: u32,
        data: &[u8],
        data_paddr: u64,
        desc_table: &mut [Adma2Desc],
        desc_paddr: u64,
    ) -> Result<(), CmdError> {
        assert!(!data.is_empty(), "adma2: buffer must not be empty");
        assert!(
            data.len().is_multiple_of(BLOCK_SIZE),
            "adma2: buffer length must be a multiple of BLOCK_SIZE"
        );

        let total_len = data.len();
        let blk_cnt = total_len / BLOCK_SIZE;

        // 计算所需描述符数量并填表：把整段缓冲拆分成 <= ADMA2_MAX_PER_DESC 的片段。
        let desc_needed = total_len.div_ceil(ADMA2_MAX_PER_DESC);
        assert!(
            desc_table.len() >= desc_needed,
            "adma2: descriptor table too small"
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

        // 让控制器看到最新的描述符表内容（写回 D-cache）。
        let desc_bytes = desc_needed * core::mem::size_of::<Adma2Desc>();
        dcache_clean_range(desc_table.as_ptr() as usize, desc_bytes);

        // DMA 写内存前先 clean 数据缓冲区，避免脏行在传输期间被回写覆盖 DMA 数据。
        dcache_clean_range(data.as_ptr() as usize, total_len);

        // 选择 ADMA2 作为 DMA 模式，并写入描述符表物理基址。
        self.regs
            .host_ctl1_pwr_bg_wup
            .modify(HOST_CTL1_PWR_BG_WUP::DMA_SEL::ADMA2);
        self.regs
            .adma_sys_addr_low
            .set((desc_paddr & 0xFFFF_FFFF) as u32);
        self.regs.adma_sys_addr_high.set(0);

        // 发送 CMD18（内部会配置块大小/块数、AUTO_CMD12，并因 dma=true 置 DMA_EN）。
        // cmd_transfer 返回时 CMD_CMPL 已完成，数据传输在后台由 DMA 引擎进行。
        self.cmd_transfer(CommandType::CMD(18), start_block, blk_cnt as _, true)
    }

    /// 从数据缓冲区读取数据（PIO）。
    ///
    /// # 参数
    /// - `data`: 用于存储读取数据的缓冲区，长度须为 [`BLOCK_SIZE`] 的整数倍
    fn read_buff(&self, data: &mut [u8]) -> Result<(), CmdError> {
        assert!(data.len().is_multiple_of(BLOCK_SIZE));

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
}
