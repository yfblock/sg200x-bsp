// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TPU PMU (Performance Monitoring Unit) 控制模块
// 基于原始 C 驱动代码 tpu_pmu.c/h 转换

use super::reg_tdma::TDMA_ENGINE_BASE_ADDR;
use super::types::TpuPlatformConfig;

/// TPU PMU 控制寄存器
pub const TPUPMU_CTRL: usize = TDMA_ENGINE_BASE_ADDR + 0x200;
/// TPU PMU 缓冲区基地址寄存器
pub const TPUPMU_BUFBASE: usize = TDMA_ENGINE_BASE_ADDR + 0x20C;
/// TPU PMU 缓冲区大小寄存器
pub const TPUPMU_BUFSIZE: usize = TDMA_ENGINE_BASE_ADDR + 0x210;

/// TPU PMU 缓冲区保护值
pub const TPUPMU_BUFGUARD: u32 = 0x12345678;

/// TPU PMU 事件类型
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuPmuEvent {
    /// Bank 冲突事件
    BankConflict = 0x0,
    /// 停顿计数事件
    StallCount = 0x1,
    /// TDMA 带宽事件
    TdmaBandwidth = 0x2,
    /// TDMA 写选通事件
    TdmaWstrb = 0x3,
}

/// TPU PMU 类型
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuPmuType {
    /// TDMA 加载
    TdmaLoad = 1,
    /// TDMA 存储
    TdmaStore = 2,
    /// TDMA 移动
    TdmaMove = 3,
    /// TIU 操作
    Tiu = 4,
}

/// TPU PMU 双事件结构
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TpuPmuDoubleEvent {
    /// 类型 (4 bits)
    pub event_type: u8,
    /// 描述符 ID (16 bits)
    pub des_id: u16,
    /// 事件计数 0 (22 bits)
    pub event_cnt0: u32,
    /// 事件计数 1 (22 bits)
    pub event_cnt1: u32,
    /// 结束时间
    pub end_time: u32,
    /// 开始时间
    pub start_time: u32,
}

/// TPU PMU 配置
#[derive(Debug, Clone, Copy, Default)]
pub struct TpuPmuConfig {
    /// 使能标志
    pub enable: bool,
    /// TPU 使能
    pub enable_tpu: bool,
    /// TDMA 使能
    pub enable_tdma: bool,
    /// 事件类型
    pub event: TpuPmuEvent,
    /// TPU 同步 ID 起始
    pub tpu_sync_id_start: u16,
    /// TPU 同步 ID 结束
    pub tpu_sync_id_end: u16,
    /// TDMA 同步 ID 起始
    pub tdma_sync_id_start: u16,
    /// TDMA 同步 ID 结束
    pub tdma_sync_id_end: u16,
    /// 缓冲区基地址 (寄存器设置需右移 4 位)
    pub buf_base_addr: u32,
    /// 缓冲区大小 (寄存器设置需右移 4 位)
    pub buf_size: u32,
}

impl Default for TpuPmuEvent {
    fn default() -> Self {
        Self::BankConflict
    }
}

/// TPU PMU 控制器
pub struct TpuPmu;

impl TpuPmu {
    /// 配置 PMU
    ///
    /// # Safety
    /// 调用者必须确保 `cfg.iomem_tdma_base` 指向有效的 MMIO 区域
    pub unsafe fn config(cfg: &TpuPlatformConfig, pmu_config: &TpuPmuConfig) {
        unsafe {
            let tdma_base = cfg.iomem_tdma_base as *mut u32;

            if pmu_config.enable {
                // 设置缓冲区起始地址和大小
                core::ptr::write_volatile(
                    tdma_base.byte_add(TPUPMU_BUFBASE),
                    pmu_config.buf_base_addr,
                );
                core::ptr::write_volatile(tdma_base.byte_add(TPUPMU_BUFSIZE), pmu_config.buf_size);

                let mut reg_value: u32 = 0;

                // 设置使能相关位
                reg_value |= 0x1;
                if pmu_config.enable_tpu {
                    reg_value |= 0x8;
                }
                if pmu_config.enable_tdma {
                    reg_value |= 0x10;
                }

                // 设置事件类型
                reg_value |= (pmu_config.event as u32) << 5;

                // 设置突发长度 = 16
                reg_value |= 0x3 << 8;

                // 启用 PMU 环形缓冲区模式
                reg_value |= 0x1 << 10;

                // 启用 PMU DCM
                reg_value &= !0xFFFF0000;

                // 设置控制寄存器
                core::ptr::write_volatile(tdma_base.byte_add(TPUPMU_CTRL), reg_value);
            } else {
                // 禁用寄存器
                let reg_value = core::ptr::read_volatile(tdma_base.byte_add(TPUPMU_CTRL));
                core::ptr::write_volatile(tdma_base.byte_add(TPUPMU_CTRL), reg_value & !0x1);
            }
        }
    }

    /// 启用/禁用 PMU
    ///
    /// # Safety
    /// 调用者必须确保 `cfg.iomem_tdma_base` 指向有效的 MMIO 区域
    pub unsafe fn enable(cfg: &TpuPlatformConfig, enable: bool, event: TpuPmuEvent) {
        unsafe {
            let config = if enable {
                let buf_addr = cfg.pmubuf_addr_p >> 4;
                let buf_size = (cfg.pmubuf_size as u64) >> 4;

                TpuPmuConfig {
                    enable: true,
                    event,
                    enable_tdma: true,
                    enable_tpu: true,
                    buf_base_addr: buf_addr as u32,
                    buf_size: buf_size as u32,
                    ..Default::default()
                }
            } else {
                TpuPmuConfig {
                    enable: false,
                    ..Default::default()
                }
            };

            Self::config(cfg, &config);
        }
    }
}
