//! # SG2002 复位控制器驱动模块
//!
//! 本模块提供 SG2002 芯片复位控制器的 Rust 驱动实现。
//!
//! ## 功能特性
//!
//! - 支持软复位控制 (SOFT_RSTN_0 ~ SOFT_RSTN_3)
//! - 支持 CPU 自动清除软复位 (SOFT_CPUAC_RSTN)
//! - 支持 CPU 软复位 (SOFT_CPU_RSTN)
//!
//! ## 硬件资源
//!
//! 复位控制器基地址: 0x03003000
//!
//! | 寄存器名称       | 偏移地址 | 描述                           |
//! |------------------|----------|--------------------------------|
//! | SOFT_RSTN_0      | 0x000    | 软复位控制寄存器 0             |
//! | SOFT_RSTN_1      | 0x004    | 软复位控制寄存器 1             |
//! | SOFT_RSTN_2      | 0x008    | 软复位控制寄存器 2             |
//! | SOFT_RSTN_3      | 0x00c    | 软复位控制寄存器 3             |
//! | SOFT_CPUAC_RSTN  | 0x020    | CPU 自动清除软复位控制寄存器   |
//! | SOFT_CPU_RSTN    | 0x024    | CPU 软复位控制寄存器           |
//!
//! ## 注意事项
//!
//! - 复位配置为低电平有效
//! - SOFT_RSTN_0 ~ 3 的复位信号不会自动清除，需要软件配置为 1 解除复位
//! - SOFT_CPUAC_RSTN 配置写 0 后，复位控制器会等待 24us 延时后才触发相应处理器复位，
//!   触发复位后对应的复位信号会持续 8us 后自动解除

#![allow(dead_code)]

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::ReadWrite,
};

// ============================================================================
// 寄存器基地址
// ============================================================================

/// 复位控制器基地址
pub const RSTC_BASE: usize = 0x03003000;

// ============================================================================
// 寄存器位域定义 (使用 tock-registers)
// ============================================================================

register_bitfields! [
    u32,

    /// 软复位控制寄存器 0 (偏移 0x000)
    /// 复位配置为低电平有效
    pub SOFT_RSTN_0 [
        /// DDR 系统软复位 (低电平有效)
        REG_SOFT_RESET_X_DDR OFFSET(2) NUMBITS(1) [],
        /// H264 IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_H264C OFFSET(3) NUMBITS(1) [],
        /// JPEG IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_JPEG OFFSET(4) NUMBITS(1) [],
        /// H265 IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_H265C OFFSET(5) NUMBITS(1) [],
        /// VIP 系统软复位 (低电平有效)
        REG_SOFT_RESET_X_VIPSYS OFFSET(6) NUMBITS(1) [],
        /// TPU_DMA IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_TDMA OFFSET(7) NUMBITS(1) [],
        /// TPU IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_TPU OFFSET(8) NUMBITS(1) [],
        /// TPU 系统软复位 (低电平有效)
        REG_SOFT_RESET_X_TPUSYS OFFSET(9) NUMBITS(1) [],
        /// USB IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_USB OFFSET(11) NUMBITS(1) [],
        /// ETH0 IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_ETH0 OFFSET(12) NUMBITS(1) [],
        /// ETH1 IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_ETH1 OFFSET(13) NUMBITS(1) [],
        /// NAND IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_NAND OFFSET(14) NUMBITS(1) [],
        /// EMMC IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_EMMC OFFSET(15) NUMBITS(1) [],
        /// SD0 IP 软复位 (低电平有效)
        REG_SOFT_RESET_X_SD0 OFFSET(16) NUMBITS(1) []
    ],

    /// 软复位控制寄存器 1 (偏移 0x004)
    pub SOFT_RSTN_1 [
        /// 全部位域 (具体位域参考手册)
        ALL OFFSET(0) NUMBITS(32) []
    ],

    /// 软复位控制寄存器 2 (偏移 0x008)
    pub SOFT_RSTN_2 [
        /// 全部位域 (具体位域参考手册)
        ALL OFFSET(0) NUMBITS(32) []
    ],

    /// 软复位控制寄存器 3 (偏移 0x00c)
    pub SOFT_RSTN_3 [
        /// 全部位域 (具体位域参考手册)
        ALL OFFSET(0) NUMBITS(32) []
    ],

    /// CPU 自动清除软复位控制寄存器 (偏移 0x020)
    /// 配置写 0 后，复位控制器会等待 24us 延时后才触发相应处理器复位
    /// 触发复位后对应的复位信号会持续 8us 后自动解除
    pub SOFT_CPUAC_RSTN [
        /// CPUCORE0 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUCORE0 OFFSET(0) NUMBITS(1) [],
        /// CPUCORE1 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUCORE1 OFFSET(1) NUMBITS(1) [],
        /// CPUCORE2 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUCORE2 OFFSET(2) NUMBITS(1) [],
        /// CPUCORE3 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUCORE3 OFFSET(3) NUMBITS(1) [],
        /// CPUSYS0 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUSYS0 OFFSET(4) NUMBITS(1) [],
        /// CPUSYS1 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUSYS1 OFFSET(5) NUMBITS(1) [],
        /// CPUSYS2 自动清除复位 (低电平有效)
        REG_AUTO_CLEAR_RESET_X_CPUSYS2 OFFSET(6) NUMBITS(1) []
    ],

    /// CPU 软复位控制寄存器 (偏移 0x024)
    /// 复位配置为低电平有效，复位信号不会自动清除
    pub SOFT_CPU_RSTN [
        /// CPUCORE0 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUCORE0 OFFSET(0) NUMBITS(1) [],
        /// CPUCORE1 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUCORE1 OFFSET(1) NUMBITS(1) [],
        /// CPUCORE2 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUCORE2 OFFSET(2) NUMBITS(1) [],
        /// CPUCORE3 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUCORE3 OFFSET(3) NUMBITS(1) [],
        /// CPUSYS0 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUSYS0 OFFSET(4) NUMBITS(1) [],
        /// CPUSYS1 软复位 (低电平有效)
        REG_SOFT_RESET_X_CPUSYS1 OFFSET(5) NUMBITS(1) [],
        /// CPUSYS2 软复位 (低电平有效) - 小核 C906@700MHz
        REG_SOFT_RESET_X_CPUSYS2 OFFSET(6) NUMBITS(1) []
    ]
];

