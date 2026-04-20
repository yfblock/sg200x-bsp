//! USB 总线拓扑：检测 **Hub**（含 QEMU 插入的虚拟 `usb-hub`）、读 Hub 描述符与端口状态，**递归**枚举下游设备并打印。
//!
//! 与 [`super::enumerate`] 配合：在 `dwc2_host_init` 之后由 `enumerate_root_port()` 调用；
//! 返回 Mass Storage 设备四元组供后续 [`crate::usb::class::mass_storage`] 使用。

use core::fmt::Write;

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::ep0 as dwc2_ep0;
use crate::usb::log::{usb_log_flush_residual, LineBufferedUsbLog};
use crate::usb::setup;

/// USB `bDeviceClass`：Hub。
const USB_CLASS_HUB: u8 = 0x09;
/// QEMU 默认 `usb-hub`（插在根口与首个外设之间）VID/PID。
const QEMU_USB_HUB_VID: u16 = 0x0409;
const QEMU_USB_HUB_PID: u16 = 0x55aa;
/// 常见 `qemu usb-storage`（亦见于枚举日志）。
const QEMU_USB_STORAGE_VID: u16 = 0x46f4;
const QEMU_USB_STORAGE_PID: u16 = 0x0001;
/// 接口类：Mass Storage。
const USB_CLASS_MSC: u8 = 0x08;
/// 接口类：Video。
const USB_CLASS_VIDEO: u8 = 0x0e;

const MAX_USB_ADDR: u8 = 127;

/// 拓扑扫描附带的设备线索（当前仅填 UVC 候选）。
#[derive(Clone, Copy, Debug, Default)]
pub struct TopologyScanExtras {
    /// 枚举到的首个 **Video(0x0e)** 类功能设备（多为 UVC 摄像头）。
    pub uvc: Option<UvcEnumerated>,
}

#[derive(Clone, Copy, Debug)]
pub struct UvcEnumerated {
    pub addr: u8,
    pub ep0_mps: u32,
    pub vid: u16,
    pub pid: u16,
}

const MAX_HUB_PORTS: u8 = 16;

#[derive(Debug, Clone, Copy)]
struct ScanState {
    next_free_addr: u8,
    msc_vid: u16,
    msc_pid: u16,
    msc_ep0_mps: u32,
    msc_addr: u32,
    have_msc: bool,
    extras: TopologyScanExtras,
}

impl ScanState {
    const fn new() -> Self {
        Self {
            next_free_addr: 1,
            msc_vid: 0,
            msc_pid: 0,
            msc_ep0_mps: 8,
            msc_addr: 0,
            have_msc: false,
            extras: TopologyScanExtras { uvc: None },
        }
    }

    fn take_addr(&mut self) -> UsbResult<u8> {
        let a = self.next_free_addr;
        if a >= MAX_USB_ADDR {
            return Err(UsbError::Protocol("usb address space full"));
        }
        self.next_free_addr = self.next_free_addr.saturating_add(1);
        Ok(a)
    }

    fn note_msc(&mut self, vid: u16, pid: u16, ep0: u32, addr: u32) {
        self.msc_vid = vid;
        self.msc_pid = pid;
        self.msc_ep0_mps = ep0;
        self.msc_addr = addr;
        self.have_msc = true;
    }
}

#[inline]
fn is_hub_device(class: u8, vid: u16, pid: u16) -> bool {
    class == USB_CLASS_HUB || (vid == QEMU_USB_HUB_VID && pid == QEMU_USB_HUB_PID)
}

#[inline]
fn is_msc_candidate(iface_class: u8, vid: u16, pid: u16) -> bool {
    iface_class == USB_CLASS_MSC || (vid == QEMU_USB_STORAGE_VID && pid == QEMU_USB_STORAGE_PID)
}

fn write_indent<W: Write>(w: &mut W, depth: u8) {
    for _ in 0..depth {
        let _ = w.write_str("  ");
    }
}

fn first_interface_class(dev: u32, ep0_mps: u32) -> UsbResult<u8> {
    let mut buf = [0u8; 64];
    dwc2_ep0::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(0, 64),
        ep0_mps,
        &mut buf,
    )?;
    let mut i: usize = 0;
    while i + 2 <= buf.len() {
        let bl = buf[i] as usize;
        if bl < 2 {
            break;
        }
        let ty = buf[i + 1];
        if ty == 4 && i + 6 <= buf.len() {
            return Ok(buf[i + 5]);
        }
        i = i.saturating_add(bl);
    }
    Ok(0)
}

fn hub_number_of_ports(hub_dev: u32, ep0_mps: u32) -> UsbResult<u8> {
    let mut buf = [0u8; 64];
    dwc2_ep0::ep0_control_read(hub_dev, setup::get_descriptor_hub(64), ep0_mps, &mut buf)?;
    if buf[0] < 3 || buf[1] != setup::USB_DT_HUB {
        return Err(UsbError::Protocol("invalid hub descriptor"));
    }
    Ok(buf[2].min(MAX_HUB_PORTS))
}

fn hub_port_status_w0(hub_dev: u32, port: u16, ep0_mps: u32) -> UsbResult<u16> {
    let mut buf = [0u8; 4];
    dwc2_ep0::ep0_control_read(
        hub_dev,
        setup::hub_get_port_status(port),
        ep0_mps,
        &mut buf,
    )?;
    Ok(u16::from_le_bytes([buf[0], buf[1]]))
}

