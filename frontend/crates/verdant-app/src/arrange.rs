//! Arrangement painting uses revision-cached pitch/span summaries and a
//! prefix-end index for visible regions. Mini notes use the domain's bounded
//! time-range query. Playback lines and meters are separate retained views.
//! Audio selection exposes split-at-playhead, join-right, duplicate and mute
//! buttons in its track header. Ctrl-click splits at the snapped cursor tick.
//! Drag the upper fade handles for linear fades; drag the other edge areas to
//! trim. Preview-only drags commit once on release, including cross-track moves.
//! Horizontal scrolling and editor navigation use the session viewport bounds.
use std::collections::{HashMap, HashSet};
use gpui::{App, BorderStyle, Bounds, Context, DispatchPhase, Entity, FontWeight, Hitbox, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TouchDragEvent, TouchPhase, PinchEvent, Pixels, ScrollDelta, ScrollEvent, SharedString, Window, canvas, div, fill, prelude::*, px, quad};
use verdant_backend::{PhrasePlan, SlotStatus};
use verdant_domain::{Project,Revision,TrackId,Command,CommandBatch};
use verdant_session::PointerId;
use crate::editor::{Bottom, Editor};
use crate::generated::controls::{CommandArg, CommandId};
use crate::paint::{Texts, b, flush_texts};
use crate::theme::{mix, with_alpha};

pub const HEADER_W: f32 = 228.0;

#[derive(Clone, Copy)]
enum RegionKind { Phrase(usize), Blocked(usize), Pending }
struct Region { start: i64, end: i64, kind: RegionKind }
#[derive(Clone)]
enum AuthoredTarget{Part(String),Audio(String)}
#[derive(Clone)]
struct AuthoredRegion{start:i64,end:i64,target:AuthoredTarget,label:String,muted:bool,fades:Option<AudioFades>,clip:Option<std::sync::Arc<verdant_media::AudioClip>>}
#[derive(Clone,Copy)]
struct AudioFades{duration:u64,fade_in:u64,fade_out:u64,rate:f64}
struct TrackPaintIndex {
    authored:Vec<AuthoredRegion>,
    low: i32,
    high: i32,
    start: i64,
    end: i64,
    regions: Vec<Region>,
    prefix_ends: Vec<i64>,
    /// Diagnostic summaries parallel to the `Blocked` kinds above: a plain
    /// "contains errors" label never told the user what to fix.
    blocked_labels: Vec<String>,
}
#[derive(Default)]
pub struct ArrangeCache {
    project: String,
    revision: u64,
    tracks: HashMap<TrackId, TrackPaintIndex>,
}
impl ArrangeCache {
    pub fn end_tick(&self)->i64{self.tracks.values().flat_map(|track|track.authored.iter().map(|region|region.end).chain(std::iter::once(track.end))).max().unwrap_or(0)}
    pub fn rebuild(&mut self, project: &Project, revision: Revision, plans: Option<&[PhrasePlan]>) {
        if self.project != project.project_id || self.revision != revision.get() {
            self.tracks.clear();
            let parts=verdant_domain::parts::Parts::load(project).ok();
            let tempo=verdant_domain::TempoMap::new(&project.tempo_map,project.ppq).ok();
            for track in &project.tracks {
                let (low, high, end) = track.notes.iter().fold((127, 0, 0), |(low, high, end), n| (low.min(n.midi_note), high.max(n.midi_note), end.max(n.end_tick())));
                let mut authored=Vec::new();
                if let Some(parts)=&parts{for part in parts.instances.iter().filter(|part|part.track==track.id){let linked=parts.instances.iter().filter(|other|other.content_id==part.content_id).count()>1;authored.push(AuthoredRegion{start:part.start_tick,end:part.start_tick+part.duration_tick,target:AuthoredTarget::Part(part.id.clone()),label:format!("{}{}",if linked{"↔ "}else{""},part.name),muted:part.muted,fades:None,clip:None});}}
                if let(Ok(clips),Some(tempo))=(verdant_media::read_clips(track),&tempo){for clip in clips{
                    let end=clip.end_tick(tempo).ceil()as i64;
                    let name=if clip.name.is_empty(){std::path::Path::new(&clip.source.path).file_name().unwrap_or_default().to_string_lossy().into_owned()}else{clip.name.clone()};
                    let badge=format!("{}{}",if clip.reversed{" ↶"}else{""},if clip.playback_rate!=1.{format!(" · {:.2}×",clip.playback_rate)}else{String::new()});
                    authored.push(AuthoredRegion{start:clip.start_tick,end,target:AuthoredTarget::Audio(clip.id.to_string()),label:format!("♫ {name}{badge}"),muted:clip.muted,fades:Some(AudioFades{duration:clip.duration_frames,fade_in:clip.fade_in_frames,fade_out:clip.fade_out_frames,rate:clip.source.sample_rate as f64*clip.playback_rate}),clip:Some(std::sync::Arc::new(clip))});
                }}
                self.tracks.insert(track.id.clone(), TrackPaintIndex { authored,low, high, start: track.notes.first().map(|n| n.start_tick).unwrap_or(0), end,
                    regions: Vec::new(), prefix_ends: Vec::new(), blocked_labels: Vec::new() });
            }
            self.project = project.project_id.clone(); self.revision = revision.get();
        }
        for track in &project.tracks {
            let Some(index) = self.tracks.get_mut(&track.id) else { continue };
            index.regions.clear(); index.prefix_ends.clear(); index.blocked_labels.clear();
            if let Some(plan) = plans.and_then(|plans| plans.iter().find(|p| p.track == track.id)) {
                // Each source list is already ordered. Merge once at plan time;
                // painting never sorts spans or clones complete note-id lists.
                let (mut phrases, mut blocked) = (plan.phrases.iter().peekable(), plan.blocked.iter().peekable());
                while phrases.peek().is_some() || blocked.peek().is_some() {
                    let take_phrase = match (phrases.peek(), blocked.peek()) { (Some(a), Some(b)) => a.start_tick <= b.start_tick, (Some(_), None) => true, _ => false };
                    if take_phrase {
                        let p = phrases.next().expect("peeked phrase"); index.regions.push(Region { start: p.start_tick, end: p.end_tick, kind: RegionKind::Phrase(p.index) });
                    } else {
                        let p = blocked.next().expect("peeked blocked span");
                        let note_ids: HashSet<_> = p.note_ids.iter().collect();
                        let errors: Vec<_> = plan.diagnostics.iter()
                            .filter(|d| d.severity == verdant_backend::Severity::Error)
                            .filter(|d| d.note_id.as_ref().is_none_or(|id| note_ids.contains(id)))
                            .collect();
                        let label = match errors.first() {
                            Some(first) if errors.len() > 1 => format!("预检未通过 · {} 处错误 · {}", errors.len(), first.message),
                            Some(first) => format!("预检未通过 · {}", first.message),
                            None => "预检未通过 · 含错误".into(),
                        };
                        index.blocked_labels.push(label);
                        index.regions.push(Region { start: p.start_tick, end: p.end_tick, kind: RegionKind::Blocked(index.blocked_labels.len() - 1) });
                    }
                }
                if index.regions.is_empty() && !track.notes.is_empty() {
                    let errors: Vec<_> = plan.diagnostics.iter()
                        .filter(|d| d.severity == verdant_backend::Severity::Error)
                        .collect();
                    if let Some(first) = errors.first() {
                        let label = if errors.len() > 1 {
                            format!("预检未通过 · {} 处错误 · {}", errors.len(), first.message)
                        } else {
                            format!("预检未通过 · {}", first.message)
                        };
                        index.blocked_labels.push(label);
                        index.regions.push(Region { start: index.start, end: index.end, kind: RegionKind::Blocked(index.blocked_labels.len() - 1) });
                    } else {
                        index.regions.push(Region { start: index.start, end: index.end, kind: RegionKind::Pending });
                    }
                }
            } else if !track.notes.is_empty() { index.regions.push(Region { start: index.start, end: index.end, kind: RegionKind::Pending }); }
            let mut end = i64::MIN;
            for region in &index.regions { end = end.max(region.end); index.prefix_ends.push(end); }
        }
    }
}

fn track_collection_revision(ed: &Editor) -> String {
    format!("{}:{}:tracks", ed.document_generation, ed.revision().get())
}

