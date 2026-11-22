//! One searchable view of the project's track, instrument and master devices.
use super::*;
use super::text_field::TextField;
use gpui::{App, AppContext, Bounds, Render, WeakEntity, WindowBounds, WindowOptions, size};

#[derive(Clone)]
struct Instance { owner: DeviceTarget, slot: usize, key: Option<InsertKey>, name: String, bus: String, id: String, status: String }
fn instances(ed: &Editor) -> Vec<Instance> {
    let mut result = Vec::new();
    for track in &ed.project().tracks {
        let owner = DeviceTarget::Track(track.id.clone());
        if let Some(spec) = track.instrument() {
            let id = spec.get("id").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
            let native = spec.get("format").and_then(serde_json::Value::as_str) == Some("builtin") && dsp::is_builtin_instrument(&id);
            result.push(Instance { owner: owner.clone(), slot: crate::instrument::INSTRUMENT_SLOT,
                key: native.then(|| crate::instrument::instrument_key_for_generation(&track.id, &id, ed.document_generation)),
                name: spec.get("name").and_then(serde_json::Value::as_str).unwrap_or("乐器").into(), bus: track.name.clone(), id,
                status: if native { "乐器".into() } else { "外部乐器 · 尚无可用原生编辑器".into() } });
        }
        append_inserts(ed, &owner, &track.name, &mut result);
    }
    append_inserts(ed, &DeviceTarget::Master, "主总线", &mut result);
    result
}
fn append_inserts(ed: &Editor, owner: &DeviceTarget, bus: &str, result: &mut Vec<Instance>) {
    for (slot, spec) in ed.device_insert_specs(owner).into_iter().enumerate() {
        let live = ed.live_device_insert(owner, slot);
        result.push(Instance { owner: owner.clone(), slot, key: live.as_ref().map(|h| h.key()), name: spec.name, bus: bus.into(), id: spec.id,
            status: if spec.bypass { "旁通".into() } else { live.as_ref().map(|h| format!("{:?}", h.status())).unwrap_or("缺失".into()) } });
    }
}
pub(crate) fn open(editor: Entity<Editor>, cx: &mut App) {
    let th = editor.read(cx).theme; let generation = editor.read(cx).document_generation;
    let weak = editor.downgrade(); let observed = editor.clone();
    let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
    if let Err(error) = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), window_min_size: Some(size(px(440.), px(300.))),
        app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title("工程插件实例 — Verdant");
        cx.new(|cx| {
            let search = TextField::new("按轨道、总线或插件名称搜索实例", "", th, cx);
            let focus = search.read(cx).focus.clone(); window.focus(&focus, cx);
            cx.observe(&search, |_, _, cx| cx.notify()).detach();
            cx.observe(&observed, |_, _, cx| cx.notify()).detach();
            Instances { editor: weak, generation, search, th }
        })
    }) { editor.update(cx, |ed, cx| { ed.notice(format!("无法打开插件实例：{error}")); cx.notify(); }); }
}
struct Instances { editor: WeakEntity<Editor>, generation: u64, search: Entity<TextField>, th: Theme }
impl Render for Instances {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else { window.remove_window(); return div().into_any_element() };
        let ed = editor.read(cx);
        if ed.document_generation != self.generation { window.remove_window(); return div().into_any_element() }
        let th = self.th;
        let query = self.search.read(cx).text.text.to_lowercase();
        let mut rows = div().id("instance-list").role(Role::List).flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(px(8.));
        let mut count = 0;
        for item in instances(ed).into_iter().filter(|i| {
            let haystack = format!("{} {} {} {}", i.name, i.bus, i.id, i.status).to_lowercase();
            query.split_whitespace().all(|word| haystack.contains(word))
        }) {
            count += 1;
            let open = item.clone(); let weak = self.editor.clone(); let generation = self.generation;
            rows = rows.child(div().id(SharedString::from(format!("instance-{count}"))).role(Role::ListItem).p(px(12.)).rounded(px(10.)).bg(th.card)
                .flex().items_center().gap(px(12.))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", item.bus, item.name)))
                    .child(div().text_xs().text_color(th.text_secondary).child(format!("{} · {}", if item.slot == crate::instrument::INSTRUMENT_SLOT { "音符乐器".into() } else { format!("插入 {}", item.slot + 1) }, item.status))))
                .child(crate::ui_controls::selectable_button(format!("open-instance-{count}"), if item.key.is_some() { "打开界面" } else { "暂不可用" }, false, th)
                    .aria_description(if item.key.is_some() { "打开当前实例的编辑器" } else { "当前实例没有可用编辑器" }).tab_index(if item.key.is_some() { 0 } else { -1 })
                    .when(item.key.is_none(), |button| button.opacity(0.5).cursor_default())
                    .on_click(move |_, _, cx| {
                        let _ = weak.update(cx, |ed, cx| {
                            if ed.document_generation != generation { return; }
                            if open.slot == crate::instrument::INSTRUMENT_SLOT {
                                if let DeviceTarget::Track(t) = &open.owner {
                                    if open.key.is_some_and(|key| ed.device_target_matches(&open.owner, open.slot, key)) { ed.open_builtin_instrument_editor(t, cx); }
                                }
                            } else if open.key.is_some_and(|key| ed.device_target_matches(&open.owner, open.slot, key)) {
                                match &open.owner { DeviceTarget::Track(t) => ed.open_insert_editor(t, open.slot, cx), DeviceTarget::Master => ed.open_master_insert_editor(open.slot, cx) }
                            }
                        });
                    })));
        }
        div().size_full().bg(th.surface).text_color(th.text).p(px(18.)).flex().flex_col().gap(px(12.))
            .font(crate::theme::plugin_ui_font(ed.ui_font.clone()))
            .child(div().flex().items_center().gap(px(8.)).child(div().flex_1().text_lg().font_weight(FontWeight::BOLD).child("工程插件实例"))
                .child(crate::ui_controls::selectable_button("instance-close", "关闭", false, th).on_click(|_, w, _| w.remove_window())))
            .child(self.search.clone())
            .child(div().text_xs().text_color(th.text_secondary).child(format!("{count} 个实例 · 轨道 / 主总线 / 乐器")))
            .child(rows).into_any_element()
    }
}
