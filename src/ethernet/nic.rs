//! SG2002 / CV1812H 板载 Synopsys DesignWare MAC（DWMAC 3.70a）轮询驱动。
//!
//! 控制器位于 `0x0407_0000`，自带内部 EPHY；本驱动覆盖：
//!
//! - 时钟门控（CLKGEN bit25/26）+ ETH MAC 软复位 + EPHY 软复位
//! - DMA 软复位、TX/RX 描述符环、DMA bus mode（PBL=8、64 byte stride）
//! - MDIO bring-up（PHY 软复位 → BMCR 自协商 → 链路状态轮询）
//! - 实现 [`axdriver_base::BaseDriverOps`] 与 [`axdriver_net::NetDriverOps`]
//!
//! 所有 GMAC / DMA 寄存器都通过 [`super::regs`] 的 `tock-registers` 视图访问，
//! 不再有零散的 `read_volatile / write_volatile`。

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{fence, Ordering};

use axdriver_base::{BaseDriverOps, DevError, DevResult, DeviceType};
use axdriver_net::{EthernetAddress, NetBuf, NetBufBox, NetBufPool, NetBufPtr, NetDriverOps};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::utils::cache::{dcache_clean_range, dcache_invalidate_range};
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

/// SG2002 GMAC + 内部 EPHY 主驱动。
pub struct CvitekEthNic {
    base: usize,
    mac_addr: [u8; 6],
    tx_pool: Arc<NetBufPool>,
    rx_pool: Arc<NetBufPool>,
    tx_descs: Vec<DmaDesc>,
    rx_descs: Vec<DmaDesc>,
    tx_bufs: Vec<Option<NetBufPtr>>,
    rx_bufs: Vec<Option<NetBufBox>>,
    tx_head: usize,
    tx_tail: usize,
    rx_cur: usize,
    rx_count: u32,
    tx_count: u32,
}

unsafe impl Send for CvitekEthNic {}
unsafe impl Sync for CvitekEthNic {}

