//! EP0 状态机：buffer DMA 模式下的 SETUP / Data / Status 三阶段处理。
//!
//! # 状态机
//!
//! ```text
//!   ┌────────────┐  SETUP 收到，解析方向/长度
//!   │ WaitSetup  │ ───────────────┬──────────────────┐
//!   └────┬───────┘                │                  │
//!        │ wLength=0              │ IN, len>0        │ OUT, len>0
//!        ▼                        ▼                  ▼
//!   ┌────────────┐          ┌────────────┐     ┌────────────┐
//!   │ StatusIn   │          │  InData    │     │  OutData   │
//!   └────┬───────┘          └────┬───────┘     └────┬───────┘
//!        │                       │                  │
//!        │ ZLP IN 完成           │ 全部 IN 完成     │ 全部 OUT 完成
//!        ▼                       ▼                  ▼
//!   WaitSetup              ┌────────────┐     ┌────────────┐
//!                          │ StatusOut  │     │ StatusIn   │
//!                          └────┬───────┘     └────┬───────┘
//!                               │ ZLP OUT 完成      │ ZLP IN 完成
//!                               ▼                  ▼
//!                            WaitSetup          WaitSetup
//! ```
//!
//! # 关键时序
//!
//! - **SET_ADDRESS**：在 SETUP 阶段**立刻**写 `DCFG.DEVADDR`，DWC2 会用新地址回应
//!   ZLP IN status（USB 2.0 §9.4.6 推荐做法）。
//! - **大响应**：buffer DMA 下 `DIEPTSIZ0.PKTCNT ∈ [1,3]`、`XFERSIZE ∈ [0,127]`，
//!   单次最多 192 字节足够覆盖典型描述符。

use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::mmio;
use crate::usb::host::dwc2::regs::{Dwc2Regs, DCFG, DIEPCTL, DOEPCTL, DSTS};
use crate::usb::platform;
use crate::utils::cache;

use super::desc::{
    DT_CONFIG, DT_DEVICE, DT_DEVICE_QUALIFIER, DT_STRING, REQ_GET_CONFIGURATION,
    REQ_GET_DESCRIPTOR, REQ_GET_INTERFACE, REQ_GET_STATUS, REQ_SET_ADDRESS,
    REQ_SET_CONFIGURATION, REQ_SET_INTERFACE, REQ_TYPE_STANDARD, REQ_RCPT_DEVICE,
    REQ_RCPT_INTERFACE, REQ_RCPT_ENDPOINT, REQ_CLEAR_FEATURE, REQ_SET_FEATURE,
};
use super::{Ep0Context, UsbDeviceClass};

/// USB 总线协商速度（DSTS.ENUMSPD）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbSpeed {
    HighSpeed,
    FullSpeed,
    LowSpeed,
}

