//! JPU 硬件 JPEG 解码器（Baseline，轮询模式）。

use super::header::{HuffTable, JpegHeaderInfo, parse_jpeg_header};
use super::mem::{PhysBuffer, copy_to_phys, init_jpu_memory, jpu_alloc, jpu_free, phys_slice};
use super::regs::{
    HUFF_ADDR_MAX, HUFF_ADDR_PTR, HUFF_PHASE_MAX, HUFF_PHASE_MIN, HUFF_PHASE_PTR, HUFF_PHASE_VAL,
    JPU_REG_BASE, MJPEG_HUFF_CTRL, MJPEG_PIC_SIZE, MJPEG_PIC_START, MJPEG_PIC_STATUS,
    MJPEG_QMAT_CTRL, QMAT_PHASE_CB, QMAT_PHASE_CR, QMAT_PHASE_Y, STREAM_BUF_SIZE, VALUE32,
    VC_REG_BASE, bbc_strm_ctrl_value, clear_pic_status_at, hardware_init_at, jpu_regs_at,
    pic_ctrl_value, wait_bbc_idle_at, FORMAT_400, FORMAT_420, FORMAT_422, FORMAT_224, FORMAT_444,
};
use crate::soc::TOP_BASE;
use crate::utils::cache::{dcache_clean_range, dcache_invalidate_range};
use crate::utils::time::Deadline;
use tock_registers::interfaces::{Readable, Writeable};

/// 解码结果：YUV420 planar，数据位于 DMA 帧缓冲（至下次 decode/Drop 有效）。
pub struct DecodeResult {
    pub width: u32,
    pub height: u32,
    pub yuv_data: &'static [u8],
    pub yuv_phys_addr: usize,
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

/// 将 CPU 可见缓冲地址转为写入 JPU DMA 寄存器的地址；恒等映射平台可传 `|v| v`。
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
    /// 调用方指定的输出缓冲（物理地址）。设了之后 `decode()` 让 JPU **直接 DMA
    /// 到这里**，不再从内部 pool 分配 frame_buf，省掉一次整帧 memcpy。
    output_buf: Option<PhysBuffer>,
    /// CPU 是否会通过 cache 读解码输出。
    /// 为 false 时跳过对输出缓冲的 dcache 维护——CPU 从不碰这块内存，
    /// 就不会有脏行写回覆盖 DMA 数据，也不需要 invalidate 去看新数据。
    cpu_reads_output: bool,
}

