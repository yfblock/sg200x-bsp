//! DWC2 主机：**通道 0** 仅 EP0 控制；**通道 1** 仅 Bulk/Isoch（避免与部分 IP/QEMU 模型在单通道复用上的异常）。
//!
//! 控制传输 + 大块 Bulk/Isoch 的统一调度入口。Class 层（[`crate::usb::class`]）通过
//! [`ep0_control_write_no_data`] / [`ep0_control_read`] / [`ep0_control_write`] /
//! [`bulk_in`] / [`bulk_out`] / [`isoch_in_uframe`] 发送各自的 SETUP 包与数据。

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::LocalRegisterCopy;

use crate::usb::error::{UsbError, UsbResult};
use crate::utils::cache;
use crate::usb::platform;
use crate::usb::setup;
use super::mmio;
use super::regs::{Dwc2HostChannel, Dwc2Regs, GHWCFG2, HCCHAR, HCINT, HCTSIZ, HFNUM};

/// `HCINT` 快照（通道 halt 时读出的中断原因位，供上层区分 XFERCOMPL / NAK / STALL 等）。
#[allow(dead_code)]
pub type HcintSnapshot = LocalRegisterCopy<u32, HCINT::Register>;

#[inline]
fn base() -> usize {
    platform::dwc2_base_virt()
}

#[inline]
fn regs() -> &'static Dwc2Regs {
    mmio::dwc2_regs().expect("DWC2 base not set (call platform::set_dwc2_base_virt)")
}

#[inline]
fn channel(ch: u32) -> &'static Dwc2HostChannel {
    mmio::dwc2_channel(ch).expect("invalid DWC2 host channel index")
}

/// EP0 控制传输固定用通道 0。
const CH_CTL: u32 = 0;
/// Bulk 传输固定用通道 1（与控制分离）。
const CH_BULK: u32 = 1;

// HCCHAR 常用位（与 [`super::regs::HCCHAR`] 字段一致；保留 raw 形式便于按位组合 `hcchar_*`）。
const HCCHAR_CHENA: u32 = 1 << 31;
const HCCHAR_CHDIS: u32 = 1 << 30;
const HCCHAR_ODDFRM: u32 = 1 << 29;
const HCCHAR_EPDIR: u32 = 1 << 15;
const HCCHAR_EPTYPE_CONTROL: u32 = 0 << 18;
/// DesignWare：`EPTYPE` = Isochronous（与 Linux `DWC2_HCCHAR_EPTYPE_*` 一致）。
const HCCHAR_EPTYPE_ISOCH: u32 = 1 << 18;
const HCCHAR_EPTYPE_BULK: u32 = 2 << 18;
/// `MC[21:20]` 字段：周期性 IN/OUT 每 (微)帧的事务数（1..=3）。
const HCCHAR_MC_SHIFT: u32 = 20;

// HCINT 写 1 清除：清完整 11 位（含 ACK/NYET 等）。
const HCINT_ALL_W1C: u32 = 0x7FF;

/// `HCTSIZ.PID` 编码：与 DesignWare 主机通道 `HCTSIZ` 字段一致（SETUP / DATA0/1/2）。
pub const PID_DATA0: u32 = 0;
pub const PID_DATA2: u32 = 1;
pub const PID_DATA1: u32 = 2;
pub const PID_SETUP: u32 = 3;

/// EP0/小缓冲区 + UVC Bulk 大块 DMA（须物理连续；末段供 `bulk_in` 组装 MJPEG）。
#[repr(C, align(256))]
struct DmaBuf {
    bytes: [u8; 1024],
    uvc_bulk: [u8; 384 * 1024],
}

static mut DMA_BUF: DmaBuf = DmaBuf {
    bytes: [0; 1024],
    uvc_bulk: [0; 384 * 1024],
};

/// `bulk_in` / UVC 使用的 DMA 区起始偏移（紧跟在 1KiB EP0 工作区之后）。
pub const DMA_OFF_UVC_BULK: usize = 1024;
/// UVC 视频缓冲容量；前 `UVC_WORK_AREA_BYTES` 用作单微帧 RX 工作区，其余拼接 JPEG。
/// 720p MJPEG 单帧典型 100-300KB，需要 ≥320KB 的 JPEG 区。
pub const UVC_BULK_DMA_CAP: usize = 384 * 1024;

/// 整个 `DmaBuf` 大小（供边界检查）。
const DMA_BUF_TOTAL: usize = 1024 + UVC_BULK_DMA_CAP;
const _: () = assert!(DMA_BUF_TOTAL <= 1024 + 384 * 1024);

