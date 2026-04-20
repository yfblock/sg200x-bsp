//! USB 行缓冲日志：通过 `fn(&str)` 回调输出（由应用注册，例如接到 `println!`）。

use core::fmt::{self, Write};

/// 单行输出回调（不含换行则由缓冲拼到出现 `\n` 或 [`usb_log_flush_residual`] 时输出）。
pub type UsbLogFn = fn(&str);

static mut G_LOG: Option<UsbLogFn> = None;

const LOG_CAP: usize = 512;
static mut LOG_LINE: [u8; LOG_CAP] = [0u8; LOG_CAP];
static mut LOG_LEN: usize = 0;

/// 注册 USB 栈日志回调（应在枚举前调用）。
pub fn set_usb_log_fn(f: UsbLogFn) {
    unsafe {
        G_LOG = Some(f);
    }
}

fn flush_line_buf() {
    unsafe {
        if LOG_LEN == 0 {
            return;
        }
        if let Ok(st) = core::str::from_utf8(&LOG_LINE[..LOG_LEN]) {
            if let Some(f) = G_LOG {
                f(st);
            }
        }
        LOG_LEN = 0;
    }
}

/// 输出缓冲区中尚未以 `\n` 结尾的残留文本（拓扑扫描结束时应调用）。
pub fn usb_log_flush_residual() {
    flush_line_buf();
}

/// 立即输出一行（先清空行缓冲，避免与 [`LineBufferedUsbLog`] 交错）。
pub fn usb_log_fmt(args: fmt::Arguments<'_>) {
    let mut tmp = [0u8; 224];
    struct Buf<'a> {
        b: &'a mut [u8],
        n: usize,
    }
    impl Write for Buf<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let take = s.len().min(self.b.len().saturating_sub(self.n));
            self.b[self.n..self.n + take].copy_from_slice(&s.as_bytes()[..take]);
            self.n += take;
            Ok(())
        }
    }
    let mut w = Buf { b: &mut tmp, n: 0 };
    let _ = fmt::write(&mut w, args);
    let n = w.n;
    flush_line_buf();
    unsafe {
        if let Some(f) = G_LOG {
            if let Ok(st) = core::str::from_utf8(&tmp[..n]) {
                f(st);
            }
        }
    }
}

/// 与 `core::fmt::Write` 配合使用；按行聚合后调用全局日志回调。
pub struct LineBufferedUsbLog;

impl Write for LineBufferedUsbLog {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            for &b in s.as_bytes() {
                if b == b'\n' {
                    if LOG_LEN > 0 {
                        if let Ok(st) = core::str::from_utf8(&LOG_LINE[..LOG_LEN]) {
                            if let Some(f) = G_LOG {
                                f(st);
                            }
                        }
                        LOG_LEN = 0;
                    }
                } else if LOG_LEN < LOG_CAP {
                    LOG_LINE[LOG_LEN] = b;
                    LOG_LEN += 1;
                }
            }
        }
        Ok(())
    }
}
