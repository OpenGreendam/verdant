//! Compact Adwaita surfaces shared by every native processor.
//! Presentation preferences follow the instance, independently of audio state.
use super::*;

pub(super) fn page(ed: &Editor, target: &Target) -> u8 {
    workflow::read_workspace(ed, target).editor_page.min(2)
}

pub(super) fn button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, selected: bool, th: Theme) -> gpui::Stateful<gpui::Div> {
    div().id(id).role(Role::Button).tab_index(0).cursor_pointer()
        .flex_none().flex().items_center().justify_center().min_h(gpui::rems(30./16.))
        .px(gpui::rems(10./16.)).py(gpui::rems(4./16.)).rounded(gpui::rems(6./16.))
        .border_1().border_color(if selected { with_alpha(th.accent, 0.45) } else { with_alpha(th.text, 0.) })
        .bg(if selected { th.checked } else { with_alpha(th.text, 0.) })
        .hover(move |s| s.bg(th.hover)).focus(move |s| s.border_color(th.accent))
        .text_sm().font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(if selected { th.accent } else { th.text_secondary }).child(label.into())
}

pub(super) fn tabs(ed: &Editor, target: &Target, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let selected = page(ed, target); let th = ed.theme;
    div().id(format!("device-pages-{}", target.slot)).role(Role::TabList).aria_label("插件工作区").flex().items_center().gap(gpui::rems(2./16.))
        .p(gpui::rems(2./16.)).rounded(gpui::rems(8./16.)).bg(th.control)
        .children([(0u8, "编辑"), (1, "高级"), (2, "计量")].into_iter().map(|(index, label)| {
            let click_target = target.clone(); let key_target = target.clone();
            button(format!("device-page-{}-{index}", target.slot), label, selected == index, th)
                .role(Role::Tab).aria_selected(selected == index).aria_label(format!("{label}工作区"))
                .when(selected == index, |d| d.bg(th.card).text_color(th.text).border_color(th.shade))
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if !ed.device_target_matches(&click_target.track, click_target.slot, click_target.key) { return; }
                    let mut state = workflow::read_workspace(ed, &click_target); state.editor_page = index;
                    workflow::write_workspace(ed, &click_target, &state); cx.notify();
                }))
                .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "enter" | "space" => index, "left" => (selected + 2) % 3, "right" => (selected + 1) % 3, _ => return,
                    };
                    if !ed.device_target_matches(&key_target.track, key_target.slot, key_target.key) { return; }
                    let mut state = workflow::read_workspace(ed, &key_target); state.editor_page = next;
                    workflow::write_workspace(ed, &key_target, &state);
                    window.prevent_default(); cx.stop_propagation(); cx.notify();
                }))
        })).into_any_element()
}

pub(super) fn section_header(ed: &Editor, target: &Target, label: &str, count: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let state = workflow::read_workspace(ed, target);
    let expanded = state.expanded_sections.contains(label); let th = ed.theme;
    let click_target = target.clone(); let key_target = target.clone();
    let click_label = label.to_string(); let key_label = label.to_string();
    div().id(format!("device-section-{}-{label}", target.slot)).role(Role::Button).tab_index(0)
        .aria_label(format!("{label} · {count} 个参数")).aria_selected(expanded)
        .flex().items_center().gap(gpui::rems(10./16.)).w_full().min_h(gpui::rems(42./16.))
        .px(gpui::rems(12./16.)).rounded(gpui::rems(8./16.)).bg(th.card)
        .border_1().border_color(th.shade).hover(move |s|s.bg(th.hover)).focus(move |s|s.border_color(th.accent)).cursor_pointer()
        .child(div().text_sm().text_color(th.text_secondary).child(if expanded { "⌄" } else { "›" }))
        .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(label.to_string()))
        .child(div().text_xs().text_color(th.text_secondary).child(format!("{count} 个参数")))
        .on_click(cx.listener(move |ed, _, _, cx| {
            let mut state = workflow::read_workspace(ed, &click_target);
            if !state.expanded_sections.remove(&click_label) { state.expanded_sections.insert(click_label.clone()); }
            workflow::write_workspace(ed, &click_target, &state); cx.notify();
        }))
        .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
            if !matches!(event.keystroke.key.as_str(), "enter" | "space") { return; }
            let mut state = workflow::read_workspace(ed, &key_target);
            if !state.expanded_sections.remove(&key_label) { state.expanded_sections.insert(key_label.clone()); }
            workflow::write_workspace(ed, &key_target, &state); window.prevent_default(); cx.stop_propagation(); cx.notify();
        })).into_any_element()
}

pub(super) fn caption(title: &str, detail: &str, th: Theme) -> gpui::AnyElement {
    div().flex().flex_wrap().items_baseline().gap(gpui::rems(8./16.))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title.to_string()))
        .child(div().text_xs().text_color(th.text_secondary).child(detail.to_string())).into_any_element()
}
