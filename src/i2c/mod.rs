//! # SG2002 I2C 驱动模块
//!
//! 本模块提供 SG2002 芯片 I2C 控制器的 Rust 驱动实现。
//!
//! ## 功能特性
//!
//! - 支持 Master 模式
//! - 支持 7 位和 10 位地址模式
//! - 支持标准模式 (~100 kbit/s) 和快速模式 (~400 kbit/s)
//! - 支持 General Call 和 Start Byte
//! - 64 x 8bit TX FIFO 和 64 x 8bit RX FIFO
//! - 支持 DMA 传输
//!
//! ## 硬件资源
//!
//! SG2002 芯片共有 6 个 I2C 控制器：
//! - I2C0-I2C4: 位于 Active Domain
//! - RTCSYS_I2C: 位于 No-die Domain (RTC 子系统)
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::i2c::{I2c, I2cInstance, I2cSpeed};
//!
//! // 创建 I2C0 驱动实例
//! let mut i2c = unsafe { I2c::new(I2cInstance::I2c0) };
//!
//! // 初始化 I2C，使用快速模式
//! i2c.init(I2cSpeed::Fast);
//!
//! // 写入数据到设备
//! let slave_addr = 0x50;
//! let data = [0x00, 0x01, 0x02];
//! i2c.write(slave_addr, &data)?;
//!
//! // 从设备读取数据
//! let mut buf = [0u8; 4];
//! i2c.read(slave_addr, &mut buf)?;
//!
//! // 写后读操作
//! let reg_addr = [0x00];
//! i2c.write_read(slave_addr, &reg_addr, &mut buf)?;
//! ```

#![allow(dead_code)]

mod consts;
mod instances;

pub use consts::{
    I2cAddressMode, I2cError, I2cRegisters, I2cSpeed, I2C_DEFAULT_TIMEOUT, I2C_RX_FIFO_DEPTH,
    I2C_TX_FIFO_DEPTH,
};
pub use instances::{
    I2cClockConfig, I2cInstance, I2C0_BASE, I2C1_BASE, I2C2_BASE, I2C3_BASE, I2C4_BASE,
    RTCSYS_I2C_BASE,
};

use consts::*;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

/// I2C 驱动结构体
///
/// 提供对 I2C 控制器的访问接口
pub struct I2c {
    /// I2C 寄存器组引用
    regs: &'static I2cRegisters,
    /// I2C 实例标识
    instance: I2cInstance,
    /// 当前速度模式
    speed: I2cSpeed,
    /// 时钟配置
    clock_config: I2cClockConfig,
}

impl I2c {
    /// 创建新的 I2C 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个相同实例导致数据竞争
    ///
    /// # 参数
    /// - `instance`: I2C 实例标识符
    pub unsafe fn new(instance: I2cInstance) -> Self {
        let base = instance.base_address();
        Self {
            regs: &*(base as *const I2cRegisters),
            instance,
            speed: I2cSpeed::Fast,
            clock_config: I2cClockConfig::CLK_100MHZ,
        }
    }

