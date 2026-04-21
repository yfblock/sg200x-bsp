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

/// 拓扑扫描附带的设备线索（UVC 摄像头 / MSC U 盘等）。
#[derive(Clone, Copy, Debug, Default)]
pub struct TopologyScanExtras {
    /// 枚举到的首个 **Video(0x0e)** 类功能设备（多为 UVC 摄像头）。
    pub uvc: Option<UvcEnumerated>,
    /// 枚举到的首个 **Mass Storage(0x08)** 类功能设备（U 盘 / 读卡器）。
    pub msc: Option<MscEnumerated>,
}

#[derive(Clone, Copy, Debug)]
pub struct UvcEnumerated {
    pub addr: u8,
    pub ep0_mps: u32,
    pub vid: u16,
    pub pid: u16,
}

/// USB Mass Storage（BOT/BBB）枚举结果：包含设备地址、EP0 MPS、VID/PID、
/// **MSC 接口号**（首个 `bInterfaceClass=0x08` 的 `bInterfaceNumber`），以及
/// **Bulk IN/OUT 端点号 + 各自 wMaxPacketSize**（HS=512 / FS=64）。
///
/// 拿到后即可调用 [`crate::usb::class::mass_storage::bulk_only_reset`] /
/// [`crate::usb::class::mass_storage::get_max_lun`]，
/// 以及上层的 SCSI 命令封装（如 INQUIRY / READ_CAPACITY / READ(10) ...）。
#[derive(Clone, Copy, Debug)]
pub struct MscEnumerated {
    pub addr: u8,
    pub ep0_mps: u32,
    pub vid: u16,
    pub pid: u16,
    /// MSC 接口号（CBW 的 `bCBWLUN`/`Mass Storage Reset` 等的 `wIndex`）。
    pub iface_num: u8,
    /// `bEndpointAddress & 0x7F`（Bulk IN）。0 表示未解析到端点。
    pub bulk_in_ep: u8,
    /// Bulk IN 端点 `wMaxPacketSize`（HS=512，FS=64）。
    pub bulk_in_mps: u16,
    /// `bEndpointAddress & 0x7F`（Bulk OUT）。0 表示未解析到端点。
    pub bulk_out_ep: u8,
    /// Bulk OUT 端点 `wMaxPacketSize`（HS=512，FS=64）。
    pub bulk_out_mps: u16,
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
            extras: TopologyScanExtras {
                uvc: None,
                msc: None,
            },
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

/// 解析配置描述符，定位首个 **MSC 接口（`bInterfaceClass = 0x08`）** 的
/// `bInterfaceNumber` 与 **同接口下** 的 Bulk IN/OUT 端点号 + `wMaxPacketSize`。
///
/// 返回 `(iface_num, bulk_in_ep, bulk_in_mps, bulk_out_ep, bulk_out_mps)`；
/// 任何端点未找到则对应字段为 `0`。
fn parse_msc_interface_endpoints(
    dev: u32,
    ep0_mps: u32,
) -> UsbResult<(u8, u8, u16, u8, u16)> {
    let mut hdr = [0u8; 9];
    dwc2_ep0::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(0, 9),
        ep0_mps,
        &mut hdr,
    )?;
    if hdr[1] != setup::USB_DT_CONFIGURATION {
        return Err(UsbError::Protocol("not a configuration descriptor"));
    }
    let total = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
    if total < 9 || total > 512 {
        return Err(UsbError::Protocol("bad cfg total length"));
    }

