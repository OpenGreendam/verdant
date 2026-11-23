//! One ruler owns a scrub until its primary contact ends. Edge scrolling uses
//! elapsed time and live clipped bounds, including while the contact is still.
use std::time::Instant;

use gpui::{App, Bounds, Entity, EntityId, Global, Hitbox, Pixels, Point, WeakEntity, Window};

use crate::{clock::Space, editor::{Bottom, Editor}};

#[derive(Clone, Copy)]
struct Scrub {
    editor: EntityId,
    arrange: bool,
    serial: u64,
    bounds: Bounds<Pixels>,
    clip: Bounds<Pixels>,
    position: Point<Pixels>,
    last_frame: Option<Instant>,
}

#[derive(Default)]
struct RulerState {
    serial: u64,
    scrub: Option<Scrub>,
    input: Option<crate::proto_editor_surface::Input>,
}
impl Global for RulerState {}

fn active(cx: &App) -> Option<Scrub> {
    cx.try_global::<RulerState>().and_then(|state| state.scrub)
}

pub(crate) fn is_active(editor: EntityId, cx: &App) -> bool {
    active(cx).is_some_and(|scrub| scrub.editor == editor)
}

pub(crate) fn cancel(editor: EntityId, window: &mut Window, cx: &mut App) {
    if active(cx).is_some_and(|scrub| scrub.editor == editor) {
        cx.global_mut::<RulerState>().scrub = None;
        cx.global_mut::<RulerState>().input = None;
        crate::proto_editor_surface::cancel_from_editor(editor,window,cx);
    }
}

fn begin_contact(
    editor: &Entity<Editor>, arrange: bool, bounds: Bounds<Pixels>, clip: Bounds<Pixels>,
    position: Point<Pixels>, input:&crate::proto_editor_surface::Input, window: &mut Window, cx: &mut App,
) -> bool {
    if active(cx).is_some() || window.captured_hitbox().is_some() || !clip.contains(&position) {
        return false;
    }
    if cx.try_global::<RulerState>().is_none() {
        cx.set_global(RulerState::default());
    }
    let state = cx.global_mut::<RulerState>();
    state.serial = state.serial.wrapping_add(1);
    let scrub = Scrub { editor: editor.entity_id(), arrange, serial: state.serial, bounds, clip, position, last_frame: None };
    state.scrub = Some(scrub);
    state.input = Some(input.clone());
    // The admitted EditorSurface contact owns capture/touch arbitration. This
    // state stores only the musical seek geometry and edge-scroll clock.
    apply_sample(editor, scrub, 0.0, window, cx);
    true
}

pub(crate) fn install(editor: &Entity<Editor>, arrange: bool, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window, cx: &mut App) {
    let editor_id = editor.entity_id();
    let clip = bounds.intersect(&hitbox.content_mask.bounds);
    if active(cx).is_some_and(|scrub| scrub.editor == editor_id && scrub.arrange == arrange) {
        let scrub = cx.global_mut::<RulerState>().scrub.as_mut().unwrap();
        scrub.bounds = bounds;
        scrub.clip = clip;
    }

    // Contact admission/terminal ownership belongs to the compiled surface.
    // This native state stores only clipping, timing and musical seek geometry.
}

pub(crate) fn begin_pointer(editor:&Entity<Editor>,arrange:bool,bounds:Bounds<Pixels>,hitbox:&Hitbox,
    input:&crate::proto_editor_surface::Input,window:&mut Window,cx:&mut App,position:Point<Pixels>)->bool {
    begin_contact(editor,arrange,bounds,bounds.intersect(&hitbox.content_mask.bounds),position,input,window,cx)
}
pub(crate) fn sample(editor:&Entity<Editor>,arrange:bool,position:Point<Pixels>,window:&mut Window,cx:&mut App) {
    if let Some(scrub)=matching(cx,editor.entity_id(),arrange) { move_to(editor,scrub,position,window,cx); }
}
pub(crate) fn finish(editor:&Entity<Editor>,arrange:bool,position:Point<Pixels>,window:&mut Window,cx:&mut App) {
    if let Some(scrub)=matching(cx,editor.entity_id(),arrange) { finish_at(editor,scrub,position,window,cx); }
}

