//! Synopsys DWC2：探测、主机模式 bring-up（M1）、后续通道传输（M2+）。
//!
//! 寄存器名与位定义对齐 Linux `drivers/usb/dwc2/hw.h`（DesignWare OTG 2.0），通过
//! [`super::regs`] 中的 `tock-registers` 结构访问。
//!
//! 启用 feature **`cv182x-host`** 时，主机初始化对齐 Linux
//! `dwc2_set_cv182x_params` + `dwc2_core_host_init` / `dwc2_config_fifos`（UTMI 16-bit、HS、动态 FIFO、
//! `GDFIFOCFG`、`PCGCTL`、`TOUTCAL`），见
//! [Sipeed LicheeRV-Nano `params.c`](https://github.com/sipeed/LicheeRV-Nano-Build/blob/d4003f15b35d43ad4842f427050ab2bba0114fa5/linux_5.10/drivers/usb/dwc2/params.c#L217)。

use tock_registers::LocalRegisterCopy;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use super::isr;
use super::regs::{
    Dwc2Regs, GAHBCFG, GDFIFOCFG, GHWCFG2, GHWCFG3, GHWCFG4, GINTSTS, GOTGCTL, GRSTCTL, GUSBCFG,
    HCFG, HPRT0,
};
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::{self, dwc2_base_virt};
use crate::utils::spin_delay;

use crate::usb::dwc2_regs;

/// `dwc2_host_init` 内超时（`wait_ahb_idle` / 软复位 / FIFO flush）时转储；与 EP0 的 `USB-TOUT ch_*` 区分。
fn dbg_dwc2_init_timeout(phase: &'static str) {
    if dwc2_base_virt() == 0 {
        return;
    }
    let regs = dwc2_regs();
    let grst = regs.grstctl.get();
    let gint = regs.gintsts.get();
    let gahb = regs.gahbcfg.get();
    let hprt = regs.hprt0.get();
    let ahb_idle = regs.grstctl.is_set(GRSTCTL::AHBIDLE);
    let csftrst = regs.grstctl.is_set(GRSTCTL::CSFTRST);
    let rst_done = regs.grstctl.is_set(GRSTCTL::CSFTRST_DONE);
    let rx_flush = regs.grstctl.is_set(GRSTCTL::RXFFLSH);
    let tx_flush = regs.grstctl.is_set(GRSTCTL::TXFFLSH);
    log::warn!(
        "USB-TOUT dwc2-init [{}] GRSTCTL={:#010x} AHBIDLE={} CSFTRST={} CSFTRST_DONE={} RXFFLSH={} TXFFLSH={}",
        phase,
        grst,
        ahb_idle,
        csftrst,
        rst_done,
        rx_flush,
        tx_flush
    );
    log::warn!(
        "USB-TOUT dwc2-init [{}] GINTSTS={:#010x} GAHBCFG={:#010x} HPRT0={:#010x}",
        phase,
        gint,
        gahb,
        hprt
    );
}

// Linux `core.h`：`snpsid >= 0x4f54291a` 时配置 `GDFIFOCFG`（`hcd.c`）。
const DWC2_CORE_REV_2_91A: u32 = 0x4f54_291a;
/// 软复位序列分界：见 Linux `dwc2_core_reset()`（≥ 此版本用 `CSFTRST_DONE`，不再傻等 `CSFTRST` 自清）。
const DWC2_CORE_REV_4_20A: u32 = 0x4f54_420a;
const DWC2_CORE_REV_MASK: u32 = 0xffff;

