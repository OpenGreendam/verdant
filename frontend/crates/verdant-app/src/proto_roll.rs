//! Native SDK owner and composition shell for the compiler-owned roll surface.
//! The nested EditorSurface now owns note, pitch and modulation contacts alike;
//! this shell shares the generated SDK with all remaining compiler units.
use gpui::{App, AppContext, Context, Entity, IntoElement, Render, RenderOnce, WeakEntity, Window};
use verdant_pitch_gesture_surface::{PitchGestureSurface,PitchGestureSurfaceHandle, native::gpui_native_owner::NativeSnapshot};
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
    let root=PitchGestureSurface::create(serde_json::json!({}),None,window,cx)
        .unwrap_or_else(|e|panic!("compiled pitch shell setup failed: {e:?}"));
    let handle=root.read(cx).handle();
    let view=cx.new(|_|NativeRoll { editor:editor.clone(),handle });
    root.update(cx,|root,cx|root.set_slot(vec![NativeSnapshot::native_view(view.into())],cx))
        .unwrap_or_else(|e|panic!("compiled pitch shell rejected its drawing view: {e:?}"));
    (root,PitchMove { editor })
}
struct NativeRoll { editor: WeakEntity<Editor>,handle:PitchGestureSurfaceHandle }
impl Render for NativeRoll {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        let content=match self.editor.upgrade() {
            Some(editor)=>crate::roll::piano_roll(editor).into_any_element(),
            None=>gpui::div().into_any_element(),
        };
        crate::proto_parent::slot(&self.handle.owner,content)
    }
}

#[derive(IntoElement)]
pub(crate) struct View{root:Entity<PitchGestureSurface>}
pub(crate) fn view(root:Entity<PitchGestureSurface>)->View{View{root}}
impl RenderOnce for View{
    fn render(self,_:&mut Window,cx:&mut App)->impl IntoElement{
        let parent=crate::proto_parent::current();
        if crate::proto_parent::changed(&self.root.read(cx).handle().owner,&parent){
            self.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled piano-roll parent");
        }
        self.root
    }
}
