//! Review foreign preset mappings before adding them to the user library.
use super::*;

pub(super) struct Imported {
    pub preset: Option<PresetFile>,
    pub report: Option<verdant_plugins::preset_interop::FabFilterImportReport>,
    name: String,
    parameters: Vec<(String, String)>,
}

pub(super) fn inspect(path: &Path, device: &str, params: &[ParamDesc]) -> Result<Imported, String> {
    if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ffp")) {
        let report = verdant_plugins::preset_interop::inspect_fabfilter_preset(&read_preset_bytes(path)?, device)?;
        let name: String = path.file_stem().and_then(|s| s.to_str()).unwrap_or("导入预设").chars().take(120).collect();
        let parameters = report.values.iter().map(|(id, value)| {
            let label = params.iter().find(|d| d.id == *id).map_or(id.as_str(), |d| d.name.as_str());
            let unit = params.iter().find(|d| d.id == *id).map_or("", |d| d.unit.as_str());
            (format!("{label} · {id}"), format!("{value:.4} {unit}"))
        }).collect();
        let preset = if report.values.is_empty() { None } else {
            let preset = PresetFile { format: "verdant-device-preset".into(), version: 1, device: device.into(),
                name: name.clone(), tags: vec!["FabFilter".into(), "Imported".into()], favorite: false,
                category: "Imported".into(), author: String::new(), description: String::new(), origin_factory_id: None,
                parameters: report.values.iter().cloned().collect() };
            preset.validate(device, params)?;
            Some(preset)
        };
        Ok(Imported { preset, report: Some(report), name, parameters })
    } else {
        let preset = read_preset(path, device, params)?;
        Ok(Imported { name: preset.name.clone(), preset: Some(preset), report: None, parameters: Vec::new() })
    }
}

pub(super) fn open(library: WeakEntity<Library>, items: Vec<Imported>, th: Theme, cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(800.), px(650.)), cx);
    let _ = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), window_min_size: Some(size(px(460.), px(360.))),
        app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title("预设参数映射 — Verdant");
        cx.new(|_| Preview { library, items, th })
    });
}

struct Preview { library: WeakEntity<Library>, items: Vec<Imported>, th: Theme }
impl Preview {
    fn body(&mut self, handle: &verdant_overlay_control::OverlayControlHandle, revision: &str, cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = self.th;
        let mut rows = crate::proto_scroll::viewport("preset-import-report", "预设参数映射报告", "vertical").flex_1().min_h_0()
            .flex().flex_col().gap(px(12.));
        for (index, item) in self.items.iter().enumerate() {
            let mut card = div().id(SharedString::from(format!("import-report-{index}"))).p(px(12.)).rounded(px(10.)).bg(th.card).flex().flex_col().gap(px(8.))
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(item.name.clone()));
            if let Some(report) = &item.report {
                card = card.child(div().text_xs().text_color(th.text_secondary)
                    .child(format!("{} · 格式 {} · 转换结果 {} 项参数", report.product, report.format_version, report.values.len())));
                for warning in &report.warnings {
                    card = card.child(div().text_sm().child(format!("转换说明：{warning}")));
                }
                for unsupported in &report.unsupported {
                    card = card.child(div().text_sm().child(format!("未映射：{unsupported}")));
                }
                if item.preset.is_none() {
                    card = card.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("没有已验证的参数映射，此文件不会导入。"));
                } else {
                    card = card.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("将导入以下参数："));
                    for (label, value) in &item.parameters {
                        card = card.child(div().flex().gap(px(10.)).text_xs()
                            .child(div().flex_1().min_w_0().child(label.clone())).child(value.clone()));
                    }
                }
            } else {
                card = card.child(div().text_sm().child("已验证的 Verdant 预设"));
            }
            rows = rows.child(card);
        }
        let count = self.items.iter().filter(|item| item.preset.is_some()).count();
        let cancel = handle.clone(); let accept = handle.clone();
        let cancel_revision = revision.to_string(); let accept_revision = revision.to_string();
        let mut actions = div().flex().flex_wrap().justify_end().gap(px(8.))
            .child(crate::ui_controls::selectable_button("import-preview-cancel", "取消", false, th)
                .authority(format!("{revision}:cancel"))
                .on_click(move |_, window, cx| crate::proto_overlay::choose(&cancel,0,&cancel_revision,window,cx)));
        if count > 0 {
            actions = actions.child(crate::ui_controls::selectable_button("import-preview-accept",
                format!("导入转换预设（{count} 个）"), true, th)
                .authority(format!("{revision}:accept"))
                .on_click(move |_, window, cx| crate::proto_overlay::choose(&accept,1,&accept_revision,window,cx)));
        }
        div().size_full().p(px(18.)).bg(th.surface).text_color(th.text)
            .font_family(crate::theme::system_ui_font(&cx.text_system().all_font_names()))
            .flex().flex_col().gap(px(12.))
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("检查预设参数映射"))
            .child(div().text_sm().text_color(th.text_secondary)
                .child("仅转换已验证的字段。未映射参数在载入时使用 Verdant 默认值，声音可能不同。导入后保存在预设库，选择“载入”才会修改当前插件。"))
            .child(rows).child(actions).into_any_element()
    }
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let revision = format!("preview:{:?}",cx.entity_id());
        let basis = revision.clone(); let entity = cx.entity();
        let draw: crate::proto_overlay::Draw = std::rc::Rc::new(move |handle,_,_,cx| {
            entity.update(cx,|view,cx|view.body(handle,&basis,cx))
        });
        let entity = cx.entity().downgrade();
        let response: crate::proto_overlay::Response = std::rc::Rc::new(move |index,window,cx| {
            let _ = entity.update(cx,|view,cx| {
                if index == 1 && view.items.iter().any(|item|item.preset.is_some()) {
                    let presets = view.items.iter().filter_map(|item|item.preset.clone()).collect();
                    let _ = view.library.update(cx,|library,cx|library.import_presets(presets,cx));
                }
                window.remove_window();
            });
        });
        let can_import = self.items.iter().any(|item|item.preset.is_some());
        crate::proto_overlay::overlay("compiled-preset-import-preview",serde_json::json!({
            "label":"预设参数映射","role":"dialog","open":true,"revision":revision,
            "choices":if can_import{2}else{1},"defaultChoice":if can_import{1}else{-1},"cancelChoice":0
        }),draw,response)
    }
}

