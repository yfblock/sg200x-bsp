//! SG2002 / CV1812H 板载 Synopsys DesignWare MAC（DWMAC 3.70a）轮询驱动 —— **纯硬件层**。
//!
//! 控制器位于 `0x0407_0000`，自带内部 EPHY；本驱动覆盖：
//!
//! - 时钟门控（CLKGEN bit25/26）+ ETH MAC 软复位 + EPHY 软复位
//! - DMA 软复位、TX/RX 描述符环、DMA bus mode（PBL=8、64 byte stride）
//! - MDIO bring-up（PHY 软复位 → BMCR 自协商 → 链路状态轮询）
//!
//! 本模块**不再实现** `axdriver_*` 的 trait —— 它只暴露**中性**的 `transmit / receive` API。
//! 与 ArceOS 的胶水（`BaseDriverOps + NetDriverOps`、`NetBufPool` 等）请放到上层 crate
//! （例如 `sg2002-arceos/modules/axdriver/src/cvitek_eth.rs`），通过 [`CvitekEthNic`] 的
//! 公开方法（`transmit / receive / mac_address / can_transmit / can_receive`）做适配，
//! 让 BSP 可独立用于其它任意 RTOS / bare-metal 工程。
//!
//! 所有 GMAC / DMA 寄存器都通过 [`super::regs`] 的 `tock-registers` 视图访问，
//! 不再有零散的 `read_volatile / write_volatile`。

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{Ordering, fence};

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::desc::{
    BUF_SIZE, DmaDesc, RDES0_ES, RDES0_FL_MASK, RDES0_FL_SHIFT, RDES0_OWN, RDES1_RBS1_MASK,
    RDES1_RER, RX_RING_SIZE, TDES0_OWN, TDES1_FS, TDES1_IC, TDES1_LS, TDES1_TBS1_MASK, TDES1_TER,
    TX_RING_SIZE,
};
use super::mdio::{mdio_read, mdio_write};
use super::regs::{
    Addr0High, DmaBusMode, DmaOperation, DmaStatus, Dwc3DmaRegs, Dwc3GmacRegs, FrameFilter,
    MacControl, dma_regs, gmac_regs,
};
use crate::utils::cache::{dcache_clean_range, dcache_invalidate_range};

/// SG2002 板载 GMAC 默认 MMIO 基地址（`cvitek,cv1810-eth` DTS 节点 `ethernet@4070000`）。
pub const ETH_BASE: usize = 0x0407_0000;

/// 默认 PHY 地址（PHY0 直接挂在内部 EPHY MDIO 总线上）。
const PHY_ADDR: u32 = 0;

/// CLKGEN 顶层基地址：`clock-controller`（`cvitek,cv181x-clk`）。
const CLKGEN_BASE: usize = 0x0300_2000;
/// `REG_CLK_EN_0` 偏移（参考 Linux `clk-cv181x.c`）。
const REG_CLK_EN_0: usize = 0x000;
/// CLKGEN bit25：clk_axi4_eth0；bit26：clk_eth0_500m（参考板级 U-Boot `cvitek_eth.c`）。
const CLKEN0_ETH_MASK: u32 = (1 << 25) | (1 << 26);

/// `RstcRegisters::soft_rstn_3` bit0/1：内部 EPHY 复位；写 1 解除复位。
const SOFT_RSTN3_EPHY_MASK: u32 = (1 << 0) | (1 << 1);

/// 最小以太帧（不含 CRC）；短帧需要 pad 到此长度。
const MIN_ETH_FRAME: usize = 60;

/// BSP 内部硬件错误码（与上层 OS 解耦，便于不同 wrapper 自行映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthError {
    /// 资源暂不可用（无可用 TX 描述符 / 无新 RX 帧）。
    Again,
    /// 申请缓冲失败。
    NoMemory,
    /// 输入 packet 过大或非法。
    BadParam,
}

/// `Result<T, EthError>` 的别名，方便上层 wrapper 引用。
pub type EthResult<T> = Result<T, EthError>;

/// `BUF_SIZE` 对齐到 64 字节的连续 DMA buffer，确保 cache 维护按行干净。
#[repr(C, align(64))]
struct DmaPktBuf([u8; BUF_SIZE]);

