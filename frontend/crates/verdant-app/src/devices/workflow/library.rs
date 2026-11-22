use super::*;
use super::text_field::TextField;
use gpui::{App, AppContext, Bounds, Render, WeakEntity, WindowBounds, WindowOptions, size};

mod import_preview;

#[derive(Clone)]
struct Row { id: String, path: Option<PathBuf>, factory_id: Option<String>, preset: PresetFile }

fn factory_rows(device: &str, favorites: &[String]) -> Vec<Row> {
    verdant_plugins::factory_presets::factory_presets(device).into_iter().map(|p| {
        let favorite = favorites.contains(&p.id);
        Row { id: p.id.clone(), path: None, factory_id: Some(p.id.clone()),
            preset: PresetFile { format: "verdant-device-preset".into(), version: 1, device: p.device, name: p.name,
                category: p.category, author: p.author, description: p.description, tags: p.tags, favorite,
                origin_factory_id: Some(p.id), parameters: p.parameters } }
    }).collect()
}

pub(in crate::devices) fn open(editor: Entity<Editor>, target: Target, device: String, cx: &mut App) {
    let Some(params) = current_params(editor.read(cx), &target) else { return };
    let generation = editor.read(cx).document_generation;
    let th = editor.read(cx).theme;
    let font = crate::theme::plugin_ui_font(editor.read(cx).ui_font.clone());
    let favorites: Vec<String> = editor.read(cx).prefs.extra.get("plugin_factory_favorites")
        .and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let factory = factory_rows(&device, &favorites);
    let weak = editor.downgrade();
    let bounds = Bounds::centered(None, size(px(900.), px(710.)), cx);
    if let Err(error) = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), window_min_size: Some(size(px(640.), px(480.))),
        app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title("插件预设库 — Verdant");
        cx.new(|cx| {
            let search = TextField::new("搜索名称、类别、作者、标签与用途", "", th, window, cx);
            let name = TextField::new("预设名称", "我的预设", th, window, cx);
            let tags = TextField::new("标签，用逗号分隔", "", th, window, cx);
            let category = TextField::new("类别", "", th, window, cx);
            let author = TextField::new("作者", "", th, window, cx);
            let description = TextField::new("用途与说明", "", th, window, cx);
            let focus = search.read(cx).focus.clone(); window.focus(&focus, cx);
            for field in [&search, &name, &tags, &category, &author, &description] { cx.observe(field, |_, _, cx| cx.notify()).detach(); }
            let scroll = gpui::ScrollHandle::new();
            let native_scroll = crate::proto_scroll::NativeScroll::new(scroll.clone());
            let mut view = Library { editor: weak, target, generation, device, params, search, name, tags, category, author, description,
                rows: factory.clone(), factory, selected: None, loaded: None, last_applied: None, original: None,
                browser_focus: cx.focus_handle(), collection: None, scroll, native_scroll,
                category_filter: String::new(), source_filter: 0, save_open: false, favorites_only: false,
                busy: false, message: "选择预设查看用途 · ↑↓ 浏览 · Enter 载入试听".into(), th, font };
            if let Some(editor) = view.editor.upgrade() { cx.observe(&editor, |_, _, cx| cx.notify()).detach(); }
            view.reload(cx);
            view
        })
    }) { editor.update(cx, |ed, cx| { ed.notice(format!("无法打开预设库：{error}")); cx.notify(); }); }
}