pub fn track_headers(ed: &Editor, cx: &mut Context<Editor>) -> impl IntoElement {
    let revision = track_collection_revision(ed);
    let count = ed.project().tracks.len();
    let cursor = ed.track_index(&ed.session.track).unwrap_or(0);
    let props = serde_json::json!({"mode":"tracklist", "label":"编曲轨道", "cursor":cursor,
        "revision":revision, "disabled":ed.session.gesture.is_some()});
    let owner = std::rc::Rc::new(std::cell::RefCell::new(None::<verdant_collection_control::CollectionControlHandle>));
    let source_owner = owner.clone(); let entity = cx.entity(); let basis = revision.clone();
    let draw = std::rc::Rc::new(move |handle:&verdant_collection_control::CollectionControlHandle, _focus:&gpui::FocusHandle, _engine: Option<crate::proto_scroll::EngineBinding>, _window:&mut Window, cx:&mut App| {
        *source_owner.borrow_mut() = Some(handle.clone());
        entity.update(cx, |ed,cx| native_track_headers(ed, handle, &basis, cx).into_any_element())
    });
    let entity = cx.entity().downgrade();
    let sink = std::rc::Rc::new(move |name:&str, payload:&serde_json::Value, window:&mut Window, cx:&mut App| {
        if !matches!(name, "select" | "activate" | "context") { return; }
        let source = owner.borrow().clone();
        let _ = entity.update(cx, |ed,cx| {
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(track_collection_revision(ed).as_str()) { return; }
            let Some(index) = crate::proto_controls::source_index(payload.get("index")) else { return; };
            let Some(track) = ed.project().tracks.get(index).map(|track|track.id.clone()) else { return; };
            if !ed.select_track(track.clone(), cx) {
                if let Some(source) = &source { crate::proto_collection::call(source, "reconcile", vec![], window, cx); }
                return;
            }
            if name == "context" {
                let pointer = payload.get("keyboard").and_then(serde_json::Value::as_bool) != Some(true);
                let anchor = if pointer {
                    gpui::point(px(payload.get("x").and_then(serde_json::Value::as_f64).unwrap_or(0.) as f32),
                        px(payload.get("y").and_then(serde_json::Value::as_f64).unwrap_or(0.) as f32))
                } else {
                    source.as_ref().and_then(|source| source.owner.borrow().bounds).map(|bounds| bounds.origin + gpui::point(px(HEADER_W / 2.), px((index as f64 * ed.arrange_track_height + ed.arrange_track_height / 2.) as f32))).unwrap_or_else(||window.mouse_position())
                };
                ed.track_menu(track, anchor, window, cx);
            }
        });
    });
    div().w(px(HEADER_W)).h(px(count as f32 * ed.arrange_track_height as f32)).flex_none()
        .child(crate::proto_collection::collection("arrange-track-list", props, vec![1;count], draw, sink))
}

fn native_track_headers(ed: &Editor, handle: &verdant_collection_control::CollectionControlHandle, revision: &str, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    let keyboard = handle.clone();
    div().w_full().flex_none().flex().flex_col()
        .on_key_down(move |event, window, cx| crate::proto_collection::key(&keyboard, event, window, cx)).children(ed.project().tracks.iter().enumerate().map(|(i, track)| {
        let selected = track.id == ed.session.track;
        let auto_expanded = ed.automation_ui.expanded_tracks.contains(&track.id);
        let color = track.id.clone();
        let select = handle.clone(); let select_revision = revision.to_string();
        let menu = handle.clone(); let menu_revision = revision.to_string();
        let assistive = handle.clone(); let assistive_revision = revision.to_string();
        let route_label = crate::ui_controls::route_short_name(verdant_backend::adapter::desktop_route(ed.project(), track, ed.scheduler.capabilities.as_ref()));
        let hdr_h = px(crate::ui_controls::HDR_BTN_H);
        div().id(SharedString::from(format!("track-{}", track.id))).role(gpui::Role::ListBoxOption).aria_label(track.name.clone()).aria_selected(selected)
            .h(px(ed.arrange_track_height as f32)).flex_none().w_full().flex().flex_col().justify_center().gap(px(4.0)).px(px(10.0))
            .bg(if selected { mix(th.surface, th.accent, 0.14) } else { th.surface }).border_b_1().border_color(th.shade)
            // Row events transport the hit-tested index to one authored list
            // owner. Nested Action roots claim their own activation first.
            .on_click(move |_, window, cx| {
                if window.default_prevented() { return; }
                crate::proto_collection::call(&select, "selectAndFocus", vec![i.into(), false.into(), select_revision.clone().into()], window, cx);
                window.prevent_default(); cx.stop_propagation();
            })
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                crate::proto_collection::call(&assistive, "selectAndFocus", vec![i.into(), false.into(), assistive_revision.clone().into()], window, cx);
            })
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                crate::proto_collection::call(&menu, "contextAt", vec![i.into(), menu_revision.clone().into(),
                    (event.position.x.as_f32() as f64).into(), (event.position.y.as_f32() as f64).into(), false.into()], window, cx);
                window.prevent_default(); cx.stop_propagation();
            })
            .child(div().flex().items_center().gap(px(5.0))
                .child(div().text_size(px(10.5)).text_color(th.text_tertiary).w(px(15.0)).flex_none().child(format!("{:02}", i + 1)))
                .child(crate::proto_controls::action("color", "切换轨道颜色").size(px(11.0)).rounded(px(3.0)).bg(th.track(track.color.as_deref()))
                    .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| ed.cycle_color(&color, cx))))
                .child(div().flex_1().truncate().text_sm().font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL }).child(track.name.clone()))
                .child(crate::ui_controls::selectable_command_button(ed, CommandId::ToggleTrackAutomation, CommandArg::Track(track.id.clone()), "A", SharedString::from(format!("auto-btn-{}", track.id)), auto_expanded, cx)
                    .h(hdr_h).w(px(23.0)).px(px(0.0)).rounded(px(4.0)).aria_label("展开/收起自动化曲线"))
                .child(crate::ui_controls::selectable_command_button(ed, CommandId::ToggleMute, CommandArg::Track(track.id.clone()), "M", SharedString::from(format!("hdr-mute-{}", track.id)), track.muted, cx)
                    .h(hdr_h).w(px(23.0)).px(px(0.0)).rounded(px(4.0)))
                .child(crate::view::command_button(ed, CommandId::ToggleSolo, CommandArg::Track(track.id.clone()), "S", SharedString::from(format!("hdr-solo-{}", track.id)), cx)
                    .h(hdr_h).w(px(23.0)).px(px(0.0)).rounded(px(4.0))
                    .aria_selected(track.solo)
                    .border_1().border_color(if track.solo { th.warning } else { th.control })
                    .when(track.solo, |d| d.bg(th.warning).text_color(th.on_track()).font_weight(FontWeight::SEMIBOLD))))
            .child(div().flex().items_center().gap(px(8.0)).text_xs().text_color(th.text_secondary)
                .child(match track.instrument() {
                    Some(spec) => format!("{} · {}", spec.get("name").and_then(serde_json::Value::as_str).unwrap_or("乐器"),
                        spec.get("preset").and_then(serde_json::Value::as_str).filter(|p| !p.is_empty()).map(str::to_string)
                        .unwrap_or_else(|| match spec.get("format").and_then(serde_json::Value::as_str) { Some("builtin") | None => "自定义".into(), Some(f) => f.to_uppercase() })),
                    None => format!("{route_label} · {}", ed.scheduler.capabilities.as_ref()
                        .and_then(|caps| verdant_backend::adapter::desktop_voice(ed.project(), track, caps).ok().flatten())
                        .map(|(_, voice)| voice.display_name.as_str()).unwrap_or("尚无已校验声库")),
                })
                .child(div().flex_1().h(px(3.0)).child(ed.playback.anchor(crate::clock::Surface::HeaderMeter(i)))))
            .when(selected && ed.integrations.selected_clip.as_ref().is_some_and(|(id,_)|id==&track.id), |header| {
                header.child(div().flex().items_center().gap(px(6.0)).text_xs().children([
                    ("分割", "在播放头分割音频片段", AudioRegionEdit::SplitPlayhead),
                    ("连接", "连接右侧相邻音频片段", AudioRegionEdit::Join),
                    ("复制", "复制音频片段到播放头", AudioRegionEdit::Duplicate),
                    ("静音", "切换音频片段静音", AudioRegionEdit::Mute),
                ].into_iter().map(|(label,description,edit)| {
                    crate::proto_controls::action(SharedString::from(format!("audio-region-{label}")), description).px(px(4.0)).rounded(px(2.0)).bg(th.control).cursor_pointer()
                        .child(label).on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, window, cx| {
                             edit_audio_region(ed,edit,cx);cx.stop_propagation();
                        }))
                })))
            })
    }))
}

