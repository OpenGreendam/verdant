//! Inline lyrics and modal prompts host the actual compiled TextField editor.
//! Their session text is a domain draft mirror only: the native Proto-UI
//! TextControl owns cursor/selection, IME, clipboard and editing history.

use gpui::{Context, Entity, SharedString, Window, div, prelude::*, px};
use crate::{editor::Editor, proto_text::{self, Commands, Event, Options}, theme::with_alpha};

fn mirror(state: &mut verdant_session::TextEditState, value: &str, composing: bool) {
    state.text = value.into();
    // The domain gate needs only whether text is provisional. This is not a
    // copy of the native editor's selection or its actual marked range.
    state.marked = composing.then_some(0..value.len());
    state.selection = value.len()..value.len();
    state.reversed = false;
}

/// The compiled input proposes a completed lyric; the existing document
/// transaction still owns revision validation, pinyin parsing and undo.
fn lyric_event(ed: &mut Editor, event: Event, window: &mut Window, cx: &mut Context<Editor>) {
    match event {
        Event::Changed { value, composing } => {
            if let Some(lyric) = &mut ed.lyric { mirror(&mut lyric.state, &value, composing); }
            cx.notify();
        }
        Event::Replaced { value } => {
            if let Some(lyric) = &mut ed.lyric { mirror(&mut lyric.state, &value, false); }
            cx.notify();
        }
        Event::CompositionEnd { value } => {
            let Some(lyric) = &mut ed.lyric else { return };
            mirror(&mut lyric.state, &value, false);
            let id = lyric.note_id.clone();
            let mut accepted = lyric.clone();
            if let verdant_session::LyricOutcome::Commit(text) = accepted.confirm() {
                if ed.commit_lyric_text(&id, &text, cx) {
                    if let Some(current) = &mut ed.lyric {
                        if current.note_id == id && current.state.text == text { current.confirm(); }
                    }
                }
            }
            cx.notify();
        }
        Event::Submit { value } => {
            if let Some(lyric) = &mut ed.lyric { mirror(&mut lyric.state, &value, false); }
            ed.end_lyric(true, None, window, cx);
        }
        Event::Blur { value } => {
            if let Some(lyric) = &mut ed.lyric { mirror(&mut lyric.state, &value, false); }
            // Source blur belongs to the textbox. A pronunciation/template
            // button is still inside this application editing region.
            if !ed.focus_lyric.contains_focused(window, cx) { ed.end_lyric(true, None, window, cx); }
        }
        Event::Advance { value, forward } => {
            if let Some(lyric) = &mut ed.lyric { mirror(&mut lyric.state, &value, false); }
            ed.end_lyric(true, Some(forward), window, cx);
        }
        Event::Cancel => ed.end_lyric(false, None, window, cx),
        // This editor does not enable the search collection policy.
        Event::Navigation { .. } => {},
        Event::Failure(error) => { ed.notice(error); cx.notify(); },
    }
}

