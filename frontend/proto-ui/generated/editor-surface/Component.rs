// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  
  pub use proto_ui_gpui_native as native;
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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("threshold").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("commandMode").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "threshold" => value.as_f64().is_some_and(f64::is_finite),
        "commandMode" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "cancelPointer" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "activate" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "doubleActivate" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "wheel" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "pinch" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<bool>,
    #[serde(rename = "threshold", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "commandMode", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData1 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "x")]
    pub field_1: f64,
    #[serde(rename = "y")]
    pub field_2: f64,
    #[serde(rename = "amount")]
    pub field_3: f64,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct EditorSurfaceHandle { pub owner: OwnerRef }
  impl EditorSurfaceHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<String, Value> { self.state("pointer").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_7(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("begin").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("begin", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_8(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_9(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_10(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_11(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("cancelPointer").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("cancelPointer", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_12(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("activate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("activate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_13(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("doubleActivate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("doubleActivate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_14(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("wheel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("wheel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("amount").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_15(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("pinch").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("pinch", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
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
  pub struct EditorSurface { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl EditorSurface {
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
            let v_7068617365: LexicalBinding<Value> = lexical_binding();
            let v_706f696e746572: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6f726967696e58: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6f726967696e59: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_66696e697465: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("threshold", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("commandMode", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":22:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":23:11
            v_706f696e746572.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("pointer"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":24:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":25:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":26:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":27:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":28:5
            { let v_7075695f6e61746976655f35 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f35.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":29:5
            { let v_7075695f6e61746976655f36 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f36.clone()).owner(), Value::string("group")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":30:5
            { let v_7075695f6e61746976655f37 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f37.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":31:5
            { let v_7075695f6e61746976655f38 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f38.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":32:5
            { let v_7075695f6e61746976655f39 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("overflow-hidden".into(), "overflow-hidden".into())])]) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":33:5
            { let v_7075695f6e61746976655f3130 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":34:5
            { let v_7075695f6e61746976655f3131 = v_646566.clone(); expose_state(&owner, Value::string("pointer"), v_706f696e746572.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":35:5
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":36:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":37:5
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":38:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":39:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":40:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":41:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":42:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":43:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":44:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":45:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":46:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":47:9
            v_6f726967696e58.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":48:9
            v_6f726967696e59.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":49:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f35613962393131623665306664653164: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":49:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f35613962393131623665306664653164.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":50:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f32363039316565353064326436633635: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":50:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32363039316565353064326436633635.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:11
            v_66696e697465.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_785f5f61333062333464373231633834646637: Value = arg(&arguments, 0);
                let v_795f5f65313738626562663963333736343034: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:55
                return { let left = number_is_finite(&(v_785f5f61333062333464373231633834646637.clone())); if left.truthy() { number_is_finite(&(v_795f5f65313738626562663963333736343034.clone())) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":52:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":53:13
                let v_6f776e65725f5f30356463623064326661616337353133 = { let v_7075695f6e61746976655f3137 = v_706f696e746572.read(); (v_7075695f6e61746976655f3137.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":54:13
                let v_6163746976655f5f61636435306134383038376265373137 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3138 = v_7068617365.read(); (v_7075695f6e61746976655f3138.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":55:13
                let v_7469636b65745f5f66663666626236373731643130633064 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":56:7
                { let v_7075695f6e61746976655f3139 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3139.clone(), Value::string("idle"), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":57:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66663666626236373731643130633064.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":57:29
                    return v_6163746976655f5f61636435306134383038376265373137.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":58:7
                { let v_7075695f6e61746976655f3230 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3230.clone(), Value::string(""), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":59:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66663666626236373731643130633064.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":59:29
                    return v_6163746976655f5f61636435306134383038376265373137.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":60:7
                if ({ let left = v_6163746976655f5f61636435306134383038376265373137.clone(); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":60:26
                    { let v_7075695f6e61746976655f3231 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3231.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f30356463623064326661616337353133.clone()), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:7
                if ({ let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66663666626236373731643130633064.clone()]); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:35
                    { let v_7075695f6e61746976655f3232 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3232.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":62:7
                return v_6163746976655f5f61636435306134383038376265373137.clone();
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":64:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f39326539646364366435333332313431: Value = arg(&arguments, 0);
                let v_785f5f31376562303431336463326162373439: Value = arg(&arguments, 1);
                let v_795f5f63316430346130636166373732396636: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":65:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3234 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3234.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3235 = v_7068617365.read(); (v_7075695f6e61746976655f3235.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6f776e65725f5f39326539646364366435333332313431.clone()), &(Value::string("")))) } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f31376562303431336463326162373439.clone(), v_795f5f63316430346130636166373732396636.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":66:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:13
                let v_7469636b65745f5f37353835383339386462653438333864 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":68:7
                { let value = v_785f5f31376562303431336463326162373439.clone(); *(v_6f726967696e58.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":69:7
                { let value = v_795f5f63316430346130636166373732396636.clone(); *(v_6f726967696e59.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":70:7
                { let v_7075695f6e61746976655f3236 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3236.clone(), v_6f776e65725f5f39326539646364366435333332313431.clone(), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f37353835383339386462653438333864.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":72:7
                { let v_7075695f6e61746976655f3237 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3237.clone(), Value::string("pressed"), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f37353835383339386462653438333864.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":74:7
                { let v_7075695f6e61746976655f3238 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3238.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f37353835383339386462653438333864.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:16
                    { let v_7075695f6e61746976655f3239 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3239.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f37353835383339386462653438333864.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":79:11
            let v_73616d706c65 = {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f62363964373233643834373736633235: Value = arg(&arguments, 0);
                let v_785f5f34343838323539343938346536663732: Value = arg(&arguments, 1);
                let v_795f5f32646235383538393331633365373934: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":80:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3330 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3330.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3331 = v_7068617365.read(); (v_7075695f6e61746976655f3331.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3332 = v_706f696e746572.read(); (v_7075695f6e61746976655f3332.clone()).get() }), &(v_6f776e65725f5f62363964373233643834373736633235.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":80:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f34343838323539343938346536663732.clone(), v_795f5f32646235383538393331633365373934.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":82:13
                let v_7469636b65745f5f61653765366661383139633362383766 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":83:13
                let v_726177 = { let left = ({ let v_7075695f6e61746976655f3333 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3333.clone()).owner(), false) }).member("threshold", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":84:13
                let v_7468726573686f6c64 = if ({ let left = { let relational_left = v_726177.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { number_is_finite(&(v_726177.clone())) } else { left } }).truthy() { v_726177.clone() } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":85:13
                let v_64785f5f36646561666661643831616439656335 = Value::number((v_785f5f34343838323539343938346536663732.clone()).scalar_number() - ({ let value = (v_6f726967696e58.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":86:13
                let v_64795f5f38323461303862623139306536396232 = Value::number((v_795f5f32646235383538393331633365373934.clone()).scalar_number() - ({ let value = (v_6f726967696e59.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":87:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3334 = v_7068617365.read(); (v_7075695f6e61746976655f3334.clone()).get() }), &(Value::string("pressed")))); if left.truthy() { { let relational_left = add(Value::number((v_64785f5f36646561666661643831616439656335.clone()).scalar_number() * (v_64785f5f36646561666661643831616439656335.clone()).scalar_number()), Value::number((v_64795f5f38323461303862623139306536396232.clone()).scalar_number() * (v_64795f5f38323461303862623139306536396232.clone()).scalar_number())); let relational_right = Value::number((v_7468726573686f6c64.clone()).scalar_number() * (v_7468726573686f6c64.clone()).scalar_number()); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":87:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":88:7
                { let v_7075695f6e61746976655f3335 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3335.clone(), Value::string("dragging"), Value::string("reason: editing movement accepted")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f61653765366661383139633362383766.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:16
                    { let v_7075695f6e61746976655f3336 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3336.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":91:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f61653765366661383139633362383766.clone()]);
            })) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":93:5
            { let v_7075695f6e61746976655f3337 = v_646566.clone(); expose_method(&owner, Value::string("sample"), v_73616d706c65.clone()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":94:5
            { let v_7075695f6e61746976655f3338 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f65636430623365326435373637643366: Value = arg(&arguments, 0);
                let v_785f5f33356639623835653236393336363837: Value = arg(&arguments, 1);
                let v_795f5f63366565323665373533633838383339: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3339 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3339.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3430 = v_7068617365.read(); (v_7075695f6e61746976655f3430.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3431 = v_706f696e746572.read(); (v_7075695f6e61746976655f3431.clone()).get() }), &(v_6f776e65725f5f65636430623365326435373637643366.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f33356639623835653236393336363837.clone(), v_795f5f63366565323665373533633838383339.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":97:13
                let v_7469636b65745f5f34646663376165383537336433636438 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:7
                { let v_7075695f6e61746976655f3432 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3432.clone(), Value::string("idle"), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f34646663376165383537336433636438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:7
                { let v_7075695f6e61746976655f3433 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3433.clone(), Value::string(""), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":101:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f34646663376165383537336433636438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":101:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":102:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":102:16
                    { let v_7075695f6e61746976655f3434 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3434.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":103:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f34646663376165383537336433636438.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:5
            { let v_7075695f6e61746976655f3435 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:5
            { let v_7075695f6e61746976655f3436 = v_646566.clone(); expose_method(&owner, Value::string("cancelPointer"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f65393532386164353130663039353035: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":107:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3437 = v_7068617365.read(); (v_7075695f6e61746976655f3437.clone()).get() }), &(Value::string("idle")))); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3438 = v_706f696e746572.read(); (v_7075695f6e61746976655f3438.clone()).get() }), &(v_6f776e65725f5f65393532386164353130663039353035.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":107:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":108:7
                return v_63616e63656c.read().call_authored(vec![]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":110:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3439 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3439.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:13
                let v_7469636b65745f5f64653861326532366634666365366135 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64653861326532366634666365366135.clone()])).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3530 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3530.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":115:7
                { let v_7075695f6e61746976655f3531 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3531.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":116:7
                return { let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64653861326532366634666365366135.clone()]); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3532 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3532.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":118:5
            { let v_7075695f6e61746976655f3533 = v_646566.clone(); expose_method(&owner, Value::string("activate"), v_6163746976617465.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:5
            { let v_7075695f6e61746976655f3534 = v_646566.clone(); expose_method(&owner, Value::string("doubleActivate"), {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_636c69636b73: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:61
                return { let left = { let left = number_is_finite(&(v_636c69636b73.clone())); if left.truthy() { { let relational_left = v_636c69636b73.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { v_6163746976617465.read().call_authored(vec![]) } else { left } };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":122:5
            { let v_7075695f6e61746976655f3535 = v_646566.clone(); expose_method(&owner, Value::string("wheel"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f33656263656437613238313961373037: Value = arg(&arguments, 0);
                let v_64795f5f39393361343331323363343563613733: Value = arg(&arguments, 1);
                let v_636f6e74726f6c: Value = arg(&arguments, 2);
                let v_7368696674: Value = arg(&arguments, 3);
                let v_616c74: Value = arg(&arguments, 4);
                let v_617272616e6765: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3536 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3536.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_64785f5f33656263656437613238313961373037.clone(), v_64795f5f39393361343331323363343563613733.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:90
                    return record(vec![("kind", Value::string("none")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:7
                if (v_636f6e74726f6c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:20
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", v_64795f5f39393361343331323363343563613733.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:7
                if ({ let left = Value::boolean(!(v_617272616e6765.clone()).truthy()); if left.truthy() { v_616c74.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:28
                    return record(vec![("kind", Value::string("zoom-y")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", v_64795f5f39393361343331323363343563613733.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:7
                if (v_7368696674.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:18
                    return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(if (Value::boolean(!strict_equal(&(v_64785f5f33656263656437613238313961373037.clone()), &(Value::number(0.0))))).truthy() { v_64785f5f33656263656437613238313961373037.clone() } else { v_64795f5f39393361343331323363343563613733.clone() }).scalar_number())), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:7
                if ({ let left = v_617272616e6765.clone(); if left.truthy() { Value::boolean(strict_equal(&(v_64785f5f33656263656437613238313961373037.clone()), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:32
                    return record(vec![("kind", Value::string("none")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":128:7
                return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(v_64785f5f33656263656437613238313961373037.clone()).scalar_number())), ("y", if (v_617272616e6765.clone()).truthy() { Value::number(0.0) } else { Value::number(-(v_64795f5f39393361343331323363343563613733.clone()).scalar_number()) }), ("amount", Value::number(0.0))]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); expose_method(&owner, Value::string("pinch"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7363616c65: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":131:7
                return { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3538 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3538.clone()).get() }).truthy()) } else { left } }; if left.truthy() { number_is_finite(&(v_7363616c65.clone())) } else { left } }; if left.truthy() { { let relational_left = v_7363616c65.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":133:5
            { let v_7075695f6e61746976655f3539 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":134:13
                let v_68616e646c655f5f34663565663535666263653738646132 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":135:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f34663565663535666263653738646132.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3630 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3630.clone()).get() } } else { left } }; if !left.truthy() { (v_6576656e74.clone()).member("repeat", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":135:90
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":136:13
                let v_617564696f = Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3631 = v_68616e646c655f5f34663565663535666263653738646132.clone(); read_props(&(v_7075695f6e61746976655f3631.clone()).owner(), false) }).member("commandMode", false)), &(Value::string("audio"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:13
                let v_7365636f6e64617279 = { let left = (v_6576656e74.clone()).member("ctrlKey", false); if !left.truthy() { (v_6576656e74.clone()).member("metaKey", false) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:11
                let v_636f6d6d616e64 = Rc::new(std::cell::RefCell::new(Value::string("")));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":139:7
                if (v_617564696f.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":140:9
                    if ({ let left = { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Backspace")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":140:82
                        { let value = if ((v_6576656e74.clone()).member("shiftKey", false)).truthy() { Value::string("ripple-delete") } else { Value::string("delete") }; *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:14
                        if ({ let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("i")))); if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:50
                            { let value = Value::string("split"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:14
                            if ({ let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("a")))); if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:50
                                { let value = Value::string("select-all"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":143:14
                                if ({ let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("j")))); if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":143:50
                                    { let value = Value::string("join"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":144:14
                                    if ({ let left = { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("r")))); if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e74.clone()).member("altKey", false)).truthy()) } else { left } }).truthy() {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":144:68
                                        { let value = Value::string("range-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":145:14
                                        if ({ let left = { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("v")))); if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e74.clone()).member("altKey", false)).truthy()) } else { left } }).truthy() {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":145:68
                                            { let value = Value::string("select-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                        } else {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":146:14
                                            if (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape"))))).truthy() {
                                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":146:42
                                                { let value = Value::string("clear-range"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":148:7
                if ({ let left = Value::boolean(strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string("")))); if left.truthy() { { let left = Value::boolean(!strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3632 = v_7068617365.read(); (v_7075695f6e61746976655f3632.clone()).get() }), &(Value::string("idle")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":148:81
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":149:13
                let v_7469636b65745f5f39326461626434623964333433616632 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":150:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":151:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39326461626434623964333433616632.clone()])).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3633 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3633.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":151:71
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":152:7
                { let v_7075695f6e61746976655f3634 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3634.clone()) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:7
                if (Value::boolean(!strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string(""))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:27
                    { let v_7075695f6e61746976655f3635 = v_68616e646c655f5f34663565663535666263653738646132.clone(); emit(&(v_7075695f6e61746976655f3635.clone()).owner(), Value::string("command"), record(vec![("command", { let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":155:5
            { let v_7075695f6e61746976655f3636 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":156:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f64313335386537373362396635626234: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:13
                let v_7469636b65745f5f66366362636265333562663437626438 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":160:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":161:7
                { let value = v_68616e646c655f5f64313335386537373362396635626234.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3637 = v_68616e646c655f5f64313335386537373362396635626234.clone(); read_props(&(v_7075695f6e61746976655f3637.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:13
                let v_6f776e65725f5f62633062346132643236373939303762 = { let v_7075695f6e61746976655f3638 = v_706f696e746572.read(); (v_7075695f6e61746976655f3638.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:13
                let v_62617369735f5f65393366376132306163343865323934 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":166:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3639 = v_7068617365.read(); (v_7075695f6e61746976655f3639.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":167:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":168:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:9
                    { let v_7075695f6e61746976655f3730 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3730.clone(), Value::string("idle"), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":171:9
                    { let v_7075695f6e61746976655f3731 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3731.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":172:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":172:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":174:7
                { let v_7075695f6e61746976655f3732 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3732.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: editing policy")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":175:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":175:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":176:7
                { let v_7075695f6e61746976655f3733 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3733.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: editing surface label")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":177:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":177:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":178:7
                { let v_7075695f6e61746976655f3734 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3734.clone()).owner(), { let v_7075695f6e61746976655f3735 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3735.clone()).get() }) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":179:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":179:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":180:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":180:25
                    { let v_7075695f6e61746976655f3736 = v_68616e646c655f5f64313335386537373362396635626234.clone(); emit(&(v_7075695f6e61746976655f3736.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f62633062346132643236373939303762.clone()), ("revision", v_62617369735f5f65393366376132306163343865323934.clone())])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f66366362636265333562663437626438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:21
                    { let v_7075695f6e61746976655f3737 = v_68616e646c655f5f64313335386537373362396635626234.clone(); request_update(&(v_7075695f6e61746976655f3737.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:13
                let v_68616e646c655f5f32653034333330356435333730633638 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:13
                let v_6163746976655f5f35633739653434386563623036616264 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3738 = v_7068617365.read(); (v_7075695f6e61746976655f3738.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":188:13
                let v_6f776e65725f5f39643066353930633334393838333233 = { let v_7075695f6e61746976655f3739 = v_706f696e746572.read(); (v_7075695f6e61746976655f3739.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:13
                let v_62617369735f5f38316365356462333331373731303931 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:13
                let v_7469636b65745f5f39643165316431613639353664313939 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":193:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":194:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:7
                { let v_7075695f6e61746976655f3830 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3830.clone(), Value::string("idle"), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39643165316431613639353664313939.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":199:7
                { let v_7075695f6e61746976655f3831 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3831.clone(), Value::string(""), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":200:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39643165316431613639353664313939.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":200:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":201:7
                if ({ let left = v_6163746976655f5f35633739653434386563623036616264.clone(); if left.truthy() { v_68616e646c655f5f32653034333330356435333730633638.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":201:29
                    { let v_7075695f6e61746976655f3832 = v_68616e646c655f5f32653034333330356435333730633638.clone(); emit(&(v_7075695f6e61746976655f3832.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f39643066353930633334393838333233.clone()), ("revision", v_62617369735f5f38316365356462333331373731303931.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":203:5
            { let v_7075695f6e61746976655f3833 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:5
            { let v_7075695f6e61746976655f3834 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f33313938363463646638333561326137: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f33313938363463646638333561326137.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":205:5
            { let v_7075695f6e61746976655f3835 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("threshold"), Value::string("commandMode")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:5
            { let v_7075695f6e61746976655f3836 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:5
            { let v_7075695f6e61746976655f3837 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:5
            return { let callable={ let pui_native_88={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:26
                return { let v_7075695f6e61746976655f3839 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_88.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_88.call)(arg(&arguments,0))})} }; match callable {
      Value::NativeFunction{identity,render} => capture_renderer(Value::NativeFunction{identity,render:render.clone()},render),
      Value::Renderer(adapter) => Value::Renderer(adapter),
      Value::Function(callback) => capture_renderer(Value::Function(callback.clone()),Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
      _ => panic!("checked setup result must be a callable")
    } };
        })) }.call_authored(vec![definition_capability(&owner)]);
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
      pub fn handle(&self) -> EditorSurfaceHandle { EditorSurfaceHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let outcome = catch_authored(|| replace_props(&self.owner, raw));
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome?; self.drive(window, cx)
      }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_focus_observation_needed(&self.owner) && !native::gpui_native_move::move_owner_has_binding(&self.owner) { self._window_activation.take(); return; }
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
      pub fn drive(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let mut dirty = false;
          let result = catch_authored(|| {
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); dirty = true; return; }
          modules_drive(&self.owner, &self.focus, window, cx);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); dirty = true; return; }
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          drive_queued_focus(&self.owner, window, cx);
          dirty = std::mem::take(&mut self.owner.borrow_mut().dirty);
          });
          // A render-time prop watcher or a terminal callback may publish an
          // event even after ordinary drawing consumed dirty. Wake observers
          // once after this native lease; do not request an authored render.
          if !notify_native_events(&self.owner, cx) && dirty { cx.notify(); }
          result
      }
  }
  impl Drop for EditorSurface { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for EditorSurface { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for EditorSurface {
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
