//! DWC2 寄存器布局（Synopsys DesignWare OTG 2.0），使用 `tock-registers` 提供
//! 类型化 MMIO 访问。布局对齐 Linux `drivers/usb/dwc2/hw.h`。

#![allow(dead_code)]

use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

register_bitfields![u32,
    /// AHB 总线配置（DMA、突发长度、全局中断使能）。
    pub GAHBCFG [
        GLBL_INTR_EN OFFSET(0) NUMBITS(1) [],
        HBSTLEN OFFSET(1) NUMBITS(4) [
            Single = 0,
            Incr = 1,
            Incr4 = 3,
            Incr8 = 5,
            Incr16 = 7,
        ],
        DMA_EN OFFSET(5) NUMBITS(1) [],
    ],
    /// USB 接口配置（TOUTCAL、UTMI 宽度、Force Host）。
    pub GUSBCFG [
        TOUTCAL OFFSET(0) NUMBITS(3) [],
        PHYIF16 OFFSET(3) NUMBITS(1) [],
        ULPI_UTMI_SEL OFFSET(4) NUMBITS(1) [],
        FORCEHOSTMODE OFFSET(29) NUMBITS(1) [],
    ],
    /// 复位与 FIFO flush。
    pub GRSTCTL [
        CSFTRST OFFSET(0) NUMBITS(1) [],
        RXFFLSH OFFSET(4) NUMBITS(1) [],
        TXFFLSH OFFSET(5) NUMBITS(1) [],
        TXFNUM OFFSET(6) NUMBITS(5) [],
        CSFTRST_DONE OFFSET(29) NUMBITS(1) [],
        AHBIDLE OFFSET(31) NUMBITS(1) [],
    ],
    /// 全局中断状态。
    pub GINTSTS [
        CURMODE_HOST OFFSET(0) NUMBITS(1) [],
    ],
    /// 全局中断掩码。
    pub GINTMSK [
        HCHINT OFFSET(25) NUMBITS(1) [],
    ],
    /// OTG 控制寄存器（`dr_mode=otg` 时主机会话 override 用）。
    pub GOTGCTL [
        VBVALOEN OFFSET(2) NUMBITS(1) [],
        VBVALOVAL OFFSET(3) NUMBITS(1) [],
        AVALOEN OFFSET(4) NUMBITS(1) [],
        AVALOVAL OFFSET(5) NUMBITS(1) [],
        DBNCE_FLTR_BYPASS OFFSET(15) NUMBITS(1) [],
    ],
    /// 硬件配置 2（包含 ARCH、Host channel 数）。
    pub GHWCFG2 [
        ARCH OFFSET(3) NUMBITS(2) [],
        NUM_HOST_CHAN OFFSET(14) NUMBITS(4) [],
    ],
    /// 硬件配置 3（DFIFO 总深度）。
    pub GHWCFG3 [
        DFIFO_DEPTH OFFSET(16) NUMBITS(16) [],
    ],
    /// 硬件配置 4（专用 FIFO 标志、UTMI PHY 数据宽度）。
    pub GHWCFG4 [
        DED_FIFO_EN OFFSET(25) NUMBITS(1) [],
        /// `UTMI_PHY_DATA_WIDTH`：bit 14..15
        ///   0b00 = 8-bit only
        ///   0b01 = 16-bit only
        ///   0b10 = 8-bit / 16-bit programmable（由 `GUSBCFG.PHYIF16` 决定）
        UTMI_PHY_DATA_WIDTH OFFSET(14) NUMBITS(2) [
            Eight = 0,
            Sixteen = 1,
            Programmable = 2,
        ],
    ],
    /// 动态 FIFO 配置（EP info base）。
    pub GDFIFOCFG [
        GDFIFOCFG OFFSET(0) NUMBITS(16) [],
        EPINFOBASE OFFSET(16) NUMBITS(16) [],
    ],
    /// 主机配置（FS/LS、PHY 时钟）。
    pub HCFG [
        FSLSPCLKSEL OFFSET(0) NUMBITS(2) [
            Pll48Mhz = 1,
        ],
        FSLSSUPP OFFSET(2) NUMBITS(1) [],
    ],
    /// 主机端口控制状态（HPRT0）。
    pub HPRT0 [
        CONNSTS OFFSET(0) NUMBITS(1) [],
        CONNDET OFFSET(1) NUMBITS(1) [],
        ENA OFFSET(2) NUMBITS(1) [],
        OVRCURACT OFFSET(4) NUMBITS(1) [],
        RST OFFSET(8) NUMBITS(1) [],
        LNSTS OFFSET(10) NUMBITS(2) [],
        PWR OFFSET(12) NUMBITS(1) [],
        SPD OFFSET(17) NUMBITS(2) [
            HighSpeed = 0,
            FullSpeed = 1,
            LowSpeed = 2,
        ],
    ],
    /// 主机帧编号（HFNUM）。
    pub HFNUM [
        FRNUM OFFSET(0) NUMBITS(16) [],
        FRREM OFFSET(16) NUMBITS(16) [],
    ],
    /// 主机所有通道中断掩码（HAINTMSK）。
    pub HAINTMSK [
        CHINT OFFSET(0) NUMBITS(16) [],
    ],
    /// 主机通道字符（HCCHAR）：MPS、EP、方向、类型、设备地址、奇偶帧、CHENA/CHDIS。
    pub HCCHAR [
        MPS OFFSET(0) NUMBITS(11) [],
        EPNUM OFFSET(11) NUMBITS(4) [],
        EPDIR OFFSET(15) NUMBITS(1) [],
        LSPDDEV OFFSET(17) NUMBITS(1) [],
        EPTYPE OFFSET(18) NUMBITS(2) [
            Control = 0,
            Isochronous = 1,
            Bulk = 2,
            Interrupt = 3,
        ],
        MC OFFSET(20) NUMBITS(2) [],
        DEVADDR OFFSET(22) NUMBITS(7) [],
        ODDFRM OFFSET(29) NUMBITS(1) [],
        CHDIS OFFSET(30) NUMBITS(1) [],
        CHENA OFFSET(31) NUMBITS(1) [],
    ],
    /// 主机通道中断（HCINT）。
    pub HCINT [
        XFERCOMPL OFFSET(0) NUMBITS(1) [],
        CHHLTD OFFSET(1) NUMBITS(1) [],
        AHBERR OFFSET(2) NUMBITS(1) [],
        STALL OFFSET(3) NUMBITS(1) [],
        NAK OFFSET(4) NUMBITS(1) [],
        ACK OFFSET(5) NUMBITS(1) [],
        NYET OFFSET(6) NUMBITS(1) [],
        XACTERR OFFSET(7) NUMBITS(1) [],
        BBLERR OFFSET(8) NUMBITS(1) [],
        FRMOVRN OFFSET(9) NUMBITS(1) [],
        DATATGLERR OFFSET(10) NUMBITS(1) [],
    ],
    /// 主机通道传输大小（HCTSIZ）。
    pub HCTSIZ [
        XFERSIZE OFFSET(0) NUMBITS(19) [],
        PKTCNT OFFSET(19) NUMBITS(10) [],
        PID OFFSET(29) NUMBITS(2) [
            Data0 = 0,
            Data2 = 1,
            Data1 = 2,
            Setup = 3,
        ],
    ],
];

