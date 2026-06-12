//! Hub 下游端口枚举：上电、reset，并对 HS 子设备回调拓扑扫描。

use crate::usb::error::UsbResult;

use super::device::{port_connected, port_enabled, port_speed_str, HubDevice};
use super::hub_log;

/// 枚举 Hub 所有下游端口：读描述符、端口上电、逐端口 reset。
///
/// 对每个 **已 enable 的 HS** 下游设备，调用 `visit_child(depth, parent_hub, port)`；
/// 调用方应在子设备仍处地址 **0** 时继续标准枚举（`GET_DESCRIPTOR` → `SET_ADDRESS` …）。
pub fn enumerate_downstream_ports<F>(
    depth: u8,
    hub: HubDevice,
    mut visit_child: F,
) -> UsbResult<()>
where
    F: FnMut(u8, u8, u8) -> UsbResult<()>,
{
    hub_log!(
        depth,
        "[USB]   -> Hub enumerated addr={} ep0_mps={}",
        hub.addr,
        hub.ep0_mps
    );

    let desc = hub.read_descriptor()?;
    let pwr_good_ms = desc.pwr_on_2_pwr_good_ms.max(20);
    hub_log!(
        depth,
        "[USB]   -> Hub descriptor: {} downstream port(s), PwrOn2PwrGood={} ms",
        desc.nports,
        pwr_good_ms
    );

    hub.power_all_ports(desc.nports, pwr_good_ms);

    for port in 1..=desc.nports {
        visit_hub_port(depth, hub, port, &mut visit_child)?;
    }
    Ok(())
}

fn visit_hub_port<F>(depth: u8, hub: HubDevice, port: u8, visit_child: &mut F) -> UsbResult<()>
where
    F: FnMut(u8, u8, u8) -> UsbResult<()>,
{
    let (status, _) = match hub.get_port_status(u16::from(port)) {
        Ok(s) => s,
        Err(e) => {
            hub_log!(depth, "[USB]   -> port {} GET_PORT_STATUS: {:?}", port, e);
            return Ok(());
        }
    };

    let conn = port_connected(status);
    hub_log!(
        depth,
        "[USB]   -> port {} wPortStatus={:#06x} {}",
        port,
        status,
        if conn { "CONNECTED" } else { "empty" }
    );
    if !conn {
        return Ok(());
    }

    let Some(after) = hub.reset_port(depth, port) else {
        return Ok(());
    };

    let enabled = port_enabled(after);
    let speed = port_speed_str(after);
    hub_log!(
        depth,
        "[USB]   -> port {} after-reset wPortStatus={:#06x} ENABLED={} SPD={}",
        port,
        after,
        enabled,
        speed
    );
    if !enabled {
        return Ok(());
    }
    if speed != "HS" {
        hub_log!(
            depth,
            "[USB]   -> port {} 设备非 HS（{}），HS hub 下 FS/LS 设备需要 split transaction，当前驱动暂不支持，跳过此端口枚举",
            port,
            speed
        );
        return Ok(());
    }

    visit_child(depth.saturating_add(1), hub.addr, port)
}
