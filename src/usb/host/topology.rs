//! USB 总线拓扑：识别 Hub / 功能设备，**递归**枚举总线并打印；Hub 端口操作见 [`crate::usb::class::hub`]。
//!
//! 与 [`super::enumerate`] 配合：在 `dwc2_host_init` 之后由 `enumerate_root_port()` 调用；
//! 返回 Mass Storage 设备四元组供后续 [`crate::usb::class::mass_storage`] 使用。

use crate::usb::UsbClass;
use crate::usb::class::hub::{self, HubDevice};
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2;
use crate::usb::setup;
use crate::utils::log_indent;

macro_rules! topo_log {
    ($depth:expr, $($tt:tt)*) => {
        ::log::info!(
            target: "sg200x_bsp::usb::topology",
            "{}{}",
            log_indent($depth),
            format_args!($($tt)*)
        )
    };
}

/// 常见 `qemu usb-storage`（亦见于枚举日志）。
const QEMU_USB_STORAGE_VID: u16 = 0x46f4;
const QEMU_USB_STORAGE_PID: u16 = 0x0001;

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
fn is_msc_candidate(iface_class: UsbClass, vid: u16, pid: u16) -> bool {
    iface_class == UsbClass::MassStorage
        || (vid == QEMU_USB_STORAGE_VID && pid == QEMU_USB_STORAGE_PID)
}