pub fn lyric_overlay(ed: &Editor, editor: &Entity<Editor>, _cx: &mut Context<Editor>) -> Option<impl IntoElement + use<>> {
    let lyric = ed.lyric.as_ref()?;
    let (x, y, w, h) = crate::roll::note_overlay_rect(ed, &lyric.note_id)?;
    let (width, height) = (w.max(180.0), h.max(26.0));
    let composing = lyric.state.is_composing();
    let th = ed.theme;
    let owner = editor.downgrade();
    let generation = ed.document_generation;
    let track = lyric.track.clone();
    let note = lyric.note_id.clone();
    let key = SharedString::from(format!("lyric-text-{generation}-{track}-{note}"));
    let commands = Commands::default();
    let field = proto_text::inline(key, "输入歌词或读音…", lyric.state.text.clone(), th,
        Options { auto_focus: true, select_all: true, submit_on_enter: true, cancel_on_escape: true,
            advance_on_tab: true, commit_on_blur: true, inline: true, numeric: None, search_navigation: false },
        move |event, window, cx| {
            let _ = owner.update(cx, |ed, cx| {
                if ed.document_generation != generation || ed.text_prompt.is_some()
                    || !ed.lyric.as_ref().is_some_and(|lyric| lyric.track == track && lyric.note_id == note) { return; }
                lyric_event(ed, event, window, cx);
            });
        }).commands(commands.clone());

    let raw_text = lyric.state.text.trim();
    let hanzi = raw_text.split('/').next().unwrap_or("").trim();
    let hanzi = if hanzi.is_empty() { ed.active_track().and_then(|t| t.note(&lyric.note_id)).map(|n| n.lyric.as_str()).unwrap_or("") } else { hanzi };
    let readings = verdant_domain::hanzi_readings(hanzi);
    let mut pills = Vec::new();
    for (index, py) in readings.iter().enumerate() {
        let full = format!("{hanzi}/{py}");
        let label = if readings.len() > 1 { format!("{hanzi} · {py}") } else { py.clone() };
        let commands = commands.clone();
        pills.push(crate::proto_controls::action(SharedString::from(format!("lyric-cand-{index}")), format!("采用读音 {full}"))
            .disabled(composing).px(px(6.)).py(px(2.)).rounded(px(4.)).bg(th.accent).text_xs().text_color(th.accent_text).child(label)
            .on_click(move |_, window, cx| { commands.replace(&full, None, window, cx); commands.invoke("requestAdvance", vec![true.into()], window, cx); })
            .into_any_element());
    }
    for (key, label, template, advance) in [
        ("sustain", "+ 延音", "+".to_string(), true),
        ("kana", "假名…", format!("{}/[ja/kana:]", if hanzi.is_empty() { "ら" } else { hanzi }), false),
        ("ipa", "IPA…", format!("{}/[en:]", if hanzi.is_empty() { "la" } else { hanzi }), false),
        ("next", "下一音 ⇥", String::new(), true),
    ] {
        let commands = commands.clone();
        pills.push(crate::proto_controls::action(SharedString::from(format!("lyric-tpl-{key}")), label)
            .disabled(composing).px(px(5.)).py(px(2.)).rounded(px(4.)).bg(th.control).border_1().border_color(th.shade)
            .text_xs().text_color(th.text_secondary).hover(move |s| s.bg(th.control_pressed).text_color(th.text)).child(label)
            .on_click(move |_, window, cx| {
                if key != "next" {
                    let position = template.encode_utf16().count().saturating_sub(1);
                    commands.replace(&template, (!advance).then_some(position..position), window, cx);
                }
                if advance { commands.invoke("requestAdvance", vec![true.into()], window, cx); }
            }).into_any_element());
    }
    Some(div().absolute().left(px(x as f32 - 2.)).top(px((y - (height - h) / 2.) as f32))
        .key_context("LyricEditor").track_focus(&ed.focus_lyric).flex().flex_col().gap(px(3.))
        .child(div().w(px(width as f32)).h(px(height as f32)).rounded(px(9.)).bg(th.control).border_1()
            .border_color(if composing { th.warning } else { th.accent }).shadow_md().child(field))
        .child(div().flex().flex_wrap().items_center().gap(px(4.)).px(px(5.)).py(px(4.)).rounded(px(9.))
            .bg(th.popover).border_1().border_color(th.shade).shadow_md().children(pills)))
}

