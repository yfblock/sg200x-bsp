//! Synopsys DesignWare USB 2.0 OTG (DWC2) **主机**侧访问层。
//!
//! 约定：**通道 0** 专用于 EP0 控制传输；**通道 1** 专用于 Bulk / Isoch（与部分 IP 在
//! 单通道上复用控制+批量时的异常行为隔离）。
//!
//! 子模块：
//! - [`regs`]：`tock-registers` 寄存器/位域/主机通道结构。
//! - [`mmio`]：虚拟基址 → `Dwc2Regs` / `Dwc2HostChannel` 视图；CV182x PHY 离散读。
//! - [`controller`]：上电、软复位、Force Host、FIFO、`HPRT0` 根口操作。
//! - [`ep0`]：SETUP/Data/Status 调度、MSC/UVC 共用 DMA 窗。

pub mod regs;
pub mod mmio;
pub mod controller;
pub mod ep0;

pub use controller::{
    debug_dump_root_port_hw, dwc2_host_init, dwc2_host_root_bus_reset_pulse, dwc2_hprt0_read,
    dwc2_probe, hprt_connsts, hprt_enabled, hprt_lnsts, hprt_pwr, hprt_speed_bits,
    suggested_bulk_mps,
};

pub use ep0::{
    bulk_in, bulk_out, current_uframe, debug_log_ep0_dma_info, dma_copy_out, dma_rx_slice,
    dma_write_at, ep0_control_read, ep0_control_read_one_byte, ep0_control_write,
    ep0_control_write_no_data, get_device_vid_pid_default_addr, hub_clear_port_feature,
    hub_set_port_feature, isoch_in,
    isoch_in_uframe, set_configuration, set_usb_address, usb_post_hub_port_reset_delay,
    usb_post_set_address_delay, DMA_OFF_CBW, DMA_OFF_CSW, DMA_OFF_SECTOR, DMA_OFF_UVC_BULK,
    MSC_SECTOR_DMA_CAP, PID_DATA0, PID_DATA1, PID_DATA2, PID_SETUP, UVC_BULK_DMA_CAP,
};
