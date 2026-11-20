//! Verdant audio: plays prepared PCM against a device sample clock.
//! No Python, ONNX, MLPG, file I/O or UI locks on the callback path.
//! Disable the default `audio-device` feature for the same mixer, automation,
//! numeric conversion and output rate adapter without CPAL or device I/O.

pub mod callback;
pub mod convert;
#[cfg(feature = "audio-device")]
pub mod device;
pub mod engine;
pub mod automation_clock;
pub mod resample;

pub use callback::CallbackRenderer;
#[cfg(feature = "audio-device")]
pub use device::{
    AudioDeviceInfo, DeviceEvent, DeviceSelection, DeviceStatus,
    OutputEngine, OutputInfo, OutputKind, SampleFmt, list_devices, open_output, open_silent,
};
pub use engine::{Chain, ChannelSettings, Clip, StereoClip, ClickEvent, ClickTrack, Control, MAX_TRACKS, Processor, Timeline, Transport};
pub use resample::resample;

/// Maximum block admitted by the shared output renderer, with or without a
/// physical device. Buffers are prepared to this bound before rendering.
pub const MAX_BLOCK: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceErrorKind {
    Enumerate, NoDevice, DeviceNotFound, UnsupportedConfig, Open, Play, Lost, Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceError {
    pub kind: DeviceErrorKind,
    pub message: String,
}

impl DeviceError {
    pub(crate) fn new(kind: DeviceErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for DeviceError {}