// ============================================================================
// 寄存器结构体定义
// ============================================================================

register_structs! {
    /// 复位控制器寄存器组
    pub RstcRegisters {
        /// 软复位控制寄存器 0 (偏移 0x000)
        (0x000 => pub soft_rstn_0: ReadWrite<u32, SOFT_RSTN_0::Register>),

        /// 软复位控制寄存器 1 (偏移 0x004)
        (0x004 => pub soft_rstn_1: ReadWrite<u32, SOFT_RSTN_1::Register>),

        /// 软复位控制寄存器 2 (偏移 0x008)
        (0x008 => pub soft_rstn_2: ReadWrite<u32, SOFT_RSTN_2::Register>),

        /// 软复位控制寄存器 3 (偏移 0x00c)
        (0x00c => pub soft_rstn_3: ReadWrite<u32, SOFT_RSTN_3::Register>),

        /// 保留 (偏移 0x010-0x01c)
        (0x010 => _reserved0),

        /// CPU 自动清除软复位控制寄存器 (偏移 0x020)
        (0x020 => pub soft_cpuac_rstn: ReadWrite<u32, SOFT_CPUAC_RSTN::Register>),

        /// CPU 软复位控制寄存器 (偏移 0x024)
        (0x024 => pub soft_cpu_rstn: ReadWrite<u32, SOFT_CPU_RSTN::Register>),

        /// 结束标记
        (0x028 => @END),
    }
}

// ============================================================================
// 复位控制器驱动
// ============================================================================

/// 复位控制器驱动结构体
pub struct Rstc {
    /// 寄存器组引用
    regs: &'static RstcRegisters,
}

impl Rstc {
    /// 创建新的复位控制器驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个实例导致数据竞争
    pub unsafe fn new() -> Self {
        unsafe {
            Self {
                regs: &*(RSTC_BASE as *const RstcRegisters),
            }
        }
    }

