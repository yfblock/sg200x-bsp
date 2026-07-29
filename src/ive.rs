//! IVE (Image Video Engine) 硬件 CSC（YUV→RGB）驱动。
//!
//! SG2002 的 IVE 在 0x0A0A0000，有专用 CSC 硬件，支持内存到内存转换。
//! 小核可访问（已验证）。CSC 在 FILTEROP 块里实现，通过直接写寄存器启动。
//!
//! 参考：osdrv/interdrv/v2/ive/hal/mars/cvi_ive_platform.c 的 _cvi_ive_csc()。
//!
//! 寄存器块偏移（相对 IVE base 0x0A0A0000）：
//!   IVE_TOP    @ +0x0000
//!   IMG_IN     @ +0x0400
//!   FILTEROP   @ +0x2000

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU32, Ordering};

/// IVE 寄存器基址。
const IVE_BASE: usize = 0x0A0A_0000;

/// IVE_TOP 寄存器偏移。
#[allow(dead_code)] // 保留：完整寄存器映射的一部分，本驱动未用到
const IVE_TOP_REG_0: usize = 0x0000; // trig_cnt [7:4]
const IVE_TOP_REG_1: usize = 0x0004; // softrst[0], shdw_sel[1], fmt_vld_fg[4]=START
const IVE_TOP_REG_2: usize = 0x0008; // width-1 [12:0], height-1 [28:16]
const IVE_TOP_REG_H10: usize = 0x0010; // top enables
const IVE_TOP_REG_90: usize = 0x0090; // frame done status (RO)
const IVE_TOP_REG_94: usize = 0x0094; // interrupt enable
const IVE_TOP_REG_98: usize = 0x0098; // interrupt status

/// IMG_IN 寄存器偏移（相对 IVE_BASE + 0x400）。
const IMG_IN_OFF: usize = 0x0400;
const IMG_IN_REG_00: usize = IMG_IN_OFF + 0x00; // src_sel[1:0], fmt_sel[7:4], burst[11:8], csc_en[12], auto_csc[13]
const IMG_IN_REG_02: usize = IMG_IN_OFF + 0x08; // width-1, height-1
const IMG_IN_REG_03: usize = IMG_IN_OFF + 0x0C; // Y pitch
const IMG_IN_REG_04: usize = IMG_IN_OFF + 0x10; // C pitch
const IMG_IN_REG_05: usize = IMG_IN_OFF + 0x14; // shrd_sel[2] = immediate update
const IMG_IN_REG_Y_BASE_0: usize = IMG_IN_OFF + 0x24;
const IMG_IN_REG_Y_BASE_1: usize = IMG_IN_OFF + 0x28;
const IMG_IN_REG_U_BASE_0: usize = IMG_IN_OFF + 0x2C;
const IMG_IN_REG_U_BASE_1: usize = IMG_IN_OFF + 0x30;
const IMG_IN_REG_V_BASE_0: usize = IMG_IN_OFF + 0x34;
const IMG_IN_REG_V_BASE_1: usize = IMG_IN_OFF + 0x38;
const IMG_IN_REG_068: usize = IMG_IN_OFF + 0x68; // ip_clr_w1t[18], ip_idle[16], ip_int[17]

/// FILTEROP 寄存器偏移（相对 IVE_BASE + 0x2000）。
const FOP_OFF: usize = 0x2000;
const FOP_REG_H10: usize = FOP_OFF + 0x10; // filterop_mode[3:0] = 1 for CSC
const FOP_REG_H14: usize = FOP_OFF + 0x14; // 3ch_en[1], op_y_wdma_en[2]
const FOP_REG_H194: usize = FOP_OFF + 0x194; // coeff_sw_update[16]
const FOP_REG_CSC_0: usize = FOP_OFF + 0x198; // 12 个 19-bit 系数
const FOP_REG_H1C8: usize = FOP_OFF + 0x1C8; // csc_enable[4], csc_enmode[3:0]

