// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  
  pub use proto_ui_gpui_native as native;
  use gpui::{prelude::*, App, Context, FocusHandle, Subscription, Window};
  use std::rc::Rc;
  use serde_json::Value as Json;
  use native::gpui_native_owner::*;
  std::thread_local! { static v_5f5f7075694d6f64756c6530: Value = {   Value::Function(Rc::new(move |arguments: Vec<Value>| {
    let v_70686173655f5f39656538633962373732623434633532: Value = arg(&arguments, 0);
    let v_6576656e745f5f32626536396664666631353062616230: Value = arg(&arguments, 1);
    let v_677561726473: Value = arg(&arguments, 2);
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":19:3
    if ({ let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("idle")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("PointerDown")))) } else { left } }).truthy() {
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
    if ({ let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("armed")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("PointerMove")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":29:5
        if (Value::boolean(!((v_677561726473.clone()).member("threshold_crossed", false)).truthy())).truthy() {
            // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":29:36
            return record(vec![("nextPhase", Value::string("armed")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![])))]);
        }
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":30:5
        return record(vec![("nextPhase", Value::string("dragging")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Preview")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":32:3
    if ({ let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("dragging")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("PointerMove")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":33:5
        return record(vec![("nextPhase", Value::string("dragging")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Preview")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":35:3
    if ({ let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("dragging")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("PointerUp")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":36:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Commit"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":38:3
    if ({ let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("armed")))); if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("PointerUp")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":39:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Discard"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":41:3
    if ({ let left = { let left = Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("armed")))); if !left.truthy() { Value::boolean(strict_equal(&(v_70686173655f5f39656538633962373732623434633532.clone()), &(Value::string("dragging")))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&(v_6576656e745f5f32626536396664666631353062616230.clone()), &(Value::string("Cancel")))) } else { left } }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":42:5
        return record(vec![("nextPhase", Value::string("idle")), ("consumed", Value::boolean(true)), ("effects", Value::Array(Rc::new(vec![Value::string("Control.Discard"), Value::string("Host.ReleasePointer")])))]);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":44:3
    return record(vec![("nextPhase", v_70686173655f5f39656538633962373732623434633532.clone()), ("consumed", Value::boolean(false)), ("effects", Value::Array(Rc::new(vec![])))]);
})) }; }
std::thread_local! { static v_5f5f7075694d6f64756c6531: Value = {   Value::Function(Rc::new(move |arguments: Vec<Value>| {
    let v_7374617274: Value = arg(&arguments, 0);
    let v_64656c74615f5f61323836306631383633333866336361: Value = arg(&arguments, 1);
    let v_74726176656c: Value = arg(&arguments, 2);
    let v_66696e655f5f37343161376531353430323939333562: Value = arg(&arguments, 3);
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":50:9
    let v_7370616e = if ({ let relational_left = v_74726176656c.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_74726176656c.clone() } else { Value::number(1.0) };
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":51:9
    let v_72617465 = if (v_66696e655f5f37343161376531353430323939333562.clone()).truthy() { Value::number(0.1) } else { Value::number(1.0) };
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":52:9
    let v_6e6578745f5f31656433613261616230343934653664 = add(v_7374617274.clone(), Value::number((Value::number((v_64656c74615f5f61323836306631383633333866336361.clone()).scalar_number() / (v_7370616e.clone()).scalar_number())).scalar_number() * (v_72617465.clone()).scalar_number()));
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":53:3
    if ({ let relational_left = v_6e6578745f5f31656433613261616230343934653664.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":53:17
        return Value::number(0.0);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":54:3
    if ({ let relational_left = v_6e6578745f5f31656433613261616230343934653664.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
        // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":54:17
        return Value::number(1.0);
    }
    // Source "frontend/proto-ui/src/control-gesture-planner.proto.ts":55:3
    return v_6e6578745f5f31656433613261616230343934653664.clone();
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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("value").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("defaultValue").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("step").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "value" => value.as_f64().is_some_and(f64::is_finite),
        "defaultValue" => value.as_f64().is_some_and(f64::is_finite),
        "step" => value.as_f64().is_some_and(f64::is_finite),
        "label" => value.is_string(),
        "disabled" => value.is_boolean(),
        "revision" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<f64>,
    #[serde(rename = "defaultValue", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<f64>,
    #[serde(rename = "step", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct NormalizedKnobHandle { pub owner: OwnerRef }
  impl NormalizedKnobHandle {
      pub fn state_0(&self) -> Result<f64, Value> { self.state("value").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_7(&self, argument_0: bool, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("begin").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("begin", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_8(&self, argument_0: f64, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("sample").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("sample", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_9(&self, argument_0: f64, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("finish").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("finish", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_10(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("cancel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("cancel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
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
  pub struct NormalizedKnob { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl NormalizedKnob {
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
            let v_636c616d70: LexicalBinding<Value> = lexical_binding();
            let v_686173456666656374: LexicalBinding<Value> = lexical_binding();
            let v_706c616e: LexicalBinding<Value> = lexical_binding();
            let v_6e6f726d616c697a6544726167: LexicalBinding<Value> = lexical_binding();
            let v_70686173655f5f37353966383966343464326634376436: LexicalBinding<Value> = lexical_binding();
            let v_76616c75655f5f31323838633235346132313638323338: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_696e697469616c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6163636570746564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_697343757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_6973417574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_7075626c697368: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<Value> = lexical_binding();
            let v_6170706c79: LexicalBinding<Value> = lexical_binding();
            let v_73616d706c654174: LexicalBinding<Value> = lexical_binding();
            let v_61646a757374: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":20:11
            v_636c616d70.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76616c75655f5f38633830393535333238636436623961: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":21:7
                if (Value::boolean(!strict_equal(&(v_76616c75655f5f38633830393535333238636436623961.clone()), &(v_76616c75655f5f38633830393535333238636436623961.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":21:28
                    return Value::number(0.0);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":22:7
                if ({ let relational_left = v_76616c75655f5f38633830393535333238636436623961.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":22:22
                    return Value::number(0.0);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":23:7
                if ({ let relational_left = v_76616c75655f5f38633830393535333238636436623961.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":23:22
                    return Value::number(1.0);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":24:7
                return v_76616c75655f5f38633830393535333238636436623961.clone();
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":26:11
            v_686173456666656374.initialize({  let v_686173456666656374 = v_686173456666656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_65666665637473: Value = arg(&arguments, 0);
                let v_656666656374: Value = arg(&arguments, 1);
                let v_696e646578: Value = { let argument = arg(&arguments, 2); if matches!(&argument, Value::Undefined) { Value::number(0.0) } else { argument } };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":27:7
                if ({ let relational_left = v_696e646578.clone(); let relational_right = (v_65666665637473.clone()).member("length", false); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":27:36
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":28:7
                if (Value::boolean(strict_equal(&({ let object = v_65666665637473.clone(); object.caught_member(&property_key(&(v_696e646578.clone())), false) }), &(v_656666656374.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":28:38
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":29:7
                return v_686173456666656374.read().call_authored(vec![v_65666665637473.clone(), v_656666656374.clone(), add(v_696e646578.clone(), Value::number(1.0))]);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":31:11
            v_706c616e.initialize(v_5f5f7075694d6f64756c6530.with(|value| value.clone()));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":32:11
            v_6e6f726d616c697a6544726167.initialize(v_5f5f7075694d6f64756c6531.with(|value| value.clone()));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":33:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("value", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("defaultValue", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("step", record(vec![("type", Value::string("number")), ("default", Value::number(0.01))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":41:11
            v_70686173655f5f37353966383966343464326634376436.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":42:11
            v_76616c75655f5f31323838633235346132313638323338.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("value"), Value::number(0.0), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":43:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":44:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":45:11
            let v_6d696e696d756d = { let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "discrete", Value::string("minimum"), Value::number(0.0), Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":46:11
            let v_6d6178696d756d = { let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "discrete", Value::string("maximum"), Value::number(1.0), Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":47:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":48:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":49:5
            { let v_7075695f6e61746976655f37 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f37.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":50:5
            { let v_7075695f6e61746976655f38 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f38.clone()).owner(), Value::string("slider")) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":51:5
            { let v_7075695f6e61746976655f39 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f39.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":52:5
            { let v_7075695f6e61746976655f3130 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3130.clone()).owner(), Value::string("valueMin"), v_6d696e696d756d.clone()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":53:5
            { let v_7075695f6e61746976655f3131 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3131.clone()).owner(), Value::string("valueMax"), v_6d6178696d756d.clone()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":54:5
            { let v_7075695f6e61746976655f3132 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3132.clone()).owner(), Value::string("valueNow"), v_76616c75655f5f31323838633235346132313638323338.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":55:5
            { let v_7075695f6e61746976655f3133 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":58:5
            { let v_7075695f6e61746976655f3134 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("increment"), record(vec![("event", Value::string("host:verdant-knob-increment"))])) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":59:5
            { let v_7075695f6e61746976655f3135 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("decrement"), record(vec![("event", Value::string("host:verdant-knob-decrement"))])) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":60:5
            { let v_7075695f6e61746976655f3136 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("setValue"), record(vec![("event", Value::string("host:verdant-knob-set-value"))])) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":61:5
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_state(&owner, Value::string("value"), v_76616c75655f5f31323838633235346132313638323338.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":62:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_70686173655f5f37353966383966343464326634376436.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":63:5
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":64:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":65:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":66:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_event(&owner, Value::string("valueCommit")) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":67:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":69:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":70:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":71:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":72:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":73:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":74:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":75:9
            v_696e697469616c.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":76:9
            v_6163636570746564.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":77:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":78:11
            v_697343757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f61313433643035643037393531393639: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":78:52
                return { let left = { let left = Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f61313433643035643037393531393639.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":79:11
            v_6973417574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f30346136653137666235633238316235: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":79:61
                return { let left = { let left = Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30346136653137666235633238316235.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":80:11
            v_7075626c697368.initialize({  let v_636c616d70 = v_636c616d70.clone(); let v_76616c75655f5f31323838633235346132313638323338 = v_76616c75655f5f31323838633235346132313638323338.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f61353935666130383935663034393638: Value = arg(&arguments, 0);
                let v_7469636b65745f5f63643231353331323065643139323732: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":81:7
                if (Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f63643231353331323065643139323732.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":81:31
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":82:7
                { let v_7075695f6e61746976655f3234 = v_76616c75655f5f31323838633235346132313638323338.read(); state_set(v_7075695f6e61746976655f3234.clone(), v_636c616d70.read().call_authored(vec![v_6e6578745f5f61353935666130383935663034393638.clone()]), Value::string("reason: normalized knob display")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":83:7
                if (Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f63643231353331323065643139323732.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":83:31
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":84:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":84:28
                    { let v_7075695f6e61746976655f3235 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3235.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":85:7
                return v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f63643231353331323065643139323732.clone()]);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":87:11
            v_63616e63656c.initialize({  let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":89:13
                let v_7469636b65745f5f37663266616437313862323162383063 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":90:7
                if ({ let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3236 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3236.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":90:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":91:13
                let v_62617369735f5f31616166613134393135396332383464 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":92:7
                { let v_7075695f6e61746976655f3237 = v_70686173655f5f37353966383966343464326634376436.read(); state_set(v_7075695f6e61746976655f3237.clone(), Value::string("idle"), Value::string("reason: normalized knob canceled")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":93:7
                if ({ let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f37663266616437313862323162383063.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f37663266616437313862323162383063.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":93:61
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":94:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":94:16
                    { let v_7075695f6e61746976655f3238 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3238.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f31616166613134393135396332383464.clone())])) };
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":95:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":97:11
            v_70726f706f7365.initialize({  let v_636c616d70 = v_636c616d70.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f30326434376662366432633036373034: Value = arg(&arguments, 0);
                let v_7469636b65745f5f61663031666633613862666163306261: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":98:7
                if ({ let left = { let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f61663031666633613862666163306261.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3239 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3239.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":98:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":99:13
                let v_70726f706f73616c5f5f39373739643130616365363232666161 = v_636c616d70.read().call_authored(vec![v_6e6578745f5f30326434376662366432633036373034.clone()]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":100:13
                let v_62617369735f5f65616661313431613866386433343461 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":103:7
                if (Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f61663031666633613862666163306261.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":103:39
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":104:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":104:16
                    { let v_7075695f6e61746976655f3330 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3330.clone()).owner(), Value::string("valueCommit"), record(vec![("value", v_70726f706f73616c5f5f39373739643130616365363232666161.clone()), ("revision", v_62617369735f5f65616661313431613866386433343461.clone())])) };
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":105:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":107:11
            v_6170706c79.initialize({  let v_636c616d70 = v_636c616d70.clone(); let v_686173456666656374 = v_686173456666656374.clone(); let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_76616c75655f5f31323838633235346132313638323338 = v_76616c75655f5f31323838633235346132313638323338.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_696e697469616c = v_696e697469616c.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6465636973696f6e: Value = arg(&arguments, 0);
                let v_70726576696577: Value = arg(&arguments, 1);
                let v_7469636b65745f5f65353461323030303739356464666234: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":108:7
                if ({ let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f65353461323030303739356464666234.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":108:44
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":109:13
                let v_72657365745f5f64633965366361353638343536353266 = v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Reset")]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":110:13
                let v_636f6d6d6974 = v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Commit")]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":111:13
                let v_70726f706f73616c5f5f39663636633230326465376332656563 = if (v_72657365745f5f64633965366361353638343536353266.clone()).truthy() { v_636c616d70.read().call_authored(vec![{ let left = ({ let v_7075695f6e61746976655f3331 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3331.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3331.clone()).owner(), false) } }).member("defaultValue", true); if left.nullish() { Value::number(0.0) } else { left } }]) } else { { let v_7075695f6e61746976655f3332 = v_76616c75655f5f31323838633235346132313638323338.read(); (v_7075695f6e61746976655f3332.clone()).get() } };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":112:7
                if (v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Begin")])).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":112:57
                    { let value = { let value = (v_6163636570746564.read()).borrow().clone(); value }; *(v_696e697469616c.read()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":113:7
                { let v_7075695f6e61746976655f3333 = v_70686173655f5f37353966383966343464326634376436.read(); state_set(v_7075695f6e61746976655f3333.clone(), (v_6465636973696f6e.clone()).member("nextPhase", false), Value::string("reason: normalized knob input")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":114:7
                if (Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f65353461323030303739356464666234.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":114:31
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":115:7
                if ({ let left = v_72657365745f5f64633965366361353638343536353266.clone(); if !left.truthy() { v_636f6d6d6974.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":115:28
                    return v_70726f706f7365.read().call_authored(vec![v_70726f706f73616c5f5f39663636633230326465376332656563.clone(), v_7469636b65745f5f65353461323030303739356464666234.clone()]);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":116:7
                if (v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Preview")])).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":117:9
                    if (Value::boolean(!(v_7075626c697368.read().call_authored(vec![v_70726576696577.clone(), v_7469636b65745f5f65353461323030303739356464666234.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":117:40
                        return Value::boolean(false);
                    }
                } else {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":118:14
                    if ({ let left = v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Discard")]); if !left.truthy() { v_686173456666656374.read().call_authored(vec![(v_6465636973696f6e.clone()).member("effects", false), Value::string("Control.Begin")]) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":119:9
                        if (Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65353461323030303739356464666234.clone()])).truthy())).truthy() {
                            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":119:41
                            return Value::boolean(false);
                        }
                    } else {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":121:9
                        if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":121:18
                            { let v_7075695f6e61746976655f3334 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3334.clone()).owner()) };
                        }
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":122:9
                        if (Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f65353461323030303739356464666234.clone()])).truthy())).truthy() {
                            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":122:33
                            return Value::boolean(false);
                        }
                    }
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":124:7
                return (v_6465636973696f6e.clone()).member("consumed", false);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":126:11
            v_73616d706c654174.initialize({  let v_706c616e = v_706c616e.clone(); let v_6e6f726d616c697a6544726167 = v_6e6f726d616c697a6544726167.clone(); let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_696e697469616c = v_696e697469616c.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_6170706c79 = v_6170706c79.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64656c74615f5f62396133383431343432316261366132: Value = arg(&arguments, 0);
                let v_63726f737365645f5f31333734646331633535323161393765: Value = arg(&arguments, 1);
                let v_66696e655f5f34313137393566323038353030303031: Value = arg(&arguments, 2);
                let v_7469636b65745f5f35656637363133353762663337616235: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":127:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f35656637363133353762663337616235.clone()])).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3335 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3335.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3336 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3336.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":127:99
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":128:7
                return v_6170706c79.read().call_authored(vec![v_706c616e.read().call_authored(vec![{ let v_7075695f6e61746976655f3337 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3337.clone()).get() }, Value::string("PointerMove"), record(vec![("hit_control", Value::boolean(true)), ("threshold_crossed", v_63726f737365645f5f31333734646331633535323161393765.clone()), ("reset_modifier", Value::boolean(false)), ("double_click", Value::boolean(false))])]), v_6e6f726d616c697a6544726167.read().call_authored(vec![{ let value = (v_696e697469616c.read()).borrow().clone(); value }, v_64656c74615f5f62396133383431343432316261366132.clone(), Value::number(180.0), v_66696e655f5f34313137393566323038353030303031.clone()]), v_7469636b65745f5f35656637363133353762663337616235.clone()]);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":132:5
            { let v_7075695f6e61746976655f3338 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_706c616e = v_706c616e.clone(); let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_6170706c79 = v_6170706c79.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72657365745f5f38356637393266353038323766656632: Value = arg(&arguments, 0);
                let v_646f75626c65: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":133:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3339 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3339.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3430 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3430.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":133:97
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":134:13
                let v_7469636b65745f5f34336164376433393362323635363337 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":135:7
                { let v_7075695f6e61746976655f3431 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3431.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":136:7
                if ({ let left = { let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f34336164376433393362323635363337.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3432 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3432.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":136:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":137:7
                return v_6170706c79.read().call_authored(vec![v_706c616e.read().call_authored(vec![{ let v_7075695f6e61746976655f3433 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3433.clone()).get() }, Value::string("PointerDown"), record(vec![("hit_control", Value::boolean(true)), ("threshold_crossed", Value::boolean(false)), ("reset_modifier", v_72657365745f5f38356637393266353038323766656632.clone()), ("double_click", v_646f75626c65.clone())])]), { let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f34336164376433393362323635363337.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":141:5
            { let v_7075695f6e61746976655f3434 = v_646566.clone(); expose_method(&owner, Value::string("sample"), {  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64656c74615f5f66353461323831303466616234313533: Value = arg(&arguments, 0);
                let v_63726f737365645f5f63306439343365636164613036303734: Value = arg(&arguments, 1);
                let v_66696e655f5f32373137656164316266643830316131: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":142:13
                let v_7469636b65745f5f35323663323935363038663466343131 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":143:7
                return v_73616d706c654174.read().call_authored(vec![v_64656c74615f5f66353461323831303466616234313533.clone(), v_63726f737365645f5f63306439343365636164613036303734.clone(), v_66696e655f5f32373137656164316266643830316131.clone(), v_7469636b65745f5f35323663323935363038663466343131.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":145:5
            { let v_7075695f6e61746976655f3435 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_706c616e = v_706c616e.clone(); let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_76616c75655f5f31323838633235346132313638323338 = v_76616c75655f5f31323838633235346132313638323338.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_6170706c79 = v_6170706c79.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64656c74615f5f34356464643063656632326265303231: Value = arg(&arguments, 0);
                let v_63726f737365645f5f63313734356666386132343132336434: Value = arg(&arguments, 1);
                let v_66696e655f5f63326432303164633635303838316535: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":146:13
                let v_7469636b65745f5f66393334386266363763366330346337 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":147:7
                if ({ let left = Value::boolean(!(v_73616d706c654174.read().call_authored(vec![v_64656c74615f5f34356464643063656632326265303231.clone(), v_63726f737365645f5f63313734356666386132343132336434.clone(), v_66696e655f5f63326432303164633635303838316535.clone(), v_7469636b65745f5f66393334386266363763366330346337.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f66393334386266363763366330346337.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":147:74
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":148:7
                return v_6170706c79.read().call_authored(vec![v_706c616e.read().call_authored(vec![{ let v_7075695f6e61746976655f3436 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3436.clone()).get() }, Value::string("PointerUp"), record(vec![("hit_control", Value::boolean(true)), ("threshold_crossed", v_63726f737365645f5f63313734356666386132343132336434.clone()), ("reset_modifier", Value::boolean(false)), ("double_click", Value::boolean(false))])]), { let v_7075695f6e61746976655f3437 = v_76616c75655f5f31323838633235346132313638323338.read(); (v_7075695f6e61746976655f3437.clone()).get() }, v_7469636b65745f5f66393334386266363763366330346337.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":152:5
            { let v_7075695f6e61746976655f3438 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":153:11
            v_61646a757374.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b6579: Value = arg(&arguments, 0);
                let v_66696e655f5f36663235663635383063353966306565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":154:13
                let v_63757272656e745f5f35366461366137346534613734613064 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":155:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!(v_63757272656e745f5f35366461366137346534613734613064.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3439 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3439.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":155:75
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":156:13
                let v_7469636b65745f5f37613631653332316630646631343166 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":157:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":161:7
                if ({ let left = { let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f37613631653332316630646631343166.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3530 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3530.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":161:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":162:11
                let v_6e6578745f5f37376264373166373864366661613037 = Rc::new(std::cell::RefCell::new({ let value = (v_6163636570746564.read()).borrow().clone(); value }));
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":163:13
                let v_72617753746570 = { let left = ({ let v_7075695f6e61746976655f3531 = v_63757272656e745f5f35366461366137346534613734613064.clone(); read_props(&(v_7075695f6e61746976655f3531.clone()).owner(), false) }).member("step", false); if left.nullish() { Value::number(0.01) } else { left } };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":164:13
                let v_73746570 = if ({ let left = { let relational_left = v_72617753746570.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_72617753746570.clone(); let relational_right = Value::number(1.0); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() { v_72617753746570.clone() } else { Value::number(0.01) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":165:13
                let v_616d6f756e74 = Value::number((v_73746570.clone()).scalar_number() * (if (v_66696e655f5f36663235663635383063353966306565.clone()).truthy() { Value::number(0.1) } else { Value::number(1.0) }).scalar_number());
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":166:7
                if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Home"))))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":166:27
                    { let value = Value::number(0.0); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":167:12
                    if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("End"))))).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":167:31
                        { let value = Value::number(1.0); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":168:12
                        if ({ let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":168:59
                            { let value = add({ let value = (v_6e6578745f5f37376264373166373864366661613037.clone()).borrow().clone(); value }, v_616d6f756e74.clone()); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":169:12
                            if ({ let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowDown")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":169:60
                                { let value = Value::number(({ let value = (v_6e6578745f5f37376264373166373864366661613037.clone()).borrow().clone(); value }).scalar_number() - (v_616d6f756e74.clone()).scalar_number()); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":170:12
                                if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("PageUp"))))).truthy() {
                                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":170:34
                                    { let value = add({ let value = (v_6e6578745f5f37376264373166373864366661613037.clone()).borrow().clone(); value }, Value::number((v_73746570.clone()).scalar_number() * (Value::number(10.0)).scalar_number())); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":171:12
                                    if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("PageDown"))))).truthy() {
                                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":171:36
                                        { let value = Value::number(({ let value = (v_6e6578745f5f37376264373166373864366661613037.clone()).borrow().clone(); value }).scalar_number() - (Value::number((v_73746570.clone()).scalar_number() * (Value::number(10.0)).scalar_number())).scalar_number()); *(v_6e6578745f5f37376264373166373864366661613037.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":172:12
                                        return Value::boolean(false);
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":173:7
                return v_70726f706f7365.read().call_authored(vec![{ let value = (v_6e6578745f5f37376264373166373864366661613037.clone()).borrow().clone(); value }, v_7469636b65745f5f37613631653332316630646631343166.clone()]);
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":175:5
            { let v_7075695f6e61746976655f3532 = v_646566.clone(); listen(&owner, Value::string("host:verdant-knob-increment"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":176:7
                v_61646a757374.read().call_authored(vec![Value::string("ArrowUp"), Value::boolean(false)]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":178:5
            { let v_7075695f6e61746976655f3533 = v_646566.clone(); listen(&owner, Value::string("host:verdant-knob-decrement"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":179:7
                v_61646a757374.read().call_authored(vec![Value::string("ArrowDown"), Value::boolean(false)]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":181:5
            { let v_7075695f6e61746976655f3534 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-knob-set-value"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_697343757272656e74 = v_697343757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f31346339663164316338336666313166: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":182:13
                let v_6576656e745f5f38323033306361623562333230323463 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":183:13
                let v_6e6578745f5f33346434383339326333616163353930 = (v_6576656e745f5f38323033306361623562333230323463.clone()).member("value", false);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":184:7
                if ({ let left = Value::boolean(!strict_equal(&(typeof_value(&(v_6e6578745f5f33346434383339326333616163353930.clone()))), &(Value::string("number")))); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_6e6578745f5f33346434383339326333616163353930.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":184:63
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":185:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3535 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3535.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":185:71
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":186:13
                let v_7469636b65745f5f30346538646661636265636661336166 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":187:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":188:7
                if ({ let left = { let left = Value::boolean(!(v_697343757272656e74.read().call_authored(vec![v_7469636b65745f5f30346538646661636265636661336166.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3536 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3536.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":188:62
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":191:7
                v_70726f706f7365.read().call_authored(vec![v_6e6578745f5f33346434383339326333616163353930.clone(), v_7469636b65745f5f30346538646661636265636661336166.clone()]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":193:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f65343636396533323833306339323731: Value = arg(&arguments, 0);
                let v_6576656e745f5f39316566653337353162303739386433: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":194:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("Escape"))))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":195:9
                    if (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3538 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3538.clone()).get() }), &(Value::string("idle"))))).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":195:37
                        { let v_7075695f6e61746976655f3539 = v_6576656e745f5f39316566653337353162303739386433.clone(); prevent(v_7075695f6e61746976655f3539.clone()) };
                    }
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":196:9
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":197:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":199:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3630 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3630.clone()).get() } } else { left } }; if !left.truthy() { (v_6576656e745f5f39316566653337353162303739386433.clone()).member("altKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f39316566653337353162303739386433.clone()).member("ctrlKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f39316566653337353162303739386433.clone()).member("metaKey", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":199:110
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":200:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("Home")))); if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("End")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("ArrowUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("ArrowRight")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("ArrowLeft")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("PageUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false)), &(Value::string("PageDown")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":203:64
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":204:7
                { let v_7075695f6e61746976655f3631 = v_6576656e745f5f39316566653337353162303739386433.clone(); prevent(v_7075695f6e61746976655f3631.clone()) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":205:7
                v_61646a757374.read().call_authored(vec![{ let left = (v_6576656e745f5f39316566653337353162303739386433.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f39316566653337353162303739386433.clone()).member("shiftKey", false)), &(Value::boolean(true))))]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":207:11
            v_73796e63.initialize({  let v_636c616d70 = v_636c616d70.clone(); let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_76616c75655f5f31323838633235346132313638323338 = v_76616c75655f5f31323838633235346132313638323338.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6973417574686f7269747943757272656e74 = v_6973417574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f64396364613464393736313131323962: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":208:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":208:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":209:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":210:13
                let v_7469636b65745f5f62636561376265623739623932313132 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":211:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":212:7
                { let value = v_63757272656e745f5f64396364613464393736313131323962.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":213:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3632 = v_63757272656e745f5f64396364613464393736313131323962.clone(); read_props(&(v_7075695f6e61746976655f3632.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":214:13
                let v_6e6578745f5f37623337313539386239613434316265 = v_636c616d70.read().call_authored(vec![{ let left = (v_70726f7073.clone()).member("value", false); if left.nullish() { Value::number(0.0) } else { left } }]);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":215:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":216:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3633 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3633.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = { let left = Value::boolean(!strict_equal(&(v_6e6578745f5f37623337313539386239613434316265.clone()), &({ let value = (v_6163636570746564.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":219:7
                { let value = v_6e6578745f5f37623337313539386239613434316265.clone(); *(v_6163636570746564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":220:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":221:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":222:9
                    { let v_7075695f6e61746976655f3634 = v_70686173655f5f37353966383966343464326634376436.read(); state_set(v_7075695f6e61746976655f3634.clone(), Value::string("idle"), Value::string("reason: normalized knob authority changed")) };
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":223:9
                    if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":223:42
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":225:7
                { let v_7075695f6e61746976655f3635 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3635.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: normalized knob label")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":226:7
                if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":226:40
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":227:7
                { let v_7075695f6e61746976655f3636 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3636.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: normalized knob policy")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":228:7
                if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":228:40
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":229:7
                { let v_7075695f6e61746976655f3637 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3637.clone()).owner(), { let v_7075695f6e61746976655f3638 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3638.clone()).get() }) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":230:7
                if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":230:40
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":231:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3639 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3639.clone()).get() }), &(Value::string("idle"))))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":232:9
                    { let v_7075695f6e61746976655f3730 = v_76616c75655f5f31323838633235346132313638323338.read(); state_set(v_7075695f6e61746976655f3730.clone(), { let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::string("reason: normalized knob authority display")) };
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":233:9
                    if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/normalized-knob.proto.ts":233:42
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":239:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":239:25
                    { let v_7075695f6e61746976655f3731 = v_63757272656e745f5f64396364613464393736313131323962.clone(); emit(&(v_7075695f6e61746976655f3731.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_6e6578745265766973696f6e.clone())])) };
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":240:7
                if (Value::boolean(!(v_6973417574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62636561376265623739623932313132.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":240:40
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":241:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":242:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":242:21
                    { let v_7075695f6e61746976655f3732 = v_63757272656e745f5f64396364613464393736313131323962.clone(); request_update(&(v_7075695f6e61746976655f3732.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":244:11
            v_726574697265.initialize({  let v_70686173655f5f37353966383966343464326634376436 = v_70686173655f5f37353966383966343464326634376436.clone(); let v_76616c75655f5f31323838633235346132313638323338 = v_76616c75655f5f31323838633235346132313638323338.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":245:13
                let v_63757272656e745f5f30373935373231353837313164373263 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":246:13
                let v_776173416374697665 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3733 = v_70686173655f5f37353966383966343464326634376436.read(); (v_7075695f6e61746976655f3733.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":247:13
                let v_62617369735f5f65316464333639373566353930636635 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":248:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":249:13
                let v_7469636b65745f5f38353834346461636565636230356436 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":250:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":251:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":252:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":253:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":257:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":257:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":258:7
                { let v_7075695f6e61746976655f3734 = v_70686173655f5f37353966383966343464326634376436.read(); state_set(v_7075695f6e61746976655f3734.clone(), Value::string("idle"), Value::string("reason: normalized knob detached")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":259:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f38353834346461636565636230356436.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":259:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":260:7
                { let v_7075695f6e61746976655f3735 = v_76616c75655f5f31323838633235346132313638323338.read(); state_set(v_7075695f6e61746976655f3735.clone(), { let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::string("reason: normalized knob detached display")) };
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":261:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f38353834346461636565636230356436.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":261:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":262:7
                if ({ let left = v_776173416374697665.clone(); if left.truthy() { v_63757272656e745f5f30373935373231353837313164373263.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":262:33
                    { let v_7075695f6e61746976655f3736 = v_63757272656e745f5f30373935373231353837313164373263.clone(); emit(&(v_7075695f6e61746976655f3736.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f65316464333639373566353930636635.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":264:5
            { let v_7075695f6e61746976655f3737 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":265:5
            { let v_7075695f6e61746976655f3738 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f65316337616464366536326635383237: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":266:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/normalized-knob.proto.ts":266:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":267:7
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":268:7
                v_73796e63.read().call_authored(vec![v_63757272656e745f5f65316337616464366536326635383237.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":270:5
            { let v_7075695f6e61746976655f3739 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("value"), Value::string("defaultValue"), Value::string("step"), Value::string("label"), Value::string("disabled"), Value::string("revision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":271:5
            { let v_7075695f6e61746976655f3830 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":275:5
            { let v_7075695f6e61746976655f3831 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":275:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":276:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":276:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/normalized-knob.proto.ts":277:5
            return { let callable={ let pui_native_83={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/normalized-knob.proto.ts":277:26
                return { let v_7075695f6e61746976655f3834 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_83.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_83.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> NormalizedKnobHandle { NormalizedKnobHandle { owner: self.owner.clone() } }
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
  impl Drop for NormalizedKnob { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for NormalizedKnob { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for NormalizedKnob {
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
