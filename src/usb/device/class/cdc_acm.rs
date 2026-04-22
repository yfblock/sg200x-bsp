//! CDC-ACM (Abstract Control Model)：USB 虚拟串口。
//!
//! PC 端识别后会出现 `/dev/ttyACM0`（Linux）或一个 COM 口（Windows）。
//!
//! # 端点分配
//!
//! - EP0 IN/OUT：control（NPTX FIFO 0）
//! - EP3 IN：interrupt notification（TX FIFO 2，8 字节 MPS）—— 描述符里必须有，
//!   实际可以一直 NAK，PC 不会强求
//! - EP1 IN：bulk data IN（TX FIFO 1，FS=64 / HS=512 字节 MPS）
//! - EP1 OUT：bulk data OUT（RX 共享，同上 MPS）
//!
//! # 数据通路
//!
//! 当前实现是 **echo loop**：PC 写一字节，板子读出后立即原样写回，便于联调。
//! 上层应用如需把 CDC-ACM 当作真实串口，可改写 [`CdcAcm::class_out_data`] /
//! 替换 [`CdcAcm::try_echo`]。
//!
//! # 已知限制
//!
//! 单笔 host write 最大 64 字节（== bulk MPS）。host 写 > MPS 字节时会被 USB
//! stack 切成多 packet 发出，本驱动当前 prime 1 个 packet 一次，遇到第二个
//! packet 紧跟到达且 EP 已 disable 的情况下 dwc2 报 `BBLEERR` 但缺少完整恢复
//! 流程（需要 SNAK→EPDISBLD→FIFO flush→重 prime）。要支持长 packet，需要进一
//! 步实现 BBLEERR 的 endpoint reset 流程。

use tock_registers::interfaces::Readable;

use crate::usb::device::desc::{encode_string_descriptor, EP_ATTR_BULK, EP_ATTR_INTERRUPT};
use crate::usb::device::ep0::{
    configure_bulk_in_ep, configure_bulk_out_ep, configure_intr_in_ep, prime_bulk_out,
    read_clear_diepint, read_clear_doepint, start_bulk_in, Ep0Reply, Setup,
};
use crate::usb::device::{Ep0Context, UsbDeviceClass, UsbSpeed};
use crate::utils::cache;

/// CDC-ACM bulk 数据 EP。
const BULK_DATA_EP: u8 = 1;
/// CDC-ACM notification interrupt EP。
const NOTIFY_EP: u8 = 3;

/// 静态 device descriptor（FS, bcdUSB=0x0200, CDC class）。
const DEVICE_DESC: [u8; 18] = [
    18,            // bLength
    0x01,          // bDescriptorType=DEVICE
    0x00, 0x02,    // bcdUSB = 0x0200
    0x02,          // bDeviceClass = CDC
    0x00,          // bDeviceSubClass
    0x00,          // bDeviceProtocol
    64,            // bMaxPacketSize0
    0x09, 0x12,    // idVendor = 0x1209 (pid.codes test VID)
    0x01, 0x00,    // idProduct = 0x0001
    0x01, 0x00,    // bcdDevice = 0x0001
    1,             // iManufacturer
    2,             // iProduct
    3,             // iSerialNumber
    1,             // bNumConfigurations
];

/// 静态 configuration descriptor（含 1 个 CDC ACM 接口 + 1 个 CDC Data 接口）。
///
/// FS bulk MPS=64；HS 时仍按 64 工作（兼容性最好；要 HS 性能再换 512 + DCFG=HS）。
const CONFIG_DESC: [u8; 67] = [
    // Configuration Descriptor
    9, 0x02, 67, 0, 2, 1, 0, 0xC0, 50,
    // Interface Descriptor 0 (Communications)
    9, 0x04, 0, 0, 1, 0x02, 0x02, 0x00, 0,
    // CDC Header Functional
    5, 0x24, 0x00, 0x10, 0x01,
    // CDC Call Management
    5, 0x24, 0x01, 0x00, 1,
    // CDC ACM Functional
    4, 0x24, 0x02, 0x02,
    // CDC Union Functional
    5, 0x24, 0x06, 0, 1,
    // Endpoint Descriptor: Notification IN (EP3, Interrupt, MPS=8, Interval=16ms)
    7, 0x05, 0x83, EP_ATTR_INTERRUPT, 8, 0, 16,
    // Interface Descriptor 1 (CDC Data)
    9, 0x04, 1, 0, 2, 0x0A, 0x00, 0x00, 0,
    // Endpoint Descriptor: Bulk OUT (EP1, MPS=64)
    7, 0x05, 0x01, EP_ATTR_BULK, 64, 0, 0,
    // Endpoint Descriptor: Bulk IN (EP1, MPS=64)
    7, 0x05, 0x81, EP_ATTR_BULK, 64, 0, 0,
];