impl JpuDecoder {
    pub fn new() -> Result<Self, &'static str> {
        Self::new_with_mmio(JpuMmio::DEFAULT, identity_dma)
    }

    /// 使用板级 iomap 后的 MMIO 基址创建解码器。
    ///
    /// # Safety
    ///
    /// 调用方须保证 `mmio` 各基址为有效 MMIO 映射，且 `dma_to_phys` 在 VA≠PA 时正确。
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
        let mut decoder = Self {
            mmio,
            dma_to_phys,
            stream_buf: PhysBuffer { addr: 0, size: 0 },
            frame_buf: PhysBuffer { addr: 0, size: 0 },
            initialized: false,
            output_buf: None,
            cpu_reads_output: true,
        };

        decoder.init()?;
        Ok(decoder)
    }

    /// 创建解码器，**跳过** `hardware_init_at`（假定 U-Boot/Bootloader 已初始化 JPU 时钟/复位）。
    /// 适用于小核（C906L）等不能写 TOP 寄存器的场景——`hardware_init_at` 里的
    /// `TOP_DDR_ADDR_MODE_OFF` 写入会改变大核的 DDR 地址映射导致大核崩溃。
    ///
    /// `dma_pool_base` / `dma_pool_size`：JPU DMA 内存池的物理地址和大小。
    /// 小核应把 pool 放在普通 DRAM（非预留区），因为预留区的 JPU DMA 地址映射可能不正确。
    pub unsafe fn new_at_skip_hw_init_with_pool(
        jpu_base: usize,
        top_base: usize,
        vc_base: usize,
        dma_to_phys: JpuDmaToPhysFn,
        dma_pool_base: usize,
        dma_pool_size: usize,
    ) -> Result<Self, &'static str> {
        let mut decoder = Self {
            mmio: JpuMmio {
                jpu_base,
                top_base,
                vc_base,
            },
            dma_to_phys,
            stream_buf: PhysBuffer { addr: 0, size: 0 },
            frame_buf: PhysBuffer { addr: 0, size: 0 },
            initialized: false,
            output_buf: None,
            cpu_reads_output: true,
        };
        // 用外部 pool 初始化（绕过静态 DMA_BUFFER 在预留区的问题）
        super::mem::init_jpu_memory_with(dma_pool_base, dma_pool_size);
        decoder.stream_buf = super::mem::jpu_alloc(STREAM_BUF_SIZE)
            .ok_or("Failed to allocate stream buffer")?;
        decoder.initialized = true;
        Ok(decoder)
    }

    /// 创建解码器，用 `hardware_init_at_no_vd_remap`（设时钟/复位/VC/软复位，但不设 VD_REMAP）。
    /// 适用于小核（C906L）：VD_REMAP 会把 32 位 DMA 地址扩展到 40 位，超出 DDR 范围。
    /// DMA pool 用外部地址（绕过静态 DMA_BUFFER 在预留区的问题）。
    pub unsafe fn new_at_no_vd_remap_with_pool(
        jpu_base: usize,
        top_base: usize,
        vc_base: usize,
        dma_to_phys: JpuDmaToPhysFn,
        dma_pool_base: usize,
        dma_pool_size: usize,
    ) -> Result<Self, &'static str> {
        let mut decoder = Self {
            mmio: JpuMmio {
                jpu_base,
                top_base,
                vc_base,
            },
            dma_to_phys,
            stream_buf: PhysBuffer { addr: 0, size: 0 },
            frame_buf: PhysBuffer { addr: 0, size: 0 },
            initialized: false,
            output_buf: None,
            cpu_reads_output: true,
        };
        super::mem::init_jpu_memory_with(dma_pool_base, dma_pool_size);
        super::regs::hardware_init_at_no_vd_remap(jpu_base, top_base, vc_base);
        decoder.stream_buf = super::mem::jpu_alloc(STREAM_BUF_SIZE)
            .ok_or("Failed to allocate stream buffer")?;
        decoder.initialized = true;
        Ok(decoder)
    }

    pub unsafe fn new_at_skip_hw_init(
        jpu_base: usize,
        top_base: usize,
        vc_base: usize,
        dma_to_phys: JpuDmaToPhysFn,
    ) -> Result<Self, &'static str> {
        let mut decoder = Self {
            mmio: JpuMmio {
                jpu_base,
                top_base,
                vc_base,
            },
            dma_to_phys,
            stream_buf: PhysBuffer { addr: 0, size: 0 },
            frame_buf: PhysBuffer { addr: 0, size: 0 },
            initialized: false,
            output_buf: None,
            cpu_reads_output: true,
        };
        decoder.init_skip_hw_init()?;
        Ok(decoder)
    }

    fn init_skip_hw_init(&mut self) -> Result<(), &'static str> {
        init_jpu_memory();
        // 不调 hardware_init_at——U-Boot 已经使能了 JPU 时钟/复位/DDR remap。
        self.stream_buf = jpu_alloc(STREAM_BUF_SIZE).ok_or("Failed to allocate stream buffer")?;
        self.initialized = true;
        Ok(())
    }

    fn init(&mut self) -> Result<(), &'static str> {
        init_jpu_memory();
        hardware_init_at(self.mmio.jpu_base, self.mmio.top_base, self.mmio.vc_base);

        self.stream_buf = jpu_alloc(STREAM_BUF_SIZE).ok_or("Failed to allocate stream buffer")?;
        self.initialized = true;
        Ok(())
    }

    /// JPU 挂死/解码出错后的硬件恢复：给 JPEG 块一次真正的复位脉冲。
    ///
    /// 光靠软复位（START_INIT）或重跑 `hardware_init_*` 都救不回来——后者只
    /// "释放"复位位，对已在运行的块是空操作。详见 [`regs::hard_reset_at`]。
    pub fn recover(&mut self) {
        super::regs::hard_reset_at(self.mmio.jpu_base, self.mmio.top_base, self.mmio.vc_base);
    }

    /// 让 `decode()` 把 YUV 直接 DMA 到 `pa`，不再用内部 pool 的 frame_buf。
    ///
    /// 典型用途：`pa` 就是最终消费者（如另一个核）读取的共享缓冲——省掉
    /// 「解码到 pool → memcpy 到共享区」这一整帧拷贝。实测 640x480 那次
    /// memcpy 要 34ms，占整帧耗时的 56%。
    ///
    /// # Safety
    /// 调用方须保证 `[pa, pa+size)` 是有效、独占、JPU DMA 可达的物理内存。
    pub unsafe fn set_output_buffer(&mut self, pa: usize, size: usize) {
        self.output_buf = Some(PhysBuffer { addr: pa, size });
    }

    /// 声明 CPU 是否会通过 cache 读解码输出（默认 true）。
    ///
    /// 设为 false 可跳过对输出缓冲的两次 dcache 维护（640x480 实测各 5.8ms）。
    /// 仅当 CPU 确实从不读这块内存时才可以设 false。
    pub fn set_cpu_reads_output(&mut self, v: bool) {
        self.cpu_reads_output = v;
    }

    pub fn decode(&mut self, jpeg_data: &[u8]) -> Result<DecodeResult, &'static str> {
        use super::trace::{mark_timed as mark, step};
        mark(step::ENTER);
        if !self.initialized {
            return Err("JPU not initialized");
        }

        let header_info = parse_jpeg_header(jpeg_data)?;
        mark(step::PARSE_HEADER);

        let copy_len = jpeg_data.len().min(self.stream_buf.size);
        copy_to_phys(self.stream_buf, &jpeg_data[..copy_len]);
        mark(step::COPY_STREAM);
        dcache_clean_range(self.stream_buf.addr, copy_len);
        mark(step::CLEAN_STREAM);

        let (frame_size, layout) = frame_layout(&header_info)?;
        mark(step::FRAME_LAYOUT);

        match self.output_buf {
            Some(out) => {
                // 外部输出缓冲：不分配也不释放，直接 DMA 过去。
                if frame_size > out.size {
                    log::warn!(
                        "[JPU] output buf too small: frame_size={} out.size={} {}x{} fmt={}",
                        frame_size, out.size, header_info.width, header_info.height, header_info.format
                    );
                    return Err("output buffer too small");
                }
                self.frame_buf = PhysBuffer { addr: out.addr, size: out.size };
                mark(step::FREE_FRAME);
                mark(step::ALLOC_FRAME);
            }
            None => {
                if !self.frame_buf.is_empty() {
                    jpu_free(self.frame_buf);
                    self.frame_buf = PhysBuffer { addr: 0, size: 0 };
                }
                mark(step::FREE_FRAME);
                self.frame_buf = jpu_alloc(frame_size).ok_or("Failed to alloc frame buf")?;
                mark(step::ALLOC_FRAME);
            }
        }
        if self.cpu_reads_output {
            dcache_invalidate_range(self.frame_buf.addr, frame_size);
        }
        mark(step::INV_FRAME);

        configure_stream_regs(
            self.mmio.jpu_base,
            self.dma_to_phys,
            &self.stream_buf,
            copy_len,
            &header_info,
            layout,
        );
        mark(step::CFG_STREAM_REGS);

        upload_huff_tables(self.mmio.jpu_base, &header_info)?;
        mark(step::HUFF);
        upload_quant_tables(self.mmio.jpu_base, &header_info)?;
        mark(step::QUANT);

        let stream_dma = (self.dma_to_phys)(self.stream_buf.addr);
        gram_setup(self.mmio.jpu_base, stream_dma, &header_info)?;
        mark(step::GRAM);

        let frame_dma = (self.dma_to_phys)(self.frame_buf.addr);
        start_decode(self.mmio.jpu_base, frame_dma, &header_info, layout)?;
        mark(step::START_DECODE);

        if let Err(e) = poll_decode_done(self.mmio.jpu_base) {
            // 挂死/出错后必须真正复位 JPEG 块，否则后续每帧都会再等满一个超时。
            self.recover();
            return Err(e);
        }
        mark(step::POLL);

        if self.cpu_reads_output {
            dcache_invalidate_range(self.frame_buf.addr, frame_size);
        }
        mark(step::INV_AFTER);

        let r = DecodeResult {
            width: header_info.width,
            height: header_info.height,
            yuv_data: phys_slice(self.frame_buf.addr, frame_size),
            yuv_phys_addr: self.frame_buf.addr,
        };
        mark(step::DONE);
        Ok(r)
    }
}