/// 安全的只读视图，供 UVC 等解析刚完成的 `bulk_in` 数据（**仅**在 `bulk_in`/cache invalidate 之后调用）。
///
/// # 参数
/// - `off`：相对内部 DMA 窗口起始的字节偏移。
/// - `len`：要暴露的连续字节长度。
#[inline]
pub fn dma_rx_slice(off: usize, len: usize) -> Option<&'static [u8]> {
    if len == 0 || off.checked_add(len)? > DMA_BUF_TOTAL {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(dma_ptr().add(off), len) })
}

/// 将数据写入内部 DMA 窗口（CPU 写，供 UVC 组装 JPEG 等；写后需自行 `dcache_clean` 若要给 DMA 读）。
///
/// # 参数
/// - `off`：相对 DMA 窗口起始的偏移。
/// - `src`：要拷贝进去的源数据。
pub fn dma_write_at(off: usize, src: &[u8]) -> UsbResult<()> {
    let end = off.checked_add(src.len()).ok_or(UsbError::Protocol("dma write overflow"))?;
    if end > DMA_BUF_TOTAL {
        return Err(UsbError::Protocol("dma write out of buf"));
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dma_ptr().add(off), src.len());
    }
    Ok(())
}

const OFF_EP0: usize = 0;
/// EP0 小缓冲读（Hub 描述符、配置前缀、`GET_PORT_STATUS`），与 Bulk DMA 区错开。
const DMA_OFF_SMALL_IO: usize = 256;
/// MSC Command Block Wrapper（31 字节，对齐到 cache line）。
pub const DMA_OFF_CBW: usize = 320;
/// MSC Command Status Wrapper（13 字节，对齐到 cache line）。
pub const DMA_OFF_CSW: usize = 384;
/// MSC SCSI 数据区（与 UVC Bulk 区共享：MSC/UVC 互斥使用）。
pub const DMA_OFF_SECTOR: usize = DMA_OFF_UVC_BULK;
/// MSC SCSI 数据区最大字节数（与 UVC Bulk 区共享）。
pub const MSC_SECTOR_DMA_CAP: usize = UVC_BULK_DMA_CAP;

#[inline]
fn spin_delay(n: u32) {
    for _ in 0..n {
        core::hint::spin_loop();
    }
}

/// DMA 工作区基址（`static mut` 仅经裸指针访问，避免 `static_mut_refs`）。
#[inline]
fn dma_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(DMA_BUF).cast::<u8>()
}

fn dma_phys(off: usize) -> u32 {
    unsafe { platform::usb_dma_phys_for(dma_ptr().add(off)) }
}

#[inline]
fn usb_bus_fence_before_dma() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence rw, rw", options(nostack));
    }
}

/// 枚举前打印 EP0/DMA 窗口 VA→PA 与 `GHWCFG2.ARCH`（调试用）。
pub fn debug_log_ep0_dma_info() {
    if base() == 0 {
        crate::usb::log::usb_log_fmt(format_args!("USB-DBG ep0_dma: DWC2 base not set"));
        return;
    }
    let r = regs();
    unsafe {
        let va = dma_ptr() as usize;
        let pa_base = platform::usb_dma_phys_for(dma_ptr());
        let pa_ep0 = platform::usb_dma_phys_for(dma_ptr().add(OFF_EP0));
        let g2 = r.ghwcfg2.get();
        let arch = r.ghwcfg2.read(GHWCFG2::ARCH);
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DBG ep0_dma va(buf)={:#010x} pa(buf)={:#010x} pa(setup)={:#010x}",
            va, pa_base, pa_ep0
        ));
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DBG GHWCFG2={:#010x} ARCH={} (0=slave 1=ext-dma 2=int-dma)",
            g2, arch
        ));
        let snpsid = r.gsnpsid.get();
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DBG GSNPSID={:#010x} core_rev={:#06x}",
            snpsid,
            snpsid & 0xffff
        ));
        if arch == 2 {
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-DBG ARCH=2 为内部 DMA：主机通道必须用 HCDMA，不能关 DMA 改纯 FIFO/slave 枚举"
            ));
        }
        if pa_base as usize == va {
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-DBG ep0_dma: VA==PA（恒等映射），HCDMA 地址与 Linux phys-virt-offset=0 一致"
            ));
        }
    }
}