/// String descriptor 0：LANGID = English (US)。
const STRING0: [u8; 4] = [4, 0x03, 0x09, 0x04];

// 编译期保留 64 字节给每个字符串 descriptor（足够 30 字符 ASCII）。
static mut STRING_BUFS: [[u8; 64]; 4] = [[0; 64]; 4];

/// 把 ASCII 字符串编码到 STRING_BUFS[idx]，返回切片。
fn encode_string(idx: usize, s: &str) -> &'static [u8] {
    let buf = unsafe { &mut STRING_BUFS[idx] };
    let n = encode_string_descriptor(s, buf);
    unsafe { core::slice::from_raw_parts(buf.as_ptr(), n) }
}

/// CDC-ACM 串口 line coding（DTE rate / stop bits / parity / data bits）。
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct LineCoding {
    dwdte_rate: u32,
    b_char_format: u8,
    b_parity_type: u8,
    b_data_bits: u8,
}

impl LineCoding {
    const DEFAULT: Self = Self {
        dwdte_rate: 115_200,
        b_char_format: 0,
        b_parity_type: 0,
        b_data_bits: 8,
    };
}

/// CDC-ACM bulk MPS（FS/HS 都用 64）。
const BULK_MPS: u32 = 64;
/// bulk OUT/IN 单个 buffer 容量。
///
/// dwc2 buffer DMA 模式下 OUT XFERCOMPL 触发条件是「收满 PKTCNT 个 packet」**或**
/// 「收到 short packet (<MPS)」。如果 host 写 N*MPS 整数倍字节又不发 ZLP（cdc-acm
/// 的 Linux host 不会主动发 OUT ZLP），那 PKTCNT > 1 时硬件会一直等更多
/// packet，导致 XFERCOMPL 永远不触发。最稳妥的做法是每次只 prime 1 个 packet，
/// 写完一个 packet 立刻 re-prime。
const BULK_BUF_SIZE: usize = 64;

/// 64-byte 对齐的 DMA buffer，避免和相邻数据共享 cache line（dcache_invalidate
/// 可能错误丢弃相邻字节，dcache_clean 也可能写脏数据回 DRAM 干扰对端 DMA）。
#[repr(C, align(64))]
struct DmaBuf([u8; BULK_BUF_SIZE]);

/// CDC-ACM 状态：line coding、控制线、当前 RX/TX 数据。
///
/// **必须**用独立 RX / TX buffer：bulk OUT DMA 写入 `rx_buf`、bulk IN DMA 读取
/// `tx_buf`。如果共用，OUT prime 后硬件随时可能写入新数据破坏正在 IN 的 echo
/// 源，导致 host 收到错位 / 损坏的字节。
pub struct CdcAcm {
    line_coding: LineCoding,
    /// `bRequest=0x22 SET_CONTROL_LINE_STATE` 的 wValue：bit0=DTR, bit1=RTS。
    control_lines: u16,
    /// bulk OUT 接收缓冲（host -> device）。
    rx_buf: DmaBuf,
    /// bulk IN 发送缓冲（device -> host），独立于 rx_buf。
    tx_buf: DmaBuf,
    /// `tx_buf` 当前待 echo 的字节数（>0 时 try_echo 会启动 IN）。
    tx_pending: usize,
    /// IN 是否在传输（避免重复 prime）。
    in_busy: bool,
    /// OUT 是否在传输。
    out_armed: bool,
    /// 上一笔 IN 是 MPS 整数倍 full packet —— XFERCOMPL 后需要再独立发一个 ZLP，
    /// 让 host 的 cdc-acm BulkIn URB 结束本次 read。
    in_need_zlp: bool,
    /// 当前协商速度。
    speed: UsbSpeed,
    configured: bool,
}

impl CdcAcm {
    pub const fn new() -> Self {
        Self {
            line_coding: LineCoding::DEFAULT,
            control_lines: 0,
            rx_buf: DmaBuf([0; BULK_BUF_SIZE]),
            tx_buf: DmaBuf([0; BULK_BUF_SIZE]),
            tx_pending: 0,
            in_busy: false,
            out_armed: false,
            in_need_zlp: false,
            speed: UsbSpeed::FullSpeed,
            configured: false,
        }
    }
}

