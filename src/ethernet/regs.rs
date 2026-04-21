//! Synopsys DesignWare MAC（DWMAC 3.70a）寄存器布局。
//!
//! 用 [`tock_registers`] 提供类型化 MMIO，整个 ethernet 子模块统一通过
//! 本文件的 [`gmac_regs`] / [`dma_regs`] 视图访问寄存器，**禁止**再写
//! `read_volatile/write_volatile`。
//!
//! 寄存器偏移与位域命名对齐 Linux `drivers/net/ethernet/stmicro/stmmac/dwmac1000.h`：
//! - GMAC 区段：base + `0x0000..0x0FFF`
//! - DMA  区段：base + `0x1000..0x1FFF`
//!
//! 本驱动只用到了 SG2002 EPHY/MAC 实际生效的寄存器，未使用的偏移用
//! `_reserved_*` 跳过，方便后续按需扩展。

use tock_registers::{register_bitfields, register_structs};
use tock_registers::registers::ReadWrite;

register_bitfields![u32,
    /// GMAC_CONTROL @ 0x0000：MAC 总开关。
    pub MacControl [
        /// Receiver Enable.
        RE          OFFSET(2)  NUMBITS(1) [],
        /// Transmitter Enable.
        TE          OFFSET(3)  NUMBITS(1) [],
        /// Deferral Check.
        DC          OFFSET(4)  NUMBITS(1) [],
        /// Auto-pad/CRC stripping for IEEE 802.3.
        ACS         OFFSET(7)  NUMBITS(1) [],
        /// Disable CRC check.
        DCRS        OFFSET(16) NUMBITS(1) [],
        /// Port select: 0 = GMII (1 Gbps)，1 = MII (10/100)。
        PS          OFFSET(15) NUMBITS(1) [],
        /// Speed: 0 = 10M，1 = 100M（仅 MII 模式有效）。
        FES         OFFSET(14) NUMBITS(1) [],
        /// Full duplex mode.
        DM          OFFSET(11) NUMBITS(1) [],
    ],

    /// GMAC_FRAME_FILTER @ 0x0004：地址过滤策略。
    pub FrameFilter [
        /// Promiscuous Mode（接收所有帧）。
        PR          OFFSET(0)  NUMBITS(1) [],
        /// Receive All（连同 CRC 错误的也收）。
        RA          OFFSET(31) NUMBITS(1) [],
    ],

    /// GMAC_MII_ADDR @ 0x0010：MDIO 地址/控制。
    pub MiiAddr [
        /// 1 = 写 MII，0 = 读。
        MII_WRITE   OFFSET(1)  NUMBITS(1) [],
        /// MDC 时钟分频选择（CSR clock range）。
        MII_CLK_CSR OFFSET(2)  NUMBITS(4) [],
        /// PHY register address (5 bits)。
        MII_REG     OFFSET(6)  NUMBITS(5) [],
        /// PHY device address (5 bits)。
        MII_PHY     OFFSET(11) NUMBITS(5) [],
        /// 1 = busy（GMAC 正在执行 MDIO 事务）。
        MII_BUSY    OFFSET(0)  NUMBITS(1) [],
    ],

    /// GMAC_ADDR0_HIGH @ 0x0040：MAC 地址高 16 bit。bit31 必须置 1（AE）。
    pub Addr0High [
        ADDR_HI     OFFSET(0)  NUMBITS(16) [],
        ADDR_ENABLE OFFSET(31) NUMBITS(1) [],
    ],

    /// DMA_BUS_MODE @ 0x1000。
    pub DmaBusMode [
        /// Software Reset。写 1 触发 DMA 软复位，硬件清零表示完成。
        SWR  OFFSET(0)  NUMBITS(1) [],
        /// Descriptor Skip Length（单位：字 = 4 字节）。
        DSL  OFFSET(2)  NUMBITS(5) [],
        /// Programmable Burst Length。
        PBL  OFFSET(8)  NUMBITS(6) [],
        /// Fixed Burst Length（与 AAL 一起决定 AXI burst 行为）。
        FB   OFFSET(16) NUMBITS(1) [],
        /// Address-Aligned Beats。
        AAL  OFFSET(25) NUMBITS(1) [],
    ],

    /// DMA_STATUS @ 0x1014：W1C 中断状态位（本驱动只用 RI/NIS）。
    pub DmaStatus [
        /// Transmit Interrupt。
        TI  OFFSET(0)  NUMBITS(1) [],
        /// Receive Interrupt。
        RI  OFFSET(6)  NUMBITS(1) [],
        /// Normal Interrupt Summary。
        NIS OFFSET(16) NUMBITS(1) [],
    ],

    /// DMA_OPERATION @ 0x1018：DMA 收发使能 + FIFO 模式。
    pub DmaOperation [
        /// Start/Stop Receive。
        SR  OFFSET(1)  NUMBITS(1) [],
        /// Operate on Second Frame（Tx 流水）。
        OSF OFFSET(2)  NUMBITS(1) [],
        /// Start/Stop Transmission。
        ST  OFFSET(13) NUMBITS(1) [],
        /// Flush Transmit FIFO（自清零）。
        FTF OFFSET(20) NUMBITS(1) [],
        /// Transmit Store-and-Forward。
        TSF OFFSET(21) NUMBITS(1) [],
        /// Receive Store-and-Forward。
        RSF OFFSET(25) NUMBITS(1) [],
    ],
];