fn dump_channel_timeout_debug(ch: u32, phase: &'static str) {
    if base() == 0 {
        return;
    }
    let r = regs();
    let c = channel(ch);
    let hprt = r.hprt0.get();
    let gint = r.gintsts.get();
    let gintm = r.gintmsk.get();
    let gahb = r.gahbcfg.get();
    let grst = r.grstctl.get();
    let gotg = r.gotgctl.get();
    let hcchar = c.hcchar.get();
    let hcint = c.hcint.get();
    let hcintm = c.hcintmsk.get();
    let hctsiz = c.hctsiz.get();
    let hcdma = c.hcdma.get();
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT [{}] ch={} HPRT0={:#010x} (CONNSTS={} SPD={})",
        phase,
        ch,
        hprt,
        (hprt & 1) != 0,
        (hprt >> 17) & 3
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT GINTSTS={:#010x} GINTMSK={:#010x} GAHBCFG={:#010x} GRSTCTL={:#010x}",
        gint, gintm, gahb, grst
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT GOTGCTL={:#010x} HCCHAR={:#010x} HCINT={:#010x} HCINTMSK={:#010x}",
        gotg, hcchar, hcint, hcintm
    ));
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-TOUT HCTSIZ={:#010x} HCDMA={:#010x}",
        hctsiz, hcdma
    ));
}

fn ch_wait_disabled(ch: u32) -> UsbResult<()> {
    let c = channel(ch);
    for _ in 0..2_000_000u32 {
        if !c.hcchar.is_set(HCCHAR::CHENA) {
            return Ok(());
        }
        spin_delay(8);
    }
    dump_channel_timeout_debug(ch, "ch_wait_disabled");
    Err(UsbError::Timeout)
}

/// 若通道仍忙，按 Linux `dwc2_hc_halt` 同时置 `CHENA|CHDIS` 请求停止。
fn ch_halt(ch: u32) {
    let c = channel(ch);
    let v = c.hcchar.get();
    if v & HCCHAR_CHENA == 0 {
        return;
    }
    c.hcchar.set(v | HCCHAR_CHENA | HCCHAR_CHDIS);
    for _ in 0..500_000u32 {
        if !c.hcchar.is_set(HCCHAR::CHENA) {
            return;
        }
        spin_delay(8);
    }
}

fn ch_wait_halted(ch: u32) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    for _ in 0..8_000_000u32 {
        let hi = c.hcint.extract();
        if hi.is_set(HCINT::CHHLTD) {
            c.hcint.set(hi.get());
            return Ok(hi);
        }
        spin_delay(8);
    }
    dump_channel_timeout_debug(ch, "ch_wait_halted");
    Err(UsbError::Timeout)
}

/// 主机通道单次传输：清中断、写 `HCTSIZ`/`HCDMA`/`HCCHAR`，等待 `CHHLTD`。
/// EP0 上对 NAK / XACTERR 做有限次重试；STALL 立即返回。
unsafe fn ch_xfer(ch: u32, hcchar: u32, hctsiz: u32, dma_off: u32) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    let dmap = dma_phys(dma_off as usize);

    // EP0 control 上：NAK = 设备未就绪，自动重试；XACTERR = CRC/PID/babble，
    // 在 reset 解除后总线还可能不稳定，也允许少量重试。STALL 立即返回。
    const NAK_RETRIES: u32 = 64;
    const XACT_RETRIES: u32 = 8;
    let mut xact_left = XACT_RETRIES;
    for attempt in 0..=NAK_RETRIES {
        ch_wait_disabled(ch)?;
        ch_halt(ch);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(hctsiz);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        c.hcchar.set(hcchar | HCCHAR_CHENA);
        let st = ch_wait_halted(ch)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.is_set(HCINT::XACTERR) {
            if xact_left == 0 {
                crate::usb::log::usb_log_fmt(format_args!(
                    "USB-XACT EXHAUSTED ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                    ch, hcchar, hctsiz, dmap, st.get()
                ));
                dump_channel_timeout_debug(ch, "ch_xfer XACT exhausted");
                return Err(UsbError::Protocol("ch xfer error (XACT)"));
            }
            xact_left -= 1;
            // XACTERR 退避更久（让 D+/D- 稳定再试），约 1ms。
            spin_delay(2_000_000);
            continue;
        }
        if st.is_set(HCINT::NAK) {
            if attempt == NAK_RETRIES {
                crate::usb::log::usb_log_fmt(format_args!(
                    "USB-NAK EXHAUSTED ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                    ch, hcchar, hctsiz, dmap, st.get()
                ));
                return Err(UsbError::Protocol("ch xfer NAK exhausted"));
            }
            // Synopsys 建议 NAK 后等待 ~1 ms 再重试（HSEOF），这里用粗粒度 spin。
            spin_delay(200_000);
            continue;
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-CHHLTD-NO-XFER ch={} hcchar={:#010x} hctsiz={:#010x} dma={:#010x} hcint={:#010x}",
                ch, hcchar, hctsiz, dmap, st.get()
            ));
            return Err(UsbError::Protocol("CHHLTD without XFERCOMPL"));
        }
        return Ok(st);
    }
    unreachable!()
}

