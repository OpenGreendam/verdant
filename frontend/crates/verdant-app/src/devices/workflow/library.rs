use super::*;
use super::text_field::TextField;
use gpui::{App, AppContext, Bounds, Render, WeakEntity, WindowBounds, WindowOptions, size};

mod import_preview;

#[derive(Clone)]
struct Row { path: PathBuf, preset: PresetFile }

pub(in crate::devices) fn open(editor: Entity<Editor>, target: Target, device: String, cx: &mut App) {
    let Some(params) = current_params(editor.read(cx), &target) else { return };
    let generation = editor.read(cx).document_generation;
    let th = editor.read(cx).theme;
    let weak = editor.downgrade();
    let bounds = Bounds::centered(None, size(px(760.), px(650.)), cx);
    if let Err(error) = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), window_min_size: Some(size(px(440.), px(400.))),
        app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title("插件预设库 — Verdant");
        cx.new(|cx| {
            let search = TextField::new("按预设名称与标签筛选", "", th, cx);
            let name = TextField::new("预设名称", "我的预设", th, cx);
            let tags = TextField::new("标签，用逗号分隔", "", th, cx);
            let focus = search.read(cx).focus.clone(); window.focus(&focus, cx);
            for field in [&search, &name, &tags] { cx.observe(field, |_, _, cx| cx.notify()).detach(); }
            let mut view = Library { editor: weak, target, generation, device, params, search, name, tags,
                rows: Vec::new(), selected: None, favorites_only: false, busy: false, message: String::new(), th };
            view.reload(cx);
            view
        })
    }) { editor.update(cx, |ed, cx| { ed.notice(format!("无法打开预设库：{error}")); cx.notify(); }); }
}

