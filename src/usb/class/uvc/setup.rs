//! UVC 类专用 SETUP 包构造（VS PROBE/COMMIT、VC SET_CUR/GET_CUR）。

/// UVC：`SET_CUR`（Video Streaming 接口，`wValue = selector<<8`）。
#[inline]
pub(crate) fn uvc_set_cur_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0x21,
        0x01,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC：`GET_CUR`（VS 探测/提交等）。
#[inline]
pub(crate) fn uvc_get_cur_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0xA1,
        0x81,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC：`GET_MAX`（最大探测结构长度）。
#[inline]
pub(crate) fn uvc_get_max_vs(interface: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    [
        0xA1,
        0x83,
        wvalue as u8,
        (wvalue >> 8) as u8,
        interface,
        0,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC 类控制 `SET_CUR`（**VideoControl 接口**，`wIndex = (entity_id<<8) | interface`）。
#[inline]
pub(crate) fn uvc_set_cur_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0x21,
        0x01,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC 类控制 `GET_CUR`（VideoControl 接口）。
#[inline]
pub(crate) fn uvc_get_cur_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0xA1,
        0x81,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}

/// UVC 类控制 `GET_DEF`（VideoControl 接口）：读取摄像头出厂默认值。
#[inline]
pub(crate) fn uvc_get_def_vc(interface: u8, entity_id: u8, selector: u8, w_length: u16) -> [u8; 8] {
    let wvalue = u16::from(selector) << 8;
    let windex = (u16::from(entity_id) << 8) | u16::from(interface);
    [
        0xA1,
        0x87,
        wvalue as u8,
        (wvalue >> 8) as u8,
        windex as u8,
        (windex >> 8) as u8,
        w_length as u8,
        (w_length >> 8) as u8,
    ]
}
