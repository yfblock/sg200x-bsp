//! DWC2 Device 模式 bring-up：core soft reset → ForceDevMode → DCFG.DEVSPD →
//! FIFO 划分（GRXFSIZ + GNPTXFSIZ + DIEPTXF1）→ DCTL.SFTDISCON 控制上线。
//!
//! 与 [`crate::usb::host::dwc2::controller`] 共用同一个 [`crate::usb::host::dwc2::regs::Dwc2Regs`] 视图与
//! [`crate::usb::platform`] 基址；本文件仅触碰 device 段（0x800+）+ 通用段
//! （GUSBCFG / GAHBCFG / GRSTCTL / GINTSTS / GHWCFG3）。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::mmio;
use crate::usb::host::dwc2::regs::{
    Dwc2Regs, DCFG, DCTL, DEPMSK, DIEPCTL, DOEPCTL, GAHBCFG, GHWCFG2, GHWCFG3, GHWCFG4, GINTSTS,
    GRSTCTL, GUSBCFG,
};
use crate::usb::platform;

#[inline]
fn base() -> usize {
    platform::dwc2_base_virt()
}

#[inline]
fn regs() -> &'static Dwc2Regs {
    mmio::dwc2_regs().expect("DWC2 base not set (call platform::set_dwc2_base_virt)")
}

#[inline]
fn spin_delay(n: u32) {
    for _ in 0..n {
        core::hint::spin_loop();
    }
}

/// 设备速度提示（DCFG.DEVSPD）。SG2002 的 UTMI PHY 走 [`HighSpeed`]；
/// 调试时可改 [`FullSpeed`] 让控制器只发 FS chirp，避免 HS 反复重试。
#[derive(Clone, Copy, Debug)]
pub enum DeviceSpeedHint {
    /// USB 2.0 High Speed，需要 PHY/clock 完全就绪。
    HighSpeed,
    /// USB 2.0 Full Speed（HS PHY 强制 FS）；调试用。
    FullSpeed,
}

static mut SPEED_HINT: DeviceSpeedHint = DeviceSpeedHint::HighSpeed;

/// 设置希望协商的设备速度（可在 [`dwc2_device_init`] 之前调用）。
pub fn dwc2_device_set_speed_hint(hint: DeviceSpeedHint) {
    unsafe {
        SPEED_HINT = hint;
    }
}

#[inline]
fn current_speed_hint() -> DeviceSpeedHint {
    unsafe { SPEED_HINT }
}

// 软复位序列分界线（与 host controller.rs 同一份常量）。
const DWC2_CORE_REV_4_20A: u32 = 0x4f54_420a;
const DWC2_CORE_REV_MASK: u32 = 0xffff;

fn wait_ahb_idle() -> UsbResult<()> {
    let r = regs();
    for _ in 0..3_000_000u32 {
        if r.grstctl.is_set(GRSTCTL::AHBIDLE) {
            return Ok(());
        }
        spin_delay(32);
    }
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
        return Err(UsbError::Timeout);
    }
    for _ in 0..3_000_000u32 {
        if r.grstctl.is_set(GRSTCTL::CSFTRST_DONE) {
            r.grstctl
                .modify(GRSTCTL::CSFTRST::CLEAR + GRSTCTL::CSFTRST_DONE::SET);
            spin_delay(4096);
            return Ok(());
        }
        spin_delay(32);
    }
    Err(UsbError::Timeout)
}

fn force_device_mode() -> UsbResult<()> {
    let r = regs();
    // 同时清掉 ForceHost，以免上一次 host 路径残留。
    r.gusbcfg
        .modify(GUSBCFG::FORCEHOSTMODE::CLEAR + GUSBCFG::FORCEDEVMODE::SET);
    spin_delay(100_000);
    for _ in 0..500_000u32 {
        // CURMODE_HOST=0 即 device 模式
        if !r.gintsts.is_set(GINTSTS::CURMODE_HOST) {
            return Ok(());
        }
        spin_delay(32);
    }
    Err(UsbError::Hardware("CURMODE stays in Host after FORCEDEVMODE"))
}

