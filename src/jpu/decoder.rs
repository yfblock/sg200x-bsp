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
        };

        decoder.init()?;
        Ok(decoder)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        init_jpu_memory();
        hardware_init_at(self.mmio.jpu_base, self.mmio.top_base, self.mmio.vc_base);

        self.stream_buf = jpu_alloc(STREAM_BUF_SIZE).ok_or("Failed to allocate stream buffer")?;
        self.initialized = true;
        Ok(())
    }

    pub fn decode(&mut self, jpeg_data: &[u8]) -> Result<DecodeResult, &'static str> {
        if !self.initialized {
            return Err("JPU not initialized");
        }

        let header_info = parse_jpeg_header(jpeg_data)?;

        let copy_len = jpeg_data.len().min(self.stream_buf.size);
        copy_to_phys(self.stream_buf, &jpeg_data[..copy_len]);
        dcache_clean_range(self.stream_buf.addr, copy_len);

        let (frame_size, layout) = frame_layout(&header_info)?;

        if !self.frame_buf.is_empty() {
            jpu_free(self.frame_buf);
            self.frame_buf = PhysBuffer { addr: 0, size: 0 };
        }
        self.frame_buf = jpu_alloc(frame_size).ok_or("Failed to alloc frame buf")?;
        dcache_invalidate_range(self.frame_buf.addr, frame_size);

        configure_stream_regs(
            self.mmio.jpu_base,
            self.dma_to_phys,
            &self.stream_buf,
            copy_len,
            &header_info,
            layout,
        );

        upload_huff_tables(self.mmio.jpu_base, &header_info)?;
        upload_quant_tables(self.mmio.jpu_base, &header_info)?;

        let stream_dma = (self.dma_to_phys)(self.stream_buf.addr);
        gram_setup(self.mmio.jpu_base, stream_dma, &header_info)?;

        let frame_dma = (self.dma_to_phys)(self.frame_buf.addr);
        start_decode(self.mmio.jpu_base, frame_dma, &header_info, layout)?;

        poll_decode_done(self.mmio.jpu_base)?;

        dcache_invalidate_range(self.frame_buf.addr, frame_size);

        Ok(DecodeResult {
            width: header_info.width,
            height: header_info.height,
            yuv_data: phys_slice(self.frame_buf.addr, frame_size),
            yuv_phys_addr: self.frame_buf.addr,
        })
    }
}

impl Drop for JpuDecoder {
    fn drop(&mut self) {
        if !self.stream_buf.is_empty() {
            jpu_free(self.stream_buf);
        }
        if !self.frame_buf.is_empty() {
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

fn poll_decode_done(jpu_base: usize) -> Result<(), &'static str> {
    let mut count = 0u32;
    const MAX_POLLS: u32 = 500_000;
    let r = jpu_regs_at(jpu_base);

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

        for _ in 0..1000 {
            core::hint::spin_loop();
        }
        count += 1;

        if count >= MAX_POLLS {
            let status = r.pic_status.get();
            log::warn!("[JPU] Timeout! status=0x{:x}, polls={}", status, count);
            return Err("JPU decode timeout");
        }
    }
}
