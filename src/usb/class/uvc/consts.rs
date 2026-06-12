pub(crate) const VS_PROBE_CONTROL: u8 = 0x01;
pub(crate) const VS_COMMIT_CONTROL: u8 = 0x02;

pub(crate) const USB_DT_CONFIGURATION: u8 = 2;
pub(crate) const USB_DT_INTERFACE: u8 = 4;
pub(crate) const USB_DT_ENDPOINT: u8 = 5;
pub(crate) const CS_INTERFACE: u8 = 0x24;

pub(crate) const VS_FORMAT_MJPEG: u8 = 0x06;
pub(crate) const VS_FRAME_MJPEG: u8 = 0x07;
pub(crate) const VS_FORMAT_UNCOMPRESSED: u8 = 0x04;
pub(crate) const VS_FRAME_UNCOMPRESSED: u8 = 0x05;

pub(crate) const USB_SUBCLASS_VIDEO_STREAMING: u8 = 0x02;
pub(crate) const USB_SUBCLASS_VIDEO_CONTROL: u8 = 0x01;

// VideoControl class-specific interface descriptor subtypes
pub(crate) const VC_HEADER: u8 = 0x01;
pub(crate) const VC_INPUT_TERMINAL: u8 = 0x02;
pub(crate) const VC_PROCESSING_UNIT: u8 = 0x05;

/// `wTerminalType = 0x0201` 表示 ITT_CAMERA（CameraTerminal）。
pub(crate) const ITT_CAMERA: u16 = 0x0201;

// ProcessingUnit selectors (wValue MSB)
#[allow(dead_code)]
pub(crate) const PU_BACKLIGHT_COMPENSATION: u8 = 0x01;
#[allow(dead_code)]
pub(crate) const PU_BRIGHTNESS_CONTROL: u8 = 0x02;
#[allow(dead_code)]
pub(crate) const PU_CONTRAST_CONTROL: u8 = 0x03;
#[allow(dead_code)]
pub(crate) const PU_GAIN_CONTROL: u8 = 0x04;
#[allow(dead_code)]
pub(crate) const PU_HUE_CONTROL: u8 = 0x06;
#[allow(dead_code)]
pub(crate) const PU_SATURATION_CONTROL: u8 = 0x07;
#[allow(dead_code)]
pub(crate) const PU_SHARPNESS_CONTROL: u8 = 0x08;
pub(crate) const PU_WHITE_BALANCE_TEMPERATURE_CONTROL: u8 = 0x0A;
pub(crate) const PU_WHITE_BALANCE_TEMPERATURE_AUTO_CONTROL: u8 = 0x0B;
pub(crate) const PU_HUE_AUTO_CONTROL: u8 = 0x10;
pub(crate) const PU_POWER_LINE_FREQUENCY_CONTROL: u8 = 0x05;

// CameraTerminal selectors
pub(crate) const CT_AE_MODE_CONTROL: u8 = 0x02;
pub(crate) const CT_AE_PRIORITY_CONTROL: u8 = 0x03;
#[allow(dead_code)]
pub(crate) const CT_EXPOSURE_TIME_ABSOLUTE_CONTROL: u8 = 0x04;
pub(crate) const CT_FOCUS_AUTO_CONTROL: u8 = 0x08;

pub(crate) const ENDPOINT_ATTR_ISOCH: u8 = 1;

pub(crate) const UVC_PROBE_COMMIT_LEN: usize = 34;
