//! Offline software attribution, research references and interface language.
//!
//! License and NOTICE content is included verbatim from the shipped files.
//! Application prose is resolved through the bilingual catalogue. Reopening
//! focuses the existing native window; the live editor owns persisted prefs.

use std::cell::RefCell;
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, FontWeight, IntoElement,
    KeyDownEvent, Render, Role, ScrollHandle, WeakEntity, Window, WindowBounds,
    WindowHandle, WindowOptions, div, prelude::*, px, size,
};

#[path = "i18n.rs"]
pub(crate) mod i18n;

use i18n::{Key, Locale, Surface};
use crate::editor::Editor;
use crate::theme::Theme;

thread_local! {
    static OPEN: RefCell<Option<WindowHandle<Disclosures>>> = const { RefCell::new(None) };
}

const VERDANT_LICENSE: &str = include_str!("../../../../LICENSE");
const THIRD_PARTY_NOTICE: &str = include_str!("../../../../NOTICE.md");
const LUCIDE_LICENSE: &str = include_str!("../assets/icons/LICENSE-lucide.txt");
const PROTO_UI_LICENSE: &str = include_str!("../../../../spec/licenses/Proto-UI-LICENSE.txt");
const GPUI_LICENSE: &str = include_str!("../../../../spec/licenses/GPUI-LICENSE-APACHE.txt");
const DIFFSINGER_LICENSE: &str = include_str!("../../../../spec/licenses/DiffSinger-LICENSE.txt");
const OPENVPI_NOTICE: &str = include_str!("../assets/licenses/OpenVPI-NOTICE.md");

const SOFTWARE_AND_DATA: &[(Key, Key)] = &[
    (Key::VerdantName, Key::VerdantNote),
    (Key::ProtoName, Key::ProtoNote),
    (Key::LucideName, Key::LucideNote),
    (Key::OriginalModelName, Key::OriginalModelNote),
    (Key::PinyinName, Key::PinyinNote),
    (Key::DiffSingerName, Key::DiffSingerNote),
    (Key::EnvironmentName, Key::EnvironmentNote),
    (Key::L8nName, Key::L8nNote),
];

const RESEARCH_REFERENCES: &[(Key, Key)] = &[
    (Key::OpenVpiName, Key::OpenVpiNote),
    (Key::DurationName, Key::DurationNote),
    (Key::BaselineName, Key::BaselineNote),
    (Key::SifiName, Key::SifiNote),
    (Key::ApbweName, Key::ApbweNote),
    (Key::LseName, Key::LseNote),
    (Key::DitName, Key::DitNote),
    (Key::ExcitationName, Key::ExcitationNote),
    (Key::VotName, Key::VotNote),
    (Key::MlpgName, Key::MlpgNote),
    (Key::HifiName, Key::HifiNote),
    (Key::WorldName, Key::WorldNote),
    (Key::AuditoryName, Key::AuditoryNote),
    (Key::RepresentationName, Key::RepresentationNote),
    (Key::VocosName, Key::VocosNote),
    (Key::RosenbergName, Key::RosenbergNote),
    (Key::DdspName, Key::DdspNote),
    (Key::PcDdspName, Key::PcDdspNote),
    (Key::GolfName, Key::GolfNote),
];

fn heading(text: &'static str, th: Theme) -> gpui::Div {
    div().flex_none().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(th.text).child(text)
}

fn body(text: impl Into<gpui::SharedString>, th: Theme) -> gpui::Div {
    div().flex_none().text_xs().text_color(th.text_secondary).child(text.into())
}

fn section(title: Key, entries: &[(Key, Key)], locale: Locale, th: Theme) -> gpui::Div {
    let mut d = div().flex_none().flex().flex_col().gap(px(6.0)).child(heading(locale.text(title), th));
    for &(name, note) in entries {
        d = d.child(
            div().flex_none().flex().flex_col().gap(px(2.0))
                .child(div().flex_none().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(th.text).child(locale.text(name)))
                .child(body(locale.text(note), th)),
        );
    }
    d
}

fn license_block(title: &'static str, text: &'static str, th: Theme) -> gpui::Div {
    div().flex_none().flex().flex_col().gap(px(4.0))
        .child(div().flex_none().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(th.text).child(title))
        .child(
            div().flex_none().w_full().p(px(8.0)).rounded(px(5.0)).bg(th.control).border_1().border_color(th.shade)
                .text_xs().text_color(th.text_secondary).font_family("monospace")
                .child(text),
        )
}

pub struct Disclosures {
    font: gpui::SharedString,
    focus: FocusHandle,
    scroll: ScrollHandle,
    editor: Option<WeakEntity<Editor>>,
    save_error: std::option::Option<std::string::String>,
}

/// The existing Help action opens this while the editor/window are borrowed.
/// Resolve its single editor owner after that callback returns, so the language
/// controls use live preferences and remain reachable through the existing menu.
pub fn open(cx: &mut App) {
    cx.defer(|cx| {
        let editor = cx.windows().into_iter().find_map(|window| {
            window.downcast::<Editor>().and_then(|window| window.entity(cx).ok())
        });
        open_window(editor.map(|editor| editor.downgrade()), cx);
    });
}

/// Main editor consumer: language buttons update the live prefs before saving.
pub fn open_for_editor(editor: Entity<Editor>, cx: &mut App) {
    open_window(Some(editor.downgrade()), cx);
}

