//! Synopsys DWC2：探测、主机模式 bring-up（M1）、后续通道传输（M2+）。
//!
//! 寄存器名与位定义对齐 Linux `drivers/usb/dwc2/hw.h`（DesignWare OTG 2.0），通过
//! [`super::regs`] 中的 `tock-registers` 结构访问。
//!
//! 启用 feature **`cv182x-host`** 时，主机初始化对齐 Linux
//! `dwc2_set_cv182x_params` + `dwc2_core_host_init` / `dwc2_config_fifos`（UTMI 16-bit、HS、动态 FIFO、
//! `GDFIFOCFG`、`PCGCTL`、`TOUTCAL`），见
//! [Sipeed LicheeRV-Nano `params.c`](https://github.com/sipeed/LicheeRV-Nano-Build/blob/d4003f15b35d43ad4842f427050ab2bba0114fa5/linux_5.10/drivers/usb/dwc2/params.c#L217)。

#[allow(unused_imports)]
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::platform;
#[allow(unused_imports)]
use super::mmio;
#[allow(unused_imports)]
use super::regs::{
    Dwc2Regs, GAHBCFG, GDFIFOCFG, GHWCFG2, GHWCFG3, GHWCFG4, GINTMSK, GINTSTS, GOTGCTL, GRSTCTL,
    GUSBCFG, HCFG,
};

#[inline]
fn base() -> usize {
    platform::dwc2_base_virt()
}

/// 返回 DWC2 寄存器视图；前置已检查 base != 0 时使用。未设置时 panic（`pub` 入口
/// 在外层先做 [`base`] 校验）。
#[inline]
fn regs() -> &'static Dwc2Regs {
    mmio::dwc2_regs().expect("DWC2 base not set (call platform::set_dwc2_base_virt)")
}

/// `dwc2_host_init` 内超时（`wait_ahb_idle` / 软复位 / FIFO flush）时转储；与 EP0 的 `USB-TOUT ch_*` 区分。
fn dbg_dwc2_init_timeout(phase: &'static str) {
    if base() == 0 {
        return;
    }
    let r = regs();
    let grst = r.grstctl.get();
    let gint = r.gintsts.get();
    let gahb = r.gahbcfg.get();
    let hprt = r.hprt0.get();
    let ahb_idle = r.grstctl.is_set(GRSTCTL::AHBIDLE);
    let csftrst = r.grstctl.is_set(GRSTCTL::CSFTRST);
    let rst_done = r.grstctl.is_set(GRSTCTL::CSFTRST_DONE);
    let rx_flush = r.grstctl.is_set(GRSTCTL::RXFFLSH);
    let tx_flush = r.grstctl.is_set(GRSTCTL::TXFFLSH);
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT dwc2-init [{}] GRSTCTL={:#010x} AHBIDLE={} CSFTRST={} CSFTRST_DONE={} RXFFLSH={} TXFFLSH={}",
        phase, grst, ahb_idle, csftrst, rst_done, rx_flush, tx_flush
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT dwc2-init [{}] GINTSTS={:#010x} GAHBCFG={:#010x} HPRT0={:#010x}",
        phase, gint, gahb, hprt
    ));
}

// Linux `core.h`：`snpsid >= 0x4f54291a` 时配置 `GDFIFOCFG`（`hcd.c`）。
const DWC2_CORE_REV_2_91A: u32 = 0x4f54_291a;
/// 软复位序列分界：见 Linux `dwc2_core_reset()`（≥ 此版本用 `CSFTRST_DONE`，不再傻等 `CSFTRST` 自清）。
const DWC2_CORE_REV_4_20A: u32 = 0x4f54_420a;
const DWC2_CORE_REV_MASK: u32 = 0xffff;

#[inline]
fn spin_delay(iterations: u32) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

/// 读取硬件配置寄存器（上电后通常非零，用于 M0/M1「控制器是否可见」自检）。
///
/// # 返回值
/// `Ok((GHWCFG1, GHWCFG2, GHWCFG3))` 原始寄存器值；基址未设或读全零则返回 [`UsbError::Hardware`]。
pub unsafe fn dwc2_probe() -> UsbResult<(u32, u32, u32)> {
    if base() == 0 {
        return Err(UsbError::Hardware("DWC2 base not set (call platform::set_dwc2_base_virt)"));
    }
    let r = regs();
    let h1 = r.ghwcfg1.get();
    let h2 = r.ghwcfg2.get();
    let h3 = r.ghwcfg3.get();
    if h2 == 0 && h3 == 0 {
        return Err(UsbError::Hardware("DWC2 GHWCFG2/3 zero (no controller?)"));
    }
    Ok((h1, h2, h3))
}

