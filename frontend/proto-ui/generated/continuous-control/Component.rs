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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("value").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("min").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("max").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("step").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("fineStep").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("defaultValue").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("dragMode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("doubleAction").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("tapAction").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("previewEcho").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("keyMode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("trackPress").map_or(true, |prop| prop.is_null() || prop.is_string()) }
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
        "keyMode" => value.is_string(),
        "trackPress" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "beginTrack" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "trackPress", default, skip_serializing_if = "Option::is_none")]
    pub field_15: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct ContinuousControlHandle { pub owner: OwnerRef }
  impl ContinuousControlHandle {
      pub fn state_0(&self) -> Result<f64, Value> { self.state("value").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("initialValue").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("dragOrigin").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_6(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_13(&self, argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_14(&self, argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("beginTrack").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("beginTrack", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_15(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_16(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_17(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_18(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_19(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_20(&self, argument_0: f64, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_21(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
            let v_647261674f726967696e: LexicalBinding<Value> = lexical_binding();
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
            let v_747261636b506f6c696379: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_656e61626c6564: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_636c616d70: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> f64>> = lexical_binding();
            let v_7075626c697368: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_63616e63656c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_70726576696577: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_65646974: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_7265736574: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_626567696e: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool, f64, bool, bool) -> bool>> = lexical_binding();
            let v_73616d706c65: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64, f64, bool, f64) -> bool>> = lexical_binding();
            let v_61646a757374: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool) -> bool>> = lexical_binding();
            let v_6b6579: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool, bool, bool) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(bool) -> Value>> = lexical_binding();
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":18:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("value", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("min", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("max", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("step", record(vec![("type", Value::string("number")), ("default", Value::number(0.01_f64))])), ("fineStep", record(vec![("type", Value::string("number")), ("default", Value::number(0.001_f64))])), ("defaultValue", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("slider"))])), ("dragMode", record(vec![("type", Value::string("string")), ("default", Value::string("absolute"))])), ("doubleAction", record(vec![("type", Value::string("string")), ("default", Value::string("reset"))])), ("tapAction", record(vec![("type", Value::string("string")), ("default", Value::string("none"))])), ("previewEcho", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("keyMode", record(vec![("type", Value::string("string")), ("default", Value::string("scalar"))])), ("trackPress", record(vec![("type", Value::string("string")), ("default", Value::string("jump"))]))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":28:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":29:11
            v_76616c7565.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("value"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":30:11
            v_696e697469616c56616c7565.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "range", Value::string("initialValue"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":32:11
            v_647261674f726967696e.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "range", Value::string("dragOrigin"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":33:11
            v_6d696e696d756d.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "range", Value::string("minimum"), Value::number(0.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":34:11
            v_6d6178696d756d.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "range", Value::string("maximum"), Value::number(1.0_f64), record(vec![("min", Value::number(-(1000000000000000.0_f64))), ("max", Value::number(1000000000000000.0_f64))])) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":35:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":36:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":37:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("slider"), Value::Undefined) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":38:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":39:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3130 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3131 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f3131.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3132 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f3132.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3133 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("valueMin"), v_6d696e696d756d.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3134 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("valueMax"), v_6d6178696d756d.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3135 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("valueNow"), v_76616c7565.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3136 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3137 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("increment"), record(vec![("event", Value::string("host:verdant-scalar-increment"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":48:5
            { let v_7075695f6e61746976655f3138 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("decrement"), record(vec![("event", Value::string("host:verdant-scalar-decrement"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":49:5
            { let v_7075695f6e61746976655f3139 = v_61636365737369626c65.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("setValue"), record(vec![("event", Value::string("host:verdant-scalar-set-value"))])) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":52:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-w-0".into(), "min-w-".into()), ("min-h-0".into(), "min-h-".into())])]) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":53:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("value"), v_76616c7565.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":54:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("initialValue"), v_696e697469616c56616c7565.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":55:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("dragOrigin"), v_647261674f726967696e.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":56:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":57:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":58:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":59:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":60:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_event(&owner, Value::string("interactionStart")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":61:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("valuePreview")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":62:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("valueCommit")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":63:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":64:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":65:5
            { let v_7075695f6e61746976655f3333 = v_646566.clone(); expose_event(&owner, Value::string("editRequest")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":67:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":68:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":69:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":70:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":71:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":72:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":73:9
            v_6163636570746564.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":74:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":75:9
            v_73746570506f6c696379.initialize(Rc::new(std::cell::RefCell::new(0.01_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":75:32
            v_66696e65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(0.001_f64)));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:9
            v_64726167506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("absolute"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:38
            v_646f75626c65506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("reset"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:66
            v_746170506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("none"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":76:90
            v_6b6579626f617264506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("scalar"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":77:9
            v_747261636b506f6c696379.initialize(Rc::new(std::cell::RefCell::new(Value::string("jump"))));
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:11
            v_656e61626c6564.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":80:36
                return { let pui_native_34={ let pui_native_36={ let pui_native_38={ let pui_native_40=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_40) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_40} }; if (pui_native_38) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_38} }; if (pui_native_36) {!({ let value=(v_73796e63696e67.read()).borrow().clone(); value })} else {pui_native_36} }; if (pui_native_34) {!(({ let v_7075695f6e61746976655f3434 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3434.clone()).get()).bool()) }).bool())} else {pui_native_34} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":81:11
            v_636c616d70.initialize({  let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let call:Rc<dyn Fn(f64)->f64>=Rc::new(move |argument_0: f64| -> f64 {
                let v_6e6578745f5f64636330346235633530396164646132: f64 = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":82:7
                if !((v_6e6578745f5f64636330346235633530396164646132.clone()).is_finite()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":82:35
                    return ({ let v_7075695f6e61746976655f3435 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3435.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":83:7
                if { let pui_native_46:f64=v_6e6578745f5f64636330346235633530396164646132.clone(); let pui_native_47:f64=({ let v_7075695f6e61746976655f3438 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3438.clone()).get()).num()) }).num(); pui_native_46 < pui_native_47 } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":83:33
                    return ({ let v_7075695f6e61746976655f3439 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f3439.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:7
                if { let pui_native_50:f64=v_6e6578745f5f64636330346235633530396164646132.clone(); let pui_native_51:f64=({ let v_7075695f6e61746976655f3532 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f3532.clone()).get()).num()) }).num(); pui_native_50 > pui_native_51 } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":84:33
                    return ({ let v_7075695f6e61746976655f3533 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f3533.clone()).get()).num()) }).num();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":85:7
                return v_6e6578745f5f64636330346235633530396164646132.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":87:11
            v_7075626c697368.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_636c616d70 = v_636c616d70.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f61623566633132376635386164396630: f64 = argument_0;
                let v_7469636b65745f5f32306336653332396162346535656238: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":88:7
                if !({ let pui_native_54={ let pui_native_56=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_56) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_56} }; if (pui_native_54) {{ let pui_native_60={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_61=v_7469636b65745f5f32306336653332396162346535656238.clone(); (pui_native_60==pui_native_61) }} else {pui_native_54} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":88:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":89:7
                { let v_7075695f6e61746976655f3632 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f3632.clone(), Value::number((v_636c616d70.read().call)(v_6e6578745f5f61623566633132376635386164396630.clone())), Value::string("reason: continuous control display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":90:7
                if !({ let pui_native_63={ let pui_native_65=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_65) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_65} }; if (pui_native_63) {{ let pui_native_69={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_70=v_7469636b65745f5f32306336653332396162346535656238.clone(); (pui_native_69==pui_native_70) }} else {pui_native_63} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":90:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":91:7
                if { let pui_native_71={ let value = (v_72756e.read()).borrow().clone(); value }; if pui_native_71.truthy() {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {(pui_native_71).bool()} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":91:28
                    { let v_7075695f6e61746976655f3733 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3733.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":92:7
                return { let pui_native_74={ let pui_native_76=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_76) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_76} }; if (pui_native_74) {{ let pui_native_80={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_81=v_7469636b65745f5f32306336653332396162346535656238.clone(); (pui_native_80==pui_native_81) }} else {pui_native_74} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":94:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":95:13
                let v_7469636b65745f5f31323661353863303837636432623864: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:7
                if { let pui_native_82=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_82) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3836 = v_7068617365.read(); (v_7075695f6e61746976655f3836.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_82} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":96:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":97:13
                let v_62617369735f5f39306538306565653035313162323461 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":98:7
                { let v_7075695f6e61746976655f3837 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3837.clone(), Value::string("idle"), Value::string("reason: continuous control canceled")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":99:7
                if { let pui_native_88=!({ let pui_native_90={ let pui_native_92=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_92) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_92} }; if (pui_native_90) {{ let pui_native_96={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_97=v_7469636b65745f5f31323661353863303837636432623864.clone(); (pui_native_96==pui_native_97) }} else {pui_native_90} }); if !(pui_native_88) {!((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f31323661353863303837636432623864.clone()))} else {pui_native_88} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":99:59
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":100:16
                    { let v_7075695f6e61746976655f3938 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3938.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", v_62617369735f5f39306538306565653035313162323461.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":101:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":103:11
            v_70726f706f7365.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_747261636b506f6c696379 = v_747261636b506f6c696379.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f62343662663233376363316334333331: f64 = argument_0;
                let v_7469636b65745f5f65383436633463323735663931313234: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":104:7
                if { let pui_native_99={ let pui_native_101=!({ let pui_native_103={ let pui_native_105=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_105) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_105} }; if (pui_native_103) {{ let pui_native_109={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_110=v_7469636b65745f5f65383436633463323735663931313234.clone(); (pui_native_109==pui_native_110) }} else {pui_native_103} }); if !(pui_native_101) {!((v_656e61626c6564.read().call)())} else {pui_native_101} }; if !(pui_native_99) {!((v_6e6578745f5f62343662663233376363316334333331.clone()).is_finite())} else {pui_native_99} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":104:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":105:13
                let v_70726f706f73616c: f64 = (v_636c616d70.read().call)(v_6e6578745f5f62343662663233376363316334333331.clone());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":106:13
                let v_62617369735f5f62313566383638363966376533663038 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":107:13
                let v_70726f70735f5f35303534303738343034346461323933 = { let v_7075695f6e61746976655f313131 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f313131.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f313131.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":112:13
                let v_6563686f65645f5f61376230656536353038383461346135: bool = { let pui_native_112={ let pui_native_114={ let pui_native_116={ let pui_native_118={ let pui_native_120={ let pui_native_122={ let pui_native_124={ let pui_native_126={ let pui_native_128={ let pui_native_130={ let pui_native_132={ let pui_native_134={ let pui_native_136=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313430 = v_7068617365.read(); (v_7075695f6e61746976655f313430.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_136) {(Value::boolean(strict_equal(&((v_70726f70735f5f35303534303738343034346461323933.clone()).member("previewEcho", true)), &(Value::boolean(true))))).bool()} else {pui_native_136} }; if (pui_native_134) {{ let pui_native_143=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("value", false)).num(); let pui_native_144=v_70726f706f73616c.clone(); (pui_native_143==pui_native_144) }} else {pui_native_134} }; if (pui_native_132) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &(v_62617369735f5f62313566383638363966376533663038.clone())))).bool()} else {pui_native_132} }; if (pui_native_130) {{ let pui_native_147=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("min", false)).num(); let pui_native_148=({ let v_7075695f6e61746976655f313531 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f313531.clone()).get()).num()) }).num(); (pui_native_147==pui_native_148) }} else {pui_native_130} }; if (pui_native_128) {{ let pui_native_152=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("max", false)).num(); let pui_native_153=({ let v_7075695f6e61746976655f313536 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f313536.clone()).get()).num()) }).num(); (pui_native_152==pui_native_153) }} else {pui_native_128} }; if (pui_native_126) {{ let pui_native_157=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("disabled", false)).bool(); let pui_native_158=true; !(pui_native_157==pui_native_158) }} else {pui_native_126} }; if (pui_native_124) {{ let pui_native_159=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("step", false)).num(); let pui_native_160={ let value=(v_73746570506f6c696379.read()).borrow().clone(); value }; (pui_native_159==pui_native_160) }} else {pui_native_124} }; if (pui_native_122) {{ let pui_native_163=((v_70726f70735f5f35303534303738343034346461323933.clone()).member("fineStep", false)).num(); let pui_native_164={ let value=(v_66696e65506f6c696379.read()).borrow().clone(); value }; (pui_native_163==pui_native_164) }} else {pui_native_122} }; if (pui_native_120) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }), &({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_120} }; if (pui_native_118) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }), &({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_118} }; if (pui_native_116) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }), &({ let value = (v_746170506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_116} }; if (pui_native_114) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }), &({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_114} }; if (pui_native_112) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f35303534303738343034346461323933.clone()).member("trackPress", false); if left.nullish() { Value::string("jump") } else { left } }), &({ let value = (v_747261636b506f6c696379.read()).borrow().clone(); value })))).bool()} else {pui_native_112} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":119:7
                { let v_7075695f6e61746976655f313737 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313737.clone(), Value::string("idle"), Value::string("reason: continuous control proposal")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":120:7
                if !({ let pui_native_178={ let pui_native_180=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_180) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_180} }; if (pui_native_178) {{ let pui_native_184={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_185=v_7469636b65745f5f65383436633463323735663931313234.clone(); (pui_native_184==pui_native_185) }} else {pui_native_178} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":120:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":121:7
                if v_6563686f65645f5f61376230656536353038383461346135.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":121:19
                    Value::number({ let value=v_70726f706f73616c.clone(); *(v_6163636570746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":122:7
                if !((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f65383436633463323735663931313234.clone())) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":122:39
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":123:16
                    { let v_7075695f6e61746976655f313836 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313836.clone()).owner(), Value::string("valueCommit"), record(vec![("value", Value::number(v_70726f706f73616c.clone())), ("revision", v_62617369735f5f62313566383638363966376533663038.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":124:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":126:11
            v_70726576696577.initialize({  let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_6e6578745f5f30633237393064313538646633613830: f64 = argument_0;
                let v_7469636b65745f5f35366262623163313863386130653830: f64 = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":127:7
                if { let pui_native_187={ let pui_native_189=!({ let pui_native_191={ let pui_native_193=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_193) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_193} }; if (pui_native_191) {{ let pui_native_197={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_198=v_7469636b65745f5f35366262623163313863386130653830.clone(); (pui_native_197==pui_native_198) }} else {pui_native_191} }); if !(pui_native_189) {!((v_656e61626c6564.read().call)())} else {pui_native_189} }; if !(pui_native_187) {!((v_6e6578745f5f30633237393064313538646633613830.clone()).is_finite())} else {pui_native_187} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":127:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":128:7
                if !((v_7075626c697368.read().call)(v_6e6578745f5f30633237393064313538646633613830.clone(), v_7469636b65745f5f35366262623163313863386130653830.clone())) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":128:35
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":129:16
                    { let v_7075695f6e61746976655f313939 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313939.clone()).owner(), Value::string("valuePreview"), record(vec![("value", Value::number(({ let v_7075695f6e61746976655f323030 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f323030.clone()).get()).num()) }).num())), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":130:7
                return { let pui_native_201={ let pui_native_203=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_203) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_203} }; if (pui_native_201) {{ let pui_native_207={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_208=v_7469636b65745f5f35366262623163313863386130653830.clone(); (pui_native_207==pui_native_208) }} else {pui_native_201} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":132:11
            v_65646974.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:7
                if !((v_656e61626c6564.read().call)()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":133:23
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":134:13
                let v_7469636b65745f5f39383438306463633765663263623166: f64 = { let pui_native_209:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_210:f64=1.0_f64; pui_native_209 + pui_native_210 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":135:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":136:7
                if { let pui_native_211=!({ let pui_native_213={ let pui_native_215=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_215) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_215} }; if (pui_native_213) {{ let pui_native_219={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_220=v_7469636b65745f5f39383438306463633765663263623166.clone(); (pui_native_219==pui_native_220) }} else {pui_native_213} }); if !(pui_native_211) {!((v_656e61626c6564.read().call)())} else {pui_native_211} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":136:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":137:16
                    { let v_7075695f6e61746976655f323231 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f323231.clone()).owner(), Value::string("editRequest"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":138:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":140:11
            v_7265736574.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":141:7
                if !((v_656e61626c6564.read().call)()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":141:23
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":142:13
                let v_7469636b65745f5f32653665393431626566653532333861: f64 = { let pui_native_222:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_223:f64=1.0_f64; pui_native_222 + pui_native_223 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":143:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:7
                if { let pui_native_224=!({ let pui_native_226={ let pui_native_228=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_228) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_228} }; if (pui_native_226) {{ let pui_native_232={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_233=v_7469636b65745f5f32653665393431626566653532333861.clone(); (pui_native_232==pui_native_233) }} else {pui_native_226} }); if !(pui_native_224) {!((v_656e61626c6564.read().call)())} else {pui_native_224} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":144:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":145:7
                return (v_70726f706f7365.read().call)({ let pui_native_234=({ let v_7075695f6e61746976655f323336 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323336.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323336.clone()).owner(), false) } }).member("defaultValue", true); if pui_native_234.nullish() {{ let value=(v_6163636570746564.read()).borrow().clone(); value }} else {(pui_native_234).num()} }, v_7469636b65745f5f32653665393431626566653532333861.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":147:11
            v_626567696e.initialize({  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_647261674f726967696e = v_647261674f726967696e.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_70726576696577 = v_70726576696577.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); let call:Rc<dyn Fn(f64, bool, f64, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: f64, argument_3: bool, argument_4: bool| -> bool {
                let v_63616e6469646174655f5f38656538643863656230393535356262: f64 = argument_0;
                let v_6a756d705f5f66323031376661333263396237326462: bool = argument_1;
                let v_6f726967696e: f64 = argument_2;
                let v_72657365744d6f6469666965725f5f36376536326239343937666439353766: bool = argument_3;
                let v_646f75626c655f5f34333034346164316534363466343031: bool = argument_4;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":148:7
                if { let pui_native_237={ let pui_native_239=!((v_656e61626c6564.read().call)()); if !(pui_native_239) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f323433 = v_7068617365.read(); (v_7075695f6e61746976655f323433.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_239} }; if !(pui_native_237) {!((v_63616e6469646174655f5f38656538643863656230393535356262.clone()).is_finite())} else {pui_native_237} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":148:80
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:7
                if { let pui_native_244=v_646f75626c655f5f34333034346164316534363466343031.clone(); if (pui_native_244) {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f323438 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323438.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323438.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("edit"))))).bool()} else {pui_native_244} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":149:63
                    return (v_65646974.read().call)();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":150:7
                if { let pui_native_249=v_72657365744d6f6469666965725f5f36376536326239343937666439353766.clone(); if !(pui_native_249) {{ let pui_native_251=v_646f75626c655f5f34333034346164316534363466343031.clone(); if (pui_native_251) {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f323535 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f323535.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f323535.clone()).owner(), false) } }).member("doubleAction", true)), &(Value::string("reset"))))).bool()} else {pui_native_251} }} else {pui_native_249} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":150:83
                    return (v_7265736574.read().call)();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":151:13
                let v_7469636b65745f5f33373137653134303164373761333537: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":152:7
                { let v_7075695f6e61746976655f323536 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323536.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:7
                if { let pui_native_257=!({ let pui_native_259={ let pui_native_261=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_261) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_261} }; if (pui_native_259) {{ let pui_native_265={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_266=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_265==pui_native_266) }} else {pui_native_259} }); if !(pui_native_257) {!((v_656e61626c6564.read().call)())} else {pui_native_257} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":153:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":154:7
                { let v_7075695f6e61746976655f323637 = v_696e697469616c56616c7565.read(); state_set(v_7075695f6e61746976655f323637.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control starting authority")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:7
                if !({ let pui_native_268={ let pui_native_270=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_270) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_270} }; if (pui_native_268) {{ let pui_native_274={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_275=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_274==pui_native_275) }} else {pui_native_268} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":155:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":156:7
                { let v_7075695f6e61746976655f323736 = v_647261674f726967696e.read(); state_set(v_7075695f6e61746976655f323736.clone(), Value::number(v_6f726967696e.clone()), Value::string("reason: continuous control starting drag origin")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:7
                if !({ let pui_native_277={ let pui_native_279=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_279) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_279} }; if (pui_native_277) {{ let pui_native_283={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_284=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_283==pui_native_284) }} else {pui_native_277} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":157:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":158:7
                { let v_7075695f6e61746976655f323835 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323835.clone(), if (Value::boolean(v_6a756d705f5f66323031376661333263396237326462.clone())).truthy() { Value::string("dragging") } else { Value::string("armed") }, Value::string("reason: continuous control press")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":159:7
                if !({ let pui_native_286={ let pui_native_288=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_288) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_288} }; if (pui_native_286) {{ let pui_native_292={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_293=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_292==pui_native_293) }} else {pui_native_286} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":159:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":160:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":160:16
                    { let v_7075695f6e61746976655f323934 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f323934.clone()).owner(), Value::string("interactionStart"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:7
                if !({ let pui_native_295={ let pui_native_297=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_297) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_297} }; if (pui_native_295) {{ let pui_native_301={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_302=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_301==pui_native_302) }} else {pui_native_295} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":161:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:7
                if v_6a756d705f5f66323031376661333263396237326462.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":162:17
                    return (v_70726576696577.read().call)(v_63616e6469646174655f5f38656538643863656230393535356262.clone(), v_7469636b65745f5f33373137653134303164373761333537.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":163:16
                    { let v_7075695f6e61746976655f333033 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f333033.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":164:7
                return { let pui_native_304={ let pui_native_306=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_306) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_306} }; if (pui_native_304) {{ let pui_native_310={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_311=v_7469636b65745f5f33373137653134303164373761333537.clone(); (pui_native_310==pui_native_311) }} else {pui_native_304} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).num(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":166:5
            { let v_7075695f6e61746976655f333132 = v_646566.clone(); expose_method(&owner, Value::string("begin"), ({  let v_6163636570746564 = v_6163636570746564.clone(); let v_626567696e = v_626567696e.clone(); let call:Rc<dyn Fn(f64, bool, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_63616e6469646174655f5f62333165333339373462383462636137: f64 = argument_0;
                let v_6a756d705f5f31643533393538323964663365326565: bool = argument_1;
                let v_72657365744d6f6469666965725f5f64356538653831373736333636393030: bool = argument_2;
                let v_646f75626c655f5f35666665316238366237386164393566: bool = argument_3;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":167:7
                return (v_626567696e.read().call)(v_63616e6469646174655f5f62333165333339373462383462636137.clone(), v_6a756d705f5f31643533393538323964663365326565.clone(), { let value=(v_6163636570746564.read()).borrow().clone(); value }, v_72657365744d6f6469666965725f5f64356538653831373736333636393030.clone(), v_646f75626c655f5f35666665316238366237386164393566.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":169:5
            { let v_7075695f6e61746976655f333133 = v_646566.clone(); expose_method(&owner, Value::string("beginTrack"), ({  let v_72756e = v_72756e.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_626567696e = v_626567696e.clone(); let call:Rc<dyn Fn(f64, bool, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_63616e6469646174655f5f61313266326463336632366437303733: f64 = argument_0;
                let v_7468756d62486974: bool = argument_1;
                let v_72657365744d6f6469666965725f5f31303932323832666534613031386439: bool = argument_2;
                let v_646f75626c655f5f32376539616232313139353437313261: bool = argument_3;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":170:7
                if (Value::boolean(!strict_equal(&(typeof_value(&(Value::boolean(v_7468756d62486974.clone())))), &(Value::string("boolean"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":170:42
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":173:13
                let v_6a756d705f5f32613262303131366562396432363363: bool = { let pui_native_316=!(v_7468756d62486974.clone()); if (pui_native_316) {(Value::boolean(strict_equal(&({ let left = ({ let v_7075695f6e61746976655f333230 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f333230.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f333230.clone()).owner(), false) } }).member("trackPress", true); if left.nullish() { Value::string("jump") } else { left } }), &(Value::string("jump"))))).bool()} else {pui_native_316} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":174:7
                return (v_626567696e.read().call)(v_63616e6469646174655f5f61313266326463336632366437303733.clone(), v_6a756d705f5f32613262303131366562396432363363.clone(), if v_6a756d705f5f32613262303131366562396432363363.clone() {(v_636c616d70.read().call)(v_63616e6469646174655f5f61313266326463336632366437303733.clone())} else {{ let value=(v_6163636570746564.read()).borrow().clone(); value }}, v_72657365744d6f6469666965725f5f31303932323832666534613031386439.clone(), v_646f75626c655f5f32376539616232313139353437313261.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":176:11
            v_73616d706c65.initialize({  let v_7068617365 = v_7068617365.clone(); let v_696e697469616c56616c7565 = v_696e697469616c56616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726576696577 = v_70726576696577.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: f64| -> bool {
                let v_63616e6469646174655f5f31343537373039363066653566366233: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f31343033633365386233646230663433: f64 = argument_1;
                let v_64656c7461585f5f34383237393936306631613738326632: f64 = argument_2;
                let v_64656c7461595f5f39393130333861376133363832633764: f64 = argument_3;
                let v_66696e655f5f66663535666165333363313235316563: bool = argument_4;
                let v_7469636b65745f5f36643764363031623333363563666435: f64 = argument_5;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":177:7
                if { let pui_native_321={ let pui_native_323=!({ let pui_native_325={ let pui_native_327=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_327) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_327} }; if (pui_native_325) {{ let pui_native_331={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_332=v_7469636b65745f5f36643764363031623333363563666435.clone(); (pui_native_331==pui_native_332) }} else {pui_native_325} }); if !(pui_native_323) {!((v_656e61626c6564.read().call)())} else {pui_native_323} }; if !(pui_native_321) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333335 = v_7068617365.read(); (v_7075695f6e61746976655f333335.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_321} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":177:69
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":178:7
                if { let pui_native_336={ let pui_native_338={ let pui_native_340=!((v_63616e6469646174655f5f31343537373039363066653566366233.clone()).is_finite()); if !(pui_native_340) {!((v_66696e6543616e6469646174655f5f31343033633365386233646230663433.clone()).is_finite())} else {pui_native_340} }; if !(pui_native_338) {!((v_64656c7461585f5f34383237393936306631613738326632.clone()).is_finite())} else {pui_native_338} }; if !(pui_native_336) {!((v_64656c7461595f5f39393130333861376133363832633764.clone()).is_finite())} else {pui_native_336} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":179:68
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":179:78
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":182:7
                if { let pui_native_342=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333436 = v_7068617365.read(); (v_7075695f6e61746976655f333436.clone()).get() }), &(Value::string("armed"))))).bool(); if (pui_native_342) {{ let pui_native_347:f64={ let pui_native_349:f64={ let pui_native_351:f64=v_64656c7461585f5f34383237393936306631613738326632.clone(); let pui_native_352:f64=v_64656c7461585f5f34383237393936306631613738326632.clone(); pui_native_351 * pui_native_352 }; let pui_native_350:f64={ let pui_native_353:f64=v_64656c7461595f5f39393130333861376133363832633764.clone(); let pui_native_354:f64=v_64656c7461595f5f39393130333861376133363832633764.clone(); pui_native_353 * pui_native_354 }; pui_native_349 + pui_native_350 }; let pui_native_348:f64=9.0_f64; pui_native_347 < pui_native_348 }} else {pui_native_342} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":182:77
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":183:7
                { let v_7075695f6e61746976655f333535 = v_7068617365.read(); state_set(v_7075695f6e61746976655f333535.clone(), Value::string("dragging"), Value::string("reason: continuous control dragging")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":184:7
                if !({ let pui_native_356={ let pui_native_358=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_358) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_358} }; if (pui_native_356) {{ let pui_native_362={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_363=v_7469636b65745f5f36643764363031623333363563666435.clone(); (pui_native_362==pui_native_363) }} else {pui_native_356} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":184:29
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":185:11
                let v_6e6578745f5f62393732373238633736316464316132: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(if v_66696e655f5f66663535666165333363313235316563.clone() {v_66696e6543616e6469646174655f5f31343033633365386233646230663433.clone()} else {v_63616e6469646174655f5f31343537373039363066653566366233.clone()}));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":186:13
                let v_68616e646c655f5f38393166343032643964346632643433 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":187:7
                if { let pui_native_364=v_68616e646c655f5f38393166343032643964346632643433.clone(); if pui_native_364.truthy() {(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f333638 = v_68616e646c655f5f38393166343032643964346632643433.clone(); read_props(&(v_7075695f6e61746976655f333638.clone()).owner(), false) }).member("dragMode", false)), &(Value::string("steps"))))).bool()} else {(pui_native_364).bool()} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":188:15
                    let v_616d6f756e745f5f33383034313063353739363162313066: f64 = if v_66696e655f5f66663535666165333363313235316563.clone() {(({ let v_7075695f6e61746976655f333731 = v_68616e646c655f5f38393166343032643964346632643433.clone(); read_props(&(v_7075695f6e61746976655f333731.clone()).owner(), false) }).member("fineStep", false)).num()} else {(({ let v_7075695f6e61746976655f333734 = v_68616e646c655f5f38393166343032643964346632643433.clone(); read_props(&(v_7075695f6e61746976655f333734.clone()).owner(), false) }).member("step", false)).num()};
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":191:15
                    let v_6d6f76656d656e74: f64 = if { let pui_native_375:f64=(Value::number((Value::number(v_64656c7461585f5f34383237393936306631613738326632.clone())).scalar_number().abs())).num(); let pui_native_376:f64=(Value::number((Value::number(v_64656c7461595f5f39393130333861376133363832633764.clone())).scalar_number().abs())).num(); pui_native_375 > pui_native_376 } {v_64656c7461585f5f34383237393936306631613738326632.clone()} else {-(v_64656c7461595f5f39393130333861376133363832633764.clone())};
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":192:9
                    Value::number({ let value={ let pui_native_377:f64=({ let v_7075695f6e61746976655f333739 = v_696e697469616c56616c7565.read(); Value::number(((v_7075695f6e61746976655f333739.clone()).get()).num()) }).num(); let pui_native_378:f64={ let pui_native_380:f64=(Value::number({ let input: f64 = (Value::number({ let pui_native_382:f64=v_6d6f76656d656e74.clone(); let pui_native_383:f64=4.0_f64; pui_native_382 / pui_native_383 })).scalar_number(); if !input.is_finite() || input == 0.0 || input.abs() >= 4503599627370496.0 { input } else if input >= -0.5 && input < 0.0 { -0.0 } else { let lower = input.floor(); if input - lower >= 0.5 { lower + 1.0 } else { lower } } })).num(); let pui_native_381:f64=v_616d6f756e745f5f33383034313063353739363162313066.clone(); pui_native_380 * pui_native_381 }; pui_native_377 + pui_native_378 }; *(v_6e6578745f5f62393732373238633736316464316132.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":194:7
                return (v_70726576696577.read().call)({ let value=(v_6e6578745f5f62393732373238633736316464316132.clone()).borrow().clone(); value }, v_7469636b65745f5f36643764363031623333363563666435.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":196:5
            { let v_7075695f6e61746976655f333834 = v_646566.clone(); expose_method(&owner, Value::string("sample"), ({  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c65 = v_73616d706c65.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_63616e6469646174655f5f34643463353263343536376635323836: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f61623134646130643764376563373238: f64 = argument_1;
                let v_64656c7461585f5f64626161316264306431663031613532: f64 = argument_2;
                let v_64656c7461595f5f33613631356566626632366434643264: f64 = argument_3;
                let v_66696e655f5f30363434646430303139383135663234: bool = argument_4;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":197:13
                let v_7469636b65745f5f31386635363832393136306435323566: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":198:7
                return (v_73616d706c65.read().call)(v_63616e6469646174655f5f34643463353263343536376635323836.clone(), v_66696e6543616e6469646174655f5f61623134646130643764376563373238.clone(), v_64656c7461585f5f64626161316264306431663031613532.clone(), v_64656c7461595f5f33613631356566626632366434643264.clone(), v_66696e655f5f30363434646430303139383135663234.clone(), v_7469636b65745f5f31386635363832393136306435323566.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":200:5
            { let v_7075695f6e61746976655f333835 = v_646566.clone(); expose_method(&owner, Value::string("finish"), ({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_73616d706c65 = v_73616d706c65.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_63616e6469646174655f5f30303037383362613966663566626361: f64 = argument_0;
                let v_66696e6543616e6469646174655f5f32346165366462643835313765623737: f64 = argument_1;
                let v_64656c7461585f5f62386136376162383264343135373538: f64 = argument_2;
                let v_64656c7461595f5f62313265616334316462313261663934: f64 = argument_3;
                let v_66696e655f5f36323033326231633463313131663633: bool = argument_4;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":201:13
                let v_7469636b65745f5f33326165303761373633613138346439: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":202:7
                if { let pui_native_386=!((v_73616d706c65.read().call)(v_63616e6469646174655f5f30303037383362613966663566626361.clone(), v_66696e6543616e6469646174655f5f32346165366462643835313765623737.clone(), v_64656c7461585f5f62386136376162383264343135373538.clone(), v_64656c7461595f5f62313265616334316462313261663934.clone(), v_66696e655f5f36323033326231633463313131663633.clone(), v_7469636b65745f5f33326165303761373633613138346439.clone())); if !(pui_native_386) {!({ let pui_native_388={ let pui_native_390=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_390) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_390} }; if (pui_native_388) {{ let pui_native_394={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_395=v_7469636b65745f5f33326165303761373633613138346439.clone(); (pui_native_394==pui_native_395) }} else {pui_native_388} })} else {pui_native_386} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":202:96
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333938 = v_7068617365.read(); (v_7075695f6e61746976655f333938.clone()).get() }), &(Value::string("dragging"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":203:39
                    return (v_70726f706f7365.read().call)(({ let v_7075695f6e61746976655f333939 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f333939.clone()).get()).num()) }).num(), v_7469636b65745f5f33326165303761373633613138346439.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":204:13
                let v_616374696f6e = { let left = ({ let v_7075695f6e61746976655f343030 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343030.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343030.clone()).owner(), false) } }).member("tapAction", true); if left.nullish() { Value::string("none") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":205:7
                { let v_7075695f6e61746976655f343031 = v_7068617365.read(); state_set(v_7075695f6e61746976655f343031.clone(), Value::string("idle"), Value::string("reason: continuous control released")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":206:7
                if { let pui_native_402=!({ let pui_native_404={ let pui_native_406=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_406) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_406} }; if (pui_native_404) {{ let pui_native_410={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_411=v_7469636b65745f5f33326165303761373633613138346439.clone(); (pui_native_410==pui_native_411) }} else {pui_native_404} }); if !(pui_native_402) {!((v_7075626c697368.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f33326165303761373633613138346439.clone()))} else {pui_native_402} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":206:59
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":207:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean((Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate"))))).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":207:41
                    { let v_7075695f6e61746976655f343134 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f343134.clone()).owner(), Value::string("activate"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":208:7
                if (Value::boolean(strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":208:32
                    return (v_70726f706f7365.read().call)({ let value=(v_6163636570746564.read()).borrow().clone(); value }, v_7469636b65745f5f33326165303761373633613138346439.clone());
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":209:7
                if ({ let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean((Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("activate"))))).bool()) } else { left } }; if left.truthy() { Value::boolean((Value::boolean(!strict_equal(&(v_616374696f6e.clone()), &(Value::string("commit"))))).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":209:64
                    { let v_7075695f6e61746976655f343231 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f343231.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":210:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":212:5
            { let v_7075695f6e61746976655f343232 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":213:5
            { let v_7075695f6e61746976655f343233 = v_646566.clone(); expose_method(&owner, Value::string("edit"), (v_65646974.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":214:5
            { let v_7075695f6e61746976655f343234 = v_646566.clone(); expose_method(&owner, Value::string("reset"), (v_7265736574.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":215:11
            v_61646a757374.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let call:Rc<dyn Fn(f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool| -> bool {
                let v_646972656374696f6e5f5f64373463323564623534313166346662: f64 = argument_0;
                let v_66696e655f5f37396438346430383736623666613233: bool = argument_1;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":216:7
                if { let pui_native_425=!((v_656e61626c6564.read().call)()); if !(pui_native_425) {!((v_646972656374696f6e5f5f64373463323564623534313166346662.clone()).is_finite())} else {pui_native_425} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":216:54
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":217:13
                let v_7469636b65745f5f61386165396263373137613331666235: f64 = { let pui_native_427:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_428:f64=1.0_f64; pui_native_427 + pui_native_428 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":218:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":219:7
                if { let pui_native_429=!({ let pui_native_431={ let pui_native_433=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_433) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_433} }; if (pui_native_431) {{ let pui_native_437={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_438=v_7469636b65745f5f61386165396263373137613331666235.clone(); (pui_native_437==pui_native_438) }} else {pui_native_431} }); if !(pui_native_429) {!((v_656e61626c6564.read().call)())} else {pui_native_429} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":219:43
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":220:13
                let v_70726f70735f5f33636334326333353262346136356239 = { let v_7075695f6e61746976655f343339 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343339.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343339.clone()).owner(), false) } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":221:13
                let v_616d6f756e745f5f39313731303861643939303161623763: f64 = if v_66696e655f5f37396438346430383736623666613233.clone() {{ let pui_native_440=(v_70726f70735f5f33636334326333353262346136356239.clone()).member("fineStep", true); if pui_native_440.nullish() {0.001_f64} else {(pui_native_440).num()} }} else {{ let pui_native_442=(v_70726f70735f5f33636334326333353262346136356239.clone()).member("step", true); if pui_native_442.nullish() {0.01_f64} else {(pui_native_442).num()} }};
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":222:7
                return (v_70726f706f7365.read().call)({ let pui_native_444:f64={ let value=(v_6163636570746564.read()).borrow().clone(); value }; let pui_native_445:f64={ let pui_native_446:f64=v_646972656374696f6e5f5f64373463323564623534313166346662.clone(); let pui_native_447:f64=v_616d6f756e745f5f39313731303861643939303161623763.clone(); pui_native_446 * pui_native_447 }; pui_native_444 + pui_native_445 }, v_7469636b65745f5f61386165396263373137613331666235.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":224:5
            { let v_7075695f6e61746976655f343438 = v_646566.clone(); expose_method(&owner, Value::string("adjust"), (v_61646a757374.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":225:5
            { let v_7075695f6e61746976655f343439 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-increment"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":225:59
                Value::boolean((v_61646a757374.read().call)(1.0_f64, false));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":226:5
            { let v_7075695f6e61746976655f343530 = v_646566.clone(); listen(&owner, Value::string("host:verdant-scalar-decrement"), false, {  let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":226:59
                Value::boolean((v_61646a757374.read().call)(-(1.0_f64), false));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":227:5
            { let v_7075695f6e61746976655f343531 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-scalar-set-value"), false, {  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f31386365623665636534653739343264: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":228:13
                let v_6576656e745f5f61333130366637353432353933636162 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":229:13
                let v_6e6578745f5f63336433633763633537626433653634: f64 = ((v_6576656e745f5f61333130366637353432353933636162.clone()).member("value", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":230:7
                if { let pui_native_452={ let pui_native_454=!((v_656e61626c6564.read().call)()); if !(pui_native_454) {(Value::boolean(!strict_equal(&(typeof_value(&(Value::number(v_6e6578745f5f63336433633763633537626433653634.clone())))), &(Value::string("number"))))).bool()} else {pui_native_454} }; if !(pui_native_452) {!((v_6e6578745f5f63336433633763633537626433653634.clone()).is_finite())} else {pui_native_452} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":230:77
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":231:13
                let v_7469636b65745f5f33323966336636383938633231333630: f64 = { let pui_native_458:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_459:f64=1.0_f64; pui_native_458 + pui_native_459 };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":232:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":233:7
                if { let pui_native_460=!({ let pui_native_462={ let pui_native_464=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_464) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_464} }; if (pui_native_462) {{ let pui_native_468={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_469=v_7469636b65745f5f33323966336636383938633231333630.clone(); (pui_native_468==pui_native_469) }} else {pui_native_462} }); if !(pui_native_460) {!((v_656e61626c6564.read().call)())} else {pui_native_460} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":233:43
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":234:7
                Value::boolean((v_70726f706f7365.read().call)(v_6e6578745f5f63336433633763633537626433653634.clone(), v_7469636b65745f5f33323966336636383938633231333630.clone()));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":236:11
            v_6b6579.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_656e61626c6564 = v_656e61626c6564.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_65646974 = v_65646974.clone(); let v_7265736574 = v_7265736574.clone(); let v_61646a757374 = v_61646a757374.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_6e616d65: Value = argument_0;
                let v_7368696674: bool = argument_1;
                let v_636f6d6d616e64: bool = argument_2;
                let v_616c74: bool = argument_3;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":237:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Escape"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":238:15
                    let v_616374697665: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f343734 = v_7068617365.read(); (v_7075695f6e61746976655f343734.clone()).get() }), &(Value::string("idle"))))).bool();
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":239:9
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":239:19
                    return v_616374697665.clone();
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":241:7
                if { let pui_native_475={ let pui_native_477=!((v_656e61626c6564.read().call)()); if !(pui_native_477) {v_616c74.clone()} else {pui_native_477} }; if !(pui_native_475) {v_636f6d6d616e64.clone()} else {pui_native_475} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":241:41
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":244:7
                if { let pui_native_479={ let pui_native_481={ let pui_native_483={ let pui_native_485={ let pui_native_487=(Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f343931 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343931.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343931.clone()).owner(), false) } }).member("keyMode", true)), &(Value::string("table-point"))))).bool(); if (pui_native_487) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_487} }; if (pui_native_485) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_485} }; if (pui_native_483) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_483} }; if (pui_native_481) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_481} }; if (pui_native_479) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-"))))).bool()} else {pui_native_479} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":245:58
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":246:7
                if { let pui_native_502={ let pui_native_504={ let pui_native_506={ let pui_native_508={ let pui_native_510={ let pui_native_512={ let pui_native_514={ let pui_native_516={ let pui_native_518={ let pui_native_520={ let pui_native_522={ let pui_native_524=(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowLeft"))))).bool(); if (pui_native_524) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_524} }; if (pui_native_522) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_522} }; if (pui_native_520) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_520} }; if (pui_native_518) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp"))))).bool()} else {pui_native_518} }; if (pui_native_516) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown"))))).bool()} else {pui_native_516} }; if (pui_native_514) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool()} else {pui_native_514} }; if (pui_native_512) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("End"))))).bool()} else {pui_native_512} }; if (pui_native_510) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_510} }; if (pui_native_508) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_508} }; if (pui_native_506) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_506} }; if (pui_native_504) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_504} }; if (pui_native_502) {(Value::boolean(!strict_equal(&(v_6e616d65.clone()), &(Value::string("-"))))).bool()} else {pui_native_502} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":248:102
                    return false;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":249:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":249:31
                    Value::boolean((v_65646974.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":249:39
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":250:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Backspace"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":250:35
                    Value::boolean((v_7265736574.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":250:44
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":251:7
                if { let pui_native_556=(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool(); if !(pui_native_556) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("End"))))).bool()} else {pui_native_556} } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":252:15
                    let v_7469636b65745f5f31316437303166616638313133643063: f64 = { let pui_native_562:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_563:f64=1.0_f64; pui_native_562 + pui_native_563 };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":252:35
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":253:9
                    if { let pui_native_564={ let pui_native_566={ let pui_native_568=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_568) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_568} }; if (pui_native_566) {{ let pui_native_572={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_573=v_7469636b65745f5f31316437303166616638313133643063.clone(); (pui_native_572==pui_native_573) }} else {pui_native_566} }; if (pui_native_564) {(v_656e61626c6564.read().call)()} else {pui_native_564} } {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":253:43
                        Value::boolean((v_70726f706f7365.read().call)(if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).bool() {({ let v_7075695f6e61746976655f353736 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f353736.clone()).get()).num()) }).num()} else {({ let v_7075695f6e61746976655f353737 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f353737.clone()).get()).num()) }).num()}, v_7469636b65745f5f31316437303166616638313133643063.clone()));
                    }
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":254:9
                    return true;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":256:13
                let v_646972656374696f6e5f5f66323766376232666630326338373137: f64 = if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp"))))).bool() {10.0_f64} else {if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown"))))).bool() {-(10.0_f64)} else {if { let pui_native_582={ let pui_native_584={ let pui_native_586=(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight"))))).bool(); if !(pui_native_586) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_586} }; if !(pui_native_584) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("+"))))).bool()} else {pui_native_584} }; if !(pui_native_582) {(Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("="))))).bool()} else {pui_native_582} } {1.0_f64} else {-(1.0_f64)}}};
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":258:7
                Value::boolean((v_61646a757374.read().call)(v_646972656374696f6e5f5f66323766376232666630326338373137.clone(), v_7368696674.clone()));
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":259:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":261:5
            { let v_7075695f6e61746976655f353936 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b6579.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":262:5
            { let v_7075695f6e61746976655f353937 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f31386132333237626564646365396437: Value = arg(&arguments, 0);
                let v_6576656e745f5f37623466666261323438326632333431: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":263:7
                if (v_6b6579.read().call)({ let left = (v_6576656e745f5f37623466666261323438326632333431.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e745f5f37623466666261323438326632333431.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), { let pui_native_600=(Value::boolean(strict_equal(&((v_6576656e745f5f37623466666261323438326632333431.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_600) {(Value::boolean(strict_equal(&((v_6576656e745f5f37623466666261323438326632333431.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_600} }, (Value::boolean(strict_equal(&((v_6576656e745f5f37623466666261323438326632333431.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":263:131
                    { let v_7075695f6e61746976655f363038 = v_6576656e745f5f37623466666261323438326632333431.clone(); prevent(v_7075695f6e61746976655f363038.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":265:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_6d696e696d756d = v_6d696e696d756d.clone(); let v_6d6178696d756d = v_6d6178696d756d.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_73746570506f6c696379 = v_73746570506f6c696379.clone(); let v_66696e65506f6c696379 = v_66696e65506f6c696379.clone(); let v_64726167506f6c696379 = v_64726167506f6c696379.clone(); let v_646f75626c65506f6c696379 = v_646f75626c65506f6c696379.clone(); let v_746170506f6c696379 = v_746170506f6c696379.clone(); let v_6b6579626f617264506f6c696379 = v_6b6579626f617264506f6c696379.clone(); let v_747261636b506f6c696379 = v_747261636b506f6c696379.clone(); let v_636c616d70 = v_636c616d70.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f66653366626161353234346166393733: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":266:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":266:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":267:13
                let v_70726f70735f5f32373639393830653033316335333862 = { let v_7075695f6e61746976655f363039 = v_68616e646c655f5f66653366626161353234346166393733.clone(); read_props(&(v_7075695f6e61746976655f363039.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":268:13
                let v_62617369735f5f64366465303139663362356566363865 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":269:13
                let v_70726576696f7573: f64 = { let value=(v_6163636570746564.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":270:13
                let v_706f6c6963794368616e676564: bool = { let pui_native_610={ let pui_native_612={ let pui_native_614={ let pui_native_616={ let pui_native_618={ let pui_native_620={ let pui_native_622={ let value=(v_73746570506f6c696379.read()).borrow().clone(); value }; let pui_native_623=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("step", false)).num(); !(pui_native_622==pui_native_623) }; if !(pui_native_620) {{ let pui_native_626={ let value=(v_66696e65506f6c696379.read()).borrow().clone(); value }; let pui_native_627=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("fineStep", false)).num(); !(pui_native_626==pui_native_627) }} else {pui_native_620} }; if !(pui_native_618) {(Value::boolean(!strict_equal(&({ let value = (v_64726167506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } })))).bool()} else {pui_native_618} }; if !(pui_native_616) {(Value::boolean(!strict_equal(&({ let value = (v_646f75626c65506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } })))).bool()} else {pui_native_616} }; if !(pui_native_614) {(Value::boolean(!strict_equal(&({ let value = (v_746170506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } })))).bool()} else {pui_native_614} }; if !(pui_native_612) {(Value::boolean(!strict_equal(&({ let value = (v_6b6579626f617264506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } })))).bool()} else {pui_native_612} }; if !(pui_native_610) {(Value::boolean(!strict_equal(&({ let value = (v_747261636b506f6c696379.read()).borrow().clone(); value }), &({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("trackPress", false); if left.nullish() { Value::string("jump") } else { left } })))).bool()} else {pui_native_610} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":274:13
                let v_6563686f65645f5f66373336663163613530303833343365: bool = { let pui_native_640={ let pui_native_642={ let pui_native_644={ let pui_native_646={ let pui_native_648={ let pui_native_650={ let pui_native_652={ let pui_native_654=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("previewEcho", false)).bool(); let pui_native_655=true; (pui_native_654==pui_native_655) }; if (pui_native_652) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f363538 = v_7068617365.read(); (v_7075695f6e61746976655f363538.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_652} }; if (pui_native_650) {(Value::boolean(strict_equal(&({ let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_650} }; if (pui_native_648) {{ let pui_native_661=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("value", false)).num(); let pui_native_662=({ let v_7075695f6e61746976655f363633 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f363633.clone()).get()).num()) }).num(); (pui_native_661==pui_native_662) }} else {pui_native_648} }; if (pui_native_646) {{ let pui_native_664=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("min", false)).num(); let pui_native_665=({ let v_7075695f6e61746976655f363638 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f363638.clone()).get()).num()) }).num(); (pui_native_664==pui_native_665) }} else {pui_native_646} }; if (pui_native_644) {{ let pui_native_669=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("max", false)).num(); let pui_native_670=({ let v_7075695f6e61746976655f363733 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f363733.clone()).get()).num()) }).num(); (pui_native_669==pui_native_670) }} else {pui_native_644} }; if (pui_native_642) {{ let pui_native_674=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("disabled", false)).bool(); let pui_native_675=true; !(pui_native_674==pui_native_675) }} else {pui_native_642} }; if (pui_native_640) {!(v_706f6c6963794368616e676564.clone())} else {pui_native_640} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":281:7
                if !(v_6563686f65645f5f66373336663163613530303833343365.clone()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":281:20
                    Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":282:13
                let v_7469636b65745f5f39323564346566383761633066376633: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":283:7
                Value::boolean({ let value=true; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":284:7
                { let value = v_68616e646c655f5f66653366626161353234346166393733.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":285:7
                Value::number({ let value=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("step", false)).num(); *(v_73746570506f6c696379.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":285:40
                Value::number({ let value=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("fineStep", false)).num(); *(v_66696e65506f6c696379.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":286:7
                { let value = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("dragMode", false); if left.nullish() { Value::string("absolute") } else { left } }; *(v_64726167506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":286:50
                { let value = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("doubleAction", false); if left.nullish() { Value::string("reset") } else { left } }; *(v_646f75626c65506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":286:96
                { let value = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("tapAction", false); if left.nullish() { Value::string("none") } else { left } }; *(v_746170506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":287:7
                { let value = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("keyMode", false); if left.nullish() { Value::string("scalar") } else { left } }; *(v_6b6579626f617264506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":287:51
                { let value = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("trackPress", false); if left.nullish() { Value::string("jump") } else { left } }; *(v_747261636b506f6c696379.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":288:13
                let v_6c6f77: f64 = ((v_70726f70735f5f32373639393830653033316335333862.clone()).member("min", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":289:13
                let v_68696768: f64 = ((v_70726f70735f5f32373639393830653033316335333862.clone()).member("max", false)).num();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":290:13
                let v_76616c6964: bool = { let pui_native_684={ let pui_native_686={ let pui_native_688={ let pui_native_690={ let pui_native_692=(v_6c6f77.clone()).is_finite(); if (pui_native_692) {(v_68696768.clone()).is_finite()} else {pui_native_692} }; if (pui_native_690) {{ let pui_native_694:f64=v_6c6f77.clone(); let pui_native_695:f64=v_68696768.clone(); pui_native_694 < pui_native_695 }} else {pui_native_690} }; if (pui_native_688) {(((v_70726f70735f5f32373639393830653033316335333862.clone()).member("value", false)).num()).is_finite()} else {pui_native_688} }; if (pui_native_686) {{ let pui_native_698:f64=v_6c6f77.clone(); let pui_native_699:f64=-(1000000000000000.0_f64); pui_native_698 >= pui_native_699 }} else {pui_native_686} }; if (pui_native_684) {{ let pui_native_700:f64=v_68696768.clone(); let pui_native_701:f64=1000000000000000.0_f64; pui_native_700 <= pui_native_701 }} else {pui_native_684} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":292:13
                let v_72616e67654368616e676564: bool = { let pui_native_702={ let pui_native_704=v_6c6f77.clone(); let pui_native_705=({ let v_7075695f6e61746976655f373036 = v_6d696e696d756d.read(); Value::number(((v_7075695f6e61746976655f373036.clone()).get()).num()) }).num(); !(pui_native_704==pui_native_705) }; if !(pui_native_702) {{ let pui_native_707=v_68696768.clone(); let pui_native_708=({ let v_7075695f6e61746976655f373039 = v_6d6178696d756d.read(); Value::number(((v_7075695f6e61746976655f373039.clone()).get()).num()) }).num(); !(pui_native_707==pui_native_708) }} else {pui_native_702} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":293:7
                { let v_7075695f6e61746976655f373130 = v_6d696e696d756d.read(); state_set(v_7075695f6e61746976655f373130.clone(), Value::number(if v_76616c6964.clone() {v_6c6f77.clone()} else {0.0_f64}), Value::string("reason: continuous control minimum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:7
                if !({ let pui_native_711={ let pui_native_713=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_713) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_713} }; if (pui_native_711) {{ let pui_native_717={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_718=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_717==pui_native_718) }} else {pui_native_711} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":294:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":295:7
                { let v_7075695f6e61746976655f373139 = v_6d6178696d756d.read(); state_set(v_7075695f6e61746976655f373139.clone(), Value::number(if v_76616c6964.clone() {v_68696768.clone()} else {1.0_f64}), Value::string("reason: continuous control maximum")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":296:7
                if !({ let pui_native_720={ let pui_native_722=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_722) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_722} }; if (pui_native_720) {{ let pui_native_726={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_727=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_726==pui_native_727) }} else {pui_native_720} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":296:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":297:13
                let v_6e6578745f5f61366533373966316530393934623933: f64 = (v_636c616d70.read().call)(((v_70726f70735f5f32373639393830653033316335333862.clone()).member("value", false)).num());
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":298:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":299:13
                let v_6563686f: bool = { let pui_native_730={ let pui_native_732={ let pui_native_734={ let pui_native_736=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("previewEcho", false)).bool(); let pui_native_737=true; (pui_native_736==pui_native_737) }; if (pui_native_734) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373430 = v_7068617365.read(); (v_7075695f6e61746976655f373430.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_734} }; if (pui_native_732) {(Value::boolean(strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_732} }; if (pui_native_730) {{ let pui_native_743=v_6e6578745f5f61366533373966316530393934623933.clone(); let pui_native_744=({ let v_7075695f6e61746976655f373435 = v_76616c7565.read(); Value::number(((v_7075695f6e61746976655f373435.clone()).get()).num()) }).num(); (pui_native_743==pui_native_744) }} else {pui_native_730} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":301:13
                let v_73686f756c6443616e63656c: bool = { let pui_native_746=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373530 = v_7068617365.read(); (v_7075695f6e61746976655f373530.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_746) {{ let pui_native_751={ let pui_native_753={ let pui_native_755={ let pui_native_757={ let pui_native_759=!(v_76616c6964.clone()); if !(pui_native_759) {v_72616e67654368616e676564.clone()} else {pui_native_759} }; if !(pui_native_757) {v_706f6c6963794368616e676564.clone()} else {pui_native_757} }; if !(pui_native_755) {{ let pui_native_761=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("disabled", false)).bool(); let pui_native_762=true; (pui_native_761==pui_native_762) }} else {pui_native_755} }; if !(pui_native_753) {(Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_753} }; if !(pui_native_751) {{ let pui_native_765=!(v_6563686f.clone()); if (pui_native_765) {{ let pui_native_767=v_6e6578745f5f61366533373966316530393934623933.clone(); let pui_native_768={ let value=(v_6163636570746564.read()).borrow().clone(); value }; !(pui_native_767==pui_native_768) }} else {pui_native_765} }} else {pui_native_751} }} else {pui_native_746} };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":303:7
                if !(v_6563686f.clone()) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":303:18
                    Value::number({ let value=v_6e6578745f5f61366533373966316530393934623933.clone(); *(v_6163636570746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":304:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":305:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":306:9
                    { let v_7075695f6e61746976655f373639 = v_7068617365.read(); state_set(v_7075695f6e61746976655f373639.clone(), Value::string("idle"), Value::string("reason: continuous control authority changed")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":307:9
                    if !({ let pui_native_770={ let pui_native_772=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_772) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_772} }; if (pui_native_770) {{ let pui_native_776={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_777=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_776==pui_native_777) }} else {pui_native_770} }) {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":307:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":309:7
                { let v_7075695f6e61746976655f373738 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f373738.clone(), { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: continuous control label")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:7
                if !({ let pui_native_779={ let pui_native_781=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_781) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_781} }; if (pui_native_779) {{ let pui_native_785={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_786=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_785==pui_native_786) }} else {pui_native_779} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":310:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":311:7
                { let v_7075695f6e61746976655f373837 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f373837.clone(), { let left = (v_70726f70735f5f32373639393830653033316335333862.clone()).member("role", false); if left.nullish() { Value::string("slider") } else { left } }, Value::string("reason: continuous control role")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":312:7
                if !({ let pui_native_788={ let pui_native_790=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_790) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_790} }; if (pui_native_788) {{ let pui_native_794={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_795=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_794==pui_native_795) }} else {pui_native_788} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":312:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":313:7
                { let v_7075695f6e61746976655f373936 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f373936.clone(), Value::boolean({ let pui_native_797=!(v_76616c6964.clone()); if !(pui_native_797) {{ let pui_native_799=((v_70726f70735f5f32373639393830653033316335333862.clone()).member("disabled", false)).bool(); let pui_native_800=true; (pui_native_799==pui_native_800) }} else {pui_native_797} }), Value::string("reason: continuous control policy")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":314:7
                if !({ let pui_native_801={ let pui_native_803=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_803) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_803} }; if (pui_native_801) {{ let pui_native_807={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_808=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_807==pui_native_808) }} else {pui_native_801} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":314:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":315:7
                { let v_7075695f6e61746976655f383039 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f383039.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f383130 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f383130.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:7
                if !({ let pui_native_811={ let pui_native_813=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_813) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_813} }; if (pui_native_811) {{ let pui_native_817={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_818=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_817==pui_native_818) }} else {pui_native_811} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":316:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":317:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f383231 = v_7068617365.read(); (v_7075695f6e61746976655f383231.clone()).get() }), &(Value::string("idle"))))).bool() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":318:9
                    { let v_7075695f6e61746976655f383232 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f383232.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control authoritative display")) };
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:9
                    if !({ let pui_native_823={ let pui_native_825=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_825) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_825} }; if (pui_native_823) {{ let pui_native_829={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_830=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_829==pui_native_830) }} else {pui_native_823} }) {
                        // Source "frontend/proto-ui/src/continuous-control.proto.ts":319:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":321:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":321:25
                    { let v_7075695f6e61746976655f383331 = v_68616e646c655f5f66653366626161353234346166393733.clone(); emit(&(v_7075695f6e61746976655f383331.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number(v_70726576696f7573.clone())), ("revision", v_62617369735f5f64366465303139663362356566363865.clone())])) };
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":322:7
                if !({ let pui_native_832={ let pui_native_834=!({ let value=(v_646973706f736564.read()).borrow().clone(); value }); if (pui_native_834) {(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool()} else {pui_native_834} }; if (pui_native_832) {{ let pui_native_838={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_839=v_7469636b65745f5f39323564346566383761633066376633.clone(); (pui_native_838==pui_native_839) }} else {pui_native_832} }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":322:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":323:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":324:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":324:21
                    { let v_7075695f6e61746976655f383430 = v_68616e646c655f5f66653366626161353234346166393733.clone(); request_update(&(v_7075695f6e61746976655f383430.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":326:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_76616c7565 = v_76616c7565.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_6163636570746564 = v_6163636570746564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let call:Rc<dyn Fn(bool)->Value>=Rc::new(move |argument_0: bool| -> Value {
                let v_7065726d616e656e74: bool = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":327:13
                let v_68616e646c655f5f38643434353039633461646363363035 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":328:13
                let v_776173416374697665: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f383433 = v_7068617365.read(); (v_7075695f6e61746976655f383433.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":329:13
                let v_62617369735f5f61346564663034373664653062343561 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":330:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":331:13
                let v_7469636b65745f5f30646135616435303666396135626265: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":332:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":332:24
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":332:42
                Value::boolean({ let value={ let pui_native_844={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_844) {v_7065726d616e656e74.clone()} else {pui_native_844} }; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":332:76
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":333:7
                if v_7065726d616e656e74.clone() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":333:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":334:7
                { let v_7075695f6e61746976655f383436 = v_7068617365.read(); state_set(v_7075695f6e61746976655f383436.clone(), Value::string("idle"), Value::string("reason: continuous control detached")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":335:7
                if { let pui_native_847={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_848=v_7469636b65745f5f30646135616435303666396135626265.clone(); !(pui_native_847==pui_native_848) } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":335:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":336:7
                { let v_7075695f6e61746976655f383439 = v_76616c7565.read(); state_set(v_7075695f6e61746976655f383439.clone(), Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value }), Value::string("reason: continuous control detached display")) };
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":337:7
                if { let pui_native_850={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_851=v_7469636b65745f5f30646135616435303666396135626265.clone(); !(pui_native_850==pui_native_851) } {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":337:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":338:7
                if ({ let left = Value::boolean(v_776173416374697665.clone()); if left.truthy() { v_68616e646c655f5f38643434353039633461646363363035.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":338:32
                    { let v_7075695f6e61746976655f383532 = v_68616e646c655f5f38643434353039633461646363363035.clone(); emit(&(v_7075695f6e61746976655f383532.clone()).owner(), Value::string("interactionCancel"), record(vec![("value", Value::number({ let value=(v_6163636570746564.read()).borrow().clone(); value })), ("revision", v_62617369735f5f61346564663034373664653062343561.clone())])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":340:5
            { let v_7075695f6e61746976655f383533 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":341:5
            { let v_7075695f6e61746976655f383534 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f30336337626333363332343135636631: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":341:43
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":341:60
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/continuous-control.proto.ts":341:77
                    v_73796e63.read().call_authored(vec![v_68616e646c655f5f30336337626333363332343135636631.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":342:5
            { let v_7075695f6e61746976655f383535 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("value"), Value::string("min"), Value::string("max"), Value::string("step"), Value::string("fineStep"), Value::string("defaultValue"), Value::string("label"), Value::string("disabled"), Value::string("revision"), Value::string("role"), Value::string("dragMode"), Value::string("doubleAction"), Value::string("tapAction"), Value::string("previewEcho"), Value::string("keyMode"), Value::string("trackPress")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":344:5
            { let v_7075695f6e61746976655f383536 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":345:5
            { let v_7075695f6e61746976655f383537 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":345:37
                return (v_726574697265.read().call)(false);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":346:5
            { let v_7075695f6e61746976655f383538 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":346:41
                return (v_726574697265.read().call)(true);
            })) }) };
            // Source "frontend/proto-ui/src/continuous-control.proto.ts":347:5
            return { let callable={ let pui_native_859={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/continuous-control.proto.ts":347:26
                return { let v_7075695f6e61746976655f383630 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_859.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_859.call)(arg(&arguments,0))})} }; match callable {
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
