//! VideoControl 实体解析与摄像头图像调节（ProcessingUnit / CameraTerminal）。

use crate::usb::UsbClass;
use crate::usb::error::UsbResult;
use crate::usb::host::dwc2;

use super::consts::*;
use super::setup::{uvc_get_cur_vc, uvc_get_def_vc, uvc_set_cur_vc};

/// `parse_uvc_control_entities` 的输出：UVC VideoControl 接口及其下的实体 ID/支持位。
#[derive(Clone, Debug, Default)]
pub struct UvcControlEntities {
    pub vc_interface: u8,
    /// CameraTerminal（输入终端，wTerminalType=0x0201）的 entity ID。
    pub camera_terminal_id: Option<u8>,
    /// CameraTerminal `bmControls` 位掩码（最多 24 位，UVC 1.5）。
    pub ct_controls: u32,
    /// ProcessingUnit 的 entity ID。
    pub processing_unit_id: Option<u8>,
    /// ProcessingUnit `bmControls` 位掩码（最多 24 位）。
    pub pu_controls: u32,
}

/// 解析配置描述符，找出 VideoControl 接口下的 CameraTerminal/ProcessingUnit 实体 ID
/// 与各自的 `bmControls`，用于后续 SET_CUR 控制（自动白平衡 / 自动曝光等）。
pub fn parse_uvc_control_entities(cfg: &[u8], cfg_total: usize) -> Option<UvcControlEntities> {
    let len = cfg_total.min(cfg.len());
    if len < 12 {
        return None;
    }
    let mut i = usize::from(cfg[0]);
    if i >= len {
        return None;
    }

    let mut cur_ifc_class = 0u8;
    let mut cur_ifc_sub = 0u8;
    let mut cur_ifc_num;
    let mut out = UvcControlEntities::default();
    let mut found_vc = false;

    while i + 2 <= len {
        let bl = cfg[i] as usize;
        if bl < 2 || i + bl > len {
            break;
        }
        let ty = cfg[i + 1];

        if ty == USB_DT_INTERFACE && bl >= 9 {
            cur_ifc_num = cfg[i + 2];
            cur_ifc_class = cfg[i + 5];
            cur_ifc_sub = cfg[i + 6];
            if cur_ifc_class == UsbClass::Video.as_raw() && cur_ifc_sub == USB_SUBCLASS_VIDEO_CONTROL {
                out.vc_interface = cur_ifc_num;
                found_vc = true;
            }
        } else if ty == CS_INTERFACE
            && cur_ifc_class == UsbClass::Video.as_raw()
            && cur_ifc_sub == USB_SUBCLASS_VIDEO_CONTROL
            && bl >= 3
        {
            let st = cfg[i + 2];
            match st {
                VC_HEADER => {}
                VC_INPUT_TERMINAL
                    // bLength=15+x，bUnitID@3, wTerminalType@4..6, bAssocTerm@6,
                    // 后续 wObjectiveFocalLengthMin/Max + wOcularFocalLength + bControlSize@14, bmControls@15..
                    if bl >= 15 => {
                        let id = cfg[i + 3];
                        let tt = u16::from_le_bytes([cfg[i + 4], cfg[i + 5]]);
                        if tt == ITT_CAMERA {
                            out.camera_terminal_id = Some(id);
                            let csize = cfg[i + 14] as usize;
                            let cmax = csize.min(bl.saturating_sub(15)).min(4);
                            let mut bm = 0u32;
                            for k in 0..cmax {
                                bm |= u32::from(cfg[i + 15 + k]) << (8 * k);
                            }
                            out.ct_controls = bm;
                        }
                    }
                VC_PROCESSING_UNIT
                    // bLength=10+n，bUnitID@3, bSourceID@4, wMaxMultiplier@5..7, bControlSize@7, bmControls@8..
                    if bl >= 9 => {
                        let id = cfg[i + 3];
                        let csize = cfg[i + 7] as usize;
                        let cmax = csize.min(bl.saturating_sub(8)).min(4);
                        let mut bm = 0u32;
                        for k in 0..cmax {
                            bm |= u32::from(cfg[i + 8 + k]) << (8 * k);
                        }
                        out.processing_unit_id = Some(id);
                        out.pu_controls = bm;
                    }
                _ => {}
            }
        }

        i += bl;
    }

    if found_vc { Some(out) } else { None }
}

/// 仅在控制传输出现 STALL 时返回 false，其它错误则当成"不支持"忽略。
fn try_set_cur_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
    value: u8,
) -> bool {
    let setup = uvc_set_cur_vc(vc_if, entity, selector, 1);
    let buf = [value];
    dwc2::ep0_control_write(dev, setup, ep0_mps, &buf).is_ok()
}