fn open_window(editor: Option<WeakEntity<Editor>>, cx: &mut App) {
    if let Some(h) = OPEN.with(|o| *o.borrow()) {
        let owner = editor.clone();
        if h.update(cx, |view, window, cx| {
            if owner.is_some() { view.editor = owner; }
            window.activate_window();
            cx.notify();
        }).is_ok() {
            return;
        }
    }
    // A standalone attribution window can be opened before the main editor.
    if cx.try_global::<Locale>().is_none() { i18n::initialize(&crate::files::load_prefs(), cx); }
    let font: gpui::SharedString = crate::theme::system_ui_font(&cx.text_system().all_font_names()).into();
    let bounds = Bounds::centered(None, size(px(680.), px(700.)), cx);
    if let Ok(h) = cx.open_window(
        WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default() },
        |window, cx| {
            window.set_window_title(i18n::current(cx).text(Key::DisclosuresWindowTitle));
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                cx.observe_global::<Locale>(|_, cx| cx.notify()).detach();
                Disclosures { font, focus, scroll: ScrollHandle::default(), editor, save_error: None }
            })
        },
    ) {
        OPEN.with(|o| *o.borrow_mut() = Some(h));
    }
}

impl Disclosures {
    fn choose_locale(&mut self, locale: Locale, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.as_ref().and_then(WeakEntity::upgrade) else { return; };
        match editor.update(cx, |ed, cx| {
            i18n::set_locale(&mut ed.prefs, locale, cx)?;
            cx.notify();
            Ok::<_, String>(())
        }) {
            Ok(()) => self.save_error = None,
            Err(error) => self.save_error = Some(error),
        }
        cx.notify();
    }
}

impl Render for Disclosures {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = Theme::for_appearance(window.appearance());
        let locale = i18n::current(cx);
        let semantics = Surface::Disclosures.semantics(locale);
        window.set_window_title(locale.text(Key::DisclosuresWindowTitle));
        let mut content = div()
            .id("disclosures")
            .role(semantics.role)
            .aria_label(semantics.label)
            .aria_description(semantics.description)
            .track_focus(&self.focus)
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift { window.focus_prev(cx); }
                    else { window.focus_next(cx); }
                    cx.stop_propagation();
                    return;
                }
                // Focused language buttons retain their native Space/Enter behavior.
                if event.keystroke.key == "space" && !view.focus.is_focused(window) { return; }
                let mut offset = view.scroll.offset();
                let limit = view.scroll.max_offset().y;
                let page = window.viewport_size().height * 0.8;
                offset.y = match event.keystroke.key.as_str() {
                    "up" => offset.y + px(40.),
                    "down" => offset.y - px(40.),
                    "pageup" => offset.y + page,
                    "pagedown" | "space" => offset.y - page,
                    "home" => px(0.),
                    "end" => -limit,
                    _ => return,
                }.clamp(-limit, px(0.));
                view.scroll.set_offset(offset);
                cx.stop_propagation();
                cx.notify();
            }))
            .bg(th.surface)
            .text_color(th.text)
            .font_family(self.font.clone())
            .p(px(20.0))
            .flex()
            .flex_col()
            .gap(px(18.0));
        if self.editor.as_ref().and_then(WeakEntity::upgrade).is_some() {
            content = content.child(
                div().id("interface-language").role(Role::Group).aria_label(locale.text(Key::Language))
                    .flex_none().flex().flex_col().gap(px(6.0))
                    .child(heading(locale.text(Key::Language), th))
                    .child(body(locale.text(Key::LanguageHelp), th))
                    .child(div().flex().gap(px(8.0)).children(Locale::ALL.into_iter().map(|choice| {
                        div().id(choice.code()).role(Role::Button).aria_label(choice.native_name())
                            .aria_selected(choice == locale).tab_index(0)
                            .px(px(10.0)).py(px(6.0)).rounded(px(4.0)).border_1().border_color(th.shade)
                            .bg(if choice == locale { th.control_pressed } else { th.control })
                            .focus(|style| style.border_color(th.accent))
                            .text_sm().child(choice.native_name())
                            .on_click(cx.listener(move |view, _, _, cx| view.choose_locale(choice, cx)))
                            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    view.choose_locale(choice, cx);
                                    cx.stop_propagation();
                                }
                            }))
                    }))),
            );
        }
        if let Some(error) = &self.save_error {
            content = content.child(div().id("language-save-error").role(Role::Alert).text_sm().text_color(th.error)
                .child(format!("{}: {error}", locale.text(Key::LanguageSaveFailed))));
        }
        content
            .child(heading(locale.text(Key::DisclosuresTitle), th))
            .child(body(locale.text(Key::DisclosuresIntro), th))
            .child(section(Key::SoftwareHeading, SOFTWARE_AND_DATA, locale, th))
            .child(section(Key::ResearchHeading, RESEARCH_REFERENCES, locale, th))
            .child(heading(locale.text(Key::LicensesHeading), th))
            .child(license_block(locale.text(Key::NoticeLicense), THIRD_PARTY_NOTICE, th))
            .child(license_block(locale.text(Key::LucideLicense), LUCIDE_LICENSE, th))
            .child(license_block(locale.text(Key::ProtoLicense), PROTO_UI_LICENSE, th))
            .child(license_block(locale.text(Key::GpuiLicense), GPUI_LICENSE, th))
            .child(license_block(locale.text(Key::DiffSingerLicense), DIFFSINGER_LICENSE, th))
            .child(license_block(locale.text(Key::OpenVpiLicense), OPENVPI_NOTICE, th))
            .child(license_block(locale.text(Key::VerdantLicense), VERDANT_LICENSE, th))
    }
}
