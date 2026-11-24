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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("x").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("y").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("minX").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxX").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("minY").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxY").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("hasX").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("hasY").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("xLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("yLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "x" => value.as_f64().is_some_and(f64::is_finite),
        "y" => value.as_f64().is_some_and(f64::is_finite),
        "minX" => value.as_f64().is_some_and(f64::is_finite),
        "maxX" => value.as_f64().is_some_and(f64::is_finite),
        "minY" => value.as_f64().is_some_and(f64::is_finite),
        "maxY" => value.as_f64().is_some_and(f64::is_finite),
        "hasX" => value.is_boolean(),
        "hasY" => value.is_boolean(),
        "label" => value.is_string(),
        "xLabel" => value.is_string(),
        "yLabel" => value.is_string(),
        "disabled" => value.is_boolean(),
        "revision" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "openContext" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "x", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<f64>,
    #[serde(rename = "y", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<f64>,
    #[serde(rename = "minX", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "maxX", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "minY", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<f64>,
    #[serde(rename = "maxY", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<f64>,
    #[serde(rename = "hasX", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "hasY", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<String>,
    #[serde(rename = "xLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<String>,
    #[serde(rename = "yLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct GraphPointHandle { pub owner: OwnerRef }
  impl GraphPointHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("x").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("y").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<String, Value> { self.state("axis").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_6(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_12(&self, argument_0: bool, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_13(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
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
    pub fn method_14(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
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
    pub fn method_15(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_16(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("openContext").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("openContext", &arguments);
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
  pub struct GraphPoint { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl GraphPoint {
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
            let v_7068617365: LexicalBinding<Value> = lexical_binding();
            let v_78: LexicalBinding<Value> = lexical_binding();
            let v_79: LexicalBinding<Value> = lexical_binding();
            let v_61786973: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654e6f77: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654d696e: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654d6178: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_616363657074656458: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_616363657074656459: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6c6f7758: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6869676858: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6c6f7759: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6869676859: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_68617358: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_68617359: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_626173654c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_784c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_794c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_636c616d70: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64) -> f64>> = lexical_binding();
            let v_70726f6a656374696f6e: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool) -> bool>> = lexical_binding();
            let v_7075626c697368: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64) -> bool>> = lexical_binding();
            let v_63616e63656c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, bool, f64) -> bool>> = lexical_binding();
            let v_73616d706c654174: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64, f64, bool, f64) -> bool>> = lexical_binding();
            let v_61646a757374: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, f64) -> bool>> = lexical_binding();
            let v_73776974636841786973: LexicalBinding<Value> = lexical_binding();
            let v_636f6e74657874: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(bool) -> Value>> = lexical_binding();
            // Source "frontend/proto-ui/src/graph-point.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("x", record(vec![("type", Value::string("number")), ("default", Value::number(0.5_f64))])), ("y", record(vec![("type", Value::string("number")), ("default", Value::number(0.5_f64))])), ("minX", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("maxX", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("minY", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("maxY", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("hasX", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("hasY", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("xLabel", record(vec![("type", Value::string("string")), ("default", Value::string("横轴"))])), ("yLabel", record(vec![("type", Value::string("string")), ("default", Value::string("纵轴"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":23:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":24:11
            v_78.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("x"), Value::number(0.5_f64), record(vec![("min", Value::number(0.0_f64)), ("max", Value::number(1.0_f64))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":25:11
            v_79.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "range", Value::string("y"), Value::number(0.5_f64), record(vec![("min", Value::number(0.0_f64)), ("max", Value::number(1.0_f64))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":26:11
            v_61786973.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("axis"), Value::string("y"), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":27:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":28:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":29:11
            v_76616c75654e6f77.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "range", Value::string("valueNow"), Value::number(0.5_f64), record(vec![("min", Value::number(0.0_f64)), ("max", Value::number(1.0_f64))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":30:11
            v_76616c75654d696e.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "range", Value::string("valueMin"), Value::number(0.0_f64), record(vec![("min", Value::number(0.0_f64)), ("max", Value::number(1.0_f64))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":31:11
            v_76616c75654d6178.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "range", Value::string("valueMax"), Value::number(1.0_f64), record(vec![("min", Value::number(0.0_f64)), ("max", Value::number(1.0_f64))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":32:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":33:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":34:5
            { let v_7075695f6e61746976655f3130 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":35:5
            { let v_7075695f6e61746976655f3131 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3131.clone()).owner(), Value::string("slider")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":36:5
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3132.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":37:5
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("valueNow"), v_76616c75654e6f77.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":38:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("valueMin"), v_76616c75654d696e.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":39:5
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("valueMax"), v_76616c75654d6178.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":40:5
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":41:5
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("increment"), record(vec![("event", Value::string("host:verdant-point-increment"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":42:5
            { let v_7075695f6e61746976655f3138 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("decrement"), record(vec![("event", Value::string("host:verdant-point-decrement"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":43:5
            { let v_7075695f6e61746976655f3139 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("setValue"), record(vec![("event", Value::string("host:verdant-point-set-value"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":46:5
            { let v_7075695f6e61746976655f3230 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3230.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-point-axis"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":47:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("rounded-full".into(), "rounded".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:39
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("x"), v_78.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:65
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("y"), v_79.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":49:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("axis"), v_61786973.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":49:37
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":50:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":50:49
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":51:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("valueCommit")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":52:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":53:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("selected")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":54:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":55:5
            { let v_7075695f6e61746976655f3333 = v_646566.clone(); expose_event(&owner, Value::string("exact")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":56:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:53
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":58:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":58:24
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:28
            v_616363657074656458.initialize(Rc::new(std::cell::RefCell::new(0.5_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:49
            v_616363657074656459.initialize(Rc::new(std::cell::RefCell::new(0.5_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:9
            v_6c6f7758.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:23
            v_6869676858.initialize(Rc::new(std::cell::RefCell::new(1.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:38
            v_6c6f7759.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:52
            v_6869676859.initialize(Rc::new(std::cell::RefCell::new(1.0_f64)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:9
            v_68617358.initialize(Rc::new(std::cell::RefCell::new(true)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:26
            v_68617359.initialize(Rc::new(std::cell::RefCell::new(true)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:43
            v_626173654c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:63
            v_784c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:80
            v_794c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":64:11
            v_636c616d70.initialize({   let call:Rc<dyn Fn(f64, f64, f64)->f64>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64| -> f64 {
                let v_76: f64 = argument_0;
                let v_6c6f77: f64 = argument_1;
                let v_68696768: f64 = argument_2;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":65:7
                if { let pui_native_34=!((v_76.clone()).is_finite()); if !(pui_native_34) {{ let pui_native_36:f64=v_76.clone(); let pui_native_37:f64=v_6c6f77.clone(); pui_native_36 < pui_native_37 }} else {pui_native_34} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":65:43
                    return v_6c6f77.clone();
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":66:7
                if { let pui_native_38:f64=v_76.clone(); let pui_native_39:f64=v_68696768.clone(); pui_native_38 > pui_native_39 } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":66:21
                    return v_68696768.clone();
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":67:7
                return v_76.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":69:11
            v_70726f6a656374696f6e.initialize({  let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_61786973 = v_61786973.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_76616c75654e6f77 = v_76616c75654e6f77.clone(); let v_76616c75654d696e = v_76616c75654d696e.clone(); let v_76616c75654d6178 = v_76616c75654d6178.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_626173654c6162656c = v_626173654c6162656c.clone(); let v_784c6162656c = v_784c6162656c.clone(); let v_794c6162656c = v_794c6162656c.clone(); let call:Rc<dyn Fn(f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool| -> bool {
                let v_7469636b65745f5f39613664623137653666373731363463: LexicalBinding<f64> = lexical_binding();
                let v_617574686f72697479: LexicalBinding<bool> = lexical_binding();
                v_7469636b65745f5f39613664623137653666373731363463.initialize(argument_0);
                v_617574686f72697479.initialize(argument_1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":70:13
                let v_76616c6964 = {  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7469636b65745f5f39613664623137653666373731363463 = v_7469636b65745f5f39613664623137653666373731363463.clone(); let v_617574686f72697479 = v_617574686f72697479.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":70:36
                    return if v_617574686f72697479.read() {{ let pui_native_40={ let pui_native_42=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_42) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_42} }; if (pui_native_40) {{ let pui_native_46={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_47=v_7469636b65745f5f39613664623137653666373731363463.read(); (pui_native_46==pui_native_47) }} else {pui_native_40} }} else {{ let pui_native_48={ let pui_native_50=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_50) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_50} }; if (pui_native_48) {{ let pui_native_54={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_55=v_7469636b65745f5f39613664623137653666373731363463.read(); (pui_native_54==pui_native_55) }} else {pui_native_48} }};
                }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":71:7
                if !((v_76616c6964.call)()) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":71:21
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":72:13
                let v_697358: bool = (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3538 = v_61786973.read(); (v_7075695f6e61746976655f3538.clone()).get() }), &(Value::string("x"))))).bool();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":73:7
                { let v_7075695f6e61746976655f3539 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3539.clone(), add(add(add({ let value = (v_626173654c6162656c.read()).borrow().clone(); value }, Value::string("，")), if (Value::boolean(v_697358.clone())).truthy() { { let value = (v_784c6162656c.read()).borrow().clone(); value } } else { { let value = (v_794c6162656c.read()).borrow().clone(); value } }), Value::string("归一化值；方向键调整，空格切换坐标轴")), Value::string("reason: graph axis label")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":74:7
                if !((v_76616c6964.call)()) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":74:21
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":75:7
                { let v_7075695f6e61746976655f3630 = v_76616c75654e6f77.read(); state_set(v_7075695f6e61746976655f3630.clone(), Value::number(if v_697358.clone() {({ let v_7075695f6e61746976655f3631 = v_78.read(); Value::number(((v_7075695f6e61746976655f3631.clone()).get()).num()) }).num()} else {({ let v_7075695f6e61746976655f3632 = v_79.read(); Value::number(((v_7075695f6e61746976655f3632.clone()).get()).num()) }).num()}), Value::string("reason: graph axis value")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":76:7
                if !((v_76616c6964.call)()) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":76:21
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":77:7
                { let v_7075695f6e61746976655f3633 = v_76616c75654d696e.read(); state_set(v_7075695f6e61746976655f3633.clone(), Value::number(if v_697358.clone() {{ let value=(v_6c6f7758.read()).borrow().clone(); value }} else {{ let value=(v_6c6f7759.read()).borrow().clone(); value }}), Value::string("reason: graph axis minimum")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":78:7
                if !((v_76616c6964.call)()) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":78:21
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":79:7
                { let v_7075695f6e61746976655f3634 = v_76616c75654d6178.read(); state_set(v_7075695f6e61746976655f3634.clone(), Value::number(if v_697358.clone() {{ let value=(v_6869676858.read()).borrow().clone(); value }} else {{ let value=(v_6869676859.read()).borrow().clone(); value }}), Value::string("reason: graph axis maximum")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":80:7
                return (v_76616c6964.call)();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":82:11
            v_7075626c697368.initialize({  let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); let call:Rc<dyn Fn(f64, f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64| -> bool {
                let v_6e657874585f5f38353066356236383363623837343936: f64 = argument_0;
                let v_6e657874595f5f37396132303731326231333461663932: f64 = argument_1;
                let v_7469636b65745f5f64616337656537363366653033333330: f64 = argument_2;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":83:7
                if !({ let pui_native_65={ let pui_native_67=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_67) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_67} }; if (pui_native_65) {{ let pui_native_71={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_72=v_7469636b65745f5f64616337656537363366653033333330.clone(); (pui_native_71==pui_native_72) }} else {pui_native_65} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":83:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":84:7
                { let v_7075695f6e61746976655f3733 = v_78.read(); state_set(v_7075695f6e61746976655f3733.clone(), Value::number((v_636c616d70.read().call)(v_6e657874585f5f38353066356236383363623837343936.clone(), { let value=(v_6c6f7758.read()).borrow().clone(); value }, { let value=(v_6869676858.read()).borrow().clone(); value })), Value::string("reason: graph horizontal preview")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":85:7
                if !({ let pui_native_74={ let pui_native_76=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_76) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_76} }; if (pui_native_74) {{ let pui_native_80={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_81=v_7469636b65745f5f64616337656537363366653033333330.clone(); (pui_native_80==pui_native_81) }} else {pui_native_74} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":85:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":86:7
                { let v_7075695f6e61746976655f3832 = v_79.read(); state_set(v_7075695f6e61746976655f3832.clone(), Value::number((v_636c616d70.read().call)(v_6e657874595f5f37396132303731326231333461663932.clone(), { let value=(v_6c6f7759.read()).borrow().clone(); value }, { let value=(v_6869676859.read()).borrow().clone(); value })), Value::string("reason: graph vertical preview")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":87:7
                if !({ let pui_native_83={ let pui_native_85=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_85) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_85} }; if (pui_native_83) {{ let pui_native_89={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_90=v_7469636b65745f5f64616337656537363366653033333330.clone(); (pui_native_89==pui_native_90) }} else {pui_native_83} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":87:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":88:7
                if !((v_70726f6a656374696f6e.read().call)(v_7469636b65745f5f64616337656537363366653033333330.clone(), false)) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":88:39
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":89:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let value=(v_6174746163686564.read()).borrow().clone(); value }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":89:28
                    { let v_7075695f6e61746976655f3931 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3931.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":90:7
                return { let pui_native_92={ let pui_native_94=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_94) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_94} }; if (pui_native_92) {{ let pui_native_98={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_99=v_7469636b65745f5f64616337656537363366653033333330.clone(); (pui_native_98==pui_native_99) }} else {pui_native_92} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":92:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:13
                let v_68616e646c655f5f35316535346639386539306362633834 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:33
                let v_7469636b65745f5f64643534393136613163326361633234: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:57
                let v_6163746976655f5f63383862373135386338353065346635: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313032 = v_7068617365.read(); (v_7075695f6e61746976655f313032.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:96
                let v_62617369735f5f65333231376239656639393033383039 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":94:7
                if !((v_68616e646c655f5f35316535346639386539306362633834.clone()).truthy()) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":94:20
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":95:7
                { let v_7075695f6e61746976655f313033 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313033.clone(), Value::string("idle"), Value::string("reason: graph interaction cancelled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":96:7
                if { let pui_native_104=!({ let pui_native_106={ let pui_native_108=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_108) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_108} }; if (pui_native_106) {{ let pui_native_112={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_113=v_7469636b65745f5f64643534393136613163326361633234.clone(); (pui_native_112==pui_native_113) }} else {pui_native_106} }); if !(pui_native_104) {!((v_7075626c697368.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f64643534393136613163326361633234.clone()))} else {pui_native_104} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":96:71
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":97:7
                if v_6163746976655f5f63383862373135386338353065346635.clone() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":97:19
                    { let v_7075695f6e61746976655f313134 = v_68616e646c655f5f35316535346639386539306362633834.clone(); emit(&(v_7075695f6e61746976655f313134.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f65333231376239656639393033383039.clone())])) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":98:7
                return v_6163746976655f5f63383862373135386338353065346635.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":100:11
            v_70726f706f7365.initialize({  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn(f64, f64, bool, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool, argument_3: f64| -> bool {
                let v_6e657874585f5f35663131336333386262623937303635: f64 = argument_0;
                let v_6e657874595f5f34616631633833323036343966323235: f64 = argument_1;
                let v_72657365745f5f30326163666164373033323462616161: bool = argument_2;
                let v_7469636b65745f5f62626435373366343634663931653936: f64 = argument_3;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":101:13
                let v_68616e646c655f5f35363332633365303633366633323631 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":102:7
                if { let pui_native_115={ let pui_native_117={ let pui_native_119=!((v_68616e646c655f5f35363332633365303633366633323631.clone()).truthy()); if !(pui_native_119) {!({ let pui_native_121={ let pui_native_123=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_123) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_123} }; if (pui_native_121) {{ let pui_native_127={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_128=v_7469636b65745f5f62626435373366343634663931653936.clone(); (pui_native_127==pui_native_128) }} else {pui_native_121} })} else {pui_native_119} }; if !(pui_native_117) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_117} }; if !(pui_native_115) {({ let v_7075695f6e61746976655f313239 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313239.clone()).get()).bool()) }).bool()} else {pui_native_115} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":102:71
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:13
                let v_70726f706f73616c58: f64 = (v_636c616d70.read().call)(v_6e657874585f5f35663131336333386262623937303635.clone(), { let value=(v_6c6f7758.read()).borrow().clone(); value }, { let value=(v_6869676858.read()).borrow().clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:58
                let v_70726f706f73616c59: f64 = (v_636c616d70.read().call)(v_6e657874595f5f34616631633833323036343966323235.clone(), { let value=(v_6c6f7759.read()).borrow().clone(); value }, { let value=(v_6869676859.read()).borrow().clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:103
                let v_62617369735f5f31363533653063616435373835376630 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":104:7
                { let v_7075695f6e61746976655f313330 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313330.clone(), Value::string("idle"), Value::string("reason: graph proposal settled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":105:7
                if { let pui_native_131=!({ let pui_native_133={ let pui_native_135=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_135) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_135} }; if (pui_native_133) {{ let pui_native_139={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_140=v_7469636b65745f5f62626435373366343634663931653936.clone(); (pui_native_139==pui_native_140) }} else {pui_native_133} }); if !(pui_native_131) {!((v_7075626c697368.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f62626435373366343634663931653936.clone()))} else {pui_native_131} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":105:71
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":106:7
                { let v_7075695f6e61746976655f313431 = v_68616e646c655f5f35363332633365303633366633323631.clone(); emit(&(v_7075695f6e61746976655f313431.clone()).owner(), Value::string("valueCommit"), record(vec![("x", Value::number(v_70726f706f73616c58.clone())), ("y", Value::number(v_70726f706f73616c59.clone())), ("reset", Value::boolean(v_72657365745f5f30326163666164373033323462616161.clone())), ("revision", v_62617369735f5f31363533653063616435373835376630.clone())])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":107:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":109:5
            { let v_7075695f6e61746976655f313432 = v_646566.clone(); expose_method(&owner, Value::string("begin"), ({  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn(bool, bool)->bool>=Rc::new(move |argument_0: bool, argument_1: bool| -> bool {
                let v_72657365745f5f36633433323537306263393436313837: bool = argument_0;
                let v_646f75626c65: bool = argument_1;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":110:13
                let v_68616e646c655f5f37306631373339333162636531396530 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":111:7
                if { let pui_native_143={ let pui_native_145={ let pui_native_147={ let pui_native_149={ let pui_native_151=!((v_68616e646c655f5f37306631373339333162636531396530.clone()).truthy()); if !(pui_native_151) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_151} }; if !(pui_native_149) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_149} }; if !(pui_native_147) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_147} }; if !(pui_native_145) {({ let v_7075695f6e61746976655f313533 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313533.clone()).get()).bool()) }).bool()} else {pui_native_145} }; if !(pui_native_143) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313536 = v_7068617365.read(); (v_7075695f6e61746976655f313536.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_143} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":111:100
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":112:13
                let v_7469636b65745f5f30333331633531306434306464373066: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":113:7
                { let v_7075695f6e61746976655f313537 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313537.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":114:7
                if { let pui_native_158={ let pui_native_160=!({ let pui_native_162={ let pui_native_164=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_164) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_164} }; if (pui_native_162) {{ let pui_native_168={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_169=v_7469636b65745f5f30333331633531306434306464373066.clone(); (pui_native_168==pui_native_169) }} else {pui_native_162} }); if !(pui_native_160) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_160} }; if !(pui_native_158) {({ let v_7075695f6e61746976655f313730 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313730.clone()).get()).bool()) }).bool()} else {pui_native_158} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":114:60
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":115:7
                { let v_7075695f6e61746976655f313731 = v_68616e646c655f5f37306631373339333162636531396530.clone(); emit(&(v_7075695f6e61746976655f313731.clone()).owner(), Value::string("selected"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":116:7
                if !({ let pui_native_172={ let pui_native_174=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_174) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_174} }; if (pui_native_172) {{ let pui_native_178={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_179=v_7469636b65745f5f30333331633531306434306464373066.clone(); (pui_native_178==pui_native_179) }} else {pui_native_172} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":116:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":117:7
                if { let pui_native_180=v_72657365745f5f36633433323537306263393436313837.clone(); if !(pui_native_180) {v_646f75626c65.clone()} else {pui_native_180} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":117:28
                    return (v_70726f706f7365.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, true, v_7469636b65745f5f30333331633531306434306464373066.clone());
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":118:7
                { let v_7075695f6e61746976655f313832 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313832.clone(), Value::string("pressed"), Value::string("reason: graph interaction acquired")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":119:7
                return (v_7075626c697368.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f30333331633531306434306464373066.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).bool(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":121:11
            v_73616d706c654174.initialize({  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: f64| -> bool {
                let v_64785f5f61643062663761373730396630373164: f64 = argument_0;
                let v_64795f5f62316539373130383130306661343733: f64 = argument_1;
                let v_74726176656c585f5f36633564306563356265393734653166: f64 = argument_2;
                let v_74726176656c595f5f31656638393139313732386633366330: f64 = argument_3;
                let v_66696e655f5f66653631613432663939663866323034: bool = argument_4;
                let v_7469636b65745f5f65333335666338323038326165343565: f64 = argument_5;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":122:7
                if { let pui_native_183={ let pui_native_185={ let pui_native_187={ let pui_native_189={ let pui_native_191=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_191) {!({ let pui_native_193={ let pui_native_195=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_195) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_195} }; if (pui_native_193) {{ let pui_native_199={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_200=v_7469636b65745f5f65333335666338323038326165343565.clone(); (pui_native_199==pui_native_200) }} else {pui_native_193} })} else {pui_native_191} }; if !(pui_native_189) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_189} }; if !(pui_native_187) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_187} }; if !(pui_native_185) {({ let v_7075695f6e61746976655f323031 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323031.clone()).get()).bool()) }).bool()} else {pui_native_185} }; if !(pui_native_183) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f323034 = v_7068617365.read(); (v_7075695f6e61746976655f323034.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_183} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":122:105
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":123:7
                if { let pui_native_205={ let pui_native_207={ let pui_native_209={ let pui_native_211={ let pui_native_213=!((v_64785f5f61643062663761373730396630373164.clone()).is_finite()); if !(pui_native_213) {!((v_64795f5f62316539373130383130306661343733.clone()).is_finite())} else {pui_native_213} }; if !(pui_native_211) {!((v_74726176656c585f5f36633564306563356265393734653166.clone()).is_finite())} else {pui_native_211} }; if !(pui_native_209) {!((v_74726176656c595f5f31656638393139313732386633366330.clone()).is_finite())} else {pui_native_209} }; if !(pui_native_207) {{ let pui_native_215:f64=v_74726176656c585f5f36633564306563356265393734653166.clone(); let pui_native_216:f64=0.0_f64; pui_native_215 <= pui_native_216 }} else {pui_native_207} }; if !(pui_native_205) {{ let pui_native_217:f64=v_74726176656c595f5f31656638393139313732386633366330.clone(); let pui_native_218:f64=0.0_f64; pui_native_217 <= pui_native_218 }} else {pui_native_205} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":124:44
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":124:54
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":125:7
                if { let pui_native_219=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f323233 = v_7068617365.read(); (v_7075695f6e61746976655f323233.clone()).get() }), &(Value::string("pressed"))))).bool(); if (pui_native_219) {{ let pui_native_224:f64={ let pui_native_226:f64={ let pui_native_228:f64=v_64785f5f61643062663761373730396630373164.clone(); let pui_native_229:f64=v_64785f5f61643062663761373730396630373164.clone(); pui_native_228 * pui_native_229 }; let pui_native_227:f64={ let pui_native_230:f64=v_64795f5f62316539373130383130306661343733.clone(); let pui_native_231:f64=v_64795f5f62316539373130383130306661343733.clone(); pui_native_230 * pui_native_231 }; pui_native_226 + pui_native_227 }; let pui_native_225:f64=9.0_f64; pui_native_224 < pui_native_225 }} else {pui_native_219} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":125:63
                    return true;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":126:7
                { let v_7075695f6e61746976655f323332 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323332.clone(), Value::string("dragging"), Value::string("reason: graph movement admitted")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":127:7
                if !({ let pui_native_233={ let pui_native_235=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_235) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_235} }; if (pui_native_233) {{ let pui_native_239={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_240=v_7469636b65745f5f65333335666338323038326165343565.clone(); (pui_native_239==pui_native_240) }} else {pui_native_233} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":127:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":128:13
                let v_7363616c65: f64 = if v_66696e655f5f66653631613432663939663866323034.clone() {0.1_f64} else {1.0_f64};
                // Source "frontend/proto-ui/src/graph-point.proto.ts":129:7
                return (v_7075626c697368.read().call)(if { let value=(v_68617358.read()).borrow().clone(); value } {{ let pui_native_241:f64={ let value=(v_616363657074656458.read()).borrow().clone(); value }; let pui_native_242:f64={ let pui_native_243:f64={ let pui_native_245:f64=v_64785f5f61643062663761373730396630373164.clone(); let pui_native_246:f64=v_74726176656c585f5f36633564306563356265393734653166.clone(); pui_native_245 / pui_native_246 }; let pui_native_244:f64=v_7363616c65.clone(); pui_native_243 * pui_native_244 }; pui_native_241 + pui_native_242 }} else {{ let value=(v_616363657074656458.read()).borrow().clone(); value }}, if { let value=(v_68617359.read()).borrow().clone(); value } {{ let pui_native_247:f64={ let value=(v_616363657074656459.read()).borrow().clone(); value }; let pui_native_248:f64={ let pui_native_249:f64={ let pui_native_251:f64=v_64795f5f62316539373130383130306661343733.clone(); let pui_native_252:f64=v_74726176656c595f5f31656638393139313732386633366330.clone(); pui_native_251 / pui_native_252 }; let pui_native_250:f64=v_7363616c65.clone(); pui_native_249 * pui_native_250 }; pui_native_247 - pui_native_248 }} else {{ let value=(v_616363657074656459.read()).borrow().clone(); value }}, v_7469636b65745f5f65333335666338323038326165343565.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":132:5
            { let v_7075695f6e61746976655f323533 = v_646566.clone(); expose_method(&owner, Value::string("sample"), ({  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_64785f5f31626565333434643663636134383465: f64 = argument_0;
                let v_64795f5f35353933383361636434396233636566: f64 = argument_1;
                let v_74726176656c585f5f30393535343037306537373834386364: f64 = argument_2;
                let v_74726176656c595f5f62343234393066646666386164646332: f64 = argument_3;
                let v_66696e655f5f33663937616266303832323738393861: bool = argument_4;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":133:7
                return (v_73616d706c654174.read().call)(v_64785f5f31626565333434643663636134383465.clone(), v_64795f5f35353933383361636434396233636566.clone(), v_74726176656c585f5f30393535343037306537373834386364.clone(), v_74726176656c595f5f62343234393066646666386164646332.clone(), v_66696e655f5f33663937616266303832323738393861.clone(), { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":135:5
            { let v_7075695f6e61746976655f323534 = v_646566.clone(); expose_method(&owner, Value::string("finish"), ({  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_64785f5f33303864303661313062303232353432: f64 = argument_0;
                let v_64795f5f63313866303337356134653163633863: f64 = argument_1;
                let v_74726176656c585f5f65343262616338663139333330666462: f64 = argument_2;
                let v_74726176656c595f5f31323664333035333237326634383364: f64 = argument_3;
                let v_66696e655f5f34636138626561666430346133626162: bool = argument_4;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":136:13
                let v_7469636b65745f5f33663766353463383461646332313334: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":137:7
                if { let pui_native_255=!((v_73616d706c654174.read().call)(v_64785f5f33303864303661313062303232353432.clone(), v_64795f5f63313866303337356134653163633863.clone(), v_74726176656c585f5f65343262616338663139333330666462.clone(), v_74726176656c595f5f31323664333035333237326634383364.clone(), v_66696e655f5f34636138626561666430346133626162.clone(), v_7469636b65745f5f33663766353463383461646332313334.clone())); if !(pui_native_255) {!({ let pui_native_257={ let pui_native_259=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_259) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_259} }; if (pui_native_257) {{ let pui_native_263={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_264=v_7469636b65745f5f33663766353463383461646332313334.clone(); (pui_native_263==pui_native_264) }} else {pui_native_257} })} else {pui_native_255} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":137:82
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":138:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f323637 = v_7068617365.read(); (v_7075695f6e61746976655f323637.clone()).get() }), &(Value::string("dragging"))))).bool() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":138:39
                    return (v_70726f706f7365.read().call)(({ let v_7075695f6e61746976655f323638 = v_78.read(); Value::number(((v_7075695f6e61746976655f323638.clone()).get()).num()) }).num(), ({ let v_7075695f6e61746976655f323639 = v_79.read(); Value::number(((v_7075695f6e61746976655f323639.clone()).get()).num()) }).num(), false, v_7469636b65745f5f33663766353463383461646332313334.clone());
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":139:7
                { let v_7075695f6e61746976655f323730 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323730.clone(), Value::string("idle"), Value::string("reason: graph click settled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":140:7
                return (v_7075626c697368.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f33663766353463383461646332313334.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":142:5
            { let v_7075695f6e61746976655f323731 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":143:11
            v_61646a757374.initialize({  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_7768696368: Value = argument_0;
                let v_616d6f756e745f5f36656230343132633832303666666530: f64 = argument_1;
                let v_656e64706f696e74: f64 = argument_2;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":144:7
                if { let pui_native_272={ let pui_native_274={ let pui_native_276={ let pui_native_278={ let pui_native_280=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_280) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_280} }; if !(pui_native_278) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_278} }; if !(pui_native_276) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_276} }; if !(pui_native_274) {({ let v_7075695f6e61746976655f323832 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323832.clone()).get()).bool()) }).bool()} else {pui_native_274} }; if !(pui_native_272) {if (Value::boolean(strict_equal(&(v_7768696368.clone()), &(Value::string("x"))))).bool() {!({ let value=(v_68617358.read()).borrow().clone(); value })} else {!({ let value=(v_68617359.read()).borrow().clone(); value })}} else {pui_native_272} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":144:106
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":145:13
                let v_7469636b65745f5f30666561623462613635346537656164: f64 = { let pui_native_285:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_286:f64=1.0_f64; pui_native_285 + pui_native_286 };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":146:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/graph-point.proto.ts":147:7
                if !({ let pui_native_287={ let pui_native_289=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_289) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_289} }; if (pui_native_287) {{ let pui_native_293={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_294=v_7469636b65745f5f30666561623462613635346537656164.clone(); (pui_native_293==pui_native_294) }} else {pui_native_287} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":147:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":148:7
                { let v_7075695f6e61746976655f323935 = v_61786973.read(); state_set(v_7075695f6e61746976655f323935.clone(), v_7768696368.clone(), Value::string("reason: graph keyboard axis")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":149:7
                if !({ let pui_native_296={ let pui_native_298=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_298) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_298} }; if (pui_native_296) {{ let pui_native_302={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_303=v_7469636b65745f5f30666561623462613635346537656164.clone(); (pui_native_302==pui_native_303) }} else {pui_native_296} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":149:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":150:11
                let v_6e657874585f5f34393034393834616436313863653131: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new({ let value=(v_616363657074656458.read()).borrow().clone(); value }));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":150:34
                let v_6e657874595f5f31643537333562343439386133333261: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new({ let value=(v_616363657074656459.read()).borrow().clone(); value }));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":151:7
                if (Value::boolean(strict_equal(&(v_7768696368.clone()), &(Value::string("x"))))).bool() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":151:26
                    Value::number({ let value=if { let pui_native_306:f64=v_656e64706f696e74.clone(); let pui_native_307:f64=0.0_f64; pui_native_306 < pui_native_307 } {{ let value=(v_6c6f7758.read()).borrow().clone(); value }} else {if { let pui_native_308:f64=v_656e64706f696e74.clone(); let pui_native_309:f64=0.0_f64; pui_native_308 > pui_native_309 } {{ let value=(v_6869676858.read()).borrow().clone(); value }} else {{ let pui_native_310:f64={ let value=(v_616363657074656458.read()).borrow().clone(); value }; let pui_native_311:f64=v_616d6f756e745f5f36656230343132633832303666666530.clone(); pui_native_310 + pui_native_311 }}}; *(v_6e657874585f5f34393034393834616436313863653131.clone()).borrow_mut()=value.clone(); value });
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":152:12
                    Value::number({ let value=if { let pui_native_312:f64=v_656e64706f696e74.clone(); let pui_native_313:f64=0.0_f64; pui_native_312 < pui_native_313 } {{ let value=(v_6c6f7759.read()).borrow().clone(); value }} else {if { let pui_native_314:f64=v_656e64706f696e74.clone(); let pui_native_315:f64=0.0_f64; pui_native_314 > pui_native_315 } {{ let value=(v_6869676859.read()).borrow().clone(); value }} else {{ let pui_native_316:f64={ let value=(v_616363657074656459.read()).borrow().clone(); value }; let pui_native_317:f64=v_616d6f756e745f5f36656230343132633832303666666530.clone(); pui_native_316 + pui_native_317 }}}; *(v_6e657874595f5f31643537333562343439386133333261.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":153:7
                return (v_70726f706f7365.read().call)({ let value=(v_6e657874585f5f34393034393834616436313863653131.clone()).borrow().clone(); value }, { let value=(v_6e657874595f5f31643537333562343439386133333261.clone()).borrow().clone(); value }, false, v_7469636b65745f5f30666561623462613635346537656164.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":155:11
            v_73776974636841786973.initialize({  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":156:7
                if { let pui_native_318={ let pui_native_320={ let pui_native_322={ let pui_native_324={ let pui_native_326={ let pui_native_328=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_328) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_328} }; if !(pui_native_326) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_326} }; if !(pui_native_324) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_324} }; if !(pui_native_322) {({ let v_7075695f6e61746976655f333330 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333330.clone()).get()).bool()) }).bool()} else {pui_native_322} }; if !(pui_native_320) {!({ let value=(v_68617358.read()).borrow().clone(); value })} else {pui_native_320} }; if !(pui_native_318) {!({ let value=(v_68617359.read()).borrow().clone(); value })} else {pui_native_318} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":156:89
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":157:13
                let v_7469636b65745f5f33386563616130383536636330386434: f64 = { let pui_native_331:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_332:f64=1.0_f64; pui_native_331 + pui_native_332 };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":158:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/graph-point.proto.ts":159:7
                if !({ let pui_native_333={ let pui_native_335=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_335) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_335} }; if (pui_native_333) {{ let pui_native_339={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_340=v_7469636b65745f5f33386563616130383536636330386434.clone(); (pui_native_339==pui_native_340) }} else {pui_native_333} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":159:29
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":160:7
                { let v_7075695f6e61746976655f333431 = v_61786973.read(); state_set(v_7075695f6e61746976655f333431.clone(), if (Value::boolean((Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333434 = v_61786973.read(); (v_7075695f6e61746976655f333434.clone()).get() }), &(Value::string("x"))))).bool())).truthy() { Value::string("y") } else { Value::string("x") }, Value::string("reason: graph assistive axis")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":161:7
                if !((v_70726f6a656374696f6e.read().call)(v_7469636b65745f5f33386563616130383536636330386434.clone(), false)) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":161:39
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":162:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":162:16
                    { let v_7075695f6e61746976655f333435 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f333435.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":164:11
            v_636f6e74657874.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":165:13
                let v_68616e646c655f5f62383562316436633662666266386533 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":166:7
                if { let pui_native_346={ let pui_native_348={ let pui_native_350={ let pui_native_352=!((v_68616e646c655f5f62383562316436633662666266386533.clone()).truthy()); if !(pui_native_352) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_352} }; if !(pui_native_350) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_350} }; if !(pui_native_348) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_348} }; if !(pui_native_346) {({ let v_7075695f6e61746976655f333534 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333534.clone()).get()).bool()) }).bool()} else {pui_native_346} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":166:74
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":167:13
                let v_7469636b65745f5f31326534323735393738663635386431: f64 = { let pui_native_355:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_356:f64=1.0_f64; pui_native_355 + pui_native_356 };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":167:33
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/graph-point.proto.ts":168:7
                if !({ let pui_native_357={ let pui_native_359=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_359) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_359} }; if (pui_native_357) {{ let pui_native_363={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_364=v_7469636b65745f5f31326534323735393738663635386431.clone(); (pui_native_363==pui_native_364) }} else {pui_native_357} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":168:29
                    return false;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":169:7
                { let v_7075695f6e61746976655f333635 = v_68616e646c655f5f62383562316436633662666266386533.clone(); emit(&(v_7075695f6e61746976655f333635.clone()).owner(), Value::string("context"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":170:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":172:5
            { let v_7075695f6e61746976655f333636 = v_646566.clone(); expose_method(&owner, Value::string("openContext"), (v_636f6e74657874.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":173:5
            { let v_7075695f6e61746976655f333637 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-axis"), false, v_73776974636841786973.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":174:5
            { let v_7075695f6e61746976655f333638 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-increment"), false, ({  let v_61786973 = v_61786973.clone(); let v_61646a757374 = v_61646a757374.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":174:56
                return (v_61646a757374.read().call)({ let v_7075695f6e61746976655f333639 = v_61786973.read(); (v_7075695f6e61746976655f333639.clone()).get() }, 0.01_f64, 0.0_f64);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":175:5
            { let v_7075695f6e61746976655f333730 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-decrement"), false, ({  let v_61786973 = v_61786973.clone(); let v_61646a757374 = v_61646a757374.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":175:56
                return (v_61646a757374.read().call)({ let v_7075695f6e61746976655f333731 = v_61786973.read(); (v_7075695f6e61746976655f333731.clone()).get() }, -(0.01_f64), 0.0_f64);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":176:5
            { let v_7075695f6e61746976655f333732 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-point-set-value"), false, {  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f68616e646c655f5f39326665653738363662653264303032: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":177:13
                let v_6576656e745f5f39353038373361366333343161373536 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":177:63
                let v_6e657874: f64 = ((v_6576656e745f5f39353038373361366333343161373536.clone()).member("value", false)).num();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":178:7
                if { let pui_native_373={ let pui_native_375={ let pui_native_377={ let pui_native_379={ let pui_native_381=(Value::boolean(!strict_equal(&(typeof_value(&(Value::number(v_6e657874.clone())))), &(Value::string("number"))))).bool(); if !(pui_native_381) {!((v_6e657874.clone()).is_finite())} else {pui_native_381} }; if !(pui_native_379) {!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy())} else {pui_native_379} }; if !(pui_native_377) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_377} }; if !(pui_native_375) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_375} }; if !(pui_native_373) {({ let v_7075695f6e61746976655f333835 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333835.clone()).get()).bool()) }).bool()} else {pui_native_373} } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":178:113
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":179:7
                Value::boolean((v_61646a757374.read().call)({ let v_7075695f6e61746976655f333836 = v_61786973.read(); (v_7075695f6e61746976655f333836.clone()).get() }, { let pui_native_387:f64=v_6e657874.clone(); let pui_native_388:f64=if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333931 = v_61786973.read(); (v_7075695f6e61746976655f333931.clone()).get() }), &(Value::string("x"))))).bool() {{ let value=(v_616363657074656458.read()).borrow().clone(); value }} else {{ let value=(v_616363657074656459.read()).borrow().clone(); value }}; pui_native_387 - pui_native_388 }, 0.0_f64));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":181:5
            { let v_7075695f6e61746976655f333932 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_61646a757374 = v_61646a757374.clone(); let v_73776974636841786973 = v_73776974636841786973.clone(); let v_636f6e74657874 = v_636f6e74657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f68616e646c655f5f61353161313432366533313466393632: Value = arg(&arguments, 0);
                let v_6576656e745f5f32396130326466303338363232373465: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":182:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Escape"))))).bool() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:37
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:47
                    { let v_7075695f6e61746976655f333935 = v_6576656e745f5f32396130326466303338363232373465.clone(); prevent(v_7075695f6e61746976655f333935.clone()) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:95
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":183:7
                if ({ let left = { let left = { let left = Value::boolean({ let pui_native_396={ let pui_native_398={ let pui_native_400={ let pui_native_402=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_402) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_402} }; if !(pui_native_400) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_400} }; if !(pui_native_398) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_398} }; if !(pui_native_396) {({ let v_7075695f6e61746976655f343034 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343034.clone()).get()).bool()) }).bool()} else {pui_native_396} }); if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("ctrlKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("metaKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("altKey", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":183:121
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":184:13
                let v_616d6f756e745f5f39613631363733393039313330373161: f64 = if ((v_6576656e745f5f32396130326466303338363232373465.clone()).member("shiftKey", false)).truthy() {0.001_f64} else {0.01_f64};
                // Source "frontend/proto-ui/src/graph-point.proto.ts":185:11
                let v_6163636570746564: Rc<std::cell::RefCell<bool>> = Rc::new(std::cell::RefCell::new(false));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":186:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowLeft"))))).bool() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":186:38
                    Value::boolean({ let value=(v_61646a757374.read().call)(if (Value::boolean({ let value=(v_68617358.read()).borrow().clone(); value })).truthy() { Value::string("x") } else { Value::string("y") }, -(v_616d6f756e745f5f39613631363733393039313330373161.clone()), 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":187:12
                    if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowRight"))))).bool() {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":187:44
                        Value::boolean({ let value=(v_61646a757374.read().call)(if (Value::boolean({ let value=(v_68617358.read()).borrow().clone(); value })).truthy() { Value::string("x") } else { Value::string("y") }, v_616d6f756e745f5f39613631363733393039313330373161.clone(), 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                    } else {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":188:12
                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowUp"))))).bool() {
                            // Source "frontend/proto-ui/src/graph-point.proto.ts":188:41
                            Value::boolean({ let value=(v_61646a757374.read().call)(if (Value::boolean({ let value=(v_68617359.read()).borrow().clone(); value })).truthy() { Value::string("y") } else { Value::string("x") }, v_616d6f756e745f5f39613631363733393039313330373161.clone(), 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                        } else {
                            // Source "frontend/proto-ui/src/graph-point.proto.ts":189:12
                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowDown"))))).bool() {
                                // Source "frontend/proto-ui/src/graph-point.proto.ts":189:43
                                Value::boolean({ let value=(v_61646a757374.read().call)(if (Value::boolean({ let value=(v_68617359.read()).borrow().clone(); value })).truthy() { Value::string("y") } else { Value::string("x") }, -(v_616d6f756e745f5f39613631363733393039313330373161.clone()), 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                            } else {
                                // Source "frontend/proto-ui/src/graph-point.proto.ts":190:12
                                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Home"))))).bool() {
                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":190:38
                                    Value::boolean({ let value=(v_61646a757374.read().call)({ let v_7075695f6e61746976655f343135 = v_61786973.read(); (v_7075695f6e61746976655f343135.clone()).get() }, 0.0_f64, -(1.0_f64)); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                } else {
                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":191:12
                                    if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("End"))))).bool() {
                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":191:37
                                        Value::boolean({ let value=(v_61646a757374.read().call)({ let v_7075695f6e61746976655f343138 = v_61786973.read(); (v_7075695f6e61746976655f343138.clone()).get() }, 0.0_f64, 1.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                    } else {
                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":192:12
                                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("PageUp"))))).bool() {
                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":192:40
                                            Value::boolean({ let value=(v_61646a757374.read().call)({ let v_7075695f6e61746976655f343231 = v_61786973.read(); (v_7075695f6e61746976655f343231.clone()).get() }, { let pui_native_422:f64=v_616d6f756e745f5f39613631363733393039313330373161.clone(); let pui_native_423:f64=10.0_f64; pui_native_422 * pui_native_423 }, 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                        } else {
                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":193:12
                                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("PageDown"))))).bool() {
                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":193:42
                                                Value::boolean({ let value=(v_61646a757374.read().call)({ let v_7075695f6e61746976655f343236 = v_61786973.read(); (v_7075695f6e61746976655f343236.clone()).get() }, { let pui_native_427:f64=-(v_616d6f756e745f5f39613631363733393039313330373161.clone()); let pui_native_428:f64=10.0_f64; pui_native_427 * pui_native_428 }, 0.0_f64); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                            } else {
                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":194:12
                                                if { let pui_native_429=(Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string(" "))))).bool(); if !(pui_native_429) {(Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Space"))))).bool()} else {pui_native_429} } {
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":194:62
                                                    v_73776974636841786973.read().call_authored(vec![]);
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":194:76
                                                    Value::boolean({ let value=true; *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                                } else {
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":195:12
                                                    if ({ let left = Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ContextMenu"))))).bool()); if !left.truthy() { { let left = Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("F10"))))).bool()); if left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("shiftKey", false) } else { left } } } else { left } }).truthy() {
                                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":195:88
                                                        Value::boolean({ let value=(v_636f6e74657874.read().call)(); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                                    } else {
                                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":196:12
                                                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Backspace"))))).bool() {
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:51
                                                            let v_7469636b65745f5f33393030366661303466663761336330: f64 = { let pui_native_441:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_442:f64=1.0_f64; pui_native_441 + pui_native_442 };
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:71
                                                            Value::boolean((v_63616e63656c.read().call)());
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:81
                                                            Value::boolean({ let value=(v_70726f706f7365.read().call)({ let value=(v_616363657074656458.read()).borrow().clone(); value }, { let value=(v_616363657074656459.read()).borrow().clone(); value }, true, v_7469636b65745f5f33393030366661303466663761336330.clone()); *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                                        } else {
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":197:12
                                                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Enter"))))).bool() {
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:47
                                                                let v_68616e646c655f5f32376631623165366561616538366165 = { let value = (v_72756e.read()).borrow().clone(); value };
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:67
                                                                let v_7469636b65745f5f34313665366265366431623635396536: f64 = { let pui_native_445:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_446:f64=1.0_f64; pui_native_445 + pui_native_446 };
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:87
                                                                Value::boolean((v_63616e63656c.read().call)());
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:97
                                                                if { let pui_native_447=v_68616e646c655f5f32376631623165366561616538366165.clone(); if pui_native_447.truthy() {{ let pui_native_449={ let pui_native_451=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_451) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_451} }; if (pui_native_449) {{ let pui_native_455={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_456=v_7469636b65745f5f34313665366265366431623635396536.clone(); (pui_native_455==pui_native_456) }} else {pui_native_449} }} else {(pui_native_447).bool()} } {
                                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":197:130
                                                                    { let v_7075695f6e61746976655f343537 = v_68616e646c655f5f32376631623165366561616538366165.clone(); emit(&(v_7075695f6e61746976655f343537.clone()).owner(), Value::string("exact"), record(vec![("axis", { let v_7075695f6e61746976655f343538 = v_61786973.read(); (v_7075695f6e61746976655f343538.clone()).get() }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":197:191
                                                                    Value::boolean({ let value=true; *(v_6163636570746564.clone()).borrow_mut()=value.clone(); value });
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":198:7
                if { let value=(v_6163636570746564.clone()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":198:21
                    { let v_7075695f6e61746976655f343539 = v_6576656e745f5f32396130326466303338363232373465.clone(); prevent(v_7075695f6e61746976655f343539.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":200:5
            { let v_7075695f6e61746976655f343630 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":201:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_626173654c6162656c = v_626173654c6162656c.clone(); let v_784c6162656c = v_784c6162656c.clone(); let v_794c6162656c = v_794c6162656c.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f61323336653831363963633263646137: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":202:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":202:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:22
                let v_7469636b65745f5f34383863353335333164346263333537: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:49
                Value::boolean({ let value=true; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:65
                { let value = v_68616e646c655f5f61323336653831363963633263646137.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":204:13
                let v_70726f7073 = { let v_7075695f6e61746976655f343631 = v_68616e646c655f5f61323336653831363963633263646137.clone(); read_props(&(v_7075695f6e61746976655f343631.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":204:47
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":205:13
                let v_70726576696f75735265766973696f6e = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":206:13
                let v_6e657874585f5f61616138646166333036303661623235: f64 = (v_636c616d70.read().call)(((v_70726f7073.clone()).member("x", false)).num(), ((v_70726f7073.clone()).member("minX", false)).num(), ((v_70726f7073.clone()).member("maxX", false)).num());
                // Source "frontend/proto-ui/src/graph-point.proto.ts":207:13
                let v_6e657874595f5f38623130646238336532383562363332: f64 = (v_636c616d70.read().call)(((v_70726f7073.clone()).member("y", false)).num(), ((v_70726f7073.clone()).member("minY", false)).num(), ((v_70726f7073.clone()).member("maxY", false)).num());
                // Source "frontend/proto-ui/src/graph-point.proto.ts":208:13
                let v_73686f756c6443616e63656c: bool = { let pui_native_474=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f343738 = v_7068617365.read(); (v_7075695f6e61746976655f343738.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_474) {{ let pui_native_479={ let pui_native_481={ let pui_native_483=(Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool(); if !(pui_native_483) {{ let pui_native_487=v_6e657874585f5f61616138646166333036303661623235.clone(); let pui_native_488={ let value=(v_616363657074656458.read()).borrow().clone(); value }; !(pui_native_487==pui_native_488) }} else {pui_native_483} }; if !(pui_native_481) {{ let pui_native_489=v_6e657874595f5f38623130646238336532383562363332.clone(); let pui_native_490={ let value=(v_616363657074656459.read()).borrow().clone(); value }; !(pui_native_489==pui_native_490) }} else {pui_native_481} }; if !(pui_native_479) {{ let pui_native_491=((v_70726f7073.clone()).member("disabled", false)).bool(); let pui_native_492=true; (pui_native_491==pui_native_492) }} else {pui_native_479} }} else {pui_native_474} };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:32
                Value::number({ let value=v_6e657874585f5f61616138646166333036303661623235.clone(); *(v_616363657074656458.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:51
                Value::number({ let value=v_6e657874595f5f38623130646238336532383562363332.clone(); *(v_616363657074656459.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:7
                Value::number({ let value=((v_70726f7073.clone()).member("minX", false)).num(); *(v_6c6f7758.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:31
                Value::number({ let value=((v_70726f7073.clone()).member("maxX", false)).num(); *(v_6869676858.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:56
                Value::number({ let value=((v_70726f7073.clone()).member("minY", false)).num(); *(v_6c6f7759.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:80
                Value::number({ let value=((v_70726f7073.clone()).member("maxY", false)).num(); *(v_6869676859.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":211:7
                Value::boolean({ let value=((v_70726f7073.clone()).member("hasX", false)).bool(); *(v_68617358.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":211:34
                Value::boolean({ let value=((v_70726f7073.clone()).member("hasY", false)).bool(); *(v_68617359.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":212:7
                { let value = { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }; *(v_626173654c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":212:38
                { let value = { let left = (v_70726f7073.clone()).member("xLabel", false); if left.nullish() { Value::string("横轴") } else { left } }; *(v_784c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":212:69
                { let value = { let left = (v_70726f7073.clone()).member("yLabel", false); if left.nullish() { Value::string("纵轴") } else { left } }; *(v_794c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":213:7
                { let v_7075695f6e61746976655f353035 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f353035.clone(), Value::boolean(((v_70726f7073.clone()).member("disabled", false)).bool()), Value::string("reason: graph policy")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":214:7
                if !({ let pui_native_508={ let pui_native_510=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_510) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_510} }; if (pui_native_508) {{ let pui_native_514={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_515=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_514==pui_native_515) }} else {pui_native_508} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":214:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":215:7
                { let v_7075695f6e61746976655f353136 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f353136.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f353137 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f353137.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":216:7
                if !({ let pui_native_518={ let pui_native_520=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_520) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_520} }; if (pui_native_518) {{ let pui_native_524={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_525=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_524==pui_native_525) }} else {pui_native_518} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":216:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":217:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":217:25
                    { let v_7075695f6e61746976655f353236 = v_7068617365.read(); state_set(v_7075695f6e61746976655f353236.clone(), Value::string("idle"), Value::string("reason: graph authority changed")) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":218:7
                if !({ let pui_native_527={ let pui_native_529=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_529) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_529} }; if (pui_native_527) {{ let pui_native_533={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_534=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_533==pui_native_534) }} else {pui_native_527} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":218:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":219:7
                if !({ let value=(v_68617359.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":219:18
                    { let v_7075695f6e61746976655f353335 = v_61786973.read(); state_set(v_7075695f6e61746976655f353335.clone(), Value::string("x"), Value::string("reason: graph available axis")) };
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":220:12
                    if !({ let value=(v_68617358.read()).borrow().clone(); value }) {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":220:23
                        { let v_7075695f6e61746976655f353336 = v_61786973.read(); state_set(v_7075695f6e61746976655f353336.clone(), Value::string("y"), Value::string("reason: graph available axis")) };
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":221:7
                if !({ let pui_native_537={ let pui_native_539=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_539) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_539} }; if (pui_native_537) {{ let pui_native_543={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_544=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_543==pui_native_544) }} else {pui_native_537} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":221:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":222:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f353437 = v_7068617365.read(); (v_7075695f6e61746976655f353437.clone()).get() }), &(Value::string("idle"))))).bool() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":223:9
                    { let v_7075695f6e61746976655f353438 = v_78.read(); state_set(v_7075695f6e61746976655f353438.clone(), Value::number({ let value=(v_616363657074656458.read()).borrow().clone(); value }), Value::string("reason: graph horizontal authority")) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":224:9
                    if !({ let pui_native_549={ let pui_native_551=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_551) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_551} }; if (pui_native_549) {{ let pui_native_555={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_556=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_555==pui_native_556) }} else {pui_native_549} }) {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":224:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":225:9
                    { let v_7075695f6e61746976655f353537 = v_79.read(); state_set(v_7075695f6e61746976655f353537.clone(), Value::number({ let value=(v_616363657074656459.read()).borrow().clone(); value }), Value::string("reason: graph vertical authority")) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":226:9
                    if !({ let pui_native_558={ let pui_native_560=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_560) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_560} }; if (pui_native_558) {{ let pui_native_564={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_565=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_564==pui_native_565) }} else {pui_native_558} }) {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":226:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":228:7
                if !((v_70726f6a656374696f6e.read().call)(v_7469636b65745f5f34383863353335333164346263333537.clone(), true)) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":228:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":229:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":229:25
                    { let v_7075695f6e61746976655f353636 = v_68616e646c655f5f61323336653831363963633263646137.clone(); emit(&(v_7075695f6e61746976655f353636.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_70726576696f75735265766973696f6e.clone())])) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":230:7
                if !({ let pui_native_567={ let pui_native_569=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_569) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_569} }; if (pui_native_567) {{ let pui_native_573={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_574=v_7469636b65745f5f34383863353335333164346263333537.clone(); (pui_native_573==pui_native_574) }} else {pui_native_567} }) {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":230:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":231:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":231:24
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":231:38
                    { let v_7075695f6e61746976655f353735 = v_68616e646c655f5f61323336653831363963633263646137.clone(); request_update(&(v_7075695f6e61746976655f353735.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":233:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let call:Rc<dyn Fn(bool)->Value>=Rc::new(move |argument_0: bool| -> Value {
                let v_7065726d616e656e74: bool = argument_0;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:13
                let v_68616e646c655f5f63643761363561653065386437613762 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:33
                let v_6163746976655f5f37346336666162376666663231363966: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f353738 = v_7068617365.read(); (v_7075695f6e61746976655f353738.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:72
                let v_62617369735f5f37656438323135666166313266383034 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:22
                let v_7469636b65745f5f30613937666130623633303565653466: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:49
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:67
                Value::boolean({ let value={ let pui_native_579={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_579) {v_7065726d616e656e74.clone()} else {pui_native_579} }; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:101
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:113
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":236:7
                if v_7065726d616e656e74.clone() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":236:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":237:7
                { let v_7075695f6e61746976655f353831 = v_7068617365.read(); state_set(v_7075695f6e61746976655f353831.clone(), Value::string("idle"), Value::string("reason: graph point detached")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":238:7
                if { let pui_native_582={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_583=v_7469636b65745f5f30613937666130623633303565653466.clone(); !(pui_native_582==pui_native_583) } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":238:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":239:7
                { let v_7075695f6e61746976655f353834 = v_78.read(); state_set(v_7075695f6e61746976655f353834.clone(), Value::number({ let value=(v_616363657074656458.read()).borrow().clone(); value }), Value::string("reason: graph point detached horizontal")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":240:7
                if { let pui_native_585={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_586=v_7469636b65745f5f30613937666130623633303565653466.clone(); !(pui_native_585==pui_native_586) } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":240:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":241:7
                { let v_7075695f6e61746976655f353837 = v_79.read(); state_set(v_7075695f6e61746976655f353837.clone(), Value::number({ let value=(v_616363657074656459.read()).borrow().clone(); value }), Value::string("reason: graph point detached vertical")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":242:7
                if { let pui_native_588={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_589=v_7469636b65745f5f30613937666130623633303565653466.clone(); !(pui_native_588==pui_native_589) } {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":242:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":243:7
                if ({ let left = Value::boolean(v_6163746976655f5f37346336666162376666663231363966.clone()); if left.truthy() { v_68616e646c655f5f63643761363561653065386437613762.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":243:29
                    { let v_7075695f6e61746976655f353930 = v_68616e646c655f5f63643761363561653065386437613762.clone(); emit(&(v_7075695f6e61746976655f353930.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f37656438323135666166313266383034.clone())])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":245:5
            { let v_7075695f6e61746976655f353931 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":246:5
            { let v_7075695f6e61746976655f353932 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f30366663363562336338346163623361: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":246:43
                Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/graph-point.proto.ts":246:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f30366663363562336338346163623361.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":247:5
            { let v_7075695f6e61746976655f353933 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("x"), Value::string("y"), Value::string("minX"), Value::string("maxX"), Value::string("minY"), Value::string("maxY"), Value::string("hasX"), Value::string("hasY"), Value::string("label"), Value::string("xLabel"), Value::string("yLabel"), Value::string("disabled"), Value::string("revision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":248:5
            { let v_7075695f6e61746976655f353934 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":248:37
                return (v_726574697265.read().call)(false);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":249:5
            { let v_7075695f6e61746976655f353935 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":249:41
                return (v_726574697265.read().call)(true);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":250:5
            return { let callable={ let pui_native_596={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":250:26
                return { let v_7075695f6e61746976655f353937 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_596.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_596.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> GraphPointHandle { GraphPointHandle { owner: self.owner.clone() } }
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
  impl Drop for GraphPoint { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for GraphPoint { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for GraphPoint {
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
