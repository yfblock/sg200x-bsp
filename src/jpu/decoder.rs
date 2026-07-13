//! JPU 硬件 JPEG 解码器（Baseline，轮询模式）。

use core::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};

use super::engine::{
    BBC_STREAM_PAGE_SIZE, GRAM_PREFETCH_PAGES, HardwareDecodeInfo, PollError, checked_dma_offset,
    checked_dma_region, checked_frame_dma_addresses, configure_stream_regs, gram_setup,
    poll_decode_done, start_decode, upload_huff_tables, upload_quant_tables,
};
use super::header::parse_jpeg_header;
use super::layout::{FrameLayout, FrameLayoutError, JpuPixelFormat, JpuScale, PlaneLayout};
use super::mem::{
    PhysBuffer, copy_to_phys, init_jpu_memory, jpu_alloc_pair, jpu_free, phys_slice,
    zero_phys_range,
};
use super::regs::{JPU_REG_BASE, VC_REG_BASE, hardware_init_at};
use crate::soc::TOP_BASE;
use crate::utils::cache::{dcache_clean_range, dcache_invalidate_range};

static JPU_IN_USE: AtomicBool = AtomicBool::new(false);

/// A borrowed planar frame produced by the JPU.
///
/// The borrow prevents a second decode or decoder drop while `yuv_data` is in
/// use. The buffer can be reused after this value is dropped.
///
/// ```compile_fail
/// use sg200x_bsp::jpu::JpuDecoder;
///
/// fn cannot_decode_twice(decoder: &mut JpuDecoder, jpeg: &[u8]) {
///     let first = decoder.decode(jpeg).unwrap();
///     let _second = decoder.decode(jpeg).unwrap();
///     let _still_borrowed = first.yuv_data;
/// }
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub struct DecodeResult<'a> {
    /// Meaningful output width after scaling, excluding coded padding.
    pub width: u32,
    /// Meaningful output height after scaling, excluding coded padding.
    pub height: u32,
    /// CPU-visible frame bytes. Plane offsets and strides are in `layout`.
    pub yuv_data: &'a [u8],
    /// DMA address corresponding to `yuv_data[0]`.
    ///
    /// It is valid only for this result's borrow. Do not cache it after the
    /// result is dropped or use it after another decode.
    pub yuv_phys_addr: usize,
    /// Complete planar format, scale, extent, offset, and stride description.
    pub layout: FrameLayout,
}

/// Error returned by the typed scaled-decode entry point.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JpuDecodeError {
    /// The decoder did not finish initialization.
    NotInitialized,
    /// A previous timeout left DMA completion unknown; reboot is required.
    Poisoned,
    /// The compressed input is empty.
    EmptyStream,
    /// JPEG marker or table parsing failed.
    InvalidJpeg(&'static str),
    /// The requested scale or planar layout cannot be represented safely.
    Layout(FrameLayoutError),
    /// The 1 MiB JPU pool cannot hold both stream and frame buffers.
    MemoryPoolExhausted,
    /// An internal buffer length or padding invariant was violated.
    BufferInvariant(&'static str),
    /// A mapped DMA address cannot be represented by the JPU registers.
    DmaAddress(&'static str),
    /// Hardware table or GRAM setup failed before decode start.
    HardwareSetup(&'static str),
    /// The JPU reported a decode error.
    DecodeFailed,
    /// The JPU did not reach a terminal state before the poll limit.
    Timeout,
}

impl JpuDecodeError {
    const fn as_static_str(self) -> &'static str {
        match self {
            Self::NotInitialized => "JPU not initialized",
            Self::Poisoned => "JPU decoder is poisoned after a timeout; reboot is required",
            Self::EmptyStream => "JPEG stream is empty",
            Self::InvalidJpeg(message)
            | Self::BufferInvariant(message)
            | Self::DmaAddress(message)
            | Self::HardwareSetup(message) => message,
            Self::Layout(error) => layout_error_message(error),
            Self::MemoryPoolExhausted => "JPU memory pool cannot fit stream and frame buffers",
            Self::DecodeFailed => "JPU decode error",
            Self::Timeout => "JPU decode timeout; decoder is poisoned until reboot",
        }
    }
}

impl fmt::Display for JpuDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_static_str())
    }
}

/// JPU MMIO 基址（与 [`crate::gpio::GPIO::new`] 相同：由板级传入已映射地址）。
#[derive(Clone, Copy, Debug)]
pub struct JpuMmio {
    pub jpu_base: usize,
    pub top_base: usize,
    pub vc_base: usize,
}

