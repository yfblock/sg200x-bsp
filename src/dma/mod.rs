//! CVITEK DMA Controller Driver (Synopsys DesignWare AXI DMA IP)
//!
//! - Memory to Memory transfers
//! - Memory to Device / Device to Memory transfers
//! - Cyclic DMA for audio streaming
//! - Scatter-Gather support via Linked List Items (LLI)

pub mod regs;

use core::ptr::{read_volatile, write_volatile};
use regs::*;
use tock_registers::interfaces::{Readable, Writeable};

/// Maximum number of DMA channels
pub const DMA_MAX_CHANNELS: usize = 8;

/// Maximum number of DMA masters
pub const DMA_MAX_MASTERS: usize = 4;

/// Maximum number of hardware requests
pub const DMA_MAX_REQUESTS: usize = 16;

/// Default block size for DMA transfers
pub const DMA_DEFAULT_BLOCK_SIZE: u32 = 1024;

/// DMA transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Memory to Memory
    MemToMem,
    /// Memory to Device (Peripheral)
    MemToDev,
    /// Device (Peripheral) to Memory
    DevToMem,
    /// No direction set
    None,
}

/// DMA transfer width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaWidth {
    Width8 = 0,
    Width16 = 1,
    Width32 = 2,
    Width64 = 3,
    Width128 = 4,
    Width256 = 5,
    Width512 = 6,
}

impl DmaWidth {
    pub fn from_bytes(bytes: usize) -> Self {
        match bytes {
            1 => DmaWidth::Width8,
            2 => DmaWidth::Width16,
            4 => DmaWidth::Width32,
            8 => DmaWidth::Width64,
            16 => DmaWidth::Width128,
            32 => DmaWidth::Width256,
            64 => DmaWidth::Width512,
            _ => DmaWidth::Width32,
        }
    }

    pub fn bytes(&self) -> usize {
        1 << (*self as usize)
    }
}

/// DMA burst size (MSIZE)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaMsize {
    Msize1 = 0,
    Msize4 = 1,
    Msize8 = 2,
    Msize16 = 3,
    Msize32 = 4,
    Msize64 = 5,
    Msize128 = 6,
    Msize256 = 7,
}

/// Flow control type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaFlowControl {
    /// DMA controller, Memory to Memory
    DmaM2M = 0,
    /// DMA controller, Memory to Peripheral
    DmaM2P = 1,
    /// DMA controller, Peripheral to Memory
    DmaP2M = 2,
    /// DMA controller, Peripheral to Peripheral
    DmaP2P = 3,
    /// Source peripheral, Peripheral to Memory
    SrcP2M = 4,
    /// Source peripheral, Peripheral to Peripheral
    SrcP2P = 5,
    /// Destination peripheral, Memory to Peripheral
    DstM2P = 6,
    /// Destination peripheral, Peripheral to Peripheral
    DstP2P = 7,
}

/// Multi-block transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaMultiBlockType {
    Contiguous = 0,
    Reload = 1,
    ShadowReg = 2,
    LinkList = 3,
}

/// DMA channel status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Error,
}

/// Linked List Item (LLI) descriptor（与 Linux `struct axi_dma_lli` 一致，**64 字节**）
///
/// 旧版多出的 `reserved` 会把 `size_of` 撑到 128，LLP 链与硬件步进 64 字节不一致，DMA 会读错下一项。
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default)]
pub struct DmaLli {
    /// Source address
    pub sar: u64,
    /// Destination address
    pub dar: u64,
    /// Block transfer size
    pub block_ts: u64,
    /// Link to next LLI
    pub llp: u64,
    /// Control register
    pub ctl: u64,
    /// Source status
    pub sstat: u64,
    /// Destination status
    pub dstat: u64,
    /// LLP / block 状态（低 32 + 高 32，与 IP 文档一致时可拆分使用）
    pub llp_status: u64,
}

