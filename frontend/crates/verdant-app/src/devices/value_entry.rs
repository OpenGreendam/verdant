//! Exact, unit-aware entry shared by native effect and instrument controls.
//! The dialog holds an immutable target/revision; a changed document cannot
//! redirect an old value entry to a different insert at the same slot.

use super::*;
use gpui::{App, Bounds, Render, WindowBounds, WindowOptions, size};
use crate::proto_text::{Field, Event as TextEvent, Options as TextOptions};

pub(super) fn parse_value(text: &str, desc: &ParamDesc) -> Result<f64, String> {
    let text = text.trim().to_lowercase();
    let (number, scale) = if desc.unit == "Hz" && text.ends_with("khz") {
        (text.strip_suffix("khz").unwrap().trim(), 1000.0)
    } else if desc.unit == "ms" && text.ends_with("ms") {
        (text.strip_suffix("ms").unwrap().trim(), 1.0)
    } else if desc.unit == "ms" && text.ends_with('s') {
        (text.strip_suffix('s').unwrap().trim(), 1000.0)
    } else if desc.unit == "s" && text.ends_with("ms") {
        (text.strip_suffix("ms").unwrap().trim(), 0.001)
    } else if !desc.unit.is_empty() && text.ends_with(&desc.unit.to_lowercase()) {
        (text[..text.len() - desc.unit.len()].trim(), 1.0)
    } else { (text.as_str(), 1.0) };
    let value = number.parse::<f64>().map_err(|_| "请输入数值，可附 Hz / kHz、ms / s、dB 或 % 单位".to_string())? * scale;
    if !value.is_finite() || value < desc.min || value > desc.max {
        return Err(format!("{} 须在 {} 至 {} {} 之间", desc.name, desc.min, desc.max, desc.unit));
    }
    verdant_plugins::insert::parameter_editor_value(desc, value).map_err(|e| e.to_string())
}

pub(super) fn open(editor: &Entity<Editor>, target: Target, desc: ParamDesc, cx: &mut App) {
    let Some(params) = current_params(editor.read(cx), &target) else { return };
    let Some(desc) = param(&params, &desc.id).cloned() else { return };
    let base = editor.read(cx).revision();
    let generation = editor.read(cx).document_generation;
    let owner = editor.downgrade();
    let bounds = Bounds::centered(None, size(px(440.), px(224.)), cx);
    if let Err(error) = cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default()
    }, move |window, cx| {
        window.set_window_title(&format!("{} — Verdant", desc.name));
        cx.new(|cx| {
            let th = owner.upgrade().map(|ed| ed.read(cx).theme).unwrap_or_else(|| Theme::for_appearance(window.appearance()));
            let text = Field::with_options(&desc.name, &format!("{}", desc.value), th,
                TextOptions { auto_focus: true, select_all: true, submit_on_enter: true, cancel_on_escape: true, ..Default::default() }, window, cx);
            cx.subscribe_in(&text, window, |view: &mut ValueEntry, _, event, window, cx| {
                match event {
                    TextEvent::Submit { .. } => view.apply(window, cx),
                    TextEvent::Cancel => window.remove_window(),
                    TextEvent::Changed { .. } => { view.error.clear(); cx.notify(); },
                    TextEvent::Failure(error) => { view.error = error.clone(); cx.notify(); },
                    _ => {},
                }
            }).detach();
            ValueEntry { editor: owner, target, desc, base, generation, text, error: String::new() }
        })
    }) {
        editor.update(cx, |ed, cx| { ed.notice(format!("无法打开数值输入：{error}")); cx.notify(); });
    }
}

struct ValueEntry {
    editor: gpui::WeakEntity<Editor>,
    target: Target,
    desc: ParamDesc,
    base: verdant_domain::Revision,
    generation: u64,
    text: Entity<Field>,
    error: String,
}

impl ValueEntry {
    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.text.read(cx).is_composing() { return; }
        let value = match parse_value(self.text.read(cx).value(), &self.desc) {
            Ok(value) => value,
            Err(error) => { self.error = error; cx.notify(); return; }
        };
        let target = self.target.clone();
        let id = self.desc.id.clone();
        let base = self.base;
        let generation = self.generation;
        let result = self.editor.update(cx, |ed, cx| {
            if ed.document_generation != generation || ed.revision() != base || current_params(ed, &target).is_none() {
                return Err("工程或插件已改变，请重新打开数值输入".to_string());
            }
            if ed.plugin_busy.contains(&target.key) {
                return Err("正在提交上一项修改，请稍后再应用".to_string());
            }
            if ed.set_device_params(target.track, target.slot, vec![(id, value)], cx) { Ok(()) }
            else { Err("参数尚未提交，请等待插件载入完成后重试".to_string()) }
        });
        match result {
            Ok(Ok(())) => window.remove_window(),
            Ok(Err(error)) => { self.error = error; cx.notify(); }
            Err(_) => { self.error = "工程窗口已关闭".into(); cx.notify(); }
        }
    }
}

impl Render for ValueEntry {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.editor.upgrade().map(|ed| ed.read(cx).theme).unwrap_or_else(|| Theme::for_appearance(gpui::WindowAppearance::Dark));
        div().id("device-value-entry").role(Role::Dialog).aria_label(self.desc.name.clone()).size_full().p(px(18.))
            .bg(th.surface).text_color(th.text).flex().flex_col().gap(px(10.)).key_context("LyricEditor")
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(self.desc.name.clone()))
            .child(self.text.clone())
            .child(div().text_xs().text_color(th.text_secondary).child(format!("{} … {} {} · Enter 应用 · Esc 取消", self.desc.min, self.desc.max, self.desc.unit)))
            .child(div().flex_1().text_xs().text_color(th.error).child(self.error.clone()))
            .child(div().flex().justify_end().gap(px(8.))
                .child(crate::ui_controls::selectable_button("value-cancel", "取消", false, th).on_click(cx.listener(|view, _, window, cx| {
                    view.text.update(cx, |field, cx| { field.invoke("requestCancel", vec![], window, cx); });
                })))
                .child(crate::ui_controls::selectable_button("value-apply", "应用", true, th).on_click(cx.listener(|view, _, window, cx| {
                    view.text.update(cx, |field, cx| { field.invoke("requestSubmit", vec![], window, cx); });
                }))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn units_convert_before_range_validation_and_nonfinite_is_rejected() {
        let hz = ParamDesc { id: "freq".into(), name: "频率".into(), value: 1000., min: 20., max: 20000., unit: "Hz".into(), step_count: 0, default_value: None };
        assert_eq!(parse_value("1.25 kHz", &hz).unwrap(), 1250.);
        assert_eq!(parse_value("1000 Hz", &hz).unwrap(), 1000.);
        assert!(parse_value("25 kHz", &hz).is_err());
        for bad in ["NaN", "inf", "-inf", "", "12xyz", "1khzkhz", "20 Hz Hz"] { assert!(parse_value(bad, &hz).is_err()); }
        let ms = ParamDesc { unit: "ms".into(), min: 0., max: 2000., ..hz.clone() };
        assert_eq!(parse_value("0.3 s", &ms).unwrap(), 300.);
        let seconds = ParamDesc { unit: "s".into(), min: 0., max: 20., ..hz };
        assert_eq!(parse_value("250 ms", &seconds).unwrap(), 0.25);
    }
}
