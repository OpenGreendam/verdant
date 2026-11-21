//! Instrument tracks: a track whose notes play through an instrument instead
//! of a singing voice. The built-in synth renders in the shared playback and
//! export path (`verdant_service::project_audio::render_instrument`); its
//! settings live in the track's `instrument` spec and every edit is one undo
//! step. The device view shows the instrument as the first card of the
//! chain, before the track's effects.

use gpui::{Context, Entity, FontWeight, IntoElement, Role, SharedString, Window, div, prelude::*, px};
use verdant_domain::{Command, CommandBatch, TRACK_COLORS, Track, TrackId};
use verdant_plugins::builtin_dsp::{self as dsp, SynthParams, SYNTH_ID, SYNTH_PRESETS, SYNTH_SPECS};
use verdant_plugins::{InsertKey, ParamDesc};

use crate::editor::Editor;

/// Device-view slot standing for a track's instrument rather than an insert.
pub const INSTRUMENT_SLOT: usize = usize::MAX;

/// A stable key for a track's instrument card (drags and animations), kept
/// apart from live insert keys by its top bit.
pub fn instrument_key(track: &TrackId) -> InsertKey {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    track.as_str().hash(&mut h);
    InsertKey(h.finish() | 1 << 63)
}

pub fn synth_spec(params: &SynthParams, preset: &str) -> serde_json::Value {
    serde_json::json!({ "format": "builtin", "id": SYNTH_ID, "name": "Verdant Synth", "preset": preset, "params": params.to_json() })
}

/// The track's synth settings, if it is a built-in synth instrument track.
pub fn synth_params(track: &Track) -> Option<SynthParams> {
    let spec = track.instrument()?;
    (spec.get("id").and_then(serde_json::Value::as_str) == Some(SYNTH_ID)).then(|| SynthParams::from_json(spec.get("params")))
}

/// The synth settings as editor parameter descriptors.
pub fn synth_descs(params: &SynthParams) -> Vec<ParamDesc> { dsp::table_descs(SYNTH_SPECS, &params.values) }

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
            self.select_track(id, cx);
        }
    }

    fn commit_instrument(&mut self, track: &TrackId, spec: Option<serde_json::Value>, label: &str, cx: &mut Context<Self>) {
        self.commit(self.revision(), CommandBatch::single(label, Command::SetTrackInstrument { track: track.clone(), instrument: spec }), cx);
        cx.notify();
    }

    /// Write synth parameters as one undo step; invalid values are refused.
    pub fn set_instrument_params(&mut self, track: &TrackId, values: Vec<(String, f64)>, cx: &mut Context<Self>) {
        let Some(t) = self.project().track(track) else { return };
        let Some(mut params) = synth_params(t) else { return };
        let preset = t.instrument().and_then(|s| s.get("preset")).and_then(serde_json::Value::as_str).unwrap_or("").to_string();
        let before = params.clone();
        for (id, value) in values {
            let Some(spec) = SYNTH_SPECS.iter().find(|s| s.id == id) else { continue };
            let desc = &dsp::table_descs(std::slice::from_ref(spec), &[value])[0];
            match verdant_plugins::insert::parameter_editor_value(desc, value).map_err(|e| e.to_string()).and_then(|v| params.set(&id, v)) {
                Ok(()) => {}
                Err(error) => { self.notice(format!("乐器参数无效：{error}")); cx.notify(); return; }
            }
        }
        if params != before { self.commit_instrument(track, Some(synth_spec(&params, &preset)), "乐器参数", cx); }
    }

    pub fn set_instrument_preset(&mut self, track: &TrackId, preset: &str, cx: &mut Context<Self>) {
        if let Some(params) = SynthParams::preset(preset) {
            self.commit_instrument(track, Some(synth_spec(&params, preset)), &format!("乐器音色：{preset}"), cx);
        }
    }

    pub fn make_instrument_track(&mut self, track: &TrackId, instrument: bool, cx: &mut Context<Self>) {
        let spec = instrument.then(|| { let (p, _) = SYNTH_PRESETS[0]; synth_spec(&SynthParams::preset(p).unwrap_or_default(), p) });
        self.commit_instrument(track, spec, if instrument { "转为乐器轨" } else { "转为人声轨" }, cx);
    }
}

/// The instrument card at the head of an instrument track's device chain.
pub fn instrument_card(ed: &mut Editor, editor: &Entity<Editor>, track: &TrackId, window: &mut Window, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let th = ed.theme;
    let t = ed.project().track(track)?;
    let spec = t.instrument()?.clone();
    let name = spec.get("name").and_then(serde_json::Value::as_str).unwrap_or("乐器").to_string();
    let Some(params) = synth_params(t) else {
        return Some(div().flex_none().w(px(320.0)).p(px(14.0)).rounded(px(12.0)).bg(th.card).flex().flex_col().gap(px(8.0))
            .child(div().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(div().text_sm().text_color(th.text_secondary).child("此版本还不能让这个乐器发声；设置已保留。"))
            .into_any_element());
    };
    let preset = spec.get("preset").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
    let key = instrument_key(track);
    let mut descs = synth_descs(&params);
    crate::devices::preview_and_animate(ed, key, &mut descs, window);
    let preset_track = track.clone();
    let preset_label = if preset.is_empty() { "自定义".to_string() } else { preset.clone() };
    let presets = crate::devices::dropdown_button(SharedString::from("synth-preset"), preset_label, th, move |_| {
        use crate::editor::MenuAction as M;
        use crate::menu::MenuItem as I;
        let mut items = vec![I::header("音色")];
        items.extend(SYNTH_PRESETS.iter().map(|(name, _)| I::action(*name, M::SetInstrumentPreset { track: preset_track.clone(), preset: (*name).into() })));
        items
    }, cx);
    let body = crate::devices::synth_editor(ed, editor, track, key, &descs, cx);
    Some(div().id("instrument-card").role(Role::Group).aria_label(format!("乐器：{name}"))
        .flex_none().w(px(640.0)).flex().flex_col().gap(px(10.0)).p(px(14.0)).rounded(px(12.0)).bg(th.card).shadow_sm()
        .border_1().border_color(crate::theme::with_alpha(th.accent, 0.35))
        .child(div().flex().items_center().gap(px(10.0)).h(px(32.0))
            .child(gpui::svg().path(crate::assets::icon("instrument")).size(px(18.0)).text_color(th.accent))
            .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).child(name))
            .child(presets))
        .child(body)
        .into_any_element())
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
        assert_eq!(synth_descs(&params).len(), SYNTH_SPECS.len());
        let key = instrument_key(&track.id);
        assert!(key.0 >> 63 == 1, "instrument keys never collide with sequential insert keys");
        assert_eq!(key, instrument_key(&track.id));
    }
}