impl JpuMmio {
    /// 物理基址（ArceOS 等 `phys-virt-offset = 0` 平台）。
    pub const DEFAULT: Self = Self {
        jpu_base: JPU_REG_BASE,
        top_base: TOP_BASE,
        vc_base: VC_REG_BASE,
    };
}

/// 将 CPU 可见缓冲地址转为写入 JPU DMA 寄存器的地址。
///
/// 映射必须在整个分配区间内保持连续；恒等映射平台可传 `|v| v`。
pub type JpuDmaToPhysFn = fn(usize) -> usize;

#[inline]
fn identity_dma(v: usize) -> usize {
    v
}

/// JPU 解码器实例（持有 stream/frame DMA 缓冲）。
pub struct JpuDecoder {
    mmio: JpuMmio,
    dma_to_phys: JpuDmaToPhysFn,
    stream_buf: PhysBuffer,
    frame_buf: PhysBuffer,
    initialized: bool,
    poisoned: bool,
}

impl JpuDecoder {
    pub fn new() -> Result<Self, &'static str> {
        Self::new_with_mmio(JpuMmio::DEFAULT, identity_dma)
    }

    /// 使用板级 iomap 后的 MMIO 基址创建解码器。
    ///
    /// # Safety
    ///
    /// 调用方须保证 `mmio` 各基址为有效 MMIO 映射。`dma_to_phys` 必须把
    /// 每个 DMA buffer 映射为物理连续区间，且整个区间可由 JPU 的 32-bit
    /// 地址寄存器表示。
    pub unsafe fn new_at(
        jpu_base: usize,
        top_base: usize,
        vc_base: usize,
        dma_to_phys: JpuDmaToPhysFn,
    ) -> Result<Self, &'static str> {
        Self::new_with_mmio(
            JpuMmio {
                jpu_base,
                top_base,
                vc_base,
            },
            dma_to_phys,
        )
    }

    fn new_with_mmio(mmio: JpuMmio, dma_to_phys: JpuDmaToPhysFn) -> Result<Self, &'static str> {
        JPU_IN_USE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "JPU is already owned by another decoder")?;

        let mut decoder = Self {
            mmio,
            dma_to_phys,
            stream_buf: PhysBuffer { addr: 0, size: 0 },
            frame_buf: PhysBuffer { addr: 0, size: 0 },
            initialized: false,
            poisoned: false,
        };

        if let Err(error) = decoder.init() {
            JPU_IN_USE.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(decoder)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        init_jpu_memory();
        hardware_init_at(self.mmio.jpu_base, self.mmio.top_base, self.mmio.vc_base);

        self.initialized = true;
        Ok(())
    }

    /// Decode at full coded resolution.
    ///
    /// This compatibility entry point is equivalent to
    /// `decode_scaled(jpeg_data, JpuScale::Full)`.
    pub fn decode<'a>(&'a mut self, jpeg_data: &[u8]) -> Result<DecodeResult<'a>, &'static str> {
        self.decode_scaled(jpeg_data, JpuScale::Full)
            .map_err(JpuDecodeError::as_static_str)
    }

    /// Decode a JPEG using one isotropic hardware downscale mode.
    ///
    /// The typed error lets callers match unsupported scale, pool exhaustion,
    /// hardware failure, and timeout separately. In particular, callers may
    /// retry [`JpuScale::Full`] after a layout error, but must not retry a
    /// poisoned decoder after timeout.
    pub fn decode_scaled<'a>(
        &'a mut self,
        jpeg_data: &[u8],
        scale: JpuScale,
    ) -> Result<DecodeResult<'a>, JpuDecodeError> {
        if !self.initialized {
            return Err(JpuDecodeError::NotInitialized);
        }
        if self.poisoned {
            return Err(JpuDecodeError::Poisoned);
        }
        if jpeg_data.is_empty() {
            return Err(JpuDecodeError::EmptyStream);
        }

        let header_info = parse_jpeg_header(jpeg_data).map_err(JpuDecodeError::InvalidJpeg)?;
        let format =
            JpuPixelFormat::from_raw(header_info.format).map_err(JpuDecodeError::Layout)?;
        let layout = FrameLayout::new(header_info.width, header_info.height, format, scale)
            .map_err(JpuDecodeError::Layout)?;
        let hardware = HardwareDecodeInfo::for_format(format);
        let stream_capacity = required_stream_capacity(jpeg_data.len(), header_info.ecs_offset)
            .map_err(JpuDecodeError::InvalidJpeg)?;
        self.ensure_buffers(stream_capacity, layout.total_len)?;

        copy_to_phys(&self.stream_buf, jpeg_data).map_err(JpuDecodeError::BufferInvariant)?;
        zero_phys_range(
            &self.stream_buf,
            jpeg_data.len(),
            stream_capacity - jpeg_data.len(),
        )
        .map_err(JpuDecodeError::BufferInvariant)?;
        dcache_clean_range(self.stream_buf.addr, stream_capacity);
        dcache_invalidate_range(self.frame_buf.addr, layout.total_len);

        let stream_dma = checked_dma_region(&self.stream_buf, stream_capacity, self.dma_to_phys)
            .map_err(JpuDecodeError::DmaAddress)?;
        let stream_data_end = checked_dma_offset(stream_dma, jpeg_data.len(), true)
            .map_err(JpuDecodeError::DmaAddress)?;
        let frame_dma = checked_dma_region(&self.frame_buf, layout.total_len, self.dma_to_phys)
            .map_err(JpuDecodeError::DmaAddress)?;
        let frame_planes =
            checked_frame_dma_addresses(frame_dma, &layout).map_err(JpuDecodeError::DmaAddress)?;

        configure_stream_regs(
            self.mmio.jpu_base,
            stream_dma,
            stream_data_end,
            jpeg_data.len(),
            &header_info,
            &layout,
            hardware,
        );

        upload_huff_tables(self.mmio.jpu_base, &header_info)
            .map_err(JpuDecodeError::HardwareSetup)?;
        upload_quant_tables(self.mmio.jpu_base, &header_info)
            .map_err(JpuDecodeError::HardwareSetup)?;

        gram_setup(self.mmio.jpu_base, stream_dma, &header_info)
            .map_err(JpuDecodeError::DmaAddress)?;

        start_decode(self.mmio.jpu_base, frame_planes, &header_info, &layout)
            .map_err(JpuDecodeError::HardwareSetup)?;

        match poll_decode_done(self.mmio.jpu_base) {
            Ok(()) => {}
            Err(PollError::Decode) => return Err(JpuDecodeError::DecodeFailed),
            Err(PollError::Timeout) => {
                self.poisoned = true;
                return Err(JpuDecodeError::Timeout);
            }
        }

        dcache_invalidate_range(self.frame_buf.addr, layout.total_len);
        let padding_was_written = clear_frame_padding(&self.frame_buf, &layout)
            .map_err(JpuDecodeError::BufferInvariant)?;
        if padding_was_written {
            dcache_clean_range(self.frame_buf.addr, layout.total_len);
        }
        let yuv_phys_addr = frame_dma.start as usize;
        let yuv_data = phys_slice(&self.frame_buf, layout.total_len)
            .map_err(JpuDecodeError::BufferInvariant)?;

        Ok(DecodeResult {
            width: layout.visible.width,
            height: layout.visible.height,
            yuv_data,
            yuv_phys_addr,
            layout,
        })
    }

    fn ensure_buffers(
        &mut self,
        stream_len: usize,
        frame_len: usize,
    ) -> Result<(), JpuDecodeError> {
        if buffer_plan(
            self.stream_buf.size,
            self.frame_buf.size,
            stream_len,
            frame_len,
        ) == BufferPlan::Reuse
        {
            return Ok(());
        }

        self.release_buffers();

        if frame_len >= stream_len {
            let (frame, stream) =
                jpu_alloc_pair(frame_len, stream_len).ok_or(JpuDecodeError::MemoryPoolExhausted)?;
            self.frame_buf = frame;
            self.stream_buf = stream;
        } else {
            let (stream, frame) =
                jpu_alloc_pair(stream_len, frame_len).ok_or(JpuDecodeError::MemoryPoolExhausted)?;
            self.stream_buf = stream;
            self.frame_buf = frame;
        }
        Ok(())
    }

    fn release_buffers(&mut self) {
        if !self.stream_buf.is_empty() {
            jpu_free(core::mem::replace(&mut self.stream_buf, PhysBuffer::EMPTY));
        }
        if !self.frame_buf.is_empty() {
            jpu_free(core::mem::replace(&mut self.frame_buf, PhysBuffer::EMPTY));
        }
    }
}