register_structs! {
    /// 单个主机通道寄存器块（占 0x20 字节，基址 = `Dwc2Regs.hc_base + n * 0x20`）。
    pub Dwc2HostChannel {
        (0x00 => pub hcchar: ReadWrite<u32, HCCHAR::Register>),
        (0x04 => pub hcsplt: ReadWrite<u32>),
        (0x08 => pub hcint: ReadWrite<u32, HCINT::Register>),
        (0x0c => pub hcintmsk: ReadWrite<u32, HCINT::Register>),
        (0x10 => pub hctsiz: ReadWrite<u32, HCTSIZ::Register>),
        (0x14 => pub hcdma: ReadWrite<u32>),
        (0x18 => _reserved18),
        (0x1c => pub hcdmab: ReadOnly<u32>),
        (0x20 => @END),
    }
}

pub const DWC2_MAX_HOST_CHANNELS: usize = 16;

register_structs! {
    /// 完整 DWC2 寄存器映射（仅声明本驱动使用到的字段；其余区段留作 reserved）。
    pub Dwc2Regs {
        (0x000 => pub gotgctl: ReadWrite<u32, GOTGCTL::Register>),
        (0x004 => pub gotgint: ReadWrite<u32>),
        (0x008 => pub gahbcfg: ReadWrite<u32, GAHBCFG::Register>),
        (0x00c => pub gusbcfg: ReadWrite<u32, GUSBCFG::Register>),
        (0x010 => pub grstctl: ReadWrite<u32, GRSTCTL::Register>),
        (0x014 => pub gintsts: ReadWrite<u32, GINTSTS::Register>),
        (0x018 => pub gintmsk: ReadWrite<u32, GINTMSK::Register>),
        (0x01c => pub grxstsr: ReadOnly<u32>),
        (0x020 => pub grxstsp: ReadOnly<u32>),
        (0x024 => pub grxfsiz: ReadWrite<u32>),
        (0x028 => pub gnptxfsiz: ReadWrite<u32>),
        (0x02c => _reserved02c),
        (0x040 => pub gsnpsid: ReadOnly<u32>),
        (0x044 => pub ghwcfg1: ReadOnly<u32>),
        (0x048 => pub ghwcfg2: ReadOnly<u32, GHWCFG2::Register>),
        (0x04c => pub ghwcfg3: ReadOnly<u32, GHWCFG3::Register>),
        (0x050 => pub ghwcfg4: ReadOnly<u32, GHWCFG4::Register>),
        (0x054 => _reserved054),
        (0x05c => pub gdfifocfg: ReadWrite<u32, GDFIFOCFG::Register>),
        (0x060 => _reserved060),
        (0x100 => pub hptxfsiz: ReadWrite<u32>),
        (0x104 => _reserved104),
        (0x400 => pub hcfg: ReadWrite<u32, HCFG::Register>),
        (0x404 => pub hfir: ReadWrite<u32>),
        (0x408 => pub hfnum: ReadOnly<u32, HFNUM::Register>),
        (0x40c => _reserved40c),
        (0x418 => pub haintmsk: ReadWrite<u32, HAINTMSK::Register>),
        (0x41c => _reserved41c),
        (0x440 => pub hprt0: ReadWrite<u32, HPRT0::Register>),
        (0x444 => _reserved444),
        (0x500 => pub hc: [Dwc2HostChannel; DWC2_MAX_HOST_CHANNELS]),
        (0x700 => _reserved700),
        (0xe00 => pub pcgctl: ReadWrite<u32>),
        (0xe04 => @END),
    }
}