/// 标准 8 字节 SETUP 包。
#[derive(Clone, Copy, Default, Debug)]
pub struct Setup {
    pub bm_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl Setup {
    fn parse(buf: &[u8]) -> Self {
        Self {
            bm_request_type: buf[0],
            b_request: buf[1],
            w_value: u16::from_le_bytes([buf[2], buf[3]]),
            w_index: u16::from_le_bytes([buf[4], buf[5]]),
            w_length: u16::from_le_bytes([buf[6], buf[7]]),
        }
    }

    #[inline]
    pub fn dir_in(&self) -> bool {
        self.bm_request_type & 0x80 != 0
    }

    #[inline]
    pub fn req_type(&self) -> u8 {
        self.bm_request_type & 0x60
    }

    #[inline]
    pub fn recipient(&self) -> u8 {
        self.bm_request_type & 0x1f
    }
}

/// EP0 control 请求的回复策略。
pub enum Ep0Reply {
    /// 错误请求，回 STALL。
    Stall,
    /// 不需要 IN/OUT 数据阶段，直接回 ZLP IN status。
    StatusOnly,
    /// IN 方向，发送 `&in_buf[..len]` 给主机。
    Data(usize),
    /// OUT 方向，主机发 `wLength` 字节后回调 [`UsbDeviceClass::class_out_data`]。
    AcceptOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ep0State {
    WaitSetup,
    InData,
    OutData,
    StatusIn,
    StatusOut,
}

/// EP0 SETUP / IN / OUT 数据缓冲区（cache line 对齐，buffer DMA 直接读写）。
const EP0_SETUP_BUF_LEN: usize = 24;
const EP0_IN_BUF_LEN: usize = 256;
const EP0_OUT_BUF_LEN: usize = 256;

#[repr(C, align(64))]
struct Ep0Bufs {
    setup: [u8; EP0_SETUP_BUF_LEN],
    in_data: [u8; EP0_IN_BUF_LEN],
    out_data: [u8; EP0_OUT_BUF_LEN],
}

static mut EP0_BUFS: Ep0Bufs = Ep0Bufs {
    setup: [0; EP0_SETUP_BUF_LEN],
    in_data: [0; EP0_IN_BUF_LEN],
    out_data: [0; EP0_OUT_BUF_LEN],
};

#[inline]
fn ep0_setup_ptr() -> *mut u8 {
    unsafe { core::ptr::addr_of_mut!(EP0_BUFS.setup) as *mut u8 }
}

#[inline]
fn ep0_in_ptr() -> *mut u8 {
    unsafe { core::ptr::addr_of_mut!(EP0_BUFS.in_data) as *mut u8 }
}

#[inline]
fn ep0_out_ptr() -> *mut u8 {
    unsafe { core::ptr::addr_of_mut!(EP0_BUFS.out_data) as *mut u8 }
}

#[inline]
fn dma_phys(p: *const u8) -> u32 {
    platform::usb_dma_phys_for(p)
}

#[inline]
fn regs() -> &'static Dwc2Regs {
    mmio::dwc2_regs().expect("DWC2 base not set")
}

/// EP0 服务对象。轮询 [`service`] 推动 EP0 + class 数据 EP。
pub struct Ep0Service {
    state: Ep0State,
    speed: UsbSpeed,
    enumerated: bool,
    pending_setup: Setup,
    /// IN data 阶段：剩余字节数；初始等于实际要发的长度（可能已截断到 `wLength`）。
    in_remaining: usize,
    in_offset: usize,
    /// OUT data 阶段：期望接收的字节数。
    out_total: usize,
    out_received: usize,
    /// 当前 SET_CONFIGURATION 值（0 = 未配置）。
    current_config: u8,
    /// SET_ADDRESS 收到的新地址，`Some` 时下次 status IN 完成后写入 DCFG（已经在 SETUP 阶段写入，
    /// 这个字段仅做日志/重试参考）。
    pending_address: Option<u8>,
}

impl Ep0Service {
    pub const fn new() -> Self {
        Self {
            state: Ep0State::WaitSetup,
            speed: UsbSpeed::HighSpeed,
            enumerated: false,
            pending_setup: Setup {
                bm_request_type: 0,
                b_request: 0,
                w_value: 0,
                w_index: 0,
                w_length: 0,
            },
            in_remaining: 0,
            in_offset: 0,
            out_total: 0,
            out_received: 0,
            current_config: 0,
            pending_address: None,
        }
    }

    /// 主循环单步：处理一轮 GINTSTS 事件，回调 class 钩子。**不会阻塞**，调用方按需轮询。
    pub fn service<C: UsbDeviceClass>(&mut self, class: &mut C) {
        if platform::dwc2_base_virt() == 0 {
            return;
        }
        let r = regs();
        let gint = r.gintsts.get();

        // USBRST：硬复位，地址清零、所有 EP NAK，软件需重新 prime EP0。
        if gint & (1 << 12) != 0 {
            r.gintsts.set(1 << 12);
            self.handle_usbrst();
        }
        // ENUMDONE：HS/FS chirp 完成，枚举速度可读，配置 EP0 MPS。
        if gint & (1 << 13) != 0 {
            r.gintsts.set(1 << 13);
            self.handle_enumdone();
        }
        // EP0 OUT/IN：DAINT 镜像在某些时机会迟，直接每轮轮询 doepint/diepint。
        // 内部 `if int == 0 return;` 保护，无 EP 事件时无副作用。
        self.handle_ep0_out(class);
        self.handle_ep0_in(class);

        let ctx = Ep0Context { speed: self.speed };
        class.poll(&ctx);
    }