/// 读配置描述符前 64 字节，返回首个 **INTERFACE** 描述符的 `bInterfaceClass`（无则 [`UsbClass::DefinedAtInterface`]）。
fn first_interface_class(dev: u32, ep0_mps: u32) -> UsbResult<UsbClass> {
    let mut buf = [0u8; 64];
    dwc2::ep0_control_read(
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
            return Ok(UsbClass::from_raw(buf[i + 5]));
        }
        i = i.saturating_add(bl);
    }
    Ok(UsbClass::DefinedAtInterface)
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
    dwc2::ep0_control_read(
        dev,
        setup::get_descriptor_configuration(0, 9),
        ep0_mps,
        &mut hdr,
    )?;
    if hdr[1] != setup::USB_DT_CONFIGURATION {
        return Err(UsbError::Protocol("not a configuration descriptor"));
    }
    let total = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
    if !(9..=512).contains(&total) {
        return Err(UsbError::Protocol("bad cfg total length"));
    }

    let mut buf = [0u8; 512];
    dwc2::ep0_control_read(
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
            in_msc_iface = UsbClass::from_raw(cls) == UsbClass::MassStorage && alt == 0;
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

/// 打印地址 0 上当前设备的 VID/PID/类信息。
fn log_device_at_addr0(
    depth: u8,
    parent_hub: u8,
    port_on_hub: u8,
    vid: u16,
    pid: u16,
    dev_class: UsbClass,
) {
    if parent_hub == 0 && port_on_hub == 0 {
        topo_log!(
            depth,
            "[USB] root dev@0 VID={:04x} PID={:04x} dev_class={:02x}",
            vid,
            pid,
            dev_class.as_raw()
        );
    } else {
        topo_log!(
            depth,
            "[USB] dev@0 (hub {} port {}) VID={:04x} PID={:04x} dev_class={:02x}",
            parent_hub,
            port_on_hub,
            vid,
            pid,
            dev_class.as_raw()
        );
    }
}

/// 分配地址并 `SET_CONFIGURATION(1)`。
fn assign_usb_address(ep0_mps: u32, st: &mut ScanState) -> UsbResult<u8> {
    let addr = st.take_addr()?;
    dwc2::set_usb_address(addr, ep0_mps)?;
    dwc2::usb_post_set_address_delay();
    dwc2::set_configuration(u32::from(addr), 1, ep0_mps)?;
    Ok(addr)
}

/// 记录 MSC / UVC 功能设备候选到 [`TopologyScanExtras`]。
fn register_function_device(
    depth: u8,
    fn_addr: u8,
    ep0_mps: u32,
    vid: u16,
    pid: u16,
    st: &mut ScanState,
) -> UsbResult<()> {
    let iface_class =
        first_interface_class(u32::from(fn_addr), ep0_mps).unwrap_or(UsbClass::DefinedAtInterface);
    topo_log!(
        depth,
        "[USB]   -> function addr={} ep0_mps={} first_ifc_class={:02x}",
        fn_addr,
        ep0_mps,
        iface_class.as_raw()
    );

    if is_msc_candidate(iface_class, vid, pid) {
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
        topo_log!(
            depth,
            "[USB]   -> Mass Storage candidate iface={} BulkIN=ep{}({}) BulkOUT=ep{}({})",
            iface_num,
            bin_ep,
            bin_mps,
            bout_ep,
            bout_mps
        );
    }

    if iface_class == UsbClass::Video && st.extras.uvc.is_none() {
        st.extras.uvc = Some(UvcEnumerated {
            addr: fn_addr,
            ep0_mps,
            vid,
            pid,
        });
        topo_log!(
            depth,
            "[USB]   -> Video class device (UVC candidate) addr={}",
            fn_addr
        );
    }

    Ok(())
}

/// 在默认地址 **0** 上枚举一台设备：`SET_ADDRESS` → `SET_CONFIGURATION` → 打印信息。
///
/// - 若为 **Hub**：分配地址、读 Hub 描述符、给各端口上电、`PORT_RESET` 后递归
///   [`visit_default_depth`]（仅支持下游 **HS** 设备，FS/LS 会跳过并打日志）。
/// - 若为 **功能设备**：把 MSC / UVC 候选写入 [`TopologyScanExtras`]。
///
/// `parent_hub==0` 且 `port_on_hub==0` 表示根口直连。
fn visit_default_depth(
    depth: u8,
    parent_hub: u8,
    port_on_hub: u8,
    st: &mut ScanState,
) -> UsbResult<()> {
    let (vid, pid, ep0_mps, dev_class) = dwc2::get_device_vid_pid_default_addr()?;
    log_device_at_addr0(depth, parent_hub, port_on_hub, vid, pid, dev_class);

    if hub::is_hub_device(dev_class, vid, pid) {
        let hub_addr = assign_usb_address(ep0_mps, st)?;
        let hub_dev = HubDevice::new(hub_addr, ep0_mps);
        return hub::enumerate_downstream_ports(depth, hub_dev, |child_depth, parent, port| {
            visit_default_depth(child_depth, parent, port, st)
        });
    }

    let fn_addr = assign_usb_address(ep0_mps, st)?;
    register_function_device(depth, fn_addr, ep0_mps, vid, pid, st)
}

/// 打印总线拓扑并返回首个 Mass Storage 设备信息（与 [`crate::usb::host::enumerate::enumerate_root_port`] 返回类型一致）。
///
/// # 返回值
/// - `Ok((vid, pid, ep0_mps, dev_addr))`：`dev_addr` 为设备 USB 地址（7 位，数值形式）。
/// - `Err(Protocol(...))`：拓扑中未发现 MSC。
pub fn enumerate_bus_print_tree() -> UsbResult<(u16, u16, u32, u32)> {
    log::info!("[USB] topology: recursive hub scan (QEMU may insert virtual usb-hub on single root port)");

    let mut st = ScanState::new();
    let visit = visit_default_depth(0, 0, 0, &mut st);
    log::info!("[USB] topology: scan finished.");
    visit?;
    if !st.have_msc {
        return Err(UsbError::Protocol("no mass storage device found"));
    }
    Ok((st.msc_vid, st.msc_pid, st.msc_ep0_mps, st.msc_addr))
}

/// 仅递归枚举并打印拓扑，**不要求**连接 Mass Storage。
///
/// # 返回值
/// [`TopologyScanExtras`]：发现的 UVC / MSC 等候选（字段可能仍为 `None`）。
pub fn enumerate_bus_print_tree_only() -> UsbResult<TopologyScanExtras> {
    log::info!("[USB] topology: recursive hub scan (QEMU may insert virtual usb-hub on single root port)");

    let mut st = ScanState::new();
    let visit = visit_default_depth(0, 0, 0, &mut st);
    log::info!("[USB] topology: scan finished.");
    visit?;
    Ok(st.extras)
}
