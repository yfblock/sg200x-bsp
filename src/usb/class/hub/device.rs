//! 已寻址 Hub 设备的 EP0 控制操作与端口状态解析。

use tock_registers::LocalRegisterCopy;

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::{self, ep0_control_write_no_data};
use crate::utils::spin_delay;

use super::hub_log;
use super::setup::{
    self, HubPortStatus, HUB_PORT_FEATURE_C_CONNECTION, HUB_PORT_FEATURE_C_RESET,
    HUB_PORT_FEATURE_POWER, HUB_PORT_FEATURE_RESET,
};

const MAX_HUB_PORTS: u8 = 16;

/// Hub 描述符关键字段。
#[derive(Clone, Copy, Debug)]
pub struct HubDescriptor {
    /// 下游端口数 `bNbrPorts`。
    pub nports: u8,
    /// `bPwrOn2PwrGood` × 2ms：端口上电后 VBUS 稳定时间。
    pub pwr_on_2_pwr_good_ms: u32,
}

/// 已分配 USB 地址并完成 `SET_CONFIGURATION` 的 Hub。
#[derive(Clone, Copy, Debug)]
pub struct HubDevice {
    pub addr: u8,
    pub ep0_mps: u32,
}

impl HubDevice {
    #[inline]
    pub const fn new(addr: u8, ep0_mps: u32) -> Self {
        Self { addr, ep0_mps }
    }

    #[inline]
    fn dev(&self) -> u32 {
        u32::from(self.addr)
    }

    /// 读取 Hub 类描述符，解析端口数与上电稳定时间。
    pub fn read_descriptor(&self) -> UsbResult<HubDescriptor> {
        let mut buf = [0u8; 64];
        dwc2::ep0_control_read(
            self.dev(),
            setup::get_descriptor_hub(64),
            self.ep0_mps,
            &mut buf,
        )?;
        if buf[0] < 7 || buf[1] != setup::USB_DT_HUB {
            return Err(UsbError::Protocol("invalid hub descriptor"));
        }
        Ok(HubDescriptor {
            nports: buf[2].min(MAX_HUB_PORTS),
            pwr_on_2_pwr_good_ms: u32::from(buf[5]).saturating_mul(2),
        })
    }

    /// `GET_PORT_STATUS`：返回 `(wPortStatus, wPortChange)`。
    pub fn get_port_status(&self, port: u16) -> UsbResult<(u16, u16)> {
        let mut buf = [0u8; 4];
        dwc2::ep0_control_read(
            self.dev(),
            setup::hub_get_port_status(port),
            self.ep0_mps,
            &mut buf,
        )?;
        Ok(setup::parse_hub_port_status(&buf))
    }

    /// `SET_PORT_FEATURE`（无数据阶段）。
    pub fn set_port_feature(&self, port: u16, feature: u16) -> UsbResult<()> {
        ep0_control_write_no_data(
            self.dev(),
            setup::hub_set_port_feature(port, feature),
            self.ep0_mps,
        )
    }

    /// `CLEAR_PORT_FEATURE`（无数据阶段）。
    pub fn clear_port_feature(&self, port: u16, feature: u16) -> UsbResult<()> {
        ep0_control_write_no_data(
            self.dev(),
            setup::hub_clear_port_feature(port, feature),
            self.ep0_mps,
        )
    }

    /// 给所有下游端口上电并等待 VBUS 稳定。
    pub fn power_all_ports(&self, nports: u8, pwr_good_ms: u32) {
        for port in 1..=nports {
            if let Err(e) = self.set_port_feature(u16::from(port), HUB_PORT_FEATURE_POWER) {
                log::info!(
                    target: "sg200x_bsp::usb::hub",
                    "[USB]   -> port {} POWER fail: {:?}",
                    port,
                    e
                );
            }
        }
        spin_delay_ms(pwr_good_ms.saturating_add(100));
    }

    /// 对单端口执行 reset，返回 reset 后的 `wPortStatus`；失败返回 `None`。
    pub fn reset_port(&self, depth: u8, port: u8) -> Option<u16> {
        let _ = self.clear_port_feature(u16::from(port), HUB_PORT_FEATURE_C_CONNECTION);
        if let Err(e) = self.set_port_feature(u16::from(port), HUB_PORT_FEATURE_RESET) {
            hub_log!(depth, "[USB]   -> port {} RESET fail: {:?}", port, e);
            return None;
        }
        post_port_reset_delay();

        let (after, _) = match self.get_port_status(u16::from(port)) {
            Ok(s) => s,
            Err(e) => {
                hub_log!(
                    depth,
                    "[USB]   -> port {} after-reset GET_PORT_STATUS: {:?}",
                    port,
                    e
                );
                return None;
            }
        };
        let _ = self.clear_port_feature(u16::from(port), HUB_PORT_FEATURE_C_RESET);
        Some(after)
    }
}

/// `wPortStatus` 是否报告端口已连接设备。
#[inline]
pub fn port_connected(status: u16) -> bool {
    LocalRegisterCopy::<u16, HubPortStatus::Register>::new(status)
        .is_set(HubPortStatus::CONNECTION)
}

/// `wPortStatus` 是否报告端口已 enable。
#[inline]
pub fn port_enabled(status: u16) -> bool {
    LocalRegisterCopy::<u16, HubPortStatus::Register>::new(status)
        .is_set(HubPortStatus::PORT_ENABLE)
}

/// USB 2.0 hub 端口速度位（`wPortStatus[10:9]`）→ 文字描述。
pub fn port_speed_str(status: u16) -> &'static str {
    let s = LocalRegisterCopy::<u16, HubPortStatus::Register>::new(status);
    if s.is_set(HubPortStatus::HIGH_SPEED) {
        "HS"
    } else if s.is_set(HubPortStatus::LOW_SPEED) {
        "LS"
    } else {
        "FS"
    }
}

/// Hub 下游端口 `PORT_RESET` 后给设备恢复时间（粗粒度忙等）。
fn post_port_reset_delay() {
    spin_delay(30_000_000);
}

/// 粗粒度毫秒延迟（hub 端口上电 / reset 等待）。
fn spin_delay_ms(ms: u32) {
    let cycles = ms.saturating_mul(250_000);
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

