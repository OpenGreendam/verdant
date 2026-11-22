// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  
  pub use proto_ui_gpui_native as native;
  use gpui::{prelude::*, App, Context, FocusHandle, Subscription, Window};
  use std::rc::Rc;
  use serde_json::Value as Json;
  use native::gpui_native_owner::*;
  std::thread_local! { static v_5f5f7075694d6f64756c6530: Value = {   Value::Function(Rc::new(move |arguments: Vec<Value>| {
    let v_7068617365: Value = arg(&arguments, 0);
    let v_6576656e74: Value = arg(&arguments, 1);
    let v_677561726473: Value = arg(&arguments, 2);
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":19:3
    if ({ let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("idle")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("PointerDown")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":20:5
        if (Value::boolean(!((v_677561726473.clone()).member("hit_control", false)).truthy())).truthy() {
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":20:30
            return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(false)), ("effects", Value::Array(Rc::new(vec![])))]);
        }
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":22:5
        if ({ let left = (v_677561726473.clone()).member("double_click", false); if !left.truthy() { (v_677561726473.clone()).member("reset_modifier", false) } else { left } }).truthy() {
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":23:7
            return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Select"), Value::string("Control.Reset")])))]);
        }
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":25:5
        return record(vec![("nextPhase", Value::string("armed")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Select"), Value::string("Control.Begin"), Value::string("Host.CapturePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":27:3
    if ({ let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("armed")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("PointerMove")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":29:5
        if (Value::boolean(!((v_677561726473.clone()).member("threshold_crossed", false)).truthy())).truthy() {
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":29:36
            return record(vec![("nextPhase", Value::string("armed")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![])))]);
        }
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":30:5
        return record(vec![("nextPhase", Value::string("dragging")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Preview")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":32:3
    if ({ let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("dragging")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("PointerMove")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":33:5
        return record(vec![("nextPhase", Value::string("dragging")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Preview")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":35:3
    if ({ let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("dragging")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("PointerUp")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":36:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Commit"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":38:3
    if ({ let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("armed")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("PointerUp")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":39:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Discard"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":41:3
    if ({ let left = { let left = Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("armed")))); if !left.truthy() { Value::boolean(strict_equal(&(v_7068617365.clone()), &(Value::string("dragging")))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&(v_6576656e74.clone()), &(Value::string("Cancel")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":42:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Discard"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":44:3
    return record(vec![("nextPhase", v_7068617365.clone()), ("consumed", Value::boolean(false)), ("effects", Value::Array(Rc::new(vec![])))]);
})) }; }
std::thread_local! { static v_5f5f7075694d6f64756c6531: Value = {   Value::Function(Rc::new(move |arguments: Vec<Value>| {
    let v_7374617274: Value = arg(&arguments, 0);
    let v_64656c7461: Value = arg(&arguments, 1);
    let v_74726176656c: Value = arg(&arguments, 2);
    let v_66696e65: Value = arg(&arguments, 3);
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":50:9
    let v_7370616e = if ({ let relational_left = v_74726176656c.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_74726176656c.clone() } else { Value::number(1.0) };
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":51:9
    let v_72617465 = if (v_66696e65.clone()).truthy() { Value::number(0.1) } else { Value::number(1.0) };
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":52:9
    let v_6e657874 = add(v_7374617274.clone(), Value::number((Value::number((v_64656c7461.clone()).scalar_number() / (v_7370616e.clone()).scalar_number())).scalar_number() * (v_72617465.clone()).scalar_number()));
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":53:3
    if ({ let relational_left = v_6e657874.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":53:17
        return Value::number(0.0);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":54:3
    if ({ let relational_left = v_6e657874.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":54:17
        return Value::number(1.0);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":55:3
    return v_6e657874.clone();
})) }; }
  
  fn style_vocabulary() -> &'static native::gpui_native_style::StyleVocabulary {
      static VOCABULARY: std::sync::OnceLock<native::gpui_native_style::StyleVocabulary> = std::sync::OnceLock::new();
      VOCABULARY.get_or_init(|| native::gpui_native_style::StyleVocabulary::from_json(include_str!(".proto-ui/fixtures/style-tokens.json")).expect("compiler-produced style vocabulary"))
  }
  fn accepts_context(key: &str, value: &Json) -> bool {
      match key {
          
          _ => false,
      }
  }
  fn accepts_props(value: &Json) -> bool { value.is_object() && true }
  fn prop_valid(key: &str, value: &Json) -> bool { match key {  _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "planControlGesture" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| (data.is_object() && data.get("hit_control").map_or(false, |field| field.is_boolean()) && data.get("threshold_crossed").map_or(false, |field| field.is_boolean()) && data.get("reset_modifier").map_or(false, |field| field.is_boolean()) && data.get("double_click").map_or(false, |field| field.is_boolean()))))) { value_type_error("invalid exposed method arguments") },
        "dragNormalized" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {

}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData1 {
    #[serde(rename = "hit_control")]
    pub field_0: bool,
    #[serde(rename = "threshold_crossed")]
    pub field_1: bool,
    #[serde(rename = "reset_modifier")]
    pub field_2: bool,
    #[serde(rename = "double_click")]
    pub field_3: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData2 {
    #[serde(rename = "nextPhase")]
    pub field_0: String,
    #[serde(rename = "consumed")]
    pub field_1: bool,
    #[serde(rename = "effects")]
    pub field_2: Vec<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct ControlGesturePlannerHandle { pub owner: OwnerRef }
  impl ControlGesturePlannerHandle {
      pub fn method_0(&self, argument_0: String, argument_1: String, argument_2: GeneratedData1, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("planControlGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("planControlGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("nextPhase").map_or(false, |field| field.is_string()) && result.get("consumed").map_or(false, |field| field.is_boolean()) && result.get("effects").map_or(false, |field| field.as_array().is_some_and(|items| items.iter().all(|item| item.is_string()))))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_1(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<f64, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("dragNormalized").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("dragNormalized", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.as_f64().is_some_and(f64::is_finite)) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
      /// Calls a compiled data-only module function without accessing an instance.
      pub fn call_data_method(name: &str, arguments: Vec<Json>) -> Result<Json, Value> {
          catch_authored(|| match name {
          "planControlGesture" => {
            let arguments: Vec<Value> = arguments.into_iter().map(Value::json).collect();
            check_method(name, &arguments);
            let result = v_5f5f7075694d6f64756c6530.with(|function| function.call_authored(arguments));
            let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("nextPhase").map_or(false, |field| field.is_string()) && result.get("consumed").map_or(false, |field| field.is_boolean()) && result.get("effects").map_or(false, |field| field.as_array().is_some_and(|items| items.iter().all(|item| item.is_string()))))) { value_type_error("exposed method returned invalid portable data") }
            result
        },
        "dragNormalized" => {
            let arguments: Vec<Value> = arguments.into_iter().map(Value::json).collect();
            check_method(name, &arguments);
            let result = v_5f5f7075694d6f64756c6531.with(|function| function.call_authored(arguments));
            let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.as_f64().is_some_and(f64::is_finite)) { value_type_error("exposed method returned invalid portable data") }
            result
        },
              _ => value_error("unknown or ineligible data method"),
          })
      }
      pub fn owner(&self) -> OwnerWeak { Rc::downgrade(&self.owner) }
      pub fn state(&self, name: &str) -> Result<Json, Value> { catch_authored(|| { let state=self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed State")); state.get().data() }) }
      pub fn call(&self, name: &str, arguments: Vec<Json>, window: &mut Window, cx: &mut App) -> Result<Json, Value> {
          catch_authored(|| {
          refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
          let callback = self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed method"));
          let arguments: Vec<Value> = arguments.into_iter().map(Value::json).collect(); check_method(name, &arguments);
          let result = invoke(&self.owner, "expose-method", &callback, arguments);
          let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
          if matches!(result, Value::Undefined) { Json::Null } else { result.data() }
          })
      }
      pub fn host_target(&self) -> Result<Value, Value> { catch_authored(|| { ensure(&self.owner()); Value::HostTarget(self.owner()) }) }
      pub fn take_events(&self) -> Vec<(String, Json)> { std::mem::take(&mut self.owner.borrow_mut().emitted) }
      pub fn take_failures(&self) -> Vec<NativeFailure> { take_native_failures(&self.owner) }
      pub fn dispose(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
          catch_authored(|| {
          let drive = self.owner.borrow().native_drive.clone();
          let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Owner::dispose(&self.owner)));
          let driven = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { if let Some(drive) = drive { drive(window, cx); } }));
          if let Err(failure) = failure { std::panic::resume_unwind(failure); }
          if let Err(failure) = driven { std::panic::resume_unwind(failure); }
          })
      }
  }
  pub struct ControlGesturePlanner { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl ControlGesturePlanner {
      pub fn create_props(props: GeneratedProps, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> { Self::create(serde_json::to_value(props).expect("portable Props"), parent, window, cx) }
      pub fn create(raw_props: Json, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> {
          let mut failure = None;
          let entity = cx.new(|cx| Self::new(raw_props, parent, window, cx).unwrap_or_else(|error| {
              failure = Some(error);
              // A failed construction still returns normally through GPUI's
              // entity insertion. This inert temporary is dropped immediately
              // below; it is never published or mounted as a native component.
              let owner = Owner::new(Json::Object(Default::default()), Default::default(), accepts_context, accepts_props, prop_valid, style_vocabulary());
              { let mut owner = owner.borrow_mut(); owner.disposed = true; owner.present = false; }
              Self { owner, focus: cx.focus_handle(), window_input: None, _focus_subscriptions: Vec::new(), _window_activation: None, _focus_lost: None, _global_key_capture: None, _entry_readiness: None }
          }));
          match failure { Some(error) => Err(error), None => Ok(entity) }
      }
      pub fn new_props(props: GeneratedProps, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut Context<Self>) -> Result<Self, Value> { Self::new(serde_json::to_value(props).expect("portable Props"), parent, window, cx) }
      pub fn new(raw_props: Json, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut Context<Self>) -> Result<Self, Value> {
          catch_authored(|| {
          let owner_ref = Owner::new(raw_props, parent.unwrap_or_default(), accepts_context, accepts_props, prop_valid, style_vocabulary());
          let owner = Rc::downgrade(&owner_ref);
          let focus = cx.focus_handle();
          let view = cx.entity().downgrade();
          let drive_owner=owner.clone(); let drive_focus=focus.clone();
          let driving = Rc::new(std::cell::Cell::new(false));
          let drive_pending = Rc::new(std::cell::Cell::new(false));
          let drive: Drive = Rc::new(move |window, cx| {
              // Platform effects need owner storage and its real FocusHandle,
              // not another lease of the generated component. Preserve their
              // synchronous effect (notably physical focus) under public calls.
              if !driving.replace(true) {
                  if let Some(owner)=drive_owner.upgrade() {
                      native_host_callback(&owner, "drive", || { modules_drive(&owner, &drive_focus, window, cx); drive_queued_focus(&owner, window, cx); });
                  }
                  driving.set(false);
              }
              if drive_pending.replace(true) { return; }
              let view=view.clone(); let pending=drive_pending.clone();
              // A public handle can be called from Entity::update. Return the
              // current host lease before requesting another component update.
              // Authored calls, State writes and platform commands remain
              // synchronous; component refresh is coalesced at the host boundary.
              window.defer(cx, move |window, cx| {
                  pending.set(false);
                  let _ = view.update(cx, |view, cx| { if let Err(error) = view.drive(window, cx) { report_native_failure(&view.owner, "drive", error); } });
              });
          });
          { let mut owner = owner_ref.borrow_mut(); owner.native_drive = Some(drive); owner.window_id = Some(window.window_handle().window_id()); }
  
          configure_native_atoms(&owner_ref, false, false);
          refresh_native_atoms(&owner_ref, window, cx);
          let _lexical_owner = enter_lexical_owner(&owner);
          let renderer = { let owner = owner.clone();  Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_646566: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":61:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); expose_method(&owner, Value::string("planControlGesture"), v_5f5f7075694d6f64756c6530.with(|value| value.clone())) };
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":62:5
            { let v_7075695f6e61746976655f31 = v_646566.clone(); expose_method(&owner, Value::string("dragNormalized"), v_5f5f7075694d6f64756c6531.with(|value| value.clone())) };
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":63:5
            return { let callable={ let pui_native_2={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":63:26
                return { let v_7075695f6e61746976655f33 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_2.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_2.call)(arg(&arguments,0))})} }; match callable {
      Value::NativeFunction{render,..} => Value::Renderer(render),
      Value::Renderer(render) => Value::Renderer(render),
      Value::Function(callback) => Value::Renderer(Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
      _ => panic!("checked setup result must be a callable")
    } };
        })) }.call_authored(vec![capability(&owner)]);
          Owner::finish(&owner_ref, renderer);
          let focus_subscriptions = if native_focus_observation_needed(&owner_ref) {
          let focus_owner = owner_ref.clone();
          let on_focus = cx.on_focus(&focus, window, move |view, window, cx| {
              native_host_callback(&focus_owner, "focus", || {
              let epoch = focus_owner.borrow().epoch;
              if !ready(&focus_owner) || focus_owner.borrow().window_id != Some(window.window_handle().window_id()) { return; }
              #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&focus_owner, window);
              native_focus_fact(&focus_owner, view.focus.is_focused(window));
              if !ready(&focus_owner) || focus_owner.borrow().epoch != epoch { return; }
              dispatch(&focus_owner, "host:focus", false, event_fields("host:focus"));
              if ready(&focus_owner) && focus_owner.borrow().epoch == epoch { view.drive(window, cx).unwrap_or_else(|error| throw_value(error)); }
              });
          });
          let blur_owner = owner_ref.clone();
          let on_blur = cx.on_blur(&focus, window, move |view, window, cx| {
              native_host_callback(&blur_owner, "blur", || {
              let epoch = blur_owner.borrow().epoch;
              if !ready(&blur_owner) || blur_owner.borrow().window_id != Some(window.window_handle().window_id()) { return; }
              #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&blur_owner, window);
              native_focus_fact(&blur_owner, view.focus.is_focused(window));
              if !ready(&blur_owner) || blur_owner.borrow().epoch != epoch { return; }
              text_blur(&blur_owner, window, cx);
              dispatch(&blur_owner, "host:blur", false, event_fields("host:blur"));
              if ready(&blur_owner) && blur_owner.borrow().epoch == epoch { view.drive(window, cx).unwrap_or_else(|error| throw_value(error)); }
              });
          });
          vec![on_focus, on_blur]
          } else { Vec::new() };
          let global_key_capture = subscribe_global_key_capture(&owner_ref, cx);
          let window_input = window_input_needed(&owner_ref).then(|| cx.focus_handle().tab_stop(false));
          let focus_lost = window_input.as_ref().map(|_| cx.on_focus_lost(window, |view, window, cx| { let owner=view.owner.clone(); native_host_callback(&owner, "focus-lost", || { if view.owner.borrow().window_id == Some(window.window_handle().window_id()) { if let Some(focus) = &view.window_input { restore_window_input(&view.owner, focus, window, cx); } } }); }));
          let mut view = Self { owner: owner_ref, focus, window_input, _focus_subscriptions: focus_subscriptions, _window_activation: None, _focus_lost: focus_lost, _global_key_capture: global_key_capture, _entry_readiness: None };
          view.bind_window_activation(window, cx);
          view
          })
      }
      #[cfg(feature="test-support")]
      pub fn native_resource_counts(&self) -> Json { let mut counts=native::gpui_native_owner::native_resource_counts(&self.owner); counts["nativeSubscriptions"] = Json::from(self._focus_subscriptions.len() + usize::from(self._window_activation.is_some()) + usize::from(self._focus_lost.is_some()) + usize::from(self._global_key_capture.is_some()) + usize::from(self._entry_readiness.is_some())); counts }
      pub fn handle(&self) -> ControlGesturePlannerHandle { ControlGesturePlannerHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| replace_props(&self.owner, raw))?; self.drive(window, cx) }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_focus_observation_needed(&self.owner) && !native::gpui_native_move::move_owner_has_binding(&self.owner) { self._window_activation.take(); return; }
          let id = window.window_handle().window_id();
          if self._window_activation.as_ref().is_some_and(|(bound, _)| *bound == id) { return; }
          self._window_activation.take();
          let owner = self.owner.clone();
          let subscription = cx.observe_window_activation(window, move |view, window, cx| {
              native_host_callback(&owner, "window-activation", || {
                  if owner.borrow().window_id == Some(window.window_handle().window_id()) && !window.is_window_active() {
                      native_window_blur(&owner);
                      view.drive(window, cx).unwrap_or_else(|error| throw_value(error));
                  }
              });
          });
          self._window_activation = Some((id, subscription));
      }
      #[cfg(feature = "experimental-focus-tree")]
      fn bind_entry_readiness(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !self.owner.borrow().modules.hooks.contains("focus-entry") { return; }
          let window_id = window.window_handle().window_id();
          if self._entry_readiness.as_ref().is_some_and(|(bound, _)| *bound == window_id) { return; }
          self._entry_readiness.take();
          let view = cx.entity().downgrade();
          let subscription = window.observe_focus_tree(cx, move |_, window, cx| {
              let _ = view.update(cx, |view, cx| {
                  if view._entry_readiness.as_ref().is_some_and(|(bound, _)| *bound == window.window_handle().window_id()) { if let Err(error)=view.drive(window, cx){report_native_failure(&view.owner,"entry-readiness",error)} }
              });
          });
          self._entry_readiness = Some((window_id, subscription));
      }
      pub fn drive(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          catch_authored(|| {
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); cx.notify(); return; }
          modules_drive(&self.owner, &self.focus, window, cx);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); cx.notify(); return; }
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          drive_queued_focus(&self.owner, window, cx);
          let dirty = std::mem::take(&mut self.owner.borrow_mut().dirty); if dirty { cx.notify(); }
          })
      }
  }
  impl Drop for ControlGesturePlanner { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for ControlGesturePlanner { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for ControlGesturePlanner {
      fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
          let report_owner = self.owner.clone();
          native_host_callback(&report_owner, "render", || {
          refresh_native_atoms(&self.owner, window, cx);
          Owner::project(&self.owner);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); return gpui::div().into_any_element(); }
          if !self.owner.borrow().present { self._window_activation.take(); detach_native_view(&self.owner); return gpui::div().into_any_element(); }
          bind_native_focus_window(&self.owner, window);
          self.bind_window_activation(window, cx);
          #[cfg(feature = "experimental-focus-tree")] self.bind_entry_readiness(window, cx);
          let drive = self.owner.borrow().native_drive.as_ref().expect("live native owner drive").clone();
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
