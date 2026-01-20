//! # I2C 驱动常量和寄存器定义
//!
//! 本模块定义了 SG2002 芯片 I2C 控制器相关的：
//! - 寄存器偏移地址
//! - 位域结构体 (使用 tock-registers)
//! - 速度模式枚举
//! - 错误类型

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

// ============================================================================
// 寄存器位域定义 (使用 tock-registers)
// ============================================================================

register_bitfields! [
    u32,

    /// I2C 控制寄存器 (偏移 0x000)
    pub IC_CON [
        /// 从机禁用 (bit6)
        /// 0: 从机使能
        /// 1: 从机禁用
        IC_SLAVE_DISABLE OFFSET(6) NUMBITS(1) [],
        /// 重启使能 (bit5)
        /// 使能 I2C 主机产生 RESTART 条件
        IC_RESTART_EN OFFSET(5) NUMBITS(1) [],
        /// 10 位主机地址模式 (bit4)
        /// 使能 10 位主机地址模式
        IC_10BITADDR_MASTER OFFSET(4) NUMBITS(1) [],
        /// 10 位从机地址模式 (bit3)
        /// 使能 10 位从机地址模式
        IC_10BITADDR_SLAVE OFFSET(3) NUMBITS(1) [],
        /// 速度模式选择 (bit1-2)
        /// 1: 标准模式 (~100 kbit/s)
        /// 2: 快速模式 (~400 kbit/s)
        SPEED OFFSET(1) NUMBITS(2) [
            Standard = 1,
            Fast = 2
        ],
        /// 主机模式使能 (bit0)
        MASTER_MODE OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 目标地址寄存器 (偏移 0x004)
    pub IC_TAR [
        /// 特殊命令 (bit11)
        /// 用于发出 General Call 或 START BYTE
        SPECIAL OFFSET(11) NUMBITS(1) [],
        /// General Call 或 Start Byte (bit10)
        /// 当 SPECIAL 为 1 时:
        /// 0: General Call
        /// 1: Start Byte
        GC_OR_START OFFSET(10) NUMBITS(1) [
            GeneralCall = 0,
            StartByte = 1
        ],
        /// 目标地址 (bit0-9)
        /// I2C 目标地址寄存器
        IC_TAR OFFSET(0) NUMBITS(10) []
    ],

    /// I2C 从机地址寄存器 (偏移 0x008)
    pub IC_SAR [
        /// 从机地址 (bit0-9)
        IC_SAR OFFSET(0) NUMBITS(10) []
    ],

    /// I2C 数据命令寄存器 (偏移 0x010)
    pub IC_DATA_CMD [
        /// 重启 (bit10)
        /// 发出 RESTART 条件
        RESTART OFFSET(10) NUMBITS(1) [],
        /// 停止 (bit9)
        /// 发出 STOP 条件
        STOP OFFSET(9) NUMBITS(1) [],
        /// 命令 (bit8)
        /// 0: 写
        /// 1: 读
        CMD OFFSET(8) NUMBITS(1) [
            Write = 0,
            Read = 1
        ],
        /// 数据 (bit0-7)
        /// 发送或接收的数据
        DAT OFFSET(0) NUMBITS(8) []
    ],

    /// 标准速度 SCL 高电平计数寄存器 (偏移 0x014)
    pub IC_SS_SCL_HCNT [
        /// SCL 高电平计数
        IC_SS_SCL_HCNT OFFSET(0) NUMBITS(16) []
    ],

    /// 标准速度 SCL 低电平计数寄存器 (偏移 0x018)
    pub IC_SS_SCL_LCNT [
        /// SCL 低电平计数
        IC_SS_SCL_LCNT OFFSET(0) NUMBITS(16) []
    ],

    /// 快速模式 SCL 高电平计数寄存器 (偏移 0x01c)
    pub IC_FS_SCL_HCNT [
        /// SCL 高电平计数
        IC_FS_SCL_HCNT OFFSET(0) NUMBITS(16) []
    ],

    /// 快速模式 SCL 低电平计数寄存器 (偏移 0x020)
    pub IC_FS_SCL_LCNT [
        /// SCL 低电平计数
        IC_FS_SCL_LCNT OFFSET(0) NUMBITS(16) []
    ],

    /// I2C 中断状态寄存器 (偏移 0x02c)
    pub IC_INTR_STAT [
        /// General Call 中断 (bit11)
        R_GEN_CALL OFFSET(11) NUMBITS(1) [],
        /// START 检测中断 (bit10)
        R_START_DET OFFSET(10) NUMBITS(1) [],
        /// STOP 检测中断 (bit9)
        R_STOP_DET OFFSET(9) NUMBITS(1) [],
        /// 活动中断 (bit8)
        R_ACTIVITY OFFSET(8) NUMBITS(1) [],
        /// 接收完成中断 (bit7)
        R_RX_DONE OFFSET(7) NUMBITS(1) [],
        /// 发送中止中断 (bit6)
        R_TX_ABRT OFFSET(6) NUMBITS(1) [],
        /// 读请求中断 (bit5)
        R_RD_REQ OFFSET(5) NUMBITS(1) [],
        /// 发送空中断 (bit4)
        R_TX_EMPTY OFFSET(4) NUMBITS(1) [],
        /// 发送溢出中断 (bit3)
        R_TX_OVER OFFSET(3) NUMBITS(1) [],
        /// 接收满中断 (bit2)
        R_RX_FULL OFFSET(2) NUMBITS(1) [],
        /// 接收溢出中断 (bit1)
        R_RX_OVER OFFSET(1) NUMBITS(1) [],
        /// 接收下溢中断 (bit0)
        R_RX_UNDER OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 中断屏蔽寄存器 (偏移 0x030)
    pub IC_INTR_MASK [
        /// General Call 中断屏蔽 (bit11)
        M_GEN_CALL OFFSET(11) NUMBITS(1) [],
        /// START 检测中断屏蔽 (bit10)
        M_START_DET OFFSET(10) NUMBITS(1) [],
        /// STOP 检测中断屏蔽 (bit9)
        M_STOP_DET OFFSET(9) NUMBITS(1) [],
        /// 活动中断屏蔽 (bit8)
        M_ACTIVITY OFFSET(8) NUMBITS(1) [],
        /// 接收完成中断屏蔽 (bit7)
        M_RX_DONE OFFSET(7) NUMBITS(1) [],
        /// 发送中止中断屏蔽 (bit6)
        M_TX_ABRT OFFSET(6) NUMBITS(1) [],
        /// 读请求中断屏蔽 (bit5)
        M_RD_REQ OFFSET(5) NUMBITS(1) [],
        /// 发送空中断屏蔽 (bit4)
        M_TX_EMPTY OFFSET(4) NUMBITS(1) [],
        /// 发送溢出中断屏蔽 (bit3)
        M_TX_OVER OFFSET(3) NUMBITS(1) [],
        /// 接收满中断屏蔽 (bit2)
        M_RX_FULL OFFSET(2) NUMBITS(1) [],
        /// 接收溢出中断屏蔽 (bit1)
        M_RX_OVER OFFSET(1) NUMBITS(1) [],
        /// 接收下溢中断屏蔽 (bit0)
        M_RX_UNDER OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 原始中断状态寄存器 (偏移 0x034)
    pub IC_RAW_INTR_STAT [
        /// General Call 地址接收 (bit11)
        IST_GEN_CALL OFFSET(11) NUMBITS(1) [],
        /// START 或 RESTART 发生 (bit10)
        IST_START_DET OFFSET(10) NUMBITS(1) [],
        /// STOP 发生 (bit9)
        IST_STOP_DET OFFSET(9) NUMBITS(1) [],
        /// I2C 活动检测 (bit8)
        IST_ACTIVITY OFFSET(8) NUMBITS(1) [],
        /// 从机发送模式收到 NACK (bit7)
        IST_RX_DONE OFFSET(7) NUMBITS(1) [],
        /// 发送中止 (bit6)
        IST_TX_ABRT OFFSET(6) NUMBITS(1) [],
        /// 从机模式等待处理器响应 (bit5)
        IST_RD_REQ OFFSET(5) NUMBITS(1) [],
        /// 发送缓冲区低于阈值 (bit4)
        IST_TX_EMPTY OFFSET(4) NUMBITS(1) [],
        /// 发送缓冲区溢出 (bit3)
        IST_TX_OVER OFFSET(3) NUMBITS(1) [],
        /// 接收缓冲区达到或超过阈值 (bit2)
        IST_RX_FULL OFFSET(2) NUMBITS(1) [],
        /// 接收缓冲区溢出 (bit1)
        IST_RX_OVER OFFSET(1) NUMBITS(1) [],
        /// 接收缓冲区为空时读取 (bit0)
        IST_RX_UNDER OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 接收 FIFO 阈值寄存器 (偏移 0x038)
    pub IC_RX_TL [
        /// 接收 FIFO 阈值 (bit0-7)
        RX_TL OFFSET(0) NUMBITS(8) []
    ],

    /// I2C 发送 FIFO 阈值寄存器 (偏移 0x03c)
    pub IC_TX_TL [
        /// 发送 FIFO 阈值 (bit0-7)
        TX_TL OFFSET(0) NUMBITS(8) []
    ],

    /// 清除所有中断寄存器 (偏移 0x040)
    pub IC_CLR_INTR [
        /// 读取清除所有中断
        CLR_INTR OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 RX_UNDER 中断寄存器 (偏移 0x044)
    pub IC_CLR_RX_UNDER [
        CLR_RX_UNDER OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 RX_OVER 中断寄存器 (偏移 0x048)
    pub IC_CLR_RX_OVER [
        CLR_RX_OVER OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 TX_OVER 中断寄存器 (偏移 0x04c)
    pub IC_CLR_TX_OVER [
        CLR_TX_OVER OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 RD_REQ 中断寄存器 (偏移 0x050)
    pub IC_CLR_RD_REQ [
        CLR_RD_REQ OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 TX_ABRT 中断寄存器 (偏移 0x054)
    pub IC_CLR_TX_ABRT [
        CLR_TX_ABRT OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 RX_DONE 中断寄存器 (偏移 0x058)
    pub IC_CLR_RX_DONE [
        CLR_RX_DONE OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 ACTIVITY 中断寄存器 (偏移 0x05c)
    pub IC_CLR_ACTIVITY [
        CLR_ACTIVITY OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 STOP_DET 中断寄存器 (偏移 0x060)
    pub IC_CLR_STOP_DET [
        CLR_STOP_DET OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 START_DET 中断寄存器 (偏移 0x064)
    pub IC_CLR_START_DET [
        CLR_START_DET OFFSET(0) NUMBITS(1) []
    ],

    /// 清除 GEN_CALL 中断寄存器 (偏移 0x068)
    pub IC_CLR_GEN_CALL [
        CLR_GEN_CALL OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 使能寄存器 (偏移 0x06c)
    pub IC_ENABLE [
        /// I2C 控制器使能 (bit0)
        ENABLE OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 状态寄存器 (偏移 0x070)
    pub IC_STATUS [
        /// 从机 FSM 活动状态 (bit6)
        ST_SLV_ACTIVITY OFFSET(6) NUMBITS(1) [],
        /// 主机 FSM 活动状态 (bit5)
        ST_MST_ACTIVITY OFFSET(5) NUMBITS(1) [],
        /// 接收 FIFO 完全满 (bit4)
        ST_RFF OFFSET(4) NUMBITS(1) [],
        /// 接收 FIFO 非空 (bit3)
        ST_RFNE OFFSET(3) NUMBITS(1) [],
        /// 发送 FIFO 完全空 (bit2)
        ST_TFE OFFSET(2) NUMBITS(1) [],
        /// 发送 FIFO 非满 (bit1)
        ST_TFNF OFFSET(1) NUMBITS(1) [],
        /// I2C 活动状态 (bit0)
        ST_ACTIVITY OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 发送 FIFO 级别寄存器 (偏移 0x074)
    pub IC_TXFLR [
        /// 发送 FIFO 级别 (bit0-6)
        TXFLR OFFSET(0) NUMBITS(7) []
    ],

    /// I2C 接收 FIFO 级别寄存器 (偏移 0x078)
    pub IC_RXFLR [
        /// 接收 FIFO 级别 (bit0-6)
        RXFLR OFFSET(0) NUMBITS(7) []
    ],

    /// SDA 保持时间寄存器 (偏移 0x07c)
    pub IC_SDA_HOLD [
        /// SDA 保持时间 (bit0-15)
        /// 相对于 SCL 下降沿的 SDA 保持时间
        IC_SDA_HOLD OFFSET(0) NUMBITS(16) []
    ],

    /// I2C 发送中止源寄存器 (偏移 0x080)
    pub IC_TX_ABRT_SOURCE [
        /// 发送中止源 (bit0-15)
        TX_ABRT_SOURCE OFFSET(0) NUMBITS(16) []
    ],

    /// 从机数据 NACK 寄存器 (偏移 0x084)
    pub IC_SLV_DATA_NACK_ONLY [
        /// 在从机接收模式产生 NACK (bit0)
        NACK OFFSET(0) NUMBITS(1) []
    ],

    /// DMA 控制寄存器 (偏移 0x088)
    pub IC_DMA_CR [
        /// 发送 DMA 使能 (bit1)
        TDMAE OFFSET(1) NUMBITS(1) [],
        /// 接收 DMA 使能 (bit0)
        RDMAE OFFSET(0) NUMBITS(1) []
    ],

    /// DMA 发送数据级别寄存器 (偏移 0x08c)
    pub IC_DMA_TDLR [
        /// DMA 发送数据级别 (bit0-5)
        DMATDL OFFSET(0) NUMBITS(6) []
    ],

    /// DMA 接收数据级别寄存器 (偏移 0x090)
    pub IC_DMA_RDLR [
        /// DMA 接收数据级别 (bit0-5)
        DMARDL OFFSET(0) NUMBITS(6) []
    ],

    /// SDA 建立时间寄存器 (偏移 0x094)
    pub IC_SDA_SETUP [
        /// SDA 建立时间 (bit0)
        SDA_SETUP OFFSET(0) NUMBITS(1) []
    ],

    /// ACK General Call 寄存器 (偏移 0x098)
    pub IC_ACK_GENERAL_CALL [
        /// 响应 General Call (bit0)
        /// 1: 响应 ACK
        /// 0: 不产生 General Call 中断
        ACK_GEN_CALL OFFSET(0) NUMBITS(1) []
    ],

    /// I2C 使能状态寄存器 (偏移 0x09c)
    pub IC_ENABLE_STATUS [
        /// 从机接收数据丢失 (bit2)
        SLV_RX_DATA_LOST OFFSET(2) NUMBITS(1) [],
        /// 从机忙时被禁用 (bit1)
        SLV_DISABLED_WHILE_BUSY OFFSET(1) NUMBITS(1) [],
        /// I2C 使能状态 (bit0)
        IC_EN OFFSET(0) NUMBITS(1) []
    ],

    /// 标准和快速模式毛刺抑制寄存器 (偏移 0x0a0)
    pub IC_FS_SPKLEN [
        /// 毛刺抑制长度 (bit0-7)
        IC_FS_SPKLEN OFFSET(0) NUMBITS(8) []
    ],

    /// 高速模式毛刺抑制寄存器 (偏移 0x0a4)
    pub IC_HS_SPKLEN [
        /// 毛刺抑制长度 (bit0-7)
        IC_HS_SPKLEN OFFSET(0) NUMBITS(8) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// I2C 控制器寄存器组
    pub I2cRegisters {
        /// I2C 控制寄存器 (偏移 0x000)
        (0x000 => pub ic_con: ReadWrite<u32, IC_CON::Register>),

        /// I2C 目标地址寄存器 (偏移 0x004)
        (0x004 => pub ic_tar: ReadWrite<u32, IC_TAR::Register>),

        /// I2C 从机地址寄存器 (偏移 0x008)
        (0x008 => pub ic_sar: ReadWrite<u32, IC_SAR::Register>),

        /// 保留 (偏移 0x00c)
        (0x00c => _reserved0),

        /// I2C 数据命令寄存器 (偏移 0x010)
        (0x010 => pub ic_data_cmd: ReadWrite<u32, IC_DATA_CMD::Register>),

        /// 标准速度 SCL 高电平计数寄存器 (偏移 0x014)
        (0x014 => pub ic_ss_scl_hcnt: ReadWrite<u32, IC_SS_SCL_HCNT::Register>),

        /// 标准速度 SCL 低电平计数寄存器 (偏移 0x018)
        (0x018 => pub ic_ss_scl_lcnt: ReadWrite<u32, IC_SS_SCL_LCNT::Register>),

        /// 快速模式 SCL 高电平计数寄存器 (偏移 0x01c)
        (0x01c => pub ic_fs_scl_hcnt: ReadWrite<u32, IC_FS_SCL_HCNT::Register>),

        /// 快速模式 SCL 低电平计数寄存器 (偏移 0x020)
        (0x020 => pub ic_fs_scl_lcnt: ReadWrite<u32, IC_FS_SCL_LCNT::Register>),

        /// 保留 (偏移 0x024-0x028)
        (0x024 => _reserved1),

        /// I2C 中断状态寄存器 (偏移 0x02c)
        (0x02c => pub ic_intr_stat: ReadWrite<u32, IC_INTR_STAT::Register>),

        /// I2C 中断屏蔽寄存器 (偏移 0x030)
        (0x030 => pub ic_intr_mask: ReadWrite<u32, IC_INTR_MASK::Register>),

        /// I2C 原始中断状态寄存器 (偏移 0x034)
        (0x034 => pub ic_raw_intr_stat: ReadWrite<u32, IC_RAW_INTR_STAT::Register>),

        /// I2C 接收 FIFO 阈值寄存器 (偏移 0x038)
        (0x038 => pub ic_rx_tl: ReadWrite<u32, IC_RX_TL::Register>),

        /// I2C 发送 FIFO 阈值寄存器 (偏移 0x03c)
        (0x03c => pub ic_tx_tl: ReadWrite<u32, IC_TX_TL::Register>),

        /// 清除所有中断寄存器 (偏移 0x040)
        (0x040 => pub ic_clr_intr: ReadWrite<u32, IC_CLR_INTR::Register>),

        /// 清除 RX_UNDER 中断寄存器 (偏移 0x044)
        (0x044 => pub ic_clr_rx_under: ReadWrite<u32, IC_CLR_RX_UNDER::Register>),

        /// 清除 RX_OVER 中断寄存器 (偏移 0x048)
        (0x048 => pub ic_clr_rx_over: ReadWrite<u32, IC_CLR_RX_OVER::Register>),

        /// 清除 TX_OVER 中断寄存器 (偏移 0x04c)
        (0x04c => pub ic_clr_tx_over: ReadWrite<u32, IC_CLR_TX_OVER::Register>),

        /// 清除 RD_REQ 中断寄存器 (偏移 0x050)
        (0x050 => pub ic_clr_rd_req: ReadWrite<u32, IC_CLR_RD_REQ::Register>),

        /// 清除 TX_ABRT 中断寄存器 (偏移 0x054)
        (0x054 => pub ic_clr_tx_abrt: ReadWrite<u32, IC_CLR_TX_ABRT::Register>),

        /// 清除 RX_DONE 中断寄存器 (偏移 0x058)
        (0x058 => pub ic_clr_rx_done: ReadWrite<u32, IC_CLR_RX_DONE::Register>),

        /// 清除 ACTIVITY 中断寄存器 (偏移 0x05c)
        (0x05c => pub ic_clr_activity: ReadWrite<u32, IC_CLR_ACTIVITY::Register>),

        /// 清除 STOP_DET 中断寄存器 (偏移 0x060)
        (0x060 => pub ic_clr_stop_det: ReadWrite<u32, IC_CLR_STOP_DET::Register>),

        /// 清除 START_DET 中断寄存器 (偏移 0x064)
        (0x064 => pub ic_clr_start_det: ReadWrite<u32, IC_CLR_START_DET::Register>),

        /// 清除 GEN_CALL 中断寄存器 (偏移 0x068)
        (0x068 => pub ic_clr_gen_call: ReadWrite<u32, IC_CLR_GEN_CALL::Register>),

        /// I2C 使能寄存器 (偏移 0x06c)
        (0x06c => pub ic_enable: ReadWrite<u32, IC_ENABLE::Register>),

        /// I2C 状态寄存器 (偏移 0x070)
        (0x070 => pub ic_status: ReadWrite<u32, IC_STATUS::Register>),

        /// I2C 发送 FIFO 级别寄存器 (偏移 0x074)
        (0x074 => pub ic_txflr: ReadWrite<u32, IC_TXFLR::Register>),

        /// I2C 接收 FIFO 级别寄存器 (偏移 0x078)
        (0x078 => pub ic_rxflr: ReadWrite<u32, IC_RXFLR::Register>),

        /// SDA 保持时间寄存器 (偏移 0x07c)
        (0x07c => pub ic_sda_hold: ReadWrite<u32, IC_SDA_HOLD::Register>),

        /// I2C 发送中止源寄存器 (偏移 0x080)
        (0x080 => pub ic_tx_abrt_source: ReadWrite<u32, IC_TX_ABRT_SOURCE::Register>),

        /// 从机数据 NACK 寄存器 (偏移 0x084)
        (0x084 => pub ic_slv_data_nack_only: ReadWrite<u32, IC_SLV_DATA_NACK_ONLY::Register>),

        /// DMA 控制寄存器 (偏移 0x088)
        (0x088 => pub ic_dma_cr: ReadWrite<u32, IC_DMA_CR::Register>),

        /// DMA 发送数据级别寄存器 (偏移 0x08c)
        (0x08c => pub ic_dma_tdlr: ReadWrite<u32, IC_DMA_TDLR::Register>),

        /// DMA 接收数据级别寄存器 (偏移 0x090)
        (0x090 => pub ic_dma_rdlr: ReadWrite<u32, IC_DMA_RDLR::Register>),

        /// SDA 建立时间寄存器 (偏移 0x094)
        (0x094 => pub ic_sda_setup: ReadWrite<u32, IC_SDA_SETUP::Register>),

        /// ACK General Call 寄存器 (偏移 0x098)
        (0x098 => pub ic_ack_general_call: ReadWrite<u32, IC_ACK_GENERAL_CALL::Register>),

        /// I2C 使能状态寄存器 (偏移 0x09c)
        (0x09c => pub ic_enable_status: ReadWrite<u32, IC_ENABLE_STATUS::Register>),

        /// 标准和快速模式毛刺抑制寄存器 (偏移 0x0a0)
        (0x0a0 => pub ic_fs_spklen: ReadWrite<u32, IC_FS_SPKLEN::Register>),

        /// 高速模式毛刺抑制寄存器 (偏移 0x0a4)
        (0x0a4 => pub ic_hs_spklen: ReadWrite<u32, IC_HS_SPKLEN::Register>),

        /// 结束标记
        (0x0a8 => @END),
    }
}

// ============================================================================
// 错误类型和枚举定义
// ============================================================================

/// I2C 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    /// 发送中止
    TxAbort,
    /// 接收溢出
    RxOverflow,
    /// 发送溢出
    TxOverflow,
    /// 超时
    Timeout,
    /// 总线忙
    BusBusy,
    /// NACK 错误
    Nack,
    /// 仲裁丢失
    ArbitrationLost,
}

/// I2C 速度模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum I2cSpeed {
    /// 标准模式 (~100 kbit/s)
    Standard = 1,
    /// 快速模式 (~400 kbit/s)
    Fast = 2,
}

impl I2cSpeed {
    /// 从 u32 值转换
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Standard),
            2 => Some(Self::Fast),
            _ => None,
        }
    }
}

/// I2C 地址模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cAddressMode {
    /// 7 位地址模式
    SevenBit,
    /// 10 位地址模式
    TenBit,
}

/// TX FIFO 深度
pub const I2C_TX_FIFO_DEPTH: usize = 64;

/// RX FIFO 深度
pub const I2C_RX_FIFO_DEPTH: usize = 64;

/// 默认超时计数
pub const I2C_DEFAULT_TIMEOUT: u32 = 100_000;