fn wait_ahb_idle() -> UsbResult<()> {
    let r = regs();
    for _ in 0..3_000_000u32 {
        if r.grstctl.is_set(GRSTCTL::AHBIDLE) {
            return Ok(());
        }
        spin_delay(32);
    }
    dbg_dwc2_init_timeout("wait_ahb_idle");
    Err(UsbError::Timeout)
}

fn core_soft_reset() -> UsbResult<()> {
    wait_ahb_idle()?;
    let r = regs();
    let snpsid = r.gsnpsid.get();
    let core_rev = snpsid & DWC2_CORE_REV_MASK;
    let new_rst_seq = core_rev >= (DWC2_CORE_REV_4_20A & DWC2_CORE_REV_MASK);

    r.grstctl.modify(GRSTCTL::CSFTRST::SET);

    if !new_rst_seq {
        for _ in 0..3_000_000u32 {
            if !r.grstctl.is_set(GRSTCTL::CSFTRST) {
                spin_delay(4096);
                return Ok(());
            }
            spin_delay(32);
        }
        dbg_dwc2_init_timeout("core_soft_reset CSFTRST (legacy)");
        return Err(UsbError::Timeout);
    }

    // Linux `dwc2_core_reset`：Core ≥ 4.20a 时等 `CSFTRST_DONE`，再清 `CSFTRST` 并置位 `CSFTRST_DONE`。
    for _ in 0..3_000_000u32 {
        if r.grstctl.is_set(GRSTCTL::CSFTRST_DONE) {
            r.grstctl
                .modify(GRSTCTL::CSFTRST::CLEAR + GRSTCTL::CSFTRST_DONE::SET);
            spin_delay(4096);
            return Ok(());
        }
        spin_delay(32);
    }
    dbg_dwc2_init_timeout("core_soft_reset CSFTRST_DONE");
    Err(UsbError::Timeout)
}

fn force_host_mode() -> UsbResult<()> {
    let r = regs();
    r.gusbcfg.modify(GUSBCFG::FORCEHOSTMODE::SET);
    spin_delay(100_000);
    for _ in 0..500_000u32 {
        if r.gintsts.is_set(GINTSTS::CURMODE_HOST) {
            return Ok(());
        }
        spin_delay(32);
    }
    Err(UsbError::Hardware("CURMODE_HOST not set after FORCEHOSTMODE"))
}

/// 配置 RX / NPTX / PTX FIFO（与 Linux `dwc2` 常见默认值同量级；QEMU `raspi3b` 可接受）。
#[cfg(not(feature = "cv182x-host"))]
fn init_fifos() {
    const RX_DEPTH: u32 = 0x210;
    const NPTX_DEPTH: u32 = 0x200;
    const PTX_DEPTH: u32 = 0x200;
    let nptx_start = RX_DEPTH;
    let ptx_start = nptx_start + NPTX_DEPTH;

    let r = regs();
    r.grxfsiz.set(RX_DEPTH);
    r.gnptxfsiz.set((NPTX_DEPTH << 16) | nptx_start);
    r.hptxfsiz.set((PTX_DEPTH << 16) | ptx_start);
}

/// 依据 `GHWCFG2.ARCH` 决定是否置位 `DMA_EN`（内部 DMA 架构时必须开启，EP0 方能用 `HCDMA`）。
#[cfg(not(feature = "cv182x-host"))]
fn init_gahb() {
    let r = regs();
    let arch = r.ghwcfg2.read(GHWCFG2::ARCH);
    r.gahbcfg.modify(
        GAHBCFG::DMA_EN::CLEAR
            + GAHBCFG::GLBL_INTR_EN::SET
            + GAHBCFG::HBSTLEN.val(3),
    );
    if arch == 2 {
        r.gahbcfg.modify(GAHBCFG::DMA_EN::SET);
    }
}

#[cfg(not(feature = "cv182x-host"))]
fn init_hcfg_fs_ls() {
    regs().hcfg.modify(
        HCFG::FSLSSUPP::SET + HCFG::FSLSPCLKSEL::Pll48Mhz,
    );
}

