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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("value").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("min").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("max").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("step").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("fineStep").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("defaultValue").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("dragMode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("doubleAction").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("tapAction").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("previewEcho").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("keyMode").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "value" => value.as_f64().is_some_and(f64::is_finite),
        "min" => value.as_f64().is_some_and(f64::is_finite),
        "max" => value.as_f64().is_some_and(f64::is_finite),
        "step" => value.as_f64().is_some_and(f64::is_finite),
        "fineStep" => value.as_f64().is_some_and(f64::is_finite),
        "defaultValue" => value.as_f64().is_some_and(f64::is_finite),
        "label" => value.is_string(),
        "disabled" => value.is_boolean(),
        "revision" => value.is_string(),
        "role" => value.is_string(),
        "dragMode" => value.is_string(),
        "doubleAction" => value.is_string(),
        "tapAction" => value.is_string(),
        "previewEcho" => value.is_boolean(),
        "keyMode" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "edit" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "reset" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "adjust" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "key" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<f64>,
    #[serde(rename = "min", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<f64>,
    #[serde(rename = "max", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "step", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "fineStep", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<f64>,
    #[serde(rename = "defaultValue", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<f64>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<String>,
    #[serde(rename = "role", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<String>,
    #[serde(rename = "dragMode", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<String>,
    #[serde(rename = "doubleAction", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<String>,
    #[serde(rename = "tapAction", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<String>,
    #[serde(rename = "previewEcho", default, skip_serializing_if = "Option::is_none")]
    pub field_13: Option<bool>,
    #[serde(rename = "keyMode", default, skip_serializing_if = "Option::is_none")]
    pub field_14: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct ContinuousControlHandle { pub owner: OwnerRef }
  impl ContinuousControlHandle {
      pub fn state_0(&self) -> Result<f64, Value> { self.state("value").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("initialValue").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_12(&self, argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
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
    pub fn method_13(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
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
    pub fn method_14(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
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
        let callback = self.owner.borrow().exposes.get("edit").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("edit", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_17(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("reset").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("reset", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_18(&self, argument_0: f64, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("adjust").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("adjust", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_19(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
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
  pub struct ContinuousControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl ContinuousControl {
      pub fn create_props(props: GeneratedProps, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> { Self::create(serde_json::to_value(props).expect("portable Props"), parent, window, cx) }
      pub fn create(raw_props: Json, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> { Self::create_with_associations(raw_props,parent,NativeInstanceAssociations::default(),window,cx) }
      pub fn create_with_associations(raw_props: Json, parent: Option<OwnerWeak>, associations: NativeInstanceAssociations, window: &mut Window, cx: &mut App) -> Result<gpui::Entity<Self>, Value> {
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
      pub fn new(raw_props: Json, parent: Option<OwnerWeak>, window: &mut Window, cx: &mut Context<Self>) -> Result<Self, Value> { Self::new_with_associations(raw_props,parent,NativeInstanceAssociations::default(),window,cx) }
      pub fn new_with_associations(raw_props: Json, parent: Option<OwnerWeak>, associations: NativeInstanceAssociations, window: &mut Window, cx: &mut Context<Self>) -> Result<Self, Value> {
          catch_authored(|| {
          let owner_ref = Owner::new(raw_props, parent.unwrap_or_default(), accepts_context, accepts_props, prop_valid, style_vocabulary());
          let owner = Rc::downgrade(&owner_ref);
          set_instance_associations(&owner_ref, associations);
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
            let v_76616c7565: LexicalBinding<Value> = lexical_binding();
            let v_696e697469616c56616c7565: LexicalBinding<Value> = lexical_binding();
            let v_6d696e696d756d: LexicalBinding<Value> = lexical_binding();
            let v_6d6178696d756d: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6163636570746564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73746570506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_66696e65506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_64726167506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646f75626c65506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_746170506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b6579626f617264506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_656e61626c6564: LexicalBinding<Value> = lexical_binding();
            let v_636c616d70: LexicalBinding<Value> = lexical_binding();
            let v_7075626c697368: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<Value> = lexical_binding();
            let v_70726576696577: LexicalBinding<Value> = lexical_binding();
            let v_65646974: LexicalBinding<Value> = lexical_binding();
            let v_7265736574: LexicalBinding<Value> = lexical_binding();
            let v_73616d706c65: LexicalBinding<Value> = lexical_binding();
            let v_61646a757374: LexicalBinding<Value> = lexical_binding();
            let v_6b6579: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":18:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("value", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("min", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("max", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("step", record(vec![("type", Value::string("number")), ("default", Value::number(0.01))])), ("fineStep", record(vec![("type", Value::string("number")), ("default", Value::number(0.001))])), ("defaultValue", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("slider"))])), ("dragMode", record(vec![("type", Value::string("string")), ("default", Value::string("absolute"))])), ("doubleAction", record(vec![("type", Value::string("string")), ("default", Value::string("reset"))])), ("tapAction", record(vec![("type", Value::string("string")), ("default", Value::string("none"))])), ("previewEcho", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("keyMode", record(vec![("type", Value::string("string")), ("default", Value::string("scalar"))]))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":28:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":29:11
            v_76616c7565.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("value"), Value::number(0.0), record(vec![("min", Value::number(-(Value::number(1000000000000000.0)).scalar_number())), ("max", Value::number(1000000000000000.0))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":30:11
            v_696e697469616c56616c7565.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "range", Value::string("initialValue"), Value::number(0.0), record(vec![("min", Value::number(-(Value::number(1000000000000000.0)).scalar_number())), ("max", Value::number(1000000000000000.0))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":31:11
            v_6d696e696d756d.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "range", Value::string("minimum"), Value::number(0.0), record(vec![("min", Value::number(-(Value::number(1000000000000000.0)).scalar_number())), ("max", Value::number(1000000000000000.0))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":32:11
            v_6d6178696d756d.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "range", Value::string("maximum"), Value::number(1.0), record(vec![("min", Value::number(-(Value::number(1000000000000000.0)).scalar_number())), ("max", Value::number(1000000000000000.0))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":33:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":34:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":35:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("slider"), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":36:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":37:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":38:5
            { let v_7075695f6e61746976655f39 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f39.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3130 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f3130.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3131 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f3131.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3132 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3132.clone()).owner(), Value::string("valueMin"), v_6d696e696d756d.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3133 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("valueMax"), v_6d6178696d756d.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3134 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("valueNow"), v_76616c7565.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3135 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3136 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("increment"), record(vec![("event", Value::string("host:verdant-scalar-increment"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3137 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("decrement"), record(vec![("event", Value::string("host:verdant-scalar-decrement"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3138 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("setValue"), record(vec![("event", Value::string("host:verdant-scalar-set-value"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":50:5
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-w-0".into(), "min-w-".into()), ("min-h-0".into(), "min-h-".into())])]) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":51:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("value"), v_76616c7565.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":52:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("initialValue"), v_696e697469616c56616c7565.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":53:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":54:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":55:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":56:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":57:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_event(&owner, Value::string("interactionStart")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":58:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_event(&owner, Value::string("valuePreview")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":59:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_event(&owner, Value::string("valueCommit")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":60:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":61:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":62:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("editRequest")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":64:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":65:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":66:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":67:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":68:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":69:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":70:9
            v_6163636570746564.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":71:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":72:9
            v_73746570506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.01))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":72:32
            v_66696e65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.001))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:9
            v_64726167506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("absolute"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:38
            v_646f75626c65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("reset"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:66
            v_746170506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("none"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:90
            v_6b6579626f617264506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("scalar"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":74:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f34653238656432366334303030303265: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":74:50
                return { let left = { let left = Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34653238656432366334303030303265.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":75:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f35313838383865333462313934656136: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":75:59
                return { let left = { let left = Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f35313838383865333462313934656136.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:11
            v_656e61626c6564.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:36
                return { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3332 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3332.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":77:11
            v_636c616d70.initialize({  let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f38306161363332333738313931363233: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":78:7
                if (Value::boolean(!(number_is_finite(&(v_6e6578745f5f38306161363332333738313931363233.clone()))).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":78:35
                    return { let v_7075695f6e61746976655f3333 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3333.clone()).get() };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":79:7
                if ({ let relational_left = v_6e6578745f5f38306161363332333738313931363233.clone(); let relational_right = { let v_7075695f6e61746976655f3334 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3334.clone()).get() }; Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":79:33
                    return { let v_7075695f6e61746976655f3335 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3335.clone()).get() };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:7
                if ({ let relational_left = v_6e6578745f5f38306161363332333738313931363233.clone(); let relational_right = { let v_7075695f6e61746976655f3336 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3336.clone()).get() }; Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:33
                    return { let v_7075695f6e61746976655f3337 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3337.clone()).get() };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":81:7
                return v_6e6578745f5f38306161363332333738313931363233.clone();
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":83:11
            v_7075626c697368.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_636c616d70 = v_636c616d70.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f34323630656562383533396238663164: Value = arg(&arguments, 0);
                let v_7469636b65745f5f64653366326261356464333666653861: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64653366326261356464333666653861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":85:7
                { let v_7075695f6e61746976655f3338 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f3338.clone(), v_636c616d70.read().call_authored(vec![v_6e6578745f5f34323630656562383533396238663164.clone()]), Value::string("reason: continuous control display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":86:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64653366326261356464333666653861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":86:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":87:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":87:28
                    { let v_7075695f6e61746976655f3339 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3339.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":88:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64653366326261356464333666653861.clone()]);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":90:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":91:13
                let v_7469636b65745f5f64326335646230333836306132666632 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":92:7
                if ({ let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3430 = v_7068617365.read(); (v_7075695f6e61746976655f3430.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":92:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":93:13
                let v_62617369735f5f35376264356135636338646163343563 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":94:7
                { let v_7075695f6e61746976655f3431 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3431.clone(), Value::string("idle"), Value::string("reason: continuous control canceled")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":95:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64326335646230333836306132666632.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f64326335646230333836306132666632.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":95:59
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:16
                    { let v_7075695f6e61746976655f3432 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3432.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", v_62617369735f5f35376264356135636338646163343563.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":97:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":99:11
            v_70726f706f7365.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f62343530636237336332633561306132: Value = arg(&arguments, 0);
                let v_7469636b65745f5f65626536366437313864613864373433: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65626536366437313864613864373433.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_6e6578745f5f62343530636237336332633561306132.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:69
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":101:13
                let v_70726f706f73616c = v_636c616d70.read().call_authored(vec![v_6e6578745f5f62343530636237336332633561306132.clone()]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":102:13
                let v_62617369735f5f31303637303538613462363564333265 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":103:13
                let v_70726f70735f5f33356666666236623964383533303061 = { let v_7075695f6e61746976655f3433 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3433.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3433.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":108:13
                let v_6563686f65645f5f30623563653365323362316238663138 = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3434 = v_7068617365.read(); (v_7075695f6e61746976655f3434.clone()).get() }), &(Value::string("idle")))); if left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f33356666666236623964383533303061.clone()).member("previewEcho", true)), &(Value::boolean(true)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f33356666666236623964383533303061.clone()).member("value", false)), &(v_70726f706f73616c.clone()))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &(v_62617369735f5f31303637303538613462363564333265.clone()))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("min", false); if left.nullish() { Value::number(0.0) } else { left } }), &({ let v_7075695f6e61746976655f3435 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3435.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("max", false); if left.nullish() { Value::number(1.0) } else { left } }), &({ let v_7075695f6e61746976655f3436 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3436.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_70726f70735f5f33356666666236623964383533303061.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("step", false); if left.nullish() { Value::number(0.01) } else { left } }), &({ let value = (v_73746570506f6c696379.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("fineStep", false); if left.nullish() { Value::number(0.001) } else { left } }), &({ let value = (v_66696e65506f6c696379.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }), &({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }), &({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }), &({ let value = (v_746170506f6c696379.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }), &({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value }))) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":115:7
                { let v_7075695f6e61746976655f3437 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3437.clone(), Value::string("idle"), Value::string("reason: continuous control proposal")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":116:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65626536366437313864613864373433.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":116:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":117:7
                if (v_6563686f65645f5f30623563653365323362316238663138.clone()).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":117:19
                    { let value = v_70726f706f73616c.clone(); *(v_6163636570746564.read()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":118:7
                if (Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65626536366437313864613864373433.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":118:39
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":119:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":119:16
                    { let v_7075695f6e61746976655f3438 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3438.clone()).owner(), Value::string("valueCommit"), record(vec![("value", v_70726f706f73616c.clone()), ("revision", v_62617369735f5f31303637303538613462363564333265.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":120:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":122:11
            v_70726576696577.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f39356437666630663563376664663234: Value = arg(&arguments, 0);
                let v_7469636b65745f5f38333136373639633633653334396636: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f38333136373639633633653334396636.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_6e6578745f5f39356437666630663563376664663234.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:69
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":124:7
                if (Value::boolean(!(v_7075626c697368.read().call_authored(vec![v_6e6578745f5f39356437666630663563376664663234.clone(), v_7469636b65745f5f38333136373639633633653334396636.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":124:35
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":125:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":125:16
                    { let v_7075695f6e61746976655f3439 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3439.clone()).owner(), Value::string("valuePreview"), record(vec![("value", { let v_7075695f6e61746976655f3530 = v_76616c7565.read(); (v_7075695f6e61746976655f3530.clone()).get() }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":126:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f38333136373639633633653334396636.clone()]);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":128:11
            v_65646974.initialize({  let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:7
                if (Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:23
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":130:13
                let v_7469636b65745f5f64646264363531316236613263386431 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":131:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":132:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64646264363531316236613263386431.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":132:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:16
                    { let v_7075695f6e61746976655f3531 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3531.clone()).owner(), Value::string("editRequest"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":134:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":136:11
            v_7265736574.initialize({  let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:7
                if (Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:23
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":138:13
                let v_7469636b65745f5f65316139323061646635356636333933 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":139:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":140:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65316139323061646635356636333933.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":140:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":141:7
                return v_70726f706f7365.read().call_authored(vec![{ let left = ({ let v_7075695f6e61746976655f3532 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3532.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3532.clone()).owner(), false) } }).member("defaultValue", true); if left.nullish() { { let value = (v_6163636570746564.read()).borrow().clone(); value } } else { left } }, v_7469636b65745f5f65316139323061646635356636333933.clone()]);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":143:5
            { let v_7075695f6e61746976655f3533 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_70726576696577 = v_70726576696577.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63616e6469646174655f5f39383836346239656138306134303166: Value = arg(&arguments, 0);
                let v_6a756d70: Value = arg(&arguments, 1);
                let v_72657365744d6f646966696572: Value = arg(&arguments, 2);
                let v_646f75626c65: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:7
                if ({ let left = { let left = Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3534 = v_7068617365.read(); (v_7075695f6e61746976655f3534.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_63616e6469646174655f5f39383836346239656138306134303166.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:80
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":145:7
                if ({ let left = v_646f75626c65.clone(); if left.truthy() { Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3535 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3535.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3535.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("edit")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":145:63
                    return v_65646974.read().call_authored(vec![]);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":146:7
                if ({ let left = v_72657365744d6f646966696572.clone(); if !left.truthy() { { let left = v_646f75626c65.clone(); if left.truthy() { Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3536 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3536.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3536.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("reset")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":146:83
                    return v_7265736574.read().call_authored(vec![]);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":147:13
                let v_7469636b65745f5f65616431666461383165326338663337 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":148:7
                { let v_7075695f6e61746976655f3537 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3537.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65616431666461383165326338663337.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":150:7
                { let v_7075695f6e61746976655f3538 = v_696e697469616c56616c7565.read(); state_set(v_7075695f6e61746976655f3538.clone(), { let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::string("reason: continuous control starting authority")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":151:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65616431666461383165326338663337.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":151:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":152:7
                { let v_7075695f6e61746976655f3539 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3539.clone(), if (v_6a756d70.clone()).truthy() { Value::string("dragging") } else { Value::string("armed") }, Value::string("reason: continuous control press")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65616431666461383165326338663337.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":154:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":154:16
                    { let v_7075695f6e61746976655f3630 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3630.clone()).owner(), Value::string("interactionStart"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65616431666461383165326338663337.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":156:7
                if (v_6a756d70.clone()).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":156:17
                    return v_70726576696577.read().call_authored(vec![v_63616e6469646174655f5f39383836346239656138306134303166.clone(), v_7469636b65745f5f65616431666461383165326338663337.clone()]);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:16
                    { let v_7075695f6e61746976655f3631 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3631.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":158:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65616431666461383165326338663337.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":160:11
            v_73616d706c65.initialize({  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_72756e = v_72756e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726576696577 = v_70726576696577.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63616e6469646174655f5f66396364663334666239346264373932: Value = arg(&arguments, 0);
                let v_66696e6543616e6469646174655f5f66383430363439363131333634386431: Value = arg(&arguments, 1);
                let v_63726f737365645f5f66393836616266333261623466386336: Value = arg(&arguments, 2);
                let v_66696e655f5f31383764643033363066623231643039: Value = arg(&arguments, 3);
                let v_7469636b65745f5f35393739333237626338626437306331: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f35393739333237626338626437306331.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3632 = v_7068617365.read(); (v_7075695f6e61746976655f3632.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:69
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:7
                if ({ let left = Value::boolean(!(number_is_finite(&(v_63616e6469646174655f5f66396364663334666239346264373932.clone()))).truthy()); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_66696e6543616e6469646174655f5f66383430363439363131333634386431.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:77
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:87
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3633 = v_7068617365.read(); (v_7075695f6e61746976655f3633.clone()).get() }), &(Value::string("armed")))); if left.truthy() { Value::boolean(!(v_63726f737365645f5f66393836616266333261623466386336.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:48
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":164:7
                { let v_7075695f6e61746976655f3634 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3634.clone(), Value::string("dragging"), Value::string("reason: continuous control dragging")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":165:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f35393739333237626338626437306331.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":165:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":166:11
                let v_6e6578745f5f35333437323333373330353138636165 = Rc::new(std::cell::RefCell::new(if (v_66696e655f5f31383764643033363066623231643039.clone()).truthy() { v_66696e6543616e6469646174655f5f66383430363439363131333634386431.clone() } else { v_63616e6469646174655f5f66396364663334666239346264373932.clone() }));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":167:13
                let v_68616e646c655f5f63633738666639336539353764376564 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":168:7
                if ({ let left = v_68616e646c655f5f63633738666639336539353764376564.clone(); if left.truthy() { Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3635 = v_68616e646c655f5f63633738666639336539353764376564.clone(); read_props(&(v_7075695f6e61746976655f3635.clone()).owner(), false) }).member("dragMode", false)), &(Value::string("steps")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":169:15
                    let v_616d6f756e745f5f65323230383363363766306662633730 = if (v_66696e655f5f31383764643033363066623231643039.clone()).truthy() { { let left = ({ let v_7075695f6e61746976655f3636 = v_68616e646c655f5f63633738666639336539353764376564.clone(); read_props(&(v_7075695f6e61746976655f3636.clone()).owner(), false) }).member("fineStep", false); if left.nullish() { Value::number(0.001) } else { left } } } else { { let left = ({ let v_7075695f6e61746976655f3637 = v_68616e646c655f5f63633738666639336539353764376564.clone(); read_props(&(v_7075695f6e61746976655f3637.clone()).owner(), false) }).member("step", false); if left.nullish() { Value::number(0.01) } else { left } } };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":170:9
                    { let value = add({ let v_7075695f6e61746976655f3638 = v_696e697469616c56616c7565.read(); (v_7075695f6e61746976655f3638.clone()).get() }, Value::number((Value::number({ let input: f64 = (v_63616e6469646174655f5f66396364663334666239346264373932.clone()).scalar_number(); if !input.is_finite() || input == 0.0 || input.abs() >= 4503599627370496.0 { input } else if input >= -0.5 && input < 0.0 { -0.0 } else { let lower = input.floor(); if input - lower >= 0.5 { lower + 1.0 } else { lower } } })).scalar_number() * (v_616d6f756e745f5f65323230383363363766306662633730.clone()).scalar_number())); *(v_6e6578745f5f35333437323333373330353138636165.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":172:7
                return v_70726576696577.read().call_authored(vec![{ let value = (v_6e6578745f5f35333437323333373330353138636165.clone()).borrow().clone(); value }, v_7469636b65745f5f35393739333237626338626437306331.clone()]);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":174:5
            { let v_7075695f6e61746976655f3639 = v_646566.clone(); expose_method(&owner, Value::string("sample"), {  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c65 = v_73616d706c65.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63616e6469646174655f5f32303334333739656635616634626663: Value = arg(&arguments, 0);
                let v_66696e6543616e6469646174655f5f63363166613436633661306534343930: Value = arg(&arguments, 1);
                let v_63726f737365645f5f36313336643463613231646436353265: Value = arg(&arguments, 2);
                let v_66696e655f5f33313035326334656364623930313239: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":175:13
                let v_7469636b65745f5f34346437656539373833343661383539 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":176:7
                return v_73616d706c65.read().call_authored(vec![v_63616e6469646174655f5f32303334333739656635616634626663.clone(), v_66696e6543616e6469646174655f5f63363166613436633661306534343930.clone(), v_63726f737365645f5f36313336643463613231646436353265.clone(), v_66696e655f5f33313035326334656364623930313239.clone(), v_7469636b65745f5f34346437656539373833343661383539.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":178:5
            { let v_7075695f6e61746976655f3730 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_73616d706c65 = v_73616d706c65.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63616e6469646174655f5f37623563613835613162643632323335: Value = arg(&arguments, 0);
                let v_66696e6543616e6469646174655f5f38646237336638656637333061623064: Value = arg(&arguments, 1);
                let v_63726f737365645f5f31373764353031323765623065626235: Value = arg(&arguments, 2);
                let v_66696e655f5f30623532613038373033653031636564: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":179:13
                let v_7469636b65745f5f31376239333931623833333938653038 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":180:7
                if ({ let left = Value::boolean(!(v_73616d706c65.read().call_authored(vec![v_63616e6469646174655f5f37623563613835613162643632323335.clone(), v_66696e6543616e6469646174655f5f38646237336638656637333061623064.clone(), v_63726f737365645f5f31373764353031323765623065626235.clone(), v_66696e655f5f30623532613038373033653031636564.clone(), v_7469636b65745f5f31376239333931623833333938653038.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31376239333931623833333938653038.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":180:89
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":181:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3731 = v_7068617365.read(); (v_7075695f6e61746976655f3731.clone()).get() }), &(Value::string("dragging"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":181:39
                    return v_70726f706f7365.read().call_authored(vec![{ let v_7075695f6e61746976655f3732 = v_76616c7565.read(); (v_7075695f6e61746976655f3732.clone()).get() }, v_7469636b65745f5f31376239333931623833333938653038.clone()]);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":182:13
                let v_616374696f6e = { let left = ({ let v_7075695f6e61746976655f3733 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3733.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3733.clone()).owner(), false) } }).member("tapAction", true); if left.nullish() { Value::string("none") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":183:7
                { let v_7075695f6e61746976655f3734 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3734.clone(), Value::string("idle"), Value::string("reason: continuous control released")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":184:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31376239333931623833333938653038.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f31376239333931623833333938653038.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":184:59
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":185:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":185:41
                    { let v_7075695f6e61746976655f3735 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3735.clone()).owner(), Value::string("activate"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":186:7
                if (Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":186:32
                    return v_70726f706f7365.read().call_authored(vec![{ let value = (v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f31376239333931623833333938653038.clone()]);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":187:7
                if ({ let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":187:64
                    { let v_7075695f6e61746976655f3736 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3736.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":188:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":190:5
            { let v_7075695f6e61746976655f3737 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":191:5
            { let v_7075695f6e61746976655f3738 = v_646566.clone(); expose_method(&owner, Value::string("edit"), v_65646974.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":192:5
            { let v_7075695f6e61746976655f3739 = v_646566.clone(); expose_method(&owner, Value::string("reset"), v_7265736574.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":193:11
            v_61646a757374.initialize({  let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_646972656374696f6e5f5f37373836363836316538343434393238: Value = arg(&arguments, 0);
                let v_66696e655f5f34396636313436306538393838323134: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":194:7
                if ({ let left = Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_646972656374696f6e5f5f37373836363836316538343434393238.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":194:54
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":195:13
                let v_7469636b65745f5f64363061646664653662356166303037 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":196:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":197:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64363061646664653662356166303037.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":197:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":198:13
                let v_70726f70735f5f62623833666435303635316464653833 = { let v_7075695f6e61746976655f3830 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3830.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3830.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":199:13
                let v_616d6f756e745f5f37646164306663626564353435613336 = if (v_66696e655f5f34396636313436306538393838323134.clone()).truthy() { { let left = (v_70726f70735f5f62623833666435303635316464653833.clone()).member("fineStep", true); if left.nullish() { Value::number(0.001) } else { left } } } else { { let left = (v_70726f70735f5f62623833666435303635316464653833.clone()).member("step", true); if left.nullish() { Value::number(0.01) } else { left } } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":200:7
                return v_70726f706f7365.read().call_authored(vec![add({ let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::number((v_646972656374696f6e5f5f37373836363836316538343434393238.clone()).scalar_number() * (v_616d6f756e745f5f37646164306663626564353435613336.clone()).scalar_number())), v_7469636b65745f5f64363061646664653662356166303037.clone()]);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":202:5
            { let v_7075695f6e61746976655f3831 = v_646566.clone(); expose_method(&owner, Value::string("adjust"), v_61646a757374.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-increment"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:59
                v_61646a757374.read().call_authored(vec![Value::number(1.0), Value::boolean(false)]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":204:5
            { let v_7075695f6e61746976655f3833 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-decrement"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":204:59
                v_61646a757374.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), Value::boolean(false)]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":205:5
            { let v_7075695f6e61746976655f3834 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-scalar-set-value"), false, {  let v_65706f6368 = v_65706f6368.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f66366332376462333063633166356339: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":206:13
                let v_6576656e745f5f66613762356539613661396230383336 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":207:13
                let v_6e6578745f5f37393139333764343563376462633531 = (v_6576656e745f5f66613762356539613661396230383336.clone()).member("value", false);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":208:7
                if ({ let left = { let left = Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(typeof_value(&(v_6e6578745f5f37393139333764343563376462633531.clone()))), &(Value::string("number")))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_6e6578745f5f37393139333764343563376462633531.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":208:77
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":209:13
                let v_7469636b65745f5f64636164386133666563333064333832 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":210:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":211:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64636164386133666563333064333832.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":211:43
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":212:7
                v_70726f706f7365.read().call_authored(vec![v_6e6578745f5f37393139333764343563376462633531.clone(), v_7469636b65745f5f64636164386133666563333064333832.clone()]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":214:11
            v_6b6579.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d65: Value = arg(&arguments, 0);
                let v_7368696674: Value = arg(&arguments, 1);
                let v_636f6d6d616e64: Value = arg(&arguments, 2);
                let v_616c74: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":215:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Escape"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":216:15
                    let v_616374697665 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3835 = v_7068617365.read(); (v_7075695f6e61746976655f3835.clone()).get() }), &(Value::string("idle"))));
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":217:9
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":217:19
                    return v_616374697665.clone();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":219:7
                if ({ let left = { let left = Value::boolean(!(v_656e61626c6564.read().call_authored(vec![])).truthy()); if !left.truthy() { v_616c74.clone() } else { left } }; if !left.truthy() { v_636f6d6d616e64.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":219:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":222:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3836 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3836.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3836.clone()).owner(), false) } }).member("keyMode", true)), &(Value::string("table-point")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("=")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":223:58
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":224:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowLeft")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Home")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("End")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("=")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":226:102
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":227:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":227:31
                    v_65646974.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":227:39
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":228:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":228:35
                    v_7265736574.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":228:44
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":229:7
                if ({ let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("End")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":230:15
                    let v_7469636b65745f5f39393631666463356235643834643032 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":230:35
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":231:9
                    if ({ let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39393631666463356235643834643032.clone()]); if left.truthy() { v_656e61626c6564.read().call_authored(vec![]) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":231:43
                        v_70726f706f7365.read().call_authored(vec![if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).truthy() { { let v_7075695f6e61746976655f3837 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3837.clone()).get() } } else { { let v_7075695f6e61746976655f3838 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3838.clone()).get() } }, v_7469636b65745f5f39393631666463356235643834643032.clone()]);
                    }
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":232:9
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":234:13
                let v_646972656374696f6e5f5f61346138613361353561336361366366 = if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp"))))).truthy() { Value::number(10.0) } else { if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown"))))).truthy() { Value::number(-(Value::number(10.0)).scalar_number()) } else { if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("+")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("=")))) } else { left } }).truthy() { Value::number(1.0) } else { Value::number(-(Value::number(1.0)).scalar_number()) } } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":236:7
                v_61646a757374.read().call_authored(vec![v_646972656374696f6e5f5f61346138613361353561336361366366.clone(), v_7368696674.clone()]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":237:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":239:5
            { let v_7075695f6e61746976655f3839 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b6579.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":240:5
            { let v_7075695f6e61746976655f3930 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f39626535623834363661396364616434: Value = arg(&arguments, 0);
                let v_6576656e745f5f33356561366436353636373335373039: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":241:7
                if (v_6b6579.read().call_authored(vec![{ let left = (v_6576656e745f5f33356561366436353636373335373039.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f33356561366436353636373335373039.clone()).member("shiftKey", false)), &(Value::boolean(true)))), { let left = Value::boolean(strict_equal(&((v_6576656e745f5f33356561366436353636373335373039.clone()).member("ctrlKey", false)), &(Value::boolean(true)))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f33356561366436353636373335373039.clone()).member("metaKey", false)), &(Value::boolean(true)))) } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f33356561366436353636373335373039.clone()).member("altKey", false)), &(Value::boolean(true))))])).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":241:131
                    { let v_7075695f6e61746976655f3931 = v_6576656e745f5f33356561366436353636373335373039.clone(); prevent(v_7075695f6e61746976655f3931.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":243:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); let v_636c616d70 = v_636c616d70.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f39343061373865643433373564383833: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":244:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":244:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":245:13
                let v_70726f70735f5f64373235346431346335363437376337 = { let v_7075695f6e61746976655f3932 = v_68616e646c655f5f39343061373865643433373564383833.clone(); read_props(&(v_7075695f6e61746976655f3932.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":246:13
                let v_62617369735f5f36396433313931386563376236353531 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":247:13
                let v_70726576696f7573 = { let value = (v_6163636570746564.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":248:13
                let v_706f6c6963794368616e676564 = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_73746570506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("step", false); if left.nullish() { Value::number(0.01) } else { left } }))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_66696e65506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("fineStep", false); if left.nullish() { Value::number(0.001) } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_746170506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }))) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":252:13
                let v_6563686f65645f5f61383832373063636565653034376361 = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("previewEcho", false)), &(Value::boolean(true)))); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3933 = v_7068617365.read(); (v_7075695f6e61746976655f3933.clone()).get() }), &(Value::string("idle")))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("value", false)), &({ let v_7075695f6e61746976655f3934 = v_76616c7565.read(); (v_7075695f6e61746976655f3934.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("min", false); if left.nullish() { Value::number(0.0) } else { left } }), &({ let v_7075695f6e61746976655f3935 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3935.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("max", false); if left.nullish() { Value::number(1.0) } else { left } }), &({ let v_7075695f6e61746976655f3936 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3936.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } }; if left.truthy() { Value::boolean(!(v_706f6c6963794368616e676564.clone()).truthy()) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":259:7
                if (Value::boolean(!(v_6563686f65645f5f61383832373063636565653034376361.clone()).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":259:20
                    { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":260:13
                let v_7469636b65745f5f31396134653934313436323235633230 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":261:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":262:7
                { let value = v_68616e646c655f5f39343061373865643433373564383833.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":263:7
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("step", false); if left.nullish() { Value::number(0.01) } else { left } }; *(v_73746570506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":263:40
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("fineStep", false); if left.nullish() { Value::number(0.001) } else { left } }; *(v_66696e65506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":264:7
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }; *(v_64726167506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":264:50
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }; *(v_646f75626c65506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":264:96
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }; *(v_746170506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":265:7
                { let value = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }; *(v_6b6579626f617264506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":266:13
                let v_6c6f77 = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("min", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":267:13
                let v_68696768 = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("max", false); if left.nullish() { Value::number(1.0) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":268:13
                let v_76616c6964 = { let left = { let left = { let left = { let left = { let left = number_is_finite(&(v_6c6f77.clone())); if left.truthy() { number_is_finite(&(v_68696768.clone())) } else { left } }; if left.truthy() { { let relational_left = v_6c6f77.clone(); let relational_right = v_68696768.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }; if left.truthy() { number_is_finite(&({ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("value", false); if left.nullish() { Value::number(0.0) } else { left } })) } else { left } }; if left.truthy() { { let relational_left = v_6c6f77.clone(); let relational_right = Value::number(-(Value::number(1000000000000000.0)).scalar_number()); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { { let relational_left = v_68696768.clone(); let relational_right = Value::number(1000000000000000.0); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":270:13
                let v_72616e67654368616e676564 = { let left = Value::boolean(!strict_equal(&(v_6c6f77.clone()), &({ let v_7075695f6e61746976655f3937 = v_6d696e696d756d.read(); (v_7075695f6e61746976655f3937.clone()).get() }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_68696768.clone()), &({ let v_7075695f6e61746976655f3938 = v_6d6178696d756d.read(); (v_7075695f6e61746976655f3938.clone()).get() }))) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":271:7
                { let v_7075695f6e61746976655f3939 = v_6d696e696d756d.read(); state_set(v_7075695f6e61746976655f3939.clone(), if (v_76616c6964.clone()).truthy() { v_6c6f77.clone() } else { Value::number(0.0) }, Value::string("reason: continuous control minimum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":272:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":272:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":273:7
                { let v_7075695f6e61746976655f313030 = v_6d6178696d756d.read(); state_set(v_7075695f6e61746976655f313030.clone(), if (v_76616c6964.clone()).truthy() { v_68696768.clone() } else { Value::number(1.0) }, Value::string("reason: continuous control maximum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":274:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":274:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":275:13
                let v_6e6578745f5f30633538646561313464366433653565 = v_636c616d70.read().call_authored(vec![{ let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("value", false); if left.nullish() { Value::number(0.0) } else { left } }]);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":276:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":277:13
                let v_6563686f = { let left = { let left = { let left = Value::boolean(strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("previewEcho", false)), &(Value::boolean(true)))); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313031 = v_7068617365.read(); (v_7075695f6e61746976655f313031.clone()).get() }), &(Value::string("idle")))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&(v_6e6578745f5f30633538646561313464366433653565.clone()), &({ let v_7075695f6e61746976655f313032 = v_76616c7565.read(); (v_7075695f6e61746976655f313032.clone()).get() }))) } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":279:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313033 = v_7068617365.read(); (v_7075695f6e61746976655f313033.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_76616c6964.clone()).truthy()); if !left.truthy() { v_72616e67654368616e676564.clone() } else { left } }; if !left.truthy() { v_706f6c6963794368616e676564.clone() } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { { let left = Value::boolean(!(v_6563686f.clone()).truthy()); if left.truthy() { Value::boolean(!strict_equal(&(v_6e6578745f5f30633538646561313464366433653565.clone()), &({ let value = (v_6163636570746564.read()).borrow().clone(); value }))) } else { left } } } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":281:7
                if (Value::boolean(!(v_6563686f.clone()).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":281:18
                    { let value = v_6e6578745f5f30633538646561313464366433653565.clone(); *(v_6163636570746564.read()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":282:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":283:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":284:9
                    { let v_7075695f6e61746976655f313034 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313034.clone(), Value::string("idle"), Value::string("reason: continuous control authority changed")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":285:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":285:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":287:7
                { let v_7075695f6e61746976655f313035 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313035.clone(), { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: continuous control label")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":288:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":288:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":289:7
                { let v_7075695f6e61746976655f313036 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f313036.clone(), { let left = (v_70726f70735f5f64373235346431346335363437376337.clone()).member("role", false); if left.nullish() { Value::string("slider") } else { left } }, Value::string("reason: continuous control role")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":290:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":290:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":291:7
                { let v_7075695f6e61746976655f313037 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313037.clone(), { let left = Value::boolean(!(v_76616c6964.clone()).truthy()); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f64373235346431346335363437376337.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } }, Value::string("reason: continuous control policy")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":292:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":292:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":293:7
                { let v_7075695f6e61746976655f313038 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313038.clone()).owner(), { let v_7075695f6e61746976655f313039 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313039.clone()).get() }) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":295:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313130 = v_7068617365.read(); (v_7075695f6e61746976655f313130.clone()).get() }), &(Value::string("idle"))))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":296:9
                    { let v_7075695f6e61746976655f313131 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f313131.clone(), { let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::string("reason: continuous control authoritative display")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":297:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":297:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":299:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":299:25
                    { let v_7075695f6e61746976655f313132 = v_68616e646c655f5f39343061373865643433373564383833.clone(); emit(&(v_7075695f6e61746976655f313132.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", v_70726576696f7573.clone()), ("revision", v_62617369735f5f36396433313931386563376236353531.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":300:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f31396134653934313436323235633230.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":300:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":301:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":302:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":302:21
                    { let v_7075695f6e61746976655f313133 = v_68616e646c655f5f39343061373865643433373564383833.clone(); request_update(&(v_7075695f6e61746976655f313133.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":304:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":305:13
                let v_68616e646c655f5f34323635393230323831323662336661 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":306:13
                let v_776173416374697665 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313134 = v_7068617365.read(); (v_7075695f6e61746976655f313134.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":307:13
                let v_62617369735f5f35373135333435383530396239363265 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":308:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":309:13
                let v_7469636b65745f5f34363766363362363463363632396330 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:24
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:42
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:76
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":311:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":311:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":312:7
                { let v_7075695f6e61746976655f313135 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313135.clone(), Value::string("idle"), Value::string("reason: continuous control detached")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":313:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34363766363362363463363632396330.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":313:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":314:7
                { let v_7075695f6e61746976655f313136 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f313136.clone(), { let value = (v_6163636570746564.read()).borrow().clone(); value }, Value::string("reason: continuous control detached display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":315:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34363766363362363463363632396330.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":315:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:7
                if ({ let left = v_776173416374697665.clone(); if left.truthy() { v_68616e646c655f5f34323635393230323831323662336661.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:32
                    { let v_7075695f6e61746976655f313137 = v_68616e646c655f5f34323635393230323831323662336661.clone(); emit(&(v_7075695f6e61746976655f313137.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", { let value = (v_6163636570746564.read()).borrow().clone(); value }), ("revision", v_62617369735f5f35373135333435383530396239363265.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":318:5
            { let v_7075695f6e61746976655f313138 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:5
            { let v_7075695f6e61746976655f313139 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f66356636646536333037326564393036: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:43
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:60
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:77
                    v_73796e63.read().call_authored(vec![v_68616e646c655f5f66356636646536333037326564393036.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":320:5
            { let v_7075695f6e61746976655f313230 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("value"), Value::string("min"), Value::string("max"), Value::string("step"), Value::string("fineStep"), Value::string("defaultValue"), Value::string("label"), Value::string("disabled"), Value::string("revision"), Value::string("role"), Value::string("dragMode"), Value::string("doubleAction"), Value::string("tapAction"), Value::string("previewEcho"), Value::string("keyMode")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":322:5
            { let v_7075695f6e61746976655f313231 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":323:5
            { let v_7075695f6e61746976655f313232 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":323:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":324:5
            { let v_7075695f6e61746976655f313233 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":324:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:5
            return { let callable={ let pui_native_124={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:26
                return { let v_7075695f6e61746976655f313235 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_124.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_124.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ContinuousControlHandle { ContinuousControlHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let outcome = catch_authored(|| replace_props(&self.owner, raw));
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome?; self.drive(window, cx)
      }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_window_observation_needed(&self.owner) { self._window_activation.take(); return; }
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
  impl Drop for ContinuousControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for ContinuousControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for ContinuousControl {
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
