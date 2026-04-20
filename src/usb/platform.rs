//! DWC2 寄存器窗口的 **虚拟基址**（由板级在访问 USB 前调用 [`set_dwc2_base_virt`] 设置）。
//!
//! EP0 `HCDMA` 需 **DMA 可见地址**：若 MMU 下 VA≠PA，由板级注册 [`set_usb_dma_to_phys_fn`]。

use core::sync::atomic::{AtomicUsize, Ordering};

static DWC2_BASE_VIRT: AtomicUsize = AtomicUsize::new(0);

/// 将 EP0 缓冲区 **虚拟地址** 转为写入 `HCDMA` 的地址（通常为物理地址）。
pub type UsbDmaToPhysFn = fn(*const u8) -> u32;

static USB_DMA_TO_PHYS: AtomicUsize = AtomicUsize::new(0);

/// 设置 DWC2 控制器的 MMIO 虚拟基址（通常为 `phys_to_virt(USB_PADDR)`）。
#[inline]
pub fn set_dwc2_base_virt(addr: usize) {
    DWC2_BASE_VIRT.store(addr, Ordering::SeqCst);
}

/// 当前设置的虚拟基址；未设置时为 0。
#[inline]
pub fn dwc2_base_virt() -> usize {
    DWC2_BASE_VIRT.load(Ordering::SeqCst)
}

/// 注册 `HCDMA` 地址转换；`None` 表示直接使用 `va as u32`（仅当 VA=PA 时安全）。
pub fn set_usb_dma_to_phys_fn(f: Option<UsbDmaToPhysFn>) {
    let bits = f.map(|p| p as usize).unwrap_or(0);
    USB_DMA_TO_PHYS.store(bits, Ordering::SeqCst);
}

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