#[cfg(test)]
#[path="arrange_controls_tests.rs"]
mod controls_tests;

pub fn region_lanes(editor: Entity<Editor>) -> impl IntoElement {
    let draw = editor.clone(); let cancel = editor.clone(); let commands = editor.clone();
    crate::proto_editor_surface::surface(editor, "arrangement", "编曲区：选择、移动、裁剪与淡入淡出", DRAG_THRESHOLD,
        move |input| native_region_lanes(draw.clone(), input).into_any_element(),
        move |_, window, cx| cancel.update(cx, |ed, cx| {
            ed.arrange_drag = None; ed.integrations.audio_edit.range_drag = None;
             cx.notify();
        })).commands("audio", move |command, window, cx| commands.update(cx, |ed, cx| {
            crate::editor::integrations::audio_edit_ui::source_command(ed, command, window, cx);
        }))
}

fn native_region_lanes(editor: Entity<Editor>, input: crate::proto_editor_surface::Input) -> impl IntoElement {
    let pre = editor.clone();
    canvas(move |bounds, window, cx| {
        pre.update(cx, |ed, _| {
            ed.arrange.width = bounds.size.width.as_f32() as f64;
            ed.arrange.height = bounds.size.height.as_f32() as f64;
            if ed.needs_arrange_fit { ed.needs_arrange_fit = false; ed.fit_arrange(); }
        });
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }, move |bounds, hitbox, window, cx| {
        let mut texts = Texts::new(); let mut labels = Vec::new();
        paint_regions(&editor, bounds, window, cx, &mut texts, &mut labels); flush_texts(window, cx, bounds, texts);
        for (clip_bounds, label) in labels { flush_texts(window, cx, clip_bounds, label); }
        let e = editor.clone(); let policy = input.clone();
        crate::proto_editor_surface::install_pointer_input(&input, bounds, &hitbox, window, move |event, window, cx| {
            use crate::proto_editor_surface::PointerPhase;
            e.update(cx, |ed, cx| match event.phase {
                PointerPhase::Begin | PointerPhase::Context => region_down(&policy,ed,event.x,event.y,
                    if event.phase == PointerPhase::Context { MouseButton::Right } else { MouseButton::Left },
                    event.clicks,event.modifiers.control,event.modifiers.shift,event.modifiers.alt,event.pointer,event.position,window,cx),
                PointerPhase::Sample => {
                    if let Some(intent) = policy.motion(ed,event.x,event.y,event.modifiers,window,cx) {
                        region_moved(ed,event.pointer,event.x,event.y,!intent.snap,cx);
                    }
                    true
                }
                PointerPhase::Finish => { region_up(ed,event.pointer,window,cx); true }
            })
        });
        install_timeline_scroll(&editor,bounds,&hitbox,&input,window);
        let e = editor.clone();let gate=input.clone();
        window.on_mouse_event(move |ev: &PinchEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || !hitbox.is_hovered(window) { return; }
            let Some(intent) = gate.pinch(1.0+f64::from(ev.delta),window,cx) else {return;};
            e.update(cx, |ed, cx| { if intent.apply(&mut ed.arrange, (f64::from(ev.position.x - bounds.origin.x), 0.)) { ed.notify_viewport(crate::clock::Space::Arrange, cx); } });
            cx.stop_propagation();
        });
    }).size_full()
}

pub(crate) fn install_timeline_scroll(editor:&Entity<Editor>,bounds:Bounds<Pixels>,hitbox:&Hitbox,input:&crate::proto_editor_surface::Input,window:&mut Window) {
    let editor = editor.clone();
    let hitbox = hitbox.clone();let input=input.clone();
    window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !hitbox.is_hovered(window) { return; }
        let Some(intent)=input.wheel(event.delta,event.modifiers,true,window,cx) else { return };
        editor.update(cx,|ed,cx| {
            if intent.apply(&mut ed.arrange, (f64::from(event.position.x-bounds.origin.x), 0.)) { ed.notify_viewport(crate::clock::Space::Arrange,cx); }
        });
        cx.stop_propagation();
    });
}

#[derive(Clone, Copy)]
enum AudioRegionEdit { SplitPlayhead, Split(i64), Join, Duplicate, Mute }

fn edit_audio_region(ed:&mut Editor,edit:AudioRegionEdit,cx:&mut Context<Editor>){
    use verdant_service::audio_edit::{AudioEdit,AudioClipRef,AudioClipProperties};
    let result=(||->Result<(),String>{
        if ed.arrange_drag.is_some(){return Err("先完成当前片段拖动".into());}
        let clip=ed.selected_audio_clip()?;let clips=ed.audio_selection();let tick=ed.tempo().tick_at(ed.playhead_seconds());
        let operation=match edit{
            AudioRegionEdit::SplitPlayhead=>if let Some(range)=ed.integrations.audio_edit.range.clone(){AudioEdit::SplitRange{tracks:range.tracks,start_tick:range.start_tick,end_tick:range.end_tick}}else{AudioEdit::Split{clips,tick}},
            AudioRegionEdit::Split(tick)=>AudioEdit::Split{clips:vec![AudioClipRef::from(&clip)],tick:tick as f64},
            AudioRegionEdit::Join=>{
                let mut selected=clips;
                if selected.len()==1{let others=verdant_media::read_clips(ed.project().track(&clip.track).ok_or("轨道不存在")?).map_err(|e|e.to_string())?;let right=others.iter().filter(|other|other.id!=clip.id&&other.tick()>=clip.tick()).min_by(|a,b|a.tick().total_cmp(&b.tick())).ok_or("右侧没有片段")?;selected.push(AudioClipRef::from(right));}
                AudioEdit::Join{clips:selected}
            },
            AudioRegionEdit::Duplicate=>AudioEdit::Duplicate{clips,at_tick:tick,target_track:None},
            AudioRegionEdit::Mute=>AudioEdit::SetProperties{clips,properties:AudioClipProperties{muted:Some(!clip.muted),..Default::default()}},
        };ed.apply_audio_edit(operation,cx)
    })();if let Err(error)=result{ed.notice(error);}cx.notify();
}