impl Drop for JpuDecoder {
    fn drop(&mut self) {
        if !self.stream_buf.is_empty() {
            jpu_free(self.stream_buf);
        }
        // 外部输出缓冲不属于内部 pool，不能 free。
        if self.output_buf.is_none() && !self.frame_buf.is_empty() {
            jpu_free(self.frame_buf);
        }
    }
}

#[derive(Clone, Copy)]
struct FrameLayout {
    aligned_width: u32,
    aligned_height: u32,
    stride_y: u32,
    stride_c: u32,
    luma_size: usize,
    chroma_size: usize,
    mcu_block_num: u32,
    comp_info: u32,
    bus_req_num: u32,
}

fn frame_layout(header: &JpegHeaderInfo) -> Result<(usize, FrameLayout), &'static str> {
    let aligned_width = match header.format {
        FORMAT_420 | FORMAT_422 => header.width.div_ceil(16) * 16,
        _ => header.width.div_ceil(8) * 8,
    };
    let aligned_height = match header.format {
        FORMAT_420 | FORMAT_224 => header.height.div_ceil(16) * 16,
        _ => header.height.div_ceil(8) * 8,
    };
    let stride_y = aligned_width;
    let stride_c = match header.format {
        FORMAT_420 | FORMAT_422 => aligned_width / 2,
        FORMAT_400 => 0,
        _ => aligned_width,
    };

    let luma_size = (stride_y * aligned_height) as usize;
    let chroma_size = match header.format {
        FORMAT_420 => (stride_c * aligned_height / 2) as usize,
        FORMAT_422 | FORMAT_224 => luma_size / 2,
        FORMAT_444 => luma_size,
        FORMAT_400 => 0,
        _ => (stride_c * aligned_height / 2) as usize,
    };

    let (mcu_block_num, comp_info) = match header.format {
        FORMAT_420 => (6, (10 << 8) | (5 << 4) | 5),
        FORMAT_422 => (4, (9 << 8) | (5 << 4) | 5),
        FORMAT_224 => (4, (6 << 8) | (5 << 4) | 5),
        FORMAT_444 => (3, (5 << 8) | (5 << 4) | 5),
        FORMAT_400 => (1, 5 << 8),
        _ => (6, (10 << 8) | (5 << 4) | 5),
    };
    let bus_req_num = match header.format {
        FORMAT_420 => 2,
        FORMAT_422 | FORMAT_224 => 3,
        FORMAT_444 | FORMAT_400 => 4,
        _ => 2,
    };

    Ok((
        luma_size + chroma_size * 2,
        FrameLayout {
            aligned_width,
            aligned_height,
            stride_y,
            stride_c,
            luma_size,
            chroma_size,
            mcu_block_num,
            comp_info,
            bus_req_num,
        },
    ))
}