    let mut buf = [0u8; 512];
    dwc2_ep0::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(0, total as u16),
        ep0_mps,
        &mut buf[..total],
    )?;

    let mut i: usize = 0;
    let mut in_msc_iface = false;
    let mut iface_num = 0u8;
    let mut bin_ep = 0u8;
    let mut bin_mps = 0u16;
    let mut bout_ep = 0u8;
    let mut bout_mps = 0u16;
    while i + 2 <= total {
        let bl = buf[i] as usize;
        if bl < 2 || i + bl > total {
            break;
        }
        let ty = buf[i + 1];
        if ty == 4 && i + 9 <= total {
            // INTERFACE descriptor
            let cls = buf[i + 5];
            let alt = buf[i + 3];
            in_msc_iface = cls == USB_CLASS_MSC && alt == 0;
            if in_msc_iface {
                iface_num = buf[i + 2];
            }
        } else if ty == 5 && in_msc_iface && i + 7 <= total {
            // ENDPOINT descriptor: bEndpointAddress(1)+bmAttributes(1)+wMaxPacketSize(2)+bInterval(1)
            let ep_addr = buf[i + 2];
            let attr = buf[i + 3] & 0x03;
            let mps = u16::from_le_bytes([buf[i + 4], buf[i + 5]]) & 0x07ff;
            if attr == 2 {
                // Bulk
                if ep_addr & 0x80 != 0 {
                    if bin_ep == 0 {
                        bin_ep = ep_addr & 0x7f;
                        bin_mps = mps;
                    }
                } else if bout_ep == 0 {
                    bout_ep = ep_addr & 0x7f;
                    bout_mps = mps;
                }
            }
        }
        i = i.saturating_add(bl);
    }
    Ok((iface_num, bin_ep, bin_mps, bout_ep, bout_mps))
}

/// Hub 描述符关键字段：端口数、`bPwrOn2PwrGood`（2ms 单位的端口上电稳定时间）。
struct HubInfo {
    nports: u8,
    pwr_on_2_pwr_good_ms: u32,
}