struct Library {
    editor: WeakEntity<Editor>, target: Target, generation: u64, device: String, params: Vec<ParamDesc>,
    search: Entity<TextField>, name: Entity<TextField>, tags: Entity<TextField>,
    category: Entity<TextField>, author: Entity<TextField>, description: Entity<TextField>,
    rows: Vec<Row>, factory: Vec<Row>, selected: Option<String>, loaded: Option<String>,
    last_applied: Option<Vec<(String, f64)>>, original: Option<Vec<(String, f64)>>,
    browser_focus: gpui::FocusHandle, collection: Option<verdant_collection_control::CollectionControlHandle>, scroll: gpui::ScrollHandle,
    native_scroll: crate::proto_scroll::NativeScroll,
    category_filter: String, source_filter: u8, save_open: bool,
    favorites_only: bool, busy: bool, message: String, th: Theme, font: gpui::Font,
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
                            Ok(preset) => rows.push(Row { id: path.to_string_lossy().to_string(), path: Some(path), factory_id: None, preset }), Err(_) => skipped += 1,
                        }
                    }
                }
                rows.sort_by(|a, b| a.preset.name.to_lowercase().cmp(&b.preset.name.to_lowercase()));
                (rows, skipped)
            }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false; view.rows = view.factory.clone(); view.rows.extend(rows);
                view.rows.sort_by(|a, b| a.preset.category.to_lowercase().cmp(&b.preset.category.to_lowercase()).then_with(|| a.preset.name.to_lowercase().cmp(&b.preset.name.to_lowercase())));
                if skipped > 0 { view.message = format!("已跳过 {skipped} 个无效或不兼容的预设文件"); }
                let query = view.search.read(cx).value().to_owned();
                if let Some(index) = view.rows.iter().filter(|row| view.visible(row, &query)).position(|row| Some(&row.id) == view.selected.as_ref()) { view.scroll.scroll_to_item(index); }
                cx.notify();
            });
        }).detach();
    }
    fn select(&mut self, row: &Row, cx: &mut Context<Self>) {
        self.selected = Some(row.id.clone());
        self.name.update(cx, |field, cx| field.set(&row.preset.name, cx));
        self.tags.update(cx, |field, cx| field.set(&row.preset.tags.join(", "), cx));
        self.category.update(cx, |field, cx| field.set(&row.preset.category, cx));
        self.author.update(cx, |field, cx| field.set(&row.preset.author, cx));
        self.description.update(cx, |field, cx| field.set(&row.preset.description, cx));
        cx.notify();
    }
    fn apply(&mut self, row: &Row, cx: &mut Context<Self>) {
        let target = self.target.clone(); let generation = self.generation;
        let preset = row.preset.clone(); let device = self.device.clone();
        let result = self.editor.update(cx, |ed, cx| -> Result<(Vec<(String, f64)>, Vec<(String, f64)>), String> {
            if ed.document_generation != generation { return Err("工程已更换，请重新打开预设库".into()); }
            let params = current_params(ed, &target).ok_or("插件已重建或移除，请重新打开预设库")?;
            preset.validate(&device, &params)?;
            let before = params.iter().map(|p| (p.id.clone(), p.value)).collect();
            let lock = read_workspace(ed, &target).wet_lock;
            let values = preset_values(&params, &preset.parameters, lock)?;
            if ed.set_device_params(target.track, target.slot, values.clone(), cx) { Ok((before, values)) }
            else { Err("正在处理其他修改，请稍后重试".into()) }
        });
        self.message = match result {
            Ok(Ok((before, values))) => {
                if self.original.is_none() { self.original = Some(before); }
                self.last_applied = Some(values); self.loaded = Some(row.id.clone());
                format!("已载入 {} · 在当前播放或演奏中试听", row.preset.name)
            }, Ok(Err(e)) => e, Err(_) => "工程已关闭".into()
        };
        self.select(row, cx);
    }
    fn restore(&mut self, cx: &mut Context<Self>) {
        let (Some(original), Some(expected)) = (self.original.clone(), self.last_applied.clone()) else { return };
        let target = self.target.clone(); let generation = self.generation;
        let result = self.editor.update(cx, |ed, cx| -> Result<(), String> {
            if ed.document_generation != generation { return Err("工程已更换，试听状态已失效".into()); }
            if ed.plugin_busy.contains(&target.key) { return Err("正在载入预设，请稍后返回原音".into()); }
            let current = current_params(ed, &target).ok_or("插件已重建或移除")?;
            if !same_values(&current, &expected) { return Err("试听后参数已修改；使用撤销保留这些修改的顺序".into()); }
            if ed.set_device_params(target.track, target.slot, original, cx) { Ok(()) }
            else { Err("正在处理其他修改，请稍后重试".into()) }
        });
        self.message = match result {
            Ok(Ok(())) => { self.original = None; self.last_applied = None; self.loaded = None; "已恢复试听前的声音".into() },
            Ok(Err(e)) => e, Err(_) => "工程已关闭".into()
        }; cx.notify();
    }
    fn visible(&self, row: &Row, query: &str) -> bool {
        row.preset.matches(query) && (!self.favorites_only || row.preset.favorite)
            && (self.category_filter.is_empty() || row.preset.category == self.category_filter)
            && (self.source_filter == 0 || (self.source_filter == 1) == row.factory_id.is_some())
    }
    fn save_current(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        if self.busy { return; }
        let Some(editor) = self.editor.upgrade() else { return };
        let ed = editor.read(cx);
        if ed.document_generation != self.generation || ed.plugin_busy.contains(&self.target.key) { self.message = "请等待当前参数提交完成".into(); cx.notify(); return; }
        let Some(params) = current_params(ed, &self.target) else { self.message = "插件已改变，请重新打开预设库".into(); cx.notify(); return };
        let mut preset = match PresetFile::new(&self.device, self.name.read(cx).value(), self.tags.read(cx).value(), &params) {
            Ok(p) => p, Err(e) => { self.message = e; cx.notify(); return; }
        };
        let selected = self.rows.iter().find(|row| Some(&row.id) == self.selected.as_ref());
        if overwrite && selected.is_none_or(|row| row.path.is_none()) { self.message = "内置预设只读，请另存为用户预设".into(); cx.notify(); return; }
        preset.category = self.category.read(cx).value().trim().into();
        preset.author = self.author.read(cx).value().trim().into();
        preset.description = self.description.read(cx).value().trim().into();
        preset.origin_factory_id = selected.and_then(|row| row.factory_id.clone().or_else(|| row.preset.origin_factory_id.clone()));
        if let Err(error) = preset.validate(&self.device, &params) { self.message = error; cx.notify(); return; }
        if overwrite { preset.favorite = selected.is_some_and(|row| row.preset.favorite); }
        let path = if overwrite { selected.and_then(|row| row.path.clone()).unwrap() } else { fresh_path(&self.device) };
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let saved_path = path.clone();
            let result = cx.background_spawn(async move { save_preset(&path, &preset) }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result { Ok(()) => { view.selected = Some(saved_path.to_string_lossy().to_string()); view.save_open = false; view.source_filter = 0; view.category_filter.clear(); view.favorites_only = false; view.search.update(cx, |field, cx| field.set("", cx)); view.message = "已保存当前参数与标签".into(); view.reload(cx); }, Err(e) => view.message = e }
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
        let Some(row) = self.rows.iter().find(|r| Some(&r.id) == self.selected.as_ref()).cloned() else { self.message = "请先选择预设".into(); cx.notify(); return };
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
        if let Some(id) = &row.factory_id {
            let id = id.clone(); let favorite = !row.preset.favorite;
            let _ = self.editor.update(cx, |ed, cx| {
                let mut favorites: Vec<String> = ed.prefs.extra.get("plugin_factory_favorites")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
                favorites.retain(|item| item != &id); if favorite { favorites.push(id.clone()); }
                favorites.sort(); favorites.dedup();
                ed.prefs.extra.insert("plugin_factory_favorites".into(), serde_json::json!(favorites)); ed.prefs_dirty = true; cx.notify();
            });
            for item in self.factory.iter_mut().chain(self.rows.iter_mut()).filter(|r| r.factory_id.as_ref() == Some(&id)) { item.preset.favorite = favorite; }
            self.message = if favorite { "已收藏内置预设" } else { "已取消收藏" }.into(); cx.notify(); return;
        }
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let favorite = !row.preset.favorite;
            let result = cx.background_spawn(async move {
                let mut preset = row.preset; preset.favorite = favorite;
                save_preset(row.path.as_ref().ok_or("内置预设只读")?, &preset)
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
        let Some(path) = self.rows.iter().find(|r| Some(&r.id) == self.selected.as_ref()).and_then(|r|r.path.clone()) else {
            self.message = "内置预设只读；用户预设可以移到回收目录".into(); cx.notify(); return;
        };
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

fn same_values(params: &[ParamDesc], expected: &[(String, f64)]) -> bool {
    expected.len() == params.len() && expected.iter().all(|(id, value)| {
        params.iter().find(|p| p.id == *id).is_some_and(|p| (p.value - value).abs() <= 1e-8 * value.abs().max(1.))
    })
}
impl Library {
    fn collection_revision(&self, cx: &App) -> String {
        let query = self.search.read(cx).value();
        let ids: Vec<_> = self.rows.iter().filter(|row| self.visible(row, query)).map(|row| row.id.as_str()).collect();
        serde_json::json!([self.generation, self.device, self.category_filter, self.source_filter,
            self.favorites_only, query, ids]).to_string()
    }

    fn compiled_list(&self, cx: &mut Context<Self>) -> crate::proto_collection::Collection {
        let query = self.search.read(cx).value();
        let visible: Vec<_> = self.rows.iter().filter(|row| self.visible(row, query)).collect();
        let count = visible.len();
        let cursor = visible.iter().position(|row| self.selected.as_ref() == Some(&row.id)).unwrap_or(0);
        let revision = self.collection_revision(cx);
        let props = serde_json::json!({"mode":"searchlist", "label":"匹配的预设", "cursor":cursor,
            "revision":revision, "autoFocus":false, "disabled":false});
        let entity = cx.entity(); let basis = revision.clone();
        let draw: crate::proto_collection::Draw = std::rc::Rc::new(move |handle, focus, window, cx| {
            entity.update(cx, |view, cx| {
                let had_focus = view.browser_focus.is_focused(window);
                view.browser_focus = focus.clone();
                view.collection = Some(handle.clone());
                if had_focus { window.focus(focus, cx); }
                let entity = cx.entity(); let handle = handle.clone(); let basis = basis.clone();
                crate::proto_scroll::content("compiled-preset-scroll", "预设列表", move |_, cx| {
                    entity.update(cx, |view, cx| view.native_list(&handle, &basis, cx))
                }).native_scroll(view.native_scroll.clone()).into_any_element()
            })
        });
        let entity = cx.entity().downgrade();
        let sink: crate::proto_collection::Sink = std::rc::Rc::new(move |name, payload, window, cx| {
            let _ = entity.update(cx, |view, cx| {
                if payload.get("revision").and_then(serde_json::Value::as_str) != Some(view.collection_revision(cx).as_str()) { return; }
                if name == "close" { window.remove_window(); return; }
                if name == "command" && payload.get("key").and_then(serde_json::Value::as_str) == Some("Find") {
                    let focus = view.search.read(cx).focus.clone();
                    window.focus(&focus, cx); return;
                }
                if name != "select" && name != "activate" { return; }
                let Some(index) = crate::proto_controls::source_index(payload.get("index")) else { return; };
                let query = view.search.read(cx).value();
                let Some(row) = view.rows.iter().filter(|row| view.visible(row, query)).nth(index).cloned() else { return; };
                view.select(&row, cx);
                view.scroll.scroll_to_item(index);
                if name == "activate" { view.apply(&row, cx); }
            });
        });
        crate::proto_collection::collection("compiled-preset-list", props, vec![1; count], draw, sink)
    }

    fn native_list(&self, collection: &verdant_collection_control::CollectionControlHandle,
        revision: &str, cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = self.th;
        let query = self.search.read(cx).value();
        let mut list = div().id("preset-list").track_scroll(&self.scroll).size_full().overflow_y_scroll().flex().flex_col().gap(px(3.));
        let mut count = 0;
        for (index, row) in self.rows.iter().filter(|row| self.visible(row, query)).enumerate() {
            count += 1;
            let selected = self.selected.as_ref() == Some(&row.id);
            let choose = collection.clone(); let basis = revision.to_owned();
            let favorite = row.id.clone();
            // Selection and the favorite button are sibling controls. Nesting
            // two triggers would give the row's hit target the star's click.
            list = list.child(div().id(SharedString::from(row.id.clone()))
                .rounded(px(8.)).border_1().border_color(if selected { with_alpha(th.accent, 0.5) } else { with_alpha(th.text, 0.) })
                .bg(if selected { th.checked } else { th.card }).hover(move |s| s.bg(if selected { th.checked } else { th.hover }))
                .flex().items_center().gap(px(10.))
                .child(crate::proto_controls::action(SharedString::from(format!("preset-option-{}", row.id)), row.preset.name.clone())
                    .role(Role::ListBoxOption).aria_selected(selected).tab_index(-1)
                    .px(px(12.)).py(px(9.)).flex_1().min_w_0().flex().items_center().gap(px(10.)).cursor_pointer()
                    .on_click(move |_, window, cx| crate::proto_collection::call(&choose, "selectAndFocus",
                        vec![index.into(), false.into(), basis.clone().into()], window, cx))
                    .child(div().flex_1().min_w_0().flex().flex_col().gap(px(3.))
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).truncate().child(row.preset.name.clone()))
                        .child(div().text_xs().truncate().text_color(th.text_secondary).child(format!("{}{}{}", row.preset.category,
                            if row.preset.category.is_empty() || row.preset.tags.is_empty() { "" } else { " · " }, row.preset.tags.join(" · ")))))
                    .child(div().text_xs().text_color(th.text_tertiary).child(if row.factory_id.is_some() { "内置" } else { "用户" })))
                .child(presentation::button(format!("favorite-preset-{}", row.id), if row.preset.favorite { "★" } else { "☆" }, row.preset.favorite, th)
                    .mr(px(12.)).aria_label(if row.preset.favorite { "取消收藏预设" } else { "收藏预设" })
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(row) = view.rows.iter().find(|row| row.id == favorite).cloned() { view.favorite(row, cx); }
                        cx.stop_propagation();
                    }))));
        }
        if count == 0 { list = list.child(div().p(px(24.)).text_sm().text_color(th.text_secondary).child("没有匹配的预设。试试其他类别、来源或搜索词。")); }
        list.into_any_element()
    }

    fn category_list(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = self.th;
        let mut categories: std::collections::BTreeSet<String> = self.rows.iter().map(|r| r.preset.category.clone()).filter(|c| !c.is_empty()).collect();
        categories.insert(String::new());
        div().id("preset-categories").flex().flex_col().gap(px(3.)).children(categories.into_iter().map(|category| {
            let selected = self.category_filter == category; let value = category.clone();
            presentation::button(format!("preset-category-{category}"), if category.is_empty() { "全部类别".to_string() } else { category }, selected, th)
                .justify_start().on_click(cx.listener(move |view, _, _, cx| { view.category_filter = value.clone(); cx.notify(); }))
        })).into_any_element()
    }

    fn save_form(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = self.th;
        let can_update = self.rows.iter().find(|r| Some(&r.id) == self.selected.as_ref()).is_some_and(|r| r.path.is_some());
        let field = |label: &'static str, entity: Entity<TextField>| div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
            .child(div().text_xs().text_color(th.text_secondary).child(label)).child(entity);
        let button = |id: &'static str, label: &'static str| presentation::button(id, label, false, th);
        let actions = div().flex().items_center().gap(px(8.))
            .child(button("preset-save-new", "另存为用户预设").on_click(cx.listener(|v, _, _, cx| v.save_current(false, cx))))
            .when(can_update, |d| {
                d.child(button("preset-update", "更新选中预设").on_click(cx.listener(|v, _, _, cx| v.save_current(true, cx))))
                    .child(button("preset-delete", "移到回收目录").on_click(cx.listener(|v, _, _, cx| v.delete(cx))))
            });
        div().id("preset-save-form").flex().flex_col().gap(px(10.)).border_t_1().border_color(th.shade).pt(px(10.))
            .child(div().text_sm().text_color(th.text_secondary).child("保存插件当前参数。内置预设将另存为用户预设；再次点击“保存当前声音”返回浏览。"))
            .child(div().flex().gap(px(10.)).child(field("名称", self.name.clone())).child(field("类别", self.category.clone())))
            .child(div().flex().gap(px(10.)).child(field("作者", self.author.clone())).child(field("标签 · 逗号分隔", self.tags.clone())))
            .child(field("用途与说明", self.description.clone()))
            .child(actions).into_any_element()
    }
}