fn configure_stream_regs(
    jpu_base: usize,
    dma_to_phys: JpuDmaToPhysFn,
    stream_buf: &PhysBuffer,
    copy_len: usize,
    header: &JpegHeaderInfo,
    layout: FrameLayout,
) {
    let r = jpu_regs_at(jpu_base);
    let stream_phys = dma_to_phys(stream_buf.addr) as u32;
    let stream_end = (dma_to_phys(stream_buf.addr) + copy_len) as u32;

    r.bbc_bas_addr.write(VALUE32::VAL.val(stream_phys));
    r.bbc_end_addr.write(VALUE32::VAL.val(stream_end));
    r.bbc_rd_ptr.write(VALUE32::VAL.val(stream_phys));
    r.bbc_wr_ptr.write(VALUE32::VAL.val(stream_end));

    let strm_pages = copy_len.div_ceil(256);
    r.bbc_strm_ctrl.set(bbc_strm_ctrl_value(strm_pages as u32));

    r.gbu_tt_cnt.write(VALUE32::VAL.val(0));
    r.gbu_tt_cnt_h.write(VALUE32::VAL.val(0));
    r.pic_errmb.write(VALUE32::VAL.val(0));

    let mut huff_dc_idx = 0u32;
    let mut huff_ac_idx = 0u32;
    for i in 0..3 {
        huff_dc_idx = (huff_dc_idx << 1) | header.dc_huff_tbl[i] as u32;
        huff_ac_idx = (huff_ac_idx << 1) | header.ac_huff_tbl[i] as u32;
    }
    r.pic_ctrl.set(pic_ctrl_value(huff_dc_idx, huff_ac_idx));

    r.pic_size.write(
        MJPEG_PIC_SIZE::WIDTH.val(layout.aligned_width)
            + MJPEG_PIC_SIZE::HEIGHT.val(layout.aligned_height),
    );
    r.rot_info.write(VALUE32::VAL.val(0));
    r.mcu_info.write(VALUE32::VAL.val((layout.mcu_block_num << 16) | (header.num_components << 12) | layout.comp_info));
    r.dpb_config.write(VALUE32::VAL.val(0));
    r.rst_intval.write(VALUE32::VAL.val(header.restart_interval));
    r.scl_info.write(VALUE32::VAL.val(0));
    r.op_info.write(VALUE32::VAL.val(layout.bus_req_num));
}

