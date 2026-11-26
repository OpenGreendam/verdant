//! Compact Adwaita surfaces shared by every native processor.
//! Presentation preferences follow the instance, independently of audio state.
use super::*;
use crate::ui_controls::AdwStates;

pub(super) fn page(ed: &Editor, target: &Target) -> u8 {
    workflow::read_workspace(ed, target).editor_page.min(2)
}

pub(super) fn button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, selected: bool, th: Theme) -> crate::proto_controls::Action {
    let label = label.into();
    crate::proto_controls::action(id, label.clone()).role(Role::Button).tab_index(0).cursor_pointer()
        .flex_none().flex().items_center().justify_center().min_h(gpui::rems(30./16.))
        .px(gpui::rems(10./16.)).py(gpui::rems(4./16.)).rounded(gpui::rems(6./16.))
        .border_1().border_color(with_alpha(th.text, 0.))
        .flat_states(th, selected).focus(move |s| s.border_color(th.accent))
        .text_sm().font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(th.text).child(label)
}

/// A flat square icon button (AdwButton.flat with an icon child). `label`
/// is its accessible name; `selected` uses the checked ramp.
pub(super) fn icon_button(id: impl Into<gpui::ElementId>, icon_name: &'static str, label: impl Into<SharedString>, selected: bool, th: Theme) -> crate::proto_controls::Action {
    let label = label.into();
    crate::proto_controls::action(id, label.clone()).role(Role::Button).aria_label(label).tab_index(0).cursor_pointer()
        .flex_none().flex().items_center().justify_center().size(gpui::rems(30./16.)).rounded(gpui::rems(6./16.))
        .border_1().border_color(with_alpha(th.text, 0.)).flat_states(th, selected).focus(move |s| s.border_color(th.accent))
        .child(svg().path(icon(icon_name)).size(gpui::rems(16./16.)).text_color(th.text))
}

/// Buttons joined into one AdwToggleGroup-style trough.
pub(super) fn linked(children: impl IntoIterator<Item = gpui::AnyElement>, th: Theme) -> gpui::AnyElement {
    div().flex().items_center().gap(gpui::rems(2./16.)).p(gpui::rems(2./16.)).rounded(gpui::rems(8./16.)).bg(th.control)
        .children(children).into_any_element()
}

/// One segment of a linked toggle group: the checked segment is a raised
/// card in the trough (AdwToggleGroup), the others are flat.
pub(super) fn segment(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, selected: bool, th: Theme) -> crate::proto_controls::Action {
    let label = label.into();
    segment_shell(id, label.clone(), selected, th).child(label)
}

/// A segment whose visible content is a colour dot and a short label while
/// `name` stays the accessible label (band and node pickers).
pub(super) fn dot_segment(id: impl Into<gpui::ElementId>, name: impl Into<SharedString>, short: impl Into<SharedString>,
    color: gpui::Rgba, selected: bool, th: Theme) -> crate::proto_controls::Action {
    let name = name.into();
    segment_shell(id, name.clone(), selected, th).aria_label(name).gap(gpui::rems(6./16.)).px(gpui::rems(8./16.))
        .child(div().flex_none().size(gpui::rems(8./16.)).rounded_full().bg(color))
        .child(short.into())
}

fn segment_shell(id: impl Into<gpui::ElementId>, label: SharedString, selected: bool, th: Theme) -> crate::proto_controls::Action {
    let base = crate::proto_controls::action(id, label).role(Role::Button).tab_index(0).cursor_pointer()
        .flex_none().flex().items_center().justify_center().min_h(gpui::rems(26./16.)).min_w(gpui::rems(30./16.))
        .px(gpui::rems(10./16.)).rounded(gpui::rems(6./16.)).border_1()
        .focus(move |s| s.border_color(th.accent))
        .text_sm().font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL }).text_color(th.text);
    if selected {
        base.border_color(th.shade).bg(th.card).hover(move |s| s.bg(th.card)).active(move |s| s.bg(crate::theme::mix(th.card, th.text, 0.06)))
    } else {
        base.border_color(with_alpha(th.text, 0.)).flat_states(th, false)
    }
}

