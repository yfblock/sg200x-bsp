//! # SD 卡驱动常量和寄存器定义
//!
//! 本模块定义了 CV1811 芯片 SD 卡控制器相关的：
//! - 基地址常量
//! - 寄存器偏移地址
//! - 位域结构体 (使用 tock-registers)
//! - 命令类型枚举

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

// ============================================================================
// 基地址定义
// ============================================================================

/// TOP 模块基地址 (系统顶层控制寄存器)
pub const TOP_BASE: usize = 0x0300_0000;

/// SD 控制器基地址 (SDIO0)
/// 所有 SD 控制器寄存器都相对于此地址偏移
pub const SD_DRIVER_BASE: usize = 0x0431_0000;

/// 软件复位模块基地址
pub const SOFT_REST_BASE_ADDR: usize = 0x0300_3000;

/// 引脚复用 (PINMUX) 模块基地址
/// 用于配置 GPIO 引脚的功能选择
pub const PINMUX_BASE: usize = 0x0300_1000;

// ============================================================================
// TOP 模块寄存器偏移
// ============================================================================

/// SD 电源开关控制寄存器偏移
/// 用于控制 SD 卡的电源电压切换
pub const REG_TOP_SD_PWRSW_CTRL: usize = 0x1F4;

/// 块大小
pub const BLOCK_SIZE: usize = 0x200;

// ============================================================================
// 寄存器位域定义 (使用 tock-registers)
// ============================================================================

