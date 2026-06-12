//! DWC2 寄存器布局（Synopsys DesignWare OTG 2.0），使用 `tock-registers` 提供
//! 类型化 MMIO 访问。偏移与位定义对齐 Linux `drivers/usb/dwc2/hw.h`。
//!
//! # 访问约定
//!
//! - **W1C**（write-1-to-clear）：读为 1 表示事件已发生；写 1 清除该位（写 0 无影响）。
//!   典型：`GINTSTS`、`HCINT`、`HPRT0` 的部分状态位。
//! - **R/W1C**：读反映当前状态；写 1 触发清除或翻转语义（`HPRT0.ENA` 写 1 = disable port）。
//! - 本文件仅声明驱动实际用到的寄存器；未映射区域在 [`Dwc2Regs`] 中留作 `_reserved*`。

#![allow(dead_code)]

use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

register_bitfields![u32,
    /// Global AHB Configuration
    ///
    /// AHB 主接口配置，偏移 `0x008`。
    ///
    /// 控制内部 DMA 使能、INCR 突发长度以及全局中断输出。
    pub GAHBCFG [
        /// Global Interrupt Enable
        ///
        /// 全局中断使能，RW；0 = 屏蔽所有 GINTSTS 到 CPU 的输出。
        GLBL_INTR_EN OFFSET(0) NUMBITS(1) [],
        /// AHB Burst Length/Type
        ///
        /// AHB 突发传输类型/长度，RW；主机 DMA 路径常用 Incr4。
        HBSTLEN OFFSET(1) NUMBITS(4) [
            /// Single Transfer   单次非突发传输
            Single = 0,
            /// Incrementing Burst   不定长递增突发
            Incr = 1,
            /// Incrementing Burst of 4   4-beat 递增突发
            Incr4 = 3,
            /// Incrementing Burst of 8   8-beat 递增突发
            Incr8 = 5,
            /// Incrementing Burst of 16   16-beat 递增突发
            Incr16 = 7,
        ],
        /// DMA Enable
        ///
        /// 内部 DMA 使能，RW；GHWCFG2.ARCH==2 时必须置 1，否则 HCDMA/DIEPDMA 无效。
        DMA_EN OFFSET(5) NUMBITS(1) [],
    ],

    /// Global USB Configuration
    ///
    /// USB 2.0 核心与 PHY 接口配置，偏移 `0x00C`。
    pub GUSBCFG [
        /// HS/FS Timeout Calibration
        ///
        /// HS/FS 超时校准值，RW；与 PHY TOUTCAL 引脚对应。
        TOUTCAL OFFSET(0) NUMBITS(3) [],
        /// UTMI+ Data Width
        ///
        /// UTMI+ 数据宽度，RW；0 = 8-bit，1 = 16-bit，CV182x HS 固定 16-bit。
        PHYIF16 OFFSET(3) NUMBITS(1) [],
        /// ULPI or UTMI Select
        ///
        /// PHY 接口选择，RW；0 = UTMI/UTMI+，1 = ULPI。
        ULPI_UTMI_SEL OFFSET(4) NUMBITS(1) [],
        /// FS Serial Interface Select
        ///
        /// FS 串行接口选择，RW；UTMI 路径通常为 0。
        FSINTF OFFSET(5) NUMBITS(1) [],
        /// PHY Type Select
        ///
        /// PHY 类型选择，RW；片内 UTMI 为 0。
        PHYSEL OFFSET(6) NUMBITS(1) [],
        /// SRP Capability
        ///
        /// SRP 能力，RW；纯主机模式清 0。
        SRPCAP OFFSET(8) NUMBITS(1) [],
        /// HNP Capability
        ///
        /// HNP 能力，RW；纯主机模式清 0。
        HNPCAP OFFSET(9) NUMBITS(1) [],
        /// USB Turnaround Time
        ///
        /// USB 周转时间，RW；UTMI 时钟到 MAC 时钟的 turnaround 周期，HS 常用 9。
        USBTRDTIM OFFSET(10) NUMBITS(4) [],
        /// UTMI Disconnect Detect Pulse Width
        ///
        /// UTMI 断开检测脉冲宽度选择，RW。
        TERMSELDLPULSE OFFSET(22) NUMBITS(1) [],
        /// ULPI Auto Resume
        ///
        /// ULPI 自动恢复，RW；UTMI 模式不使用。
        ULPI_AUTORES OFFSET(23) NUMBITS(1) [],
        /// IND Signal Polarity
        ///
        /// IND 信号极性，RW。
        IND_COMPLEMENT OFFSET(25) NUMBITS(1) [],
        /// IND Pass-Through Mode
        ///
        /// IND 直通模式，RW。
        IND_PASSTHROUGH OFFSET(26) NUMBITS(1) [],
        /// ULPI Interrupt Protection Disable
        ///
        /// ULPI 中断保护禁用，RW。
        ULPI_INT_PROT_DIS OFFSET(27) NUMBITS(1) [],
        /// Force Host Mode
        ///
        /// 强制主机模式，RW；置 1 后等待 GINTSTS.CURMODE_HOST。
        FORCEHOSTMODE OFFSET(29) NUMBITS(1) [],
        /// Force Device Mode
        ///
        /// 强制设备模式，RW；与主机路径互斥。
        FORCEDEVMODE OFFSET(30) NUMBITS(1) [],
    ],

    /// Global Reset Control
    ///
    /// 核心软复位与 TX/RX FIFO flush，偏移 `0x010`。
    pub GRSTCTL [
        /// Core Soft Reset
        ///
        /// 核心软复位请求，RW；写 1 启动，Core ≥ 4.20a 配合 CSFTRSTDONE 使用。
        CSFTRST OFFSET(0) NUMBITS(1) [],
        /// RX FIFO Flush
        ///
        /// 接收 FIFO flush，RW；写 1 启动，自清。
        RXFFLSH OFFSET(4) NUMBITS(1) [],
        /// TX FIFO Flush
        ///
        /// 发送 FIFO flush，RW；写 1 启动，自清。
        TXFFLSH OFFSET(5) NUMBITS(1) [],
        /// TX FIFO Number
        ///
        /// 要 flush 的 TX FIFO 编号，RW；0x10 = 全部非周期 TX FIFO。
        TXFNUM OFFSET(6) NUMBITS(5) [],
        /// Core Soft Reset Done
        ///
        /// 软复位完成标志，RO；Core ≥ 4.20a 新增，读后需写 1 清除。
        CSFTRST_DONE OFFSET(29) NUMBITS(1) [],
        /// AHB Master Idle
        ///
        /// AHB 主接口空闲，RO；软复位/FIFO 操作前须等待此位为 1。
        AHBIDLE OFFSET(31) NUMBITS(1) [],
    ],

    /// Global Interrupt Status
    ///
    /// 全局中断状态（除 `CURMODE_HOST` 外多数为 **W1C**），偏移 `0x014`。
    pub GINTSTS [
        /// Current Mode
        ///
        /// 当前工作模式，RO；0 = Device，1 = Host。
        CURMODE_HOST OFFSET(0) NUMBITS(1) [],
        /// Mode Mismatch
        ///
        /// 模式不匹配，W1C；在 Host 模式访问 Device 寄存器时置位。
        MODEMIS OFFSET(1) NUMBITS(1) [],
        /// OTG Interrupt
        ///
        /// OTG 协议事件，W1C；GOTGINT 有挂起位时置位。
        OTGINT OFFSET(2) NUMBITS(1) [],
        /// Start of Frame
        ///
        /// 帧起始，W1C；收到或发出 SOF 包时置位。
        SOF OFFSET(3) NUMBITS(1) [],
        /// RX FIFO Non-Empty
        ///
        /// RX FIFO 非空，W1C；须读 GRXSTSP 弹出条目。
        RXFLVL OFFSET(4) NUMBITS(1) [],
        /// Non-Periodic TX FIFO Empty
        ///
        /// 非周期 TX FIFO 空，W1C。
        NPTXFEMP OFFSET(5) NUMBITS(1) [],
        /// Global IN NAK Effective
        ///
        /// 全局 IN NAK 生效，W1C。
        GINNAKEFF OFFSET(6) NUMBITS(1) [],
        /// Global OUT NAK Effective
        ///
        /// 全局 OUT NAK 生效，W1C。
        GOUTNAKEFF OFFSET(7) NUMBITS(1) [],
        /// Early Suspend
        ///
        /// 早期挂起检测，W1C。
        ERLYSUSP OFFSET(10) NUMBITS(1) [],
        /// USB Suspend
        ///
        /// USB 挂起，W1C。
        USBSUSP OFFSET(11) NUMBITS(1) [],
        /// USB Reset
        ///
        /// USB 总线复位，W1C；设备枚举起点。
        USBRST OFFSET(12) NUMBITS(1) [],
        /// Enumeration Done
        ///
        /// 速度枚举完成，W1C；可读 DSTS.ENUMSPD 获取实际速度。
        ENUMDONE OFFSET(13) NUMBITS(1) [],
        /// Isochronous OUT Packet Dropped
        ///
        /// 同步 OUT 丢包，W1C。
        ISOOUTDROP OFFSET(14) NUMBITS(1) [],
        /// End of Periodic Frame
        ///
        /// 周期帧结束，W1C；设备中断传输使用。
        EOPF OFFSET(15) NUMBITS(1) [],
        /// Endpoint Mismatch
        ///
        /// 端点不匹配，W1C。
        EPMIS OFFSET(17) NUMBITS(1) [],
        /// IN Endpoint Interrupt
        ///
        /// IN 端点中断，W1C；任一 IN 端点 DIEPINT 挂起时置位。
        IEPINT OFFSET(18) NUMBITS(1) [],
        /// OUT Endpoint Interrupt
        ///
        /// OUT 端点中断，W1C；任一 OUT 端点 DOEPINT 挂起时置位。
        OEPINT OFFSET(19) NUMBITS(1) [],
        /// Incomplete Isochronous OUT
        ///
        /// 不完整 Isoch OUT 传输，W1C。
        INCOMPLPOUT OFFSET(20) NUMBITS(1) [],
        /// Incomplete Isochronous IN
        ///
        /// 不完整 Isoch IN 传输，W1C。
        INCOMPLPIN OFFSET(21) NUMBITS(1) [],
        /// Data Fetch Suspended
        ///
        /// 数据 FIFO 非空时挂起，W1C。
        FETSUSP OFFSET(22) NUMBITS(1) [],
        /// Reset Detected
        ///
        /// 复位检测，W1C；OTG 会话期间检测到复位信号。
        RSTDET OFFSET(23) NUMBITS(1) [],
        /// Host Port Interrupt
        ///
        /// 主机端口中断，W1C；需查 HPRT0 定位具体事件。
        PRTINT OFFSET(24) NUMBITS(1) [],
        /// Host Channel Interrupt
        ///
        /// 主机通道中断，W1C；需读 HAINT 定位具体通道。
        HCHINT OFFSET(25) NUMBITS(1) [],
        /// Connector ID Status Change
        ///
        /// 连接器 ID 状态变化，W1C。
        CONIDSTSCHNG OFFSET(28) NUMBITS(1) [],
        /// Disconnect Detected
        ///
        /// 会话断开，W1C。
        DISCONNINT OFFSET(29) NUMBITS(1) [],
        /// Session Request
        ///
        /// 会话请求，W1C。
        SESSREQINT OFFSET(30) NUMBITS(1) [],
        /// Remote Wakeup Interrupt
        ///
        /// 远程唤醒，W1C。
        WKUPINT OFFSET(31) NUMBITS(1) [],
    ],

    /// Global Interrupt Mask
    ///
    /// 全局中断掩码（位号与 [`GINTSTS`] 对应，写 1 使能对应中断），偏移 `0x018`。
    pub GINTMSK [
        /// Mode Mismatch Mask
        ///
        /// 模式不匹配中断掩码，RW。
        MODEMIS OFFSET(1) NUMBITS(1) [],
        /// OTG Interrupt Mask
        ///
        /// OTG 协议事件中断掩码，RW。
        OTGINT OFFSET(2) NUMBITS(1) [],
        /// Start of Frame Mask
        ///
        /// 帧起始中断掩码，RW。
        SOF OFFSET(3) NUMBITS(1) [],
        /// RX FIFO Non-Empty Mask
        ///
        /// RX FIFO 非空中断掩码，RW。
        RXFLVL OFFSET(4) NUMBITS(1) [],
        /// Non-Periodic TX FIFO Empty Mask
        ///
        /// 非周期 TX FIFO 空中断掩码，RW。
        NPTXFEMP OFFSET(5) NUMBITS(1) [],
        /// Global IN NAK Effective Mask
        ///
        /// 全局 IN NAK 生效中断掩码，RW。
        GINNAKEFF OFFSET(6) NUMBITS(1) [],
        /// Global OUT NAK Effective Mask
        ///
        /// 全局 OUT NAK 生效中断掩码，RW。
        GOUTNAKEFF OFFSET(7) NUMBITS(1) [],
        /// Early Suspend Mask
        ///
        /// 早期挂起检测中断掩码，RW。
        ERLYSUSP OFFSET(10) NUMBITS(1) [],
        /// USB Suspend Mask
        ///
        /// USB 挂起中断掩码，RW。
        USBSUSP OFFSET(11) NUMBITS(1) [],
        /// USB Reset Mask
        ///
        /// USB 总线复位中断掩码，RW。
        USBRST OFFSET(12) NUMBITS(1) [],
        /// Enumeration Done Mask
        ///
        /// 速度枚举完成中断掩码，RW。
        ENUMDONE OFFSET(13) NUMBITS(1) [],
        /// Isochronous OUT Packet Dropped Mask
        ///
        /// 同步 OUT 丢包中断掩码，RW。
        ISOOUTDROP OFFSET(14) NUMBITS(1) [],
        /// End of Periodic Frame Mask
        ///
        /// 周期帧结束中断掩码，RW。
        EOPF OFFSET(15) NUMBITS(1) [],
        /// Endpoint Mismatch Mask
        ///
        /// 端点不匹配中断掩码，RW。
        EPMIS OFFSET(17) NUMBITS(1) [],
        /// IN Endpoint Interrupt Mask
        ///
        /// IN 端点中断掩码，RW。
        IEPINT OFFSET(18) NUMBITS(1) [],
        /// OUT Endpoint Interrupt Mask
        ///
        /// OUT 端点中断掩码，RW。
        OEPINT OFFSET(19) NUMBITS(1) [],
        /// Incomplete Isochronous OUT Mask
        ///
        /// 不完整 Isoch OUT 中断掩码，RW。
        INCOMPLPOUT OFFSET(20) NUMBITS(1) [],
        /// Incomplete Isochronous IN Mask
        ///
        /// 不完整 Isoch IN 中断掩码，RW。
        INCOMPLPIN OFFSET(21) NUMBITS(1) [],
        /// Data Fetch Suspended Mask
        ///
        /// 数据 FIFO 挂起中断掩码，RW。
        FETSUSP OFFSET(22) NUMBITS(1) [],
        /// Reset Detected Mask
        ///
        /// 复位检测中断掩码，RW。
        RSTDET OFFSET(23) NUMBITS(1) [],
        /// Host Port Interrupt Mask
        ///
        /// 主机端口中断掩码，RW。
        PRTINT OFFSET(24) NUMBITS(1) [],
        /// Host Channel Interrupt Mask
        ///
        /// 主机通道中断掩码，RW；需配合 HAINTMSK 定位具体通道。
        HCHINT OFFSET(25) NUMBITS(1) [],
        /// Connector ID Status Change Mask
        ///
        /// 连接器 ID 状态变化中断掩码，RW。
        CONIDSTSCHNG OFFSET(28) NUMBITS(1) [],
        /// Disconnect Detected Mask
        ///
        /// 会话断开中断掩码，RW。
        DISCONNINT OFFSET(29) NUMBITS(1) [],
        /// Session Request Mask
        ///
        /// 会话请求中断掩码，RW。
        SESSREQINT OFFSET(30) NUMBITS(1) [],
        /// Remote Wakeup Interrupt Mask
        ///
        /// 远程唤醒中断掩码，RW。
        WKUPINT OFFSET(31) NUMBITS(1) [],
    ],

    /// Global Receive Status Pop
    ///
    /// RX FIFO 弹出寄存器（只读，读操作从 FIFO 弹出一项），偏移 `0x020`。
    ///
    /// 设备模式用于解析 SETUP/OUT 包；`PKTSTS` 编码见枚举变体。
    pub GRXSTSP [
        /// Endpoint Number
        ///
        /// 端点号，RO。
        EPNUM OFFSET(0) NUMBITS(4) [],
        /// Byte Count
        ///
        /// 本条目字节数，RO。
        BCNT OFFSET(4) NUMBITS(11) [],
        /// Data PID
        ///
        /// 数据 PID：DATA0/DATA1/DATA2，RO。
        DPID OFFSET(15) NUMBITS(2) [],
        /// Packet Status
        ///
        /// FIFO 弹出条目类型，RO。
        PKTSTS OFFSET(17) NUMBITS(4) [
            /// Global OUT NAK   所有 OUT 端点被 NAK
            GlobalOutNak = 1,
            /// OUT Data Packet   OUT 数据包已写入 RX FIFO
            OutData = 2,
            /// OUT Transfer Complete   OUT 传输完成
            OutXferCompl = 3,
            /// SETUP Transaction Complete   SETUP 事务完成
            SetupCompl = 4,
            /// SETUP Data Packet   SETUP 数据包 8 字节在 FIFO 中
            SetupData = 6,
        ],
        /// Frame Number
        ///
        /// 帧号低 4 位，RO。
        FRMNUM OFFSET(21) NUMBITS(4) [],
    ],

    /// OTG Control and Status
    ///
    /// OTG 会话与 VBUS 控制（`dr_mode=otg` 时主机会话 override），偏移 `0x000`。
    pub GOTGCTL [
        /// VBUS Valid Override Enable
        ///
        /// VBUS 有效信号覆盖使能，RW。
        VBVALOEN OFFSET(2) NUMBITS(1) [],
        /// VBUS Valid Override Value
        ///
        /// VBUS 有效覆盖值，RW。
        VBVALOVAL OFFSET(3) NUMBITS(1) [],
        /// A-Device Override Enable
        ///
        /// A 设备覆盖使能，RW。
        AVALOEN OFFSET(4) NUMBITS(1) [],
        /// A-Device Override Value
        ///
        /// A 设备覆盖值，RW。
        AVALOVAL OFFSET(5) NUMBITS(1) [],
        /// Debounce Filter Bypass
        ///
        /// ID/VBUS 去抖滤波旁路，RW。
        DBNCE_FLTR_BYPASS OFFSET(15) NUMBITS(1) [],
    ],

    /// Global Hardware Configuration 2
    ///
    /// 硬件能力配置 2（只读，上电后固定），偏移 `0x048`。
    pub GHWCFG2 [
        /// Architecture
        ///
        /// 0=Slave，1=External DMA，2=Internal DMA，RO。
        ARCH OFFSET(3) NUMBITS(2) [],
        /// Number of Host Channels
        ///
        /// 主机通道数减一编码，实际 = 值+1，RO。
        NUM_HOST_CHAN OFFSET(14) NUMBITS(4) [],
    ],

    /// Global Hardware Configuration 3
    ///
    /// 硬件能力配置 3（DFIFO 深度等），偏移 `0x04C`。
    pub GHWCFG3 [
        /// Data FIFO Depth
        ///
        /// 数据 FIFO 总深度，32-bit 字，RO。
        DFIFO_DEPTH OFFSET(16) NUMBITS(16) [],
    ],

    /// Global Hardware Configuration 4
    ///
    /// 硬件能力配置 4，偏移 `0x050`。
    pub GHWCFG4 [
        /// Dedicated FIFO Enable
        ///
        /// 每 IN EP 独立 TX FIFO，RO。
        DED_FIFO_EN OFFSET(25) NUMBITS(1) [],
        /// UTMI PHY Data Width
        ///
        /// PHY 数据宽度能力，RO。
        UTMI_PHY_DATA_WIDTH OFFSET(14) NUMBITS(2) [
            /// 8-bit Only   仅 8-bit
            Eight = 0,
            /// 16-bit Only   仅 16-bit
            Sixteen = 1,
            /// 8/16-bit Programmable   由 GUSBCFG.PHYIF16 选择
            Programmable = 2,
        ],
    ],

    /// Global Data FIFO Configuration
    ///
    /// 动态 FIFO 配置（Core ≥ 2.91a），偏移 `0x05C`。
    pub GDFIFOCFG [
        /// Global DFIFO Configuration
        ///
        /// 可分配 FIFO 空间总量，32-bit 字，RW。
        GDFIFOCFG OFFSET(0) NUMBITS(16) [],
        /// Endpoint Info Base Address
        ///
        /// EP 信息区在 DFIFO 中的起始地址，RW。
        EPINFOBASE OFFSET(16) NUMBITS(16) [],
    ],

    /// Host Configuration
    ///
    /// 主机模式配置，偏移 `0x400`。
    pub HCFG [
        /// FS/LS PHY Clock Select
        ///
        /// FS/LS PHY 时钟选择，RW。
        FSLSPCLKSEL OFFSET(0) NUMBITS(2) [
            /// 48 MHz PLL Clock   48 MHz，FS 枚举常用
            Pll48Mhz = 1,
        ],
        /// FS/LS Only Support
        ///
        /// 1=仅 FS/LS 无 HS chirp；HS 主机须清 0，RW。
        FSLSSUPP OFFSET(2) NUMBITS(1) [],
    ],

    /// Host Port Control and Status Register 0
    ///
    /// 根端口状态与控制（仅通道 0 / 根 Hub 口），偏移 `0x440`。
    ///
    /// **W1C 位**（RMW 写入时须 mask 掉读回的 1，见 `controller::hprt0_modify_safe`）：
    /// `CONNDET`、`ENA`、`ENACHG`、`OVRCURCHG`。
    pub HPRT0 [
        /// Port Connect Status
        ///
        /// 1=有设备连接，RO。
        CONNSTS OFFSET(0) NUMBITS(1) [],
        /// Port Connect Detected
        ///
        /// 连接检测变化，W1C；写 1 清除。
        CONNDET OFFSET(1) NUMBITS(1) [],
        /// Port Enable
        ///
        /// 1=端口已 enable；**写 1 = disable 端口**，R/W1C。
        ENA OFFSET(2) NUMBITS(1) [],
        /// Port Enable/Disable Change
        ///
        /// enable 状态变化，W1C。
        ENACHG OFFSET(3) NUMBITS(1) [],
        /// Port Overcurrent Active
        ///
        /// 过流 active，RO。
        OVRCURACT OFFSET(4) NUMBITS(1) [],
        /// Port Overcurrent Change
        ///
        /// 过流变化，W1C。
        OVRCURCHG OFFSET(5) NUMBITS(1) [],
        /// Port Reset
        ///
        /// USB 总线复位 active，须保持 ≥50 ms TDRSTR，RW。
        RST OFFSET(8) NUMBITS(1) [],
        /// Port Line Status
        ///
        /// 线路状态 SE0/J/K/SE1，RO。
        LNSTS OFFSET(10) NUMBITS(2) [],
        /// Port Power
        ///
        /// 端口电源 VBUS/PHY 供电，RW。
        PWR OFFSET(12) NUMBITS(1) [],
        /// Port Speed
        ///
        /// 设备速度，chirp 完成后更新，RO。
        SPD OFFSET(17) NUMBITS(2) [
            /// High-Speed   高速
            HighSpeed = 0,
            /// Full-Speed   全速
            FullSpeed = 1,
            /// Low-Speed   低速
            LowSpeed = 2,
        ],
    ],

    /// Host Frame Number
    ///
    /// 主机微帧编号（只读），偏移 `0x408`。
    pub HFNUM [
        /// Frame Number
        ///
        /// 当前帧号；HS 每帧 8 微帧，125 µs 递增，RO。
        FRNUM OFFSET(0) NUMBITS(16) [],
        /// Frame Time Remaining
        ///
        /// 当前微帧剩余 bit 时间，RO。
        FRREM OFFSET(16) NUMBITS(16) [],
    ],

    /// Host All Channels Interrupt
    ///
    /// 主机所有通道中断状态（只读），偏移 `0x414`。
    ///
    /// `CHINT[n]` = 1 表示通道 n 的 `HCINT` 有未处理位。
    pub HAINT [
        /// Host All Channels Interrupt
        ///
        /// 通道 0..15 中断挂起位图，RO。
        CHINT OFFSET(0) NUMBITS(16) [],
    ],

    /// Host All Channels Interrupt Mask
    ///
    /// 主机通道中断掩码，偏移 `0x418`。
    pub HAINTMSK [
        /// Host All Channels Interrupt Mask
        ///
        /// 写 1 允许通道中断汇总到 GINTSTS.HCHINT，RW。
        CHINT OFFSET(0) NUMBITS(16) [],
    ],

    /// Host Channel Characteristics
    ///
    /// 主机通道特性（端点/设备/类型/MPS），偏移 通道 `+0x00`。
    ///
    /// 传输启动：置 `CHENA`；停止：同时置 `CHENA|CHDIS`。Isoch 须配合 `ODDFRM` 对齐微帧。
    pub HCCHAR [
        /// Maximum Packet Size
        ///
        /// 端点最大包长，字节，≤1024，RW。
        MPS OFFSET(0) NUMBITS(11) [],
        /// Endpoint Number
        ///
        /// 端点号 0..15，RW。
        EPNUM OFFSET(11) NUMBITS(4) [],
        /// Endpoint Direction
        ///
        /// 0=OUT，1=IN，RW。
        EPDIR OFFSET(15) NUMBITS(1) [],
        /// Low-Speed Device
        ///
        /// 1=目标为 LS 设备，经 Hub FS 前置，RW。
        LSPDDEV OFFSET(17) NUMBITS(1) [],
        /// Endpoint Type
        ///
        /// 端点类型，RW。
        EPTYPE OFFSET(18) NUMBITS(2) [
            /// Control Transfer   控制传输
            Control = 0,
            /// Isochronous Transfer   同步传输
            Isochronous = 1,
            /// Bulk Transfer   批量传输
            Bulk = 2,
            /// Interrupt Transfer   中断传输
            Interrupt = 3,
        ],
        /// Multi Count
        ///
        /// HS 高带宽 Isoch 每微帧事务数减一，实际=MC+1，RW。
        MC OFFSET(20) NUMBITS(2) [],
        /// Device Address
        ///
        /// 目标 USB 设备地址 7 bit，RW。
        DEVADDR OFFSET(22) NUMBITS(7) [],
        /// Odd Frame
        ///
        /// 1=在奇数 microframe 启动，Isoch 调度，RW。
        ODDFRM OFFSET(29) NUMBITS(1) [],
        /// Channel Disable
        ///
        /// 通道禁用请求，与 CHENA 同写生效，RW。
        CHDIS OFFSET(30) NUMBITS(1) [],
        /// Channel Enable
        ///
        /// 通道使能/启动传输，RW。
        CHENA OFFSET(31) NUMBITS(1) [],
    ],

    /// Host Channel Interrupt
    ///
    /// 主机通道中断（**全部 W1C**，写 1 清除），偏移 通道 `+0x08`。
    pub HCINT [
        /// Transfer Complete
        ///
        /// 传输成功完成，W1C。
        XFERCOMPL OFFSET(0) NUMBITS(1) [],
        /// Channel Halted
        ///
        /// 通道已 halt；轮询/ISR 主要等待此位，W1C。
        CHHLTD OFFSET(1) NUMBITS(1) [],
        /// AHB Error
        ///
        /// AHB 总线错误，W1C。
        AHBERR OFFSET(2) NUMBITS(1) [],
        /// STALL Response
        ///
        /// 设备返回 STALL handshake，W1C。
        STALL OFFSET(3) NUMBITS(1) [],
        /// NAK Response
        ///
        /// 设备返回 NAK；EP0 可重试，Bulk IN 视频流常见，W1C。
        NAK OFFSET(4) NUMBITS(1) [],
        /// ACK Response
        ///
        /// 收到 ACK，W1C。
        ACK OFFSET(5) NUMBITS(1) [],
        /// NYET Response
        ///
        /// 收到 NYET，HS 高带宽拆分，W1C。
        NYET OFFSET(6) NUMBITS(1) [],
        /// Transaction Error
        ///
        /// CRC/超时/PID 等事务错误，W1C。
        XACTERR OFFSET(7) NUMBITS(1) [],
        /// Babble Error
        ///
        /// Babble 错误，W1C。
        BBLERR OFFSET(8) NUMBITS(1) [],
        /// Frame Overrun
        ///
        /// 帧超限，Isoch 错过微帧窗口，W1C。
        FRMOVRN OFFSET(9) NUMBITS(1) [],
        /// Data Toggle Error
        ///
        /// DATA toggle 不匹配，W1C。
        DATATGLERR OFFSET(10) NUMBITS(1) [],
    ],

    /// Host Channel Transfer Size
    ///
    /// 主机通道传输长度与 PID，偏移 通道 `+0x10`。
    pub HCTSIZ [
        /// Transfer Size
        ///
        /// 传输总字节数；完成后 初始值−读回值=实际字节数，RW。
        XFERSIZE OFFSET(0) NUMBITS(19) [],
        /// Packet Count
        ///
        /// 包个数；完成后硬件递减，RW。
        PKTCNT OFFSET(19) NUMBITS(10) [],
        /// Data PID
        ///
        /// 首包 PID 编码，与 channel::PID_* 常量一致，RW。
        PID OFFSET(29) NUMBITS(2) [
            /// DATA0 PID   数据 PID DATA0
            Data0 = 0,
            /// DATA2 PID   HS 高带宽 Isoch 三事务
            Data2 = 1,
            /// DATA1 PID   数据 PID DATA1
            Data1 = 2,
            /// SETUP PID   控制传输 SETUP 阶段 PID
            Setup = 3,
        ],
    ],

    /// Device Configuration
    ///
    /// 设备配置（速度、地址、帧间隔），偏移 `0x800`。
    pub DCFG [
        /// Device Speed
        ///
        /// 设备速度配置，须与 PHY/枚举结果一致。
        DEVSPD OFFSET(0) NUMBITS(2) [
            /// High-Speed   高速
            HighSpeed = 0,
            /// Full-Speed in HS PHY   HS PHY 下全速
            FullSpeedHs = 1,
            /// Low-Speed in LS PHY   LS PHY 下低速
            LowSpeedLs = 2,
            /// Full-Speed in FS PHY   FS PHY 下全速
            FullSpeedFs = 3,
        ],
        /// Non-Zero-Length Status OUT Handshake
        ///
        /// 非零长度状态 OUT 握手行为。
        NZSTSOUTHSHK OFFSET(2) NUMBITS(1) [],
        /// Enable 32KHz Suspend
        ///
        /// 挂起时启用 32 kHz 时钟。
        ENA32KHZSUSP OFFSET(3) NUMBITS(1) [],
        /// Device Address
        ///
        /// USB 设备地址，SET_ADDRESS 后写入。
        DEVADDR OFFSET(4) NUMBITS(7) [],
        /// Periodic Frame Interval
        ///
        /// 周期帧中断间隔，用于设备模式节能。
        PERFRINT OFFSET(11) NUMBITS(2) [
            /// 80% of Frame Interval   帧间隔 80%
            Frm80 = 0,
            /// 85% of Frame Interval   帧间隔 85%
            Frm85 = 1,
            /// 90% of Frame Interval   帧间隔 90%
            Frm90 = 2,
            /// 95% of Frame Interval   帧间隔 95%
            Frm95 = 3,
        ],
        /// Enable Device OUT NAK
        ///
        /// 使能设备级 OUT NAK 功能。
        ENDEVOUTNAK OFFSET(13) NUMBITS(1) [],
        /// Transceiver Delay
        ///
        /// 收发器延迟补偿。
        XCVRDLY OFFSET(14) NUMBITS(1) [],
        /// Erratic Error Timing
        ///
        /// 异常错误时序控制。
        ERRATIM OFFSET(15) NUMBITS(1) [],
        /// Descriptor DMA
        ///
        /// 使能描述符 DMA 模式。
        DESCDMA OFFSET(23) NUMBITS(1) [],
        /// Periodic Scheduling Interval
        ///
        /// 周期端点调度间隔。
        PERSCHINTVL OFFSET(24) NUMBITS(2) [],
        /// Resume Validation
        ///
        /// 恢复信号有效检测时间。
        RESVALID OFFSET(26) NUMBITS(6) [],
    ],

    /// Device Control
    ///
    /// 设备控制（软断开、全局 NAK、测试模式），偏移 `0x804`。
    pub DCTL [
        /// Remote Wakeup Signaling
        ///
        /// 远程唤醒信号。
        RWUSIG OFFSET(0) NUMBITS(1) [],
        /// Soft Disconnect
        ///
        /// 软断开，置 1 后 D+/D- 上拉断开，主机看到设备消失。
        SFTDISCON OFFSET(1) NUMBITS(1) [],
        /// Global Non-Periodic IN NAK Status
        ///
        /// 全局非周期 IN NAK 状态，只读。
        GNPINNAKSTS OFFSET(2) NUMBITS(1) [],
        /// Global OUT NAK Status
        ///
        /// 全局 OUT NAK 状态，只读。
        GOUTNAKSTS OFFSET(3) NUMBITS(1) [],
        /// Test Control
        ///
        /// USB 测试模式选择。
        TSTCTL OFFSET(4) NUMBITS(3) [],
        /// Set Global Non-Periodic IN NAK
        ///
        /// 设置全局非周期 IN NAK。
        SGNPINNAK OFFSET(7) NUMBITS(1) [],
        /// Clear Global Non-Periodic IN NAK
        ///
        /// 清除全局非周期 IN NAK。
        CGNPINNAK OFFSET(8) NUMBITS(1) [],
        /// Set Global OUT NAK
        ///
        /// 设置全局 OUT NAK。
        SGOUTNAK OFFSET(9) NUMBITS(1) [],
        /// Clear Global OUT NAK
        ///
        /// 清除全局 OUT NAK。
        CGOUTNAK OFFSET(10) NUMBITS(1) [],
        /// Power-On Programming Done
        ///
        /// 上电寄存器编程完成标志。
        PWRONPRGDONE OFFSET(11) NUMBITS(1) [],
        /// Global Multi Count
        ///
        /// 全局多事务计数，ISO 端点使用。
        GMC OFFSET(13) NUMBITS(2) [],
        /// Ignore Frame Number
        ///
        /// 忽略帧号，ISO 传输 DMA 模式使用。
        IGNRFRMNUM OFFSET(15) NUMBITS(1) [],
        /// NAK on Babble Error
        ///
        /// Babble 错误时自动设置 NAK。
        NAKONBBLE OFFSET(16) NUMBITS(1) [],
    ],

    /// Device Status
    ///
    /// 设备状态（只读），偏移 `0x808`。
    pub DSTS [
        /// Suspend Status
        ///
        /// 总线挂起状态，只读。
        SUSPSTS OFFSET(0) NUMBITS(1) [],
        /// Enumerated Speed
        ///
        /// 枚举完成后实际速度，只读。
        ENUMSPD OFFSET(1) NUMBITS(2) [
            /// High-Speed   高速
            HighSpeed = 0,
            /// Full-Speed in HS PHY   HS PHY 下全速
            FullSpeedHs = 1,
            /// Low-Speed in LS PHY   LS PHY 下低速
            LowSpeedLs = 2,
            /// Full-Speed in FS PHY   FS PHY 下全速
            FullSpeedFs = 3,
        ],
        /// Erratic Error
        ///
        /// 异常错误检测，只读。
        ERRTICERR OFFSET(3) NUMBITS(1) [],
        /// Frame Number of Received SOF
        ///
        /// 收到的 SOF 帧号，只读。
        SOFFN OFFSET(8) NUMBITS(14) [],
    ],

    /// Device IN/OUT Endpoint Common Interrupt Mask
    ///
    /// IN/OUT 端点中断掩码（位定义相同），偏移 `0x810` / `0x814`。
    pub DEPMSK [
        /// Transfer Completed Mask
        ///
        /// 传输完成中断掩码。
        XFERCOMPL OFFSET(0) NUMBITS(1) [],
        /// Endpoint Disabled Mask
        ///
        /// 端点禁用完成中断掩码。
        EPDISBLD OFFSET(1) NUMBITS(1) [],
        /// AHB Error Mask
        ///
        /// AHB 总线错误中断掩码。
        AHBERR OFFSET(2) NUMBITS(1) [],
        /// Timeout / SETUP Received Mask
        ///
        /// IN 超时或 OUT SETUP 包接收中断掩码。
        TIMEOUT_OR_SETUP OFFSET(3) NUMBITS(1) [],
        /// IN Token Received with TX FIFO Empty Mask
        ///
        /// 收到 IN 令牌时 TX FIFO 空中断掩码。
        INTKNTXFEMP OFFSET(4) NUMBITS(1) [],
        /// IN Token Received with EP Mismatch Mask
        ///
        /// IN 令牌端点不匹配中断掩码。
        INTKNEPMIS OFFSET(5) NUMBITS(1) [],
        /// IN Endpoint NAK Effective Mask
        ///
        /// IN 端点 NAK 生效中断掩码。
        INEPNAKEFF OFFSET(6) NUMBITS(1) [],
        /// TX FIFO Underrun Mask
        ///
        /// TX FIFO 下溢中断掩码。
        TXFIFOUNDRN OFFSET(8) NUMBITS(1) [],
        /// BNA Interrupt Mask
        ///
        /// 缓冲区不可用中断掩码，描述符 DMA 模式。
        BNAINTR OFFSET(9) NUMBITS(1) [],
        /// Babble Error Mask
        ///
        /// Babble 错误中断掩码。
        BBLEERR OFFSET(12) NUMBITS(1) [],
        /// NAK Interrupt Mask
        ///
        /// NAK 中断掩码。
        NAK OFFSET(13) NUMBITS(1) [],
        /// NYET Interrupt Mask
        ///
        /// NYET 中断掩码。
        NYET OFFSET(14) NUMBITS(1) [],
    ],

    /// Device All Endpoints Interrupt Mask
    ///
    /// 各端点中断掩码（IN EP0..15 / OUT EP0..15），偏移 `0x81C`。
    pub DAINTMSK [
        /// IN Endpoint Interrupt Mask
        ///
        /// IN 端点中断掩码，对应 EP0..15。
        IEPMSK OFFSET(0) NUMBITS(16) [],
        /// OUT Endpoint Interrupt Mask
        ///
        /// OUT 端点中断掩码，对应 EP0..15。
        OEPMSK OFFSET(16) NUMBITS(16) [],
    ],

    /// Device IN Endpoint Control
    ///
    /// 设备 IN 端点控制，偏移 IN EP `+0x00`。
    pub DIEPCTL [
        /// Maximum Packet Size
        ///
        /// 最大包长度。
        MPS OFFSET(0) NUMBITS(11) [],
        /// USB Active Endpoint
        ///
        /// 端点已在当前配置中激活。
        USBACTEP OFFSET(15) NUMBITS(1) [],
        /// Endpoint Data PID
        ///
        /// DATA PID 状态；ISO 为帧号，非 ISO 为 DATA0/DATA1 翻转，只读。
        DPID OFFSET(16) NUMBITS(1) [],
        /// NAK Status
        ///
        /// 端点当前处于 NAK 状态，只读。
        NAKSTS OFFSET(17) NUMBITS(1) [],
        /// Endpoint Type
        ///
        /// 端点类型。
        EPTYPE OFFSET(18) NUMBITS(2) [
            /// Control Transfer   控制传输
            Control = 0,
            /// Isochronous Transfer   同步传输
            Isochronous = 1,
            /// Bulk Transfer   批量传输
            Bulk = 2,
            /// Interrupt Transfer   中断传输
            Interrupt = 3,
        ],
        /// STALL Handshake
        ///
        /// 置 1 使端点发送 STALL 握手。
        STALL OFFSET(21) NUMBITS(1) [],
        /// TX FIFO Number
        ///
        /// 绑定的 TX FIFO 编号，动态 FIFO 布局。
        TXFNUM OFFSET(22) NUMBITS(4) [],
        /// Clear NAK
        ///
        /// 写 1 清除 NAK。
        CNAK OFFSET(26) NUMBITS(1) [],
        /// Set NAK
        ///
        /// 写 1 设置 NAK。
        SNAK OFFSET(27) NUMBITS(1) [],
        /// Set DATA0 PID
        ///
        /// 强制将端点 PID 设置为 DATA0。
        SETD0PID OFFSET(28) NUMBITS(1) [],
        /// Set DATA1 PID
        ///
        /// 强制将端点 PID 设置为 DATA1。
        SETD1PID OFFSET(29) NUMBITS(1) [],
        /// Endpoint Disable
        ///
        /// 端点禁用。
        EPDIS OFFSET(30) NUMBITS(1) [],
        /// Endpoint Enable
        ///
        /// 端点使能，启动传输前须置位。
        EPENA OFFSET(31) NUMBITS(1) [],
    ],

    /// Device OUT Endpoint Control
    ///
    /// 设备 OUT 端点控制，偏移 OUT EP `+0x00`。
    pub DOEPCTL [
        /// Maximum Packet Size
        ///
        /// 最大包长度。
        MPS OFFSET(0) NUMBITS(11) [],
        /// USB Active Endpoint
        ///
        /// 端点已在当前配置中激活。
        USBACTEP OFFSET(15) NUMBITS(1) [],
        /// Endpoint Data PID
        ///
        /// DATA PID 状态，只读。
        DPID OFFSET(16) NUMBITS(1) [],
        /// NAK Status
        ///
        /// 端点当前处于 NAK 状态，只读。
        NAKSTS OFFSET(17) NUMBITS(1) [],
        /// Endpoint Type
        ///
        /// 端点类型。
        EPTYPE OFFSET(18) NUMBITS(2) [
            /// Control Transfer   控制传输
            Control = 0,
            /// Isochronous Transfer   同步传输
            Isochronous = 1,
            /// Bulk Transfer   批量传输
            Bulk = 2,
            /// Interrupt Transfer   中断传输
            Interrupt = 3,
        ],
        /// Snoop Mode
        ///
        /// 忽略 OUT 包长度与 `DOEPTSIZ` 差异，DMA 模式常用。
        SNP OFFSET(20) NUMBITS(1) [],
        /// STALL Handshake
        ///
        /// 置 1 使端点发送 STALL 握手。
        STALL OFFSET(21) NUMBITS(1) [],
        /// Clear NAK
        ///
        /// 写 1 清除 NAK。
        CNAK OFFSET(26) NUMBITS(1) [],
        /// Set NAK
        ///
        /// 写 1 设置 NAK。
        SNAK OFFSET(27) NUMBITS(1) [],
        /// Set DATA0 PID
        ///
        /// 强制将端点 PID 设置为 DATA0。
        SETD0PID OFFSET(28) NUMBITS(1) [],
        /// Set DATA1 PID
        ///
        /// 强制将端点 PID 设置为 DATA1。
        SETD1PID OFFSET(29) NUMBITS(1) [],
        /// Endpoint Disable
        ///
        /// 端点禁用。
        EPDIS OFFSET(30) NUMBITS(1) [],
        /// Endpoint Enable
        ///
        /// 端点使能，启动传输前须置位。
        EPENA OFFSET(31) NUMBITS(1) [],
    ],

    /// Device IN Endpoint Interrupt
    ///
    /// IN 端点中断（W1C），偏移 IN EP `+0x08`。
    pub DIEPINT [
        /// Transfer Completed
        ///
        /// 传输完成，W1C。
        XFERCOMPL OFFSET(0) NUMBITS(1) [],
        /// Endpoint Disabled
        ///
        /// 端点禁用完成，W1C。
        EPDISBLD OFFSET(1) NUMBITS(1) [],
        /// AHB Error
        ///
        /// AHB 总线错误，W1C。
        AHBERR OFFSET(2) NUMBITS(1) [],
        /// Timeout Condition
        ///
        /// IN 令牌超时，W1C。
        TIMEOUT OFFSET(3) NUMBITS(1) [],
        /// IN Token Received with TX FIFO Empty
        ///
        /// 收到 IN 令牌时 TX FIFO 空，W1C。
        INTKNTXFEMP OFFSET(4) NUMBITS(1) [],
        /// IN Token Received with EP Mismatch
        ///
        /// IN 令牌端点不匹配，W1C。
        INTKNEPMIS OFFSET(5) NUMBITS(1) [],
        /// IN Endpoint NAK Effective
        ///
        /// IN 端点 NAK 生效，W1C。
        INEPNAKEFF OFFSET(6) NUMBITS(1) [],
        /// TX FIFO Empty
        ///
        /// TX FIFO 为空，只读。
        TXFEMP OFFSET(7) NUMBITS(1) [],
        /// TX FIFO Underrun
        ///
        /// TX FIFO 下溢，W1C。
        TXFIFOUNDRN OFFSET(8) NUMBITS(1) [],
        /// BNA Interrupt
        ///
        /// 缓冲区不可用，描述符 DMA 模式，W1C。
        BNAINTR OFFSET(9) NUMBITS(1) [],
        /// Packet Drop Status
        ///
        /// 包丢弃状态，W1C。
        PKTDRPSTS OFFSET(11) NUMBITS(1) [],
        /// Babble Error
        ///
        /// Babble 错误，W1C。
        BBLEERR OFFSET(12) NUMBITS(1) [],
        /// NAK Interrupt
        ///
        /// NAK 中断，W1C。
        NAKINTRPT OFFSET(13) NUMBITS(1) [],
        /// NYET Interrupt
        ///
        /// NYET 响应已发送，W1C。
        NYET OFFSET(14) NUMBITS(1) [],
    ],

    /// Device OUT Endpoint Interrupt
    ///
    /// OUT 端点中断（W1C），偏移 OUT EP `+0x08`。
    pub DOEPINT [
        /// Transfer Completed
        ///
        /// 传输完成，W1C。
        XFERCOMPL OFFSET(0) NUMBITS(1) [],
        /// Endpoint Disabled
        ///
        /// 端点禁用完成，W1C。
        EPDISBLD OFFSET(1) NUMBITS(1) [],
        /// AHB Error
        ///
        /// AHB 总线错误，W1C。
        AHBERR OFFSET(2) NUMBITS(1) [],
        /// SETUP Phase Done
        ///
        /// 收到 SETUP 包，EP0 控制传输，W1C。
        SETUP OFFSET(3) NUMBITS(1) [],
        /// OUT Token Received When Endpoint Disabled
        ///
        /// 端点禁用时收到 OUT 令牌，W1C。
        OUTTKNEPDIS OFFSET(4) NUMBITS(1) [],
        /// Status Phase Received
        ///
        /// 状态阶段完成，EP0 ZLP 握手，W1C。
        STSPHSERCVD OFFSET(5) NUMBITS(1) [],
        /// Back-to-Back SETUP Packets Received
        ///
        /// 连续收到两个 SETUP 包，W1C。
        BACK2BACKSETUP OFFSET(6) NUMBITS(1) [],
        /// OUT Packet Error
        ///
        /// OUT 包错误，W1C。
        OUTPKTERR OFFSET(8) NUMBITS(1) [],
        /// BNA Interrupt
        ///
        /// 缓冲区不可用，描述符 DMA 模式，W1C。
        BNAINTR OFFSET(9) NUMBITS(1) [],
        /// Packet Drop Status
        ///
        /// 包丢弃状态，W1C。
        PKTDRPSTS OFFSET(11) NUMBITS(1) [],
        /// Babble Error
        ///
        /// Babble 错误，W1C。
        BBLEERR OFFSET(12) NUMBITS(1) [],
        /// NAK Interrupt
        ///
        /// NAK 中断，W1C。
        NAKINTRPT OFFSET(13) NUMBITS(1) [],
        /// NYET Interrupt
        ///
        /// NYET 响应，W1C。
        NYET OFFSET(14) NUMBITS(1) [],
        /// Setup Packet Received
        ///
        /// 收到 SETUP 包，W1C。
        STUPPKTRCVD OFFSET(15) NUMBITS(1) [],
    ],

    /// Device IN/OUT Endpoint 0 Transfer Size
    ///
    /// EP0 传输大小，偏移 EP0 `+0x10`。
    pub DEPTSIZ0 [
        /// Transfer Size
        ///
        /// 传输字节数，SETUP 阶段为 8。
        XFERSIZE OFFSET(0) NUMBITS(7) [],
        /// Packet Count
        ///
        /// 包计数，EP0 通常为 1。
        PKTCNT OFFSET(19) NUMBITS(2) [],
        /// Setup Packet Count
        ///
        /// SETUP 包计数，OUT EP0 接收时使用。
        SUPCNT OFFSET(29) NUMBITS(2) [],
    ],

    /// Device Endpoint Transfer Size
    ///
    /// Bulk/Interrupt/Isoch 传输大小，偏移 非 EP0 `+0x10`。
    pub DEPTSIZ [
        /// Transfer Size
        ///
        /// 传输字节总数。
        XFERSIZE OFFSET(0) NUMBITS(19) [],
        /// Packet Count
        ///
        /// 包计数。
        PKTCNT OFFSET(19) NUMBITS(10) [],
        /// Multi Count / RX Data PID
        ///
        /// IN Isoch 多事务计数或 OUT Isoch PID 信息。
        MC_OR_RXDPID OFFSET(29) NUMBITS(2) [],
    ],
];