fn page_command(target: Target, index: u8, cx: &Context<Editor>) -> SemanticCommand {
    semantic_command(cx.weak_entity(), cx.weak_entity(), move |ed, _, window, cx| {
        if !DeviceScope::for_window(window).allows(&target.track, ed)
            || !ed.device_target_matches(&target.track, target.slot, target.key) { return; }
        let mut state = workflow::read_workspace(ed, &target); state.editor_page = index;
        workflow::write_workspace(ed, &target, &state); cx.notify();
    })
}

pub(super) fn section_command(target: Target, label: String, cx: &Context<Editor>) -> SemanticCommand {
    semantic_command(cx.weak_entity(), cx.weak_entity(), move |ed, _, window, cx| {
        if !DeviceScope::for_window(window).allows(&target.track, ed)
            || !ed.device_target_matches(&target.track, target.slot, target.key) { return; }
        let mut state = workflow::read_workspace(ed, &target);
        if !state.expanded_sections.remove(&label) { state.expanded_sections.insert(label.clone()); }
        workflow::write_workspace(ed, &target, &state); cx.notify();
    })
}

pub(super) fn tabs(ed: &Editor, target: &Target, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let selected = page(ed, target); let th = ed.theme;
    div().id(format!("device-pages-{}", target.slot)).role(Role::TabList).aria_label("插件工作区").flex().items_center().gap(gpui::rems(2./16.))
        .p(gpui::rems(2./16.)).rounded(gpui::rems(8./16.)).bg(th.control)
        .children([(0u8, "编辑"), (1, "高级"), (2, "计量")].into_iter().map(|(index, label)| {
            let command = page_command(target.clone(), index, cx);
            let navigate: Vec<_> = (0..3).map(|page| page_command(target.clone(), page, cx)).collect();
            semantic_click(segment(format!("device-page-{}-{index}", target.slot), label, selected == index, th)
                .authority(suite::action_authority(ed, target))
                .role(Role::Tab).aria_selected(selected == index).aria_label(format!("{label}工作区")), command)
                .navigation(index as usize, 3, move |index, window, cx| {
                    if let Some(command) = navigate.get(index) { command(None, window, cx); }
                })
        })).into_any_element()
}

pub(super) fn section_header(ed: &Editor, target: &Target, label: &str, count: usize, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let state = workflow::read_workspace(ed, target);
    let expanded = state.expanded_sections.contains(label); let th = ed.theme;
    let command = section_command(target.clone(), label.to_string(), cx);
    semantic_click(crate::proto_controls::action(format!("device-section-{}-{label}", target.slot), label.to_string()).role(Role::Button).tab_index(0)
        .authority(suite::action_authority(ed, target))
        .aria_label(format!("{label} · {count} 个参数")).aria_expanded(expanded)
        .flex().items_center().gap(gpui::rems(10./16.)).w_full().min_h(gpui::rems(42./16.))
        .px(gpui::rems(12./16.)).rounded(gpui::rems(8./16.)).bg(th.card)
        .border_1().border_color(th.shade).hover(move |s|s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover))).focus(move |s|s.border_color(th.accent)).cursor_pointer()
        .child(div().text_sm().text_color(th.text_secondary).child(if expanded { "⌄" } else { "›" }))
        .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(label.to_string()))
        .child(div().text_xs().text_color(th.text_secondary).child(format!("{count} 个参数"))), command)
        .into_any_element()
}

