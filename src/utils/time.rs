//! 基于 `rdtime` 的时间基准与超时判定。
//!
//! 为什么需要：外设等待循环若按"轮询次数"设上限，实际超时时长取决于核心主频
//! 和循环体开销，无法预期。SG2002 上 JPU 的 `poll_decode_done` 就吃过这个亏——
//! 内层 1000 次 `spin_loop` 在小核 C906L 上一轮要 ~461us，500k 轮的上限实测要
//! **230 秒**才走完；JPU 一挂整个核就被堵这么久，外部看起来像彻底死机。
//! 按时间判定则与主频无关，超时时长可预期。

/// `rdtime`（mtime）计数频率。SG2002 为 25 MHz——实测 25.005 MHz
/// （小核连续采样 `rdtime`，300,333,869 ticks / 12.011 s）。
pub const TIMEBASE_HZ: u64 = 25_000_000;

/// 读 64 位 `rdtime`。非 riscv64 平台返回 `None`（调用方需退化到次数上限）。
#[inline]
pub fn now() -> Option<u64> {
    #[cfg(target_arch = "riscv64")]
    {
        let t: usize;
        // `rdtime` 在 M-mode / S-mode 都可读（C906 已验证）。
        unsafe { core::arch::asm!("rdtime {0}", out(reg) t, options(nomem, nostack)) };
        Some(t as u64)
    }
    #[cfg(not(target_arch = "riscv64"))]
    {
        None
    }
}

/// 超时判定。没有可用时间源时 `expired()` 恒为 `false`，
/// 调用方必须同时保留一个次数上限作为兜底。
#[derive(Clone, Copy)]
pub struct Deadline {
    end: u64,
    valid: bool,
}

impl Deadline {
    /// 从当前时刻起 `ms` 毫秒后到期。
    #[inline]
    pub fn after_ms(ms: u64) -> Self {
        match now() {
            Some(t) => Self {
                end: t.wrapping_add(ms * (TIMEBASE_HZ / 1000)),
                valid: true,
            },
            None => Self { end: 0, valid: false },
        }
    }

    /// 是否已到期。用 wrapping 差值比较，天然容忍计数器回绕。
    #[inline]
    pub fn expired(&self) -> bool {
        if !self.valid {
            return false;
        }
        match now() {
            // (now - end) 的最高位为 0 表示 now 已追上/超过 end
            Some(t) => (t.wrapping_sub(self.end) as i64) >= 0,
            None => false,
        }
    }
}

/// 忙等 `ms` 毫秒（有时间源时精确，否则退化为 NOP 循环）。
#[inline]
pub fn spin_ms(ms: u64) {
    let dl = Deadline::after_ms(ms);
    if now().is_none() {
        // 没有时间源：粗略估一个 NOP 数
        super::delay::delay(ms as usize * 10_000);
        return;
    }
    while !dl.expired() {
        core::hint::spin_loop();
    }
}