register_structs! {
    /// 单个主机通道寄存器块（ stride = `0x20`，基址 = `0x500 + n * 0x20`）。
    pub Dwc2HostChannel {
        /// Host Channel Characteristics
        ///
        /// 通道特性与启动/停止控制。
        (0x00 => pub hcchar: ReadWrite<u32, HCCHAR::Register>),
        /// Host Channel Split Control
        ///
        /// Split 事务控制（直连设备通常为 0）。
        (0x04 => pub hcsplt: ReadWrite<u32>),
        /// Host Channel Interrupt
        ///
        /// 通道中断状态（W1C）。
        (0x08 => pub hcint: ReadWrite<u32, HCINT::Register>),
        /// Host Channel Interrupt Mask
        ///
        /// 通道中断掩码。
        (0x0c => pub hcintmsk: ReadWrite<u32, HCINT::Register>),
        /// Host Channel Transfer Size
        ///
        /// 传输大小与 PID。
        (0x10 => pub hctsiz: ReadWrite<u32, HCTSIZ::Register>),
        /// Host Channel DMA Address
        ///
        /// DMA 缓冲区地址（须满足总线地址转换）。
        (0x14 => pub hcdma: ReadWrite<u32>),
        (0x18 => _reserved18),
        /// Host Channel DMA Buffer Address
        ///
        /// DMA 当前缓冲区地址（32-bit 平台只读为 0）。
        (0x1c => pub hcdmab: ReadOnly<u32>),
        (0x20 => @END),
    }
}

