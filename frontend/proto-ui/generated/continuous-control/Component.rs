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
        "sample" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
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
            let v_76616c7565: LexicalBinding<Value> = lexical_binding();
            let v_696e697469616c56616c7565: LexicalBinding<Value> = lexical_binding();
            let v_6d696e696d756d: LexicalBinding<Value> = lexical_binding();
            let v_6d6178696d756d: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6163636570746564: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73746570506f6c696379: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_66696e65506f6c696379: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_64726167506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646f75626c65506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_746170506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b6579626f617264506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_656e61626c6564: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_636c616d70: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> f64>> = lexical_binding();
            let v_7075626c697368: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_63616e63656c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_70726576696577: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_65646974: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_7265736574: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_73616d706c65: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64, f64, bool, f64) -> bool>> = lexical_binding();
            let v_61646a757374: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool) -> bool>> = lexical_binding();
            let v_6b6579: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool, bool, bool) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(bool) -> Value>> = lexical_binding();
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":18:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("value", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("min", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("max", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("step", record(vec![("type", Value::string("number")), ("default", Value::number(0.01_f64))])), ("fineStep", record(vec![("type", Value::string("number")), ("default", Value::number(0.001_f64))])), ("defaultValue", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("slider"))])), ("dragMode", record(vec![("type", Value::string("string")), ("default", Value::string("absolute"))])), ("doubleAction", record(vec![("type", Value::string("string")), ("default", Value::string("reset"))])), ("tapAction", record(vec![("type", Value::string("string")), ("default", Value::string("none"))])), ("previewEcho", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("keyMode", record(vec![("type", Value::string("string")), ("default", Value::string("scalar"))]))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":28:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":29:11
            v_76616c7565.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("value"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":30:11
            v_696e697469616c56616c7565.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "range", Value::string("initialValue"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":31:11
            v_6d696e696d756d.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "range", Value::string("minimum"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":32:11
            v_6d6178696d756d.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "range", Value::string("maximum"), Value::number(1.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
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
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":66:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":67:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":68:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":69:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":70:9
            v_6163636570746564.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":71:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":72:9
            v_73746570506f6c696379.initialize(Rc::new(std::cell::RefCell::new(0.01_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":72:32
            v_66696e65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(0.001_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:9
            v_64726167506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("absolute"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:38
            v_646f75626c65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("reset"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:66
            v_746170506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("none"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:90
            v_6b6579626f617264506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("scalar"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:11
            v_656e61626c6564.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:36
                return { let pui_native_32={ let pui_native_34={ let pui_native_36={ let pui_native_38=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_38) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_38} }; if (pui_native_36) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_36} }; if (pui_native_34) {!({ let value=(v_73796e63696e67.read()).borrow().clone(); value })} else {pui_native_34} }; if (pui_native_32) {!(({ let v_7075695f6e61746976655f3432 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3432.clone()).get()).bool()) }).bool())} else {pui_native_32} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":77:11
            v_636c616d70.initialize({  let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let call:Rc<dyn Fn(f64)->f64>=Rc::new(move |argument_0: f64| -> f64 {
                let v_6e6578745f5f38306161363332333738313931363233: f64 = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":78:7
                if !((v_6e6578745f5f38306161363332333738313931363233.clone()).is_finite()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":78:35
                    return ({ let v_7075695f6e61746976655f3433 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3433.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":79:7
                if { let pui_native_44:f64=v_6e6578745f5f38306161363332333738313931363233.clone(); let pui_native_45:f64=({ let v_7075695f6e61746976655f3436 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3436.clone()).get()).num()) }).num(); pui_native_44 < pui_native_45 } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":79:33
                    return ({ let v_7075695f6e61746976655f3437 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3437.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:7
                if { let pui_native_48:f64=v_6e6578745f5f38306161363332333738313931363233.clone(); let pui_native_49:f64=({ let v_7075695f6e61746976655f3530 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f3530.clone()).get()).num()) }).num(); pui_native_48 > pui_native_49 } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:33
                    return ({ let v_7075695f6e61746976655f3531 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f3531.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":81:7
                return v_6e6578745f5f38306161363332333738313931363233.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":83:11
            v_7075626c697368.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_636c616d70 = v_636c616d70.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f34323630656562383533396238663164: f64 = argument_0;
                let v_7469636b65745f5f64653366326261356464333666653861: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:7
                if !({ let pui_native_52={ let pui_native_54=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_54) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_54} }; if (pui_native_52) {{ let pui_native_58={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_59=v_7469636b65745f5f64653366326261356464333666653861.clone(); (pui_native_58==pui_native_59) }} else {pui_native_52} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":85:7
                { let v_7075695f6e61746976655f3630 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f3630.clone(), Value::number((v_636c616d70.read().call)(v_6e6578745f5f34323630656562383533396238663164.clone())), Value::string("reason: continuous control display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":86:7
                if !({ let pui_native_61={ let pui_native_63=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_63) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_63} }; if (pui_native_61) {{ let pui_native_67={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_68=v_7469636b65745f5f64653366326261356464333666653861.clone(); (pui_native_67==pui_native_68) }} else {pui_native_61} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":86:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":87:7
                if { let pui_native_69={ let value = (v_72756e.read()).borrow().clone(); value }; if pui_native_69.truthy() {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {(pui_native_69).bool()} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":87:28
                    { let v_7075695f6e61746976655f3731 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3731.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":88:7
                return { let pui_native_72={ let pui_native_74=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_74) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_74} }; if (pui_native_72) {{ let pui_native_78={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_79=v_7469636b65745f5f64653366326261356464333666653861.clone(); (pui_native_78==pui_native_79) }} else {pui_native_72} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":90:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":91:13
                let v_7469636b65745f5f64326335646230333836306132666632: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":92:7
                if { let pui_native_80=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_80) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3834 = v_7068617365.read(); (v_7075695f6e61746976655f3834.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_80} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":92:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":93:13
                let v_62617369735f5f35376264356135636338646163343563 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":94:7
                { let v_7075695f6e61746976655f3835 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3835.clone(), Value::string("idle"), Value::string("reason: continuous control canceled")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":95:7
                if { let pui_native_86=!({ let pui_native_88={ let pui_native_90=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_90) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_90} }; if (pui_native_88) {{ let pui_native_94={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_95=v_7469636b65745f5f64326335646230333836306132666632.clone(); (pui_native_94==pui_native_95) }} else {pui_native_88} }); if !(pui_native_86) {!((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f64326335646230333836306132666632.clone()))} else {pui_native_86} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":95:59
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:16
                    { let v_7075695f6e61746976655f3936 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3936.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", v_62617369735f5f35376264356135636338646163343563.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":97:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":99:11
            v_70726f706f7365.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f62343530636237336332633561306132: f64 = argument_0;
                let v_7469636b65745f5f65626536366437313864613864373433: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:7
                if { let pui_native_97={ let pui_native_99=!({ let pui_native_101={ let pui_native_103=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_103) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_103} }; if (pui_native_101) {{ let pui_native_107={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_108=v_7469636b65745f5f65626536366437313864613864373433.clone(); (pui_native_107==pui_native_108) }} else {pui_native_101} }); if !(pui_native_99) {!((v_656e61626c6564.read().call)())} else {pui_native_99} }; if !(pui_native_97) {!((v_6e6578745f5f62343530636237336332633561306132.clone()).is_finite())} else {pui_native_97} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":101:13
                let v_70726f706f73616c: f64 = (v_636c616d70.read().call)(v_6e6578745f5f62343530636237336332633561306132.clone());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":102:13
                let v_62617369735f5f31303637303538613462363564333265 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":103:13
                let v_70726f70735f5f33356666666236623964383533303061 = { let v_7075695f6e61746976655f313039 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f313039.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f313039.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":108:13
                let v_6563686f65645f5f30623563653365323362316238663138: bool = { let pui_native_110={ let pui_native_112={ let pui_native_114={ let pui_native_116={ let pui_native_118={ let pui_native_120={ let pui_native_122={ let pui_native_124={ let pui_native_126={ let pui_native_128={ let pui_native_130={ let pui_native_132=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313336 = v_7068617365.read(); (v_7075695f6e61746976655f313336.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_132) {(Value::boolean(strict_equal(&((v_70726f70735f5f33356666666236623964383533303061.clone()).member("previewEcho", true)), &(Value::boolean(true))))).bool()} else {pui_native_132} }; if (pui_native_130) {{ let pui_native_139=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("value", false)).num(); let pui_native_140=v_70726f706f73616c.clone(); (pui_native_139==pui_native_140) }} else {pui_native_130} }; if (pui_native_128) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &(v_62617369735f5f31303637303538613462363564333265.clone())))).bool()} else {pui_native_128} }; if (pui_native_126) {{ let pui_native_143=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("min", false)).num(); let pui_native_144=({ let v_7075695f6e61746976655f313437 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f313437.clone()).get()).num()) }).num(); (pui_native_143==pui_native_144) }} else {pui_native_126} }; if (pui_native_124) {{ let pui_native_148=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("max", false)).num(); let pui_native_149=({ let v_7075695f6e61746976655f313532 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f313532.clone()).get()).num()) }).num(); (pui_native_148==pui_native_149) }} else {pui_native_124} }; if (pui_native_122) {{ let pui_native_153=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("disabled", false)).bool(); let pui_native_154=true; !(pui_native_153==pui_native_154) }} else {pui_native_122} }; if (pui_native_120) {{ let pui_native_155=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("step", false)).num(); let pui_native_156={ let value=(v_73746570506f6c696379.read()).borrow().clone(); value }; (pui_native_155==pui_native_156) }} else {pui_native_120} }; if (pui_native_118) {{ let pui_native_159=((v_70726f70735f5f33356666666236623964383533303061.clone()).member("fineStep", false)).num(); let pui_native_160={ let value=(v_66696e65506f6c696379.read()).borrow().clone(); value }; (pui_native_159==pui_native_160) }} else {pui_native_118} }; if (pui_native_116) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }), &({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_116} }; if (pui_native_114) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }), &({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_114} }; if (pui_native_112) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }), &({ let value = (v_746170506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_112} }; if (pui_native_110) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f33356666666236623964383533303061.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }), &({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_110} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":115:7
                { let v_7075695f6e61746976655f313731 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313731.clone(), Value::string("idle"), Value::string("reason: continuous control proposal")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":116:7
                if !({ let pui_native_172={ let pui_native_174=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_174) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_174} }; if (pui_native_172) {{ let pui_native_178={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_179=v_7469636b65745f5f65626536366437313864613864373433.clone(); (pui_native_178==pui_native_179) }} else {pui_native_172} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":116:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":117:7
                if v_6563686f65645f5f30623563653365323362316238663138.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":117:19
                    Value::number({ let value=v_70726f706f73616c.clone(); *(v_6163636570746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":118:7
                if !((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65626536366437313864613864373433.clone())) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":118:39
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":119:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":119:16
                    { let v_7075695f6e61746976655f313830 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313830.clone()).owner(), Value::string("valueCommit"), record(vec![("value", Value::number(v_70726f706f73616c.clone())), ("revision", v_62617369735f5f31303637303538613462363564333265.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":120:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":122:11
            v_70726576696577.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f39356437666630663563376664663234: f64 = argument_0;
                let v_7469636b65745f5f38333136373639633633653334396636: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:7
                if { let pui_native_181={ let pui_native_183=!({ let pui_native_185={ let pui_native_187=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_187) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_187} }; if (pui_native_185) {{ let pui_native_191={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_192=v_7469636b65745f5f38333136373639633633653334396636.clone(); (pui_native_191==pui_native_192) }} else {pui_native_185} }); if !(pui_native_183) {!((v_656e61626c6564.read().call)())} else {pui_native_183} }; if !(pui_native_181) {!((v_6e6578745f5f39356437666630663563376664663234.clone()).is_finite())} else {pui_native_181} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":124:7
                if !((v_7075626c697368.read().call)(v_6e6578745f5f39356437666630663563376664663234.clone(), v_7469636b65745f5f38333136373639633633653334396636.clone())) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":124:35
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":125:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":125:16
                    { let v_7075695f6e61746976655f313933 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313933.clone()).owner(), Value::string("valuePreview"), record(vec![("value", Value::number(({ let v_7075695f6e61746976655f313934 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f313934.clone()).get()).num()) }).num())), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":126:7
                return { let pui_native_195={ let pui_native_197=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_197) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_197} }; if (pui_native_195) {{ let pui_native_201={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_202=v_7469636b65745f5f38333136373639633633653334396636.clone(); (pui_native_201==pui_native_202) }} else {pui_native_195} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":128:11
            v_65646974.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:7
                if !((v_656e61626c6564.read().call)()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:23
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":130:13
                let v_7469636b65745f5f64646264363531316236613263386431: f64 = { let pui_native_203:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_204:f64=1.0_f64; pui_native_203 + pui_native_204 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":131:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":132:7
                if { let pui_native_205=!({ let pui_native_207={ let pui_native_209=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_209) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_209} }; if (pui_native_207) {{ let pui_native_213={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_214=v_7469636b65745f5f64646264363531316236613263386431.clone(); (pui_native_213==pui_native_214) }} else {pui_native_207} }); if !(pui_native_205) {!((v_656e61626c6564.read().call)())} else {pui_native_205} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":132:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:16
                    { let v_7075695f6e61746976655f323135 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f323135.clone()).owner(), Value::string("editRequest"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":134:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":136:11
            v_7265736574.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:7
                if !((v_656e61626c6564.read().call)()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:23
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":138:13
                let v_7469636b65745f5f65316139323061646635356636333933: f64 = { let pui_native_216:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_217:f64=1.0_f64; pui_native_216 + pui_native_217 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":139:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":140:7
                if { let pui_native_218=!({ let pui_native_220={ let pui_native_222=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_222) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_222} }; if (pui_native_220) {{ let pui_native_226={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_227=v_7469636b65745f5f65316139323061646635356636333933.clone(); (pui_native_226==pui_native_227) }} else {pui_native_220} }); if !(pui_native_218) {!((v_656e61626c6564.read().call)())} else {pui_native_218} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":140:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":141:7
                return (v_70726f706f7365.read().call)({ let pui_native_228=({ let v_7075695f6e61746976655f323330 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323330.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323330.clone()).owner(), false) } }).member("defaultValue", true); if pui_native_228.nullish() {{ let value=(v_6163636570746564.read()).borrow().clone(); value }} else {(pui_native_228).num()} }, v_7469636b65745f5f65316139323061646635356636333933.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":143:5
            { let v_7075695f6e61746976655f323331 = v_646566.clone(); expose_method(&owner, Value::string("begin"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_70726576696577 = v_70726576696577.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); let call:Rc<dyn Fn(f64, bool, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_63616e6469646174655f5f39383836346239656138306134303166: f64 = argument_0;
                let v_6a756d70: bool = argument_1;
                let v_72657365744d6f646966696572: bool = argument_2;
                let v_646f75626c65: bool = argument_3;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:7
                if { let pui_native_232={ let pui_native_234=!((v_656e61626c6564.read().call)()); if !(pui_native_234) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f323338 = v_7068617365.read(); (v_7075695f6e61746976655f323338.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_234} }; if !(pui_native_232) {!((v_63616e6469646174655f5f39383836346239656138306134303166.clone()).is_finite())} else {pui_native_232} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:80
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":145:7
                if { let pui_native_239=v_646f75626c65.clone(); if (pui_native_239) {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f323433 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323433.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323433.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("edit"))))).bool()} else {pui_native_239} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":145:63
                    return (v_65646974.read().call)();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":146:7
                if { let pui_native_244=v_72657365744d6f646966696572.clone(); if !(pui_native_244) {{ let pui_native_246=v_646f75626c65.clone(); if (pui_native_246) {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f323530 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323530.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323530.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("reset"))))).bool()} else {pui_native_246} }} else {pui_native_244} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":146:83
                    return (v_7265736574.read().call)();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":147:13
                let v_7469636b65745f5f65616431666461383165326338663337: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":148:7
                { let v_7075695f6e61746976655f323531 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323531.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:7
                if { let pui_native_252=!({ let pui_native_254={ let pui_native_256=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_256) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_256} }; if (pui_native_254) {{ let pui_native_260={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_261=v_7469636b65745f5f65616431666461383165326338663337.clone(); (pui_native_260==pui_native_261) }} else {pui_native_254} }); if !(pui_native_252) {!((v_656e61626c6564.read().call)())} else {pui_native_252} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":150:7
                { let v_7075695f6e61746976655f323632 = v_696e697469616c56616c7565.read(); state_set(v_7075695f6e61746976655f323632.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control starting authority")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":151:7
                if !({ let pui_native_263={ let pui_native_265=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_265) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_265} }; if (pui_native_263) {{ let pui_native_269={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_270=v_7469636b65745f5f65616431666461383165326338663337.clone(); (pui_native_269==pui_native_270) }} else {pui_native_263} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":151:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":152:7
                { let v_7075695f6e61746976655f323731 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323731.clone(), if (Value::boolean(v_6a756d70.clone())).truthy() { Value::string("dragging") } else { Value::string("armed") }, Value::string("reason: continuous control press")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:7
                if !({ let pui_native_272={ let pui_native_274=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_274) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_274} }; if (pui_native_272) {{ let pui_native_278={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_279=v_7469636b65745f5f65616431666461383165326338663337.clone(); (pui_native_278==pui_native_279) }} else {pui_native_272} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":154:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":154:16
                    { let v_7075695f6e61746976655f323830 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f323830.clone()).owner(), Value::string("interactionStart"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:7
                if !({ let pui_native_281={ let pui_native_283=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_283) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_283} }; if (pui_native_281) {{ let pui_native_287={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_288=v_7469636b65745f5f65616431666461383165326338663337.clone(); (pui_native_287==pui_native_288) }} else {pui_native_281} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":156:7
                if v_6a756d70.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":156:17
                    return (v_70726576696577.read().call)(v_63616e6469646174655f5f39383836346239656138306134303166.clone(), v_7469636b65745f5f65616431666461383165326338663337.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:16
                    { let v_7075695f6e61746976655f323839 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f323839.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":158:7
                return { let pui_native_290={ let pui_native_292=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_292) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_292} }; if (pui_native_290) {{ let pui_native_296={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_297=v_7469636b65745f5f65616431666461383165326338663337.clone(); (pui_native_296==pui_native_297) }} else {pui_native_290} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":160:11
            v_73616d706c65.initialize({  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726576696577 = v_70726576696577.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: f64| -> bool {
                let v_63616e6469646174655f5f66396364663334666239346264373932: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f66383430363439363131333634386431: f64 = argument_1;
                let v_64656c7461585f5f66393836616266333261623466386336: f64 = argument_2;
                let v_64656c7461595f5f31383033393639656133386138653663: f64 = argument_3;
                let v_66696e655f5f61653330373137313965666631323665: bool = argument_4;
                let v_7469636b65745f5f33326632303534323961666139656235: f64 = argument_5;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:7
                if { let pui_native_298={ let pui_native_300=!({ let pui_native_302={ let pui_native_304=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_304) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_304} }; if (pui_native_302) {{ let pui_native_308={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_309=v_7469636b65745f5f33326632303534323961666139656235.clone(); (pui_native_308==pui_native_309) }} else {pui_native_302} }); if !(pui_native_300) {!((v_656e61626c6564.read().call)())} else {pui_native_300} }; if !(pui_native_298) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333132 = v_7068617365.read(); (v_7075695f6e61746976655f333132.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_298} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:7
                if { let pui_native_313={ let pui_native_315={ let pui_native_317=!((v_63616e6469646174655f5f66396364663334666239346264373932.clone()).is_finite()); if !(pui_native_317) {!((v_66696e6543616e6469646174655f5f66383430363439363131333634386431.clone()).is_finite())} else {pui_native_317} }; if !(pui_native_315) {!((v_64656c7461585f5f66393836616266333261623466386336.clone()).is_finite())} else {pui_native_315} }; if !(pui_native_313) {!((v_64656c7461595f5f31383033393639656133386138653663.clone()).is_finite())} else {pui_native_313} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:68
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:78
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":166:7
                if { let pui_native_319=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333233 = v_7068617365.read(); (v_7075695f6e61746976655f333233.clone()).get() }), &(Value::string("armed"))))).bool(); if (pui_native_319) {{ let pui_native_324:f64={ let pui_native_326:f64={ let pui_native_328:f64=v_64656c7461585f5f66393836616266333261623466386336.clone(); let pui_native_329:f64=v_64656c7461585f5f66393836616266333261623466386336.clone(); pui_native_328 * pui_native_329 }; let pui_native_327:f64={ let pui_native_330:f64=v_64656c7461595f5f31383033393639656133386138653663.clone(); let pui_native_331:f64=v_64656c7461595f5f31383033393639656133386138653663.clone(); pui_native_330 * pui_native_331 }; pui_native_326 + pui_native_327 }; let pui_native_325:f64=9.0_f64; pui_native_324 < pui_native_325 }} else {pui_native_319} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":166:77
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":167:7
                { let v_7075695f6e61746976655f333332 = v_7068617365.read(); state_set(v_7075695f6e61746976655f333332.clone(), Value::string("dragging"), Value::string("reason: continuous control dragging")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":168:7
                if !({ let pui_native_333={ let pui_native_335=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_335) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_335} }; if (pui_native_333) {{ let pui_native_339={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_340=v_7469636b65745f5f33326632303534323961666139656235.clone(); (pui_native_339==pui_native_340) }} else {pui_native_333} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":168:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":169:11
                let v_6e6578745f5f38636639303065333661393637616433: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(if v_66696e655f5f61653330373137313965666631323665.clone() {v_66696e6543616e6469646174655f5f66383430363439363131333634386431.clone()} else {v_63616e6469646174655f5f66396364663334666239346264373932.clone()}));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":170:13
                let v_68616e646c655f5f65386438316263313465343037333631 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":171:7
                if { let pui_native_341=v_68616e646c655f5f65386438316263313465343037333631.clone(); if pui_native_341.truthy() {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f333435 = v_68616e646c655f5f65386438316263313465343037333631.clone(); read_props(&(v_7075695f6e61746976655f333435.clone()).owner(), false) }).member("dragMode", false)), &(Value::string("steps"))))).bool()} else {(pui_native_341).bool()} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":172:15
                    let v_616d6f756e745f5f34386662663932356662323831666535: f64 = if v_66696e655f5f61653330373137313965666631323665.clone() {(({ let v_7075695f6e61746976655f333438 = v_68616e646c655f5f65386438316263313465343037333631.clone(); read_props(&(v_7075695f6e61746976655f333438.clone()).owner(), false) }).member("fineStep", false)).num()} else {(({ let v_7075695f6e61746976655f333531 = v_68616e646c655f5f65386438316263313465343037333631.clone(); read_props(&(v_7075695f6e61746976655f333531.clone()).owner(), false) }).member("step", false)).num()};
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":175:15
                    let v_6d6f76656d656e74: f64 = if { let pui_native_352:f64=(Value::number((Value::number(v_64656c7461585f5f66393836616266333261623466386336.clone())).scalar_number().abs())).num(); let pui_native_353:f64=(Value::number((Value::number(v_64656c7461595f5f31383033393639656133386138653663.clone())).scalar_number().abs())).num(); pui_native_352 > pui_native_353 } {v_64656c7461585f5f66393836616266333261623466386336.clone()} else {-(v_64656c7461595f5f31383033393639656133386138653663.clone())};
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":176:9
                    Value::number({ let value={ let pui_native_354:f64=({ let v_7075695f6e61746976655f333536 = v_696e697469616c56616c7565.read(); Value::number(((v_7075695f6e61746976655f333536.clone()).get()).num()) }).num(); let pui_native_355:f64={ let pui_native_357:f64=(Value::number({ let input: f64 = (Value::number({ let pui_native_359:f64=v_6d6f76656d656e74.clone(); let pui_native_360:f64=4.0_f64; pui_native_359 / pui_native_360 })).scalar_number(); if !input.is_finite() || input == 0.0 || input.abs() >= 4503599627370496.0 { input } else if input >= -0.5 && input < 0.0 { -0.0 } else { let lower = input.floor(); if input - lower >= 0.5 { lower + 1.0 } else { lower } } })).num(); let pui_native_358:f64=v_616d6f756e745f5f34386662663932356662323831666535.clone(); pui_native_357 * pui_native_358 }; pui_native_354 + pui_native_355 }; *(v_6e6578745f5f38636639303065333661393637616433.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":178:7
                return (v_70726576696577.read().call)({ let value=(v_6e6578745f5f38636639303065333661393637616433.clone()).borrow().clone(); value }, v_7469636b65745f5f33326632303534323961666139656235.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":180:5
            { let v_7075695f6e61746976655f333631 = v_646566.clone(); expose_method(&owner, Value::string("sample"), ({  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c65 = v_73616d706c65.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_63616e6469646174655f5f39336366326163636462316530393864: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f63386339633230336563616665313334: f64 = argument_1;
                let v_64656c7461585f5f33366565323037643963643935326634: f64 = argument_2;
                let v_64656c7461595f5f36303139383239366236666538373137: f64 = argument_3;
                let v_66696e655f5f36663838663339393630336163303335: bool = argument_4;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":181:13
                let v_7469636b65745f5f65373766383865636334353035623262: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":182:7
                return (v_73616d706c65.read().call)(v_63616e6469646174655f5f39336366326163636462316530393864.clone(), v_66696e6543616e6469646174655f5f63386339633230336563616665313334.clone(), v_64656c7461585f5f33366565323037643963643935326634.clone(), v_64656c7461595f5f36303139383239366236666538373137.clone(), v_66696e655f5f36663838663339393630336163303335.clone(), v_7469636b65745f5f65373766383865636334353035623262.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":184:5
            { let v_7075695f6e61746976655f333632 = v_646566.clone(); expose_method(&owner, Value::string("finish"), ({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_73616d706c65 = v_73616d706c65.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_63616e6469646174655f5f38363461356430633064333164633733: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f34313036316530313962663234623861: f64 = argument_1;
                let v_64656c7461585f5f37353331323462353962383436396162: f64 = argument_2;
                let v_64656c7461595f5f38353335343265633938393838623533: f64 = argument_3;
                let v_66696e655f5f63363465666666613136643663663262: bool = argument_4;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":185:13
                let v_7469636b65745f5f65663231363737356565633832303237: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":186:7
                if { let pui_native_363=!((v_73616d706c65.read().call)(v_63616e6469646174655f5f38363461356430633064333164633733.clone(), v_66696e6543616e6469646174655f5f34313036316530313962663234623861.clone(), v_64656c7461585f5f37353331323462353962383436396162.clone(), v_64656c7461595f5f38353335343265633938393838623533.clone(), v_66696e655f5f63363465666666613136643663663262.clone(), v_7469636b65745f5f65663231363737356565633832303237.clone())); if !(pui_native_363) {!({ let pui_native_365={ let pui_native_367=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_367) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_367} }; if (pui_native_365) {{ let pui_native_371={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_372=v_7469636b65745f5f65663231363737356565633832303237.clone(); (pui_native_371==pui_native_372) }} else {pui_native_365} })} else {pui_native_363} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":186:96
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":187:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333735 = v_7068617365.read(); (v_7075695f6e61746976655f333735.clone()).get() }), &(Value::string("dragging"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":187:39
                    return (v_70726f706f7365.read().call)(({ let v_7075695f6e61746976655f333736 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f333736.clone()).get()).num()) }).num(), v_7469636b65745f5f65663231363737356565633832303237.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":188:13
                let v_616374696f6e = { let left = ({ let v_7075695f6e61746976655f333737 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f333737.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f333737.clone()).owner(), false) } }).member("tapAction", true); if left.nullish() { Value::string("none") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":189:7
                { let v_7075695f6e61746976655f333738 = v_7068617365.read(); state_set(v_7075695f6e61746976655f333738.clone(), Value::string("idle"), Value::string("reason: continuous control released")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":190:7
                if { let pui_native_379=!({ let pui_native_381={ let pui_native_383=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_383) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_383} }; if (pui_native_381) {{ let pui_native_387={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_388=v_7469636b65745f5f65663231363737356565633832303237.clone(); (pui_native_387==pui_native_388) }} else {pui_native_381} }); if !(pui_native_379) {!((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65663231363737356565633832303237.clone()))} else {pui_native_379} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":190:59
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":191:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean((Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate"))))).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":191:41
                    { let v_7075695f6e61746976655f333931 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f333931.clone()).owner(), Value::string("activate"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":192:7
                if (Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":192:32
                    return (v_70726f706f7365.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65663231363737356565633832303237.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":193:7
                if ({ let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean((Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate"))))).bool()) } else { left } }; if left.truthy() { Value::boolean((Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit"))))).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":193:64
                    { let v_7075695f6e61746976655f333938 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f333938.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":194:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":196:5
            { let v_7075695f6e61746976655f333939 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":197:5
            { let v_7075695f6e61746976655f343030 = v_646566.clone(); expose_method(&owner, Value::string("edit"), (v_65646974.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":198:5
            { let v_7075695f6e61746976655f343031 = v_646566.clone(); expose_method(&owner, Value::string("reset"), (v_7265736574.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":199:11
            v_61646a757374.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn(f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool| -> bool {
                let v_646972656374696f6e5f5f32613135663234346466336133313535: f64 = argument_0;
                let v_66696e655f5f33616363373361626131336530333664: bool = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":200:7
                if { let pui_native_402=!((v_656e61626c6564.read().call)()); if !(pui_native_402) {!((v_646972656374696f6e5f5f32613135663234346466336133313535.clone()).is_finite())} else {pui_native_402} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":200:54
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":201:13
                let v_7469636b65745f5f36636164663664393936396239633830: f64 = { let pui_native_404:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_405:f64=1.0_f64; pui_native_404 + pui_native_405 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":202:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:7
                if { let pui_native_406=!({ let pui_native_408={ let pui_native_410=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_410) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_410} }; if (pui_native_408) {{ let pui_native_414={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_415=v_7469636b65745f5f36636164663664393936396239633830.clone(); (pui_native_414==pui_native_415) }} else {pui_native_408} }); if !(pui_native_406) {!((v_656e61626c6564.read().call)())} else {pui_native_406} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":204:13
                let v_70726f70735f5f38353234323063636134356661343637 = { let v_7075695f6e61746976655f343136 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343136.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343136.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":205:13
                let v_616d6f756e745f5f62383430643364386432353632366364: f64 = if v_66696e655f5f33616363373361626131336530333664.clone() {{ let pui_native_417=(v_70726f70735f5f38353234323063636134356661343637.clone()).member("fineStep", true); if pui_native_417.nullish() {0.001_f64} else {(pui_native_417).num()} }} else {{ let pui_native_419=(v_70726f70735f5f38353234323063636134356661343637.clone()).member("step", true); if pui_native_419.nullish() {0.01_f64} else {(pui_native_419).num()} }};
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":206:7
                return (v_70726f706f7365.read().call)({ let pui_native_421:f64={ let value=(v_6163636570746564.read()).borrow().clone(); value }; let pui_native_422:f64={ let pui_native_423:f64=v_646972656374696f6e5f5f32613135663234346466336133313535.clone(); let pui_native_424:f64=v_616d6f756e745f5f62383430643364386432353632366364.clone(); pui_native_423 * pui_native_424 }; pui_native_421 + pui_native_422 }, v_7469636b65745f5f36636164663664393936396239633830.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":208:5
            { let v_7075695f6e61746976655f343235 = v_646566.clone(); expose_method(&owner, Value::string("adjust"), (v_61646a757374.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":209:5
            { let v_7075695f6e61746976655f343236 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-increment"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":209:59
                Value::boolean((v_61646a757374.read().call)(1.0_f64, false));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":210:5
            { let v_7075695f6e61746976655f343237 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-decrement"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":210:59
                Value::boolean((v_61646a757374.read().call)(-(1.0_f64), false));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":211:5
            { let v_7075695f6e61746976655f343238 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-scalar-set-value"), false, {  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f61643666373164336465636165303861: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":212:13
                let v_6576656e745f5f65343436366465333034656232383831 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":213:13
                let v_6e6578745f5f32336661353430633563303161666336: f64 = ((v_6576656e745f5f65343436366465333034656232383831.clone()).member("value", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":214:7
                if { let pui_native_429={ let pui_native_431=!((v_656e61626c6564.read().call)()); if !(pui_native_431) {(Value::boolean(!strict_equal(&(typeof_value(&(Value::number(v_6e6578745f5f32336661353430633563303161666336.clone())))), &(Value::string("number"))))).bool()} else {pui_native_431} }; if !(pui_native_429) {!((v_6e6578745f5f32336661353430633563303161666336.clone()).is_finite())} else {pui_native_429} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":214:77
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":215:13
                let v_7469636b65745f5f34333264396235353065306531663033: f64 = { let pui_native_435:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_436:f64=1.0_f64; pui_native_435 + pui_native_436 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":216:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":217:7
                if { let pui_native_437=!({ let pui_native_439={ let pui_native_441=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_441) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_441} }; if (pui_native_439) {{ let pui_native_445={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_446=v_7469636b65745f5f34333264396235353065306531663033.clone(); (pui_native_445==pui_native_446) }} else {pui_native_439} }); if !(pui_native_437) {!((v_656e61626c6564.read().call)())} else {pui_native_437} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":217:43
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":218:7
                Value::boolean((v_70726f706f7365.read().call)(v_6e6578745f5f32336661353430633563303161666336.clone(), v_7469636b65745f5f34333264396235353065306531663033.clone()));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":220:11
            v_6b6579.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); let v_61646a757374 = v_61646a757374.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_6e616d65: Value = argument_0;
                let v_7368696674: bool = argument_1;
                let v_636f6d6d616e64: bool = argument_2;
                let v_616c74: bool = argument_3;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":221:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Escape"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":222:15
                    let v_616374697665: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f343531 = v_7068617365.read(); (v_7075695f6e61746976655f343531.clone()).get() }), &(Value::string("idle"))))).bool();
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":223:9
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":223:19
                    return v_616374697665.clone();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":225:7
                if { let pui_native_452={ let pui_native_454=!((v_656e61626c6564.read().call)()); if !(pui_native_454) {v_616c74.clone()} else {pui_native_454} }; if !(pui_native_452) {v_636f6d6d616e64.clone()} else {pui_native_452} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":225:41
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":228:7
                if { let pui_native_456={ let pui_native_458={ let pui_native_460={ let pui_native_462={ let pui_native_464=(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f343638 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343638.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343638.clone()).owner(), false) } }).member("keyMode", true)), &(Value::string("table-point"))))).bool(); if (pui_native_464) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_464} }; if (pui_native_462) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_462} }; if (pui_native_460) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_460} }; if (pui_native_458) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_458} }; if (pui_native_456) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-"))))).bool()} else {pui_native_456} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":229:58
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":230:7
                if { let pui_native_479={ let pui_native_481={ let pui_native_483={ let pui_native_485={ let pui_native_487={ let pui_native_489={ let pui_native_491={ let pui_native_493={ let pui_native_495={ let pui_native_497={ let pui_native_499={ let pui_native_501=(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowLeft"))))).bool(); if (pui_native_501) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_501} }; if (pui_native_499) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_499} }; if (pui_native_497) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_497} }; if (pui_native_495) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp"))))).bool()} else {pui_native_495} }; if (pui_native_493) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown"))))).bool()} else {pui_native_493} }; if (pui_native_491) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool()} else {pui_native_491} }; if (pui_native_489) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("End"))))).bool()} else {pui_native_489} }; if (pui_native_487) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_487} }; if (pui_native_485) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_485} }; if (pui_native_483) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_483} }; if (pui_native_481) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_481} }; if (pui_native_479) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-"))))).bool()} else {pui_native_479} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":232:102
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":233:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":233:31
                    Value::boolean((v_65646974.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":233:39
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":234:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":234:35
                    Value::boolean((v_7265736574.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":234:44
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":235:7
                if { let pui_native_533=(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool(); if !(pui_native_533) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("End"))))).bool()} else {pui_native_533} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":236:15
                    let v_7469636b65745f5f31373862623064353863336566666534: f64 = { let pui_native_539:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_540:f64=1.0_f64; pui_native_539 + pui_native_540 };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":236:35
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":237:9
                    if { let pui_native_541={ let pui_native_543={ let pui_native_545=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_545) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_545} }; if (pui_native_543) {{ let pui_native_549={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_550=v_7469636b65745f5f31373862623064353863336566666534.clone(); (pui_native_549==pui_native_550) }} else {pui_native_543} }; if (pui_native_541) {(v_656e61626c6564.read().call)()} else {pui_native_541} } {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":237:43
                        Value::boolean((v_70726f706f7365.read().call)(if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool() {({ let v_7075695f6e61746976655f353533 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f353533.clone()).get()).num()) }).num()} else {({ let v_7075695f6e61746976655f353534 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f353534.clone()).get()).num()) }).num()}, v_7469636b65745f5f31373862623064353863336566666534.clone()));
                    }
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":238:9
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":240:13
                let v_646972656374696f6e5f5f64616434623038313734383763303131: f64 = if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp"))))).bool() {10.0_f64} else {if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown"))))).bool() {-(10.0_f64)} else {if { let pui_native_559={ let pui_native_561={ let pui_native_563=(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight"))))).bool(); if !(pui_native_563) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_563} }; if !(pui_native_561) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_561} }; if !(pui_native_559) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_559} } {1.0_f64} else {-(1.0_f64)}}};
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":242:7
                Value::boolean((v_61646a757374.read().call)(v_646972656374696f6e5f5f64616434623038313734383763303131.clone(), v_7368696674.clone()));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":243:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":245:5
            { let v_7075695f6e61746976655f353733 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b6579.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":246:5
            { let v_7075695f6e61746976655f353734 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f34313933383166636538303063383035: Value = arg(&arguments, 0);
                let v_6576656e745f5f31393134373664333562306235383165: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":247:7
                if (v_6b6579.read().call)({ let left = (v_6576656e745f5f31393134373664333562306235383165.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e745f5f31393134373664333562306235383165.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), { let pui_native_577=(Value::boolean(strict_equal(&((v_6576656e745f5f31393134373664333562306235383165.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_577) {(Value::boolean(strict_equal(&((v_6576656e745f5f31393134373664333562306235383165.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_577} }, (Value::boolean(strict_equal(&((v_6576656e745f5f31393134373664333562306235383165.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":247:131
                    { let v_7075695f6e61746976655f353835 = v_6576656e745f5f31393134373664333562306235383165.clone(); prevent(v_7075695f6e61746976655f353835.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":249:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_636c616d70 = v_636c616d70.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f36336438393036336165356662623232: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":250:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":250:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":251:13
                let v_70726f70735f5f36366335663130336633653436356335 = { let v_7075695f6e61746976655f353836 = v_68616e646c655f5f36336438393036336165356662623232.clone(); read_props(&(v_7075695f6e61746976655f353836.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":252:13
                let v_62617369735f5f65653461666539393933663764616433 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":253:13
                let v_70726576696f7573: f64 = { let value=(v_6163636570746564.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":254:13
                let v_706f6c6963794368616e676564: bool = { let pui_native_587={ let pui_native_589={ let pui_native_591={ let pui_native_593={ let pui_native_595={ let pui_native_597={ let value=(v_73746570506f6c696379.read()).borrow().clone(); value }; let pui_native_598=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("step", false)).num(); !(pui_native_597==pui_native_598) }; if !(pui_native_595) {{ let pui_native_601={ let value=(v_66696e65506f6c696379.read()).borrow().clone(); value }; let pui_native_602=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("fineStep", false)).num(); !(pui_native_601==pui_native_602) }} else {pui_native_595} }; if !(pui_native_593) {(Value::boolean(!strict_equal(&({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } })))).bool()} else {pui_native_593} }; if !(pui_native_591) {(Value::boolean(!strict_equal(&({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } })))).bool()} else {pui_native_591} }; if !(pui_native_589) {(Value::boolean(!strict_equal(&({ let value = (v_746170506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } })))).bool()} else {pui_native_589} }; if !(pui_native_587) {(Value::boolean(!strict_equal(&({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } })))).bool()} else {pui_native_587} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":258:13
                let v_6563686f65645f5f31396134653934313436323235633230: bool = { let pui_native_613={ let pui_native_615={ let pui_native_617={ let pui_native_619={ let pui_native_621={ let pui_native_623={ let pui_native_625={ let pui_native_627=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("previewEcho", false)).bool(); let pui_native_628=true; (pui_native_627==pui_native_628) }; if (pui_native_625) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f363331 = v_7068617365.read(); (v_7075695f6e61746976655f363331.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_625} }; if (pui_native_623) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_623} }; if (pui_native_621) {{ let pui_native_634=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("value", false)).num(); let pui_native_635=({ let v_7075695f6e61746976655f363336 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f363336.clone()).get()).num()) }).num(); (pui_native_634==pui_native_635) }} else {pui_native_621} }; if (pui_native_619) {{ let pui_native_637=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("min", false)).num(); let pui_native_638=({ let v_7075695f6e61746976655f363431 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f363431.clone()).get()).num()) }).num(); (pui_native_637==pui_native_638) }} else {pui_native_619} }; if (pui_native_617) {{ let pui_native_642=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("max", false)).num(); let pui_native_643=({ let v_7075695f6e61746976655f363436 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f363436.clone()).get()).num()) }).num(); (pui_native_642==pui_native_643) }} else {pui_native_617} }; if (pui_native_615) {{ let pui_native_647=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("disabled", false)).bool(); let pui_native_648=true; !(pui_native_647==pui_native_648) }} else {pui_native_615} }; if (pui_native_613) {!(v_706f6c6963794368616e676564.clone())} else {pui_native_613} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":265:7
                if !(v_6563686f65645f5f31396134653934313436323235633230.clone()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":265:20
                    Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":266:13
                let v_7469636b65745f5f64666237623462333064306437613839: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":267:7
                Value::boolean({ let value=true; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":268:7
                { let value = v_68616e646c655f5f36336438393036336165356662623232.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":269:7
                Value::number({ let value=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("step", false)).num(); *(v_73746570506f6c696379.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":269:40
                Value::number({ let value=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("fineStep", false)).num(); *(v_66696e65506f6c696379.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":270:7
                { let value = { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }; *(v_64726167506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":270:50
                { let value = { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }; *(v_646f75626c65506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":270:96
                { let value = { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }; *(v_746170506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":271:7
                { let value = { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }; *(v_6b6579626f617264506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":272:13
                let v_6c6f77: f64 = ((v_70726f70735f5f36366335663130336633653436356335.clone()).member("min", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":273:13
                let v_68696768: f64 = ((v_70726f70735f5f36366335663130336633653436356335.clone()).member("max", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":274:13
                let v_76616c6964: bool = { let pui_native_657={ let pui_native_659={ let pui_native_661={ let pui_native_663={ let pui_native_665=(v_6c6f77.clone()).is_finite(); if (pui_native_665) {(v_68696768.clone()).is_finite()} else {pui_native_665} }; if (pui_native_663) {{ let pui_native_667:f64=v_6c6f77.clone(); let pui_native_668:f64=v_68696768.clone(); pui_native_667 < pui_native_668 }} else {pui_native_663} }; if (pui_native_661) {(((v_70726f70735f5f36366335663130336633653436356335.clone()).member("value", false)).num()).is_finite()} else {pui_native_661} }; if (pui_native_659) {{ let pui_native_671:f64=v_6c6f77.clone(); let pui_native_672:f64=-(1000000000000000.0_f64); pui_native_671 >= pui_native_672 }} else {pui_native_659} }; if (pui_native_657) {{ let pui_native_673:f64=v_68696768.clone(); let pui_native_674:f64=1000000000000000.0_f64; pui_native_673 <= pui_native_674 }} else {pui_native_657} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":276:13
                let v_72616e67654368616e676564: bool = { let pui_native_675={ let pui_native_677=v_6c6f77.clone(); let pui_native_678=({ let v_7075695f6e61746976655f363739 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f363739.clone()).get()).num()) }).num(); !(pui_native_677==pui_native_678) }; if !(pui_native_675) {{ let pui_native_680=v_68696768.clone(); let pui_native_681=({ let v_7075695f6e61746976655f363832 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f363832.clone()).get()).num()) }).num(); !(pui_native_680==pui_native_681) }} else {pui_native_675} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":277:7
                { let v_7075695f6e61746976655f363833 = v_6d696e696d756d.read(); state_set(v_7075695f6e61746976655f363833.clone(), Value::number(if v_76616c6964.clone() {v_6c6f77.clone()} else {0.0_f64}), Value::string("reason: continuous control minimum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":278:7
                if !({ let pui_native_684={ let pui_native_686=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_686) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_686} }; if (pui_native_684) {{ let pui_native_690={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_691=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_690==pui_native_691) }} else {pui_native_684} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":278:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":279:7
                { let v_7075695f6e61746976655f363932 = v_6d6178696d756d.read(); state_set(v_7075695f6e61746976655f363932.clone(), Value::number(if v_76616c6964.clone() {v_68696768.clone()} else {1.0_f64}), Value::string("reason: continuous control maximum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":280:7
                if !({ let pui_native_693={ let pui_native_695=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_695) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_695} }; if (pui_native_693) {{ let pui_native_699={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_700=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_699==pui_native_700) }} else {pui_native_693} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":280:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":281:13
                let v_6e6578745f5f36383332346533313730393962653834: f64 = (v_636c616d70.read().call)(((v_70726f70735f5f36366335663130336633653436356335.clone()).member("value", false)).num());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":282:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":283:13
                let v_6563686f: bool = { let pui_native_703={ let pui_native_705={ let pui_native_707={ let pui_native_709=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("previewEcho", false)).bool(); let pui_native_710=true; (pui_native_709==pui_native_710) }; if (pui_native_707) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373133 = v_7068617365.read(); (v_7075695f6e61746976655f373133.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_707} }; if (pui_native_705) {(Value::boolean(strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_705} }; if (pui_native_703) {{ let pui_native_716=v_6e6578745f5f36383332346533313730393962653834.clone(); let pui_native_717=({ let v_7075695f6e61746976655f373138 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f373138.clone()).get()).num()) }).num(); (pui_native_716==pui_native_717) }} else {pui_native_703} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":285:13
                let v_73686f756c6443616e63656c: bool = { let pui_native_719=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373233 = v_7068617365.read(); (v_7075695f6e61746976655f373233.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_719) {{ let pui_native_724={ let pui_native_726={ let pui_native_728={ let pui_native_730={ let pui_native_732=!(v_76616c6964.clone()); if !(pui_native_732) {v_72616e67654368616e676564.clone()} else {pui_native_732} }; if !(pui_native_730) {v_706f6c6963794368616e676564.clone()} else {pui_native_730} }; if !(pui_native_728) {{ let pui_native_734=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("disabled", false)).bool(); let pui_native_735=true; (pui_native_734==pui_native_735) }} else {pui_native_728} }; if !(pui_native_726) {(Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_726} }; if !(pui_native_724) {{ let pui_native_738=!(v_6563686f.clone()); if (pui_native_738) {{ let pui_native_740=v_6e6578745f5f36383332346533313730393962653834.clone(); let pui_native_741={ let value=(v_6163636570746564.read()).borrow().clone(); value }; !(pui_native_740==pui_native_741) }} else {pui_native_738} }} else {pui_native_724} }} else {pui_native_719} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":287:7
                if !(v_6563686f.clone()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":287:18
                    Value::number({ let value=v_6e6578745f5f36383332346533313730393962653834.clone(); *(v_6163636570746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":288:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":289:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":290:9
                    { let v_7075695f6e61746976655f373432 = v_7068617365.read(); state_set(v_7075695f6e61746976655f373432.clone(), Value::string("idle"), Value::string("reason: continuous control authority changed")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":291:9
                    if !({ let pui_native_743={ let pui_native_745=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_745) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_745} }; if (pui_native_743) {{ let pui_native_749={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_750=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_749==pui_native_750) }} else {pui_native_743} }) {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":291:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":293:7
                { let v_7075695f6e61746976655f373531 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f373531.clone(), { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: continuous control label")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:7
                if !({ let pui_native_752={ let pui_native_754=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_754) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_754} }; if (pui_native_752) {{ let pui_native_758={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_759=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_758==pui_native_759) }} else {pui_native_752} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":295:7
                { let v_7075695f6e61746976655f373630 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f373630.clone(), { let left = (v_70726f70735f5f36366335663130336633653436356335.clone()).member("role", false); if left.nullish() { Value::string("slider") } else { left } }, Value::string("reason: continuous control role")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":296:7
                if !({ let pui_native_761={ let pui_native_763=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_763) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_763} }; if (pui_native_761) {{ let pui_native_767={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_768=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_767==pui_native_768) }} else {pui_native_761} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":296:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":297:7
                { let v_7075695f6e61746976655f373639 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f373639.clone(), Value::boolean({ let pui_native_770=!(v_76616c6964.clone()); if !(pui_native_770) {{ let pui_native_772=((v_70726f70735f5f36366335663130336633653436356335.clone()).member("disabled", false)).bool(); let pui_native_773=true; (pui_native_772==pui_native_773) }} else {pui_native_770} }), Value::string("reason: continuous control policy")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":298:7
                if !({ let pui_native_774={ let pui_native_776=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_776) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_776} }; if (pui_native_774) {{ let pui_native_780={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_781=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_780==pui_native_781) }} else {pui_native_774} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":298:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":299:7
                { let v_7075695f6e61746976655f373832 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f373832.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f373833 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f373833.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":300:7
                if !({ let pui_native_784={ let pui_native_786=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_786) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_786} }; if (pui_native_784) {{ let pui_native_790={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_791=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_790==pui_native_791) }} else {pui_native_784} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":300:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":301:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f373934 = v_7068617365.read(); (v_7075695f6e61746976655f373934.clone()).get() }), &(Value::string("idle"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":302:9
                    { let v_7075695f6e61746976655f373935 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f373935.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control authoritative display")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":303:9
                    if !({ let pui_native_796={ let pui_native_798=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_798) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_798} }; if (pui_native_796) {{ let pui_native_802={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_803=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_802==pui_native_803) }} else {pui_native_796} }) {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":303:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":305:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":305:25
                    { let v_7075695f6e61746976655f383034 = v_68616e646c655f5f36336438393036336165356662623232.clone(); emit(&(v_7075695f6e61746976655f383034.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number(v_70726576696f7573.clone())), ("revision", v_62617369735f5f65653461666539393933663764616433.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":306:7
                if !({ let pui_native_805={ let pui_native_807=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_807) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_807} }; if (pui_native_805) {{ let pui_native_811={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_812=v_7469636b65745f5f64666237623462333064306437613839.clone(); (pui_native_811==pui_native_812) }} else {pui_native_805} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":306:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":307:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":308:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":308:21
                    { let v_7075695f6e61746976655f383133 = v_68616e646c655f5f36336438393036336165356662623232.clone(); request_update(&(v_7075695f6e61746976655f383133.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let call:Rc<dyn Fn(bool)->Value>=Rc::new(move |argument_0: bool| -> Value {
                let v_7065726d616e656e74: bool = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":311:13
                let v_68616e646c655f5f38666363333032316461376139346532 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":312:13
                let v_776173416374697665: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f383136 = v_7068617365.read(); (v_7075695f6e61746976655f383136.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":313:13
                let v_62617369735f5f36363861353839323935326330643833 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":314:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":315:13
                let v_7469636b65745f5f38343532613537643236383337363133: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:24
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:42
                Value::boolean({ let value={ let pui_native_817={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_817) {v_7065726d616e656e74.clone()} else {pui_native_817} }; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:76
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":317:7
                if v_7065726d616e656e74.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":317:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":318:7
                { let v_7075695f6e61746976655f383139 = v_7068617365.read(); state_set(v_7075695f6e61746976655f383139.clone(), Value::string("idle"), Value::string("reason: continuous control detached")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:7
                if { let pui_native_820={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_821=v_7469636b65745f5f38343532613537643236383337363133.clone(); !(pui_native_820==pui_native_821) } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":320:7
                { let v_7075695f6e61746976655f383232 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f383232.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control detached display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":321:7
                if { let pui_native_823={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_824=v_7469636b65745f5f38343532613537643236383337363133.clone(); !(pui_native_823==pui_native_824) } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":321:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":322:7
                if ({ let left = Value::boolean(v_776173416374697665.clone()); if left.truthy() { v_68616e646c655f5f38666363333032316461376139346532.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":322:32
                    { let v_7075695f6e61746976655f383235 = v_68616e646c655f5f38666363333032316461376139346532.clone(); emit(&(v_7075695f6e61746976655f383235.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", v_62617369735f5f36363861353839323935326330643833.clone())])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":324:5
            { let v_7075695f6e61746976655f383236 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:5
            { let v_7075695f6e61746976655f383237 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f62653465376565396665643030613436: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:43
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:60
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":325:77
                    v_73796e63.read().call_authored(vec![v_68616e646c655f5f62653465376565396665643030613436.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":326:5
            { let v_7075695f6e61746976655f383238 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("value"), Value::string("min"), Value::string("max"), Value::string("step"), Value::string("fineStep"), Value::string("defaultValue"), Value::string("label"), Value::string("disabled"), Value::string("revision"), Value::string("role"), Value::string("dragMode"), Value::string("doubleAction"), Value::string("tapAction"), Value::string("previewEcho"), Value::string("keyMode")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":328:5
            { let v_7075695f6e61746976655f383239 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":329:5
            { let v_7075695f6e61746976655f383330 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":329:37
                return (v_726574697265.read().call)(false);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":330:5
            { let v_7075695f6e61746976655f383331 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":330:41
                return (v_726574697265.read().call)(true);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":331:5
            return { let callable={ let pui_native_832={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":331:26
                return { let v_7075695f6e61746976655f383333 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_832.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_832.call)(arg(&arguments,0))})} }; match callable {
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
