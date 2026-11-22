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
impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let mut actions = div().flex().flex_wrap().justify_end().gap(px(8.))
            .child(crate::ui_controls::selectable_button("import-preview-cancel", "取消", false, th)
                .on_click(|_, window, _| window.remove_window()));
        if count > 0 {
            actions = actions.child(crate::ui_controls::selectable_button("import-preview-accept",
                format!("导入转换预设（{count} 个）"), true, th)
                .on_click(cx.listener(|view, _, window, cx| {
                    let presets = view.items.iter().filter_map(|item| item.preset.clone()).collect();
                    let _ = view.library.update(cx, |library, cx| library.import_presets(presets, cx));
                    window.remove_window();
                })));
        }
        div().size_full().p(px(18.)).bg(th.surface).text_color(th.text)
            .font_family(crate::theme::system_ui_font(&cx.text_system().all_font_names()))
            .flex().flex_col().gap(px(12.))
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("检查预设参数映射"))
            .child(div().text_sm().text_color(th.text_secondary)
                .child("仅转换已验证的字段。未映射参数在载入时使用 Verdant 默认值，声音可能不同。导入后保存在预设库，选择“载入”才会修改当前插件。"))
            .child(rows).child(actions)
    }
}