/// CV182x：UTMI 数据宽度按 `GHWCFG4.UTMI_PHY_DATA_WIDTH` 自适配（与 host 路径同一逻辑）。
fn init_gusbcfg_cv182x() {
    let r = regs();
    let utmi_w = r.ghwcfg4.read(GHWCFG4::UTMI_PHY_DATA_WIDTH);
    let want_16bit = utmi_w == 1;
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV GHWCFG4.UTMI_PHY_DATA_WIDTH={utmi_w} => PHYIF16={}",
        if want_16bit { 1 } else { 0 }
    ));
    let mut field = GUSBCFG::FORCEDEVMODE::SET
        + GUSBCFG::FORCEHOSTMODE::CLEAR
        + GUSBCFG::ULPI_UTMI_SEL::CLEAR
        + GUSBCFG::TOUTCAL.val(0x7);
    if want_16bit {
        field += GUSBCFG::PHYIF16::SET;
    } else {
        field += GUSBCFG::PHYIF16::CLEAR;
    }
    r.gusbcfg.modify(field);
}

fn init_gahb_dma() {
    let r = regs();
    let arch = r.ghwcfg2.read(GHWCFG2::ARCH);
    r.gahbcfg
        .modify(GAHBCFG::HBSTLEN::Incr16 + GAHBCFG::GLBL_INTR_EN::SET);
    if arch == 2 {
        r.gahbcfg.modify(GAHBCFG::DMA_EN::SET);
    }
}

/// 关键：SG2002 上 DFIFO 总深度约 0x4e8 (1256 words)。
///
/// Device 模式 FIFO 划分（与 vendor U-Boot device 模式一致量级，留 EP0 + 2 个
/// bulk EP 用）：
/// - GRXFSIZ：RX 共享，按 `(MaxOutPktSizeInWords) * 2 + 10` 估，HS bulk 64 字节
///   ≈ 16 words → 给 256 words 留余量。
/// - GNPTXFSIZ：EP0 IN 用 non-periodic TX FIFO，HS EP0 MPS=64 = 16 words → 64 words。
/// - DIEPTXF1..：每个 IN EP 专用 TX FIFO，HS bulk 512 字节 = 128 words → 256 words。
fn init_device_fifos() {
    let r = regs();
    let total = r.ghwcfg3.read(GHWCFG3::DFIFO_DEPTH);

    // 默认大小（words）：覆盖 EP0 + 1 对 bulk + 1 个 interrupt notification IN。
    let rx: u32 = 256;
    let nptx: u32 = 64;
    let mut ep1tx: u32 = 256;
    let ep2tx: u32 = 32; // CDC-ACM notification EP3 IN（也可被其他类用作小 IN）

    // 总和不能超过 DFIFO_DEPTH，超了就按比例缩小 ep1tx（最大头）。
    let used = rx + nptx + ep1tx + ep2tx;
    if used > total {
        let leftover = total.saturating_sub(rx + nptx + ep2tx);
        ep1tx = leftover;
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DEV FIFO total={} too small, shrink ep1tx={}",
            total, ep1tx
        ));
    }

    r.grxfsiz.set(rx & 0xffff);
    r.gnptxfsiz.set(((nptx & 0xffff) << 16) | (rx & 0xffff));

    // DIEPTXFn (n>=1) 寄存器：0x104 + (n-1)*4。
    // n=1 → 0x104（DIEPTXF1，CDC bulk IN）
    // n=2 → 0x108（DIEPTXF2，CDC notification IN）
    let dieptxf1 = ((ep1tx & 0xffff) << 16) | ((rx + nptx) & 0xffff);
    let dieptxf2 = ((ep2tx & 0xffff) << 16) | ((rx + nptx + ep1tx) & 0xffff);
    unsafe {
        core::ptr::write_volatile((base() + 0x104) as *mut u32, dieptxf1);
        core::ptr::write_volatile((base() + 0x108) as *mut u32, dieptxf2);
    }

    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV FIFO total={} GRXFSIZ={:#x} GNPTXFSIZ={:#x} DIEPTXF1={:#x} DIEPTXF2={:#x}",
        total,
        r.grxfsiz.get(),
        r.gnptxfsiz.get(),
        dieptxf1,
        dieptxf2,
    ));

    let _ = used;
}