impl DmaPktBuf {
    fn new() -> Self {
        Self([0u8; BUF_SIZE])
    }
}

/// SG2002 GMAC + 内部 EPHY 主驱动 —— **不依赖任何 axdriver crate**。
///
/// 调用流程：
/// 1. [`CvitekEthNic::init`] 完成时钟/复位 → DMA → MDIO → 启动 TX/RX。
/// 2. 业务侧在轮询循环里：
///    - 如果 [`Self::can_transmit`] 返回 true，可调用 [`Self::transmit`] 发包；
///    - 如果 [`Self::can_receive`] 返回 true，调用 [`Self::receive`] 拿到 [`RxToken`]，
///      读完 `frame()` 后让 token drop 即自动把 buffer 还给 DMA。
pub struct CvitekEthNic {
    base: usize,
    mac_addr: [u8; 6],
    tx_descs: Vec<DmaDesc>,
    rx_descs: Vec<DmaDesc>,
    /// 每个 TX desc 对应一个静态 cache-line 对齐 buffer，避免上层做 DMA 一致性。
    tx_bufs: Vec<Box<DmaPktBuf>>,
    /// 每个 RX desc 对应一个 cache-line 对齐 buffer，DMA 直接写入这里。
    rx_bufs: Vec<Box<DmaPktBuf>>,
    tx_head: usize,
    tx_tail: usize,
    rx_cur: usize,
    rx_count: u32,
    tx_count: u32,
}

unsafe impl Send for CvitekEthNic {}
unsafe impl Sync for CvitekEthNic {}

impl CvitekEthNic {
    /// 软件视角的 TX 队列容量（== 描述符环深度）。
    pub const TX_QUEUE_SIZE: usize = TX_RING_SIZE;
    /// 软件视角的 RX 队列容量（== 描述符环深度）。
    pub const RX_QUEUE_SIZE: usize = RX_RING_SIZE;
    /// 单帧最大字节数（含 FCS 之前），等同于 DMA buffer 长度。
    pub const MAX_FRAME_LEN: usize = BUF_SIZE;

    #[inline]
    fn gmac(&self) -> &Dwc3GmacRegs {
        unsafe { gmac_regs(self.base) }
    }

    #[inline]
    fn dma(&self) -> &Dwc3DmaRegs {
        unsafe { dma_regs(self.base) }
    }

    fn dma_reset(&self) {
        let dma = self.dma();
        dma.bus_mode.modify(DmaBusMode::SWR::SET);
        let mut t = 100_000u32;
        while dma.bus_mode.is_set(DmaBusMode::SWR) {
            t = t.wrapping_sub(1);
            if t == 0 {
                log::warn!("cvitek-eth: DMA reset timeout");
                break;
            }
        }
    }

    fn read_mac_from_hw(&self) -> [u8; 6] {
        let gmac = self.gmac();
        let hi = gmac.addr0_high.get();
        let lo = gmac.addr0_low.get();
        [
            (lo & 0xFF) as u8,
            ((lo >> 8) & 0xFF) as u8,
            ((lo >> 16) & 0xFF) as u8,
            ((lo >> 24) & 0xFF) as u8,
            (hi & 0xFF) as u8,
            ((hi >> 8) & 0xFF) as u8,
        ]
    }

    fn set_mac_hw(&self, m: &[u8; 6]) {
        let lo = (m[0] as u32)
            | ((m[1] as u32) << 8)
            | ((m[2] as u32) << 16)
            | ((m[3] as u32) << 24);
        let hi_field = (m[4] as u32) | ((m[5] as u32) << 8);
        self.gmac().addr0_low.set(lo);
        self.gmac()
            .addr0_high
            .write(Addr0High::ADDR_HI.val(hi_field) + Addr0High::ADDR_ENABLE::SET);
    }

    fn flush_desc(desc: &DmaDesc) {
        dcache_clean_range(desc as *const DmaDesc as usize, core::mem::size_of::<DmaDesc>());
    }