#[cfg(test)]
mod native_preview_tests {
    use super::*;
    use gpui::{TestAppContext,Render};
    use verdant_overlay_control::{OverlayControlHandle,native::gpui_native_owner::{all_owners,resolved}};
    struct Harness { _editor:Entity<Editor>, _library:Entity<Library>, preview:Entity<Preview> }
    impl Render for Harness {fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{self.preview.clone()}}
    fn owner(name:&str,window:&Window)->verdant_overlay_control::native::gpui_native_owner::OwnerRef {
        all_owners().into_iter().find(|owner| {let owner=owner.borrow();owner.mounted && owner.window_id==Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|value|resolved(value).text()==name)}).unwrap()
    }
    #[gpui::test]
    fn native_import_preview_uses_overlay_cancel_policy_and_preserves_the_actual_report_viewport(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(800.),px(650.)),|window,cx| {
            let editor=cx.new(|cx|Editor::for_native_test(crate::native_test_support::instrument_project(),window,cx));
            let track=editor.read(cx).session.track.clone();let generation=editor.read(cx).document_generation;
            let key=crate::instrument::instrument_key_for_generation(&track,dsp::SYNTH_ID,generation);
            let target=Target{track:track.into(),slot:crate::instrument::INSTRUMENT_SLOT,key};
            let params=current_params(editor.read(cx),&target).unwrap();let th=editor.read(cx).theme;
            let font=crate::theme::plugin_ui_font(editor.read(cx).ui_font.clone());let weak=editor.downgrade();
            let library=cx.new(|cx|Library::create(weak,target,generation,dsp::SYNTH_ID.into(),params,Vec::new(),th,font,window,cx));
            let library_weak=library.downgrade();
            let preview=cx.new(|_|Preview{library:library_weak,th,items:(0..30).map(|index|Imported{preset:None,report:None,name:format!("Mapping {index}"),parameters:Vec::new()}).collect()});
            Harness{_editor:editor,_library:library,preview}
        });
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx| {
            let overlay=OverlayControlHandle{owner:owner("预设参数映射",window)};
            assert_eq!(overlay.state("scopeActive").unwrap(),serde_json::json!(true));
            let revision=overlay.owner.borrow().props["revision"].as_str().unwrap().to_string();
            assert_eq!(overlay.call("choose",vec![1.into(),revision.into()],window,cx).unwrap(),serde_json::json!(false),"no validated preset means no accept response");
            let report=owner("预设参数映射报告",window);
            assert!(report.borrow().parent.ptr_eq(&std::rc::Rc::downgrade(&overlay.owner)));
            assert!(report.borrow().modules.scroll.handle.as_ref().unwrap().max_offset().y>px(500.),"the original report has one physical overflowing viewport");
            let focus=overlay.owner.borrow().modules.focus.native.clone().unwrap();window.focus(&focus,cx);
        }).unwrap();
        cx.simulate_keystrokes(window.into(),"escape");cx.run_until_parked();
        assert!(cx.update_window(window.into(),|_,_,_|()).is_err(),"source cancel response closes the actual preview window");
    }
}