fn paint_regions(editor: &Entity<Editor>, bounds: Bounds<Pixels>, window: &mut Window, cx: &App, texts: &mut Texts, labels: &mut Vec<(Bounds<Pixels>, Texts)>) {
    let ed = editor.read(cx);
    let (th, vp, o) = (ed.theme, ed.arrange, bounds.origin);
    let (w, height) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    let (from, to) = (vp.start_tick.floor() as i64, vp.x_to_tick(w).ceil() as i64);
    let tempo = ed.tempo();
    window.paint_quad(fill(bounds, th.window));
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        for tick in ed.meter().bars(from,to){window.paint_quad(fill(b(vp.tick_to_x(tick as f64),0.,1.,height,o),th.grid_beat));}
        for (row, track) in ed.project().tracks.iter().enumerate() {
            let y = row as f64 * ed.arrange_track_height;
            if y > height { break; }
            if track.id == ed.session.track { window.paint_quad(fill(b(0.0, y, w, ed.arrange_track_height, o), with_alpha(th.accent, 0.06))); }
            window.paint_quad(fill(b(0.0, y + ed.arrange_track_height - 1.0, w, 1.0, o), th.shade));
            let Some(index) = ed.arrange_cache.tracks.get(&track.id) else { continue };
            let low = index.prefix_ends.partition_point(|end| *end <= from);
            let high = index.regions.partition_point(|span| span.start < to);
            let color = th.track(track.color.as_deref());
            let pitch_span = (index.high - index.low).max(6) as f64;
            for region in &index.regions[low..high] {
                if region.end <= from { continue; }
                let (x0, x1) = (vp.tick_to_x(region.start as f64), vp.tick_to_x(region.end as f64));
                let (left, right) = (x0.max(0.0), x1.min(w));
                if right <= left { continue; }
                let (body_y, body_h) = (y + 4.0, ed.arrange_track_height - 8.0);
                let blocked = matches!(region.kind, RegionKind::Blocked(_));
                let (label, audio) = match region.kind {
                    RegionKind::Phrase(phrase) => {
                        let state = ed.phrase_slots.get(&(track.id.clone(), phrase)).and_then(|i| ed.scheduler.slot_status(*i));
                        let label = match state {
                            Some(SlotStatus::Ready { rendered_revision, .. }) => format!("就绪 · r{rendered_revision}"),
                            Some(SlotStatus::Rendering { .. }) => "渲染中".into(), Some(SlotStatus::Verifying { .. }) => "校验中".into(),
                            Some(SlotStatus::Failed { .. }) => "渲染失败".into(), _ => "等待渲染".into(),
                        };
                        (label, ed.render_audio.get(&(track.id.clone(), phrase)))
                    }
                    RegionKind::Blocked(i) => (index.blocked_labels.get(i).cloned().unwrap_or_else(|| "预检未通过 · 含错误".into()), None),
                    RegionKind::Pending => ("等待当前编辑的渲染预检".into(), None),
                };
                window.paint_quad(quad(b(left, body_y, (right - left).max(2.0), body_h, o), px(4.0), if blocked { th.control } else { mix(color, th.window, 0.5) },
                    px(1.0), if blocked { th.error } else { color }, if blocked { BorderStyle::Dashed } else { BorderStyle::Solid }));
                if right - left > 40.0 { texts.push((label, gpui::point(o.x + px(left as f32 + 4.0), o.y + px(body_y as f32)), 10.0, if blocked { th.error } else { th.text })); }
                let (notes_y, notes_h) = (body_y + 16.0, body_h - 18.0);
                let notes = track.notes_in_range(region.start.max(from), region.end.min(to));
                let note_span = |note: &verdant_domain::Note| {
                    let x = vp.tick_to_x(note.start_tick as f64).max(left);
                    let end = vp.tick_to_x(note.end_tick() as f64).min(right);
                    (x, (end - x).max(1.0))
                };
                let mut previous_right = f64::NEG_INFINITY;
                let disjoint_markers = notes.clone().all(|note| {
                    let (x, width) = note_span(note);
                    let disjoint = x >= previous_right;
                    previous_right = x + width;
                    disjoint
                });
                let paint_markers = |window: &mut Window| {
                    for note in notes {
                        let (x, width) = note_span(note);
                        let y = notes_y + (1.0 - (note.midi_note - index.low) as f64 / pitch_span) * (notes_h - 3.0);
                        window.paint_quad(fill(b(x, y, width, 2.0, o), if blocked { th.text_tertiary } else { color }));
                    }
                };
                if disjoint_markers {
                    window.paint_layer(paint_markers);
                } else {
                    paint_markers(window);
                }
                if let Some(audio) = audio {
                    let start = ed.phrase_slots.get(&(track.id.clone(), match region.kind { RegionKind::Phrase(i) => i, _ => unreachable!() }))
                        .and_then(|i| ed.scheduler.wanted().get(*i)).map(|w| w.phrase.start_seconds).unwrap_or(0.0);
                    let (middle, amplitude) = (notes_y + notes_h * 0.5, notes_h * 0.4);
                    for pixel in left.floor() as i64..right.ceil() as i64 {
                        let a = (((tempo.seconds_at(vp.x_to_tick(pixel as f64)) - start) * audio.sample_rate as f64).max(0.0) as usize).min(audio.samples.len());
                        let z = (((tempo.seconds_at(vp.x_to_tick(pixel as f64 + 1.0)) - start) * audio.sample_rate as f64).max(0.0) as usize).min(audio.samples.len());
                        if z <= a { continue; }
                        let peak = audio.samples[a..z].iter().fold(0.0f32, |a, s| a.max(s.abs())) as f64;
                        window.paint_quad(fill(b(pixel as f64, middle - peak * amplitude, 1.0, (peak * amplitude * 2.0).max(1.0), o), with_alpha(color, 0.5)));
                    }
                }
            }
            let has_parts=index.authored.iter().any(|region|matches!(region.target,AuthoredTarget::Part(_)));
            for region in &index.authored{
                let mut start=region.clip.as_ref().map_or(region.start as f64,|clip|clip.tick());
                let mut end=region.clip.as_ref().map_or(region.end as f64,|clip|clip.end_tick(&tempo));
                let mut row_y=y;
                let(part,selected)=match &region.target{
                    AuthoredTarget::Part(id)=>(true,ed.selected_part.as_ref()==Some(id)),
                    AuthoredTarget::Audio(id)=>(false,ed.integrations.audio_edit.selected.iter().any(|reference|reference.track==track.id&&reference.id.to_string()==*id)
                        ||ed.integrations.selected_clip.as_ref().is_some_and(|(t,c)|t==&track.id&&c==id))
                };
                if let Some(drag)=&ed.arrange_drag{
                    let primary=drag.target.same(&region.target)&&drag.source_track==track.id;
                    let grouped=!part&&selected&&drag.mode==RegionDragMode::Move&&matches!(drag.target,AuthoredTarget::Audio(_));
                    if primary||grouped{
                        if grouped{let delta=drag.audio.as_ref().map_or((drag.start-drag.original_start)as f64,|audio|audio.delta);start+=delta;end=region.clip.as_ref().map_or(end+delta,|clip|tempo.tick_at(tempo.seconds_at(start)+clip.duration_seconds()));}
                        else if let Some(audio)=&drag.audio{start=audio.start;end=audio.end;}
                        else{start=drag.start as f64;end=drag.end as f64;}
                        let origin=ed.project().tracks.iter().position(|track|track.id==drag.source_track).unwrap_or(0)as isize;
                        let destination=ed.project().tracks.iter().position(|track|track.id==drag.destination_track).unwrap_or(0)as isize;
                        row_y+=(destination-origin)as f64*ed.arrange_track_height;
                    }
                }
                let left=vp.tick_to_x(start).max(0.);let right=vp.tick_to_x(end).min(w);if right<=left{continue;}
                let top=row_y+if part||!has_parts{3.}else{27.};let body=if part{21.}else if has_parts{(ed.arrange_track_height-30.).max(8.)}else{ed.arrange_track_height-6.};
                let background=with_alpha(color,if region.muted{0.16}else{0.72});
                window.paint_quad(quad(b(left,top,(right-left).max(2.),body,o),px(4.),background,px(if selected{2.}else{1.}),if selected{th.text}else{color},BorderStyle::Solid));
                if let (AuthoredTarget::Audio(id),Some(clip))=(&region.target,&region.clip){
                    if let Some(preview)=ed.integrations.media_previews.get(id){
                        let trimming=ed.arrange_drag.as_ref().is_some_and(|drag|drag.target.same(&region.target)&&matches!(drag.mode,RegionDragMode::Left|RegionDragMode::Right));
                        if !trimming&&preview.frame_count>0&&preview.bins>0&&body>21.{
                            let seconds=tempo.seconds_at(start);let channels=preview.channels as usize;
                            let lane_height=(body-17.)/channels as f64;
                            for pixel in left.floor()as i64..right.ceil()as i64{
                                let elapsed=tempo.seconds_at(vp.x_to_tick(pixel as f64))-seconds;if elapsed<0.{continue;}
                                let playback_frame=(elapsed*clip.source.sample_rate as f64*clip.playback_rate).round()as u64;
                                if playback_frame>=preview.frame_count{continue;}
                                let source_frame=if clip.reversed{preview.frame_count-1-playback_frame}else{playback_frame};
                                let bin=((source_frame as usize)*preview.bins/preview.frame_count as usize).min(preview.bins-1);
                                let gain=clip.gain*clip.envelope_gain(playback_frame as f64);
                                for channel in 0..channels{
                                    let peak=preview.peaks[bin*channels+channel];let middle=top+15.+lane_height*(channel as f64+0.5);let amplitude=(lane_height*0.44).max(1.);
                                    let min=(peak.min*gain).clamp(-1.,1.)as f64;let max=(peak.max*gain).clamp(-1.,1.)as f64;
                                    window.paint_quad(fill(b(pixel as f64,middle-max*amplitude,1.,((max-min)*amplitude).max(1.),o),with_alpha(th.text,if region.muted{0.2}else{0.72})));
                                }
                            }
                            if channels==2{window.paint_quad(fill(b(left,top+15.+lane_height,right-left,1.,o),with_alpha(th.text,0.12)));}
                        }
                    }
                    let mut envelope=clip.as_ref().clone();
                    if let Some(drag)=ed.arrange_drag.as_ref().filter(|drag|drag.target.same(&region.target)){
                        if drag.mode==RegionDragMode::FadeIn{envelope.fade_window=None;envelope.fade_in_frames=drag.fade_frames;}
                        if drag.mode==RegionDragMode::FadeOut{envelope.fade_window=None;envelope.fade_out_frames=drag.fade_frames;}
                    }
                    let seconds=tempo.seconds_at(start);let rate=clip.source.sample_rate as f64*clip.playback_rate;
                    if envelope.fade_in_frames>0||envelope.fade_out_frames>0||envelope.fade_window.is_some(){
                        for pixel in left.floor()as i64..right.ceil()as i64{
                            let frame=((tempo.seconds_at(vp.x_to_tick(pixel as f64))-seconds)*rate).max(0.);
                            let gain=envelope.envelope_gain(frame)as f64;
                            if gain<1.{window.paint_quad(fill(b(pixel as f64,top+3.+(1.-gain)*(body-6.),1.,1.,o),th.text));}
                        }
                    }
                    if selected{
                        let fade_x=|frame:u64|vp.tick_to_x(tempo.tick_at(seconds+frame as f64/rate));
                        for x in [fade_x(envelope.fade_in_frames),fade_x(envelope.duration_frames-envelope.fade_out_frames)]{
                            window.paint_quad(fill(b(x-3.,top+2.,6.,6.,o),th.text));
                        }
                    }
                }
                if right-left>22.{labels.push((b(left+2.,top,(right-left-4.).max(0.),body,o),vec![(region.label.clone(),gpui::point(o.x+px(left as f32+5.),o.y+px(top as f32+2.)),10.,th.text)]));}
            }
            if let Some(range)=ed.integrations.audio_edit.range.as_ref().filter(|range|range.tracks.contains(&track.id)){
                let left=vp.tick_to_x(range.start_tick).max(0.);let right=vp.tick_to_x(range.end_tick).min(w);
                if right>left{
                    window.paint_quad(fill(b(left,y,right-left,ed.arrange_track_height,o),with_alpha(th.accent,0.14)));
                    for x in [left,right-1.]{window.paint_quad(fill(b(x,y,1.,ed.arrange_track_height,o),th.accent));}
                }
            }

        }
    });
}


