// Copyright (C) Cvitek Co., Ltd. 2019-2021. All rights reserved.
//
// TPU 平台控制模块
// 基于原始 C 驱动代码 tpu_platform.c/h 转换

use super::pmu::{TpuPmu, TpuPmuEvent};
use super::reg_tdma::*;
use super::reg_tiu::*;
use super::types::*;

/// 超时时间 (毫秒)
pub const TIMEOUT_MS: u32 = 60 * 1000;

/// 读取 32 位寄存器
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn raw_read32(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// 写入 32 位寄存器
///
/// # Safety
/// 调用者必须确保地址有效
#[inline]
pub unsafe fn raw_write32(addr: usize, value: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, value) }
}

/// TPU 平台驱动
pub struct TpuPlatform {
    /// TDMA 基地址
    tdma_base: usize,
    /// TIU 基地址
    tiu_base: usize,
    /// 寄存器备份
    reg_backup: TpuRegBackupInfo,
    /// 同步备份标志
    sync_backup: bool,
    /// 挂起处理中断标志
    suspend_handle_int: bool,
}

impl TpuPlatform {
    /// 创建新的 TPU 平台驱动实例
    pub const fn new(tdma_base: usize, tiu_base: usize) -> Self {
        Self {
            tdma_base,
            tiu_base,
            reg_backup: TpuRegBackupInfo {
                tdma_int_mask: 0,
                tdma_sync_status: 0,
                tiu_ctrl_base_address: 0,
                tdma_arraybase0_l: 0,
                tdma_arraybase1_l: 0,
                tdma_arraybase2_l: 0,
                tdma_arraybase3_l: 0,
                tdma_arraybase4_l: 0,
                tdma_arraybase5_l: 0,
                tdma_arraybase6_l: 0,
                tdma_arraybase7_l: 0,
                tdma_arraybase0_h: 0,
                tdma_arraybase1_h: 0,
                tdma_des_base: 0,
                tdma_dbg_mode: 0,
                tdma_dcm_disable: 0,
                tdma_ctrl: 0,
            },
            sync_backup: false,
            suspend_handle_int: false,
        }
    }

    /// 获取平台配置
    pub fn get_config(&self, pmubuf_addr_p: u64, pmubuf_size: u32) -> TpuPlatformConfig {
        TpuPlatformConfig {
            iomem_tdma_base: self.tdma_base,
            iomem_tiu_base: self.tiu_base,
            pmubuf_size,
            pmubuf_addr_p,
        }
    }