/// 读取根端口寄存器 `HPRT0` 原始值（调试与端口状态轮询）。
///
/// # 返回值
/// 未设置 MMIO 基址时返回 **0** 且不访问硬件；否则为 `HPRT0` 当前读回值。
pub unsafe fn dwc2_hprt0_read() -> u32 {
    if base() == 0 {
        return 0;
    }
    regs().hprt0.get()
}

/// `HPRT0` **CONNSTS**（bit0）：根口是否检测到设备连接。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
pub fn hprt_connsts(hprt: u32) -> bool {
    hprt & (1 << 0) != 0
}

/// `HPRT0` **PWR**（bit12）：根口电源是否开启。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
pub fn hprt_pwr(hprt: u32) -> bool {
    hprt & (1 << 12) != 0
}

/// `HPRT0` **ENA**（bit2）：端口是否已使能。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
#[allow(dead_code)]
pub fn hprt_enabled(hprt: u32) -> bool {
    hprt & (1 << 2) != 0
}

/// 解析 `HPRT0[18:17]` 端口速度：**0**=HS、**1**=FS、**2**=LS。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
pub fn hprt_speed_bits(hprt: u32) -> u32 {
    (hprt >> 17) & 3
}

/// 根据根口当前速度给出典型 Bulk `wMaxPacketSize`（HS→512，否则→64）。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
pub fn suggested_bulk_mps(hprt: u32) -> u32 {
    if hprt_speed_bits(hprt) == 0 {
        512
    } else {
        64
    }
}

/// HPRT0 的 W1C（write-1-to-clear）位掩码，read-modify-write 时**必须** mask 掉，
/// 否则会误把 ENA(2)/ENACHG(3)/CONNDET(1)/OVRCURCHG(5) 当成新写入：
/// - bit 1 CONNDET (R/W1C)
/// - bit 2 ENA      (R/W1C — 写 1 = disable port！这是最容易踩的坑)
/// - bit 3 ENACHG   (R/W1C)
/// - bit 5 OVRCURCHG(R/W1C)
///
/// 与 Linux `drivers/usb/dwc2/hcd.c::dwc2_clear_hprt_intr_bits()` 等价。
const HPRT0_W1C_MASK: u32 = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 5);

#[inline]
fn hprt0_rmw_safe() -> u32 {
    regs().hprt0.get() & !HPRT0_W1C_MASK
}

fn port_power_on() {
    let r = regs();
    let mut w = hprt0_rmw_safe();
    w |= 1 << 12; // PWR
    r.hprt0.set(w);
}

fn port_reset_pulse() {
    let r = regs();
    // USB 2.0 spec TDRSTR (root hub reset) min = 50ms（实测 cv182x 的 PHY chirp K/J
    // 必须在 PRTRST 期间完成，不够长 chirp 不会发生，HPRT0.SPD 只能停在 FS）。
    // 这里给到 ≥60ms 留余量，并保留 PWR；同时**单独**清掉 CONNDET（W1C）。
    let cur = r.hprt0.get();
    if cur & (1 << 1) != 0 {
        // 写 1 清 CONNDET，但避免触碰其它 W1C 位
        r.hprt0.set((cur & !HPRT0_W1C_MASK) | (1 << 1));
    }
    let base = hprt0_rmw_safe() | (1 << 12); // 保留 PWR
    r.hprt0.set(base | (1 << 8)); // 拉 PRTRST
    spin_delay(15_000_000); // ~PRTRST 60ms+
    let base2 = hprt0_rmw_safe() | (1 << 12);
    r.hprt0.set(base2 & !(1 << 8)); // 解 PRTRST
    // TRSTRCY：reset 解除到首次 SETUP 之间 ≥10ms，慢 U 盘需 50–100ms 让 PHY 完成
    // chirp K-J-K-J + 内部 controller 启动。这里给 ~80ms 保守余量。
    spin_delay(20_000_000);
}

/// 读取 `HPRT0[11:10]` **LNSTS**（线路状态）。
///
/// # 参数
/// - `hprt`：[`dwc2_hprt0_read`] 的返回值。
#[inline]
pub fn hprt_lnsts(hprt: u32) -> u32 {
    (hprt >> 10) & 3
}

/// 在已检测到设备连接后发出 **USB 总线复位**（应在 `CONNSTS==1` 之后调用，符合主机枚举顺序）。
///
/// 会先对 `CONNDET` 做写 1 清除（若置位），再拉 `PRTRST`。
pub fn dwc2_host_root_bus_reset_pulse() -> UsbResult<()> {
    if base() == 0 {
        return Err(UsbError::Hardware("DWC2 base not set (call platform::set_dwc2_base_virt)"));
    }
    let r = regs();
    let cur = r.hprt0.get();
    if cur & (1 << 1) != 0 {
        // 写 1 清 CONNDET，避免触碰 ENA/ENACHG/OVRCURCHG 等其它 W1C 位
        r.hprt0.set((cur & !HPRT0_W1C_MASK) | (1 << 1));
    }
    port_reset_pulse();
    Ok(())
}