/// FILTEROP ODMA（输出 DMA）寄存器。
const FOP_ODMA_00: usize = FOP_OFF + 0x120; // fmt_sel[11:8], dma_blen[0], dma_en[12]
const FOP_ODMA_01: usize = FOP_OFF + 0x124; // output Y/R base low
const FOP_ODMA_02: usize = FOP_OFF + 0x128; // output Y/R base high
const FOP_ODMA_03: usize = FOP_OFF + 0x12C; // output U/G base low
const FOP_ODMA_04: usize = FOP_OFF + 0x130; // output U/G base high
const FOP_ODMA_05: usize = FOP_OFF + 0x134; // output V/B base low
const FOP_ODMA_06: usize = FOP_OFF + 0x138; // output V/B base high
const FOP_ODMA_07: usize = FOP_OFF + 0x13C; // output Y/R pitch
const FOP_ODMA_08: usize = FOP_OFF + 0x140; // output C pitch
const FOP_ODMA_11: usize = FOP_OFF + 0x148; // output width-1
const FOP_ODMA_12: usize = FOP_OFF + 0x14C; // output height-1

/// BT.601 limited range YUV→RGB 系数（Video BT601 YUV2RGB, mode 0）。
/// 布局：{c00,c01,c02,off0, c10,c11,c12,off1, c20,c21,c22,off2}
/// 来自 cvi_ive_platform.c coef_BT601_to_GBR_16_235。
const CSC_COEF_BT601_LIMIT: [u32; 12] = [
    1024, 0, 1404, 179188, 1024, 344, 715, 136040, 1024, 1774, 0, 226505,
];

/// 输入格式：YUV420 planar = 0x0
const FMT_YUV420P: u32 = 0x0;
/// 输出格式：RGB888 planar = 0x2
const FMT_RGB888_PLANAR: u32 = 0x2;

/// IMG_IN 的 `fmt_sel` 覆盖值。
///
/// 硬件格式枚举没有可用的数据手册，`FMT_YUV420P = 0` 是逆向 osdrv 得到的，
/// 摄像头实际输出是 YUV422，对应的编码未知。做成运行时可写，
/// 配合大核的比对工具扫描候选值——哪个值让 IVE 输出与软件 CSC 参考一致，
/// 哪个就是对的。`u32::MAX` 表示"用默认值"。
static FMT_SEL_OVERRIDE: AtomicU32 = AtomicU32::new(u32::MAX);

/// 设置 IMG_IN `fmt_sel` 覆盖值（`u32::MAX` = 恢复默认）。
pub fn set_input_fmt(fmt: u32) {
    FMT_SEL_OVERRIDE.store(fmt, Ordering::Relaxed);
}

/// 读当前生效的 `fmt_sel`。
pub fn input_fmt() -> u32 {
    match FMT_SEL_OVERRIDE.load(Ordering::Relaxed) {
        u32::MAX => FMT_YUV420P,
        v => v & 0xF,
    }
}

#[inline]
fn w(addr: usize, val: u32) {
    unsafe { write_volatile((IVE_BASE + addr) as *mut u32, val) };
}

#[inline]
fn r(addr: usize) -> u32 {
    unsafe { read_volatile((IVE_BASE + addr) as *const u32) }
}

#[inline]
fn write_addr(lo_reg: usize, hi_reg: usize, pa: u64) {
    w(lo_reg, pa as u32);
    w(hi_reg, (pa >> 32) as u32);
}

