//! USB Mass Storage Class（MSC，Bulk-Only Transport / BBB）。
//!
//! 本模块提供两层 API：
//!
//! 1. **协议层 SETUP 包构造 / EP0 控制传输辅助**：
//!    - [`bulk_only_reset`]：BOT class-specific request `Mass Storage Reset`（`bRequest = 0xFF`）。
//!    - [`get_max_lun`]：BOT class-specific request `Get Max LUN`（`bRequest = 0xFE`）。
//!
//! 2. **BBB（CBW + Data + CSW）+ SCSI 命令封装**：
//!    - [`MscDevice`]：保存 USB 地址、Bulk IN/OUT 端点号 + MPS、IN/OUT 数据 PID toggle 状态。
//!    - [`MscDevice::inquiry`]、[`MscDevice::test_unit_ready`]、
//!      [`MscDevice::read_capacity_10`]、[`MscDevice::read_10`] 等 SCSI 命令。
//!    - 数据落到 [`crate::usb::host::dwc2::ep0::DMA_OFF_SECTOR`] 起始的 DMA 缓冲，
//!      读后请用 [`MscDevice::read_data`] 取出已 invalidate 的数据。
//!
//! 拓扑扫描期间，[`crate::usb::host::topology`] 会同时把 **MSC 接口号** 和
//! **Bulk IN/OUT 端点 + MPS** 一并塞进 [`crate::usb::host::MscEnumerated`]，
//! caller 可直接 [`MscDevice::from_enumerated`] 构造一个就绪的设备句柄。

use crate::usb::error::{UsbError, UsbResult};
use crate::usb::host::dwc2::ep0::{
    self, bulk_in, bulk_out, dma_rx_slice, DMA_OFF_CBW, DMA_OFF_CSW, DMA_OFF_SECTOR,
    MSC_SECTOR_DMA_CAP, PID_DATA0, PID_DATA1,
};
use crate::usb::host::MscEnumerated;

/// USB MSC `Bulk-Only Mass Storage Reset`（`bmRequestType=0x21`，`bRequest=0xFF`）的 SETUP 字节。
///
/// # 参数
/// - `interface`：MSC 接口号，填入 SETUP 的 `wIndex`（低字节为接口号）。
#[inline]
pub fn mass_storage_reset_setup(interface: u16) -> [u8; 8] {
    [
        0x21,
        0xFF,
        0x00,
        0x00,
        interface as u8,
        (interface >> 8) as u8,
        0x00,
        0x00,
    ]
}

/// `GET_MAX_LUN`（`bmRequestType=0xA1`，`bRequest=0xFE`，`wLength=1`）的 SETUP 字节。
///
/// # 参数
/// - `interface`：MSC 接口号，对应 SETUP 的 `wIndex`。
#[inline]
pub fn get_max_lun_setup(interface: u16) -> [u8; 8] {
    [
        0xA1,
        0xFE,
        0x00,
        0x00,
        interface as u8,
        (interface >> 8) as u8,
        0x01,
        0x00,
    ]
}

/// 对已寻址 MSC 设备发出 `Bulk-Only Mass Storage Reset`（无数据阶段）。
///
/// # 参数
/// - `dev`：设备 USB 地址（7 位数值，与主机通道 `HCCHAR` 中 DevAddr 一致）。
/// - `interface`：MSC 接口号（`wIndex`）。
/// - `ep0_mps`：该设备控制端点 0 的最大包长（字节，来自设备描述符）。
pub fn bulk_only_reset(dev: u32, interface: u16, ep0_mps: u32) -> UsbResult<()> {
    ep0::ep0_control_write_no_data(dev, mass_storage_reset_setup(interface), ep0_mps)
}

/// 对已寻址 MSC 设备执行 `GET_MAX_LUN`，返回 `bMaxLun`（多 LUN 设备使用）。
///
/// # 参数
/// - `dev`、`interface`、`ep0_mps`：含义同 [`bulk_only_reset`]。
///
/// # 返回值
/// `bMaxLun`：最大 LUN 编号（逻辑单元数 = `bMaxLun + 1`）。
pub fn get_max_lun(dev: u32, interface: u16, ep0_mps: u32) -> UsbResult<u8> {
    ep0::ep0_control_read_one_byte(dev, get_max_lun_setup(interface), ep0_mps)
}