    fn handle_usbrst(&mut self) {
        crate::usb::log::usb_log_fmt(format_args!("USB-DEV USBRST"));
        let r = regs();
        // 把 DCFG.DEVADDR 清回 0
        r.dcfg.modify(DCFG::DEVADDR.val(0));
        // 全 NAK
        for ep in 0..crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
            if r.diep[ep].diepctl.is_set(DIEPCTL::EPENA) {
                r.diep[ep]
                    .diepctl
                    .modify(DIEPCTL::SNAK::SET + DIEPCTL::EPDIS::SET);
            }
            if r.doep[ep].doepctl.is_set(DOEPCTL::EPENA) {
                r.doep[ep]
                    .doepctl
                    .modify(DOEPCTL::SNAK::SET + DOEPCTL::EPDIS::SET);
            }
        }
        self.state = Ep0State::WaitSetup;
        self.in_remaining = 0;
        self.in_offset = 0;
        self.out_total = 0;
        self.out_received = 0;
        self.current_config = 0;
        self.pending_address = None;
        self.enumerated = false;
    }

    fn handle_enumdone(&mut self) {
        let r = regs();
        let speed = match r.dsts.read(DSTS::ENUMSPD) {
            0 => UsbSpeed::HighSpeed,
            1 | 3 => UsbSpeed::FullSpeed,
            2 => UsbSpeed::LowSpeed,
            _ => UsbSpeed::FullSpeed,
        };
        self.speed = speed;
        self.enumerated = true;
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DEV ENUMDONE speed={:?} DSTS={:#010x}",
            speed,
            r.dsts.get()
        ));
        // EP0 MPS 编码（高速 / 全速：00=64；低速：11=8）
        let mps_field: u32 = if speed == UsbSpeed::LowSpeed { 3 } else { 0 };
        r.diep[0].diepctl.modify(DIEPCTL::MPS.val(mps_field));
        r.doep[0].doepctl.modify(DOEPCTL::MPS.val(mps_field));
        // 准备接收第一个 SETUP
        self.prime_ep0_setup();
    }

    /// 重新装填 EP0 OUT 接收 SETUP buffer。
    ///
    /// **SUPCNT=1**：dwc2 在 buffer DMA 模式下，若 SUPCNT>1，硬件会把 SETUP 后紧
    /// 跟到来的 OUT data 当作连续 SETUP 包写入 setup_buf，覆盖原 SETUP，导致软件
    /// 永远收不到真实的 OUT data。USB 协议本身禁止 back-to-back SETUP，所以
    /// SUPCNT=1 完全够用：硬件接到 1 个 SETUP 后立刻 NAK，软件 prime OUT data EP
    /// 之后再继续接收。
    fn prime_ep0_setup(&self) {
        let r = regs();
        unsafe {
            cache::dcache_invalidate_after_dma(ep0_setup_ptr(), 24);
        }
        let pa = dma_phys(ep0_setup_ptr());
        r.doep[0].doepdma.set(pa);
        // SUPCNT[30:29]=1, PKTCNT[19]=1, XFERSIZE[6:0]=8（1 个 SETUP 包 = 8 字节）
        let val: u32 = (1 << 29) | (1 << 19) | 8;
        r.doep[0].doeptsiz.set(val);
        // EPENA + CNAK；DOEPCTL EP0 写 EPDIS 是非法的（不要置位）
        r.doep[0]
            .doepctl
            .modify(DOEPCTL::EPENA::SET + DOEPCTL::CNAK::SET);
    }

    /// prime EP0 OUT 接收一个 OUT data packet（用于 OUT data 阶段）。
    ///
    /// **关键**：DWC2 buffer DMA 模式下 EP0 OUT 的 XFERSIZE 必须等于**实际期望
    /// 接收字节数**（≤ MPS），不能直接写 MPS。vendor `setdma_rx` 同样取
    /// `length = min(remaining, ep->ep.maxpacket)`。如果写 MPS=64 而 host 只
    /// 发 7 字节 short packet，cv182x 的 dwc2 实测不会把 7 字节写到 DOEPDMA
    /// 指向的新 buffer，反而留在 SETUP buffer 中。
    ///
    /// `expected_remaining` 是本 packet 期望接收的字节数（`wLength` 减去已收）。
    /// PKTCNT=1，SUPCNT=0（data 阶段必须 0，否则后续 OUT data 被误当 SETUP 写
    /// 入 setup_buf）。
    fn prime_ep0_out_data(&self, expected_remaining: usize) {
        let r = regs();
        let mps = self.ep0_mps();
        // EP0 buffer DMA：XFERSIZE 字段 7 bit，最大 127；至少 1 字节避免硬件异常
        let xfer = expected_remaining.min(mps).max(1);
        unsafe {
            cache::dcache_invalidate_after_dma(ep0_out_ptr(), mps);
        }
        let pa = dma_phys(ep0_out_ptr());
        r.doep[0].doepdma.set(pa);
        let val: u32 = (1 << 19) | ((xfer as u32) & 0x7f);
        r.doep[0].doeptsiz.set(val);
        // vendor 风格全字写 doepctl：清 EPDIS (bit 30)，置 EPENA + CNAK，保留其他位。
        let cur = r.doep[0].doepctl.get();
        let new_ctrl = (cur & !(1u32 << 30)) | (1u32 << 31) | (1u32 << 26);
        r.doep[0].doepctl.set(new_ctrl);
        // EPENA + CNAK 后 cv182x dwc2 立刻置位 STSPHSERCVD/XFERCOMPL（spurious），
        // 必须 clear，否则下一轮 service 误判为 OUT data 完成（actual=0）。
        r.doep[0].doepint.set(0xffff_ffff);
        // 不在此打印日志：log 串口写要 ~7ms，会错过 host OUT data 的 NAK 重试窗口。
        let _ = xfer;
    }

    /// prime EP0 OUT 接收 ZLP（status 阶段）。
    fn prime_ep0_status_out(&self) {
        let r = regs();
        let pa = dma_phys(ep0_out_ptr());
        r.doep[0].doepdma.set(pa);
        // SUPCNT=0；只是普通 OUT ZLP
        let val: u32 = (1 << 19) | 0;
        r.doep[0].doeptsiz.set(val);
        r.doep[0]
            .doepctl
            .modify(DOEPCTL::EPENA::SET + DOEPCTL::CNAK::SET);
    }

    /// prime EP0 IN 发送 `len` 字节（已写入 in_data buffer）。`len ≤ 127`。
    fn prime_ep0_in_data(&self, len: usize) {
        let r = regs();
        let mps = self.ep0_mps();
        let pkt_cnt = if len == 0 { 1 } else { (len + mps - 1) / mps };
        let pkt_cnt = pkt_cnt.min(3) as u32;
        unsafe {
            cache::dcache_clean_for_dma(ep0_in_ptr(), len.max(1));
        }
        let pa = dma_phys(ep0_in_ptr());
        r.diep[0].diepdma.set(pa);
        let val: u32 = (pkt_cnt << 19) | ((len as u32) & 0x7f);
        r.diep[0].dieptsiz.set(val);
        r.diep[0]
            .diepctl
            .modify(DIEPCTL::EPENA::SET + DIEPCTL::CNAK::SET);
    }

    /// prime EP0 IN 发送 ZLP（status 阶段）。
    fn prime_ep0_status_in(&self) {
        let r = regs();
        let pa = dma_phys(ep0_in_ptr());
        r.diep[0].diepdma.set(pa);
        let val: u32 = (1 << 19) | 0;
        r.diep[0].dieptsiz.set(val);
        r.diep[0]
            .diepctl
            .modify(DIEPCTL::EPENA::SET + DIEPCTL::CNAK::SET);
    }

    fn stall_ep0(&mut self) {
        let r = regs();
        r.diep[0].diepctl.modify(DIEPCTL::STALL::SET);
        r.doep[0].doepctl.modify(DOEPCTL::STALL::SET);
        self.state = Ep0State::WaitSetup;
        self.prime_ep0_setup();
    }

    #[inline]
    fn ep0_mps(&self) -> usize {
        if self.speed == UsbSpeed::LowSpeed { 8 } else { 64 }
    }

    fn handle_ep0_out<C: UsbDeviceClass>(&mut self, class: &mut C) {
        let r = regs();
        let int = r.doep[0].doepint.get();
        if int == 0 {
            return;
        }
        // SETUP done：bit3 STUP；只在收到完整 SETUP 包后置位
        if int & (1 << 3) != 0 {
            // **先 prime OUT/IN，最后再 log**：cv182x dwc2 在 SETUP 结束后短时间内
            // 必须把 EP0 OUT 重新 prime（否则 host 发 OUT data 时 dwc2 NAK→超时）。
            // 串口 115200 一行 log ≈ 7ms，足以让 host SET_LINE_CODING 的 OUT data
            // 因 NAK 超时被丢弃。所以先解析 + dispatch + prime，最后才打印。
            r.doep[0].doepint.set(0xffff_ffff);
            unsafe {
                cache::dcache_invalidate_after_dma(ep0_setup_ptr(), 24);
            }
            let mut s = [0u8; 8];
            unsafe {
                core::ptr::copy_nonoverlapping(ep0_setup_ptr(), s.as_mut_ptr(), 8);
            }
            self.pending_setup = Setup::parse(&s);
            self.dispatch_setup(class);
            return;
        }
        // STSPHSERCVD (bit 5) 只对 Scatter-Gather DMA 有效；buffer DMA 下偶尔会
        // 看到，仅清除即可，不要拿它当 OUT data 完成信号。
        if int & (1 << 5) != 0 {
            r.doep[0].doepint.set(1 << 5);
        }
        if int & (1 << 0) != 0 {
            // OUT XFERCOMPL：buffer DMA 下表示当前 OUT 传输的所有 packet 已收齐
            r.doep[0].doepint.set(1 << 0);
            match self.state {
                Ep0State::OutData => {
                    let mps = self.ep0_mps();
                    let expected = self
                        .out_total
                        .saturating_sub(self.out_received)
                        .min(mps)
                        .max(1);
                    let tsiz = r.doep[0].doeptsiz.get();
                    let residual = (tsiz & 0x7f) as usize;
                    let actual = expected.saturating_sub(residual);
                    if actual > 0 {
                        unsafe {
                            cache::dcache_invalidate_after_dma(ep0_out_ptr(), actual);
                        }
                    }
                    self.out_received += actual;
                    let len = actual.min(self.out_total);
                    let s = self.pending_setup;
                    let buf = unsafe { core::slice::from_raw_parts(ep0_out_ptr(), len) };
                    class.class_out_data(&s, buf);
                    // 单 packet 接收完成立刻 prime status IN（host 紧接发 IN token）
                    self.state = Ep0State::StatusIn;
                    self.prime_ep0_status_in();
                    let _ = tsiz;
                }
                Ep0State::StatusOut => {
                    // OUT ZLP 完成，整个传输结束
                    self.state = Ep0State::WaitSetup;
                    self.prime_ep0_setup();
                }
                _ => {
                    // 异常：在不应收 OUT 的状态收到了。重新 prime SETUP
                    self.state = Ep0State::WaitSetup;
                    self.prime_ep0_setup();
                }
            }
        }
    }

    fn handle_ep0_in<C: UsbDeviceClass>(&mut self, _class: &mut C) {
        let r = regs();
        let int = r.diep[0].diepint.get();
        if int == 0 {
            return;
        }
        if int & (1 << 0) != 0 {
            // IN XFERCOMPL
            r.diep[0].diepint.set(1 << 0);
            match self.state {
                Ep0State::InData => {
                    let mps = self.ep0_mps();
                    let chunk = self.in_remaining.min(mps);
                    self.in_remaining -= chunk;
                    self.in_offset += chunk;
                    if self.in_remaining == 0 {
                        // IN 完成，进入 OUT status
                        self.state = Ep0State::StatusOut;
                        self.prime_ep0_status_out();
                    } else {
                        // 还有剩，继续发——先把数据拷到 in_buf 头（buffer DMA 写后我们只
                        // 用了一个 buffer，可以直接重新 prime in_offset 起点的指针）
                        // 为简化，把后续数据 memmove 到 in_buf 头部
                        unsafe {
                            core::ptr::copy(
                                ep0_in_ptr().add(chunk),
                                ep0_in_ptr(),
                                self.in_remaining,
                            );
                        }
                        self.in_offset = 0;
                        self.prime_ep0_in_data(self.in_remaining);
                    }
                }
                Ep0State::StatusIn => {
                    // ZLP IN 完成，结束传输
                    if let Some(addr) = self.pending_address.take() {
                        crate::usb::log::usb_log_fmt(format_args!(
                            "USB-DEV SET_ADDRESS({}) status complete",
                            addr
                        ));
                    }
                    self.state = Ep0State::WaitSetup;
                    self.prime_ep0_setup();
                }
                _ => {
                    self.state = Ep0State::WaitSetup;
                    self.prime_ep0_setup();
                }
            }
        }
        // 其它中断（NAK/Timeout/EPDISBLD）忽略
        if int & !(1 << 0) != 0 {
            r.diep[0].diepint.set(int & !(1 << 0));
        }
    }

    /// SETUP 包解析与请求分发。
    ///
    /// **不打印 SETUP log**：cv182x dwc2 EP0 控制传输的 status 阶段（IN/OUT
    /// ZLP）window 极紧（约几十 ms），串口 115200 一行 log ≈ 7ms，会让下一阶
    /// 段的 prime 来不及，host 放弃。仅 class 路径打印 class request 类型。
    fn dispatch_setup<C: UsbDeviceClass>(&mut self, class: &mut C) {
        let s = self.pending_setup;
        // 标准请求
        if s.req_type() == REQ_TYPE_STANDARD {
            self.handle_standard(class);
            return;
        }
        // class / vendor 请求委托给 class
        let in_buf = unsafe { core::slice::from_raw_parts_mut(ep0_in_ptr(), EP0_IN_BUF_LEN) };
        let reply = class.class_setup(&s, in_buf);
        match reply {
            Ep0Reply::Stall => self.stall_ep0(),
            Ep0Reply::StatusOnly => {
                self.state = Ep0State::StatusIn;
                self.prime_ep0_status_in();
            }
            Ep0Reply::Data(len) => {
                let len = len.min(s.w_length as usize).min(in_buf.len());
                self.start_in_data(len);
            }
            Ep0Reply::AcceptOut => {
                let len = (s.w_length as usize).min(EP0_OUT_BUF_LEN);
                self.out_total = len;
                self.out_received = 0;
                if len == 0 {
                    self.state = Ep0State::StatusIn;
                    self.prime_ep0_status_in();
                } else {
                    self.state = Ep0State::OutData;
                    self.prime_ep0_out_data(len);
                }
            }
        }
    }

    fn handle_standard<C: UsbDeviceClass>(&mut self, class: &mut C) {
        let s = self.pending_setup;
        match s.b_request {
            REQ_GET_DESCRIPTOR => {
                let desc_type = (s.w_value >> 8) as u8;
                let desc_idx = (s.w_value & 0xff) as u8;
                let bytes: Option<&[u8]> = match desc_type {
                    DT_DEVICE => Some(class.device_descriptor()),
                    DT_CONFIG => {
                        if desc_idx == 0 {
                            Some(class.config_descriptor())
                        } else {
                            None
                        }
                    }
                    DT_STRING => class.string_descriptor(desc_idx),
                    DT_DEVICE_QUALIFIER => None, // FS 不支持，HS 设备可选；先 STALL
                    _ => None,
                };
                match bytes {
                    Some(src) => {
                        let max = (s.w_length as usize).min(src.len()).min(EP0_IN_BUF_LEN);
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                src.as_ptr(),
                                ep0_in_ptr(),
                                max,
                            );
                        }
                        self.start_in_data(max);
                    }
                    None => self.stall_ep0(),
                }
            }
            REQ_SET_ADDRESS => {
                let addr = (s.w_value & 0x7f) as u8;
                // DWC2 推荐：在 SETUP 阶段就写 DCFG.DEVADDR，硬件用新地址回应 ZLP
                regs().dcfg.modify(DCFG::DEVADDR.val(addr as u32));
                self.pending_address = Some(addr);
                self.state = Ep0State::StatusIn;
                self.prime_ep0_status_in();
            }
            REQ_SET_CONFIGURATION => {
                let cfg = (s.w_value & 0xff) as u8;
                self.current_config = cfg;
                let ctx = Ep0Context { speed: self.speed };
                class.on_configured(cfg, &ctx);
                self.state = Ep0State::StatusIn;
                self.prime_ep0_status_in();
            }
            REQ_GET_CONFIGURATION => {
                unsafe {
                    *ep0_in_ptr() = self.current_config;
                }
                self.start_in_data(1);
            }
            REQ_SET_INTERFACE => {
                let alt = (s.w_value & 0xff) as u8;
                let iface = (s.w_index & 0xff) as u8;
                class.on_set_interface(iface, alt);
                self.state = Ep0State::StatusIn;
                self.prime_ep0_status_in();
            }
            REQ_GET_INTERFACE => {
                unsafe {
                    *ep0_in_ptr() = 0;
                }
                self.start_in_data(1);
            }
            REQ_GET_STATUS => {
                unsafe {
                    *ep0_in_ptr() = 0;
                    *ep0_in_ptr().add(1) = 0;
                }
                self.start_in_data(2);
            }
            REQ_CLEAR_FEATURE | REQ_SET_FEATURE => {
                // feature 处理（HALT 在 EP recipient）：按 spec 接受但不做实际处理
                let _ = (REQ_RCPT_DEVICE, REQ_RCPT_INTERFACE, REQ_RCPT_ENDPOINT);
                self.state = Ep0State::StatusIn;
                self.prime_ep0_status_in();
            }
            _ => {
                crate::usb::log::usb_log_fmt(format_args!(
                    "USB-DEV unsupported standard request {:#04x}, STALL",
                    s.b_request
                ));
                self.stall_ep0();
            }
        }
    }

    fn start_in_data(&mut self, len: usize) {
        let mps = self.ep0_mps();
        if len == 0 {
            self.state = Ep0State::StatusOut;
            self.prime_ep0_status_in();
            return;
        }
        self.in_remaining = len;
        self.in_offset = 0;
        self.state = Ep0State::InData;
        let chunk = len.min(mps * 3).min(127);
        // 一次能塞进的最多字节
        let _ = chunk;
        self.prime_ep0_in_data(len.min(127).min(mps * 3));
    }
}

