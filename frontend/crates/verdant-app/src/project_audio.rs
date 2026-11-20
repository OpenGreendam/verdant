//! Playback and offline export share the headless project-audio service.
pub(crate) use verdant_service::project_audio::{
    MediaPreparation, NativeBinding, NativeBindings, PhrasePcmCache, PhraseSource,
    bind_native_automation, extend_realtime_timeline, master_insert_specs,
    prepare_timeline,
};