/// CBW `dCBWSignature` = `'USBC'`（小端：`0x43425355`）。
pub const CBW_SIGNATURE: u32 = 0x4342_5355;
/// CSW `dCSWSignature` = `'USBS'`（小端：`0x53425355`）。
pub const CSW_SIGNATURE: u32 = 0x5342_5355;
/// `bmCBWFlags` Bit7：1=Data-In（设备→Host），0=Data-Out。
pub const CBW_FLAG_DATA_IN: u8 = 0x80;

/// SCSI Operation Codes（仅本模块使用）。
pub const SCSI_TEST_UNIT_READY: u8 = 0x00;
pub const SCSI_REQUEST_SENSE: u8 = 0x03;
pub const SCSI_INQUIRY: u8 = 0x12;
pub const SCSI_READ_CAPACITY_10: u8 = 0x25;
pub const SCSI_READ_10: u8 = 0x28;

/// `MscDevice`：BBB + SCSI 上层句柄。
///
/// 内部维护 IN / OUT bulk 端点的 **数据 PID toggle**：每次 bulk transfer 后按
/// 已传输的整 packet 数（`ceil(actual / mps)`）翻转一次，与设备端的 endpoint
/// toggle 保持同步。`bulk_only_reset` 后请调用 [`MscDevice::reset_data_toggle`]
/// 把 IN/OUT toggle 都清回 `DATA0`（实际重置在设备端通过 BBB Reset 完成）。
#[derive(Clone, Copy, Debug)]
pub struct MscDevice {
    pub addr: u32,
    pub iface: u8,
    pub ep0_mps: u32,
    pub bulk_in_ep: u32,
    pub bulk_in_mps: u32,
    pub bulk_out_ep: u32,
    pub bulk_out_mps: u32,
    in_pid: u32,
    out_pid: u32,
    next_tag: u32,
}

impl MscDevice {
    /// 由枚举或手工参数构造 MSC 句柄。
    ///
    /// # 参数
    /// - `addr`：设备 USB 地址（7 位数值）。
    /// - `iface`：MSC 接口号（CBW/类请求 `wIndex`）。
    /// - `ep0_mps`：控制端点 0 最大包长。
    /// - `bulk_in_ep` / `bulk_out_ep`：Bulk 端点号（**不含**方向位 `0x80`，与描述符 `bEndpointAddress & 0x0F` 一致）。
    /// - `bulk_in_mps` / `bulk_out_mps`：对应 Bulk 端点 `wMaxPacketSize` 低 11 位（HS 常为 512，FS 为 64）。
    pub fn new(
        addr: u32,
        iface: u8,
        ep0_mps: u32,
        bulk_in_ep: u32,
        bulk_in_mps: u32,
        bulk_out_ep: u32,
        bulk_out_mps: u32,
    ) -> Self {
        Self {
            addr,
            iface,
            ep0_mps,
            bulk_in_ep,
            bulk_in_mps,
            bulk_out_ep,
            bulk_out_mps,
            in_pid: PID_DATA0,
            out_pid: PID_DATA0,
            next_tag: 0xc0ff_ee01,
        }
    }

    /// 从 [`MscEnumerated`]（拓扑扫描结果）构造。
    pub fn from_enumerated(en: &MscEnumerated) -> UsbResult<Self> {
        if en.bulk_in_ep == 0 || en.bulk_out_ep == 0 {
            return Err(UsbError::Protocol("MSC bulk endpoints not enumerated"));
        }
        if en.bulk_in_mps == 0 || en.bulk_out_mps == 0 {
            return Err(UsbError::Protocol("MSC bulk MPS unknown"));
        }
        Ok(Self::new(
            u32::from(en.addr),
            en.iface_num,
            en.ep0_mps,
            u32::from(en.bulk_in_ep),
            u32::from(en.bulk_in_mps),
            u32::from(en.bulk_out_ep),
            u32::from(en.bulk_out_mps),
        ))
    }

    /// 把 BBB 数据 PID toggle 都重置为 `DATA0`（用于 BBB Reset 之后）。
    pub fn reset_data_toggle(&mut self) {
        self.in_pid = PID_DATA0;
        self.out_pid = PID_DATA0;
    }