impl Default for Ep0Service {
    fn default() -> Self {
        Self::new()
    }
}

/// 帮 class 配置 IN bulk endpoint（buffer DMA 模式）。
///
/// `ep_num` 1..=15，`mps` 高速 bulk=512 / 全速=64，`tx_fifo_num` 与 [`super::controller`]
/// 中分配的 DIEPTXFn 槽位一致（默认 1）。
pub fn configure_bulk_in_ep(ep_num: u8, mps: u32, tx_fifo_num: u32) -> UsbResult<()> {
    let ep = ep_num as usize;
    if ep == 0 || ep >= crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
        return Err(UsbError::Protocol("invalid bulk IN ep number"));
    }
    let r = regs();
    r.diep[ep].diepctl.set(0);
    r.diep[ep].diepctl.modify(
        DIEPCTL::MPS.val(mps & 0x7ff)
            + DIEPCTL::EPTYPE::Bulk
            + DIEPCTL::USBACTEP::SET
            + DIEPCTL::TXFNUM.val(tx_fifo_num & 0xf)
            + DIEPCTL::SETD0PID::SET,
    );
    // 同时打开 DAINTMSK 中对应位
    let prev = r.daintmsk.get();
    r.daintmsk.set(prev | (1u32 << ep));
    Ok(())
}