/// 在默认地址 **0** 上的设备开始枚举；`parent_hub==0` 且 `port==0` 表示根口。
fn visit_default_depth(
    depth: u8,
    parent_hub: u8,
    port_on_hub: u8,
    st: &mut ScanState,
) -> UsbResult<()> {
    let mut w = LineBufferedUsbLog;
    let (vid, pid, ep0_mps, dev_class) = dwc2_ep0::get_device_vid_pid_default_addr()?;

    write_indent(&mut w, depth);
    if parent_hub == 0 && port_on_hub == 0 {
        let _ = writeln!(
            w,
            "[USB] root dev@0 VID={:04x} PID={:04x} dev_class={:02x}",
            vid, pid, dev_class
        );
    } else {
        let _ = writeln!(
            w,
            "[USB] dev@0 (hub {} port {}) VID={:04x} PID={:04x} dev_class={:02x}",
            parent_hub, port_on_hub, vid, pid, dev_class
        );
    }

    if is_hub_device(dev_class, vid, pid) {
        let hub_addr = st.take_addr()?;
        dwc2_ep0::set_usb_address(hub_addr, ep0_mps)?;
        dwc2_ep0::usb_post_set_address_delay();
        dwc2_ep0::set_configuration(u32::from(hub_addr), 1, ep0_mps)?;

        write_indent(&mut w, depth);
        let _ = writeln!(
            w,
            "[USB]   -> Hub enumerated addr={} ep0_mps={}",
            hub_addr, ep0_mps
        );

        let nports = hub_number_of_ports(u32::from(hub_addr), ep0_mps)?;
        write_indent(&mut w, depth);
        let _ = writeln!(w, "[USB]   -> Hub descriptor: {} downstream port(s)", nports);

        for port in 1..=nports {
            let status = match hub_port_status_w0(u32::from(hub_addr), u16::from(port), ep0_mps) {
                Ok(s) => s,
                Err(e) => {
                    write_indent(&mut w, depth);
                    let _ = writeln!(
                        w,
                        "[USB]   -> port {} GET_PORT_STATUS: {:?}",
                        port, e
                    );
                    continue;
                }
            };
            let conn = status & 1 != 0;
            write_indent(&mut w, depth);
            let _ = writeln!(
                w,
                "[USB]   -> port {} wPortStatus={:#06x} {}",
                port,
                status,
                if conn { "CONNECTED" } else { "empty" }
            );
            if !conn {
                continue;
            }

            dwc2_ep0::hub_set_port_feature(
                u32::from(hub_addr),
                u16::from(port),
                setup::HUB_PORT_FEATURE_RESET,
                ep0_mps,
            )?;
            dwc2_ep0::usb_post_hub_port_reset_delay();
            visit_default_depth(depth.saturating_add(1), hub_addr, port, st)?;
        }
        return Ok(());
    }

    // 普通功能设备
    let fn_addr = st.take_addr()?;
    dwc2_ep0::set_usb_address(fn_addr, ep0_mps)?;
    dwc2_ep0::usb_post_set_address_delay();
    dwc2_ep0::set_configuration(u32::from(fn_addr), 1, ep0_mps)?;

    let iface_class = first_interface_class(u32::from(fn_addr), ep0_mps).unwrap_or(0);
    write_indent(&mut w, depth);
    let _ = writeln!(
        w,
        "[USB]   -> function addr={} ep0_mps={} first_ifc_class={:02x}",
        fn_addr, ep0_mps, iface_class
    );

    if is_msc_candidate(iface_class, vid, pid) {
        // BOT (Bulk-Only Transport) reset 已搬到 [`crate::usb::class::mass_storage`]，
        // 由 caller 拿到设备四元组后再调用；保持 host 拓扑层与 class 协议层解耦。
        st.note_msc(vid, pid, ep0_mps, u32::from(fn_addr));
        write_indent(&mut w, depth);
        let _ = writeln!(
            w,
            "[USB]   -> Mass Storage candidate (BOT reset deferred to class layer)"
        );
    }

    if iface_class == USB_CLASS_VIDEO && st.extras.uvc.is_none() {
        st.extras.uvc = Some(UvcEnumerated {
            addr: fn_addr,
            ep0_mps,
            vid,
            pid,
        });
        write_indent(&mut w, depth);
        let _ = writeln!(
            w,
            "[USB]   -> Video class device (UVC candidate) addr={}",
            fn_addr
        );
    }

    Ok(())
}

/// 打印总线拓扑并返回 Mass Storage 设备信息（与旧 `enumerate_root_port` ABI 一致）。
pub fn enumerate_bus_print_tree() -> UsbResult<(u16, u16, u32, u32)> {
    let mut w = LineBufferedUsbLog;
    let _ = writeln!(
        w,
        "[USB] topology: recursive hub scan (QEMU may insert virtual usb-hub on single root port)"
    );

    let mut st = ScanState::new();
    let visit = visit_default_depth(0, 0, 0, &mut st);
    let _ = writeln!(w, "[USB] topology: scan finished.");
    usb_log_flush_residual();
    visit?;
    if !st.have_msc {
        return Err(UsbError::Protocol("no mass storage device found"));
    }
    Ok((st.msc_vid, st.msc_pid, st.msc_ep0_mps, st.msc_addr))
}

/// 仅递归枚举并打印拓扑，**不要求**连接 Mass Storage。
pub fn enumerate_bus_print_tree_only() -> UsbResult<TopologyScanExtras> {
    let mut w = LineBufferedUsbLog;
    let _ = writeln!(
        w,
        "[USB] topology: recursive hub scan (QEMU may insert virtual usb-hub on single root port)"
    );

    let mut st = ScanState::new();
    let visit = visit_default_depth(0, 0, 0, &mut st);
    let _ = writeln!(w, "[USB] topology: scan finished.");
    usb_log_flush_residual();
    visit?;
    Ok(st.extras)
}
