//! MMIO 工具：保留 `read32/write32` 仅供 CV182x PHY 离散偏移转储使用；
//! DWC2 自身寄存器一律走 [`super::regs`] 的 `tock-registers` 访问器（[`dwc2_regs`] / [`dwc2_channel`]）。

use core::ptr::{read_volatile, write_volatile};

use crate::usb::platform;
use super::regs::{Cv182xUsb2Phy, Dwc2HostChannel, Dwc2Regs, DWC2_MAX_HOST_CHANNELS};

/// 读取 32-bit MMIO（仅 PHY/调试转储用，DWC2 主寄存器请用 [`dwc2_regs`]）。
#[inline(always)]
pub unsafe fn read32(addr: usize) -> u32 {
    unsafe { read_volatile(addr as *const u32) }
}

/// 写 32-bit MMIO（仅 PHY/调试转储用，DWC2 主寄存器请用 [`dwc2_regs`]）。
#[inline(always)]
pub unsafe fn write32(addr: usize, val: u32) {
    unsafe { write_volatile(addr as *mut u32, val) }
}

#[allow(dead_code)]
#[inline(always)]
pub unsafe fn modify32(addr: usize, mask: u32, bits: u32) {
    let v = unsafe { read32(addr) };
    unsafe { write32(addr, (v & !mask) | (bits & mask)) }
}

/// 取 DWC2 全局寄存器视图（基址未设置时返回 `None`）。
#[inline]
pub fn dwc2_regs() -> Option<&'static Dwc2Regs> {
    let base = platform::dwc2_base_virt();
    if base == 0 {
        return None;
    }
    Some(unsafe { &*(base as *const Dwc2Regs) })
}

/// 取第 `ch` 号主机通道寄存器（`ch >= 16` 或基址未设置时返回 `None`）。
#[inline]
pub fn dwc2_channel(ch: u32) -> Option<&'static Dwc2HostChannel> {
    let regs = dwc2_regs()?;
    let idx = ch as usize;
    if idx >= DWC2_MAX_HOST_CHANNELS {
        return None;
    }
    Some(&regs.hc[idx])
}

/// 取 CV182x 片内 USB2 PHY 视图（仅在启用 `cv182x-host` 时有意义）。
#[cfg(feature = "cv182x-host")]
#[inline]
pub fn cv182x_phy_regs() -> &'static Cv182xUsb2Phy {
    const CV182X_USB2_PHY_MMIO: usize = 0x0300_6000;
    unsafe { &*(CV182X_USB2_PHY_MMIO as *const Cv182xUsb2Phy) }
}

#[cfg(not(feature = "cv182x-host"))]
#[allow(dead_code)]
#[inline]
pub(crate) fn _phantom_phy() -> *const Cv182xUsb2Phy {
    core::ptr::null()
}