impl DmaLli {
    pub const fn new() -> Self {
        Self {
            sar: 0,
            dar: 0,
            block_ts: 0,
            llp: 0,
            ctl: 0,
            sstat: 0,
            dstat: 0,
            llp_status: 0,
        }
    }
}

const _: () = assert!(core::mem::size_of::<DmaLli>() == 64);

/// DMA slave configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct DmaSlaveConfig {
    pub direction: DmaDirection,
    pub src_addr: u64,
    pub dst_addr: u64,
    pub src_addr_width: DmaWidth,
    pub dst_addr_width: DmaWidth,
    pub src_maxburst: u32,
    pub dst_maxburst: u32,
    pub device_fc: bool,
}

impl Default for DmaDirection {
    fn default() -> Self {
        DmaDirection::None
    }
}

impl Default for DmaWidth {
    fn default() -> Self {
        DmaWidth::Width32
    }
}

/// DMA channel state
pub struct DmaChannel {
    /// Channel index
    pub index: usize,
    /// Channel mask (1 << index)
    pub mask: u64,
    /// Channel registers base address
    pub regs: usize,
    /// Current direction
    pub direction: DmaDirection,
    /// Slave configuration
    pub config: DmaSlaveConfig,
    /// Channel priority (0-7, 7 is highest)
    pub priority: u8,
    /// Block size
    pub block_size: u32,
    /// Source peripheral ID
    pub src_id: u8,
    /// Destination peripheral ID
    pub dst_id: u8,
    /// Memory master interface
    pub m_master: u8,
    /// Peripheral master interface
    pub p_master: u8,
    /// Is cyclic transfer
    pub is_cyclic: bool,
    /// Is initialized
    pub is_initialized: bool,
    /// Is paused
    pub is_paused: bool,
}

impl DmaChannel {
    pub const fn new(index: usize, base: usize) -> Self {
        Self {
            index,
            mask: 1 << index,
            regs: base + 0x100 + index * 0x100,
            direction: DmaDirection::None,
            config: DmaSlaveConfig {
                direction: DmaDirection::None,
                src_addr: 0,
                dst_addr: 0,
                src_addr_width: DmaWidth::Width32,
                dst_addr_width: DmaWidth::Width32,
                src_maxburst: 0,
                dst_maxburst: 0,
                device_fc: false,
            },
            priority: 0,
            block_size: DMA_DEFAULT_BLOCK_SIZE,
            src_id: 0,
            dst_id: 0,
            m_master: 0,
            p_master: 1,
            is_cyclic: false,
            is_initialized: false,
            is_paused: false,
        }
    }

    /// Get channel registers
    #[inline]
    fn ch_regs(&self) -> &ChannelRegisters {
        unsafe { &*(self.regs as *const ChannelRegisters) }
    }

    /// Read channel register (64-bit)
    #[inline]
    pub fn read_reg(&self, offset: usize) -> u64 {
        unsafe { read_volatile((self.regs + offset) as *const u64) }
    }

    /// Write channel register (64-bit)
    #[inline]
    pub fn write_reg(&self, offset: usize, value: u64) {
        unsafe { write_volatile((self.regs + offset) as *mut u64, value) }
    }

    /// Read SAR (Source Address Register)
    pub fn read_sar(&self) -> u64 {
        self.ch_regs().sar.get()
    }

    /// Write SAR
    pub fn write_sar(&self, addr: u64) {
        self.ch_regs().sar.set(addr);
    }

    /// Read DAR (Destination Address Register)
    pub fn read_dar(&self) -> u64 {
        self.ch_regs().dar.get()
    }

    /// Write DAR
    pub fn write_dar(&self, addr: u64) {
        self.ch_regs().dar.set(addr);
    }

    /// Read LLP (Linked List Pointer)
    pub fn read_llp(&self) -> u64 {
        self.ch_regs().llp.get()
    }

    /// Write LLP
    pub fn write_llp(&self, addr: u64) {
        self.ch_regs().llp.set(addr);
    }

