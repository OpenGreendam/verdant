//! Instrument tracks: a track whose notes play through an instrument instead
//! of a singing voice. The built-in synth renders in the shared playback and
//! export path (`verdant_service::project_audio::render_instrument`); its
//! settings live in the track's `instrument` spec and every edit is one undo
//! step. The device view shows the instrument as the first card of the
//! chain, before the track's effects.

use gpui::{Context, Entity, FontWeight, IntoElement, Role, SharedString, Window, div, prelude::*, px};
use verdant_domain::{Command, CommandBatch, TRACK_COLORS, Track, TrackId};
use verdant_plugins::builtin_dsp::{self as dsp, SynthParams, SYNTH_ID, ONE_SYNTH_ID, SYNTH_PRESETS, ONE_PRESETS};
use verdant_plugins::{InsertKey, ParamDesc};

use crate::editor::Editor;

/// Device-view slot standing for a track's instrument rather than an insert.
pub const INSTRUMENT_SLOT: usize = usize::MAX;

/// A stable key for a track's instrument card (drags and animations), kept
/// apart from live insert keys by its top bit.
#[cfg(test)]
pub fn instrument_key(track: &TrackId) -> InsertKey {
    instrument_key_for(track, SYNTH_ID)
}

#[cfg(test)]
pub fn instrument_key_for(track: &TrackId, id: &str) -> InsertKey {
    instrument_key_for_generation(track, id, 0)
}

pub fn instrument_key_for_generation(track: &TrackId, id: &str, generation: u64) -> InsertKey {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    track.as_str().hash(&mut h);
    id.hash(&mut h);
    generation.hash(&mut h);
    InsertKey(h.finish() | 1 << 63)
}

pub fn synth_spec(params: &SynthParams, preset: &str) -> serde_json::Value {
    builtin_instrument_spec(SYNTH_ID, params, preset)
}

pub fn builtin_instrument_spec(id: &str, params: &SynthParams, preset: &str) -> serde_json::Value {
    serde_json::json!({ "format": "builtin", "id": id, "name": dsp::instrument_name(id).unwrap_or("Verdant Synth"), "preset": preset, "params": params.to_json() })
}

/// The track's synth settings, if it is a built-in synth instrument track.
pub fn synth_params(track: &Track) -> Option<SynthParams> {
    let spec = track.instrument()?;
    let id = spec.get("id").and_then(serde_json::Value::as_str)?;
    (spec.get("format").and_then(serde_json::Value::as_str) == Some("builtin") && dsp::is_builtin_instrument(id))
        .then(|| SynthParams::from_json_for(id, spec.get("params")))
}

/// Playback admission is separate from editor admission: a bypassed native
/// instrument remains fully editable and keeps its saved parameters.
pub(crate) fn synth_monitor_enabled(track:&Track)->bool{
    track.instrument().is_some_and(|spec|
        spec.get("bypass").and_then(serde_json::Value::as_bool)!=Some(true)
        &&spec.get("format").and_then(serde_json::Value::as_str)==Some("builtin")
        &&spec.get("id").and_then(serde_json::Value::as_str).is_some_and(dsp::is_builtin_instrument))
}

/// The synth settings as editor parameter descriptors.
#[cfg(test)]
pub fn synth_descs(params: &SynthParams) -> Vec<ParamDesc> { synth_descs_for(SYNTH_ID, params) }

pub fn synth_descs_for(id: &str, params: &SynthParams) -> Vec<ParamDesc> {
    let specs = dsp::instrument_specs(id);
    let values: Vec<_> = specs.iter().map(|spec| params.get(spec.id)).collect();
    dsp::table_descs(&specs, &values)
}