fn flush_all_fifos() -> UsbResult<()> {
    wait_ahb_idle()?;
    let r = regs();
    // TXFNUM = 0x10 → flush all TX FIFO
    r.grstctl
        .write(GRSTCTL::TXFFLSH::SET + GRSTCTL::TXFNUM.val(0x10));
    for _ in 0..3_000_000u32 {
        if !r.grstctl.is_set(GRSTCTL::TXFFLSH) {
            break;
        }
        spin_delay(8);
    }
    r.grstctl.write(GRSTCTL::RXFFLSH::SET);
    for _ in 0..3_000_000u32 {
        if !r.grstctl.is_set(GRSTCTL::RXFFLSH) {
            break;
        }
        spin_delay(8);
    }
    spin_delay(2_000);
    Ok(())
}

/// 配置 EP0 OUT/IN（仅设置 EPTYPE/MPS/USBACTEP；具体 prime 在 ep0.rs 完成）。
fn ep0_pre_config() {
    let r = regs();
    // EP0 默认 MPS：HS=64, FS=64（DCFG.DEVSPD=0/1 都是 64；LS 才 8）
    let mps_field = 0u32; // DIEPCTL/DOEPCTL EP0 MPS 编码：00=64, 01=32, 10=16, 11=8
    let in_ep0 = &r.diep[0];
    let out_ep0 = &r.doep[0];
    in_ep0.diepctl.modify(
        DIEPCTL::MPS.val(mps_field) + DIEPCTL::EPTYPE::Control + DIEPCTL::USBACTEP::SET,
    );
    out_ep0.doepctl.modify(
        DOEPCTL::MPS.val(mps_field) + DOEPCTL::EPTYPE::Control + DOEPCTL::USBACTEP::SET,
    );
    // 中断掩码：本驱动靠轮询，但启用底层中断标志方便 dump 时看到。
    r.diepmsk
        .write(DEPMSK::XFERCOMPL::SET + DEPMSK::TIMEOUT_OR_SETUP::SET + DEPMSK::EPDISBLD::SET);
    r.doepmsk
        .write(DEPMSK::XFERCOMPL::SET + DEPMSK::TIMEOUT_OR_SETUP::SET + DEPMSK::EPDISBLD::SET);
    // 允许 EP0 IN/OUT 在 DAINT 中聚合（虽然我们不开 GINTMSK.IEPINT/OEPINT，
    // DAINT 仍然是 RO 镜像）。
    r.daintmsk
        .write(crate::usb::host::dwc2::regs::DAINTMSK::IEPMSK.val(1)
             + crate::usb::host::dwc2::regs::DAINTMSK::OEPMSK.val(1));
}

