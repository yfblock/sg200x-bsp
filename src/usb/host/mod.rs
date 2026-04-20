//! USB 主机栈：DWC2 控制器 + 拓扑枚举入口。
//!
//! 子模块组织：
//! - [`dwc2`]：Synopsys DWC2 控制器（寄存器、bring-up、EP0/Bulk/Isoch 通道）。
//! - [`enumerate`]：根口连接检查 + 委托 [`topology`] 做递归扫描。
//! - [`topology`]：Hub 描述符解析与端口递归枚举（标记 MSC / UVC 候选）。

pub mod dwc2;
pub mod enumerate;
pub mod topology;

pub use enumerate::{enumerate_root_port, enumerate_topology_only};
pub use topology::{
    enumerate_bus_print_tree, enumerate_bus_print_tree_only, TopologyScanExtras, UvcEnumerated,
};