    fn invalidate_desc(desc: &DmaDesc) {
        dcache_invalidate_range(desc as *const DmaDesc as usize, core::mem::size_of::<DmaDesc>());
    }

    fn setup_tx_ring(&mut self) {
        for i in 0..TX_RING_SIZE {
            let buf_pa = self.tx_bufs[i].0.as_ptr() as u32;
            let d = &mut self.tx_descs[i];
            d.des0 = 0;
            d.des1 = if i == TX_RING_SIZE - 1 { TDES1_TER } else { 0 };
            d.des2 = buf_pa;
            d.des3 = 0;
            Self::flush_desc(d);
        }
    }

    fn setup_rx_ring(&mut self) {
        for i in 0..RX_RING_SIZE {
            let data_pa = self.rx_bufs[i].0.as_ptr() as u32;

            let d = &mut self.rx_descs[i];
            d.des2 = data_pa;
            d.des1 = {
                let mut v = (BUF_SIZE as u32).min(RDES1_RBS1_MASK);
                if i == RX_RING_SIZE - 1 {
                    v |= RDES1_RER;
                }
                v
            };
            d.des3 = 0;
            fence(Ordering::Release);
            d.des0 = RDES0_OWN;
            Self::flush_desc(d);
        }
    }

    /// 时钟门 + ETH/EPHY 软复位释放（CLKGEN + RSTC）。
    ///
    /// 与原始 U-Boot `cvi_eth_init`/`cv181x_init` 行为一致：
    /// - CLKGEN.REG_CLK_EN_0 |= bit25 | bit26 → 打开 ETH AXI/500M 时钟。
    /// - RSTC.SOFT_RSTN_0 |= bit12      → 释放 ETH0 MAC 复位（低有效，写 1 = 解除）。
    /// - RSTC.SOFT_RSTN_3 |= bit0|bit1  → 释放内部 EPHY 复位。
    ///
    /// # Safety
    /// 仅应在 BSP 初始化路径中调用一次。
    unsafe fn enable_clocks_and_release_resets() {
        unsafe {
            let clk_en0 = (CLKGEN_BASE + REG_CLK_EN_0) as *mut u32;
            let v = core::ptr::read_volatile(clk_en0);
            core::ptr::write_volatile(clk_en0, v | CLKEN0_ETH_MASK);
        }

        let rstc = unsafe { crate::rstc::Rstc::new() };
        let regs = rstc.regs();

        let v0 = regs.soft_rstn_0.get();
        regs.soft_rstn_0.set(v0 | (1 << 12));

        let v3 = regs.soft_rstn_3.get();
        regs.soft_rstn_3.set(v3 | SOFT_RSTN3_EPHY_MASK);

        for _ in 0..2_000_000u32 {
            core::hint::spin_loop();
        }
    }