pub(super) fn caption(title: &str, detail: &str, th: Theme) -> gpui::AnyElement {
    div().flex().flex_wrap().items_baseline().gap(gpui::rems(8./16.))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title.to_string()))
        .child(div().text_xs().text_color(th.text_secondary).child(detail.to_string())).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{App, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, size};
    use verdant_normalized_knob::{NormalizedKnobHandle, native::gpui_native_owner::all_owners};

    fn visible_knob(window: &Window) -> NormalizedKnobHandle {
        all_owners().into_iter().filter_map(|owner| {
            let visible = {
                let state = owner.borrow();
                state.mounted && state.window_id == Some(window.window_handle().window_id())
                    && state.declared_events.contains("valueCommit")
                    && state.native_hitbox.as_ref().is_some_and(|hitbox| {
                        let center = hitbox.bounds.center();
                        center.y > px(36.) && crate::host_pointer::contains_window_point(window, (f64::from(center.x), f64::from(center.y)))
                    })
            };
            visible.then_some(NormalizedKnobHandle { owner })
        }).find(|knob| knob.state("value").unwrap().as_f64().is_some_and(|value| value < 0.8))
            .expect("a visible production knob with room for a positive drag")
    }

    fn begin(window: &mut Window, cx: &mut App) -> (NormalizedKnobHandle, gpui::Point<gpui::Pixels>, serde_json::Value) {
        let knob = visible_knob(window);
        let accepted = knob.state("value").unwrap();
        let start = knob.owner.borrow().native_hitbox.as_ref().unwrap().bounds.center();
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position: start, button: MouseButton::Left, ..Default::default() }), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent { position: start + point(px(0.), px(-36.)), pressed_button: Some(MouseButton::Left), ..Default::default() }), cx);
        assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
        (knob, start, accepted)
    }

    fn release(start: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: start + point(px(0.), px(-36.)), button: MouseButton::Left, ..Default::default() }), cx);
    }

    // GPUI's action dispatcher is crate-private. Enter the production semantic
    // callbacks used by both click routes; pointer input, subtree rendering,
    // compiled leases and project commits are real GPUI/native operations.
    #[gpui::test]
    fn page_and_section_switches_cancel_before_subtree_detach_and_late_up(cx: &mut TestAppContext) {
        let main = cx.open_window(size(px(1300.), px(900.)), |window, cx| {
            let mut ed = Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx);
            ed.needs_fit = false; ed
        });
        let (editor, target) = main.update(cx, |ed, _, cx| {
            let track = ed.session.track.clone();
            let key = crate::instrument::instrument_key_for_generation(&track, dsp::SYNTH_ID, ed.document_generation);
            ed.open_builtin_instrument_editor(&track, cx);
            (cx.entity(), Target { track: track.into(), slot: crate::instrument::INSTRUMENT_SLOT, key })
        }).unwrap();
        cx.run_until_parked();
        let native = main.update(cx, |ed, _, _| *ed.native_plugin_windows.values().next().unwrap()).unwrap();
        crate::native_test_support::frames(native, cx);
        let page = editor.update(cx, |_, cx| page_command(target.clone(), 1, cx));
        let retired = cx.update_window(native.into(), |_, window, cx| {
            let (knob, start, accepted) = begin(window, cx);
            page(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            assert!(!editor.read(cx).has_proto_pointer_contact());
            release(start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 0);
            knob
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        assert!(!retired.owner.borrow().mounted, "the old primary workspace really detached");
        let section = editor.update(cx, |_, cx| section_command(target.clone(), "音色与齐奏".into(), cx));
        cx.update_window(native.into(), |_, window, cx| section(None, window, cx)).unwrap();
        crate::native_test_support::frames(native, cx);
        let folded = cx.update_window(native.into(), |_, window, cx| {
            let (knob, start, accepted) = begin(window, cx);
            section(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            assert!(!editor.read(cx).has_proto_pointer_contact());
            release(start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 0);
            knob
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        assert!(!folded.owner.borrow().mounted, "the collapsed section really detached");
        cx.update_window(native.into(), |_, window, cx| section(None, window, cx)).unwrap();
        crate::native_test_support::frames(native, cx);
        cx.update_window(native.into(), |_, window, cx| {
            let (_, start, _) = begin(window, cx);
            release(start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 1, "the reopened production section accepts a fresh lease");
        }).unwrap();
    }
}
