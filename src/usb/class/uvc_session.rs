//! UVC 会话层：把"枚举 → 协商 → 抓帧 → 错误分级 → 重协商/重枚举 → 热拔插"打包成
//! 一个 no_std、不依赖 OS 的状态机。
//!
//! 上层（ArceOS helloworld 等）只需在完成平台初始化（时钟 / PHY / VBUS）后调用
//! [`UvcSession::open`]，随后循环 [`UvcSession::capture_recovering`] 抓帧；抓帧失败时
//! 会话层自动按 sdmmc 风格重试（中止通道 + 重协商），仍失败则要求重枚举；拔插由
//! [`UvcSession::poll_hotplug`] / [`UvcSession::teardown`] 配合外层循环处理。
//!
//! 设计依据见 `bsp-spec` §1.1：本层只依赖 BSP 中性 API（`host`/`dwc2`/`uvc`），不引入
//! axhal/axstd；计时/退避沿用各驱动已有的 `spin_delay` 风格。

use crate::usb::class::uvc::{self, CaptureHealth, UvcImageTuning, UVC_ASSEMBLED_JPEG_DMA_OFF};
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::{self, dwc2};
use crate::usb::host::UvcEnumerated;

/// 抓帧失败时给上层的恢复指引。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// 根口设备已断开（`HPRT0.CONNSTS=0`）：外层应 `teardown` 后等重新插入再 `open`。
    Disconnected,
    /// 重协商耗尽 / 致命错误（STALL/AHBERR）：外层应 `teardown` 后重新 `open`（重枚举）。
    NeedsReenum,
}

/// 热拔插事件（[`UvcSession::poll_hotplug`] 返回）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotplugEvent {
    /// 本轮检测到设备刚接入。
    Connected,
    /// 本轮检测到设备刚断开。
    Disconnected,
    /// 连接状态未变。
    NoChange,
}

/// 会话内部状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionState {
    /// 已协商就绪，可抓帧。
    Streaming,
    /// 需要重协商（仍连着，但流可能错乱）。
    NeedsRearm,
}

/// 单次 [`UvcSession::capture_recovering`] 内的"中止通道 + 重协商"重试次数（对齐 sdmmc
/// `XFER_RETRY = 5`）。NAK/XACT 的细粒度退避已在 `ch_xfer`/`bulk_in` 内部处理，会话层
/// 不重复加退避。
const REARM_RETRIES: u32 = 5;

/// UVC 会话：持有已协商的设备地址 / 流参数，提供带错误恢复的抓帧与热拔插检测。
///
/// 生命周期：`open()` 成功 → 反复 `capture_recovering()` → 失败或 `poll_hotplug()` 报
/// `Disconnected` 时 `teardown()` → 外层重新 `open()`。
pub struct UvcSession {
    dev: u32,
    ep0: u32,
    sel: uvc::UvcStreamSelection,
    state: SessionState,
    last_connected: bool,
}

impl UvcSession {
    /// 枚举 UVC 设备 + 解析配置 + 协商视频流 + 1 帧 warmup。
    ///
    /// **前提**：调用方已完成平台初始化（时钟 / PHY / VBUS / `set_dwc2_base_virt` /
    /// `set_usb_dma_to_phys_fn` / `set_cv182x_phy_base_virt`）。帧尺寸 / interval 偏好由
    /// 调用方在调用前通过 `uvc::set_preferred_frame_size` / `set_preferred_frame_interval`
    /// 设置。
    ///
    /// # 参数
    /// - `tune`：图像调节覆盖（`UvcImageTuning::default()` 表示沿用摄像头出厂 `GET_DEF`）。
    ///
    /// # 返回值
    /// - `Ok(session)`：流已就绪，可立即 `capture_recovering`。
    /// - `Err(...)`：拓扑扫描未发现 UVC 设备 / 协商失败（外层可延时后重试 `open`）。
    pub fn open(tune: &UvcImageTuning) -> UsbResult<Self> {
        let extras = enumerate_topology_retry()?;
        let cam: UvcEnumerated = extras.uvc.ok_or(UsbError::Hardware("no UVC device on bus"))?;
        log::info!(
            "UVC-session: open addr={} VID={:04x} PID={:04x} ep0_mps={}",
            cam.addr,
            cam.vid,
            cam.pid,
            cam.ep0_mps
        );

        let dev = u32::from(cam.addr);
        let ep0 = cam.ep0_mps;

        let cfg_buf = uvc::read_configuration_descriptor(dev, ep0, 1)?;
        let cfg_total = u16::from_le_bytes([cfg_buf[2], cfg_buf[3]]) as usize;
        let cfg_slice = &cfg_buf[..cfg_total.min(cfg_buf.len())];

        let mut sel = uvc::parse_uvc_video_stream(cfg_slice, cfg_total)?;
        let entities = uvc::parse_uvc_control_entities(cfg_slice, cfg_total);
        if let Some(ent) = &entities {
            let _ = uvc::uvc_init_camera_controls(dev, ep0, ent, tune);
        }
        uvc::uvc_start_video_stream(dev, ep0, &mut sel)?;
        log::info!(
            "UVC-session: stream armed {}x{} payload={} frame_size={}",
            sel.frame_w,
            sel.frame_h,
            sel.negotiated_payload_size,
            sel.negotiated_frame_size
        );

        // warmup：丢弃首个不完整帧。
        let _ = uvc::uvc_capture_one_frame(dev, ep0, &sel);

        Ok(Self {
            dev,
            ep0,
            sel,
            state: SessionState::Streaming,
            last_connected: true,
        })
    }

