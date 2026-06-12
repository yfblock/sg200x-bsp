//! USB 主机枚举入口：初始化 DWC2 后委托 [`super::topology`] 做 Hub 检测与递归端口遍历。

use tock_registers::interfaces::Readable;

use crate::usb::dwc2_regs;
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2;
use crate::usb::host::dwc2::regs::HPRT0;
use crate::usb::host::topology::{self, TopologyScanExtras};

/// 复位 DWC2、上电根端口，**递归枚举** USB 拓扑（含 QEMU 虚拟 Hub），并打印各设备。
///
/// # 返回值
/// 成功时返回 `(VID, PID, bMaxPacketSize0, dev_addr)`，均为扫描到的**首个** Mass Storage
/// 设备信息。直连 MSC 时地址多为 **1**；经 QEMU `usb-hub` 时 Hub 多为 **1**、MSC 多为 **2**（以日志为准）。
pub fn enumerate_root_port() -> UsbResult<(u16, u16, u32, u32)> {
    dwc2::dwc2_host_init()?;
    check_root_device_connected()?;
    dwc2::dwc2_host_root_bus_reset_pulse()?;
    topology::enumerate_bus_print_tree()
}

/// 初始化主机并只做拓扑扫描（**不要求**总线上存在 Mass Storage）。
///
/// # 返回值
/// - [`TopologyScanExtras`]：枚举过程中发现的 UVC / MSC 等设备线索（可能为 `None`）。
pub fn enumerate_topology_only() -> UsbResult<TopologyScanExtras> {
    dwc2::dwc2_host_init()?;
    check_root_device_connected()?;
    let pre = dwc2_regs().hprt0.extract();
    log::debug!(
        "USB-DBG pre-reset HPRT0={:#010x} CONNSTS={} ENABLE={} SPD={} (0=HS 1=FS 2=LS)",
        pre.get(),
        pre.is_set(HPRT0::CONNSTS),
        pre.is_set(HPRT0::ENA),
        pre.read(HPRT0::SPD),
    );
    dwc2::dwc2_host_root_bus_reset_pulse()?;
    let post = dwc2_regs().hprt0.extract();
    log::debug!(
        "USB-DBG post-reset HPRT0={:#010x} CONNSTS={} ENABLE={} SPD={} (0=HS 1=FS 2=LS)",
        post.get(),
        post.is_set(HPRT0::CONNSTS),
        post.is_set(HPRT0::ENA),
        post.read(HPRT0::SPD),
    );
    topology::enumerate_bus_print_tree_only()
}

/// 轮询 `HPRT0.CONNSTS`，直到根口报告已连接设备或超时。
///
/// 超时常见于未供电 / 无线缆 / PHY 未切到 host；日志中会打印 `HPRT0` 快照。
fn check_root_device_connected() -> UsbResult<()> {
    const SPIN_PER_TRY: u32 = 256;
    /// 轮询次数（粗粒度）；慢速 Hub/上电后可等到 CONNSTS。
    const TRIES: u32 = 400_000;

    for t in 0..TRIES {
        let hprt = dwc2_regs().hprt0.extract();
        if hprt.is_set(HPRT0::CONNSTS) {
            if t > 0 {
                log::debug!(
                    "USB-DBG root connect after {} polls HPRT0={:#010x}",
                    t,
                    hprt.get()
                );
            }
            return Ok(());
        }
        for _ in 0..SPIN_PER_TRY {
            core::hint::spin_loop();
        }
    }

    let hprt: tock_registers::LocalRegisterCopy<u32, HPRT0::Register> = dwc2_regs().hprt0.extract();
    log::debug!(
        "USB-DBG no root connect: HPRT0={:#010x} PWR={} CONNSTS={} LNSTS={}",
        hprt.get(),
        hprt.is_set(HPRT0::PWR),
        hprt.is_set(HPRT0::CONNSTS),
        hprt.read(HPRT0::LNSTS),
    );
    Err(UsbError::Hardware(
        "HPRT0 CONNSTS=0: no device on root port (enable VBUS e.g. GPIOB6 / cable / PHY)",
    ))
}