impl Drop for JpuDecoder {
    fn drop(&mut self) {
        if self.poisoned {
            return;
        }
        self.release_buffers();
        JPU_IN_USE.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BufferPlan {
    Reuse,
    ReplaceBoth,
}

const fn buffer_plan(
    stream_capacity: usize,
    frame_capacity: usize,
    stream_len: usize,
    frame_len: usize,
) -> BufferPlan {
    if stream_capacity >= stream_len && frame_capacity >= frame_len {
        BufferPlan::Reuse
    } else {
        BufferPlan::ReplaceBoth
    }
}

fn required_stream_capacity(jpeg_len: usize, ecs_offset: usize) -> Result<usize, &'static str> {
    if ecs_offset >= jpeg_len {
        return Err("JPEG entropy-coded data starts outside the stream");
    }
    let prefetch_start = ecs_offset & !(BBC_STREAM_PAGE_SIZE - 1);
    let prefetch_len = BBC_STREAM_PAGE_SIZE * GRAM_PREFETCH_PAGES;
    let prefetch_end = prefetch_start
        .checked_add(prefetch_len)
        .ok_or("JPU GRAM prefetch range overflow")?;
    Ok(jpeg_len.max(prefetch_end))
}

fn clear_frame_padding(buffer: &PhysBuffer, layout: &FrameLayout) -> Result<bool, &'static str> {
    let mut previous_end = 0usize;
    let mut padding_was_written = clear_plane_and_gap_padding(buffer, layout.y, &mut previous_end)?;
    if let Some(cb) = layout.cb {
        padding_was_written |= clear_plane_and_gap_padding(buffer, cb, &mut previous_end)?;
    }
    if let Some(cr) = layout.cr {
        padding_was_written |= clear_plane_and_gap_padding(buffer, cr, &mut previous_end)?;
    }
    let trailing_padding = layout
        .total_len
        .checked_sub(previous_end)
        .ok_or("JPU frame layout planes exceed total length")?;
    if trailing_padding != 0 {
        zero_phys_range(buffer, previous_end, trailing_padding)?;
        padding_was_written = true;
    }
    Ok(padding_was_written)
}