fn upload_huff_tables(jpu_base: usize, header: &JpegHeaderInfo) -> Result<(), &'static str> {
    let r = jpu_regs_at(jpu_base);

    r.huff_ctrl
        .write(MJPEG_HUFF_CTRL::PHASE.val(HUFF_PHASE_MIN));
    for table_idx in [0, 2, 1, 3] {
        for j in 0..16 {
            let huff_data = header.huff_tables[table_idx].min_codes[j];
            let temp = HuffTable::sign_extend_16(huff_data);
            r.huff_data.write(VALUE32::VAL.val(((temp & 0xFFFF) << 16) | huff_data));
        }
    }

    r.huff_ctrl
        .write(MJPEG_HUFF_CTRL::PHASE.val(HUFF_PHASE_MAX));
    r.huff_addr.write(VALUE32::VAL.val(HUFF_ADDR_MAX));
    for table_idx in [0, 2, 1, 3] {
        for j in 0..16 {
            let huff_data = header.huff_tables[table_idx].max_codes[j];
            let temp = HuffTable::sign_extend_16(huff_data);
            r.huff_data.write(VALUE32::VAL.val(((temp & 0xFFFF) << 16) | huff_data));
        }
    }

    r.huff_ctrl
        .write(MJPEG_HUFF_CTRL::PHASE.val(HUFF_PHASE_PTR));
    r.huff_addr.write(VALUE32::VAL.val(HUFF_ADDR_PTR));
    for table_idx in [0, 2, 1, 3] {
        for j in 0..16 {
            let huff_data = header.huff_tables[table_idx].ptrs[j] as u32;
            let temp = HuffTable::sign_extend_8(huff_data);
            r.huff_data.write(VALUE32::VAL.val(((temp & 0xFFFFFF) << 8) | huff_data));
        }
    }

    r.huff_ctrl
        .write(MJPEG_HUFF_CTRL::PHASE.val(HUFF_PHASE_VAL));
    for &table_idx in &[0, 2, 1, 3] {
        let is_dc = table_idx == 0 || table_idx == 2;
        let max_count = if is_dc { 12 } else { 162 };
        let bits_len = if is_dc { 12 } else { 16 };
        let count: usize = header.huff_tables[table_idx].bits[..bits_len]
            .iter()
            .map(|&b| b as usize)
            .sum();

        for j in 0..count.min(header.huff_tables[table_idx].num_values) {
            let val = header.huff_tables[table_idx].values[j] as u32;
            let temp = HuffTable::sign_extend_8(val);
            r.huff_data.write(VALUE32::VAL.val(((temp & 0xFFFFFF) << 8) | val));
        }
        for _ in count..max_count {
            r.huff_data.write(VALUE32::VAL.val(0xFFFF_FFFF));
        }
    }

    r.huff_ctrl.write(MJPEG_HUFF_CTRL::PHASE.val(0));
    Ok(())
}