/// 帮 class 配置 OUT bulk endpoint（buffer DMA 模式）。调用方在 [`UsbDeviceClass::poll`]
/// 中调用 [`prime_bulk_out`] 实际开始接收。
pub fn configure_bulk_out_ep(ep_num: u8, mps: u32) -> UsbResult<()> {
    let ep = ep_num as usize;
    if ep == 0 || ep >= crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
        return Err(UsbError::Protocol("invalid bulk OUT ep number"));
    }
    let r = regs();
    r.doep[ep].doepctl.set(0);
    r.doep[ep].doepctl.modify(
        DOEPCTL::MPS.val(mps & 0x7ff)
            + DOEPCTL::EPTYPE::Bulk
            + DOEPCTL::USBACTEP::SET
            + DOEPCTL::SETD0PID::SET,
    );
    let prev = r.daintmsk.get();
    r.daintmsk.set(prev | (1u32 << (ep + 16)));
    Ok(())
}

/// 配置 INTERRUPT IN endpoint（CDC ACM Notification EP 等）。
pub fn configure_intr_in_ep(ep_num: u8, mps: u32, tx_fifo_num: u32) -> UsbResult<()> {
    let ep = ep_num as usize;
    if ep == 0 || ep >= crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
        return Err(UsbError::Protocol("invalid intr IN ep number"));
    }
    let r = regs();
    r.diep[ep].diepctl.set(0);
    r.diep[ep].diepctl.modify(
        DIEPCTL::MPS.val(mps & 0x7ff)
            + DIEPCTL::EPTYPE::Interrupt
            + DIEPCTL::USBACTEP::SET
            + DIEPCTL::TXFNUM.val(tx_fifo_num & 0xf)
            + DIEPCTL::SETD0PID::SET,
    );
    let prev = r.daintmsk.get();
    r.daintmsk.set(prev | (1u32 << ep));
    Ok(())
}

