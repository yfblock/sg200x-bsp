//! USB Device (Slave / Peripheral) 模式：DWC2 ForceDevMode 路径下的最小 EP0
//! 状态机 + class 钩子。
//!
//! # 工作模型
//!
//! 本模块**轮询驱动**，没有挂中断。应用流程：
//!
//! 1. 板级 bring-up：`platform::set_dwc2_base_virt()` + 板上 PHY/clock 切到
//!    device 模式（VBUS 由 PC 提供，需关板上 VBUS_GPIO）。
//! 2. 调 [`controller::dwc2_device_init`]：复位、Force Device、DCFG=HS、FIFO
//!    划分、EP0 初始化（**保持 `SFTDISCON=1` 不上线**）。
//! 3. 注册一个实现 [`UsbDeviceClass`] 的 class 对象（如 [`class::cdc_acm`]）。
//! 4. 调 [`controller::dwc2_device_softconnect`] 上线 D+ 上拉，PC 看到 connect。
//! 5. 主循环里不断调 [`ep0::Ep0Service::service`]，它会：
//!    - 处理 GINTSTS 的 USBRST / ENUMDONE / SETUP 等事件
//!    - 接收 SETUP 包，分发给标准请求 handler 或 [`UsbDeviceClass::class_setup`]
//!    - SET_CONFIGURATION 完成后回调 [`UsbDeviceClass::on_configured`]
//!    - 然后调用 [`UsbDeviceClass::poll`] 让 class 推动数据 EP
//!
//! # 与 host 模式共存
//!
//! `device-mode` 与 host 路径在源码层共存编译，但**同一时刻只能有一个生效**——
//! 板级 bring-up 决定走哪条。host 端的 [`crate::usb::host`] 与本模块**互不调用**。
//!
//! # SG2002 / CV182x 板级注意事项
//!
//! - PHY ID toggle workaround：vendor host 路径要 device→host；device 路径要 host→device。
//! - VBUS：device 模式下板上的 VBUS_GPIO **必须关掉**，由 PC 通过 USB 线供电；
//!   不然会回灌，PC 拒绝枚举（甚至损坏 USB 端口）。
//! - clock：与 host 模式共用 `clk_axi4_usb / clk_125m_usb / clk_12m_usb`，必须
//!   把 `CLK_BYP_0` bit17/18 清零让其走 fpll，否则 HS chirp 失败。

pub mod controller;
pub mod desc;
pub mod ep0;

#[cfg(feature = "device-cdc-acm")]
pub mod class;

pub use controller::{
    dwc2_device_dump_status, dwc2_device_init, dwc2_device_set_speed_hint,
    dwc2_device_softconnect, dwc2_device_softdisconnect, DeviceSpeedHint,
};
pub use ep0::{Ep0Reply, Ep0Service, Setup, UsbSpeed};

/// 应用在 [`UsbDeviceClass::on_configured`] 中需要操控的几件事都封装在这。
///
/// 由 [`Ep0Service`] 在每次回调前注入，class 通过 `&Ep0Context` 拿到最新枚举速度，
/// 由此决定 bulk EP MPS（HS=512, FS=64）。
pub struct Ep0Context {
    /// 主机协商出的总线速度（HS / FS / LS）。
    pub speed: UsbSpeed,
}

/// EP0 标准请求 + class 请求的统一处理 trait。
///
/// 所有方法都在 main loop 的轮询线程内调用，不要做长阻塞操作；class 只需要把
/// 静态描述符 / 当前应该回的 IN 数据返回即可。
pub trait UsbDeviceClass {
    /// 18 字节 device descriptor。
    fn device_descriptor(&self) -> &'static [u8];

    /// 完整 configuration descriptor（含 interface + endpoint 描述符）。第二段
    /// `wTotalLength` 必须与 slice 长度一致。
    fn config_descriptor(&self) -> &'static [u8];

    /// String descriptor（idx=0 是 LANGID list，1.. 是字符串）。返回 `None`
    /// 时 EP0 会 STALL，PC 一般会忽略可选字符串。
    fn string_descriptor(&self, _idx: u8) -> Option<&'static [u8]> {
        None
    }

    /// 处理 class / vendor 请求（标准请求由 [`Ep0Service`] 自行处理）。
    /// `in_buf` 长度通常 ≥ 64，class 把 IN 数据写在头上并通过 [`Ep0Reply::Data`]
    /// 返回长度；OUT 类请求可返回 [`Ep0Reply::AcceptOut`]，数据收完后由 EP0 调
    /// 用 [`UsbDeviceClass::class_out_data`]。
    fn class_setup(
        &mut self,
        _setup: &Setup,
        _in_buf: &mut [u8],
    ) -> Ep0Reply {
        Ep0Reply::Stall
    }

    /// `class_setup` 返回 [`Ep0Reply::AcceptOut`] 后，EP0 收完数据回调本方法。
    fn class_out_data(&mut self, _setup: &Setup, _data: &[u8]) {}

    /// 主机 SET_CONFIGURATION(non-zero) 完成后回调；class 应在这里 prime 数据
    /// EP（DIEPCTL/DOEPCTL 写 EPENA 等）。`SET_CONFIGURATION(0)` 也会回调，参
    /// 数 `cfg = 0` 表示 deconfigure。
    fn on_configured(&mut self, _cfg: u8, _ctx: &Ep0Context) {}

    /// 主机 SET_INTERFACE 后回调；不实现的 class 可以忽略。
    fn on_set_interface(&mut self, _iface: u8, _alt: u8) {}

    /// 每轮 EP0 service 之后调用一次，class 推动数据 EP（如 bulk IN/OUT echo）。
    fn poll(&mut self, _ctx: &Ep0Context) {}
}