impl Render for Library {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.th; let query = self.search.read(cx).value().to_owned();
        if !self.busy && !self.rows.iter().any(|r| self.selected.as_ref() == Some(&r.id) && self.visible(r, &query)) {
            if let Some(first) = self.rows.iter().find(|r| self.visible(r, &query)).cloned() {
                self.select(&first, cx); self.scroll.scroll_to_item(0);
            } else { self.selected = None; }
        }
        let categories = cx.entity();
        let category_list = div().w(px(140.)).flex_none().child(crate::proto_scroll::content("compiled-preset-categories-scroll", "预设类别", move |_, cx|
            categories.update(cx, |view, cx| view.category_list(cx))));
        let count = self.rows.iter().filter(|row| self.visible(row, &query)).count();
        let list = self.compiled_list(cx);
        let selected = self.rows.iter().find(|r| Some(&r.id) == self.selected.as_ref());
        let mut details = div().flex().flex_col().gap(px(5.)).p(px(12.)).rounded(px(10.)).bg(th.card).border_1().border_color(th.shade);
        if let Some(row) = selected {
            let mut status = if self.loaded.as_ref() == Some(&row.id) { "当前声音".to_string() } else { "选中预设".to_string() };
            if self.loaded.as_ref() == Some(&row.id) {
                if let (Some(editor), Some(expected)) = (self.editor.upgrade(), self.last_applied.as_ref()) {
                    if current_params(editor.read(cx), &self.target).is_some_and(|params| !same_values(&params, expected)) { status = "当前声音 · 已编辑".into(); }
                }
            }
            details = details.child(div().flex().items_center().gap(px(8.))
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.preset.name.clone()))
                .child(div().text_xs().text_color(th.text_secondary).child(status)))
                .child(div().text_xs().text_color(th.text_secondary).child(format!("{}{}{}", row.preset.category,
                    if row.preset.category.is_empty() || row.preset.author.is_empty() { "" } else { " · " }, row.preset.author)))
                .child(div().text_sm().text_color(th.text_secondary).child(if row.preset.description.is_empty() { row.preset.tags.join(" · ") } else { row.preset.description.clone() }));
        } else { details = details.child(div().text_sm().text_color(th.text_secondary).child("选择预设查看作者、适用场景与使用建议")); }
        let button = |id: &'static str, label: &'static str| presentation::button(id, label, false, th);
        let mut source = div().flex().items_center().gap(px(4.));
        for (value, label) in [(0u8, "全部"), (1, "内置预设"), (2, "我的预设")] {
            source = source.child(presentation::button(format!("preset-source-{value}"), label, self.source_filter == value, th)
                .on_click(cx.listener(move |v, _, _, cx| { v.source_filter = value; cx.notify(); })));
        }
        let save_form = cx.entity();
        div().id("preset-library").role(Role::Group).aria_label("插件预设库").size_full().bg(th.surface).text_color(th.text)
            .font(self.font.clone()).p(px(16.)).flex().flex_col().gap(px(10.))
            .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                if window.default_prevented() { return; }
                let Some(collection) = view.collection.clone() else { return };
                let fields = [&view.name, &view.tags, &view.category, &view.author, &view.description];
                if view.search.read(cx).is_composing() || fields.iter().any(|field| field.read(cx).is_composing()) { return; }
                let origin = if view.search.read(cx).focus.is_focused(window) { "search" }
                    else if fields.iter().any(|field| field.read(cx).focus.is_focused(window)) { "form" }
                    else { "root" };
                let revision = view.collection_revision(cx);
                crate::proto_collection::key_from(&collection, origin, &revision, event, window, cx);
            }))
            .child(div().flex().items_center().gap(px(6.)).child(div().flex_1().text_lg().font_weight(FontWeight::BOLD).child("预设库"))
                .child(button("preset-import", "导入").on_click(cx.listener(|v, _, _, cx| v.import(cx))))
                .child(button("preset-export", "导出").on_click(cx.listener(|v, _, _, cx| v.export(cx))))
                .child(presentation::button("preset-save-panel", "保存当前声音", self.save_open, th).on_click(cx.listener(|v, _, _, cx| { v.save_open = !v.save_open; cx.notify(); })))
                .child(button("preset-close", "关闭").on_click(|_, w, _| w.remove_window())))
            .when(!self.save_open, |d| d.child(div().flex().items_center().gap(px(8.)).child(div().flex_1().child(self.search.clone()))
                .child(presentation::button("preset-favorites-only", "★ 收藏", self.favorites_only, th)
                    .on_click(cx.listener(|view, _, _, cx| { view.favorites_only = !view.favorites_only; cx.notify(); }))))
            .child(div().flex().items_center().gap(px(8.)).child(source).child(div().flex_1())
                .child(div().text_xs().text_color(th.text_secondary).child(format!("{count} 个预设"))))
            .child(div().flex_1().min_h_0().flex().gap(px(12.)).child(category_list).child(list))
            .child(details)
            .child(div().flex().flex_wrap().items_center().gap(px(8.))
                .child(presentation::button("preset-audition", "载入试听", true, th).on_click(cx.listener(|v, _, _, cx| {
                    if let Some(row) = v.rows.iter().find(|r|Some(&r.id) == v.selected.as_ref()).cloned() { v.apply(&row, cx); }
                })))
                .when(self.original.is_some(), |d| d.child(button("preset-restore", "返回原音").on_click(cx.listener(|v, _, _, cx| v.restore(cx)))))
                .child(div().flex_1()).child(div().text_xs().text_color(th.text_secondary).child("↑↓ 浏览 · Enter 载入 · 关闭保留当前声音"))))
            .when(self.save_open, |d| d.child(div().flex_1().min_h_0().child(crate::proto_scroll::content("compiled-preset-save-scroll", "保存预设", move |_, cx|
                save_form.update(cx, |view, cx| view.save_form(cx))))))
            .child(div().id("preset-library-status").role(Role::Status).text_xs().text_color(th.text_secondary)
                .child(if self.busy { "正在读取或保存…".into() } else { self.message.clone() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audition_restore_requires_the_complete_unchanged_parameter_state() {
        let p = ParamDesc { id: "mix".into(), name: "Mix".into(), value: 50., min: 0., max: 100., unit: "%".into(), step_count: 0, default_value: Some(100.) };
        assert!(same_values(&[p.clone()], &[("mix".into(), 50.)]));
        assert!(!same_values(&[p.clone()], &[("mix".into(), 49.)]));
        assert!(!same_values(&[p.clone()], &[("removed".into(), 50.)]));
        assert!(!same_values(&[p], &[]));
    }
}
