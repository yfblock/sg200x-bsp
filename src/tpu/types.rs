// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TPU 数据类型定义
// 基于原始 C 驱动代码转换

/// CPU 引擎描述符数量
pub const CPU_ENGINE_DESCRIPTOR_NUM: usize = 56;

/// DMA 缓冲区头部魔数 (主)
pub const TPU_DMABUF_HEADER_M: u16 = 0xB5B5;

/// CPU 同步描述符
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuSyncDesc {
    /// 操作类型 (CPU_CMD_ACCPI0)
    pub op_type: u32,
    /// BD 命令数量 (CPU_CMD_ACCPI1)
    pub num_bd: u32,
    /// GDMA 命令数量 (CPU_CMD_ACCPI2)
    pub num_gdma: u32,
    /// BD 偏移 (CPU_CMD_ACCPI3)
    pub offset_bd: u32,
    /// GDMA 偏移 (CPU_CMD_ACCPI4)
    pub offset_gdma: u32,
    /// 保留字段 (CPU_CMD_ACCPI5-CPU_CMD_ACCPI6)
    pub reserved: [u32; 2],
    /// 字符串数据
    pub str_data: [u8; (CPU_ENGINE_DESCRIPTOR_NUM - 7) * 4],
}

impl Default for CpuSyncDesc {
    fn default() -> Self {
        Self {
            op_type: 0,
            num_bd: 0,
            num_gdma: 0,
            offset_bd: 0,
            offset_gdma: 0,
            reserved: [0; 2],
            str_data: [0; (CPU_ENGINE_DESCRIPTOR_NUM - 7) * 4],
        }
    }
}

/// DMA 缓冲区头部结构
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DmaHeader {
    /// DMA 缓冲区魔数 (主)
    pub dmabuf_magic_m: u16,
    /// DMA 缓冲区魔数 (次)
    pub dmabuf_magic_s: u16,
    /// DMA 缓冲区大小
    pub dmabuf_size: u32,
    /// CPU 描述符数量
    pub cpu_desc_count: u32,
    /// BD 描述符数量 (16 bytes)
    pub bd_desc_count: u32,
    /// TDMA 描述符数量
    pub tdma_desc_count: u32,
    /// TPU 时钟频率
    pub tpu_clk_rate: u32,
    /// PMU 缓冲区大小
    pub pmubuf_size: u32,
    /// PMU 缓冲区偏移 (32 bytes)
    pub pmubuf_offset: u32,
    /// Array base 0 低 32 位
    pub arraybase_0_l: u32,
    /// Array base 0 高 32 位
    pub arraybase_0_h: u32,
    /// Array base 1 低 32 位
    pub arraybase_1_l: u32,
    /// Array base 1 高 32 位 (48 bytes)
    pub arraybase_1_h: u32,
    /// Array base 2 低 32 位
    pub arraybase_2_l: u32,
    /// Array base 2 高 32 位
    pub arraybase_2_h: u32,
    /// Array base 3 低 32 位
    pub arraybase_3_l: u32,
    /// Array base 3 高 32 位 (64 bytes)
    pub arraybase_3_h: u32,
    /// Array base 4 低 32 位
    pub arraybase_4_l: u32,
    /// Array base 4 高 32 位
    pub arraybase_4_h: u32,
    /// Array base 5 低 32 位
    pub arraybase_5_l: u32,
    /// Array base 5 高 32 位
    pub arraybase_5_h: u32,
    /// Array base 6 低 32 位
    pub arraybase_6_l: u32,
    /// Array base 6 高 32 位
    pub arraybase_6_h: u32,
    /// Array base 7 低 32 位
    pub arraybase_7_l: u32,
    /// Array base 7 高 32 位
    pub arraybase_7_h: u32,
    /// 保留字段 (128 bytes, 128 bytes 对齐)
    pub reserve: [u32; 8],
}

/// 命令 ID 节点
#[derive(Debug, Clone, Copy, Default)]
pub struct CmdIdNode {
    /// BD 命令 ID
    pub bd_cmd_id: u32,
    /// TDMA 命令 ID
    pub tdma_cmd_id: u32,
}