fn matching(cx: &App, editor: EntityId, arrange: bool) -> Option<Scrub> {
    active(cx).filter(|scrub| scrub.editor == editor && scrub.arrange == arrange)
}

fn edge(scrub: Scrub) -> (f64, f64) {
    let dx = (scrub.position.x - scrub.bounds.origin.x).as_f32() as f64;
    let left = (scrub.clip.left() - scrub.bounds.left()).as_f32() as f64;
    let right = (scrub.clip.right() - scrub.bounds.left()).as_f32() as f64;
    let outside = dx - dx.clamp(left, right.max(left));
    // Keep the indicator's pixel inside the right edge of the roll.
    (dx.clamp(left, (right - 1.0).max(left)), outside)
}

fn apply_sample(editor: &Entity<Editor>, scrub: Scrub, elapsed: f64, window:&mut Window,cx: &mut App) -> bool {
    let (x, outside) = edge(scrub);
    let Some(input)=cx.global::<RulerState>().input.clone() else {return false;};
    editor.update(cx, |ed, cx| {
        if !scrub.arrange && ed.bottom != Bottom::Editor { return false; }
        let Some(intent)=input.ruler_sample(ed,x,outside,elapsed,window,cx) else {return false;};
        let viewport = if scrub.arrange { &mut ed.arrange } else { &mut ed.session.viewport };
        let before = viewport.start_tick;
        viewport.pan_px(intent.delta,0.0);
        let raw_tick = viewport.x_to_tick(intent.position);
        let tick = if intent.snap { ed.session.snap.snap(raw_tick) as f64 } else { raw_tick };
        let changed = before != viewport.start_tick;
        let at_start = raw_tick <= 0.0;
        ed.seek_tick(tick.max(0.0));
        cx.notify();
        if changed {
            ed.notify_viewport(if scrub.arrange { Space::Arrange } else { Space::Editor }, cx);
        }
        intent.active && (outside > 0.0 || (outside < 0.0 && !at_start))
    })
}

fn move_to(editor: &Entity<Editor>, mut scrub: Scrub, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    scrub.position = position;
    let at_edge = apply_sample(editor, scrub, 0.0, window,cx);
    let schedule = at_edge && scrub.last_frame.is_none();
    if schedule {
        scrub.last_frame = Some(Instant::now());
    } else if !at_edge {
        scrub.last_frame = None;
    }
    cx.global_mut::<RulerState>().scrub = Some(scrub);
    if schedule {
        next_frame(editor.downgrade(), scrub, window);
    }
}

fn finish_at(_: &Entity<Editor>, scrub: Scrub, _: Point<Pixels>, window: &mut Window, cx: &mut App) {
    // The contact adapter delivered the final accepted sample before finish.
    cancel(scrub.editor, window, cx);
}

fn next_frame(editor: WeakEntity<Editor>, expected: Scrub, window: &Window) {
    window.on_next_frame(move |window, cx| {
        let Some(mut scrub) = matching(cx, expected.editor, expected.arrange)
            .filter(|scrub| scrub.serial == expected.serial && scrub.last_frame == expected.last_frame) else { return };
        let Some(editor) = editor.upgrade() else {
            cancel(expected.editor, window, cx);
            return;
        };
        let input=cx.global::<RulerState>().input.clone();
        if !window.is_window_active() || !input.as_ref().is_some_and(|input|input.contact_current(window,cx)) {
            if let Some(input)=input {input.cancel(window,cx);}
            cancel(expected.editor, window, cx);
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(scrub.last_frame.unwrap()).as_secs_f64();
        let keep = apply_sample(&editor, scrub, elapsed, window,cx);
        scrub.last_frame = keep.then_some(now);
        cx.global_mut::<RulerState>().scrub = Some(scrub);
        if keep { next_frame(editor.downgrade(), scrub, window); }
    });
}