fn upload_quant_tables(jpu_base: usize, header: &JpegHeaderInfo) -> Result<(), &'static str> {
    let r = jpu_regs_at(jpu_base);
    let qmat_phases = [QMAT_PHASE_Y, QMAT_PHASE_CB, QMAT_PHASE_CR];
    let comp_count = (header.num_components as usize).min(3);
    for (comp_idx, &phase) in qmat_phases.iter().enumerate().take(comp_count) {
        let table_idx = header.quant_tbl[comp_idx];
        if table_idx >= 4 || table_idx >= header.quant_table_count {
            continue;
        }

        r.qmat_ctrl.write(MJPEG_QMAT_CTRL::PHASE.val(phase));
        for j in 0..64 {
            r.qmat_data.write(VALUE32::VAL.val(header.quant_tables[table_idx].values[j] as u32));
        }
        r.qmat_ctrl.write(MJPEG_QMAT_CTRL::PHASE.val(0));
    }
    Ok(())
}

fn gram_setup(jpu_base: usize, stream_phys: usize, header: &JpegHeaderInfo) -> Result<(), &'static str> {
    let r = jpu_regs_at(jpu_base);
    let ecs_offset = header.ecs_offset;
    let page_ptr = ecs_offset >> 8;
    let mut word_ptr = (ecs_offset & 0xF0) >> 2;
    let bit_ptr = (ecs_offset & 0xF) << 3;

    if page_ptr & 1 != 0 {
        word_ptr += 64;
    }
    if word_ptr & 1 != 0 {
        word_ptr -= 1;
    }

    for i in 0..2 {
        let cur_page = page_ptr + i;
        r.bbc_cur_pos.write(VALUE32::VAL.val(cur_page as u32));
        r.bbc_ext_addr.write(VALUE32::VAL.val((stream_phys as u32) + ((cur_page as u32) << 8)));
        r.bbc_int_addr.write(VALUE32::VAL.val(((cur_page & 1) as u32) << 6));
        r.bbc_data_cnt.write(VALUE32::VAL.val(256 / 4));
        r.bbc_command.write(VALUE32::VAL.val(0));
        wait_bbc_idle_at(jpu_base);
    }

    r.bbc_cur_pos.write(VALUE32::VAL.val((page_ptr + 2) as u32));
    r.bbc_ctrl.write(VALUE32::VAL.val(1));

    r.gbu_wd_ptr.write(VALUE32::VAL.val(word_ptr as u32));
    r.gbu_bbsr.write(VALUE32::VAL.val(0));
    r.gbu_bber.write(VALUE32::VAL.val(((256 / 4) * 2) - 1));

    if page_ptr & 1 != 0 {
        r.gbu_bbir.write(VALUE32::VAL.val(0));
        r.gbu_bbhr.write(VALUE32::VAL.val(0));
    } else {
        r.gbu_bbir.write(VALUE32::VAL.val(256 / 4));
        r.gbu_bbhr.write(VALUE32::VAL.val(256 / 4));
    }

    r.gbu_ctrl.write(VALUE32::VAL.val(4));
    r.gbu_ff_rptr.write(VALUE32::VAL.val(bit_ptr as u32));
    Ok(())
}

