// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  
  pub use proto_ui_gpui_native as native;
  pub use native::gpui_native_owner::{ControlLabelRef, InstanceAssociations};
  use gpui::{prelude::*, App, Context, FocusHandle, Subscription, Window};
  use std::rc::Rc;
  use serde_json::Value as Json;
  use native::gpui_native_owner::*;
  
  
  fn style_vocabulary() -> &'static native::gpui_native_style::StyleVocabulary {
      static VOCABULARY: std::sync::OnceLock<native::gpui_native_style::StyleVocabulary> = std::sync::OnceLock::new();
      VOCABULARY.get_or_init(|| native::gpui_native_style::StyleVocabulary::from_json(include_str!(".proto-ui/fixtures/style-tokens.json")).expect("compiler-produced style vocabulary"))
  }
  fn accepts_context(key: &str, value: &Json) -> bool {
      match key {
          
          _ => false,
      }
  }
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("open").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("defaultChoice").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("cancelChoice").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("choices").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("responseKinds").map_or(true, |prop| prop.is_null() || (prop.as_array().is_some_and(|items| items.iter().all(|item| item.as_f64().is_some_and(f64::is_finite))) || prop.is_null() || prop.is_null())) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "revision" => value.is_string(),
        "open" => value.is_boolean(),
        "disabled" => value.is_boolean(),
        "defaultChoice" => value.as_f64().is_some_and(f64::is_finite),
        "cancelChoice" => value.as_f64().is_some_and(f64::is_finite),
        "choices" => value.as_f64().is_some_and(f64::is_finite),
        "responseKinds" => (value.as_array().is_some_and(|items| items.iter().all(|item| item.as_f64().is_some_and(f64::is_finite))) || value.is_null() || value.is_null()), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "choose" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "key" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "chooseSelection" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "requestResponse" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "focusSelf" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData1 {
    Alternative0(Vec<f64>),
    Alternative1(()),
    Alternative2(()),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "description", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "role", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<String>,
    #[serde(rename = "open", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "defaultChoice", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<f64>,
    #[serde(rename = "cancelChoice", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<f64>,
    #[serde(rename = "choices", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
    #[serde(rename = "responseKinds", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<GeneratedData1>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct OverlayControlHandle { pub owner: OwnerRef }
  impl OverlayControlHandle {
      pub fn state_0(&self) -> Result<bool, Value> { self.state("open").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("defaultChoice").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<f64, Value> { self.state("cancelChoice").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("scopeActive").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_8(&self, argument_0: f64, argument_1: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("choose").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("choose", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_9(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("key").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("key", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_10(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("chooseSelection").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("chooseSelection", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_11(&self, argument_0: String, argument_1: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("requestResponse").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestResponse", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_12(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("focusSelf").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("focusSelf", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_null()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
      pub fn owner(&self) -> OwnerWeak { Rc::downgrade(&self.owner) }
      pub fn state(&self, name: &str) -> Result<Json, Value> { catch_authored(|| { let state=self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed State")); state.get().data() }) }
      pub fn call(&self, name: &str, arguments: Vec<Json>, window: &mut Window, cx: &mut App) -> Result<Json, Value> {
          let outcome = catch_authored(|| {
          refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
          let callback = self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed method"));
          let arguments: Vec<Value> = arguments.into_iter().map(Value::json).collect(); check_method(name, &arguments);
          let result = invoke(&self.owner, "expose-method", &callback, arguments);
          let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
          if matches!(result, Value::Undefined) { Json::Null } else { result.data() }
          });
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome
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
  pub struct OverlayControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl OverlayControl {
      pub fn create_props(props: GeneratedProps, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> { Self::create(serde_json::to_value(props).expect("portable Props"), parent, window, cx) }
      pub fn create(raw_props: Json, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> {
          Self::create_with_associations(raw_props, parent, InstanceAssociations::default(), window, cx)
      }
      pub fn create_with_associations(raw_props: Json, parent: Option<OwnerWeak>, associations: InstanceAssociations, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> {
          let mut failure = None;
          let entity = cx.new(|cx| Self::new_with_associations(raw_props, parent, associations, window, cx).unwrap_or_else(|error| {
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
          Self::new_with_associations(raw_props, parent, InstanceAssociations::default(), window, cx)
      }
      pub fn new_with_associations(raw_props: Json, parent: Option<OwnerWeak>, associations: InstanceAssociations, window: &mut Window, cx: &mut Context<Self>) -> Result<Self, Value> {
          catch_authored(|| {
          let owner_ref = Owner::new(raw_props, parent.unwrap_or_default(), accepts_context, accepts_props, prop_valid, style_vocabulary());
          initialize_instance_associations(&owner_ref, associations);
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
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_6465736372697074696f6e: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_6f70656e: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_64656661756c7443686f696365: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c43686f696365: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_73636f7065: LexicalBinding<Value> = lexical_binding();
            let v_6f7665726c6179: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7265616479: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_76616c69644b696e6473: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, f64) -> bool>> = lexical_binding();
            let v_66697273744b696e64: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, f64, f64) -> f64>> = lexical_binding();
            let v_76616c696443686f69636573: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value) -> bool>> = lexical_binding();
            let v_726573706f6e7365496e646578: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool) -> f64>> = lexical_binding();
            let v_61646d697473: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, Value) -> bool>> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, Value) -> bool>> = lexical_binding();
            let v_6b6579: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":16:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("dialog"))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("open", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("defaultChoice", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("cancelChoice", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("choices", record(vec![("type", Value::string("number")), ("default", Value::number(2.0_f64))])), ("responseKinds", record(vec![("type", Value::string("object")), ("default", Value::Json(Json::Null))]))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":24:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":24:56
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":25:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("dialog"), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":25:60
            v_6f70656e.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("open"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":26:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":26:63
            let v_6d6f64616c = { let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("modal"), Value::boolean(true), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":27:11
            v_64656661756c7443686f696365.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "discrete", Value::string("defaultChoice"), Value::number(-(1.0_f64)), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":28:11
            v_63616e63656c43686f696365.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cancelChoice"), Value::number(-(1.0_f64)), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":29:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":29:34
            { let v_7075695f6e61746976655f39 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f39.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":30:11
            v_73636f7065.initialize(native_builtin_hook(&owner, "asFocusScope", || focus_setup(&owner, "focus-scope")));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":31:5
            { let v_7075695f6e61746976655f3130 = v_73636f7065.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus-scope", record(vec![("trap", Value::boolean(true)), ("loop", Value::boolean(true)), ("navigation", Value::string("tab")), ("entry", Value::string("first")), ("emptyPolicy", Value::string("container")), ("restore", Value::string("previous"))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":32:11
            v_6f7665726c6179.initialize(native_builtin_hook(&owner, "asOverlay", || overlay_setup(&owner)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":33:5
            { let v_7075695f6e61746976655f3131 = v_6f7665726c6179.read(); overlay_configure(&(v_7075695f6e61746976655f3131.clone()).owner(), record(vec![("defaultOpen", Value::boolean(true)), ("modal", Value::boolean(true)), ("portal", Value::boolean(false)), ("anchored", Value::boolean(false)), ("closeOnEscape", Value::boolean(false)), ("closeOnOutsidePress", Value::boolean(false)), ("closeOnFocusOutside", Value::boolean(false)), ("entry", Value::string("manual")), ("restore", Value::string("none")), ("layerRole", Value::string("dialog-content"))]), false) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3132 = v_6f7665726c6179.read(); overlay_keep_mounted(&(v_7075695f6e61746976655f3132.clone()).owner()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":37:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":37:34
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3133.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":37:51
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3134.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":37:69
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3135.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("modal"), v_6d6f64616c.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":38:33
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("open"), v_6f70656e.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":39:37
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("defaultChoice"), v_64656661756c7443686f696365.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":40:55
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("cancelChoice"), v_63616e63656c43686f696365.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("scopeActive"), (v_73636f7065.read()).member("active", false)) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":41:52
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_event(&owner, Value::string("response")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_event(&owner, Value::string("selection")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":44:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":45:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":45:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":45:53
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":46:11
            v_7265616479.initialize({  let v_6f70656e = v_6f70656e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":46:34
                return { let pui_native_26={ let pui_native_28={ let pui_native_30={ let pui_native_32=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_32) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_32} }; if (pui_native_30) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_30} }; if (pui_native_28) {({ let v_7075695f6e61746976655f3336 = v_6f70656e.read(); Value::boolean(((v_7075695f6e61746976655f3336.clone()).get()).bool()) }).bool()} else {pui_native_28} }; if (pui_native_26) {!(({ let v_7075695f6e61746976655f3337 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3337.clone()).get()).bool()) }).bool())} else {pui_native_26} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":50:11
            v_76616c69644b696e6473.initialize({  let v_76616c69644b696e6473 = v_76616c69644b696e6473.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_76616c7565735f5f30303633653934313764613265333933: Value = argument_0;
                let v_73746172745f5f31633637383433323661356438343363: f64 = argument_1;
                let v_656e645f5f66393965663861323962396665343436: f64 = argument_2;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":51:7
                if { let pui_native_38:f64=v_656e645f5f66393965663861323962396665343436.clone(); let pui_native_39:f64=v_73746172745f5f31633637383433323661356438343363.clone(); pui_native_38 <= pui_native_39 } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":51:25
                    return true;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":52:7
                if { let pui_native_40={ let pui_native_42:f64=v_656e645f5f66393965663861323962396665343436.clone(); let pui_native_43:f64=v_73746172745f5f31633637383433323661356438343363.clone(); pui_native_42 - pui_native_43 }; let pui_native_41=1.0_f64; (pui_native_40==pui_native_41) } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":52:30
                    return { let pui_native_44={ let pui_native_46={ let pui_native_48=(Value::boolean(strict_equal(&({ let object = v_76616c7565735f5f30303633653934313764613265333933.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f31633637383433323661356438343363.clone()))), false) }), &(Value::number(0.0_f64))))).bool(); if !(pui_native_48) {(Value::boolean(strict_equal(&({ let object = v_76616c7565735f5f30303633653934313764613265333933.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f31633637383433323661356438343363.clone()))), false) }), &(Value::number(1.0_f64))))).bool()} else {pui_native_48} }; if !(pui_native_46) {(Value::boolean(strict_equal(&({ let object = v_76616c7565735f5f30303633653934313764613265333933.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f31633637383433323661356438343363.clone()))), false) }), &(Value::number(2.0_f64))))).bool()} else {pui_native_46} }; if !(pui_native_44) {(Value::boolean(strict_equal(&({ let object = v_76616c7565735f5f30303633653934313764613265333933.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f31633637383433323661356438343363.clone()))), false) }), &(Value::number(3.0_f64))))).bool()} else {pui_native_44} };
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":53:13
                let v_6d6964646c655f5f39333536656166343635363637303133: f64 = { let pui_native_58:f64={ let pui_native_60:f64={ let pui_native_62:f64=v_73746172745f5f31633637383433323661356438343363.clone(); let pui_native_63:f64=v_656e645f5f66393965663861323962396665343436.clone(); pui_native_62 + pui_native_63 }; let pui_native_61:f64={ let pui_native_64:f64={ let pui_native_66:f64=v_73746172745f5f31633637383433323661356438343363.clone(); let pui_native_67:f64=v_656e645f5f66393965663861323962396665343436.clone(); pui_native_66 + pui_native_67 }; let pui_native_65:f64=2.0_f64; pui_native_64 % pui_native_65 }; pui_native_60 - pui_native_61 }; let pui_native_59:f64=2.0_f64; pui_native_58 / pui_native_59 };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":54:7
                return { let pui_native_68=(v_76616c69644b696e6473.read().call)(v_76616c7565735f5f30303633653934313764613265333933.clone(), v_73746172745f5f31633637383433323661356438343363.clone(), v_6d6964646c655f5f39333536656166343635363637303133.clone()); if (pui_native_68) {(v_76616c69644b696e6473.read().call)(v_76616c7565735f5f30303633653934313764613265333933.clone(), v_6d6964646c655f5f39333536656166343635363637303133.clone(), v_656e645f5f66393965663861323962396665343436.clone())} else {pui_native_68} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":56:11
            v_66697273744b696e64.initialize({  let v_66697273744b696e64 = v_66697273744b696e64.clone(); let call:Rc<dyn Fn(Value, f64, f64, f64)->f64>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64, argument_3: f64| -> f64 {
                let v_76616c7565735f5f38343233666162373131616264303738: Value = argument_0;
                let v_73746172745f5f65346330643561633939373934646164: f64 = argument_1;
                let v_656e645f5f62366563616236383762663636386232: f64 = argument_2;
                let v_6b696e64: f64 = argument_3;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":57:7
                if { let pui_native_70:f64=v_656e645f5f62366563616236383762663636386232.clone(); let pui_native_71:f64=v_73746172745f5f65346330643561633939373934646164.clone(); pui_native_70 <= pui_native_71 } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":57:25
                    return -(1.0_f64);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":58:7
                if { let pui_native_72={ let pui_native_74:f64=v_656e645f5f62366563616236383762663636386232.clone(); let pui_native_75:f64=v_73746172745f5f65346330643561633939373934646164.clone(); pui_native_74 - pui_native_75 }; let pui_native_73=1.0_f64; (pui_native_72==pui_native_73) } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":58:30
                    return if (Value::boolean(strict_equal(&({ let object = v_76616c7565735f5f38343233666162373131616264303738.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f65346330643561633939373934646164.clone()))), false) }), &(Value::number(v_6b696e64.clone()))))).bool() {v_73746172745f5f65346330643561633939373934646164.clone()} else {-(1.0_f64)};
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":59:13
                let v_6d6964646c655f5f65623764376161306563623533383334: f64 = { let pui_native_78:f64={ let pui_native_80:f64={ let pui_native_82:f64=v_73746172745f5f65346330643561633939373934646164.clone(); let pui_native_83:f64=v_656e645f5f62366563616236383762663636386232.clone(); pui_native_82 + pui_native_83 }; let pui_native_81:f64={ let pui_native_84:f64={ let pui_native_86:f64=v_73746172745f5f65346330643561633939373934646164.clone(); let pui_native_87:f64=v_656e645f5f62366563616236383762663636386232.clone(); pui_native_86 + pui_native_87 }; let pui_native_85:f64=2.0_f64; pui_native_84 % pui_native_85 }; pui_native_80 - pui_native_81 }; let pui_native_79:f64=2.0_f64; pui_native_78 / pui_native_79 };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":60:13
                let v_6669727374: f64 = (v_66697273744b696e64.read().call)(v_76616c7565735f5f38343233666162373131616264303738.clone(), v_73746172745f5f65346330643561633939373934646164.clone(), v_6d6964646c655f5f65623764376161306563623533383334.clone(), v_6b696e64.clone());
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":61:7
                return if { let pui_native_88:f64=v_6669727374.clone(); let pui_native_89:f64=0.0_f64; pui_native_88 >= pui_native_89 } {v_6669727374.clone()} else {(v_66697273744b696e64.read().call)(v_76616c7565735f5f38343233666162373131616264303738.clone(), v_6d6964646c655f5f65623764376161306563623533383334.clone(), v_656e645f5f62366563616236383762663636386232.clone(), v_6b696e64.clone())};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":63:11
            v_76616c696443686f69636573.initialize({  let v_76616c69644b696e6473 = v_76616c69644b696e6473.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_70726f70735f5f62396131356338343431353265353933: Value = argument_0;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":64:13
                let v_636f756e74: f64 = { let pui_native_90=(v_70726f70735f5f62396131356338343431353265353933.clone()).member("choices", false); if pui_native_90.nullish() {2.0_f64} else {(pui_native_90).num()} };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":65:7
                if { let pui_native_92={ let pui_native_94={ let pui_native_96=!((v_636f756e74.clone()).is_finite()); if !(pui_native_96) {{ let pui_native_98={ let pui_native_100:f64=v_636f756e74.clone(); let pui_native_101:f64=1.0_f64; pui_native_100 % pui_native_101 }; let pui_native_99=0.0_f64; !(pui_native_98==pui_native_99) }} else {pui_native_96} }; if !(pui_native_94) {{ let pui_native_102:f64=v_636f756e74.clone(); let pui_native_103:f64=0.0_f64; pui_native_102 < pui_native_103 }} else {pui_native_94} }; if !(pui_native_92) {{ let pui_native_104:f64=v_636f756e74.clone(); let pui_native_105:f64=1000000.0_f64; pui_native_104 > pui_native_105 }} else {pui_native_92} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":65:87
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":66:13
                let v_6b696e64735f5f36316661373931653133316433366664 = (v_70726f70735f5f62396131356338343431353265353933.clone()).member("responseKinds", false);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":67:7
                return { let pui_native_106={ let pui_native_108=(Value::boolean(strict_equal(&(v_6b696e64735f5f36316661373931653133316433366664.clone()), &(Value::Json(Json::Null))))).bool(); if !(pui_native_108) {(Value::boolean(strict_equal(&(v_6b696e64735f5f36316661373931653133316433366664.clone()), &(Value::Undefined)))).bool()} else {pui_native_108} }; if !(pui_native_106) {{ let pui_native_114={ let pui_native_116=(Value::boolean(matches!(&(v_6b696e64735f5f36316661373931653133316433366664.clone()), Value::Array(_) | Value::Json(Json::Array(_))))).bool(); if (pui_native_116) {{ let pui_native_118=((v_6b696e64735f5f36316661373931653133316433366664.clone()).member("length", false)).num(); let pui_native_119=v_636f756e74.clone(); (pui_native_118==pui_native_119) }} else {pui_native_116} }; if (pui_native_114) {(v_76616c69644b696e6473.read().call)(v_6b696e64735f5f36316661373931653133316433366664.clone(), 0.0_f64, ((v_6b696e64735f5f36316661373931653133316433366664.clone()).member("length", false)).num())} else {pui_native_114} }} else {pui_native_106} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":69:11
            v_726573706f6e7365496e646578.initialize({  let v_66697273744b696e64 = v_66697273744b696e64.clone(); let v_76616c696443686f69636573 = v_76616c696443686f69636573.clone(); let call:Rc<dyn Fn(Value, bool)->f64>=Rc::new(move |argument_0: Value, argument_1: bool| -> f64 {
                let v_70726f70735f5f62373933336635656261626231656335: Value = argument_0;
                let v_63616e63656c: bool = argument_1;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":70:7
                if !((v_76616c696443686f69636573.read().call)(v_70726f70735f5f62373933336635656261626231656335.clone())) {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":70:33
                    return -(1.0_f64);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":71:13
                let v_6b696e64735f5f33316330343361343239343663336331 = (v_70726f70735f5f62373933336635656261626231656335.clone()).member("responseKinds", false);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":72:7
                if { let pui_native_120=(Value::boolean(!strict_equal(&(v_6b696e64735f5f33316330343361343239343663336331.clone()), &(Value::Json(Json::Null))))).bool(); if (pui_native_120) {(Value::boolean(!strict_equal(&(v_6b696e64735f5f33316330343361343239343663336331.clone()), &(Value::Undefined)))).bool()} else {pui_native_120} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":73:15
                    let v_5f5f7075694d6f64756c6530: f64 = (v_66697273744b696e64.read().call)(v_6b696e64735f5f33316330343361343239343663336331.clone(), 0.0_f64, ((v_6b696e64735f5f33316330343361343239343663336331.clone()).member("length", false)).num(), if v_63616e63656c.clone() {2.0_f64} else {3.0_f64});
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":74:9
                    return if { let pui_native_126={ let pui_native_128:f64=v_5f5f7075694d6f64756c6530.clone(); let pui_native_129:f64=0.0_f64; pui_native_128 >= pui_native_129 }; if !(pui_native_126) {v_63616e63656c.clone()} else {pui_native_126} } {v_5f5f7075694d6f64756c6530.clone()} else {(v_66697273744b696e64.read().call)(v_6b696e64735f5f33316330343361343239343663336331.clone(), 0.0_f64, ((v_6b696e64735f5f33316330343361343239343663336331.clone()).member("length", false)).num(), 1.0_f64)};
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":76:13
                let v_696e6465785f5f64653939636237346336636535313738: f64 = if v_63616e63656c.clone() {{ let pui_native_130=(v_70726f70735f5f62373933336635656261626231656335.clone()).member("cancelChoice", false); if pui_native_130.nullish() {0.0_f64} else {(pui_native_130).num()} }} else {{ let pui_native_132=(v_70726f70735f5f62373933336635656261626231656335.clone()).member("defaultChoice", false); if pui_native_132.nullish() {1.0_f64} else {(pui_native_132).num()} }};
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":77:7
                return if { let pui_native_134={ let pui_native_136={ let pui_native_138=(v_696e6465785f5f64653939636237346336636535313738.clone()).is_finite(); if (pui_native_138) {{ let pui_native_140={ let pui_native_142:f64=v_696e6465785f5f64653939636237346336636535313738.clone(); let pui_native_143:f64=1.0_f64; pui_native_142 % pui_native_143 }; let pui_native_141=0.0_f64; (pui_native_140==pui_native_141) }} else {pui_native_138} }; if (pui_native_136) {{ let pui_native_144:f64=v_696e6465785f5f64653939636237346336636535313738.clone(); let pui_native_145:f64=0.0_f64; pui_native_144 >= pui_native_145 }} else {pui_native_136} }; if (pui_native_134) {{ let pui_native_146:f64=v_696e6465785f5f64653939636237346336636535313738.clone(); let pui_native_147:f64={ let pui_native_148=(v_70726f70735f5f62373933336635656261626231656335.clone()).member("choices", false); if pui_native_148.nullish() {2.0_f64} else {(pui_native_148).num()} }; pui_native_146 < pui_native_147 }} else {pui_native_134} } {v_696e6465785f5f64653939636237346336636535313738.clone()} else {-(1.0_f64)};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":79:11
            v_61646d697473.initialize({  let v_72756e = v_72756e.clone(); let v_7265616479 = v_7265616479.clone(); let v_76616c696443686f69636573 = v_76616c696443686f69636573.clone(); let call:Rc<dyn Fn(f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value| -> bool {
                let v_696e6465785f5f32373839383963313039383662326438: f64 = argument_0;
                let v_62617369735f5f66373432323564313239326366396138: Value = argument_1;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":80:7
                if { let pui_native_150=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_150) {!((v_7265616479.read().call)())} else {pui_native_150} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":80:29
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":81:13
                let v_70726f70735f5f34383666323938613962366436343563 = { let v_7075695f6e61746976655f313532 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313532.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":82:7
                if { let pui_native_153={ let pui_native_155={ let pui_native_157={ let pui_native_159={ let pui_native_161=(Value::boolean(!strict_equal(&(v_62617369735f5f66373432323564313239326366396138.clone()), &({ let left = (v_70726f70735f5f34383666323938613962366436343563.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool(); if !(pui_native_161) {!((v_76616c696443686f69636573.read().call)(v_70726f70735f5f34383666323938613962366436343563.clone()))} else {pui_native_161} }; if !(pui_native_159) {!((v_696e6465785f5f32373839383963313039383662326438.clone()).is_finite())} else {pui_native_159} }; if !(pui_native_157) {{ let pui_native_165={ let pui_native_167:f64=v_696e6465785f5f32373839383963313039383662326438.clone(); let pui_native_168:f64=1.0_f64; pui_native_167 % pui_native_168 }; let pui_native_166=0.0_f64; !(pui_native_165==pui_native_166) }} else {pui_native_157} }; if !(pui_native_155) {{ let pui_native_169:f64=v_696e6465785f5f32373839383963313039383662326438.clone(); let pui_native_170:f64=0.0_f64; pui_native_169 < pui_native_170 }} else {pui_native_155} }; if !(pui_native_153) {{ let pui_native_171:f64=v_696e6465785f5f32373839383963313039383662326438.clone(); let pui_native_172:f64=((v_70726f70735f5f34383666323938613962366436343563.clone()).member("choices", false)).num(); pui_native_171 >= pui_native_172 }} else {pui_native_153} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":82:161
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":83:13
                let v_6b696e64735f5f62613262326438376465663065633465 = (v_70726f70735f5f34383666323938613962366436343563.clone()).member("responseKinds", false);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:7
                if { let pui_native_175={ let pui_native_177=(Value::boolean(!strict_equal(&(v_6b696e64735f5f62613262326438376465663065633465.clone()), &(Value::Json(Json::Null))))).bool(); if (pui_native_177) {(Value::boolean(!strict_equal(&(v_6b696e64735f5f62613262326438376465663065633465.clone()), &(Value::Undefined)))).bool()} else {pui_native_177} }; if (pui_native_175) {(Value::boolean(strict_equal(&({ let object = v_6b696e64735f5f62613262326438376465663065633465.clone(); object.caught_member(&property_key(&(Value::number(v_696e6465785f5f32373839383963313039383662326438.clone()))), false) }), &(Value::number(0.0_f64))))).bool()} else {pui_native_175} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:72
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":85:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:11
            v_63686f6f7365.initialize({  let v_72756e = v_72756e.clone(); let v_61646d697473 = v_61646d697473.clone(); let call:Rc<dyn Fn(f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value| -> bool {
                let v_696e6465785f5f32373363326464346666386338356461: f64 = argument_0;
                let v_62617369735f5f30346335356530326130383361313137: Value = argument_1;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":88:7
                if { let pui_native_185=!((v_61646d697473.read().call)(v_696e6465785f5f32373363326464346666386338356461.clone(), v_62617369735f5f30346335356530326130383361313137.clone())); if !(pui_native_185) {!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy())} else {pui_native_185} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":88:42
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":89:7
                { let v_7075695f6e61746976655f313837 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313837.clone()).owner(), Value::string("response"), record(vec![("index", Value::number(v_696e6465785f5f32373363326464346666386338356461.clone())), ("revision", v_62617369735f5f30346335356530326130383361313137.clone())])) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":90:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":92:11
            v_6b6579.initialize({  let v_72756e = v_72756e.clone(); let v_7265616479 = v_7265616479.clone(); let v_726573706f6e7365496e646578 = v_726573706f6e7365496e646578.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6e616d655f5f66346261636235356164383535326464: Value = argument_0;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":93:7
                if { let pui_native_188=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_188) {!((v_7265616479.read().call)())} else {pui_native_188} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":93:29
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":94:13
                let v_70726f70735f5f36653130366238326466373363336637 = { let v_7075695f6e61746976655f313930 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313930.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":95:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f66346261636235356164383535326464.clone()), &(Value::string("Escape"))))).bool() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":95:30
                    return (v_63686f6f7365.read().call)((v_726573706f6e7365496e646578.read().call)(v_70726f70735f5f36653130366238326466373363336637.clone(), true), { let left = (v_70726f70735f5f36653130366238326466373363336637.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } });
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":96:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f66346261636235356164383535326464.clone()), &(Value::string("Enter"))))).bool() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":96:29
                    return (v_63686f6f7365.read().call)((v_726573706f6e7365496e646578.read().call)(v_70726f70735f5f36653130366238326466373363336637.clone(), false), { let left = (v_70726f70735f5f36653130366238326466373363336637.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } });
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":97:7
                return false;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":99:5
            { let v_7075695f6e61746976655f313935 = v_646566.clone(); expose_method(&owner, Value::string("choose"), (v_63686f6f7365.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":99:42
            { let v_7075695f6e61746976655f313936 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b6579.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":102:5
            { let v_7075695f6e61746976655f313937 = v_646566.clone(); expose_method(&owner, Value::string("chooseSelection"), ({  let v_72756e = v_72756e.clone(); let v_61646d697473 = v_61646d697473.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f66356235303038356234656332386233: f64 = argument_0;
                let v_73656c6563746564: bool = argument_1;
                let v_62617369735f5f63393931323163356139666265343763: Value = argument_2;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":103:7
                if { let pui_native_198={ let pui_native_200=(Value::boolean(!strict_equal(&(typeof_value(&(Value::boolean(v_73656c6563746564.clone())))), &(Value::string("boolean"))))).bool(); if !(pui_native_200) {!((v_61646d697473.read().call)(v_696e6465785f5f66356235303038356234656332386233.clone(), v_62617369735f5f63393931323163356139666265343763.clone()))} else {pui_native_200} }; if !(pui_native_198) {!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy())} else {pui_native_198} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":103:75
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":104:7
                { let v_7075695f6e61746976655f323034 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f323034.clone()).owner(), Value::string("selection"), record(vec![("index", Value::number(v_696e6465785f5f66356235303038356234656332386233.clone())), ("selected", Value::boolean(v_73656c6563746564.clone())), ("revision", v_62617369735f5f63393931323163356139666265343763.clone())])) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":105:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":109:5
            { let v_7075695f6e61746976655f323035 = v_646566.clone(); expose_method(&owner, Value::string("requestResponse"), ({  let v_72756e = v_72756e.clone(); let v_6b6579 = v_6b6579.clone(); let call:Rc<dyn Fn(Value, Value)->bool>=Rc::new(move |argument_0: Value, argument_1: Value| -> bool {
                let v_6e616d655f5f35363566396663313366663731636639: Value = argument_0;
                let v_62617369735f5f36326363396133303639633438633038: Value = argument_1;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":110:7
                if { let pui_native_206=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_206) {(Value::boolean(!strict_equal(&(v_62617369735f5f36326363396133303639633438633038.clone()), &({ let left = ({ let v_7075695f6e61746976655f323130 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f323130.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_206} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":110:63
                    return false;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":111:7
                return (v_6b6579.read().call)(v_6e616d655f5f35363566396663313366663731636639.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), arg(&arguments,1)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":113:5
            { let v_7075695f6e61746976655f323131 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":113:44
                if (v_7265616479.read().call)() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":113:57
                    { let v_7075695f6e61746976655f323132 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323132.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":114:5
            { let v_7075695f6e61746976655f323133 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e74: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":115:7
                if (v_6b6579.read().call)({ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }) {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":115:33
                    { let v_7075695f6e61746976655f323134 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f323134.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":117:11
            v_73796e63.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6f70656e = v_6f70656e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_64656661756c7443686f696365 = v_64656661756c7443686f696365.clone(); let v_63616e63656c43686f696365 = v_63616e63656c43686f696365.clone(); let v_666f637573 = v_666f637573.clone(); let v_73636f7065 = v_73636f7065.clone(); let v_6f7665726c6179 = v_6f7665726c6179.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_726573706f6e7365496e646578 = v_726573706f6e7365496e646578.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f38386435313231356239363632396432: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":118:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":118:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":119:7
                { let value = v_6f776e65725f5f38386435313231356239363632396432.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":119:26
                let v_7469636b6574: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":119:50
                let v_70726f70735f5f32343164613164313638373632356661 = { let v_7075695f6e61746976655f323135 = v_6f776e65725f5f38386435313231356239363632396432.clone(); read_props(&(v_7075695f6e61746976655f323135.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":120:7
                { let v_7075695f6e61746976655f323136 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f323136.clone(), { let left = (v_70726f70735f5f32343164613164313638373632356661.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: overlay name")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":121:7
                if { let pui_native_217={ let pui_native_219=v_7469636b6574.clone(); let pui_native_220={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_219==pui_native_220) }; if !(pui_native_217) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_217} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":121:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":122:7
                { let v_7075695f6e61746976655f323231 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f323231.clone(), { let left = (v_70726f70735f5f32343164613164313638373632356661.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: overlay description")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":123:7
                if { let pui_native_222={ let pui_native_224=v_7469636b6574.clone(); let pui_native_225={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_224==pui_native_225) }; if !(pui_native_222) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_222} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":123:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":124:7
                { let v_7075695f6e61746976655f323236 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f323236.clone(), { let left = (v_70726f70735f5f32343164613164313638373632356661.clone()).member("role", false); if left.nullish() { Value::string("dialog") } else { left } }, Value::string("reason: overlay role")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":125:7
                if { let pui_native_227={ let pui_native_229=v_7469636b6574.clone(); let pui_native_230={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_229==pui_native_230) }; if !(pui_native_227) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_227} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":125:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":126:7
                { let v_7075695f6e61746976655f323331 = v_64656661756c7443686f696365.read(); state_set(v_7075695f6e61746976655f323331.clone(), Value::number((v_726573706f6e7365496e646578.read().call)(v_70726f70735f5f32343164613164313638373632356661.clone(), false)), Value::string("reason: overlay default response")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":127:7
                if { let pui_native_232={ let pui_native_234=v_7469636b6574.clone(); let pui_native_235={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_234==pui_native_235) }; if !(pui_native_232) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_232} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":127:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":128:7
                { let v_7075695f6e61746976655f323336 = v_63616e63656c43686f696365.read(); state_set(v_7075695f6e61746976655f323336.clone(), Value::number((v_726573706f6e7365496e646578.read().call)(v_70726f70735f5f32343164613164313638373632356661.clone(), true)), Value::string("reason: overlay cancel response")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":129:7
                if { let pui_native_237={ let pui_native_239=v_7469636b6574.clone(); let pui_native_240={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_239==pui_native_240) }; if !(pui_native_237) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_237} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":129:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":130:7
                { let v_7075695f6e61746976655f323431 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f323431.clone(), Value::boolean(((v_70726f70735f5f32343164613164313638373632356661.clone()).member("disabled", false)).bool()), Value::string("reason: overlay policy")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":131:7
                if { let pui_native_244={ let pui_native_246=v_7469636b6574.clone(); let pui_native_247={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_246==pui_native_247) }; if !(pui_native_244) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_244} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":131:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":132:7
                { let v_7075695f6e61746976655f323438 = v_6f70656e.read(); state_set(v_7075695f6e61746976655f323438.clone(), Value::boolean(((v_70726f70735f5f32343164613164313638373632356661.clone()).member("open", false)).bool()), Value::string("reason: accepted overlay visibility")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":133:7
                if { let pui_native_251={ let pui_native_253=v_7469636b6574.clone(); let pui_native_254={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_253==pui_native_254) }; if !(pui_native_251) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_251} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":133:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":134:7
                { let v_7075695f6e61746976655f323535 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f323535.clone()).owner(), Value::boolean({ let pui_native_256=!(({ let v_7075695f6e61746976655f323538 = v_6f70656e.read(); Value::boolean(((v_7075695f6e61746976655f323538.clone()).get()).bool()) }).bool()); if !(pui_native_256) {({ let v_7075695f6e61746976655f323539 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323539.clone()).get()).bool()) }).bool()} else {pui_native_256} })) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":135:7
                if { let pui_native_260={ let pui_native_262=v_7469636b6574.clone(); let pui_native_263={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_262==pui_native_263) }; if !(pui_native_260) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_260} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":135:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":136:7
                if ({ let v_7075695f6e61746976655f323634 = v_6f70656e.read(); Value::boolean(((v_7075695f6e61746976655f323634.clone()).get()).bool()) }).bool() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":136:23
                    { let v_7075695f6e61746976655f323635 = v_6f7665726c6179.read(); overlay_intent(&(v_7075695f6e61746976655f323635.clone()).owner(), "openOverlay", Value::string("controlled.sync")) };
                } else {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":136:68
                    { let v_7075695f6e61746976655f323636 = v_6f7665726c6179.read(); overlay_intent(&(v_7075695f6e61746976655f323636.clone()).owner(), "close", Value::string("controlled.sync")) };
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":137:7
                if { let pui_native_267={ let pui_native_269={ let pui_native_271=v_7469636b6574.clone(); let pui_native_272={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_271==pui_native_272) }; if !(pui_native_269) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_269} }; if !(pui_native_267) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_267} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":137:54
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":138:7
                if { let pui_native_273=({ let v_7075695f6e61746976655f323735 = v_6f70656e.read(); Value::boolean(((v_7075695f6e61746976655f323735.clone()).get()).bool()) }).bool(); if (pui_native_273) {!(({ let v_7075695f6e61746976655f323736 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f323736.clone()).owner())).bool()) }).bool())} else {pui_native_273} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":138:44
                    { let v_7075695f6e61746976655f323737 = v_73636f7065.read(); focus_activate(&(v_7075695f6e61746976655f323737.clone()).owner(), Value::Undefined) };
                } else {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":139:12
                    if { let pui_native_278=!(({ let v_7075695f6e61746976655f323830 = v_6f70656e.read(); Value::boolean(((v_7075695f6e61746976655f323830.clone()).get()).bool()) }).bool()); if (pui_native_278) {({ let v_7075695f6e61746976655f323831 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f323831.clone()).owner())).bool()) }).bool()} else {pui_native_278} } {
                        // Source "frontend/proto-ui/src/overlay-control.proto.ts":139:49
                        { let v_7075695f6e61746976655f323832 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f323832.clone()).owner(), Value::Undefined) };
                    }
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":140:7
                if { let pui_native_283={ let pui_native_285=v_7469636b6574.clone(); let pui_native_286={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_285==pui_native_286) }; if !(pui_native_283) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_283} } {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":140:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":141:7
                { let v_7075695f6e61746976655f323837 = v_6f776e65725f5f38386435313231356239363632396432.clone(); request_update(&(v_7075695f6e61746976655f323837.clone()).owner()) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":143:5
            { let v_7075695f6e61746976655f323838 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":144:5
            { let v_7075695f6e61746976655f323839 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f66666633343831316437333438656534: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":144:42
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":144:59
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":144:76
                    v_73796e63.read().call_authored(vec![v_6f776e65725f5f66666633343831316437333438656534.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":145:5
            { let v_7075695f6e61746976655f323930 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description"), Value::string("role"), Value::string("open"), Value::string("disabled"), Value::string("defaultChoice"), Value::string("cancelChoice"), Value::string("choices"), Value::string("responseKinds"), Value::string("revision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:5
            { let v_7075695f6e61746976655f323931 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_73636f7065 = v_73636f7065.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:39
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:48
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:66
                if ({ let v_7075695f6e61746976655f323932 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f323932.clone()).owner())).bool()) }).bool() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:88
                    { let v_7075695f6e61746976655f323933 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f323933.clone()).owner(), Value::Undefined) };
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":146:108
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":147:5
            { let v_7075695f6e61746976655f323934 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":147:43
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":147:52
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":147:69
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":147:87
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":148:5
            { let v_7075695f6e61746976655f323935 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":149:5
            return { let callable={ let pui_native_296={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":149:24
                return { let v_7075695f6e61746976655f323937 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_296.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_296.call)(arg(&arguments,0))})} }; match callable {
      Value::NativeFunction{identity,render} => capture_renderer(Value::NativeFunction{identity,render:render.clone()},render),
      Value::Renderer(adapter) => Value::Renderer(adapter),
      Value::Function(callback) => capture_renderer(Value::Function(callback.clone()),Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
      _ => panic!("checked setup result must be a callable")
    } };
        })) }.call_authored(vec![definition_capability(&owner)]);
          Owner::finish(&owner_ref, renderer);
          // Establish the App's default-action snapshot before any authored
          // global key capture can replace focus during the same keystroke.
          register_sequential_focus_owner(&owner_ref, cx);
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
              notify_native_change(cx);
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
              notify_native_change(cx);
          });
          // A native slot descendant can start this root's keyboard press
          // while the root FocusHandle itself is not focused. Its departure
          // from the physical subtree is distinct from the root's own blur.
          let focus_out_owner = Rc::downgrade(&owner_ref);
          let on_focus_out = cx.on_focus_out(&focus, window, move |view, _, window, cx| {
              let Some(owner) = focus_out_owner.upgrade() else { return; };
              native_host_callback(&owner, "focus-out", || {
              let epoch = owner.borrow().epoch;
              if !ready(&owner) || owner.borrow().window_id != Some(window.window_handle().window_id()) || view.focus.contains_focused(window, cx) { return; }
              // This retires only an existing root press lease. Do not forge
              // host:blur or change the root's own Focus observation here.
              native_press_blur(&owner);
              if ready(&owner) && owner.borrow().epoch == epoch { view.drive(window, cx).unwrap_or_else(|error| throw_value(error)); }
              });
              notify_native_change(cx);
          });
          vec![on_focus, on_blur, on_focus_out]
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
      pub fn handle(&self) -> OverlayControlHandle { OverlayControlHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let outcome = catch_authored(|| replace_props(&self.owner, raw));
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome?; self.drive(window, cx)
      }
      pub fn set_instance_associations(&mut self, associations: InstanceAssociations, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let outcome = catch_authored(|| replace_instance_associations(&self.owner, associations));
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome?; drive_owners(window, cx); self.drive(window, cx)
      }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_focus_observation_needed(&self.owner) && !native::gpui_native_move::move_owner_has_binding(&self.owner) && !control_label_is_label(&self.owner) { self._window_activation.take(); return; }
          let id = window.window_handle().window_id();
          if self._window_activation.as_ref().is_some_and(|(bound, _)| *bound == id) { return; }
          self._window_activation.take();
          let owner = self.owner.clone();
          let subscription = cx.observe_window_activation(window, move |view, window, cx| {
              let deactivated = owner.borrow().window_id == Some(window.window_handle().window_id()) && !window.is_window_active();
              native_host_callback(&owner, "window-activation", || {
                  if deactivated {
                      native_window_blur(&owner);
                      view.drive(window, cx).unwrap_or_else(|error| throw_value(error));
                  }
              });
              if deactivated { notify_native_change(cx); }
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
      fn notify_after_turn(&self, cx: &mut Context<Self>) {
          // GPUI suppresses Entity observer effects while a window is drawing,
          // including its focus/blur phase. Keep authored state synchronous,
          // then publish the host change after that physical draw lease ends.
          // A weak target cannot keep a detached component alive for delivery.
          notify_native_change(cx);
      }
      pub fn drive(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let result = catch_authored(|| {
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); return; }
          modules_drive(&self.owner, &self.focus, window, cx);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); return; }
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          drive_queued_focus(&self.owner, window, cx);
          });
          // A callback can publish before failing or retiring its owner. That
          // output is independent from repaint dirtiness and an unread queue.
          let dirty = std::mem::take(&mut self.owner.borrow_mut().dirty);
          if !notify_native_events(&self.owner, cx) && dirty { self.notify_after_turn(cx); }
          result
      }
  }
  impl Drop for OverlayControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for OverlayControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for OverlayControl {
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
          // Drawing consumes only repaint dirtiness. A parent Render can call
          // an exposed method before this child is laid out: its independent
          // pending_native_events flag survives until the coalesced drive wakes
          // the consumer. Repeated slot projection cannot create a notify loop.
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