register_bitfields![u32,
    /// CV182x 片内 USB2 PHY `REG014`（UTMI 控制覆盖，与 vendor `platform.c` 字段一致）：
    /// - `UTMI_OVERRIDE` (bit0)：1 = 软件接管 UTMI 信号；0 = 由 DWC2 接管（host 路径必须为 0）
    /// - `OPMODE` (bit1..2)：UTMI OPMODE，host 路径为 00 (Normal)
    /// - `XCVRSEL` (bit3..4)：UTMI XCVRSEL，host 路径为 00 (HS)，01=FS、10=LS、11=FS+LS
    /// - `TERMSEL` (bit5)：1 = 强制 FS termination（设为 1 会禁掉 HS chirp）
    /// - `DPPULLDOWN/DMPULLDOWN` (bit6/bit7)：host 模式 PHY 必须给 D+/D- 加下拉，此处为 SW override 使能
    /// - `UTMI_RESET` (bit8)：UTMI 总线复位
    pub PhyReg014 [
        UTMI_OVERRIDE OFFSET(0) NUMBITS(1) [],
        OPMODE OFFSET(1) NUMBITS(2) [
            Normal = 0,
            NonDriving = 1,
            DisableBitStuffNRZI = 2,
        ],
        XCVRSEL OFFSET(3) NUMBITS(2) [
            HighSpeed = 0,
            FullSpeed = 1,
            LowSpeed = 2,
            FsLs = 3,
        ],
        TERMSEL OFFSET(5) NUMBITS(1) [],
        DPPULLDOWN OFFSET(6) NUMBITS(1) [],
        DMPULLDOWN OFFSET(7) NUMBITS(1) [],
        UTMI_RESET OFFSET(8) NUMBITS(1) [],
    ],
];

register_structs! {
    /// CV182x 片内 USB2 PHY MMIO（DTS `usb@04340000` 第二段 `reg=<0x03006000 0x58>`）。
    /// 字段名对齐 vendor Linux `drivers/usb/dwc2/platform.c` 中的 `REGxxx` 宏。
    pub Cv182xUsb2Phy {
        (0x000 => pub reg000: ReadWrite<u32>),
        (0x004 => pub reg004: ReadWrite<u32>),
        (0x008 => pub reg008: ReadWrite<u32>),
        (0x00c => pub reg00c: ReadWrite<u32>),
        (0x010 => pub reg010: ReadWrite<u32>),
        (0x014 => pub reg014: ReadWrite<u32, PhyReg014::Register>),
        (0x018 => pub reg018: ReadWrite<u32>),
        (0x01c => pub reg01c: ReadWrite<u32>),
        (0x020 => pub reg020: ReadWrite<u32>),
        (0x024 => pub reg024: ReadWrite<u32>),
        (0x028 => pub reg028: ReadWrite<u32>),
        (0x02c => pub reg02c: ReadWrite<u32>),
        (0x030 => pub reg030: ReadWrite<u32>),
        (0x034 => pub reg034: ReadWrite<u32>),
        (0x038 => pub reg038: ReadWrite<u32>),
        (0x03c => pub reg03c: ReadWrite<u32>),
        (0x040 => pub reg040: ReadWrite<u32>),
        (0x044 => pub reg044: ReadWrite<u32>),
        (0x048 => pub reg048: ReadWrite<u32>),
        (0x04c => pub reg04c: ReadWrite<u32>),
        (0x050 => pub reg050: ReadWrite<u32>),
        (0x054 => pub reg054: ReadWrite<u32>),
        (0x058 => @END),
    }
}