/// Prompt confirmation is one domain intent. Text composition and editing
/// shortcuts remain entirely in the same generated editor used for lyrics.
pub fn prompt_overlay(ed: &Editor, editor: &Entity<Editor>, _cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let prompt = ed.text_prompt.as_ref()?;
    let revision = format!("{}:{}:{}", ed.document_generation, prompt.base.get(), prompt.instance);
    let props = serde_json::json!({"label":prompt.title, "role":"dialog", "open":true,
        "revision":revision, "choices":2, "defaultChoice":1, "cancelChoice":0});
    let commands = Commands::default();
    let draw_commands = commands.clone(); let draw_editor = editor.clone();
    let generation = ed.document_generation; let instance = prompt.instance.clone();
    let draw: crate::proto_overlay::Draw = std::rc::Rc::new(move |_, focus, _, cx| {
        draw_editor.update(cx, |ed, cx| {
            if ed.document_generation != generation || !ed.text_prompt.as_ref().is_some_and(|prompt| prompt.instance == instance) {
                return div().into_any_element();
            }
            ed.focus_lyric = focus.clone();
            native_prompt_overlay(ed, &cx.entity(), draw_commands.clone(), cx).unwrap_or_else(|| div().into_any_element())
        })
    });
    let response: crate::proto_overlay::Response = std::rc::Rc::new(move |index, window, cx| {
        // Even a response from a sibling button goes through the same compiled
        // input. Its current composition and view epoch guard the domain commit.
        commands.invoke(if index == 0 { "requestCancel" } else { "requestSubmit" }, vec![], window, cx);
    });
    Some(crate::proto_overlay::overlay(SharedString::from(format!("compiled-workflow-prompt-{}", prompt.instance)),
        props, draw, response).into_any_element())
}

fn native_prompt_overlay(ed: &Editor, editor: &Entity<Editor>, commands: Commands, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let prompt = ed.text_prompt.as_ref()?;
    let th = ed.theme;
    let title = prompt.title.clone();
    let generation = ed.document_generation;
    let base = prompt.base;
    let instance = prompt.instance.clone();
    let key = SharedString::from(format!("prompt-text-{generation}-{instance}"));
    let owner = editor.downgrade();
    let field = proto_text::inline(key, "输入文字…", prompt.state.text.clone(), th,
        Options { auto_focus: true, select_all: true, submit_on_enter: true, cancel_on_escape: true,
            inline: true, ..Default::default() }, move |event, window, cx| {
            let _ = owner.update(cx, |ed, cx| {
                if ed.document_generation != generation || !ed.text_prompt.as_ref().is_some_and(|p| p.base == base && p.instance == instance) { return; }
                match event {
                    Event::Changed { value, composing } => { if let Some(p) = &mut ed.text_prompt { mirror(&mut p.state, &value, composing); } cx.notify(); },
                    Event::Replaced { value } | Event::CompositionEnd { value } => { if let Some(p) = &mut ed.text_prompt { mirror(&mut p.state, &value, false); } cx.notify(); },
                    Event::Submit { value } => { if let Some(p) = &mut ed.text_prompt { mirror(&mut p.state, &value, false); } ed.finish_prompt(true, window, cx); },
                    Event::Cancel => ed.finish_prompt(false, window, cx),
                    Event::Failure(error) => { ed.notice(error); cx.notify(); },
                    _ => {},
                }
            });
        }).commands(commands.clone());
    let cancel = commands.clone();
    let composing = prompt.state.is_composing();
    Some(div().absolute().inset_0().flex().items_center().justify_center().bg(with_alpha(th.window, 0.78)).occlude()
        .child(div().id("workflow-prompt").w(px(560.)).p(px(18.)).rounded(px(12.))
            .bg(th.popover).border_1().border_color(th.shade).shadow_lg().flex().flex_col().gap(px(12.))
            .key_context("TextPrompt")
            .child(div().text_sm().child(title)).children(ed.voice_import_details(cx))
            .child(div().h(px(36.)).w_full().overflow_hidden().bg(th.control).border_1().border_color(th.accent).child(field))
            .child(div().flex().justify_end().gap(px(8.))
                .child(crate::proto_controls::action("workflow-prompt-cancel", "取消").disabled(composing).p(px(6.)).child("取消 · Esc")
                    .on_click(move |_, window, cx| cancel.invoke("requestCancel", vec![], window, cx)))
                .child(crate::proto_controls::action("workflow-prompt-confirm", "应用").disabled(composing).p(px(6.)).bg(th.control).child("应用 · Enter")
                    .on_click(move |_, window, cx| commands.invoke("requestSubmit", vec![], window, cx)))))
        .into_any_element())
}

