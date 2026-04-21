//! USB 主机枚举入口：初始化 DWC2 后委托 [`super::topology`] 做 Hub 检测与递归端口遍历。

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2;
use crate::usb::host::topology::{self, TopologyScanExtras};

/// 复位 DWC2、上电根端口，**递归枚举** USB 拓扑（含 QEMU 虚拟 Hub），串口打印各设备；
/// 返回 Mass Storage 设备的 `(VID, PID, bMaxPacketSize0, dev_addr)`。
///
/// 直连 MSC 时设备通常为地址 **1**；经 QEMU `usb-hub` 时 Hub 多为 **1**、MSC 多为 **2**（以实际打印为准）。
pub fn enumerate_root_port() -> UsbResult<(u16, u16, u32, u32)> {
    dwc2::dwc2_host_init()?;
    check_root_device_connected()?;
    dwc2::dwc2_host_root_bus_reset_pulse()?;
    topology::enumerate_bus_print_tree()
}

/// 初始化主机端口并只做拓扑扫描（无 MSC 要求）；附带 UVC 等设备线索。
pub fn enumerate_topology_only() -> UsbResult<TopologyScanExtras> {
    dwc2::dwc2_host_init()?;
    check_root_device_connected()?;
    let hprt0 = unsafe { dwc2::dwc2_hprt0_read() };
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG pre-reset HPRT0={:#010x} CONNSTS={} ENABLE={} SPD={} (0=HS 1=FS 2=LS)",
        hprt0,
        hprt0 & 1,
        (hprt0 >> 2) & 1,
        dwc2::hprt_speed_bits(hprt0),
    ));
    dwc2::debug_dump_root_port_hw("pre-reset");
    dwc2::dwc2_host_root_bus_reset_pulse()?;
    let hprt = unsafe { dwc2::dwc2_hprt0_read() };
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG post-reset HPRT0={:#010x} CONNSTS={} ENABLE={} SPD={} (0=HS 1=FS 2=LS)",
        hprt,
        hprt & 1,
        (hprt >> 2) & 1,
        dwc2::hprt_speed_bits(hprt),
    ));
    dwc2::debug_dump_root_port_hw("post-reset");
    topology::enumerate_bus_print_tree_only()
}

fn check_root_device_connected() -> UsbResult<()> {
    const SPIN_PER_TRY: u32 = 256;
    /// 轮询次数（粗粒度）；慢速 Hub/上电后可等到 CONNSTS。
    const TRIES: u32 = 400_000;

    for t in 0..TRIES {
        let hprt = unsafe { dwc2::dwc2_hprt0_read() };
        if dwc2::hprt_connsts(hprt) {
            if t > 0 {
                crate::usb::log::usb_log_fmt(format_args!(
                    "USB-DBG root connect after {} polls HPRT0={:#010x}",
                    t, hprt
                ));
            }
            return Ok(());
        }
        for _ in 0..SPIN_PER_TRY {
            core::hint::spin_loop();
        }
    }

    let hprt = unsafe { dwc2::dwc2_hprt0_read() };
    crate::usb::log::usb_log_fmt(format_args!(
        "USB-DBG no root connect: HPRT0={:#010x} PWR={} CONNSTS={} LNSTS={}",
        hprt,
        dwc2::hprt_pwr(hprt),
        dwc2::hprt_connsts(hprt),
        dwc2::hprt_lnsts(hprt),
    ));
    dwc2::debug_dump_root_port_hw("no root connect");
    Err(UsbError::Hardware(
        "HPRT0 CONNSTS=0: no device on root port (enable VBUS e.g. GPIOB6 / cable / PHY)",
    ))
}
