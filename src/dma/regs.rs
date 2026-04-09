//! CVITEK DMA Controller Register Definitions
//!
//! Based on Synopsys DesignWare AXI DMA IP

use tock_registers::{
    register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

// =============================================================================
// Global DMA Register Offsets
// =============================================================================

pub const DMA_ID_OFFSET: usize = 0x00;
pub const DMA_COMPVER_OFFSET: usize = 0x08;
pub const DMA_CFG_OFFSET: usize = 0x10;
pub const DMA_CH_EN_OFFSET: usize = 0x18;
pub const DMA_INT_STATUS_OFFSET: usize = 0x30;
pub const DMA_COMM_INT_CLEAR_OFFSET: usize = 0x38;
pub const DMA_COMM_INT_STATUS_OFFSET: usize = 0x40;
pub const DMA_RESET_OFFSET: usize = 0x58;

// =============================================================================
// Channel Register Offsets (relative to channel base)
// =============================================================================

pub const CH_SAR_OFFSET: usize = 0x00;
pub const CH_DAR_OFFSET: usize = 0x08;
pub const CH_BLOCK_TS_OFFSET: usize = 0x10;
pub const CH_CTL_OFFSET: usize = 0x18;
pub const CH_CFG_OFFSET: usize = 0x20;
pub const CH_LLP_OFFSET: usize = 0x28;
pub const CH_STATUS_OFFSET: usize = 0x30;
pub const CH_SWHSSRC_OFFSET: usize = 0x38;
pub const CH_SWHSDST_OFFSET: usize = 0x40;
pub const CH_BLK_TFR_RESUME_OFFSET: usize = 0x48;
pub const CH_AXI_ID_OFFSET: usize = 0x50;
pub const CH_AXI_QOS_OFFSET: usize = 0x58;
pub const CH_SSTAT_OFFSET: usize = 0x60;
pub const CH_DSTAT_OFFSET: usize = 0x68;
pub const CH_SSTATAR_OFFSET: usize = 0x70;
pub const CH_DSTATAR_OFFSET: usize = 0x78;
pub const CH_INT_STATUS_EN_OFFSET: usize = 0x80;
pub const CH_INT_STATUS_OFFSET: usize = 0x88;
pub const CH_INT_SIGNAL_EN_OFFSET: usize = 0x90;
pub const CH_INT_CLEAR_OFFSET: usize = 0x98;

// =============================================================================
// Global CFG Register Bits
// =============================================================================

pub const CFG_DMA_EN: u64 = 1 << 0;
pub const CFG_INT_EN: u64 = 1 << 1;

// =============================================================================
// Channel Enable Register Bits
// =============================================================================

pub const CH_EN_WE_OFFSET: u32 = 8;
pub const CH_PAUSE_OFFSET: u32 = 16;
pub const CH_PAUSE_EN_OFFSET: u32 = 24;
pub const CH_ABORT_OFFSET: u32 = 32;
pub const CH_ABORT_WE_OFFSET: u32 = 40;

// =============================================================================
// Channel CTL Register Bits
// =============================================================================

pub const CTL_SMS_SHIFT: u32 = 0;
pub const CTL_DMS_SHIFT: u32 = 2;
pub const CTL_SINC_SHIFT: u32 = 4;
pub const CTL_DINC_SHIFT: u32 = 6;
pub const CTL_SRC_WIDTH_SHIFT: u32 = 8;
pub const CTL_DST_WIDTH_SHIFT: u32 = 11;
pub const CTL_SRC_MSIZE_SHIFT: u32 = 14;
pub const CTL_DST_MSIZE_SHIFT: u32 = 18;
pub const CTL_ARLEN_EN_SHIFT: u32 = 38;
pub const CTL_ARLEN_SHIFT: u32 = 39;
pub const CTL_AWLEN_EN_SHIFT: u32 = 47;
pub const CTL_AWLEN_SHIFT: u32 = 48;
pub const CTL_SRC_STA_EN_SHIFT: u32 = 56;
pub const CTL_DST_STA_EN_SHIFT: u32 = 57;
pub const CTL_IOC_BLT_EN_SHIFT: u32 = 58;
pub const CTL_LLI_LAST_SHIFT: u32 = 62;
pub const CTL_LLI_VALID_SHIFT: u32 = 63;

pub const CTL_SRC_INC: u64 = 0 << CTL_SINC_SHIFT;
pub const CTL_SRC_FIX: u64 = 1 << CTL_SINC_SHIFT;
pub const CTL_DST_INC: u64 = 0 << CTL_DINC_SHIFT;
pub const CTL_DST_FIX: u64 = 1 << CTL_DINC_SHIFT;
pub const CTL_SRC_STA_EN: u64 = 1 << CTL_SRC_STA_EN_SHIFT;
pub const CTL_DST_STA_EN: u64 = 1 << CTL_DST_STA_EN_SHIFT;
pub const CTL_IOC_BLT_EN: u64 = 1 << CTL_IOC_BLT_EN_SHIFT;
pub const CTL_LLI_LAST: u64 = 1 << CTL_LLI_LAST_SHIFT;
pub const CTL_LLI_VALID: u64 = 1 << CTL_LLI_VALID_SHIFT;

pub const CTL_ARLEN_EN: u64 = 1 << CTL_ARLEN_EN_SHIFT;
pub const CTL_AWLEN_EN: u64 = 1 << CTL_AWLEN_EN_SHIFT;

// =============================================================================
// Channel CFG Register Bits
// =============================================================================

pub const CFG_SRC_MULTBLK_TYPE_SHIFT: u32 = 0;
pub const CFG_DST_MULTBLK_TYPE_SHIFT: u32 = 2;
pub const CFG_TT_FC_SHIFT: u32 = 32;
pub const CFG_HS_SEL_SRC_SHIFT: u32 = 35;
pub const CFG_HS_SEL_DST_SHIFT: u32 = 36;
pub const CFG_SRC_HWHS_POL_SHIFT: u32 = 37;
pub const CFG_DST_HWHS_POL_SHIFT: u32 = 38;
pub const CFG_SRC_PER_SHIFT: u32 = 39;
pub const CFG_DST_PER_SHIFT: u32 = 44;
pub const CFG_CH_PRIOR_SHIFT: u32 = 49;
pub const CFG_LOCK_CH_SHIFT: u32 = 52;
pub const CFG_LOCK_CH_L_SHIFT: u32 = 53;
pub const CFG_SRC_OSR_LMT_SHIFT: u32 = 55;
pub const CFG_DST_OSR_LMT_SHIFT: u32 = 59;

/// DesignWare DMAC：位为 **1** 表示 **软件** 握手（Linux `DWC_CFG_HS_SEL_*_SW`）。
/// 硬件外设握手应保持对应位为 **0**（旧版本错误地把「HW」写成 `1<<shift`，会导致 UART 等永远等不到硬件请求）。
pub const CFG_HS_SEL_SRC_SW: u64 = 1 << CFG_HS_SEL_SRC_SHIFT;
pub const CFG_HS_SEL_DST_SW: u64 = 1 << CFG_HS_SEL_DST_SHIFT;
pub const CFG_SRC_HWHS_POL_H: u64 = 1 << CFG_SRC_HWHS_POL_SHIFT;
pub const CFG_DST_HWHS_POL_H: u64 = 1 << CFG_DST_HWHS_POL_SHIFT;

// =============================================================================
// Interrupt Status Bits
// =============================================================================

pub const INT_BLOCK_TFR_DONE: u64 = 1 << 0;
pub const INT_DMA_TFR_DONE: u64 = 1 << 1;
pub const INT_SRC_TRANS_COMP: u64 = 1 << 3;
pub const INT_DST_TRANS_COMP: u64 = 1 << 4;
pub const INT_SRC_DEC_ERR: u64 = 1 << 5;
pub const INT_DST_DEC_ERR: u64 = 1 << 6;
pub const INT_SRC_SLV_ERR: u64 = 1 << 7;
pub const INT_DST_SLV_ERR: u64 = 1 << 8;
pub const INT_LLI_RD_DEC_ERR: u64 = 1 << 9;
pub const INT_LLI_WR_DEC_ERR: u64 = 1 << 10;
pub const INT_LLI_RD_SLV_ERR: u64 = 1 << 11;
pub const INT_LLI_WR_SLV_ERR: u64 = 1 << 12;
pub const INT_SHADOWREG_OR_LLI_INVALID: u64 = 1 << 13;
pub const INT_SLVIF_MULTIBLKTYPE_ERR: u64 = 1 << 14;
pub const INT_SLVIF_DEC_ERR: u64 = 1 << 16;
pub const INT_SLVIF_WR2RO_ERR: u64 = 1 << 17;
pub const INT_SLVIF_RD2WO_ERR: u64 = 1 << 18;
pub const INT_SLVIF_WRONCHEN_ERR: u64 = 1 << 19;
pub const INT_SLVIF_SHADOWREG_WRON_VALID_ERR: u64 = 1 << 20;
pub const INT_SLVIF_WRONHOLD_ERR: u64 = 1 << 21;
pub const INT_CH_LOCK_CLEARED: u64 = 1 << 27;
pub const INT_CH_SRC_SUSPENDED: u64 = 1 << 28;
pub const INT_CH_SUSPENDED: u64 = 1 << 29;
pub const INT_CH_DISABLED: u64 = 1 << 30;
pub const INT_CH_ABORTED: u64 = 1 << 31;

pub const INT_ALL_ERR: u64 = INT_SRC_DEC_ERR
    | INT_DST_DEC_ERR
    | INT_SRC_SLV_ERR
    | INT_DST_SLV_ERR
    | INT_LLI_RD_DEC_ERR
    | INT_LLI_WR_DEC_ERR
    | INT_LLI_RD_SLV_ERR
    | INT_LLI_WR_SLV_ERR;

// =============================================================================
// Block Transfer Size
// =============================================================================

pub const BLOCK_TS_MASK: u64 = 0x3FFFFF;

// =============================================================================
// LLP Register Bits
// =============================================================================

pub const LLP_LOC_MASK: u64 = !0x3F;

#[inline]
pub const fn llp_loc(llp: u64) -> u64 {
    llp & LLP_LOC_MASK
}

// =============================================================================
// Register Structures using tock-registers
// =============================================================================

register_structs! {
    /// Global DMA Registers
    pub DmaRegisters {
        (0x00 => pub id: ReadOnly<u64>),
        (0x08 => pub comp_ver: ReadOnly<u64>),
        (0x10 => pub cfg: ReadWrite<u64>),
        (0x18 => pub ch_en: ReadWrite<u64>),
        (0x20 => _reserved0: [u8; 0x10]),
        (0x30 => pub int_status: ReadOnly<u64>),
        (0x38 => pub comm_int_clear: WriteOnly<u64>),
        (0x40 => pub comm_int_status: ReadOnly<u64>),
        (0x48 => _reserved1: [u8; 0x10]),
        (0x58 => pub reset: ReadWrite<u64>),
        (0x60 => @END),
    }
}

register_structs! {
    /// Channel Registers
    pub ChannelRegisters {
        (0x00 => pub sar: ReadWrite<u64>),
        (0x08 => pub dar: ReadWrite<u64>),
        (0x10 => pub block_ts: ReadWrite<u64>),
        (0x18 => pub ctl: ReadWrite<u64>),
        (0x20 => pub cfg: ReadWrite<u64>),
        (0x28 => pub llp: ReadWrite<u64>),
        (0x30 => pub status: ReadOnly<u64>),
        (0x38 => pub swhssrc: ReadWrite<u64>),
        (0x40 => pub swhsdst: ReadWrite<u64>),
        (0x48 => pub blk_tfr_resume: WriteOnly<u64>),
        (0x50 => pub axi_id: ReadWrite<u64>),
        (0x58 => pub axi_qos: ReadWrite<u64>),
        (0x60 => pub sstat: ReadOnly<u64>),
        (0x68 => pub dstat: ReadOnly<u64>),
        (0x70 => pub sstatar: ReadWrite<u64>),
        (0x78 => pub dstatar: ReadWrite<u64>),
        (0x80 => pub int_status_en: ReadWrite<u64>),
        (0x88 => pub int_status: ReadOnly<u64>),
        (0x90 => pub int_signal_en: ReadWrite<u64>),
        (0x98 => pub int_clear: WriteOnly<u64>),
        (0xA0 => @END),
    }
}

// =============================================================================
// Helper functions for CTL register field construction
// =============================================================================

#[inline]
pub const fn ctl_dst_msize(msize: u8) -> u64 { (msize as u64) << CTL_DST_MSIZE_SHIFT }
#[inline]
pub const fn ctl_src_msize(msize: u8) -> u64 { (msize as u64) << CTL_SRC_MSIZE_SHIFT }
#[inline]
pub const fn ctl_dst_width(width: u8) -> u64 { (width as u64) << CTL_DST_WIDTH_SHIFT }
#[inline]
pub const fn ctl_src_width(width: u8) -> u64 { (width as u64) << CTL_SRC_WIDTH_SHIFT }
#[inline]
pub const fn ctl_dms(master: u8) -> u64 { (master as u64) << CTL_DMS_SHIFT }
#[inline]
pub const fn ctl_sms(master: u8) -> u64 { (master as u64) << CTL_SMS_SHIFT }
#[inline]
pub const fn ctl_arlen(len: u8) -> u64 { (len as u64) << CTL_ARLEN_SHIFT }
#[inline]
pub const fn ctl_awlen(len: u8) -> u64 { (len as u64) << CTL_AWLEN_SHIFT }

// =============================================================================
// Helper functions for CFG register field construction
// =============================================================================

#[inline]
pub const fn cfg_src_per(id: u8) -> u64 { (id as u64) << CFG_SRC_PER_SHIFT }
#[inline]
pub const fn cfg_dst_per(id: u8) -> u64 { (id as u64) << CFG_DST_PER_SHIFT }
#[inline]
pub const fn cfg_ch_prior(priority: u8) -> u64 { (priority as u64) << CFG_CH_PRIOR_SHIFT }
#[inline]
pub const fn cfg_src_osr_lmt(limit: u8) -> u64 { (limit as u64) << CFG_SRC_OSR_LMT_SHIFT }
#[inline]
pub const fn cfg_dst_osr_lmt(limit: u8) -> u64 { (limit as u64) << CFG_DST_OSR_LMT_SHIFT }
#[inline]
pub const fn cfg_tt_fc(fc: u8) -> u64 { (fc as u64) << CFG_TT_FC_SHIFT }
#[inline]
pub const fn cfg_src_multblk_type(mbt: u8) -> u64 { (mbt as u64) << CFG_SRC_MULTBLK_TYPE_SHIFT }
#[inline]
pub const fn cfg_dst_multblk_type(mbt: u8) -> u64 { (mbt as u64) << CFG_DST_MULTBLK_TYPE_SHIFT }
