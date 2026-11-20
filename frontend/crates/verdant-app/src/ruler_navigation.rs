//! One ruler owns a scrub until its primary contact ends. Edge scrolling uses
//! elapsed time and live clipped bounds, including while the contact is still.
use std::time::Instant;

use gpui::{
    App, Bounds, Entity, EntityId, Global, Hitbox, HitboxId, MouseButton, MouseExitEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, TouchDragEvent, TouchPhase, WeakEntity, Window,
};

use crate::{clock::Space, editor::{Bottom, Editor}};

#[derive(Clone, Copy, PartialEq)]
enum Contact {
    Mouse(HitboxId),
    Touch(Point<Pixels>),
}

#[derive(Clone, Copy)]
struct Scrub {
    editor: EntityId,
    arrange: bool,
    serial: u64,
    contact: Contact,
    bounds: Bounds<Pixels>,
    clip: Bounds<Pixels>,
    position: Point<Pixels>,
    last_frame: Option<Instant>,
}

#[derive(Default)]
struct RulerState {
    serial: u64,
    scrub: Option<Scrub>,
}
impl Global for RulerState {}

fn active(cx: &App) -> Option<Scrub> {
    cx.try_global::<RulerState>().and_then(|state| state.scrub)
}

pub(crate) fn is_active(editor: EntityId, cx: &App) -> bool {
    active(cx).is_some_and(|scrub| scrub.editor == editor)
}

pub(crate) fn cancel(editor: EntityId, window: &mut Window, cx: &mut App) {
    if let Some(scrub) = active(cx).filter(|scrub| scrub.editor == editor) {
        cx.global_mut::<RulerState>().scrub = None;
        if let Contact::Mouse(id) = scrub.contact {
            if window.captured_hitbox() == Some(id) {
                window.release_pointer();
            }
        }
    }
}

pub(crate) fn begin(
    editor: &Entity<Editor>, arrange: bool, bounds: Bounds<Pixels>, hitbox: &Hitbox,
    window: &mut Window, cx: &mut App, position: Point<Pixels>,
) -> bool {
    begin_contact(editor, arrange, bounds, bounds.intersect(&hitbox.content_mask.bounds), Contact::Mouse(hitbox.id), position, window, cx)
}

fn begin_contact(
    editor: &Entity<Editor>, arrange: bool, bounds: Bounds<Pixels>, clip: Bounds<Pixels>,
    contact: Contact, position: Point<Pixels>, window: &mut Window, cx: &mut App,
) -> bool {
    if active(cx).is_some() || window.captured_hitbox().is_some() || !clip.contains(&position) {
        return false;
    }
    if cx.try_global::<RulerState>().is_none() {
        cx.set_global(RulerState::default());
    }
    let state = cx.global_mut::<RulerState>();
    state.serial = state.serial.wrapping_add(1);
    let scrub = Scrub { editor: editor.entity_id(), arrange, serial: state.serial, contact, bounds, clip, position, last_frame: None };
    state.scrub = Some(scrub);
    match contact {
        Contact::Mouse(id) => window.capture_pointer(id),
        Contact::Touch(_) => window.prevent_default(),
    }
    editor.update(cx, |ed, cx| {
        window.focus(if arrange { &ed.focus_arrange } else { &ed.focus_roll }, cx);
    });
    apply_sample(editor, scrub, 0.0, true, cx);
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

    let player = editor.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if !phase.capture() { return; }
        let Some(scrub) = matching(cx, editor_id, arrange).filter(|scrub| matches!(scrub.contact, Contact::Mouse(_))) else { return };
        if event.pressed_button != Some(MouseButton::Left) || !owns_mouse(scrub, window) {
            cancel(editor_id, window, cx);
            return;
        }
        move_to(&player, scrub, event.position, window, cx);
        cx.stop_propagation();
    });
    let player = editor.clone();
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if !phase.capture() || event.button != MouseButton::Left { return; }
        let Some(scrub) = matching(cx, editor_id, arrange).filter(|scrub| matches!(scrub.contact, Contact::Mouse(_))) else { return };
        if owns_mouse(scrub, window) {
            finish_at(&player, scrub, event.position, window, cx);
            cx.stop_propagation();
        } else {
            cancel(editor_id, window, cx);
        }
    });
    window.on_mouse_event(move |event: &MouseExitEvent, phase, window, cx| {
        if !phase.capture() || event.pressed_button == Some(MouseButton::Left) { return; }
        if matching(cx, editor_id, arrange).is_some_and(|scrub| matches!(scrub.contact, Contact::Mouse(_))) {
            cancel(editor_id, window, cx);
        }
    });
    let player = editor.clone();
    let hitbox = hitbox.clone();
    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
        if event.phase == TouchPhase::Started {
            if phase.bubble() && !window.default_prevented() && hitbox.is_hovered_at(event.start_position, window)
                && begin_contact(&player, arrange, bounds, clip, Contact::Touch(event.start_position), event.start_position, window, cx)
            {
                cx.stop_propagation();
            }
            return;
        }
        if !phase.capture() { return; }
        let Some(scrub) = matching(cx, editor_id, arrange).filter(|scrub| scrub.contact == Contact::Touch(event.start_position)) else { return };
        match event.phase {
            TouchPhase::Moved => move_to(&player, scrub, event.position, window, cx),
            TouchPhase::Ended => finish_at(&player, scrub, event.position, window, cx),
            TouchPhase::Cancelled => cancel(editor_id, window, cx),
            TouchPhase::Started => unreachable!(),
        }
        cx.stop_propagation();
    });
}

