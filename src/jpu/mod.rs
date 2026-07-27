//! SG2002 JPU（JPEG Processing Unit）纯 Rust 驱动。
//!
//! 对照 U-Boot CVitek 驱动实现（`drivers/jpeg/`），在裸机上以轮询方式完成
//! Baseline JPEG 硬件解码，输出 YUV420 planar。

mod decoder;
mod header;
pub mod mem;
pub mod regs;

pub use decoder::{DecodeResult, JpuDecoder, JpuDmaToPhysFn, JpuMmio};

/// 解码进度 trace：`decode()` 每走一步就把步号写到调用方指定的物理地址。
///
/// 为什么需要：JPU 在连续解码若干帧后会卡住且 `decode()` 永不返回，而驱动里
/// 唯一的循环（`poll_decode_done`）是有上限的——必须能看到究竟停在哪一步。
/// 裸机小核不能靠串口（和大核共用），所以把步号写进共享 DRAM 由另一核读。
///
/// 默认 0 = 关闭，无开销。调用方设成一个可写物理地址即启用。
pub mod trace {
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub static TRACE_ADDR: AtomicUsize = AtomicUsize::new(0);

    /// 步号：见 `decoder::decode()` 里的调用点。
    pub mod step {
        pub const ENTER: u32 = 1;
        pub const PARSE_HEADER: u32 = 2;
        pub const COPY_STREAM: u32 = 3;
        pub const CLEAN_STREAM: u32 = 4;
        pub const FRAME_LAYOUT: u32 = 5;
        pub const FREE_FRAME: u32 = 6;
        pub const ALLOC_FRAME: u32 = 7;
        pub const INV_FRAME: u32 = 8;
        pub const CFG_STREAM_REGS: u32 = 9;
        pub const HUFF: u32 = 10;
        pub const QUANT: u32 = 11;
        pub const GRAM: u32 = 12;
        pub const START_DECODE: u32 = 13;
        pub const POLL: u32 = 14;
        pub const INV_AFTER: u32 = 15;
        pub const DONE: u32 = 16;
    }

    /// 启用 trace，把步号写到 `pa`（须为可写、两核都能看到的物理地址）。
    pub fn set_addr(pa: usize) {
        TRACE_ADDR.store(pa, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn mark(step: u32) {
        let a = TRACE_ADDR.load(Ordering::Relaxed);
        if a != 0 {
            unsafe { core::ptr::write_volatile(a as *mut u32, step) };
        }
    }

    /// 在轮询循环里更新进度：`+8` 写轮次，`+12` 写 `rdtime` 低 32 位。
    /// 卡住时靠这两个字段判断"到底在推进吗、推进多快"——主循环被堵住时
    /// 外部只能看到冻结的计数器，没有这个就分不清死循环和慢循环。
    /// （`+4` 属于调用方，别碰。）
    #[inline]
    pub(crate) fn mark_poll(count: u32) {
        let a = TRACE_ADDR.load(Ordering::Relaxed);
        if a != 0 {
            unsafe {
                core::ptr::write_volatile((a + 8) as *mut u32, count);
                #[cfg(target_arch = "riscv64")]
                {
                    let t: usize;
                    core::arch::asm!("rdtime {0}", out(reg) t, options(nomem, nostack));
                    core::ptr::write_volatile((a + 12) as *mut u32, t as u32);
                }
            }
        }
    }
}
