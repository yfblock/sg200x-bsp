// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TDMA 寄存器定义
// 基于原始 C 驱动代码 reg_tdma.h 转换

/// TDMA 描述符寄存器字节数
pub const TDMA_DESC_REG_BYTES: usize = 0x40;

/// TDMA 引擎描述符数量
pub const TDMA_ENGINE_DESCRIPTOR_NUM: usize = TDMA_DESC_REG_BYTES >> 2;

/// TDMA 基址寄存器数量
pub const TDMA_NUM_BASE_REGS: usize = 0x8;

// GDMA 数据类型定义 (向后兼容)
/// f32 类型
pub const GDMA_TYPE_F32: u32 = 0;
/// f16 类型
pub const GDMA_TYPE_F16: u32 = 1;
/// i32 类型
pub const GDMA_TYPE_I32: u32 = 2;
/// i16 类型
pub const GDMA_TYPE_I16: u32 = 3;
/// i8 类型
pub const GDMA_TYPE_I8: u32 = 4;
/// i4 类型
pub const GDMA_TYPE_I4: u32 = 5;
/// i2 类型
pub const GDMA_TYPE_I2: u32 = 6;
/// i1 类型
pub const GDMA_TYPE_I1: u32 = 7;
/// 最后一个 i1 类型
pub const LAST_GDMA_TYPE_I1: u32 = 8;

// TDMA 控制寄存器定义 (基于虚拟地址)
/// TDMA 引擎基地址
pub const TDMA_ENGINE_BASE_ADDR: usize = 0;

/// TDMA 控制寄存器
pub const TDMA_CTRL: usize = TDMA_ENGINE_BASE_ADDR + 0x0;
/// TDMA 描述符基地址寄存器
pub const TDMA_DES_BASE: usize = TDMA_ENGINE_BASE_ADDR + 0x4;
/// TDMA 中断掩码寄存器
pub const TDMA_INT_MASK: usize = TDMA_ENGINE_BASE_ADDR + 0x8;
/// TDMA 同步状态寄存器
pub const TDMA_SYNC_STATUS: usize = TDMA_ENGINE_BASE_ADDR + 0xC;

/// TDMA 命令累加器 0
pub const TDMA_CMD_ACCP0: usize = TDMA_ENGINE_BASE_ADDR + 0x10;
/// TDMA 命令累加器 1
pub const TDMA_CMD_ACCP1: usize = TDMA_ENGINE_BASE_ADDR + 0x14;
/// TDMA 命令累加器 2
pub const TDMA_CMD_ACCP2: usize = TDMA_ENGINE_BASE_ADDR + 0x18;
/// TDMA 命令累加器 3
pub const TDMA_CMD_ACCP3: usize = TDMA_ENGINE_BASE_ADDR + 0x1C;
/// TDMA 命令累加器 4
pub const TDMA_CMD_ACCP4: usize = TDMA_ENGINE_BASE_ADDR + 0x20;
/// TDMA 命令累加器 5
pub const TDMA_CMD_ACCP5: usize = TDMA_ENGINE_BASE_ADDR + 0x24;
/// TDMA 命令累加器 6
pub const TDMA_CMD_ACCP6: usize = TDMA_ENGINE_BASE_ADDR + 0x28;
/// TDMA 命令累加器 7
pub const TDMA_CMD_ACCP7: usize = TDMA_ENGINE_BASE_ADDR + 0x2C;
/// TDMA 命令累加器 8
pub const TDMA_CMD_ACCP8: usize = TDMA_ENGINE_BASE_ADDR + 0x30;
/// TDMA 命令累加器 9
pub const TDMA_CMD_ACCP9: usize = TDMA_ENGINE_BASE_ADDR + 0x34;
/// TDMA 命令累加器 10
pub const TDMA_CMD_ACCP10: usize = TDMA_ENGINE_BASE_ADDR + 0x38;
/// TDMA 命令累加器 11
pub const TDMA_CMD_ACCP11: usize = TDMA_ENGINE_BASE_ADDR + 0x3C;
/// TDMA 命令累加器 12
pub const TDMA_CMD_ACCP12: usize = TDMA_ENGINE_BASE_ADDR + 0x40;
/// TDMA 命令累加器 13
pub const TDMA_CMD_ACCP13: usize = TDMA_ENGINE_BASE_ADDR + 0x44;
/// TDMA 命令累加器 14
pub const TDMA_CMD_ACCP14: usize = TDMA_ENGINE_BASE_ADDR + 0x48;
/// TDMA 命令累加器 15
pub const TDMA_CMD_ACCP15: usize = TDMA_ENGINE_BASE_ADDR + 0x4C;