fn matching(cx: &App, editor: EntityId, arrange: bool) -> Option<Scrub> {
    active(cx).filter(|scrub| scrub.editor == editor && scrub.arrange == arrange)
}

fn owns_mouse(scrub: Scrub, window: &Window) -> bool {
    match scrub.contact {
        Contact::Mouse(id) => window.captured_hitbox() == Some(id),
        Contact::Touch(_) => true,
    }
}

fn edge(scrub: Scrub) -> (f64, f64) {
    let dx = (scrub.position.x - scrub.bounds.origin.x).as_f32() as f64;
    let left = (scrub.clip.left() - scrub.bounds.left()).as_f32() as f64;
    let right = (scrub.clip.right() - scrub.bounds.left()).as_f32() as f64;
    let outside = dx - dx.clamp(left, right.max(left));
    // Keep the indicator's pixel inside the right edge of the roll.
    (dx.clamp(left, (right - 1.0).max(left)), outside)
}

fn pan_velocity(outside_px: f64) -> f64 {
    if outside_px == 0.0 { return 0.0; }
    outside_px.signum() * (100.0 + 9.0 * outside_px.abs()).min(2000.0)
}

fn apply_sample(editor: &Entity<Editor>, scrub: Scrub, elapsed: f64, snap: bool, cx: &mut App) -> bool {
    let (x, outside) = edge(scrub);
    editor.update(cx, |ed, cx| {
        if !scrub.arrange && ed.bottom != Bottom::Editor { return false; }
        let viewport = if scrub.arrange { &mut ed.arrange } else { &mut ed.session.viewport };
        let before = viewport.start_tick;
        if outside != 0.0 && elapsed > 0.0 {
            viewport.pan_px(pan_velocity(outside) * elapsed, 0.0);
        }
        let raw_tick = viewport.x_to_tick(x);
        let tick = if snap { ed.session.snap.snap(raw_tick) as f64 } else { raw_tick };
        let changed = before != viewport.start_tick;
        let at_start = raw_tick <= 0.0;
        ed.seek_tick(tick.max(0.0));
        cx.notify();
        if changed {
            ed.notify_viewport(if scrub.arrange { Space::Arrange } else { Space::Editor }, cx);
        }
        outside > 0.0 || (outside < 0.0 && !at_start)
    })
}

fn move_to(editor: &Entity<Editor>, mut scrub: Scrub, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    scrub.position = position;
    let at_edge = apply_sample(editor, scrub, 0.0, false, cx);
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

fn finish_at(editor: &Entity<Editor>, mut scrub: Scrub, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    if scrub.position != position {
        scrub.position = position;
        apply_sample(editor, scrub, 0.0, false, cx);
    }
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
        if !window.is_window_active() || !owns_mouse(scrub, window) {
            cancel(expected.editor, window, cx);
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(scrub.last_frame.unwrap()).as_secs_f64().min(0.05);
        let keep = apply_sample(&editor, scrub, elapsed, false, cx);
        scrub.last_frame = keep.then_some(now);
        cx.global_mut::<RulerState>().scrub = Some(scrub);
        if keep { next_frame(editor.downgrade(), scrub, window); }
    });
}