/// 视频 Bulk/Isoch：**NAK 单独返回**（设备尚无帧数据时常见），调用方忙等重试；**不与 EP0 共用**。
unsafe fn ch_xfer_video_retryable(
    ch: u32,
    hcchar: u32,
    hctsiz: u32,
    dma_off: u32,
) -> UsbResult<HcintSnapshot> {
    let c = channel(ch);
    ch_wait_disabled(ch)?;
    ch_halt(ch);
    c.hcsplt.set(0);
    c.hcint.set(HCINT_ALL_W1C);
    c.hctsiz.set(hctsiz);
    let dmap = dma_phys(dma_off as usize);
    usb_bus_fence_before_dma();
    c.hcdma.set(dmap);
    usb_bus_fence_before_dma();
    c.hcchar.set(hcchar | HCCHAR_CHENA);
    let st = ch_wait_halted(ch)?;
    if st.is_set(HCINT::STALL) {
        return Err(UsbError::Stall);
    }
    if st.is_set(HCINT::XACTERR) {
        return Err(UsbError::Protocol("ch xfer XACTERR"));
    }
    if st.is_set(HCINT::NAK) {
        return Err(UsbError::Nak);
    }
    if !st.is_set(HCINT::XFERCOMPL) {
        return Err(UsbError::Protocol("CHHLTD without XFERCOMPL"));
    }
    Ok(st)
}

unsafe fn hcchar_control(dev: u32, ep: u32, mps: u32, dir_in: bool) -> u32 {
    let mut v = mps & 0x7ff;
    v |= (ep & 0xf) << 11;
    if dir_in {
        v |= HCCHAR_EPDIR;
    }
    v |= HCCHAR_EPTYPE_CONTROL;
    v |= (dev & 0x7f) << 22;
    v
}

unsafe fn hcchar_bulk(dev: u32, ep: u32, mps: u32, dir_in: bool) -> u32 {
    let mut v = mps & 0x7ff;
    v |= (ep & 0xf) << 11;
    if dir_in {
        v |= HCCHAR_EPDIR;
    }
    v |= HCCHAR_EPTYPE_BULK;
    v |= (dev & 0x7f) << 22;
    v
}

unsafe fn hcchar_isoch(dev: u32, ep: u32, mps: u32, mult: u32, dir_in: bool) -> u32 {
    let mut v = mps & 0x7ff;
    v |= (ep & 0xf) << 11;
    if dir_in {
        v |= HCCHAR_EPDIR;
    }
    v |= HCCHAR_EPTYPE_ISOCH;
    let mc = mult.clamp(1, 3) & 0x3;
    v |= mc << HCCHAR_MC_SHIFT;
    v |= (dev & 0x7f) << 22;
    v
}

/// 读 HFNUM 决定下个微帧奇偶；若当前帧 LSB=0（偶），下一帧为奇 -> 设 ODDFRM；反之清 0。
#[inline]
fn next_uframe_oddfrm() -> u32 {
    let fr = regs().hfnum.read(HFNUM::FRNUM);
    if (fr & 1) == 0 { HCCHAR_ODDFRM } else { 0 }
}

/// 当前 USB 微帧编号（`HFNUM` 低 16 位）；每 microframe (125µs) 递增并回绕。
/// 用于 UVC 抓帧的时间统计（避免 lib 直接依赖 axhal）。
#[inline]
pub fn current_uframe() -> u32 {
    regs().hfnum.read(HFNUM::FRNUM)
}

unsafe fn hctsiz(pid: u32, pktcnt: u32, xfersize: u32) -> u32 {
    (HCTSIZ::PID.val(pid) + HCTSIZ::PKTCNT.val(pktcnt) + HCTSIZ::XFERSIZE.val(xfersize)).value
}

/// `SET_ADDRESS` 后粗延时，满足 USB 2.0 在下一事务前使用新地址的要求。
pub fn usb_post_set_address_delay() {
    spin_delay(20_000_000);
}

/// Hub 下游端口 `PORT_RESET` 后给设备恢复时间（粗粒度忙等）。
pub fn usb_post_hub_port_reset_delay() {
    spin_delay(30_000_000);
}

#[inline]
fn normalize_ep0_mps(b: u8) -> u32 {
    match b {
        8 | 16 | 32 | 64 => b as u32,
        _ => 8,
    }
}