/// 读取硬件配置寄存器（上电后通常非零，用于 M0/M1「控制器是否可见」自检）。
///
/// # Safety
///
/// 调用方须已通过 [`set_dwc2_base_virt`] 设置有效 MMIO 基址；在未完成控制器初始化的上下文中并发访问同一寄存器块可能导致未定义行为。
///
/// # 返回值
/// `Ok((GHWCFG1, GHWCFG2, GHWCFG3))` 原始寄存器值；基址未设或读全零则返回 [`UsbError::Hardware`]。
pub unsafe fn dwc2_probe() -> UsbResult<(u32, u32, u32)> {
    if dwc2_base_virt() == 0 {
        return Err(UsbError::Hardware(
            "DWC2 base not set (call set_dwc2_base_virt)",
        ));
    }
    let regs = dwc2_regs();
    let h1 = regs.ghwcfg1.get();
    let h2 = regs.ghwcfg2.get();
    let h3 = regs.ghwcfg3.get();
    if h2 == 0 && h3 == 0 {
        return Err(UsbError::Hardware("DWC2 GHWCFG2/3 zero (no controller?)"));
    }
    Ok((h1, h2, h3))
}

fn wait_ahb_idle() -> UsbResult<()> {
    let regs = dwc2_regs();
    for _ in 0..3_000_000u32 {
        if regs.grstctl.is_set(GRSTCTL::AHBIDLE) {
            return Ok(());
        }
        spin_delay(32);
    }
    dbg_dwc2_init_timeout("wait_ahb_idle");
    Err(UsbError::Timeout)
}