fn clear_plane_and_gap_padding(
    buffer: &PhysBuffer,
    plane: PlaneLayout,
    previous_end: &mut usize,
) -> Result<bool, &'static str> {
    let gap_len = plane
        .offset
        .checked_sub(*previous_end)
        .ok_or("JPU frame planes overlap")?;
    let mut padding_was_written = gap_len != 0;
    if gap_len != 0 {
        zero_phys_range(buffer, *previous_end, gap_len)?;
    }

    let stride = usize::try_from(plane.stride).map_err(|_| "JPU plane stride overflow")?;
    let row_bytes = usize::try_from(plane.storage.width).map_err(|_| "JPU plane width overflow")?;
    let rows = usize::try_from(plane.storage.height).map_err(|_| "JPU plane height overflow")?;
    let row_padding = stride
        .checked_sub(row_bytes)
        .ok_or("JPU plane width exceeds its stride")?;
    if row_padding != 0 {
        padding_was_written = true;
        for row in 0..rows {
            let row_offset = row
                .checked_mul(stride)
                .and_then(|offset| offset.checked_add(plane.offset))
                .and_then(|offset| offset.checked_add(row_bytes))
                .ok_or("JPU plane row padding offset overflow")?;
            zero_phys_range(buffer, row_offset, row_padding)?;
        }
    }

    *previous_end = plane
        .offset
        .checked_add(plane.len)
        .ok_or("JPU plane end overflow")?;
    Ok(padding_was_written)
}