register_bitfields! [
    u32,

    /// SDMA 系统地址 / 参数寄存器 (偏移 0x00)
    pub SDMA_SYS_ADDR [
        /// SDMA 系统地址 / 参数
        ADDR OFFSET(0) NUMBITS(32) []
    ],

    /// 块大小和块计数寄存器 (偏移 0x04)
    pub BLK_SIZE_AND_CNT [
        /// 块计数 (bit16-31)
        /// 用于多块传输时指定传输的块数量
        BLK_CNT OFFSET(16) NUMBITS(16) [],
        /// SDMA 缓冲区边界 (bit12-14)
        /// 指定 SDMA 缓冲区的大小
        SDMA_BUF_BDARY OFFSET(12) NUMBITS(3) [],
        /// 传输块大小 (bit0-11)
        /// 0x001: 1 字节
        /// 0x002: 2 字节
        /// 0x200: 512 字节 (标准块大小)
        /// 0x800: 2048 字节
        XFER_BLK_SIZE OFFSET(0) NUMBITS(12) []
    ],

    /// 参数 1 寄存器 (偏移 0x08)
    pub ARGUMENT1 [
        /// 命令参数
        ARG OFFSET(0) NUMBITS(32) []
    ],

    /// 传输模式和命令寄存器 (偏移 0x0C)
    pub XFER_MODE_AND_CMD [
        /// 命令索引 (bit24-29)
        /// SD 命令号 (0-63)
        CMD_IDX OFFSET(24) NUMBITS(6) [],
        /// 命令类型 (bit22-23)
        /// 00: 普通, 01: 挂起, 10: 恢复, 11: 中止
        CMD_TYPE OFFSET(22) NUMBITS(2) [
            Normal = 0,
            Suspend = 1,
            Resume = 2,
            Abort = 3
        ],
        /// 数据存在选择 (bit21)
        /// 1: 命令带有数据传输
        DATA_PRESENT OFFSET(21) NUMBITS(1) [],
        /// 命令索引校验使能 (bit20)
        CMD_IDX_CHK_EN OFFSET(20) NUMBITS(1) [],
        /// 命令 CRC 校验使能 (bit19)
        CMD_CRC_CHK_EN OFFSET(19) NUMBITS(1) [],
        /// 子命令标志 (bit18)
        SUB_CMD_FLAG OFFSET(18) NUMBITS(1) [],
        /// 响应类型选择 (bit16-17)
        /// 00: 无响应, 01: 136位, 10: 48位, 11: 48位带忙
        RESP_TYPE OFFSET(16) NUMBITS(2) [
            NoResponse = 0,
            Response136 = 1,
            Response48 = 2,
            Response48Busy = 3
        ],
        /// 响应中断使能 (bit8)
        RESP_INT_EN OFFSET(8) NUMBITS(1) [],
        /// 响应错误校验使能 (bit7)
        RESP_ERR_CHK_EN OFFSET(7) NUMBITS(1) [],
        /// 响应类型 R1/R5 (bit6)
        RESP_TYPE_R1R5 OFFSET(6) NUMBITS(1) [],
        /// 多块选择 (bit5)
        /// 0: 单块, 1: 多块
        MULTI_BLK_SEL OFFSET(5) NUMBITS(1) [],
        /// 数据传输方向 (bit4)
        /// 0: 写 (主机到卡), 1: 读 (卡到主机)
        DAT_XFER_DIR OFFSET(4) NUMBITS(1) [
            Write = 0,
            Read = 1
        ],
        /// 自动命令使能 (bit2-3)
        /// 00: 禁用, 01: CMD12, 10: CMD23, 11: 保留
        AUTO_CMD_EN OFFSET(2) NUMBITS(2) [
            Disabled = 0,
            AutoCmd12 = 1,
            AutoCmd23 = 2
        ],
        /// 块计数使能 (bit1)
        BLK_CNT_EN OFFSET(1) NUMBITS(1) [],
        /// DMA 使能 (bit0)
        DMA_EN OFFSET(0) NUMBITS(1) []
    ],

    /// 响应寄存器 0 (偏移 0x10)
    pub RESPONSE0 [
        /// 响应位 [31:0]
        RESP OFFSET(0) NUMBITS(32) []
    ],

    /// 响应寄存器 1 (偏移 0x14)
    pub RESPONSE1 [
        /// 响应位 [63:32]
        RESP OFFSET(0) NUMBITS(32) []
    ],

    /// 响应寄存器 2 (偏移 0x18)
    pub RESPONSE2 [
        /// 响应位 [95:64]
        RESP OFFSET(0) NUMBITS(32) []
    ],

    /// 响应寄存器 3 (偏移 0x1C)
    pub RESPONSE3 [
        /// 响应位 [127:96]
        RESP OFFSET(0) NUMBITS(32) []
    ],

    /// 数据缓冲区端口寄存器 (偏移 0x20)
    pub BUF_DATA_PORT [
        /// 数据缓冲区
        DATA OFFSET(0) NUMBITS(32) []
    ],

    /// 当前状态寄存器 (偏移 0x24)
    pub PRESENT_STATE [
        /// CMD 线状态 (bit24)
        CMD_LINE_STATE OFFSET(24) NUMBITS(1) [],
        /// DAT[3:0] 线状态 (bit20-23)
        DAT_3_0_STATE OFFSET(20) NUMBITS(4) [],
        /// 卡写保护状态 (bit19, 0=可写, 1=写保护)
        CARD_WP_STATE OFFSET(19) NUMBITS(1) [],
        /// 卡检测状态 (bit18)
        CARD_CD_STATE OFFSET(18) NUMBITS(1) [],
        /// 卡状态稳定 (bit17)
        CARD_STABLE OFFSET(17) NUMBITS(1) [],
        /// 卡已插入 (bit16)
        CARD_INSERTED OFFSET(16) NUMBITS(1) [],
        /// 缓冲区读使能 (bit11)
        BUF_RD_EN OFFSET(11) NUMBITS(1) [],
        /// 缓冲区写使能 (bit10)
        BUF_WR_EN OFFSET(10) NUMBITS(1) [],
        /// 读传输活跃 (bit9)
        RD_XFER_ACTIVE OFFSET(9) NUMBITS(1) [],
        /// 写传输活跃 (bit8)
        WR_XFER_ACTIVE OFFSET(8) NUMBITS(1) [],
        /// 重新调谐请求 (bit3)
        RE_TUNE_REQ OFFSET(3) NUMBITS(1) [],
        /// DAT 线活跃 (bit2)
        DAT_LINE_ACTIVE OFFSET(2) NUMBITS(1) [],
        /// DAT 线命令禁止 (bit1, 数据传输中)
        CMD_INHIBIT_DAT OFFSET(1) NUMBITS(1) [],
        /// CMD 线命令禁止 (bit0, 命令执行中)
        CMD_INHIBIT OFFSET(0) NUMBITS(1) []
    ],

    /// 主机控制 1、电源、背景和唤醒控制寄存器 (偏移 0x28)
    pub HOST_CTL1_PWR_BG_WUP [
        /// 卡移除唤醒使能 (bit26)
        WAKEUP_ON_CARD_REMV OFFSET(26) NUMBITS(1) [],
        /// 卡插入唤醒使能 (bit25)
        WAKEUP_ON_CARD_INSERT OFFSET(25) NUMBITS(1) [],
        /// 卡中断唤醒使能 (bit24)
        WAKEUP_ON_CARD_INT OFFSET(24) NUMBITS(1) [],
        /// 块间隙中断 (bit19)
        INT_BG OFFSET(19) NUMBITS(1) [],
        /// 读等待控制 (bit18)
        READ_WAIT OFFSET(18) NUMBITS(1) [],
        /// 继续请求 (bit17)
        CONTINUE_REQ OFFSET(17) NUMBITS(1) [],
        /// 停止块间隙请求 (bit16)
        STOP_BG_REQ OFFSET(16) NUMBITS(1) [],
        /// SD 总线电压选择 (bit9-11)
        /// 111b = 3.3V, 110b = 3.0V, 101b = 1.8V
        SD_BUS_VOL_SEL OFFSET(9) NUMBITS(3) [
            V33 = 0b111,
            V30 = 0b110,
            V18 = 0b101
        ],
        /// SD 总线电源使能 (bit8)
        SD_BUS_PWR OFFSET(8) NUMBITS(1) [],
        /// 卡检测信号选择 (bit7)
        CARD_DET_SEL OFFSET(7) NUMBITS(1) [],
        /// 卡检测测试电平 (bit6)
        CARD_DET_TEST OFFSET(6) NUMBITS(1) [],
        /// 扩展数据传输宽度 (bit5, 8位模式)
        EXT_DAT_WIDTH OFFSET(5) NUMBITS(1) [],
        /// DMA 选择 (bit3-4, 00=SDMA, 01=保留, 10=ADMA2, 11=ADMA2/3)
        DMA_SEL OFFSET(3) NUMBITS(2) [
            SDMA = 0,
            Reserved = 1,
            ADMA2 = 2,
            ADMA2_3 = 3
        ],
        /// 高速模式使能 (bit2)
        HS_EN OFFSET(2) NUMBITS(1) [
            NormalSpeed = 0,
            HighSpeed = 1
        ],
        /// 数据传输宽度 (bit1, 0=1位, 1=4位)
        DAT_XFER_WIDTH OFFSET(1) NUMBITS(1) [
            Width1Bit = 0,
            Width4Bit = 1
        ],
        /// LED 控制 (bit0)
        LED_CTL OFFSET(0) NUMBITS(1) []
    ],

    /// 时钟控制和超时控制寄存器 (偏移 0x2C)
    pub CLK_CTL [
        /// DAT 线软件复位 (bit26)
        SW_RST_DAT OFFSET(26) NUMBITS(1) [],
        /// CMD 线软件复位 (bit25)
        SW_RST_CMD OFFSET(25) NUMBITS(1) [],
        /// 全部软件复位 (bit24)
        SW_RST_ALL OFFSET(24) NUMBITS(1) [],
        /// 数据超时计数值 (bit16-19)
        /// 超时时间 = TMCLK × 2^(13+tout_cnt)
        TOUT_CNT OFFSET(16) NUMBITS(4) [
            TMCLK2p13 = 0,
            TMCLK2p14 = 1,
            TMCLK2p15 = 2,
            TMCLK2p16 = 3,
            TMCLK2p17 = 4,
            TMCLK2p18 = 5,
            TMCLK2p19 = 6,
            TMCLK2p20 = 7,
            TMCLK2p21 = 8,
            TMCLK2p22 = 9,
            TMCLK2p23 = 10,
            TMCLK2p24 = 11,
            TMCLK2p25 = 12,
            TMCLK2p26 = 13,
            TMCLK2p27 = 14,
            Reserved = 15
        ],
        /// 时钟分频选择 (bit8-15)
        /// SD 时钟频率 = 基础时钟 / (2 × freq_sel)
        /// freq_sel=0 时不分频
        FREQ_SEL OFFSET(8) NUMBITS(8) [],
        /// 高位分频选择 (bit6-7)
        UP_FREQ_SEL OFFSET(6) NUMBITS(2) [],
        /// PLL 使能 (bit3)
        PLL_EN OFFSET(3) NUMBITS(1) [],
        /// SD 时钟使能 (bit2)
        /// 控制 SDCLK 引脚的时钟输出
        SD_CLK_EN OFFSET(2) NUMBITS(1) [],
        /// 内部时钟稳定 (bit1, 只读)
        /// 1 表示内部时钟已稳定
        INT_CLK_STABLE OFFSET(1) NUMBITS(1) [],
        /// 内部时钟使能 (bit0)
        INT_CLK_EN OFFSET(0) NUMBITS(1) []
    ],

    /// 普通中断和错误中断状态寄存器 (偏移 0x30)
    pub NORM_AND_ERR_INT_STS [
        /// 启动确认错误 (bit28)
        BOOT_ACK_ERR OFFSET(28) NUMBITS(1) [],
        /// 调谐错误 (bit26)
        TUNE_ERR OFFSET(26) NUMBITS(1) [],
        /// ADMA 错误 (bit25)
        ADMA_ERR OFFSET(25) NUMBITS(1) [],
        /// 自动命令错误 (bit24)
        AUTO_CMD_ERR OFFSET(24) NUMBITS(1) [],
        /// 电流限制错误 (bit23)
        CURR_LIMIT_ERR OFFSET(23) NUMBITS(1) [],
        /// 数据结束位错误 (bit22)
        DAT_ENDBIT_ERR OFFSET(22) NUMBITS(1) [],
        /// 数据 CRC 错误 (bit21)
        DAT_CRC_ERR OFFSET(21) NUMBITS(1) [],
        /// 数据超时错误 (bit20)
        DAT_TOUT_ERR OFFSET(20) NUMBITS(1) [],
        /// 命令索引错误 (bit19)
        CMD_IDX_ERR OFFSET(19) NUMBITS(1) [],
        /// 命令结束位错误 (bit18)
        CMD_ENDBIT_ERR OFFSET(18) NUMBITS(1) [],
        /// 命令 CRC 错误 (bit17)
        CMD_CRC_ERR OFFSET(17) NUMBITS(1) [],
        /// 命令超时错误 (bit16)
        CMD_TOUT_ERR OFFSET(16) NUMBITS(1) [],
        /// 错误中断 (bit15)
        /// 任何错误位置位时，此位也会置位
        ERR_INT OFFSET(15) NUMBITS(1) [],
        /// CQE 事件 (bit14)
        CQE_EVENT OFFSET(14) NUMBITS(1) [],
        /// 重新调谐事件 (bit12)
        RE_TUNE_EVENT OFFSET(12) NUMBITS(1) [],
        /// 中断 C (bit11, 厂商定义)
        INT_C OFFSET(11) NUMBITS(1) [],
        /// 中断 B (bit10, 厂商定义)
        INT_B OFFSET(10) NUMBITS(1) [],
        /// 中断 A (bit9, 厂商定义)
        INT_A OFFSET(9) NUMBITS(1) [],
        /// 卡中断 (bit8)
        CARD_INT OFFSET(8) NUMBITS(1) [],
        /// 卡移除中断 (bit7)
        CARD_REMOVE_INT OFFSET(7) NUMBITS(1) [],
        /// 卡插入中断 (bit6)
        CARD_INSERT_INT OFFSET(6) NUMBITS(1) [],
        /// 缓冲区读就绪 (bit5)
        /// 可以从缓冲区读取数据
        BUF_RRDY OFFSET(5) NUMBITS(1) [],
        /// 缓冲区写就绪 (bit4)
        /// 可以向缓冲区写入数据
        BUF_WRDY OFFSET(4) NUMBITS(1) [],
        /// DMA 中断 (bit3)
        DMA_INT OFFSET(3) NUMBITS(1) [],
        /// 块间隙事件 (bit2)
        BG_EVENT OFFSET(2) NUMBITS(1) [],
        /// 传输完成 (bit1)
        XFER_CMPL OFFSET(1) NUMBITS(1) [],
        /// 命令完成 (bit0)
        CMD_CMPL OFFSET(0) NUMBITS(1) []
    ],

    /// 普通中断和错误中断状态使能寄存器 (偏移 0x34)
    pub NORM_AND_ERR_INT_STS_EN [
        /// 与 NORM_AND_ERR_INT_STS 位域相同
        /// 用于使能对应的中断状态位
        BOOT_ACK_ERR_EN OFFSET(28) NUMBITS(1) [],
        TUNE_ERR_EN OFFSET(26) NUMBITS(1) [],
        ADMA_ERR_EN OFFSET(25) NUMBITS(1) [],
        AUTO_CMD_ERR_EN OFFSET(24) NUMBITS(1) [],
        CURR_LIMIT_ERR_EN OFFSET(23) NUMBITS(1) [],
        DAT_ENDBIT_ERR_EN OFFSET(22) NUMBITS(1) [],
        DAT_CRC_ERR_EN OFFSET(21) NUMBITS(1) [],
        DAT_TOUT_ERR_EN OFFSET(20) NUMBITS(1) [],
        CMD_IDX_ERR_EN OFFSET(19) NUMBITS(1) [],
        CMD_ENDBIT_ERR_EN OFFSET(18) NUMBITS(1) [],
        CMD_CRC_ERR_EN OFFSET(17) NUMBITS(1) [],
        CMD_TOUT_ERR_EN OFFSET(16) NUMBITS(1) [],
        CQE_EVENT_EN OFFSET(14) NUMBITS(1) [],
        RE_TUNE_EVENT_EN OFFSET(12) NUMBITS(1) [],
        INT_C_EN OFFSET(11) NUMBITS(1) [],
        INT_B_EN OFFSET(10) NUMBITS(1) [],
        INT_A_EN OFFSET(9) NUMBITS(1) [],
        CARD_INT_EN OFFSET(8) NUMBITS(1) [],
        CARD_REMOVE_INT_EN OFFSET(7) NUMBITS(1) [],
        CARD_INSERT_INT_EN OFFSET(6) NUMBITS(1) [],
        BUF_RRDY_EN OFFSET(5) NUMBITS(1) [],
        BUF_WRDY_EN OFFSET(4) NUMBITS(1) [],
        DMA_INT_EN OFFSET(3) NUMBITS(1) [],
        BG_EVENT_EN OFFSET(2) NUMBITS(1) [],
        XFER_CMPL_EN OFFSET(1) NUMBITS(1) [],
        CMD_CMPL_EN OFFSET(0) NUMBITS(1) []
    ],

    /// 普通中断和错误中断信号使能寄存器 (偏移 0x38)
    pub NORM_AND_ERR_INT_SIG_EN [
        /// 与 NORM_AND_ERR_INT_STS 位域相同
        /// 用于使能对应的中断信号
        BOOT_ACK_ERR_SIG_EN OFFSET(28) NUMBITS(1) [],
        TUNE_ERR_SIG_EN OFFSET(26) NUMBITS(1) [],
        ADMA_ERR_SIG_EN OFFSET(25) NUMBITS(1) [],
        AUTO_CMD_ERR_SIG_EN OFFSET(24) NUMBITS(1) [],
        CURR_LIMIT_ERR_SIG_EN OFFSET(23) NUMBITS(1) [],
        DAT_ENDBIT_ERR_SIG_EN OFFSET(22) NUMBITS(1) [],
        DAT_CRC_ERR_SIG_EN OFFSET(21) NUMBITS(1) [],
        DAT_TOUT_ERR_SIG_EN OFFSET(20) NUMBITS(1) [],
        CMD_IDX_ERR_SIG_EN OFFSET(19) NUMBITS(1) [],
        CMD_ENDBIT_ERR_SIG_EN OFFSET(18) NUMBITS(1) [],
        CMD_CRC_ERR_SIG_EN OFFSET(17) NUMBITS(1) [],
        CMD_TOUT_ERR_SIG_EN OFFSET(16) NUMBITS(1) [],
        CQE_EVENT_SIG_EN OFFSET(14) NUMBITS(1) [],
        RE_TUNE_EVENT_SIG_EN OFFSET(12) NUMBITS(1) [],
        INT_C_SIG_EN OFFSET(11) NUMBITS(1) [],
        INT_B_SIG_EN OFFSET(10) NUMBITS(1) [],
        INT_A_SIG_EN OFFSET(9) NUMBITS(1) [],
        CARD_INT_SIG_EN OFFSET(8) NUMBITS(1) [],
        CARD_REMOVE_INT_SIG_EN OFFSET(7) NUMBITS(1) [],
        CARD_INSERT_INT_SIG_EN OFFSET(6) NUMBITS(1) [],
        BUF_RRDY_SIG_EN OFFSET(5) NUMBITS(1) [],
        BUF_WRDY_SIG_EN OFFSET(4) NUMBITS(1) [],
        DMA_INT_SIG_EN OFFSET(3) NUMBITS(1) [],
        BG_EVENT_SIG_EN OFFSET(2) NUMBITS(1) [],
        XFER_CMPL_SIG_EN OFFSET(1) NUMBITS(1) [],
        CMD_CMPL_SIG_EN OFFSET(0) NUMBITS(1) []
    ],

    /// 自动命令错误和主机控制 2 寄存器 (偏移 0x3C)
    pub AUTO_CMD_ERR_AND_HOST_CTL2 [
        /// 当前值使能 (bit31)
        PRESENT_VAL_EN OFFSET(31) NUMBITS(1) [],
        /// 异步中断使能 (bit30)
        ASYNC_INT_EN OFFSET(30) NUMBITS(1) [],
        /// 采样时钟选择 (bit23)
        SAMPLE_CLK_SEL OFFSET(23) NUMBITS(1) [],
        /// 执行调谐 (bit22)
        EXECUTE_TUNE OFFSET(22) NUMBITS(1) [],
        /// 驱动强度选择 (bit20-21)
        DRV_SEL OFFSET(20) NUMBITS(2) [],
        /// 1.8V 信号使能 (bit19)
        EN_18_SIG OFFSET(19) NUMBITS(1) [],
        /// UHS 模式选择 (bit16-18)
        /// 000: SDR12, 001: SDR25, 010: SDR50
        /// 011: SDR104, 100: DDR50
        UHS_MODE_SEL OFFSET(16) NUMBITS(3) [
            SDR12 = 0,
            SDR25 = 1,
            SDR50 = 2,
            SDR104 = 3,
            DDR50 = 4
        ],
        /// CMD12 未发出命令 (bit7)
        CMD_NOT_ISSUE_BY_CMD12 OFFSET(7) NUMBITS(1) [],
        /// 自动命令索引错误 (bit4)
        AUTO_CMD_IDX_ERR OFFSET(4) NUMBITS(1) [],
        /// 自动命令结束位错误 (bit3)
        AUTO_CMD_ENDBIT_ERR OFFSET(3) NUMBITS(1) [],
        /// 自动命令 CRC 错误 (bit2)
        AUTO_CMD_CRC_ERR OFFSET(2) NUMBITS(1) [],
        /// 自动命令超时错误 (bit1)
        AUTO_CMD_TOUT_ERR OFFSET(1) NUMBITS(1) [],
        /// 自动 CMD12 未执行 (bit0)
        AUTO_CMD12_NO_EXE OFFSET(0) NUMBITS(1) []
    ],

    /// 能力寄存器 1 (偏移 0x40)
    pub CAPABILITIES1 [
        /// 插槽类型 (bit30-31)
        /// 00: 可移除卡, 01: 嵌入式, 10: 共享总线
        SLOT_TYPE OFFSET(30) NUMBITS(2) [
            Removable = 0,
            Embedded = 1,
            SharedBus = 2
        ],
        /// 异步中断支持 (bit29)
        ASYNC_INT_SUPPORT OFFSET(29) NUMBITS(1) [],
        /// 64 位系统总线支持 (bit28)
        BUS64_SUPPORT OFFSET(28) NUMBITS(1) [],
        /// 1.8V 电压支持 (bit26)
        V18_SUPPORT OFFSET(26) NUMBITS(1) [],
        /// 3.0V 电压支持 (bit25)
        V30_SUPPORT OFFSET(25) NUMBITS(1) [],
        /// 3.3V 电压支持 (bit24)
        V33_SUPPORT OFFSET(24) NUMBITS(1) [],
        /// 挂起/恢复支持 (bit23)
        SUSP_RES_SUPPORT OFFSET(23) NUMBITS(1) [],
        /// SDMA 支持 (bit22)
        SDMA_SUPPORT OFFSET(22) NUMBITS(1) [],
        /// 高速模式支持 (bit21)
        HS_SUPPORT OFFSET(21) NUMBITS(1) [],
        /// ADMA2 支持 (bit19)
        ADMA2_SUPPORT OFFSET(19) NUMBITS(1) [],
        /// 嵌入式 8 位支持 (bit18)
        EMBEDDED_8BIT OFFSET(18) NUMBITS(1) [],
        /// 最大块长度 (bit16-17)
        /// 00: 512, 01: 1024, 10: 2048
        MAX_BLK_LEN OFFSET(16) NUMBITS(2) [],
        /// 基础时钟频率 (bit8-15, MHz)
        BASE_CLK_FREQ OFFSET(8) NUMBITS(8) [],
        /// 超时时钟单位 (bit7, 0: KHz, 1: MHz)
        TOUT_CLK_UNIT OFFSET(7) NUMBITS(1) [],
        /// 超时时钟频率 (bit0-5)
        TOUT_CLK_FREQ OFFSET(0) NUMBITS(6) []
    ],

    /// 能力寄存器 2 (偏移 0x44)
    pub CAPABILITIES2 [
        /// 时钟倍频器 (bit16-23)
        CLK_MULTIPLIER OFFSET(16) NUMBITS(8) [],
        /// 重新调谐模式 (bit14-15)
        RETUNE_MODE OFFSET(14) NUMBITS(2) [],
        /// SDR50 需要调谐 (bit13)
        TUNE_SDR50 OFFSET(13) NUMBITS(1) [],
        /// 重新调谐定时器 (bit8-11)
        RETUNE_TIMER OFFSET(8) NUMBITS(4) [],
        /// 驱动类型 D 支持 (bit6)
        DRV_D_SUPPORT OFFSET(6) NUMBITS(1) [],
        /// 驱动类型 C 支持 (bit5)
        DRV_C_SUPPORT OFFSET(5) NUMBITS(1) [],
        /// 驱动类型 A 支持 (bit4)
        DRV_A_SUPPORT OFFSET(4) NUMBITS(1) [],
        /// DDR50 支持 (bit2)
        DDR50_SUPPORT OFFSET(2) NUMBITS(1) [],
        /// SDR104 支持 (bit1)
        SDR104_SUPPORT OFFSET(1) NUMBITS(1) [],
        /// SDR50 支持 (bit0)
        SDR50_SUPPORT OFFSET(0) NUMBITS(1) []
    ],

    /// eMMC 控制寄存器 (偏移 0x200)
    pub EMMC_CTL [
        /// 定时器时钟选择 (bit13)
        TIMER_CLK_SEL OFFSET(13) NUMBITS(1) [],
        /// CQE 预取禁用 (bit10)
        CQE_PREFETCH_DISABLE OFFSET(10) NUMBITS(1) [],
        /// CQE 算法选择 (bit9)
        CQE_ALGO_SEL OFFSET(9) NUMBITS(1) [],
        /// eMMC RST_n 输出使能 (bit6)
        EMMC_RSTN_OEN OFFSET(6) NUMBITS(1) [],
        /// eMMC RST_n 信号值 (bit5)
        EMMC_RSTN OFFSET(5) NUMBITS(1) [],
        /// 禁用数据 CRC 校验 (bit3)
        DISABLE_DATA_CRC_CHECK OFFSET(3) NUMBITS(1) [],
        /// 时钟自由运行使能 (bit2)
        CLK_FREE_EN OFFSET(2) NUMBITS(1) [],
        /// 1T 延迟使能 (bit1)
        LATANCY_1T OFFSET(1) NUMBITS(1) [],
        /// eMMC 功能使能 (bit0)
        EMMC_FUNC_EN OFFSET(0) NUMBITS(1) []
    ],

    /// TOP SD 电源开关控制寄存器
    pub TOP_SD_PWRSW_CTRL [
        /// 电源开关控制值
        PWRSW_CTRL OFFSET(0) NUMBITS(8) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// SDMMC 控制器寄存器组
    ///
    /// 基地址: SD_DRIVER_BASE (0x0431_0000)
    pub SdmmcRegisters {
        /// SDMA 系统地址 / 参数寄存器 (偏移 0x00)
        (0x000 => pub sdma_sys_addr: ReadWrite<u32, SDMA_SYS_ADDR::Register>),

        /// 块大小和块计数寄存器 (偏移 0x04)
        (0x004 => pub blk_size_and_cnt: ReadWrite<u32, BLK_SIZE_AND_CNT::Register>),

        /// 参数 1 寄存器 (偏移 0x08)
        (0x008 => pub argument1: ReadWrite<u32, ARGUMENT1::Register>),

        /// 传输模式和命令寄存器 (偏移 0x0C)
        (0x00C => pub xfer_mode_and_cmd: ReadWrite<u32, XFER_MODE_AND_CMD::Register>),

        /// 响应寄存器 0 (偏移 0x10)
        (0x010 => pub response0: ReadWrite<u32, RESPONSE0::Register>),

        /// 响应寄存器 1 (偏移 0x14)
        (0x014 => pub response1: ReadWrite<u32, RESPONSE1::Register>),

        /// 响应寄存器 2 (偏移 0x18)
        (0x018 => pub response2: ReadWrite<u32, RESPONSE2::Register>),

        /// 响应寄存器 3 (偏移 0x1C)
        (0x01C => pub response3: ReadWrite<u32, RESPONSE3::Register>),

        /// 数据缓冲区端口寄存器 (偏移 0x20)
        (0x020 => pub buf_data_port: ReadWrite<u32, BUF_DATA_PORT::Register>),

        /// 当前状态寄存器 (偏移 0x24)
        (0x024 => pub present_state: ReadWrite<u32, PRESENT_STATE::Register>),

        /// 主机控制 1、电源、背景和唤醒控制寄存器 (偏移 0x28)
        (0x028 => pub host_ctl1_pwr_bg_wup: ReadWrite<u32, HOST_CTL1_PWR_BG_WUP::Register>),

        /// 时钟控制和超时控制寄存器 (偏移 0x2C)
        (0x02C => pub clk_ctl: ReadWrite<u32, CLK_CTL::Register>),

        /// 普通中断和错误中断状态寄存器 (偏移 0x30)
        (0x030 => pub norm_and_err_int_sts: ReadWrite<u32, NORM_AND_ERR_INT_STS::Register>),

        /// 普通中断和错误中断状态使能寄存器 (偏移 0x34)
        (0x034 => pub norm_and_err_int_sts_en: ReadWrite<u32, NORM_AND_ERR_INT_STS_EN::Register>),

        /// 普通中断和错误中断信号使能寄存器 (偏移 0x38)
        (0x038 => pub norm_and_err_int_sig_en: ReadWrite<u32, NORM_AND_ERR_INT_SIG_EN::Register>),

        /// 自动命令错误和主机控制 2 寄存器 (偏移 0x3C)
        (0x03C => pub auto_cmd_err_and_host_ctl2: ReadWrite<u32, AUTO_CMD_ERR_AND_HOST_CTL2::Register>),

        /// 能力寄存器 1 (偏移 0x40)
        (0x040 => pub capabilities1: ReadWrite<u32, CAPABILITIES1::Register>),

        /// 能力寄存器 2 (偏移 0x44)
        (0x044 => pub capabilities2: ReadWrite<u32, CAPABILITIES2::Register>),

        /// 保留 (偏移 0x48-0x1FC)
        (0x048 => _reserved0),

        /// eMMC 控制寄存器 (偏移 0x200)
        (0x200 => pub emmc_ctl: ReadWrite<u32, EMMC_CTL::Register>),

        /// 结束标记
        (0x204 => @END),
    }
}

register_structs! {
    /// TOP 模块寄存器组 (部分)
    ///
    /// 基地址: TOP_BASE (0x0300_0000)
    pub TopRegisters {
        /// 保留 (偏移 0x00-0x1F0)
        (0x000 => _reserved0),

        /// SD 电源开关控制寄存器 (偏移 0x1F4)
        (0x1F4 => pub sd_pwrsw_ctrl: ReadWrite<u32, TOP_SD_PWRSW_CTRL::Register>),

        /// 结束标记
        (0x1F8 => @END),
    }
}

// ============================================================================
// 错误类型和命令类型定义
// ============================================================================

/// 命令执行错误类型
#[derive(Debug)]
pub enum CmdError {
    /// 中断错误
    /// 表示在命令执行或数据传输过程中发生了错误中断
    IntError,
    /// 命令超时错误
    CmdTimeout,
    /// 数据超时错误
    DataTimeout,
    /// CRC 错误
    CrcError,
}

/// SD 命令类型
///
/// SD 卡协议定义了两类命令：
/// - CMD: 标准命令 (CMD0-CMD63)
/// - ACMD: 应用特定命令，需要先发送 CMD55
#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    /// 标准命令
    /// 参数为命令索引 (0-63)
    CMD(u8),
    /// 应用命令
    /// 参数为命令索引，发送前需要先发送 CMD55
    ACMD(u8),
}

impl CommandType {
    /// 获取命令索引号
    ///
    /// # 返回值
    /// 命令的数字索引 (0-63)
    pub fn num(&self) -> u8 {
        match self {
            CommandType::CMD(t) => *t,
            CommandType::ACMD(t) => *t,
        }
    }
}

/// SD 卡电源电压等级
///
/// 用于配置 SD 卡总线的供电电压
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerLevel {
    /// 3.3V 电压 (标准 SD 卡电压)
    V33,
    /// 3.0V 电压
    V30,
    /// 1.8V 电压 (UHS-I 模式)
    V18,
    /// 关闭电源
    Close,
}

/// 响应类型
#[derive(Debug, Clone, Copy)]
pub enum ResponseType {
    /// 无响应
    None,
    /// R1 响应 (48位，带 CRC 和索引校验)
    R1,
    /// R1b 响应 (48位，带忙检测)
    R1b,
    /// R2 响应 (136位，CID/CSD)
    R2,
    /// R3 响应 (48位，OCR，无校验)
    R3,
    /// R6 响应 (48位，RCA)
    R6,
    /// R7 响应 (48位，接口条件)
    R7,
}
