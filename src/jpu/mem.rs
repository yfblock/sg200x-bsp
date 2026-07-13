//! JPU DMA 物理内存分配器。
//!
//! 在静态对齐缓冲上实现 16 KiB 页位图分配；所有 `unsafe` 集中在本模块。

use core::cell::UnsafeCell;

use super::regs::{JPU_DRAM_PHYSICAL_SIZE, VMEM_PAGE_SIZE};

/// 由 JPU 内存池分配的物理地址区间。
#[derive(Debug)]
pub struct PhysBuffer {
    pub addr: usize,
    pub size: usize,
}

impl PhysBuffer {
    pub const EMPTY: Self = Self { addr: 0, size: 0 };

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
        let Some(aligned_base) = base
            .checked_add(VMEM_PAGE_SIZE - 1)
            .map(|value| value & !(VMEM_PAGE_SIZE - 1))
        else {
            self.base_addr = 0;
            self.size = 0;
            self.num_pages = 0;
            self.bitmap.fill(0);
            return;
        };
        let alignment_prefix = aligned_base - base;
        let usable_size = size.saturating_sub(alignment_prefix);
        let available_pages = usable_size / VMEM_PAGE_SIZE;
        let bitmap_pages = self.bitmap.len() * u64::BITS as usize;

        self.base_addr = aligned_base;
        self.num_pages = available_pages.min(bitmap_pages);
        self.size = self.num_pages * VMEM_PAGE_SIZE;
        for word in &mut self.bitmap {
            *word = u64::MAX;
        }
    }

    fn alloc(&mut self, size: usize) -> Option<PhysBuffer> {
        if size == 0 {
            return None;
        }
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

    fn alloc_pair(
        &mut self,
        first_size: usize,
        second_size: usize,
    ) -> Option<(PhysBuffer, PhysBuffer)> {
        let first = self.alloc(first_size)?;
        match self.alloc(second_size) {
            Some(second) => Some((first, second)),
            None => {
                self.free(first);
                None
            }
        }
    }

    fn free(&mut self, buf: PhysBuffer) {
        let Some(pool_end) = self.base_addr.checked_add(self.size) else {
            return;
        };
        let Some(buffer_end) = buf.addr.checked_add(buf.size) else {
            return;
        };
        if buf.is_empty()
            || buf.addr < self.base_addr
            || buffer_end > pool_end
            || !(buf.addr - self.base_addr).is_multiple_of(VMEM_PAGE_SIZE)
            || !buf.size.is_multiple_of(VMEM_PAGE_SIZE)
        {
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
// SAFETY: decoder.rs admits one live JPU owner with JPU_IN_USE before any
// memory entry point is called. Decode requires &mut JpuDecoder, so pool
// operations cannot overlap even when that owner moves between cores.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY: the singleton decoder ownership described above serializes
        // every access to this cell.
        unsafe { f(&mut *self.0.get()) }
    }
}

#[repr(C, align(16384))]
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

pub fn jpu_alloc_pair(first_size: usize, second_size: usize) -> Option<(PhysBuffer, PhysBuffer)> {
    MEM_STATE.with_mut(|state| {
        if !state.initialized {
            None
        } else {
            state.pool.alloc_pair(first_size, second_size)
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
pub fn copy_to_phys(buf: &PhysBuffer, src: &[u8]) -> Result<(), &'static str> {
    if src.len() > buf.size {
        return Err("JPU stream buffer is too small");
    }
    if src.is_empty() {
        return Ok(());
    }
    // SAFETY: the buffer is allocated and exclusively owned by the decoder;
    // the length check above keeps the write inside that allocation.
    let dst = unsafe { core::slice::from_raw_parts_mut(buf.addr as *mut u8, src.len()) };
    dst.copy_from_slice(src);
    Ok(())
}

/// Zero a subrange that the JPU may prefetch beyond the logical bitstream end.
pub fn zero_phys_range(buf: &PhysBuffer, offset: usize, len: usize) -> Result<(), &'static str> {
    let end = offset
        .checked_add(len)
        .ok_or("JPU stream zero range overflow")?;
    if end > buf.size {
        return Err("JPU stream zero range exceeds its allocation");
    }
    if len == 0 {
        return Ok(());
    }
    // SAFETY: the decoder exclusively owns the stream buffer and the checked
    // range is contained in its allocation.
    let range = unsafe { core::slice::from_raw_parts_mut((buf.addr + offset) as *mut u8, len) };
    range.fill(0);
    Ok(())
}

/// 读取帧缓冲中的 YUV 数据（调用方需保证 `[addr, addr+len)` 仍在分配期内）。
pub fn phys_slice(buf: &PhysBuffer, len: usize) -> Result<&[u8], &'static str> {
    if len > buf.size {
        return Err("JPU frame view exceeds its allocation");
    }
    // SAFETY: `buf` is allocated and remains borrowed for the returned
    // lifetime; the length check keeps the view inside that allocation.
    Ok(unsafe { core::slice::from_raw_parts(buf.addr as *const u8, len) })
}

#[cfg(test)]
mod tests {
    use super::{
        JpuMemoryPool, PhysBuffer, VMEM_PAGE_SIZE, copy_to_phys, phys_slice, zero_phys_range,
    };

    #[repr(C, align(16384))]
    struct TestMemory([u8; VMEM_PAGE_SIZE * 2]);

    #[test]
    fn init_excludes_alignment_prefix_from_usable_range() {
        let original_base = 0x1003usize;
        let original_size = VMEM_PAGE_SIZE * 3;
        let mut pool = JpuMemoryPool::new();
        pool.init(original_base, original_size);

        assert_eq!(pool.base_addr, VMEM_PAGE_SIZE);
        assert_eq!(pool.size, VMEM_PAGE_SIZE * 2);
        assert_eq!(pool.num_pages, 2);
        assert!(pool.base_addr + pool.size <= original_base + original_size);
    }

    #[test]
    fn init_caps_capacity_to_bitmap() {
        let mut pool = JpuMemoryPool::new();
        let bitmap_pages = pool.bitmap.len() * u64::BITS as usize;
        pool.init(0x4000, (bitmap_pages + 10) * VMEM_PAGE_SIZE);

        assert_eq!(pool.num_pages, bitmap_pages);
        assert_eq!(pool.size, bitmap_pages * VMEM_PAGE_SIZE);
    }

    #[test]
    fn alloc_rejects_zero_and_rounds_up_to_pages() {
        let mut pool = JpuMemoryPool::new();
        pool.init(0x4000, VMEM_PAGE_SIZE * 2);

        assert!(pool.alloc(0).is_none());
        let buffer = pool.alloc(VMEM_PAGE_SIZE + 1).expect("two pages fit");
        assert_eq!(buffer.addr, 0x4000);
        assert_eq!(buffer.size, VMEM_PAGE_SIZE * 2);
        assert!(pool.alloc(1).is_none());
    }

    #[test]
    fn allocator_reuses_a_freed_contiguous_run() {
        let mut pool = JpuMemoryPool::new();
        pool.init(0x4000, VMEM_PAGE_SIZE * 4);

        let first = pool.alloc(VMEM_PAGE_SIZE * 2).expect("first run");
        let first_addr = first.addr;
        let second = pool.alloc(VMEM_PAGE_SIZE * 2).expect("second run");
        assert_eq!(second.addr, first_addr + VMEM_PAGE_SIZE * 2);
        pool.free(first);

        let reused = pool.alloc(VMEM_PAGE_SIZE * 2).expect("freed run");
        assert_eq!(reused.addr, first_addr);
    }

    #[test]
    fn pair_allocation_rolls_back_when_second_buffer_does_not_fit() {
        let mut pool = JpuMemoryPool::new();
        pool.init(0x4000, VMEM_PAGE_SIZE * 3);

        assert!(
            pool.alloc_pair(VMEM_PAGE_SIZE * 2, VMEM_PAGE_SIZE * 2)
                .is_none()
        );
        let whole_pool = pool
            .alloc(VMEM_PAGE_SIZE * 3)
            .expect("failed pair must release its first allocation");
        assert_eq!(whole_pool.addr, 0x4000);
        assert_eq!(whole_pool.size, VMEM_PAGE_SIZE * 3);
    }

    #[test]
    fn half_scale_large_fixture_and_actual_stream_fit_one_mib_pool() {
        let layout = crate::jpu::FrameLayout::new(
            1279,
            1706,
            crate::jpu::JpuPixelFormat::Yuv420,
            crate::jpu::JpuScale::Half,
        )
        .expect("valid half-scale layout");
        let mut pool = JpuMemoryPool::new();
        pool.init(0x4000, 1024 * 1024);

        let (frame, stream) = pool
            .alloc_pair(layout.total_len, 110 * 1024)
            .expect("half-scale frame and representative JPEG stream fit");
        assert_eq!(frame.size, 51 * VMEM_PAGE_SIZE);
        assert_eq!(stream.size, 7 * VMEM_PAGE_SIZE);
        assert!(frame.size + stream.size <= 1024 * 1024);
    }

    #[test]
    fn exact_copy_rejects_oversize_without_modifying_memory() {
        let mut memory = TestMemory([0xA5; VMEM_PAGE_SIZE * 2]);
        let buffer = PhysBuffer {
            addr: memory.0.as_mut_ptr() as usize,
            size: VMEM_PAGE_SIZE,
        };

        assert!(copy_to_phys(&buffer, &[0x11; VMEM_PAGE_SIZE + 1]).is_err());
        assert!(memory.0.iter().all(|byte| *byte == 0xA5));
    }

    #[test]
    fn exact_copy_and_borrowed_slice_stay_within_buffer() {
        let mut memory = TestMemory([0; VMEM_PAGE_SIZE * 2]);
        let buffer = PhysBuffer {
            addr: memory.0.as_mut_ptr() as usize,
            size: VMEM_PAGE_SIZE,
        };
        let input = [1, 2, 3, 4, 5];

        copy_to_phys(&buffer, &input).expect("exact copy fits");
        assert_eq!(
            phys_slice(&buffer, input.len()).expect("bounded view"),
            input
        );
        assert!(phys_slice(&buffer, VMEM_PAGE_SIZE + 1).is_err());
    }

    #[test]
    fn zero_range_clears_prefetch_tail_and_checks_bounds() {
        let mut memory = TestMemory([0xA5; VMEM_PAGE_SIZE * 2]);
        let buffer = PhysBuffer {
            addr: memory.0.as_mut_ptr() as usize,
            size: VMEM_PAGE_SIZE,
        };

        zero_phys_range(&buffer, VMEM_PAGE_SIZE - 8, 8).expect("tail is in bounds");
        assert!(
            memory.0[..VMEM_PAGE_SIZE - 8]
                .iter()
                .all(|byte| *byte == 0xA5)
        );
        assert_eq!(&memory.0[VMEM_PAGE_SIZE - 8..VMEM_PAGE_SIZE], &[0; 8]);
        assert!(zero_phys_range(&buffer, VMEM_PAGE_SIZE - 7, 8).is_err());
    }
}