    /// Read CTL (Control Register)
    pub fn read_ctl(&self) -> u64 {
        self.ch_regs().ctl.get()
    }

    /// Write CTL
    pub fn write_ctl(&self, value: u64) {
        self.ch_regs().ctl.set(value);
    }

    /// Read CFG (Configuration Register)
    pub fn read_cfg(&self) -> u64 {
        self.ch_regs().cfg.get()
    }

    /// Write CFG
    pub fn write_cfg(&self, value: u64) {
        self.ch_regs().cfg.set(value);
    }

    /// Read block transfer size
    pub fn read_block_ts(&self) -> u64 {
        self.ch_regs().block_ts.get()
    }

    /// Write block transfer size
    pub fn write_block_ts(&self, size: u64) {
        self.ch_regs().block_ts.set(size);
    }

    /// Read interrupt status
    pub fn read_int_status(&self) -> u64 {
        self.ch_regs().int_status.get()
    }

    /// Clear interrupt
    pub fn clear_interrupt(&self, mask: u64) {
        self.ch_regs().int_clear.set(mask);
    }

    /// Enable interrupt
    pub fn enable_interrupt(&self, mask: u64) {
        self.ch_regs().int_status_en.set(mask);
    }

    /// Disable interrupt
    pub fn disable_interrupt(&self) {
        self.ch_regs().int_status_en.set(0);
    }
}

/// DMA Controller
pub struct DmaController {
    /// Base address of DMA registers
    base: usize,
    /// Number of channels
    pub nr_channels: usize,
    /// Number of masters
    pub nr_masters: usize,
    /// Data width for each master
    pub data_width: [u32; DMA_MAX_MASTERS],
    /// Block size
    pub block_size: u32,
    /// Channels in use bitmask
    pub in_use: u64,
    /// DMA channels
    pub channels: [DmaChannel; DMA_MAX_CHANNELS],
}

impl DmaController {
    /// Create a new DMA controller instance
    ///
    /// # Safety
    /// The base address must point to valid DMA controller registers
    pub const fn new(base: usize) -> Self {
        Self {
            base,
            nr_channels: DMA_MAX_CHANNELS,
            nr_masters: 2,
            data_width: [4, 4, 4, 4],
            block_size: DMA_DEFAULT_BLOCK_SIZE,
            in_use: 0,
            channels: [
                DmaChannel::new(0, base),
                DmaChannel::new(1, base),
                DmaChannel::new(2, base),
                DmaChannel::new(3, base),
                DmaChannel::new(4, base),
                DmaChannel::new(5, base),
                DmaChannel::new(6, base),
                DmaChannel::new(7, base),
            ],
        }
    }

    /// Get global DMA registers
    #[inline]
    fn regs(&self) -> &DmaRegisters {
        unsafe { &*(self.base as *const DmaRegisters) }
    }

    /// Read global register (64-bit)
    #[inline]
    pub fn read_reg(&self, offset: usize) -> u64 {
        unsafe { read_volatile((self.base + offset) as *const u64) }
    }

    /// Write global register (64-bit)
    #[inline]
    pub fn write_reg(&self, offset: usize, value: u64) {
        unsafe { write_volatile((self.base + offset) as *mut u64, value) }
    }

    /// Initialize the DMA controller
    pub fn init(&mut self) {
        self.reset();
        self.disable();

        for i in 0..self.nr_channels {
            self.channels[i].priority = (self.nr_channels - i - 1) as u8;
            self.channels[i].block_size = self.block_size;
            self.channels[i].clear_interrupt(0xFFFF_FFFF);
        }

        self.clear_channel_enable_bits();
    }

    /// Reset DMA controller
    pub fn reset(&self) {
        self.regs().reset.set(1);
        while self.regs().reset.get() != 0 {}
    }

    /// Enable DMA controller
    pub fn enable(&self) {
        self.regs().cfg.set(CFG_DMA_EN | CFG_INT_EN);
    }

