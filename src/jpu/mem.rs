//! JPU DMA 物理内存分配器。
//!
//! 在静态对齐缓冲上实现 16 KiB 页位图分配；所有 `unsafe` 集中在本模块。

use core::cell::UnsafeCell;

use super::regs::{JPU_DRAM_PHYSICAL_SIZE, VMEM_PAGE_SIZE};

/// 由 JPU 内存池分配的物理地址区间。
#[derive(Clone, Copy, Debug)]
pub struct PhysBuffer {
    pub addr: usize,
    pub size: usize,
}

impl PhysBuffer {
    pub fn is_empty(&self) -> bool {
        self.addr == 0 || self.size == 0
    }
}

struct JpuMemoryPool {
    base_addr: usize,
    size: usize,
    num_pages: usize,
    bitmap: [u64; 32],
}

impl JpuMemoryPool {
    const fn new() -> Self {
        Self {
            base_addr: 0,
            size: 0,
            num_pages: 0,
            bitmap: [0; 32],
        }
    }

    fn init(&mut self, base: usize, size: usize) {
        self.base_addr = (base + VMEM_PAGE_SIZE - 1) & !(VMEM_PAGE_SIZE - 1);
        self.size = size & !(VMEM_PAGE_SIZE - 1);
        self.num_pages = self.size / VMEM_PAGE_SIZE;
        for word in &mut self.bitmap {
            *word = u64::MAX;
        }
    }

    fn alloc(&mut self, size: usize) -> Option<PhysBuffer> {
        let npages = size.div_ceil(VMEM_PAGE_SIZE);
        let mut consecutive = 0usize;
        let mut start_page = 0usize;

        for page_idx in 0..self.num_pages {
            let word_idx = page_idx / 64;
            let bit_idx = page_idx % 64;
            if word_idx >= self.bitmap.len() {
                break;
            }

            if self.bitmap[word_idx] & (1 << bit_idx) != 0 {
                if consecutive == 0 {
                    start_page = page_idx;
                }
                consecutive += 1;
                if consecutive >= npages {
                    for i in 0..npages {
                        let p = start_page + i;
                        self.bitmap[p / 64] &= !(1 << (p % 64));
                    }
                    let addr = self.base_addr + start_page * VMEM_PAGE_SIZE;
                    return Some(PhysBuffer {
                        addr,
                        size: npages * VMEM_PAGE_SIZE,
                    });
                }
            } else {
                consecutive = 0;
            }
        }
        None
    }

    fn free(&mut self, buf: PhysBuffer) {
        if buf.is_empty() || buf.addr < self.base_addr || buf.addr >= self.base_addr + self.size {
            return;
        }
        let start_page = (buf.addr - self.base_addr) / VMEM_PAGE_SIZE;
        let npages = buf.size.div_ceil(VMEM_PAGE_SIZE);
        for i in 0..npages {
            let p = start_page + i;
            if p >= self.num_pages {
                break;
            }
            self.bitmap[p / 64] |= 1 << (p % 64);
        }
    }
}

struct SyncUnsafeCell<T>(UnsafeCell<T>);
// 裸机单核环境：JPU 驱动仅在 bring-up 线程中使用静态池。
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY: 单核裸机上下文，无并发访问静态池。
        unsafe { f(&mut *self.0.get()) }
    }
}

#[repr(C, align(4096))]
struct AlignedMem<const N: usize>([u8; N]);

struct JpuMemState {
    pool: JpuMemoryPool,
    initialized: bool,
}

impl JpuMemState {
    const fn new() -> Self {
        Self {
            pool: JpuMemoryPool::new(),
            initialized: false,
        }
    }
}

static MEM_STATE: SyncUnsafeCell<JpuMemState> = SyncUnsafeCell::new(JpuMemState::new());
static DMA_BUFFER: SyncUnsafeCell<AlignedMem<{ JPU_DRAM_PHYSICAL_SIZE }>> =
    SyncUnsafeCell::new(AlignedMem([0u8; JPU_DRAM_PHYSICAL_SIZE]));

/// 初始化 DMA 内存池（在 `JpuDecoder::init` 最开始调用）。
pub fn init_jpu_memory() {
    MEM_STATE.with_mut(|state| {
        if state.initialized {
            return;
        }
        let buf_addr = DMA_BUFFER.with_mut(|buf| buf.0.as_ptr() as usize);
        state.pool.init(buf_addr, JPU_DRAM_PHYSICAL_SIZE);
        state.initialized = true;
    });
}

pub fn jpu_alloc(size: usize) -> Option<PhysBuffer> {
    MEM_STATE.with_mut(|state| {
        if !state.initialized {
            None
        } else {
            state.pool.alloc(size)
        }
    })
}

pub fn jpu_free(buf: PhysBuffer) {
    if buf.is_empty() {
        return;
    }
    MEM_STATE.with_mut(|state| {
        if state.initialized {
            state.pool.free(buf);
        }
    });
}

/// 将 JPEG bitstream 拷贝到已分配的 stream 物理缓冲。
pub fn copy_to_phys(buf: PhysBuffer, src: &[u8]) {
    let len = src.len().min(buf.size);
    if len == 0 {
        return;
    }
    let dst = phys_slice_mut(buf.addr, len);
    dst.copy_from_slice(&src[..len]);
}

/// 读取帧缓冲中的 YUV 数据（调用方需保证 `[addr, addr+len)` 仍在分配期内）。
pub fn phys_slice(addr: usize, len: usize) -> &'static [u8] {
    // SAFETY: `addr`/`len` 来自本模块分配且尚未 free。
    unsafe { core::slice::from_raw_parts(addr as *const u8, len) }
}

fn phys_slice_mut(addr: usize, len: usize) -> &'static mut [u8] {
    // SAFETY: 同上，写入 stream 缓冲。
    unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, len) }
}
