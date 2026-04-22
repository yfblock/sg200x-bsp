//! D-cache / DMA 一致性维护。
//!
//! 本模块同时提供两组语义不同的 API，给不同外设使用：
//!
//! 1. **按 cache line 精细维护**（[`dcache_clean_range`] / [`dcache_invalidate_range`]）：
//!    - 仅作用于 `[start, start+size)` 覆盖到的若干 64B 行。
//!    - 适合频繁、细粒度的 DMA 描述符 / 数据缓冲一致性，例如以太网 RX/TX ring。
//!
//! 2. **粗粒度 DMA 一致性**（[`dcache_clean_for_dma`] / [`dcache_invalidate_after_dma`]）：
//!    - AArch64 走标准 `dc cvac` / `dc ivac` + `dsb sy`，按行精确。
//!    - 适合 USB DWC2 这种缓冲生命周期复杂、需要"做了肯定有效"语义的子系统。
//!
//! # 平台与 feature 矩阵
//!
//! | 目标             | feature `c906`  | 实际行为                                                    |
//! |------------------|-----------------|------------------------------------------------------------|
//! | `riscv64` SG2002 | **on (默认)**   | 使用 T-Head C906 非标指令 `dcache.cva/iva/ciall`            |
//! | `riscv64` 其它   | off             | 退化为 `fence iorw, iorw` —— **不做** line clean/invalidate |
//! | `aarch64`        | n/a             | 标准 `dc cvac/ivac` + `dsb sy`                              |
//! | 其它 ISA         | n/a             | 全部退化为空操作                                             |
//!
//! C906 非标指令编码与 OpenSBI / U-Boot `t-head_cache.S` 一致：
//! - `dcache.cva  va`：funct12=0x025
//! - `dcache.iva  va`：funct12=0x026
//! - `dcache.ciall`：编码 `0x0030000b`
//!
//! 在 SG2002 上必须开 `c906`；关闭后只能保证内存序，**不能保证** DMA 看到的数据是 cache 之外的最新值。

#[allow(dead_code)]
const CACHE_LINE: usize = 64;

// 与标准 RISC-V Zicbom 缓存指令冲突时给出明确报错——zicbom 与 C906 自定义指令编码空间重叠，
// 同时打开会导致 binutils 解码歧义。
#[cfg(all(
    target_arch = "riscv64",
    target_feature = "zicbom",
    feature = "c906"
))]
compile_error!(
    "feature `c906` 与 RISC-V `zicbom` 标准缓存指令冲突，请二选一（C906 仅支持自定义编码）。"
);

// ============================================================================
// (1) 按行精细维护：dcache.cva / dcache.iva（C906 非标指令）
// ============================================================================

#[cfg(all(target_arch = "riscv64", feature = "c906"))]
#[inline(always)]
unsafe fn dcache_cva(va: usize) {
    unsafe {
        core::arch::asm!(".insn i 0x0b, 0, x0, {0}, 0x025", in(reg) va);
    }
}

#[cfg(all(target_arch = "riscv64", feature = "c906"))]
#[inline(always)]
unsafe fn dcache_iva(va: usize) {
    unsafe {
        core::arch::asm!(".insn i 0x0b, 0, x0, {0}, 0x026", in(reg) va);
    }
}

#[cfg(not(all(target_arch = "riscv64", feature = "c906")))]
#[inline(always)]
#[allow(dead_code)]
unsafe fn dcache_cva(_va: usize) {}

#[cfg(not(all(target_arch = "riscv64", feature = "c906")))]
#[inline(always)]
#[allow(dead_code)]
unsafe fn dcache_iva(_va: usize) {}

/// 把 `[start, start+size)` 之内的所有缓存行 **clean**（写回到内存），
/// 用于 CPU 写完 DMA 描述符 / TX 数据后、把寄存器交给 DMA 之前。
#[inline]
pub fn dcache_clean_range(start: usize, size: usize) {
    if size == 0 {
        return;
    }
    #[cfg(all(target_arch = "riscv64", feature = "c906"))]
    {
        let mut addr = start & !(CACHE_LINE - 1);
        let end = start + size;
        while addr < end {
            unsafe { dcache_cva(addr) };
            addr += CACHE_LINE;
        }
        unsafe { core::arch::asm!("fence iorw, iorw") };
    }
    #[cfg(all(target_arch = "riscv64", not(feature = "c906")))]
    {
        let _ = (start, size);
        unsafe { core::arch::asm!("fence iorw, iorw") };
    }
    #[cfg(not(target_arch = "riscv64"))]
    {
        let _ = (start, size);
    }
}