register_structs! {
    /// GMAC（DWMAC 3.70a）顶层寄存器（base + 0x0000..0x0FFF）。
    pub Dwc3GmacRegs {
        /// 0x0000 GMAC_CONTROL
        (0x0000 => pub mac_control:   ReadWrite<u32, MacControl::Register>),
        /// 0x0004 GMAC_FRAME_FILTER
        (0x0004 => pub frame_filter:  ReadWrite<u32, FrameFilter::Register>),
        /// 0x0008 GMAC_HASH_HIGH
        (0x0008 => pub hash_high:     ReadWrite<u32>),
        /// 0x000C GMAC_HASH_LOW
        (0x000C => pub hash_low:      ReadWrite<u32>),
        /// 0x0010 GMAC_MII_ADDR
        (0x0010 => pub mii_addr:      ReadWrite<u32, MiiAddr::Register>),
        /// 0x0014 GMAC_MII_DATA（仅低 16 bit）
        (0x0014 => pub mii_data:      ReadWrite<u32>),
        (0x0018 => _reserved0),
        /// 0x0020 DWMAC version 寄存器（只读）。
        (0x0020 => pub version:       ReadWrite<u32>),
        (0x0024 => _reserved1),
        /// 0x003C 中断屏蔽。
        (0x003C => pub int_mask:      ReadWrite<u32>),
        /// 0x0040 / 0x0044 MAC 地址 0。
        (0x0040 => pub addr0_high:    ReadWrite<u32, Addr0High::Register>),
        (0x0044 => pub addr0_low:     ReadWrite<u32>),
        (0x0048 => _reserved2),
        /// 0x0100 MMC counter control（写 1 复位计数器）。
        (0x0100 => pub mmc_cntrl:     ReadWrite<u32>),
        (0x0104 => _reserved3),
        (0x1000 => @END),
    }
}

register_structs! {
    /// GMAC DMA 子模块寄存器（base + 0x1000..0x1FFF）。
    pub Dwc3DmaRegs {
        /// 0x0000(=base+0x1000) DMA_BUS_MODE。
        (0x0000 => pub bus_mode:      ReadWrite<u32, DmaBusMode::Register>),
        /// 0x0004 DMA_TX_POLL（写任意值唤醒 TX DMA）。
        (0x0004 => pub tx_poll:       ReadWrite<u32>),
        /// 0x0008 DMA_RX_POLL。
        (0x0008 => pub rx_poll:       ReadWrite<u32>),
        /// 0x000C DMA_RX_BASE_ADDR：RX 描述符环物理基址（32 bit）。
        (0x000C => pub rx_base:       ReadWrite<u32>),
        /// 0x0010 DMA_TX_BASE_ADDR：TX 描述符环物理基址（32 bit）。
        (0x0010 => pub tx_base:       ReadWrite<u32>),
        /// 0x0014 DMA_STATUS（W1C）。
        (0x0014 => pub status:        ReadWrite<u32, DmaStatus::Register>),
        /// 0x0018 DMA_OPERATION。
        (0x0018 => pub operation:     ReadWrite<u32, DmaOperation::Register>),
        /// 0x001C DMA_INTR_ENA。
        (0x001C => pub intr_ena:      ReadWrite<u32>),
        (0x0020 => @END),
    }
}

/// MDIO `MII_CLK_CSR` 字段值：CSR 时钟落在 60–100 MHz 时使用 `/42` 分频（值 = 4）。
pub const MII_CLK_CSR_60_100M_DIV42: u32 = 0x4;

/// 用 GMAC base 地址构造 [`Dwc3GmacRegs`] 视图。
///
/// # Safety
/// `base` 必须是经过虚拟映射、长度 ≥ 0x100 的 GMAC MMIO 区域。整个驱动声明
/// 同时只持有一个 [`crate::ethernet::CvitekEthNic`]，不会出现并发访问。
#[inline]
pub unsafe fn gmac_regs(base: usize) -> &'static Dwc3GmacRegs {
    unsafe { &*(base as *const Dwc3GmacRegs) }
}

/// 用 GMAC base 地址构造 [`Dwc3DmaRegs`] 视图（自动加上 `+0x1000` 偏移）。
///
/// # Safety
/// 同 [`gmac_regs`]。
#[inline]
pub unsafe fn dma_regs(base: usize) -> &'static Dwc3DmaRegs {
    unsafe { &*((base + 0x1000) as *const Dwc3DmaRegs) }
}
