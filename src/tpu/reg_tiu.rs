// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TIU (Tensor Instruction Unit) 寄存器定义
// 基于原始 C 驱动代码 reg_tiu.h 转换

/// BDC 引擎命令对齐位
pub const BDC_ENGINE_CMD_ALIGNED_BIT: u32 = 8;

// 基于虚拟地址的基地址定义
/// TIU 引擎基地址
pub const TIU_ENGINE_BASE_ADDR: usize = 0;
/// BD 命令基地址
pub const BD_CMD_BASE_ADDR: usize = TIU_ENGINE_BASE_ADDR + 0;
/// BD 控制基地址
pub const BD_CTRL_BASE_ADDR: usize = TIU_ENGINE_BASE_ADDR + 0x100;

// BD 控制位定义 (基于 BD_CTRL_BASE_ADDR)
/// TPU 使能位
pub const BD_TPU_EN: u32 = 0;
/// Lane 数量位 [29:22]
pub const BD_LANE_NUM: u32 = 22;
/// 描述符地址有效位 (启用描述符模式)
pub const BD_DES_ADDR_VLD: u32 = 30;
/// TIU 中断全局使能位
pub const BD_INTR_ENABLE: u32 = 31;

/// TIU Lane 数量枚举
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiuLaneNum {
    /// 2 lanes
    Lane2 = 0x1,
    /// 4 lanes
    Lane4 = 0x2,
    /// 8 lanes
    Lane8 = 0x3,
    /// 16 lanes
    Lane16 = 0x4,
    /// 32 lanes
    Lane32 = 0x5,
    /// 64 lanes
    Lane64 = 0x6,
}

impl TiuLaneNum {
    /// 从 u32 值转换
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x1 => Some(Self::Lane2),
            0x2 => Some(Self::Lane4),
            0x3 => Some(Self::Lane8),
            0x4 => Some(Self::Lane16),
            0x5 => Some(Self::Lane32),
            0x6 => Some(Self::Lane64),
            _ => None,
        }
    }

    /// 获取实际的 lane 数量
    pub fn count(&self) -> u32 {
        match self {
            Self::Lane2 => 2,
            Self::Lane4 => 4,
            Self::Lane8 => 8,
            Self::Lane16 => 16,
            Self::Lane32 => 32,
            Self::Lane64 => 64,
        }
    }
}