fn try_get_cur_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u8> {
    let setup = uvc_get_cur_vc(vc_if, entity, selector, 1);
    let mut buf = [0u8; 1];
    if dwc2::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(buf[0])
    } else {
        None
    }
}

fn try_get_cur_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u16> {
    let setup = uvc_get_cur_vc(vc_if, entity, selector, 2);
    let mut buf = [0u8; 2];
    if dwc2::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(u16::from_le_bytes(buf))
    } else {
        None
    }
}

fn try_get_def_u8(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u8> {
    let setup = uvc_get_def_vc(vc_if, entity, selector, 1);
    let mut buf = [0u8; 1];
    if dwc2::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(buf[0])
    } else {
        None
    }
}

fn try_get_def_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
) -> Option<u16> {
    let setup = uvc_get_def_vc(vc_if, entity, selector, 2);
    let mut buf = [0u8; 2];
    if dwc2::ep0_control_read(dev, setup, ep0_mps, &mut buf).is_ok() {
        Some(u16::from_le_bytes(buf))
    } else {
        None
    }
}

fn try_set_cur_u16(
    dev: u32,
    ep0_mps: u32,
    vc_if: u8,
    entity: u8,
    selector: u8,
    value: u16,
) -> bool {
    let setup = uvc_set_cur_vc(vc_if, entity, selector, 2);
    let buf = value.to_le_bytes();
    dwc2::ep0_control_write(dev, setup, ep0_mps, &buf).is_ok()
}

/// ProcessingUnit 单项控制描述（selector / 宽度 / 可选覆盖值）。
struct PuCtrl {
    bit: u32,
    selector: u8,
    width: u8,
    _name: &'static str,
    override_val: Option<u16>,
}

