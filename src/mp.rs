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
//! | 地址       | 名称                    | 描述                           |
//! |------------|-------------------------|--------------------------------|
//! | 0x03003024 | SOFT_CPU_RSTN           | CPU 软复位控制寄存器           |
//! | 0x020B0004 | SEC_SYS_CTRL            | 安全子系统控制寄存器           |
//! | 0x020B0020 | SEC_SYS_BOOT_ADDR_L     | 协处理器启动地址低 32 位       |
//! | 0x020B0024 | SEC_SYS_BOOT_ADDR_H     | 协处理器启动地址高 32 位       |
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::mp::{SecSys, start_secondary_core};
//!
//! // 方式 1: 使用便捷函数
//! unsafe {
//!     start_secondary_core(entry_address);
//! }
//!
//! // 方式 2: 使用 SecSys 驱动
//! let sec_sys = unsafe { SecSys::new() };
//! unsafe {
//!     sec_sys.start_secondary_core(entry_address);
//! }
//! ```

#![allow(dead_code)]

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

use crate::rstc::{Rstc, RSTC_BASE};

// ============================================================================
// 寄存器基地址
// ============================================================================

/// 安全子系统寄存器基地址
pub const SEC_SYS_BASE: usize = 0x020B0000;

/// SOFT_CPU_RSTN 寄存器地址 (复位控制器)
pub const SOFT_CPU_RSTN_ADDR: usize = RSTC_BASE + 0x024;

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
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new() -> Self {
        unsafe {
            Self {
                regs: &*(SEC_SYS_BASE as *const SecSysRegisters),
            }
        }
    }

    /// 从指定基地址创建安全子系统驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize) -> Self {
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
            let rstc = Rstc::from_base_address(RSTC_BASE);

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

// ============================================================================
// 便捷函数
// ============================================================================

/// 启动协处理器 (小核 C906@700MHz)
///
/// 这是一个便捷函数，封装了 `SecSys::start_secondary_core`
///
/// # 参数
/// - `entry`: 协处理器启动入口地址 (物理地址)
///
/// # Safety
///
/// 调用者必须确保:
/// - 启动地址有效且包含有效的可执行代码
/// - 协处理器的内存区域已正确配置
///
/// # 示例
///
/// ```rust,ignore
/// use sg200x_bsp::mp::start_secondary_core;
///
/// // 假设 _secondary_entry 是协处理器的入口函数
/// extern "C" {
///     fn _secondary_entry();
/// }
///
/// unsafe {
///     let entry_addr = _secondary_entry as *const () as u64;
///     start_secondary_core(entry_addr);
/// }
/// ```
pub unsafe fn start_secondary_core(entry: u64) {
    unsafe {
        let sec_sys = SecSys::new();
        sec_sys.start_secondary_core(entry);
    }
}

/// 使用原始寄存器地址启动协处理器
///
/// 此函数直接操作寄存器，不依赖 `Rstc` 和 `SecSys` 结构体。
/// 适用于需要更底层控制的场景。
///
/// # 参数
/// - `entry`: 协处理器启动入口地址 (物理地址)
///
/// # Safety
///
/// 调用者必须确保:
/// - 启动地址有效且包含有效的可执行代码
/// - 协处理器的内存区域已正确配置
/// - 寄存器地址可访问
pub unsafe fn start_secondary_core_raw(entry: u64) {
    unsafe {
        // 复位控制寄存器地址
        let rst_ptr = SOFT_CPU_RSTN_ADDR as *mut u32;
        // 安全子系统控制寄存器地址
        let sec_ctrl_ptr = (SEC_SYS_BASE + 0x004) as *mut u32;
        // 启动地址低 32 位寄存器
        let boot_addr_l_ptr = (SEC_SYS_BASE + 0x020) as *mut u32;
        // 启动地址高 32 位寄存器
        let boot_addr_h_ptr = (SEC_SYS_BASE + 0x024) as *mut u32;

        // CPUSYS2 (小核) 的位掩码 (bit 6)
        const CPUSYS2_MASK: u32 = 1 << 6;
        // SEC_CPU_EN 的位掩码 (bit 13)
        const SEC_CPU_EN_MASK: u32 = 1 << 13;

        // 1. 将协处理器置于复位状态 (写 0 到 bit 6)
        let rst_val = rst_ptr.read_volatile();
        rst_ptr.write_volatile(rst_val & !CPUSYS2_MASK);

        // 2. 使能协处理器 (写 1 到 bit 13)
        let ctrl_val = sec_ctrl_ptr.read_volatile();
        sec_ctrl_ptr.write_volatile(ctrl_val | SEC_CPU_EN_MASK);

        // 3. 设置启动地址
        boot_addr_l_ptr.write_volatile(entry as u32);
        boot_addr_h_ptr.write_volatile((entry >> 32) as u32);

        // 4. 执行内存屏障
        #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
        {
            core::arch::asm!("fence.i");
        }
        #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
        {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }

        // 5. 解除复位，启动协处理器 (写 1 到 bit 6)
        let rst_val = rst_ptr.read_volatile();
        rst_ptr.write_volatile(rst_val | CPUSYS2_MASK);

        // 6. 再次执行内存屏障
        #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
        {
            core::arch::asm!("fence.i");
        }
        #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
        {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }
    }
}
