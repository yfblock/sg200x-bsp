//! # SG2002 多处理器启动模块
//!
//! 本模块提供 SG2002 芯片多处理器 (MP) 启动功能的 Rust 驱动实现。
//!
//! ## 功能特性
//!
//! - 支持启动协处理器 (小核 C906@700MHz)
//! - 支持设置协处理器启动地址
//!
//! ## 硬件资源
//!
//! SG2002 芯片包含以下处理器核心：
//! - 大核: RISC-V C906@1GHz 或 ARM Cortex-A53@1GHz
//! - 小核 (协处理器): RISC-V C906@700MHz
//!
//! ## 相关寄存器
//!
//! | 偏移（相对 [`SEC_SYS_BASE`]） | 名称 | 描述 |
//! |------|------|------|
//! | 0x004 | SEC_SYS_CTRL | 安全子系统控制寄存器 |
//! | 0x020 | SEC_SYS_BOOT_ADDR_L | 协处理器启动地址低 32 位 |
//! | 0x024 | SEC_SYS_BOOT_ADDR_H | 协处理器启动地址高 32 位 |
//!
//! CPU 软复位见 [`crate::rstc`]（[`crate::soc::RSTC_BASE`] + 0x024，`SOFT_CPU_RSTN`）。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::mp::SecSys;
//! use sg200x_bsp::soc::SEC_SYS_BASE;
//!
//! let sec_sys = unsafe { SecSys::new(SEC_SYS_BASE) };
//! unsafe {
//!     sec_sys.start_secondary_core(entry_address);
//! }
//! ```

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

use crate::rstc::{Rstc, RSTC_BASE};

pub use crate::soc::SEC_SYS_BASE;

// ============================================================================
// 寄存器位域定义 (使用 tock-registers)
// ============================================================================

register_bitfields! [
    u32,

    /// 安全子系统控制寄存器 (偏移 0x004)
    pub SEC_SYS_CTRL [
        /// 协处理器 (小核) 使能位
        /// 1 = 使能协处理器
        /// 0 = 禁用协处理器
        SEC_CPU_EN OFFSET(13) NUMBITS(1) []
    ],

    /// 协处理器启动地址低 32 位 (偏移 0x020)
    pub SEC_SYS_BOOT_ADDR_L [
        /// 启动地址低 32 位
        ADDR_L OFFSET(0) NUMBITS(32) []
    ],

    /// 协处理器启动地址高 32 位 (偏移 0x024)
    pub SEC_SYS_BOOT_ADDR_H [
        /// 启动地址高 32 位
        ADDR_H OFFSET(0) NUMBITS(32) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// 安全子系统寄存器组
    pub SecSysRegisters {
        /// 保留 (偏移 0x000)
        (0x000 => _reserved0),

        /// 安全子系统控制寄存器 (偏移 0x004)
        (0x004 => pub ctrl: ReadWrite<u32, SEC_SYS_CTRL::Register>),

        /// 保留 (偏移 0x008-0x01c)
        (0x008 => _reserved1),

        /// 协处理器启动地址低 32 位 (偏移 0x020)
        (0x020 => pub boot_addr_l: ReadWrite<u32, SEC_SYS_BOOT_ADDR_L::Register>),

        /// 协处理器启动地址高 32 位 (偏移 0x024)
        (0x024 => pub boot_addr_h: ReadWrite<u32, SEC_SYS_BOOT_ADDR_H::Register>),

        /// 结束标记
        (0x028 => @END),
    }
}

// ============================================================================
// 安全子系统驱动
// ============================================================================

/// 安全子系统驱动结构体
///
/// 用于控制协处理器 (小核 C906@700MHz) 的启动
pub struct SecSys {
    /// 寄存器组引用
    regs: &'static SecSysRegisters,
}

impl SecSys {
    /// 创建新的安全子系统驱动实例
    ///
    /// # 参数
    ///
    /// - `base`: 安全子系统 MMIO 基地址（见 [`SEC_SYS_BASE`]）
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new(base: usize) -> Self {
        unsafe {
            Self {
                regs: &*(base as *const SecSysRegisters),
            }
        }
    }

    /// 使能协处理器
    pub fn enable_secondary_cpu(&self) {
        use tock_registers::interfaces::ReadWriteable;
        self.regs.ctrl.modify(SEC_SYS_CTRL::SEC_CPU_EN::SET);
    }

    /// 禁用协处理器
    pub fn disable_secondary_cpu(&self) {
        use tock_registers::interfaces::ReadWriteable;
        self.regs.ctrl.modify(SEC_SYS_CTRL::SEC_CPU_EN::CLEAR);
    }

    /// 检查协处理器是否使能
    pub fn is_secondary_cpu_enabled(&self) -> bool {
        use tock_registers::interfaces::Readable;
        self.regs.ctrl.is_set(SEC_SYS_CTRL::SEC_CPU_EN)
    }

    /// 设置协处理器启动地址
    ///
    /// # 参数
    /// - `addr`: 64 位启动地址
    pub fn set_boot_address(&self, addr: u64) {
        use tock_registers::interfaces::Writeable;
        self.regs
            .boot_addr_l
            .write(SEC_SYS_BOOT_ADDR_L::ADDR_L.val(addr as u32));
        self.regs
            .boot_addr_h
            .write(SEC_SYS_BOOT_ADDR_H::ADDR_H.val((addr >> 32) as u32));
    }

    /// 获取协处理器启动地址
    pub fn get_boot_address(&self) -> u64 {
        use tock_registers::interfaces::Readable;
        let low = self.regs.boot_addr_l.read(SEC_SYS_BOOT_ADDR_L::ADDR_L) as u64;
        let high = self.regs.boot_addr_h.read(SEC_SYS_BOOT_ADDR_H::ADDR_H) as u64;
        (high << 32) | low
    }

    /// 启动协处理器 (小核 C906@700MHz)
    ///
    /// 此函数执行以下步骤：
    /// 1. 将协处理器置于复位状态
    /// 2. 使能协处理器
    /// 3. 设置启动地址
    /// 4. 执行内存屏障
    /// 5. 解除复位，启动协处理器
    ///
    /// # 参数
    /// - `entry`: 协处理器启动入口地址 (物理地址)
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 启动地址有效且包含有效的可执行代码
    /// - 协处理器的内存区域已正确配置
    pub unsafe fn start_secondary_core(&self, entry: u64) {
        unsafe {
            let rstc = Rstc::new(RSTC_BASE);

            // 1. 将协处理器 (CPUSYS2) 置于复位状态
            rstc.assert_cpu_sys_reset(2);

            // 2. 使能协处理器
            self.enable_secondary_cpu();

            // 3. 设置启动地址
            self.set_boot_address(entry);

            // 4. 执行内存屏障，确保所有写操作完成
            #[cfg(target_arch = "riscv64")]
            {
                core::arch::asm!("fence.i");
            }
            #[cfg(target_arch = "riscv32")]
            {
                core::arch::asm!("fence.i");
            }
            #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
            {
                core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            }

            // 5. 解除复位，启动协处理器
            rstc.release_cpu_sys_reset(2);

            // 6. 再次执行内存屏障
            #[cfg(target_arch = "riscv64")]
            {
                core::arch::asm!("fence.i");
            }
            #[cfg(target_arch = "riscv32")]
            {
                core::arch::asm!("fence.i");
            }
            #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
            {
                core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    /// 获取寄存器组引用
    pub fn regs(&self) -> &'static SecSysRegisters {
        self.regs
    }
}