    /// 完整 bring-up：时钟/复位 → DMA 软复位 → 描述符环 → MAC 配置 → PHY 协商 → 启动 TX/RX。
    pub fn init(base: usize) -> EthResult<Self> {
        let mut tx_descs = Vec::with_capacity(TX_RING_SIZE);
        let mut rx_descs = Vec::with_capacity(RX_RING_SIZE);
        for _ in 0..TX_RING_SIZE {
            tx_descs.push(DmaDesc::zero());
        }
        for _ in 0..RX_RING_SIZE {
            rx_descs.push(DmaDesc::zero());
        }

        let mut tx_bufs: Vec<Box<DmaPktBuf>> = Vec::with_capacity(TX_RING_SIZE);
        let mut rx_bufs: Vec<Box<DmaPktBuf>> = Vec::with_capacity(RX_RING_SIZE);
        for _ in 0..TX_RING_SIZE {
            tx_bufs.push(Box::new(DmaPktBuf::new()));
        }
        for _ in 0..RX_RING_SIZE {
            rx_bufs.push(Box::new(DmaPktBuf::new()));
        }

        let mut nic = Self {
            base,
            mac_addr: [0; 6],
            tx_descs,
            rx_descs,
            tx_bufs,
            rx_bufs,
            tx_head: 0,
            tx_tail: 0,
            rx_cur: 0,
            rx_count: 0,
            tx_count: 0,
        };

        let ver = nic.gmac().version.get();
        log::info!("cvitek-eth: DWMAC version {:#x}", ver);

        nic.mac_addr = nic.read_mac_from_hw();
        if nic.mac_addr == [0; 6] || nic.mac_addr == [0xFF; 6] {
            nic.mac_addr = [0x00, 0x50, 0x43, 0x02, 0x02, 0x02];
        }
        log::info!(
            "cvitek-eth: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            nic.mac_addr[0], nic.mac_addr[1], nic.mac_addr[2],
            nic.mac_addr[3], nic.mac_addr[4], nic.mac_addr[5],
        );

        unsafe {
            Self::enable_clocks_and_release_resets();
        }

        nic.dma_reset();

        nic.setup_tx_ring();
        nic.setup_rx_ring();

        // DMA bus mode：PBL=8（突发 8 word）、DSL=12（描述符 stride 64 字节）、
        // FB=1 + AAL=1（固定突发 + 地址对齐，避免 EPHY DMA 桥跨 4K 拆包）。
        nic.dma().bus_mode.write(
            DmaBusMode::PBL.val(8)
                + DmaBusMode::DSL.val(12)
                + DmaBusMode::FB::SET
                + DmaBusMode::AAL::SET,
        );

        nic.dma().tx_base.set(nic.tx_descs.as_ptr() as u32);
        nic.dma().rx_base.set(nic.rx_descs.as_ptr() as u32);

        // 关 GMAC 中断（PMT/MMC 等）+ 关 DMA 中断（轮询模式）。
        nic.gmac().int_mask.set(0x60F);
        nic.dma().intr_ena.set(0);

        nic.set_mac_hw(&nic.mac_addr);

        // FrameFilter：促 promiscuous + receive-all + multicast hash 全开，
        // 让上层协议栈自行过滤；硬件层只做 CRC。
        nic.gmac()
            .frame_filter
            .write(FrameFilter::PR::SET + FrameFilter::RA::SET);
        nic.gmac().hash_high.set(0xFFFF_FFFF);
        nic.gmac().hash_low.set(0xFFFF_FFFF);

        // PHY init：BMCR.RESET → 等清零 → BMCR=0x3300（自协商使能 + 100M FD）。
        mdio_write(nic.gmac(), PHY_ADDR, 0, 0x8000);
        for _ in 0..1_000_000u32 {
            core::hint::spin_loop();
        }
        for _ in 0..100u32 {
            if mdio_read(nic.gmac(), PHY_ADDR, 0) & 0x8000 == 0 {
                break;
            }
            for _ in 0..100_000u32 {
                core::hint::spin_loop();
            }
        }
        mdio_write(nic.gmac(), PHY_ADDR, 0, 0x3300);

        // BMSR bit2 = link status；BMSR 是 latch-low，连读两次拿当前态。
        // 自协商完整等到 ~5s，避免 host 立即 ARP 时还在协商。
        let mut link_up = false;
        let mut bmcr = 0u16;
        // 0u16 仅作占位：for 循环里中途 break 或后续 if !link_up 路径都会重新赋值，
        // 但编译器流分析无法识别，所以这里的初值在某些路径下是被覆盖前不读 ——
        // 显式标注 unused_assignments 以避免 warning 噪声。
        #[allow(unused_assignments)]
        let mut bmsr = 0u16;
        let mut lpa = 0u16;
        for i in 0..2500u32 {
            let _ = mdio_read(nic.gmac(), PHY_ADDR, 1);
            bmsr = mdio_read(nic.gmac(), PHY_ADDR, 1);
            if bmsr & 4 != 0 {
                bmcr = mdio_read(nic.gmac(), PHY_ADDR, 0);
                lpa = mdio_read(nic.gmac(), PHY_ADDR, 5);
                log::info!(
                    "cvitek-eth: link UP after {} polls bmcr=0x{:04x} bmsr=0x{:04x} lpa=0x{:04x}",
                    i, bmcr, bmsr, lpa
                );
                link_up = true;
                break;
            }
            for _ in 0..200_000u32 {
                core::hint::spin_loop();
            }
        }
        if !link_up {
            bmcr = mdio_read(nic.gmac(), PHY_ADDR, 0);
            bmsr = mdio_read(nic.gmac(), PHY_ADDR, 1);
            lpa = mdio_read(nic.gmac(), PHY_ADDR, 5);
            log::warn!(
                "cvitek-eth: link still DOWN after autoneg timeout, continuing (bmcr=0x{:04x} bmsr=0x{:04x} lpa=0x{:04x})",
                bmcr, bmsr, lpa
            );
        }

        // U-Boot 留下的 mac_control 速度/双工很可能不对（实测 10M HD），必须根据 PHY 协商结果显式重写：
        // ANAR (reg 4) 是本端能力，LPA (reg 5) 是对端能力，按优先级 100M FD > 100M HD > 10M FD > 10M HD 取最高。
        let anar = mdio_read(nic.gmac(), PHY_ADDR, 4);
        let common = anar & lpa;
        // 速度/双工选择：高位优先（100MFD > 100MHD > 10MFD > 10MHD）。
        // 默认按 PHY 配置 100M FD，避免 link 没起来时配错。
        let (speed_100m, full_duplex) = if common & (1 << 8) != 0 {
            (true, true)
        } else if common & (1 << 7) != 0 {
            (true, false)
        } else if common & (1 << 6) != 0 {
            (false, true)
        } else if common & (1 << 5) != 0 {
            (false, false)
        } else {
            // 链路 DOWN 或自协商未完成：按 PHY 我们写入的 100M FD 默认值
            (
                bmcr & (1 << 13) != 0 || true,
                bmcr & (1 << 8) != 0 || true,
            )
        };
        log::info!(
            "cvitek-eth: link mode = {} {} (anar=0x{:04x} lpa=0x{:04x})",
            if speed_100m { "100M" } else { "10M" },
            if full_duplex { "FD" } else { "HD" },
            anar, lpa
        );

        // GMAC mac_control: PS=1(MII), FES=speed, DM=duplex, TE/RE=enable。
        // 直接 write 整字，丢弃 U-Boot 不可信的旧值。
        let mut mc = MacControl::PS::SET + MacControl::TE::SET + MacControl::RE::SET;
        if speed_100m {
            mc += MacControl::FES::SET;
        }
        if full_duplex {
            mc += MacControl::DM::SET;
        }
        nic.gmac().mac_control.write(mc);

        // 复位 MMC 计数器（写 1 自清）。
        nic.gmac().mmc_cntrl.set(0x01);

        // 启动 DMA：先 TSF/RSF/OSF/FTF，等 FTF 自清，再 ST/SR + RX poll。
        nic.dma().operation.write(
            DmaOperation::TSF::SET
                + DmaOperation::RSF::SET
                + DmaOperation::OSF::SET
                + DmaOperation::FTF::SET,
        );
        let mut t = 100_000u32;
        while nic.dma().operation.is_set(DmaOperation::FTF) {
            t = t.wrapping_sub(1);
            if t == 0 {
                break;
            }
        }
        nic.dma()
            .operation
            .modify(DmaOperation::ST::SET + DmaOperation::SR::SET);
        nic.dma().rx_poll.set(1);

        let g = nic.gmac();
        let d = nic.dma();
        log::debug!(
            "cvitek-eth: DUMP mac_ctl=0x{:08x} frame_filter=0x{:08x} addr0_hi=0x{:08x} addr0_lo=0x{:08x}",
            g.mac_control.get(), g.frame_filter.get(), g.addr0_high.get(), g.addr0_low.get()
        );
        log::debug!(
            "cvitek-eth: DUMP dma_bus=0x{:08x} dma_op=0x{:08x} dma_status=0x{:08x} tx_base=0x{:08x} rx_base=0x{:08x}",
            d.bus_mode.get(), d.operation.get(), d.status.get(), d.tx_base.get(), d.rx_base.get()
        );

        log::info!("cvitek-eth: initialized OK");
        Ok(nic)
    }