const fn layout_error_message(error: FrameLayoutError) -> &'static str {
    match error {
        FrameLayoutError::ZeroDimension => "JPEG dimensions must be non-zero",
        FrameLayoutError::DimensionOverflow => "aligned JPEG dimensions overflow",
        FrameLayoutError::UnsupportedScaledDimensions => {
            "JPU scaling requires both aligned dimensions to be at least 128"
        }
        FrameLayoutError::UnsupportedPixelFormat => "unsupported JPU pixel format",
        FrameLayoutError::BufferSizeOverflow => "JPU frame layout size overflow",
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        BufferPlan, PhysBuffer, buffer_plan, checked_dma_region, checked_frame_dma_addresses,
        clear_frame_padding, required_stream_capacity,
    };
    use crate::jpu::engine::DmaRegion;
    use crate::jpu::{FrameLayout, JpuPixelFormat, JpuScale};

    fn identity(value: usize) -> usize {
        value
    }

    #[test]
    fn buffer_plan_reuses_only_when_both_capacities_fit() {
        assert_eq!(buffer_plan(128, 1024, 127, 1024), BufferPlan::Reuse);
        assert_eq!(buffer_plan(128, 1024, 129, 100), BufferPlan::ReplaceBoth);
        assert_eq!(buffer_plan(128, 1024, 100, 1025), BufferPlan::ReplaceBoth);
    }

    #[test]
    fn dma_region_rejects_capacity_and_u32_range_violations() {
        let buffer = PhysBuffer {
            addr: u32::MAX as usize - 8,
            size: 16,
        };

        assert_eq!(
            checked_dma_region(&buffer, 8, identity),
            Ok(DmaRegion {
                start: u32::MAX - 8,
                end: u32::MAX,
            })
        );
        assert!(checked_dma_region(&buffer, 9, identity).is_err());
        assert!(checked_dma_region(&buffer, 17, identity).is_err());
    }

    #[test]
    fn frame_dma_addresses_include_aligned_plane_offsets() {
        let layout = FrameLayout::new(129, 129, JpuPixelFormat::Yuv420, JpuScale::Eighth)
            .expect("valid layout");
        let region = DmaRegion {
            start: 0x1000,
            end: 0x1000 + layout.total_len as u32,
        };

        let addresses = checked_frame_dma_addresses(region, &layout).expect("valid addresses");
        assert_eq!(addresses.y, 0x1000);
        assert_eq!(addresses.cb, 0x1000 + 576);
        assert_eq!(addresses.cr, 0x1000 + 664);
    }

    #[test]
    fn stream_capacity_covers_two_page_gram_prefetch() {
        assert_eq!(required_stream_capacity(1000, 500), Ok(1000));
        assert_eq!(required_stream_capacity(1000, 900), Ok(1280));
        assert_eq!(required_stream_capacity(16_384, 16_383), Ok(16_640));
        assert!(required_stream_capacity(1000, 1000).is_err());
        assert!(required_stream_capacity(1000, 1001).is_err());
        assert!(required_stream_capacity(usize::MAX, usize::MAX - 1).is_err());
    }

    #[test]
    fn frame_padding_is_cleared_without_touching_plane_samples() {
        let mut memory = std::boxed::Box::new([0xa5u8; 1024]);
        let buffer = PhysBuffer {
            addr: memory.as_mut_ptr() as usize,
            size: memory.len(),
        };
        let layout = FrameLayout::new(129, 129, JpuPixelFormat::Yuv420, JpuScale::Eighth)
            .expect("valid layout");
        assert_eq!(buffer.addr, memory.as_ptr() as usize);
        assert_eq!(layout.y.storage.width, 18);
        assert_eq!(layout.y.stride, 32);
        assert!(memory.iter().all(|byte| *byte == 0xa5));

        assert!(clear_frame_padding(&buffer, &layout).expect("padding layout is valid"));
        assert_eq!(buffer.addr, memory.as_ptr() as usize);

        for row in 0..18 {
            let start = row * 32;
            assert!(
                memory[start..start + 18].iter().all(|byte| *byte == 0xa5),
                "row {row} active bytes: {:?}",
                &memory[start..start + 18]
            );
            assert!(memory[start + 18..start + 32].iter().all(|byte| *byte == 0));
        }
        assert!(memory[576..657].iter().all(|byte| *byte == 0xa5));
        assert!(memory[657..664].iter().all(|byte| *byte == 0));
        assert!(memory[664..745].iter().all(|byte| *byte == 0xa5));
        assert!(memory[745..752].iter().all(|byte| *byte == 0));
        assert!(memory[752..].iter().all(|byte| *byte == 0xa5));
    }

    #[test]
    fn naturally_packed_frame_skips_padding_writes() {
        let layout = FrameLayout::new(1279, 1706, JpuPixelFormat::Yuv420, JpuScale::Half)
            .expect("valid packed layout");
        let mut memory = std::vec![0xa5u8; layout.total_len];
        let buffer = PhysBuffer {
            addr: memory.as_mut_ptr() as usize,
            size: memory.len(),
        };

        assert!(!clear_frame_padding(&buffer, &layout).expect("layout is valid"));
        assert!(memory.iter().all(|byte| *byte == 0xa5));
    }
}