impl Default for CdcAcm {
    fn default() -> Self {
        Self::new()
    }
}

/// CDC class request `bRequest`（CDC PSTN 1.20 §6.3）。
const CDC_REQ_SET_LINE_CODING: u8 = 0x20;
const CDC_REQ_GET_LINE_CODING: u8 = 0x21;
const CDC_REQ_SET_CONTROL_LINE_STATE: u8 = 0x22;
const CDC_REQ_SEND_BREAK: u8 = 0x23;

impl UsbDeviceClass for CdcAcm {
    fn device_descriptor(&self) -> &'static [u8] {
        &DEVICE_DESC
    }

    fn config_descriptor(&self) -> &'static [u8] {
        &CONFIG_DESC
    }

    fn string_descriptor(&self, idx: u8) -> Option<&'static [u8]> {
        match idx {
            0 => Some(&STRING0),
            1 => Some(encode_string(1, "sg200x-bsp")),
            2 => Some(encode_string(2, "SG2002 CDC-ACM")),
            3 => Some(encode_string(3, "SN-2026")),
            _ => None,
        }
    }

    fn class_setup(&mut self, setup: &Setup, in_buf: &mut [u8]) -> Ep0Reply {
        // CDC class 请求：interface recipient
        if setup.recipient() != crate::usb::device::desc::REQ_RCPT_INTERFACE {
            return Ep0Reply::Stall;
        }
        match setup.b_request {
            CDC_REQ_SET_LINE_CODING => {
                if setup.dir_in() || setup.w_length as usize != 7 {
                    return Ep0Reply::Stall;
                }
                Ep0Reply::AcceptOut
            }
            CDC_REQ_GET_LINE_CODING => {
                if !setup.dir_in() || setup.w_length < 7 {
                    return Ep0Reply::Stall;
                }
                let lc = self.line_coding;
                in_buf[0] = (lc.dwdte_rate & 0xff) as u8;
                in_buf[1] = ((lc.dwdte_rate >> 8) & 0xff) as u8;
                in_buf[2] = ((lc.dwdte_rate >> 16) & 0xff) as u8;
                in_buf[3] = ((lc.dwdte_rate >> 24) & 0xff) as u8;
                in_buf[4] = lc.b_char_format;
                in_buf[5] = lc.b_parity_type;
                in_buf[6] = lc.b_data_bits;
                Ep0Reply::Data(7)
            }
            CDC_REQ_SET_CONTROL_LINE_STATE => {
                self.control_lines = setup.w_value;
                crate::usb::log::usb_log_fmt(format_args!(
                    "USB-DEV CDC SET_CONTROL_LINE_STATE = {:#06x} (DTR={} RTS={})",
                    setup.w_value,
                    (setup.w_value >> 0) & 1,
                    (setup.w_value >> 1) & 1,
                ));
                Ep0Reply::StatusOnly
            }
            CDC_REQ_SEND_BREAK => Ep0Reply::StatusOnly,
            _ => Ep0Reply::Stall,
        }
    }

    fn class_out_data(&mut self, setup: &Setup, data: &[u8]) {
        if setup.b_request == CDC_REQ_SET_LINE_CODING && data.len() >= 7 {
            let dwdte_rate = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            self.line_coding.dwdte_rate = dwdte_rate;
            self.line_coding.b_char_format = data[4];
            self.line_coding.b_parity_type = data[5];
            self.line_coding.b_data_bits = data[6];
            crate::usb::log::usb_log_fmt(format_args!(
                "USB-DEV CDC SET_LINE_CODING rate={} fmt={} parity={} bits={}",
                dwdte_rate, data[4], data[5], data[6]
            ));
        }
    }

    fn on_configured(&mut self, cfg: u8, ctx: &Ep0Context) {
        if cfg == 0 {
            self.configured = false;
            self.in_busy = false;
            self.out_armed = false;
            self.in_need_zlp = false;
            self.tx_pending = 0;
            return;
        }
        self.speed = ctx.speed;
        let _ = configure_bulk_out_ep(BULK_DATA_EP, BULK_MPS);
        let _ = configure_bulk_in_ep(BULK_DATA_EP, BULK_MPS, 1);
        let _ = configure_intr_in_ep(NOTIFY_EP, 8, 2);
        self.configured = true;
        self.in_busy = false;
        self.out_armed = false;
        self.in_need_zlp = false;
        self.tx_pending = 0;
        crate::usb::log::usb_log_fmt(format_args!(
            "USB-DEV CDC configured: bulk={} mps={} speed={:?}",
            BULK_DATA_EP, BULK_MPS, ctx.speed
        ));
    }

    fn poll(&mut self, _ctx: &Ep0Context) {
        if !self.configured {
            return;
        }

        // Bulk IN：上一笔传输完成 → 解锁；若 last 是 full packet，独立发一个 ZLP
        if self.in_busy {
            let int = read_clear_diepint(BULK_DATA_EP);
            if int & 0x1 != 0 {
                self.in_busy = false;
                if self.in_need_zlp {
                    self.in_need_zlp = false;
                    let pa = crate::usb::platform::usb_dma_phys_for(self.tx_buf.0.as_ptr());
                    if start_bulk_in(BULK_DATA_EP, pa, 0, 1).is_ok() {
                        self.in_busy = true;
                    }
                }
            }
        }

        // Bulk OUT：上一笔接收完成 → 复制到 tx_buf 启动 echo
        //
        // dwc2 在 PKTCNT=1, XFERSIZE=MPS 接收时，host 紧接着发第二个 packet 但
        // 我们的 EP 已 XFERCOMPL → disabled，硬件标记为 BBLEERR (DOEPINT bit 12)
        // 而非 XFERCOMPL。此时 buffer 实际已经收满 1 个 full packet，把 BBLEERR
        // 等同 XFERCOMPL 处理，让 echo + re-prime 继续，host 会重试丢失的第二个
        // packet。
        if self.out_armed {
            let int = read_clear_doepint(BULK_DATA_EP);
            let xfercompl = int & 0x1 != 0;
            let bbl = int & (1 << 12) != 0;
            if xfercompl || bbl {
                self.out_armed = false;
                let original = BULK_BUF_SIZE as u32;
                let r = crate::usb::host::dwc2::mmio::dwc2_regs().unwrap();
                let residual = r.doep[BULK_DATA_EP as usize].doeptsiz.get() & 0x7ffff;
                let mut actual = original.saturating_sub(residual) as usize;
                if bbl && actual == 0 {
                    // BBLEERR 时 residual 也可能未更新；buffer 应该收满。
                    actual = BULK_BUF_SIZE;
                }
                if actual > 0 {
                    unsafe {
                        cache::dcache_invalidate_after_dma(
                            self.rx_buf.0.as_mut_ptr(),
                            BULK_BUF_SIZE,
                        );
                    }
                    let n = actual.min(BULK_BUF_SIZE);
                    self.tx_buf.0[..n].copy_from_slice(&self.rx_buf.0[..n]);
                    self.tx_pending = n;
                    self.try_echo();
                }
            }
        }

        // Bulk OUT prime：半双工 —— IN 还在传时不 prime 新的 OUT，host 端
        // NAK 一两次重试，等 IN 完成再接收。这样 tx_buf 不会在 IN DMA 中被
        // 下一次 OUT XFERCOMPL 的复制覆写。
        if !self.out_armed && !self.in_busy && self.tx_pending == 0 {
            unsafe {
                cache::dcache_invalidate_after_dma(self.rx_buf.0.as_mut_ptr(), BULK_BUF_SIZE);
            }
            let pa = crate::usb::platform::usb_dma_phys_for(self.rx_buf.0.as_ptr());
            let _ = prime_bulk_out(BULK_DATA_EP, pa, BULK_BUF_SIZE as u32, 1);
            self.out_armed = true;
        }
    }
}

impl CdcAcm {
    fn try_echo(&mut self) {
        if self.in_busy || self.tx_pending == 0 {
            return;
        }
        let n = self.tx_pending as u32;
        unsafe {
            cache::dcache_clean_for_dma(self.tx_buf.0.as_ptr(), self.tx_pending);
        }
        let pa = crate::usb::platform::usb_dma_phys_for(self.tx_buf.0.as_ptr());
        // 只发数据 packet 不内嵌 ZLP —— 这颗 dwc2 在 PKTCNT=2/XFERSIZE=64 写法
        // 下不触发 XFERCOMPL 中断，导致 in_busy 永远不释放。完成后由 poll() 里
        // 的 in_need_zlp 路径独立 start_bulk_in(0, 1) 发 ZLP。
        let pkts = n.div_ceil(BULK_MPS).max(1);
        if start_bulk_in(BULK_DATA_EP, pa, n, pkts).is_ok() {
            self.in_busy = true;
            self.in_need_zlp = n > 0 && n % BULK_MPS == 0;
            self.tx_pending = 0;
        }
    }
}