/// 打印根口与片内 PHY 快照（`CONNSTS==0` 时排障用）。
///
/// # 参数
/// - `tag`：日志前缀，便于区分多次 dump。
pub fn debug_dump_root_port_hw(tag: &str) {
    if base() == 0 {
        return;
    }
    let r = regs();
    let hprt = r.hprt0.get();
    let ln = hprt_lnsts(hprt);
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG {} HPRT0={:#010x} LNSTS={} CONNSTS={} CONNDET={} RST={} PWR={} SPD={}",
        tag,
        hprt,
        ln,
        hprt & 1,
        (hprt >> 1) & 1,
        (hprt >> 8) & 1,
        (hprt >> 12) & 1,
        hprt_speed_bits(hprt),
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG {} GOTGCTL={:#010x} GUSBCFG={:#010x} GAHBCFG={:#010x}",
        tag,
        r.gotgctl.get(),
        r.gusbcfg.get(),
        r.gahbcfg.get()
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG {} GINTSTS={:#010x} PCGCTL={:#010x} HCFG={:#010x}",
        tag,
        r.gintsts.get(),
        r.pcgctl.get(),
        r.hcfg.get()
    ));
    #[cfg(feature = "cv182x-host")]
    {
        let phy = CV182X_USB2_PHY_MMIO;
        unsafe {
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-PHY {} 00={:#010x} 04={:#010x} 08={:#010x} 0c={:#010x}",
                tag,
                mmio::read32(phy + 0x00),
                mmio::read32(phy + 0x04),
                mmio::read32(phy + 0x08),
                mmio::read32(phy + 0x0c),
            ));
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-PHY {} 10={:#010x} 14={:#010x} 18={:#010x} 1c={:#010x}",
                tag,
                mmio::read32(phy + 0x10),
                mmio::read32(phy + 0x14),
                mmio::read32(phy + 0x18),
                mmio::read32(phy + 0x1c),
            ));
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-PHY {} 20={:#010x} 24={:#010x} 28={:#010x} 2c={:#010x}",
                tag,
                mmio::read32(phy + 0x20),
                mmio::read32(phy + 0x24),
                mmio::read32(phy + 0x28),
                mmio::read32(phy + 0x2c),
            ));
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-PHY {} 30={:#010x} 3c={:#010x} 40={:#010x} 48={:#010x} 4c={:#010x} 50={:#010x}",
                tag,
                mmio::read32(phy + 0x30),
                mmio::read32(phy + 0x3c),
                mmio::read32(phy + 0x40),
                mmio::read32(phy + 0x48),
                mmio::read32(phy + 0x4c),
                mmio::read32(phy + 0x50),
            ));
        }
    }
}

// --- CV182x / SG2002 主机（Linux `dwc2_set_cv182x_params` + `dwc2_core_host_init`）---

#[cfg(feature = "cv182x-host")]
fn wait_grstctl_handshake(field: tock_registers::fields::Field<u32, GRSTCTL::Register>, set: bool) -> UsbResult<()> {
    let r = regs();
    for _ in 0..3_000_000u32 {
        let on = r.grstctl.is_set(field);
        if on == set {
            spin_delay(64);
            return Ok(());
        }
        spin_delay(8);
    }
    let label = "wait_grstctl handshake";
    dbg_dwc2_init_timeout(label);
    Err(UsbError::Timeout)
}

#[cfg(feature = "cv182x-host")]
fn flush_rx_fifo_host() -> UsbResult<()> {
    wait_ahb_idle()?;
    regs().grstctl.write(GRSTCTL::RXFFLSH::SET);
    wait_grstctl_handshake(GRSTCTL::RXFFLSH, false)?;
    spin_delay(2_000);
    Ok(())
}

#[cfg(feature = "cv182x-host")]
fn flush_tx_fifo_host_all() -> UsbResult<()> {
    wait_ahb_idle()?;
    regs()
        .grstctl
        .write(GRSTCTL::TXFFLSH::SET + GRSTCTL::TXFNUM.val(0x10));
    wait_grstctl_handshake(GRSTCTL::TXFFLSH, false)?;
    spin_delay(2_000);
    Ok(())
}