    /// Disable DMA controller
    pub fn disable(&self) {
        self.regs().cfg.set(0);
    }

    /// Check if DMA is enabled
    pub fn is_enabled(&self) -> bool {
        (self.regs().cfg.get() & CFG_DMA_EN) != 0
    }

    /// Read channel enable register
    pub fn read_ch_en(&self) -> u64 {
        self.regs().ch_en.get()
    }

    /// Check if channel is enabled
    pub fn is_channel_enabled(&self, ch: usize) -> bool {
        (self.read_ch_en() & (1 << ch)) != 0
    }

    /// Enable a channel
    pub fn enable_channel(&self, ch: usize) {
        let mask = 1u64 << ch;
        let we_mask = mask << CH_EN_WE_OFFSET;
        self.regs().ch_en.set(mask | we_mask);
    }

    /// Disable a channel
    pub fn disable_channel(&self, ch: usize) {
        let mask = 1u64 << ch;
        let we_mask = mask << CH_EN_WE_OFFSET;
        let abort_we = mask << CH_ABORT_WE_OFFSET;
        let abort = mask << CH_ABORT_OFFSET;

        let mut val = self.read_ch_en();
        val |= we_mask | abort_we | abort;
        val &= !mask;
        self.regs().ch_en.set(val);

        while self.is_channel_enabled(ch) {}
    }

    /// Pause a channel
    pub fn pause_channel(&self, ch: usize) {
        let pause_bit = 1u64 << (ch as u32 + CH_PAUSE_OFFSET);
        let pause_en_bit = 1u64 << (ch as u32 + CH_PAUSE_EN_OFFSET);
        let val = self.regs().ch_en.get() | pause_bit | pause_en_bit;
        self.regs().ch_en.set(val);
    }

    /// Resume a channel
    pub fn resume_channel(&self, ch: usize) {
        let pause_bit = 1u64 << (ch as u32 + CH_PAUSE_OFFSET);
        let val = self.regs().ch_en.get() & !pause_bit;
        self.regs().ch_en.set(val);
    }

    /// Clear all channel enable bits
    fn clear_channel_enable_bits(&self) {
        let we_mask = 0xFF << CH_EN_WE_OFFSET;
        let pause_en_mask = 0xFF << CH_PAUSE_EN_OFFSET;
        self.regs().ch_en.set(we_mask | pause_en_mask);
    }

    /// Read global interrupt status
    pub fn read_int_status(&self) -> u64 {
        self.regs().int_status.get()
    }

    /// Clear common interrupt
    pub fn clear_common_int(&self, mask: u64) {
        self.regs().comm_int_clear.set(mask);
    }

    /// Allocate a channel
    pub fn alloc_channel(&mut self, ch: usize) -> Result<&mut DmaChannel, &'static str> {
        if ch >= self.nr_channels {
            return Err("Invalid channel number");
        }
        if (self.in_use & (1 << ch)) != 0 {
            return Err("Channel already in use");
        }

        if self.in_use == 0 {
            self.enable();
        }

