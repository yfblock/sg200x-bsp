//! USB 子系统：基于 Synopsys **DWC2** 的 **主机**（[`host`]）栈；启用 feature `device-mode`
//! 时另有 **设备** 子模块 `device`；类协议在 [`class`]。
//!
//! # 使用顺序（主机）
//!
//! 1. [`set_dwc2_base_virt`] 指向控制器 MMIO；若 VA≠PA，再注册 [`set_usb_dma_to_phys_fn`]。
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
//! - [`set_usb_dma_to_phys_fn`] / [`usb_dma_phys_for`]：EP0 `HCDMA` 用的 VA→PA。
//!
//! DMA 与 CPU 视图一致性由 [`crate::utils::cache`] 的 clean / invalidate 辅助完成。

use core::sync::atomic::{AtomicUsize, Ordering};

pub mod error;
pub mod setup;

pub mod host;
pub mod class;
#[cfg(feature = "device-mode")]
pub mod device;

pub use error::{UsbError, UsbResult};

// 由板级在首次访问 USB 前一次性写入；多核下应在单核初始化阶段完成。
static DWC2_BASE_VIRT: AtomicUsize = AtomicUsize::new(0);

/// 将 EP0 / 通道 DMA 缓冲区的 **虚拟地址** 转为写入 `HCDMA` 的地址（通常为物理地址）。
pub type UsbDmaToPhysFn = fn(*const u8) -> u32;

static USB_DMA_TO_PHYS: AtomicUsize = AtomicUsize::new(0);

/// 设置 DWC2 控制器的 MMIO **虚拟**基址（通常为 `phys_to_virt([`crate::soc::DWC2_BASE`])`）。
///
/// # 参数
/// - `addr`：控制器寄存器块起始虚拟地址；传 **0** 表示未初始化，后续 USB API 会失败。
#[inline]
pub fn set_dwc2_base_virt(addr: usize) {
    DWC2_BASE_VIRT.store(addr, Ordering::SeqCst);
}

/// 返回当前设置的 MMIO 虚拟基址；未调用 [`set_dwc2_base_virt`] 时为 **0**。
#[inline]
pub fn dwc2_base_virt() -> usize {
    DWC2_BASE_VIRT.load(Ordering::SeqCst)
}

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
