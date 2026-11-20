//! Inline lyric editor: GPUI `EntityInputHandler` bridge over the pure
//! `TextEditState` (SPEC §7). Platform IME traffic arrives in UTF-16 and is
//! converted at char boundaries; marked text is drawn underlined and never
//! reaches the project until composition ends.
//! Candidate-window placement still depends on the pinned GPUI platform IME
//! implementation; these UTF-16 ranges do not establish native placement.

use std::ops::Range;

use gpui::{
    App, Bounds, Context, ElementInputHandler, Entity, EntityInputHandler, Pixels, Point, SharedString, TextRun, UTF16Selection,
    UnderlineStyle, Window, canvas, div, fill, point, prelude::*, px, size,
};

use crate::editor::Editor;
use crate::theme::with_alpha;

impl EntityInputHandler for Editor {
    fn text_for_range(&mut self,range:Range<usize>,actual:&mut Option<Range<usize>>,_:&mut Window,_:&mut Context<Self>)->Option<String>{let state=self.text_state()?;let(text,adjusted)=state.text_for_range_utf16(range);actual.replace(adjusted);Some(text)}
    fn selected_text_range(&mut self,_:bool,_:&mut Window,_:&mut Context<Self>)->Option<UTF16Selection>{let state=self.text_state()?;Some(UTF16Selection{range:state.range_to_utf16(&state.selection),reversed:state.reversed})}
    fn set_selected_text_range(&mut self,range:Range<usize>,window:&mut Window,cx:&mut Context<Self>){if let Some(state)=self.text_state_mut(){state.selection=state.range_from_utf16(&range);state.reversed=false;window.notify_text_selection_changed(&self.focus_lyric);cx.notify();}}
    fn text_length_utf16(&mut self,_:&mut Window,_:&mut Context<Self>)->Option<usize>{self.text_state().map(|s|s.text.encode_utf16().count())}
    fn marked_text_range(&self,_:&mut Window,_:&mut Context<Self>)->Option<Range<usize>>{let state=self.text_state()?;state.marked.as_ref().map(|m|state.range_to_utf16(m))}
    fn unmark_text(&mut self,window:&mut Window,cx:&mut Context<Self>){if let Some(state)=self.text_state_mut(){state.unmark_text();window.notify_text_input_changed(&self.focus_lyric);cx.notify();}}
    fn replace_text_in_range(&mut self,range:Option<Range<usize>>,text:&str,window:&mut Window,cx:&mut Context<Self>){
        if self.text_prompt.is_none()&&self.lyric.as_ref().is_some_and(|l|l.state.is_composing()){self.lyric_ime_commit(range,text,cx);}
        else if let Some(state)=self.text_state_mut(){state.replace_text_in_range(range,text);}
        window.notify_text_input_changed(&self.focus_lyric);cx.notify();
    }
    fn replace_and_mark_text_in_range(&mut self,range:Option<Range<usize>>,text:&str,selected:Option<Range<usize>>,window:&mut Window,cx:&mut Context<Self>){if let Some(state)=self.text_state_mut(){state.replace_and_mark_text_in_range(range,text,selected);window.notify_text_input_changed(&self.focus_lyric);cx.notify();}}
    fn bounds_for_range(&mut self,range:Range<usize>,bounds:Bounds<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<Bounds<Pixels>>{let state=self.text_state()?;let line=self.lyric_line.as_ref()?;if line.text.as_ref()!=state.text{return None;}let r=state.range_from_utf16(&range);Some(Bounds::from_corners(point(bounds.left()+px(6.)+line.x_for_index(r.start),bounds.top()),point(bounds.left()+px(6.)+line.x_for_index(r.end),bounds.bottom())))}
    fn character_index_for_point(&mut self,p:Point<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<usize>{let state=self.text_state()?;let local=self.lyric_bounds?.localize(&p)?;let line=self.lyric_line.as_ref()?;if line.text.as_ref()!=state.text{return None;}Some(state.offset_to_utf16(line.closest_index_for_x(local.x-px(6.))))}
}

/// Absolutely positioned editor over the note being edited. Lives *outside*
/// the `PianoRoll` key context so transport keys (Space) never fire while
/// typing; the IME owns Enter/Space/Escape while composing.
pub fn lyric_overlay(ed: &Editor, editor: &Entity<Editor>, cx: &mut Context<Editor>) -> Option<impl IntoElement + use<>> {
    let l = ed.lyric.as_ref()?;
    let (x, y, w, h) = crate::roll::note_overlay_rect(ed, &l.note_id)?;
    let width = w.max(180.0);
    let height = h.max(26.0);
    let composing = l.state.is_composing();
    let th = ed.theme;
    let e = editor.clone();

    let raw_text = l.state.text.trim();
    let base_hanzi = raw_text.split('/').next().unwrap_or("").trim();
    let hanzi_source = if base_hanzi.is_empty() {
        ed.active_track().and_then(|t| t.note(&l.note_id)).map(|n| n.lyric.as_str()).unwrap_or("")
    } else {
        base_hanzi
    };
    let hanzi_owned = hanzi_source.to_string();
    let readings = verdant_domain::hanzi_readings(hanzi_source);

    let mut quick_pills: Vec<gpui::AnyElement> = Vec::new();
    for (idx, py) in readings.iter().enumerate() {
        let full = format!("{hanzi_owned}/{py}");
        let label = if readings.len() > 1 { format!("{} · {py}", hanzi_owned) } else { py.clone() };
        quick_pills.push(
            div()
                .id(SharedString::from(format!("lyric-cand-{idx}")))
                .role(gpui::Role::Button)
                .aria_label(format!("采用读音 {full}"))
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(th.accent)
                .text_xs()
                .text_color(th.accent_text)
                .child(label)
                .on_click(cx.listener(move |ed, _, window, cx| {
                    if let Some(st) = ed.text_state_mut() {
                        *st = verdant_session::TextEditState::new(&full);
                    }
                    ed.end_lyric(true, Some(true), window, cx);
                    cx.stop_propagation();
                }))
                .into_any_element(),
        );
    }
    for (key, lbl, tpl, advance) in [
        ("sustain", "+ 延音", "+".to_string(), true),
        ("kana", "假名…", format!("{}/[ja/kana:]", if hanzi_owned.is_empty() { "ら" } else { &hanzi_owned }), false),
        ("ipa", "IPA…", format!("{}/[en:]", if hanzi_owned.is_empty() { "la" } else { &hanzi_owned }), false),
        ("next", "下一音 ⇥", String::new(), true),
    ] {
        quick_pills.push(
            div()
                .id(SharedString::from(format!("lyric-tpl-{key}")))
                .role(gpui::Role::Button)
                .px(px(5.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(th.control)
                .border_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text_secondary)
                .hover(move |s| s.bg(th.control_pressed).text_color(th.text))
                .child(lbl)
                .on_click(cx.listener(move |ed, _, window, cx| {
                    if key == "next" {
                        ed.end_lyric(true, Some(true), window, cx);
                    } else {
                        if let Some(st) = ed.text_state_mut() {
                            *st = verdant_session::TextEditState::new(&tpl);
                            if !advance && tpl.ends_with(']') {
                                let pos = tpl.len().saturating_sub(1);
                                st.selection = pos..pos;
                            }
                        }
                        if advance {
                            ed.end_lyric(true, Some(true), window, cx);
                        } else {
                            window.focus(&ed.focus_lyric, cx);
                            cx.notify();
                        }
                    }
                    cx.stop_propagation();
                }))
                .into_any_element(),
        );
    }

    Some(
        div()
            .absolute()
            .left(px(x as f32 - 2.0))
            .top(px((y - (height - h) / 2.0) as f32))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .child(
                div()
                    .w(px(width as f32))
                    .h(px(height as f32))
                    .key_context("LyricEditor")
                    .track_focus(&ed.focus_lyric)
                    .on_action(cx.listener(|ed, _: &crate::actions::LyricConfirm, window, cx| {
                        if !ed.lyric.as_ref().is_some_and(|l| l.state.is_composing()) {
                            ed.end_lyric(true, None, window, cx);
                        }
                    }))
                    .on_action(cx.listener(|ed, _: &crate::actions::LyricNext, window, cx| ed.end_lyric(true, Some(true), window, cx)))
                    .on_action(cx.listener(|ed, _: &crate::actions::LyricPrev, window, cx| ed.end_lyric(true, Some(false), window, cx)))
                    .on_action(cx.listener(|ed, _: &crate::actions::LyricCancel, window, cx| {
                        if !ed.text_state().is_some_and(|s| s.is_composing()) {
                            ed.end_lyric(false, None, window, cx);
                        }
                    }))
                    .on_action(cx.listener(|ed, _: &crate::actions::LyricSpace, _, cx| edit(ed, cx, |s| s.replace_text_in_range(None, " "))))
                    .on_action(cx.listener(|ed, _: &crate::actions::PasteText, _, cx| {
                        if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                            edit(ed, cx, |s| s.replace_text_in_range(None, &text));
                        }
                    }))
                    .on_action(cx.listener(|ed, _: &crate::actions::Backspace, _, cx| edit(ed, cx, |s| s.backspace())))
                    .on_action(cx.listener(|ed, _: &crate::actions::Delete, _, cx| edit(ed, cx, |s| s.delete())))
                    .on_action(cx.listener(|ed, _: &crate::actions::Left, _, cx| edit(ed, cx, |s| s.left())))
                    .on_action(cx.listener(|ed, _: &crate::actions::Right, _, cx| edit(ed, cx, |s| s.right())))
                    .on_action(cx.listener(|ed, _: &crate::actions::SelectLeft, _, cx| edit(ed, cx, |s| s.select_left())))
                    .on_action(cx.listener(|ed, _: &crate::actions::SelectRight, _, cx| edit(ed, cx, |s| s.select_right())))
                    .on_action(cx.listener(|ed, _: &crate::actions::Home, _, cx| edit(ed, cx, |s| s.home())))
                    .on_action(cx.listener(|ed, _: &crate::actions::End, _, cx| edit(ed, cx, |s| s.end())))
                    .on_action(cx.listener(|ed, _: &crate::actions::SelectAllText, _, cx| edit(ed, cx, |s| s.select_all())))
                    .rounded(px(9.0))
                    .bg(th.control)
                    .border_1()
                    .border_color(if composing { th.warning } else { th.accent })
                    .shadow_md()
                    .child(canvas(|_, _, _| (), move |bounds, (), window, cx| paint_lyric_field(&e, bounds, window, cx)).size_full()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(5.0))
                    .py(px(4.0))
                    .rounded(px(9.0))
                    .bg(th.popover)
                    .border_1()
                    .border_color(th.shade)
                    .shadow_md()
                    .children(quick_pills),
            ),
    )
}

fn edit(ed: &mut Editor, cx: &mut Context<Editor>, f: impl FnOnce(&mut verdant_session::TextEditState)) {
    if let Some(state) = ed.text_state_mut() {
        edit_unmarked_text(state, f);
    }
    cx.notify();
}

/// Native composition owns editing keys until the platform commits or unmarks
/// its UTF-16 range. Shared by inline lyrics and the batch input prompt.
pub(crate) fn edit_unmarked_text(state: &mut verdant_session::TextEditState, f: impl FnOnce(&mut verdant_session::TextEditState)) {
    if !state.is_composing() {
        f(state);
    }
}


/// Modal text uses the existing UTF-16/IME bridge. Confirmation is one atomic
/// document intent; composition and cancelled input never enter the journal.
pub fn prompt_overlay(ed:&Editor,editor:&Entity<Editor>,cx:&mut Context<Editor>)->Option<gpui::AnyElement>{
    let prompt=ed.text_prompt.as_ref()?;let title=prompt.title.clone();let e=editor.clone();let th=ed.theme;
    Some(div().absolute().inset_0().flex().items_center().justify_center().bg(with_alpha(th.window,0.78)).occlude()
        .child(div().id("workflow-prompt").role(gpui::Role::Dialog).aria_label(title.clone()).w(px(560.)).p(px(18.)).rounded(px(12.)).bg(th.popover).border_1().border_color(th.shade).shadow_lg().flex().flex_col().gap(px(12.))
            .key_context("TextPrompt").track_focus(&ed.focus_lyric)
            .on_action(cx.listener(|ed,_:&crate::actions::LyricConfirm,w,cx|ed.finish_prompt(true,w,cx)))
            .on_action(cx.listener(|ed,_:&crate::actions::LyricCancel,w,cx|{if !ed.text_state().is_some_and(|s|s.is_composing()){ed.finish_prompt(false,w,cx);}}))
            .on_action(cx.listener(|ed,_:&crate::actions::LyricSpace,_,cx|edit(ed,cx,|s|s.replace_text_in_range(None," "))))
            .on_action(cx.listener(|ed,_:&crate::actions::Backspace,_,cx|edit(ed,cx,|s|s.backspace())))
            .on_action(cx.listener(|ed,_:&crate::actions::Delete,_,cx|edit(ed,cx,|s|s.delete())))
            .on_action(cx.listener(|ed,_:&crate::actions::Left,_,cx|edit(ed,cx,|s|s.left())))
            .on_action(cx.listener(|ed,_:&crate::actions::Right,_,cx|edit(ed,cx,|s|s.right())))
            .on_action(cx.listener(|ed,_:&crate::actions::SelectLeft,_,cx|edit(ed,cx,|s|s.select_left())))
            .on_action(cx.listener(|ed,_:&crate::actions::SelectRight,_,cx|edit(ed,cx,|s|s.select_right())))
            .on_action(cx.listener(|ed,_:&crate::actions::Home,_,cx|edit(ed,cx,|s|s.home())))
            .on_action(cx.listener(|ed,_:&crate::actions::End,_,cx|edit(ed,cx,|s|s.end())))
            .on_action(cx.listener(|ed,_:&crate::actions::SelectAllText,_,cx|edit(ed,cx,|s|s.select_all())))
            .on_action(cx.listener(|ed,_:&crate::actions::PasteText,_,cx|{if let Some(text)=cx.read_from_clipboard().and_then(|c|c.text()){edit(ed,cx,|s|s.replace_text_in_range(None,&text));}}))
            .child(div().text_sm().child(title))
            .children(ed.voice_import_details(cx))
            .child(div().id("workflow-prompt-input").role(gpui::Role::TextInput).h(px(36.)).w_full().overflow_hidden().bg(th.control).border_1().border_color(th.accent).child(canvas(|_,_,_|(),move|bounds,(),window,cx|paint_lyric_field(&e,bounds,window,cx)).size_full()))
            .child(div().flex().justify_end().gap(px(8.))
                .child(div().id("workflow-prompt-cancel").role(gpui::Role::Button).aria_label("取消").p(px(6.)).child("取消 · Esc").on_click(cx.listener(|ed,_,w,cx|ed.finish_prompt(false,w,cx))))
                .child(div().id("workflow-prompt-confirm").role(gpui::Role::Button).aria_label("应用").p(px(6.)).bg(th.control).child("应用 · Enter").on_click(cx.listener(|ed,_,w,cx|ed.finish_prompt(true,w,cx))))))
        .into_any_element())
}

fn paint_lyric_field(editor: &Entity<Editor>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let (state, focus, th) = {
        let ed = editor.read(cx);
        let Some(state) = ed.text_state() else { return };
        (state.clone(), ed.focus_lyric.clone(), ed.theme)
    };
    window.handle_input(&focus, ElementInputHandler::new(bounds, editor.clone()), cx);
    let style = window.text_style();
    let font_size = px(15.0);
    let placeholder = state.text.is_empty();
    let shown: SharedString = if placeholder { "输入文字…".into() } else { state.text.clone().into() };
    let base = TextRun {
        len: shown.len(),
        font: style.font(),
        color: if placeholder { th.text_tertiary.into() } else { th.text.into() },
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let runs: Vec<TextRun> = match (&state.marked, placeholder) {
        (Some(m), false) => [
            TextRun { len: m.start, ..base.clone() },
            TextRun {
                len: m.end - m.start,
                underline: Some(UnderlineStyle { color: Some(th.warning.into()), thickness: px(1.5), wavy: false }),
                color: th.text.into(),
                ..base.clone()
            },
            TextRun { len: shown.len() - m.end, ..base.clone() },
        ]
        .into_iter()
        .filter(|r| r.len > 0)
        .collect(),
        _ => vec![base],
    };
    let line = window.text_system().shape_line(shown, font_size, &runs, None);
    let line_h = px(20.0);
    let origin = point(bounds.left() + px(6.0), bounds.top() + (bounds.size.height - line_h) / 2.0);
    if !placeholder && !state.selection.is_empty() {
        let x0 = line.x_for_index(state.selection.start);
        let x1 = line.x_for_index(state.selection.end);
        window.paint_quad(fill(Bounds::new(point(origin.x + x0, origin.y), size(x1 - x0, line_h)), with_alpha(th.accent, 0.3)));
    }
    let _ = line.paint(origin, line_h, gpui::TextAlign::Left, None, window, cx);
    if focus.is_focused(window) && state.selection.is_empty() {
        let cx_ = if placeholder { px(0.0) } else { line.x_for_index(state.cursor()) };
        window.paint_quad(fill(Bounds::new(point(origin.x + cx_, origin.y + px(1.0)), size(px(1.5), line_h - px(2.0))), th.accent));
    }
    let shaped = if placeholder { None } else { Some(line) };
    editor.update(cx, |ed, _| {
        ed.lyric_line = shaped;
        ed.lyric_bounds = Some(bounds);
    });
}