/// 控制传输无数据阶段：`SETUP` + `STATUS` IN（零长度）。
///
/// # 参数
/// - `dev`：目标设备 USB 地址（7 位数值，写入主机通道 DevAddr）。
/// - `setup`：8 字节标准 SETUP 包（小端字段已拼好）。
/// - `ep0_mps`：该设备 EP0 最大包长（字节，8/16/32/64）。
pub fn ep0_control_write_no_data(dev: u32, setup: [u8; 8], ep0_mps: u32) -> UsbResult<()> {
    unsafe {
        core::ptr::copy_nonoverlapping(setup.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(CH_CTL, hc, hctsiz(PID_SETUP, 1, 8), OFF_EP0 as u32)?;

        let hc = hcchar_control(dev, 0, ep0_mps, true);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 0),
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// 在默认地址 0 上发送 `SET_ADDRESS`。
///
/// # 参数
/// - `addr`：设备新地址，合法 **1..=127**。
/// - `ep0_mps`：地址 0 阶段使用的 EP0 MPS（枚举首步常用 64）。
pub fn set_usb_address(addr: u8, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(0, setup::set_address(addr), ep0_mps)
}

/// 对已寻址设备发送 `SET_CONFIGURATION`。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `cfg`：`bConfigurationValue`（通常非 0 表示激活配置）。
/// - `ep0_mps`：该设备 EP0 最大包长。
pub fn set_configuration(dev: u32, cfg: u8, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(dev, setup::set_configuration(cfg), ep0_mps)
}

/// `GET_CONFIGURATION`：返回当前 `bConfigurationValue`（单字节）。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep0_mps`：该设备 EP0 最大包长。
#[allow(dead_code)]
pub fn get_configuration(dev: u32, ep0_mps: u32) -> UsbResult<u8> {
    unsafe {
        let setup_pkt = setup::get_configuration();
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_SETUP, 1, 8),
            OFF_EP0 as u32,
        )?;

        hc = hcchar_control(dev, 0, ep0_mps, true);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 1),
            OFF_EP0 as u32,
        )?;
        cache::dcache_invalidate_after_dma(dma_ptr().add(OFF_EP0), 1);
        let v = dma_ptr().add(OFF_EP0).read();

        hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 0),
            OFF_EP0 as u32,
        )?;

        Ok(v)
    }
}

/// `GET_DESCRIPTOR(DEVICE, 18)` @ 地址 0；返回 VID、PID、EP0 MPS、`bDeviceClass`。
///
/// # 返回值
/// `(vid, pid, ep0_mps, b_device_class)`，均在设备描述符前 18 字节内解析。
pub fn get_device_vid_pid_default_addr() -> UsbResult<(u16, u16, u32, u8)> {
    unsafe {
        let wlen: u16 = 18;
        let setup_pkt = setup::get_descriptor_device(wlen);
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_control(0, 0, 64, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_SETUP, 1, 8),
            OFF_EP0 as u32,
        )?;

        hc = hcchar_control(0, 0, 64, true);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, wlen as u32),
            OFF_EP0 as u32,
        )?;
        cache::dcache_invalidate_after_dma(dma_ptr().add(OFF_EP0), wlen as usize);

        let sl = core::slice::from_raw_parts(dma_ptr().add(OFF_EP0), wlen as usize);
        if sl.len() < 12 {
            return Err(UsbError::Protocol("short descriptor"));
        }
        let vid = u16::from_le_bytes([sl[8], sl[9]]);
        let pid = u16::from_le_bytes([sl[10], sl[11]]);
        let ep0_mps = normalize_ep0_mps(sl[7]);
        let b_device_class = sl[4];

        hc = hcchar_control(0, 0, 64, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 0),
            OFF_EP0 as u32,
        )?;

        Ok((vid, pid, ep0_mps, b_device_class))
    }
}

/// 对 **已寻址** Hub 发送 `SET_PORT_FEATURE`（无数据阶段）。
///
/// # 参数
/// - `dev`：Hub 的 USB 地址。
/// - `port`：下游端口号（从 1 开始）。
/// - `feature`：Hub 端口特性选择子（如 [`crate::usb::setup::HUB_PORT_FEATURE_POWER`]）。
/// - `ep0_mps`：Hub 的 EP0 最大包长。
pub fn hub_set_port_feature(dev: u32, port: u16, feature: u16, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(dev, setup::hub_set_port_feature(port, feature), ep0_mps)
}

/// 对 **已寻址** Hub 发送 `CLEAR_PORT_FEATURE`（清除 `C_PORT_*` 等变化位）。
///
/// # 参数
/// - `dev`、`port`、`ep0_mps`：含义同 [`hub_set_port_feature`]。
/// - `feature`：要清除的端口特性或 `C_PORT_*` 常量。
pub fn hub_clear_port_feature(dev: u32, port: u16, feature: u16, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(dev, setup::hub_clear_port_feature(port, feature), ep0_mps)
}

