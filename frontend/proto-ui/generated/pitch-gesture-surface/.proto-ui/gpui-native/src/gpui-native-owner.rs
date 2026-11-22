use std::{cell::{Cell, RefCell}, collections::{BTreeMap, BTreeSet, VecDeque}, rc::{Rc, Weak}};
use serde_json::{Value as Json, Map};
use gpui::{prelude::*, AnyElement, App, Bounds, FocusHandle, Pixels, Window, div, canvas};
pub type OwnerRef = Rc<RefCell<Owner>>;
pub type OwnerWeak = Weak<RefCell<Owner>>;
pub type Callback = Rc<dyn Fn(Vec<Value>) -> Value>;
// Native frame factories capture an authored-render snapshot; they do not store
// semantic tags or re-run the authored render function during native repaint.
pub type NativeRenderer = Rc<dyn Fn(Vec<Value>) -> NativeSnapshot>;
// A native helper is an ordinary typed closure with its own runtime identity.
// Its identity is independent of both its FunctionIR declaration and child snapshots.
pub struct NativeCallable<F:?Sized> { pub identity:Rc<()>, pub call:Rc<F> }
impl<F:?Sized> Clone for NativeCallable<F> {
    fn clone(&self)->Self { Self { identity:self.identity.clone(), call:self.call.clone() } }
}
impl<F:?Sized> NativeCallable<F> {
    pub fn truthy(&self)->bool { true }
    pub fn nullish(&self)->bool { false }
    pub fn equals_native<G:?Sized>(&self,other:&NativeCallable<G>)->bool { Rc::ptr_eq(&self.identity,&other.identity) }
    pub fn equals_value(&self,other:&Value)->bool { matches!(other,Value::NativeFunction{identity,..} if Rc::ptr_eq(&self.identity,identity)) }
}
pub type NativeFrame = Rc<dyn Fn(&OwnerRef,&FocusHandle,&str,&Drive,&[NativeSnapshot],&mut usize)->Vec<AnyElement>>;
#[derive(Clone)]
pub struct NativeSnapshot { pub identity:Rc<()>, pub frame:NativeFrame, pub text:Rc<dyn Fn(&[NativeSnapshot])->String>, pub observation:Option<Value>, pub valid:bool, pub slot_count:u8 }
impl NativeSnapshot {
    // C-TEMPLATE-0005: count authored insertion points, not external native
    // children. Saturating at two is sufficient to reject duplicates without
    // overflow. This is validation metadata, never a semantic node/tag plan.
    pub fn assert_children(&self) {
        if !self.valid{value_error("invalid template children")}
        if self.slot_count>1{value_error("a render output permits at most one anonymous slot")}
    }

    // Scalar observations preserve authored operations before a result is used
    // as children. No native node is boxed into Value or a semantic tree.
    pub fn scalar_value(&self)->Value{self.observation.clone().expect("structural template is not scalar data")}
    pub fn scalar_number(&self)->f64{self.scalar_value().scalar_number()}
    pub fn scalar_text(&self)->String{self.scalar_value().scalar_text()}
    pub fn text(&self)->String{self.scalar_value().text()}
    pub fn member(&self,key:&str,optional:bool)->Value{self.scalar_value().member(key,optional)}

    pub fn truthy(&self)->bool{self.observation.as_ref().map(Value::truthy).unwrap_or(true)}
    pub fn nullish(&self)->bool{self.observation.as_ref().is_some_and(Value::nullish)}
    pub fn equals(&self,other:&Self)->bool{match(&self.observation,&other.observation){(Some(a),Some(b))=>strict_equal(a,b),(None,None)=>Rc::ptr_eq(&self.identity,&other.identity),_=>false}}
    pub fn equals_value(&self,other:&Value)->bool{self.observation.as_ref().is_some_and(|value|strict_equal(value,other))}
    pub fn invalid(value:Value)->Self{let mut result=Self::empty();result.observation=Some(value);result.valid=false;result}
    pub fn text_value(value:Value)->Self{assert!(matches!(&value,Value::Number(_)|Value::Json(Json::String(_)|Json::Number(_))),"native text children require string or number");let text=value.scalar_text();let frame_text=text.clone();Self{identity:Rc::new(()),observation:Some(value),valid:true,slot_count:0,frame:Rc::new(move|_,_,_,_,_,index|{*index+=1;vec![frame_text.clone().into_any_element()]}),text:Rc::new(move|_|text.clone())}}
    // These are ordinary native child factories, not tagged template/IR nodes.
    pub fn sequence(children:Vec<Self>,observation:Option<Value>)->Self{let valid=children.iter().all(|child|child.valid);let slot_count=children.iter().fold(0u8,|count,child|count.saturating_add(child.slot_count).min(2));let frame_children=children.clone();Self{identity:Rc::new(()),observation:Some(observation.unwrap_or_else(||Value::Opaque(Rc::new(())))),valid,slot_count,
        frame:Rc::new(move|owner,focus,path,drive,slot,index|{let mut result=Vec::new();for child in &frame_children{result.extend((child.frame)(owner,focus,path,drive,slot,index));}result}),
        text:Rc::new(move|slot|children.iter().map(|child|(child.text)(slot)).collect::<Vec<_>>().join("")),}}

    pub fn empty()->Self { Self { identity:Rc::new(()),frame:Rc::new(|_,_,_,_,_,_|Vec::new()), text:Rc::new(|_|String::new()), observation:Some(Value::Json(Json::Null)),valid:true,slot_count:0 } }
    pub fn slot()->Self { Self { identity:Rc::new(()),observation:None,valid:true,slot_count:1,
        frame:Rc::new(|owner,focus,path,drive,slot,index|{let mut result=Vec::new();for child in slot{result.extend((child.frame)(owner,focus,path,drive,&[],index));}result}),
        text:Rc::new(|slot|slot.iter().map(|child|(child.text)(&[])).collect::<Vec<_>>().join("")),
    } }
    pub fn native_view(view:gpui::AnyView)->Self { Self { identity:Rc::new(()),observation:None,valid:true,slot_count:0, frame:Rc::new(move|_,_,_,_,_,index|{*index+=1;vec![view.clone().into_any_element()]}),text:Rc::new(|_|String::new()) } }
}
// External slot children are already native factories. Validate the whole
// incoming collection before replacing the retained slot; no semantic walk.
pub fn assert_native_slot_children(children:&[NativeSnapshot]) {
    for child in children {
        child.assert_children();
        if child.slot_count!=0{value_error("external native children cannot contain the receiving owner's slot marker")}
    }
}
#[derive(Clone)]
pub enum Value {
    Undefined, Opaque(Rc<()>), ContextKey(NativeContextKey), Number(f64), Json(Json), Record(Rc<NativeRecord>), Array(Rc<Vec<Value>>), CaughtStringUnit(u16),
    Function(Callback), NativeFunction{identity:Rc<()>,render:NativeRenderer}, Renderer(NativeRenderer), Owner(OwnerWeak), State(Rc<State>), BorrowedState(Rc<State>),
    Style(Vec<(String, String)>), Resource(Rc<Cell<bool>>),
    Input(Rc<Input>),
    StaticCapability(&'static StaticDeclaration), Handle(OwnerWeak, &'static str),
    Observed(OwnerWeak, &'static str), Part(OwnerWeak, String), ObjectRef(OwnerWeak), HostTarget(OwnerWeak),
}
// Diagnostics must not recursively traverse arbitrary authored records or
// execute their callables. Public Result APIs can still use ordinary Rust
// expect/unwrap without requiring every capability to implement Debug.
impl std::fmt::Debug for Value {
    fn fmt(&self,output:&mut std::fmt::Formatter<'_>)->std::fmt::Result{match self{
        Self::Undefined=>output.write_str("undefined"),Self::Number(value)=>std::fmt::Debug::fmt(value,output),Self::Json(value)=>std::fmt::Debug::fmt(value,output),
        Self::Record(record)=>{let fields=record.fields.borrow();let scalars:Vec<_>=fields.iter().filter_map(|(name,value)|match value{Self::Undefined|Self::Number(_)|Self::Json(Json::Null|Json::Bool(_)|Json::Number(_)|Json::String(_))=>Some((name,value)),_=>None}).collect();output.debug_struct("AuthoredRecord").field("scalar_fields",&scalars).finish()},
        Self::Array(items)=>output.debug_struct("AuthoredArray").field("length",&items.len()).finish(),
        Self::Function(_)|Self::NativeFunction{..}|Self::Renderer(_)=>output.write_str("[authored callable]"),_=>output.write_str("[native capability]"),
    }}
}
#[derive(Debug)]
pub struct NativeFailure {pub phase:String,pub error:Value}
pub fn report_native_failure(owner:&OwnerRef,phase:&str,error:Value){
    eprintln!("[Proto-UI:{phase}] authored callback failed: {error:?}");
    owner.borrow_mut().failures.push(NativeFailure{phase:phase.into(),error});
}
// GPUI leases are not unwind-safe. Host callbacks without a Result return
// channel report exact authored errors while returning normally to GPUI.
pub fn native_host_callback<T>(owner:&OwnerRef,phase:&str,body:impl FnOnce()->T)->Option<T>{
    match catch_authored(body){Ok(value)=>Some(value),Err(error)=>{report_native_failure(owner,phase,error);None}}
}
pub fn take_native_failures(owner:&OwnerRef)->Vec<NativeFailure>{std::mem::take(&mut owner.borrow_mut().failures)}
impl Value {
    pub fn json(value: Json) -> Self { match value { Json::Object(entries)=>record_map(entries.into_iter().map(|(key,value)|(key,Self::json(value))).collect()),Json::Array(items)=>Self::Array(Rc::new(items.into_iter().map(Self::json).collect())),Json::Number(number)=>Self::Number(number.as_f64().expect("JSON number")),other=>Self::Json(other) } }
    pub fn string(value: &str) -> Self { Self::Json(Json::String(value.into())) }
    pub fn boolean(value: bool) -> Self { Self::Json(Json::Bool(value)) }
    pub fn number(value: f64) -> Self { Self::Number(value) }
    pub fn try_data(&self) -> Option<Json> { match self {
        Self::Number(value) => value.is_finite().then(||Json::from(*value)),
        Self::Json(value) => Some(value.clone()),
        Self::Record(entries) => entries.fields.borrow().iter().map(|(key,value)|Some((key.clone(),value.try_data()?))).collect::<Option<Map<String,Json>>>().map(Json::Object),
        Self::Array(items) => items.iter().map(Self::try_data).collect::<Option<Vec<_>>>().map(Json::Array),
        _ => None,
    } }
    pub fn data(&self) -> Json { match self {
        Self::Number(value)=>{if !value.is_finite(){value_type_error("non-finite portable data")}Json::from(*value)},Self::Json(value) => value.clone(),
        Self::Record(value) => Json::Object(value.fields.borrow().iter().map(|(k,v)|(k.clone(),v.data())).collect()),
        Self::Array(value) => Json::Array(value.iter().map(Self::data).collect()),
        _ => value_type_error("semantic capability is not portable data"),
    } }
    pub fn member(&self, key: &str, optional: bool) -> Self { match self {
        Self::Undefined | Self::Json(Json::Null) if optional => Self::Undefined,
        Self::Json(Json::Object(value)) => value.get(key).cloned().map(Self::json).unwrap_or(Self::Undefined),
        Self::Record(value) => value.fields.borrow().get(key).cloned().unwrap_or(Self::Undefined),
        Self::Array(value) => if key=="length"{Self::number(value.len()as f64)}else{key.parse::<usize>().ok().and_then(|index|value.get(index).cloned()).unwrap_or(Self::Undefined)},
        Self::Input(_) if key=="control" => self.clone(),
        Self::Input(value) => value.fields.get(key).cloned().map(Self::Json).unwrap_or(Self::Undefined),
        Self::ContextKey(_) if key=="__brand" => Self::string("ContextKey"),
        Self::Handle(owner, kind) => capability_member(owner, kind, key),
        Self::Part(_, role) if key == "role" => Self::string(role),
        _ => self.caught_member(key, optional),
    } }
    pub fn caught_member(&self, key:&str, optional:bool)->Self { match self {
        Self::Undefined | Self::Json(Json::Null) if optional => Self::Undefined,
        Self::Undefined | Self::Json(Json::Null) => throw_value(record(vec![("name",Self::string("TypeError")),("message",Self::string("Cannot read a property of a nullish value"))])),
        Self::Json(Json::String(value)) if key=="length" => Self::number(value.encode_utf16().count()as f64),
        Self::Json(Json::String(value)) => key.parse::<usize>().ok().filter(|index|index.to_string()==key).and_then(|index|value.encode_utf16().nth(index)).map(Self::CaughtStringUnit).unwrap_or(Self::Undefined),
        Self::CaughtStringUnit(_) if key=="length" => Self::number(1.),
        Self::CaughtStringUnit(value) if key=="0" => Self::CaughtStringUnit(*value),
        Self::Array(items) => if key=="length" { Self::number(items.len()as f64) } else { key.parse::<usize>().ok().filter(|index|index.to_string()==key).and_then(|index|items.get(index).cloned()).unwrap_or(Self::Undefined) },
        Self::Json(Json::Array(items)) => if key=="length" { Self::number(items.len()as f64) } else { key.parse::<usize>().ok().filter(|index|index.to_string()==key).and_then(|index|items.get(index).cloned()).map(Self::json).unwrap_or(Self::Undefined) },
        Self::Record(_) | Self::Json(Json::Object(_)) => self.member(key,optional),
        _ => Self::Undefined,
    } }
    pub fn text(&self) -> String { match self { Self::Json(Json::String(v))=>v.clone(), Self::Number(v)=>ecma_number_text(*v),Self::Json(Json::Number(v))=>ecma_number_text(v.as_f64().unwrap()), _=>value_type_error("expected string or number") } }
    pub fn bool(&self) -> bool { match self {Self::Json(Json::Bool(v))=>*v,_=>value_type_error("expected boolean")} }
    pub fn num(&self) -> f64 { match self {Self::Number(v)=>*v,Self::Json(Json::Number(v))=>v.as_f64().unwrap(),_=>value_type_error("expected number")} }
    pub fn truthy(&self)->bool{match self{Self::Undefined|Self::Json(Json::Null)=>false,Self::Json(Json::Bool(v))=>*v,Self::Number(v)=>*v!=0.&&!v.is_nan(),Self::Json(Json::Number(v))=>v.as_f64().is_some_and(|v|v!=0.&&!v.is_nan()),Self::Json(Json::String(v))=>!v.is_empty(),_=>true}}
    pub fn scalar_text(&self)->String{match self{Self::Undefined=>"undefined".into(),Self::Json(Json::Null)=>"null".into(),Self::Json(Json::Bool(v))=>v.to_string(),Self::Json(Json::String(v))=>v.clone(),Self::Number(v)=>ecma_number_text(*v),Self::Json(Json::Number(v))=>ecma_number_text(v.as_f64().unwrap()),_=>panic!("structured coercion is not admitted")}}
    pub fn scalar_number(&self)->f64{match self{Self::Number(v)=>*v,Self::Json(Json::Number(v))=>v.as_f64().unwrap(),Self::Json(Json::Null)=>0.,Self::Json(Json::Bool(v))=>if *v{1.}else{0.},Self::Json(Json::String(v))=>ecma_string_number(v),Self::Undefined=>f64::NAN,_=>panic!("structured coercion is not admitted")}}
    pub fn nullish(&self) -> bool { matches!(self,Self::Undefined|Self::Json(Json::Null)) }
    pub fn call(&self,args:Vec<Self>)->Result<Self,Self>{catch_authored(||self.call_authored(args))}
    // Compiler-emitted authored composition stays in one exception scope.
    // Native consumers use call(), which returns before any GPUI lease unwinds.
    pub fn call_authored(&self, args: Vec<Self>) -> Self { match self { Self::Function(f)=>f(args), _=>value_type_error("Value is not callable") } }
    pub fn get(&self) -> Self { match self { Self::State(state)|Self::BorrowedState(state)=>{ensure(&state.owner);state.value.borrow().clone()}, Self::Observed(owner, key)=>observed(owner,key), _=>value_type_error("expected State handle") } }
    pub fn owner(&self) -> OwnerWeak { match self { Self::Owner(owner)|Self::Handle(owner,_)|Self::HostTarget(owner)|Self::Part(owner,_)|Self::ObjectRef(owner)=>owner.clone(), _=>panic!("expected owner capability") } }
    pub fn items(&self) -> Vec<Self> { match self {Self::Array(v)=>(**v).clone(),Self::Json(Json::Array(v))=>v.iter().cloned().map(Self::json).collect(),_=>value_type_error("expected array")} }
}
pub fn strict_equal(a:&Value,b:&Value)->bool {
    match (a,b) {
        (Value::CaughtStringUnit(a),Value::CaughtStringUnit(b))=>return a==b,
        (Value::CaughtStringUnit(unit),Value::Json(Json::String(text)))|(Value::Json(Json::String(text)),Value::CaughtStringUnit(unit))=>return text.encode_utf16().eq(std::iter::once(*unit)),
        _=>{},
    }
    match (a,b){(Value::Undefined,Value::Undefined)=>true,(Value::Opaque(a),Value::Opaque(b))=>Rc::ptr_eq(a,b),(Value::ContextKey(a),Value::ContextKey(b))=>a==b,(Value::Number(a),Value::Number(b))=>a==b,(Value::Json(a),Value::Json(b))=>match(a,b){(Json::Array(_)|Json::Object(_),_)=>false,(Json::Number(a),Json::Number(b))=>a.as_f64()==b.as_f64(),_=>a==b},(Value::Record(a),Value::Record(b))=>Rc::ptr_eq(a,b),(Value::Array(a),Value::Array(b))=>Rc::ptr_eq(a,b),(Value::State(a),Value::State(b))|(Value::BorrowedState(a),Value::BorrowedState(b))=>Rc::ptr_eq(a,b),(Value::Function(a),Value::Function(b))=>Rc::ptr_eq(a,b),(Value::NativeFunction{identity:a,..},Value::NativeFunction{identity:b,..})=>Rc::ptr_eq(a,b),(Value::StaticCapability(a),Value::StaticCapability(b))=>a.identity()==b.identity(),(Value::Handle(a,ak),Value::Handle(b,bk))=>ak==bk&&Weak::ptr_eq(a,b),(Value::Observed(a,ak),Value::Observed(b,bk))=>ak==bk&&Weak::ptr_eq(a,b),(Value::ObjectRef(a),Value::ObjectRef(b))|(Value::HostTarget(a),Value::HostTarget(b))=>Weak::ptr_eq(a,b),(Value::Part(a,ar),Value::Part(b,br))=>ar==br&&Weak::ptr_eq(a,b),_=>false}}
fn same_value(a:&Value,b:&Value)->bool{
    if matches!(a,Value::Number(_)|Value::Json(Json::Number(_)))&&matches!(b,Value::Number(_)|Value::Json(Json::Number(_))){
        let a=a.num();let b=b.num();a.to_bits()==b.to_bits()||(a.is_nan()&&b.is_nan())
    }else{strict_equal(a,b)}
}
pub fn add(a:Value,b:Value)->Value {if matches!(&a,Value::Json(Json::String(_)))||matches!(&b,Value::Json(Json::String(_))){Value::string(&(a.scalar_text()+&b.scalar_text()))}else{Value::number(a.scalar_number()+b.scalar_number())}}
pub fn record(entries:Vec<(&str,Value)>)->Value{record_map(entries.into_iter().map(|(k,v)|(k.to_owned(),v)).collect())}
// The unwind payload is Send; authored values retain their original UI-thread
// identity in a private slot. Only this exact marker, on its creating thread,
// is an authored throw. Unrelated native panic payloads are never translated.
std::thread_local! { static AUTHORED_THROWS:RefCell<Vec<Option<Value>>> = const { RefCell::new(Vec::new()) }; }
struct AuthoredThrow { slot:usize, thread:std::thread::ThreadId, live:bool }
fn take_authored_throw(slot:usize)->Option<Value> {
    AUTHORED_THROWS.try_with(|values| {
        let mut values=values.borrow_mut();
        let value=values.get_mut(slot).and_then(Option::take);
        while values.last().is_some_and(Option::is_none) { values.pop(); }
        value
    }).ok().flatten()
}
impl Drop for AuthoredThrow {
    fn drop(&mut self) {
        if self.live&&self.thread==std::thread::current().id() { drop(take_authored_throw(self.slot)); }
    }
}
fn authored_payload(payload:&(dyn std::any::Any+Send))->bool {
    payload.downcast_ref::<AuthoredThrow>().is_some_and(|value|value.thread==std::thread::current().id())
}
pub fn throw_value(value:Value)->! {
    let slot=AUTHORED_THROWS.with(|values| {
        let mut values=values.borrow_mut();
        if let Some(slot)=values.iter().position(Option::is_none) { values[slot]=Some(value);slot }
        else { let slot=values.len();values.push(Some(value));slot }
    });
    // resume_unwind starts the authored unwind without changing or invoking
    // the process-wide native panic hook.
    std::panic::resume_unwind(Box::new(AuthoredThrow{slot,thread:std::thread::current().id(),live:true}))
}
pub fn throw_error(code:&str,message:&str)->! {
    throw_value(record(vec![("name",Value::string("Error")),("code",Value::string(code)),("message",Value::string(message))]))
}
pub fn catch_authored<T>(action:impl FnOnce()->T)->Result<T,Value> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(value)=>Ok(value),
        Err(payload) if authored_payload(payload.as_ref())=> {
            let mut marker=payload.downcast::<AuthoredThrow>().unwrap_or_else(|_|unreachable!());
            marker.live=false;
            Err(take_authored_throw(marker.slot).expect("authored exception slot missing"))
        },
        Err(payload)=>std::panic::resume_unwind(payload),
    }
}
fn authored_finally_outcome<B,F>(body:impl FnOnce()->B,finalize:impl FnOnce()->F)->(std::thread::Result<B>,F) {
    let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    if result.as_ref().err().is_some_and(|payload|!authored_payload(payload.as_ref())) {
        // Cleanup still runs on an unrelated native failure. Its completion
        // cannot turn that failure into success or replace its payload.
        if let Err(secondary)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(finalize)) {
            if let Err(destructor)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||drop(secondary))) {
                std::mem::forget(destructor);
            }
        }
        match result { Err(payload)=>std::panic::resume_unwind(payload),Ok(_)=>unreachable!() }
    }
    let finalized=finalize();
    (result,finalized)
}
pub fn authored_finally<T>(body:impl FnOnce()->std::ops::ControlFlow<T>,finalize:impl FnOnce()->std::ops::ControlFlow<T>)->std::ops::ControlFlow<T> {
    let (result,finalized)=authored_finally_outcome(body,finalize);
    match finalized {
        std::ops::ControlFlow::Break(value)=>std::ops::ControlFlow::Break(value),
        std::ops::ControlFlow::Continue(())=>match result { Ok(value)=>value,Err(payload)=>std::panic::resume_unwind(payload) },
    }
}
pub fn authored_finally_override<B,T>(body:impl FnOnce()->B,finalize:impl FnOnce()->T)->T {
    let (_,finalized)=authored_finally_outcome(body,finalize);
    finalized
}
pub fn arg(args:&[Value],index:usize)->Value{args.get(index).cloned().unwrap_or(Value::Undefined)}
pub fn ensure(owner:&OwnerWeak)->OwnerRef{let owner=owner.upgrade().unwrap_or_else(||throw_error("OWNER_RELEASED","owner released"));if owner.borrow().disposed{throw_error("OWNER_DISPOSED","terminally disposed owner")}owner}
pub fn setup(owner:&OwnerWeak)->OwnerRef{let owner=ensure(owner);if owner.borrow().phase!="setup"{value_error("setup-only capability")}owner}
pub fn runtime(owner:&OwnerWeak)->OwnerRef{let owner=ensure(owner);if matches!(owner.borrow().phase,"setup"|"render"|"before-dispose"){value_error("callback capability required")}owner}
pub fn capability(owner:&OwnerWeak)->Value{Value::Owner(owner.clone())}
pub struct State { pub value:RefCell<Value>, pub kind:&'static str, pub spec:Value, pub owner:OwnerWeak }
#[derive(Clone)]
struct Watcher { active:Rc<Cell<bool>>, keys:Option<Vec<String>>, raw:bool, callback:Value }
#[derive(Clone)]
struct Listener { active:Rc<Cell<bool>>, kind:String, global:bool, capture:bool, once:bool, consumed_epoch:Rc<Cell<Option<u64>>>, passive:bool, numeric_value:bool, callback:Value }
#[derive(Clone)]
struct Contribution {active:Rc<Cell<bool>>, predicate:Option<Value>, styles:Vec<(String,String)>}
#[derive(Clone)]
struct ContextSubscription {active:Rc<Cell<bool>>, key:NativeContextKey, optional:bool, callback:Value}
#[derive(Clone)]
pub struct Input {pub fields:Map<String,Json>,pub prevented:Cell<bool>,pub passive:bool,pub live:Cell<bool>}
#[derive(Default)]
struct NativePress{key:Option<String>,global_key:Option<String>,pointer:bool,global_pointer:bool,cancelled:bool,global_keyboard_click:bool,global_pointer_cancelled:bool}
struct ContextNotice {target:OwnerWeak,registration:ContextSubscription,next:Value,previous:Value}
#[derive(Default)]
struct ContextChannel {pending:VecDeque<ContextNotice>}
thread_local! {static OWNERS:RefCell<Vec<OwnerWeak>>=RefCell::new(Vec::new());static CONTEXT_CHANNELS:RefCell<BTreeMap<(usize,NativeContextKey),Rc<RefCell<ContextChannel>>>>=RefCell::new(BTreeMap::new());}
pub struct Owner {
    atoms:NativeHostAtoms,
    lexical_slots:Vec<Weak<dyn LexicalLifetime>>,
    authored:AuthoredHooks,
    public_values:PublicValues,
    failures:Vec<NativeFailure>,
    pub phase:&'static str,pub disposed:bool,pub disposing:bool,pub present:bool,pub mounted:bool,pub epoch:u64,
    pub dirty:bool,pub native_drive:Option<Drive>,pub window_id:Option<gpui::WindowId>,pub update_requested:bool,pub projection_revision:u64,pub committed_revision:u64,
    pub projection:Option<NativeSnapshot>,pub renderer:Value,pub raw:Json,pub raw_value:Value,pub props_value:Value,pub props:Json,pub schemas:BTreeMap<String,Value>,pub defaults:BTreeMap<String,Value>,
    pub states:BTreeMap<String,Rc<State>>,pub exposes:BTreeMap<String,Value>,pub declared_events:BTreeSet<String>,pub emitted:Vec<(String,Json)>,
    pub lifecycle:BTreeMap<String,Vec<Value>>,watchers:Vec<Watcher>,listeners:Vec<Listener>,contributions:Vec<Contribution>,patch:Vec<(String,String)>,suppressed:BTreeSet<String>,
    pub parent:OwnerWeak,pub providers:BTreeMap<NativeContextKey,Value>,subscriptions:Vec<ContextSubscription>,
    pub context_accepts:fn(&str,&Json)->bool,pub props_accepts:fn(&Json)->bool,pub prop_valid:fn(&str,&Json)->bool,pub previous_valid:BTreeMap<String,Json>,pub default_layers:Vec<BTreeMap<String,Value>>,
    pub focusable:bool,pub focus_disabled:bool,pub focus_requested:bool,pub focus_entry:bool,pub focused:bool,pub suppress_click:bool,
    pub accessible:bool,pub accessible_role:Value,pub accessible_states:BTreeMap<String,Value>,pub accessible_state_conditions:BTreeMap<String,Value>,pub accessible_actions:BTreeSet<String>,pub name_content:bool,
    pub slot:Vec<NativeSnapshot>,pub bounds:Option<Bounds<Pixels>>,pub native_hitbox:Option<gpui::Hitbox>,pub native_translation:gpui::Point<Pixels>,pub theme:String,pub dark:bool,
    pub style_vocabulary:&'static crate::gpui_native_style::StyleVocabulary,
    pub modules:Modules,press:NativePress,press_global:bool,window_input_observed:Option<gpui::WeakFocusHandle>,
}
#[cfg(feature="test-support")]
pub fn native_owner_allocation_count()->usize{OWNERS.with(|owners|owners.borrow().iter().filter(|owner|owner.strong_count()>0).count())}
#[cfg(feature="test-support")]
pub fn native_resource_counts(owner:&OwnerRef)->Json{
    let o=owner.borrow();serde_json::json!({
        "states":o.states.len(),"hooks":o.modules.hooks.len(),"listeners":o.listeners.len(),
        "stateWatchers":o.modules.state_watchers.len(),"propsWatchers":o.watchers.len(),
        "contextSubscriptions":o.subscriptions.len(),"anatomySubscriptions":o.modules.topology.subscriptions.len(),
        "collectionStates":o.modules.topology.handles.len(),"scrollHandles":usize::from(o.modules.scroll.handle.is_some()),
        "boundaryRegions":o.modules.boundary.regions.len(),"boundaryCallbacks":o.modules.boundary.callbacks.len(),
        "metadataKeys":o.atoms.meta.len(),"schedulerBindings":usize::from(o.atoms.app.is_some()),
        "delays":o.atoms.delays.len(),"presenceOwners":o.atoms.presence.len(),
        "pointerInput":native_pointer_input_needed(owner),"keyInput":native_key_input_needed(owner),
        "boundaryInput":native_boundary_input_needed(owner),"focusObservation":native_focus_observation_needed(owner)
    })
}
impl Owner {
    pub fn new(raw:Json,parent:OwnerWeak,context_accepts:fn(&str,&Json)->bool,props_accepts:fn(&Json)->bool,prop_valid:fn(&str,&Json)->bool,style_vocabulary:&'static crate::gpui_native_style::StyleVocabulary)->OwnerRef {
        if !raw.is_object(){value_type_error("raw props must be a record")}
        let owner=Rc::new(RefCell::new(Self{atoms:NativeHostAtoms::default(),lexical_slots:Vec::new(),authored:AuthoredHooks::default(),public_values:PublicValues::default(),failures:Vec::new(),phase:"setup",disposed:false,disposing:false,present:true,mounted:false,epoch:0,dirty:true,native_drive:None,window_id:None,update_requested:false,projection_revision:0,committed_revision:0,projection:None,renderer:Value::Undefined,raw_value:Value::json(raw.clone()),props_value:Value::Undefined,raw,props:Json::Object(Map::new()),schemas:BTreeMap::new(),defaults:BTreeMap::new(),states:BTreeMap::new(),exposes:BTreeMap::new(),declared_events:BTreeSet::new(),emitted:Vec::new(),lifecycle:BTreeMap::new(),watchers:Vec::new(),listeners:Vec::new(),contributions:Vec::new(),patch:Vec::new(),suppressed:BTreeSet::new(),parent,providers:BTreeMap::new(),subscriptions:Vec::new(),context_accepts,props_accepts,prop_valid,previous_valid:BTreeMap::new(),default_layers:Vec::new(),focusable:false,focus_disabled:false,focus_requested:false,focus_entry:true,focused:false,suppress_click:false,accessible:false,accessible_role:Value::Undefined,accessible_states:BTreeMap::new(),accessible_state_conditions:BTreeMap::new(),accessible_actions:BTreeSet::new(),name_content:false,slot:Vec::new(),bounds:None,native_hitbox:None,native_translation:gpui::Point::default(),theme:"shadcn".into(),dark:false,style_vocabulary,modules:Modules::default(),press:NativePress::default(),press_global:false,window_input_observed:None}));
        OWNERS.with(|all|all.borrow_mut().push(Rc::downgrade(&owner)));owner
    }
    pub fn finish(owner:&OwnerRef,renderer:Value){finish_public_exposes(owner);owner.borrow_mut().renderer=renderer;owner.borrow_mut().phase="idle";resolve_props(owner);accessible_alive(owner);
        if let Err(failure)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||life(owner,"created"))){
            attempt_failed_created_cleanup(||Self::failed_created_cleanup(owner));
            std::panic::resume_unwind(failure);
        }
    }
    // C-LIFECYCLE-0002-J: only acquired synchronous created failure owns this recovery.
    fn failed_created_cleanup(owner:&OwnerRef){
        if owner.borrow().disposed{return}
        attempt_failed_created_cleanup(||begin_native_detach(owner));
        for release in NATIVE_VIEW_RELEASES{attempt_failed_created_cleanup(||release(owner));}
        let callbacks=owner.borrow().lifecycle.get("before-dispose").cloned().unwrap_or_default();
        for callback in callbacks{attempt_failed_created_cleanup(||invoke(owner,"before-dispose",&callback,vec![capability(&Rc::downgrade(owner))]));}
        attempt_failed_created_cleanup(||clear_lexicals(owner));
        attempt_failed_created_cleanup(||Self::failed_created_retire(owner));
        attempt_failed_created_cleanup(topology_refresh);
        attempt_failed_created_cleanup(poll_state_watchers);
    }
    pub fn project(owner:&OwnerRef){
        let needed={let o=owner.borrow();o.present&&!o.disposed&&(o.projection.is_none()||o.update_requested)};
        if !needed{return}
        let previous=owner.borrow().phase;
        let renderer={let mut o=owner.borrow_mut();o.update_requested=false;o.phase="render";o.renderer.clone()};
        // Retire the render phase even when the authored renderer or validation panics.
        let guard=PhaseGuard{owner:Rc::downgrade(owner),previous};
        // C-CORE-SYNTAX-0004-C: void setup projects one anonymous slot.
        let _lexical_owner=enter_lexical_owner(&Rc::downgrade(owner));
        let result=match renderer{Value::Undefined=>NativeSnapshot::slot(),Value::Renderer(render)=>render(vec![capability(&Rc::downgrade(owner))]),_=>panic!("expected compiled native renderer")};
        result.assert_children();
        {let mut o=owner.borrow_mut();o.projection=Some(result);o.projection_revision+=1;}
        drop(guard);
    }
    pub fn commit(owner:&OwnerRef,revision:u64){
        let(event,epoch)={let mut o=owner.borrow_mut();if o.disposed||!o.present||o.bounds.is_none()||o.projection_revision!=revision||o.committed_revision==revision{return}o.committed_revision=revision;let event=if !o.mounted{o.mounted=true;o.epoch+=1;let epoch=o.epoch;if let Some(projection)=o.modules.focus.entry_projection.as_mut(){projection.binding.epoch=epoch;}"mounted"}else{"updated"};(event,o.epoch)};
        modules_commit(owner);
        {let o=owner.borrow();if o.disposed||!o.present||!o.mounted||o.epoch!=epoch||o.projection_revision!=revision||o.committed_revision!=revision{return}}
        life(owner,event);
    }
    pub fn dispose(owner:&OwnerRef){
        {let mut owner=owner.borrow_mut();if owner.disposed||owner.disposing{return}owner.disposing=true;}
        // The first authored failure stops its callback list, but cannot skip
        // terminal private cleanup. Keep the original payload, including 0/false.
        let mut failure=None;
        attempt_native_cleanup(&mut failure,||crate::gpui_native_move::move_dispose(owner));
        attempt_native_cleanup(&mut failure,||detach_native_view(owner));
        attempt_native_cleanup(&mut failure,||life(owner,"before-dispose"));
        attempt_native_cleanup(&mut failure,||clear_lexicals(owner));
        attempt_native_cleanup(&mut failure,||Self::failed_created_retire(owner));
        attempt_native_cleanup(&mut failure,topology_refresh);
        attempt_native_cleanup(&mut failure,poll_state_watchers);
        if let Some(failure)=failure{std::panic::resume_unwind(failure);}
    }
    fn retire(owner:&OwnerRef){
        retire_entry_intent(owner);clear_scope_recipient(owner);
        let mut o=owner.borrow_mut();o.disposed=true;o.present=false;o.mounted=false;o.bounds=None;o.native_hitbox=None;o.projection=None;o.renderer=Value::Undefined;o.watchers.clear();o.listeners.clear();o.contributions.clear();o.subscriptions.clear();o.providers.clear();o.exposes.clear();o.states.clear();o.lifecycle.clear();o.slot.clear();o.authored=AuthoredHooks::default();o.public_values=PublicValues::default();o.atoms=NativeHostAtoms::default();o.modules=Modules::default();
    }
    // Failed-created retirement extracts owned values inside one short borrow so
    // one destructor panic cannot abort the remaining terminal release attempts;
    // each drop runs under its own guard after the borrow ends. Reachable
    // acquired resources (including module-held tasks/leases inside Modules)
    // therefore retire even when a callback capture's destructor panics.
    fn failed_created_retire(owner:&OwnerRef){
        retire_entry_intent(owner);clear_scope_recipient(owner);
            let extracted={let mut o=owner.borrow_mut();o.disposed=true;o.present=false;o.mounted=false;o.bounds=None;o.native_hitbox=None;
            (std::mem::take(&mut o.projection),std::mem::take(&mut o.renderer),std::mem::take(&mut o.watchers),std::mem::take(&mut o.listeners),std::mem::take(&mut o.contributions),std::mem::take(&mut o.subscriptions),std::mem::take(&mut o.providers),std::mem::take(&mut o.exposes),std::mem::take(&mut o.states),std::mem::take(&mut o.lifecycle),std::mem::take(&mut o.slot),std::mem::take(&mut o.modules))};
        let(projection,renderer,watchers,listeners,contributions,subscriptions,providers,exposes,states,lifecycle,slot,modules)=extracted;
        attempt_failed_created_cleanup(||drop(projection));
        attempt_failed_created_cleanup(||drop(renderer));
        attempt_failed_created_cleanup(||drop(watchers));
        attempt_failed_created_cleanup(||drop(listeners));
        attempt_failed_created_cleanup(||drop(contributions));
        attempt_failed_created_cleanup(||drop(subscriptions));
        attempt_failed_created_cleanup(||drop(providers));
        attempt_failed_created_cleanup(||drop(exposes));
        attempt_failed_created_cleanup(||drop(states));
        attempt_failed_created_cleanup(||drop(lifecycle));
        attempt_failed_created_cleanup(||drop(slot));
        attempt_failed_created_cleanup(||drop(modules));
        let (authored,atoms,public_values)={let mut owner=owner.borrow_mut();(std::mem::take(&mut owner.authored),std::mem::take(&mut owner.atoms),std::mem::take(&mut owner.public_values))};
        attempt_failed_created_cleanup(||drop(authored));attempt_failed_created_cleanup(||drop(atoms));attempt_failed_created_cleanup(||drop(public_values));
    }
}
impl Drop for Owner { fn drop(&mut self){for slot in self.lexical_slots.drain(..){if let Some(slot)=slot.upgrade(){slot.clear();}}} }
fn attempt_failed_created_cleanup<T>(operation:impl FnOnce()->T){
    if let Err(failure)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||drop(operation()))){
        if let Err(drop_failure)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||drop(failure))){
            // Quarantine the destructor's failure: dropping it could panic again and skip retirement.
            std::mem::forget(drop_failure);
        }
    }
}
fn attempt_native_cleanup<T>(failure:&mut Option<Box<dyn std::any::Any+Send>>,operation:impl FnOnce()->T){
    if let Err(next)=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||drop(operation()))){
        if failure.is_none(){*failure=Some(next);}else{attempt_failed_created_cleanup(||drop(next));}
    }
}
const NATIVE_VIEW_RELEASES:[fn(&OwnerRef);7]=[text_detach,image_detach,positioning_detach,overlay_detach,scroll_detach,atoms_detach,crate::gpui_native_move::move_detach];
fn begin_native_detach(owner:&OwnerRef)->bool{
    withdraw_entry_view(owner);
    let mut o=owner.borrow_mut();let mounted=o.mounted;o.epoch=o.epoch.wrapping_add(1);o.mounted=false;o.bounds=None;o.native_hitbox=None;o.focused=false;o.modules.focus.visible=false;o.modules.focus.native=None;o.modules.a11y_node=None;o.modules.a11y_frame=0;o.modules.a11y_bindings=0;o.committed_revision=0;o.press=NativePress::default();mounted
}
pub fn bind_native_focus_window(owner:&OwnerRef,window:&Window){let id=window.window_handle().window_id();if owner.borrow().window_id==Some(id){return}crate::gpui_native_move::move_blur(owner);withdraw_entry_view(owner);let mut o=owner.borrow_mut();o.epoch=o.epoch.wrapping_add(1);o.modules.focus.native=None;o.window_id=Some(id);}
pub fn detach_native_view(owner:&OwnerRef){
    let mounted=begin_native_detach(owner);
    let mut failure=None;
    for release in NATIVE_VIEW_RELEASES{attempt_native_cleanup(&mut failure,||release(owner));}
    if mounted{attempt_native_cleanup(&mut failure,topology_refresh);attempt_native_cleanup(&mut failure,||life(owner,"unmounted"));}
    // An unmounted callback may create another task. Cancellation after that
    // callback pass is unconditional, including an authored thrown value.
    attempt_native_cleanup(&mut failure,||atoms_detach(owner));
    if mounted{attempt_native_cleanup(&mut failure,poll_state_watchers);}
    if let Some(failure)=failure{std::panic::resume_unwind(failure);}
}
struct PhaseGuard{owner:OwnerWeak,previous:&'static str}
impl Drop for PhaseGuard{fn drop(&mut self){if let Some(owner)=self.owner.upgrade(){owner.borrow_mut().phase=self.previous;}}}
fn invoke_scoped<T>(owner:&OwnerRef,phase:&'static str,body:impl FnOnce()->T)->T{if owner.borrow().disposed{throw_error("OWNER_DISPOSED","owner disposed")}let previous=owner.borrow().phase;owner.borrow_mut().phase=phase;let guard=PhaseGuard{owner:Rc::downgrade(owner),previous};let _lexical_owner=enter_lexical_owner(&Rc::downgrade(owner));let result=body();drop(guard);if phase!="before-dispose"{poll_state_watchers();}result}
pub fn invoke(owner:&OwnerRef,phase:&'static str,callback:&Value,args:Vec<Value>)->Value{invoke_scoped(owner,phase,||callback.call_authored(args))}
pub fn life(owner:&OwnerRef,kind:&'static str){let callbacks=owner.borrow().lifecycle.get(kind).cloned().unwrap_or_default();for callback in callbacks{invoke(owner,kind,&callback,vec![capability(&Rc::downgrade(owner))]);}}
pub fn register_life(owner:&OwnerWeak,kind:&str,callback:Value)->Value{setup(owner).borrow_mut().lifecycle.entry(kind.into()).or_default().push(callback);Value::Undefined}
pub fn request_update(owner:&OwnerWeak)->Value{let owner=runtime(owner);let mut o=owner.borrow_mut();o.update_requested=true;o.dirty=true;Value::Undefined}
pub fn set_present(owner:&OwnerWeak,present:Value)->Value{let owner=runtime(owner);let present=present.bool();{let mut o=owner.borrow_mut();if o.present==present{return Value::Undefined}o.present=present;o.dirty=true;if present{o.projection=None;o.update_requested=true;}}Value::Undefined}
pub fn props_define(owner:&OwnerWeak,schema:Value)->Value{let owner=setup(owner);match schema{Value::Record(entries)=>{owner.borrow_mut().schemas.extend(entries.fields.borrow().iter().map(|(k,v)|(k.clone(),v.clone())));},_=>panic!("checked Props schema record")};resolve_props(&owner);Value::Undefined}
pub fn set_defaults(owner:&OwnerWeak,defaults:Value)->Value{let owner=setup(owner);match defaults{Value::Record(entries)=>owner.borrow_mut().default_layers.push(entries.fields.borrow().clone()),_=>panic!("checked defaults record")};resolve_props(&owner);Value::Undefined}
fn resolve_props(owner:&OwnerRef){
let(raw,schemas,layers,previous_valid,valid,strict)={let o=owner.borrow();(o.raw.clone(),o.schemas.clone(),o.default_layers.clone(),o.previous_valid.clone(),o.prop_valid,o.phase!="setup")};
let mut resolved=Map::new();let mut previous_valid=previous_valid;
for(key,schema)in schemas{
let provided=raw.get(&key);let empty_policy=schema.member("empty",false);let error=strict&&matches!(&empty_policy,Value::Json(Json::String(s))if s=="error");
let range=schema.member("range",false);
let accepts=|candidate:&Json|{if !(valid)(&key,candidate){return false}if !range.nullish(){let min=range.member("min",false);let max=range.member("max",false);let Some(number)=candidate.as_f64()else{return false};if !min.nullish()&&number<min.num(){return false}if !max.nullish()&&number>max.num(){return false}}true};
let mut value=None;
if let Some(raw)=provided{if !raw.is_null()&&accepts(raw){value=Some(raw.clone());previous_valid.insert(key.clone(),raw.clone());}else if raw.is_null()&&matches!(&empty_policy,Value::Json(Json::String(s))if s=="accept"){value=Some(Json::Null);}else if let Some(previous)=previous_valid.get(&key){if !previous.is_null()&&accepts(previous){value=Some(previous.clone());}}}
if value.is_none(){for layer in &layers{if let Some(candidate)=layer.get(&key){let candidate=candidate.data();if candidate.is_null()&&!error||!candidate.is_null()&&accepts(&candidate){value=Some(candidate);break}}}}
if value.is_none(){let default=schema.member("default",false);if !matches!(default,Value::Undefined){let default=default.data();if default.is_null()&&!error||!default.is_null()&&accepts(&default){value=Some(default);}}}
let value=value.unwrap_or_else(||{if error{value_error(&format!("missing, empty or invalid prop without nonempty fallback: {key}"))}Json::Null});if provided.is_some()&&!value.is_null(){previous_valid.insert(key.clone(),value.clone());}resolved.insert(key,value);
}
let props=Json::Object(resolved);let mut o=owner.borrow_mut();if !(o.props_accepts)(&props){value_type_error("invalid resolved Props value")}o.previous_valid=previous_valid;o.props_value=Value::json(props.clone());o.props=props;
}
pub fn read_props(owner:&OwnerWeak,raw:bool)->Value{let owner=ensure(owner);let o=owner.borrow();if raw{o.raw_value.clone()}else{o.props_value.clone()}}
pub fn is_provided(owner:&OwnerWeak,key:Value)->Value{Value::boolean(ensure(owner).borrow().raw.get(key.text()).is_some())}
pub fn watch_props(owner:&OwnerWeak,keys:Option<Value>,raw:bool,callback:Value)->Value{let owner=setup(owner);let active=Rc::new(Cell::new(true));let keys=keys.map(|v|match v{Value::Json(Json::String(s))=>vec![s],_=>v.items().iter().map(Value::text).collect()});owner.borrow_mut().watchers.push(Watcher{active:active.clone(),keys,raw,callback});disposer(owner,active,false)}
pub fn replace_props(owner:&OwnerRef,raw:Json){if !raw.is_object(){value_type_error("raw props must be a record")}let (old_raw,old_props,watchers)={let o=owner.borrow();if o.disposed{throw_error("OWNER_DISPOSED","terminally disposed owner")}(o.raw.clone(),o.props.clone(),o.watchers.clone())};{let mut o=owner.borrow_mut();o.raw_value=Value::json(raw.clone());o.raw=raw;o.dirty=true;}resolve_props(owner);let (new_raw,new_props)={let o=owner.borrow();(o.raw.clone(),o.props.clone())};let mut pending=Vec::new();for watcher in watchers{let(previous,next)=if watcher.raw{(&old_raw,&new_raw)}else{(&old_props,&new_props)};let all:BTreeSet<String>=previous.as_object().unwrap().keys().chain(next.as_object().unwrap().keys()).cloned().collect();let changed:Vec<String>=all.into_iter().filter(|key|previous.get(key)!=next.get(key)).collect();if changed.is_empty()||watcher.keys.as_ref().is_some_and(|keys|!changed.iter().any(|key|keys.contains(key))){continue}let info=record(vec![("changedKeys",Value::Array(Rc::new(changed.iter().map(|k|Value::string(k)).collect())))]);pending.push((watcher,next.clone(),previous.clone(),info));}for(watcher,next,previous,info)in pending{if watcher.active.get(){invoke(owner,"props-watch",&watcher.callback,vec![capability(&Rc::downgrade(owner)),Value::json(next),Value::json(previous),info]);}}}
// Scalar domain validation is a compiler-profile policy. Its failures are
// authored errors, never Rust invariants: source try/catch and native Result
// boundaries must be able to recover without unwinding a GPUI entity lease.
pub fn create_state(owner:&OwnerWeak,kind:&'static str,key:Value,value:Value,spec:Value)->Value{
    let owner_ref=setup(owner);
    let Value::Json(Json::String(key))=key else{value_error("[State] state name must be a non-empty string.")};
    let value=normalize_state(kind,value,&spec,true);
    // Failed validation must not reserve the name in this capture frame.
    let storage_key=claim_state_name(&owner_ref,&key);
    let state=Rc::new(State{value:RefCell::new(value),kind,spec,owner:owner.clone()});
    owner_ref.borrow_mut().states.insert(storage_key,state.clone());capture_state(&owner_ref,key,state.clone());Value::State(state)
}
fn normalize_state(kind:&str,value:Value,spec:&Value,initial:bool)->Value{
    match kind{
        "bool"=>{value.bool();},
        "string"|"enum"=>{
            if !matches!(value,Value::Json(Json::String(_))){value_error("State string required")}
            let options=spec.member("options",true);
            if options.nullish()&&kind=="enum"{value_error("enum options required")}
            if !options.nullish()&&!options.items().iter().any(|option|strict_equal(option,&value)){value_error("State value outside options")}
        },
        "discrete"|"range"=>{
            let number=value.num();if !number.is_finite(){value_error("State finite number required")}
            let min=spec.member("min",true);let max=spec.member("max",true);
            if kind=="range"{
                if min.nullish()||max.nullish()||!(min.num()<=max.num()){value_error("invalid range")}
                let clamp=spec.member("clamp",true);
                if initial&&!clamp.nullish()&&clamp.bool(){return Value::number(number.max(min.num()).min(max.num()))}
                if !(number>=min.num()&&number<=max.num()){value_error("State range violation")}
            }else{
                if !min.nullish()&&!(number>=min.num()){value_error("State below min")}
                if !max.nullish()&&!(number<=max.num()){value_error("State above max")}
                let step=spec.member("step",true);
                if !step.nullish(){
                    if !(step.num()>0.){value_error("State step must be positive")}
                    let origin=if min.nullish(){0.}else{min.num()};
                    if !(((number-origin)/step.num()).fract().abs()<1e-9){value_error("State step violation")}
                }
            }
        },
        _=>panic!("unknown State constructor"),
    }
    let options=spec.member("options",true);
    if !options.nullish()&&!options.items().iter().any(|item|strict_equal(item,&value)){value_error("State option violation")}
    value
}
pub fn state_set(receiver:Value,value:Value,reason:Value)->Value{match receiver{Value::State(state)=>{runtime(&state.owner);write_state_value(&state,value,reason);},_=>panic!("State handle required")};Value::Undefined}
fn expose_key(owner:&OwnerRef,key:Value)->String{
    let key=key.text();if key.is_empty(){throw_error("EXPOSE_KEY_INVALID","Expose key must be nonempty")}
    let owner=owner.borrow();if owner.exposes.contains_key(&key)||owner.declared_events.contains(&key){throw_error("EXPOSE_DUPLICATE",&format!("duplicate Expose key: {key}"))}key
}
pub fn expose_state(owner:&OwnerWeak,key:Value,state:Value)->Value{
    let owner_ref=setup(owner);let key=expose_key(&owner_ref,key);
    let mut owner=owner_ref.borrow_mut();
    if let Value::Observed(_,observation)=&state {
        let name=match *observation {
            "focus.focused"=>Some("focused"),
            "focus.visible"=>Some("focusVisible"),
            "focus.ready"=>Some("focusable"),
            _=>None,
        };
        if let (Some(name),Some(frame))=(name,owner.authored.frames.last_mut()) {
            frame.states.insert(name.into(),state.clone());
        }
    }
    owner.exposes.insert(key,state);Value::Undefined
}
pub fn expose_value(owner:&OwnerWeak,key:Value,value:Value)->Value{let owner_ref=setup(owner);let key=expose_key(&owner_ref,key);owner_ref.borrow_mut().exposes.insert(key,value);Value::Undefined}
pub fn expose_method(owner:&OwnerWeak,key:Value,callback:Value)->Value{let owner_ref=setup(owner);let key=expose_key(&owner_ref,key);capture_method(&owner_ref,key.clone(),callback.clone());owner_ref.borrow_mut().exposes.insert(key,callback);Value::Undefined}
pub fn expose_event(owner:&OwnerWeak,key:Value)->Value{let owner_ref=setup(owner);let key=expose_key(&owner_ref,key);owner_ref.borrow_mut().declared_events.insert(key);Value::Undefined}
pub fn emit(owner:&OwnerWeak,key:Value,payload:Value)->Value{let owner=runtime(owner);let key=key.text();assert!(owner.borrow().declared_events.contains(&key),"undeclared Expose event");let payload=if matches!(payload,Value::Undefined){Json::Null}else{payload.data()};owner.borrow_mut().emitted.push((key,payload));Value::Undefined}
pub fn disposer(owner:OwnerRef,active:Rc<Cell<bool>>,setup_only:bool)->Value{let owner=Rc::downgrade(&owner);Value::Function(Rc::new(move |_|{if setup_only{setup(&owner);}else{ensure(&owner);}active.set(false);Value::Undefined}))}
pub fn style_use(owner:&OwnerWeak,handles:Vec<Value>)->Value{let owner=setup(owner);let active=Rc::new(Cell::new(true));owner.borrow_mut().contributions.push(Contribution{active:active.clone(),predicate:None,styles:style_items(handles)});disposer(owner,active,true)}
pub fn rule(owner:&OwnerWeak,predicate:Value,handles:Vec<Value>)->Value{let owner=setup(owner);let active=Rc::new(Cell::new(true));owner.borrow_mut().contributions.push(Contribution{active:active.clone(),predicate:Some(predicate),styles:style_items(handles)});Value::Resource(active)}
pub fn rule_dispose(owner:&OwnerWeak,receiver:Value)->Value{setup(owner);match receiver{Value::Resource(active)=>active.set(false),_=>panic!("Rule handle required")};Value::Undefined}
pub fn style_items(handles:Vec<Value>)->Vec<(String,String)>{handles.into_iter().flat_map(|v|match v{Value::Style(items)=>items,_=>panic!("style handle required")}).collect()}
pub fn patch(owner:&OwnerWeak,handles:Vec<Value>,suppress:bool)->Value{let owner=runtime(owner);let items=style_items(handles);let mut o=owner.borrow_mut();o.dirty=true;for item in items{if suppress{o.patch.retain(|(_,group)|group!=&item.1);o.suppressed.insert(item.1);}else{o.suppressed.remove(&item.1);o.patch.retain(|(_,group)|group!=&item.1);o.patch.push(item);}}Value::Undefined}
pub fn clear_patch(owner:&OwnerWeak)->Value{let owner=runtime(owner);let mut o=owner.borrow_mut();o.patch.clear();o.suppressed.clear();o.dirty=true;Value::Undefined}
pub fn styles(owner:&OwnerRef)->Vec<String>{let(contributions,patch,suppressed)={let o=owner.borrow();(o.contributions.clone(),o.patch.clone(),o.suppressed.clone())};let mut merged=Vec::<(String,String)>::new();for contribution in contributions{if !contribution.active.get()||contribution.predicate.is_some_and(|predicate|!predicate.call_authored(vec![]).bool()){continue}for item in contribution.styles{merged.retain(|(_,group)|group!=&item.1);merged.push(item)}}for item in patch{merged.retain(|(_,group)|group!=&item.1);merged.push(item)}merged.into_iter().filter(|(_,group)|!suppressed.contains(group)).map(|(token,_)|token).collect()}
pub fn context_provide(owner:&OwnerWeak,key:Value,value:Value)->Value {
    let owner=setup(owner);let key=context_identity(key);
    let data=value.try_data().unwrap_or_else(||throw_error("CONTEXT_VALUE_INVALID","invalid Context data"));
    if !(owner.borrow().context_accepts)(&key.descriptor,&data) { throw_error("CONTEXT_VALUE_INVALID","invalid Context data"); }
    if owner.borrow().providers.contains_key(&key) { throw_error("CONTEXT_DUPLICATE_PROVIDE","duplicate Context provider"); }
    owner.borrow_mut().providers.insert(key,value);Value::Undefined
}
fn provider(owner:&OwnerRef,key:&NativeContextKey)->Option<OwnerRef>{let mut current=Some(owner.clone());let mut visited=BTreeSet::new();while let Some(node)=current{assert!(visited.insert(Rc::as_ptr(&node)as usize),"cyclic Context ancestry");let o=node.borrow();if o.disposed{return None}if o.providers.contains_key(key){drop(o);return Some(node)}current=o.parent.upgrade();}None}
pub fn context_subscribe(owner:&OwnerWeak,key:Value,optional:bool,callback:Value)->Value {
    let owner=setup(owner);let key=context_identity(key);
    if !optional&&provider(&owner,&key).is_none() { throw_error("CONTEXT_PROVIDER_MISSING","required Context provider missing"); }
    let active=Rc::new(Cell::new(true));
    owner.borrow_mut().subscriptions.push(ContextSubscription{active:active.clone(),key,optional,callback});
    disposer(owner,active,false)
}
pub fn context_read(owner:&OwnerWeak,key:Value,optional:bool)->Value {
    let owner=ensure(owner);let key=context_identity(key);
    if !owner.borrow().subscriptions.iter().any(|s|s.key==key&&s.optional==optional) { throw_error("CONTEXT_SUBSCRIPTION_REQUIRED","Context read requires corresponding subscription"); }
    match provider(&owner,&key) {
        Some(provider)=>provider.borrow().providers[&key].clone(),
        None if optional=>Value::Json(Json::Null),
        None=>throw_error("CONTEXT_DISCONNECTED","required Context provider missing"),
    }
}
struct ContextDispatchReset {key:(usize,NativeContextKey)}
impl Drop for ContextDispatchReset {
    fn drop(&mut self) {CONTEXT_CHANNELS.with(|channels|channels.borrow_mut().remove(&self.key));}
}
pub fn context_update(owner:&OwnerWeak,key:Value,next:Value,optional:bool)->Value {
    let owner=runtime(owner);let key=context_identity(key);
    if !owner.borrow().subscriptions.iter().any(|s|s.key==key&&s.optional==optional) { throw_error("CONTEXT_SUBSCRIPTION_REQUIRED","Context update requires corresponding subscription"); }
    let Some(bound)=provider(&owner,&key) else {
        if optional { return Value::boolean(false); }
        throw_error("CONTEXT_DISCONNECTED","required Context provider missing")
    };
    let previous=bound.borrow().providers[&key].clone();
    let value=if matches!(next,Value::Function(_)){next.call_authored(vec![previous.clone()])}else{next};
    ensure(&Rc::downgrade(&bound));
    let data=value.try_data().unwrap_or_else(||throw_error("CONTEXT_VALUE_INVALID","invalid Context data"));
    if !(owner.borrow().context_accepts)(&key.descriptor,&data) { throw_error("CONTEXT_VALUE_INVALID","invalid Context data"); }
    bound.borrow_mut().providers.insert(key.clone(),value.clone());
    let mut recorded=Vec::new();
    for participant in all_owners(){
        if provider(&participant,&key).is_some_and(|current|Rc::ptr_eq(&current,&bound)){
            let subscriptions=participant.borrow().subscriptions.clone();
            for registration in subscriptions{
                if registration.active.get()&&registration.key==key&&!matches!(registration.callback,Value::Undefined){
                    recorded.push(ContextNotice{target:Rc::downgrade(&participant),registration,next:value.clone(),previous:previous.clone()});
                }
            }
        }
    }
    if !recorded.is_empty(){
        // Core Context channels serialize reentry only for the same provider/key.
        // A different channel remains an ordinary nested callback delivery.
        let channel_key=(Rc::as_ptr(&bound)as usize,key.clone());
        let (channel,dispatch)=CONTEXT_CHANNELS.with(|channels|{
            let mut channels=channels.borrow_mut();
            if let Some(channel)=channels.get(&channel_key){(channel.clone(),false)}else{
                let channel=Rc::new(RefCell::new(ContextChannel::default()));channels.insert(channel_key.clone(),channel.clone());(channel,true)
            }
        });
        channel.borrow_mut().pending.extend(recorded);
        if dispatch{
            let reset=ContextDispatchReset{key:channel_key};let mut failures=Vec::new();
            loop{
                let notice=channel.borrow_mut().pending.pop_front();let Some(notice)=notice else{break};
                if !notice.registration.active.get(){continue}
                let Some(target)=notice.target.upgrade().filter(|owner|!owner.borrow().disposed)else{continue};
                if !provider(&target,&key).is_some_and(|current|Rc::ptr_eq(&current,&bound)){continue}
                if let Err(error)=catch_authored(||invoke(&target,"context-watch",&notice.registration.callback,vec![capability(&notice.target),notice.next,notice.previous])){failures.push(error);}
            }
            drop(reset);
            if failures.len()==1{throw_value(failures.pop().unwrap());}
            if !failures.is_empty(){throw_value(record(vec![("name",Value::string("AggregateError")),("message",Value::string("[Context] callback delivery failed")),("errors",Value::Array(Rc::new(failures)))]));}
        }
    }
    if optional { Value::boolean(true) } else { Value::Undefined }
}
fn trigger_parent(owner:&OwnerRef)->Option<OwnerRef>{if !owner.borrow().modules.hooks.contains("trigger"){return None}owner.borrow().parent.upgrade().filter(|parent|{let parent=parent.borrow();!parent.disposed&&parent.modules.hooks.contains("trigger")})}
fn trigger_child(owner:&OwnerRef)->Option<OwnerRef>{owner.borrow().modules.trigger_child.upgrade().filter(|child|{let child=child.borrow();!child.disposed&&child.modules.hooks.contains("trigger")&&child.parent.upgrade().is_some_and(|parent|Rc::ptr_eq(&parent,owner))})}
fn validate_trigger_child(parent:&OwnerRef,child:&OwnerRef)->Result<(),&'static str>{if trigger_child(parent).is_none_or(|current|Rc::ptr_eq(&current,child)){Ok(())}else{Err("Trigger group cannot branch into sibling surfaces")}}
pub fn as_trigger(owner:&OwnerWeak)->Value{let owner=setup(owner);let parent=owner.borrow().parent.upgrade().filter(|parent|parent.borrow().modules.hooks.contains("trigger"));if let Some(parent)=&parent{validate_trigger_child(parent,&owner).expect("invalid Trigger declaration");}owner.borrow_mut().modules.hooks.insert("trigger");if let Some(parent)=parent{parent.borrow_mut().modules.trigger_child=Rc::downgrade(&owner);}Value::Undefined}
pub fn set_native_parent(owner:&OwnerRef,parent:OwnerWeak)->Result<(),&'static str>{let next=parent.upgrade();if let Some(parent)=&next{if descendant(parent,owner){return Err("cyclic logical ancestry")}if owner.borrow().modules.hooks.contains("trigger")&&parent.borrow().modules.hooks.contains("trigger"){validate_trigger_child(parent,owner)?;}}if let Some(previous)=trigger_parent(owner){if previous.borrow().modules.trigger_child.ptr_eq(&Rc::downgrade(owner)){previous.borrow_mut().modules.trigger_child=Weak::new();}}owner.borrow_mut().parent=parent;if let Some(parent)=trigger_parent(owner){parent.borrow_mut().modules.trigger_child=Rc::downgrade(owner);}topology_refresh();Ok(())}
fn dispatch_root_route(target:&OwnerRef,kind:&str,fields:Map<String,Json>)->bool{if !ready(target)||trigger_child(target).is_some(){return false}let window=target.borrow().window_id;let mut current=Some(target.clone());let mut prevented=false;while let Some(owner)=current{current=trigger_parent(&owner);if owner.borrow().window_id==window{prevented|=dispatch(&owner,kind,false,fields.clone());}}prevented}
pub fn as_focusable(owner:&OwnerWeak)->Value{setup(owner).borrow_mut().focusable=true;module_handle(owner,"focus")}
pub fn as_accessible(owner:&OwnerWeak)->Value{setup(owner).borrow_mut().accessible=true;module_handle(owner,"accessible")}
pub fn focus_configure(owner:&OwnerWeak,config:Value)->Value{let owner=setup(owner);let disabled=config.member("disabled",false);let entry=config.member("entry",false);let mut o=owner.borrow_mut();if !disabled.nullish(){o.focus_disabled=disabled.bool()}if !entry.nullish(){o.focus_entry=entry.bool()}Value::Undefined}
pub fn focus_disabled(owner:&OwnerWeak,disabled:Value)->Value{let owner=runtime(owner);let disabled=disabled.bool();let mut o=owner.borrow_mut();o.focus_disabled=disabled;o.dirty=true;if disabled{o.focus_requested=false;o.modules.focus.request_source=None;o.focused=false;o.modules.focus.visible=false;o.modules.focus.module_active=false;o.modules.focus.blur_requested=true;}Value::Undefined}
pub fn focus_self(owner:&OwnerWeak)->Value{focus_request(owner,Value::Undefined,false)}
pub fn accessible_role(owner:&OwnerWeak,role:Value)->Value{setup(owner).borrow_mut().accessible_role=role;Value::Undefined}
pub fn accessible_state(owner:&OwnerWeak,key:Value,state:Value,options:Value)->Value{
    let owner=setup(owner);let condition=options.member("when",true);
    // Omission alone is unconditional. Null, raw booleans and non-State values
    // cannot become an applicability sentinel or bypass State ownership.
    if !matches!(condition,Value::Undefined){condition.get().bool();}
    let mut owner=owner.borrow_mut();let key=key.text();
    owner.accessible_states.insert(key.clone(),state);
    owner.accessible_state_conditions.remove(&key);
    if !matches!(condition,Value::Undefined){owner.accessible_state_conditions.insert(key,condition);}
    Value::Undefined
}
pub fn accessible_action(owner:&OwnerWeak,key:Value)->Value{setup(owner).borrow_mut().accessible_actions.insert(key.text());Value::Undefined}
pub fn name_content(owner:&OwnerWeak)->Value{let owner=setup(owner);let mut o=owner.borrow_mut();o.name_content=true;o.modules.accessible_text.remove("name");Value::Undefined}
pub fn listen(owner:&OwnerWeak,key:Value,global:bool,callback:Value,options:Value)->Value{listen_with_schema(owner,key,global,callback,options,false)}
pub fn listen_numeric_value(owner:&OwnerWeak,key:Value,global:bool,callback:Value,options:Value)->Value{assert!(!global,"numeric accessibility actions are owner-local");listen_with_schema(owner,key,global,callback,options,true)}
fn listen_with_schema(owner:&OwnerWeak,key:Value,global:bool,callback:Value,options:Value,numeric_value:bool)->Value{let owner=setup(owner);let active=Rc::new(Cell::new(true));let kind=key.text();if global&&kind.starts_with("press."){owner.borrow_mut().press_global=true;}let capture=options.member("capture",true);let once=options.member("once",true);let passive=options.member("passive",true);owner.borrow_mut().listeners.push(Listener{active:active.clone(),kind,global,capture:!capture.nullish()&&capture.bool(),once:!once.nullish()&&once.bool(),consumed_epoch:Rc::new(Cell::new(None)),passive:!passive.nullish()&&passive.bool(),numeric_value,callback});disposer(owner,active,false)}
pub fn prevent(input:Value)->Value{match input{Value::Input(input)=>{if !input.live.get(){throw_error("INPUT_CONTROL_EXPIRED","input control expired")}if !input.passive{input.prevented.set(true)}},_=>panic!("input capability required")};Value::Undefined}
struct InputLifetime<'a>(&'a Input);
impl Drop for InputLifetime<'_>{fn drop(&mut self){self.0.live.set(false);}}
pub fn dispatch(owner:&OwnerRef,kind:&str,global:bool,fields:Map<String,Json>)->bool{dispatch_phase(owner,kind,global,false,fields)}
fn has_listener(owner:&OwnerRef,kind:&str,global:bool,capture:bool)->bool{let owner=owner.borrow();owner.listeners.iter().any(|listener|listener.active.get()&&listener.kind==kind&&listener.global==global&&listener.capture==capture&&(!listener.once||listener.consumed_epoch.get()!=Some(owner.epoch)))}
fn dispatch_phase(owner:&OwnerRef,kind:&str,global:bool,capture:bool,fields:Map<String,Json>)->bool{
    if owner.borrow().disposed||!owner.borrow().present||!owner.borrow().mounted{return false}
    if !global&&(kind.starts_with("pointer.")||kind=="press.commit"||kind=="context.menu")&&!hit_participates(owner){return false}
    let listeners=owner.borrow().listeners.clone();let mut prevented=false;
    for listener in listeners{
        if owner.borrow().disposed||!owner.borrow().mounted{break}
        if !listener.active.get()||listener.global!=global||listener.capture!=capture||listener.kind!=kind{continue}
        if listener.numeric_value&&!fields.get("value").and_then(Json::as_f64).is_some_and(f64::is_finite){continue}
        if listener.once{let epoch=owner.borrow().epoch;if listener.consumed_epoch.get()==Some(epoch){continue}listener.consumed_epoch.set(Some(epoch));}
        let input=Rc::new(Input{fields:fields.clone(),prevented:Cell::new(false),passive:listener.passive,live:Cell::new(true)});
        let delivered=native_host_callback(owner,kind,||invoke_scoped(owner,"event",||{
            let _lifetime=InputLifetime(input.as_ref());listener.callback.call_authored(vec![capability(&Rc::downgrade(owner)),Value::Input(input.clone())])
        }));
        // A prevention request already made by the callback survives its error.
        prevented|=input.prevented.get();if delivered.is_none(){break}
    }
    prevented
}
pub fn event_fields(kind:&str)->Map<String,Json>{let mut fields=Map::new();fields.insert("type".into(),Json::String(kind.into()));fields}
pub fn default_accessibility_event(action:&str)->String{if action=="setValue"{"host:a11y-set-value".into()}else{format!("a11y:{action}")}}
/// One bridge shared by the actual GPUI callback and native consumer tests.
/// A SetValue request carries finite numeric input; policy and State stay authored.
pub fn dispatch_accessibility_action(owner:&OwnerRef,action:&str,data:Option<&gpui::accesskit::ActionData>)->bool{
    let kind={let o=owner.borrow();if o.disposed||!o.present||!o.mounted||!o.accessible||!o.accessible_actions.contains(action){return false}o.modules.action_events.get(action).cloned().unwrap_or_else(||default_accessibility_event(action))};
    // C-EVENT-TYPE-0002: assistive activation enters the same portable intent
    // route as a click. The activate event names the outward signal authored by
    // the Prototype; emitting it here would bypass disabled/selection policy.
    // Explicit host:* input routes retain their authored meaning (for example,
    // changing a multi-axis editor's active axis instead of pressing it).
    if action=="activate"&&!kind.starts_with("host:"){return dispatch_root_route(owner,"press.commit",event_fields("press.commit"))}
    let mut fields=event_fields(&kind);
    if action=="setValue"{
        let Some(gpui::accesskit::ActionData::NumericValue(value))=data else{return false};
        if !value.is_finite(){return false}
        fields.insert("value".into(),Json::from(*value));
    }
    dispatch(owner,&kind,false,fields)
}
pub fn text_content(value:&Option<NativeSnapshot>,slot:&[NativeSnapshot])->String{value.as_ref().map(|snapshot|(snapshot.text)(slot)).unwrap_or_default()}
pub type Drive=Rc<dyn Fn(&mut Window,&mut App)>;
#[derive(Clone,Copy)]
pub struct NativeTranslation{x:crate::gpui_native_style::Dimension,y:crate::gpui_native_style::Dimension}
impl NativeTranslation{const ZERO:Self=Self{x:crate::gpui_native_style::Dimension::ZERO,y:crate::gpui_native_style::Dimension::ZERO};fn is_zero(self)->bool{self.x==Self::ZERO.x&&self.y==Self::ZERO.y}}
struct TranslatedElement{element:AnyElement,translation:NativeTranslation,owner:Option<OwnerWeak>}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for TranslatedElement{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for TranslatedElement{
    type RequestLayoutState=();type PrepaintState=();
    fn id(&self)->Option<gpui::ElementId>{None}
    fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){(self.element.request_layout(window,cx),())}
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,bounds:Bounds<Pixels>,_:&mut (),window:&mut Window,cx:&mut App){
        let offset=gpui::point(gpui::px(self.translation.x.resolve(bounds.size.width.into())),gpui::px(self.translation.y.resolve(bounds.size.height.into())));
        if let Some(owner)=self.owner.as_ref().and_then(Weak::upgrade){owner.borrow_mut().native_translation=offset;}
        window.with_element_offset(offset,|window|self.element.prepaint(window,cx));
    }
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),_:&mut (),window:&mut Window,cx:&mut App){self.element.paint(window,cx)}
}
pub fn translated_element(element:AnyElement,translation:NativeTranslation,owner:Option<OwnerWeak>)->AnyElement{if translation.is_zero(){element}else{TranslatedElement{element,translation,owner}.into_any_element()}}
pub fn native_presentation(owner:&OwnerRef,tokens:&[String])->(gpui::StyleRefinement,bool,NativeTranslation){
    use crate::gpui_native_style::{themes,ColorScheme,Substitution,LengthContext,evaluate_length};
    let(vocabulary,theme)={let o=owner.borrow();(o.style_vocabulary,themes().get(&o.theme,if o.dark{ColorScheme::Dark}else{ColorScheme::Light}).expect("unknown design language"))};
    let mut resolved=vocabulary.resolve_all(tokens.iter().map(String::as_str));assert!(resolved.unknown.is_empty(),"unmapped style tokens: {:?}",resolved.unknown);
    let local:BTreeMap<_,_>=resolved.declarations.extract_if(..,|property,_|property.starts_with("--")).collect();
    let substitute=|value:&str|match theme.substitute_with(value,&local){Substitution::Resolved(text)=>text,other=>panic!("unresolved style declaration: {:?}",other)};
    for value in resolved.declarations.values_mut(){*value=substitute(value);}
    let translation=if resolved.declarations.remove("transform").is_some(){
        for property in ["--pui-scale-x","--pui-scale-y"]{if let Some(value)=local.get(property){assert!(substitute(value).parse::<f32>().is_ok_and(|value|value==1.),"GPUI scale style requires its physical transformed surface");}}
        let dimension=|property:&str|local.get(property).map(|value|evaluate_length(&substitute(value),LengthContext::default()).expect("native translation length")).unwrap_or(crate::gpui_native_style::Dimension::ZERO);
        NativeTranslation{x:dimension("--pui-translate-x"),y:dimension("--pui-translate-y")}
    }else{NativeTranslation::ZERO};
    // A viewport wrapper, rather than Absolute alone, realizes fixed positioning.
    let fixed=resolved.declarations.get("position").is_some_and(|position|position=="fixed");if fixed{resolved.declarations.insert("position".into(),"absolute".into());}resolved.declarations.entry("position".into()).or_insert_with(||"relative".into());
    let mapped=crate::gpui_native_style_mapping::map(&resolved,LengthContext::default());assert!(mapped.unmapped_properties().is_empty(),"GPUI style host prerequisite: {:?}",mapped.unmapped);(mapped.refinement,fixed,translation)
}
pub fn native_style(owner:&OwnerRef,tokens:&[String])->gpui::StyleRefinement{let(style,fixed,translation)=native_presentation(owner,tokens);assert!(!fixed,"fixed style requires its physical viewport surface");assert!(translation.is_zero(),"translation style requires its physical element surface");style}
pub fn paint_native_root(value:&NativeSnapshot,owner:&OwnerRef,focus:&FocusHandle,path:String,drive:Drive)->AnyElement{
// The stable native host Root retains channels, feedback and commit ownership.
// Authored children are captured native factories, never semantic tag dispatch.
let mut native_role=None;let mut element=div().id(gpui::ElementId::Name(path.clone().into()));let tokens=styles(owner);owner.borrow_mut().native_translation=gpui::Point::default();let(style,fixed,translation)=native_presentation(owner,&tokens);*element.style()=style;
if image_supported(owner){element=element.role(gpui::Role::Image);}
if owner.borrow().modules.hooks.contains("scroll"){
    let projection={let o=owner.borrow();let s=&o.modules.scroll;s.builtin_projection().then(||(text_or(field(&s.config,"axes"),"both"),s.handle().clone()))};
    if let Some((axes,handle))=projection{element=element.track_scroll(&handle);if axes!="vertical"{element=element.overflow_x_scroll();}if axes!="horizontal"{element=element.overflow_y_scroll();}}
    let scrolled=owner.clone();let scroll_drive=drive.clone();element=element.on_scroll_wheel(move|_,window,cx|{if scroll_ready(&scrolled){scroll_input(&scrolled);scroll_drive(window,cx);}});
}
let (accessible,role,name,actions,slot)={let o=owner.borrow();(o.accessible,o.accessible_role.clone(),o.name_content,o.accessible_actions.clone(),o.slot.clone())};if native_focus_available(owner)&&!text_disabled(owner){element=element.track_focus(focus)}if text_supported(owner){element=element.role(gpui::Role::TextInput);}if accessible{let role=resolved(&role);if !role.nullish(){if let Some(role)=role_from_name(&role.text()){native_role=Some(role);if role!=gpui::Role::GenericContainer{element=element.role(role);}}}if name{element=element.aria_label((value.text)(&slot));}for action in actions{let owner=owner.clone();let drive=drive.clone();let kind=action.clone();element=element.on_a11y_action(action_from_name(&action),move |data,window,cx|{if dispatch_accessibility_action(&owner,&kind,data){window.prevent_default()}drive(window,cx)});}}
if native_pointer_input_needed(owner){
let capture_down=owner.clone();let capture_drive=drive.clone();element.interactivity().capture_any_mouse_down(move|event,window,cx|{let mut fields=event_fields("host:mousedown");modifiers(&mut fields,event.modifiers);if dispatch_phase(&capture_down,"host:mousedown",false,true,fields){window.prevent_default()}capture_drive(window,cx)});
let on_down=owner.clone();let down_drive=drive.clone();element.interactivity().on_any_mouse_down(move|event,window,cx|{if claim_pointer(&on_down){let mut fields=event_fields("pointer.down");modifiers(&mut fields,event.modifiers);let mut prevented=dispatch_root_route(&on_down,"pointer.down",fields.clone());if event.button==gpui::MouseButton::Left{prevented|=pointer_intent(&on_down,"press.start",press_fields("press.start",&fields));}if prevented{window.prevent_default()}else if !window.default_prevented(){text_pointer_down(&on_down,event,window,cx);}if event.button==gpui::MouseButton::Right&&dispatch_root_route(&on_down,"context.menu",press_fields("context.menu",&fields)){window.prevent_default()}}let mut fields=event_fields("host:mousedown");modifiers(&mut fields,event.modifiers);if dispatch(&on_down,"host:mousedown",false,fields){window.prevent_default()}down_drive(window,cx)});
let capture_up=owner.clone();let capture_drive=drive.clone();element.interactivity().capture_any_mouse_up(move|event,window,cx|{let mut fields=event_fields("host:mouseup");modifiers(&mut fields,event.modifiers);if dispatch_phase(&capture_up,"host:mouseup",false,true,fields){window.prevent_default()}capture_drive(window,cx)});
let on_up=owner.clone();let up_drive=drive.clone();element.interactivity().on_any_mouse_up(move|event,window,cx|{if claim_pointer(&on_up){let mut fields=event_fields("pointer.up");modifiers(&mut fields,event.modifiers);let mut prevented=dispatch_root_route(&on_up,"pointer.up",fields.clone());if event.button==gpui::MouseButton::Left{prevented|=pointer_intent(&on_up,"press.end",press_fields("press.end",&fields));}if prevented{window.prevent_default()}}let mut fields=event_fields("host:mouseup");modifiers(&mut fields,event.modifiers);if dispatch(&on_up,"host:mouseup",false,fields){window.prevent_default()}up_drive(window,cx)});
let on_move=owner.clone();let move_drive=drive.clone();element=element.on_mouse_move(move|event,window,cx|{if claim_pointer(&on_move){let mut fields=event_fields("pointer.move");modifiers(&mut fields,event.modifiers);if dispatch_root_route(&on_move,"pointer.move",fields){window.prevent_default()}else if !window.default_prevented(){text_pointer_move(&on_move,event,window,cx);}}let mut fields=event_fields("host:mousemove");modifiers(&mut fields,event.modifiers);if dispatch(&on_move,"host:mousemove",false,fields){window.prevent_default()}move_drive(window,cx)});
let on_hover=owner.clone();let hover_drive=drive.clone();element=element.on_hover(move|hover,window,cx|{let kind=if *hover{"pointer.enter"}else{"pointer.leave"};let mut prevented=dispatch_root_route(&on_hover,kind,event_fields(kind));if !*hover{prevented|=pointer_intent(&on_hover,"press.cancel",event_fields("press.cancel"));}if prevented{window.prevent_default()}let kind=if *hover{"host:mouseenter"}else{"host:mouseleave"};if dispatch(&on_hover,kind,false,event_fields(kind)){window.prevent_default()}hover_drive(window,cx)});
let on_click=owner.clone();let click_drive=drive.clone();element=element.on_click(move|event,window,cx|{let(suppress,cancelled)={let mut o=on_click.borrow_mut();(std::mem::take(&mut o.suppress_click),std::mem::take(&mut o.press.cancelled))};if !cancelled&&!(suppress&&matches!(event,gpui::ClickEvent::Keyboard(_)))&&claim_click(&on_click)&&dispatch_root_route(&on_click,"press.commit",click_event_fields("press.commit",event)){window.prevent_default()}if dispatch(&on_click,"host:click",false,click_event_fields("host:click",event)){window.prevent_default()}global_native_click(event,window,cx);click_drive(window,cx)});

}
if native_key_input_needed(owner){
let capture_key=owner.clone();element=element.capture_key_down(move|event,window,_|{reset_key();let mut fields=event_fields("host:keydown");if let Some(key)=portable_key(&event.keystroke){fields.insert("key".into(),Json::String(key));}modifiers(&mut fields,event.keystroke.modifiers);if dispatch_phase(&capture_key,"host:keydown",false,true,fields){window.prevent_default()}});let capture_key=owner.clone();element=element.capture_key_up(move|event,window,_|{reset_key();let mut fields=event_fields("host:keyup");if let Some(key)=portable_key(&event.keystroke){fields.insert("key".into(),Json::String(key));}modifiers(&mut fields,event.keystroke.modifiers);if dispatch_phase(&capture_key,"host:keyup",false,true,fields){window.prevent_default()}});
let on_key=owner.clone();element=element.on_key_down(move|event,window,cx|{let mut fields=event_fields("host:keydown");fields.insert("key".into(),Json::String(portable_key(&event.keystroke).expect("portable key string")));modifiers(&mut fields,event.keystroke.modifiers);if dispatch(&on_key,"host:keydown",false,fields){window.prevent_default()}collect_key(&on_key);global_key_down(event,window,cx);});
let on_key=owner.clone();element=element.on_key_up(move|event,window,cx|{let mut fields=event_fields("host:keyup");fields.insert("key".into(),Json::String(portable_key(&event.keystroke).expect("portable key string")));modifiers(&mut fields,event.keystroke.modifiers);if dispatch(&on_key,"host:keyup",false,fields){window.prevent_default()}collect_key(&on_key);global_key_up(event,window,cx);});
}
let measured=owner.clone();let committed=owner.clone();let revision=owner.borrow().projection_revision;let commit_drive=drive.clone();let commit_focus=focus.clone();
element=element.child(canvas(move|_,window,_|{note_host_order(&measured);},move|_,_,window,cx|{
    if native_pointer_input_needed(&committed){register_pointer_input(window,Some(Rc::downgrade(&committed)));}
    if crate::gpui_native_move::move_owner_has_binding(&committed){crate::gpui_native_move::register_move_input(window,Rc::downgrade(&committed));}
    if native_boundary_input_needed(&committed){window.on_mouse_event::<gpui::MouseDownEvent>(native_pointer_press);}
    if committed.borrow().modules.hooks.contains("overlay"){window.on_mouse_event::<gpui::ScrollWheelEvent>(overlay_modal_wheel);}
    let owner=committed.clone();let drive=commit_drive.clone();let focus=commit_focus.clone();let epoch=owner.borrow().epoch;
    window.defer(cx,move|window,cx|{
        native_host_callback(&owner,"commit",||{
            let current={let o=owner.borrow();!o.disposed&&o.present&&o.bounds.is_some()&&o.epoch==epoch&&o.window_id==Some(window.window_handle().window_id())&&o.projection_revision==revision};
            if !current{return}if native_focus_observation_needed(&owner){owner.borrow_mut().modules.focus.native=Some(focus);}
            #[cfg(feature="experimental-focus-tree")]refresh_native_entry(&owner,window);
            Owner::commit(&owner,revision);if owner.borrow().disposed||!owner.borrow().present{return}
            poll_state_watchers();if !owner.borrow().disposed&&owner.borrow().present{drive(window,cx)}
        });
    });
}).absolute().size_full());
if image_supported(owner){let fill_height=matches!(element.style().size.height,Some(gpui::Length::Definite(_)));element=element.child(image_element(owner,fill_height));}
if text_supported(owner){let fill_height=matches!(element.style().size.height,Some(gpui::Length::Definite(_)));let scrolled=owner.clone();element=element.on_scroll_wheel(move|event,window,cx|{if text_scroll(&scrolled,event,window,cx){cx.stop_propagation();}}).child(NativeTextElement::new(owner,focus,fill_height));}
let slot=owner.borrow().slot.clone();let mut child_index=0;for child in (value.frame)(owner,focus,&path,&drive,&slot,&mut child_index){element=element.child(child);}
// External virtual surfaces establish their real extents while prepainting the
// slot. Observe and apply deferred requests after those children, not before.
if owner.borrow().modules.hooks.contains("scroll"){let scrolled=owner.clone();element=element.child(canvas(move|_,window,_|scroll_frame(&scrolled,window),|_,_,_,_|{}).absolute().size_full());}
let element=translated_element(crate::gpui_native_a11y::Semantics{element:element.into_element(),owner:Rc::downgrade(owner),role:native_role}.into_any_element(),translation,Some(Rc::downgrade(owner)));let(element,fixed)=positioning_root(owner,element,fixed);overlay_root(owner,element,fixed)
}
pub fn modifiers(fields:&mut Map<String,Json>,modifiers:gpui::Modifiers){fields.insert("altKey".into(),Json::Bool(modifiers.alt));fields.insert("ctrlKey".into(),Json::Bool(modifiers.control));fields.insert("shiftKey".into(),Json::Bool(modifiers.shift));fields.insert("metaKey".into(),Json::Bool(modifiers.platform));}
pub fn portable_key(key:&gpui::Keystroke)->Option<String>{Some(crate::gpui_native_key::portable_key(key).unwrap_or_else(||"Unidentified".into()))}
pub fn role_from_name(name:&str)->Option<gpui::Role>{
    // ARIA role tokens use the first recognized ASCII-case-insensitive token.
    // Unknown tokens preserve the physical host's native role.
    for token in name.split_ascii_whitespace(){
        if token.len()>32{continue}
        let mut folded=[0u8;32];for(output,input)in folded.iter_mut().zip(token.bytes()){*output=input.to_ascii_lowercase();}
        let token=std::str::from_utf8(&folded[..token.len()]).expect("ASCII case folding preserves UTF-8");
        let role=match token{
            "alert"=>gpui::Role::Alert,
            "alertdialog"=>gpui::Role::AlertDialog,
            "application"=>gpui::Role::Application,
            "article"=>gpui::Role::Article,
            "banner"=>gpui::Role::Banner,
            "blockquote"=>gpui::Role::Blockquote,
            "button"=>gpui::Role::Button,
            "caption"=>gpui::Role::Caption,
            "cell"=>gpui::Role::Cell,
            "checkbox"=>gpui::Role::CheckBox,
            "code"=>gpui::Role::Code,
            "columnheader"=>gpui::Role::ColumnHeader,
            "combobox"=>gpui::Role::ComboBox,
            "comment"=>gpui::Role::Comment,
            "complementary"=>gpui::Role::Complementary,
            "contentinfo"=>gpui::Role::ContentInfo,
            "definition"=>gpui::Role::Definition,
            "deletion"=>gpui::Role::ContentDeletion,
            "dialog"=>gpui::Role::Dialog,
            "directory"=>gpui::Role::List,
            "document"=>gpui::Role::Document,
            "emphasis"=>gpui::Role::Emphasis,
            "feed"=>gpui::Role::Feed,
            "figure"=>gpui::Role::Figure,
            "form"=>gpui::Role::Form,
            "generic"=>gpui::Role::GenericContainer,
            "grid"=>gpui::Role::Grid,
            "gridcell"=>gpui::Role::GridCell,
            "group"=>gpui::Role::Group,
            "heading"=>gpui::Role::Heading,
            "img"=>gpui::Role::Image,
            "insertion"=>gpui::Role::ContentInsertion,
            "link"=>gpui::Role::Link,
            "list"=>gpui::Role::List,
            "listbox"=>gpui::Role::ListBox,
            "listitem"=>gpui::Role::ListItem,
            "log"=>gpui::Role::Log,
            "main"=>gpui::Role::Main,
            "mark"=>gpui::Role::Mark,
            "marquee"=>gpui::Role::Marquee,
            "math"=>gpui::Role::Math,
            "menu"=>gpui::Role::Menu,
            "menubar"=>gpui::Role::MenuBar,
            "menuitem"=>gpui::Role::MenuItem,
            "menuitemcheckbox"=>gpui::Role::MenuItemCheckBox,
            "menuitemradio"=>gpui::Role::MenuItemRadio,
            "meter"=>gpui::Role::Meter,
            "navigation"=>gpui::Role::Navigation,
            "none"=>gpui::Role::GenericContainer,
            "note"=>gpui::Role::Note,
            "option"=>gpui::Role::ListBoxOption,
            "paragraph"=>gpui::Role::Paragraph,
            "presentation"=>gpui::Role::GenericContainer,
            "progressbar"=>gpui::Role::ProgressIndicator,
            "radio"=>gpui::Role::RadioButton,
            "radiogroup"=>gpui::Role::RadioGroup,
            "region"=>gpui::Role::Region,
            "row"=>gpui::Role::Row,
            "rowgroup"=>gpui::Role::RowGroup,
            "rowheader"=>gpui::Role::RowHeader,
            "scrollbar"=>gpui::Role::ScrollBar,
            "search"=>gpui::Role::Search,
            "searchbox"=>gpui::Role::SearchInput,
            "separator"=>gpui::Role::Splitter,
            "slider"=>gpui::Role::Slider,
            "spinbutton"=>gpui::Role::SpinButton,
            "status"=>gpui::Role::Status,
            "strong"=>gpui::Role::Strong,
            "suggestion"=>gpui::Role::Suggestion,
            "switch"=>gpui::Role::Switch,
            "tab"=>gpui::Role::Tab,
            "table"=>gpui::Role::Table,
            "tablist"=>gpui::Role::TabList,
            "tabpanel"=>gpui::Role::TabPanel,
            "term"=>gpui::Role::Term,
            "time"=>gpui::Role::Time,
            "timer"=>gpui::Role::Timer,
            "toolbar"=>gpui::Role::Toolbar,
            "tooltip"=>gpui::Role::Tooltip,
            "tree"=>gpui::Role::Tree,
            "treegrid"=>gpui::Role::TreeGrid,
            "treeitem"=>gpui::Role::TreeItem,
            "graphics-document"=>gpui::Role::GraphicsDocument,
            "graphics-object"=>gpui::Role::GraphicsObject,
            "graphics-symbol"=>gpui::Role::GraphicsSymbol,
            "doc-abstract"=>gpui::Role::DocAbstract,
            "doc-acknowledgements"=>gpui::Role::DocAcknowledgements,
            "doc-afterword"=>gpui::Role::DocAfterword,
            "doc-appendix"=>gpui::Role::DocAppendix,
            "doc-backlink"=>gpui::Role::DocBackLink,
            "doc-biblioentry"=>gpui::Role::DocBiblioEntry,
            "doc-bibliography"=>gpui::Role::DocBibliography,
            "doc-biblioref"=>gpui::Role::DocBiblioRef,
            "doc-chapter"=>gpui::Role::DocChapter,
            "doc-colophon"=>gpui::Role::DocColophon,
            "doc-conclusion"=>gpui::Role::DocConclusion,
            "doc-cover"=>gpui::Role::DocCover,
            "doc-credit"=>gpui::Role::DocCredit,
            "doc-credits"=>gpui::Role::DocCredits,
            "doc-dedication"=>gpui::Role::DocDedication,
            "doc-endnote"=>gpui::Role::DocEndnote,
            "doc-endnotes"=>gpui::Role::DocEndnotes,
            "doc-epigraph"=>gpui::Role::DocEpigraph,
            "doc-epilogue"=>gpui::Role::DocEpilogue,
            "doc-errata"=>gpui::Role::DocErrata,
            "doc-example"=>gpui::Role::DocExample,
            "doc-footnote"=>gpui::Role::DocFootnote,
            "doc-foreword"=>gpui::Role::DocForeword,
            "doc-glossary"=>gpui::Role::DocGlossary,
            "doc-glossref"=>gpui::Role::DocGlossRef,
            "doc-index"=>gpui::Role::DocIndex,
            "doc-introduction"=>gpui::Role::DocIntroduction,
            "doc-noteref"=>gpui::Role::DocNoteRef,
            "doc-notice"=>gpui::Role::DocNotice,
            "doc-pagebreak"=>gpui::Role::DocPageBreak,
            "doc-pagefooter"=>gpui::Role::DocPageFooter,
            "doc-pageheader"=>gpui::Role::DocPageHeader,
            "doc-pagelist"=>gpui::Role::DocPageList,
            "doc-part"=>gpui::Role::DocPart,
            "doc-preface"=>gpui::Role::DocPreface,
            "doc-prologue"=>gpui::Role::DocPrologue,
            "doc-pullquote"=>gpui::Role::DocPullquote,
            "doc-qna"=>gpui::Role::DocQna,
            "doc-subtitle"=>gpui::Role::DocSubtitle,
            "doc-tip"=>gpui::Role::DocTip,
            "doc-toc"=>gpui::Role::DocToc,
            _=>continue,
        };return Some(role)
    }None
}
pub fn action_from_name(name:&str)->gpui::AccessibleAction{match name{"activate"|"click"|"press"=>gpui::AccessibleAction::Click,"focus"=>gpui::AccessibleAction::Focus,"increment"=>gpui::AccessibleAction::Increment,"decrement"=>gpui::AccessibleAction::Decrement,"setValue"=>gpui::AccessibleAction::SetValue,_=>panic!("unsupported native accessibility action: {name}")}}
thread_local!{static KEY_TARGET:RefCell<Option<OwnerWeak>>=RefCell::new(None);static KEY_DISPATCHED:Cell<bool>=Cell::new(false);static CLICK_TARGET:RefCell<Option<OwnerWeak>>=RefCell::new(None);static POINTER_TARGET:RefCell<Option<OwnerWeak>>=RefCell::new(None);static POINTER_DISPATCHED:Cell<bool>=Cell::new(false);static GLOBAL_CLICK_DISPATCHED:Cell<bool>=Cell::new(false);}
thread_local!{static KEY_SCOPES:RefCell<Vec<(gpui::WindowId,gpui::WeakFocusHandle)>>=RefCell::new(Vec::new());}
pub fn register_key_scope(focus:&FocusHandle,window:&Window){let id=window.window_handle().window_id();KEY_SCOPES.with(|scopes|{let mut scopes=scopes.borrow_mut();if scopes.iter().any(|(window,scope)|*window==id&&scope==focus){return}scopes.retain(|(_,scope)|scope.upgrade().is_some());scopes.push((id,focus.downgrade()));});}
// Resolve against the current physical tree, including cached or reparented views.
// Equal handles are excluded: SDK contains(h,h) is not a physical-presence check.
pub fn outer_key_scope(focus:&FocusHandle,window:&Window)->bool{let id=window.window_handle().window_id();KEY_SCOPES.with(|scopes|!scopes.borrow().iter().any(|(scope_window,scope)|*scope_window==id&&scope!=focus&&scope.upgrade().is_some_and(|parent|parent.contains(focus,window))))}
// Hooks and listeners are installed during checked setup; these direct host
// predicates acquire only the native input resources actually requested.
pub fn native_focus_observation_needed(owner:&OwnerRef)->bool{let o=owner.borrow();o.focusable||o.modules.hooks.iter().any(|hook|matches!(*hook,"focus"|"focus-entry"|"focus-scope"|"focus-roving"|"trigger"|"text-control"))||o.listeners.iter().any(|listener|matches!(listener.kind.as_str(),"host:focus"|"host:blur"|"focus"|"blur")||listener.kind.starts_with("press."))}
pub fn native_pointer_input_needed(owner:&OwnerRef)->bool{let o=owner.borrow();o.modules.hooks.contains("trigger")||o.modules.hooks.contains("text-control")||o.listeners.iter().any(|listener|listener.kind.starts_with("pointer.")||listener.kind.starts_with("press.")||listener.kind.starts_with("host:mouse")||matches!(listener.kind.as_str(),"host:click"|"context.menu"))}
pub fn native_key_input_needed(owner:&OwnerRef)->bool{let o=owner.borrow();o.focusable||o.modules.hooks.iter().any(|hook|matches!(*hook,"focus-scope"|"focus-roving"|"trigger"|"text-control"))||o.listeners.iter().any(|listener|listener.kind.starts_with("key.")||listener.kind.starts_with("press.")||matches!(listener.kind.as_str(),"host:keydown"|"host:keyup"))}
pub fn native_boundary_input_needed(owner:&OwnerRef)->bool{let o=owner.borrow();o.modules.boundary.observing||o.modules.hooks.contains("overlay")}
pub fn window_input_needed(owner:&OwnerRef)->bool{let owner=owner.borrow();owner.modules.hooks.contains("focus-scope")||owner.modules.focus.roving_declared||owner.listeners.iter().any(|listener|listener.global&&(listener.kind.starts_with("key.")||listener.kind.starts_with("press.")||matches!(listener.kind.as_str(),"host:keydown"|"host:keyup")))}
pub fn restore_window_input(owner:&OwnerRef,focus:&FocusHandle,window:&mut Window,cx:&mut App){
    if !ready(owner){return}
    // Focus-lost also fires when a retained handle's native node disappears.
    // Respect any earlier native fallback that already chose a different handle.
    let current=window.focused(cx);let unchanged={let owner=owner.borrow();match(current.as_ref(),owner.window_input_observed.as_ref()){(None,None)=>true,(Some(current),Some(observed))=>observed==current,_=>false}};
    if current.is_none()||unchanged{focus.focus(window,cx);}
}
// A non-tab-stop dispatch carrier, not the component's authored Focus or Root.
// Its listeners run only while no actual control owns native keyboard focus.
struct WindowInput{element:AnyElement,focus:FocusHandle,owner:OwnerWeak}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl IntoElement for WindowInput{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for WindowInput{
    type RequestLayoutState=();type PrepaintState=();
    fn id(&self)->Option<gpui::ElementId>{None}
    fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){(self.element.request_layout(window,cx),())}
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),window:&mut Window,cx:&mut App){register_key_scope(&self.focus,window);if window.focused(cx).is_none(){self.focus.focus(window,cx);}window.set_focus_handle(&self.focus,cx);if let Some(owner)=self.owner.upgrade(){owner.borrow_mut().window_input_observed=window.focused(cx).map(|focus|focus.downgrade());}self.element.prepaint(window,cx);}
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),_:&mut (),window:&mut Window,cx:&mut App){
        // SDK focus-lost fallback runs after paint. Test the lease at delivery,
        // so restoring a carrier does not require an unrelated component redraw.
        let down=self.focus.clone();window.on_key_event::<gpui::KeyDownEvent>(move|event,phase,window,cx|{if !down.is_focused(window){return}if phase.capture(){reset_key();}else{global_key_down(event,window,cx);}});
        let up=self.focus.clone();window.on_key_event::<gpui::KeyUpEvent>(move|event,phase,window,cx|{if phase.capture(){if up.is_focused(window){reset_key();}if outer_key_scope(&up,window){global_host_key_up_capture(event,window,cx);}}else if up.is_focused(window){global_key_up(event,window,cx);}});
        self.element.paint(window,cx);
    }
}
pub fn window_input_scope(element:AnyElement,focus:&FocusHandle,owner:&OwnerRef)->AnyElement{WindowInput{element,focus:focus.clone(),owner:Rc::downgrade(owner)}.into_any_element()}
pub fn reset_key(){GLOBAL_CLICK_DISPATCHED.with(|dispatched|dispatched.set(false));KEY_TARGET.with(|target|*target.borrow_mut()=None);KEY_DISPATCHED.with(|dispatched|dispatched.set(false));}
pub fn reset_click(){CLICK_TARGET.with(|target|*target.borrow_mut()=None);}
pub fn collect_key(owner:&OwnerRef){KEY_TARGET.with(|target|{if target.borrow().is_none(){*target.borrow_mut()=Some(Rc::downgrade(owner));}});}
pub fn claim_click(owner:&OwnerRef)->bool{CLICK_TARGET.with(|target|{if target.borrow().is_none(){*target.borrow_mut()=Some(Rc::downgrade(owner));}target.borrow().as_ref().and_then(Weak::upgrade).is_some_and(|target|Rc::ptr_eq(&target,owner))})}
fn claim_pointer(owner:&OwnerRef)->bool{POINTER_TARGET.with(|target|{let mut target=target.borrow_mut();if target.is_some(){false}else{*target=Some(Rc::downgrade(owner));true}})}
fn press_fields(kind:&str,fields:&Map<String,Json>)->Map<String,Json>{let mut fields=fields.clone();fields.insert("type".into(),Json::String(kind.into()));fields}
fn pointer_intent(owner:&OwnerRef,kind:&str,fields:Map<String,Json>)->bool{
    if !ready(owner)||trigger_child(owner).is_some()||!hit_participates(owner){return false}
    let emit={let mut owner=owner.borrow_mut();match kind{
        "press.start"=>{let start=!owner.press.pointer;owner.press.pointer=true;owner.press.cancelled=false;start},
        "press.end"=>std::mem::take(&mut owner.press.pointer),
        "press.cancel"=>{let held=std::mem::take(&mut owner.press.pointer);owner.press.cancelled|=held;held},_=>false,
    }};emit&&dispatch_root_route(owner,kind,fields)
}
pub fn native_press_blur(owner:&OwnerRef)->bool{let held=owner.borrow_mut().press.key.take().is_some();held&&dispatch_root_route(owner,"press.cancel",event_fields("press.cancel"))}
pub fn native_window_blur(owner:&OwnerRef)->bool{
    crate::gpui_native_move::move_blur(owner);
    let(root,global)={let mut o=owner.borrow_mut();let root=o.press.key.take().is_some()||o.press.pointer;let global=o.press.global_key.take().is_some()||o.press.global_pointer;let keyboard_click=o.press.global_keyboard_click;let pointer_cancelled=o.press.cancelled||o.press.pointer;let global_pointer_cancelled=o.press.global_pointer_cancelled||o.press.global_pointer;o.press=NativePress::default();o.press.global_keyboard_click=keyboard_click;o.press.cancelled=pointer_cancelled;o.press.global_pointer_cancelled=global_pointer_cancelled;(root,global)};
    let mut prevented=false;if root{prevented|=dispatch_root_route(owner,"press.cancel",event_fields("press.cancel"));}if global{prevented|=dispatch(owner,"press.cancel",true,event_fields("press.cancel"));}prevented
}
pub fn click_event_fields(kind:&str,event:&gpui::ClickEvent)->Map<String,Json>{
    let mut fields=event_fields(kind);modifiers(&mut fields,event.modifiers());fields
}
pub fn global_native_click(event:&gpui::ClickEvent,window:&mut Window,cx:&mut App){
    if matches!(event,gpui::ClickEvent::Mouse(event)if event.up.button!=gpui::MouseButton::Left){return}
    if GLOBAL_CLICK_DISPATCHED.with(|dispatched|dispatched.replace(true)){return}
    let keyboard=matches!(event,gpui::ClickEvent::Keyboard(_));let mut prevented=false;
    for owner in owners_in(window){if !owner.borrow().press_global{continue}let suppress={let mut owner=owner.borrow_mut();let suppress=if keyboard{owner.press.global_keyboard_click}else{owner.press.global_pointer_cancelled};owner.press.global_keyboard_click=false;owner.press.global_pointer_cancelled=false;suppress};if !suppress{prevented|=dispatch(&owner,"press.commit",true,click_event_fields("press.commit",event));}}
    if prevented{window.prevent_default()}drive_owners(window,cx);
}
fn global_pointer(kind:&str,host_kind:&str,modifiers_value:gpui::Modifiers,primary:bool,phase:gpui::DispatchPhase,window:&mut Window,cx:&mut App){
    if phase.capture(){POINTER_TARGET.with(|target|*target.borrow_mut()=None);POINTER_DISPATCHED.with(|dispatched|dispatched.set(false));if kind=="pointer.up"{reset_click();GLOBAL_CLICK_DISPATCHED.with(|dispatched|dispatched.set(false));}return}
    if !phase.bubble()||POINTER_DISPATCHED.with(|dispatched|dispatched.replace(true)){return}
    let mut fields=event_fields(kind);modifiers(&mut fields,modifiers_value);let mut host_fields=event_fields(host_kind);modifiers(&mut host_fields,modifiers_value);let mut prevented=false;
    for owner in owners_in(window){
        prevented|=dispatch(&owner,kind,true,fields.clone());prevented|=dispatch(&owner,host_kind,true,host_fields.clone());
        if primary&&ready(&owner)&&owner.borrow().press_global{let intent={let mut owner=owner.borrow_mut();match kind{"pointer.down"=>{owner.press.global_keyboard_click=false;owner.press.global_pointer_cancelled=false;if owner.press.global_pointer{None}else{owner.press.global_pointer=true;Some("press.start")}},"pointer.up"=>if std::mem::take(&mut owner.press.global_pointer){Some("press.end")}else{None},_=>None}};if let Some(intent)=intent{prevented|=dispatch(&owner,intent,true,press_fields(intent,&fields));}}
        if primary&&kind=="pointer.up"&&owner.borrow().press.pointer{let intent=if native_host_hit(&owner,window.mouse_position(),window){"press.end"}else{"press.cancel"};prevented|=pointer_intent(&owner,intent,press_fields(intent,&fields));}
    }
    if prevented{window.prevent_default();}drive_owners(window,cx);
}
fn global_host_pointer_capture(owner:&Option<OwnerWeak>,kind:&str,modifiers_value:gpui::Modifiers,phase:gpui::DispatchPhase,window:&mut Window,cx:&mut App){if !phase.capture(){return}let Some(owner)=owner.as_ref().and_then(Weak::upgrade)else{return};if !ready(&owner)||owner.borrow().window_id!=Some(window.window_handle().window_id())||!has_listener(&owner,kind,true,true){return}let mut fields=event_fields(kind);modifiers(&mut fields,modifiers_value);if dispatch_phase(&owner,kind,true,true,fields){window.prevent_default()}let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx)}}
pub fn register_pointer_input(window:&mut Window,owner:Option<OwnerWeak>){
    let down=owner.clone();window.on_mouse_event::<gpui::MouseDownEvent>(move|event,phase,window,cx|{global_host_pointer_capture(&down,"host:mousedown",event.modifiers,phase,window,cx);global_pointer("pointer.down","host:mousedown",event.modifiers,event.button==gpui::MouseButton::Left,phase,window,cx);});
    let up=owner.clone();window.on_mouse_event::<gpui::MouseUpEvent>(move|event,phase,window,cx|{global_host_pointer_capture(&up,"host:mouseup",event.modifiers,phase,window,cx);global_pointer("pointer.up","host:mouseup",event.modifiers,event.button==gpui::MouseButton::Left,phase,window,cx);});
    window.on_mouse_event::<gpui::MouseMoveEvent>(move|event,phase,window,cx|{global_host_pointer_capture(&owner,"host:mousemove",event.modifiers,phase,window,cx);global_pointer("pointer.move","host:mousemove",event.modifiers,false,phase,window,cx);});
}
fn global_host_key_down_capture(owner:&OwnerRef,event:&gpui::KeystrokeEvent,window:&mut Window,cx:&mut App){if !ready(owner)||owner.borrow().window_id!=Some(window.window_handle().window_id())||!has_listener(owner,"host:keydown",true,true){return}let mut fields=event_fields("host:keydown");fields.insert("key".into(),Json::String(portable_key(&event.keystroke).expect("portable key string")));modifiers(&mut fields,event.keystroke.modifiers);if dispatch_phase(owner,"host:keydown",true,true,fields){window.prevent_default()}let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx)}}
pub fn subscribe_global_key_capture(owner:&OwnerRef,cx:&mut App)->Option<gpui::Subscription>{if !has_listener(owner,"host:keydown",true,true){return None}let owner=Rc::downgrade(owner);Some(cx.intercept_keystrokes(move|event,window,cx|{if let Some(owner)=owner.upgrade(){global_host_key_down_capture(&owner,event,window,cx);}}))}
pub fn global_host_key_up_capture(event:&gpui::KeyUpEvent,window:&mut Window,cx:&mut App){let mut fields=None;let mut prevented=false;for owner in owners_in(window){if !has_listener(&owner,"host:keyup",true,true){continue}let fields=fields.get_or_insert_with(||{let mut fields=event_fields("host:keyup");fields.insert("key".into(),Json::String(portable_key(&event.keystroke).expect("portable key string")));modifiers(&mut fields,event.keystroke.modifiers);fields});prevented|=dispatch_phase(&owner,"host:keyup",true,true,fields.clone());}if fields.is_some(){if prevented{window.prevent_default()}drive_owners(window,cx);}}
fn owners_in(window:&Window)->Vec<OwnerRef>{let id=window.window_handle().window_id();OWNERS.with(|owners|{let mut owners=owners.borrow_mut();owners.retain(|owner|owner.strong_count()!=0);owners.iter().filter_map(Weak::upgrade).filter(|owner|{let o=owner.borrow();!o.disposed&&o.mounted&&o.present&&o.window_id==Some(id)}).collect()})}
pub fn drive_owners(window:&mut Window,cx:&mut App){for owner in owners_in(window){let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx)}}}
pub fn global_key_down(event:&gpui::KeyDownEvent,window:&mut Window,cx:&mut App){
    if KEY_DISPATCHED.with(|dispatched|dispatched.replace(true)){return}
    let key=portable_key(&event.keystroke);let mut fields=event_fields("key.down");if let Some(key)=&key{fields.insert("key".into(),Json::String(key.clone()));}fields.insert("repeat".into(),Json::Bool(event.is_held));modifiers(&mut fields,event.keystroke.modifiers);
    let mut host_fields=None;
    let target=KEY_TARGET.with(|target|target.borrow().as_ref().and_then(Weak::upgrade));let mut prevented=false;
    for owner in owners_in(window){
        if !matches!(key.as_deref(),Some("Enter"|" ")){owner.borrow_mut().suppress_click=false;}
        if has_listener(&owner,"host:keydown",true,false){prevented|=dispatch(&owner,"host:keydown",true,host_fields.get_or_insert_with(||press_fields("host:keydown",&fields)).clone());}
        prevented|=dispatch(&owner,"key.down",true,fields.clone());
        if !ready(&owner)||!owner.borrow().press_global{continue}
        if let Some(key)=key.as_deref().filter(|key|matches!(*key,"Enter"|" ")){let start={let mut owner=owner.borrow_mut();let start=owner.press.global_key.is_none();if start{owner.press.global_key=Some(key.into());}owner.press.global_keyboard_click=true;start};if start{prevented|=dispatch(&owner,"press.start",true,press_fields("press.start",&fields));}prevented|=dispatch(&owner,"press.commit",true,press_fields("press.commit",&fields));}else{owner.borrow_mut().press.global_keyboard_click=false;}
    }
    if let Some(target)=&target{prevented|=dispatch_root_route(target,"key.down",fields.clone());}
    if prevented||window.default_prevented(){window.prevent_default();cx.stop_propagation();}
    else if !target.as_ref().is_some_and(text_is_composing)&&focus_key_navigation(event,window){cx.stop_propagation();}
    else if let Some(target)=target.filter(ready){
        if text_supported(&target){if text_key_down(&target,event,window,cx){window.prevent_default();cx.stop_propagation();}}
        else if let Some(key)=key.filter(|key|key=="Enter"||key==" ").filter(|_|trigger_child(&target).is_none()){
            let start={let mut target=target.borrow_mut();let start=target.press.key.is_none();if start{target.press.key=Some(key.clone());}target.suppress_click=true;start};if start&&dispatch_root_route(&target,"press.start",press_fields("press.start",&fields)){window.prevent_default();}let mut fields=event_fields("press.commit");fields.insert("key".into(),Json::String(key));fields.insert("repeat".into(),Json::Bool(event.is_held));modifiers(&mut fields,event.keystroke.modifiers);
            if dispatch_root_route(&target,"press.commit",fields){window.prevent_default();cx.stop_propagation();}
        }
    }
    drive_owners(window,cx);
}
pub fn global_key_up(event:&gpui::KeyUpEvent,window:&mut Window,cx:&mut App){
    if KEY_DISPATCHED.with(|dispatched|dispatched.replace(true)){return}
    let key=portable_key(&event.keystroke);let mut fields=event_fields("key.up");if let Some(key)=&key{fields.insert("key".into(),Json::String(key.clone()));}fields.insert("repeat".into(),Json::Bool(false));modifiers(&mut fields,event.keystroke.modifiers);
    let mut host_fields=None;
    let target=KEY_TARGET.with(|target|target.borrow().as_ref().and_then(Weak::upgrade));let mut prevented=false;
    let owners=owners_in(window);
    for owner in &owners{if has_listener(owner,"host:keyup",true,false){prevented|=dispatch(owner,"host:keyup",true,host_fields.get_or_insert_with(||press_fields("host:keyup",&fields)).clone());}prevented|=dispatch(owner,"key.up",true,fields.clone());let end={let mut owner=owner.borrow_mut();if owner.press.global_key.as_deref()==key.as_deref(){owner.press.global_key.take().is_some()}else{false}};if end{prevented|=dispatch(owner,"press.end",true,press_fields("press.end",&fields));}}
    if let Some(target)=&target{prevented|=dispatch_root_route(target,"key.up",fields.clone());}
    for owner in &owners{let end={let mut owner=owner.borrow_mut();if owner.press.key.as_deref()==key.as_deref(){owner.press.key.take().is_some()}else{false}};if end{prevented|=dispatch_root_route(owner,"press.end",press_fields("press.end",&fields));}}
    if prevented{window.prevent_default();cx.stop_propagation();}drive_owners(window,cx);
}

// These are scalar conversion primitives, not an evaluator for authored IR.
// StringToNumber accepts the ECMAScript numeric-string grammar; Rust's FromStr
// additionally accepts inf/infinity and has a different whitespace predicate.
fn ecma_numeric_space(value:char)->bool { matches!(value,
    '\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|
    '\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}') }
fn ecma_radix_number(text:&str,radix:u8,width:u32)->f64 {
    if text.is_empty(){return f64::NAN}
    let(mut bits,mut significand,mut guard,mut sticky)=(0usize,0u64,false,false);
    for byte in text.bytes(){
        let digit=match byte{b'0'..=b'9'=>byte-b'0',b'a'..=b'f'=>byte-b'a'+10,b'A'..=b'F'=>byte-b'A'+10,_=>return f64::NAN};
        if digit>=radix{return f64::NAN}
        for shift in (0..width).rev(){
            let bit=(digit>>shift)&1;
            if bits==0&&bit==0{continue}
            bits=(bits+1).min(1025);
            if bits<=53{significand=(significand<<1)|u64::from(bit)}
            else if bits==54{guard=bit!=0}else{sticky|=bit!=0}
        }
    }
    if bits<=53{return significand as f64}
    if bits>1024{return f64::INFINITY}
    // Binary/octal/hexadecimal are powers of two. Round the exact integer once,
    // including halfway cases, instead of repeatedly rounding an f64 accumulator.
    if guard&&(sticky||significand&1!=0){significand+=1}
    (significand as f64)*2f64.powi((bits-53)as i32)
}
fn ecma_string_number(text:&str)->f64 {
    let text=text.trim_matches(ecma_numeric_space);
    if text.is_empty(){return 0.}
    match text{"Infinity"|"+Infinity"=>return f64::INFINITY,"-Infinity"=>return f64::NEG_INFINITY,_=>{}}
    if let Some(digits)=text.strip_prefix("0x").or_else(||text.strip_prefix("0X")){return ecma_radix_number(digits,16,4)}
    if let Some(digits)=text.strip_prefix("0b").or_else(||text.strip_prefix("0B")){return ecma_radix_number(digits,2,1)}
    if let Some(digits)=text.strip_prefix("0o").or_else(||text.strip_prefix("0O")){return ecma_radix_number(digits,8,3)}
    let bytes=text.as_bytes();let mut index=0;
    if matches!(bytes.first(),Some(b'+'|b'-')){index+=1}
    let mut digits=0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit){index+=1;digits+=1}
    if bytes.get(index)==Some(&b'.'){
        index+=1;while bytes.get(index).is_some_and(u8::is_ascii_digit){index+=1;digits+=1}
    }
    if digits==0{return f64::NAN}
    if matches!(bytes.get(index),Some(b'e'|b'E')){
        index+=1;if matches!(bytes.get(index),Some(b'+'|b'-')){index+=1}
        let start=index;while bytes.get(index).is_some_and(u8::is_ascii_digit){index+=1}
        if index==start{return f64::NAN}
    }
    if index!=bytes.len(){return f64::NAN}
    text.parse().unwrap_or(f64::NAN)
}
fn ecma_number_text(value:f64)->String {
    if value.is_nan(){return "NaN".into()}
    if value==0.{return "0".into()}
    if value==f64::INFINITY{return "Infinity".into()}
    if value==f64::NEG_INFINITY{return "-Infinity".into()}
    // Use the existing JSON dependency's shortest correctly rounded digits.
    // Rust Display rounds decimal halfway cases away from zero (e.g. 2^-25),
    // whereas the source operator reference uses the even final decimal digit.
    // Only the digits are reused; apply Number::toString's own layout below.
    let shortest=serde_json::Number::from_f64(value.abs()).expect("finite native number").to_string();
    let (mantissa,exponent)=shortest.split_once('e').or_else(||shortest.split_once('E'))
        .map(|(digits,exponent)|(digits,exponent.parse::<i32>().expect("native float exponent")))
        .unwrap_or((&shortest,0));
    let decimal=mantissa.find('.').unwrap_or(mantissa.len())as i32+exponent;
    let all=mantissa.replace('.',"");let leading=all.bytes().take_while(|digit|*digit==b'0').count();
    let digits=all[leading..].trim_end_matches('0');let n=decimal-leading as i32;let k=digits.len()as i32;
    let text=if k<=n&&n<=21{format!("{digits}{}","0".repeat((n-k)as usize))}
        else if 0<n&&n<=21{format!("{}.{}",&digits[..n as usize],&digits[n as usize..])}
        else if -6<n&&n<=0{format!("0.{}{digits}","0".repeat((-n)as usize))}
        else{let fraction=if k==1{String::new()}else{format!(".{}",&digits[1..])};format!("{}{fraction}e{:+}",&digits[..1],n-1)};
    if value.is_sign_negative(){format!("-{text}")}else{text}
}
// Source descriptors are diagnostic/type-checking metadata, not key identity.
// Each generated Rust module owns one OnceLock token per declaration; two
// independent crates with the same source ID cannot accidentally share Context.
#[derive(Clone)]
pub struct NativeContextKey { token:std::sync::Arc<()>, descriptor:String }
impl PartialEq for NativeContextKey {fn eq(&self,other:&Self)->bool{std::sync::Arc::ptr_eq(&self.token,&other.token)}}
impl Eq for NativeContextKey {}
impl PartialOrd for NativeContextKey {fn partial_cmp(&self,other:&Self)->Option<std::cmp::Ordering>{Some(self.cmp(other))}}
impl Ord for NativeContextKey {fn cmp(&self,other:&Self)->std::cmp::Ordering{(std::sync::Arc::as_ptr(&self.token)as usize).cmp(&(std::sync::Arc::as_ptr(&other.token)as usize))}}
pub fn native_context_key(slot:&'static std::sync::OnceLock<NativeContextKey>,descriptor:&str)->Value {
    Value::ContextKey(slot.get_or_init(||NativeContextKey{token:std::sync::Arc::new(()),descriptor:descriptor.into()}).clone())
}
fn context_identity(key:Value)->NativeContextKey {match key{Value::ContextKey(key)=>key,_=>value_type_error("Expected a nominal ContextKey")}}
// A structural record type describes fields, not ownership. Cloning a Value
// preserves its allocation; only an explicit host snapshot copies the fields.
pub struct NativeRecord { pub fields:RefCell<BTreeMap<String,Value>>, frozen:Cell<bool> }
pub fn record_map(fields:BTreeMap<String,Value>)->Value {
    Value::Record(Rc::new(NativeRecord{fields:RefCell::new(fields),frozen:Cell::new(false)}))
}
pub fn freeze(value:Value)->Value { if let Value::Record(record)=&value{record.frozen.set(true);}value }
pub fn set_member(object:Value,key:&str,value:Value)->Value {
    let Value::Record(record)=object else { value_type_error("Property assignment requires an object") };
    if record.frozen.get(){value_type_error("Cannot assign to a frozen object")}
    record.fields.borrow_mut().insert(key.into(),value.clone());value
}
pub fn value_type_error(message:&str)->! {
    throw_value(record(vec![("name",Value::string("TypeError")),("message",Value::string(message))]))
}
pub fn value_error(message:&str)->! {
    throw_value(record(vec![("name",Value::string("Error")),("message",Value::string(message))]))
}
pub fn typeof_value(value:&Value)->Value { Value::string(match value {
    Value::Undefined=>"undefined",Value::Number(_)|Value::Json(Json::Number(_))=>"number",
    Value::Json(Json::Bool(_))=>"boolean",Value::Json(Json::String(_))|Value::CaughtStringUnit(_)=>"string",
    Value::Function(_)|Value::NativeFunction{..}|Value::Renderer(_)=>"function",_=>"object",
}) }
pub fn property_key(value:&Value)->String { match value {
    Value::Number(value) if *value==0.=>"0".into(),Value::Number(value) if *value==f64::INFINITY=>"Infinity".into(),
    Value::Number(value) if *value==f64::NEG_INFINITY=>"-Infinity".into(),_=>value.scalar_text(),
} }
impl Value { pub fn error_message(&self)->String{if matches!(self,Self::Undefined){String::new()}else{self.scalar_text()}} }
pub fn number_is_finite(value:&Value)->Value { Value::boolean(match value {
    Value::Number(value)=>value.is_finite(),Value::Json(Json::Number(value))=>value.as_f64().is_some_and(f64::is_finite),_=>false,
}) }
// ECMAScript's nearest integer ties toward positive infinity. Rust round()
// rounds ties away from zero; adding 0.5 first also loses adjacent-double cases.
pub fn math_round(value:Value)->Value {
    let value=value.num();
    let result=if !value.is_finite()||value==0.{value}
        else if value>=-0.5&&value<0.{-0.0}
        else {let floor=value.floor();if value-floor<0.5{floor}else{floor+1.}};
    Value::number(result)
}
pub fn has_own(value:&Value,key:&Value)->Value {
    let key=property_key(key);Value::boolean(match value {
        Value::Record(record)=>record.fields.borrow().contains_key(&key),
        Value::Json(Json::Object(record))=>record.contains_key(&key),
        Value::Array(items)=>key=="length"||key.parse::<usize>().ok().is_some_and(|index|index.to_string()==key&&index<items.len()),
        Value::Json(Json::String(text))=>key=="length"||key.parse::<usize>().ok().is_some_and(|index|index.to_string()==key&&index<text.encode_utf16().count()),
        Value::Undefined|Value::Json(Json::Null)=>value_type_error("Cannot inspect own properties of a nullish value"),_=>false,
    })
}
pub fn own_entries(value:Value)->BTreeMap<String,Value> { match value {
    Value::Record(record)=>record.fields.borrow().clone(),
    Value::Json(Json::Object(record))=>record.into_iter().map(|(key,value)|(key,Value::json(value))).collect(),
    Value::Array(items)=>items.iter().enumerate().map(|(index,value)|(index.to_string(),value.clone())).collect(),
    Value::Json(Json::String(text))=>text.encode_utf16().enumerate().map(|(index,value)|(index.to_string(),Value::CaughtStringUnit(value))).collect(),
    _=>BTreeMap::new(),
} }
pub fn array_every(value:Value,callback:Value)->Value {
    let values=value.items();let mut all=true;
    for(index,item)in values.into_iter().enumerate(){if !callback.call_authored(vec![item,Value::number(index as f64),value.clone()]).truthy(){all=false;break}}
    Value::boolean(all)
}
// Captured declarations are allocated before initializers. Reading a slot is
// delayed until execution, so forward and mutually recursive closures observe
// the same binding and the source temporal dead zone.
pub trait LexicalLifetime { fn clear(&self); }
struct LexicalStorage<T:Clone> { value:RefCell<Option<T>> }
impl<T:Clone> LexicalLifetime for LexicalStorage<T> { fn clear(&self){self.value.borrow_mut().take();} }
pub struct LexicalBinding<T:Clone> { storage:Rc<LexicalStorage<T>> }
impl<T:Clone> Clone for LexicalBinding<T> { fn clone(&self)->Self{Self{storage:self.storage.clone()}} }
impl<T:Clone> LexicalBinding<T> {
    pub fn initialize(&self,value:T){let mut current=self.storage.value.borrow_mut();assert!(current.is_none(),"lexical binding initialized twice");*current=Some(value);}
    pub fn read(&self)->T{self.storage.value.borrow().as_ref().cloned().unwrap_or_else(||throw_value(record(vec![("name",Value::string("ReferenceError")),("message",Value::string("Cannot access a binding before initialization"))])))}
}
thread_local!{static LEXICAL_OWNERS:RefCell<Vec<OwnerWeak>>=const{RefCell::new(Vec::new())};}
pub struct LexicalOwnerGuard;
pub fn enter_lexical_owner(owner:&OwnerWeak)->LexicalOwnerGuard{LEXICAL_OWNERS.with(|owners|owners.borrow_mut().push(owner.clone()));LexicalOwnerGuard}
impl Drop for LexicalOwnerGuard{fn drop(&mut self){LEXICAL_OWNERS.with(|owners|{owners.borrow_mut().pop();});}}
pub fn lexical_binding<T:Clone+'static>()->LexicalBinding<T> {
    let storage=Rc::new(LexicalStorage{value:RefCell::new(None)});
    let owner=LEXICAL_OWNERS.with(|owners|owners.borrow().last().cloned()).expect("captured lexical allocation requires its native invocation owner");
    let erased:Rc<dyn LexicalLifetime>=storage.clone();let owner=ensure(&owner);let mut owner=owner.borrow_mut();
    owner.lexical_slots.retain(|slot|slot.strong_count()>0);owner.lexical_slots.push(Rc::downgrade(&erased));
    LexicalBinding{storage}
}
fn clear_lexicals(owner:&OwnerRef){let slots=std::mem::take(&mut owner.borrow_mut().lexical_slots);for slot in slots{if let Some(slot)=slot.upgrade(){slot.clear();}}}

#[derive(Default)]
struct AuthoredHooks { entries:BTreeMap<String,Rc<RefCell<AuthoredEntry>>>, builtins:BTreeMap<String,(u64,Value)>, kinds:BTreeMap<String,bool>, frames:Vec<AuthoredFrame>, root_names:BTreeSet<String>, order:u64 }
struct AuthoredFrame { id:u64, names:BTreeSet<String>, states:BTreeMap<String,Value>, methods:BTreeMap<String,Value>, children:Vec<Value> }
struct AuthoredEntry { order:u64, mode:&'static str, api:Value, result:Value, handle:Value, handle_set:bool }
struct CaptureGuard { owner:OwnerWeak, open:bool }
impl Drop for CaptureGuard { fn drop(&mut self){if self.open{if let Some(owner)=self.owner.upgrade(){owner.borrow_mut().authored.frames.pop();}}} }
fn claim_state_name(owner:&OwnerRef,name:&str)->String {
    let mut owner=owner.borrow_mut();let authored=&mut owner.authored;
    let (names,id)=if let Some(frame)=authored.frames.last_mut(){(&mut frame.names,Some(frame.id))}else{(&mut authored.root_names,None)};
    if name.is_empty(){value_error("[State] state name must be a non-empty string.")}
    if !names.insert(name.into()){value_error(&format!("[State] duplicate state name in setup frame: {name}"))}
    id.map(|id|format!("hook:{id}:{name}")).unwrap_or_else(||name.into())
}
fn capture_state(owner:&OwnerRef,name:String,state:Rc<State>){if let Some(frame)=owner.borrow_mut().authored.frames.last_mut(){frame.states.insert(name,Value::BorrowedState(state));}}
fn capture_method(owner:&OwnerRef,name:String,method:Value){if let Some(frame)=owner.borrow_mut().authored.frames.last_mut(){frame.methods.insert(name,method);}}
fn record_hook_child(owner:&OwnerRef,name:&str,mode:&str,order:u64,privileged:bool,result:Value,handle:Value){
    if let Some(frame)=owner.borrow_mut().authored.frames.last_mut(){frame.children.push(freeze(record(vec![
        ("name",Value::string(name)),("order",Value::number(order as f64)),("privileged",Value::boolean(privileged)),
        ("mode",Value::string(mode)),("result",result),("handle",handle),
    ])));}
}
fn record_authored_child(owner:&OwnerRef,name:&str,mode:&str,entry:&AuthoredEntry)->Value{
    // A pending or failed first registration still owns the name. Core returns
    // a fresh truthy empty record while no result exists; it never retries the
    // losing installer. A projector that explicitly returned undefined is set.
    let result=if entry.result.nullish(){record(vec![])}else{entry.result.clone()};
    let handle=if entry.handle_set{entry.handle.clone()}else{result.clone()};
    record_hook_child(owner,name,mode,entry.order,false,result,handle.clone());handle
}
fn register_hook_kind(owner:&OwnerRef,name:&str,privileged:bool){
    let mut owner=owner.borrow_mut();
    if owner.authored.kinds.get(name).is_some_and(|previous|*previous!=privileged){throw_error("HOOK_KIND_CONFLICT","privileged and authored hooks cannot share a registration name")}
    owner.authored.kinds.insert(name.into(),privileged);
}
// Core's privileged wrapper records the facade result itself as both raw result
// and handle; it does not begin an authored effect-capture frame. Every reuse is
// still a child record and shares the owner's registration-order sequence.
pub fn native_builtin_hook(owner:&OwnerWeak,name:&str,install:impl FnOnce()->Value)->Value{
    let owner_ref=setup(owner);register_hook_kind(&owner_ref,name,true);
    let previous=owner_ref.borrow().authored.builtins.get(name).cloned();
    let (order,result)=if let Some(previous)=previous{previous}else{
        let order={let mut owner=owner_ref.borrow_mut();let order=owner.authored.order;owner.authored.order+=1;order};
        let result=install();owner_ref.borrow_mut().authored.builtins.insert(name.into(),(order,result.clone()));(order,result)
    };
    record_hook_child(&owner_ref,name,"once",order,true,result.clone(),result.clone());result
}
fn finish_capture(frame:AuthoredFrame,render:Value)->Value {
    let mut fields=BTreeMap::new();
    if !frame.states.is_empty(){let states=freeze(record_map(frame.states));fields.insert("stateHandles".into(),states.clone());fields.insert("getState".into(),Value::Function(Rc::new(move|args|states.member(&arg(&args,0).text(),false))));}
    if !frame.methods.is_empty(){let methods=frame.methods;fields.insert("getMethod".into(),Value::Function(Rc::new(move|args|methods.get(&arg(&args,0).text()).cloned().unwrap_or_default())));}
    if !frame.children.is_empty(){
        let children=Rc::new(frame.children);let raw=children.clone();
        fields.insert("getAsHook".into(),Value::Function(Rc::new(move|args|{let name=arg(&args,0);raw.iter().find(|child|strict_equal(&child.member("name",false),&name)).cloned().unwrap_or_default()})));
        fields.insert("getAsHookHandle".into(),Value::Function(Rc::new(move|args|{let name=arg(&args,0);children.iter().find(|child|strict_equal(&child.member("name",false),&name)).map(|child|child.member("handle",false)).unwrap_or_default()})));
    }
    if matches!(render,Value::Function(_)|Value::NativeFunction{..}|Value::Renderer(_)){fields.insert("render".into(),render);}
    freeze(record_map(fields))
}
pub fn authored_hook(owner:&OwnerWeak,name:&str,mode:&'static str,input:Value,install:Value,project:Value,configure:Value)->Value {
    let owner_ref=setup(owner);
    register_hook_kind(&owner_ref,name,false);
    let previous=if mode=="multiple"{None}else{owner_ref.borrow().authored.entries.get(name).cloned()};
    let warning_name=name.to_owned();let conflict_name=name.to_owned();
    let tools=freeze(record(vec![
        ("warn",Value::Function(Rc::new(move|args|{eprintln!("[Hook:{}] {}",warning_name,arg(&args,0).scalar_text());Value::Undefined}))),
        ("conflict",Value::Function(Rc::new(move|args|throw_error("HOOK_CONFLICT",&format!("[Hook:{}] {}",conflict_name,arg(&args,0).scalar_text()))))),
    ]));
    if let Some(previous)=previous{
        let (previous_mode,api)={let entry=previous.borrow();(entry.mode,entry.api.clone())};
        if previous_mode!="once"&&mode!="once"&&!configure.nullish(){configure.call_authored(vec![api,input,tools]);}
        return record_authored_child(&owner_ref,name,mode,&previous.borrow())
    }
    let order={let mut owner=owner_ref.borrow_mut();let order=owner.authored.order;owner.authored.order+=1;order};
    let api=freeze(record(vec![("name",Value::string(name)),("order",Value::number(order as f64)),("store",record(vec![]))]));
    let entry=Rc::new(RefCell::new(AuthoredEntry{order,mode,api:api.clone(),result:Value::Undefined,handle:Value::Undefined,handle_set:false}));
    {let mut owner=owner_ref.borrow_mut();if mode!="multiple"{owner.authored.entries.insert(name.into(),entry.clone());}
     owner.authored.frames.push(AuthoredFrame{id:order,names:BTreeSet::new(),states:BTreeMap::new(),methods:BTreeMap::new(),children:Vec::new()});}
    let mut guard=CaptureGuard{owner:owner.clone(),open:true};
    let render=install.call_authored(vec![capability(owner),input.clone(),api.clone()]);
    let frame=owner_ref.borrow_mut().authored.frames.pop().expect("capture frame");guard.open=false;
    let result=finish_capture(frame,render);
    // Capture succeeded even if projection subsequently throws. Preserve that
    // raw result for the already-owned registration, as the Core wrapper does.
    entry.borrow_mut().result=result.clone();
    let handle=if project.nullish(){result.clone()}else{project.call_authored(vec![result.clone()])};
    {let mut entry=entry.borrow_mut();entry.handle=handle.clone();entry.handle_set=true;}
    record_authored_child(&owner_ref,name,mode,&entry.borrow());
    if mode=="configurable"&&!configure.nullish(){configure.call_authored(vec![api,input,tools]);}
    handle
}
// Only outward callable trees enter callback scope. Raw capture getters retain
// authored setup methods. Weak owner-local projections preserve public aliases
// without retaining every temporary returned object for the owner's lifetime.
type WeakPublicCallback=Weak<dyn Fn(Vec<Value>)->Value>;
#[derive(Default)]
struct PublicValues {
    next_receiver:u64,
    records:BTreeMap<usize,(Weak<NativeRecord>,Weak<NativeRecord>,u64)>,
    arrays:BTreeMap<usize,(Weak<Vec<Value>>,Weak<Vec<Value>>,u64)>,
    functions:BTreeMap<(u64,usize),(WeakPublicCallback,WeakPublicCallback)>,
    projected_records:BTreeMap<usize,(Weak<NativeRecord>,u64)>,
    projected_arrays:BTreeMap<usize,(Weak<Vec<Value>>,u64)>,
    projected_functions:BTreeMap<usize,WeakPublicCallback>,
}
fn needs_public_projection(value:&Value,seen:&mut BTreeSet<usize>)->bool{match value{
    Value::Function(_)=>true,
    Value::Record(record)=>seen.insert(Rc::as_ptr(record)as usize)&&record.fields.borrow().values().any(|value|needs_public_projection(value,seen)),
    Value::Array(items)=>seen.insert(Rc::as_ptr(items)as usize)&&items.iter().any(|value|needs_public_projection(value,seen)),
    _=>false,
}}
fn outward_value(owner:&OwnerWeak,value:Value)->Value {
    fn project(owner:&OwnerWeak,value:Value,receiver:u64,projecting:&mut BTreeSet<usize>)->Value {match value{
        Value::Function(callback)=>{
            let id=Rc::as_ptr(&callback)as *const ()as usize;let owner_ref=ensure(owner);
            // Echoing an already public value through the same owner is an
            // identity operation. Keep the original lifetime/phase guard.
            if owner_ref.borrow().public_values.projected_functions.get(&id).and_then(Weak::upgrade).is_some_and(|known|Rc::ptr_eq(&known,&callback)){return Value::Function(callback)}
            if let Some(result)=owner_ref.borrow().public_values.functions.get(&(receiver,id)).and_then(|(_,value)|value.upgrade()){return Value::Function(result)}
            let weak=owner.clone();let raw=callback.clone();
            let wrapped:Callback=Rc::new(move|args|{let owner=ensure(&weak);let result=invoke(&owner,"expose-method",&Value::Function(raw.clone()),args);outward_value(&weak,result)});
            let mut owner=owner_ref.borrow_mut();let values=&mut owner.public_values;
            values.functions.retain(|_,(_,value)|value.strong_count()>0);
            values.projected_functions.retain(|_,value|value.strong_count()>0);
            values.projected_functions.insert(Rc::as_ptr(&wrapped)as *const ()as usize,Rc::downgrade(&wrapped));
            values.functions.insert((receiver,id),(Rc::downgrade(&callback),Rc::downgrade(&wrapped)));Value::Function(wrapped)
        },
        Value::Record(record)=>{
            let id=Rc::as_ptr(&record)as usize;let owner_ref=ensure(owner);
            let projected=owner_ref.borrow().public_values.projected_records.get(&id).filter(|(known,_)|known.upgrade().is_some_and(|known|Rc::ptr_eq(&known,&record))).cloned();
            let(result,receiver)=if let Some((_,receiver))=projected{(record.clone(),receiver)}else{
                let previous=owner_ref.borrow().public_values.records.get(&id).filter(|(source,_,_)|source.upgrade().is_some_and(|source|Rc::ptr_eq(&source,&record))).cloned();
                let result=previous.as_ref().and_then(|(_,value,_)|value.upgrade()).unwrap_or_else(||{let Value::Record(result)=record_map(BTreeMap::new())else{unreachable!()};result});
                let mut owner=owner_ref.borrow_mut();let values=&mut owner.public_values;
                let receiver=previous.map(|(_,_,id)|id).unwrap_or_else(||{values.next_receiver+=1;values.next_receiver});
                values.records.retain(|_,(source,value,_)|source.strong_count()>0||value.strong_count()>0);
                values.projected_records.retain(|_,(value,_)|value.strong_count()>0);
                values.projected_records.insert(Rc::as_ptr(&result)as usize,(Rc::downgrade(&result),receiver));
                values.records.insert(id,(Rc::downgrade(&record),Rc::downgrade(&result),receiver));(result,receiver)
            };
            if !projecting.insert(id){return Value::Record(result)}
            // A projected record stays the same object, but authored code may
            // have replaced a callable field since its previous public return.
            // Visit current fields in place; already guarded callables retain
            // identity while newly installed closures acquire this owner guard.
            let fields=record.fields.borrow().clone();let mut refreshed=BTreeMap::new();
            for(key,value)in fields{let value=if needs_public_projection(&value,&mut BTreeSet::new()){project(owner,value,receiver,projecting)}else{value};refreshed.insert(key,value);}
            *result.fields.borrow_mut()=refreshed;projecting.remove(&id);Value::Record(result)
        },
        Value::Array(items)=>{
            let id=Rc::as_ptr(&items)as usize;let owner_ref=ensure(owner);
            let projected=owner_ref.borrow().public_values.projected_arrays.get(&id).filter(|(known,_)|known.upgrade().is_some_and(|known|Rc::ptr_eq(&known,&items))).cloned();
            if let Some((_,receiver))=projected{
                if !projecting.insert(id){return Value::Array(items)}
                for value in items.iter(){if needs_public_projection(value,&mut BTreeSet::new()){project(owner,value.clone(),receiver,projecting);}}
                projecting.remove(&id);return Value::Array(items)
            }
            let previous=owner_ref.borrow().public_values.arrays.get(&id).filter(|(source,_,_)|source.upgrade().is_some_and(|source|Rc::ptr_eq(&source,&items))).cloned();
            let receiver={let mut owner=owner_ref.borrow_mut();previous.as_ref().map(|(_,_,id)|*id).unwrap_or_else(||{owner.public_values.next_receiver+=1;owner.public_values.next_receiver})};
            if let Some(result)=previous.as_ref().and_then(|(_,value,_)|value.upgrade()){
                if !projecting.insert(id){return Value::Array(result)}
                for value in items.iter(){if needs_public_projection(value,&mut BTreeSet::new()){project(owner,value.clone(),receiver,projecting);}}
                projecting.remove(&id);return Value::Array(result)
            }
            let result:Rc<Vec<Value>>=Rc::new(items.iter().map(|value|if needs_public_projection(value,&mut BTreeSet::new()){project(owner,value.clone(),receiver,projecting)}else{value.clone()}).collect());
            let mut owner=owner_ref.borrow_mut();owner.public_values.arrays.retain(|_,(source,value,_)|source.strong_count()>0||value.strong_count()>0);
            owner.public_values.projected_arrays.retain(|_,(value,_)|value.strong_count()>0);
            owner.public_values.projected_arrays.insert(Rc::as_ptr(&result)as usize,(Rc::downgrade(&result),receiver));
            owner.public_values.arrays.insert(id,(Rc::downgrade(&items),Rc::downgrade(&result),receiver));Value::Array(result)
        },
        other=>other,
    }}
    if needs_public_projection(&value,&mut BTreeSet::new()){project(owner,value,0,&mut BTreeSet::new())}else{value}
}
fn finish_public_exposes(owner:&OwnerRef){
    let raw=std::mem::take(&mut owner.borrow_mut().exposes);let weak=Rc::downgrade(owner);
    let wrapped=raw.into_iter().map(|(key,value)|(key,outward_value(&weak,value))).collect();owner.borrow_mut().exposes=wrapped;
}

#[derive(Default)]
struct NativeHostAtoms {
    scheduler_enabled:bool,metadata_enabled:bool,
    app:Option<gpui::AsyncApp>,window:Option<gpui::AnyWindowHandle>,meta:BTreeMap<String,Value>,
    delays:Vec<Rc<NativeDelay>>,presence:Vec<Rc<RefCell<NativePresence>>>,
}
struct NativeDelay { active:Cell<bool>, callback:RefCell<Option<Value>>, task:RefCell<Option<gpui::Task<()>>> }
struct NativePresence { reference:Value,writer:Value,frame:Value,binding:Value,pending:Value,callback:Value }
pub fn configure_native_atoms(owner:&OwnerRef,scheduler:bool,metadata:bool){let mut owner=owner.borrow_mut();owner.atoms.scheduler_enabled=scheduler;owner.atoms.metadata_enabled=metadata;}
pub fn refresh_native_atoms(owner:&OwnerRef,window:&Window,cx:&App){
    let mut owner=owner.borrow_mut();let atoms=&mut owner.atoms;
    if atoms.scheduler_enabled{atoms.app=Some(cx.to_async());atoms.window=Some(window.window_handle());}
    if !atoms.metadata_enabled{return}
    let motion=Value::string(if cx.reduce_motion(){"reduce"}else{"no-preference"});
    atoms.meta.insert("reducedMotion".into(),motion.clone());atoms.meta.insert("preference.reducedMotion".into(),motion);
    atoms.meta.insert("colorScheme".into(),Value::string(match window.appearance(){gpui::WindowAppearance::Dark|gpui::WindowAppearance::VibrantDark=>"dark",_=>"light"}));
    atoms.meta.insert("viewport.width".into(),Value::number(f32::from(window.viewport_size().width)as f64));
}
pub fn native_meta_get(owner:&OwnerWeak,key:Value)->Value {
    let owner=ensure(owner);let key=key.text();let owner=owner.borrow();owner.atoms.meta.get(&key).cloned().unwrap_or_else(||{
        if key.starts_with("preference.")||key.starts_with("styleSupport.")||key=="viewport.width"{Value::string("unknown")}else{Value::Undefined}
    })
}
fn cancel_delay(delay:&NativeDelay){delay.active.set(false);delay.callback.borrow_mut().take();delay.task.borrow_mut().take();}
pub fn schedule_delay(owner:&OwnerWeak,duration:Value,callback:Value)->Value {
    let owner_ref=runtime(owner);let duration=duration.num();
    if !duration.is_finite()||duration<0.{value_type_error("Delay duration must be finite and nonnegative")}
    if !matches!(callback,Value::Function(_)){value_type_error("Delay callback must be callable")}
    let (app,window)={let owner=owner_ref.borrow();(owner.atoms.app.clone().expect("native scheduler is not bound"),owner.atoms.window.expect("native scheduler window is not bound"))};
    let delay=Rc::new(NativeDelay{active:Cell::new(true),callback:RefCell::new(Some(callback)),task:RefCell::new(None)});
    let executor=app.background_executor().clone();let start=executor.now();let weak=owner.clone();let pending=delay.clone();
    // Bound every host timer, rather than overflowing Duration/Instant for a
    // large finite authored number. Zero still crosses the foreground scheduler.
    let task=app.spawn(async move|cx|{
        let mut remaining=duration;
        loop{
            executor.timer(std::time::Duration::from_secs_f64(remaining.min(86_400_000.)/1000.)).await;
            if !pending.active.get(){return}
            remaining=duration-executor.now().saturating_duration_since(start).as_secs_f64()*1000.;
            if remaining<=0.{break}
        }
        let failed=pending.clone();
        let result=window.update(cx,move|_,window,cx|{
            let Some(owner)=weak.upgrade().filter(|owner|!owner.borrow().disposed)else{cancel_delay(&pending);return};
            if !pending.active.replace(false){return}
            if let Some(task)=pending.task.borrow_mut().take(){task.detach();}
            let callback=pending.callback.borrow_mut().take();
            owner.borrow_mut().atoms.delays.retain(|delay|delay.active.get());
            refresh_native_atoms(&owner,window,cx);
            if let Some(callback)=callback{native_host_callback(&owner,"delay",||invoke(&owner,"delay",&callback,vec![]));}
            let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx);}
        });
        if result.is_err(){failed.active.set(false);failed.callback.borrow_mut().take();if let Some(task)=failed.task.borrow_mut().take(){task.detach();}}
    });
    *delay.task.borrow_mut()=Some(task);
    {let mut owner=owner_ref.borrow_mut();owner.atoms.delays.retain(|delay|delay.active.get());owner.atoms.delays.push(delay.clone());}
    freeze(record(vec![("cancel",Value::Function(Rc::new(move|_|{cancel_delay(&delay);Value::Undefined})))]))
}
fn own_fields(value:&Value,names:&[&str])->BTreeMap<String,Value>{
    let fields=match value{Value::Record(record)=>record.fields.borrow().clone(),_=>value_type_error("Presence requires own data fields")};
    if fields.len()!=names.len()||fields.keys().any(|key|!names.contains(&key.as_str())){value_type_error("Unexpected presence record shape")}
    fields
}
fn finite_presence(value:&Value,positive:bool)->f64 {
    let number=match value{Value::Number(number)=>*number,Value::Json(Json::Number(number))=>number.as_f64().unwrap(),_=>value_type_error("Presence duration must be a number")};
    if !number.is_finite()||if positive{number<=0.}else{number<0.}{value_type_error("Invalid finite presence duration")};number
}
fn integer_presence(value:&Value)->f64 {let value=finite_presence(value,false);if value.fract()!=0.||value>9_007_199_254_740_991.{value_type_error("Invalid presence revision")};value}
pub fn snapshot_completion(value:Value)->Value {
    if strict_equal(&value,&Value::string("timer")){return value}
    let fields=own_fields(&value,&["kind","timeoutMs"]);if !strict_equal(&field(&fields,"kind"),&Value::string("presentation")){value_type_error("Invalid completion kind")}
    freeze(record(vec![("kind",Value::string("presentation")),("timeoutMs",Value::number(finite_presence(&field(&fields,"timeoutMs"),true)))]))
}
fn snapshot_presence_frame(value:Value)->Value {
    let fields=own_fields(&value,&["owner","revision","phase","attempt","completion","durationMs","timeoutMs"]);
    let owner=field(&fields,"owner");if !matches!(owner,Value::Opaque(_)){value_type_error("Invalid presentation owner reference")}
    let phase=field(&fields,"phase").text();if !["closed","preparing","entering","entered","leaving"].contains(&phase.as_str()){value_type_error("Invalid presence phase")}
    let completion=field(&fields,"completion").text();if completion!="timer"&&completion!="presentation"{value_type_error("Invalid completion kind")}
    let timeout=field(&fields,"timeoutMs");let timeout=if matches!(timeout,Value::Json(Json::Null)){timeout}else{Value::number(finite_presence(&timeout,true))};
    if (completion=="timer")!=matches!(timeout,Value::Json(Json::Null)){value_type_error("Presence timeout disagrees with completion kind")}
    let attempt=field(&fields,"attempt");let attempt=if matches!(attempt,Value::Json(Json::Null)){attempt}else{
        let parts=own_fields(&attempt,&["ownerEpoch","phaseAttempt"]);freeze(record(vec![("ownerEpoch",Value::number(integer_presence(&field(&parts,"ownerEpoch")))),("phaseAttempt",Value::number(integer_presence(&field(&parts,"phaseAttempt"))))]))
    };
    if matches!(phase.as_str(),"entering"|"leaving")==matches!(attempt,Value::Json(Json::Null)){value_type_error("Presence phase and attempt disagree")}
    freeze(record(vec![("owner",owner),("revision",Value::number(integer_presence(&field(&fields,"revision")))),("phase",Value::string(&phase)),("attempt",attempt),
        ("completion",Value::string(&completion)),("durationMs",Value::number(finite_presence(&field(&fields,"durationMs"),false))),("timeoutMs",timeout)]))
}
fn find_presence(owner:&OwnerRef,writer:&Value)->Option<Rc<RefCell<NativePresence>>>{owner.borrow().atoms.presence.iter().find(|entry|strict_equal(&entry.borrow().writer,writer)).cloned()}
fn require_presence(owner:&OwnerWeak,writer:&Value)->Rc<RefCell<NativePresence>>{let owner=ensure(owner);find_presence(&owner,writer).unwrap_or_else(||value_type_error("Foreign presentation writer"))}
pub fn presence_owner(owner:&OwnerWeak)->Value {
    let owner_ref=setup(owner);let reference=Value::Opaque(Rc::new(()));let writer=Value::Opaque(Rc::new(()));
    let state=Rc::new(RefCell::new(NativePresence{reference:reference.clone(),writer:writer.clone(),frame:Value::Undefined,binding:Value::Undefined,pending:Value::Undefined,callback:Value::Undefined}));
    owner_ref.borrow_mut().atoms.presence.push(state.clone());let weak=owner.clone();
    freeze(record(vec![("ref",reference),("writer",writer),("onCompletionRequest",Value::Function(Rc::new(move|args|{
        setup(&weak);let callback=arg(&args,0);if !matches!(callback,Value::Function(_))||!state.borrow().callback.nullish(){value_type_error("Exactly one presence completion receiver is required")}
        state.borrow_mut().callback=callback;Value::Undefined
    })))]))
}
pub fn presence_publish(owner:&OwnerWeak,writer:Value,value:Value)->Value{
    runtime(owner);let state=require_presence(owner,&writer);let frame=snapshot_presence_frame(value);let mut state=state.borrow_mut();
    if !strict_equal(&state.reference,&frame.member("owner",false))||!state.frame.nullish()&&frame.member("revision",false).num()<=state.frame.member("revision",false).num(){value_type_error("Foreign owner or nonmonotonic presence revision")}
    state.pending=Value::Undefined;state.frame=frame;Value::Undefined
}
pub fn presence_is_current(owner:&OwnerWeak,writer:Value,receipt:Value)->Value{
    let Some(owner)=owner.upgrade().filter(|owner|!owner.borrow().disposed)else{return Value::boolean(false)};
    let Some(state)=find_presence(&owner,&writer)else{return Value::boolean(false)};let state=state.borrow();
    if state.pending.nullish()||!strict_equal(&state.pending,&receipt)||state.frame.nullish(){return Value::boolean(false)}
    let attempt=state.frame.member("attempt",false);if attempt.nullish(){return Value::boolean(false)}
    let receipt_attempt=receipt.member("attempt",true);
    Value::boolean(strict_equal(&state.reference,&receipt.member("owner",true))&&strict_equal(&state.binding,&receipt.member("viewBinding",true))
        &&strict_equal(&state.frame.member("revision",false),&receipt.member("ownerRevision",true))&&strict_equal(&receipt.member("targetRevision",true),&Value::number(0.))
        &&strict_equal(&attempt.member("ownerEpoch",false),&receipt_attempt.member("ownerEpoch",true))&&strict_equal(&attempt.member("phaseAttempt",false),&receipt_attempt.member("phaseAttempt",true)))
}
pub fn presence_settle(owner:&OwnerWeak,writer:Value,outcome:Value)->Value{
    let owner_ref=runtime(owner);let state=require_presence(owner,&writer);
    let (frame,pending)={let state=state.borrow();(state.frame.clone(),state.pending.clone())};
    if frame.nullish()||frame.member("attempt",false).nullish()||!strict_equal(&frame.member("completion",false),&Value::string("presentation"))||presence_is_current(owner,writer.clone(),pending).bool(){return Value::Undefined}
    let (receipt,callback)={let mut state=state.borrow_mut();if state.binding.nullish(){state.binding=Value::Opaque(Rc::new(()));}
        let outcome=if strict_equal(&outcome,&Value::string("finished")){Value::string("missing-capability")}else{outcome};
        let receipt=freeze(record(vec![("owner",state.reference.clone()),("attempt",frame.member("attempt",false)),("viewBinding",state.binding.clone()),
            ("ownerRevision",frame.member("revision",false)),("targetRevision",Value::number(0.)),("channels",Value::Array(Rc::new(vec![]))),("outcome",outcome)]));
        state.pending=receipt.clone();(receipt,state.callback.clone())};
    if !callback.nullish()&&presence_is_current(owner,writer,receipt.clone()).bool(){invoke(&owner_ref,"presence-completion",&callback,vec![capability(owner),freeze(record(vec![("receipt",receipt)]))]);}Value::Undefined
}
fn atoms_detach(owner:&OwnerRef){
    let delays=std::mem::take(&mut owner.borrow_mut().atoms.delays);for delay in delays{cancel_delay(&delay);}
    let states=owner.borrow().atoms.presence.clone();for state in states{let mut state=state.borrow_mut();state.binding=Value::Undefined;state.pending=Value::Undefined;}
}

pub struct StaticDeclaration { pub id:&'static str, pub kind:&'static str, pub config:fn()->Value }
// A named Rust static is the declaration identity; its source ID is metadata.
impl StaticDeclaration { pub fn identity(&'static self)->usize{self as *const Self as usize} }
impl Default for Value { fn default()->Self{Self::Undefined} }
#[derive(Default)]
pub struct Modules {
    pub boundary:BoundaryModule,pub hit:HitModule,pub scroll:ScrollModule,
    pub topology:Topology,pub hooks:BTreeSet<&'static str>,pub focus:FocusModule,pub state_watchers:Vec<NativeStateWatch>,
    pub text:TextModule,pub image:ImageModule,pub positioning:PositioningModule,pub overlay:OverlayModule,
    pub accessible_text:BTreeMap<String,Value>,pub accessible_relations:BTreeMap<String,Value>,
    pub accessible_tree:Value,pub accessible_level:Value,pub action_events:BTreeMap<String,String>,pub a11y_node:Option<gpui::accesskit::NodeId>,
    pub a11y_frame:u64,pub a11y_bindings:usize,
    pub trigger_child:OwnerWeak,
}
#[derive(Default)]
pub struct FocusModule {
    pub config:BTreeMap<String,Value>,pub entry:BTreeMap<String,Value>,pub scope:BTreeMap<String,Value>,pub roving:BTreeMap<String,Value>,
    pub entry_disabled:bool,pub scope_active:bool,pub visible:bool,pub selected:bool,pub active:bool,
    pub module_active:bool,pub has_focused:bool,pub bypass_gate:bool,
    pub roving_declared:bool,
    pub reason:&'static str,pub prevent_scroll:bool,pub blur_requested:bool,pub previous:OwnerWeak,pub remembered:OwnerWeak,
    pub pending:Option<(&'static str,Value)>,pub native:Option<FocusHandle>,
    pub intent:u64,pub scope_activation:u64,pub scope_intent:u64,pub scope_recipient:Option<(OwnerWeak,u64)>,
    pub entry_intent:Option<FocusEntryIntent>,pub request_source:Option<(OwnerWeak,u64)>,
    entry_projection:Option<FocusEntryProjection>,entry_self_focused:bool,
}
pub struct FocusEntryIntent {
    id:u64,activation:Option<u64>,reason:&'static str,prevent_scroll:bool,attempted:Option<FocusEntryBinding>,
}
#[derive(Clone,Copy,PartialEq,Eq)]
struct FocusEntryBinding { epoch:u64,window:gpui::WindowId,generation:u64 }
struct FocusEntryProjection { binding:FocusEntryBinding,resolved:bool,self_fallback:bool }
thread_local!{
    static ACTIVE_SCOPES:RefCell<Vec<OwnerWeak>>=RefCell::new(Vec::new());
    static ENTRY_INTENTS:RefCell<Vec<(OwnerWeak,u64)>>=RefCell::new(Vec::new());
    static FOCUS_INTENT_EPOCH:Cell<u64>=Cell::new(0);
}
pub fn all_owners()->Vec<OwnerRef>{OWNERS.with(|all|{let mut all=all.borrow_mut();all.retain(|v|v.strong_count()>0);all.iter().filter_map(Weak::upgrade).filter(|o|!o.borrow().disposed).collect()})}
pub fn descendant(owner:&OwnerRef,ancestor:&OwnerRef)->bool{let mut current=Some(owner.clone());let mut visited=BTreeSet::new();while let Some(node)=current{assert!(visited.insert(Rc::as_ptr(&node)as usize),"cyclic logical ancestry");if Rc::ptr_eq(&node,ancestor){return true}current=node.borrow().parent.upgrade();}false}
pub fn ready(owner:&OwnerRef)->bool{let o=owner.borrow();!o.disposed&&o.mounted&&o.present&&o.bounds.is_some()}
fn field(config:&BTreeMap<String,Value>,key:&str)->Value{config.get(key).cloned().unwrap_or(Value::Undefined)}
fn text_or(value:Value,fallback:&str)->String{if value.nullish(){fallback.into()}else{value.text()}}
fn bool_or(value:Value,fallback:bool)->bool{if value.nullish(){fallback}else{value.bool()}}
fn merge_config(config:&mut BTreeMap<String,Value>,patch:Value){let Value::Record(entries)=patch else{panic!("module configuration record required")};config.extend(entries.fields.borrow().iter().map(|(k,v)|(k.clone(),v.clone())));}
pub fn resolved(value:&Value)->Value{match value{Value::State(_)|Value::BorrowedState(_)|Value::Observed(_,_)=>value.get(),_=>value.clone()}}
pub fn module_handle(owner:&OwnerWeak,kind:&'static str)->Value{setup(owner).borrow_mut().modules.hooks.insert(kind);Value::Handle(owner.clone(),kind)}
pub fn capability_member(owner:&OwnerWeak,kind:&str,key:&str)->Value{
    ensure(owner);
    if kind=="collection"||kind=="collection-item"{return ensure(owner).borrow().modules.topology.handles.get(key).expect("unknown Collection state").clone()}
    if kind=="table-structure"{return match key{"role"=>Value::string(&ensure(owner).borrow().modules.topology.table.role),"states"=>Value::Handle(owner.clone(),"table-states"),_=>panic!("unknown Table capability member")}}
    if kind=="table-states"{return ensure(owner).borrow().modules.topology.handles.get(&format!("table.{key}")).expect("unknown Table state").clone()}
    if kind=="scroll"{match key{"horizontal"=>return Value::Handle(owner.clone(),"scroll-horizontal"),"vertical"=>return Value::Handle(owner.clone(),"scroll-vertical"),"endFollow"=>return Value::Handle(owner.clone(),"scroll-follow"),_=>{}}}
    if kind=="overlay"&&key=="open"{return Value::Observed(owner.clone(),"overlay.open")}
    let observation=match(kind,key){
        ("focus","focused")=>"focus.focused",("focus","focusVisible")=>"focus.visible",("focus","focusable")=>"focus.ready",
        ("focus-scope","active")=>"scope.active",("focus-roving","active")=>"roving.active",
        ("focus-scope","hasFocused")=>"scope.hasFocused",("focus-roving","hasFocused")=>"roving.hasFocused",
        ("scroll","axes")=>"scroll.axes",("scroll","projection")=>"scroll.projection",("scroll","scrolling")=>"scroll.scrolling",
        ("scroll-horizontal","position")=>"horizontal.position",("scroll-horizontal","visibleRatio")=>"horizontal.visibleRatio",("scroll-horizontal","canScrollBefore")=>"horizontal.canScrollBefore",("scroll-horizontal","canScrollAfter")=>"horizontal.canScrollAfter",("scroll-horizontal","atEnd")=>"horizontal.atEnd",
        ("scroll-vertical","position")=>"vertical.position",("scroll-vertical","visibleRatio")=>"vertical.visibleRatio",("scroll-vertical","canScrollBefore")=>"vertical.canScrollBefore",("scroll-vertical","canScrollAfter")=>"vertical.canScrollAfter",("scroll-vertical","atEnd")=>"vertical.atEnd",
        ("scroll-follow","state")=>"endFollow.state",("scroll-follow","requestStatus")=>"endFollow.requestStatus",
        _=>panic!("unmapped native capability member {kind}.{key}"),
    };Value::Observed(owner.clone(),observation)
}
pub fn observed(owner:&OwnerWeak,key:&str)->Value{
    if let Some((section,field))=key.split_once('.') {if ["scroll","horizontal","vertical","endFollow"].contains(&section){let snapshot=scroll_snapshot(owner);return if section=="scroll"{snapshot.member(field,false)}else{snapshot.member(section,false).member(field,false)}}}
    let owner=ensure(owner);
    match key{
        "focus.focused"=>Value::boolean(owner.borrow().focused),
        "overlay.open"=>Value::boolean(owner.borrow().modules.overlay.open),
        "focus.visible"=>Value::boolean(owner.borrow().modules.focus.visible),
        "focus.ready"=>Value::boolean(owner.borrow().focusable&&!owner.borrow().focus_disabled),
        "scope.active"=>Value::boolean(owner.borrow().modules.focus.scope_active),
        "roving.active"=>Value::boolean(owner.borrow().modules.focus.module_active),
        "scope.hasFocused"|"roving.hasFocused"=>Value::boolean(owner.borrow().modules.focus.has_focused),
        _=>panic!("unmapped native observation {key}"),
    }
}
pub fn focus_setup(owner:&OwnerWeak,kind:&'static str)->Value{
    assert!(kind!="focus-entry"||cfg!(feature="experimental-focus-tree"),"published GPUI SDK lacks the physical FocusEntry API");let handle=module_handle(owner,kind);if kind=="focus-roving"{ensure(owner).borrow_mut().modules.focus.roving_declared=true;}handle}
pub fn native_focus_available(owner:&OwnerRef)->bool{let owner=owner.borrow();owner.focusable&&!owner.focus_disabled||owner.modules.hooks.contains("focus-entry")&&!owner.modules.focus.entry_disabled}
pub fn native_tab_participation(owner:&OwnerRef)->bool{let owner=owner.borrow();owner.focusable&&!owner.focus_disabled&&owner.focus_entry||owner.modules.hooks.contains("focus-entry")&&!owner.modules.focus.entry_disabled&&owner.modules.focus.entry_projection.as_ref().is_some_and(|projection|projection.self_fallback&&projection.binding.epoch==owner.epoch&&Some(projection.binding.window)==owner.window_id)}
pub fn focus_setup_config(owner:&OwnerWeak,kind:&str,patch:Value)->Value{
    assert!(cfg!(feature="experimental-focus-tree")||kind!="focus-scope"||!strict_equal(&patch.member("entryFallback",true),&Value::string("entry")),"published GPUI SDK lacks physical Scope entry fallback");
    let owner=setup(owner);
    {let mut o=owner.borrow_mut();match kind{
        "focus"=>{let disabled=patch.member("disabled",false);if !disabled.nullish(){o.focus_disabled=disabled.bool();}let auto=patch.member("autoFocus",false);if !auto.nullish()&&auto.bool(){o.focus_requested=true;}let nav=patch.member("navParticipation",false);if !nav.nullish(){o.focus_entry=nav.text()!="none";}merge_config(&mut o.modules.focus.config,patch);},
        "focus-entry"=>{let disabled=patch.member("disabled",false);if !disabled.nullish(){o.modules.focus.entry_disabled=disabled.bool();}merge_config(&mut o.modules.focus.entry,patch);},
        "focus-scope"=>{let group=patch.member("group",false);if !group.nullish()&&!matches!(group,Value::Json(Json::Bool(false))){o.modules.focus.roving_declared=true;if matches!(group,Value::Record(_)){merge_config(&mut o.modules.focus.roving,group);}}merge_config(&mut o.modules.focus.scope,patch);},
        "focus-roving"=>merge_config(&mut o.modules.focus.roving,patch),
        _=>panic!("invalid focus configuration capability"),
    }}Value::Undefined
}
pub fn focus_entry_disabled(owner:&OwnerWeak,value:Value)->Value{let owner=runtime(owner);let disabled=value.bool();{let mut o=owner.borrow_mut();o.modules.focus.entry_disabled=disabled;o.dirty=true;}if disabled{retire_entry_intent(&owner);}Value::Undefined}
pub fn focus_nav(owner:&OwnerWeak,value:Value)->Value{let owner=runtime(owner);let text=value.text();assert!(text=="auto"||text=="none","invalid navigation participation");let mut o=owner.borrow_mut();o.focus_entry=text=="auto";o.modules.focus.config.insert("navParticipation".into(),value);o.dirty=true;Value::Undefined}
pub fn focus_status(owner:&OwnerWeak,value:Value)->Value{let owner=runtime(owner);let selected=value.member("selected",false);let active=value.member("active",false);let mut o=owner.borrow_mut();if !selected.nullish(){o.modules.focus.selected=selected.bool();}if !active.nullish(){o.modules.focus.active=active.bool();}o.dirty=true;Value::Undefined}
pub fn focus_set_roving(owner:&OwnerWeak,key:&str,value:Value)->Value{let owner=runtime(owner);if key=="loop"{value.bool();}else{assert!(["both","horizontal","vertical"].contains(&value.text().as_str()),"invalid roving orientation");}owner.borrow_mut().modules.focus.roving.insert(key.into(),value);Value::Undefined}
fn focus_provider(owner:&OwnerRef)->Option<OwnerRef>{
    let key=field(&owner.borrow().modules.focus.config,"groupKey");let mut current=owner.borrow().parent.upgrade();
    while let Some(candidate)=current{let o=candidate.borrow();let matches=o.modules.focus.roving_declared&&(key.nullish()||strict_equal(&field(&o.modules.focus.roving,"key"),&key));current=o.parent.upgrade();drop(o);if matches{return Some(candidate)}}None
}
pub fn focus_members(owner:&OwnerRef,roving:bool)->Vec<OwnerRef>{
    focus_members_for(owner,roving,false)
}
fn focus_members_for(owner:&OwnerRef,roving:bool,logical:bool)->Vec<OwnerRef>{
    let window=owner.borrow().window_id;
    // Scope entry/traversal uses natural navigation members; roving and explicit
    // requests still admit programmatic-only targets (C-AS-FOCUSABLE-0001-F).
    let mut members:Vec<_>=all_owners().into_iter().filter(|candidate|{let c=candidate.borrow();let eligible=!Rc::ptr_eq(candidate,owner)&&c.focusable&&!c.focus_disabled&&(roving||c.focus_entry)&&(logical||c.present)&&(window.is_none()||c.window_id.is_none()||c.window_id==window);drop(c);eligible&&if roving{focus_provider(candidate).is_some_and(|p|Rc::ptr_eq(&p,owner))}else{let scope_key=field(&owner.borrow().modules.focus.scope,"key");let member_key=field(&candidate.borrow().modules.focus.config,"scopeKey");descendant(candidate,owner)&&(member_key.nullish()||strict_equal(&member_key,&scope_key))}}).collect();
    members.sort_by_key(|member|member.borrow().modules.topology.order);members
}
fn top_scope_in(window:Option<gpui::WindowId>)->Option<OwnerRef>{ACTIVE_SCOPES.with(|scopes|{let mut scopes=scopes.borrow_mut();scopes.retain(|s|s.upgrade().is_some_and(|s|!s.borrow().disposed&&s.borrow().modules.focus.scope_active));scopes.iter().rev().filter_map(Weak::upgrade).find(|s|window.is_none()||s.borrow().window_id.is_none()||s.borrow().window_id==window)})}
fn top_scope(owner:&OwnerRef)->Option<OwnerRef>{top_scope_in(owner.borrow().window_id)}
fn focus_window(owner:&OwnerRef)->Option<gpui::WindowId>{let mut current=Some(owner.clone());while let Some(owner)=current{let o=owner.borrow();if o.window_id.is_some(){return o.window_id}current=o.parent.upgrade();}None}
fn same_focus_window(left:Option<gpui::WindowId>,right:Option<gpui::WindowId>)->bool{left.is_none()||right.is_none()||left==right}
fn focus_intent_epoch(owner:&OwnerRef)->u64{let window=focus_window(owner);OWNERS.with(|owners|owners.borrow().iter().filter_map(Weak::upgrade).filter(|candidate|!candidate.borrow().disposed&&same_focus_window(window,focus_window(candidate))).map(|candidate|candidate.borrow().modules.focus.intent).max().unwrap_or(0))}
fn pending_focus_since(owner:&OwnerRef,after:u64)->bool{let window=focus_window(owner);OWNERS.with(|owners|owners.borrow().iter().filter_map(Weak::upgrade).any(|candidate|{if !same_focus_window(window,focus_window(&candidate)){return false}let o=candidate.borrow();!o.disposed&&o.modules.focus.intent>after&&(o.focus_requested||o.modules.focus.entry_intent.is_some())}))}
fn clear_focus_recipient(owner:&OwnerRef,id:u64,recipient:OwnerWeak){
    if let Some(recipient)=recipient.upgrade(){let weak=Rc::downgrade(owner);let mut target=recipient.borrow_mut();if target.modules.focus.request_source.as_ref().is_some_and(|(source,intent)|Weak::ptr_eq(source,&weak)&&*intent==id){target.modules.focus.request_source=None;target.focus_requested=false;target.modules.focus.bypass_gate=false;target.dirty=true;}}
}
fn clear_scope_recipient(owner:&OwnerRef){let recipient=owner.borrow_mut().modules.focus.scope_recipient.take();if let Some((recipient,id))=recipient{clear_focus_recipient(owner,id,recipient);}}
// Detachment withdraws only physical readiness. A retained activation may resolve
// again in its new view; it can never carry a FocusHandle across that boundary.
fn withdraw_entry_view(owner:&OwnerRef){let mut o=owner.borrow_mut();o.modules.focus.entry_projection=None;o.modules.focus.entry_self_focused=false;if let Some(intent)=o.modules.focus.entry_intent.as_mut(){intent.attempted=None;}}
fn retire_entry_intent(owner:&OwnerRef){
    let pending=owner.borrow_mut().modules.focus.entry_intent.take();let Some(pending)=pending else{return};let weak=Rc::downgrade(owner);
    ENTRY_INTENTS.with(|entries|entries.borrow_mut().retain(|(candidate,id)|!Weak::ptr_eq(candidate,&weak)||*id!=pending.id));
}
fn retire_scope_entry(owner:&OwnerRef){if owner.borrow().modules.focus.entry_intent.as_ref().is_some_and(|intent|intent.activation.is_some()){retire_entry_intent(owner);}}
fn note_focus_intent_in(owner:&OwnerRef,window:Option<gpui::WindowId>)->u64{
    let id=FOCUS_INTENT_EPOCH.with(|epoch|{let id=epoch.get().wrapping_add(1);epoch.set(id);id});owner.borrow_mut().modules.focus.intent=id;
    ACTIVE_SCOPES.with(|scopes|{for scope in scopes.borrow().iter().filter_map(Weak::upgrade){if same_focus_window(window,focus_window(&scope)){clear_scope_recipient(&scope);}}});
    loop{let previous=ENTRY_INTENTS.with(|entries|{let mut entries=entries.borrow_mut();let index=entries.iter().position(|(source,_)|source.upgrade().map_or(true,|source|same_focus_window(window,focus_window(&source))));index.map(|index|entries.swap_remove(index))});let Some((source,_))=previous else{break};if let Some(source)=source.upgrade(){retire_entry_intent(&source);}}id
}
fn note_focus_intent(owner:&OwnerRef)->u64{note_focus_intent_in(owner,focus_window(owner))}
fn focus_request_reason(options:&Value)->&'static str{
    let reason=options.member("reason",true);
    match reason{Value::Json(Json::String(reason)) if reason=="keyboard"=>"keyboard",Value::Json(Json::String(reason)) if reason=="pointer"=>"pointer",_=>"programmatic"}
}
fn entry_policy(owner:&OwnerRef)->(bool,bool){let o=owner.borrow();(matches!(o.modules.focus.entry.get("strategy"),Some(Value::Json(Json::String(strategy))) if strategy=="descendant-first"),!matches!(o.modules.focus.entry.get("fallback"),Some(Value::Json(Json::String(fallback))) if fallback=="none"))}
#[cfg(feature="experimental-focus-tree")]
fn native_entry_target(owner:&OwnerRef,focus:&FocusHandle,window:&Window)->Option<FocusHandle>{
    let (descendants,fallback)=entry_policy(owner);
    match window.resolve_focus_entry(focus){
        gpui::FocusEntryResolution::Unavailable=>None,
        gpui::FocusEntryResolution::Descendant(target) if descendants=>Some(target),
        gpui::FocusEntryResolution::Descendant(_)|gpui::FocusEntryResolution::Empty if fallback=>Some(focus.clone()),
        _=>None,
    }
}
fn project_native_entry(owner:&OwnerRef,binding:FocusEntryBinding,target:Option<&FocusHandle>,focus:&FocusHandle){
    let self_fallback=target.is_some_and(|target|target==focus);
    let mut o=owner.borrow_mut();let previous=o.modules.focus.entry_projection.as_ref().is_some_and(|projection|projection.self_fallback);o.modules.focus.entry_projection=Some(FocusEntryProjection{binding,resolved:target.is_some(),self_fallback});if previous!=self_fallback{o.dirty=true;}
}
// Refresh before authored callbacks at a real native boundary, including the
// first mounted callback. The cache only admits independent Entry's first
// request; application always queries again and never retains this handle.
#[cfg(feature="experimental-focus-tree")]
pub fn refresh_native_entry(owner:&OwnerRef,window:&Window){
    let binding={let o=owner.borrow();if o.disposed||!o.present||o.bounds.is_none()||o.window_id!=Some(window.window_handle().window_id())||!o.modules.hooks.contains("focus-entry"){return}FocusEntryBinding{epoch:o.epoch,window:window.window_handle().window_id(),generation:window.focus_tree_generation()}};
    if owner.borrow().modules.focus.entry_projection.as_ref().is_some_and(|projection|projection.binding==binding){return}
    let focus=owner.borrow().modules.focus.native.clone();let Some(focus)=focus else{return};
    let target=native_entry_target(owner,&focus,window);project_native_entry(owner,binding,target.as_ref(),&focus);
}
fn entry_was_resolved(owner:&OwnerRef)->bool{let o=owner.borrow();!o.disposed&&o.present&&o.mounted&&o.bounds.is_some()&&!o.modules.focus.entry_disabled&&o.modules.focus.entry_projection.as_ref().is_some_and(|projection|projection.resolved&&projection.binding.epoch==o.epoch&&Some(projection.binding.window)==o.window_id)}
fn retain_entry_intent(owner:&OwnerRef,activation:Option<u64>,reason:&'static str,prevent_scroll:bool){
    let id=note_focus_intent(owner);let mut o=owner.borrow_mut();o.modules.focus.reason=reason;o.modules.focus.prevent_scroll=prevent_scroll;o.modules.focus.entry_intent=Some(FocusEntryIntent{id,activation,reason,prevent_scroll,attempted:None});o.dirty=true;drop(o);
    ENTRY_INTENTS.with(|entries|entries.borrow_mut().push((Rc::downgrade(owner),id)));
}
fn entry_intent_current(owner:&OwnerRef,id:u64,binding:FocusEntryBinding)->bool{
    let o=owner.borrow();!o.disposed&&o.present&&o.mounted&&o.bounds.is_some()&&!o.modules.focus.entry_disabled&&o.epoch==binding.epoch&&o.window_id==Some(binding.window)&&o.modules.focus.intent==id&&o.modules.focus.entry_intent.as_ref().is_some_and(|intent|intent.id==id&&intent.activation.map_or(true,|activation|o.modules.focus.scope_activation==activation))
}
#[cfg(feature="experimental-focus-tree")]
fn drive_entry_intent(owner:&OwnerRef,focus:&FocusHandle,window:&mut Window,cx:&mut App){
    let window_id=window.window_handle().window_id();
    if owner.borrow().window_id!=Some(window_id){return}
    if !ready(owner){withdraw_entry_view(owner);return}
    if !owner.borrow().modules.hooks.contains("focus-entry"){return}
    let binding=FocusEntryBinding{epoch:owner.borrow().epoch,window:window_id,generation:window.focus_tree_generation()};
    let request={let o=owner.borrow();o.modules.focus.entry_intent.as_ref().map(|intent|(intent.id,intent.reason,intent.prevent_scroll,intent.attempted))};
    let refresh=owner.borrow().modules.focus.entry_projection.as_ref().is_none_or(|projection|projection.binding!=binding);
    let attempt=request.is_some_and(|(_,_,_,attempted)|attempted!=Some(binding));
    if !refresh&&!attempt{return}
    // The SDK owns physical discovery. Nothing in the intent or projection stores
    // a descendant handle, and ordinary AnyView children need no Proto registration.
    let target=native_entry_target(owner,focus,window);
    let self_fallback=target.as_ref().is_some_and(|target|target==focus);
    project_native_entry(owner,binding,target.as_ref(),focus);
    let Some((id,reason,prevent_scroll,attempted))=request else{return};
    if !entry_intent_current(owner,id,binding){if owner.borrow().modules.focus.entry_intent.as_ref().is_some_and(|intent|intent.id==id){retire_entry_intent(owner);}return}
    if attempted==Some(binding){return}
    owner.borrow_mut().modules.focus.entry_intent.as_mut().expect("current native Entry intent").attempted=Some(binding);
    let Some(target)=target else{return};
    // Options belong to the actual generated target when it has one. Foreign
    // handles stay native; this existing owner metadata is never a candidate list.
    OWNERS.with(|owners|{for recipient in owners.borrow().iter().filter_map(Weak::upgrade){let mut recipient=recipient.borrow_mut();if !recipient.disposed&&recipient.mounted&&recipient.present&&recipient.window_id==Some(window_id)&&recipient.modules.focus.native.as_ref()==Some(&target){recipient.modules.focus.reason=reason;recipient.modules.focus.prevent_scroll=prevent_scroll;}}});
    if !entry_intent_current(owner,id,binding)||window.focus_tree_generation()!=binding.generation{return}
    window.focus(&target,cx);
    if !entry_intent_current(owner,id,binding)||window.focus_tree_generation()!=binding.generation||!target.is_focused(window){return}
    // Settle this request before any fact observer can create a new one.
    retire_entry_intent(owner);
    if self_fallback{owner.borrow_mut().modules.focus.entry_self_focused=true;native_focus_fact(owner,true);}
}
fn queue_focus(owner:&OwnerRef,reason:&'static str,prevent_scroll:bool,bypass_gate:bool){
    let mut o=owner.borrow_mut();o.modules.focus.request_source=None;o.modules.focus.reason=reason;o.modules.focus.prevent_scroll=prevent_scroll;o.modules.focus.bypass_gate=bypass_gate;o.focus_requested=true;o.dirty=true;
}
fn queue_native_focus(owner:&OwnerRef,options:&Value,bypass_gate:bool){queue_focus(owner,focus_request_reason(options),bool_or(options.member("preventScroll",true),false),bypass_gate);}
fn request_native_focus(owner:&OwnerRef,options:Value)->Option<u64>{
    if !owner.borrow().focusable||owner.borrow().focus_disabled{return None}
    if ready(owner)&&top_scope(owner).is_some_and(|scope|!descendant(owner,&scope)){return None}
    let id=note_focus_intent(owner);queue_native_focus(owner,&options,false);Some(id)
}
pub fn focus_request(owner:&OwnerWeak,options:Value,entry:bool)->Value{
    let owner=runtime(owner);
    if entry{if entry_was_resolved(&owner){retain_entry_intent(&owner,None,focus_request_reason(&options),bool_or(options.member("preventScroll",true),false));}return Value::Undefined}
    request_native_focus(&owner,options);Value::Undefined
}
pub fn focus_blur(owner:&OwnerWeak)->Value{let owner=runtime(owner);{let mut o=owner.borrow_mut();o.modules.focus.scope_activation=o.modules.focus.scope_activation.wrapping_add(1);}retire_entry_intent(&owner);clear_scope_recipient(&owner);let source=owner.borrow().modules.focus.request_source.clone();if let Some((source,id))=source{if let Some(source)=source.upgrade(){if source.borrow().modules.focus.entry_intent.as_ref().is_some_and(|intent|intent.id==id){retire_entry_intent(&source);}}}let mut o=owner.borrow_mut();o.focus_requested=false;o.modules.focus.request_source=None;o.focused=false;o.modules.focus.visible=false;o.modules.focus.module_active=false;o.modules.focus.blur_requested=true;o.dirty=true;Value::Undefined}
pub fn focus_is_focused(owner:&OwnerWeak)->Value{Value::boolean(ensure(owner).borrow().focused)}
pub fn focus_is_active(owner:&OwnerWeak)->Value{Value::boolean(ensure(owner).borrow().modules.focus.scope_active)}
pub fn focus_get_roving(owner:&OwnerWeak)->Value{ensure(owner).borrow_mut().modules.focus.roving_declared=true;Value::Handle(owner.clone(),"focus-roving")}
pub fn focus_navigate(owner:&OwnerWeak,roving:bool,operation:&'static str,options:Value)->Value{
    let owner=runtime(owner);
    if roving||owner.borrow().modules.focus.roving_declared{navigate_focus(&owner,true,operation,options);}
    else if operation=="first"&&text_or(field(&owner.borrow().modules.focus.scope,"emptyPolicy"),"none")=="container"&&!owner.borrow().focus_disabled{
        let mut o=owner.borrow_mut();o.modules.focus.module_active=true;o.modules.focus.has_focused=false;o.focused=false;o.modules.focus.visible=false;o.dirty=true;
    }else{request_native_focus(&owner,Value::Undefined);}
    Value::Undefined
}
fn navigate_focus(owner:&OwnerRef,roving:bool,operation:&'static str,options:Value)->bool{
    let members=focus_members(owner,roving);if members.is_empty(){if roving&&bool_or(options.member("defer",true),false){owner.borrow_mut().modules.focus.pending=Some((operation,options));}return false}
    let current=members.iter().position(|m|m.borrow().focused).or_else(||if roving{members.iter().position(|m|m.borrow().modules.focus.active)}else{owner.borrow().modules.focus.remembered.upgrade().and_then(|remembered|members.iter().position(|m|Rc::ptr_eq(m,&remembered)))});
    let config=if roving{owner.borrow().modules.focus.roving.clone()}else{owner.borrow().modules.focus.scope.clone()};let looping=bool_or(field(&config,"loop"),false);
    let index=match operation{"first"=>Some(0),"last"=>Some(members.len()-1),"selected"=>Some(members.iter().position(|m|m.borrow().modules.focus.selected).unwrap_or(0)),"next"|"prev"=>{let next=match(current,operation){(Some(i),"next")=>i as isize+1,(Some(i),_)=>i as isize-1,(None,"next")=>0,(None,_)=>members.len()as isize-1};if looping{Some(next.rem_euclid(members.len()as isize)as usize)}else if next>=0&&(next as usize)<members.len(){Some(next as usize)}else if !roving{Some(next.clamp(0,members.len()as isize-1)as usize)}else{None}},_=>panic!("unknown native focus navigation")};
    if let Some(index)=index{let target=&members[index];let pending=!ready(target)&&bool_or(options.member("defer",true),false);owner.borrow_mut().modules.focus.pending=if pending{Some((operation,options.clone()))}else{None};let options=if options.nullish(){record(vec![("reason",Value::string("keyboard"))])}else{options};request_native_focus(target,options);true}else{false}
}
pub fn focus_activate(owner:&OwnerWeak,options:Value)->Value{
    let owner=runtime(owner);let use_entry=matches!(owner.borrow().modules.focus.scope.get("entryFallback"),Some(Value::Json(Json::String(mode))) if mode=="entry");
    let activation={let mut o=owner.borrow_mut();o.modules.focus.scope_activation=o.modules.focus.scope_activation.wrapping_add(1);o.modules.focus.scope_intent=FOCUS_INTENT_EPOCH.with(Cell::get);o.modules.focus.scope_activation};retire_scope_entry(&owner);clear_scope_recipient(&owner);let request_epoch=focus_intent_epoch(&owner);
    let reason=focus_request_reason(&options);let prevent_scroll=bool_or(options.member("preventScroll",true),false);
    let window=owner.borrow().window_id;let previous=all_owners().into_iter().find(|o|o.borrow().window_id==window&&o.borrow().focused&&!descendant(o,&owner));{let mut o=owner.borrow_mut();o.modules.focus.previous=previous.as_ref().map(Rc::downgrade).unwrap_or_default();o.modules.focus.scope_active=true;o.modules.focus.module_active=true;o.modules.focus.has_focused=true;o.dirty=true;}
    ACTIVE_SCOPES.with(|scopes|{let mut scopes=scopes.borrow_mut();scopes.retain(|s|!Weak::ptr_eq(s,&Rc::downgrade(&owner)));scopes.push(Rc::downgrade(&owner));});
    if use_entry{poll_state_watchers();if owner.borrow().disposed||owner.borrow().modules.focus.scope_activation!=activation||request_epoch!=focus_intent_epoch(&owner){return Value::Undefined}}
    let entry=text_or(field(&owner.borrow().modules.focus.scope,"entry"),"first");if entry!="manual"&&text_or(options.member("entry",true),"configured")!="manual"{
        let roving=owner.borrow().modules.focus.roving_declared;let members=focus_members_for(&owner,roving,use_entry);
        if let Some(target)=members.first(){if let Some(id)=request_native_focus(target,options){target.borrow_mut().modules.focus.request_source=Some((Rc::downgrade(&owner),id));let mut o=owner.borrow_mut();o.modules.focus.scope_intent=id;o.modules.focus.scope_recipient=Some((Rc::downgrade(target),id));}}
        else{
            if text_or(field(&owner.borrow().modules.focus.scope,"emptyPolicy"),"none")=="none"{let mut o=owner.borrow_mut();o.modules.focus.scope_active=false;o.modules.focus.module_active=false;}
            if use_entry{poll_state_watchers();if owner.borrow().disposed||owner.borrow().modules.focus.scope_activation!=activation||request_epoch!=focus_intent_epoch(&owner){return Value::Undefined}
                let enabled={let o=owner.borrow();o.modules.hooks.contains("focus-entry")&&!o.modules.focus.entry_disabled};
                if enabled{retain_entry_intent(&owner,Some(activation),reason,prevent_scroll);let mut o=owner.borrow_mut();o.modules.focus.scope_intent=o.modules.focus.intent;}
            }
        }
    }
    Value::Undefined
}
pub fn focus_restore(owner:&OwnerWeak)->Value{focus_request(owner,Value::Undefined,false)}
pub fn focus_deactivate(owner:&OwnerWeak,options:Value)->Value{
    let owner=runtime(owner);{let mut o=owner.borrow_mut();o.modules.focus.scope_activation=o.modules.focus.scope_activation.wrapping_add(1);}retire_scope_entry(&owner);clear_scope_recipient(&owner);
    let request_epoch=focus_intent_epoch(&owner);let activation=owner.borrow().modules.focus.scope_activation;let scope_intent=owner.borrow().modules.focus.scope_intent;let superseded=request_epoch>scope_intent&&pending_focus_since(&owner,scope_intent);
    let was_active={let mut o=owner.borrow_mut();o.modules.focus.module_active=false;o.dirty=true;std::mem::take(&mut o.modules.focus.scope_active)};
    if !was_active{return Value::Undefined}
    for member in focus_members(&owner,false){let mut member=member.borrow_mut();member.modules.focus.module_active=false;if member.focused{member.modules.focus.blur_requested=true;member.focused=false;member.modules.focus.visible=false;}member.dirty=true;}
    poll_state_watchers();if owner.borrow().disposed||owner.borrow().modules.focus.scope_activation!=activation||superseded||request_epoch!=focus_intent_epoch(&owner)||text_or(field(&owner.borrow().modules.focus.scope,"restore"),"previous")=="none"{return Value::Undefined}
    let previous=owner.borrow().modules.focus.previous.upgrade();if let Some(previous)=previous{if ready(&previous)&&previous.borrow().focusable&&!previous.borrow().focus_disabled{note_focus_intent(&previous);queue_native_focus(&previous,&options,true);}}Value::Undefined
}
pub fn native_focus_fact(owner:&OwnerRef,focused:bool){
    if owner.borrow().disposed{return}
    if !focused{native_press_blur(owner);}
    let focused={let mut o=owner.borrow_mut();if !focused{o.modules.focus.entry_self_focused=false;}focused&&(o.focusable&&!o.focus_disabled||!o.focusable&&o.modules.focus.entry_self_focused)};
    {let mut o=owner.borrow_mut();o.focused=focused;o.modules.focus.visible=focused&&o.modules.focus.reason=="keyboard";o.modules.focus.module_active=focused;if focused{o.modules.focus.has_focused=true;}o.dirty=true;}
    if focused{for scope in all_owners(){if !Rc::ptr_eq(&scope,owner)&&descendant(owner,&scope){scope.borrow_mut().modules.focus.remembered=Rc::downgrade(owner);scope.borrow_mut().dirty=true;}}}
    poll_state_watchers();
}
pub fn modules_drive(owner:&OwnerRef,focus:&FocusHandle,window:&mut Window,cx:&mut App){
    refresh_native_atoms(owner,window,cx);
    if owner.borrow().disposed{retire_entry_intent(owner);withdraw_entry_view(owner);return}
    if owner.borrow().window_id!=Some(window.window_handle().window_id()){return}
    let live=ready(owner);
    if !live{withdraw_entry_view(owner);}
    overlay_input_scope(owner,cx);
    positioning_drive(window);
    if owner.borrow().disposed{return}
    if live&&ready(owner)&&native_focus_observation_needed(owner){owner.borrow_mut().modules.focus.native=Some(focus.clone());}
    let blur=std::mem::take(&mut owner.borrow_mut().modules.focus.blur_requested);if (blur||text_disabled(owner))&&focus.is_focused(window){window.blur(cx);}
    #[cfg(feature="experimental-focus-tree")]
    drive_entry_intent(owner,focus,window,cx);
    if owner.borrow().disposed{return}
    let blur=std::mem::take(&mut owner.borrow_mut().modules.focus.blur_requested);if blur&&focus.is_focused(window){window.blur(cx);}
    let request=owner.borrow().focus_requested;
    if request&&live&&ready(owner)&&native_focus_available(owner)&&!text_disabled(owner){let (bypass_gate,source)={let mut o=owner.borrow_mut();o.focus_requested=false;(std::mem::take(&mut o.modules.focus.bypass_gate),o.modules.focus.request_source.take())};if bypass_gate||top_scope(owner).map_or(true,|scope|descendant(owner,&scope)){window.focus(focus,cx);if focus.is_focused(window){if let Some((source,id))=source{if let Some(source)=source.upgrade(){let selected=source.borrow().modules.focus.scope_recipient.as_ref().is_some_and(|(_,intent)|*intent==id);if selected{source.borrow_mut().modules.focus.scope_recipient=None;}}}}}}
}
pub fn drive_queued_focus(owner:&OwnerRef,window:&mut Window,cx:&mut App){for other in owners_in(window){if Rc::ptr_eq(owner,&other){continue}let focus=other.borrow().modules.focus.native.clone();if let Some(focus)=focus{modules_drive(&other,&focus,window,cx);}}}
pub fn modules_commit(owner:&OwnerRef){overlay_commit(owner);image_mount(owner);topology_refresh();for owner in all_owners(){let pending=owner.borrow().modules.focus.pending.clone();if let Some((operation,options))=pending{navigate_focus(&owner,true,operation,options);}}poll_state_watchers();}
pub fn focus_key_navigation(event:&gpui::KeyDownEvent,window:&mut Window)->bool{
    let Some(key)=portable_key(&event.keystroke)else{return false};let id=Some(window.window_handle().window_id());
    if key=="Tab"{if let Some(scope)=top_scope_in(id).filter(ready){let trapped={let scope=scope.borrow();let config=&scope.modules.focus.scope;bool_or(field(config,"trap"),false)&&matches!(text_or(field(config,"navigation"),"tab").as_str(),"tab"|"tab+arrow")};if trapped{native_host_callback(&scope,"focus-navigation",||navigate_focus(&scope,false,if event.keystroke.modifiers.shift{"prev"}else{"next"},record(vec![("reason",Value::string("keyboard"))])));window.prevent_default();return true;}}return false;}
    let Some(target)=all_owners().into_iter().find(|o|o.borrow().focused&&o.borrow().window_id==id)else{return false};let Some(provider)=focus_provider(&target)else{return false};
    let config=provider.borrow().modules.focus.roving.clone();let navigation=text_or(field(&config,"navigation"),"none");if !matches!(navigation.as_str(),"arrow"|"tab+arrow"){return false}let orientation=text_or(field(&config,"orientation"),"vertical");
    let op=match key.as_str(){"ArrowDown" if orientation!="horizontal"=>Some("next"),"ArrowUp" if orientation!="horizontal"=>Some("prev"),"ArrowRight" if orientation!="vertical"=>Some("next"),"ArrowLeft" if orientation!="vertical"=>Some("prev"),"Home"=>Some("first"),"End"=>Some("last"),_=>None};
    if let Some(op)=op{if native_host_callback(&provider,"focus-navigation",||navigate_focus(&provider,true,op,record(vec![("reason",Value::string("keyboard"))]))).unwrap_or(true){window.prevent_default();return true;}}false
}
pub fn accessible_text(owner:&OwnerWeak,key:&str,value:Value)->Value{let owner=setup(owner);if key=="name"{owner.borrow_mut().name_content=false;}owner.borrow_mut().modules.accessible_text.insert(key.into(),value);Value::Undefined}
pub fn accessible_tree(owner:&OwnerWeak,value:Value)->Value{let owner=setup(owner);let mut o=owner.borrow_mut();let mut fields=own_entries(o.modules.accessible_tree.clone());fields.extend(own_entries(value));o.modules.accessible_tree=record_map(fields);Value::Undefined}
pub fn accessible_level_value(value:&Value)->Option<usize>{let number=match resolved(value){Value::Number(number)=>number,Value::Json(Json::Number(number))=>number.as_f64()?,_=>return None};if number>=1.&&number<=6.&&number.fract()==0.{Some(number as usize)}else{None}}
pub fn accessible_level(owner:&OwnerWeak,value:Value)->Value{let owner=setup(owner);assert!(accessible_level_value(&value).is_some(),"[A11y] level must be an integer in range 1-6");owner.borrow_mut().modules.accessible_level=value;Value::Undefined}
pub fn accessible_alive(owner:&OwnerRef){for condition in owner.borrow().accessible_state_conditions.values(){condition.get().bool();}let level=owner.borrow().modules.accessible_level.clone();if matches!(level,Value::State(_)|Value::BorrowedState(_)|Value::Observed(_,_)){assert!(accessible_level_value(&level).is_some(),"[A11y] level must be an integer in range 1-6");}}
pub fn accessible_relation(owner:&OwnerWeak,key:Value,mut value:Value)->Value{
    let owner=setup(owner);let target=value.member("target",false);
    if let Value::Array(items)=target{let mut refs=Vec::new();for item in items.iter(){let Value::ObjectRef(target)=item else{panic!("[A11y] relation reference lists accept semantic-object refs only")};if !refs.iter().any(|item|matches!(item,Value::ObjectRef(previous)if Weak::ptr_eq(previous,target))){refs.push(item.clone());}}let mut spec=own_entries(value);spec.insert("target".into(),Value::Array(Rc::new(refs)));value=record_map(spec);}
    else{assert!(matches!(target,Value::Json(Json::String(_))|Value::State(_)|Value::BorrowedState(_)|Value::Observed(_,_)|Value::ObjectRef(_)),"[A11y] relation target must be a string, State, or semantic-object ref");}
    owner.borrow_mut().modules.accessible_relations.insert(key.text(),value);Value::Undefined
}
pub fn accessible_action_spec(owner:&OwnerWeak,key:Value,spec:Value)->Value{let owner=setup(owner);let key=key.text();let event=text_or(spec.member("event",true),&default_accessibility_event(&key));let mut o=owner.borrow_mut();o.accessible_actions.insert(key.clone());o.modules.action_events.insert(key,event);Value::Undefined}
#[derive(Clone)]
pub struct NativeStateWatch{source:Value,previous:Value,callback:Value,active:Rc<Cell<bool>>,disconnected:bool}
pub fn state_default(receiver:Value,value:Value)->Value{let (Value::State(state)|Value::BorrowedState(state))=receiver else{value_type_error("State handle required")};setup(&state.owner);let next=normalize_state(state.kind,value,&state.spec,true);*state.value.borrow_mut()=next;Value::Undefined}
pub fn watch_state(owner:&OwnerWeak,source:Value,callback:Value)->Value{let owner=setup(owner);let previous=source.get();let active=Rc::new(Cell::new(true));owner.borrow_mut().modules.state_watchers.push(NativeStateWatch{source,previous,callback,active:active.clone(),disconnected:false});disposer(owner,active,false)}
enum StateNotice {
    Observed{owner:OwnerWeak,watch:NativeStateWatch,event:Value,terminal:bool},
}
thread_local!{
    static STATE_DISPATCHING:Cell<bool>=Cell::new(false);
    static STATE_NOTICES:RefCell<VecDeque<StateNotice>>=RefCell::new(VecDeque::new());
}
pub fn write_state_value(state:&Rc<State>,value:Value,reason:Value){
    let next=normalize_state(state.kind,value,&state.spec,false);
    let previous=state.value.borrow().clone();
    if same_value(&previous,&next){return}
    *state.value.borrow_mut()=next.clone();
    ensure(&state.owner).borrow_mut().dirty=true;
    let event=record(vec![("type",Value::string("next")),("prev",previous),("next",next),("reason",reason)]);
    // Record registration identities before callbacks run. Reentrant changes append
    // after every notification from this transition; cancellation stays immediate.
    let mut recorded=Vec::new();
    for owner in all_owners(){
        if matches!(owner.borrow().phase,"setup"|"before-dispose"){continue}
        let watchers:Vec<_>=owner.borrow().modules.state_watchers.iter().filter(|watch|watch.active.get()&&!watch.disconnected&&matches!(&watch.source,Value::State(source)|Value::BorrowedState(source)if Rc::ptr_eq(source,state))).cloned().collect();
        for watch in watchers{recorded.push(StateNotice::Observed{owner:Rc::downgrade(&owner),watch,event:event.clone(),terminal:false});}
    }
    STATE_NOTICES.with(|queue|queue.borrow_mut().extend(recorded));
    drain_state_notices();
}
fn drain_state_notices(){
    if STATE_DISPATCHING.with(|flag|flag.replace(true)){return}
    struct Reset;
    impl Drop for Reset{fn drop(&mut self){STATE_DISPATCHING.with(|flag|flag.set(false))}}
    let _reset=Reset;
    let mut failures=Vec::new();
    loop {
        let notice=STATE_NOTICES.with(|queue|queue.borrow_mut().pop_front());
        match notice {
            None=>break,
            Some(StateNotice::Observed{owner,watch,event,terminal})=>{
                let source_alive=match &watch.source{Value::State(source)|Value::BorrowedState(source)=>source.owner.upgrade().is_some_and(|owner|!owner.borrow().disposed),Value::Observed(owner,_)=>owner.upgrade().is_some_and(|owner|!owner.borrow().disposed),_=>false};
                if watch.active.get()&&(terminal||source_alive){
                    if let Some(owner)=owner.upgrade().filter(|owner|!owner.borrow().disposed){
                        if let Err(failure)=catch_authored(||invoke(&owner,"state-watch",&watch.callback,vec![capability(&Rc::downgrade(&owner)),event])){failures.push(failure);}
                    }
                }
                if terminal{watch.active.set(false);}
            },
        }
    }
    if failures.len()==1{throw_value(failures.pop().unwrap());}
    if !failures.is_empty(){throw_value(record(vec![("name",Value::string("AggregateError")),("message",Value::string("[State] subscriber failures")),("errors",Value::Array(Rc::new(failures)))]));}
}
pub fn poll_state_watchers(){
    for owner in all_owners(){
        if matches!(owner.borrow().phase,"setup"|"before-dispose"){continue}
        let watchers=owner.borrow().modules.state_watchers.clone();
        for (index,watch) in watchers.into_iter().enumerate(){
            if !watch.active.get()||watch.disconnected{continue}
            let source_owner=match &watch.source{Value::State(state)|Value::BorrowedState(state)=>state.owner.clone(),Value::Observed(owner,_)=>owner.clone(),_=>panic!("State watch source required")};
            let terminal=!source_owner.upgrade().is_some_and(|source|!source.borrow().disposed);
            let event=if terminal {
                owner.borrow_mut().modules.state_watchers[index].disconnected=true;
                record(vec![("type",Value::string("disconnect")),("reason",Value::string("unmount"))])
            } else {
                if matches!(&watch.source,Value::State(_)|Value::BorrowedState(_)){continue}
                let next=watch.source.get();
                if same_value(&watch.previous,&next){continue}
                owner.borrow_mut().modules.state_watchers[index].previous=next.clone();
                let mut fields=vec![("type",Value::string("next")),("next",next),("prev",watch.previous.clone())];
                if let Value::Observed(source,"overlay.open")=&watch.source{let reason=ensure(source).borrow().modules.overlay.reason.clone();if !matches!(reason,Value::Undefined){fields.push(("reason",reason));}}
                record(fields)
            };
            STATE_NOTICES.with(|queue|queue.borrow_mut().push_back(StateNotice::Observed{owner:Rc::downgrade(&owner),watch,event,terminal}));
        }
    }
    drain_state_notices();
}
pub fn host_target(owner:&OwnerWeak)->Value{let owner_ref=ensure(owner);if ready(&owner_ref){Value::HostTarget(owner.clone())}else{Value::Json(Json::Null)}}

#[derive(Clone)]
pub struct AnatomyClaim{pub family:&'static StaticDeclaration,pub role:String,pub profile:Option<String>}
#[derive(Clone)]
pub struct AnatomyWatch{pub family:&'static StaticDeclaration,pub role:String,pub active:Rc<Cell<bool>>,pub callback:Value,pub signature:Vec<usize>}
#[derive(Default)]
pub struct Topology{pub claims:BTreeMap<usize,AnatomyClaim>,pub subscriptions:Vec<AnatomyWatch>,pub signatures:BTreeMap<usize,Vec<usize>>,pub versions:BTreeMap<usize,u64>,pub collection:BTreeMap<String,Value>,pub item:BTreeMap<String,Value>,pub handles:BTreeMap<String,Value>,pub table:TableModule,pub order:u64}
thread_local!{static HOST_ORDER:Cell<u64>=Cell::new(0);static TOPOLOGY_UPDATING:Cell<bool>=Cell::new(false);}
pub fn note_host_order(owner:&OwnerRef){HOST_ORDER.with(|n|{let order=n.get().wrapping_add(1);n.set(order);owner.borrow_mut().modules.topology.order=order;});}
pub fn declaration(value:&Value,kind:&str)->&'static StaticDeclaration{match value{Value::StaticCapability(value) if value.kind==kind=>value,_=>panic!("expected nominal {kind} declaration")}}
pub fn anatomy_claim(owner:&OwnerWeak,family:Value,config:Value)->Value{
    let owner=setup(owner);let family=declaration(&family,"anatomy-family");let role=config.member("role",false).text();let declaration=(family.config)();assert!(!declaration.member("roles",false).member(&role,false).nullish(),"undeclared Anatomy role");let profile=config.member("profile",false);let profile=if profile.nullish(){None}else{let name=profile.text();assert!(!declaration.member("profiles",true).member(&name,true).nullish(),"unknown Anatomy profile");Some(name)};
    let mut o=owner.borrow_mut();assert!(!o.modules.topology.claims.contains_key(&family.identity()),"duplicate Anatomy family claim");o.modules.topology.claims.insert(family.identity(),AnatomyClaim{family,role,profile});Value::Undefined
}
fn anatomy_root(owner:&OwnerRef,family:usize)->Option<OwnerRef>{let mut current=Some(owner.clone());let mut visited=BTreeSet::new();while let Some(candidate)=current{assert!(visited.insert(Rc::as_ptr(&candidate)as usize),"cyclic Anatomy ancestry");let o=candidate.borrow();let is_root=o.modules.topology.claims.get(&family).is_some_and(|c|c.role=="root");current=o.parent.upgrade();drop(o);if is_root{return Some(candidate)}}None}
fn anatomy_members(owner:&OwnerRef,family:usize,role:Option<&str>,strict:bool)->Vec<OwnerRef>{let Some(root)=anatomy_root(owner,family)else{if strict{throw_error("ANATOMY_CLAIM_INVALID","current owner is not in an Anatomy domain");}return vec![]};let mut parts:Vec<_>=all_owners().into_iter().filter(|part|{let p=part.borrow();let included=p.mounted&&p.present&&p.modules.topology.claims.get(&family).is_some_and(|claim|role.map_or(true,|role|claim.role==role));drop(p);included&&anatomy_root(part,family).is_some_and(|r|Rc::ptr_eq(&r,&root))}).collect();parts.sort_by_key(|p|p.borrow().modules.topology.order);parts}
pub fn anatomy_parts(owner:&OwnerWeak,family:Value,role:Value)->Value{let owner=ensure(owner);let family=declaration(&family,"anatomy-family");let role=if role.nullish(){None}else{Some(role.text())};Value::Array(Rc::new(anatomy_members(&owner,family.identity(),role.as_deref(),true).into_iter().map(|part|{let role=part.borrow().modules.topology.claims[&family.identity()].role.clone();Value::Part(Rc::downgrade(&part),role)}).collect()))}
pub fn anatomy_has(owner:&OwnerWeak,family:Value,role:Value)->Value{Value::boolean(!anatomy_parts(owner,family,role).items().is_empty())}
pub fn anatomy_version(owner:&OwnerWeak,family:Value)->Value{let owner=ensure(owner);let family=declaration(&family,"anatomy-family");let root=anatomy_root(&owner,family.identity()).unwrap_or_else(||throw_error("ANATOMY_CLAIM_INVALID","current owner is not in an Anatomy domain"));let version=root.borrow().modules.topology.versions.get(&family.identity()).copied().unwrap_or(0);Value::number(version as f64)}
pub fn anatomy_neighbor(owner:&OwnerWeak,family:Value,role:Value,delta:i32)->Value{let owner_ref=ensure(owner);let parts=anatomy_parts(owner,family,role).items();let index=parts.iter().position(|part|Weak::ptr_eq(&part.owner(),&Rc::downgrade(&owner_ref)));if delta==0{return Value::number(index.map_or(-1.,|i|i as f64))}index.and_then(|i|{let next=i as isize+delta as isize;if next<0{None}else{parts.get(next as usize).cloned()}}).unwrap_or(Value::Json(Json::Null))}
pub fn anatomy_subscribe(owner:&OwnerWeak,family:Value,role:Value,callback:Value)->Value{let owner=setup(owner);let family=declaration(&family,"anatomy-family");let role=role.text();let signature=anatomy_members(&owner,family.identity(),Some(&role),false).iter().map(|p|Rc::as_ptr(p)as usize).collect();let active=Rc::new(Cell::new(true));owner.borrow_mut().modules.topology.subscriptions.push(AnatomyWatch{family,role,callback,signature,active:active.clone()});disposer(owner,active,true)}
pub fn part_has_expose(part:Value,key:Value)->Value{Value::boolean(ensure(&part.owner()).borrow().exposes.contains_key(&key.text()))}
pub fn part_get_expose(part:Value,key:Value)->Value{ensure(&part.owner()).borrow().exposes.get(&key.text()).cloned().unwrap_or(Value::Json(Json::Null))}
pub fn part_has_hook(part:Value,key:Value)->Value{let key=key.text();let kind=match key.as_str(){"asFocusable"=>"focus","asFocusEntry"=>"focus-entry","asFocusScope"=>"focus-scope","asFocusRoving"=>"focus-roving","asAccessible"=>"accessible","asCollection"=>"collection","asCollectionItem"=>"collection-item","asOverlay"=>"overlay","asTrigger"=>"trigger",_=>key.as_str()};Value::boolean(ensure(&part.owner()).borrow().modules.hooks.contains(kind))}
fn owned_state(owner:&OwnerWeak,key:&str,kind:&'static str,value:Value)->Value{create_state(owner,kind,Value::string(key),value,Value::Undefined)}
fn internal_state(owner:&OwnerRef,key:&str,value:Value){let handle=owner.borrow().modules.topology.handles.get(key).cloned();if let Some(Value::State(state))=handle{write_state_value(&state,value,Value::string("native-topology"));}}
pub fn collection_setup(owner:&OwnerWeak,item:bool)->Value{
    let kind=if item{"collection-item"}else{"collection"};let handle=module_handle(owner,kind);let owner_ref=ensure(owner);if item{for(key,name,value,kind)in[("collectionIndex","collectionIndex",Value::number(-1.),"discrete"),("collectionTotal","collectionTotal",Value::number(0.),"discrete"),("collectionFirst","collectionFirst",Value::boolean(false),"bool"),("collectionLast","collectionLast",Value::boolean(false),"bool")]{if !owner_ref.borrow().modules.topology.handles.contains_key(key){let state=owned_state(owner,name,kind,value);owner_ref.borrow_mut().modules.topology.handles.insert(key.into(),state);}}}else if !owner_ref.borrow().modules.topology.handles.contains_key("count"){let state=owned_state(owner,"collectionCount","discrete",Value::number(0.));owner_ref.borrow_mut().modules.topology.handles.insert("count".into(),state);}handle
}
pub fn collection_configure(owner:&OwnerWeak,patch:Value,item:bool)->Value{
    let owner_ref=setup(owner);let family=patch.member("family",false);declaration(&family,"anatomy-family");let config=match patch{Value::Record(entries)=>entries.fields.borrow().clone(),_=>panic!("Collection config required")};
    if item{let role=text_or(field(&config,"role"),"item");anatomy_claim(owner,family,record(vec![("role",Value::string(&role))]));owner_ref.borrow_mut().modules.topology.item=config.clone();for(key,option,default)in[("collectionIndex","exposeIndexStateKey","collectionIndex"),("collectionTotal","exposeTotalStateKey","collectionTotal"),("collectionFirst","exposeFirstStateKey","collectionFirst"),("collectionLast","exposeLastStateKey","collectionLast")]{let state=owner_ref.borrow().modules.topology.handles[key].clone();expose_state(owner,Value::string(&text_or(field(&config,option),default)),state);}let key=text_or(field(&config,"exposeSnapshotMethodKey"),"getCollectionItem");let weak=owner.clone();expose_method(owner,Value::string(&key),Value::Function(Rc::new(move |_|collection_item_snapshot(&weak))));let meta_key=text_or(field(&config,"metaExposeKey"),"__collectionItem");let weak=owner.clone();expose_method(owner,Value::string(&meta_key),Value::Function(Rc::new(move |_|collection_item_meta(&weak))));
    }else{let role=field(&config,"ownerRole");let role=if role.nullish(){field(&config,"rootRole")}else{role};if !role.nullish()&&!matches!(role,Value::Json(Json::Bool(false))){anatomy_claim(owner,family,record(vec![("role",role)]));}owner_ref.borrow_mut().modules.topology.collection=config.clone();let count=owner_ref.borrow().modules.topology.handles["count"].clone();expose_state(owner,Value::string(&text_or(field(&config,"exposeCountStateKey"),"count")),count);let weak=owner.clone();expose_method(owner,Value::string(&text_or(field(&config,"exposeItemsMethodKey"),"getCollectionItems")),Value::Function(Rc::new(move |_|collection_items(&weak))));let weak=owner.clone();expose_method(owner,Value::string(&text_or(field(&config,"exposeCountMethodKey"),"getCollectionCount")),Value::Function(Rc::new(move |_|collection_count(&weak))));}
    Value::Undefined
}
fn collection_item_meta(owner:&OwnerWeak)->Value{let owner_ref=ensure(owner);let callback=field(&owner_ref.borrow().modules.topology.item,"getMeta");if callback.nullish(){record(vec![])}else{invoke(&owner_ref,"expose-method",&callback,vec![capability(owner)])}}
fn collection_position(owner:&OwnerRef)->(i64,usize){let config=owner.borrow().modules.topology.item.clone();let family=field(&config,"family");if family.nullish(){return(-1,0)}let family=declaration(&family,"anatomy-family");let role=text_or(field(&config,"role"),"item");let parts=anatomy_members(owner,family.identity(),Some(&role),false);(parts.iter().position(|p|Rc::ptr_eq(p,owner)).map_or(-1,|i|i as i64),parts.len())}
pub fn collection_item_snapshot(owner:&OwnerWeak)->Value{let owner_ref=ensure(owner);let(index,total)=collection_position(&owner_ref);let meta=collection_item_meta(owner);let Value::Record(meta)=meta else{panic!("Collection item metadata must be portable record")};let mut fields=meta.fields.borrow().clone();fields.insert("index".into(),Value::number(index as f64));fields.insert("total".into(),Value::number(total as f64));fields.insert("first".into(),Value::boolean(index==0));fields.insert("last".into(),Value::boolean(index>=0&&index as usize+1==total));record_map(fields)}
pub fn collection_items(owner:&OwnerWeak)->Value{let owner=ensure(owner);let config=owner.borrow().modules.topology.collection.clone();let family=field(&config,"family");if family.nullish(){return Value::Array(Rc::new(vec![]))}let family=declaration(&family,"anatomy-family");let role=text_or(field(&config,"itemRole"),"item");let meta_key=text_or(field(&config,"itemMetaExposeKey"),"__collectionItem");let parts=anatomy_members(&owner,family.identity(),Some(&role),false);let total=parts.len();Value::Array(Rc::new(parts.into_iter().enumerate().map(|(index,part)|{let meta=part.borrow().exposes.get(&meta_key).cloned();let meta=match meta{Some(Value::Function(f))=>invoke(&part,"expose-method",&Value::Function(f),vec![]),Some(value)=>value,None=>record(vec![])};let Value::Record(meta)=meta else{panic!("Collection item metadata must be portable record")};let mut fields=meta.fields.borrow().clone();fields.insert("index".into(),Value::number(index as f64));fields.insert("total".into(),Value::number(total as f64));fields.insert("first".into(),Value::boolean(index==0));fields.insert("last".into(),Value::boolean(index+1==total));record_map(fields)}).collect()))}
pub fn collection_count(owner:&OwnerWeak)->Value{Value::number(collection_items(owner).items().len()as f64)}
pub fn topology_refresh(){
    if TOPOLOGY_UPDATING.with(|flag|flag.replace(true)){return}struct Reset;impl Drop for Reset{fn drop(&mut self){TOPOLOGY_UPDATING.with(|flag|flag.set(false));}}let _reset=Reset;
    let owners=all_owners();let mut callbacks=Vec::new();
    for owner in &owners{let claims=owner.borrow().modules.topology.claims.clone();for(id,claim)in claims{if claim.role=="root"{let signature:Vec<_>=anatomy_members(owner,id,None,false).iter().map(|p|Rc::as_ptr(p)as usize).collect();let changed=owner.borrow().modules.topology.signatures.get(&id)!=Some(&signature);if changed{let mut o=owner.borrow_mut();o.modules.topology.signatures.insert(id,signature);*o.modules.topology.versions.entry(id).or_default()+=1;}}}
        if owner.borrow().mounted{let watches=owner.borrow().modules.topology.subscriptions.clone();for(index,watch)in watches.into_iter().enumerate(){if !watch.active.get(){continue}let parts=anatomy_members(owner,watch.family.identity(),Some(&watch.role),false);let signature:Vec<_>=parts.iter().map(|p|Rc::as_ptr(p)as usize).collect();if signature!=watch.signature{owner.borrow_mut().modules.topology.subscriptions[index].signature=signature;callbacks.push((owner.clone(),watch.callback,Value::Array(Rc::new(parts.into_iter().map(|p|Value::Part(Rc::downgrade(&p),watch.role.clone())).collect()))));}}}
        if !owner.borrow().modules.topology.collection.is_empty(){let config=owner.borrow().modules.topology.collection.clone();let family=field(&config,"family");let family=declaration(&family,"anatomy-family");let role=text_or(field(&config,"itemRole"),"item");let count=anatomy_members(owner,family.identity(),Some(&role),false).len();internal_state(owner,"count",Value::number(count as f64));}
        if !owner.borrow().modules.topology.item.is_empty(){let(index,total)=collection_position(owner);internal_state(owner,"collectionIndex",Value::number(index as f64));internal_state(owner,"collectionTotal",Value::number(total as f64));internal_state(owner,"collectionFirst",Value::boolean(index==0));internal_state(owner,"collectionLast",Value::boolean(index>=0&&index as usize+1==total));}
    }
    table_refresh(&owners);
    for(owner,callback,parts)in callbacks{if !owner.borrow().disposed&&owner.borrow().mounted{invoke(&owner,"mounted",&callback,vec![capability(&Rc::downgrade(&owner)),parts]);}}
}

const TABLE_FAMILY_ID:&str="@proto.ui/module-table-structure#TABLE_STRUCTURE_FAMILY";
pub static TABLE_STRUCTURE_FAMILY:StaticDeclaration=StaticDeclaration{id:TABLE_FAMILY_ID,kind:"anatomy-family",config:||Value::json(serde_json::from_str(r#"{"roles":{"root":{"cardinality":{"min":1,"max":1}},"caption":{"cardinality":{"min":0,"max":1}},"row":{"cardinality":{"min":1,"max":"*"}},"headerCell":{"cardinality":{"min":1,"max":"*"}},"cell":{"cardinality":{"min":1,"max":"*"}}},"relations":[{"kind":"contains","parent":"root","child":"caption"},{"kind":"contains","parent":"root","child":"row"},{"kind":"contains","parent":"row","child":"headerCell"},{"kind":"contains","parent":"row","child":"cell"}]}"#).expect("checked Table family declaration"))};
#[derive(Default)]
pub struct TableModule{pub role:String,pub config:BTreeMap<String,Value>,pub snapshot:Value}
#[derive(Clone)]
struct TableCell{owner:OwnerRef,row:usize,column:u64,row_span:u64,column_span:u64,kind:String,column_headers:Vec<Value>,row_headers:Vec<Value>,ordered_headers:Vec<Value>}
pub fn table_setup(owner:&OwnerWeak,role:Value)->Value{let role=role.text();assert!(["root","caption","row","headerCell","cell"].contains(&role.as_str()),"invalid table role");let owner_ref=setup(owner);let previous=owner_ref.borrow().modules.topology.table.role.clone();assert!(previous.is_empty()||previous==role,"table role already declared");if previous.is_empty(){for(key,name,kind,initial)in[("table.a11yRole","tableA11yRole","string",Value::string("")),("table.rowCount","tableRowCount","discrete",Value::number(0.)),("table.columnCount","tableColumnCount","discrete",Value::number(0.)),("table.row","tableRow","discrete",Value::number(-1.)),("table.column","tableColumn","discrete",Value::number(-1.)),("table.rowSpan","tableRowSpan","discrete",Value::number(0.)),("table.columnSpan","tableColumnSpan","discrete",Value::number(0.))]{let state=owned_state(owner,name,kind,initial);owner_ref.borrow_mut().modules.topology.handles.insert(key.into(),state);}owner_ref.borrow_mut().modules.topology.table.role=role;}module_handle(owner,"table-structure")}
pub fn table_configure(owner:&OwnerWeak,patch:Value)->Value{let owner=runtime(owner);merge_config(&mut owner.borrow_mut().modules.topology.table.config,patch);topology_refresh();Value::Undefined}
pub fn table_snapshot(owner:&OwnerWeak)->Value{let owner=ensure(owner);let snapshot=owner.borrow().modules.topology.table.snapshot.clone();if snapshot.nullish(){Value::Json(Json::Null)}else{snapshot}}
pub fn table_object_ref(owner:&OwnerWeak)->Value{ensure(owner);Value::ObjectRef(owner.clone())}
fn table_diagnostic(code:&str,owner:Option<&OwnerRef>,row:Option<usize>,key:Option<&str>)->Value{let mut fields=vec![("code",Value::string(code))];if let Some(owner)=owner{fields.push(("ref",Value::ObjectRef(Rc::downgrade(owner))))}if let Some(row)=row{fields.push(("row",Value::number(row as f64)))}if let Some(key)=key{fields.push(("headerKey",Value::string(key)))}record(fields)}
fn table_clear(owner:&OwnerRef){internal_state(owner,"table.a11yRole",Value::string(""));for key in ["table.rowCount","table.columnCount","table.row","table.column","table.rowSpan","table.columnSpan"]{internal_state(owner,key,Value::number(0.));}let mut o=owner.borrow_mut();o.modules.topology.table.snapshot=Value::Undefined;for key in ["columnHeaders","rowHeaders","caption","labelledBy"]{o.modules.accessible_relations.remove(key);}}
fn table_ref(owner:&OwnerRef)->Value{Value::ObjectRef(Rc::downgrade(owner))}
fn table_relation(owner:&OwnerRef,key:&str,targets:Vec<Value>){owner.borrow_mut().modules.accessible_relations.insert(key.into(),record(vec![("target",Value::Array(Rc::new(targets)))]));}
fn table_refresh(owners:&[OwnerRef]){
    for owner in owners{if !owner.borrow().modules.topology.table.role.is_empty(){table_clear(owner)}}
    for root in owners{
        let family=TABLE_STRUCTURE_FAMILY.identity();
        if !ready(root)||root.borrow().modules.topology.table.role!="root"||!root.borrow().modules.topology.claims.get(&family).is_some_and(|c|c.role=="root"){continue}
        let mut diagnostics=Vec::new();let domain=anatomy_members(root,family,None,false);let mut records=Vec::new();for owner in domain{let o=owner.borrow();let role=&o.modules.topology.table.role;if role.is_empty(){continue}if o.modules.topology.claims[&family].role!=*role{diagnostics.push(table_diagnostic("role-mismatch",Some(&owner),None,None));}else{drop(o);records.push(owner);}}
        let captions:Vec<_>=records.iter().filter(|o|o.borrow().modules.topology.table.role=="caption").cloned().collect();let rows:Vec<_>=records.iter().filter(|o|o.borrow().modules.topology.table.role=="row").cloned().collect();if captions.len()>1{diagnostics.push(table_diagnostic("multiple-captions",None,None,None))}if rows.is_empty(){diagnostics.push(table_diagnostic("missing-row",None,None,None))}
        let mut row_cells:Vec<Vec<OwnerRef>>=vec![vec![];rows.len()];let mut headers=0;let mut normal=0;
        for owner in records.iter().filter(|o|["headerCell","cell"].contains(&o.borrow().modules.topology.table.role.as_str())){let mut parent=owner.borrow().parent.upgrade();let mut row=None;while let Some(p)=parent{if let Some(index)=rows.iter().position(|r|Rc::ptr_eq(r,&p)){row=Some(index);break}if Rc::ptr_eq(&p,root){break}parent=p.borrow().parent.upgrade();}if let Some(row)=row{row_cells[row].push(owner.clone());}else{diagnostics.push(table_diagnostic("missing-row-parent",Some(owner),None,None));}}
        let mut occupied:BTreeMap<usize,Vec<(u64,u64)>>=BTreeMap::new();let mut placed=Vec::<TableCell>::new();let mut column_count=0u64;let mut headers_by_key:BTreeMap<String,Vec<OwnerRef>>=BTreeMap::new();
        for(row,cells)in row_cells.iter().enumerate(){if cells.is_empty(){diagnostics.push(table_diagnostic("empty-row",Some(&rows[row]),Some(row),None));}let mut cursor=0u64;
            for owner in cells{let config=owner.borrow().modules.topology.table.config.clone();let header=owner.borrow().modules.topology.table.role=="headerCell";let header_key=text_or(field(&config,"headerKey"),"");let header_kind=text_or(field(&config,"headerKind"),"");if header{headers+=1;if header_key.is_empty(){diagnostics.push(table_diagnostic("missing-header-key",Some(owner),Some(row),None))}else{headers_by_key.entry(header_key).or_default().push(owner.clone());}if header_kind!="column"&&header_kind!="row"{diagnostics.push(table_diagnostic("missing-header-kind",Some(owner),Some(row),None));}}else{normal+=1;}
                let rs=field(&config,"rowSpan");let cs=field(&config,"columnSpan");let rs=if rs.nullish(){1.}else{rs.num()};let cs=if cs.nullish(){1.}else{cs.num()};if !rs.is_finite()||!cs.is_finite()||rs<1.||cs<1.||rs.fract()!=0.||cs.fract()!=0.||rs>9007199254740991.||cs>9007199254740991.{diagnostics.push(table_diagnostic("invalid-span",Some(owner),None,None));continue}let rs=rs as u64;let cs=cs as u64;if row as u64+rs>rows.len()as u64{diagnostics.push(table_diagnostic("row-span-out-of-range",Some(owner),Some(row),None));continue}
                let mut column=cursor;loop{if column+cs>9007199254740991{break}let mut next=column;for r in row..row+rs as usize{if let Some(intervals)=occupied.get(&r){for(start,end)in intervals{if *end>column&&*start<column+cs{next=next.max(*end);}}}}if next==column{break}column=next;}
                if column+cs>9007199254740991{diagnostics.push(table_diagnostic("column-range-out-of-range",Some(owner),Some(row),None));continue}for r in row..row+rs as usize{occupied.entry(r).or_default().push((column,column+cs));}cursor=column+cs;column_count=column_count.max(cursor);placed.push(TableCell{owner:owner.clone(),row,column,row_span:rs,column_span:cs,kind:if !header{"cell"}else if header_kind=="row"{"row-header"}else{"column-header"}.into(),column_headers:vec![],row_headers:vec![],ordered_headers:vec![]});
            }
        }
        if headers==0{diagnostics.push(table_diagnostic("missing-header-cell",None,None,None))}if normal==0{diagnostics.push(table_diagnostic("missing-cell",None,None,None))}for(key,matches)in &headers_by_key{if matches.len()>1{diagnostics.push(table_diagnostic("duplicate-header-key",matches.first(),None,Some(key)))}}
        for(row,cells)in row_cells.iter().enumerate(){for owner in cells{let config=owner.borrow().modules.topology.table.config.clone();let refs=field(&config,"headers");let refs=if refs.nullish(){vec![]}else{refs.items()};if refs.is_empty()&&owner.borrow().modules.topology.table.role=="cell"{diagnostics.push(table_diagnostic("missing-cell-headers",Some(owner),Some(row),None));}let source=placed.iter().position(|c|Rc::ptr_eq(&c.owner,owner));let mut seen=BTreeSet::new();for key in refs{let key=key.text();let error=if key.is_empty(){Some("empty-header-reference")}else if !seen.insert(key.clone()){Some("duplicate-header-reference")}else{None};if let Some(error)=error{diagnostics.push(table_diagnostic(error,Some(owner),Some(row),Some(&key)));continue}let Some(matches)=headers_by_key.get(&key)else{diagnostics.push(table_diagnostic("missing-header-target",Some(owner),Some(row),Some(&key)));continue};if matches.len()!=1{diagnostics.push(table_diagnostic("ambiguous-header-target",Some(owner),Some(row),Some(&key)));continue}let Some(target)=placed.iter().find(|c|Rc::ptr_eq(&c.owner,&matches[0])).cloned()else{diagnostics.push(table_diagnostic("unprojectable-header-target",Some(owner),Some(row),Some(&key)));continue};if let Some(index)=source{let cell=&mut placed[index];if owner.borrow().modules.topology.table.role=="headerCell"&&(target.row>cell.row||target.row==cell.row&&target.column>=cell.column){diagnostics.push(table_diagnostic("non-upstream-header-target",Some(owner),Some(row),Some(&key)));continue}let target_ref=table_ref(&target.owner);cell.ordered_headers.push(target_ref.clone());if target.kind=="row-header"{cell.row_headers.push(target_ref)}else{cell.column_headers.push(target_ref)}}}}}
        let valid=diagnostics.is_empty();let row_values:Vec<_>=rows.iter().enumerate().map(|(index,row)|{let cells=placed.iter().filter(|c|c.row==index).map(|cell|record(vec![("ref",table_ref(&cell.owner)),("kind",Value::string(&cell.kind)),("row",Value::number(cell.row as f64)),("column",Value::number(cell.column as f64)),("rowSpan",Value::number(cell.row_span as f64)),("columnSpan",Value::number(cell.column_span as f64)),("columnHeaders",Value::Array(Rc::new(cell.column_headers.clone()))),("rowHeaders",Value::Array(Rc::new(cell.row_headers.clone()))),("orderedHeaders",Value::Array(Rc::new(cell.ordered_headers.clone())))] )).collect();record(vec![("ref",table_ref(row)),("index",Value::number(index as f64)),("cells",Value::Array(Rc::new(cells)))])}).collect();let caption=if captions.len()==1{table_ref(&captions[0])}else{Value::Undefined};let snapshot=record(vec![("root",table_ref(root)),("caption",caption.clone()),("rowCount",Value::number(rows.len()as f64)),("columnCount",Value::number(column_count as f64)),("rows",Value::Array(Rc::new(row_values))),("valid",Value::boolean(valid)),("diagnostics",Value::Array(Rc::new(diagnostics)))]);root.borrow_mut().modules.topology.table.snapshot=snapshot;
        if !valid{continue}internal_state(root,"table.a11yRole",Value::string("table"));internal_state(root,"table.rowCount",Value::number(rows.len()as f64));internal_state(root,"table.columnCount",Value::number(column_count as f64));if captions.len()==1{internal_state(&captions[0],"table.a11yRole",Value::string("caption"));table_relation(root,"labelledBy",vec![caption]);}for(index,row)in rows.iter().enumerate(){internal_state(row,"table.a11yRole",Value::string("row"));internal_state(row,"table.row",Value::number(index as f64+1.));}for cell in placed{let role=match cell.kind.as_str(){"column-header"=>"columnheader","row-header"=>"rowheader",_=>"cell"};internal_state(&cell.owner,"table.a11yRole",Value::string(role));internal_state(&cell.owner,"table.row",Value::number(cell.row as f64+1.));internal_state(&cell.owner,"table.column",Value::number(cell.column as f64+1.));internal_state(&cell.owner,"table.rowSpan",Value::number(cell.row_span as f64));internal_state(&cell.owner,"table.columnSpan",Value::number(cell.column_span as f64));if !cell.ordered_headers.is_empty(){let mut labels=cell.ordered_headers;labels.push(table_ref(&cell.owner));table_relation(&cell.owner,"labelledBy",labels);}}
    }
}

#[derive(Clone)]
pub struct NativeRegion{owner:OwnerWeak,active:Rc<Cell<bool>>,options:Value}
#[derive(Default)]
pub struct BoundaryModule{pub config:BTreeMap<String,Value>,pub regions:Vec<NativeRegion>,pub callbacks:Vec<(Rc<Cell<bool>>,Value)>,pub observing:bool,pub active:bool,pub stack_order:u64}
#[derive(Default)]
pub struct HitModule{pub config:BTreeMap<String,Value>,pub regions:Vec<NativeRegion>}
thread_local!{static BOUNDARY_ORDER:Cell<u64>=Cell::new(0);}
pub fn boundary_setup(owner:&OwnerWeak,hit:bool)->Value{module_handle(owner,if hit{"hit-participation"}else{"boundary"})}
pub fn boundary_configure(owner:&OwnerWeak,patch:Value,hit:bool)->Value{let owner=setup(owner);if hit{let mode=patch.member("mode",false);if !mode.nullish(){assert!(["participating","disabled","passthrough"].contains(&mode.text().as_str()),"invalid hit participation mode");}merge_config(&mut owner.borrow_mut().modules.hit.config,patch);}else{merge_config(&mut owner.borrow_mut().modules.boundary.config,patch);}Value::Undefined}
pub fn boundary_observe(owner:&OwnerWeak,observation:Value)->Value{assert!(observation.text()=="pointer.press","unsupported Boundary observation");setup(owner).borrow_mut().modules.boundary.observing=true;Value::Undefined}
pub fn boundary_active(owner:&OwnerWeak,value:Value)->Value{let owner=runtime(owner);let active=value.bool();let mut o=owner.borrow_mut();o.modules.boundary.active=active;if active{BOUNDARY_ORDER.with(|n|{let next=n.get()+1;n.set(next);o.modules.boundary.stack_order=next;});}Value::Undefined}
pub fn boundary_register(owner:&OwnerWeak,target:Value,options:Value,hit:bool)->Value{let owner_ref=ensure(owner);assert!(owner_ref.borrow().phase!="render"&&owner_ref.borrow().phase!="before-dispose","binding phase required");let target=target.owner();ensure(&target);let active=Rc::new(Cell::new(true));let region=NativeRegion{owner:target,active:active.clone(),options};if hit{owner_ref.borrow_mut().modules.hit.regions.push(region)}else{owner_ref.borrow_mut().modules.boundary.regions.push(region)}disposer(owner_ref,active,false)}
pub fn boundary_unregister(owner:&OwnerWeak,target:Value,hit:bool)->Value{let owner=runtime(owner);let target=target.owner();let mut o=owner.borrow_mut();let regions=if hit{&mut o.modules.hit.regions}else{&mut o.modules.boundary.regions};for region in regions.iter(){if Weak::ptr_eq(&region.owner,&target){region.active.set(false)}}regions.retain(|region|region.active.get());Value::Undefined}
pub fn boundary_subscribe(owner:&OwnerWeak,callback:Value)->Value{let owner_ref=ensure(owner);let active=Rc::new(Cell::new(true));owner_ref.borrow_mut().modules.boundary.callbacks.push((active.clone(),callback));disposer(owner_ref,active,false)}
fn classify_boundary(owner:&OwnerRef,sample:&Value)->&'static str{if !ready(owner){return "unknown"}let target=sample.member("target",true);let target=match target{Value::HostTarget(owner)|Value::Part(owner,_)=>owner.upgrade(),Value::Undefined|Value::Json(Json::Null)=>None,_=>panic!("Boundary target requires a native HostTarget")};let Some(target)=target else{return "unknown"};if descendant(&target,owner){return "inside"}let owner=owner.borrow();if owner.modules.boundary.regions.iter().any(|region|region.active.get()&&region.owner.upgrade().is_some_and(|region|ready(&region)&&descendant(&target,&region))){return "inside"}for region in [&owner.modules.overlay.trigger,&owner.modules.overlay.anchor,&owner.modules.overlay.content]{if region.upgrade().is_some_and(|region|ready(&region)&&descendant(&target,&region)){return "inside"}}"outside"}
pub fn boundary_classify(owner:&OwnerWeak,sample:Value,notify:bool)->Value{let owner=runtime(owner);let classification=classify_boundary(&owner,&sample);if notify&&classification=="outside"{let callbacks=owner.borrow().modules.boundary.callbacks.clone();let event=record(vec![("classification",Value::string("outside")),("sample",sample)]);for(active,callback)in callbacks{if active.get(){invoke(&owner,"event",&callback,vec![event.clone()]);}}}Value::string(classification)}
pub fn native_host_hit(owner:&OwnerRef,position:gpui::Point<Pixels>,window:&Window)->bool{ready(owner)&&hit_participates(owner)&&owner.borrow().native_hitbox.as_ref().is_some_and(|hitbox|hitbox.is_hovered_at(position,window))}
pub fn native_boundary_press(position:gpui::Point<Pixels>,window:&mut Window,cx:&mut App){
    let owners=owners_in(window);let target=owners.iter().filter(|owner|native_host_hit(owner,position,window)).max_by_key(|owner|owner.borrow().modules.topology.order).cloned();
    let mut candidates:Vec<_>=owners.into_iter().filter(|owner|owner.borrow().modules.boundary.observing).collect();
    let active=candidates.iter().filter(|owner|owner.borrow().modules.boundary.active).max_by_key(|owner|owner.borrow().modules.boundary.stack_order).cloned();if let Some(active)=active{candidates.retain(|owner|Rc::ptr_eq(owner,&active));}
    for owner in candidates{
        let inside=native_host_hit(&owner,position,window)||{let owner=owner.borrow();owner.modules.boundary.regions.iter().any(|region|region.active.get()&&region.owner.upgrade().is_some_and(|region|native_host_hit(&region,position,window)))||[&owner.modules.overlay.trigger,&owner.modules.overlay.anchor,&owner.modules.overlay.content].iter().any(|region|region.upgrade().is_some_and(|region|native_host_hit(&region,position,window)))};if inside{continue}
        let sample=record(vec![("type",Value::string("pointer.press")),("target",target.as_ref().map(|target|Value::HostTarget(Rc::downgrade(target))).unwrap_or(Value::Json(Json::Null)))]);
        native_host_callback(&owner,"boundary-press",||{
            overlay_outside(&owner);let callbacks=owner.borrow().modules.boundary.callbacks.clone();let event=record(vec![("classification",Value::string("outside")),("sample",sample)]);
            for(active,callback)in callbacks{if active.get(){invoke(&owner,"event",&callback,vec![event.clone()]);}}
        });
    }drive_owners(window,cx);
}
pub fn hit_participates(owner:&OwnerRef)->bool{for participant in all_owners(){let p=participant.borrow();let mode=text_or(field(&p.modules.hit.config,"mode"),"participating");if mode!="participating"&&Rc::ptr_eq(&participant,owner){return false}for region in &p.modules.hit.regions{if region.active.get()&&region.owner.upgrade().is_some_and(|target|descendant(owner,&target)){let mode=text_or(region.options.member("mode",true),&mode);if mode!="participating"{return false}}}}true}

/// A retained physical target. Native content keeps using its original handle
/// (including virtual-list item reveal); Proto observes and requests on it.
/// This type is a host API and is never exposed to authored prototype code.
#[derive(Clone)]
pub struct NativeScrollTarget(Rc<NativeScrollTargetState>);
struct NativeScrollTargetState{handle:gpui::ScrollHandle,claim:RefCell<Option<(OwnerWeak,u64)>>}
impl NativeScrollTarget{
    pub fn new(handle:gpui::ScrollHandle)->Self{Self(Rc::new(NativeScrollTargetState{handle,claim:RefCell::new(None)}))}
    pub fn same_target(&self,other:&Self)->bool{Rc::ptr_eq(&self.0,&other.0)}
}
struct NativeScrollBinding{target:NativeScrollTarget,generation:u64,window:gpui::WindowId}
/// Dropping an old lease cannot detach a replacement or write foreign offsets.
pub struct NativeScrollLease{owner:OwnerWeak,generation:u64}
impl NativeScrollLease{
    pub fn is_current(&self)->bool{self.owner.upgrade().is_some_and(|owner|{let o=owner.borrow();!o.disposed&&o.modules.scroll.external.as_ref().is_some_and(|binding|binding.generation==self.generation)})}
    /// Native wheels/touch may be consumed inside a virtualized child. This
    /// reports physical input only; it makes no authored navigation decision.
    pub fn note_input(&self,window:&mut Window)->bool{
        let Some(owner)=self.owner.upgrade()else{return false};
        if !self.is_current()||!scroll_ready(&owner)||owner.borrow().window_id!=Some(window.window_handle().window_id()){return false}
        scroll_input(&owner);window.request_animation_frame();true
    }
}
impl Drop for NativeScrollLease{fn drop(&mut self){if let Some(owner)=self.owner.upgrade(){let current=owner.borrow().modules.scroll.external.as_ref().is_some_and(|binding|binding.generation==self.generation);if current{scroll_release_external(&owner);}}}}
#[derive(Default)]
pub struct ScrollModule{pub config:BTreeMap<String,Value>,pub handle:Option<gpui::ScrollHandle>,pub last_input:Option<std::time::Instant>,pub following:bool,pub pending_end:Option<(bool,u64)>,pub request_status:String,pub previous_max:gpui::Point<Pixels>,external:Option<NativeScrollBinding>,external_expected:bool,binding_generation:u64}
impl ScrollModule{
    fn current_handle(&self)->Option<&gpui::ScrollHandle>{if self.external_expected{self.external.as_ref().map(|binding|&binding.target.0.handle)}else{self.handle.as_ref()}}
    fn handle(&self)->&gpui::ScrollHandle{self.current_handle().expect("ScrollHandle requires a current scroll target")}
    fn builtin_projection(&self)->bool{!self.external_expected}
}
fn scroll_release_external(owner:&OwnerRef){
    let binding={let mut o=owner.borrow_mut();let s=&mut o.modules.scroll;if s.pending_end.take().is_some(){s.request_status="rejected".into();}s.last_input=None;s.previous_max=gpui::Point::default();let binding=s.external.take();o.dirty=true;binding};
    if let Some(binding)=binding{let mut claim=binding.target.0.claim.borrow_mut();if claim.as_ref().is_some_and(|(claimed,generation)|claimed.ptr_eq(&Rc::downgrade(owner))&&*generation==binding.generation){*claim=None;}}
}
/// Attach an existing native surface instead of creating a second scroll box.
/// The caller retains the target identity and lease for the physical view life.
pub fn attach_scroll_surface(owner:&OwnerWeak,target:NativeScrollTarget,window:&Window)->Result<NativeScrollLease,&'static str>{
    let owner=owner.upgrade().ok_or("owner-disposed")?;
    {let o=owner.borrow();if o.disposed{return Err("owner-disposed")}if !o.modules.hooks.contains("scroll"){return Err("not-declared")}}
    let busy=target.0.claim.borrow().as_ref().is_some_and(|(claimed,generation)|claimed.upgrade().is_some_and(|claimed|{let o=claimed.borrow();!o.disposed&&o.modules.scroll.external.as_ref().is_some_and(|binding|binding.generation==*generation)}));
    if busy{return Err("target-already-attached")}
    scroll_release_external(&owner);
    let generation={let mut o=owner.borrow_mut();let s=&mut o.modules.scroll;s.binding_generation=s.binding_generation.checked_add(1).expect("scroll binding generation exhausted");s.external_expected=true;let generation=s.binding_generation;s.external=Some(NativeScrollBinding{target:target.clone(),generation,window:window.window_handle().window_id()});o.dirty=true;generation};
    *target.0.claim.borrow_mut()=Some((Rc::downgrade(&owner),generation));
    Ok(NativeScrollLease{owner:Rc::downgrade(&owner),generation})
}
/// Explicitly restore the compiler root as the physical scroll surface.
pub fn use_builtin_scroll_surface(owner:&OwnerWeak)->Result<(),&'static str>{
    let owner=owner.upgrade().ok_or("owner-disposed")?;
    {let o=owner.borrow();if o.disposed{return Err("owner-disposed")}if !o.modules.hooks.contains("scroll"){return Err("not-declared")}}
    scroll_release_external(&owner);owner.borrow_mut().modules.scroll.external_expected=false;Ok(())
}
pub fn scroll_detach(owner:&OwnerRef){if owner.borrow().modules.scroll.external.is_some(){scroll_release_external(owner);}}
fn scroll_ready(owner:&OwnerRef)->bool{
    if !ready(owner){return false}let o=owner.borrow();let s=&o.modules.scroll;
    if s.external_expected{s.external.as_ref().is_some_and(|binding|Some(binding.window)==o.window_id&&binding.target.0.handle.bounds().size.width>gpui::px(0.)&&binding.target.0.handle.bounds().size.height>gpui::px(0.))}else{s.handle.is_some()}
}
pub fn scroll_setup(owner:&OwnerWeak)->Value{let result=module_handle(owner,"scroll");setup(owner).borrow_mut().modules.scroll.handle.get_or_insert_with(gpui::ScrollHandle::default);result}
pub fn scroll_configure(owner:&OwnerWeak,patch:Value)->Value{let owner=setup(owner);let required=patch.member("requireProjection",false);assert!(required.nullish()||required.text()=="system","GPUI system ScrollHandle has no composed-chrome projection");let end=patch.member("endFollow",false);let mut o=owner.borrow_mut();if !end.nullish(){o.modules.scroll.following=text_or(end.member("mode",false),"off")=="while-at-end";}merge_config(&mut o.modules.scroll.config,patch);Value::Undefined}
pub fn scroll_sync(owner:&OwnerWeak,patch:Value)->Value{
    let owner=runtime(owner);let axes=patch.member("axes",true);if axes.nullish(){return Value::Undefined}
    let axes=axes.text();assert!(matches!(axes.as_str(),"horizontal"|"vertical"|"both"),"invalid scroll axes");
    let mut o=owner.borrow_mut();let s=&mut o.modules.scroll;if text_or(field(&s.config,"axes"),"both")==axes{return Value::Undefined}
    s.config.insert("axes".into(),Value::string(&axes));
    if s.pending_end.is_some_and(|(horizontal,_)|axes!="both"&&horizontal!=(axes=="horizontal")){s.pending_end=None;s.request_status="rejected".into();}
    o.dirty=true;Value::Undefined
}
fn scroll_axis(owner:&OwnerRef,horizontal:bool)->Value{let o=owner.borrow();let scroll=&o.modules.scroll;let (offset,max,bounds)=scroll.current_handle().map(|handle|(handle.offset(),handle.max_offset(),handle.bounds())).unwrap_or_default();let extent=f64::from(f32::from(if horizontal{max.x}else{max.y})).max(0.);let size=f64::from(f32::from(if horizontal{bounds.size.width}else{bounds.size.height}));let offset=-f64::from(f32::from(if horizontal{offset.x}else{offset.y}));record(vec![("position",Value::number(if extent>0.{(offset/extent).clamp(0.,1.)}else{0.})),("visibleRatio",Value::number(if size+extent>0.{(size/(size+extent)).clamp(0.,1.)}else{1.})),("canScrollBefore",Value::boolean(offset>0.5)),("canScrollAfter",Value::boolean(extent-offset>0.5)),("atEnd",Value::boolean(extent-offset<=1.))])}
pub fn scroll_snapshot(owner:&OwnerWeak)->Value{let owner=ensure(owner);let (axes,scrolling,state,status)={let o=owner.borrow();let s=&o.modules.scroll;let follow=field(&s.config,"endFollow");let enabled=!follow.nullish()&&text_or(follow.member("mode",false),"off")=="while-at-end";(text_or(field(&s.config,"axes"),"both"),s.last_input.is_some_and(|time|time.elapsed()<std::time::Duration::from_millis(120)),if !enabled{"off"}else if !o.mounted{"pending"}else if s.following{"following"}else{"paused"},if s.request_status.is_empty(){"idle".into()}else{s.request_status.clone()})};record(vec![("axes",Value::string(&axes)),("horizontal",scroll_axis(&owner,true)),("vertical",scroll_axis(&owner,false)),("scrolling",Value::boolean(scrolling)),("projection",Value::string(if scroll_ready(&owner){"system"}else{"unresolved"})),("endFollow",record(vec![("state",Value::string(state)),("requestStatus",Value::string(&status))]))])}
fn scroll_rejected(reason:&str)->Value{record(vec![("kind",Value::string("rejected")),("reason",Value::string(reason))])}
pub fn scroll_request(owner:&OwnerWeak,request:Value)->Value{
    let owner=runtime(owner);
    {
        let o=owner.borrow();
        if !o.modules.hooks.contains("scroll"){return scroll_rejected("not-declared")}
    }
    // Refusal paths inspect only a literal kind, without coercion or property errors.
    let to_end=match &request{
        Value::Record(record)=>record.fields.try_borrow().is_ok_and(|fields|matches!(fields.get("kind"),Some(Value::Json(Json::String(kind))) if kind=="to-end")),
        Value::Json(Json::Object(fields))=>fields.get("kind").and_then(Json::as_str)==Some("to-end"),
        _=>false,
    };
    {
        let mut o=owner.borrow_mut();
        if !o.mounted{
            if to_end{o.modules.scroll.request_status="rejected".into();}
            return scroll_rejected("not-mounted")
        }
    }
    if !scroll_ready(&owner){
        if to_end{owner.borrow_mut().modules.scroll.request_status="rejected".into();}
        return scroll_rejected("unavailable")
    }
    let axis=request.member("axis",false).text();
    assert!(axis=="horizontal"||axis=="vertical","invalid scroll axis");
    let horizontal=axis=="horizontal";
    let kind=request.member("kind",false).text();
    let mut o=owner.borrow_mut();
    let epoch=o.epoch;
    let s=&mut o.modules.scroll;
    let axes=text_or(field(&s.config,"axes"),"both");
    if axes!="both"&&axes!=axis||s.current_handle().is_none(){
        if kind=="to-end"{s.request_status="rejected".into();}
        return scroll_rejected("unavailable")
    }
    if kind=="to-end"{
        s.pending_end=Some((horizontal,epoch));
        s.request_status="pending".into();
    }else{
        let mut offset=s.handle().offset();
        let max=s.handle().max_offset();
        let bounds=s.handle().bounds();
        let max=f32::from(if horizontal{max.x}else{max.y}).max(0.);
        let current=-f32::from(if horizontal{offset.x}else{offset.y});
        let next=match kind.as_str(){
            "by"=>current+request.member("delta",false).num()as f32,
            "to"|"control-drag"=>request.member("position",false).num()as f32*max,
            "page"=>{
                let direction=request.member("direction",false).text();
                assert!(direction=="before"||direction=="after","invalid scroll page direction");
                current+if direction=="before"{-1.}else{1.}*f32::from(if horizontal{bounds.size.width}else{bounds.size.height})
            },
            _=>panic!("invalid native scroll request")
        }.clamp(0.,max);
        if horizontal{offset.x=gpui::px(-next)}else{offset.y=gpui::px(-next)}
        s.handle().set_offset(offset);
        s.following=max-next<=1.;
    }
    o.dirty=true;
    // Dispatch is delivery to this native engine, including its layout-ready
    // end movement. Queued is reserved for synchronous host attachment.
    record(vec![("kind",Value::string("dispatched"))])
}
pub fn scroll_frame(owner:&OwnerRef,window:&mut Window){
    let enabled=owner.borrow().modules.hooks.contains("scroll");
    if !enabled{return}
    if owner.borrow().modules.scroll.current_handle().is_none(){return}
    {let o=owner.borrow();if o.modules.scroll.external.as_ref().is_some_and(|binding|binding.window!=window.window_handle().window_id()||Some(binding.window)!=o.window_id){return}}
    let mut o=owner.borrow_mut();
    let epoch=o.epoch;
    let s=&mut o.modules.scroll;
    let max=s.handle().max_offset();
    let follow=field(&s.config,"endFollow");
    let configured=!follow.nullish()&&text_or(follow.member("mode",false),"off")=="while-at-end";
    let axis=if configured{text_or(follow.member("axis",false),"vertical")}else{"vertical".into()};
    let mut end=None;
    let mut retired_end=false;
    if let Some((horizontal,requested_epoch))=s.pending_end.take(){
        if requested_epoch==epoch{end=Some(horizontal)}else{s.request_status="rejected".into();retired_end=true;}
    }
    // Retirement discards this frame's growth observation; updating previous_max
    // below consumes it rather than deferring old-view movement to a later frame.
    if !retired_end&&end.is_none()&&configured&&s.following&&max!=s.previous_max{end=Some(axis=="horizontal");}
    if let Some(horizontal)=end{
        let mut offset=s.handle().offset();
        if horizontal{offset.x=-max.x}else{offset.y=-max.y}
        s.handle().set_offset(offset);
        s.request_status="applied".into();
        s.following=true;
        window.request_animation_frame();
    }
    s.previous_max=max;
    if s.last_input.is_some_and(|time|time.elapsed()<std::time::Duration::from_millis(120)){window.request_animation_frame();}else{s.last_input=None;}
}
pub fn scroll_input(owner:&OwnerRef){let mut o=owner.borrow_mut();o.modules.scroll.last_input=Some(std::time::Instant::now());o.modules.scroll.following=false;o.dirty=true;}

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
#[derive(Clone)]
struct TextListener {kind:&'static str,callback:Value,active:Rc<Cell<bool>>}
#[derive(Clone)]
struct TextEdit {value:gpui::SharedString,selection:Range<usize>,reversed:bool}
#[derive(Default)]
pub struct TextModule {
    supported:bool,multiline:bool,declared:bool,initialized:bool,controlled:bool,
    pub patch:BTreeMap<String,Value>,value:String,composing:bool,lease_epoch:u64,
    editor:gpui::SharedString,placeholder:gpui::SharedString,selection:Range<usize>,reversed:bool,marked:Option<Range<usize>>,
    listeners:Vec<TextListener>,geometry:Option<Rc<TextGeometry>>,drag_anchor:Option<usize>,
    undo:Vec<TextEdit>,redo:Vec<TextEdit>,changed:bool,
    viewport:gpui::Point<Pixels>,reveal_caret:bool,focused:bool,
}
pub struct TextGeometry {
    text:gpui::SharedString,style:gpui::TextStyle,font_size:Pixels,line_height:Pixels,
    wrap:Option<Pixels>,marked:Option<Range<usize>>,lines:Vec<gpui::WrappedLine>,
    bounds:Cell<Bounds<Pixels>>,viewport:Cell<gpui::Point<Pixels>>,content:gpui::Size<Pixels>,placeholder:bool,
}
pub fn text_declaration(owner:&OwnerWeak,multiline:bool){
    let owner=setup(owner);let mut o=owner.borrow_mut();assert!(!o.modules.image.supported,"one physical Root cannot be both TextControl and ImageView");let text=&mut o.modules.text;
    assert!(!text.supported,"duplicate TextControl declaration");text.supported=true;text.multiline=multiline;
}
pub fn text_setup(owner:&OwnerWeak)->Value{
    let owner_ref=setup(owner);{let mut o=owner_ref.borrow_mut();let text=&mut o.modules.text;
        assert!(text.supported,"TextControl requires a static declaration");assert!(!text.declared,"one TextControl per owner");text.declared=true;
        o.modules.hooks.insert("text-control");o.focusable=true;
    }Value::Handle(owner.clone(),"text-control")
}
fn canonical_text(value:&str,multiline:bool)->String{
    if !value.contains('\r')&&(multiline||!value.contains('\n')){return value.to_owned()}
    let mut result=String::with_capacity(value.len());let mut characters=value.chars().peekable();
    while let Some(character)=characters.next(){
        if character=='\r'{if characters.peek()==Some(&'\n'){characters.next();}if multiline{result.push('\n');}}
        else if character!='\n'||multiline{result.push(character);}
    }result
}
pub fn text_on(owner:&OwnerWeak,kind:Value,callback:Value)->Value{
    let owner=setup(owner);let kind=kind.text();
    let kind=match kind.as_str(){"input"=>"input","change"=>"change","compositionstart"=>"compositionstart","compositionupdate"=>"compositionupdate","compositionend"=>"compositionend",_=>panic!("invalid TextControl event")};
    let active=Rc::new(Cell::new(true));owner.borrow_mut().modules.text.listeners.push(TextListener{kind,callback,active:active.clone()});
    disposer(owner,active,true)
}
pub fn text_sync(owner:&OwnerWeak,patch:Value)->Value{
    let owner=runtime(owner);let Value::Record(entries)=patch else{panic!("TextControl patch required")};let entries=entries.fields.borrow();
    let mut o=owner.borrow_mut();let text=&mut o.modules.text;
    assert!(text.declared,"TextControl is not declared");
    let rows=entries.get("rows");let wrap=entries.get("wrap");
    assert!(text.multiline||!rows.is_some_and(|v|matches!(v,Value::Number(_)|Value::Json(Json::Number(_))))&&!wrap.is_some_and(|v|!matches!(v,Value::Undefined)),"rows/wrap require multiline TextControl");
    if !text.initialized {
        let mode=text_or(entries.get("valueMode").cloned().unwrap_or_default(),"uncontrolled");
        assert!(mode=="controlled"||mode=="uncontrolled","invalid TextControl value mode");text.controlled=mode=="controlled";
        let key=if text.controlled{"value"}else{"defaultValue"};
        text.value=canonical_text(&text_or(entries.get(key).cloned().unwrap_or_default(),""),text.multiline);
        text.initialized=true;text.editor=text.value.clone().into();text.selection=text.editor.len()..text.editor.len();
        text.reveal_caret=true;
    }
    for(key,value)in entries.iter(){
        if key=="valueMode"{continue}
        if key=="value"||key=="defaultValue" {
            if matches!(value,Value::Json(Json::String(_))){text.patch.insert(key.clone(),Value::string(&canonical_text(&value.text(),text.multiline)));}
        }else{text.patch.insert(key.clone(),value.clone());}
    }
    if text.controlled{text.value=text_or(field(&text.patch,"value"),"");}
    if entries.contains_key("placeholder"){text.placeholder=text_or(field(&text.patch,"placeholder"),"").into();}
    if !text.composing {project_text_value(text);}
    o.dirty=true;Value::Undefined
}
fn project_text_value(text:&mut TextModule){
    if text.editor.as_ref()!=text.value {
        let selection=byte_utf16(&text.editor,text.selection.start)..byte_utf16(&text.editor,text.selection.end);
        text.editor=text.value.clone().into();text.selection=utf16_range(&text.editor,selection);
        text.marked=None;text.geometry=None;
    }
}
pub fn text_snapshot(owner:&OwnerWeak)->Value{
    let owner=ensure(owner);let o=owner.borrow();let text=&o.modules.text;
    if !text.declared{return Value::Json(Json::Null)}
    record(vec![("value",Value::string(&text.value)),("composing",Value::boolean(text.composing))])
}
pub fn text_reset_value(owner:&OwnerWeak,value:Value)->Value{
    let owner=runtime(owner);
    if !ready(&owner){return Value::boolean(false)}
    if !matches!(value,Value::Undefined|Value::Json(Json::String(_))){return Value::boolean(false)}
    let mut o=owner.borrow_mut();let text=&mut o.modules.text;
    if !text.declared||!text.initialized{return Value::boolean(false)}
    let next=if text.controlled{text.value.clone()}else{
        canonical_text(&if matches!(value,Value::Undefined){text_or(field(&text.patch,"defaultValue"),"")}else{value.text()},text.multiline)
    };
    let accepted=!text.controlled;
    // A reset is an editing-lease replacement, not an input/change proposal.
    // Old IME handles and queued controlled restoration cannot write through.
    text.lease_epoch=text.lease_epoch.wrapping_add(1);text.value=next;
    text.editor=text.value.clone().into();text.selection=text.editor.len()..text.editor.len();
    text.reversed=false;text.marked=None;text.composing=false;text.drag_anchor=None;text.geometry=None;
    text.undo.clear();text.redo.clear();text.changed=false;text.viewport=Default::default();text.reveal_caret=true;
    o.dirty=true;Value::boolean(accepted)
}
pub fn text_detach(owner:&OwnerRef){
    let mut o=owner.borrow_mut();let text=&mut o.modules.text;
    text.lease_epoch=text.lease_epoch.wrapping_add(1);text.composing=false;text.marked=None;text.drag_anchor=None;text.geometry=None;
    text.viewport=Default::default();text.focused=false;text.reveal_caret=true;
    project_text_value(text);
}
pub fn text_supported(owner:&OwnerRef)->bool{owner.borrow().modules.text.supported}
pub fn text_disabled(owner:&OwnerRef)->bool{bool_or(field(&owner.borrow().modules.text.patch,"disabled"),false)}
/// Only the current mounted text owner can claim composition keys. A retired
/// view's retained editor is not an IME session in another native view.
pub fn text_is_composing(owner:&OwnerRef)->bool{
    if !ready(owner){return false}let o=owner.borrow();o.modules.text.supported&&o.modules.text.composing
}
/// Apply a source-authored UTF-16 selection request to this exact live editor.
/// Selection itself is permitted in readonly text, but cannot disturb a marked
/// range, a disabled owner, another window, or a detached editing lease.
pub fn text_select_utf16(owner:&OwnerRef,start:usize,end:usize,window:&mut Window,cx:&mut App)->bool{
    if !ready(owner)||!text_supported(owner)||text_disabled(owner){return false}
    let focus={let mut o=owner.borrow_mut();
        if o.window_id!=Some(window.window_handle().window_id()){return false}
        let text=&mut o.modules.text;if text.composing{return false}
        text.selection=utf16_range(&text.editor,start..end.max(start));text.reversed=false;
        text.drag_anchor=None;text.reveal_caret=true;o.dirty=true;o.modules.focus.native.clone()
    };
    if let Some(focus)=focus{window.notify_text_selection_changed(&focus);}
    text_drive(owner,window,cx);true
}
pub fn text_accessible(owner:&OwnerRef,node:&mut gpui::accesskit::Node){
    let o=owner.borrow();let text=&o.modules.text;if !text.supported{return}
    node.set_value(text.editor.as_ref());
    if !text.placeholder.is_empty(){node.set_placeholder(text.placeholder.as_ref());}
    if bool_or(field(&text.patch,"required"),false){node.set_required();}
    if bool_or(field(&text.patch,"disabled"),false){node.set_disabled();}
    if bool_or(field(&text.patch,"readOnly"),false){node.set_read_only();}
    if text.multiline&&node.role()==gpui::accesskit::Role::TextInput{node.set_role(gpui::accesskit::Role::MultilineTextInput);}
}
fn text_editable(owner:&OwnerRef)->bool{
    let o=owner.borrow();let text=&o.modules.text;
    !bool_or(field(&text.patch,"disabled"),false)&&!bool_or(field(&text.patch,"readOnly"),false)
}
fn byte_boundary(text:&str,index:usize,ceil:bool)->usize{
    let mut index=index.min(text.len());while !text.is_char_boundary(index){if ceil{index+=1}else{index-=1}}index
}
fn utf16_byte(text:&str,index:usize,ceil:bool)->usize{
    let mut units=0;for(byte,ch)in text.char_indices(){if units==index{return byte}let next=units+ch.len_utf16();if next>index{return if ceil{byte+ch.len_utf8()}else{byte}}units=next;}text.len()
}
fn byte_utf16(text:&str,index:usize)->usize{text[..byte_boundary(text,index,false)].encode_utf16().count()}
fn utf16_range(text:&str,range:Range<usize>)->Range<usize>{
    let start=utf16_byte(text,range.start,false);let end=if range.is_empty(){start}else{utf16_byte(text,range.end,true).max(start)};start..end
}
fn text_event(owner:&OwnerRef,kind:&str,value:&str,composing:bool,data:Option<&str>,input_type:Option<&str>,window:&mut Window,cx:&mut App){
    let(epoch,controlled,listeners)={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        text.composing=composing;if !text.controlled&&kind=="input"{text.value=value.to_owned();}
        o.dirty=true;let text=&o.modules.text;(text.lease_epoch,text.controlled,text.listeners.len())};
    let event=record(vec![("type",Value::string(kind)),("value",Value::string(value)),("composing",Value::boolean(composing)),
        ("data",data.map(Value::string).unwrap_or(Value::Json(Json::Null))),("inputType",input_type.map(Value::string).unwrap_or(Value::Json(Json::Null)))]);
    for index in 0..listeners {
        if owner.borrow().disposed||!owner.borrow().mounted||!owner.borrow().present||owner.borrow().modules.text.lease_epoch!=epoch{break}
        let callback=owner.borrow().modules.text.listeners.get(index).filter(|listener|listener.active.get()&&listener.kind==kind).map(|listener|listener.callback.clone());
        if let Some(callback)=callback{if native_host_callback(owner,"text-event",||invoke(owner,"text-event",&callback,vec![capability(&Rc::downgrade(owner)),event.clone()])).is_none(){break}}
    }
    if controlled&&(kind=="input"&&!composing||kind=="compositionend") {
        let weak=Rc::downgrade(owner);window.defer(cx,move|window,cx|{
            if let Some(owner)=weak.upgrade().filter(|owner|{let o=owner.borrow();!o.disposed&&o.mounted&&o.present&&o.modules.text.lease_epoch==epoch}){
                {let mut o=owner.borrow_mut();if !o.modules.text.composing{project_text_value(&mut o.modules.text);}o.dirty=true;}
                text_drive(&owner,window,cx);
            }
        });
    }
}
fn text_drive(owner:&OwnerRef,window:&mut Window,cx:&mut App){let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx)}}
fn remember_text_edit(text:&mut TextModule){text.undo.push(TextEdit{value:text.editor.clone(),selection:text.selection.clone(),reversed:text.reversed});text.redo.clear();}
fn text_insertion(text:&TextModule,range:&Range<usize>,input:&str,marked:bool)->String{
    let mut input=canonical_text(input,text.multiline);
    let limit=field(&text.patch,"maxLength");
    if !marked&&!limit.nullish()&&limit.num()>=0. {
        let retained=text.editor[..range.start].encode_utf16().count()+text.editor[range.end..].encode_utf16().count();
        let available=(limit.num()as usize).saturating_sub(retained);let end=utf16_byte(&input,available,false);input.truncate(end);
    }
    input
}
fn replace_editor(text:&mut TextModule,range:Range<usize>,input:String)->String{
    let mut next=text.editor.to_string();next.replace_range(range.clone(),&input);text.editor=next.into();
    let caret=range.start+input.len();text.selection=caret..caret;text.reversed=false;text.geometry=None;text.changed=true;text.reveal_caret=true;input
}
pub fn text_blur(owner:&OwnerRef,window:&mut Window,cx:&mut App){
    if !text_supported(owner)||owner.borrow().disposed{return}
    let(value,composing,changed)={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        text.marked=None;text.drag_anchor=None;(text.editor.clone(),text.composing,std::mem::take(&mut text.changed))};
    if composing{text_event(owner,"compositionend",&value,false,None,None,window,cx);}
    if changed&&!owner.borrow().disposed&&owner.borrow().mounted&&owner.borrow().present{text_event(owner,"change",&value,false,None,None,window,cx);}
}
pub struct NativeTextInput {owner:OwnerWeak,epoch:u64}
impl NativeTextInput {
    fn replace(&mut self,range:Option<Range<usize>>,input:&str,input_type:Option<&str>,window:&mut Window,cx:&mut App){
        let Some(owner)=self.live().filter(text_editable)else{return};
        let(value,data,composing)={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
            let range=range.map(|range|utf16_range(&text.editor,range)).or_else(||text.marked.clone()).unwrap_or_else(||text.selection.clone());
            let composing=text.composing;let data=text_insertion(text,&range,input,false);
            if data.is_empty()&&range.is_empty()&&!composing{return}
            if !composing{remember_text_edit(text);}let data=replace_editor(text,range,data);text.marked=None;
            (text.editor.clone(),data,composing)};
        if composing{text_event(&owner,"compositionend",&value,false,Some(&data),None,window,cx);}
        if self.live().is_some(){text_event(&owner,"input",&value,false,if data.is_empty(){None}else{Some(&data)},if composing{Some("insertCompositionText")}else{input_type.or(if data.is_empty(){None}else{Some("insertText")})},window,cx);}
        text_drive(&owner,window,cx);
    }
    fn live(&self)->Option<OwnerRef>{self.owner.upgrade().filter(|owner|{let o=owner.borrow();!o.disposed&&o.mounted&&o.present&&o.modules.text.lease_epoch==self.epoch})}
}
impl gpui::InputHandler for NativeTextInput {
    fn selected_text_range(&mut self,ignore_disabled:bool,_:&mut Window,_:&mut App)->Option<gpui::UTF16Selection>{
        let owner=self.live()?;let o=owner.borrow();let text=&o.modules.text;
        if !ignore_disabled&&bool_or(field(&text.patch,"disabled"),false){return None}
        Some(gpui::UTF16Selection{range:byte_utf16(&text.editor,text.selection.start)..byte_utf16(&text.editor,text.selection.end),reversed:text.reversed})
    }
    fn marked_text_range(&mut self,_:&mut Window,_:&mut App)->Option<Range<usize>>{
        let owner=self.live()?;let o=owner.borrow();let text=&o.modules.text;let range=text.marked.as_ref()?;
        Some(byte_utf16(&text.editor,range.start)..byte_utf16(&text.editor,range.end))
    }
    fn text_for_range(&mut self,range:Range<usize>,adjusted:&mut Option<Range<usize>>,_:&mut Window,_:&mut App)->Option<String>{
        let owner=self.live()?;let o=owner.borrow();let text=&o.modules.text;let range=utf16_range(&text.editor,range);
        *adjusted=Some(byte_utf16(&text.editor,range.start)..byte_utf16(&text.editor,range.end));Some(text.editor[range].to_owned())
    }
    fn replace_text_in_range(&mut self,range:Option<Range<usize>>,input:&str,window:&mut Window,cx:&mut App){
        self.replace(range,input,None,window,cx);
    }
    fn paste(&mut self,item:gpui::ClipboardItem,window:&mut Window,cx:&mut App){
        if let Some(input)=item.text(){self.replace(None,&input,Some("insertFromPaste"),window,cx);}
    }
    fn replace_and_mark_text_in_range(&mut self,range:Option<Range<usize>>,input:&str,selected:Option<Range<usize>>,window:&mut Window,cx:&mut App){
        let Some(owner)=self.live().filter(text_editable)else{return};
        let(value,data,started)={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
            let range=range.map(|range|utf16_range(&text.editor,range)).or_else(||text.marked.clone()).unwrap_or_else(||text.selection.clone());
            let start=range.start;let started=!text.composing;if started{remember_text_edit(text);}
            let data=text_insertion(text,&range,input,true);let data=replace_editor(text,range,data);text.marked=Some(start..start+data.len());
            if let Some(selected)=selected{let selected=utf16_range(&data,selected);text.selection=start+selected.start..start+selected.end;}
            (text.editor.clone(),data,started)};
        if started{text_event(&owner,"compositionstart",&value,true,Some(&data),None,window,cx);}
        if self.live().is_some(){text_event(&owner,"compositionupdate",&value,true,Some(&data),None,window,cx);}
        if self.live().is_some(){text_event(&owner,"input",&value,true,Some(&data),Some("insertCompositionText"),window,cx);}
        text_drive(&owner,window,cx);
    }
    fn unmark_text(&mut self,window:&mut Window,cx:&mut App){
        let Some(owner)=self.live()else{return};let(value,composing,data)={let mut o=owner.borrow_mut();let text=&mut o.modules.text;let data=text.marked.take().map(|range|text.editor[range].to_owned());(text.editor.clone(),text.composing,data)};
        if composing{text_event(&owner,"compositionend",&value,false,data.as_deref(),None,window,cx);text_drive(&owner,window,cx);}
    }
    fn bounds_for_range(&mut self,range:Range<usize>,_:&mut Window,_:&mut App)->Option<Bounds<Pixels>>{
        let owner=self.live()?;let o=owner.borrow();let text=&o.modules.text;let geometry=text.geometry.as_ref()?;
        let range=utf16_range(&text.editor,range);let start=geometry.position(range.start);let end=geometry.position(range.end);
        Some(Bounds::from_corners(start,gpui::point(if start.y==end.y{end.x.max(start.x+gpui::px(1.))}else{geometry.bounds.get().right()},end.y+geometry.line_height)))
    }
    fn character_index_for_point(&mut self,point:gpui::Point<Pixels>,_:&mut Window,_:&mut App)->Option<usize>{
        let owner=self.live()?;let o=owner.borrow();let text=&o.modules.text;Some(byte_utf16(&text.editor,text.geometry.as_ref()?.index(point)))
    }
    fn set_selected_text_range(&mut self,range:Range<usize>,window:&mut Window,cx:&mut App){
        let Some(owner)=self.live()else{return};{let mut o=owner.borrow_mut();let text=&mut o.modules.text;text.selection=utf16_range(&text.editor,range);text.reversed=false;text.reveal_caret=true;o.dirty=true;}text_drive(&owner,window,cx);
    }
    fn element_bounds(&mut self,_:&mut Window,_:&mut App)->Option<Bounds<Pixels>>{let owner=self.live()?;let o=owner.borrow();Some(o.modules.text.geometry.as_ref()?.bounds.get())}
    fn text_length_utf16(&mut self,_:&mut Window,_:&mut App)->Option<usize>{Some(self.live()?.borrow().modules.text.editor.encode_utf16().count())}
    fn accepts_text_input(&mut self,_:&mut Window,_:&mut App)->bool{self.live().is_some_and(|owner|text_editable(&owner))}
    fn prefers_ime_for_printable_keys(&mut self,_:&mut Window,_:&mut App)->bool{true}
    fn text_input_configuration(&mut self,_:&mut Window,_:&mut App)->gpui::TextInputConfiguration{
        let Some(owner)=self.live()else{return gpui::TextInputConfiguration::default()};let o=owner.borrow();let text=&o.modules.text;
        let input_action=match text_or(field(&text.patch,"enterKeyHint"),"").as_str(){
            "enter"=>gpui::TextInputAction::Enter,"done"=>gpui::TextInputAction::Done,"go"=>gpui::TextInputAction::Go,"next"=>gpui::TextInputAction::Next,
            "previous"=>gpui::TextInputAction::Previous,"search"=>gpui::TextInputAction::Search,"send"=>gpui::TextInputAction::Send,_=>gpui::TextInputAction::Unspecified};
        gpui::TextInputConfiguration{input_action,..Default::default()}
    }
}
impl TextGeometry {
    fn content_position(&self,index:usize)->gpui::Point<Pixels>{
        let mut start=0;let mut y=gpui::px(0.);for line in &self.lines{
            if index<=start+line.len(){return line.position_for_index(index.saturating_sub(start),self.line_height).unwrap_or_default()+gpui::point(gpui::px(0.),y)}
            start+=line.len()+1;y+=line.size(self.line_height).height;
        }gpui::point(gpui::px(0.),y)
    }
    fn position(&self,index:usize)->gpui::Point<Pixels>{self.bounds.get().origin+self.content_position(index)-self.viewport.get()}
    fn index(&self,point:gpui::Point<Pixels>)->usize{
        if self.placeholder{return 0}let point=point-self.bounds.get().origin+self.viewport.get();let mut start=0;let mut y=gpui::px(0.);
        for(index,line)in self.lines.iter().enumerate(){let height=line.size(self.line_height).height;
            if point.y<y+height||index+1==self.lines.len(){return start+line.closest_index_for_position(gpui::point(point.x,point.y-y),self.line_height).unwrap_or_else(|index|index)}
            start+=line.len()+1;y+=height;
        }self.text.len()
    }
    fn clamp_viewport(&self,viewport:gpui::Point<Pixels>)->gpui::Point<Pixels>{
        let bounds=self.bounds.get();gpui::point(viewport.x.max(gpui::px(0.)).min((self.content.width+gpui::px(1.)-bounds.size.width).max(gpui::px(0.))),viewport.y.max(gpui::px(0.)).min((self.content.height-bounds.size.height).max(gpui::px(0.))))
    }
}
fn update_text_viewport(text:&mut TextModule,geometry:&Rc<TextGeometry>,bounds:Bounds<Pixels>,focused:bool){
    geometry.bounds.set(bounds);let mut viewport=geometry.clamp_viewport(text.viewport);
    if focused&&(text.reveal_caret||!text.focused){
        let caret=geometry.content_position(if text.reversed{text.selection.start}else{text.selection.end});
        if caret.x<viewport.x{viewport.x=caret.x}else if caret.x+gpui::px(1.)>viewport.x+bounds.size.width{viewport.x=caret.x+gpui::px(1.)-bounds.size.width}
        if caret.y<viewport.y{viewport.y=caret.y}else if caret.y+geometry.line_height>viewport.y+bounds.size.height{viewport.y=caret.y+geometry.line_height-bounds.size.height}
        viewport=geometry.clamp_viewport(viewport);text.reveal_caret=false;
    }
    text.focused=focused;text.viewport=viewport;geometry.viewport.set(viewport);
}
pub struct NativeTextElement {owner:OwnerWeak,focus:FocusHandle,fill_height:bool}
impl NativeTextElement {pub fn new(owner:&OwnerRef,focus:&FocusHandle,fill_height:bool)->Self{Self{owner:Rc::downgrade(owner),focus:focus.clone(),fill_height}}}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for NativeTextElement {type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for NativeTextElement {
    type RequestLayoutState=();type PrepaintState=Rc<TextGeometry>;
    fn id(&self)->Option<gpui::ElementId>{None}
    fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){
        let owner=self.owner.upgrade().expect("live text owner");let o=owner.borrow();let text=&o.modules.text;
        let rows=if text.multiline{let rows=field(&text.patch,"rows");if rows.nullish(){2.}else{rows.num().floor().max(1.)}}else{1.};
        let mut style=gpui::Style::default();style.size.width=gpui::relative(1.).into();style.size.height=if self.fill_height{gpui::relative(1.).into()}else{(window.line_height()*rows as f32).into()};
        (window.request_layout(style,[],cx),())
    }
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,bounds:Bounds<Pixels>,_:&mut (),window:&mut Window,_:&mut App)->Rc<TextGeometry>{
        let owner=self.owner.upgrade().expect("live text owner");let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        let placeholder=text.editor.is_empty();let display=if placeholder{text.placeholder.clone()}else{text.editor.clone()};
        let style=window.text_style();let font_size=style.font_size.to_pixels(window.rem_size());let line_height=window.line_height();
        let wrap=if text.multiline{Some(bounds.size.width)}else{None};
        if let Some(cache)=text.geometry.clone(){if cache.text==display&&cache.style==style&&cache.font_size==font_size&&cache.line_height==line_height&&cache.wrap==wrap&&cache.marked==text.marked&&cache.placeholder==placeholder{update_text_viewport(text,&cache,bounds,self.focus.is_focused(window));return cache}}
        let run=gpui::TextRun{len:display.len(),font:style.font(),color:if placeholder{gpui::hsla(0.,0.,0.5,0.7)}else{style.color},background_color:None,underline:None,strikethrough:None};
        let mut runs=Vec::with_capacity(if text.marked.is_some(){3}else{1});
        if let Some(marked)=text.marked.as_ref().filter(|_|!placeholder){
            if marked.start>0{runs.push(gpui::TextRun{len:marked.start,..run.clone()});}
            if !marked.is_empty(){runs.push(gpui::TextRun{len:marked.end-marked.start,underline:Some(gpui::UnderlineStyle{color:Some(run.color),thickness:gpui::px(1.),wavy:false}),..run.clone()});}
            if marked.end<display.len(){runs.push(gpui::TextRun{len:display.len()-marked.end,..run});}
        }else{runs.push(run);}
        let lines=window.text_system().shape_text(display.clone(),font_size,&runs,wrap,None).expect("native text shaping").into_vec();
        let content=lines.iter().fold(gpui::size(gpui::px(0.),gpui::px(0.)),|content,line|{let size=line.size(line_height);gpui::size(content.width.max(size.width),content.height+size.height)});
        let geometry=Rc::new(TextGeometry{text:display,style,font_size,line_height,wrap,marked:text.marked.clone(),lines,bounds:Cell::new(bounds),viewport:Cell::new(text.viewport),content,placeholder});
        update_text_viewport(text,&geometry,bounds,self.focus.is_focused(window));
        text.geometry=Some(geometry.clone());geometry
    }
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,bounds:Bounds<Pixels>,_:&mut (),geometry:&mut Rc<TextGeometry>,window:&mut Window,cx:&mut App){
        let owner=self.owner.upgrade().expect("live text owner");let(selection,epoch)={let o=owner.borrow();let text=&o.modules.text;(text.selection.clone(),text.lease_epoch)};
        window.handle_input(&self.focus,NativeTextInput{owner:self.owner.clone(),epoch},cx);
        let origin=bounds.origin-geometry.viewport.get();
        window.with_content_mask(Some(gpui::ContentMask{bounds}),|window|{
            if self.focus.is_focused(window) {
                if selection.is_empty(){let position=geometry.position(selection.end);window.paint_quad(gpui::fill(Bounds::new(position,gpui::size(gpui::px(1.),geometry.line_height)),geometry.style.color));}
                else{let mut offset=0;let mut y=origin.y;for line in &geometry.lines{
                    let start=selection.start.saturating_sub(offset).min(line.len());let end=selection.end.saturating_sub(offset).min(line.len());
                    if start<end{let mut row_start=start;let mut from=line.position_for_index(start,geometry.line_height).unwrap_or_default();
                        for(byte,_)in line.text.char_indices().filter(|(byte,_)|*byte>start&&*byte<end).chain(std::iter::once((end,'\0'))){
                            let to=line.position_for_index(byte,geometry.line_height).unwrap_or_default();
                            if to.y!=from.y{window.paint_quad(gpui::fill(Bounds::new(gpui::point(origin.x+from.x,y+from.y),gpui::size((bounds.size.width-from.x).max(gpui::px(1.)),geometry.line_height)),gpui::rgba(0x3366cc44)));row_start=byte;from=to;}
                            if byte==end&&row_start<end{window.paint_quad(gpui::fill(Bounds::new(gpui::point(origin.x+from.x,y+from.y),gpui::size((to.x-from.x).max(gpui::px(1.)),geometry.line_height)),gpui::rgba(0x3366cc44)));}
                        }
                    }offset+=line.len()+1;y+=line.size(geometry.line_height).height;
                }}
            }
            let mut y=origin.y;for line in &geometry.lines{line.paint(gpui::point(origin.x,y),geometry.line_height,gpui::TextAlign::Left,None,window,cx).expect("native text paint");y+=line.size(geometry.line_height).height;}
        });
    }
}
pub fn text_scroll(owner:&OwnerRef,event:&gpui::ScrollWheelEvent,window:&mut Window,cx:&mut App)->bool{
    let changed={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        let Some(geometry)=text.geometry.as_ref()else{return false};
        if !geometry.bounds.get().contains(&event.position)||bool_or(field(&text.patch,"disabled"),false){return false}
        let next=geometry.clamp_viewport(text.viewport-event.delta.pixel_delta(geometry.line_height));
        if next==text.viewport{false}else{text.viewport=next;geometry.viewport.set(next);text.reveal_caret=false;o.dirty=true;true}
    };if changed{text_drive(owner,window,cx);}changed
}
pub fn text_key_down(owner:&OwnerRef,event:&gpui::KeyDownEvent,window:&mut Window,cx:&mut App)->bool{
    if !text_supported(owner)||text_disabled(owner){return false}
    // The platform IME owns candidate navigation, deletion and confirmation.
    // App-level editing must not rewrite the active provisional text.
    if text_is_composing(owner){return true}
    let named=crate::gpui_native_key::named_key(&event.keystroke.key);let key=named.as_deref().unwrap_or(event.keystroke.key.as_str());
    let command=event.keystroke.modifiers.platform||event.keystroke.modifiers.control;
    if command&&matches!(key,"c"|"C"|"x"|"X"){
        let selection={let o=owner.borrow();let text=&o.modules.text;text.editor[text.selection.clone()].to_owned()};
        if !selection.is_empty(){cx.write_to_clipboard(gpui::ClipboardItem::new_string(selection));if matches!(key,"x"|"X")&&text_editable(owner){let epoch=owner.borrow().modules.text.lease_epoch;NativeTextInput{owner:Rc::downgrade(owner),epoch}.replace(None,"",Some("deleteByCut"),window,cx);}}return true
    }
    if command&&matches!(key,"v"|"V"){
        if let Some(item)=cx.read_from_clipboard(){if let Some(input)=item.text(){let epoch=owner.borrow().modules.text.lease_epoch;NativeTextInput{owner:Rc::downgrade(owner),epoch}.replace(None,&input,Some("insertFromPaste"),window,cx);}}return true
    }
    if command&&matches!(key,"z"|"Z"|"y"|"Y")&&text_editable(owner){
        let redo=event.keystroke.modifiers.shift||matches!(key,"y"|"Y");let next={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
            let next=if redo{text.redo.pop()}else{text.undo.pop()};if let Some(next)=next{let current=TextEdit{value:text.editor.clone(),selection:text.selection.clone(),reversed:text.reversed};if redo{text.undo.push(current)}else{text.redo.push(current)}text.editor=next.value;text.selection=next.selection;text.reversed=next.reversed;text.marked=None;text.composing=false;text.geometry=None;text.changed=true;text.reveal_caret=true;Some(text.editor.clone())}else{None}};
        if let Some(value)=next{text_event(owner,"input",&value,false,None,Some(if redo{"historyRedo"}else{"historyUndo"}),window,cx);text_drive(owner,window,cx);}return true
    }
    if key=="Enter"{
        if owner.borrow().modules.text.multiline {let epoch=owner.borrow().modules.text.lease_epoch;NativeTextInput{owner:Rc::downgrade(owner),epoch}.replace(None,"\n",Some("insertLineBreak"),window,cx);}
        else{let value={let mut o=owner.borrow_mut();let text=&mut o.modules.text;if std::mem::take(&mut text.changed){Some(text.editor.clone())}else{None}};if let Some(value)=value{text_event(owner,"change",&value,false,None,None,window,cx);text_drive(owner,window,cx);}}return true
    }
    if command&&matches!(key,"a"|"A"){let mut o=owner.borrow_mut();let text=&mut o.modules.text;text.selection=0..text.editor.len();text.reversed=false;text.reveal_caret=true;o.dirty=true;drop(o);text_drive(owner,window,cx);return true}
    if !matches!(key,"ArrowLeft"|"ArrowRight"|"ArrowUp"|"ArrowDown"|"Home"|"End"|"Backspace"|"Delete"){return false}
    let deleting=key=="Backspace"||key=="Delete";if deleting&&!text_editable(owner){return true}
    let range={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        let caret=if text.reversed{text.selection.start}else{text.selection.end};
        let previous=||text.editor[..caret].grapheme_indices(true).next_back().map(|(i,_)|i).unwrap_or(0);
        let next=||text.editor[caret..].grapheme_indices(true).nth(1).map(|(i,_)|caret+i).unwrap_or(text.editor.len());
        let target=match key{"ArrowLeft"|"Backspace"=>if command{text.editor[..caret].unicode_word_indices().next_back().map(|(i,_)|i).unwrap_or(0)}else{previous()},"ArrowRight"|"Delete"=>if command{text.editor[caret..].unicode_word_indices().next().map(|(i,word)|caret+i+word.len()).unwrap_or(text.editor.len())}else{next()},
            "Home"=>if command{0}else{text.editor[..caret].rfind('\n').map(|i|i+1).unwrap_or(0)},"End"=>if command{text.editor.len()}else{text.editor[caret..].find('\n').map(|i|caret+i).unwrap_or(text.editor.len())},
            "ArrowUp"|"ArrowDown"=>text.geometry.as_ref().map(|geometry|{let position=geometry.position(caret);geometry.index(position+gpui::point(gpui::px(0.),if key=="ArrowUp"{-geometry.line_height}else{geometry.line_height}))}).unwrap_or(caret),_=>caret};
        if deleting{if text.selection.is_empty(){Some(caret.min(target)..caret.max(target))}else{Some(text.selection.clone())}}
        else{let target=if !event.keystroke.modifiers.shift&&!text.selection.is_empty()&&matches!(key,"ArrowLeft"|"ArrowRight"){if key=="ArrowLeft"{text.selection.start}else{text.selection.end}}else{target};
            let anchor=if event.keystroke.modifiers.shift{if text.reversed{text.selection.end}else{text.selection.start}}else{target};text.selection=anchor.min(target)..anchor.max(target);text.reversed=target<anchor;text.reveal_caret=true;o.dirty=true;None}
    };
    if let Some(range)=range{let epoch=owner.borrow().modules.text.lease_epoch;let range={let o=owner.borrow();let text=&o.modules.text;byte_utf16(&text.editor,range.start)..byte_utf16(&text.editor,range.end)};NativeTextInput{owner:Rc::downgrade(owner),epoch}.replace(Some(range),"",Some(if key=="Delete"{"deleteContentForward"}else{"deleteContentBackward"}),window,cx);return true}
    text_drive(owner,window,cx);true
}
pub fn text_pointer_down(owner:&OwnerRef,event:&gpui::MouseDownEvent,window:&mut Window,cx:&mut App){
    if !text_supported(owner)||text_disabled(owner)||event.button!=gpui::MouseButton::Left{return}
    {let mut o=owner.borrow_mut();let text=&mut o.modules.text;let Some(geometry)=&text.geometry else{return};let target=geometry.index(event.position);
        let(anchor,target)=if event.click_count>=3{(text.editor[..target].rfind('\n').map(|i|i+1).unwrap_or(0),text.editor[target..].find('\n').map(|i|target+i).unwrap_or(text.editor.len()))}
        else if event.click_count==2{text.editor.unicode_word_indices().find(|(start,word)|target>=*start&&target<=start+word.len()).map(|(start,word)|(start,start+word.len())).unwrap_or((target,target))}
        else{(if event.modifiers.shift{if text.reversed{text.selection.end}else{text.selection.start}}else{target},target)};
        text.selection=anchor.min(target)..anchor.max(target);text.reversed=target<anchor;text.drag_anchor=Some(anchor);text.reveal_caret=true;o.dirty=true;
    }
    note_focus_intent_in(owner,Some(window.window_handle().window_id()));queue_focus(owner,"pointer",false,false);text_drive(owner,window,cx);
}
pub fn text_pointer_move(owner:&OwnerRef,event:&gpui::MouseMoveEvent,window:&mut Window,cx:&mut App){
    if !text_supported(owner){return}let changed={let mut o=owner.borrow_mut();let text=&mut o.modules.text;
        if event.pressed_button!=Some(gpui::MouseButton::Left){text.drag_anchor=None;false}
        else if let(Some(anchor),Some(geometry))=(text.drag_anchor,text.geometry.as_ref()){let target=geometry.index(event.position);text.selection=anchor.min(target)..anchor.max(target);text.reversed=target<anchor;text.reveal_caret=true;o.dirty=true;true}else{false}};
    if changed{text_drive(owner,window,cx);}
}
#[cfg(all(test, feature="test-support"))]
mod text_owner_tests {
    use super::*;
    use gpui::{AppContext, InputHandler, Render, Context, TestAppContext, WindowHandle};

    struct Empty;
    impl Render for Empty {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{div()}
    }
    fn open(cx:&mut TestAppContext)->WindowHandle<Empty>{
        cx.update(|cx|cx.open_window(Default::default(),|_,cx|cx.new(|_|Empty)).unwrap())
    }
    fn text_owner(window:&Window,value:&str)->OwnerRef{
        text_owner_mode(window,value,false)
    }
    fn text_owner_mode(window:&Window,value:&str,controlled:bool)->OwnerRef{
        let owner=Owner::new(serde_json::json!({}),Default::default(),|_,_|true,|_|true,|_,_|true,crate::gpui_native_style::vocabulary());
        let weak=Rc::downgrade(&owner);text_declaration(&weak,false);text_setup(&weak);
        {let mut o=owner.borrow_mut();o.phase="callback";o.mounted=true;o.window_id=Some(window.window_handle().window_id());o.bounds=Some(Bounds::default());}
        text_sync(&weak,record(vec![(if controlled{"value"}else{"defaultValue"},Value::string(value)),("valueMode",Value::string(if controlled{"controlled"}else{"uncontrolled"}))]));owner
    }
    fn input(owner:&OwnerRef)->NativeTextInput{NativeTextInput{owner:Rc::downgrade(owner),epoch:owner.borrow().modules.text.lease_epoch}}

    #[gpui::test]
    fn source_selection_snaps_utf16_and_obeys_exact_live_owner(cx:&mut TestAppContext){
        let window=open(cx);let other=open(cx);
        let owner=window.update(cx,|_,window,cx|{
            let owner=text_owner(window,"a😀候b");
            assert!(text_select_utf16(&owner,2,3,window,cx));
            assert_eq!(input(&owner).selected_text_range(false,window,cx).unwrap().range,1..3);
            assert!(text_select_utf16(&owner,2,2,window,cx));
            assert_eq!(input(&owner).selected_text_range(false,window,cx).unwrap().range,1..1);
            assert!(text_select_utf16(&owner,999,1000,window,cx));
            assert_eq!(input(&owner).selected_text_range(false,window,cx).unwrap().range,5..5);
            text_sync(&Rc::downgrade(&owner),record(vec![("readOnly",Value::boolean(true))]));
            assert!(text_select_utf16(&owner,0,5,window,cx),"readonly text remains selectable");
            text_sync(&Rc::downgrade(&owner),record(vec![("disabled",Value::boolean(true))]));
            assert!(!text_select_utf16(&owner,0,0,window,cx));
            text_sync(&Rc::downgrade(&owner),record(vec![("disabled",Value::boolean(false))]));
            owner
        }).unwrap();
        other.update(cx,|_,window,cx|assert!(!text_select_utf16(&owner,0,0,window,cx))).unwrap();
        window.update(cx,|_,window,cx|{
            owner.borrow_mut().mounted=false;
            assert!(!text_select_utf16(&owner,0,0,window,cx));
            owner.borrow_mut().mounted=true;Owner::dispose(&owner);
            assert!(!text_select_utf16(&owner,0,0,window,cx));
        }).unwrap();
    }

    #[gpui::test]
    fn composition_keys_cannot_edit_but_live_ime_can_replace(cx:&mut TestAppContext){
        let window=open(cx);
        window.update(cx,|_,window,cx|{
            let owner=text_owner(window,"old");let mut editor=input(&owner);
            editor.replace_and_mark_text_in_range(Some(0..3),"候😀",Some(3..3),window,cx);
            assert!(text_is_composing(&owner));
            let before=owner.borrow().modules.text.editor.clone();
            let selection=editor.selected_text_range(false,window,cx).unwrap().range;
            assert!(!text_select_utf16(&owner,0,0,window,cx));
            for key in ["enter","escape","left","right","home","end","backspace","delete","ctrl-z","ctrl-v"]{
                let key=gpui::Keystroke::parse(key).unwrap();
                assert!(text_key_down(&owner,&gpui::KeyDownEvent{keystroke:key,is_held:false},window,cx));
                assert_eq!(owner.borrow().modules.text.editor,before);
                assert_eq!(editor.selected_text_range(false,window,cx).unwrap().range,selection);
                assert!(text_is_composing(&owner));
            }
            editor.replace_text_in_range(None,"选😀",window,cx);
            assert!(!text_is_composing(&owner));
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"选😀");
            text_detach(&owner);owner.borrow_mut().mounted=false;
            assert!(!text_is_composing(&owner));
            owner.borrow_mut().mounted=true;
            editor.replace_text_in_range(None,"stale",window,cx);
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"选😀","retired IME lease cannot edit a remount");
            Owner::dispose(&owner);
        }).unwrap();
    }

    #[gpui::test]
    fn reset_replaces_editing_lease_and_keeps_controlled_authority(cx:&mut TestAppContext){
        let window=open(cx);
        window.update(cx,|_,window,cx|{
            let owner=text_owner(window,"seed");let mut old=input(&owner);
            old.replace_and_mark_text_in_range(Some(0..4),"候选",Some(2..2),window,cx);
            assert!(text_is_composing(&owner));
            assert!(text_reset_value(&Rc::downgrade(&owner),Value::string("fresh\r\n😀")).bool());
            assert!(!text_is_composing(&owner));
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"fresh😀");
            old.replace_text_in_range(None,"stale",window,cx);
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"fresh😀");
            let undo=gpui::KeyDownEvent{keystroke:gpui::Keystroke::parse("ctrl-z").unwrap(),is_held:false};
            text_key_down(&owner,&undo,window,cx);
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"fresh😀","reset cannot undo into the retired draft");
            assert!(text_reset_value(&Rc::downgrade(&owner),Value::Undefined).bool());
            assert_eq!(text_snapshot(&Rc::downgrade(&owner)).member("value",false).text(),"seed");

            let controlled=text_owner_mode(window,"truth",true);let mut old_controlled=input(&controlled);
            old_controlled.replace_and_mark_text_in_range(Some(0..5),"candidate",Some(9..9),window,cx);
            assert!(!text_reset_value(&Rc::downgrade(&controlled),Value::string("ignored")).bool());
            assert!(!text_is_composing(&controlled));
            assert_eq!(text_snapshot(&Rc::downgrade(&controlled)).member("value",false).text(),"truth");
            old_controlled.replace_text_in_range(None,"stale",window,cx);
            assert_eq!(controlled.borrow().modules.text.editor.as_ref(),"truth");
            controlled.borrow_mut().mounted=false;
            assert!(!text_reset_value(&Rc::downgrade(&controlled),Value::string("detached")).bool());
            Owner::dispose(&owner);Owner::dispose(&controlled);
        }).unwrap();
    }
}


#[derive(Clone)]
struct ImageListener {callback:Value,active:Rc<Cell<bool>>}
#[derive(Clone)]
enum ImageLocation {Resource(gpui::Resource),Inline(std::sync::Arc<gpui::Image>),Invalid(gpui::ImageCacheError)}
pub struct ImageModule {
    pub supported:bool,declared:bool,requested:String,pub source:String,alternative:String,mode:&'static str,
    pub fit:&'static str,pub status:&'static str,generation:u64,lease_epoch:u64,attached:bool,
    last_hosted:Option<u64>,scheduled:Option<(u64,u64)>,location:Option<ImageLocation>,last_diagnostic:String,
    pub rendered:Option<std::sync::Arc<gpui::RenderImage>>,listeners:Vec<ImageListener>,
}
impl Default for ImageModule {fn default()->Self{Self{supported:false,declared:false,requested:String::new(),source:String::new(),alternative:String::new(),mode:"informative",fit:"contain",status:"idle",generation:0,lease_epoch:0,attached:false,last_hosted:None,scheduled:None,location:None,last_diagnostic:String::new(),rendered:None,listeners:Vec::new()}}}
struct InlineImageAsset;
impl gpui::Asset for InlineImageAsset {
    type Source=std::sync::Arc<gpui::Image>;
    type Output=Result<std::sync::Arc<gpui::RenderImage>,gpui::ImageCacheError>;
    fn load(source:Self::Source,cx:&mut App)->impl std::future::Future<Output=Self::Output>+Send+'static{
        let renderer=cx.svg_renderer();async move{source.to_image_data(renderer).map_err(Into::into)}
    }
}
fn image_location(source:&str)->ImageLocation {
    if source.get(..5).is_some_and(|prefix|prefix.eq_ignore_ascii_case("data:")){
        let decoded=(||{
            let data=data_url::DataUrl::process(source).map_err(|error|gpui::ImageCacheError::Asset(error.to_string().into()))?;
            let(bytes,_)=data.decode_to_vec().map_err(|error|gpui::ImageCacheError::Asset(error.to_string().into()))?;
            // Match the pinned GPUI resource loader: content recognition, then real SVG decoding.
            let format=image::guess_format(&bytes).ok().and_then(|format|gpui::ImageFormat::from_mime_type(format.to_mime_type())).unwrap_or(gpui::ImageFormat::Svg);
            Ok(std::sync::Arc::new(gpui::Image::from_bytes(format,bytes)))
        })();return match decoded{Ok(image)=>ImageLocation::Inline(image),Err(error)=>ImageLocation::Invalid(error)}
    }
    if std::path::Path::new(source).is_absolute(){return ImageLocation::Resource(gpui::Resource::Path(std::path::PathBuf::from(source).into()))}
    let gpui::ImageSource::Resource(resource)=gpui::ImageSource::from(source)else{unreachable!()};ImageLocation::Resource(resource)
}
fn image_has_alternative(value:&str)->bool {
    !value.trim_matches(|character|matches!(character,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')).is_empty()
}
fn image_source_valid(image:&mut ImageModule)->bool {
    if image.requested.is_empty(){image.last_diagnostic.clear();return false}
    let alternative=image_has_alternative(&image.alternative);
    if image.mode=="informative"&&alternative||image.mode=="decorative"&&!alternative{image.last_diagnostic.clear();return true}
    let diagnostic=format!("{}:{}:{}",image.mode,image.requested,image.alternative);
    if diagnostic!=image.last_diagnostic{image.last_diagnostic=diagnostic;eprintln!("[ImageView] rejected contradictory or missing accessibility input; source failed closed to idle.");}
    false
}
fn image_fit(value:Value)->&'static str{match value.text().as_str(){"contain"=>"contain","cover"=>"cover","fill"=>"fill",_=>panic!("invalid ImageView fit")}}
fn image_mode(value:Value)->&'static str{match value.text().as_str(){"informative"=>"informative","decorative"=>"decorative",_=>panic!("invalid ImageView accessibility mode")}}
pub fn image_declaration(owner:&OwnerWeak,config:Value){
    let owner=setup(owner);let mut o=owner.borrow_mut();assert!(!o.modules.text.supported,"one physical Root cannot be both TextControl and ImageView");let image=&mut o.modules.image;
    assert!(!image.supported,"duplicate ImageView declaration");image.supported=true;image.requested=config.member("source",false).text();image.alternative=config.member("alternativeText",false).text();image.mode=image_mode(config.member("a11yMode",false));image.fit=image_fit(config.member("fit",false));
    if image_source_valid(image){image.source=image.requested.clone();image.generation=1;image.status="loading";image.location=Some(image_location(&image.source));}
}
pub fn image_setup(owner:&OwnerWeak)->Value{
    let owner_ref=setup(owner);{let mut o=owner_ref.borrow_mut();let image=&mut o.modules.image;assert!(image.supported,"ImageView requires a static declaration");assert!(!image.declared,"one ImageView per owner");image.declared=true;o.modules.hooks.insert("image-view");}Value::Handle(owner.clone(),"image-view")
}
pub fn image_on(owner:&OwnerWeak,kind:Value,callback:Value)->Value{
    let owner=setup(owner);assert!(kind.text()=="loadingStatusChange","invalid ImageView event");let active=Rc::new(Cell::new(true));owner.borrow_mut().modules.image.listeners.push(ImageListener{callback,active:active.clone()});disposer(owner,active,true)
}
fn image_transition(owner:&OwnerRef,status:&'static str){
    let event={let mut o=owner.borrow_mut();let image=&mut o.modules.image;if image.status==status{return}let previous=image.status;image.status=status;o.dirty=true;let image=&o.modules.image;record(vec![("status",Value::string(status)),("previousStatus",Value::string(previous)),("source",Value::string(&image.source))])};
    let listeners=owner.borrow().modules.image.listeners.len();for index in 0..listeners{
        if owner.borrow().disposed{break}
        let callback=owner.borrow().modules.image.listeners.get(index).filter(|listener|listener.active.get()).map(|listener|listener.callback.clone());
        if let Some(callback)=callback{invoke(owner,"image-event",&callback,vec![capability(&Rc::downgrade(owner)),event.clone()]);}
    }
}
pub fn image_sync(owner:&OwnerWeak,patch:Value)->Value{
    let owner=runtime(owner);assert!(matches!(patch,Value::Record(_)|Value::Json(Json::Object(_))),"ImageView patch required");
    let source=patch.member("source",true);let alternative=patch.member("alternativeText",true);let mode=patch.member("a11yMode",true);let fit=patch.member("fit",true);
    let changed={let mut o=owner.borrow_mut();let image=&mut o.modules.image;assert!(image.declared,"ImageView is not declared");
        if matches!(source,Value::Json(Json::String(_))){image.requested=source.text();}if matches!(alternative,Value::Json(Json::String(_))){image.alternative=alternative.text();}if !mode.nullish(){image.mode=image_mode(mode);}if !fit.nullish(){image.fit=image_fit(fit);}
        let valid=image_source_valid(image);let source=if valid{image.requested.as_str()}else{""};let changed=source!=image.source;
        if changed{image.source=source.to_owned();image.generation=image.generation.wrapping_add(1);image.location=if image.source.is_empty(){None}else{Some(image_location(&image.source))};image.rendered=None;image.scheduled=None;}
        if image.attached{image.last_hosted=Some(image.generation);}o.dirty=true;changed
    };
    // loadingStatus is a module observation, never an authored status override.
    if changed{let status=if owner.borrow().modules.image.source.is_empty(){"idle"}else{"loading"};image_transition(&owner,status);}Value::Undefined
}
pub fn image_snapshot(owner:&OwnerWeak)->Value{
    let owner=ensure(owner);let o=owner.borrow();let image=&o.modules.image;if !image.declared{return Value::Json(Json::Null)}
    record(vec![("source",Value::string(&image.source)),("loadingStatus",Value::string(image.status)),("fit",Value::string(image.fit))])
}
pub fn image_mount(owner:&OwnerRef){
    let reload={let mut o=owner.borrow_mut();let image=&mut o.modules.image;if !image.declared||image.attached{return}image.attached=true;image.lease_epoch=image.lease_epoch.wrapping_add(1);image.scheduled=None;
        let reload=!image.source.is_empty()&&image.last_hosted==Some(image.generation);if reload{image.generation=image.generation.wrapping_add(1);}image.last_hosted=Some(image.generation);o.dirty=true;reload};
    if reload{image_transition(owner,"loading");}
}
pub fn image_detach(owner:&OwnerRef){let mut o=owner.borrow_mut();let image=&mut o.modules.image;image.attached=false;image.lease_epoch=image.lease_epoch.wrapping_add(1);image.scheduled=None;}
pub fn image_supported(owner:&OwnerRef)->bool{owner.borrow().modules.image.supported}
pub fn image_accessible(owner:&OwnerRef,node:&mut gpui::accesskit::Node){
    let o=owner.borrow();let image=&o.modules.image;if !image.supported{return}
    if image.mode=="decorative"{node.clear_label();node.set_hidden();}else{node.set_label(image.alternative.as_str());}
}
fn image_complete(owner:&OwnerWeak,generation:u64,epoch:u64,status:&'static str,window:&mut Window,cx:&mut App){
    let Some(owner)=owner.upgrade().filter(|owner|{let o=owner.borrow();let image=&o.modules.image;!o.disposed&&o.mounted&&o.present&&image.attached&&image.generation==generation&&image.lease_epoch==epoch&&!image.source.is_empty()&&image.status=="loading"})else{return};
    native_host_callback(&owner,"image-completion",||image_transition(&owner,status));let drive=owner.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx)}
}
pub fn image_element(owner:&OwnerRef,fill_height:bool)->AnyElement{
    use gpui::StyledImage as _;
    let(location,generation,epoch,fit)={let o=owner.borrow();let image=&o.modules.image;(image.location.clone(),image.generation,image.lease_epoch,image.fit)};
    let Some(location)=location else{return div().into_any_element()};let owner=Rc::downgrade(owner);
    let mut element=gpui::img(move|window:&mut Window,cx:&mut App|{
        let owner_ref=owner.upgrade().filter(|owner|{let o=owner.borrow();let image=&o.modules.image;!o.disposed&&o.mounted&&o.present&&image.attached&&image.generation==generation&&image.lease_epoch==epoch})?;
        let result=match &location{ImageLocation::Resource(resource)=>window.use_asset::<gpui::ImgResourceLoader>(resource,cx),ImageLocation::Inline(image)=>window.use_asset::<InlineImageAsset>(image,cx),ImageLocation::Invalid(error)=>Some(Err(error.clone()))};
        if let Some(result)=&result{
            let schedule={let mut o=owner_ref.borrow_mut();let image=&mut o.modules.image;image.rendered=result.as_ref().ok().cloned();if image.status=="loading"&&image.scheduled!=Some((generation,epoch)){image.scheduled=Some((generation,epoch));true}else{false}};
            if schedule{let owner=owner.clone();let status=if result.is_ok(){"loaded"}else{"error"};window.defer(cx,move|window,cx|image_complete(&owner,generation,epoch,status,window,cx));}
        }result
    }).object_fit(match fit{"contain"=>gpui::ObjectFit::Contain,"cover"=>gpui::ObjectFit::Cover,"fill"=>gpui::ObjectFit::Fill,_=>unreachable!()}).w_full();
    if fill_height{element=element.h_full();}element.into_any_element()
}

#[derive(Clone,Copy,PartialEq)]
struct PositionConfig {
    side:&'static str,align:&'static str,strategy:&'static str,
    side_offset:f32,align_offset:f32,padding:f32,collisions:bool,viewport:bool,exclude_translation:bool,
}
impl PositionConfig {
    fn read(value:&Value)->Self {
        let side=match value.member("side",false).text().as_str(){"top"=>"top","right"=>"right","bottom"=>"bottom","left"=>"left",_=>panic!("invalid anchored side")};
        let align=match value.member("align",false).text().as_str(){"start"=>"start","center"=>"center","end"=>"end",_=>panic!("invalid anchored alignment")};
        let strategy=match value.member("strategy",false).text().as_str(){"absolute"=>"absolute","fixed"=>"fixed",_=>panic!("invalid anchored strategy")};
        let viewport=match value.member("collisionBoundary",false).text().as_str(){"viewport"=>true,"clippingAncestors"=>false,_=>panic!("invalid anchored collision boundary")};
        let finite=|key:&str|{let number=value.member(key,false).num()as f32;assert!(number.is_finite(),"native anchored distance must be finite");number};
        Self{side,align,strategy,side_offset:finite("sideOffset"),align_offset:finite("alignOffset"),padding:finite("collisionPadding"),collisions:value.member("avoidCollisions",false).bool(),viewport,exclude_translation:bool_or(value.member("excludeAnchorTranslation",false),false)}
    }
}
#[derive(Clone,Copy,PartialEq)]
struct PositionGeometry {anchor:Bounds<Pixels>,floating:gpui::Size<Pixels>,clipping:Bounds<Pixels>}
struct PositionConnection {anchor:OwnerWeak,floating:OwnerWeak,config:PositionConfig,callback:Value,generation:u64,requested:bool,geometry:Option<PositionGeometry>}
#[derive(Default)]
pub struct PositioningModule {connection:Option<PositionConnection>,snapshot:Value,generation:u64}
thread_local!{static POSITION_DIRTY:Cell<bool>=Cell::new(false);static POSITION_CONTROLLERS:RefCell<Vec<OwnerWeak>>=RefCell::new(Vec::new());}
fn position_changed(){POSITION_DIRTY.with(|dirty|dirty.set(true));}
pub fn positioning_drive(window:&mut Window){if POSITION_DIRTY.with(|dirty|dirty.replace(false)){window.refresh();}}
pub fn positioning_connect(owner:&OwnerWeak,connection:Value)->Value {
    let owner=ensure(owner);assert!(!matches!(owner.borrow().phase,"render"|"before-dispose"),"positioning binding phase required");
    let anchor=connection.member("anchor",false).owner();let floating=connection.member("floating",false).owner();
    ensure(&anchor);ensure(&floating);let config=PositionConfig::read(&connection.member("config",false));let callback=connection.member("onResolved",false);
    assert!(matches!(callback,Value::Undefined|Value::Function(_)),"anchored resolution requires a callable");
    let weak=Rc::downgrade(&owner);let mut owner=owner.borrow_mut();let position=&mut owner.modules.positioning;
    let registered=position.connection.is_some();
    let same=position.connection.as_ref().is_some_and(|old|Weak::ptr_eq(&old.anchor,&anchor)&&Weak::ptr_eq(&old.floating,&floating));
    position.generation=position.generation.wrapping_add(1);
    if !same{position.snapshot=Value::Json(Json::Null);}
    position.connection=Some(PositionConnection{anchor,floating,config,callback,generation:position.generation,requested:true,geometry:None});
    drop(owner);if !registered{POSITION_CONTROLLERS.with(|controllers|controllers.borrow_mut().push(weak));}position_changed();Value::Undefined
}
pub fn positioning_update(owner:&OwnerWeak,config:Value)->Value {
    let owner=runtime(owner);if owner.borrow().modules.positioning.connection.is_none(){return Value::Undefined}
    let config=PositionConfig::read(&config);let mut owner=owner.borrow_mut();let position=&mut owner.modules.positioning;
    position.generation=position.generation.wrapping_add(1);let generation=position.generation;
    let connection=position.connection.as_mut().expect("connected anchored position");connection.config=config;connection.generation=generation;connection.requested=true;
    drop(owner);position_changed();Value::Undefined
}
pub fn positioning_request(owner:&OwnerWeak)->Value {
    let owner=runtime(owner);if let Some(connection)=owner.borrow_mut().modules.positioning.connection.as_mut(){connection.requested=true;position_changed();}Value::Undefined
}
pub fn positioning_detach(owner:&OwnerRef){let weak=Rc::downgrade(owner);let mut owner=owner.borrow_mut();let position=&mut owner.modules.positioning;let connected=position.connection.take().is_some();position.snapshot=Value::Json(Json::Null);position.generation=position.generation.wrapping_add(1);drop(owner);if connected{POSITION_CONTROLLERS.with(|controllers|controllers.borrow_mut().retain(|controller|!Weak::ptr_eq(controller,&weak)));position_changed();}}
pub fn positioning_disconnect(owner:&OwnerWeak)->Value{let owner=runtime(owner);positioning_detach(&owner);Value::Undefined}
pub fn positioning_snapshot(owner:&OwnerWeak)->Value{let owner=ensure(owner);let value=owner.borrow().modules.positioning.snapshot.clone();if matches!(value,Value::Undefined){Value::Json(Json::Null)}else{value}}
fn position_controller(floating:&OwnerRef)->Option<OwnerRef>{let floating=Rc::downgrade(floating);POSITION_CONTROLLERS.with(|controllers|{let mut controllers=controllers.borrow_mut();controllers.retain(|controller|controller.strong_count()!=0);controllers.iter().rev().filter_map(Weak::upgrade).find(|owner|{let owner=owner.borrow();!owner.disposed&&owner.modules.positioning.connection.as_ref().is_some_and(|connection|Weak::ptr_eq(&connection.floating,&floating))})})}
fn placed_origin(anchor:Bounds<Pixels>,floating:gpui::Size<Pixels>,config:PositionConfig,side:&str,align:&str)->gpui::Point<Pixels>{
    let vertical=side=="top"||side=="bottom";
    let mut origin=gpui::point(anchor.origin.x+(anchor.size.width-floating.width)/2.,anchor.origin.y+(anchor.size.height-floating.height)/2.);
    match side{"top"=>origin.y=anchor.top()-floating.height-gpui::px(config.side_offset),"bottom"=>origin.y=anchor.bottom()+gpui::px(config.side_offset),"left"=>origin.x=anchor.left()-floating.width-gpui::px(config.side_offset),"right"=>origin.x=anchor.right()+gpui::px(config.side_offset),_=>unreachable!()}
    if vertical{if align=="start"{origin.x=anchor.left();}else if align=="end"{origin.x=anchor.right()-floating.width;}origin.x+=gpui::px(config.align_offset);}
    else{if align=="start"{origin.y=anchor.top();}else if align=="end"{origin.y=anchor.bottom()-floating.height;}origin.y+=gpui::px(config.align_offset);}
    origin
}
fn overflow(bounds:Bounds<Pixels>,clipping:Bounds<Pixels>,side:&str,align:&str,reference:gpui::Size<Pixels>)->[f32;3]{
    let top:f32=(clipping.top()-bounds.top()).into();let right:f32=(bounds.right()-clipping.right()).into();let bottom:f32=(bounds.bottom()-clipping.bottom()).into();let left:f32=(clipping.left()-bounds.left()).into();
    let vertical=side=="top"||side=="bottom";
    let primary_end=(align=="start")^if vertical{reference.width>bounds.size.width}else{reference.height>bounds.size.height};
    let(cross,opposite)=if vertical{if primary_end{(right,left)}else{(left,right)}}else if primary_end{(bottom,top)}else{(top,bottom)};
    match side{"top"=>[top,cross,opposite],"bottom"=>[bottom,cross,opposite],"left"=>[left,cross,opposite],"right"=>[right,cross,opposite],_=>unreachable!()}
}
fn resolve_position(geometry:PositionGeometry,config:PositionConfig)->(gpui::Point<Pixels>,&'static str,&'static str){
    let mut side=config.side;let mut align=config.align;let mut origin=placed_origin(geometry.anchor,geometry.floating,config,side,align);
    if !config.collisions{return(origin,side,align)}
    let opposite=match side{"top"=>"bottom","bottom"=>"top","left"=>"right","right"=>"left",_=>unreachable!()};
    let alternate=match align{"start"=>"end","end"=>"start",_=>"center"};
    let candidates=if align=="center"{[(side,align),(opposite,align),(opposite,align),(opposite,align)]}else{[(side,align),(side,alternate),(opposite,align),(opposite,alternate)]};
    let count=if align=="center"{2}else{4};let mut best=f32::INFINITY;let mut main_fit=None;
    for(candidate_side,candidate_align)in candidates.into_iter().take(count) {
        let candidate=placed_origin(geometry.anchor,geometry.floating,config,candidate_side,candidate_align);let values=overflow(Bounds::new(candidate,geometry.floating),geometry.clipping,candidate_side,candidate_align,geometry.anchor.size);
        let score=values.iter().map(|value|value.max(0.)).sum::<f32>();
        if score<best{best=score;origin=candidate;side=candidate_side;align=candidate_align;}
        if values.iter().all(|value|*value<=0.){main_fit=None;break}
        if values[0]<=0.&&main_fit.as_ref().is_none_or(|(primary,_,_,_)|values[1]<*primary){main_fit=Some((values[1],candidate,candidate_side,candidate_align));}
    }
    // flip first prefers a fitting main axis and its primary alignment side;
    // total positive overflow is only the fallback when no main axis fits.
    if let Some((_,candidate,candidate_side,candidate_align))=main_fit{origin=candidate;side=candidate_side;align=candidate_align;}
    // Floating UI shift's default main axis is the axis along the alignment.
    if side=="top"||side=="bottom"{origin.x=origin.x.max(geometry.clipping.left()).min((geometry.clipping.right()-geometry.floating.width).max(geometry.clipping.left()));}
    else{origin.y=origin.y.max(geometry.clipping.top()).min((geometry.clipping.bottom()-geometry.floating.height).max(geometry.clipping.top()));}
    (origin,side,align)
}
struct PositionedRoot {element:AnyElement,owner:OwnerWeak,controller:OwnerWeak}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for PositionedRoot{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for PositionedRoot {
    type RequestLayoutState=gpui::LayoutId;type PrepaintState=();
    fn id(&self)->Option<gpui::ElementId>{None}
    fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,gpui::LayoutId){
        let child=self.element.request_layout(window,cx);let layout=window.request_layout(gpui::Style{position:gpui::Position::Absolute,display:gpui::Display::Flex,..Default::default()},[child],cx);(layout,child)
    }
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,child:&mut gpui::LayoutId,window:&mut Window,cx:&mut App){
        let Some(controller)=self.controller.upgrade().filter(|owner|!owner.borrow().disposed)else{self.element.prepaint(window,cx);return};
        let Some(floating)=self.owner.upgrade()else{return};
        let Some((anchor,config,generation,callback))=controller.borrow().modules.positioning.connection.as_ref().filter(|connection|Weak::ptr_eq(&connection.floating,&self.owner)).map(|connection|(connection.anchor.clone(),connection.config,connection.generation,connection.callback.clone()))else{self.element.prepaint(window,cx);return};
        let Some(anchor)=anchor.upgrade().filter(ready)else{self.element.prepaint(window,cx);return};
        let mut anchor_bounds=anchor.borrow().bounds.expect("ready native anchor bounds");if config.exclude_translation{anchor_bounds.origin-=anchor.borrow().native_translation;}
        let child_bounds=window.layout_bounds(*child);let viewport=Bounds::new(gpui::Point::default(),window.viewport_size());
        let mut clipping=if config.viewport{viewport}else{window.content_mask().bounds.intersect(&viewport)};
        let padding=gpui::px(config.padding);clipping.origin+=gpui::point(padding,padding);clipping.size.width=(clipping.size.width-padding*2.).max(gpui::px(0.));clipping.size.height=(clipping.size.height-padding*2.).max(gpui::px(0.));
        let geometry=PositionGeometry{anchor:anchor_bounds,floating:child_bounds.size,clipping};let(origin,side,align)=resolve_position(geometry,config);
        window.with_element_offset(origin-child_bounds.origin,|window|self.element.prepaint(window,cx));
        let notify={let mut controller=controller.borrow_mut();let Some(connection)=controller.modules.positioning.connection.as_mut().filter(|connection|connection.generation==generation)else{return};let notify=connection.requested||connection.geometry!=Some(geometry);connection.requested=false;connection.geometry=Some(geometry);notify};
        if notify{let owner=self.controller.clone();let floating=Rc::downgrade(&floating);let anchor=Rc::downgrade(&anchor);let epoch=controller.borrow().epoch;window.defer(cx,move|window,cx|{
            let Some(controller)=owner.upgrade().filter(|owner|ready(owner)&&owner.borrow().epoch==epoch)else{return};
            if !floating.upgrade().is_some_and(|owner|ready(&owner))||!anchor.upgrade().is_some_and(|owner|ready(&owner)){return}
            if !controller.borrow().modules.positioning.connection.as_ref().is_some_and(|connection|connection.generation==generation){return}
            let snapshot=record(vec![("side",Value::string(side)),("align",Value::string(align)),("strategy",Value::string(config.strategy))]);controller.borrow_mut().modules.positioning.snapshot=snapshot.clone();
            if matches!(callback,Value::Function(_)){native_host_callback(&controller,"positioning",||invoke(&controller,"positioning",&callback,vec![snapshot]));}
            let drive=controller.borrow().native_drive.clone();if let Some(drive)=drive{drive(window,cx);}
        });}
    }
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut gpui::LayoutId,_:&mut (),window:&mut Window,cx:&mut App){self.element.paint(window,cx)}
}
pub fn positioning_root(owner:&OwnerRef,element:AnyElement,source_fixed:bool)->(AnyElement,bool) {
    let Some(controller)=position_controller(owner)else{return(element,source_fixed)};
    let fixed=controller.borrow().modules.positioning.connection.as_ref().is_some_and(|connection|connection.config.strategy=="fixed");
    let element=PositionedRoot{element,owner:Rc::downgrade(owner),controller:Rc::downgrade(&controller)}.into_any_element();
    (if fixed{gpui::deferred(element).into_any_element()}else{element},false)
}

#[derive(Default)]
pub struct OverlayModule {
    config:BTreeMap<String,Value>,open:bool,reason:Value,view_active:bool,retained:bool,presence:Value,
    presence_active:Option<Rc<Cell<bool>>>,trigger:OwnerWeak,anchor:OwnerWeak,content:OwnerWeak,
    materialized:bool,modal_locked:bool,layer_order:u64,
}
thread_local!{static OVERLAY_LAYERS:Cell<u64>=Cell::new(0);static OVERLAY_OWNERS:RefCell<Vec<OwnerWeak>>=RefCell::new(Vec::new());static POINTER_PRESS_DISPATCHED:Cell<bool>=Cell::new(false);}
fn overlay_boolean(owner:&OwnerRef,key:&str)->bool{bool_or(field(&owner.borrow().modules.overlay.config,key),false)}
fn overlay_position_config(owner:&OwnerRef)->Value {
    let owner=owner.borrow();let config=&owner.modules.overlay.config;
    record(vec![("side",Value::string(&text_or(field(config,"placement"),"bottom"))),("align",Value::string(&text_or(field(config,"align"),"start"))),
        ("sideOffset",config.get("sideOffset").cloned().unwrap_or(Value::number(4.))),("alignOffset",config.get("alignOffset").cloned().unwrap_or(Value::number(0.))),
        ("strategy",Value::string(&text_or(field(config,"strategy"),"absolute"))),("avoidCollisions",config.get("avoidCollisions").cloned().unwrap_or(Value::boolean(true))),
        ("collisionBoundary",Value::string(&text_or(field(config,"collisionBoundary"),"clippingAncestors"))),("collisionPadding",config.get("collisionPadding").cloned().unwrap_or(Value::number(0.))),
        ("excludeAnchorTranslation",config.get("excludeAnchorTranslation").cloned().unwrap_or(Value::boolean(false)))])
}
fn overlay_view_active(owner:&OwnerRef,active:bool){
    let changed={let mut owner=owner.borrow_mut();let overlay=&mut owner.modules.overlay;if overlay.view_active==active{false}else{overlay.view_active=active;if !active{overlay.modal_locked=false;}owner.dirty=true;true}};
    if changed&&!active{positioning_detach(owner);}if changed{position_changed();}
}
pub fn overlay_detach(owner:&OwnerRef){let mut owner=owner.borrow_mut();let overlay=&mut owner.modules.overlay;overlay.materialized=false;overlay.modal_locked=false;overlay.layer_order=0;}
fn overlay_reconcile(owner:&OwnerRef){
    if !ready(owner)||!owner.borrow().modules.hooks.contains("overlay"){return}
    let active=owner.borrow().modules.overlay.view_active;if !active{return}
    {let mut owner=owner.borrow_mut();let overlay=&mut owner.modules.overlay;if !overlay.materialized{overlay.materialized=true;OVERLAY_LAYERS.with(|sequence|{overlay.layer_order=sequence.get();sequence.set(sequence.get().wrapping_add(1));});owner.dirty=true;position_changed();}}
    let modal=overlay_boolean(owner,"modal");owner.borrow_mut().modules.overlay.modal_locked=modal;
    let (anchor,floating)={let owner=owner.borrow();let overlay=&owner.modules.overlay;(overlay.anchor.upgrade().or_else(||overlay.trigger.upgrade()),overlay.content.upgrade())};
    if !overlay_boolean(owner,"anchored")||anchor.is_none(){positioning_detach(owner);return}
    let anchor=anchor.expect("anchored overlay reference");let floating=floating.unwrap_or_else(||owner.clone());let config=overlay_position_config(owner);let parsed=PositionConfig::read(&config);
    let same=owner.borrow().modules.positioning.connection.as_ref().is_some_and(|connection|Weak::ptr_eq(&connection.anchor,&Rc::downgrade(&anchor))&&Weak::ptr_eq(&connection.floating,&Rc::downgrade(&floating))&&connection.config==parsed);
    if !same{positioning_connect(&Rc::downgrade(owner),record(vec![("anchor",Value::HostTarget(Rc::downgrade(&anchor))),("floating",Value::HostTarget(Rc::downgrade(&floating))),("config",config)]));}
}
pub fn overlay_commit(owner:&OwnerRef){overlay_reconcile(owner);}
fn overlay_drive_presence(owner:&OwnerRef,open:bool){
    let(presence,retained)={let owner=owner.borrow();(owner.modules.overlay.presence.clone(),owner.modules.overlay.retained)};
    if matches!(presence,Value::Undefined){overlay_view_active(owner,open);if !retained{set_present(&Rc::downgrade(owner),Value::boolean(open));}}
    else{presence.member(if open{"enter"}else{"leave"},false).call_authored(vec![]);}
}
fn overlay_set_open(owner:&OwnerRef,open:bool,reason:Value){
    let changed={let mut owner=owner.borrow_mut();let overlay=&mut owner.modules.overlay;overlay.reason=reason;let changed=overlay.open!=open;overlay.open=open;if changed{owner.dirty=true;}changed};
    if !changed{if open{overlay_reconcile(owner);}return}
    if owner.borrow().phase!="setup"{overlay_drive_presence(owner,open);}
    // Observed next events belong to each intent, not the enclosing callback's final value.
    poll_state_watchers();
    let current=owner.borrow().modules.overlay.open;let mut owner=owner.borrow_mut();owner.modules.boundary.active=current;
    if current{BOUNDARY_ORDER.with(|sequence|{owner.modules.boundary.stack_order=sequence.get().wrapping_add(1);sequence.set(owner.modules.boundary.stack_order);});}
}
pub fn overlay_intent(owner:&OwnerWeak,operation:&str,reason:Value)->Value{let owner=runtime(owner);let open=match operation{"openOverlay"=>true,"close"=>false,"toggle"=>!owner.borrow().modules.overlay.open,_=>panic!("unknown Overlay intent")};overlay_set_open(&owner,open,reason);Value::Undefined}
pub fn overlay_is_open(owner:&OwnerWeak)->Value{Value::boolean(ensure(owner).borrow().modules.overlay.open)}
pub fn overlay_configure(owner:&OwnerWeak,patch:Value,position_only:bool)->Value{
    let owner=if position_only{runtime(owner)}else{setup(owner)};
    let keys=if position_only{&["placement","align","sideOffset","alignOffset","strategy","avoidCollisions","collisionBoundary","collisionPadding","excludeAnchorTranslation"][..]}else{&["defaultOpen","closeOnEscape","closeOnOutsidePress","closeOnFocusOutside","closeOnAnchorPress","closeOnTriggerPress","placement","align","sideOffset","alignOffset","anchored","strategy","avoidCollisions","collisionBoundary","collisionPadding","excludeAnchorTranslation","entry","restore","portal","modal","layerRole","layerOffset"][..]};
    {let mut owner=owner.borrow_mut();for key in keys{let value=patch.member(key,false);if !matches!(value,Value::Undefined){owner.modules.overlay.config.insert((*key).into(),value);}}
        let meta=patch.member("meta",false);if !position_only&&!matches!(meta,Value::Undefined){let mut merged=BTreeMap::new();let previous=field(&owner.modules.overlay.config,"meta");if !previous.nullish(){merge_config(&mut merged,previous);}merge_config(&mut merged,meta);owner.modules.overlay.config.insert("meta".into(),record_map(merged));}}
    if !position_only{let observing=overlay_boolean(&owner,"closeOnOutsidePress");owner.borrow_mut().modules.boundary.observing=observing;if overlay_boolean(&owner,"defaultOpen"){overlay_set_open(&owner,true,Value::string("programmatic"));}}
    else{overlay_reconcile(&owner);}Value::Undefined
}
fn overlay_lifecycle(owner:&OwnerWeak,kind:&str)->Value{
    let owner=ensure(owner);
    match kind{
        "created"=>{let presence=owner.borrow().modules.overlay.presence.clone();let open=owner.borrow().modules.overlay.open;if matches!(presence,Value::Undefined){overlay_drive_presence(&owner,open);}},
        "presence-created"=>{let presence=owner.borrow().modules.overlay.presence.clone();let open=owner.borrow().modules.overlay.open;overlay_view_active(&owner,presence.member("present",false).get().bool());presence.member(if open{"enter"}else{"leave"},false).call_authored(vec![]);},
        "mounted"=>{owner.borrow_mut().modules.overlay.content=Rc::downgrade(&owner);overlay_reconcile(&owner);},
        "unmounted"=>overlay_detach(&owner),
        "before-dispose"=>{if let Some(active)=owner.borrow_mut().modules.overlay.presence_active.take(){active.set(false);}overlay_detach(&owner);},
        _=>panic!("unknown Overlay lifecycle phase"),
    }Value::Undefined
}
pub fn overlay_setup(owner:&OwnerWeak)->Value{
    let owner_ref=setup(owner);if owner_ref.borrow().modules.hooks.contains("overlay"){return Value::Handle(owner.clone(),"overlay")}
    let value=module_handle(owner,"overlay");OVERLAY_OWNERS.with(|owners|owners.borrow_mut().push(owner.clone()));
    for kind in ["created","mounted","unmounted","before-dispose"]{let weak=owner.clone();register_life(owner,kind,Value::Function(Rc::new(move|_|overlay_lifecycle(&weak,kind))));}
    value
}
pub fn overlay_register(owner:&OwnerWeak,target:Value,kind:&str)->Value{
    let owner=ensure(owner);assert!(!matches!(owner.borrow().phase,"render"|"before-dispose"),"Overlay binding phase required");
    let target=if target.nullish(){OwnerWeak::new()}else{let target=target.owner();ensure(&target);target};
    {let mut owner=owner.borrow_mut();let overlay=&mut owner.modules.overlay;match kind{"trigger"=>overlay.trigger=target,"anchor"=>overlay.anchor=target,"content"=>overlay.content=target,_=>panic!("unknown Overlay registration")}}
    overlay_reconcile(&owner);Value::Undefined
}
pub fn overlay_keep_mounted(owner:&OwnerWeak)->Value{let owner=setup(owner);let mut owner=owner.borrow_mut();assert!(matches!(owner.modules.overlay.presence,Value::Undefined),"cannot retain a Presence-bound overlay");owner.modules.overlay.retained=true;Value::Undefined}
pub fn overlay_bind_presence(owner:&OwnerWeak,binding:Value)->Value{
    let owner_ref=setup(owner);{let owner=owner_ref.borrow();assert!(!owner.modules.overlay.retained,"cannot bind Presence after keepMounted");if !matches!(owner.modules.overlay.presence,Value::Undefined){assert!(strict_equal(&owner.modules.overlay.presence,&binding),"Overlay Presence already bound");return Value::Undefined}}
    assert!(matches!(binding.member("enter",false),Value::Function(_))&&matches!(binding.member("leave",false),Value::Function(_)),"Presence requires callable enter/leave");let present=binding.member("present",false);let active=present.get().bool();
    owner_ref.borrow_mut().modules.overlay.presence=binding;overlay_view_active(&owner_ref,active);
    let weak=owner.clone();watch_state(owner,present,Value::Function(Rc::new(move|arguments|{let event=arg(&arguments,1);if event.member("type",false).text()=="next"{let owner=ensure(&weak);overlay_view_active(&owner,event.member("next",false).bool());overlay_reconcile(&owner);}Value::Undefined})));let active=owner_ref.borrow().modules.state_watchers.last().expect("Presence watcher").active.clone();owner_ref.borrow_mut().modules.overlay.presence_active=Some(active);
    // Registered at bind time, after the bound transition's created callback.
    let weak=owner.clone();register_life(owner,"created",Value::Function(Rc::new(move|_|overlay_lifecycle(&weak,"presence-created"))));Value::Undefined
}
fn overlays_in(window:&Window)->Vec<OwnerRef>{let id=window.window_handle().window_id();OVERLAY_OWNERS.with(|owners|{let mut owners=owners.borrow_mut();owners.retain(|owner|owner.strong_count()!=0);owners.iter().filter_map(Weak::upgrade).filter(|owner|ready(owner)&&owner.borrow().window_id==Some(id)).collect()})}
fn overlay_escape(window:&Window)->bool{let candidate=overlays_in(window).into_iter().filter(|owner|{let owner=owner.borrow();owner.modules.overlay.open&&owner.modules.overlay.view_active&&bool_or(field(&owner.modules.overlay.config,"closeOnEscape"),false)}).max_by_key(|owner|owner.borrow().modules.boundary.stack_order);if let Some(owner)=candidate{overlay_set_open(&owner,false,Value::string("escape"));true}else{false}}
struct OverlayInputScope{_keyboard:gpui::Subscription}
impl gpui::Global for OverlayInputScope{}
pub fn overlay_input_scope(owner:&OwnerRef,cx:&mut App){
    if !owner.borrow().modules.hooks.contains("overlay")||cx.has_global::<OverlayInputScope>(){return}
    let keyboard=cx.intercept_keystrokes(|event,window,cx|{if portable_key(&event.keystroke).as_deref()==Some("Escape")&&overlay_escape(window){drive_owners(window,cx);}});
    cx.set_global(OverlayInputScope{_keyboard:keyboard});
}
pub fn overlay_outside(owner:&OwnerRef){if owner.borrow().modules.hooks.contains("overlay")&&owner.borrow().modules.overlay.open&&overlay_boolean(owner,"closeOnOutsidePress"){overlay_set_open(owner,false,Value::string("outside.press"));}}
pub fn overlay_modal_wheel(event:&gpui::ScrollWheelEvent,phase:gpui::DispatchPhase,window:&mut Window,cx:&mut App){if !phase.capture(){return}let locked=overlays_in(window).into_iter().any(|owner|owner.borrow().modules.overlay.modal_locked&&!native_host_hit(&owner,event.position,window));if locked{window.prevent_default();cx.stop_propagation();}}
fn overlay_presentation(owner:&OwnerRef)->Option<(bool,usize)>{
    let owner=owner.borrow();let overlay=&owner.modules.overlay;if !owner.modules.hooks.contains("overlay")||!overlay.materialized&&!overlay.view_active{return None}
    let role=text_or(field(&overlay.config,"layerRole"),"overlay");let role_offset=match role.as_str(){"dialog-mask"=>1000,"dialog-content"=>1010,_=>0};let offset=field(&overlay.config,"layerOffset");let offset=if offset.nullish(){0.}else{offset.num().trunc()};
    Some((bool_or(field(&overlay.config,"portal"),false),(10000.+overlay.layer_order as f64+role_offset as f64+offset).max(0.)as usize))
}
struct ViewportRoot{element:AnyElement}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for ViewportRoot{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for ViewportRoot{
    type RequestLayoutState=();type PrepaintState=();fn id(&self)->Option<gpui::ElementId>{None}fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){let child=self.element.request_layout(window,cx);let viewport=window.viewport_size();let layout=window.request_layout(gpui::Style{position:gpui::Position::Absolute,size:gpui::size(viewport.width.into(),viewport.height.into()),..Default::default()},[child],cx);(layout,())}
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,bounds:Bounds<Pixels>,_:&mut (),window:&mut Window,cx:&mut App){window.with_element_offset(-bounds.origin,|window|self.element.prepaint(window,cx));}
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),_:&mut (),window:&mut Window,cx:&mut App){self.element.paint(window,cx)}
}
struct ClippedLayer{element:AnyElement,mask:Rc<Cell<Option<gpui::ContentMask<Pixels>>>>}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for ClippedLayer{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for ClippedLayer{
    type RequestLayoutState=();type PrepaintState=();fn id(&self)->Option<gpui::ElementId>{None}fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){(self.element.request_layout(window,cx),())}
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),window:&mut Window,cx:&mut App){window.with_content_mask(self.mask.get(),|window|self.element.prepaint(window,cx));}
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),_:&mut (),window:&mut Window,cx:&mut App){window.with_content_mask(self.mask.get(),|window|self.element.paint(window,cx));}
}
struct OverlayLayer{element:Option<AnyElement>,mask:Option<Rc<Cell<Option<gpui::ContentMask<Pixels>>>>>,priority:usize}
#[cfg(not(feature = "gpui-modern-element-api"))]
impl gpui::IntoElement for OverlayLayer{type Element=Self;fn into_element(self)->Self{self}}
impl gpui::Element for OverlayLayer{
    type RequestLayoutState=();type PrepaintState=();fn id(&self)->Option<gpui::ElementId>{None}fn source_location(&self)->Option<&'static std::panic::Location<'static>>{None}
    fn request_layout(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,window:&mut Window,cx:&mut App)->(gpui::LayoutId,()){(self.element.as_mut().expect("layer content").request_layout(window,cx),())}
    fn prepaint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),window:&mut Window,_:&mut App){let mask=self.mask.as_ref().map(|mask|{let value=window.content_mask();mask.set(Some(value));value});let offset=window.element_offset();window.defer_draw(self.element.take().expect("layer content"),offset,self.priority,mask);}
    fn paint(&mut self,_:Option<&gpui::GlobalElementId>,_:Option<&gpui::InspectorElementId>,_:Bounds<Pixels>,_:&mut (),_:&mut (),_:&mut Window,_:&mut App){}
}
pub fn viewport_root(element:AnyElement)->AnyElement{gpui::deferred(ViewportRoot{element}).into_any_element()}
pub fn overlay_root(owner:&OwnerRef,element:AnyElement,fixed:bool)->AnyElement{
    let Some((portal,priority))=overlay_presentation(owner)else{return if fixed{viewport_root(element)}else{element}};let portal=portal||fixed;
    let(mask,element)=if portal{(None,ViewportRoot{element}.into_any_element())}else{let mask=Rc::new(Cell::new(None));(Some(mask.clone()),ClippedLayer{element,mask}.into_any_element())};
    OverlayLayer{element:Some(element),mask,priority}.into_any_element()
}
pub fn native_pointer_press(event:&gpui::MouseDownEvent,phase:gpui::DispatchPhase,window:&mut Window,cx:&mut App){
    if phase.capture(){POINTER_PRESS_DISPATCHED.with(|sample|sample.set(false));return}
    if !phase.bubble()||POINTER_PRESS_DISPATCHED.with(|sample|sample.replace(true)){return}
    native_boundary_press(event.position,window,cx);
}