    /// 从指定基地址创建复位控制器驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize) -> Self {
        unsafe {
            Self {
                regs: &*(base as *const RstcRegisters),
            }
        }
    }

    // ========================================================================
    // CPU 核心复位控制
    // ========================================================================

    /// 触发 CPU 核心软复位 (使用 SOFT_CPU_RSTN)
    ///
    /// # 参数
    /// - `core`: CPU 核心索引 (0-3)
    ///
    /// 注意：复位信号不会自动清除，需要调用 `release_cpu_core_reset` 解除复位
    pub fn assert_cpu_core_reset(&self, core: u8) {
        let mask = 1u32 << core;
        let val = self.regs.soft_cpu_rstn.get();
        // 写 0 触发复位 (低电平有效)
        self.regs.soft_cpu_rstn.set(val & !mask);
    }

    /// 解除 CPU 核心软复位 (使用 SOFT_CPU_RSTN)
    ///
    /// # 参数
    /// - `core`: CPU 核心索引 (0-3)
    pub fn release_cpu_core_reset(&self, core: u8) {
        let mask = 1u32 << core;
        let val = self.regs.soft_cpu_rstn.get();
        // 写 1 解除复位
        self.regs.soft_cpu_rstn.set(val | mask);
    }

    /// 触发 CPU 子系统软复位 (使用 SOFT_CPU_RSTN)
    ///
    /// # 参数
    /// - `sys`: CPU 子系统索引 (0-2)，其中 2 为小核 C906@700MHz
    ///
    /// 注意：复位信号不会自动清除，需要调用 `release_cpu_sys_reset` 解除复位
    pub fn assert_cpu_sys_reset(&self, sys: u8) {
        let mask = 1u32 << (4 + sys);
        let val = self.regs.soft_cpu_rstn.get();
        // 写 0 触发复位 (低电平有效)
        self.regs.soft_cpu_rstn.set(val & !mask);
    }

    /// 解除 CPU 子系统软复位 (使用 SOFT_CPU_RSTN)
    ///
    /// # 参数
    /// - `sys`: CPU 子系统索引 (0-2)，其中 2 为小核 C906@700MHz
    pub fn release_cpu_sys_reset(&self, sys: u8) {
        let mask = 1u32 << (4 + sys);
        let val = self.regs.soft_cpu_rstn.get();
        // 写 1 解除复位
        self.regs.soft_cpu_rstn.set(val | mask);
    }

    // ========================================================================
    // CPU 自动清除复位控制
    // ========================================================================

    /// 触发 CPU 核心自动清除复位 (使用 SOFT_CPUAC_RSTN)
    ///
    /// # 参数
    /// - `core`: CPU 核心索引 (0-3)
    ///
    /// 配置写 0 后，复位控制器会等待 24us 延时后才触发相应处理器复位
    /// 触发复位后对应的复位信号会持续 8us 后自动解除
    pub fn trigger_cpu_core_auto_reset(&self, core: u8) {
        let mask = 1u32 << core;
        let val = self.regs.soft_cpuac_rstn.get();
        // 写 0 触发自动清除复位
        self.regs.soft_cpuac_rstn.set(val & !mask);
    }

    /// 触发 CPU 子系统自动清除复位 (使用 SOFT_CPUAC_RSTN)
    ///
    /// # 参数
    /// - `sys`: CPU 子系统索引 (0-2)，其中 2 为小核 C906@700MHz
    pub fn trigger_cpu_sys_auto_reset(&self, sys: u8) {
        let mask = 1u32 << (4 + sys);
        let val = self.regs.soft_cpuac_rstn.get();
        // 写 0 触发自动清除复位
        self.regs.soft_cpuac_rstn.set(val & !mask);
    }

    // ========================================================================
    // 原始寄存器访问
    // ========================================================================

    /// 读取 SOFT_CPU_RSTN 寄存器原始值
    pub fn read_soft_cpu_rstn(&self) -> u32 {
        use tock_registers::interfaces::Readable;
        self.regs.soft_cpu_rstn.get()
    }

    /// 写入 SOFT_CPU_RSTN 寄存器原始值
    pub fn write_soft_cpu_rstn(&self, value: u32) {
        use tock_registers::interfaces::Writeable;
        self.regs.soft_cpu_rstn.set(value);
    }

    /// 读取 SOFT_CPUAC_RSTN 寄存器原始值
    pub fn read_soft_cpuac_rstn(&self) -> u32 {
        use tock_registers::interfaces::Readable;
        self.regs.soft_cpuac_rstn.get()
    }

    /// 写入 SOFT_CPUAC_RSTN 寄存器原始值
    pub fn write_soft_cpuac_rstn(&self, value: u32) {
        use tock_registers::interfaces::Writeable;
        self.regs.soft_cpuac_rstn.set(value);
    }

    /// 获取寄存器组引用
    pub fn regs(&self) -> &'static RstcRegisters {
        self.regs
    }
}