/// 控制传输：SETUP + 若干 IN 数据包（DATA1/DATA0 交替）+ STATUS OUT（ZLP，DATA1）。
///
/// 数据写入 `out`（总长度 = `out.len()`）。适用于 Hub 描述符、配置前缀、`GET_PORT_STATUS` 等。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `setup_pkt`：8 字节 SETUP（`wLength` 应等于 `out.len()` 的期望读长）。
/// - `ep0_mps`：EP0 最大包长。
/// - `out`：接收缓冲区，长度须与 SETUP 中 `wLength` 一致且在 `(0,4096]`。
pub fn ep0_control_read(dev: u32, setup_pkt: [u8; 8], ep0_mps: u32, out: &mut [u8]) -> UsbResult<()> {
    if out.is_empty() || out.len() > 4096 {
        return Err(UsbError::Protocol("bad ep0 read len"));
    }
    let total = out.len() as u32;
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_SETUP, 1, 8),
            OFF_EP0 as u32,
        )?;

        let mut left = total;
        let mut out_off: usize = 0;
        let mut toggle = PID_DATA1;
        while left > 0 {
            let chunk = left.min(ep0_mps);
            let pkts = pktcnt_for(ep0_mps, chunk);
            hc = hcchar_control(dev, 0, ep0_mps, true);
            ch_xfer(
                CH_CTL,
                hc,
                hctsiz(toggle, pkts, chunk),
                DMA_OFF_SMALL_IO as u32,
            )?;
            cache::dcache_invalidate_after_dma(dma_ptr().add(DMA_OFF_SMALL_IO), chunk as usize);
            core::ptr::copy_nonoverlapping(
                dma_ptr().add(DMA_OFF_SMALL_IO),
                out.as_mut_ptr().add(out_off),
                chunk as usize,
            );
            out_off += chunk as usize;
            left -= chunk;
            toggle = if toggle == PID_DATA1 {
                PID_DATA0
            } else {
                PID_DATA1
            };
        }

        hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 0),
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// 控制写：`SETUP` + `DATA` OUT（可多包）+ `STATUS` IN（ZLP）。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `setup_pkt`：8 字节 SETUP（`wLength` 应等于 `data.len()`）。
/// - `ep0_mps`：EP0 最大包长。
/// - `data`：OUT 数据阶段负载（最大 4096 字节）。
pub fn ep0_control_write(dev: u32, setup_pkt: [u8; 8], ep0_mps: u32, data: &[u8]) -> UsbResult<()> {
    if data.len() > 4096 {
        return Err(UsbError::Protocol("bad ep0 write data len"));
    }
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_SETUP, 1, 8),
            OFF_EP0 as u32,
        )?;

        let mut left = data.len() as u32;
        let mut src: usize = 0;
        let mut toggle = PID_DATA1;
        while left > 0 {
            let chunk = left.min(ep0_mps);
            let pkts = pktcnt_for(ep0_mps, chunk);
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(src),
                dma_ptr().add(DMA_OFF_SMALL_IO),
                chunk as usize,
            );
            cache::dcache_clean_for_dma(dma_ptr().add(DMA_OFF_SMALL_IO), chunk as usize);
            hc = hcchar_control(dev, 0, ep0_mps, false);
            ch_xfer(
                CH_CTL,
                hc,
                hctsiz(toggle, pkts, chunk),
                DMA_OFF_SMALL_IO as u32,
            )?;
            src += chunk as usize;
            left -= chunk;
            toggle = if toggle == PID_DATA1 {
                PID_DATA0
            } else {
                PID_DATA1
            };
        }

        hc = hcchar_control(dev, 0, ep0_mps, true);
        ch_xfer(
            CH_CTL,
            hc,
            hctsiz(PID_DATA1, 1, 0),
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// EP0 控制读：固定读 **1** 字节数据 IN（含 cache 维护），用于 MSC `GET_MAX_LUN` 等。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `setup_pkt`：SETUP（`wLength` 应为 1）。
/// - `ep0_mps`：EP0 最大包长。
pub fn ep0_control_read_one_byte(dev: u32, setup_pkt: [u8; 8], ep0_mps: u32) -> UsbResult<u8> {
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(CH_CTL, hc, hctsiz(PID_SETUP, 1, 8), OFF_EP0 as u32)?;

        hc = hcchar_control(dev, 0, ep0_mps, true);
        ch_xfer(CH_CTL, hc, hctsiz(PID_DATA1, 1, 1), OFF_EP0 as u32)?;
        cache::dcache_invalidate_after_dma(dma_ptr().add(OFF_EP0), 1);
        let v = dma_ptr().add(OFF_EP0).read();

        hc = hcchar_control(dev, 0, ep0_mps, false);
        ch_xfer(CH_CTL, hc, hctsiz(PID_DATA1, 1, 0), OFF_EP0 as u32)?;
        Ok(v)
    }
}