/// Right-click on empty lane space: what can be made here, then the track.
fn lane_menu(ed:&mut Editor,track:TrackId,tick:f64,position:gpui::Point<Pixels>,window:&mut Window,cx:&mut Context<Editor>){
    use crate::editor::{MenuAction as M,WorkflowAction as A};use crate::menu::MenuItem as I;
    let free=verdant_domain::parts::Parts::load(ed.project()).ok().and_then(|parts|new_part_span(&ed.meter(),&parts,&track,tick)).is_some();
    let mut items=vec![
        I::action("在此新建片段",M::Workflow(A::CreatePartAt(track.clone(),tick.max(0.).floor()as i64))).with_hint("双击").enabled(free),
        I::action("播放头移到此处",M::SeekTick(tick.max(0.).floor()as i64)),
        I::action("导入音频到此轨…",M::Workflow(A::Integration(crate::editor::integrations::IntegrationAction::ImportAudio))),
        I::separator(),I::header("轨道"),
    ];
    items.extend(crate::menu::track_menu_items(ed.project(),&track));
    ed.show_menu(position,items,window,cx);
}

/// Movement (px) before a region press turns into a drag.
pub(crate) const DRAG_THRESHOLD: f64 = 3.0;

/// Smallest on-screen beat before arrangement snapping moves to whole bars.
const MIN_BEAT_PX: f64 = 14.0;
/// Track-area snapping follows the meter at the current zoom: beats when a
/// beat is wide enough to target, otherwise bars. (The piano-roll grid is
/// far too fine for regions.)
pub(crate) fn snap_to_meter(meter:&verdant_domain::MeterMap,tick:f64,px_per_tick:f64)->i64{
    let t=tick.max(0.);let bar=meter.bar_start(t.floor()as i64);let next=meter.next_bar(bar);
    let beat=meter.beat_ticks_at(bar).max(1);
    let unit=if beat as f64*px_per_tick>=MIN_BEAT_PX{beat}else{(next-bar).max(1)};
    (bar+((t-bar as f64)/unit as f64).round()as i64*unit).min(next)
}
fn arrange_snap(ed:&Editor,tick:f64)->i64{snap_to_meter(&ed.meter(),tick,ed.arrange.px_per_tick)}

/// The free span for a new one-bar part at `tick`: the bar around it, trimmed
/// to neighbouring parts on the same track. None if the spot is occupied.
pub(crate) fn new_part_span(meter:&verdant_domain::MeterMap,parts:&verdant_domain::parts::Parts,track:&TrackId,tick:f64)->Option<(i64,i64)>{
    let t=tick.max(0.).floor()as i64;let mut start=meter.bar_start(t);let mut end=meter.next_bar(start);
    for p in parts.instances.iter().filter(|p|&p.track==track){
        let(a,b)=(p.start_tick,p.start_tick+p.duration_tick);
        if a<=t&&t<b{return None;}
        if b<=t{start=start.max(b);}else if a>t{end=end.min(a);}
    }
    (end>start).then_some((start,end))
}

/// Double-click on an empty lane: an empty one-bar part named "片段 N",
/// selected and opened in the piano roll so notes can be drawn into it.
pub(crate) fn create_part_at(ed:&mut Editor,track:&TrackId,tick:f64,window:&mut Window,cx:&mut Context<Editor>){
    let result=(||->Result<String,String>{
        let mut parts=verdant_domain::parts::Parts::load(ed.project())?;
        let(start,end)=new_part_span(&ed.meter(),&parts,track,tick).ok_or("此处已有片段")?;
        let name=parts.next_name("片段");let id=parts.create_empty(ed.project(),track,start,end-start,name)?;
        if !ed.commit(ed.revision(),CommandBatch::single("新建片段",parts.command()),cx){return Err("片段未提交".into());}
        Ok(id)
    })();
    match result{
        Ok(id)=>{
            ed.integrations.selected_clip=None;ed.integrations.audio_edit.selected.clear();ed.integrations.audio_edit.range=None;ed.session.selection.clear();ed.selected_part=Some(id.clone());
            if let Some(part)=verdant_domain::parts::Parts::load(ed.project()).ok().and_then(|p|p.instance(&id).cloned()){
                ed.bottom=Bottom::Editor;let vp=&mut ed.session.viewport;
                let target=(part.start_tick as f64-vp.width*0.1/vp.px_per_tick).max(0.);let dx=(target-vp.start_tick)*vp.px_per_tick;vp.pan_px(dx,0.);
            }
        }
        Err(e)=>ed.notice(e),
    }
    cx.notify();
}