impl Editor {
    pub fn add_instrument_track(&mut self, cx: &mut Context<Self>) {
        let n = self.project().tracks.len();
        if n >= verdant_audio::MAX_TRACKS {
            self.notice(format!("最多 {} 条轨道", verdant_audio::MAX_TRACKS));
            return;
        }
        let color = TRACK_COLORS[n % TRACK_COLORS.len()];
        let mut track = Track::new(format!("乐器 {}", n + 1), color);
        let (preset, _) = SYNTH_PRESETS[0];
        let params = SynthParams::preset(preset).unwrap_or_default();
        track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(), synth_spec(&params, preset));
        let id = track.id.clone();
        let index = self.track_index(&self.session.track).map(|i| i + 1).unwrap_or(n);
        if self.commit(self.revision(), CommandBatch::single("新建乐器轨", Command::AddTrack { index, track }), cx) {
            self.sync_mixer();
            self.select_track(id.clone(), cx);
            // The instrument edits inline as the first card of the device chain.
            self.bottom = crate::editor::Bottom::Devices;
        }
    }

    fn commit_instrument(&mut self, track: &TrackId, spec: Option<serde_json::Value>, label: &str, cx: &mut Context<Self>) -> bool {
        let committed = self.commit(self.revision(), CommandBatch::single(label, Command::SetTrackInstrument { track: track.clone(), instrument: spec }), cx);
        cx.notify();
        committed
    }

    /// Write synth parameters as one undo step; invalid values are refused.
    pub fn set_instrument_params(&mut self, track: &TrackId, values: Vec<(String, f64)>, cx: &mut Context<Self>) -> bool {
        let Some(t) = self.project().track(track) else { return false };
        let Some(mut params) = synth_params(t) else { return false };
        let mut saved = t.instrument().cloned().expect("checked instrument");
        let id = saved.get("id").and_then(serde_json::Value::as_str).unwrap_or(SYNTH_ID).to_string();
        let specs = dsp::instrument_specs(&id);
        let before = params.clone();
        for (id, value) in values {
            let Some(spec) = specs.iter().find(|s| s.id == id) else { return false };
            let desc = &dsp::table_descs(std::slice::from_ref(spec), &[value])[0];
            match verdant_plugins::insert::parameter_editor_value(desc, value).map_err(|e| e.to_string()).and_then(|v| params.set(&id, v)) {
                Ok(()) => {}
                Err(error) => { self.notice(format!("乐器参数无效：{error}")); cx.notify(); return false; }
            }
        }
        if params != before {
            saved["params"] = params.to_json();
            saved["preset"] = "".into();
            return self.commit_instrument(track, Some(saved), "乐器参数", cx);
        }
        true
    }

    pub fn set_instrument_preset(&mut self, track: &TrackId, preset: &str, cx: &mut Context<Self>) {
        let Some(mut spec) = self.project().track(track).and_then(|track| track.instrument().cloned()) else { return };
        let id = spec.get("id").and_then(serde_json::Value::as_str).unwrap_or(SYNTH_ID);
        if let Some(params) = SynthParams::preset_for(id, preset) {
            spec["params"] = params.to_json();
            spec["preset"] = preset.into();
            self.commit_instrument(track, Some(spec), &format!("乐器音色：{preset}"), cx);
        }
    }

    /// Play the track through a scanned third-party instrument.
    pub fn set_hosted_instrument(&mut self, track: &TrackId, format: &str, id: &str, cx: &mut Context<Self>) {
        let Some(desc) = self.plugin_catalog().into_iter().find(|d| d.format.tag() == format && d.id == id) else {
            self.notice("乐器插件已不在插件列表中；请重新扫描"); cx.notify(); return;
        };
        let spec = serde_json::json!({ "format": desc.format.tag(), "id": desc.id, "name": desc.name, "version": desc.version, "path": desc.path });
        self.commit_instrument(track, Some(spec), &format!("乐器：{}", desc.name), cx);
    }

    pub fn use_builtin_synth(&mut self, track: &TrackId, cx: &mut Context<Self>) {
        let (p, _) = SYNTH_PRESETS[0];
        if self.commit_instrument(track, Some(synth_spec(&SynthParams::preset(p).unwrap_or_default(), p)), "乐器：内置合成器", cx) {
            // The instrument edits inline as the first card of the device chain.
            self.bottom = crate::editor::Bottom::Devices;
        }
    }

    pub fn use_builtin_one(&mut self, track: &TrackId, cx: &mut Context<Self>) {
        if self.commit_instrument(track, Some(builtin_instrument_spec(ONE_SYNTH_ID, &SynthParams::one_defaults(), "")), "乐器：Verdant One", cx) {
            // The instrument edits inline as the first card of the device chain.
            self.bottom = crate::editor::Bottom::Devices;
        }
    }

    pub fn make_instrument_track(&mut self, track: &TrackId, instrument: bool, cx: &mut Context<Self>) {
        let spec = instrument.then(|| { let (p, _) = SYNTH_PRESETS[0]; synth_spec(&SynthParams::preset(p).unwrap_or_default(), p) });
        if self.commit_instrument(track, spec, if instrument { "转为乐器轨" } else { "转为人声轨" }, cx) && instrument {
            // The instrument edits inline as the first card of the device chain.
            self.bottom = crate::editor::Bottom::Devices;
        }
    }
}

