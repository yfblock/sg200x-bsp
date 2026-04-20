//! Synopsys DesignWare USB 2.0 OTG (DWC2) host controller.
//!
//! 子模块组织：
//! - [`regs`]：`tock-registers` 的全部寄存器/位域/通道结构定义。
//! - [`mmio`]：基址解析、寄存器视图工厂、CV182x PHY 的离散偏移读写。
//! - [`controller`]：控制器 bring-up（软复位、Force Host、FIFO、HCFG、根口上电）。
//! - [`ep0`]：通道 0 控制传输 + 通道 1 Bulk/Isoch 调度。

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
    ep0_control_write_no_data, get_device_vid_pid_default_addr, hub_set_port_feature, isoch_in,
    isoch_in_uframe, set_configuration, set_usb_address, usb_post_hub_port_reset_delay,
    usb_post_set_address_delay, DMA_OFF_CBW, DMA_OFF_CSW, DMA_OFF_SECTOR, DMA_OFF_UVC_BULK,
    PID_DATA0, PID_DATA1, PID_DATA2, PID_SETUP, UVC_BULK_DMA_CAP,
};