    /// 清除中断
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn clear_int(&mut self) {
        unsafe {
            // 获取中断状态
            let reg_value = raw_read32(self.tdma_base + TDMA_INT_MASK);
            let int_status = (reg_value >> 16) & !TDMA_MASK_INIT;

            if int_status != TDMA_INT_EOD && int_status != TDMA_INT_EOPMU {
                // 错误: 中断状态异常
                let _sync_status = raw_read32(self.tdma_base + TDMA_SYNC_STATUS);
            }

            raw_write32(self.tdma_base + TDMA_INT_MASK, 0xFFFF0000);

            self.reg_backup.tdma_int_mask = raw_read32(self.tdma_base + TDMA_INT_MASK);
            self.reg_backup.tdma_sync_status = raw_read32(self.tdma_base + TDMA_SYNC_STATUS);
            self.reg_backup.tiu_ctrl_base_address = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);

            self.sync_backup = true;
        }
    }

    /// 处理 TDMA 中断
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn handle_tdma_irq(&mut self) {
        unsafe { self.clear_int() }
    }

    /// 设置 TDMA 描述符并触发
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    unsafe fn set_tdma_descriptor_fire(&mut self, desc_offset: u64, num_tdma: u32) {
        unsafe {
            // 设置 TDMA 描述符地址
            raw_write32(self.tdma_base + TDMA_DES_BASE, desc_offset as u32);

            // 确保调试模式禁用
            raw_write32(self.tdma_base + TDMA_DEBUG_MODE, 0x0);

            // 启用 TDMA DCM
            raw_write32(self.tdma_base + TDMA_DCM_DISABLE, 0x0);

            // 初始化中断掩码
            raw_write32(self.tdma_base + TDMA_INT_MASK, TDMA_MASK_INIT);

            self.sync_backup = false;

            raw_write32(
                self.tdma_base + TDMA_CTRL,
                (0x1 << TDMA_CTRL_ENABLE_BIT)
                    | (0x1 << TDMA_CTRL_MODESEL_BIT)
                    | (num_tdma << TDMA_CTRL_DESNUM_BIT)
                    | (0x3 << TDMA_CTRL_BURSTLEN_BIT)
                    | (0x1 << TDMA_CTRL_FORCE_1ARRAY)
                    | (0x1 << TDMA_CTRL_INTRA_CMD_OFF)
                    | (0x1 << TDMA_CTRL_64BYTE_ALIGN_EN),
            );
        }
    }

    /// 设置 TIU 描述符
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    unsafe fn set_tiu_descriptor(&self, desc_offset: u64, _num_bd: u32) {
        unsafe {
            let desc_addr = desc_offset << BDC_ENGINE_CMD_ALIGNED_BIT;

            // 设置 TIU 描述符地址
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR + 0x4,
                (desc_addr & 0xFFFFFFFF) as u32,
            );
            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR + 0x8);
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR + 0x8,
                (reg_val & 0xFFFFFF00) | ((desc_addr >> 32) as u32 & 0xFF),
            );

            // 禁用 TIU pre_exe
            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR + 0xC);
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR + 0xC,
                reg_val | (0x1 << 11),
            );

            // 设置 1 array, lane=8
            let mut reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);
            reg_val &= !0x3FC00000;
            raw_write32(self.tiu_base + BD_CTRL_BASE_ADDR, reg_val | (3 << 22));

            // 触发 TIU
            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR,
                reg_val | (0x1 << BD_DES_ADDR_VLD) | (0x1 << BD_INTR_ENABLE) | (0x1 << BD_TPU_EN),
            );
        }
    }

    /// 重新同步命令 ID
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn resync_cmd_id(&self) {
        unsafe {
            // 重置 TIU ID
            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR + 0xC);
            raw_write32(self.tiu_base + BD_CTRL_BASE_ADDR + 0xC, reg_val | 0x1);
            raw_write32(self.tiu_base + BD_CTRL_BASE_ADDR + 0xC, reg_val & !0x1);

            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR,
                reg_val & !((0x1 << BD_TPU_EN) | (0x1 << BD_DES_ADDR_VLD)),
            );

            // 重置 TIU 中断状态
            let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);
            raw_write32(self.tiu_base + BD_CTRL_BASE_ADDR, reg_val | (0x1 << 1));

            // 重置 DMA ID
            raw_write32(
                self.tdma_base + TDMA_CTRL,
                0x1 << TDMA_CTRL_RESET_SYNCID_BIT,
            );
            raw_write32(self.tdma_base + TDMA_CTRL, 0x0);

            // 重置 DMA 中断状态
            raw_write32(self.tdma_base + TDMA_INT_MASK, 0xFFFF0000);
        }
    }

    /// 轮询命令缓冲区完成
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn poll_cmdbuf_done(&self, id_node: &CmdIdNode) -> Result<(), i32> {
        unsafe {
            if id_node.tdma_cmd_id > 0 {
                let reg_val2 = self.reg_backup.tdma_sync_status;
                if (reg_val2 >> 16) < id_node.tdma_cmd_id {
                    // 错误: TDMA 中断已触发但 ID 不是最后一个
                    return Err(-1);
                }
            }

            if id_node.bd_cmd_id > 0 {
                // 轮询直到 BD 完成
                loop {
                    let reg_val = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);
                    if (((reg_val >> 6) & 0xFFFF) >= id_node.bd_cmd_id)
                        && ((reg_val & (0x1 << 1)) != 0)
                    {
                        raw_write32(self.tiu_base + BD_CTRL_BASE_ADDR, reg_val | (0x1 << 1));
                        break;
                    }
                    core::hint::spin_loop();
                }
            }

            Ok(())
        }
    }

    /// 设置数组基地址
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    unsafe fn set_array_base(&self, header: &DmaHeader) {
        unsafe {
            raw_write32(self.tdma_base + TDMA_ARRAYBASE0_L, header.arraybase_0_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE1_L, header.arraybase_1_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE2_L, header.arraybase_2_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE3_L, header.arraybase_3_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE4_L, header.arraybase_4_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE5_L, header.arraybase_5_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE6_L, header.arraybase_6_l);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE7_L, header.arraybase_7_l);

            // 假设高位始终为 0
            raw_write32(self.tdma_base + TDMA_ARRAYBASE0_H, 0);
            raw_write32(self.tdma_base + TDMA_ARRAYBASE1_H, 0);
        }
    }

    /// 运行 DMA 缓冲区
    ///
    /// # Safety
    /// 调用者必须确保所有指针和地址有效
    pub unsafe fn run_dmabuf(
        &mut self,
        dmabuf_v: *const u8,
        dmabuf_p: u64,
        wait_completion: impl Fn() -> Result<(), i32>,
    ) -> Result<(), i32> {
        unsafe {
            let header = &*(dmabuf_v as *const DmaHeader);

            if header.dmabuf_magic_m != TPU_DMABUF_HEADER_M {
                return Err(-1);
            }

            self.sync_backup = false;
            self.suspend_handle_int = false;

            let cfg = self.get_config(dmabuf_p + header.pmubuf_offset as u64, header.pmubuf_size);

            self.set_array_base(header);

            // 检查是否启用 PMU
            let pmu_enable = header.pmubuf_offset != 0 && header.pmubuf_size != 0;

            if pmu_enable {
                TpuPmu::enable(&cfg, true, TpuPmuEvent::TdmaBandwidth);
            }

            let desc_base = dmabuf_v.add(core::mem::size_of::<DmaHeader>()) as *const CpuSyncDesc;

            for i in 0..header.cpu_desc_count {
                let desc = &*desc_base.add(i as usize);
                let bd_num = desc.num_bd & 0xFFFF;
                let tdma_num = desc.num_gdma & 0xFFFF;
                let bd_offset = desc.offset_bd;
                let tdma_offset = desc.offset_gdma;

                self.resync_cmd_id();

                let id_node = CmdIdNode {
                    bd_cmd_id: bd_num,
                    tdma_cmd_id: tdma_num,
                };

                if bd_num > 0 {
                    self.set_tiu_descriptor(bd_offset as u64, bd_num);
                }

                if tdma_num > 0 {
                    self.set_tdma_descriptor_fire(tdma_offset as u64, tdma_num);
                }

                // 等待完成
                if tdma_num > 0 {
                    wait_completion()?;
                }

                // 检查 TDMA/TIU 当前描述符
                if !self.suspend_handle_int {
                    self.poll_cmdbuf_done(&id_node)?;
                }
            }

            // 禁用 PMU
            if pmu_enable {
                TpuPmu::enable(&cfg, false, TpuPmuEvent::TdmaBandwidth);

                if !self.suspend_handle_int {
                    wait_completion()?;
                }
            }

            Ok(())
        }
    }

    /// 设置 TDMA PIO 模式
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    unsafe fn set_tdma_pio(&self, pio_array: &[u32; 16]) {
        unsafe {
            self.resync_cmd_id();

            for i in 0..16 {
                raw_write32(self.tdma_base + TDMA_CMD_ACCP0 + (i << 2), pio_array[i]);
            }

            // 确保调试模式禁用
            raw_write32(self.tdma_base + TDMA_DEBUG_MODE, 0x0);

            // 启用 TDMA DCM
            raw_write32(self.tdma_base + TDMA_DCM_DISABLE, 0x0);

            // 初始化中断掩码
            raw_write32(self.tdma_base + TDMA_INT_MASK, TDMA_MASK_INIT);

            raw_write32(
                self.tdma_base + TDMA_CTRL,
                (0x1 << TDMA_CTRL_ENABLE_BIT)
                    | (0x1 << TDMA_CTRL_DESNUM_BIT)
                    | (0x3 << TDMA_CTRL_BURSTLEN_BIT)
                    | (0x1 << TDMA_CTRL_FORCE_1ARRAY)
                    | (0x1 << TDMA_CTRL_INTRA_CMD_OFF)
                    | (0x1 << TDMA_CTRL_64BYTE_ALIGN_EN),
            );
        }
    }

    /// 运行 PIO 模式传输
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn run_pio(
        &self,
        info: &TdmaPioInfo,
        wait_completion: impl Fn() -> Result<(), i32>,
    ) -> Result<(), i32> {
        unsafe {
            let mut reg = TdmaReg::new();
            let mut pio_array = [0u32; 16];

            reg.vld = 1;
            reg.trans_dir = 2; // 0:tg2l, 1:l2tg, 2:g2g, 3:l2l
            reg.src_base_addr_low = info.paddr_src as u32;
            reg.src_base_addr_high = (info.paddr_src >> 32) as u32;
            reg.dst_base_addr_low = info.paddr_dst as u32;
            reg.dst_base_addr_high = (info.paddr_dst >> 32) as u32;
            reg.eod = 1;
            reg.intp_en = 1;

            if info.enable_2d != 0 {
                reg.trans_fmt = 0; // 0:tensor, 1:common
                reg.src_n = 1;
                reg.src_c = 1;
                reg.src_h = info.h;
                reg.src_w = info.w_bytes;

                reg.dst_c = 1;
                reg.dst_h = reg.src_h;
                reg.dst_w = reg.src_w;

                reg.src_n_stride = info.stride_bytes_src * info.h;
                reg.src_h_stride = info.stride_bytes_src;

                reg.dst_n_stride = info.stride_bytes_dst * info.h;
                reg.dst_h_stride = info.stride_bytes_dst;
            } else {
                reg.trans_fmt = 1; // 0:tensor, 1:common
                reg.src_n_stride = info.leng_bytes;
            }

            reg.emit(&mut pio_array);
            self.set_tdma_pio(&pio_array);

            wait_completion()
        }
    }

    /// 挂起 TPU
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn suspend(&mut self) {
        unsafe {
            self.reg_backup.tdma_int_mask = raw_read32(self.tdma_base + TDMA_INT_MASK);
            self.reg_backup.tdma_sync_status = raw_read32(self.tdma_base + TDMA_SYNC_STATUS);
            self.reg_backup.tiu_ctrl_base_address = raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);

            self.reg_backup.tdma_arraybase0_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE0_L);
            self.reg_backup.tdma_arraybase1_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE1_L);
            self.reg_backup.tdma_arraybase2_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE2_L);
            self.reg_backup.tdma_arraybase3_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE3_L);
            self.reg_backup.tdma_arraybase4_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE4_L);
            self.reg_backup.tdma_arraybase5_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE5_L);
            self.reg_backup.tdma_arraybase6_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE6_L);
            self.reg_backup.tdma_arraybase7_l = raw_read32(self.tdma_base + TDMA_ARRAYBASE7_L);
            self.reg_backup.tdma_arraybase0_h = raw_read32(self.tdma_base + TDMA_ARRAYBASE0_H);
            self.reg_backup.tdma_arraybase1_h = raw_read32(self.tdma_base + TDMA_ARRAYBASE1_H);

            self.reg_backup.tdma_des_base = raw_read32(self.tdma_base + TDMA_DES_BASE);
            self.reg_backup.tdma_dbg_mode = raw_read32(self.tdma_base + TDMA_DEBUG_MODE);
            self.reg_backup.tdma_dcm_disable = raw_read32(self.tdma_base + TDMA_DCM_DISABLE);
            self.reg_backup.tdma_ctrl = raw_read32(self.tdma_base + TDMA_CTRL);

            if self.reg_backup.tdma_ctrl & (0x1 << TDMA_CTRL_ENABLE_BIT) != 0 {
                // 如果需要轮询中断
                if !self.sync_backup {
                    // 轮询等待中断
                    loop {
                        let reg_value = raw_read32(self.tdma_base + TDMA_INT_MASK);
                        let int_status = (reg_value >> 16) & !TDMA_MASK_INIT;

                        if int_status == TDMA_INT_EOD || int_status == TDMA_INT_EOPMU {
                            break;
                        }

                        core::hint::spin_loop();
                    }

                    self.reg_backup.tdma_int_mask = raw_read32(self.tdma_base + TDMA_INT_MASK);
                    self.reg_backup.tdma_sync_status =
                        raw_read32(self.tdma_base + TDMA_SYNC_STATUS);
                    self.reg_backup.tiu_ctrl_base_address =
                        raw_read32(self.tiu_base + BD_CTRL_BASE_ADDR);

                    self.sync_backup = true;
                    self.suspend_handle_int = true;
                }
            }
        }
    }

    /// 恢复 TPU
    ///
    /// # Safety
    /// 调用者必须确保 MMIO 地址有效
    pub unsafe fn resume(&self) {
        unsafe {
            raw_write32(
                self.tdma_base + TDMA_INT_MASK,
                self.reg_backup.tdma_int_mask,
            );
            raw_write32(
                self.tdma_base + TDMA_SYNC_STATUS,
                self.reg_backup.tdma_sync_status,
            );
            raw_write32(
                self.tiu_base + BD_CTRL_BASE_ADDR,
                self.reg_backup.tiu_ctrl_base_address,
            );

            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE0_L,
                self.reg_backup.tdma_arraybase0_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE1_L,
                self.reg_backup.tdma_arraybase1_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE2_L,
                self.reg_backup.tdma_arraybase2_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE3_L,
                self.reg_backup.tdma_arraybase3_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE4_L,
                self.reg_backup.tdma_arraybase4_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE5_L,
                self.reg_backup.tdma_arraybase5_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE6_L,
                self.reg_backup.tdma_arraybase6_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE7_L,
                self.reg_backup.tdma_arraybase7_l,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE0_H,
                self.reg_backup.tdma_arraybase0_h,
            );
            raw_write32(
                self.tdma_base + TDMA_ARRAYBASE1_H,
                self.reg_backup.tdma_arraybase1_h,
            );

            raw_write32(self.tdma_base + TDMA_DES_BASE, self.reg_backup.tdma_des_base);
            raw_write32(
                self.tdma_base + TDMA_DEBUG_MODE,
                self.reg_backup.tdma_dbg_mode,
            );
            raw_write32(
                self.tdma_base + TDMA_DCM_DISABLE,
                self.reg_backup.tdma_dcm_disable,
            );
        }
    }

    /// 重置 TPU
    ///
    /// 注意: 实际的复位控制需要在调用者处实现
    pub fn reset(&self) {
        // 复位控制需要平台特定实现
        // 在原始 C 代码中使用 reset_control_assert/deassert
    }

    /// 初始化 TPU
    ///
    /// 注意: 时钟使能需要在调用者处实现
    pub fn init(&self) {
        // 时钟使能需要平台特定实现
        self.reset();
    }

    /// 反初始化 TPU
    ///
    /// 注意: 时钟禁用需要在调用者处实现
    pub fn deinit(&self) {
        // 时钟禁用需要平台特定实现
    }
}
