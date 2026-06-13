//! EP0 控制传输与标准枚举便捷函数（通道 0）。

use super::channel::{self, CH_CTL, PID_DATA0, PID_DATA1, PID_SETUP, hcchar_build, pktcnt_for};
use super::dma::{self, DMA_OFF_SMALL_IO, OFF_EP0};
use super::regs::{HCCHAR, HCTSIZ};
use crate::usb::UsbClass;
use crate::usb::error::{UsbError, UsbResult};
use crate::usb::setup;
use crate::utils::{cache, spin_delay};

/// `SET_ADDRESS` 后粗延时，满足 USB 2.0 在下一事务前使用新地址的要求。
pub fn usb_post_set_address_delay() {
    spin_delay(20_000_000);
}

#[inline]
fn normalize_ep0_mps(b: u8) -> u32 {
    match b {
        8 | 16 | 32 | 64 => b as u32,
        _ => 8,
    }
}

/// 控制传输无数据阶段：`SETUP` + `STATUS` IN（零长度）。
///
/// # 参数
/// - `dev`：目标设备 USB 地址（7 位数值，写入主机通道 DevAddr）。
/// - `setup`：8 字节标准 SETUP 包（小端字段已拼好）。
/// - `ep0_mps`：该设备 EP0 最大包长（字节，8/16/32/64）。
pub fn ep0_control_write_no_data(dev: u32, setup: [u8; 8], ep0_mps: u32) -> UsbResult<()> {
    unsafe {
        core::ptr::copy_nonoverlapping(setup.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        let hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, true, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// 在默认地址 0 上发送 `SET_ADDRESS`。
///
/// # 参数
/// - `addr`：设备新地址，合法 **1..=127**。
/// - `ep0_mps`：地址 0 阶段使用的 EP0 MPS（枚举首步常用 64）。
pub fn set_usb_address(addr: u8, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(0, setup::set_address(addr), ep0_mps)
}

/// 对已寻址设备发送 `SET_CONFIGURATION`。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `cfg`：`bConfigurationValue`（通常非 0 表示激活配置）。
/// - `ep0_mps`：该设备 EP0 最大包长。
pub fn set_configuration(dev: u32, cfg: u8, ep0_mps: u32) -> UsbResult<()> {
    ep0_control_write_no_data(dev, setup::set_configuration(cfg), ep0_mps)
}

/// `GET_CONFIGURATION`：返回当前 `bConfigurationValue`（单字节）。
///
/// # 参数
/// - `dev`：设备 USB 地址。
/// - `ep0_mps`：该设备 EP0 最大包长。
#[allow(dead_code)]
pub fn get_configuration(dev: u32, ep0_mps: u32) -> UsbResult<u8> {
    unsafe {
        let setup_pkt = setup::get_configuration();
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, true, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(1)).value,
            OFF_EP0 as u32,
        )?;
        cache::dcache_invalidate_after_dma(dma::dma_ptr().add(OFF_EP0), 1);
        let v = dma::dma_ptr().add(OFF_EP0).read();

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;

        Ok(v)
    }
}

/// `GET_DESCRIPTOR(DEVICE, 18)` @ 地址 0；返回 VID、PID、EP0 MPS、`bDeviceClass`。
///
/// # 返回值
/// `(vid, pid, ep0_mps, b_device_class)`，均在设备描述符前 18 字节内解析。
pub fn get_device_vid_pid_default_addr() -> UsbResult<(u16, u16, u32, UsbClass)> {
    unsafe {
        let wlen: u16 = 18;
        let setup_pkt = setup::get_descriptor_device(wlen);
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_build(0, 0, 64, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        hc = hcchar_build(0, 0, 64, HCCHAR::EPTYPE::Control, true, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1)
                + HCTSIZ::PKTCNT.val(1)
                + HCTSIZ::XFERSIZE.val(wlen as u32))
            .value,
            OFF_EP0 as u32,
        )?;
        cache::dcache_invalidate_after_dma(dma::dma_ptr().add(OFF_EP0), wlen as usize);

        let sl = core::slice::from_raw_parts(dma::dma_ptr().add(OFF_EP0), wlen as usize);
        if sl.len() < 12 {
            return Err(UsbError::Protocol("short descriptor"));
        }
        let vid = u16::from_le_bytes([sl[8], sl[9]]);
        let pid = u16::from_le_bytes([sl[10], sl[11]]);
        let ep0_mps = normalize_ep0_mps(sl[7]);
        let dev_class = UsbClass::from_raw(sl[4]);

        hc = hcchar_build(0, 0, 64, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;

        Ok((vid, pid, ep0_mps, dev_class))
    }
}

/// 控制传输：SETUP + 若干 IN 数据包（DATA1/DATA0 交替）+ STATUS OUT（ZLP，DATA1）。
pub fn ep0_control_read(
    dev: u32,
    setup_pkt: [u8; 8],
    ep0_mps: u32,
    out: &mut [u8],
) -> UsbResult<()> {
    if out.is_empty() || out.len() > 4096 {
        return Err(UsbError::Protocol("bad ep0 read len"));
    }
    let total = out.len() as u32;
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        let mut left = total;
        let mut out_off: usize = 0;
        let mut toggle = PID_DATA1;
        while left > 0 {
            let chunk = left.min(ep0_mps);
            let pkts = pktcnt_for(ep0_mps, chunk);
            hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, true, 0);
            channel::ch_xfer(
                CH_CTL,
                hc,
                (HCTSIZ::PID.val(toggle) + HCTSIZ::PKTCNT.val(pkts) + HCTSIZ::XFERSIZE.val(chunk))
                    .value,
                DMA_OFF_SMALL_IO as u32,
            )?;
            cache::dcache_invalidate_after_dma(
                dma::dma_ptr().add(DMA_OFF_SMALL_IO),
                chunk as usize,
            );
            core::ptr::copy_nonoverlapping(
                dma::dma_ptr().add(DMA_OFF_SMALL_IO),
                out.as_mut_ptr().add(out_off),
                chunk as usize,
            );
            out_off += chunk as usize;
            left -= chunk;
            toggle = if toggle == PID_DATA1 {
                PID_DATA0
            } else {
                PID_DATA1
            };
        }

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// 控制写：`SETUP` + `DATA` OUT（可多包）+ `STATUS` IN（ZLP）。
pub fn ep0_control_write(dev: u32, setup_pkt: [u8; 8], ep0_mps: u32, data: &[u8]) -> UsbResult<()> {
    if data.len() > 4096 {
        return Err(UsbError::Protocol("bad ep0 write data len"));
    }
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        let mut left = data.len() as u32;
        let mut src: usize = 0;
        let mut toggle = PID_DATA1;
        while left > 0 {
            let chunk = left.min(ep0_mps);
            let pkts = pktcnt_for(ep0_mps, chunk);
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(src),
                dma::dma_ptr().add(DMA_OFF_SMALL_IO),
                chunk as usize,
            );
            cache::dcache_clean_for_dma(dma::dma_ptr().add(DMA_OFF_SMALL_IO), chunk as usize);
            hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
            channel::ch_xfer(
                CH_CTL,
                hc,
                (HCTSIZ::PID.val(toggle) + HCTSIZ::PKTCNT.val(pkts) + HCTSIZ::XFERSIZE.val(chunk))
                    .value,
                DMA_OFF_SMALL_IO as u32,
            )?;
            src += chunk as usize;
            left -= chunk;
            toggle = if toggle == PID_DATA1 {
                PID_DATA0
            } else {
                PID_DATA1
            };
        }

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, true, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;
        Ok(())
    }
}

