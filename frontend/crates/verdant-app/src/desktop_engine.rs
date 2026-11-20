//! Desktop singer choices retain admitted sources in one undo step.
//! Only the explicitly unlocked original singer may select the restricted route.
use verdant_backend::{Route, Capabilities};
use verdant_backend::adapter;
use verdant_backend::voices::{DESKTOP_ORIGINAL_REFERENCE, is_desktop_original};
use verdant_domain::{Command, CommandBatch, Project, TrackId};

pub fn original_usable(caps: &Capabilities) -> bool {
    caps.supports_route(Route::C0) && caps.voices.as_ref().is_some_and(|catalog| {
        catalog.version == 1 && catalog.voices.get(DESKTOP_ORIGINAL_REFERENCE)
            .is_some_and(|voice| !voice.cache_identity.is_empty() && is_desktop_original(voice))
    })
}

pub fn singer_choice_batch(project: &Project, track_id: &TrackId, reference: &str, caps: &Capabilities)
    -> Result<CommandBatch, String>
{
    let voice = caps.voices.as_ref().and_then(|catalog| catalog.voices.get(reference)).ok_or("声库尚未安装")?;
    let route = if reference == DESKTOP_ORIGINAL_REFERENCE && original_usable(caps) {
        Route::C0
    } else if adapter::is_desktop_voice(voice) {
        Route::VerdantVoice
    } else {
        return Err("该声库尚未通过桌面声库准入".into());
    };
    let mut batch = verdant_backend::voices::singer_choice_batch(project, track_id, reference, caps)?;
    for command in &mut batch.commands {
        if let Command::SetTrackHeader { header, .. } = command {
            header.render_route = Some(route.as_str().into());
        }
    }
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_domain::ProjectStore;

    fn caps() -> Capabilities {
        let token = concat!("821b7cd9e7b489077dd55bf581ba90e1", "dffd3816c6fedcedc320744b498deead");
        let model = "213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff";
        let mut caps:Capabilities=serde_json::from_value(serde_json::json!({"routes":["C0","verdant-voice"],"voices":{"version":1,"default_voice":"final",
            "voices":{
                "verdant-original-sn16k":{"available":true,"cache_identity":"original", "files":{
                    "token":{"available":true,"sha256":token},"model":{"available":true,"sha256":model}}},
                "final":{"available":true,"cache_identity":"final", "source_sha256":{"verdant_voice_manifest":"abcd"},"files":{
                    "voice_source_admission":{"available":true,"sha256":"abcd"}}}
            }}})).unwrap();
        let voice=caps.voices.as_mut().unwrap().voices.get_mut("final").unwrap();
        voice.voice_package=Some(verdant_backend::voices::VoicePackage {
            name:"verdant-voice".into(),version:env!("CARGO_PKG_VERSION").into()});
        for (role,_) in verdant_backend::voices::PACKAGE_FILES {
            voice.files.insert((*role).into(),verdant_backend::wire::FileStatus {available:true,sha256:Some("abcd".into())});
        }
        caps
    }

    #[test]
    fn original_singer_choice_is_verified_atomic_and_clears_previous_language() {
        let mut project = Project::new_empty();
        project.tracks[0].unknown.insert("voice_language".into(), serde_json::json!({"language":"ja"}));
        let original = project.clone();
        let track = project.tracks[0].id.clone();
        let mut store = ProjectStore::new(project);
        let caps = caps();
        let batch = singer_choice_batch(store.project(), &track, DESKTOP_ORIGINAL_REFERENCE, &caps).unwrap();
        store.apply(store.revision(), batch).unwrap();
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(adapter::desktop_route(store.project(), &store.project().tracks[0], Some(&caps)), Route::C0);
        assert!(!store.project().tracks[0].unknown.contains_key("voice_language"));
        store.undo().unwrap();
        assert_eq!(store.project().tracks, original.tracks);
        let mut unverified = caps.clone();
        unverified.voices.as_mut().unwrap().voices.get_mut(DESKTOP_ORIGINAL_REFERENCE).unwrap().files.get_mut("token").unwrap().sha256 = Some("0".repeat(64));
        assert!(!original_usable(&unverified));
        assert!(singer_choice_batch(store.project(), &track, DESKTOP_ORIGINAL_REFERENCE, &unverified).is_err());
    }

    #[test]
    fn voice_choice_replaces_original_pin_and_rejects_unknown_singers() {
        let mut project = Project::new_empty();
        project.tracks[0].voice_ref = Some(DESKTOP_ORIGINAL_REFERENCE.into());
        project.tracks[0].render_route = Some("C0".into());
        let track = project.tracks[0].id.clone();
        let caps = caps();
        let mut store = ProjectStore::new(project);
        let batch = singer_choice_batch(store.project(), &track, "final", &caps).unwrap();
        store.apply(store.revision(), batch).unwrap();
        assert_eq!(store.project().tracks[0].voice_ref.as_deref(), Some("final"));
        assert_eq!(store.project().tracks[0].unknown["voice_selection"]["reference"], "final");
        assert!(singer_choice_batch(store.project(), &track, "vocoder", &caps).is_err());
    }

    #[test]
    fn canonical_singer_choice_preserves_the_explicit_versioned_pin() {
        use verdant_backend::voices::{DESKTOP_VOICE_REFERENCE,PACKAGE_FILES,VoicePackage};
        let mut caps=caps();let catalog=caps.voices.as_mut().unwrap();
        let mut voice=catalog.voices["final"].clone();voice.files.clear();
        for (role,_) in PACKAGE_FILES {
            voice.files.insert((*role).into(),verdant_backend::wire::FileStatus{available:true,sha256:Some("a".repeat(64))});
        }
        voice.files.insert("voice_source_admission".into(),verdant_backend::wire::FileStatus{available:true,sha256:Some("a".repeat(64))});
        voice.voice_package=Some(VoicePackage{name:"verdant-voice".into(),version:env!("CARGO_PKG_VERSION").into()});
        voice.source_sha256=voice.files.iter().map(|(role,status)|(role.clone(),status.sha256.clone().unwrap())).collect();
        catalog.voices.insert(DESKTOP_VOICE_REFERENCE.into(),voice);catalog.default_voice=Some(DESKTOP_VOICE_REFERENCE.into());
        let project=Project::new_empty();let track=project.tracks[0].id.clone();let mut store=ProjectStore::new(project);
        let batch=singer_choice_batch(store.project(),&track,DESKTOP_VOICE_REFERENCE,&caps).unwrap();
        store.apply(store.revision(),batch).unwrap();
        assert_eq!(store.project().tracks[0].voice_ref.as_deref(),Some(DESKTOP_VOICE_REFERENCE));
        assert_eq!(store.project().tracks[0].unknown["voice_selection"]["reference"],DESKTOP_VOICE_REFERENCE);
        let mut changed = caps.clone();
        let voice = changed.voices.as_mut().unwrap().voices.get_mut(DESKTOP_VOICE_REFERENCE).unwrap();
        let new_root = "b".repeat(64);
        voice.source_sha256.insert("verdant_voice_manifest".into(), new_root.clone());
        for role in ["verdant_voice_manifest", "voice_source_admission"] {
            voice.files.get_mut(role).unwrap().sha256 = Some(new_root.clone());
        }
        assert!(adapter::desktop_voice(store.project(), &store.project().tracks[0], &changed).is_err());
        caps.voices.as_mut().unwrap().voices.remove(DESKTOP_VOICE_REFERENCE);
        assert!(adapter::desktop_voice(store.project(), &store.project().tracks[0], &caps).is_err());
    }
}