fn start_decode(
    jpu_base: usize,
    frame_phys: usize,
    header: &JpegHeaderInfo,
    layout: FrameLayout,
) -> Result<(), &'static str> {
    let r = jpu_regs_at(jpu_base);
    r.rst_index.write(VALUE32::VAL.val(0));
    r.rst_count.write(VALUE32::VAL.val(0));
    r.dpcm_diff_y.write(VALUE32::VAL.val(0));
    r.dpcm_diff_cb.write(VALUE32::VAL.val(0));
    r.dpcm_diff_cr.write(VALUE32::VAL.val(0));

    let bit_ptr = (header.ecs_offset & 0xF) << 3;
    r.gbu_ff_rptr.write(VALUE32::VAL.val(bit_ptr as u32));
    r.gbu_ctrl.write(VALUE32::VAL.val(3));

    r.dpb_base_y.write(VALUE32::VAL.val(frame_phys as u32));
    let cb_phys = frame_phys + layout.luma_size;
    r.dpb_base_cb.write(VALUE32::VAL.val(cb_phys as u32));
    let cr_phys = cb_phys + layout.chroma_size;
    r.dpb_base_cr.write(VALUE32::VAL.val(cr_phys as u32));

    r.dpb_ystride.write(VALUE32::VAL.val(layout.stride_y));
    r.dpb_cstride.write(VALUE32::VAL.val(layout.stride_c));
    r.clp_info.write(VALUE32::VAL.val(0));

    clear_pic_status_at(jpu_base, r.pic_status.get());
    r.pic_start.write(MJPEG_PIC_START::START_PIC::SET);
    Ok(())
}

/// 单帧解码的等待上限。640×480 baseline MJPEG 正常 1~5ms 完成，200ms 已极宽松。
///
/// **必须按时间而非轮询次数判定**：原来只有 `MAX_POLLS` 次数上限，而每轮内层
/// 1000 次 `spin_loop` 在小核 C906L 上要 ~461us（实测 ~2168 轮/秒），500k 轮实际
/// 要 **230 秒**才超时。JPU 一挂，小核就被这个循环堵 230 秒，`frame_count` 冻结，
/// 外部看起来像彻底死机——而不是预期的"2 秒后超时并复位"。
const DECODE_TIMEOUT_MS: u64 = 200;
/// 次数兜底：仅在没有 `rdtime`（非 riscv64）时生效。
const MAX_POLLS: u32 = 500_000;

fn poll_decode_done(jpu_base: usize) -> Result<(), &'static str> {
    let mut count = 0u32;
    let r = jpu_regs_at(jpu_base);
    let deadline = Deadline::after_ms(DECODE_TIMEOUT_MS);

    loop {
        if r.pic_status.is_set(MJPEG_PIC_STATUS::DONE) {
            clear_pic_status_at(jpu_base, r.pic_status.get());
            return Ok(());
        }

        if r.pic_status.is_set(MJPEG_PIC_STATUS::ERROR) {
            let status = r.pic_status.get();
            let err_mb = r.pic_errmb.get();
            log::warn!(
                "[JPU] Error! status=0x{:x}, err_mb=0x{:x}",
                status,
                err_mb
            );
            clear_pic_status_at(jpu_base, status);
            return Err("JPU decode error");
        }

        // 轮询间隔别太大：这里每轮的开销直接决定超时判定的粒度。
        for _ in 0..64 {
            core::hint::spin_loop();
        }
        count += 1;
        super::trace::mark_poll(count);

        if deadline.expired() || count >= MAX_POLLS {
            let status = r.pic_status.get();
            log::warn!("[JPU] Timeout! status=0x{:x}, polls={}", status, count);
            return Err("JPU decode timeout");
        }
    }
}