/// TPU TDMA PIO 信息
#[derive(Debug, Clone, Copy, Default)]
pub struct TdmaPioInfo {
    /// 源物理地址
    pub paddr_src: u64,
    /// 目标物理地址
    pub paddr_dst: u64,
    /// 高度
    pub h: u32,
    /// 宽度 (字节)
    pub w_bytes: u32,
    /// 源步长 (字节)
    pub stride_bytes_src: u32,
    /// 目标步长 (字节)
    pub stride_bytes_dst: u32,
    /// 启用 2D 传输
    pub enable_2d: u32,
    /// 长度 (字节)
    pub leng_bytes: u32,
}

/// TPU 平台配置
#[derive(Debug, Clone, Copy)]
pub struct TpuPlatformConfig {
    /// TDMA 基地址
    pub iomem_tdma_base: usize,
    /// TIU 基地址
    pub iomem_tiu_base: usize,
    /// PMU 缓冲区大小
    pub pmubuf_size: u32,
    /// PMU 缓冲区物理地址
    pub pmubuf_addr_p: u64,
}

impl Default for TpuPlatformConfig {
    fn default() -> Self {
        Self {
            iomem_tdma_base: 0,
            iomem_tiu_base: 0,
            pmubuf_size: 0,
            pmubuf_addr_p: 0,
        }
    }
}

/// TEE 加载信息
#[derive(Debug, Clone, Copy, Default)]
pub struct TpuTeeLoadInfo {
    /// REE 命令缓冲区地址
    pub cmdbuf_addr_ree: u64,
    /// REE 命令缓冲区长度
    pub cmdbuf_len_ree: u32,
    /// REE 权重地址
    pub weight_addr_ree: u64,
    /// REE 权重长度
    pub weight_len_ree: u32,
    /// REE 神经元地址
    pub neuron_addr_ree: u64,
    /// TEE DMA 缓冲区地址
    pub dmabuf_addr_tee: u64,
}

/// TEE 提交信息
#[derive(Debug, Clone, Copy, Default)]
pub struct TpuTeeSubmitInfo {
    /// DMA 缓冲区物理地址
    pub dmabuf_paddr: u64,
    /// 全局地址基址 2
    pub gaddr_base2: u64,
    /// 全局地址基址 3
    pub gaddr_base3: u64,
    /// 全局地址基址 4
    pub gaddr_base4: u64,
    /// 全局地址基址 5
    pub gaddr_base5: u64,
    /// 全局地址基址 6
    pub gaddr_base6: u64,
    /// 全局地址基址 7
    pub gaddr_base7: u64,
}

/// TPU 安全 SMC 调用类型
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuSecSmcCall {
    /// 加载命令
    LoadCmd = 0x1001,
    /// 运行
    Run = 0x1002,
    /// 等待
    Wait = 0x1003,
}

/// TPU 寄存器备份信息
#[derive(Debug, Clone, Copy, Default)]
pub struct TpuRegBackupInfo {
    pub tdma_int_mask: u32,
    pub tdma_sync_status: u32,
    pub tiu_ctrl_base_address: u32,
    pub tdma_arraybase0_l: u32,
    pub tdma_arraybase1_l: u32,
    pub tdma_arraybase2_l: u32,
    pub tdma_arraybase3_l: u32,
    pub tdma_arraybase4_l: u32,
    pub tdma_arraybase5_l: u32,
    pub tdma_arraybase6_l: u32,
    pub tdma_arraybase7_l: u32,
    pub tdma_arraybase0_h: u32,
    pub tdma_arraybase1_h: u32,
    pub tdma_des_base: u32,
    pub tdma_dbg_mode: u32,
    pub tdma_dcm_disable: u32,
    pub tdma_ctrl: u32,
}

