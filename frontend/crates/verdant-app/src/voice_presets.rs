//! Track singer choices retain admitted source pins. Language choices select
//! the ordinary per-track lyric frontend without rewriting explicit readings.

use gpui::{AccessibleAction, Context, Div, div, prelude::*, px};
use verdant_backend::wire::Capabilities;
use verdant_domain::{Command, CommandBatch, Project, TrackId};
use crate::editor::Editor;

pub fn language_choice_batch(project: &Project, track_id: &TrackId, language: Option<&str>, caps: &Capabilities)
    -> Result<CommandBatch, String> {
    let track = project.track(track_id).ok_or("轨道不存在")?;
    let value = if let Some(language) = language {
        if !matches!(language, "zh" | "en" | "ja") { return Err("当前编辑器没有该语言的读音接口".into()); }
        let (_, voice) = verdant_backend::adapter::desktop_voice(project, track, caps)?.ok_or("请先选择已验证的本地歌手")?;
        let support = voice.languages.get(language).ok_or("所选歌手未声明该语言")?;
        if !matches!(support.as_str(), "native" | "approximate") { return Err("歌手语言声明无效".into()); }
        Some(serde_json::json!({"schema":"verdant-track-language/1", "language":language, "support":support}))
    } else { None };
    Ok(CommandBatch::single("选择轨道语言", Command::SetTrackData { track: track_id.clone(), key: "voice_language".into(), value }))
}

pub fn language_label(language: &str, support: &str) -> String {
    let name = match language { "zh" => "中文", "en" => "英语（需填写读音）", "ja" => "日语", _ => language };
    format!("{name}{}", if support == "approximate" { " · 跨语言近似" } else { "" })
}

impl Editor {
    pub fn singer_menu_items(&self) -> Result<Vec<crate::menu::MenuItem>, String> {
        use crate::editor::integrations::IntegrationAction;
        use crate::editor::{MenuAction, WorkflowAction};
        let caps = self.scheduler.capabilities.as_ref().ok_or("等待后端报告已安装歌手")?;
        let mut items = Vec::new();
        if let Some(catalog) = &caps.voices {
            for (reference, voice) in &catalog.voices {
                if !verdant_backend::adapter::is_desktop_voice(voice)
                    && !(reference == verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE
                        && crate::desktop_engine::original_usable(caps)) { continue; }
                let declared = voice.languages.iter().map(|(language, support)| language_label(language, support)).collect::<Vec<_>>().join(" / ");
                let selected = self.active_track().and_then(|track| verdant_backend::adapter::desktop_voice(self.project(), track, caps).ok().flatten())
                    .is_some_and(|(effective, _)| effective == reference);
                let label = format!("{} · {reference} · {}", voice.display_name,
                    if declared.is_empty() { "语言未声明" } else { &declared });
                let command = MenuAction::Workflow(WorkflowAction::Integration(IntegrationAction::ChooseSinger(reference.clone())));
                items.push(if voice.available { crate::menu::MenuItem::action(label, command).checked(selected) }
                    else { crate::menu::MenuItem::disabled(format!("{label} · 不可用")) });
            }
            if items.is_empty() { items.push(crate::menu::MenuItem::disabled("尚未安装本地歌手")); }
        } else { items.push(crate::menu::MenuItem::disabled("引擎仅配置了启动时的默认歌手")); }
        items.push(crate::menu::MenuItem::separator());
        items.push(crate::menu::MenuItem::action("导入 verdant-voice 声库…", MenuAction::Workflow(WorkflowAction::Integration(IntegrationAction::InstallVoice))));
        Ok(items)
    }