        self.in_use |= 1 << ch;
        Ok(&mut self.channels[ch])
    }

    /// Free a channel
    pub fn free_channel(&mut self, ch: usize) {
        if ch >= self.nr_channels {
            return;
        }

        self.disable_channel(ch);
        self.channels[ch].disable_interrupt();
        self.channels[ch].is_initialized = false;
        self.channels[ch].is_cyclic = false;

        self.in_use &= !(1 << ch);

        if self.in_use == 0 {
            self.disable();
        }
    }

    /// Configure a channel for transfer
    pub fn configure_channel(&mut self, ch: usize, config: &DmaSlaveConfig) -> Result<(), &'static str> {
        if ch >= self.nr_channels {
            return Err("Invalid channel number");
        }

        let channel = &mut self.channels[ch];
        channel.config = *config;
        channel.direction = config.direction;

        Ok(())
    }

    /// Initialize channel for transfer
    pub fn init_channel(&mut self, ch: usize) {
        let channel = &self.channels[ch];
        if channel.is_initialized {
            return;
        }

        let mut cfg: u64 = 0;

        cfg |= (channel.dst_id as u64) << CFG_DST_PER_SHIFT;
        cfg |= (channel.src_id as u64) << CFG_SRC_PER_SHIFT;
        cfg |= (15u64) << CFG_SRC_OSR_LMT_SHIFT;
        cfg |= (15u64) << CFG_DST_OSR_LMT_SHIFT;
        cfg |= (channel.priority as u64) << CFG_CH_PRIOR_SHIFT;
        cfg |= (DmaMultiBlockType::LinkList as u64) << CFG_DST_MULTBLK_TYPE_SHIFT;
        cfg |= (DmaMultiBlockType::LinkList as u64) << CFG_SRC_MULTBLK_TYPE_SHIFT;

        match channel.direction {
            DmaDirection::MemToMem => {
                cfg |= (DmaFlowControl::DmaM2M as u64) << CFG_TT_FC_SHIFT;
            }
            DmaDirection::MemToDev => {
                let fc = if channel.config.device_fc {
                    DmaFlowControl::DstM2P
                } else {
                    DmaFlowControl::DmaM2P
                };
                cfg |= (fc as u64) << CFG_TT_FC_SHIFT;
                // 位 36 = 0：目标侧 **硬件** 握手（与 Linux `DWC_CFG_HS_SEL_DST_HW` 即 0<<36 一致）
            }
            DmaDirection::DevToMem => {
                let fc = if channel.config.device_fc {
                    DmaFlowControl::SrcP2M
                } else {
                    DmaFlowControl::DmaP2M
                };
                cfg |= (fc as u64) << CFG_TT_FC_SHIFT;
                // 位 35 = 0：源侧 **硬件** 握手
            }
            DmaDirection::None => {}
        }

        channel.write_cfg(cfg);

        let int_en = if channel.is_cyclic {
            INT_BLOCK_TFR_DONE
        } else {
            INT_DMA_TFR_DONE
        };
        channel.enable_interrupt(int_en);

        self.channels[ch].is_initialized = true;
    }

    /// Start a transfer with LLI
    pub fn start_transfer(&mut self, ch: usize, lli_phys: u64) -> Result<(), &'static str> {
        if ch >= self.nr_channels {
            return Err("Invalid channel number");
        }

        let mut retry = 0;
        while self.is_channel_enabled(ch) {
            retry += 1;
            if retry > 3000 {
                return Err("Channel busy timeout");
            }
        }

        self.init_channel(ch);

        self.channels[ch].write_llp(lli_phys);

        self.enable_channel(ch);

        Ok(())
    }

    /// Handle interrupt for all channels
    pub fn handle_interrupt(&mut self) -> u64 {
        let status = self.read_int_status();
        if status == 0 {
            return 0;
        }

        self.clear_common_int(0x10F);

        let mut handled = 0u64;

        for ch in 0..self.nr_channels {
            let ch_status = self.channels[ch].read_int_status();
            if ch_status != 0 {
                self.channels[ch].clear_interrupt(ch_status);
                handled |= 1 << ch;
            }
        }

        handled
    }

    /// Get transfer residue for a channel
    pub fn get_residue(&self, ch: usize) -> u64 {
        if ch >= self.nr_channels {
            return 0;
        }

        let channel = &self.channels[ch];
        let block_ts = channel.read_block_ts() & BLOCK_TS_MASK;
        let ctl = channel.read_ctl();
        let width = (ctl >> 8) & 0x7;

        (block_ts + 1) * (1 << width)
    }
}