fn core_soft_reset() -> UsbResult<()> {
    wait_ahb_idle()?;
    let regs = dwc2_regs();
    let snpsid = regs.gsnpsid.get();
    let core_rev = snpsid & DWC2_CORE_REV_MASK;
    let new_rst_seq = core_rev >= (DWC2_CORE_REV_4_20A & DWC2_CORE_REV_MASK);

    regs.grstctl.modify(GRSTCTL::CSFTRST::SET);

    if !new_rst_seq {
        for _ in 0..3_000_000u32 {
            if !regs.grstctl.is_set(GRSTCTL::CSFTRST) {
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
        if regs.grstctl.is_set(GRSTCTL::CSFTRST_DONE) {
            regs.grstctl
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
    let regs = dwc2_regs();
    regs.gusbcfg.modify(GUSBCFG::FORCEHOSTMODE::SET);
    spin_delay(100_000);
    for _ in 0..500_000u32 {
        if regs.gintsts.is_set(GINTSTS::CURMODE_HOST) {
            return Ok(());
        }
        spin_delay(32);
    }
    Err(UsbError::Hardware(
        "CURMODE_HOST not set after FORCEHOSTMODE",
    ))
}

/// 配置 RX / NPTX / PTX FIFO（与 Linux `dwc2` 常见默认值同量级；QEMU `raspi3b` 可接受）。
#[cfg(not(feature = "cv182x-host"))]
fn init_fifos() {
    const RX_DEPTH: u32 = 0x210;
    const NPTX_DEPTH: u32 = 0x200;
    const PTX_DEPTH: u32 = 0x200;
    let nptx_start = RX_DEPTH;
    let ptx_start = nptx_start + NPTX_DEPTH;

    let regs = dwc2_regs();
    regs.grxfsiz.set(RX_DEPTH);
    regs.gnptxfsiz.set((NPTX_DEPTH << 16) | nptx_start);
    regs.hptxfsiz.set((PTX_DEPTH << 16) | ptx_start);
}

/// 依据 `GHWCFG2.ARCH` 决定是否置位 `DMA_EN`（内部 DMA 架构时必须开启，EP0 方能用 `HCDMA`）。
#[cfg(not(feature = "cv182x-host"))]
fn init_gahb() {
    let regs = dwc2_regs();
    let arch = regs.ghwcfg2.read(GHWCFG2::ARCH);
    regs.gahbcfg
        .modify(GAHBCFG::DMA_EN::CLEAR + GAHBCFG::GLBL_INTR_EN::SET + GAHBCFG::HBSTLEN.val(3));
    if arch == 2 {
        regs.gahbcfg.modify(GAHBCFG::DMA_EN::SET);
    }
}

#[cfg(not(feature = "cv182x-host"))]
fn init_hcfg_fs_ls() {
    dwc2_regs()
        .hcfg
        .modify(HCFG::FSLSSUPP::SET + HCFG::FSLSPCLKSEL::Pll48Mhz);
}

/// Bulk/Isoch 主机通道号（与 [`super::channel`] 中 `CH_BULK` 一致）。
/// HPRT0 的 W1C 位掩码：RMW 写入时须清零，否则会把读回的 1 原样写回并触发 W1C 副作用
///（`ENA` 写 1 = disable port）。与 Linux `dwc2_clear_hprt_intr_bits()` 等价。
const HPRT0_W1C_MASK: u32 =
    HPRT0::CONNDET.mask | HPRT0::ENA.mask | HPRT0::ENACHG.mask | HPRT0::OVRCURCHG.mask;

/// 安全修改 `HPRT0`：先剥掉 W1C 位再合并新字段，避免 `modify()` 误触发 W1C。
#[inline]
fn hprt0_modify_safe(
    regs: &Dwc2Regs,
    fields: tock_registers::fields::FieldValue<u32, HPRT0::Register>,
) {
    let mut h = LocalRegisterCopy::<u32, HPRT0::Register>::new(regs.hprt0.get() & !HPRT0_W1C_MASK);
    h.modify(fields);
    regs.hprt0.set(h.get());
}

/// 写 1 清除 `HPRT0.CONNDET`（W1C）；仅在已置位时写入，避免多余 MMIO。
#[inline]
fn clear_hprt_condet_if_set(regs: &Dwc2Regs) {
    if regs.hprt0.is_set(HPRT0::CONNDET) {
        hprt0_modify_safe(regs, HPRT0::CONNDET::SET);
    }
}

fn port_power_on() {
    hprt0_modify_safe(dwc2_regs(), HPRT0::PWR::SET);
}

fn port_reset_pulse() {
    let regs = dwc2_regs();
    // USB 2.0 spec TDRSTR (root hub reset) min = 50ms（实测 cv182x 的 PHY chirp K/J
    // 必须在 PRTRST 期间完成，不够长 chirp 不会发生，HPRT0.SPD 只能停在 FS）。
    // 这里给到 ≥60ms 留余量，并保留 PWR；同时**单独**清掉 CONNDET（W1C）。
    clear_hprt_condet_if_set(regs);
    hprt0_modify_safe(regs, HPRT0::PWR::SET + HPRT0::RST::SET);
    spin_delay(15_000_000); // ~PRTRST 60ms+
    hprt0_modify_safe(regs, HPRT0::PWR::SET + HPRT0::RST::CLEAR);
    // TRSTRCY：reset 解除到首次 SETUP 之间 ≥10ms，慢 U 盘需 50–100ms 让 PHY 完成
    // chirp K-J-K-J + 内部 controller 启动。这里给 ~80ms 保守余量。
    spin_delay(20_000_000);
}

/// 在已检测到设备连接后发出 **USB 总线复位**（应在 `CONNSTS==1` 之后调用，符合主机枚举顺序）。
///
/// 会先对 `CONNDET` 做写 1 清除（若置位），再拉 `PRTRST`。
pub fn dwc2_host_root_bus_reset_pulse() -> UsbResult<()> {
    if dwc2_base_virt() == 0 {
        return Err(UsbError::Hardware(
            "DWC2 base not set (call set_dwc2_base_virt)",
        ));
    }
    clear_hprt_condet_if_set(dwc2_regs());
    port_reset_pulse();
    Ok(())
}

// --- CV182x / SG2002 主机（Linux `dwc2_set_cv182x_params` + `dwc2_core_host_init`）---

#[cfg(feature = "cv182x-host")]
fn wait_grstctl_handshake(
    field: tock_registers::fields::Field<u32, GRSTCTL::Register>,
    set: bool,
) -> UsbResult<()> {
    let regs = dwc2_regs();
    for _ in 0..3_000_000u32 {
        let on = regs.grstctl.is_set(field);
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
    dwc2_regs().grstctl.write(GRSTCTL::RXFFLSH::SET);
    wait_grstctl_handshake(GRSTCTL::RXFFLSH, false)?;
    spin_delay(2_000);
    Ok(())
}

#[cfg(feature = "cv182x-host")]
fn flush_tx_fifo_host_all() -> UsbResult<()> {
    wait_ahb_idle()?;
    dwc2_regs()
        .grstctl
        .write(GRSTCTL::TXFFLSH::SET + GRSTCTL::TXFNUM.val(0x10));
    wait_grstctl_handshake(GRSTCTL::TXFFLSH, false)?;
    spin_delay(2_000);
    Ok(())
}

#[cfg(feature = "cv182x-host")]
#[inline]
fn total_dfifo_depth_words() -> u32 {
    dwc2_regs().ghwcfg3.read(GHWCFG3::DFIFO_DEPTH)
}

#[cfg(feature = "cv182x-host")]
#[inline]
fn host_channel_count() -> u32 {
    1 + dwc2_regs().ghwcfg2.read(GHWCFG2::NUM_HOST_CHAN)
}

/// 动态 FIFO：RX 扩大到 1024 words (4096B) 以支持 mult=3 isoch 高带宽传输（3060B/微帧）。
/// PTX 相应缩小——当前无周期性 OUT 流量，不影响功能。
#[cfg(feature = "cv182x-host")]
fn init_host_fifos_cv182x() -> UsbResult<()> {
    let total = total_dfifo_depth_words();
    let hc = host_channel_count();
    let mut rx: u32 = 1024;
    let mut nptx: u32 = 32;
    let mut ptx: u32 = 256;

    if rx.saturating_add(nptx).saturating_add(ptx) > total {
        rx = 516 + hc;
        nptx = 256;
        ptx = 256;
    }
    let sum = rx.saturating_add(nptx).saturating_add(ptx);
    if sum > total {
        ptx = total.saturating_sub(rx).saturating_sub(nptx);
    }

    let regs = dwc2_regs();
    regs.grxfsiz.set(rx & 0xffff);
    regs.gnptxfsiz
        .set(((nptx << 16) & 0xffff_0000) | (rx & 0xffff));
    regs.hptxfsiz
        .set(((ptx << 16) & 0xffff_0000) | ((rx + nptx) & 0xffff));

    log::info!(
        "USB: FIFO config: total_depth={} words, RX={} words ({}B), NPTX={} words, PTX={} words ({}B)",
        total,
        rx,
        rx * 4,
        nptx,
        ptx,
        ptx * 4
    );

    let snpsid = regs.gsnpsid.get();
    let ded = regs.ghwcfg4.is_set(GHWCFG4::DED_FIFO_EN);
    if ded && snpsid >= DWC2_CORE_REV_2_91A {
        let epbase = rx.wrapping_add(nptx).wrapping_add(ptx);
        regs.gdfifocfg.modify(GDFIFOCFG::EPINFOBASE.val(epbase));
    }

    Ok(())
}

/// `dr_mode=otg` 时常用：使能 override 并置位 A-session / VBUS valid，否则根口可能无电气活动。
#[cfg(feature = "cv182x-host")]
fn init_gotgctl_otg_host_session_overrides() {
    dwc2_regs().gotgctl.modify(
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
    let regs = dwc2_regs();
    let utmi_w = regs.ghwcfg4.read(GHWCFG4::UTMI_PHY_DATA_WIDTH);
    let want_16bit = utmi_w == 1; // 16-bit only 时才必须 PHYIF16=1
    log::debug!(
        "USB-DBG GHWCFG4.UTMI_PHY_DATA_WIDTH={utmi_w} (0=8 only, 1=16 only, 2=programmable) => PHYIF16={}",
        if want_16bit { 1 } else { 0 }
    );
    let mut field =
        GUSBCFG::FORCEHOSTMODE::SET + GUSBCFG::ULPI_UTMI_SEL::CLEAR + GUSBCFG::TOUTCAL.val(0x7);
    if want_16bit {
        field += GUSBCFG::PHYIF16::SET;
    } else {
        field += GUSBCFG::PHYIF16::CLEAR;
    }
    regs.gusbcfg.modify(field);
}

#[cfg(feature = "cv182x-host")]
fn init_gahb_dma_cv182x() {
    let regs = dwc2_regs();
    let arch = regs.ghwcfg2.read(GHWCFG2::ARCH);
    regs.gahbcfg
        .modify(GAHBCFG::HBSTLEN::Incr16 + GAHBCFG::GLBL_INTR_EN::SET);
    if arch == 2 {
        regs.gahbcfg.modify(GAHBCFG::DMA_EN::SET);
    }
    #[cfg(feature = "usb-force-no-dma")]
    {
        regs.gahbcfg.modify(GAHBCFG::DMA_EN::CLEAR);
        log::debug!(
            "USB-DBG usb-force-no-dma: DMA_EN cleared (ARCH={arch}, expect fail on int-DMA IP)"
        );
    }
}

/// 与厂商 Linux `platform.c` host 路径对齐：**不设 `UTMI_OVERRIDE`**。
///
/// DWC2 在 host 模式下通过 UTMI 接口自行驱动 `dp_pulldown` / `dm_pulldown` 信号；
/// 若 `UTMI_OVERRIDE`=1，PHY 忽略 DWC2 的 UTMI 信号，可能干扰控制器的连接检测。
///
/// 写 `REG014=0` 将控制权还给 DWC2（vendor kernel host 路径不碰 `REG014`；
/// `utmi_chgdet_prepare`/`utmi_reset` 仅在 `CONFIG_USB_DWC2_PERIPHERAL` 充电检测里使用）。
#[cfg(feature = "cv182x-host")]
fn cv182x_usb2_phy_host_clear_utmi_override() {
    let phy = usb::cv182x_phy_regs()
        .expect("CV182x USB2 PHY base not set (call set_cv182x_phy_base_virt)");
    let old = phy.reg014.get();
    phy.reg014.set(0);
    spin_delay(200_000);
    let now = phy.reg014.get();
    log::debug!(
        "USB-DBG REG014 {:#06x}->{:#06x} (UTMI_OVERRIDE cleared, DWC2 drives pulldowns)",
        old,
        now
    );
}

/// Linux 在 `speed == HS` 时**不**置 `HCFG_FSLSSUPP`；RPi/全速演示才需要 FSLS。
#[cfg(feature = "cv182x-host")]
fn hcfg_clear_fs_ls_for_high_speed() {
    dwc2_regs()
        .hcfg
        .modify(HCFG::FSLSSUPP::CLEAR + HCFG::FSLSPCLKSEL.val(0));
}

/// M1：软复位、强制 Host、FIFO、GAHB、HCFG、根口上电（及 CV182x PHY 下拉）。
///
/// **不在此处** 发 USB 总线复位：应在确认 `HPRT0.CONNSTS` 后调用 [`dwc2_host_root_bus_reset_pulse`]。
///
/// 成功返回 Ok，不保证已有设备连接；请读 `HPRT0.CONNSTS`。
pub fn dwc2_host_init() -> UsbResult<()> {
    if dwc2_base_virt() == 0 {
        return Err(UsbError::Hardware(
            "DWC2 base not set (call set_dwc2_base_virt)",
        ));
    }
    isr::dwc2_host_irq_mask_and_clear();

    core_soft_reset()?;
    force_host_mode()?;
    core_soft_reset()?;

    #[cfg(feature = "cv182x-host")]
    {
        init_gotgctl_otg_host_session_overrides();
        init_gusbcfg_cv182x_utmi16_hs();
        dwc2_regs().pcgctl.set(0);
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

    isr::dwc2_host_irq_enable();

    port_power_on();

    #[cfg(feature = "cv182x-host")]
    {
        cv182x_usb2_phy_host_clear_utmi_override();
        init_gotgctl_otg_host_session_overrides();
    }

    Ok(())
}