impl AuthoredTarget{fn same(&self,other:&Self)->bool{match(self,other){(Self::Part(a),Self::Part(b))|(Self::Audio(a),Self::Audio(b))=>a==b,_=>false}}}
#[derive(Clone,Copy,PartialEq)]enum RegionDragMode{Move,Left,Right,FadeIn,FadeOut}
struct AudioDragGeometry{
    original_start:f64,original_end:f64,start:f64,end:f64,delta:f64,
    seconds:f64,rate:f64,before:u64,after:u64,duration:u64,group_min_start:f64,
    destination_rows:(usize,usize),
}
pub struct RegionDrag{origin_raw:f64,audio:Option<AudioDragGeometry>,pointer:PointerId,press:(f64,f64),armed:bool,base:Revision,target:AuthoredTarget,source_track:TrackId,destination_track:TrackId,mode:RegionDragMode,origin_tick:i64,original_start:i64,original_end:i64,start:i64,end:i64,min_left:i64,max_right:i64,audio_seconds:Option<f64>,fade_frames:u64,original_fade_frames:u64}
fn authored_hit(ed:&Editor,track:&TrackId,tick:f64,row_y:f64)->Option<AuthoredTarget>{
    let index=ed.arrange_cache.tracks.get(track)?;let has_parts=index.authored.iter().any(|r|matches!(r.target,AuthoredTarget::Part(_)));
    index.authored.iter().rev().find(|region|tick>=region.start as f64&&tick<region.end as f64&&match region.target{AuthoredTarget::Part(_)=>row_y<27.,AuthoredTarget::Audio(_)=>!has_parts||row_y>=27.}).map(|region|region.target.clone())
}
fn region_down(input:&crate::proto_editor_surface::Input,ed:&mut Editor,x:f64,y:f64,button:MouseButton,clicks:usize,split_at_click:bool,extend_selection:bool,unsnapped:bool,pointer:PointerId,position:gpui::Point<Pixels>,window:&mut Window,cx:&mut Context<Editor>)->bool{
    let row=(y/ed.arrange_track_height).floor()as usize;let Some(track)=ed.project().tracks.get(row).map(|t|t.id.clone())else{return false;};
    let tick=ed.arrange.x_to_tick(x);let hit=authored_hit(ed,&track,tick,y-row as f64*ed.arrange_track_height);
    let hit_kind=match hit.as_ref(){Some(AuthoredTarget::Part(_))=>"part",Some(AuthoredTarget::Audio(_))=>"audio",None=>"empty"};
    let(mut left,mut right,mut fade_in,mut fade_out,mut fade_band)=(-1.,-1.,-1.,-1.,false);
    if let Some(index)=ed.arrange_cache.tracks.get(&track) {
        if let Some(region)=hit.as_ref().and_then(|target|index.authored.iter().find(|region|region.target.same(target))) {
            left=(x-ed.arrange.tick_to_x(region.start as f64)).abs();right=(x-ed.arrange.tick_to_x(region.end as f64)).abs();
            if let Some(clip)=region.clip.as_ref() {
                let has_parts=index.authored.iter().any(|region|matches!(region.target,AuthoredTarget::Part(_)));
                let top=row as f64*ed.arrange_track_height+if has_parts{27.}else{3.};
                fade_band=y>=top&&y<top+10.;
                let tempo=ed.tempo();let seconds=tempo.seconds_at(region.start as f64);let rate=clip.source.sample_rate as f64*clip.playback_rate;
                fade_in=(x-ed.arrange.tick_to_x(tempo.tick_at(seconds+clip.fade_in_frames as f64/rate))).abs();
                fade_out=(x-ed.arrange.tick_to_x(tempo.tick_at(seconds+(clip.duration_frames-clip.fade_out_frames)as f64/rate))).abs();
            }
        }
    }
    let Some(intent)=input.intent(ed,"regionGesture",vec![hit_kind.into(),ed.integrations.audio_edit.range_tool.into(),
        (button==MouseButton::Right).into(),extend_selection.into(),split_at_click.into(),(clicks as u64).into(),
        left.into(),right.into(),fade_in.into(),fade_out.into(),fade_band.into(),unsnapped.into()],window,cx) else{return false;};
    if !ed.select_track(track.clone(),cx){return false;}
    if intent.kind=="range"{
        let tick=if intent.snap{arrange_snap(ed,tick).max(0)as f64}else{tick.max(0.)};
        ed.integrations.audio_edit.range_drag=Some(crate::editor::integrations::audio_edit_ui::RangeDrag{pointer,anchor_tick:tick,anchor_track:row});
        ed.select_audio_range(crate::editor::integrations::audio_edit_ui::AudioRange{tracks:vec![track],start_tick:tick,end_tick:tick});
        cx.notify();return true;
    }
    let Some(target)=hit else{match intent.kind.as_str(){"lane-menu"=>lane_menu(ed,track,tick,position,window,cx),"create-part"=>create_part_at(ed,&track,tick,window,cx),"seek"=>{ed.integrations.selected_clip=None;ed.integrations.audio_edit.selected.clear();ed.integrations.audio_edit.range=None;ed.run_menu_action(crate::editor::MenuAction::SeekTick(tick.max(0.).round()as i64),window,cx);},_=>{}}cx.notify();return false;};
    use crate::editor::WorkflowAction as A;use crate::editor::MenuAction as M;use crate::menu::MenuItem as I;
    let(start,end,min_left,max_right,audio_seconds)=match &target{
        AuthoredTarget::Part(id)=>{
            ed.integrations.selected_clip=None;ed.integrations.audio_edit.selected.clear();ed.integrations.audio_edit.range=None;
            ed.run_workflow(A::SelectPart(id.clone()),window,cx);let Ok(parts)=verdant_domain::parts::Parts::load(ed.project())else{return false;};let Some(part)=parts.instance(id)else{return false;};
            (part.start_tick,part.start_tick+part.duration_tick,(part.start_tick-part.source_offset_tick).max(0),i64::MAX,None)
        }
        AuthoredTarget::Audio(id)=>{
            ed.integrations.selected_clip=Some((track.clone(),id.clone()));let Ok(clip)=ed.selected_audio_clip()else{return false;};
            ed.select_audio_clip(&clip,intent.selection=="extend",true);
            if intent.kind=="select"{cx.notify();return false;}
            let tempo=ed.tempo();let start_seconds=clip.start_seconds(&tempo);let duration=clip.duration_seconds();let rate=clip.source.sample_rate as f64*clip.playback_rate;
            let before=if clip.reversed{clip.source.frames-clip.source_offset_frames-clip.duration_frames}else{clip.source_offset_frames};
            let after=if clip.reversed{clip.source_offset_frames}else{clip.source.frames-clip.source_offset_frames-clip.duration_frames};
            (clip.start_tick,clip.end_tick(&tempo).ceil()as i64,tempo.tick_at((start_seconds-before as f64/rate).max(0.)).ceil()as i64,tempo.tick_at(start_seconds+duration+after as f64/rate).floor()as i64,Some(duration))
        }
    };
    if intent.kind=="region-menu"{
        let items=match &target{AuthoredTarget::Part(id)=>{let muted=verdant_domain::parts::Parts::load(ed.project()).ok().and_then(|p|p.instance(id).map(|p|p.muted)).unwrap_or(false);vec![
            I::action("片段名称…",M::Workflow(A::RenamePart)),I::action("精确位置…",M::Workflow(A::MovePart)),I::action("精确裁剪…",M::Workflow(A::TrimPart)),I::separator(),
            I::action("在此分割",M::Workflow(A::SplitPart(arrange_snap(ed,tick)))),I::action("连接右侧片段",M::Workflow(A::JoinPart)),I::action("链接复制片段",M::Workflow(A::DuplicatePart(true))),I::action("独立复制片段",M::Workflow(A::DuplicatePart(false))),I::action("取消片段链接",M::Workflow(A::UnlinkPart)),I::separator(),
            I::action("静音",M::Workflow(A::MutePart)).checked(muted),I::action("删除片段",M::Workflow(A::DeletePart))]},AuthoredTarget::Audio(_)=>ed.audio_menu_items()};ed.show_menu(position,items,window,cx);return false;
    }
    if intent.kind=="split" {
        edit_audio_region(ed,AudioRegionEdit::Split(arrange_snap(ed,tick)),cx);return false;
    }
    if intent.kind=="open-part"{ed.bottom=Bottom::Editor;let target=(start as f64-ed.session.viewport.width*0.1/ed.session.viewport.px_per_tick).max(0.);let dx=(target-ed.session.viewport.start_tick)*ed.session.viewport.px_per_tick;ed.session.viewport.pan_px(dx,0.);window.focus(&ed.focus_roll,cx);cx.notify();return false;}
    let mode=match intent.kind.as_str(){"move-region"=>RegionDragMode::Move,"resize-left"=>RegionDragMode::Left,"resize-right"=>RegionDragMode::Right,"fade-in"=>RegionDragMode::FadeIn,"fade-out"=>RegionDragMode::FadeOut,_=>return false};
    let fade_frames=ed.selected_audio_clip().ok().map_or(0,|clip|match mode{RegionDragMode::FadeIn=>clip.fade_in_frames,RegionDragMode::FadeOut=>clip.fade_out_frames,_=>0});
    let audio=if matches!(target,AuthoredTarget::Audio(_)){
        ed.selected_audio_clip().ok().map(|clip|{
            let tempo=ed.tempo();let seconds=clip.start_seconds(&tempo);let rate=clip.source.sample_rate as f64*clip.playback_rate;
            let selected=ed.audio_selection();let(mut min_row,mut max_row,mut minimum)=(row,row,clip.tick());
            for(index,track)in ed.project().tracks.iter().enumerate(){
                if let Ok(clips)=verdant_media::read_clips(track){for candidate in clips{
                    if selected.iter().any(|reference|reference.track==track.id&&reference.id==candidate.id){
                        min_row=min_row.min(index);max_row=max_row.max(index);minimum=minimum.min(candidate.tick());
                    }
                }}
            }
            AudioDragGeometry{original_start:clip.tick(),original_end:clip.end_tick(&tempo),
                start:clip.tick(),end:clip.end_tick(&tempo),delta:0.,seconds,rate,
                before:if clip.reversed{clip.source.frames-clip.source_offset_frames-clip.duration_frames}else{clip.source_offset_frames},
                after:if clip.reversed{clip.source_offset_frames}else{clip.source.frames-clip.source_offset_frames-clip.duration_frames},
                duration:clip.duration_frames,group_min_start:minimum,
                destination_rows:(row-min_row,row+ed.project().tracks.len()-1-max_row)}
        })
    }else{None};
    ed.arrange_drag=Some(RegionDrag{origin_raw:tick,audio,pointer,press:(x,y),armed:false,base:ed.revision(),target,source_track:track.clone(),destination_track:track,mode,origin_tick:arrange_snap(ed,tick),original_start:start,original_end:end,start,end,min_left,max_right,audio_seconds,fade_frames,original_fade_frames:fade_frames});cx.notify();true
}
fn region_moved(ed:&mut Editor,pointer:PointerId,x:f64,y:f64,unsnapped:bool,cx:&mut Context<Editor>){
    if let Some(drag)=ed.integrations.audio_edit.range_drag.clone().filter(|drag|drag.pointer==pointer){
        let tick=if unsnapped{ed.arrange.x_to_tick(x).max(0.)}else{arrange_snap(ed,ed.arrange.x_to_tick(x)).max(0)as f64};
        let row=((y.max(0.)/ed.arrange_track_height).floor()as usize).min(ed.project().tracks.len().saturating_sub(1));
        let tracks=ed.project().tracks[drag.anchor_track.min(row)..=drag.anchor_track.max(row)].iter().map(|track|track.id.clone()).collect();
        ed.select_audio_range(crate::editor::integrations::audio_edit_ui::AudioRange{tracks,start_tick:drag.anchor_tick.min(tick),end_tick:drag.anchor_tick.max(tick)});
        cx.notify();return;
    }

    // A press only becomes a drag after DRAG_THRESHOLD px, so a click selects.
    let Some(drag)=ed.arrange_drag.as_mut().filter(|drag|drag.pointer==pointer)else{return;};
    // The compiled surface admits movement after its authored threshold.
    drag.armed = true;
    let tick=ed.arrange.x_to_tick(x);let at=if unsnapped{tick.round()as i64}else{arrange_snap(ed,tick)};let tempo=ed.tempo();
    let mut destination_row=(y.max(0.)/ed.arrange_track_height).floor()as usize;
    if let Some(audio)=ed.arrange_drag.as_ref().and_then(|drag|drag.audio.as_ref()){
        destination_row=destination_row.clamp(audio.destination_rows.0,audio.destination_rows.1);
    }
    let destination=ed.project().tracks.get(destination_row).map(|t|t.id.clone());
    let fades=ed.arrange_drag.as_ref().and_then(|drag|ed.arrange_cache.tracks.get(&drag.source_track)
        .and_then(|index|index.authored.iter().find(|region|region.target.same(&drag.target))).and_then(|region|region.fades));
    let Some(drag)=ed.arrange_drag.as_mut().filter(|drag|drag.pointer==pointer)else{return;};let delta=at.saturating_sub(drag.origin_tick);
    if let Some(audio)=&mut drag.audio{
        let delta=if unsnapped{tick-drag.origin_raw}else{delta as f64};
        match drag.mode{
            RegionDragMode::Move=>{
                audio.delta=delta.max(-audio.group_min_start);
                audio.start=audio.original_start+audio.delta;
                audio.end=tempo.tick_at(tempo.seconds_at(audio.start)+audio.duration as f64/audio.rate);
                if let Some(destination)=destination{drag.destination_track=destination;}
            }
            RegionDragMode::Left=>{
                let frames=((tempo.seconds_at((audio.original_start+delta).max(0.))-audio.seconds)*audio.rate).round()
                    .clamp(-(audio.before as f64),(audio.duration-1)as f64)
                    .max((-audio.seconds*audio.rate).ceil());
                audio.start=tempo.tick_at(audio.seconds+frames/audio.rate);
            }
            RegionDragMode::Right=>{
                let frames=((tempo.seconds_at((audio.original_end+delta).max(0.))-audio.seconds)*audio.rate).round()
                    .clamp(1.,(audio.duration+audio.after)as f64);
                audio.end=tempo.tick_at(audio.seconds+frames/audio.rate);
            }
            RegionDragMode::FadeIn|RegionDragMode::FadeOut=>{
                let frame=((tempo.seconds_at(tick)-audio.seconds)*audio.rate).round().clamp(0.,audio.duration as f64)as u64;
                drag.fade_frames=if drag.mode==RegionDragMode::FadeIn{frame}else{audio.duration-frame};
            }
        }
        drag.start=audio.start.floor()as i64;drag.end=audio.end.ceil()as i64;
        cx.notify();return;
    }
    match drag.mode{
        RegionDragMode::Move=>{drag.start=drag.original_start.saturating_add(delta).max(0);drag.end=if let Some(seconds)=drag.audio_seconds{tempo.tick_at(tempo.seconds_at(drag.start as f64)+seconds).ceil()as i64}else{drag.start+drag.original_end-drag.original_start};if let Some(destination)=destination{drag.destination_track=destination;}},
        RegionDragMode::Left=>{drag.start=drag.original_start.saturating_add(delta).clamp(drag.min_left,drag.original_end-1);},
        RegionDragMode::Right=>{drag.end=drag.original_end.saturating_add(delta).clamp(drag.original_start+1,drag.max_right.max(drag.original_start+1));},
        RegionDragMode::FadeIn|RegionDragMode::FadeOut=>{if let Some(fades)=fades{
            let frame=((tempo.seconds_at(tick)-tempo.seconds_at(drag.original_start as f64))*fades.rate as f64).round().clamp(0.,fades.duration as f64)as u64;
            drag.fade_frames=if drag.mode==RegionDragMode::FadeIn{frame}else{fades.duration-frame};
        }},
    }cx.notify();
}
fn region_up(ed:&mut Editor,pointer:PointerId,window:&mut Window,cx:&mut Context<Editor>){
    if ed.integrations.audio_edit.range_drag.as_ref().is_some_and(|drag|drag.pointer==pointer){
        ed.integrations.audio_edit.range_drag=None;cx.notify();return;
    }

    if !ed.arrange_drag.as_ref().is_some_and(|drag|drag.pointer==pointer){return;}let drag=ed.arrange_drag.take().unwrap();
    if !drag.armed {
        if matches!(drag.target,AuthoredTarget::Audio(_)) {
            if let Ok(clip)=ed.selected_audio_clip(){ed.select_audio_clip(&clip,false,false);}
        }
        cx.notify();return;
    }
    if drag.base!=ed.revision(){ed.notice("片段拖动期间工程已改变，未写入预览");cx.notify();return;}
    if drag.start==drag.original_start&&drag.end==drag.original_end&&drag.source_track==drag.destination_track&&drag.fade_frames==drag.original_fade_frames&&drag.audio.as_ref().is_none_or(|audio|audio.start==audio.original_start&&audio.end==audio.original_end){cx.notify();return;}
    let result=(||->Result<(),String>{match drag.target{
        AuthoredTarget::Part(id)=>{let mut parts=verdant_domain::parts::Parts::load(ed.project())?;let part=parts.instances.iter_mut().find(|part|part.id==id).ok_or("片段已不存在")?;if drag.mode==RegionDragMode::Left{part.source_offset_tick+=drag.start-drag.original_start;}part.start_tick=drag.start;part.duration_tick=drag.end-drag.start;part.track=drag.destination_track.clone();parts.validate(ed.project())?;if !ed.commit(drag.base,CommandBatch::single("拖动片段",parts.command()),cx){return Err("片段未提交".into());}}
        AuthoredTarget::Audio(id)=>{
            use verdant_service::audio_edit::{AudioEdit,AudioClipRef,AudioClipProperties};
            let clip=verdant_media::read_clips(ed.project().track(&drag.source_track).ok_or("音频轨道不存在")?).map_err(|e|e.to_string())?.into_iter().find(|clip|clip.id.to_string()==id).ok_or("音频片段不存在")?;
            let reference=AudioClipRef::from(&clip);let tempo=ed.tempo();let rate=clip.source.sample_rate as f64*clip.playback_rate;
            let operation=match drag.mode{
                RegionDragMode::Move=>{
                    let source=ed.project().tracks.iter().position(|track|track.id==drag.source_track).ok_or("源轨道不存在")?as i32;
                    let destination=ed.project().tracks.iter().position(|track|track.id==drag.destination_track).ok_or("目标轨道不存在")?as i32;
                    AudioEdit::Move{clips:ed.audio_selection(),delta_ticks:drag.audio.as_ref().map_or((drag.start-drag.original_start)as f64,|audio|audio.delta),track_offset:destination-source}
                },
                RegionDragMode::Left=>{
                    let start=drag.audio.as_ref().map_or(drag.start as f64,|audio|audio.start);
                    let frames=((tempo.seconds_at(start)-clip.start_seconds(&tempo))*rate).round()as i128;
                    let offset=clip.source_offset_frames as i128+if clip.reversed{0}else{frames};
                    let duration=clip.duration_frames as i128-frames;
                    if offset<0||duration<1{return Err("音源裁剪超出范围".into());}
                    let start=tempo.tick_at(clip.start_seconds(&tempo)+frames as f64/rate);
                    AudioEdit::Trim{clip:reference,source_offset_frames:offset as u64,duration_frames:duration as u64,start_tick:Some(start)}
                },
                RegionDragMode::Right=>{
                    let end=drag.audio.as_ref().map_or(drag.end as f64,|audio|audio.end);
                    let duration=((tempo.seconds_at(end)-clip.start_seconds(&tempo))*rate).round()as i128;
                    let offset=clip.source_offset_frames as i128+if clip.reversed{clip.duration_frames as i128-duration}else{0};
                    if offset<0||duration<1{return Err("音源裁剪超出范围".into());}
                    AudioEdit::Trim{clip:reference,source_offset_frames:offset as u64,duration_frames:duration as u64,start_tick:None}
                },
                RegionDragMode::FadeIn|RegionDragMode::FadeOut=>{
                    let mut properties=AudioClipProperties::default();let milliseconds=drag.fade_frames as f64/rate*1000.;
                    if drag.mode==RegionDragMode::FadeIn{properties.fade_in_ms=Some(milliseconds);}else{properties.fade_out_ms=Some(milliseconds);}
                    AudioEdit::SetProperties{clips:vec![reference],properties}
                },
            };ed.apply_audio_edit(operation,cx)?;
        }

    }ed.select_track(drag.destination_track,cx);Ok(())})();if let Err(error)=result{ed.notice(error);}cx.notify();
}