/// Retained only for pure domain tests; no live UI input route calls this.
#[cfg(test)]
pub(crate) fn edit_unmarked_text(state: &mut verdant_session::TextEditState, edit: impl FnOnce(&mut verdant_session::TextEditState)) {
    if !state.is_composing() { edit(state); }
}


#[cfg(test)]
mod prompt_instance_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, WindowHandle, size};
    use crate::editor::PromptPurpose;
    use verdant_text_field::{TextFieldHandle,native::gpui_native_owner::all_owners};

    struct Harness { editor: Entity<Editor> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let content=self.editor.update(cx,|editor,cx|prompt_overlay(editor,&cx.entity(),cx));
            div().size_full().children(content)
        }
    }
    fn text(window: &Window) -> TextFieldHandle {
        let owner=all_owners().into_iter().find(|owner|{
            let owner=owner.borrow();owner.present&&owner.mounted&&!owner.disposed
                &&owner.window_id==Some(window.window_handle().window_id())&&owner.exposes.contains_key("viewEpoch")
        }).expect("mounted compiled prompt editor");
        TextFieldHandle { owner }
    }
    fn frames(window: WindowHandle<Harness>,cx:&mut TestAppContext){crate::native_test_support::frames(window,cx);}

    #[gpui::test]
    fn reopened_prompt_rejects_the_still_mounted_previous_instances_submit(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(720.),px(320.)),|window,cx|{
            let editor=cx.new(|cx|Editor::for_native_test(crate::native_test_support::instrument_project(),window,cx));
            cx.observe(&editor,|_:&mut Harness,_,cx|cx.notify()).detach();
            Harness{editor}
        });
        window.update(cx,|host,window,cx|{
            window.activate_window();
            host.editor.update(cx,|editor,cx|{
                let track=editor.session.track.clone();
                editor.prompt_text("Track name",PromptPurpose::RenameTrack(track),"First draft",window,cx);
            });
        }).unwrap();frames(window,cx);
        window.update(cx,|host,window,cx|{
            let previous=text(window);
            host.editor.update(cx,|editor,cx|{
                let old=editor.text_prompt.as_ref().unwrap().instance.clone();
                let base=editor.text_prompt.as_ref().unwrap().base;
                let track=editor.session.track.clone();
                editor.finish_prompt(false,window,cx);
                editor.prompt_text("Track name",PromptPurpose::RenameTrack(track),"Replacement draft",window,cx);
                let prompt=editor.text_prompt.as_ref().unwrap();
                assert_ne!(prompt.instance,old);assert_eq!(prompt.base,base);
            });
            // The next layout has not removed the old physical root yet.
            // Its accepted source response must still fail the prompt identity guard.
            assert_eq!(previous.call("requestSubmit",vec![],window,cx).unwrap(),serde_json::json!(true));
        }).unwrap();frames(window,cx);
        window.update(cx,|host,_,cx|{
            let editor=host.editor.read(cx);
            assert_eq!(editor.text_prompt.as_ref().unwrap().state.text,"Replacement draft");
            assert_eq!(editor.revision().get(),0);
        }).unwrap();
        cx.simulate_input(window.into(),"Accepted name");cx.simulate_keystrokes(window.into(),"enter");frames(window,cx);
        window.update(cx,|host,_,cx|{
            let editor=host.editor.read(cx);assert!(editor.text_prompt.is_none());
            assert_eq!(editor.project().track(&editor.session.track).unwrap().name,"Accepted name");
            assert_eq!(editor.revision().get(),1);
        }).unwrap();
    }
}