/// 把 `[start, start+size)` 之内的所有缓存行 **invalidate**（丢掉脏数据，
/// 强制下次读回内存），用于 DMA 写完 RX 帧、CPU 读取之前。
#[inline]
pub fn dcache_invalidate_range(start: usize, size: usize) {
    if size == 0 {
        return;
    }
    #[cfg(all(target_arch = "riscv64", feature = "c906"))]
    {
        let mut addr = start & !(CACHE_LINE - 1);
        let end = start + size;
        while addr < end {
            unsafe { dcache_iva(addr) };
            addr += CACHE_LINE;
        }
        unsafe { core::arch::asm!("fence iorw, iorw") };
    }
    #[cfg(all(target_arch = "riscv64", not(feature = "c906")))]
    {
        let _ = (start, size);
        unsafe { core::arch::asm!("fence iorw, iorw") };
    }
    #[cfg(not(target_arch = "riscv64"))]
    {
        let _ = (start, size);
    }
}

// ============================================================================
// (2) 粗粒度 DMA 一致性：AArch64 按行 / C906 全清
// ============================================================================

#[cfg(target_arch = "aarch64")]
#[derive(Clone, Copy)]
enum DcacheOp {
    Clean,
    Invalidate,
}

#[cfg(target_arch = "aarch64")]
unsafe fn dcache_op_range(mut addr: usize, end: usize, op: DcacheOp) {
    addr &= !(CACHE_LINE - 1);
    while addr < end {
        let a = addr as u64;
        unsafe {
            match op {
                DcacheOp::Clean => core::arch::asm!("dc cvac, {}", in(reg) a, options(nostack)),
                DcacheOp::Invalidate => core::arch::asm!("dc ivac, {}", in(reg) a, options(nostack)),
            }
        }
        addr = addr.saturating_add(CACHE_LINE);
    }
    unsafe {
        core::arch::asm!("dsb sy", options(nostack));
    }
}

/// DMA 从内存读走前：clean（或 clean+invalidate）。
#[cfg(target_arch = "aarch64")]
pub unsafe fn dcache_clean_for_dma(ptr: *const u8, len: usize) {
    if len == 0 {
        return;
    }
    unsafe { dcache_op_range(ptr as usize, ptr as usize + len, DcacheOp::Clean) }
}

/// DMA 向内存写入后：invalidate，再由 CPU 读。
#[cfg(target_arch = "aarch64")]
pub unsafe fn dcache_invalidate_after_dma(ptr: *mut u8, len: usize) {
    if len == 0 {
        return;
    }
    unsafe { dcache_op_range(ptr as usize, ptr as usize + len, DcacheOp::Invalidate) }
}

/// T-Head C906：`dcache.ciall` 清洗并无效全部 D-Cache（与 ArceOS `dma.md` 示例一致）。
#[cfg(all(target_arch = "riscv64", feature = "c906"))]
#[inline(always)]
unsafe fn riscv64_c906_dcache_ciall() {
    unsafe {
        core::arch::asm!(
            ".long 0x0030000b",
            "fence rw, rw",
            options(nostack),
        );
    }
}

/// USB DWC2 通过 `HCDMA` 访问内存：缓冲区须在 **DMA 可见** 的相干视图上；C906 上须 flush。
#[cfg(all(target_arch = "riscv64", feature = "c906"))]
pub unsafe fn dcache_clean_for_dma(_ptr: *const u8, _len: usize) {
    let _ = (_ptr, _len);
    unsafe { riscv64_c906_dcache_ciall() }
}

#[cfg(all(target_arch = "riscv64", feature = "c906"))]
pub unsafe fn dcache_invalidate_after_dma(_ptr: *mut u8, _len: usize) {
    let _ = (_ptr, _len);
    unsafe { riscv64_c906_dcache_ciall() }
}

/// 非 C906 RISC-V 核：无法精确 line clean，仅做 `fence rw, rw` 保证内存序。
/// 上层若依赖 cache 一致性必须自行采用 nocache 映射或 IOMMU。
#[cfg(all(target_arch = "riscv64", not(feature = "c906")))]
pub unsafe fn dcache_clean_for_dma(_ptr: *const u8, _len: usize) {
    unsafe { core::arch::asm!("fence rw, rw", options(nostack)) }
}

#[cfg(all(target_arch = "riscv64", not(feature = "c906")))]
pub unsafe fn dcache_invalidate_after_dma(_ptr: *mut u8, _len: usize) {
    unsafe { core::arch::asm!("fence rw, rw", options(nostack)) }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
pub unsafe fn dcache_clean_for_dma(_ptr: *const u8, _len: usize) {}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
pub unsafe fn dcache_invalidate_after_dma(_ptr: *mut u8, _len: usize) {}