#[cfg(test)]
mod blocked_label_tests {
    use super::*;

    fn track_plan(track: &verdant_domain::Track, diagnostics: Vec<verdant_backend::Diagnostic>) -> PhrasePlan {
        let note_ids: Vec<_> = track.notes.iter().map(|n| n.id.clone()).collect();
        PhrasePlan {
            track: track.id.clone(), route: verdant_backend::Route::VerdantVoice, phrases: Vec::new(),
            blocked: vec![verdant_backend::BlockedPhrase { note_ids, start_tick: 0, end_tick: 960 }],
            diagnostics,
        }
    }

    #[test]
    fn blocked_region_label_names_the_first_error() {
        let mut project = Project::new_empty();
        let track = &mut project.tracks[0];
        let note = verdant_domain::Note::new(0, 960, 60);
        let id = note.id.clone();
        track.notes = vec![note];
        track.refresh_index();
        let diagnostics = vec![
            verdant_backend::Diagnostic { code: "E_PRONUNCIATION_UNSET", severity: verdant_backend::Severity::Error, note_id: Some(id.clone()), message: "「熱」缺少已确认的读音".into(), detail: "x".into() },
            verdant_backend::Diagnostic { code: "E_PRONUNCIATION_UNSET", severity: verdant_backend::Severity::Error, note_id: Some(id), message: "「異」缺少已确认的读音".into(), detail: "x".into() },
        ];
        let track_id = project.tracks[0].id.clone();
        let plan = track_plan(&project.tracks[0], diagnostics);
        let mut cache = ArrangeCache::default();
        cache.rebuild(&project, Revision::new(1).unwrap(), Some(std::slice::from_ref(&plan)));
        let index = &cache.tracks[&track_id];
        assert_eq!(index.blocked_labels, vec!["预检未通过 · 2 处错误 · 「熱」缺少已确认的读音".to_string()]);
        // Diagnostics outside the blocked span must not leak into its label.
        let plan = PhrasePlan { blocked: vec![verdant_backend::BlockedPhrase { note_ids: vec![], start_tick: 0, end_tick: 960 }], ..track_plan(&project.tracks[0], Vec::new()) };
        let mut cache = ArrangeCache::default();
        cache.rebuild(&project, Revision::new(1).unwrap(), Some(std::slice::from_ref(&plan)));
        assert_eq!(cache.tracks[&track_id].blocked_labels, vec!["预检未通过 · 含错误".to_string()]);
    }
}

