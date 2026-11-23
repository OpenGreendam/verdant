//! Native SDK owner and composition shell for the compiler-owned roll surface.
//! The nested EditorSurface now owns note, pitch and modulation contacts alike;
//! this shell shares the generated SDK with all remaining compiler units.
use gpui::{App, AppContext, Context, Entity, IntoElement, Render, WeakEntity, Window};
use verdant_pitch_gesture_surface::{PitchGestureSurface, native::gpui_native_owner::NativeSnapshot};
use crate::editor::Editor;

pub(crate) struct PitchMove { editor: WeakEntity<Editor> }
impl PitchMove {
    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor)=self.editor.upgrade() { crate::proto_editor_surface::cancel_from_editor(editor.entity_id(),window,cx); }
    }
    pub(crate) fn cancel_deferred(&self, cx: &mut App) {
        if let Some(editor)=self.editor.upgrade() { crate::proto_editor_surface::cancel_from_editor_deferred(editor.entity_id(),cx); }
    }
}

pub(crate) fn mount(editor: WeakEntity<Editor>, window: &mut Window, cx: &mut App)
    -> (Entity<PitchGestureSurface>, PitchMove) {
    let view=cx.new(|_|NativeRoll { editor:editor.clone() });
    let root=PitchGestureSurface::create(serde_json::json!({}),None,window,cx)
        .unwrap_or_else(|e|panic!("compiled pitch shell setup failed: {e:?}"));
    root.update(cx,|root,cx|root.set_slot(vec![NativeSnapshot::native_view(view.into())],cx))
        .unwrap_or_else(|e|panic!("compiled pitch shell rejected its drawing view: {e:?}"));
    (root,PitchMove { editor })
}
struct NativeRoll { editor: WeakEntity<Editor> }
impl Render for NativeRoll {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        match self.editor.upgrade() {
            Some(editor)=>crate::roll::piano_roll(editor).into_any_element(),
            None=>gpui::div().into_any_element(),
        }
    }
}