/// prime bulk OUT 接收一段缓冲区。`buf` 必须是 cache line 对齐的 DMA 可见缓冲。
///
/// 必须用 vendor-style 全字写而不是 RMW —— XFERCOMPL 之后 dwc2 在某些版本上会
/// 自动把 EPDIS（bit 30）设为 1，普通 modify(EPENA + CNAK) 会保留 EPDIS 导致
/// 下次 prime 不真正接收。
pub fn prime_bulk_out(ep_num: u8, buf_pa: u32, len: u32, num_packets: u32) -> UsbResult<()> {
    let ep = ep_num as usize;
    if ep == 0 || ep >= crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
        return Err(UsbError::Protocol("invalid bulk OUT ep number"));
    }
    let r = regs();
    r.doep[ep].doepdma.set(buf_pa);
    let val: u32 = (num_packets << 19) | (len & 0x7ffff);
    r.doep[ep].doeptsiz.set(val);
    let cur = r.doep[ep].doepctl.get();
    let new_ctl = (cur & !(1u32 << 30)) | (1u32 << 31) | (1u32 << 26);
    r.doep[ep].doepctl.set(new_ctl);
    Ok(())
}

/// 启动 bulk IN 发送。`len` 是要发的字节数；`num_packets` 通常为 `ceil(len / mps)` 且至少为 1（ZLP）。
///
/// 同样使用全字写避免 EPDIS 残留。
pub fn start_bulk_in(ep_num: u8, buf_pa: u32, len: u32, num_packets: u32) -> UsbResult<()> {
    let ep = ep_num as usize;
    if ep == 0 || ep >= crate::usb::host::dwc2::regs::DWC2_MAX_DEV_ENDPOINTS {
        return Err(UsbError::Protocol("invalid bulk IN ep number"));
    }
    let r = regs();
    r.diep[ep].diepdma.set(buf_pa);
    let val: u32 = (num_packets << 19) | (len & 0x7ffff);
    r.diep[ep].dieptsiz.set(val);
    let cur = r.diep[ep].diepctl.get();
    let new_ctl = (cur & !(1u32 << 30)) | (1u32 << 31) | (1u32 << 26);
    r.diep[ep].diepctl.set(new_ctl);
    Ok(())
}

/// 读取 EP n 的 DOEPINT 并清除指定位（class 实现里轮询 OUT XFERCOMPL 时使用）。
pub fn read_clear_doepint(ep_num: u8) -> u32 {
    let r = regs();
    let v = r.doep[ep_num as usize].doepint.get();
    if v != 0 {
        r.doep[ep_num as usize].doepint.set(v);
    }
    v
}

/// 读取 EP n 的 DIEPINT 并清除指定位。
pub fn read_clear_diepint(ep_num: u8) -> u32 {
    let r = regs();
    let v = r.diep[ep_num as usize].diepint.get();
    if v != 0 {
        r.diep[ep_num as usize].diepint.set(v);
    }
    v
}