fn hub_info(hub_dev: u32, ep0_mps: u32) -> UsbResult<HubInfo> {
    let mut buf = [0u8; 64];
    dwc2_ep0::ep0_control_read(hub_dev, setup::get_descriptor_hub(64), ep0_mps, &mut buf)?;
    if buf[0] < 7 || buf[1] != setup::USB_DT_HUB {
        return Err(UsbError::Protocol("invalid hub descriptor"));
    }
    let nports = buf[2].min(MAX_HUB_PORTS);
    let pwr_on = u32::from(buf[5]).saturating_mul(2);
    Ok(HubInfo {
        nports,
        pwr_on_2_pwr_good_ms: pwr_on,
    })
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

/// 粗粒度毫秒延迟，仅用于 hub 端口上电后稳定（`bPwrOn2PwrGood`）+ 端口 reset 等待。
/// 1ms ≈ 250_000 spin-loop 在当前 sg2002 配置下经验值（与 ep0 内部 `spin_delay`
/// 校准的同一档参数：`spin_delay(20_000_000)` ≈ 80ms）。
fn spin_delay_ms(ms: u32) {
    let cycles = ms.saturating_mul(250_000);
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// USB 2.0 hub 端口速度位（`wPortStatus[10:9]`）→ 文字描述。
fn port_speed_str(status: u16) -> &'static str {
    let ls = (status >> 9) & 1;
    let hs = (status >> 10) & 1;
    match (hs, ls) {
        (1, _) => "HS",
        (_, 1) => "LS",
        _ => "FS",
    }
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

        let info = hub_info(u32::from(hub_addr), ep0_mps)?;
        let nports = info.nports;
        let pwr_good_ms = info.pwr_on_2_pwr_good_ms.max(20); // 给 ≥20ms 富余
        write_indent(&mut w, depth);
        let _ = writeln!(
            w,
            "[USB]   -> Hub descriptor: {} downstream port(s), PwrOn2PwrGood={} ms",
            nports, pwr_good_ms
        );

        // ① 给所有下游端口供电：USB 2.0 spec §11.11.1：hub 上电后端口默认 PowerOff，
        //    必须由 host 显式 SET_PORT_FEATURE(PORT_POWER) 才会给下游 VBUS。
        for port in 1..=nports {
            if let Err(e) = dwc2_ep0::hub_set_port_feature(
                u32::from(hub_addr),
                u16::from(port),
                setup::HUB_PORT_FEATURE_POWER,
                ep0_mps,
            ) {
                write_indent(&mut w, depth);
                let _ = writeln!(w, "[USB]   -> port {} POWER fail: {:?}", port, e);
            }
        }
        // ② 等 PwrOn2PwrGood + 100ms 让下游设备 VBUS 稳定 + 自检
        spin_delay_ms(pwr_good_ms.saturating_add(100));

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

            // ③ 清 C_PORT_CONNECTION（连接变化位），再 PORT_RESET
            let _ = dwc2_ep0::hub_clear_port_feature(
                u32::from(hub_addr),
                u16::from(port),
                setup::HUB_PORT_FEATURE_C_CONNECTION,
                ep0_mps,
            );

            if let Err(e) = dwc2_ep0::hub_set_port_feature(
                u32::from(hub_addr),
                u16::from(port),
                setup::HUB_PORT_FEATURE_RESET,
                ep0_mps,
            ) {
                write_indent(&mut w, depth);
                let _ = writeln!(w, "[USB]   -> port {} RESET fail: {:?}", port, e);
                continue;
            }
            // USB 2.0 §7.1.7.5：TDRSTR ≥ 50ms；hub 完成 reset 后会自动置 C_PORT_RESET。
            dwc2_ep0::usb_post_hub_port_reset_delay();

            // ④ 读端口状态：必须 PORT_ENABLE=1，否则 reset 失败
            let after = match hub_port_status_w0(u32::from(hub_addr), u16::from(port), ep0_mps) {
                Ok(s) => s,
                Err(e) => {
                    write_indent(&mut w, depth);
                    let _ = writeln!(
                        w,
                        "[USB]   -> port {} after-reset GET_PORT_STATUS: {:?}",
                        port, e
                    );
                    continue;
                }
            };
            let _ = dwc2_ep0::hub_clear_port_feature(
                u32::from(hub_addr),
                u16::from(port),
                setup::HUB_PORT_FEATURE_C_RESET,
                ep0_mps,
            );
            let enabled = (after >> 1) & 1 != 0;
            let speed = port_speed_str(after);
            write_indent(&mut w, depth);
            let _ = writeln!(
                w,
                "[USB]   -> port {} after-reset wPortStatus={:#06x} ENABLED={} SPD={}",
                port, after, enabled, speed
            );
            if !enabled {
                continue;
            }

            // ⑤ 速度提示：HS hub 下若挂 FS/LS 设备需要 split transaction
            //    （HCSPLT 编程），当前 host 通道未实现，无法访问 EP0 → 跳过。
            if speed != "HS" {
                write_indent(&mut w, depth);
                let _ = writeln!(
                    w,
                    "[USB]   -> port {} 设备非 HS（{}），HS hub 下 FS/LS 设备需要 split transaction，当前驱动暂不支持，跳过此端口枚举",
                    port, speed
                );
                continue;
            }

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
        let (iface_num, bin_ep, bin_mps, bout_ep, bout_mps) =
            parse_msc_interface_endpoints(u32::from(fn_addr), ep0_mps).unwrap_or((0, 0, 0, 0, 0));
        if st.extras.msc.is_none() {
            st.extras.msc = Some(MscEnumerated {
                addr: fn_addr,
                ep0_mps,
                vid,
                pid,
                iface_num,
                bulk_in_ep: bin_ep,
                bulk_in_mps: bin_mps,
                bulk_out_ep: bout_ep,
                bulk_out_mps: bout_mps,
            });
        }
        write_indent(&mut w, depth);
        let _ = writeln!(
            w,
            "[USB]   -> Mass Storage candidate iface={} BulkIN=ep{}({}) BulkOUT=ep{}({})",
            iface_num, bin_ep, bin_mps, bout_ep, bout_mps
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