/// TDMA 寄存器结构
#[derive(Debug, Clone, Copy, Default)]
pub struct TdmaReg {
    pub vld: u32,
    pub compress_en: u32,
    pub eod: u32,
    pub intp_en: u32,
    pub bar_en: u32,
    pub check_bf16_value: u32,
    pub trans_dir: u32,
    pub rsv00: u32,
    pub trans_fmt: u32,
    pub transpose_md: u32,
    pub rsv01: u32,
    pub intra_cmd_paral: u32,
    pub outstanding_en: u32,
    pub cmd_id: u32,
    pub spec_func: u32,
    pub dst_fmt: u32,
    pub src_fmt: u32,
    pub cmprs_fmt: u32,
    pub sys_dtype: u32,
    pub rsv2_1: u32,
    pub int8_sign: u32,
    pub compress_zero_guard: u32,
    pub int8_rnd_mode: u32,
    pub wait_id_tpu: u32,
    pub wait_id_other_tdma: u32,
    pub wait_id_sdma: u32,
    pub const_val: u32,
    pub src_base_reg_sel: u32,
    pub mv_lut_idx: u32,
    pub dst_base_reg_sel: u32,
    pub mv_lut_base: u32,
    pub rsv4_5: u32,
    pub dst_h_stride: u32,
    pub dst_c_stride_low: u32,
    pub dst_n_stride: u32,
    pub src_h_stride: u32,
    pub src_c_stride_low: u32,
    pub src_n_stride: u32,
    pub dst_c: u32,
    pub src_c: u32,
    pub dst_w: u32,
    pub dst_h: u32,
    pub src_w: u32,
    pub src_h: u32,
    pub dst_base_addr_low: u32,
    pub src_base_addr_low: u32,
    pub src_n: u32,
    pub dst_base_addr_high: u32,
    pub src_base_addr_high: u32,
    pub src_c_stride_high: u32,
    pub dst_c_stride_high: u32,
    pub compress_bias0: u32,
    pub compress_bias1: u32,
    pub layer_id: u32,
}

impl TdmaReg {
    /// 创建新的 TDMA 寄存器并重置为默认值
    pub fn new() -> Self {
        let mut reg = Self::default();
        reg.reset();
        reg
    }

    /// 重置 TDMA 寄存器为默认值
    pub fn reset(&mut self) {
        self.vld = 0x0;
        self.compress_en = 0x0;
        self.eod = 0x0;
        self.intp_en = 0x0;
        self.bar_en = 0x0;
        self.check_bf16_value = 0x0;
        self.trans_dir = 0x0;
        self.rsv00 = 0x0;
        self.trans_fmt = 0x0;
        self.transpose_md = 0x0;
        self.rsv01 = 0x0;
        self.intra_cmd_paral = 0x0;
        self.outstanding_en = 0x0;
        self.cmd_id = 0x0;
        self.spec_func = 0x0;
        self.dst_fmt = 0x1;
        self.src_fmt = 0x1;
        self.cmprs_fmt = 0x0;
        self.sys_dtype = 0x0;
        self.rsv2_1 = 0x0;
        self.int8_sign = 0x0;
        self.compress_zero_guard = 0x0;
        self.int8_rnd_mode = 0x0;
        self.wait_id_tpu = 0x0;
        self.wait_id_other_tdma = 0x0;
        self.wait_id_sdma = 0x0;
        self.const_val = 0x0;
        self.src_base_reg_sel = 0x0;
        self.mv_lut_idx = 0x0;
        self.dst_base_reg_sel = 0x0;
        self.mv_lut_base = 0x0;
        self.rsv4_5 = 0x0;
        self.dst_h_stride = 0x1;
        self.dst_c_stride_low = 0x1;
        self.dst_n_stride = 0x1;
        self.src_h_stride = 0x1;
        self.src_c_stride_low = 0x1;
        self.src_n_stride = 0x1;
        self.dst_c = 0x1;
        self.src_c = 0x1;
        self.dst_w = 0x1;
        self.dst_h = 0x1;
        self.src_w = 0x1;
        self.src_h = 0x1;
        self.dst_base_addr_low = 0x0;
        self.src_base_addr_low = 0x0;
        self.src_n = 0x1;
        self.dst_base_addr_high = 0x0;
        self.src_base_addr_high = 0x0;
        self.src_c_stride_high = 0x0;
        self.dst_c_stride_high = 0x0;
        self.compress_bias0 = 0x0;
        self.compress_bias1 = 0x0;
        self.layer_id = 0x0;
    }