struct Library {
    editor: WeakEntity<Editor>, target: Target, generation: u64, device: String, params: Vec<ParamDesc>,
    search: Entity<TextField>, name: Entity<TextField>, tags: Entity<TextField>, rows: Vec<Row>,
    selected: Option<PathBuf>, favorites_only: bool, busy: bool, message: String, th: Theme,
}
impl Library {
    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.busy { return; } self.busy = true;
        let device = self.device.clone(); let params = self.params.clone();
        cx.spawn(async move |this, cx| {
            let (rows, skipped) = cx.background_spawn(async move {
                let mut rows = Vec::new(); let mut skipped = 0;
                if let Ok(entries) = std::fs::read_dir(preset_dir(&device)) {
                    for entry in entries.take(2048).flatten() {
                        let path = entry.path();
                        if path.extension().is_none_or(|e| e != "vpreset") { continue; }
                        match read_preset(&path, &device, &params) {
                            Ok(preset) => rows.push(Row { path, preset }), Err(_) => skipped += 1,
                        }
                    }
                }
                rows.sort_by(|a, b| a.preset.name.to_lowercase().cmp(&b.preset.name.to_lowercase()));
                (rows, skipped)
            }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false; view.rows = rows;
                if skipped > 0 { view.message = format!("已跳过 {skipped} 个无效或不兼容的预设文件"); }
                cx.notify();
            });
        }).detach();
    }
    fn select(&mut self, row: &Row, cx: &mut Context<Self>) {
        self.selected = Some(row.path.clone());
        self.name.update(cx, |field, cx| field.set(&row.preset.name, cx));
        self.tags.update(cx, |field, cx| field.set(&row.preset.tags.join(", "), cx));
        cx.notify();
    }
    fn apply(&mut self, row: &Row, cx: &mut Context<Self>) {
        let target = self.target.clone(); let generation = self.generation;
        let preset = row.preset.clone();
        let device = self.device.clone();
        let result = self.editor.update(cx, |ed, cx| -> Result<(), String> {
            if ed.document_generation != generation { return Err("工程已更换，请重新打开预设库".into()); }
            let params = current_params(ed, &target).ok_or("插件已重建或移除，请重新打开预设库")?;
            preset.validate(&device, &params)?;
            let lock = read_workspace(ed, &target).wet_lock;
            let values = preset_values(&params, &preset.parameters, lock)?;
            if ed.set_device_params(target.track, target.slot, values, cx) { Ok(()) }
            else { Err("正在处理其他修改，请稍后重试".into()) }
        });
        self.message = match result { Ok(Ok(())) => format!("已提交预设：{}", row.preset.name), Ok(Err(e)) => e, Err(_) => "工程已关闭".into() };
        self.select(row, cx);
    }
    fn save_current(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        if self.busy { return; }
        let Some(editor) = self.editor.upgrade() else { return };
        let ed = editor.read(cx);
        if ed.document_generation != self.generation || ed.plugin_busy.contains(&self.target.key) { self.message = "请等待当前参数提交完成".into(); cx.notify(); return; }
        let Some(params) = current_params(ed, &self.target) else { self.message = "插件已改变，请重新打开预设库".into(); cx.notify(); return };
        let mut preset = match PresetFile::new(&self.device, &self.name.read(cx).text.text, &self.tags.read(cx).text.text, &params) {
            Ok(p) => p, Err(e) => { self.message = e; cx.notify(); return; }
        };
        if overwrite { preset.favorite = self.rows.iter().find(|row| Some(&row.path) == self.selected.as_ref()).is_some_and(|row| row.preset.favorite); }
        let path = if overwrite { self.selected.clone().unwrap_or_else(|| fresh_path(&self.device)) } else { fresh_path(&self.device) };
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let saved_path = path.clone();
            let result = cx.background_spawn(async move { save_preset(&path, &preset) }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result { Ok(()) => { view.selected = Some(saved_path); view.message = "已保存当前参数与标签".into(); view.reload(cx); }, Err(e) => view.message = e }
                cx.notify();
            });
        }).detach();
    }
    fn import(&mut self, cx: &mut Context<Self>) {
        if self.busy { return; }
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions { files: true, directories: false, multiple: true,
            prompt: Some("导入预设".into()) });
        let device = self.device.clone(); let params = self.params.clone();
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let paths = match rx.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => { let _ = this.update(cx, |view, cx| { view.busy = false; cx.notify(); }); return; },
                result => {
                    let message = match result {
                        Ok(Err(error)) => format!("无法打开文件选择器：{error}"),
                        Err(error) => format!("文件选择器已关闭：{error}"),
                        _ => unreachable!(),
                    };
                    let _ = this.update(cx, |view, cx| { view.busy = false; view.message = message; cx.notify(); }); return;
                }
            };
            let result = cx.background_spawn(async move {
                paths.iter().map(|path| import_preview::inspect(path, &device, &params)).collect::<Result<Vec<_>, String>>()
            }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(imports) if imports.iter().any(|item| item.report.is_some()) =>
                        import_preview::open(cx.entity().downgrade(), imports, view.th, cx),
                    Ok(imports) => view.import_presets(imports.into_iter().filter_map(|item| item.preset).collect(), cx),
                    Err(error) => { view.message = error; cx.notify(); }
                }
            });
        }).detach();
    }
    fn import_presets(&mut self, presets: Vec<PresetFile>, cx: &mut Context<Self>) {
        if self.busy || presets.is_empty() { return; }
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move {
                for preset in &presets { save_preset(&fresh_path(&preset.device), preset)?; }
                Ok::<_, String>(presets.len())
            }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(count) => { view.message = format!("已导入 {count} 个预设，可选择后载入"); view.reload(cx); },
                    Err(error) => view.message = error,
                }
                cx.notify();
            });
        }).detach();
    }
    fn export(&mut self, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|r| Some(&r.path) == self.selected.as_ref()).cloned() else { self.message = "请先选择预设".into(); cx.notify(); return };
        let rx = cx.prompt_for_new_path(&PathBuf::from("."), Some("verdant-preset.vpreset"));
        cx.spawn(async move |this, cx| {
            let result = match rx.await {
                Ok(Ok(Some(path))) => cx.background_spawn(async move { save_preset(&path, &row.preset) }).await,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => Err(format!("无法打开保存对话框：{error}")),
                Err(error) => Err(format!("保存对话框已关闭：{error}")),
            };
            let _ = this.update(cx, |view, cx| { view.message = match result { Ok(()) => "已导出预设".into(), Err(e) => e }; cx.notify(); });
        }).detach();
    }
    fn favorite(&mut self, row: Row, cx: &mut Context<Self>) {
        if self.busy { return; }
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let favorite = !row.preset.favorite;
            let result = cx.background_spawn(async move {
                let mut preset = row.preset; preset.favorite = favorite;
                save_preset(&row.path, &preset)
            }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(()) => { view.message = if favorite { "已收藏预设" } else { "已取消收藏" }.into(); view.reload(cx); },
                    Err(error) => view.message = error,
                }
                cx.notify();
            });
        }).detach();
    }
    fn delete(&mut self, cx: &mut Context<Self>) {
        if self.busy { return; }
        let Some(path) = self.selected.clone() else { return };
        self.busy = true;
        let trash = preset_dir(&self.device).join(".trash").join(path.file_name().unwrap_or_default());
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move {
                std::fs::create_dir_all(trash.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::rename(path, trash).map_err(|e| e.to_string())
            }).await;
            let _ = this.update(cx, |view, cx| { view.busy = false;
                match result { Ok(()) => { view.selected = None; view.message = "预设已移到预设库回收目录".into(); view.reload(cx); }, Err(e) => view.message = e }
                cx.notify();
            });
        }).detach();
    }
}
impl Render for Library {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.th;
        let query = self.search.read(cx).text.text.clone();
        let mut list = div().id("preset-list").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(px(6.));
        let mut count = 0;
        for row in self.rows.iter().filter(|r| r.preset.matches(&query) && (!self.favorites_only || r.preset.favorite)) {
            count += 1;
            let selected = self.selected.as_ref() == Some(&row.path);
            let pick = row.clone(); let apply = row.clone(); let favorite = row.clone();
            list = list.child(div().id(SharedString::from(row.path.display().to_string())).role(Role::ListItem).rounded(px(8.))
                .bg(if selected { th.control_pressed } else { th.card }).p(px(10.)).flex().items_center().gap(px(10.))
                .child(div().id(format!("preset-select-{count}")).flex_1().min_w_0().cursor_pointer().on_click(cx.listener(move |view, _, _, cx| view.select(&pick, cx)))
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.preset.name.clone()))
                    .child(div().text_xs().text_color(th.text_secondary).child(row.preset.tags.join(" · "))))
                .child(crate::ui_controls::selectable_button(format!("favorite-preset-{count}"), if row.preset.favorite { "★" } else { "☆" }, row.preset.favorite, th)
                    .aria_label(if row.preset.favorite { "取消收藏预设" } else { "收藏预设" }).tab_index(0)
                    .on_click(cx.listener(move |view, _, _, cx| view.favorite(favorite.clone(), cx))))
                .child(crate::ui_controls::selectable_button(format!("load-preset-{count}"), "载入", false, th).tab_index(0)
                    .on_click(cx.listener(move |view, _, _, cx| view.apply(&apply, cx)))));
        }
        if count == 0 { list = list.child(div().p(px(18.)).text_sm().text_color(th.text_secondary).child("没有匹配的用户预设。调整音色后可在下方保存。")); }
        let button = |id, label| crate::ui_controls::selectable_button(id, label, false, th).tab_index(0);
        div().id("preset-library").role(Role::Group).aria_label("插件预设库").size_full().bg(th.surface).text_color(th.text)
            .font_family(crate::theme::system_ui_font(&cx.text_system().all_font_names())).p(px(18.)).flex().flex_col().gap(px(10.))
            .child(div().flex().items_center().gap(px(8.)).child(div().flex_1().text_lg().font_weight(FontWeight::BOLD).child("预设库"))
                .child(button("preset-import", "导入").on_click(cx.listener(|v, _, _, cx| v.import(cx))))
                .child(button("preset-export", "导出").on_click(cx.listener(|v, _, _, cx| v.export(cx))))
                .child(button("preset-close", "关闭").on_click(|_, w, _| w.remove_window())))
            .child(self.search.clone())
            .child(div().flex().items_center().gap(px(8.))
                .child(div().flex_1().text_xs().text_color(th.text_secondary).child(format!("{count} 个预设 · 按名称与标签搜索")))
                .child(crate::ui_controls::selectable_button("preset-favorites-only", "只看收藏", self.favorites_only, th).tab_index(0)
                    .on_click(cx.listener(|view, _, _, cx| { view.favorites_only = !view.favorites_only; cx.notify(); }))))
            .child(list)
            .child(div().border_t_1().border_color(th.shade).pt(px(12.)).flex().flex_col().gap(px(6.))
                .child(div().text_sm().child("名称")).child(self.name.clone())
                .child(div().text_sm().child("标签（逗号分隔）")).child(self.tags.clone())
                .child(div().flex().flex_wrap().items_center().gap(px(8.))
                    .child(button("preset-save-new", "另存当前参数").on_click(cx.listener(|v, _, _, cx| v.save_current(false, cx))))
                    .child(button("preset-update", "更新选中预设").on_click(cx.listener(|v, _, _, cx| v.save_current(true, cx))))
                    .child(button("preset-delete", "移到回收目录").on_click(cx.listener(|v, _, _, cx| v.delete(cx))))))
            .child(div().id("preset-library-status").role(Role::Status).text_xs().text_color(th.text_secondary).child(if self.busy { "正在读取或保存…".into() } else { self.message.clone() }))
    }
}