    /// 从指定基地址创建 I2C 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize, instance: I2cInstance) -> Self {
        Self {
            regs: &*(base as *const I2cRegisters),
            instance,
            speed: I2cSpeed::Fast,
            clock_config: I2cClockConfig::CLK_100MHZ,
        }
    }

    /// 设置时钟配置
    pub fn set_clock_config(&mut self, config: I2cClockConfig) {
        self.clock_config = config;
    }

    /// 获取 I2C 实例标识
    pub fn instance(&self) -> I2cInstance {
        self.instance
    }

    /// 获取当前速度模式
    pub fn speed(&self) -> I2cSpeed {
        self.speed
    }

    /// 禁用 I2C 控制器
    fn disable(&self) {
        self.regs.ic_enable.write(IC_ENABLE::ENABLE::CLEAR);

        // 等待控制器禁用完成
        let mut timeout = I2C_DEFAULT_TIMEOUT;
        while self.regs.ic_enable_status.is_set(IC_ENABLE_STATUS::IC_EN) {
            timeout -= 1;
            if timeout == 0 {
                log::warn!("I2C disable timeout");
                break;
            }
        }
    }

    /// 使能 I2C 控制器
    fn enable(&self) {
        self.regs.ic_enable.write(IC_ENABLE::ENABLE::SET);
    }

    /// 检查 I2C 是否使能
    pub fn is_enabled(&self) -> bool {
        self.regs.ic_enable_status.is_set(IC_ENABLE_STATUS::IC_EN)
    }

    /// 初始化 I2C 控制器
    ///
    /// # 参数
    /// - `speed`: 速度模式 (标准模式或快速模式)
    pub fn init(&mut self, speed: I2cSpeed) {
        self.speed = speed;

        // 1. 禁用 I2C 控制器
        self.disable();

        // 2. 配置控制寄存器
        self.regs.ic_con.write(
            IC_CON::MASTER_MODE::SET
                + IC_CON::SPEED.val(speed as u32)
                + IC_CON::IC_10BITADDR_SLAVE::CLEAR
                + IC_CON::IC_10BITADDR_MASTER::CLEAR
                + IC_CON::IC_RESTART_EN::SET
                + IC_CON::IC_SLAVE_DISABLE::SET,
        );

        // 3. 配置 SCL 时序
        self.configure_scl_timing();

        // 4. 配置 SDA 时序
        self.regs
            .ic_sda_hold
            .write(IC_SDA_HOLD::IC_SDA_HOLD.val(self.clock_config.sda_hold as u32));
        self.regs
            .ic_sda_setup
            .write(IC_SDA_SETUP::SDA_SETUP.val(self.clock_config.sda_setup as u32));

        // 5. 配置毛刺抑制
        self.regs
            .ic_fs_spklen
            .write(IC_FS_SPKLEN::IC_FS_SPKLEN.val(self.clock_config.fs_spklen as u32));

        // 6. 配置 FIFO 阈值
        self.regs.ic_rx_tl.write(IC_RX_TL::RX_TL.val(0));
        self.regs.ic_tx_tl.write(IC_TX_TL::TX_TL.val(0));

        // 7. 禁用所有中断
        self.regs.ic_intr_mask.set(0);

        // 8. 清除所有中断
        let _ = self.regs.ic_clr_intr.get();

        log::debug!(
            "I2C{} initialized with {:?} mode",
            self.instance.index(),
            speed
        );
    }

    /// 配置 SCL 时序
    fn configure_scl_timing(&self) {
        // 标准模式时序
        self.regs
            .ic_ss_scl_hcnt
            .write(IC_SS_SCL_HCNT::IC_SS_SCL_HCNT.val(self.clock_config.ss_scl_hcnt as u32));
        self.regs
            .ic_ss_scl_lcnt
            .write(IC_SS_SCL_LCNT::IC_SS_SCL_LCNT.val(self.clock_config.ss_scl_lcnt as u32));

        // 快速模式时序
        self.regs
            .ic_fs_scl_hcnt
            .write(IC_FS_SCL_HCNT::IC_FS_SCL_HCNT.val(self.clock_config.fs_scl_hcnt as u32));
        self.regs
            .ic_fs_scl_lcnt
            .write(IC_FS_SCL_LCNT::IC_FS_SCL_LCNT.val(self.clock_config.fs_scl_lcnt as u32));
    }

    /// 设置目标从机地址
    ///
    /// # 参数
    /// - `addr`: 7 位或 10 位从机地址
    /// - `mode`: 地址模式
    fn set_target_address(&self, addr: u16, mode: I2cAddressMode) {
        // 必须在禁用状态下修改目标地址
        let was_enabled = self.is_enabled();
        if was_enabled {
            self.disable();
        }

        // 配置地址模式
        match mode {
            I2cAddressMode::SevenBit => {
                self.regs.ic_con.modify(IC_CON::IC_10BITADDR_MASTER::CLEAR);
            }
            I2cAddressMode::TenBit => {
                self.regs.ic_con.modify(IC_CON::IC_10BITADDR_MASTER::SET);
            }
        }

        // 设置目标地址
        self.regs.ic_tar.write(
            IC_TAR::IC_TAR.val(addr as u32)
                + IC_TAR::SPECIAL::CLEAR
                + IC_TAR::GC_OR_START::GeneralCall,
        );

        if was_enabled {
            self.enable();
        }
    }

    /// 等待发送 FIFO 非满
    fn wait_tx_fifo_not_full(&self) -> Result<(), I2cError> {
        let mut timeout = I2C_DEFAULT_TIMEOUT;
        while !self.regs.ic_status.is_set(IC_STATUS::ST_TFNF) {
            timeout -= 1;
            if timeout == 0 {
                return Err(I2cError::Timeout);
            }
        }
        Ok(())
    }

    /// 等待接收 FIFO 非空
    fn wait_rx_fifo_not_empty(&self) -> Result<(), I2cError> {
        let mut timeout = I2C_DEFAULT_TIMEOUT;
        while !self.regs.ic_status.is_set(IC_STATUS::ST_RFNE) {
            // 检查发送中止
            if self.regs.ic_raw_intr_stat.is_set(IC_RAW_INTR_STAT::IST_TX_ABRT) {
                let _ = self.regs.ic_clr_tx_abrt.get();
                return Err(I2cError::TxAbort);
            }
            timeout -= 1;
            if timeout == 0 {
                return Err(I2cError::Timeout);
            }
        }
        Ok(())
    }

    /// 等待传输完成
    fn wait_transfer_complete(&self) -> Result<(), I2cError> {
        let mut timeout = I2C_DEFAULT_TIMEOUT;
        while self.regs.ic_status.is_set(IC_STATUS::ST_ACTIVITY)
            || !self.regs.ic_status.is_set(IC_STATUS::ST_TFE)
        {
            // 检查发送中止
            if self.regs.ic_raw_intr_stat.is_set(IC_RAW_INTR_STAT::IST_TX_ABRT) {
                let _ = self.regs.ic_clr_tx_abrt.get();
                return Err(I2cError::TxAbort);
            }
            timeout -= 1;
            if timeout == 0 {
                return Err(I2cError::Timeout);
            }
        }
        Ok(())
    }

    /// 检查并处理错误
    fn check_errors(&self) -> Result<(), I2cError> {
        let raw_stat = self.regs.ic_raw_intr_stat.get();

        if raw_stat & (1 << 6) != 0 {
            // TX_ABRT
            let _ = self.regs.ic_clr_tx_abrt.get();
            return Err(I2cError::TxAbort);
        }

        if raw_stat & (1 << 1) != 0 {
            // RX_OVER
            let _ = self.regs.ic_clr_rx_over.get();
            return Err(I2cError::RxOverflow);
        }

        if raw_stat & (1 << 3) != 0 {
            // TX_OVER
            let _ = self.regs.ic_clr_tx_over.get();
            return Err(I2cError::TxOverflow);
        }

        Ok(())
    }

    /// 向 I2C 设备写入数据
    ///
    /// # 参数
    /// - `addr`: 7 位从机地址
    /// - `data`: 要写入的数据
    ///
    /// # 返回值
    /// - `Ok(())`: 写入成功
    /// - `Err(I2cError)`: 写入失败
    pub fn write(&self, addr: u8, data: &[u8]) -> Result<(), I2cError> {
        self.write_with_mode(addr as u16, I2cAddressMode::SevenBit, data)
    }

    /// 向 I2C 设备写入数据 (指定地址模式)
    ///
    /// # 参数
    /// - `addr`: 从机地址
    /// - `mode`: 地址模式
    /// - `data`: 要写入的数据
    pub fn write_with_mode(
        &self,
        addr: u16,
        mode: I2cAddressMode,
        data: &[u8],
    ) -> Result<(), I2cError> {
        if data.is_empty() {
            return Ok(());
        }

        // 设置目标地址
        self.set_target_address(addr, mode);

        // 使能 I2C
        self.enable();

        // 写入数据
        let len = data.len();
        for (i, &byte) in data.iter().enumerate() {
            self.wait_tx_fifo_not_full()?;

            let mut cmd = IC_DATA_CMD::DAT.val(byte as u32) + IC_DATA_CMD::CMD::Write;

            // 最后一个字节发送 STOP
            if i == len - 1 {
                cmd += IC_DATA_CMD::STOP::SET;
            }

            self.regs.ic_data_cmd.write(cmd);
        }

        // 等待传输完成
        self.wait_transfer_complete()?;

        // 检查错误
        self.check_errors()
    }

    /// 从 I2C 设备读取数据
    ///
    /// # 参数
    /// - `addr`: 7 位从机地址
    /// - `buffer`: 用于存储读取数据的缓冲区
    ///
    /// # 返回值
    /// - `Ok(())`: 读取成功
    /// - `Err(I2cError)`: 读取失败
    pub fn read(&self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        self.read_with_mode(addr as u16, I2cAddressMode::SevenBit, buffer)
    }

    /// 从 I2C 设备读取数据 (指定地址模式)
    ///
    /// # 参数
    /// - `addr`: 从机地址
    /// - `mode`: 地址模式
    /// - `buffer`: 用于存储读取数据的缓冲区
    pub fn read_with_mode(
        &self,
        addr: u16,
        mode: I2cAddressMode,
        buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        if buffer.is_empty() {
            return Ok(());
        }

        // 设置目标地址
        self.set_target_address(addr, mode);

        // 使能 I2C
        self.enable();

        // 发送读命令并接收数据
        let len = buffer.len();
        for (i, byte) in buffer.iter_mut().enumerate() {
            self.wait_tx_fifo_not_full()?;

            let mut cmd = IC_DATA_CMD::CMD::Read;

            // 最后一个字节发送 STOP
            if i == len - 1 {
                cmd += IC_DATA_CMD::STOP::SET;
            }

            self.regs.ic_data_cmd.write(cmd);

            // 等待数据可用
            self.wait_rx_fifo_not_empty()?;

            // 读取数据
            *byte = self.regs.ic_data_cmd.read(IC_DATA_CMD::DAT) as u8;
        }

        // 检查错误
        self.check_errors()
    }

    /// 先写后读操作 (常用于寄存器读取)
    ///
    /// # 参数
    /// - `addr`: 7 位从机地址
    /// - `write_data`: 要写入的数据 (通常是寄存器地址)
    /// - `read_buffer`: 用于存储读取数据的缓冲区
    ///
    /// # 返回值
    /// - `Ok(())`: 操作成功
    /// - `Err(I2cError)`: 操作失败
    pub fn write_read(
        &self,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        self.write_read_with_mode(
            addr as u16,
            I2cAddressMode::SevenBit,
            write_data,
            read_buffer,
        )
    }

    /// 先写后读操作 (指定地址模式)
    ///
    /// # 参数
    /// - `addr`: 从机地址
    /// - `mode`: 地址模式
    /// - `write_data`: 要写入的数据
    /// - `read_buffer`: 用于存储读取数据的缓冲区
    pub fn write_read_with_mode(
        &self,
        addr: u16,
        mode: I2cAddressMode,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        if write_data.is_empty() && read_buffer.is_empty() {
            return Ok(());
        }

        // 设置目标地址
        self.set_target_address(addr, mode);

        // 使能 I2C
        self.enable();

        // 写入阶段
        for &byte in write_data {
            self.wait_tx_fifo_not_full()?;
            self.regs
                .ic_data_cmd
                .write(IC_DATA_CMD::DAT.val(byte as u32) + IC_DATA_CMD::CMD::Write);
        }

        // 读取阶段
        let read_len = read_buffer.len();
        for (i, byte) in read_buffer.iter_mut().enumerate() {
            self.wait_tx_fifo_not_full()?;

            let mut cmd = IC_DATA_CMD::CMD::Read;

            // 第一个读命令发送 RESTART
            if i == 0 && !write_data.is_empty() {
                cmd += IC_DATA_CMD::RESTART::SET;
            }

            // 最后一个字节发送 STOP
            if i == read_len - 1 {
                cmd += IC_DATA_CMD::STOP::SET;
            }

            self.regs.ic_data_cmd.write(cmd);

            // 等待数据可用
            self.wait_rx_fifo_not_empty()?;

            // 读取数据
            *byte = self.regs.ic_data_cmd.read(IC_DATA_CMD::DAT) as u8;
        }

        // 检查错误
        self.check_errors()
    }

    /// 获取发送 FIFO 级别
    pub fn tx_fifo_level(&self) -> u8 {
        self.regs.ic_txflr.read(IC_TXFLR::TXFLR) as u8
    }

    /// 获取接收 FIFO 级别
    pub fn rx_fifo_level(&self) -> u8 {
        self.regs.ic_rxflr.read(IC_RXFLR::RXFLR) as u8
    }

    /// 获取原始中断状态
    pub fn raw_interrupt_status(&self) -> u32 {
        self.regs.ic_raw_intr_stat.get()
    }

    /// 清除所有中断
    pub fn clear_all_interrupts(&self) {
        let _ = self.regs.ic_clr_intr.get();
    }

    /// 使能 DMA 发送
    pub fn enable_tx_dma(&self) {
        self.regs.ic_dma_cr.modify(IC_DMA_CR::TDMAE::SET);
    }

    /// 使能 DMA 接收
    pub fn enable_rx_dma(&self) {
        self.regs.ic_dma_cr.modify(IC_DMA_CR::RDMAE::SET);
    }

    /// 禁用 DMA
    pub fn disable_dma(&self) {
        self.regs
            .ic_dma_cr
            .modify(IC_DMA_CR::TDMAE::CLEAR + IC_DMA_CR::RDMAE::CLEAR);
    }

    /// 设置 DMA 发送数据级别
    pub fn set_dma_tx_level(&self, level: u8) {
        self.regs
            .ic_dma_tdlr
            .write(IC_DMA_TDLR::DMATDL.val(level as u32));
    }

    /// 设置 DMA 接收数据级别
    pub fn set_dma_rx_level(&self, level: u8) {
        self.regs
            .ic_dma_rdlr
            .write(IC_DMA_RDLR::DMARDL.val(level as u32));
    }
}

// ============================================================================
// 便捷函数
// ============================================================================

/// 创建 I2C0 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn i2c0() -> I2c {
    I2c::new(I2cInstance::I2c0)
}

/// 创建 I2C1 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn i2c1() -> I2c {
    I2c::new(I2cInstance::I2c1)
}

/// 创建 I2C2 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn i2c2() -> I2c {
    I2c::new(I2cInstance::I2c2)
}

/// 创建 I2C3 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn i2c3() -> I2c {
    I2c::new(I2cInstance::I2c3)
}

/// 创建 I2C4 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn i2c4() -> I2c {
    I2c::new(I2cInstance::I2c4)
}

/// 创建 RTCSYS_I2C 驱动实例
///
/// # Safety
///
/// 调用者必须确保不会创建多个实例导致数据竞争
pub unsafe fn rtcsys_i2c() -> I2c {
    I2c::new(I2cInstance::RtcsysI2c)
}