    /// 将 TDMA 寄存器编码为 u32 数组
    pub fn emit(&self, p: &mut [u32; 16]) {
        p[15] = (self.compress_bias0 & ((1u32 << 8) - 1))
            | ((self.compress_bias1 & ((1u32 << 8) - 1)) << 8)
            | ((self.layer_id & ((1u32 << 16) - 1)) << 16);

        p[14] = (self.src_c_stride_high & ((1u32 << 16) - 1))
            | ((self.dst_c_stride_high & ((1u32 << 16) - 1)) << 16);

        p[13] = (self.src_n & ((1u32 << 16) - 1))
            | ((self.dst_base_addr_high & ((1u32 << 8) - 1)) << 16)
            | ((self.src_base_addr_high & ((1u32 << 8) - 1)) << 24);

        p[12] = self.src_base_addr_low;
        p[11] = self.dst_base_addr_low;

        p[10] = (self.src_w & ((1u32 << 16) - 1)) | ((self.src_h & ((1u32 << 16) - 1)) << 16);

        p[9] = (self.dst_w & ((1u32 << 16) - 1)) | ((self.dst_h & ((1u32 << 16) - 1)) << 16);

        p[8] = (self.dst_c & ((1u32 << 16) - 1)) | ((self.src_c & ((1u32 << 16) - 1)) << 16);

        p[7] = self.src_n_stride;

        p[6] = (self.src_h_stride & ((1u32 << 16) - 1))
            | ((self.src_c_stride_low & ((1u32 << 16) - 1)) << 16);

        p[5] = self.dst_n_stride;

        p[4] = (self.dst_h_stride & ((1u32 << 16) - 1))
            | ((self.dst_c_stride_low & ((1u32 << 16) - 1)) << 16);

        p[3] = (self.const_val & ((1u32 << 16) - 1))
            | ((self.src_base_reg_sel & ((1u32 << 3) - 1)) << 16)
            | ((self.mv_lut_idx & 1) << 19)
            | ((self.dst_base_reg_sel & ((1u32 << 3) - 1)) << 20)
            | ((self.mv_lut_base & 1) << 23)
            | ((self.rsv4_5 & ((1u32 << 8) - 1)) << 24);

        p[2] = (self.wait_id_other_tdma & ((1u32 << 16) - 1))
            | ((self.wait_id_sdma & ((1u32 << 16) - 1)) << 16);

        p[1] = (self.spec_func & ((1u32 << 3) - 1))
            | ((self.dst_fmt & ((1u32 << 2) - 1)) << 3)
            | ((self.src_fmt & ((1u32 << 2) - 1)) << 5)
            | ((self.cmprs_fmt & 1) << 7)
            | ((self.sys_dtype & 1) << 8)
            | ((self.rsv2_1 & ((1u32 << 4) - 1)) << 9)
            | ((self.int8_sign & 1) << 13)
            | ((self.compress_zero_guard & 1) << 14)
            | ((self.int8_rnd_mode & 1) << 15)
            | ((self.wait_id_tpu & ((1u32 << 16) - 1)) << 16);

        p[0] = (self.vld & 1)
            | ((self.compress_en & 1) << 1)
            | ((self.eod & 1) << 2)
            | ((self.intp_en & 1) << 3)
            | ((self.bar_en & 1) << 4)
            | ((self.check_bf16_value & 1) << 5)
            | ((self.trans_dir & ((1u32 << 2) - 1)) << 6)
            | ((self.rsv00 & ((1u32 << 2) - 1)) << 8)
            | ((self.trans_fmt & 1) << 10)
            | ((self.transpose_md & ((1u32 << 2) - 1)) << 11)
            | ((self.rsv01 & 1) << 13)
            | ((self.intra_cmd_paral & 1) << 14)
            | ((self.outstanding_en & 1) << 15)
            | ((self.cmd_id & ((1u32 << 16) - 1)) << 16);
    }
}