impl CvitekEthNic {
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
            let d = &mut self.tx_descs[i];
            d.des0 = 0;
            d.des1 = if i == TX_RING_SIZE - 1 { TDES1_TER } else { 0 };
            d.des2 = 0;
            d.des3 = 0;
            Self::flush_desc(d);
        }
    }

    fn setup_rx_ring(&mut self) {
        for i in 0..RX_RING_SIZE {
            let buf = self.rx_pool.alloc_boxed().expect("RX buf alloc");
            let data_pa = buf.raw_buf().as_ptr() as u32;

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
            self.rx_bufs[i] = Some(buf);
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
    pub fn init(base: usize) -> DevResult<Self> {
        let tx_pool = NetBufPool::new(TX_RING_SIZE + 16, BUF_SIZE)?;
        let rx_pool = NetBufPool::new(RX_RING_SIZE + 16, BUF_SIZE)?;

        let mut tx_descs = Vec::with_capacity(TX_RING_SIZE);
        let mut rx_descs = Vec::with_capacity(RX_RING_SIZE);
        for _ in 0..TX_RING_SIZE {
            tx_descs.push(DmaDesc::zero());
        }
        for _ in 0..RX_RING_SIZE {
            rx_descs.push(DmaDesc::zero());
        }

        let mut tx_bufs: Vec<Option<NetBufPtr>> = Vec::with_capacity(TX_RING_SIZE);
        let mut rx_bufs: Vec<Option<NetBufBox>> = Vec::with_capacity(RX_RING_SIZE);
        for _ in 0..TX_RING_SIZE {
            tx_bufs.push(None);
        }
        for _ in 0..RX_RING_SIZE {
            rx_bufs.push(None);
        }

        let mut nic = Self {
            base,
            mac_addr: [0; 6],
            tx_pool,
            rx_pool,
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
        // 让 smoltcp 自行过滤；硬件层只做 CRC。
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
}

impl BaseDriverOps for CvitekEthNic {
    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn device_name(&self) -> &str {
        "cvitek-eth"
    }
}

impl NetDriverOps for CvitekEthNic {
    fn mac_address(&self) -> EthernetAddress {
        EthernetAddress(self.mac_addr)
    }

    fn can_transmit(&self) -> bool {
        Self::invalidate_desc(&self.tx_descs[self.tx_head]);
        let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[self.tx_head].des0) };
        des0 & TDES0_OWN == 0
    }

    fn can_receive(&self) -> bool {
        Self::invalidate_desc(&self.rx_descs[self.rx_cur]);
        let des0 = unsafe { core::ptr::read_volatile(&self.rx_descs[self.rx_cur].des0) };
        des0 & RDES0_OWN == 0
    }

    fn rx_queue_size(&self) -> usize {
        RX_RING_SIZE
    }

    fn tx_queue_size(&self) -> usize {
        TX_RING_SIZE
    }

    fn recycle_rx_buffer(&mut self, rx_buf: NetBufPtr) -> DevResult {
        unsafe {
            let _ = NetBuf::from_buf_ptr(rx_buf);
        }
        Ok(())
    }

    fn recycle_tx_buffers(&mut self) -> DevResult {
        while self.tx_tail != self.tx_head {
            Self::invalidate_desc(&self.tx_descs[self.tx_tail]);
            let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[self.tx_tail].des0) };
            if des0 & TDES0_OWN != 0 {
                break;
            }
            if let Some(buf) = self.tx_bufs[self.tx_tail].take() {
                unsafe {
                    let _ = NetBuf::from_buf_ptr(buf);
                }
            }
            self.tx_tail = (self.tx_tail + 1) % TX_RING_SIZE;
        }
        Ok(())
    }

    fn transmit(&mut self, tx_buf: NetBufPtr) -> DevResult {
        let idx = self.tx_head;

        Self::invalidate_desc(&self.tx_descs[idx]);
        let des0 = unsafe { core::ptr::read_volatile(&self.tx_descs[idx].des0) };
        if des0 & TDES0_OWN != 0 {
            return Err(DevError::Again);
        }

        let mut len = tx_buf.packet_len();
        if len < 60 {
            // pad 到最小以太网帧（CRC 由 MAC 自动补）。
            let end = tx_buf.packet().as_ptr() as usize + len;
            unsafe { core::ptr::write_bytes(end as *mut u8, 0, 60 - len) };
            len = 60;
        }
        let data = tx_buf.packet().as_ptr() as usize;
        dcache_clean_range(data, len);

        let d = &mut self.tx_descs[idx];

        let mut tdes1 = TDES1_IC | TDES1_FS | TDES1_LS | ((len as u32) & TDES1_TBS1_MASK);
        if idx == TX_RING_SIZE - 1 {
            tdes1 |= TDES1_TER;
        }

        unsafe {
            core::ptr::write_volatile(&mut d.des2, data as u32);
            core::ptr::write_volatile(&mut d.des1, tdes1);
        }

        fence(Ordering::Release);
        unsafe { core::ptr::write_volatile(&mut d.des0, TDES0_OWN) };
        Self::flush_desc(d);
        // 数据缓冲再 clean 一次：上面 mut 写完 desc，可能 evict 了同 cache line 的数据。
        dcache_clean_range(data, len);

        self.tx_count = self.tx_count.wrapping_add(1);
        if log::log_enabled!(log::Level::Trace) {
            let raw = tx_buf.packet();
            let n = raw.len().min(20);
            log::trace!(
                "cvitek-eth: TX#{} idx={} len={} hdr={:02x?}",
                self.tx_count, idx, len, &raw[..n]
            );
        }

        self.tx_bufs[idx] = Some(tx_buf);
        self.tx_head = (self.tx_head + 1) % TX_RING_SIZE;

        self.dma().tx_poll.set(1);
        Ok(())
    }

    fn receive(&mut self) -> DevResult<NetBufPtr> {
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
            return Err(DevError::Again);
        }

        if des0 & RDES0_ES != 0 {
            // 错误帧：还回 DMA，跳过。
            unsafe { core::ptr::write_volatile(&mut self.rx_descs[idx].des0, RDES0_OWN) };
            Self::flush_desc(&self.rx_descs[idx]);
            self.rx_cur = (self.rx_cur + 1) % RX_RING_SIZE;
            self.dma().rx_poll.set(1);
            return Err(DevError::Again);
        }

        let frame_len = ((des0 & RDES0_FL_MASK) >> RDES0_FL_SHIFT) as usize;
        // RDES0.FL 含 4 字节 FCS（CRC），交付上层时去掉。
        let frame_len = if frame_len >= 4 { frame_len - 4 } else { frame_len };

        let mut buf = self.rx_bufs[idx].take().ok_or(DevError::Again)?;
        dcache_invalidate_range(buf.raw_buf().as_ptr() as usize, frame_len);

        self.rx_count = self.rx_count.wrapping_add(1);
        if log::log_enabled!(log::Level::Trace) {
            let raw = buf.raw_buf();
            let n = frame_len.min(20);
            log::trace!(
                "cvitek-eth: RX#{} idx={} len={} hdr={:02x?}",
                self.rx_count, idx, frame_len, &raw[..n]
            );
        }

        buf.set_packet_len(frame_len);
        let buf_ptr = buf.into_buf_ptr();

        // 给 RX desc 补一个新缓冲并交给 DMA。
        if let Some(new_buf) = self.rx_pool.alloc_boxed() {
            let pa = new_buf.raw_buf().as_ptr() as u32;
            let d = &mut self.rx_descs[idx];
            unsafe {
                core::ptr::write_volatile(&mut d.des2, pa);
                let mut rdes1 = (BUF_SIZE as u32).min(RDES1_RBS1_MASK);
                if idx == RX_RING_SIZE - 1 {
                    rdes1 |= RDES1_RER;
                }
                core::ptr::write_volatile(&mut d.des1, rdes1);
                core::ptr::write_volatile(&mut d.des3, 0);
                fence(Ordering::Release);
                core::ptr::write_volatile(&mut d.des0, RDES0_OWN);
            }
            Self::flush_desc(d);
            self.rx_bufs[idx] = Some(new_buf);
        } else {
            log::warn!("cvitek-eth: RX buf alloc failed");
        }

        self.rx_cur = (self.rx_cur + 1) % RX_RING_SIZE;
        self.dma()
            .status
            .write(DmaStatus::RI::SET + DmaStatus::NIS::SET);
        self.dma().rx_poll.set(1);

        Ok(buf_ptr)
    }

    fn alloc_tx_buffer(&mut self, size: usize) -> DevResult<NetBufPtr> {
        let mut buf = self.tx_pool.alloc_boxed().ok_or(DevError::NoMemory)?;
        buf.set_packet_len(size);
        Ok(buf.into_buf_ptr())
    }
}

