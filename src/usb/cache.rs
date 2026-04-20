//! DMA 与 CPU D-Cache 一致性维护：AArch64 按行；**RISC-V T-Head C906**（SG2002 大核）按全清+无效粗粒度。

#[cfg(target_arch = "aarch64")]
unsafe fn dcache_op_range(mut addr: usize, end: usize, op: DcacheOp) {
    const LINE: usize = 64;
    addr &= !(LINE - 1);
    while addr < end {
        let a = addr as u64;
        unsafe {
            match op {
                DcacheOp::Clean => core::arch::asm!("dc cvac, {}", in(reg) a, options(nostack)),
                DcacheOp::Invalidate => core::arch::asm!("dc ivac, {}", in(reg) a, options(nostack)),
            }
        }
        addr = addr.saturating_add(LINE);
    }
    unsafe {
        core::arch::asm!("dsb sy", options(nostack));
    }
}

#[cfg(target_arch = "aarch64")]
#[derive(Clone, Copy)]
enum DcacheOp {
    Clean,
    Invalidate,
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
#[cfg(target_arch = "riscv64")]
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
#[cfg(target_arch = "riscv64")]
pub unsafe fn dcache_clean_for_dma(_ptr: *const u8, _len: usize) {
    let _ = (_ptr, _len);
    unsafe { riscv64_c906_dcache_ciall() }
}

#[cfg(target_arch = "riscv64")]
pub unsafe fn dcache_invalidate_after_dma(_ptr: *mut u8, _len: usize) {
    let _ = (_ptr, _len);
    unsafe { riscv64_c906_dcache_ciall() }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
pub unsafe fn dcache_clean_for_dma(_ptr: *const u8, _len: usize) {}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
pub unsafe fn dcache_invalidate_after_dma(_ptr: *mut u8, _len: usize) {}