    /// Track & Voice panel `<Select>` dropdowns for Singer and Singing Language.
    pub fn singer_language_controls(&self, cx: &mut Context<Self>) -> Div {
        let th = self.theme;
        let track_id = self.session.track.clone();
        let mut controls = div().flex().flex_col();
        let Some(track) = self.project().track(&track_id) else { return controls; };
        let selected = self.scheduler.capabilities.as_ref().and_then(|caps| verdant_backend::adapter::desktop_voice(self.project(), track, caps).ok().flatten());
        let singer_name = selected
            .map(|(_, voice)| voice.display_name.clone())
            .unwrap_or_else(|| "选择已校验的桌面声库…".into());
        let open_singers = |ed: &mut Editor, window: &mut gpui::Window, cx: &mut Context<Editor>| {
            if let Err(error) = ed.run_integration(crate::editor::integrations::IntegrationAction::Singers, window, cx) {
                ed.notice(error);
                cx.notify();
            }
        };
        controls = controls.child(
            crate::ui_controls::combo_row("track-singer-picker", Some("mic-vocal"), singer_name, "选择轨道歌手", th)
                .authority(crate::proto_controls::editor_authority(self))
                .on_click(cx.listener(move |ed, _, window, cx| open_singers(ed, window, cx)))
                .on_a11y_action(AccessibleAction::Click, crate::view::a11y_listener(cx, move |ed, _, window, cx| open_singers(ed, window, cx))),
        );

        let current_language = track.unknown.get("voice_language").and_then(|value| value["language"].as_str()).map(str::to_string);
        let declared_languages: Vec<(String, String)> = selected
            .map(|(_, voice)| voice.languages.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let lang_display = match &current_language {
            None => "语言 · 跟随读音器".to_string(),
            Some(code) => {
                let sup = declared_languages.iter().find(|(k, _)| k == code).map(|(_, s)| s.as_str()).unwrap_or("native");
                format!("语言 · {}", language_label(code, sup))
            }
        };
        let segmented = verdant_backend::phonemizer::settings_from_project(self.project())
            .is_ok_and(|settings| settings.segments.values().any(|entry| entry.track == track_id));
        let (tid_click, langs_click, cur_click) = (track_id.clone(), declared_languages.clone(), current_language.clone());
        let open_lang_menu = move |ed: &mut Editor, anchor: gpui::Point<gpui::Pixels>, window: &mut gpui::Window, cx: &mut Context<Editor>| {
            let mut items = vec![crate::menu::MenuItem::action(
                "跟随读音器（默认）",
                crate::editor::MenuAction::SetTrackLanguage { track: tid_click.clone(), language: None },
            ).checked(cur_click.is_none())];
            if !langs_click.is_empty() {
                items.push(crate::menu::MenuItem::separator());
                for (code, sup) in &langs_click {
                    let enabled = matches!(code.as_str(), "zh" | "en" | "ja");
                    let sel = cur_click.as_deref() == Some(code.as_str());
                    let lbl = language_label(code, sup);
                    items.push(
                        crate::menu::MenuItem::action(
                            lbl,
                            crate::editor::MenuAction::SetTrackLanguage {
                                track: tid_click.clone(),
                                language: Some(code.clone()),
                            },
                        )
                        .enabled(enabled)
                        .checked(sel),
                    );
                }
            } else {
                items.push(crate::menu::MenuItem::separator());
                items.push(crate::menu::MenuItem::disabled("先选择已声明多语言的本地歌手以切换轨道语言"));
            }
            ed.show_menu(anchor, items, window, cx);
        };
        let open_lang_a11y = open_lang_menu.clone();
        controls = controls.child(
            crate::ui_controls::combo_row("track-language-picker", Some("globe"), lang_display, "选择演唱语言", th).border_t_1().border_color(th.shade)
                .authority(crate::proto_controls::editor_authority(self))
                .on_click(cx.listener(move |ed, ev: &crate::proto_controls::Activation, window, cx| open_lang_menu(ed, ev.position(), window, cx)))
                .on_a11y_action(AccessibleAction::Click, crate::view::a11y_listener(cx, move |ed, _, window, cx| open_lang_a11y(ed, window.mouse_position(), window, cx))),
        );
        controls.when(segmented, |c| {
            c.child(div().px(px(12.0)).pb(px(10.0)).text_size(px(10.5)).text_color(th.text_secondary).child("本轨已配置多语言时间分段，分段内音符优先使用分段读音器"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use verdant_backend::{voices::PACKAGE_FILES, wire::FileStatus};
    use verdant_domain::{ProjectStore, Track};

    #[test]
    fn language_selection_is_bank_declared_track_scoped_and_one_undo_step() {
        let mut project = Project::new_empty();
        project.tracks[0].voice_ref = Some("local".into());
        project.tracks.push(Track::new("Other", "blue"));
        let track = project.tracks[0].id.clone();
        let mut caps: Capabilities = serde_json::from_value(json!({"voices":{"version":1,"voices":{
            "local":{"display_name":"Local", "available":true, "cache_identity":"admitted-source",
                "voice_package":{"name":"verdant-voice","version":"0.1.0"},
                "source_sha256":{"token":"a".repeat(64),"model":"b".repeat(64)},
                "languages":{"zh":"native","ja":"approximate"}}
        }}})).unwrap();
        let files = &mut caps.voices.as_mut().unwrap().voices.get_mut("local").unwrap().files;
        let manifest = FileStatus { available: true, sha256: Some("c".repeat(64)) };
        for (role, _) in PACKAGE_FILES {
            files.insert((*role).into(), manifest.clone());
        }
        files.insert("voice_source_admission".into(), manifest);
        let before = project.clone();
        let mut store = ProjectStore::new(project);
        let batch = language_choice_batch(store.project(), &track, Some("ja"), &caps).unwrap();
        store.apply(store.revision(), batch).unwrap().unwrap();
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(store.project().tracks[0].unknown["voice_language"]["support"], "approximate");
        assert_eq!(store.project().tracks[1], before.tracks[1]);
        assert_eq!(store.project().tracks[0].notes, before.tracks[0].notes);
        assert!(language_choice_batch(store.project(), &track, Some("en"), &caps).is_err());
        assert!(language_choice_batch(store.project(), &track, Some("de"), &caps).is_err());
        assert!(language_label("ja", "approximate").contains("近似"));
        store.undo().unwrap();
        assert!(!store.project().tracks[0].unknown.contains_key("voice_language"));
        store.redo().unwrap();
        let clear = language_choice_batch(store.project(), &track, None, &caps).unwrap();
        store.apply(store.revision(), clear).unwrap().unwrap();
        assert!(!store.project().tracks[0].unknown.contains_key("voice_language"));
    }

}