pub(crate) fn preset_button(ed: &Editor, track: &TrackId, id: &str, key: InsertKey, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let available = if id == ONE_SYNTH_ID { ONE_PRESETS } else { SYNTH_PRESETS };
    let preset = ed.project().track(track).and_then(|track| track.instrument())
        .and_then(|spec| spec.get("preset")).and_then(serde_json::Value::as_str).unwrap_or("");
    let label = if preset.is_empty() { "自定义" } else { preset }.to_string();
    let track = track.clone();
    crate::devices::dropdown_button(SharedString::from("synth-preset"), label, ed.theme, crate::proto_controls::editor_authority(ed), move |_| {
        use crate::editor::MenuAction as M;
        use crate::menu::MenuItem as I;
        let mut items = vec![I::header("音色")];
        items.extend(available.iter().map(|(name, _)| I::action(*name, M::SetInstrumentPreset { track: track.clone(), expected_key: key, preset: (*name).into() })));
        items
    }, cx).into_any_element()
}

/// The instrument card at the head of an instrument track's device chain.
pub fn instrument_card(ed: &mut Editor, editor: &Entity<Editor>, track: &TrackId, window: &mut Window, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let th = ed.theme;
    let t = ed.project().track(track)?;
    let spec = t.instrument()?.clone();
    let name = spec.get("name").and_then(serde_json::Value::as_str).unwrap_or("乐器").to_string();
    let chooser_track = track.clone();
    let chooser = crate::devices::dropdown_button(SharedString::from("instrument-choose"), "换乐器".into(), th, crate::proto_controls::editor_authority(ed), move |ed| instrument_menu(ed, &chooser_track), cx);
    let Some(params) = synth_params(t) else {
        let format = spec.get("format").and_then(serde_json::Value::as_str).unwrap_or("").to_uppercase();
        return Some(div().id("instrument-card").role(Role::Group).aria_label(format!("乐器：{name}"))
            .flex_none().w(px(360.0)).p(px(14.0)).rounded(px(12.0)).bg(th.card).shadow_sm().flex().flex_col().gap(px(10.0))
            .border_1().border_color(crate::theme::with_alpha(th.accent, 0.35))
            .child(div().flex().items_center().gap(px(10.0)).h(px(32.0))
                .child(gpui::svg().path(crate::assets::icon("instrument")).size(px(18.0)).text_color(th.accent))
                .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).child(name))
                .child(chooser))
            .child(div().text_sm().text_color(th.text_secondary).child(format!("{format} 乐器插件 · 音符在后台离线渲染，改动后自动更新")))
            .into_any_element());
    };
    let id = spec.get("id").and_then(serde_json::Value::as_str).unwrap_or(SYNTH_ID);
    let key = instrument_key_for_generation(track, id, ed.document_generation);
    let mut descs = synth_descs_for(id, &params);
    crate::devices::preview_and_animate(ed, key, &mut descs, window);
    let presets = preset_button(ed, track, id, key, cx);
    let body = crate::devices::synth_editor(ed, editor, track, key, &descs, cx);
    let open_track = track.clone();
    Some(div().id("instrument-card").role(Role::Group).aria_label(format!("乐器：{name}"))
        .flex_none().w(px(640.0)).flex().flex_col().gap(px(10.0)).p(px(14.0)).rounded(px(12.0)).bg(th.card).shadow_sm()
        .border_1().border_color(crate::theme::with_alpha(th.accent, 0.35))
        .child(div().flex().items_center().gap(px(10.0)).h(px(32.0))
            .child(gpui::svg().path(crate::assets::icon("instrument")).size(px(18.0)).text_color(th.accent))
            .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(crate::ui_controls::selectable_button("instrument-open", "打开界面", false, th)
                .on_click(cx.listener(move |ed, _, _, cx| ed.open_builtin_instrument_editor(&open_track, cx))))
            .child(presets).child(chooser))
        .child(body)
        .into_any_element())
}