#[cfg(feature = "cv182x-host")]
#[inline]
fn total_dfifo_depth_words() -> u32 {
    regs().ghwcfg3.read(GHWCFG3::DFIFO_DEPTH)
}

#[cfg(feature = "cv182x-host")]
#[inline]
fn host_channel_count() -> u32 {
    1 + regs().ghwcfg2.read(GHWCFG2::NUM_HOST_CHAN)
}

/// 动态 FIFO：优先采用设备树常用值；超出 `GHWCFG3` 总深度时按 Linux `dwc2_calculate_dynamic_fifo` 收缩。
#[cfg(feature = "cv182x-host")]
fn init_host_fifos_cv182x() -> UsbResult<()> {
    let total = total_dfifo_depth_words();
    let hc = host_channel_count();
    let mut rx: u32 = 536;
    let mut nptx: u32 = 32;
    let mut ptx: u32 = 768;

    if rx.saturating_add(nptx).saturating_add(ptx) > total {
        rx = 516 + hc;
        nptx = 256;
        ptx = 768;
    }
    let sum = rx.saturating_add(nptx).saturating_add(ptx);
    if sum > total {
        ptx = total.saturating_sub(rx).saturating_sub(nptx);
    }

    let r = regs();
    r.grxfsiz.set(rx & 0xffff);
    r.gnptxfsiz
        .set(((nptx << 16) & 0xffff_0000) | (rx & 0xffff));
    r.hptxfsiz
        .set(((ptx << 16) & 0xffff_0000) | ((rx + nptx) & 0xffff));

    let snpsid = r.gsnpsid.get();
    let ded = r.ghwcfg4.is_set(GHWCFG4::DED_FIFO_EN);
    if ded && snpsid >= DWC2_CORE_REV_2_91A {
        let epbase = rx.wrapping_add(nptx).wrapping_add(ptx);
        r.gdfifocfg.modify(GDFIFOCFG::EPINFOBASE.val(epbase));
    }

    Ok(())
}

/// `dr_mode=otg` 时常用：使能 override 并置位 A-session / VBUS valid，否则根口可能无电气活动。
#[cfg(feature = "cv182x-host")]
fn init_gotgctl_otg_host_session_overrides() {
    regs().gotgctl.modify(
        GOTGCTL::DBNCE_FLTR_BYPASS::SET
            + GOTGCTL::AVALOEN::SET
            + GOTGCTL::AVALOVAL::SET
            + GOTGCTL::VBVALOEN::SET
            + GOTGCTL::VBVALOVAL::SET,
    );
    spin_delay(200_000);
}

/// UTMI 数据宽度按 `GHWCFG4.UTMI_PHY_DATA_WIDTH` 自适配；HS 超时校准；
/// 保持 `FORCEHOSTMODE`（与 [`force_host_mode`] 一致）。
///
/// **PHYIF16 设错是 chirp 失败的关键根因之一**：cv182x 的 PHY 实测为 8-bit UTMI
/// （vendor U-Boot `usb_gusbcfg = 0x40081400` 中 bit3 = 0 = 8-bit；
/// vendor Linux 也未显式设 PHYIF16）。如果 IP 报告 8-bit-only 或 programmable，
/// **必须** 把 PHYIF16 清零，否则 DWC2 与 PHY 的 UTMI 总线宽度不匹配，
/// chirp K/J 信号无法被正确解码，HPRT0.SPD 永远停在 FS。
#[cfg(feature = "cv182x-host")]
fn init_gusbcfg_cv182x_utmi16_hs() {
    let r = regs();
    let utmi_w = r.ghwcfg4.read(GHWCFG4::UTMI_PHY_DATA_WIDTH);
    let want_16bit = utmi_w == 1; // 16-bit only 时才必须 PHYIF16=1
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG GHWCFG4.UTMI_PHY_DATA_WIDTH={utmi_w} (0=8 only, 1=16 only, 2=programmable) => PHYIF16={}",
        if want_16bit { 1 } else { 0 }
    ));
    let mut field = GUSBCFG::FORCEHOSTMODE::SET
        + GUSBCFG::ULPI_UTMI_SEL::CLEAR
        + GUSBCFG::TOUTCAL.val(0x7);
    if want_16bit {
        field += GUSBCFG::PHYIF16::SET;
    } else {
        field += GUSBCFG::PHYIF16::CLEAR;
    }
    r.gusbcfg.modify(field);
}

