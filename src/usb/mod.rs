//! USB 子系统：基于 Synopsys **DWC2** 的 **主机**（[`host`]）栈；类协议在 [`class`]。
//!
//! # 使用顺序（主机）
//!
//! 1. [`set_dwc2_base_virt`] 指向控制器 MMIO；启用 `cv182x-host` 时再 [`set_cv182x_phy_base_virt`]。
//!    若 VA≠PA，再注册 [`set_usb_dma_to_phys_fn`]。
//! 2. 板级实现 [`log`](https://docs.rs/log) 的 `Logger` trait（例如接到 `println!` / 串口）。
//! 3. 调 [`host::enumerate_root_port`] 或自行组合 `host::dwc2` + `host::topology`。
//!
//! # 公共子模块
//!
//! - [`error`]：[`error::UsbError`] / [`error::UsbResult`]。
//! - [`setup`]：标准 SETUP 字节数组；**类专用** SETUP 见 [`class::uvc`]、[`class::mass_storage`]。
//!
//! # 板级配置
//!
//! - [`set_dwc2_base_virt`] / [`dwc2_base_virt`]：DWC2 MMIO 虚拟基址。
//! - [`dwc2_regs`] / [`dwc2_channel`]：虚拟基址 → `Dwc2Regs` / `Dwc2HostChannel` 视图。
//! - [`set_cv182x_phy_base_virt`] / [`cv182x_phy_base_virt`]（`cv182x-host`）：CV182x USB2 PHY MMIO。
//! - [`cv182x_phy_regs`]（`cv182x-host`）：PHY 寄存器视图。
//! - [`set_usb_dma_to_phys_fn`] / [`usb_dma_phys_for`]：EP0 `HCDMA` 用的 VA→PA。
//!
//! DMA 与 CPU 视图一致性由 [`crate::utils::cache`] 的 clean / invalidate 辅助完成。

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::usb::host::dwc2::regs::{Dwc2HostChannel, Dwc2Regs, DWC2_MAX_HOST_CHANNELS};
#[cfg(feature = "cv182x-host")]
use crate::usb::host::dwc2::regs::Cv182xUsb2Phy;

pub mod error;
pub mod setup;

pub mod host;
pub mod class;

pub use error::{UsbError, UsbResult};

// ---------------------------------------------------------------------------
// 板级 MMIO 基址（首次访问 USB 前一次性写入；多核下应在单核初始化阶段完成）
// ---------------------------------------------------------------------------

static DWC2_BASE_VIRT: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "cv182x-host")]
static CV182X_PHY_BASE_VIRT: AtomicUsize = AtomicUsize::new(0);

/// 设置 DWC2 控制器的 MMIO **虚拟**基址（通常为 `phys_to_virt([`crate::soc::DWC2_BASE`])`）。
///
/// # 参数
/// - `addr`：控制器寄存器块起始虚拟地址；传 **0** 表示未初始化，后续 USB API 会失败。
#[inline]
pub fn set_dwc2_base_virt(addr: usize) {
    DWC2_BASE_VIRT.store(addr, Ordering::SeqCst);
}

/// 返回当前设置的 DWC2 MMIO 虚拟基址；未调用 [`set_dwc2_base_virt`] 时为 **0**。
#[inline]
pub fn dwc2_base_virt() -> usize {
    DWC2_BASE_VIRT.load(Ordering::SeqCst)
}

/// 设置 CV182x 片内 USB2 PHY 的 MMIO **虚拟**基址（通常为
/// `phys_to_virt([`crate::soc::CV182X_USB2_PHY_BASE`])`）。
#[cfg(feature = "cv182x-host")]
#[inline]
pub fn set_cv182x_phy_base_virt(addr: usize) {
    CV182X_PHY_BASE_VIRT.store(addr, Ordering::SeqCst);
}

/// 返回当前设置的 CV182x USB2 PHY MMIO 虚拟基址；未调用 [`set_cv182x_phy_base_virt`] 时为 **0**。
#[cfg(feature = "cv182x-host")]
#[inline]
pub fn cv182x_phy_base_virt() -> usize {
    CV182X_PHY_BASE_VIRT.load(Ordering::SeqCst)
}

// DWC2 寄存器一律走 [`host::dwc2::regs`] 的 `tock-registers` 访问器。

/// 取 DWC2 全局寄存器视图。
///
/// # Panics
/// 未调用 [`set_dwc2_base_virt`]（或基址为 0）时 panic。
#[inline]
pub fn dwc2_regs() -> &'static Dwc2Regs {
    let base = dwc2_base_virt();
    assert!(base != 0, "DWC2 base not set (call set_dwc2_base_virt)");
    unsafe { &*(base as *const Dwc2Regs) }
}

/// 取第 `ch` 号主机通道寄存器块。
///
/// # 参数
/// - `ch`：主机通道索引；本栈约定 **0** 为 EP0 控制、**5** 为 Bulk/Isoch。
///
/// # Panics
/// 基址未设置或 `ch` 超出 IP 支持数量时 panic。
#[inline]
pub fn dwc2_channel(ch: u32) -> &'static Dwc2HostChannel {
    let idx = ch as usize;
    assert!(idx < DWC2_MAX_HOST_CHANNELS, "invalid DWC2 host channel index");
    &dwc2_regs().hc[idx]
}

/// 取 CV182x 片内 USB2 PHY 寄存器视图。
///
/// # 返回值
/// 未设置 MMIO 基址（或为 0）时返回 `None`。
#[cfg(feature = "cv182x-host")]
#[inline]
pub fn cv182x_phy_regs() -> Option<&'static Cv182xUsb2Phy> {
    let base = cv182x_phy_base_virt();
    if base == 0 {
        return None;
    }
    Some(unsafe { &*(base as *const Cv182xUsb2Phy) })
}

// ---------------------------------------------------------------------------
// DMA 地址转换
// ---------------------------------------------------------------------------

/// 将 EP0 / 通道 DMA 缓冲区的 **虚拟地址** 转为写入 `HCDMA` 的地址（通常为物理地址）。
pub type UsbDmaToPhysFn = fn(*const u8) -> u32;

static USB_DMA_TO_PHYS: AtomicUsize = AtomicUsize::new(0);

/// 注册 `HCDMA` 用的 **虚拟地址 → DMA 总线地址** 转换函数。
///
/// # 参数
/// - `f`：`Some(fn)` 时，主机通道写 `HCDMA` 前会对缓冲区指针调用 `fn`；`None` 时退化为
///   `ptr as u32`（仅 **恒等映射** 或 VA 等于 PA 时安全）。
pub fn set_usb_dma_to_phys_fn(f: Option<UsbDmaToPhysFn>) {
    let bits = f.map(|p| p as usize).unwrap_or(0);
    USB_DMA_TO_PHYS.store(bits, Ordering::SeqCst);
}

/// 将 CPU 可见缓冲区指针转为写入 `HCDMA` 的地址。
///
/// # 参数
/// - `ptr`：DMA 缓冲区内某字节的虚拟地址（或恒等映射下的物理地址）。
#[inline]
pub fn usb_dma_phys_for(ptr: *const u8) -> u32 {
    let bits = USB_DMA_TO_PHYS.load(Ordering::SeqCst);
    if bits == 0 {
        ptr as usize as u32
    } else {
        let fp: UsbDmaToPhysFn =
            unsafe { core::mem::transmute::<usize, UsbDmaToPhysFn>(bits) };
        fp(ptr)
    }
}