register_structs! {
    /// 单个 Device IN 端点寄存器块（stride = `0x20`，基址 = `0x900 + n * 0x20`）。
    pub Dwc2DevInEp {
        /// Device IN Endpoint Control
        ///
        /// IN 端点控制。
        (0x00 => pub diepctl: ReadWrite<u32, DIEPCTL::Register>),
        (0x04 => _reserved04),
        /// Device IN Endpoint Interrupt
        ///
        /// IN 端点中断状态。
        (0x08 => pub diepint: ReadWrite<u32, DIEPINT::Register>),
        (0x0c => _reserved0c),
        /// Device IN Endpoint Transfer Size
        ///
        /// 传输长度；EP0 使用 [`DEPTSIZ0`] 位域解析。
        (0x10 => pub dieptsiz: ReadWrite<u32>),
        /// Device IN Endpoint DMA Address
        ///
        /// IN DMA 缓冲区地址。
        (0x14 => pub diepdma: ReadWrite<u32>),
        /// Device IN Endpoint TX FIFO Status
        ///
        /// TX FIFO 剩余空间（32-bit 字）。
        (0x18 => pub dtxfsts: ReadOnly<u32>),
        /// Device IN Endpoint DMA Buffer Address
        ///
        /// IN DMA 当前缓冲区地址。
        (0x1c => pub diepdmab: ReadOnly<u32>),
        (0x20 => @END),
    }
}