/// Build CTL register value for memory-to-memory transfer
pub fn build_ctl_m2m(
    src_width: DmaWidth,
    dst_width: DmaWidth,
    src_msize: DmaMsize,
    dst_msize: DmaMsize,
) -> u64 {
    let mut ctl: u64 = 0;

    ctl |= (dst_msize as u64) << CTL_DST_MSIZE_SHIFT;
    ctl |= (src_msize as u64) << CTL_SRC_MSIZE_SHIFT;
    ctl |= (dst_width as u64) << CTL_DST_WIDTH_SHIFT;
    ctl |= (src_width as u64) << CTL_SRC_WIDTH_SHIFT;
    ctl |= CTL_DST_INC;
    ctl |= CTL_SRC_INC;
    ctl |= CTL_DST_STA_EN;
    ctl |= CTL_SRC_STA_EN;

    ctl
}

/// Build CTL register value for slave transfer
pub fn build_ctl_slave(
    direction: DmaDirection,
    mem_width: DmaWidth,
    reg_width: DmaWidth,
    msize: DmaMsize,
) -> u64 {
    let mut ctl: u64 = 0;

    ctl |= (msize as u64) << CTL_DST_MSIZE_SHIFT;
    ctl |= (msize as u64) << CTL_SRC_MSIZE_SHIFT;
    ctl |= CTL_DST_STA_EN;
    ctl |= CTL_SRC_STA_EN;

    match direction {
        DmaDirection::MemToDev => {
            ctl |= (reg_width as u64) << CTL_DST_WIDTH_SHIFT;
            ctl |= (mem_width as u64) << CTL_SRC_WIDTH_SHIFT;
            ctl |= CTL_DST_FIX;
            ctl |= CTL_SRC_INC;
        }
        DmaDirection::DevToMem => {
            ctl |= (mem_width as u64) << CTL_DST_WIDTH_SHIFT;
            ctl |= (reg_width as u64) << CTL_SRC_WIDTH_SHIFT;
            ctl |= CTL_DST_INC;
            ctl |= CTL_SRC_FIX;
        }
        _ => {}
    }

    ctl
}

/// Prepare a memory-to-memory LLI chain
///
/// `lli_array_phys` 必须是 `lli_array[0]` 的 **物理地址**。DMAC 通过 LLP 取下一描述符时只用物理地址；
/// 若误填虚拟地址（例如把 `&lli_array[1]` 直接当 `u64`），在 MMU 下 VA≠PA 时传输会失败或挂死。
///
/// Returns the number of LLIs used
pub fn prepare_memcpy_lli(
    lli_array: &mut [DmaLli],
    lli_array_phys: u64,
    src: u64,
    dst: u64,
    len: usize,
    block_size: u32,
    data_width: u32,
) -> usize {
    if lli_array.is_empty() || len == 0 {
        return 0;
    }

    let trans_width = (data_width | src as u32 | dst as u32 | len as u32).trailing_zeros() as u8;
    let trans_width = trans_width.min(6);

    let ctl = build_ctl_m2m(
        DmaWidth::from_bytes(1 << trans_width),
        DmaWidth::from_bytes(1 << trans_width),
        DmaMsize::Msize32,
        DmaMsize::Msize32,
    );

    let max_block = block_size as usize;
    let mut offset = 0usize;
    let mut lli_count = 0usize;

    while offset < len && lli_count < lli_array.len() {
        let xfer_count = ((len - offset) >> trans_width).min(max_block >> trans_width);

        let lli = &mut lli_array[lli_count];
        lli.sar = src + offset as u64;
        lli.dar = dst + offset as u64;
        lli.block_ts = (xfer_count - 1) as u64;
        lli.ctl = ctl | CTL_LLI_VALID;

        offset += xfer_count << trans_width;
        lli_count += 1;
    }

    if lli_count > 0 {
        lli_array[lli_count - 1].ctl |= CTL_LLI_LAST;
        lli_array[lli_count - 1].llp = 0;

        let stride = core::mem::size_of::<DmaLli>() as u64;
        for i in 0..lli_count - 1 {
            lli_array[i].llp = lli_array_phys + (i as u64 + 1) * stride;
        }
    }

    lli_count
}