/// 把 ProcessingUnit 的 1/2 字节控制项设到 `override_val`；为 `None` 则用 `GET_DEF`。
/// 只有 `bmControls` 标记支持的 selector 才会发送。
fn pu_apply_one(dev: u32, ep0_mps: u32, vc_if: u8, pu: u8, bm: u32, ctrl: PuCtrl) {
    if (bm & (1u32 << ctrl.bit)) == 0 {
        return;
    }
    match ctrl.width {
        1 => {
            let cur = try_get_cur_u8(dev, ep0_mps, vc_if, pu, ctrl.selector);
            let want = match ctrl.override_val {
                Some(v) => Some(v as u8),
                None => try_get_def_u8(dev, ep0_mps, vc_if, pu, ctrl.selector),
            };
            match (cur, want) {
                (Some(c), Some(d)) if c != d => {
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, ctrl.selector, d);
                }
                _ => {}
            }
        }
        2 => {
            let cur = try_get_cur_u16(dev, ep0_mps, vc_if, pu, ctrl.selector);
            let want = match ctrl.override_val {
                Some(v) => Some(v),
                None => try_get_def_u16(dev, ep0_mps, vc_if, pu, ctrl.selector),
            };
            match (cur, want) {
                (Some(c), Some(d)) if c != d => {
                    let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, ctrl.selector, d);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// 上层可选的图像调节覆盖。每一项 `Some(v)` 表示用 `v` SET_CUR 覆盖摄像头出厂默认；
/// `None` 表示沿用 `GET_DEF` 出厂默认。
///
/// 使用例：把 `brightness=Some(96)` 让画面比 def 的 128 暗一些。
#[derive(Clone, Copy, Debug, Default)]
pub struct UvcImageTuning {
    pub brightness: Option<u16>,
    pub contrast: Option<u16>,
    pub hue: Option<u16>,
    pub saturation: Option<u16>,
    pub sharpness: Option<u16>,
    pub gamma: Option<u16>,
    pub backlight: Option<u16>,
    pub gain: Option<u16>,
    /// `Some(K)` = 关 Auto WB、用手动色温 K（典型 2800–6500）。`None` = 开 Auto WB。
    pub white_balance_temp_k: Option<u16>,
    /// 0=Disabled, 1=50Hz, 2=60Hz；`None` 时按 50Hz 设置。
    pub power_line_freq: Option<u8>,
}

/// 摄像头初始化常用控制：**自动白平衡**、**自动曝光**、**关闭手动 Hue**、**Power-line 50Hz**、
/// **Focus-Auto**。每一项都按 `bmControls` 决定是否发送，不支持就跳过；STALL 也只是日志，不致命。
///
/// `tune` 中 `Some(v)` 字段会用 `v` 覆盖出厂 def，`None` 字段则沿用 def。
///
/// `0c45:64ab` 等 SunplusIT/Sonix 摄像头出厂在某些场景下默认白平衡是手动模式，
/// 这是图像偏色（偏蓝/偏紫）的最常见原因。
pub fn uvc_init_camera_controls(
    dev: u32,
    ep0_mps: u32,
    ent: &UvcControlEntities,
    tune: &UvcImageTuning,
) -> UsbResult<()> {

    if let Some(pu) = ent.processing_unit_id {
        let vc_if = ent.vc_interface;
        let bm = ent.pu_controls;

        // ① 图像调节参数：tune.* 为 Some 则覆盖；None 则用 GET_DEF。
        // PU bmControls 位定义（UVC 1.5）：
        //   D0=Brightness D1=Contrast D2=Hue D3=Saturation D4=Sharpness
        //   D5=Gamma D6=WB Temp D8=Backlight D9=Gain D10=PowerLineFreq
        //   D11=Hue Auto D12=WB Temp Auto
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 0, selector: PU_BRIGHTNESS_CONTROL, width: 2, _name: "Brightness", override_val: tune.brightness });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 1, selector: PU_CONTRAST_CONTROL, width: 2, _name: "Contrast", override_val: tune.contrast });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 2, selector: PU_HUE_CONTROL, width: 2, _name: "Hue", override_val: tune.hue });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 3, selector: PU_SATURATION_CONTROL, width: 2, _name: "Saturation", override_val: tune.saturation });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 4, selector: PU_SHARPNESS_CONTROL, width: 2, _name: "Sharpness", override_val: tune.sharpness });
        // PU_GAMMA selector = 0x09
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 5, selector: 0x09, width: 2, _name: "Gamma", override_val: tune.gamma });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 8, selector: PU_BACKLIGHT_COMPENSATION, width: 2, _name: "Backlight", override_val: tune.backlight });
        pu_apply_one(dev, ep0_mps, vc_if, pu, bm, PuCtrl { bit: 9, selector: PU_GAIN_CONTROL, width: 2, _name: "Gain", override_val: tune.gain });

        // ② 白平衡
        match tune.white_balance_temp_k {
            // 用户指定手动色温 → 关 Auto，写手动 K
            Some(k) if (bm & (1 << 6)) != 0 => {
                if (bm & (1 << 12)) != 0 {
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 0);
                }
                let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, k);
            }
            // 默认走 Auto WB（若支持 D12）
            _ => {
                if (bm & (1 << 12)) != 0 {
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 0);
                    if (bm & (1 << 6)) != 0
                        && let Some(d) = try_get_def_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL) {
                            let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, d);
                        }
                    let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL, 1);
                } else if (bm & (1 << 6)) != 0 {
                    let val = try_get_def_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL).unwrap_or(4500);
                    let _ = try_set_cur_u16(dev, ep0_mps, vc_if, pu, PU_WHITE_BALANCE_TEMPERATURE_CONTROL, val);
                }
            }
        }

        if (bm & (1 << 11)) != 0 {
            let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_HUE_AUTO_CONTROL, 1);
        }

        if (bm & (1 << 10)) != 0 {
            let plf = tune.power_line_freq.unwrap_or(1);
            let _ = try_set_cur_u8(dev, ep0_mps, vc_if, pu, PU_POWER_LINE_FREQUENCY_CONTROL, plf);
        }
    }

    if let Some(ct) = ent.camera_terminal_id {
        // CT bmControls：D1=AE Mode, D2=AE Priority, D17=Focus Auto
        if (ent.ct_controls & (1 << 1)) != 0 {
            // AE Mode 是位掩码（UVC 1.5）：0x01=Manual, 0x02=Auto,
            // 0x04=Shutter Priority, 0x08=Aperture Priority。
            // 廉价 webcam 多数只接受 0x08（光圈优先=自动曝光），先试 0x02 失败则降级。
            for &mode in &[0x02u8, 0x08, 0x04] {
                if try_set_cur_u8(dev, ep0_mps, ent.vc_interface, ct, CT_AE_MODE_CONTROL, mode) {
                    break;
                }
            }

            if (ent.ct_controls & (1 << 2)) != 0 {
                let _ = try_set_cur_u8(
                    dev,
                    ep0_mps,
                    ent.vc_interface,
                    ct,
                    CT_AE_PRIORITY_CONTROL,
                    1,
                );
            }
        }

        if (ent.ct_controls & (1 << 17)) != 0 {
            let _ = try_set_cur_u8(
                dev,
                ep0_mps,
                ent.vc_interface,
                ct,
                CT_FOCUS_AUTO_CONTROL,
                1,
            );
        }
    }

    Ok(())
}