    /// 发起 BBB Reset：EP0 类请求 + 重置 host 端 toggle。
    /// 注：标准还要求 `CLEAR_FEATURE(ENDPOINT_HALT)` 两个 bulk 端点；
    /// 当前实现仅在设备未 STALL 时使用，故直接把 toggle 清回 DATA0。
    pub fn bulk_only_reset(&mut self) -> UsbResult<()> {
        bulk_only_reset(self.addr, u16::from(self.iface), self.ep0_mps)?;
        self.reset_data_toggle();
        Ok(())
    }

    fn next_tag(&mut self) -> u32 {
        let t = self.next_tag;
        self.next_tag = self.next_tag.wrapping_add(1);
        t
    }

    fn build_cbw(buf: &mut [u8; 31], tag: u32, data_len: u32, dir_in: bool, lun: u8, cdb: &[u8]) {
        debug_assert!(!cdb.is_empty() && cdb.len() <= 16);
        buf[0..4].copy_from_slice(&CBW_SIGNATURE.to_le_bytes());
        buf[4..8].copy_from_slice(&tag.to_le_bytes());
        buf[8..12].copy_from_slice(&data_len.to_le_bytes());
        buf[12] = if dir_in { CBW_FLAG_DATA_IN } else { 0 };
        buf[13] = lun;
        buf[14] = cdb.len() as u8;
        for i in 0..16 {
            buf[15 + i] = if i < cdb.len() { cdb[i] } else { 0 };
        }
    }

    #[inline]
    fn pkt_count(bytes: usize, mps: u32) -> u32 {
        if bytes == 0 {
            1 // ZLP 也算一个 packet
        } else {
            ((bytes as u32) + mps - 1) / mps
        }
    }

    #[inline]
    fn flip_pid(pid: u32, packets: u32) -> u32 {
        if packets & 1 == 0 {
            pid
        } else if pid == PID_DATA0 {
            PID_DATA1
        } else {
            PID_DATA0
        }
    }

    fn send_cbw(
        &mut self,
        tag: u32,
        data_len: u32,
        dir_in: bool,
        lun: u8,
        cdb: &[u8],
    ) -> UsbResult<()> {
        let mut cbw = [0u8; 31];
        Self::build_cbw(&mut cbw, tag, data_len, dir_in, lun, cdb);
        bulk_out(
            self.addr,
            self.bulk_out_ep,
            self.bulk_out_mps,
            self.out_pid,
            &cbw,
            DMA_OFF_CBW,
        )?;
        let pkts = Self::pkt_count(cbw.len(), self.bulk_out_mps);
        self.out_pid = Self::flip_pid(self.out_pid, pkts);
        Ok(())
    }

    fn recv_csw(&mut self, expect_tag: u32) -> UsbResult<u32> {
        let actual = bulk_in(
            self.addr,
            self.bulk_in_ep,
            self.bulk_in_mps,
            self.in_pid,
            13,
            DMA_OFF_CSW,
        )?;
        let pkts = Self::pkt_count(actual, self.bulk_in_mps);
        self.in_pid = Self::flip_pid(self.in_pid, pkts);
        if actual < 13 {
            return Err(UsbError::Protocol("CSW short"));
        }
        let csw = dma_rx_slice(DMA_OFF_CSW, 13).ok_or(UsbError::Protocol("CSW dma slice"))?;
        let sig = u32::from_le_bytes([csw[0], csw[1], csw[2], csw[3]]);
        let tag = u32::from_le_bytes([csw[4], csw[5], csw[6], csw[7]]);
        let residue = u32::from_le_bytes([csw[8], csw[9], csw[10], csw[11]]);
        let status = csw[12];
        if sig != CSW_SIGNATURE {
            return Err(UsbError::Protocol("CSW signature mismatch"));
        }
        if tag != expect_tag {
            return Err(UsbError::Protocol("CSW tag mismatch"));
        }
        match status {
            0 => Ok(residue),
            1 => Err(UsbError::Protocol("SCSI command failed (CSW=1)")),
            _ => Err(UsbError::Protocol("SCSI phase error (CSW=2)")),
        }
    }