/// Built-in synth plus every scanned instrument plug-in.
fn instrument_menu(ed: &Editor, track: &TrackId) -> Vec<crate::menu::MenuItem> {
    use crate::editor::MenuAction as M;
    use crate::menu::MenuItem as I;
    let current = ed.project().track(track).and_then(|t| t.instrument().cloned());
    let is = |format: &str, id: &str| current.as_ref().is_some_and(|s| s.get("format").and_then(serde_json::Value::as_str) == Some(format) && s.get("id").and_then(serde_json::Value::as_str) == Some(id));
    let mut items = vec![I::header("乐器"),
        I::action("Verdant Synth（内置）", M::UseBuiltinSynth { track: track.clone() }).checked(is("builtin", SYNTH_ID)),
        I::action("Verdant One（内置）", M::UseBuiltinOne { track: track.clone() }).checked(is("builtin", ONE_SYNTH_ID))];
    let hosted: Vec<_> = ed.plugin_catalog().into_iter().filter(|d| d.is_instrument()).collect();
    if hosted.is_empty() {
        items.push(I::disabled("未找到乐器插件（CLAP / VST3）"));
    } else {
        items.push(I::separator());
        for d in hosted {
            let label = format!("{} · {}", d.name, d.format.tag().to_uppercase());
            items.push(I::action(label, M::SetHostedInstrument { track: track.clone(), format: d.format.tag().into(), id: d.id.clone() }).checked(is(d.format.tag(), &d.id)));
        }
    }
    items.push(I::separator());
    items.push(I::action("重新扫描插件", M::RescanPlugins));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrument_spec_round_trips_and_keys_stay_apart_from_inserts() {
        let params = SynthParams::preset("贝斯").unwrap();
        let mut track = Track::new("乐器 1", TRACK_COLORS[0]);
        assert!(synth_params(&track).is_none());
        track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(), synth_spec(&params, "贝斯"));
        assert_eq!(synth_params(&track), Some(params.clone()));
        assert_eq!(synth_descs(&params).len(), dsp::SYNTH_SPECS.len()+dsp::MOD_SPECS.len());
        let key = instrument_key(&track.id);
        assert!(key.0 >> 63 == 1, "instrument keys never collide with sequential insert keys");
        assert_eq!(key, instrument_key(&track.id));
        assert_ne!(key, instrument_key_for(&track.id, ONE_SYNTH_ID), "replacing the instrument invalidates old edit targets");
        assert_ne!(key, instrument_key_for_generation(&track.id, SYNTH_ID, 1), "reopening a document invalidates old instrument controls");
    }

    #[test]
    fn one_spec_exposes_only_one_parameters_and_preserves_its_defaults() {
        let mut track = Track::new("One", TRACK_COLORS[0]);
        let params = SynthParams::one_defaults();
        track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(), builtin_instrument_spec(ONE_SYNTH_ID, &params, ""));
        assert_eq!(synth_params(&track), Some(params.clone()));
        let descs = synth_descs_for(ONE_SYNTH_ID, &params);
        assert_eq!(descs.len(), dsp::ONE_PARAM_IDS.len()+dsp::MOD_SPECS.len());
        assert!(descs.iter().all(|desc| dsp::ONE_PARAM_IDS.contains(&desc.id.as_str())||dsp::modulation_parameter(&desc.id).is_some()));
        assert!(!descs.iter().any(|desc|matches!(desc.id.as_str(),"osc2"|"arp_on"|"mpe"|"filter2_enabled")));
        assert!(descs.iter().all(|desc| desc.value >= desc.min && desc.value <= desc.max));
    }
}