#[cfg(feature = "cv182x-host")]
fn init_gahb_dma_cv182x() {
    let r = regs();
    let arch = r.ghwcfg2.read(GHWCFG2::ARCH);
    r.gahbcfg.modify(
        GAHBCFG::HBSTLEN::Incr16 + GAHBCFG::GLBL_INTR_EN::SET,
    );
    if arch == 2 {
        r.gahbcfg.modify(GAHBCFG::DMA_EN::SET);
    }
    #[cfg(feature = "usb-force-no-dma")]
    {
        r.gahbcfg.modify(GAHBCFG::DMA_EN::CLEAR);
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DBG usb-force-no-dma: DMA_EN cleared (ARCH={arch}, expect fail on int-DMA IP)"
        ));
    }
}

/// CV182x 片内 USB2 PHY（与 Linux `usb@04340000` 第二段 `reg` 一致）。`REG014` 见 `dwc2/platform.c`。
#[cfg(feature = "cv182x-host")]
const CV182X_USB2_PHY_MMIO: usize = 0x0300_6000;

/// 与厂商 Linux `platform.c` host 路径对齐：**不设 `UTMI_OVERRIDE`**。
///
/// DWC2 在 host 模式下通过 UTMI 接口自行驱动 `dp_pulldown` / `dm_pulldown` 信号；
/// 若 `UTMI_OVERRIDE`=1，PHY 忽略 DWC2 的 UTMI 信号，可能干扰控制器的连接检测。
///
/// 写 `REG014=0` 将控制权还给 DWC2（vendor kernel host 路径不碰 `REG014`；
/// `utmi_chgdet_prepare`/`utmi_reset` 仅在 `CONFIG_USB_DWC2_PERIPHERAL` 充电检测里使用）。
#[cfg(feature = "cv182x-host")]
fn cv182x_usb2_phy_host_clear_utmi_override() {
    let phy = mmio::cv182x_phy_regs();
    let old = phy.reg014.get();
    phy.reg014.set(0);
    spin_delay(200_000);
    let now = phy.reg014.get();
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG REG014 {:#06x}->{:#06x} (UTMI_OVERRIDE cleared, DWC2 drives pulldowns)",
        old, now
    ));
}

/// Linux 在 `speed == HS` 时**不**置 `HCFG_FSLSSUPP`；RPi/全速演示才需要 FSLS。
#[cfg(feature = "cv182x-host")]
fn hcfg_clear_fs_ls_for_high_speed() {
    regs().hcfg.modify(
        HCFG::FSLSSUPP::CLEAR + HCFG::FSLSPCLKSEL.val(0),
    );
}

/// M1：软复位、强制 Host、FIFO、GAHB、HCFG、根口上电（及 CV182x PHY 下拉）。
///
/// **不在此处** 发 USB 总线复位：应在确认 [`hprt_connsts`] 后调用 [`dwc2_host_root_bus_reset_pulse`]。
///
/// 成功返回 Ok，不保证已有设备连接；请读 [`dwc2_hprt0_read`] 的 `CONNSTS`。
pub fn dwc2_host_init() -> UsbResult<()> {
    if base() == 0 {
        return Err(UsbError::Hardware("DWC2 base not set (call platform::set_dwc2_base_virt)"));
    }
    let r = regs();
    r.gintmsk.set(0);
    r.gintsts.set(0xFFFF_FFFF);

    core_soft_reset()?;
    force_host_mode()?;
    core_soft_reset()?;

    #[cfg(feature = "cv182x-host")]
    {
        init_gotgctl_otg_host_session_overrides();
        init_gusbcfg_cv182x_utmi16_hs();
        r.pcgctl.set(0);
        init_gahb_dma_cv182x();
        hcfg_clear_fs_ls_for_high_speed();
        init_host_fifos_cv182x()?;
        flush_tx_fifo_host_all()?;
        flush_rx_fifo_host()?;
    }
    #[cfg(not(feature = "cv182x-host"))]
    {
        init_fifos();
        init_gahb();
        init_hcfg_fs_ls();
    }

    r.haintmsk.set((1 << 0) | (1 << 1));
    r.gintmsk.modify(GINTMSK::HCHINT::SET);

    r.gintsts.set(0xFFFF_FFFF);

    port_power_on();

    #[cfg(feature = "cv182x-host")]
    {
        cv182x_usb2_phy_host_clear_utmi_override();
        init_gotgctl_otg_host_session_overrides();
    }

    Ok(())
}