/// EP0 控制读：固定读 **1** 字节数据 IN（含 cache 维护），用于 MSC `GET_MAX_LUN` 等。
pub fn ep0_control_read_one_byte(dev: u32, setup_pkt: [u8; 8], ep0_mps: u32) -> UsbResult<u8> {
    unsafe {
        core::ptr::copy_nonoverlapping(setup_pkt.as_ptr(), dma::dma_ptr().add(OFF_EP0), 8);
        cache::dcache_clean_for_dma(dma::dma_ptr().add(OFF_EP0), 8);

        let mut hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_SETUP) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(8)).value,
            OFF_EP0 as u32,
        )?;

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, true, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(1)).value,
            OFF_EP0 as u32,
        )?;
        cache::dcache_invalidate_after_dma(dma::dma_ptr().add(OFF_EP0), 1);
        let v = dma::dma_ptr().add(OFF_EP0).read();

        hc = hcchar_build(dev, 0, ep0_mps, HCCHAR::EPTYPE::Control, false, 0);
        channel::ch_xfer(
            CH_CTL,
            hc,
            (HCTSIZ::PID.val(PID_DATA1) + HCTSIZ::PKTCNT.val(1) + HCTSIZ::XFERSIZE.val(0)).value,
            OFF_EP0 as u32,
        )?;
        Ok(v)
    }
}