register_structs! {
    /// 单个 Device OUT 端点寄存器块（stride = `0x20`，基址 = `0xB00 + n * 0x20`）。
    pub Dwc2DevOutEp {
        /// Device OUT Endpoint Control
        ///
        /// OUT 端点控制。
        (0x00 => pub doepctl: ReadWrite<u32, DOEPCTL::Register>),
        (0x04 => _reserved04),
        /// Device OUT Endpoint Interrupt
        ///
        /// OUT 端点中断状态。
        (0x08 => pub doepint: ReadWrite<u32, DOEPINT::Register>),
        (0x0c => _reserved0c),
        /// Device OUT Endpoint Transfer Size
        ///
        /// OUT 端点传输长度。
        (0x10 => pub doeptsiz: ReadWrite<u32>),
        /// Device OUT Endpoint DMA Address
        ///
        /// OUT DMA 缓冲区地址。
        (0x14 => pub doepdma: ReadWrite<u32>),
        (0x18 => _reserved18),
        /// Device OUT Endpoint DMA Buffer Address
        ///
        /// OUT DMA 当前缓冲区地址。
        (0x1c => pub doepdmab: ReadOnly<u32>),
        (0x20 => @END),
    }
}

/// 主机通道数组长度（`GHWCFG2.NUM_HOST_CHAN` 决定实际可用数，预留 16 槽）。
pub const DWC2_MAX_HOST_CHANNELS: usize = 16;
/// Device 端点数组长度（DWC2 通常 4–8 对，预留 16 与 host 对称）。
pub const DWC2_MAX_DEV_ENDPOINTS: usize = 16;