/// 板级 bring-up：核心软复位 + ForceDevMode + DCFG + GAHB + FIFO + EP0 预配置。
///
/// **保持 `DCTL.SFTDISCON=1`**（D+ 上拉断开），等到调用方注册了 class、调
/// [`dwc2_device_softconnect`] 才上线。
pub fn dwc2_device_init() -> UsbResult<()> {
    if base() == 0 {
        return Err(UsbError::Hardware(
            "DWC2 base not set (call platform::set_dwc2_base_virt)",
        ));
    }
    let r = regs();
    // 屏蔽所有中断 + W1C 清旧 GINTSTS
    r.gintmsk.set(0);
    r.gintsts.set(0xFFFF_FFFF);

    core_soft_reset()?;

    #[cfg(feature = "cv182x-host")]
    init_gusbcfg_cv182x();
    #[cfg(not(feature = "cv182x-host"))]
    {
        let r = regs();
        r.gusbcfg
            .modify(GUSBCFG::FORCEHOSTMODE::CLEAR + GUSBCFG::FORCEDEVMODE::SET);
    }

    force_device_mode()?;
    core_soft_reset()?;
    // 第二次复位后 ForceDevMode 可能被清，重新 set。
    #[cfg(feature = "cv182x-host")]
    init_gusbcfg_cv182x();
    #[cfg(not(feature = "cv182x-host"))]
    regs()
        .gusbcfg
        .modify(GUSBCFG::FORCEHOSTMODE::CLEAR + GUSBCFG::FORCEDEVMODE::SET);
    force_device_mode()?;

    // PCGCTL 必须为 0（否则 PHY 会被门控，没有 USBRST 事件）
    regs().pcgctl.set(0);

    init_gahb_dma();

    // DCFG：设备速度 + 默认地址 0
    let speed = current_speed_hint();
    let dcfg_speed = match speed {
        DeviceSpeedHint::HighSpeed => DCFG::DEVSPD::HighSpeed,
        DeviceSpeedHint::FullSpeed => DCFG::DEVSPD::FullSpeedHs,
    };
    regs().dcfg.modify(
        dcfg_speed
            + DCFG::DEVADDR.val(0)
            + DCFG::NZSTSOUTHSHK::CLEAR
            + DCFG::DESCDMA::CLEAR
            + DCFG::PERFRINT::Frm80,
    );

    // DCTL：保持 SFTDISCON=1（默认就是 1，这里显式确认），数据上拉断开
    regs().dctl.modify(DCTL::SFTDISCON::SET);

    init_device_fifos();
    flush_all_fifos()?;

    ep0_pre_config();

    // 屏蔽掉所有 GINTMSK——本驱动靠轮询 GINTSTS。
    regs().gintmsk.set(0);

    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV init done speed={:?} GUSBCFG={:#010x} DCFG={:#010x} DCTL={:#010x}",
        speed,
        regs().gusbcfg.get(),
        regs().dcfg.get(),
        regs().dctl.get(),
    ));

    Ok(())
}

/// 把 D+ 上拉拉起来，PC 端会看到 connect / chirp / 枚举开始。
///
/// 仅在 [`dwc2_device_init`] 之后、class 已经准备好处理 EP0 时调用。
pub fn dwc2_device_softconnect() {
    if base() == 0 {
        return;
    }
    regs().dctl.modify(DCTL::SFTDISCON::CLEAR);
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV softconnect: DCTL={:#010x}",
        regs().dctl.get()
    ));
}

/// 主动断开：写 `DCTL.SFTDISCON=1`，PC 端立刻看到 disconnect。
pub fn dwc2_device_softdisconnect() {
    if base() == 0 {
        return;
    }
    regs().dctl.modify(DCTL::SFTDISCON::SET);
}

/// 调试：打印 device 段关键寄存器（可在主循环里偶尔调一次）。
pub fn dwc2_device_dump_status(tag: &str) {
    if base() == 0 {
        return;
    }
    let r = regs();
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV {tag} GINTSTS={:#010x} GUSBCFG={:#010x} DCFG={:#010x} DCTL={:#010x} DSTS={:#010x}",
        r.gintsts.get(),
        r.gusbcfg.get(),
        r.dcfg.get(),
        r.dctl.get(),
        r.dsts.get()
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV {tag} DAINT={:#010x} DAINTMSK={:#010x} DIEPCTL0={:#010x} DOEPCTL0={:#010x}",
        r.daint.get(),
        r.daintmsk.get(),
        r.diep[0].diepctl.get(),
        r.doep[0].doepctl.get(),
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DEV {tag} DOEPINT0={:#010x} DOEPTSIZ0={:#010x} DOEPDMA0={:#010x} DIEPINT0={:#010x} DIEPTSIZ0={:#010x}",
        r.doep[0].doepint.get(),
        r.doep[0].doeptsiz.get(),
        r.doep[0].doepdma.get(),
        r.diep[0].diepint.get(),
        r.diep[0].dieptsiz.get(),
    ));
}
