//! Compiled points over the existing native device graph. Only parameter units,
//! graph geometry and the durable multi-parameter transaction remain native.
use super::*;
use std::cell::{Cell,RefCell};
use gpui::{App,AppContext,AnyElement,Render,RenderOnce,Subscription,WeakEntity};
use serde_json::{Value,json};
use verdant_graph_point::{GraphPoint,GraphPointHandle,native::{
    gpui_native_owner::NativeSnapshot,
    gpui_native_move::{MoveGestureBinding,MoveGestureCancelContext,MoveGestureCancelReason,MoveGestureInput,MoveGestureLease,MoveGestureSample,attach_move_gesture},
}};

#[derive(Clone)]
struct PointTarget {
    editor:WeakEntity<Editor>,target:Target,scope:DeviceScope,point:Handle,basis:Option<DeviceBasis>,
    plot:Rc<Cell<Plot>>,surface:crate::host_pointer::SurfaceToken,menu:Option<HandleMenu>,index:usize,
}
impl PointTarget {
    fn revision(&self)->String { json!({"basis":self.basis.as_ref().map(|basis|json!({"project":basis.project,"generation":basis.generation,"revision":basis.revision.to_string(),"params":basis.params})),
        "control":format!("{:?}",self.point.control),"x":format!("{:?}",self.point.x),"y":format!("{:?}",self.point.y)}).to_string() }
    fn current(&self,ed:&Editor,cx:&App)->bool {
        if self.editor.upgrade().is_none_or(|editor|crate::proto_editor_surface::has_contact(editor.entity_id(),cx)){return false;}
        self.surface.is_live()&&self.scope.allows(&self.target.track,ed)&&self.basis.is_some()
            &&self.target.basis(ed)==self.basis&&device_input_available(ed)
            &&ed.device_drag.as_ref().is_none_or(|drag|self.owns(drag))
    }
    fn owns(&self,drag:&DeviceDrag)->bool {drag.key==self.target.key&&drag.control==self.point.control&&drag.surface.same(&self.surface)}
    fn values(&self,x:f64,y:f64)->Option<Vec<(String,f64)>> {
        if !x.is_finite()||!y.is_finite()||!(0.0..=1.0).contains(&x)||!(0.0..=1.0).contains(&y){return None;}
        let params=&self.basis.as_ref()?.params;let mut values=Vec::new();
        for (coordinate,axis) in [(x,&self.point.x),(y,&self.point.y)] {
            if let Some((id,axis))=axis {
                let desc=param(params,id)?;
                values.push((id.clone(),bounded_graph_value(desc,*axis,coordinate)));
            }
        }
        values.extend(self.point.also.iter().cloned());
        values.iter().all(|(_,value)|value.is_finite()).then_some(values)
    }
}
fn state(handle:&GraphPointHandle,name:&str)->Option<Value>{handle.state(name).ok()}
fn idle(handle:&GraphPointHandle)->bool{state(handle,"phase").as_ref().and_then(Value::as_str)==Some("idle")}
fn report(target:&PointTarget,message:impl Into<String>,cx:&mut App){let message=message.into();let _=target.editor.update(cx,|ed,cx|{ed.notice(message);cx.notify();});}
fn clear_preview(target:&PointTarget,window:&mut Window,cx:&mut App){
    let _=target.editor.update(cx,|ed,cx| {
        if !ed.device_drag.as_ref().is_some_and(|drag|target.owns(drag)){return;}
        if let Some(mut drag)=ed.device_drag.take(){if let Some(capture)=drag.capture.take(){capture.release(window,cx);}}
        cx.notify();
    });
}
fn preview(handle:&GraphPointHandle,target:&PointTarget,cx:&mut App){
    let Some(x)=state(handle,"x").and_then(|v|v.as_f64())else{return;};
    let Some(y)=state(handle,"y").and_then(|v|v.as_f64())else{return;};
    let Some(values)=target.values(x,y)else{return;};
    let phase=if state(handle,"phase").as_ref().and_then(Value::as_str)==Some("dragging"){GesturePhase::Dragging}else{GesturePhase::Armed};
    let _=target.editor.update(cx,|ed,cx|{if let Some(drag)=ed.device_drag.as_mut().filter(|drag|target.owns(drag)){
        // A projection only: no native code makes a transition decision.
        drag.phase=phase;drag.preview=values;cx.notify();
    }});
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {x:f64,y:f64,reset:bool,revision:String}
fn drain(handle:&GraphPointHandle,target:&PointTarget,window:&mut Window,cx:&mut App)->bool {
    let failures=handle.take_failures();let events=handle.take_events();
    if !failures.is_empty(){clear_preview(target,window,cx);report(target,format!("图形控件交互失败：{failures:?}"),cx);return false;}
    // Validate every prospective write before delivering any event in a batch.
    if events.iter().any(|(name,payload)|name=="valueCommit"&&serde_json::from_value::<Commit>(payload.clone()).ok()
        .is_none_or(|commit|!commit.x.is_finite()||!commit.y.is_finite()||!(0.0..=1.0).contains(&commit.x)||!(0.0..=1.0).contains(&commit.y))){
        clear_preview(target,window,cx);report(target,"图形控件返回无效的原子提议",cx);return false;
    }
    for (name,payload) in events {
        match name.as_str(){
            "interactionCancel"=>clear_preview(target,window,cx),
            "selected"=>{let _=target.editor.update(cx,|ed,cx|{if target.current(ed,cx){if let Control::Band(index)=target.point.control{select_band(ed,target.target.key,index);cx.notify();}}});},
            "valueCommit"=>{
                let commit:Commit=serde_json::from_value(payload).expect("validated graph proposal");
                if commit.revision!=target.revision(){continue;}
                let values=if commit.reset{Some(target.point.defaults.clone())}else{target.values(commit.x,commit.y)};
                let Some(values)=values else{continue;};
                let _=target.editor.update(cx,|ed,cx|{
                    if !target.current(ed,cx){return;}
                    if let Some(mut drag)=ed.device_drag.take(){if let Some(capture)=drag.capture.take(){capture.release(window,cx);}}
                    write_if_current(ed,&target.target,values,cx);cx.notify();
                });
            },
            "context"=>{
                if payload.get("revision").and_then(Value::as_str)!=Some(target.revision().as_str()){continue;}
                let Some(menu)=target.menu.clone()else{continue;};
                let _=target.editor.update(cx,|ed,cx|{if target.current(ed,cx){
                    if let Control::Band(index)=target.point.control{select_band(ed,target.target.key,index);}
                    let items=menu(ed,target.index);ed.show_menu(window.mouse_position(),items,window,cx);
                }});
            },
            "exact"=>{
                if payload.get("revision").and_then(Value::as_str)!=Some(target.revision().as_str()){continue;}
                let axis=if payload.get("axis").and_then(Value::as_str)==Some("x"){&target.point.x}else{&target.point.y};
                if let Some((id,_))=axis {
                    if let Some(editor)=target.editor.upgrade(){
                        if target.current(editor.read(cx),cx){if let Some(desc)=target.basis.as_ref().and_then(|basis|param(&basis.params,id)).cloned(){open_parameter_value(&editor,target.target.track.clone(),target.target.slot,target.target.key,desc,cx);}}
                    }
                }
            },_=>{},
        }
    }
    true
}
fn cancel_source(handle:&GraphPointHandle,target:&PointTarget,window:&mut Window,cx:&mut App){
    let _=handle.call("cancel",vec![],window,cx);drain(handle,target,window,cx);clear_preview(target,window,cx);
}

/// Kept by the domain preview so editor-wide cancellation retires the compiled
/// acquisition. A queued retirement cannot cancel a later reuse of the root.
#[derive(Clone)]
pub(super) struct Retirement {handle:GraphPointHandle,contact:Rc<crate::proto_knob::Contact>,window:gpui::AnyWindowHandle}
impl Retirement {
    pub(super) fn cancel(&self,cx:&mut App){
        let expected=self.contact.generation.get();self.contact.input.set(None);
        let this=self.clone();cx.defer(move |cx|{
            if !this.contact.is_current(expected){return;}
            let _=this.window.update(cx,|_,window,cx|{
                let _=this.handle.call("cancel",vec![],window,cx);this.handle.take_events();this.handle.take_failures();
                let lease=this.contact.lease.borrow().as_ref().and_then(std::rc::Weak::upgrade);
                if let Some(lease)=lease{lease.cancel(MoveGestureCancelReason::HostCancel,window,cx);}
            });
        });
    }
}
struct Binding {handle:GraphPointHandle,target:Rc<RefCell<PointTarget>>,double:Rc<Cell<bool>>,
    contact:Rc<crate::proto_knob::Contact>,window:gpui::AnyWindowHandle,app:gpui::AsyncApp}
impl Binding {
    fn inside(&self,sample:&MoveGestureSample,window:&Window)->bool{
        self.handle.owner.borrow().native_hitbox.as_ref().is_some_and(|hitbox|crate::host_pointer::contains_window_point(window,
            (f64::from(hitbox.bounds.origin.x)+sample.position.x,f64::from(hitbox.bounds.origin.y)+sample.position.y)))
    }
    fn arguments(&self,sample:&MoveGestureSample)->Vec<Value>{let plot=self.target.borrow().plot.get();
        vec![sample.total_delta.x.into(),sample.total_delta.y.into(),plot.inner_w().into(),plot.inner_h().into(),sample.modifiers.shift.into()]}
    fn call(&self,method:&str,args:Vec<Value>,window:&mut Window,cx:&mut App)->bool{
        let target=self.target.borrow().clone();
        let result=self.handle.call(method,args,window,cx).ok().and_then(|v|v.as_bool()).unwrap_or(false);
        if !drain(&self.handle,&target,window,cx){self.contact.input.set(None);cancel_source(&self.handle,&target,window,cx);return false;}
        result
    }
}
impl MoveGestureBinding for Binding {
    fn should_start(&self,sample:&MoveGestureSample,window:&mut Window,cx:&mut App)->bool{
        if !matches!(sample.input,MoveGestureInput::Mouse|MoveGestureInput::Touch)||!self.inside(sample,window){return false;}
        let target=self.target.borrow();
        target.editor.upgrade().is_some_and(|entity|{let ed=entity.read(cx);
            target.current(ed,cx)&&may_begin_device_input(ed)&&!crate::proto_editor_surface::has_contact(entity.entity_id(),cx)&&!state(&self.handle,"disabled").and_then(|v|v.as_bool()).unwrap_or(true)})
    }
    fn on_start(&self,sample:&MoveGestureSample,window:&mut Window,cx:&mut App){
        let target=self.target.borrow().clone();self.contact.begin(sample.input);
        let _=target.editor.update(cx,|ed,_|ed.proto_pointer_contact=Some(Rc::downgrade(&self.contact)));
        if !idle(&self.handle){cancel_source(&self.handle,&target,window,cx);}
        let double=sample.input==MoveGestureInput::Mouse&&self.double.replace(false);
        if !self.call("begin",vec![sample.modifiers.alt.into(),double.into()],window,cx)||idle(&self.handle){self.contact.input.set(None);return;}
        let Some(hitbox)=self.handle.owner.borrow().native_hitbox.as_ref().map(|hitbox|hitbox.id)else{self.contact.input.set(None);cancel_source(&self.handle,&target,window,cx);return;};
        let Some(basis)=target.basis.clone()else{return;};
        let x=state(&self.handle,"x").and_then(|v|v.as_f64()).unwrap_or(0.);
        let y=state(&self.handle,"y").and_then(|v|v.as_f64()).unwrap_or(0.);
        let initial=target.values(x,y).unwrap_or_default();
        let capture=crate::host_pointer::OwnedPointerCapture::acquire(hitbox,window,cx);
        let retirement=Retirement{handle:self.handle.clone(),contact:self.contact.clone(),window:self.window};
        let _=target.editor.update(cx,|ed,cx|{
            if !target.current(ed,cx){return;}
            ed.device_drag=Some(DeviceDrag{track:target.target.track.clone(),slot:target.target.slot,key:target.target.key,
                control:target.point.control.clone(),phase:GesturePhase::Armed,capture:Some(capture),surface:target.surface.clone(),basis,
                scope:target.scope,preview:initial,source:Some(retirement)});cx.notify();
        });
    }
    fn on_move(&self,sample:&MoveGestureSample,window:&mut Window,cx:&mut App){
        if self.contact.input.get()!=Some(sample.input){return;}
        let target=self.target.borrow().clone();
        if !self.inside(sample,window)||!target.editor.upgrade().is_some_and(|e|target.current(e.read(cx),cx)){
            self.contact.input.set(None);cancel_source(&self.handle,&target,window,cx);return;
        }
        self.call("sample",self.arguments(sample),window,cx);preview(&self.handle,&target,cx);
    }
    fn on_end(&self,sample:&MoveGestureSample,window:&mut Window,cx:&mut App){
        if self.contact.input.replace(None)!=Some(sample.input){return;}
        let target=self.target.borrow().clone();
        if !self.inside(sample,window)||!target.editor.upgrade().is_some_and(|e|target.current(e.read(cx),cx)){cancel_source(&self.handle,&target,window,cx);return;}
        self.call("finish",self.arguments(sample),window,cx);clear_preview(&target,window,cx);
    }
    fn on_cancel(&self,_:MoveGestureCancelReason,context:Option<MoveGestureCancelContext<'_>>){
        if self.contact.input.replace(None).is_none(){return;}
        let target=self.target.borrow().clone();
        if let Some(context)=context{cancel_source(&self.handle,&target,context.window,context.cx);}
        else{let expected=self.contact.generation.get();let contact=self.contact.clone();let handle=self.handle.clone();let window=self.window;
            self.app.spawn(async move|cx|{if !contact.is_current(expected){return;}let _=window.update(cx,|_,window,cx|cancel_source(&handle,&target,window,cx));}).detach();}
    }
}
struct Drawing {handle:GraphPointHandle,target:Rc<RefCell<PointTarget>>,double:Rc<Cell<bool>>,theme:Theme}
impl Render for Drawing {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{
        let focused=state(&self.handle,"focusVisible").and_then(|v|v.as_bool()).unwrap_or(false);
        let double=self.double.clone();let handle=self.handle.clone();let target=self.target.clone();let th=self.theme;
        let drawing=canvas(|bounds,window,_|window.insert_hitbox(bounds,HitboxBehavior::Normal),move|bounds,hitbox,window,_|{
            if focused{window.paint_quad(quad(bounds,px(13.),gpui::transparent_black(),px(2.),th.accent,gpui::BorderStyle::Solid));}
            window.on_mouse_event(move|event:&MouseDownEvent,phase,window,cx|{
                if !hitbox.is_hovered(window){return;}
                if phase.capture()&&event.button==MouseButton::Left{double.set(event.click_count>=2);}
                if phase.bubble()&&event.button==MouseButton::Right{
                    let _=handle.call("openContext",vec![],window,cx);drain(&handle,&target.borrow(),window,cx);cx.stop_propagation();
                }
            });
        }).size_full();
        crate::proto_parent::slot(&self.handle.owner,drawing.into_any_element())
    }
}
struct Mount {root:Entity<GraphPoint>,drawing:Entity<Drawing>,target:Rc<RefCell<PointTarget>>,contact:Rc<crate::proto_knob::Contact>,lease:Rc<MoveGestureLease>,props:Value,_changes:Subscription}
#[derive(IntoElement)]
struct Point {target:PointTarget,theme:Theme}
impl RenderOnce for Point {
    fn render(mut self,window:&mut Window,cx:&mut App)->impl IntoElement{
        self.target.scope=DeviceScope::for_window(window);
        let editor=self.target.editor.upgrade();
        if let Some(editor)=editor.as_ref(){self.target.basis=self.target.target.basis(editor.read(cx));}
        let disabled=editor.as_ref().is_none_or(|editor|!self.target.current(editor.read(cx),cx));
        let params=self.target.basis.as_ref().map(|basis|basis.params.as_slice()).unwrap_or(&[]);
        let axis=|value:&Option<(String,Axis)>,fallback:f64|value.as_ref().and_then(|(id,axis)|param(params,id).map(|desc|
            (axis.norm(desc.value),axis.norm(desc.min).min(axis.norm(desc.max)),axis.norm(desc.min).max(axis.norm(desc.max)),desc.name.clone())))
            .unwrap_or((fallback,0.,1.,String::new()));
        let x=axis(&self.target.point.x,self.target.point.at.0);let y=axis(&self.target.point.y,self.target.point.at.1);
        let name=if matches!(self.target.point.control,Control::Band(_)){format!("频段 {}",self.target.point.label)}else if !self.target.point.label.is_empty(){self.target.point.label.clone()}else{[x.3.as_str(),y.3.as_str()].into_iter().filter(|name|!name.is_empty()).collect::<Vec<_>>().join(" / ")};
        let props=json!({"x":x.0,"y":y.0,"minX":x.1,"maxX":x.2,"minY":y.1,"maxY":y.2,
            "hasX":self.target.point.x.is_some(),"hasY":self.target.point.y.is_some(),"xLabel":x.3,"yLabel":y.3,
            "label":name,
            "disabled":disabled,"revision":self.target.revision()});
        let key=SharedString::from(format!("proto-graph-{:?}-{}-{:?}",editor.as_ref().map(Entity::entity_id),self.target.target.key.0,self.target.point.control));
        let parent=crate::proto_parent::current();let initial_parent=parent.clone();
        let initial=props.clone();let target=self.target.clone();let theme=self.theme;
        let mount=window.use_keyed_state(key,cx,move|window,cx|{
            let root=GraphPoint::create(initial.clone(),initial_parent,window,cx).unwrap_or_else(|e|panic!("compiled graph point setup failed: {e:?}"));
            let handle=root.read(cx).handle();let target=Rc::new(RefCell::new(target));let double=Rc::new(Cell::new(false));
            let contact=Rc::new(crate::proto_knob::Contact::default());
            let drawing=cx.new(|_|Drawing{handle:handle.clone(),target:target.clone(),double:double.clone(),theme});
            root.update(cx,|root,cx|root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())],cx)).expect("graph native drawing slot");
            let lease=Rc::new(attach_move_gesture(handle.owner(),Box::new(Binding{handle,target:target.clone(),double,contact:contact.clone(),window:window.window_handle(),app:cx.to_async()})).expect("graph physical input binding"));
            contact.window.set(Some(window.window_handle()));*contact.lease.borrow_mut()=Some(Rc::downgrade(&lease));
            let window=window.window_handle();
            let changes=cx.observe(&root,move|mount:&mut Mount,_,cx|{
                mount.drawing.update(cx,|_,cx|cx.notify());let handle=mount.root.read(cx).handle();let target=mount.target.borrow().clone();
                let lease=mount.lease.clone();let contact=mount.contact.clone();let expected=contact.generation.get();
                cx.defer(move|cx|{if !contact.is_current(expected){return;}let _=window.update(cx,|_,window,cx|{
                    let okay=drain(&handle,&target,window,cx);
                    if !okay||idle(&handle){lease.cancel(MoveGestureCancelReason::HostCancel,window,cx);clear_preview(&target,window,cx);}
                });});
            });
            Mount{root,drawing,target,contact,lease,props:initial,_changes:changes}
        });
        mount.update(cx,|mount,cx|{
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner,&parent){
                mount.root.update(cx,|root,cx|root.set_parent(parent,cx)).expect("compiled graph parent");
            }
            if mount.props!=props{
                let _=mount.root.update(cx,|root,cx|root.set_props(props.clone(),window,cx));
                let handle=mount.root.read(cx).handle();drain(&handle,&mount.target.borrow(),window,cx);
                if idle(&handle){mount.lease.cancel(MoveGestureCancelReason::HostCancel,window,cx);}
                mount.props=props;
            }
            *mount.target.borrow_mut()=self.target;mount.drawing.update(cx,|drawing,cx|{drawing.theme=self.theme;cx.notify();});
            mount.root.clone()
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn graph(editor:Entity<Editor>,target:Target,params:Vec<ParamDesc>,name:&str,label:&str,th:Theme,
    painter:Painter,handles:Vec<Handle>,menu:Option<HandleMenu>,double:Option<DoubleClick>)->gpui::Stateful<gpui::Div>{
    let name=name.to_string();let label=label.to_string();
    let draw_editor=editor.clone();let source_key=format!("device-graph-{}-{name}",target.key.0);
    div().id(SharedString::from(format!("{name}-{}",target.slot))).rounded(gpui::rems(10./16.)).bg(th.view).border_1().border_color(th.shade).overflow_hidden()
        .child(crate::proto_editor_surface::surface(editor,source_key,label,0.,move|input|Frame{editor:draw_editor.clone(),target:target.clone(),params:params.clone(),
            name:name.clone(),theme:th,painter:painter.clone(),handles:handles.clone(),menu:menu.clone(),double:double.clone(),input}.into_any_element(),|_,_,_|{}))
}
#[derive(IntoElement)]
struct Frame{editor:Entity<Editor>,target:Target,params:Vec<ParamDesc>,name:String,theme:Theme,painter:Painter,
    handles:Vec<Handle>,menu:Option<HandleMenu>,double:Option<DoubleClick>,input:crate::proto_editor_surface::Input}
impl RenderOnce for Frame{
    fn render(self,window:&mut Window,cx:&mut App)->impl IntoElement{
        let weak=self.editor.downgrade();let surface_key=SharedString::from(format!("device-graph-{:?}-{:?}-{}-{}-{}",self.editor.entity_id(),self.target.track,self.target.slot,self.target.key.0,self.name));
        let surface=crate::host_pointer::track_surface(surface_key,window,cx,move|surface,cx|{
            let _=weak.update(cx,|ed,cx|{if ed.device_drag.as_ref().is_some_and(|drag|drag.surface.same(&surface)){ed.cancel_device_drag(cx);}});
        });
        let plot=Rc::new(Cell::new(Plot{o:gpui::point(px(0.),px(0.)),w:1.,h:1.}));
        let points:Vec<_>=self.handles.iter().enumerate().map(|(index,handle)|{
            let target=PointTarget{editor:self.editor.downgrade(),target:self.target.clone(),scope:DeviceScope::for_window(window),point:handle.clone(),basis:None,
                plot:plot.clone(),surface:surface.clone(),menu:self.menu.clone(),index};
            div().absolute().left(gpui::relative(handle.at.0 as f32)).top(gpui::relative((1.-handle.at.1)as f32))
                .ml(px((GRAPH_PAD*(1.-2.*handle.at.0)-13.)as f32)).mt(px((GRAPH_PAD*(2.*handle.at.1-1.)-13.)as f32))
                .size(px(26.)).child(Point{target,theme:self.theme})
        }).collect();
        div().relative().size_full().child(paint_surface(self.editor,self.target,self.params,self.theme,self.painter,self.handles,self.double,self.input,plot,surface)).children(points)
    }
}
#[allow(clippy::too_many_arguments)]
fn paint_surface(editor:Entity<Editor>,target:Target,params:Vec<ParamDesc>,th:Theme,painter:Painter,handles:Vec<Handle>,double:Option<DoubleClick>,
    input:crate::proto_editor_surface::Input,plot_cell:Rc<Cell<Plot>>,surface:crate::host_pointer::SurfaceToken)->impl IntoElement{
    let key=target.key;
    let frame_editor=editor.clone();let frame_target=target.clone();
    canvas(move|bounds,window,cx|{
        let basis=frame_target.basis(frame_editor.read(cx));
        (window.insert_hitbox(bounds,HitboxBehavior::Normal),basis)
    },move|bounds,(hitbox,basis),window,cx|{
            let plot = Plot { o: bounds.origin, w: f64::from(bounds.size.width), h: f64::from(bounds.size.height) };
            plot_cell.set(plot);
            let mut texts = Texts::new();
            painter(window, plot, &params, &mut texts);
            let points: Rc<Vec<(f64, f64)>> = Rc::new(handles.iter().map(|h| (plot.x(h.at.0), plot.y(h.at.1))).collect());
            let mouse = window.mouse_position();
            let near_points = points.clone();
            let near = move |pos: gpui::Point<Pixels>| -> Option<usize> {
                let points = &near_points;
                let (x, y) = (f64::from(pos.x - plot.o.x), f64::from(pos.y - plot.o.y));
                points.iter().enumerate().map(|(i, &(hx, hy))| (i, (hx - x).hypot(hy - y))).filter(|(_, d)| *d <= 13.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
            };
            let hovered = bounds.contains(&mouse).then(|| near(mouse)).flatten();
            let dragging = editor.read(cx).device_drag.as_ref().filter(|d| d.key == key).map(|d| d.control.clone());
            for (i, (h, &(x, y))) in handles.iter().zip(points.iter()).enumerate() {
                let active = dragging.as_ref() == Some(&h.control);
                // State layers: hover grows the handle, a drag adds a halo.
                let r = if active { 9.0 } else if hovered == Some(i) || h.selected { 8.0 } else { 6.5 };
                if active || hovered == Some(i) {
                    let halo = r + 6.0;
                    window.paint_quad(quad(b(x - halo, y - halo, halo * 2.0, halo * 2.0, plot.o), px(halo as f32), with_alpha(h.color, 0.22), px(0.0), h.color, gpui::BorderStyle::Solid));
                }
                if h.selected {
                    window.paint_quad(quad(b(x - r - 2.0, y - r - 2.0, (r + 2.0) * 2.0, (r + 2.0) * 2.0, plot.o), px((r + 2.0) as f32), gpui::rgb(0xffffff), px(0.0), gpui::rgb(0xffffff), gpui::BorderStyle::Solid));
                }
                if h.hollow {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, plot.o), px(r as f32), th.view, px(2.0), with_alpha(h.color, 0.8), gpui::BorderStyle::Solid));
                } else {
                    window.paint_quad(quad(b(x - r, y - r, r * 2.0, r * 2.0, plot.o), px(r as f32), h.color, px(0.0), h.color, gpui::BorderStyle::Solid));
                }
                if !h.label.is_empty() {
                    let ink = if h.hollow { th.text_secondary } else { gpui::rgb(0x000000) };
                    texts.push((h.label.clone(), p(x - 3.2 * h.label.chars().count() as f64, y - 7.5, plot.o), 10.0, ink));
                }
            }
            let text_scale = f32::from(window.rem_size()) / 16.;
            for text in &mut texts { text.2 *= text_scale; }
            flush_texts(window, cx, bounds, texts);
            if hovered.is_some() || dragging.is_some() { window.set_cursor_style(gpui::CursorStyle::PointingHand, &hitbox); }


            let hb=hitbox.clone();let near_move=near.clone();
            window.on_mouse_event(move|ev:&MouseMoveEvent,phase,window,_|{
                if phase.bubble()&&hb.is_hovered(window).then(||near_move(ev.position)).flatten()!=hovered{window.refresh();}
            });
            let editor=editor.clone();let target=target.clone();let input=input.clone();let double=double.clone();
            window.on_mouse_event(move|event:&MouseDownEvent,phase,window,cx|{
                if !phase.bubble()||event.button!=MouseButton::Left||!hitbox.is_hovered(window)||near(event.position).is_some(){return;}
                let Some(double)=double.as_ref()else{return;};
                let Some(basis)=basis.as_ref()else{return;};
                // The blank graph and its captured domain action share the
                // frame's accepted parameters, including refreshes that do
                // not advance the project revision. Never adopt a new basis
                // while retaining this frame's double-click closure.
                let available={let ed=editor.read(cx);surface.is_live()&&DeviceScope::for_window(window).allows(&target.track,ed)
                    &&may_begin_device_input(ed)&&target.basis(ed).as_ref()==Some(basis)};
                if !available||!input.double_activate(event.click_count,window,cx){return;}
                let (x,y)=(f64::from(event.position.x-plot.o.x),f64::from(event.position.y-plot.o.y));
                let normalized=(((x-GRAPH_PAD)/plot.inner_w()).clamp(0.,1.),((plot.h-GRAPH_PAD-y)/plot.inner_h()).clamp(0.,1.));
                editor.update(cx,|ed,cx|{
                    // Source focus/observers may synchronously replace the
                    // target or retire the surface before domain evaluation.
                    if !surface.is_live()||!DeviceScope::for_window(window).allows(&target.track,ed)
                        ||!may_begin_device_input(ed)||target.basis(ed).as_ref()!=Some(basis){return;}
                    if let Some((writes,selected))=double(normalized){if let Some(index)=selected{select_band(ed,target.key,index);}write_if_current(ed,&target,writes,cx);}
                    else{ed.notice("所有频段都已在使用；右键频段可关闭它");cx.notify();}
                });cx.stop_propagation();
            });
    }).size_full()
}