    /// 通用 SCSI Data-In 命令：CBW (out) → Data (in) → CSW (in)。
    /// 返回数据阶段实际收到的字节数（写入 `DMA_OFF_SECTOR` 起始）。
    pub fn scsi_command_in(&mut self, lun: u8, cdb: &[u8], data_len: u32) -> UsbResult<usize> {
        if data_len as usize > MSC_SECTOR_DMA_CAP {
            return Err(UsbError::Protocol("SCSI data exceeds DMA window"));
        }
        let tag = self.next_tag();
        self.send_cbw(tag, data_len, true, lun, cdb)?;
        let actual = if data_len > 0 {
            let n = bulk_in(
                self.addr,
                self.bulk_in_ep,
                self.bulk_in_mps,
                self.in_pid,
                data_len as usize,
                DMA_OFF_SECTOR,
            )?;
            let pkts = Self::pkt_count(n, self.bulk_in_mps);
            self.in_pid = Self::flip_pid(self.in_pid, pkts);
            n
        } else {
            0
        };
        let _residue = self.recv_csw(tag)?;
        Ok(actual)
    }

    /// 通用 SCSI No-Data 命令：CBW (out) → CSW (in)。
    pub fn scsi_command_no_data(&mut self, lun: u8, cdb: &[u8]) -> UsbResult<()> {
        let tag = self.next_tag();
        self.send_cbw(tag, 0, false, lun, cdb)?;
        let _residue = self.recv_csw(tag)?;
        Ok(())
    }

    /// SCSI `INQUIRY (0x12)`，标准返回 36 字节。
    pub fn inquiry(&mut self, lun: u8) -> UsbResult<[u8; 36]> {
        let cdb = [SCSI_INQUIRY, 0, 0, 0, 36, 0];
        let actual = self.scsi_command_in(lun, &cdb, 36)?;
        if actual < 36 {
            return Err(UsbError::Protocol("INQUIRY data short"));
        }
        let mut out = [0u8; 36];
        if let Some(s) = dma_rx_slice(DMA_OFF_SECTOR, 36) {
            out.copy_from_slice(s);
        }
        Ok(out)
    }

    /// SCSI `TEST UNIT READY (0x00)`：成功返回 `Ok(())`，失败返回 `CSW=1`。
    pub fn test_unit_ready(&mut self, lun: u8) -> UsbResult<()> {
        let cdb = [SCSI_TEST_UNIT_READY, 0, 0, 0, 0, 0];
        self.scsi_command_no_data(lun, &cdb)
    }

    /// SCSI `READ CAPACITY (10) (0x25)`：返回 `(last_lba, block_size_bytes)`。
    /// 总容量 = `(last_lba + 1) * block_size_bytes` 字节。
    pub fn read_capacity_10(&mut self, lun: u8) -> UsbResult<(u32, u32)> {
        let cdb = [SCSI_READ_CAPACITY_10, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let actual = self.scsi_command_in(lun, &cdb, 8)?;
        if actual < 8 {
            return Err(UsbError::Protocol("READ_CAPACITY data short"));
        }
        let s = dma_rx_slice(DMA_OFF_SECTOR, 8).ok_or(UsbError::Protocol("READ_CAPACITY dma"))?;
        let lba = u32::from_be_bytes([s[0], s[1], s[2], s[3]]);
        let block = u32::from_be_bytes([s[4], s[5], s[6], s[7]]);
        Ok((lba, block))
    }

    /// SCSI `READ (10) (0x28)`：从 `lba` 开始连续读 `blocks` 个块，
    /// 每块 `block_size` 字节，总长度 = `blocks * block_size`，写入 `DMA_OFF_SECTOR`。
    /// 返回实际收到字节数；调用方可用 [`MscDevice::read_data`] 取出。
    pub fn read_10(
        &mut self,
        lun: u8,
        lba: u32,
        blocks: u16,
        block_size: u32,
    ) -> UsbResult<usize> {
        let bytes = (blocks as u32).checked_mul(block_size).ok_or(UsbError::Protocol("read len overflow"))?;
        let cdb = [
            SCSI_READ_10,
            0,
            (lba >> 24) as u8,
            (lba >> 16) as u8,
            (lba >> 8) as u8,
            lba as u8,
            0,
            (blocks >> 8) as u8,
            blocks as u8,
            0,
        ];
        self.scsi_command_in(lun, &cdb, bytes)
    }

    /// 取出 `READ (10)` 等命令落到 DMA 缓冲的数据切片（`'static`，仅 invalidate 后使用）。
    #[inline]
    pub fn read_data(&self, off: usize, len: usize) -> Option<&'static [u8]> {
        dma_rx_slice(DMA_OFF_SECTOR.checked_add(off)?, len)
    }
}