#[cfg(test)]
mod part_gesture_tests {
    use super::*;
    use verdant_domain::model::TimeSignature;
    use verdant_domain::parts::{PartInstance, Parts};

    fn meter() -> verdant_domain::MeterMap {
        let sig = |tick, numerator, denominator| TimeSignature { tick, numerator, denominator, unknown: Default::default() };
        // 4/4 bars of 3840 ticks, then 3/4 from bar 3 (tick 7680).
        verdant_domain::MeterMap::new(&[sig(0, 4, 4), sig(7680, 3, 4)], 960).unwrap()
    }

    #[test]
    fn arrangement_snap_uses_beats_or_bars_by_zoom() {
        let m = meter();
        // Zoomed in: a beat is 960 * 0.05 = 48 px, so snap to beats.
        assert_eq!(snap_to_meter(&m, 1500.0, 0.05), 1920);
        assert_eq!(snap_to_meter(&m, 1300.0, 0.05), 960);
        // Zoomed out: a beat is ~5 px, so snap to bars (meter-aware).
        assert_eq!(snap_to_meter(&m, 1500.0, 0.005), 0);
        assert_eq!(snap_to_meter(&m, 2000.0, 0.005), 3840);
        assert_eq!(snap_to_meter(&m, 7680.0 + 1500.0, 0.005), 7680 + 2880);
        assert_eq!(snap_to_meter(&m, -50.0, 0.005), 0);
    }

    #[test]
    fn new_part_fills_the_free_part_of_the_bar() {
        let m = meter();
        let track = TrackId::from("t");
        let part = |start: i64, duration: i64| PartInstance { id: format!("p{start}"), content_id: "c".into(), name: "x".into(),
            track: track.clone(), start_tick: start, duration_tick: duration, source_offset_tick: 0, muted: false, bindings: Default::default() };
        let mut parts = Parts::default();
        assert_eq!(new_part_span(&m, &parts, &track, 4000.0), Some((3840, 7680)));
        assert_eq!(new_part_span(&m, &parts, &track, 8000.0), Some((7680, 7680 + 2880)), "3/4 bar");
        parts.instances.push(part(3840, 960));
        parts.instances.push(part(6720, 960));
        assert_eq!(new_part_span(&m, &parts, &track, 5000.0), Some((4800, 6720)), "trimmed to neighbours");
        assert_eq!(new_part_span(&m, &parts, &track, 4000.0), None, "occupied");
        let other = TrackId::from("u");
        assert_eq!(new_part_span(&m, &parts, &other, 4000.0), Some((3840, 7680)), "other tracks ignored");
    }
}

#[cfg(test)]
mod source_keyboard_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, size};

    struct Harness { editor: Entity<Editor> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(region_lanes(self.editor.clone()))
        }
    }

    #[gpui::test]
    fn audio_source_shortcut_and_escape_retire_the_same_native_range_contact(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(320.0), px(140.0)), |window, cx| {
            let editor = cx.new(|cx| {
                let mut ed = Editor::for_native_test(Project::new_empty(), window, cx);
                ed.needs_arrange_fit = false; ed.arrange.start_tick = 0.0; ed.arrange.px_per_tick = 1.0;
                ed
            });
            Harness { editor }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, cx| assert!(crate::proto_editor_surface::focus_commands("audio", window, cx))).unwrap();
        cx.simulate_keystrokes(window.into(), "r");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).integrations.audio_edit.range_tool);
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position:gpui::point(px(50.0),px(20.0)),button:MouseButton::Left,..Default::default()
            }), cx);
            assert!(host.editor.read(cx).integrations.audio_edit.range_drag.is_some());
            assert!(crate::proto_editor_surface::has_contact(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_some());
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "escape");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let ed = host.editor.read(cx);
            assert!(!ed.integrations.audio_edit.range_tool);
            assert!(ed.integrations.audio_edit.range.is_none());
            assert!(ed.integrations.audio_edit.range_drag.is_none());
            assert_eq!(ed.revision().get(), 0);
            assert!(!crate::proto_editor_surface::has_contact(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_none());
        }).unwrap();
    }
}