register_structs! {
    /// 完整 DWC2 寄存器映射（基址由 [`crate::usb::set_dwc2_base_virt`] 提供）。
    ///
    /// SG2002 DTS：`usb@04340000`，中断 30。仅声明本驱动访问的寄存器。
    pub Dwc2Regs {
        /// OTG Control and Status
        ///
        /// OTG 控制与状态。
        (0x000 => pub gotgctl: ReadWrite<u32, GOTGCTL::Register>),
        /// OTG Interrupt
        ///
        /// OTG 中断（W1C）。
        (0x004 => pub gotgint: ReadWrite<u32>),
        /// AHB Configuration
        ///
        /// AHB 总线配置。
        (0x008 => pub gahbcfg: ReadWrite<u32, GAHBCFG::Register>),
        /// USB Configuration
        ///
        /// USB/PHY 配置。
        (0x00c => pub gusbcfg: ReadWrite<u32, GUSBCFG::Register>),
        /// Reset Control
        ///
        /// 软复位与 FIFO flush。
        (0x010 => pub grstctl: ReadWrite<u32, GRSTCTL::Register>),
        /// Interrupt Status
        ///
        /// 全局中断状态。
        (0x014 => pub gintsts: ReadWrite<u32, GINTSTS::Register>),
        /// Interrupt Mask
        ///
        /// 全局中断掩码。
        (0x018 => pub gintmsk: ReadWrite<u32, GINTMSK::Register>),
        /// Receive Status Debug Read
        ///
        /// RX 状态（读不弹出 FIFO）。
        (0x01c => pub grxstsr: ReadOnly<u32>),
        /// Receive Status Read/Pop
        ///
        /// RX 状态弹出。
        (0x020 => pub grxstsp: ReadOnly<u32>),
        /// Receive FIFO Size
        ///
        /// RX FIFO 深度（32-bit 字）及起始偏移。
        (0x024 => pub grxfsiz: ReadWrite<u32>),
        /// Non-Periodic Transmit FIFO Size
        ///
        /// 非周期 TX FIFO 深度与起始偏移 `{depth, start}`。
        (0x028 => pub gnptxfsiz: ReadWrite<u32>),
        (0x02c => _reserved02c),
        /// Synopsys ID
        ///
        /// Synopsys Core 版本号（如 `0x4F54_420A`）。
        (0x040 => pub gsnpsid: ReadOnly<u32>),
        /// Hardware Configuration 1
        ///
        /// 硬件配置 1（端点方向能力位图）。
        (0x044 => pub ghwcfg1: ReadOnly<u32>),
        /// Hardware Configuration 2
        ///
        /// 硬件配置 2。
        (0x048 => pub ghwcfg2: ReadOnly<u32, GHWCFG2::Register>),
        /// Hardware Configuration 3
        ///
        /// 硬件配置 3（DFIFO 深度）。
        (0x04c => pub ghwcfg3: ReadOnly<u32, GHWCFG3::Register>),
        /// Hardware Configuration 4
        ///
        /// 硬件配置 4。
        (0x050 => pub ghwcfg4: ReadOnly<u32, GHWCFG4::Register>),
        (0x054 => _reserved054),
        /// Global Data FIFO Configuration
        ///
        /// 动态 FIFO 配置。
        (0x05c => pub gdfifocfg: ReadWrite<u32, GDFIFOCFG::Register>),
        (0x060 => _reserved060),
        /// Host Periodic Transmit FIFO Size
        ///
        /// 周期 TX FIFO 深度与起始（用于 Interrupt/Isoch IN）。
        (0x100 => pub hptxfsiz: ReadWrite<u32>),
        (0x104 => _reserved104),
        /// Host Configuration
        ///
        /// 主机配置。
        (0x400 => pub hcfg: ReadWrite<u32, HCFG::Register>),
        /// Host Frame Interval
        ///
        /// 帧间隔寄存器（微帧时基校准）。
        (0x404 => pub hfir: ReadWrite<u32>),
        /// Host Frame Number/Time Remaining
        ///
        /// 帧/微帧编号与剩余时间。
        (0x408 => pub hfnum: ReadOnly<u32, HFNUM::Register>),
        (0x40c => _reserved40c),
        /// Host All Channels Interrupt
        ///
        /// 主机全通道中断汇总（只读）。
        (0x414 => pub haint: ReadOnly<u32, HAINT::Register>),
        /// Host All Channels Interrupt Mask
        ///
        /// 主机通道中断掩码。
        (0x418 => pub haintmsk: ReadWrite<u32, HAINTMSK::Register>),
        (0x41c => _reserved41c),
        /// Host Port Control and Status
        ///
        /// 根端口状态与控制。
        (0x440 => pub hprt0: ReadWrite<u32, HPRT0::Register>),
        (0x444 => _reserved444),
        /// Host Channel Register Array
        ///
        /// 主机通道 0..[`DWC2_MAX_HOST_CHANNELS`] 寄存器组。
        (0x500 => pub hc: [Dwc2HostChannel; DWC2_MAX_HOST_CHANNELS]),
        (0x700 => _reserved700),
        /// Device Configuration
        ///
        /// 设备配置（DWC2 device 角色寄存器，当前驱动未使用）。
        (0x800 => pub dcfg: ReadWrite<u32, DCFG::Register>),
        /// Device Control
        ///
        /// 设备控制。
        (0x804 => pub dctl: ReadWrite<u32, DCTL::Register>),
        /// Device Status
        ///
        /// 设备状态。
        (0x808 => pub dsts: ReadOnly<u32, DSTS::Register>),
        (0x80c => _reserved80c),
        /// Device IN Endpoint Common Interrupt Mask
        ///
        /// IN 端点公共中断掩码。
        (0x810 => pub diepmsk: ReadWrite<u32, DEPMSK::Register>),
        /// Device OUT Endpoint Common Interrupt Mask
        ///
        /// OUT 端点公共中断掩码。
        (0x814 => pub doepmsk: ReadWrite<u32, DEPMSK::Register>),
        /// Device All Endpoints Interrupt
        ///
        /// 端点中断挂起位图（只读）。
        (0x818 => pub daint: ReadOnly<u32, DAINTMSK::Register>),
        /// Device All Endpoints Interrupt Mask
        ///
        /// 端点中断掩码。
        (0x81c => pub daintmsk: ReadWrite<u32, DAINTMSK::Register>),
        (0x820 => _reserved820),
        /// Device IN Endpoint Register Array
        ///
        /// IN 端点寄存器组。
        (0x900 => pub diep: [Dwc2DevInEp; DWC2_MAX_DEV_ENDPOINTS]),
        /// Device OUT Endpoint Register Array
        ///
        /// OUT 端点寄存器组。
        (0xb00 => pub doep: [Dwc2DevOutEp; DWC2_MAX_DEV_ENDPOINTS]),
        (0xd00 => _reservedd00),
        /// Power and Clock Gating Control
        ///
        /// PHY 时钟门控（CV182x 主机初始化写 0 保持 PHY 时钟）。
        (0xe00 => pub pcgctl: ReadWrite<u32>),
        (0xe04 => @END),
    }
}

