//! CIF 寄存器定义
//!
//! 使用 tock-registers 定义 CIF 相关寄存器

/// CIF 寄存器基地址
pub const DPHY_TOP_BASE: usize = 0x0A0D_0000;
pub const DPHY_4L_BASE: usize = 0x0A0D_0300;
pub const DPHY_2L_BASE: usize = 0x0A0D_0600;
pub const SENSOR_MAC0_BASE: usize = 0x0A0C_2000;
pub const SENSOR_MAC1_BASE: usize = 0x0A0C_4000;
pub const SENSOR_MAC_VI_BASE: usize = 0x0A0C_6000;

pub const SENSOR_CSI0_BASE: usize = 0x0A0C_2400;
pub const SENSOR_CSI1_BASE: usize = 0x0A0C_4400;

/// CAM PLL 时钟配置寄存器
pub const CLK_CAM0_SRC_DIV: usize = 0x0300_28C0;
pub const CLK_CAM1_SRC_DIV: usize = 0x0300_28C4;

/// 中断号
pub const CSIMAC0_INTR_NUM: u32 = 22;
pub const CSIMAC1_INTR_NUM: u32 = 23;

/// 中断状态位偏移
pub const CIF_INT_STS_ECC_ERR_OFFSET: u32 = 0;
pub const CIF_INT_STS_CRC_ERR_OFFSET: u32 = 1;
pub const CIF_INT_STS_HDR_ERR_OFFSET: u32 = 2;
pub const CIF_INT_STS_WC_ERR_OFFSET: u32 = 3;
pub const CIF_INT_STS_FIFO_FULL_OFFSET: u32 = 4;

/// 中断状态掩码
pub const CIF_INT_STS_ECC_ERR_MASK: u32 = 1 << CIF_INT_STS_ECC_ERR_OFFSET;
pub const CIF_INT_STS_CRC_ERR_MASK: u32 = 1 << CIF_INT_STS_CRC_ERR_OFFSET;
pub const CIF_INT_STS_HDR_ERR_MASK: u32 = 1 << CIF_INT_STS_HDR_ERR_OFFSET;
pub const CIF_INT_STS_WC_ERR_MASK: u32 = 1 << CIF_INT_STS_WC_ERR_OFFSET;
pub const CIF_INT_STS_FIFO_FULL_MASK: u32 = 1 << CIF_INT_STS_FIFO_FULL_OFFSET;

/// 获取 MAC 物理寄存器基地址
pub fn get_mac_phys_reg_bases(link: u32) -> usize {
    match link {
        0 => SENSOR_MAC0_BASE,
        1 => SENSOR_MAC1_BASE,
        2 => SENSOR_MAC_VI_BASE,
        _ => 0,
    }
}

/// 获取 Wrap 物理寄存器基地址
pub fn get_wrap_phys_reg_bases(_link: u32) -> usize {
    DPHY_TOP_BASE
}

/// 读取寄存器
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn reg_read(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// 写入寄存器
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn reg_write(addr: usize, val: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, val) };
}

/// 修改寄存器位
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn reg_modify(addr: usize, clear_mask: u32, set_mask: u32) {
    let val = unsafe { reg_read(addr) };
    let val = (val & !clear_mask) | set_mask;
    unsafe { reg_write(addr, val) };
}

/// 设置寄存器位
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn reg_setbits(addr: usize, mask: u32) {
    let val = unsafe { reg_read(addr) };
    unsafe { reg_write(addr, val | mask) };
}

/// 清除寄存器位
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn reg_clrbits(addr: usize, mask: u32) {
    let val = unsafe { reg_read(addr) };
    unsafe { reg_write(addr, val & !mask) };
}