    /// 已协商设备地址。
    #[inline]
    pub fn dev(&self) -> u32 {
        self.dev
    }
    /// 当前流参数（含协商后的 payload/frame size、Isoch alt）。
    #[inline]
    pub fn selection(&self) -> &uvc::UvcStreamSelection {
        &self.sel
    }

    /// 根口是否仍连着（`HPRT0.CONNSTS`）。
    #[inline]
    pub fn connected(&self) -> bool {
        dwc2::root_port_connected()
    }

    /// 抓 1 帧（带错误恢复）。成功返回 DMA 内的 MJPEG 字节切片（在下一次调用前有效）。
    ///
    /// 恢复策略（对齐 sdmmc `for attempt in 0..N`）：
    /// 1. 先查热拔插；断开 → `Err(Disconnected)`。
    /// 2. 抓帧；`Transient`（超时/NAK 耗尽/0 字节）且未达 `REARM_RETRIES` → 中止通道 +
    ///    `uvc_restart_stream` 后重试。
    /// 3. `Fatal`（STALL/AHBERR）或重协商耗尽 → `Err(NeedsReenum)`。
    pub fn capture_recovering(&mut self) -> Result<&'static [u8], SessionError> {
        if !dwc2::root_port_connected() {
            self.last_connected = false;
            return Err(SessionError::Disconnected);
        }

        for attempt in 0..REARM_RETRIES {
            if attempt > 0 {
                log::warn!("UVC-session: rearm attempt {attempt}/{REARM_RETRIES}");
            }
            // 每轮抓帧前确认还在连接（拔插可能发生在重协商间隙）。
            if !dwc2::root_port_connected() {
                self.last_connected = false;
                return Err(SessionError::Disconnected);
            }

            match uvc::uvc_capture_one_frame(self.dev, self.ep0, &self.sel) {
                Ok(n) => {
                    self.state = SessionState::Streaming;
                    let slice = dwc2::dma_rx_slice(UVC_ASSEMBLED_JPEG_DMA_OFF, n)
                        .ok_or(SessionError::NeedsReenum)?;
                    return Ok(slice);
                }
                Err(e) => {
                    let health = CaptureHealth::from_err(e);
                    log::warn!("UVC-session: capture err={e:?} health={health:?} (attempt {attempt})");
                    if health == CaptureHealth::Fatal {
                        return Err(SessionError::NeedsReenum);
                    }
                    // Transient：中止通道 + 重协商后重试。
                    dwc2::abort_bulk_channel();
                    if uvc::uvc_restart_stream(self.dev, self.ep0, &mut self.sel).is_err() {
                        log::warn!("UVC-session: restart_stream failed -> NeedsReenum");
                        return Err(SessionError::NeedsReenum);
                    }
                    self.state = SessionState::NeedsRearm;
                }
            }
        }
        log::warn!("UVC-session: rearm exhausted after {REARM_RETRIES} attempts -> NeedsReenum");
        Err(SessionError::NeedsReenum)
    }

    /// 轮询根口连接状态，与上次比对返回热拔插事件。同时清 `CONNDET` 变化位。
    pub fn poll_hotplug(&mut self) -> HotplugEvent {
        let now = dwc2::root_port_connected();
        let ev = match (self.last_connected, now) {
            (false, true) => HotplugEvent::Connected,
            (true, false) => HotplugEvent::Disconnected,
            _ => HotplugEvent::NoChange,
        };
        self.last_connected = now;
        dwc2::clear_port_connect_detect();
        ev
    }

    /// 干净拆除：停流 + 失能根口 + 清抓帧连续性。`open` 失败或热拔插后调用。
    pub fn teardown(&mut self) {
        let _ = uvc::uvc_stop_streaming(self.dev, self.ep0, self.sel.vs_interface);
        uvc::reset_frame_continuity();
        dwc2::disable_root_port();
        self.state = SessionState::NeedsRearm;
        self.last_connected = dwc2::root_port_connected();
    }
}

/// 拓扑扫描（含重试），对齐 arceos `usb_camera::init` 的 4 次重试逻辑。
fn enumerate_topology_retry() -> UsbResult<host::TopologyScanExtras> {
    let mut last_err: Option<UsbError> = None;
    for attempt in 0..4u32 {
        if attempt > 0 {
            spin_delay_ms(attempt.saturating_mul(1_500));
        }
        match host::enumerate_topology_only() {
            Ok(ex) => return Ok(ex),
            Err(e) => {
                log::warn!("UVC-session: enumerate attempt {} failed: {:?}", attempt + 1, e);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or(UsbError::Hardware("enumerate failed")))
}

/// 粗粒度毫秒延时（与 `topology::spin_delay_ms` 同档：1ms ≈ 250_000 spin）。
fn spin_delay_ms(ms: u32) {
    let cycles = ms.saturating_mul(250_000);
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}