/// 从内部 DMA 窗口拷贝到 `dst`（**不**做 cache 维护；调用方须已 invalidate 或仅 CPU 写入区）。
///
/// # 参数
/// - `off`：源数据在 DMA 窗口内的起始字节偏移。
/// - `dst`：目标缓冲区；拷贝长度为 `dst.len()`。
pub fn dma_copy_out(off: usize, dst: &mut [u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(dma_ptr().add(off), dst.as_mut_ptr(), dst.len());
    }
}

/// 计算 `HCTSIZ.PKTCNT`：按 `mps` 分包后的包数（至少为 1）。
///
/// # 参数
/// - `mps`：端点最大包长（字节），为 0 时按 1 包处理。
/// - `nbytes`：本段传输总字节数。
fn pktcnt_for(mps: u32, nbytes: u32) -> u32 {
    if mps == 0 {
        return 1;
    }
    (nbytes + mps - 1) / mps
}

/// Bulk OUT：将 `data` 写入内部 DMA 窗口后，经主机通道 1 发出。
///
/// # 参数
/// - `dev`：设备 USB 地址（7 位数值）。
/// - `ep`：Bulk OUT 端点号（`bEndpointAddress & 0x0F`，不含 `0x80` 方向位）。
/// - `mps`：该端点 `wMaxPacketSize` 的低 11 位（字节）。
/// - `pid`：`HCTSIZ` 数据 PID，取 `PID_DATA0` / `PID_DATA1` 等常量。
/// - `data`：OUT 事务负载（非空，且长度受实现上限约束）。
/// - `dma_off`：DMA 窗口内放置数据的起始偏移；须能容纳 `data` 且已 `dcache_clean`。
pub fn bulk_out(dev: u32, ep: u32, mps: u32, pid: u32, data: &[u8], dma_off: usize) -> UsbResult<()> {
    if data.is_empty() || data.len() > 0x7ffff {
        return Err(UsbError::Protocol("bad bulk out len"));
    }
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), dma_ptr().add(dma_off), data.len());
        cache::dcache_clean_for_dma(dma_ptr().add(dma_off), data.len());
        let hc = hcchar_bulk(dev, ep, mps, false);
        let pkts = pktcnt_for(mps, data.len() as u32);
        ch_xfer(
            CH_BULK,
            hc,
            hctsiz(pid, pkts, data.len() as u32),
            dma_off as u32,
        )?;
        Ok(())
    }
}

#[inline]
fn spin_short() {
    spin_delay(64);
}

/// Bulk IN；遇 NAK 自动重试（UVC 常见）。返回本事务实际收到的字节数。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep`：Bulk IN 端点号（`bEndpointAddress & 0x0F`）。
/// - `mps`：端点 `wMaxPacketSize` 低 11 位。
/// - `pid`：`HCTSIZ` 数据 PID（`PID_DATA0` / `PID_DATA1`）。
/// - `len`：本次请求的最大接收长度；亦写入 `HCTSIZ.XFERSIZE`。
/// - `dma_off`：接收数据落入 DMA 窗口的偏移；成功返回后对应区间已 invalidate。
pub fn bulk_in(dev: u32, ep: u32, mps: u32, pid: u32, len: usize, dma_off: usize) -> UsbResult<usize> {
    if len == 0 || len > 0x7ffff {
        return Err(UsbError::Protocol("bad bulk in len"));
    }
    unsafe {
        let hc = hcchar_bulk(dev, ep, mps, true);
        let pkts = pktcnt_for(mps, len as u32);
        let tsiz = hctsiz(pid, pkts, len as u32);

        for _ in 0..4_000_000u32 {
            match ch_xfer_video_retryable(CH_BULK, hc, tsiz, dma_off as u32) {
                Ok(_) => {}
                Err(UsbError::Nak) => {
                    spin_short();
                    continue;
                }
                Err(e) => return Err(e),
            }

            let rem = channel(CH_BULK).hctsiz.read(HCTSIZ::XFERSIZE);
            let mut actual = (len as u32).saturating_sub(rem) as usize;
            actual = actual.min(len);
            if actual == 0 && len > 0 {
                actual = len;
            }
            if actual > 0 {
                cache::dcache_invalidate_after_dma(dma_ptr().add(dma_off), actual);
            }
            return Ok(actual);
        }
        Err(UsbError::Timeout)
    }
}

