//! USB Video Class（UVC）：配置描述符解析、`PROBE`/`COMMIT`、**Isoch IN** 抓一帧。
//!
//! 等时传输见 [`crate::usb::host::dwc2::isoch_in_uframe`] /
//! [`crate::usb::host::dwc2::isoch_in_uframe_batch`]。

mod capture;
mod consts;
mod control;
mod parse;
mod setup;
mod stream;

pub use capture::{
    reset_frame_continuity, uvc_capture_one_frame, FRAME_DEBUG, LAST_EOF_FID,
    UVC_ASSEMBLED_JPEG_DMA_OFF, UVC_WORK_AREA_BYTES,
};
pub use control::{
    parse_uvc_control_entities, uvc_init_camera_controls, UvcControlEntities, UvcImageTuning,
};
pub use parse::{
    parse_uvc_video_stream, read_configuration_descriptor, set_preferred_max_pixels,
    PREFERRED_MAX_PIXELS, UvcStreamSelection,
};
pub use stream::{uvc_start_video_stream, uvc_stop_streaming};