register_bitfields![u32,
    /// PHY Register 014
    ///
    /// CV182x 片内 USB2 PHY UTMI 信号软件覆盖，偏移 `0x014`。
    ///
    /// Host 路径要求 `UTMI_OVERRIDE=0`（DWC2 接管 UTMI）；`TERMSEL=0` 以允许 HS chirp。
    pub PhyReg014 [
        /// UTMI Override
        ///
        /// 软件驱动 UTMI 信号；0 = DWC2 接管，host 必须为 0。
        UTMI_OVERRIDE OFFSET(0) NUMBITS(1) [],
        /// UTMI Operating Mode
        ///
        /// UTMI 工作模式；host 用 `Normal`。
        OPMODE OFFSET(1) NUMBITS(2) [
            /// Normal Mode   正常模式
            Normal = 0,
            /// Non-Driving Mode   非驱动模式
            NonDriving = 1,
            /// Disable Bit Stuffing and NRZI   禁用位填充与 NRZI
            DisableBitStuffNRZI = 2,
        ],
        /// Transceiver Select
        ///
        /// 收发器选择；HS host 用 `HighSpeed`。
        XCVRSEL OFFSET(3) NUMBITS(2) [
            /// High-Speed Transceiver   高速收发器
            HighSpeed = 0,
            /// Full-Speed Transceiver   全速收发器
            FullSpeed = 1,
            /// Low-Speed Transceiver   低速收发器
            LowSpeed = 2,
            /// FS/LS Transceiver   全速/低速收发器
            FsLs = 3,
        ],
        /// Termination Select
        ///
        /// FS 终端电阻；host HS 模式须为 0 以允许 chirp。
        TERMSEL OFFSET(5) NUMBITS(1) [],
        /// DP Pull-Down
        ///
        /// D+ 下拉电阻；host 模式须使能。
        DPPULLDOWN OFFSET(6) NUMBITS(1) [],
        /// DM Pull-Down
        ///
        /// D- 下拉电阻；host 模式须使能。
        DMPULLDOWN OFFSET(7) NUMBITS(1) [],
        /// UTMI Reset
        ///
        /// UTMI 接口复位。
        UTMI_RESET OFFSET(8) NUMBITS(1) [],
    ],
];

