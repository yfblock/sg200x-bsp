// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TPU (Tensor Processing Unit) 驱动模块
//
// 本模块提供 SG2002/CV181x 系列芯片 TPU 的 Rust 驱动实现
// 基于原始 C 驱动代码转换
//
// # 模块结构
//
// - `types`: 数据类型定义
// - `reg_tdma`: TDMA 寄存器定义
// - `reg_tiu`: TIU 寄存器定义
// - `pmu`: PMU (性能监控单元) 控制
// - `platform`: 平台驱动实现
//
// # 使用示例
//
// ```rust,ignore
// use sg200x_bsp::tpu::{TpuPlatform, TdmaPioInfo};
//
// // 创建 TPU 平台驱动
// let mut tpu = TpuPlatform::new(tdma_base_addr, tiu_base_addr);
//
// // 初始化
// tpu.init();
//
// // 运行 PIO 传输
// let info = TdmaPioInfo {
//     paddr_src: src_addr,
//     paddr_dst: dst_addr,
//     leng_bytes: 1024,
//     ..Default::default()
// };
//
// unsafe {
//     tpu.run_pio(&info, || Ok(()))?;
// }
//
// // 反初始化
// tpu.deinit();
// ```

#![allow(dead_code)]

pub mod platform;
pub mod pmu;
pub mod reg_tdma;
pub mod reg_tiu;
pub mod types;

// 重新导出常用类型
pub use platform::TpuPlatform;
pub use pmu::{TpuPmu, TpuPmuEvent, TpuPmuType};
pub use types::{
    CmdIdNode, CpuSyncDesc, DmaHeader, TdmaPioInfo, TdmaReg, TpuPlatformConfig, TpuRegBackupInfo,
    TpuSecSmcCall, TpuTeeLoadInfo, TpuTeeSubmitInfo,
};

// 重新导出寄存器常量
pub use reg_tdma::{
    GDMA_TYPE_F16, GDMA_TYPE_F32, GDMA_TYPE_I1, GDMA_TYPE_I16, GDMA_TYPE_I2, GDMA_TYPE_I32,
    GDMA_TYPE_I4, GDMA_TYPE_I8, TDMA_ALL_IDLE, TDMA_CTRL, TDMA_DES_BASE, TDMA_INT_EOD,
    TDMA_INT_EOPMU, TDMA_INT_MASK, TDMA_MASK_INIT, TDMA_STATUS, TDMA_SYNC_STATUS,
};

pub use reg_tiu::{
    BDC_ENGINE_CMD_ALIGNED_BIT, BD_CTRL_BASE_ADDR, BD_DES_ADDR_VLD, BD_INTR_ENABLE, BD_LANE_NUM,
    BD_TPU_EN, TiuLaneNum,
};