/// 执行一次 YUV420→RGB888 CSC 转换。
///
/// # 参数
/// - `y_pa`, `u_pa`, `v_pa`: 输入 YUV420 planar 的 Y/U/V 物理地址
/// - `y_pitch`, `c_pitch`: 输入 Y/C 的 stride（字节）
/// - `r_pa`, `g_pa`, `b_pa`: 输出 RGB888 planar 的 R/G/B 物理地址
/// - `r_pitch`: 输出 RGB 的 stride（字节，RGB planar 的 G/B pitch 相同）
/// - `width`, `height`: 图像尺寸
///
/// 返回 `Ok(())` 或错误描述。阻塞直到转换完成。
pub fn csc_yuv420_to_rgb888(
    y_pa: usize, u_pa: usize, v_pa: usize,
    y_pitch: u32, c_pitch: u32,
    r_pa: usize, g_pa: usize, b_pa: usize,
    r_pitch: u32,
    width: u32, height: u32,
) -> Result<(), &'static str> {
    let wm1 = width - 1;
    let hm1 = height - 1;

    // 1. 软复位 IVE
    w(IVE_TOP_REG_1, 1); // softrst
    crate::utils::delay::delay(100);
    w(IVE_TOP_REG_1, 0);

    // 2. 图像尺寸
    w(IVE_TOP_REG_2, (hm1 << 16) | (wm1 & 0x1FFF));

    // 3. FILTEROP 配置为 CSC
    w(FOP_REG_H10, 1); // filterop_mode = 1 (FILTER3CH)
    w(FOP_REG_H14, 0); // 清使能

    // 4. CSC 系数
    for i in 0..12 {
        w(FOP_REG_CSC_0 + i * 4, CSC_COEF_BT601_LIMIT[i] & 0x7FFFF);
    }
    w(FOP_REG_H194, 1 << 16); // coeff_sw_update = 1（用软件系数）

    // 5. CSC 使能 + 模式（mode 0 = BT601 limited YUV2RGB）
    w(FOP_REG_H1C8, (1 << 4) | 0); // csc_enable=1, enmode=0

    // 6. 清 IMG_IN IP
    w(IMG_IN_REG_068, 1 << 18); // ip_clr_w1t
    crate::utils::delay::delay(100);
    w(IMG_IN_REG_068, 0);

    // 7. 输入图像配置
    w(IMG_IN_REG_03, y_pitch);
    w(IMG_IN_REG_04, c_pitch);
    w(IMG_IN_REG_02, (hm1 << 16) | (wm1 & 0xFFFF));
    // src_sel=2(DRAM), fmt_sel=YUV420P, burst=8, auto_csc=0
    w(IMG_IN_REG_00, (2 << 0) | (input_fmt() << 4) | (8 << 8) | (1 << 16));
    write_addr(IMG_IN_REG_Y_BASE_0, IMG_IN_REG_Y_BASE_1, y_pa as u64);
    write_addr(IMG_IN_REG_U_BASE_0, IMG_IN_REG_U_BASE_1, u_pa as u64);
    write_addr(IMG_IN_REG_V_BASE_0, IMG_IN_REG_V_BASE_1, v_pa as u64);
    w(IMG_IN_REG_05, 1 << 2); // shrd_sel = 1 (immediate update)

    // 8. 使能 FILTEROP top
    w(IVE_TOP_REG_H10, (1 << 0) | (1 << 3) | (1 << 15)); // img_in + csc + filterop

    // 9. 输出 DMA 配置
    w(FOP_ODMA_07, r_pitch); // Y/R pitch
    w(FOP_ODMA_08, r_pitch); // C pitch (same for RGB planar)
    w(FOP_ODMA_11, wm1 & 0xFFF);
    w(FOP_ODMA_12, hm1 & 0xFFF);
    write_addr(FOP_ODMA_01, FOP_ODMA_02, r_pa as u64);
    write_addr(FOP_ODMA_03, FOP_ODMA_04, g_pa as u64);
    write_addr(FOP_ODMA_05, FOP_ODMA_06, b_pa as u64);
    // fmt_sel=RGB888_PLANAR, dma_blen=1, dma_en=1
    w(FOP_ODMA_00, (FMT_RGB888_PLANAR << 8) | (1 << 0) | (1 << 12));

    // 10. 禁用 Y/C WDMA（CSC 走 ODMA）
    w(FOP_REG_H14, 0); // op_y_wdma_en = 0

    // 11. 清状态 + 启动
    w(IVE_TOP_REG_90, 0);
    w(IVE_TOP_REG_98, 0);
    w(IVE_TOP_REG_94, 0); // 不用中断
    // fmt_vld_fg = 1 启动
    w(IVE_TOP_REG_1, 1 << 4);

    // 12. 轮询完成（REG_90 任意 done bit 置位即完成）
    for _ in 0..1_000_000u32 {
        let status = r(IVE_TOP_REG_90);
        if status != 0 {
            return Ok(());
        }
        crate::utils::delay::delay(10);
    }
    Err("IVE CSC timeout")
}
