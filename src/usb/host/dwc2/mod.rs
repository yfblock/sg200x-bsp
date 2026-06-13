//! Synopsys DesignWare USB 2.0 OTG (DWC2) **主机**侧访问层。
//!
//! 约定：**通道 0** 专用于 EP0 控制传输；**通道 5** 专用于 Bulk / Isoch。
//!
//! 子模块：
//! - [`regs`]：`tock-registers` 寄存器/位域/主机通道结构。
//! - [`controller`]：上电、软复位、Force Host、FIFO、`HPRT0` 根口操作。
//! - [`isr`]：PLIC 中断处理、`GINTMSK`/`HAINTMSK` 配置。
//! - [`dma`]：共用 DMA 窗口与偏移常量。
//! - [`channel`]：主机通道调度原语。
//! - [`control`]：EP0 控制传输与标准枚举便捷函数。
//! - [`bulk`]：Bulk IN/OUT。
//! - [`isoch`]：Isochronous IN。

pub mod bulk;
pub mod channel;
pub mod control;
pub mod controller;
pub mod dma;
pub mod isoch;
pub mod isr;
pub mod regs;

pub use controller::{dwc2_host_init, dwc2_host_root_bus_reset_pulse, dwc2_probe};

pub use isr::{DWC2_IRQ_NUM, dwc2_interrupt_handler};

pub use bulk::{bulk_in, bulk_out};
pub use channel::{PID_DATA0, PID_DATA1, PID_DATA2, PID_SETUP};
pub use control::{
    ep0_control_read, ep0_control_read_one_byte, ep0_control_write, ep0_control_write_no_data,
    get_device_vid_pid_default_addr, set_configuration, set_usb_address,
    usb_post_set_address_delay,
};
pub use dma::{
    DMA_OFF_CBW, DMA_OFF_CSW, DMA_OFF_SECTOR, DMA_OFF_UVC_BULK, MSC_SECTOR_DMA_CAP,
    UVC_BULK_DMA_CAP, dma_copy_out, dma_rx_slice, dma_write_at,
};
pub(crate) use dma::dma_append_unchecked;
pub use isoch::{current_uframe, isoch_in, isoch_in_uframe, isoch_in_uframe_batch};