register_structs! {
    /// CV182x 片内 USB2 PHY MMIO（DTS `usb@04340000` 第二段 `reg`）。
    ///
    /// 物理基址 [`crate::soc::CV182X_USB2_PHY_BASE`]；仅 `reg014` 有位域定义，其余为 vendor 原始寄存器。
    pub Cv182xUsb2Phy {
        /// PHY Vendor Control 0
        ///
        /// PHY 厂商控制寄存器 0。
        (0x000 => pub reg000: ReadWrite<u32>),
        /// PHY Vendor Control 1
        ///
        /// PHY 厂商控制寄存器 1。
        (0x004 => pub reg004: ReadWrite<u32>),
        /// PHY Vendor Control 2
        ///
        /// PHY 厂商控制寄存器 2。
        (0x008 => pub reg008: ReadWrite<u32>),
        /// PHY Vendor Control 3
        ///
        /// PHY 厂商控制寄存器 3。
        (0x00c => pub reg00c: ReadWrite<u32>),
        /// PHY Vendor Control 4
        ///
        /// PHY 厂商控制寄存器 4。
        (0x010 => pub reg010: ReadWrite<u32>),
        /// UTMI Override and Control
        ///
        /// UTMI 信号软件覆盖与控制（Host bring-up 关键寄存器）。
        (0x014 => pub reg014: ReadWrite<u32, PhyReg014::Register>),
        /// PHY Vendor Control 6
        ///
        /// PHY 厂商控制寄存器 6。
        (0x018 => pub reg018: ReadWrite<u32>),
        /// PHY Vendor Control 7
        ///
        /// PHY 厂商控制寄存器 7。
        (0x01c => pub reg01c: ReadWrite<u32>),
        /// PHY Vendor Control 8
        ///
        /// PHY 厂商控制寄存器 8。
        (0x020 => pub reg020: ReadWrite<u32>),
        /// PHY Vendor Control 9
        ///
        /// PHY 厂商控制寄存器 9。
        (0x024 => pub reg024: ReadWrite<u32>),
        /// PHY Vendor Control 10
        ///
        /// PHY 厂商控制寄存器 10。
        (0x028 => pub reg028: ReadWrite<u32>),
        /// PHY Vendor Control 11
        ///
        /// PHY 厂商控制寄存器 11。
        (0x02c => pub reg02c: ReadWrite<u32>),
        /// PHY Vendor Control 12
        ///
        /// PHY 厂商控制寄存器 12。
        (0x030 => pub reg030: ReadWrite<u32>),
        /// PHY Vendor Control 13
        ///
        /// PHY 厂商控制寄存器 13。
        (0x034 => pub reg034: ReadWrite<u32>),
        /// PHY Vendor Control 14
        ///
        /// PHY 厂商控制寄存器 14。
        (0x038 => pub reg038: ReadWrite<u32>),
        /// PHY Vendor Control 15
        ///
        /// PHY 厂商控制寄存器 15。
        (0x03c => pub reg03c: ReadWrite<u32>),
        /// PHY Vendor Control 16
        ///
        /// PHY 厂商控制寄存器 16。
        (0x040 => pub reg040: ReadWrite<u32>),
        /// PHY Vendor Control 17
        ///
        /// PHY 厂商控制寄存器 17。
        (0x044 => pub reg044: ReadWrite<u32>),
        /// PHY Vendor Control 18
        ///
        /// PHY 厂商控制寄存器 18。
        (0x048 => pub reg048: ReadWrite<u32>),
        /// PHY Vendor Control 19
        ///
        /// PHY 厂商控制寄存器 19。
        (0x04c => pub reg04c: ReadWrite<u32>),
        /// PHY Vendor Control 20
        ///
        /// PHY 厂商控制寄存器 20。
        (0x050 => pub reg050: ReadWrite<u32>),
        /// PHY Vendor Control 21
        ///
        /// PHY 厂商控制寄存器 21。
        (0x054 => pub reg054: ReadWrite<u32>),
        (0x058 => @END),
    }
}