/// TDMA 数组基地址 0 低位
pub const TDMA_ARRAYBASE0_L: usize = TDMA_ENGINE_BASE_ADDR + 0x70;
/// TDMA 数组基地址 1 低位
pub const TDMA_ARRAYBASE1_L: usize = TDMA_ENGINE_BASE_ADDR + 0x74;
/// TDMA 数组基地址 2 低位
pub const TDMA_ARRAYBASE2_L: usize = TDMA_ENGINE_BASE_ADDR + 0x78;
/// TDMA 数组基地址 3 低位
pub const TDMA_ARRAYBASE3_L: usize = TDMA_ENGINE_BASE_ADDR + 0x7C;
/// TDMA 数组基地址 4 低位
pub const TDMA_ARRAYBASE4_L: usize = TDMA_ENGINE_BASE_ADDR + 0x80;
/// TDMA 数组基地址 5 低位
pub const TDMA_ARRAYBASE5_L: usize = TDMA_ENGINE_BASE_ADDR + 0x84;
/// TDMA 数组基地址 6 低位
pub const TDMA_ARRAYBASE6_L: usize = TDMA_ENGINE_BASE_ADDR + 0x88;
/// TDMA 数组基地址 7 低位
pub const TDMA_ARRAYBASE7_L: usize = TDMA_ENGINE_BASE_ADDR + 0x8C;
/// TDMA 数组基地址 0 高位
pub const TDMA_ARRAYBASE0_H: usize = TDMA_ENGINE_BASE_ADDR + 0x90;
/// TDMA 数组基地址 1 高位
pub const TDMA_ARRAYBASE1_H: usize = TDMA_ENGINE_BASE_ADDR + 0x94;

/// TDMA 调试模式寄存器
pub const TDMA_DEBUG_MODE: usize = TDMA_ENGINE_BASE_ADDR + 0xA0;
/// TDMA DCM 禁用寄存器
pub const TDMA_DCM_DISABLE: usize = TDMA_ENGINE_BASE_ADDR + 0xA4;
/// TDMA 状态寄存器
pub const TDMA_STATUS: usize = TDMA_ENGINE_BASE_ADDR + 0xEC;

// TDMA 控制位定义
/// TDMA 使能位
pub const TDMA_CTRL_ENABLE_BIT: u32 = 0;
/// TDMA 模式选择位
pub const TDMA_CTRL_MODESEL_BIT: u32 = 1;
/// TDMA 重置同步 ID 位
pub const TDMA_CTRL_RESET_SYNCID_BIT: u32 = 2;
/// TDMA 强制 1 数组位
pub const TDMA_CTRL_FORCE_1ARRAY: u32 = 5;
/// TDMA 强制 2 数组位
pub const TDMA_CTRL_FORCE_2ARRAY: u32 = 6;
/// TDMA 突发长度位
pub const TDMA_CTRL_BURSTLEN_BIT: u32 = 8;
/// TDMA 64 字节对齐使能位
pub const TDMA_CTRL_64BYTE_ALIGN_EN: u32 = 10;
/// TDMA 内部命令关闭位
pub const TDMA_CTRL_INTRA_CMD_OFF: u32 = 13;
/// TDMA 描述符数量位
pub const TDMA_CTRL_DESNUM_BIT: u32 = 16;

// TDMA 中断相关常量
/// TDMA 掩码初始值 (忽略描述符 nchw/stride=0 错误)
pub const TDMA_MASK_INIT: u32 = 0x20;
/// TDMA 描述符结束中断
pub const TDMA_INT_EOD: u32 = 0x1;
/// TDMA PMU 结束中断
pub const TDMA_INT_EOPMU: u32 = 0x8000;
/// TDMA 全部空闲状态
pub const TDMA_ALL_IDLE: u32 = 0x1F;