    /// 当前驱动持有的 MAC 地址（自协商前从硬件读出，全 0/0xff 时回退到固定值）。
    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_addr
    }

    /// 当前 TX 描述符是否空闲（=== 可调用 [`Self::transmit`] 而不会立刻拒收）。
    pub fn can_transmit(&self) -> bool {
        Self::invalidate_desc(&self.tx_descs[self.tx_head]);
        let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[self.tx_head].des0) };
        des0 & TDES0_OWN == 0
    }

    /// 当前 RX 描述符是否已经被 DMA 写满（=== [`Self::receive`] 不会返回 [`EthError::Again`]）。
    pub fn can_receive(&self) -> bool {
        Self::invalidate_desc(&self.rx_descs[self.rx_cur]);
        let des0 = unsafe { core::ptr::read_volatile(&self.rx_descs[self.rx_cur].des0) };
        des0 & RDES0_OWN == 0
    }

    /// 回收已完成的 TX 描述符（不持有上层 buffer，所以只是推进 tx_tail）。
    /// 调用方一般不用单独调，因为 `transmit()` 内部会先回收一次。
    pub fn reclaim_tx(&mut self) {
        while self.tx_tail != self.tx_head {
            Self::invalidate_desc(&self.tx_descs[self.tx_tail]);
            let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[self.tx_tail].des0) };
            if des0 & TDES0_OWN != 0 {
                break;
            }
            self.tx_tail = (self.tx_tail + 1) % TX_RING_SIZE;
        }
    }

    /// 把 `packet` 复制到内部 TX buffer 后启动 DMA。
    ///
    /// - 短帧（< 60 字节）会自动 0-pad 到最小以太帧（CRC 由 MAC 自动补 4 字节）。
    /// - `packet.len() > MAX_FRAME_LEN` 返回 [`EthError::BadParam`]。
    /// - 没空闲描述符返回 [`EthError::Again`]。
    pub fn transmit(&mut self, packet: &[u8]) -> EthResult<()> {
        if packet.len() > Self::MAX_FRAME_LEN {
            return Err(EthError::BadParam);
        }

        self.reclaim_tx();

        let idx = self.tx_head;
        Self::invalidate_desc(&self.tx_descs[idx]);
        let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[idx].des0) };
        if des0 & TDES0_OWN != 0 {
            return Err(EthError::Again);
        }

        // 拷贝到内部 buffer，按需 pad 到 60 字节。
        let buf = &mut self.tx_bufs[idx].0;
        buf[..packet.len()].copy_from_slice(packet);
        let mut len = packet.len();
        if len < MIN_ETH_FRAME {
            for byte in &mut buf[len..MIN_ETH_FRAME] {
                *byte = 0;
            }
            len = MIN_ETH_FRAME;
        }
        let data_pa = buf.as_ptr() as usize;
        dcache_clean_range(data_pa, len);

        let d = &mut self.tx_descs[idx];

        let mut tdes1 = TDES1_IC | TDES1_FS | TDES1_LS | ((len as u32) & TDES1_TBS1_MASK);
        if idx == TX_RING_SIZE - 1 {
            tdes1 |= TDES1_TER;
        }

        unsafe {
            core::ptr::write_volatile(&mut d.des2, data_pa as u32);
            core::ptr::write_volatile(&mut d.des1, tdes1);
        }

        fence(Ordering::Release);
        unsafe { core::ptr::write_volatile(&mut d.des0, TDES0_OWN) };
        Self::flush_desc(d);
        // 数据 buffer 再 clean 一次：上面 mut 写 desc 可能 evict 了同 cache line 的数据。
        dcache_clean_range(data_pa, len);

        self.tx_count = self.tx_count.wrapping_add(1);
        if log::log_enabled!(log::Level::Trace) {
            let n = packet.len().min(20);
            log::trace!(
                "cvitek-eth: TX#{} idx={} len={} hdr={:02x?}",
                self.tx_count, idx, len, &packet[..n]
            );
        }

        self.tx_head = (self.tx_head + 1) % TX_RING_SIZE;

        self.dma().tx_poll.set(1);
        Ok(())
    }

    /// 取出当前 RX 帧。返回的 [`RxToken`] 在 drop 时会自动把 buffer 还给 DMA。
    pub fn receive(&mut self) -> EthResult<RxToken<'_>> {
        let idx = self.rx_cur;

        // 软件 ack RI/NIS（写 1 清），DMA 才会继续在下一帧到达时触发。
        if self.dma().status.is_set(DmaStatus::RI) {
            self.dma()
                .status
                .write(DmaStatus::RI::SET + DmaStatus::NIS::SET);
        }

        Self::invalidate_desc(&self.rx_descs[idx]);
        let des0 = unsafe { core::ptr::read_volatile(&self.rx_descs[idx].des0) };

        if des0 & RDES0_OWN != 0 {
            return Err(EthError::Again);
        }

        if des0 & RDES0_ES != 0 {
            // 错误帧：还回 DMA，跳过。
            self.requeue_rx(idx);
            self.rx_cur = (self.rx_cur + 1) % RX_RING_SIZE;
            return Err(EthError::Again);
        }

        let frame_len = ((des0 & RDES0_FL_MASK) >> RDES0_FL_SHIFT) as usize;
        // RDES0.FL 含 4 字节 FCS（CRC），交付上层时去掉。
        let frame_len = if frame_len >= 4 { frame_len - 4 } else { frame_len }
            .min(BUF_SIZE);

        // DMA 写完的数据：在交给调用方读之前 invalidate 自己的 buffer。
        let buf_va = self.rx_bufs[idx].0.as_ptr() as usize;
        dcache_invalidate_range(buf_va, frame_len);

        self.rx_cur = (self.rx_cur + 1) % RX_RING_SIZE;
        self.rx_count = self.rx_count.wrapping_add(1);
        if log::log_enabled!(log::Level::Trace) {
            let raw = &self.rx_bufs[idx].0[..];
            let n = frame_len.min(20);
            log::trace!(
                "cvitek-eth: RX#{} idx={} len={} hdr={:02x?}",
                self.rx_count, idx, frame_len, &raw[..n]
            );
        }

        Ok(RxToken {
            nic: self,
            slot: idx,
            len: frame_len,
        })
    }

    /// 把 RX desc 重新交给 DMA，让它在下一轮继续接收。
    fn requeue_rx(&mut self, slot: usize) {
        let pa = self.rx_bufs[slot].0.as_ptr() as u32;
        let d = &mut self.rx_descs[slot];
        unsafe {
            core::ptr::write_volatile(&mut d.des2, pa);
            let mut rdes1 = (BUF_SIZE as u32).min(RDES1_RBS1_MASK);
            if slot == RX_RING_SIZE - 1 {
                rdes1 |= RDES1_RER;
            }
            core::ptr::write_volatile(&mut d.des1, rdes1);
            core::ptr::write_volatile(&mut d.des3, 0);
            fence(Ordering::Release);
            core::ptr::write_volatile(&mut d.des0, RDES0_OWN);
        }
        Self::flush_desc(d);
        self.dma()
            .status
            .write(DmaStatus::RI::SET + DmaStatus::NIS::SET);
        self.dma().rx_poll.set(1);
    }
}

/// [`CvitekEthNic::receive`] 返回的"借出"的 RX 帧。
///
/// 持有期间该 RX slot 的 DMA buffer 借给调用方使用；drop 时 BSP 自动把 desc 重新交给 DMA。
/// 这样上层 wrapper（例如 axdriver_net 的 `NetDriverOps::receive`）可以把数据复制到自己的
/// `NetBufPool`，然后让 token 自然 drop，无需显式 release。
pub struct RxToken<'a> {
    nic: &'a mut CvitekEthNic,
    slot: usize,
    len: usize,
}

impl<'a> RxToken<'a> {
    /// 当前 RX 帧的字节切片（不含 FCS）。生命周期与 `RxToken` 绑定。
    #[inline]
    pub fn frame(&self) -> &[u8] {
        &self.nic.rx_bufs[self.slot].0[..self.len]
    }

    /// RX 帧长度（不含 FCS）。
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// 帮助 Clippy/lint：与标准 `is_empty()` 习惯保持一致。
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> Drop for RxToken<'a> {
    fn drop(&mut self) {
        let slot = self.slot;
        self.nic.requeue_rx(slot);
    }
}