/// Isoch IN 高带宽：在 **下一微帧** 启动一次通道，最多接收 `mult` 个 USB 事务（每个 ≤ `mps` 字节）。
///
/// `mps_raw` 为端点描述符 `wMaxPacketSize` 原始值（含高两位表示 HS 高带宽倍率）。
/// 返回本次实际收到的字节数（0 表示设备本微帧无数据 / 0-byte 包）。
///
/// **PID 编码（DWC2）**：单事务 DATA0；双事务 DATA1；三事务 DATA2。
/// **MC**：写入 `HCCHAR.MC` = `mult`。
/// **ODDFRM**：根据 `HFNUM` 选择下个微帧的奇偶。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep`：Isoch IN 端点号（`bEndpointAddress & 0x0F`）。
/// - `mps_raw`：端点描述符中的 `wMaxPacketSize` 原值（低 11 位为每事务字节数，bit12..11 为高带宽倍数减一）。
/// - `dma_off`：本微帧接收缓冲在内部 DMA 窗口中的起始偏移。
pub fn isoch_in_uframe(dev: u32, ep: u32, mps_raw: u16, dma_off: usize) -> UsbResult<usize> {
    let mps = u32::from(mps_raw & 0x7ff);
    let mult = u32::from((mps_raw >> 11) & 0x3) + 1;
    if mps == 0 || mult == 0 || mult > 3 {
        return Err(UsbError::Protocol("bad isoch mps_raw"));
    }
    let xfersize = mps.saturating_mul(mult);
    if (xfersize as usize) > UVC_BULK_DMA_CAP {
        return Err(UsbError::Protocol("isoch xfer > dma cap"));
    }
    let pid = match mult {
        3 => PID_DATA2,
        2 => PID_DATA1,
        _ => PID_DATA0,
    };
    let pktcnt = mult;

    unsafe {
        let hc_base = hcchar_isoch(dev, ep, mps, mult, true);
        let tsiz = hctsiz(pid, pktcnt, xfersize);

        let c = channel(CH_BULK);
        ch_wait_disabled(CH_BULK)?;
        ch_halt(CH_BULK);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(tsiz);
        let dmap = dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let oddfrm = next_uframe_oddfrm();
        c.hcchar.set(hc_base | oddfrm | HCCHAR_CHENA);

        let st = ch_wait_halted(CH_BULK)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.is_set(HCINT::AHBERR) {
            return Err(UsbError::Hardware("AHBERR on isoch"));
        }
        if st.is_set(HCINT::FRMOVRN)
            || st.is_set(HCINT::XACTERR)
            || st.is_set(HCINT::BBLERR)
            || st.is_set(HCINT::DATATGLERR)
            || st.is_set(HCINT::NYET)
            || st.is_set(HCINT::NAK)
        {
            return Ok(0);
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            return Ok(0);
        }
        let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
        let actual = xfersize.saturating_sub(rem) as usize;
        if actual > 0 {
            cache::dcache_invalidate_after_dma(dma_ptr().add(dma_off), actual);
        }
        Ok(actual)
    }
}

/// 兼容旧 API：单事务 Isoch IN（仅供调试，非高带宽）。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep`：Isoch IN 端点号。
/// - `mps`：单包最大字节数（与端点 `wMaxPacketSize` 一致）。
/// - `len`：本次读取长度，须 **≤ `mps`** 且非零。
/// - `dma_off`：DMA 窗口内接收偏移。
#[allow(dead_code)]
pub fn isoch_in(dev: u32, ep: u32, mps: u32, len: usize, dma_off: usize) -> UsbResult<usize> {
    if len == 0 || len > 0x7ffff || len > mps as usize {
        return Err(UsbError::Protocol("bad isoch in len"));
    }
    unsafe {
        let hc = hcchar_isoch(dev, ep, mps, 1, true);
        let pkts = pktcnt_for(mps, len as u32);
        let tsiz = hctsiz(PID_DATA0, pkts, len as u32);

        let c = channel(CH_BULK);
        ch_wait_disabled(CH_BULK)?;
        ch_halt(CH_BULK);
        c.hcsplt.set(0);
        c.hcint.set(HCINT_ALL_W1C);
        c.hctsiz.set(tsiz);
        let dmap = dma_phys(dma_off);
        usb_bus_fence_before_dma();
        c.hcdma.set(dmap);
        usb_bus_fence_before_dma();
        let oddfrm = next_uframe_oddfrm();
        c.hcchar.set(hc | oddfrm | HCCHAR_CHENA);

        let st = ch_wait_halted(CH_BULK)?;
        if st.is_set(HCINT::STALL) {
            return Err(UsbError::Stall);
        }
        if st.is_set(HCINT::FRMOVRN)
            || st.is_set(HCINT::XACTERR)
            || st.is_set(HCINT::BBLERR)
            || st.is_set(HCINT::NYET)
        {
            return Ok(0);
        }
        if !st.is_set(HCINT::XFERCOMPL) {
            return Ok(0);
        }
        let rem = c.hctsiz.read(HCTSIZ::XFERSIZE);
        let actual = (len as u32).saturating_sub(rem) as usize;
        if actual > 0 {
            cache::dcache_invalidate_after_dma(dma_ptr().add(dma_off), actual);
        }
        Ok(actual)
    }
}
