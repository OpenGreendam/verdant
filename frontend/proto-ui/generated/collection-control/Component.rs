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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("mode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("cursor").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("orientation").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("columns").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "mode" => value.is_string(),
        "label" => value.is_string(),
        "description" => value.is_string(),
        "cursor" => value.as_f64().is_some_and(f64::is_finite),
        "orientation" => value.is_string(),
        "expanded" => value.as_f64().is_some_and(f64::is_finite),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "autoFocus" => value.is_boolean(),
        "columns" => value.as_f64().is_some_and(f64::is_finite), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "filterKey" => if !(arguments.len() <= 7 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(6).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "revealGeometry" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "setItems" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_array().is_some_and(|items| items.iter().all(|item| item.as_f64().is_some_and(f64::is_finite)))))) { value_type_error("invalid exposed method arguments") },
        "choose" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "selectIndex" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "keyAt" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "selectAndFocus" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "key" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "contextAt" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "reconcile" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "dismiss" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "focusSelf" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "mode", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "description", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<String>,
    #[serde(rename = "cursor", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "orientation", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<String>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<f64>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<bool>,
    #[serde(rename = "columns", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<f64>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct CollectionControlHandle { pub owner: OwnerRef }
  impl CollectionControlHandle {
      pub fn state_0(&self) -> Result<f64, Value> { self.state("scrollPosition").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("scrollAtEnd").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("horizontalPosition").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<String, Value> { self.state("mode").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<f64, Value> { self.state("cursor").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<f64, Value> { self.state("expanded").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_6(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_7(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_14(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: String, argument_6: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("filterKey").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("filterKey", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_15(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("revealGeometry").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("revealGeometry", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_16(&self, argument_0: Vec<f64>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("setItems").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("setItems", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_17(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
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
    pub fn method_18(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("selectIndex").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectIndex", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_19(&self, argument_0: f64, argument_1: String, argument_2: bool, argument_3: bool, argument_4: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyAt").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyAt", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_20(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("selectAndFocus").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectAndFocus", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_21(&self, argument_0: String, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
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
    pub fn method_22(&self, argument_0: f64, argument_1: String, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("contextAt").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("contextAt", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_23(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("reconcile").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("reconcile", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_24(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("dismiss").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("dismiss", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_null()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_25(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
  pub struct CollectionControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl CollectionControl {
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
            let v_637572736f72: LexicalBinding<Value> = lexical_binding();
            let v_657870616e646564: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_6465736372697074696f6e: LexicalBinding<Value> = lexical_binding();
            let v_7461626c65: LexicalBinding<Value> = lexical_binding();
            let v_726f7773: LexicalBinding<Value> = lexical_binding();
            let v_636f6c756d6e73: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_7363726f6c6c: LexicalBinding<Value> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7265616479: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_6974656d73: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b696e64: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> f64>> = lexical_binding();
            let v_636f756e74: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> f64>> = lexical_binding();
            let v_6d696e: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> f64>> = lexical_binding();
            let v_6d6178: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> f64>> = lexical_binding();
            let v_66696e64: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, bool) -> f64>> = lexical_binding();
            let v_7365656b: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64, f64, bool) -> f64>> = lexical_binding();
            let v_76616c69644974656d73: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, f64) -> bool>> = lexical_binding();
            let v_656d6974: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, bool, Value) -> Value>> = lexical_binding();
            let v_7265717565737452657665616c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> Value>> = lexical_binding();
            let v_6368616e6765457870616e73696f6e: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, Value) -> bool>> = lexical_binding();
            let v_73656c656374: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool, bool) -> bool>> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool) -> bool>> = lexical_binding();
            let v_636f6e746578744174: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, Value, f64, f64, bool) -> bool>> = lexical_binding();
            let v_6b65795f5f63636538616136363530633839623964: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool, bool) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/collection-control.proto.ts":14:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("mode", record(vec![("type", Value::string("string")), ("default", Value::string("listbox"))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("cursor", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("orientation", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("expanded", record(vec![("type", Value::string("number")), ("default", Value::number(-(1.0_f64)))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("columns", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))]))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":22:11
            v_637572736f72.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cursor"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":23:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "discrete", Value::string("expanded"), Value::number(-(1.0_f64)), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":24:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":25:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("listbox"), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":26:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":27:11
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":28:11
            v_7461626c65.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("table"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:11
            v_726f7773.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("rows"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:11
            v_636f6c756d6e73.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "discrete", Value::string("columns"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:34
            { let v_7075695f6e61746976655f3130 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:11
            v_7363726f6c6c.initialize(native_builtin_hook(&owner, "asScrollSurface", || scroll_setup(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:39
            { let v_7075695f6e61746976655f3131 = v_7363726f6c6c.read(); scroll_configure(&(v_7075695f6e61746976655f3131.clone()).owner(), record(vec![("axes", Value::string("both")), ("projection", Value::string("system"))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:34
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3132.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:51
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3133.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:69
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3134.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:100
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:5
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("rowCount"), v_726f7773.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:52
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("columnCount"), v_636f6c756d6e73.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("scrollPosition"), ((v_7363726f6c6c.read()).member("vertical", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:67
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("scrollAtEnd"), ((v_7363726f6c6c.read()).member("vertical", false)).member("atEnd", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("horizontalPosition"), ((v_7363726f6c6c.read()).member("horizontal", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("mode"), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("cursor"), v_637572736f72.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":38:41
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("expanded"), v_657870616e646564.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:49
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_event(&owner, Value::string("select")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_event(&owner, Value::string("expand")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("close")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":46:9
            v_63757272656e74.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":47:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":48:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":49:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":50:11
            v_7265616479.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":50:34
                return { let pui_native_32={ let pui_native_34={ let pui_native_36=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e74.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_36) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_36} }; if (pui_native_34) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_34} }; if (pui_native_32) {!(({ let v_7075695f6e61746976655f3430 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3430.clone()).get()).bool()) }).bool())} else {pui_native_32} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":51:9
            v_6974656d73.initialize(Rc::new(std::cell::RefCell::new(Value::Array(Rc::new(vec![])))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":52:11
            v_6b696e64.initialize({  let v_6974656d73 = v_6974656d73.clone(); let call:Rc<dyn Fn(f64)->f64>=Rc::new(move |argument_0: f64| -> f64 {
                let v_696e6465785f5f36363464363464383135616264373833: f64 = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":52:45
                return if { let pui_native_41={ let pui_native_43={ let pui_native_45=(v_696e6465785f5f36363464363464383135616264373833.clone()).is_finite(); if (pui_native_45) {{ let pui_native_47={ let pui_native_49:f64=v_696e6465785f5f36363464363464383135616264373833.clone(); let pui_native_50:f64=1.0_f64; pui_native_49 % pui_native_50 }; let pui_native_48=0.0_f64; (pui_native_47==pui_native_48) }} else {pui_native_45} }; if (pui_native_43) {{ let pui_native_51:f64=v_696e6465785f5f36363464363464383135616264373833.clone(); let pui_native_52:f64=0.0_f64; pui_native_51 >= pui_native_52 }} else {pui_native_43} }; if (pui_native_41) {{ let pui_native_53:f64=v_696e6465785f5f36363464363464383135616264373833.clone(); let pui_native_54:f64=(({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false)).num(); pui_native_53 < pui_native_54 }} else {pui_native_41} } {{ let pui_native_55={ let object = { let value = (v_6974656d73.read()).borrow().clone(); value }; object.caught_member(&property_key(&(Value::number(v_696e6465785f5f36363464363464383135616264373833.clone()))), false) }; if pui_native_55.nullish() {0.0_f64} else {(pui_native_55).num()} }} else {0.0_f64};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":53:11
            v_636f756e74.initialize({  let v_6974656d73 = v_6974656d73.clone(); let call:Rc<dyn Fn()->f64>=Rc::new(move || -> f64 {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":53:33
                return (({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false)).num();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":54:11
            v_6d696e.initialize({   let call:Rc<dyn Fn(f64, f64)->f64>=Rc::new(move |argument_0: f64, argument_1: f64| -> f64 {
                let v_615f5f66613437356230386530336138663863: f64 = argument_0;
                let v_625f5f63366162623939393938333937383534: f64 = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":54:51
                return if { let pui_native_57:f64=v_615f5f66613437356230386530336138663863.clone(); let pui_native_58:f64=v_625f5f63366162623939393938333937383534.clone(); pui_native_57 < pui_native_58 } {v_615f5f66613437356230386530336138663863.clone()} else {v_625f5f63366162623939393938333937383534.clone()};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":55:11
            v_6d6178.initialize({   let call:Rc<dyn Fn(f64, f64)->f64>=Rc::new(move |argument_0: f64, argument_1: f64| -> f64 {
                let v_615f5f34616131306637366338373831393632: f64 = argument_0;
                let v_625f5f61393064373237633066333932316664: f64 = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":55:51
                return if { let pui_native_59:f64=v_615f5f34616131306637366338373831393632.clone(); let pui_native_60:f64=v_625f5f61393064373237633066333932316664.clone(); pui_native_59 > pui_native_60 } {v_615f5f34616131306637366338373831393632.clone()} else {v_625f5f61393064373237633066333932316664.clone()};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":56:11
            v_66696e64.initialize({  let v_6b696e64 = v_6b696e64.clone(); let v_66696e64 = v_66696e64.clone(); let call:Rc<dyn Fn(f64, f64, bool)->f64>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool| -> f64 {
                let v_73746172745f5f32326334346537333664336138613034: f64 = argument_0;
                let v_656e645f5f62383832313466396664336637333765: f64 = argument_1;
                let v_666f7277617264: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":57:7
                if { let pui_native_61:f64=v_656e645f5f62383832313466396664336637333765.clone(); let pui_native_62:f64=v_73746172745f5f32326334346537333664336138613034.clone(); pui_native_61 <= pui_native_62 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":57:25
                    return -(1.0_f64);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":58:7
                if { let pui_native_63={ let pui_native_65:f64=v_656e645f5f62383832313466396664336637333765.clone(); let pui_native_66:f64=v_73746172745f5f32326334346537333664336138613034.clone(); pui_native_65 - pui_native_66 }; let pui_native_64=1.0_f64; (pui_native_63==pui_native_64) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":58:30
                    return if { let pui_native_67:f64=(v_6b696e64.read().call)(v_73746172745f5f32326334346537333664336138613034.clone()); let pui_native_68:f64=0.0_f64; pui_native_67 > pui_native_68 } {v_73746172745f5f32326334346537333664336138613034.clone()} else {-(1.0_f64)};
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":59:13
                let v_73756d5f5f33303961306230316337396238383531: f64 = { let pui_native_69:f64=v_73746172745f5f32326334346537333664336138613034.clone(); let pui_native_70:f64=v_656e645f5f62383832313466396664336637333765.clone(); pui_native_69 + pui_native_70 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":60:13
                let v_6d6964646c655f5f37386164383235636239616665353633: f64 = { let pui_native_71:f64={ let pui_native_73:f64=v_73756d5f5f33303961306230316337396238383531.clone(); let pui_native_74:f64={ let pui_native_75:f64=v_73756d5f5f33303961306230316337396238383531.clone(); let pui_native_76:f64=2.0_f64; pui_native_75 % pui_native_76 }; pui_native_73 - pui_native_74 }; let pui_native_72:f64=2.0_f64; pui_native_71 / pui_native_72 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":61:13
                let v_63616e6469646174655f5f63303864656465393134663333643331: f64 = if v_666f7277617264.clone() {(v_66696e64.read().call)(v_73746172745f5f32326334346537333664336138613034.clone(), v_6d6964646c655f5f37386164383235636239616665353633.clone(), true)} else {(v_66696e64.read().call)(v_6d6964646c655f5f37386164383235636239616665353633.clone(), v_656e645f5f62383832313466396664336637333765.clone(), false)};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":62:7
                if { let pui_native_77:f64=v_63616e6469646174655f5f63303864656465393134663333643331.clone(); let pui_native_78:f64=0.0_f64; pui_native_77 >= pui_native_78 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":62:27
                    return v_63616e6469646174655f5f63303864656465393134663333643331.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":63:7
                return if v_666f7277617264.clone() {(v_66696e64.read().call)(v_6d6964646c655f5f37386164383235636239616665353633.clone(), v_656e645f5f62383832313466396664336637333765.clone(), true)} else {(v_66696e64.read().call)(v_73746172745f5f32326334346537333664336138613034.clone(), v_6d6964646c655f5f37386164383235636239616665353633.clone(), false)};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":65:11
            v_7365656b.initialize({  let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_66696e64 = v_66696e64.clone(); let call:Rc<dyn Fn(f64, f64, f64, bool)->f64>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: bool| -> f64 {
                let v_7265717565737465645f5f62643933396431636635323762306436: f64 = argument_0;
                let v_646972656374696f6e: f64 = argument_1;
                let v_5f72656d61696e696e67: f64 = argument_2;
                let v_77726170: bool = argument_3;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":66:13
                let v_696e6465785f5f33396633613562393662616462336531: f64 = (v_6d696e.read().call)((v_636f756e74.read().call)(), (v_6d6178.read().call)(-(1.0_f64), v_7265717565737465645f5f62643933396431636635323762306436.clone()));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":67:13
                let v_63616e6469646174655f5f61653531383666613035326231303664: f64 = if { let pui_native_79:f64=v_646972656374696f6e.clone(); let pui_native_80:f64=0.0_f64; pui_native_79 > pui_native_80 } {(v_66696e64.read().call)((v_6d6178.read().call)(0.0_f64, v_696e6465785f5f33396633613562393662616462336531.clone()), (v_636f756e74.read().call)(), true)} else {(v_66696e64.read().call)(0.0_f64, (v_6d696e.read().call)((v_636f756e74.read().call)(), { let pui_native_81:f64=v_696e6465785f5f33396633613562393662616462336531.clone(); let pui_native_82:f64=1.0_f64; pui_native_81 + pui_native_82 }), false)};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":68:7
                if { let pui_native_83={ let pui_native_85:f64=v_63616e6469646174655f5f61653531383666613035326231303664.clone(); let pui_native_86:f64=0.0_f64; pui_native_85 >= pui_native_86 }; if !(pui_native_83) {!(v_77726170.clone())} else {pui_native_83} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":68:36
                    return v_63616e6469646174655f5f61653531383666613035326231303664.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":69:7
                return if { let pui_native_87:f64=v_646972656374696f6e.clone(); let pui_native_88:f64=0.0_f64; pui_native_87 > pui_native_88 } {(v_66696e64.read().call)(0.0_f64, (v_6d6178.read().call)(0.0_f64, v_696e6465785f5f33396633613562393662616462336531.clone()), true)} else {(v_66696e64.read().call)((v_6d696e.read().call)((v_636f756e74.read().call)(), { let pui_native_89:f64=v_696e6465785f5f33396633613562393662616462336531.clone(); let pui_native_90:f64=1.0_f64; pui_native_89 + pui_native_90 }), (v_636f756e74.read().call)(), false)};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":71:11
            v_76616c69644974656d73.initialize({  let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_76616c756573: Value = argument_0;
                let v_73746172745f5f62396333303062343363623338616636: f64 = argument_1;
                let v_656e645f5f37653636623161383836346433353835: f64 = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":72:7
                if { let pui_native_91:f64=v_656e645f5f37653636623161383836346433353835.clone(); let pui_native_92:f64=v_73746172745f5f62396333303062343363623338616636.clone(); pui_native_91 <= pui_native_92 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":72:25
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":73:7
                if { let pui_native_93={ let pui_native_95:f64=v_656e645f5f37653636623161383836346433353835.clone(); let pui_native_96:f64=v_73746172745f5f62396333303062343363623338616636.clone(); pui_native_95 - pui_native_96 }; let pui_native_94=1.0_f64; (pui_native_93==pui_native_94) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":73:30
                    return { let pui_native_97={ let pui_native_99=(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f62396333303062343363623338616636.clone()))), false) }), &(Value::number(0.0_f64))))).bool(); if !(pui_native_99) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f62396333303062343363623338616636.clone()))), false) }), &(Value::number(1.0_f64))))).bool()} else {pui_native_99} }; if !(pui_native_97) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f62396333303062343363623338616636.clone()))), false) }), &(Value::number(2.0_f64))))).bool()} else {pui_native_97} };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":74:13
                let v_73756d5f5f37653963393564326132353162383639: f64 = { let pui_native_107:f64=v_73746172745f5f62396333303062343363623338616636.clone(); let pui_native_108:f64=v_656e645f5f37653636623161383836346433353835.clone(); pui_native_107 + pui_native_108 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":75:13
                let v_6d6964646c655f5f33303439393264393338623235383761: f64 = { let pui_native_109:f64={ let pui_native_111:f64=v_73756d5f5f37653963393564326132353162383639.clone(); let pui_native_112:f64={ let pui_native_113:f64=v_73756d5f5f37653963393564326132353162383639.clone(); let pui_native_114:f64=2.0_f64; pui_native_113 % pui_native_114 }; pui_native_111 - pui_native_112 }; let pui_native_110:f64=2.0_f64; pui_native_109 / pui_native_110 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":76:7
                return { let pui_native_115=(v_76616c69644974656d73.read().call)(v_76616c756573.clone(), v_73746172745f5f62396333303062343363623338616636.clone(), v_6d6964646c655f5f33303439393264393338623235383761.clone()); if (pui_native_115) {(v_76616c69644974656d73.read().call)(v_76616c756573.clone(), v_6d6964646c655f5f33303439393264393338623235383761.clone(), v_656e645f5f37653636623161383836346433353835.clone())} else {pui_native_115} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":78:11
            v_656d6974.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let call:Rc<dyn Fn(Value, f64, bool, Value)->Value>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: bool, argument_3: Value| -> Value {
                let v_6e616d655f5f38393536316239363236373961313664: Value = argument_0;
                let v_696e6465785f5f65316365343832353735633961623635: f64 = argument_1;
                let v_657874656e645f5f36353566626264643163366330363164: bool = argument_2;
                let v_6b65795f5f37383635303832363661313933636137: Value = argument_3;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":79:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":79:20
                    { let v_7075695f6e61746976655f313137 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313137.clone()).owner(), v_6e616d655f5f38393536316239363236373961313664.clone(), record(vec![("index", Value::number(v_696e6465785f5f65316365343832353735633961623635.clone())), ("extend", Value::boolean(v_657874656e645f5f36353566626264643163366330363164.clone())), ("key", v_6b65795f5f37383635303832363661313933636137.clone()), ("revision", { let left = ({ let v_7075695f6e61746976655f313138 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313138.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), arg(&arguments,3))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":81:11
            v_7265717565737452657665616c.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); let call:Rc<dyn Fn(f64)->Value>=Rc::new(move |argument_0: f64| -> Value {
                let v_696e6465785f5f33393531626561343663653166626431: f64 = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":82:13
                let v_6f776e65725f5f61353730616338633666353139383837 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":83:7
                if { let pui_native_119=!((v_6f776e65725f5f61353730616338633666353139383837.clone()).truthy()); if !(pui_native_119) {!((v_7265616479.read().call)())} else {pui_native_119} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":83:31
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":84:13
                let v_6d6f64655f5f32386364333135663730633139386234 = ({ let v_7075695f6e61746976655f313231 = v_6f776e65725f5f61353730616338633666353139383837.clone(); read_props(&(v_7075695f6e61746976655f313231.clone()).owner(), false) }).member("mode", false);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":85:7
                if { let pui_native_122=(Value::boolean(strict_equal(&(v_6d6f64655f5f32386364333135663730633139386234.clone()), &(Value::string("filterlist"))))).bool(); if !(pui_native_122) {(Value::boolean(strict_equal(&(v_6d6f64655f5f32386364333135663730633139386234.clone()), &(Value::string("tracklist"))))).bool()} else {pui_native_122} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":85:58
                    (v_656d6974.read().call)(Value::string("command"), v_696e6465785f5f33393531626561343663653166626431.clone(), false, Value::string("reveal"));
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":87:11
            v_6368616e6765457870616e73696f6e.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); let call:Rc<dyn Fn(f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value| -> bool {
                let v_6e6578745f5f31643832333866323937646431306436: f64 = argument_0;
                let v_6e616d655f5f34323666653430356465353638646632: Value = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":88:7
                if !((v_7265616479.read().call)()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":88:21
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":89:13
                let v_7469636b65745f5f35396236643266323430666663633037: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":90:7
                if { let pui_native_128={ let pui_native_130:f64=v_6e6578745f5f31643832333866323937646431306436.clone(); let pui_native_131:f64=0.0_f64; pui_native_130 < pui_native_131 }; if (pui_native_128) {{ let pui_native_132:f64=({ let v_7075695f6e61746976655f313334 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f313334.clone()).get()).num()) }).num(); let pui_native_133:f64=0.0_f64; pui_native_132 >= pui_native_133 }} else {pui_native_128} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":91:9
                    { let v_7075695f6e61746976655f313335 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313335.clone(), Value::number(({ let v_7075695f6e61746976655f313336 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f313336.clone()).get()).num()) }).num()), Value::string("reason: closing submenu restores parent")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":92:9
                    if { let pui_native_137={ let pui_native_139=v_7469636b65745f5f35396236643266323430666663633037.clone(); let pui_native_140={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_139==pui_native_140) }; if !(pui_native_137) {!((v_7265616479.read().call)())} else {pui_native_137} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":92:43
                        return false;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":94:7
                { let v_7075695f6e61746976655f313431 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f313431.clone(), Value::number(v_6e6578745f5f31643832333866323937646431306436.clone()), Value::string("reason: submenu expansion")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":95:7
                if { let pui_native_142={ let pui_native_144=v_7469636b65745f5f35396236643266323430666663633037.clone(); let pui_native_145={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_144==pui_native_145) }; if !(pui_native_142) {!((v_7265616479.read().call)())} else {pui_native_142} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":95:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":96:7
                (v_656d6974.read().call)(Value::string("expand"), v_6e6578745f5f31643832333866323937646431306436.clone(), false, v_6e616d655f5f34323666653430356465353638646632.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":97:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let pui_native_146=v_7469636b65745f5f35396236643266323430666663633037.clone(); let pui_native_147={ let value=(v_65706f6368.read()).borrow().clone(); value }; (pui_native_146==pui_native_147) }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":97:40
                    { let v_7075695f6e61746976655f313438 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f313438.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":98:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":100:11
            v_73656c656374.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); let call:Rc<dyn Fn(f64, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool| -> bool {
                let v_696e6465785f5f30396366383834383461353234623464: f64 = argument_0;
                let v_657874656e645f5f65343537633465636263636362333362: bool = argument_1;
                let v_6578706c69636974: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":101:7
                if { let pui_native_149=!((v_7265616479.read().call)()); if !(pui_native_149) {{ let pui_native_151=(v_6b696e64.read().call)(v_696e6465785f5f30396366383834383461353234623464.clone()); let pui_native_152=0.0_f64; (pui_native_151==pui_native_152) }} else {pui_native_149} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":101:42
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":104:7
                if { let pui_native_153={ let pui_native_155=!(v_6578706c69636974.clone()); if (pui_native_155) {{ let pui_native_157=v_696e6465785f5f30396366383834383461353234623464.clone(); let pui_native_158=({ let v_7075695f6e61746976655f313539 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f313539.clone()).get()).num()) }).num(); (pui_native_157==pui_native_158) }} else {pui_native_155} }; if (pui_native_153) {!(v_657874656e645f5f65343537633465636263636362333362.clone())} else {pui_native_153} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":104:59
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":105:13
                let v_7469636b65745f5f37626238383031343734313532396437: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":106:7
                { let v_7075695f6e61746976655f313630 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313630.clone(), Value::number(v_696e6465785f5f30396366383834383461353234623464.clone()), Value::string("reason: collection selection")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":107:7
                if { let pui_native_161={ let pui_native_163=v_7469636b65745f5f37626238383031343734313532396437.clone(); let pui_native_164={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_163==pui_native_164) }; if !(pui_native_161) {!((v_7265616479.read().call)())} else {pui_native_161} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":107:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":108:7
                (v_656d6974.read().call)(Value::string("select"), v_696e6465785f5f30396366383834383461353234623464.clone(), v_657874656e645f5f65343537633465636263636362333362.clone(), Value::string(""));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":109:7
                if { let pui_native_165={ let pui_native_167=v_7469636b65745f5f37626238383031343734313532396437.clone(); let pui_native_168={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_167==pui_native_168) }; if !(pui_native_165) {!((v_7265616479.read().call)())} else {pui_native_165} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":109:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":110:7
                (v_7265717565737452657665616c.read().call)(v_696e6465785f5f30396366383834383461353234623464.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":111:7
                if { let pui_native_169={ let pui_native_171=v_7469636b65745f5f37626238383031343734313532396437.clone(); let pui_native_172={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_171==pui_native_172) }; if !(pui_native_169) {!((v_7265616479.read().call)())} else {pui_native_169} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":111:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":112:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let pui_native_173=v_7469636b65745f5f37626238383031343734313532396437.clone(); let pui_native_174={ let value=(v_65706f6368.read()).borrow().clone(); value }; (pui_native_173==pui_native_174) }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":112:40
                    { let v_7075695f6e61746976655f313735 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f313735.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":113:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":115:11
            v_63686f6f7365.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let call:Rc<dyn Fn(f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool| -> bool {
                let v_696e6465785f5f30646536643164336436666136653830: f64 = argument_0;
                let v_657874656e645f5f34633161383735333230353534306130: bool = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":116:7
                if { let pui_native_176=!((v_7265616479.read().call)()); if !(pui_native_176) {{ let pui_native_178=(v_6b696e64.read().call)(v_696e6465785f5f30646536643164336436666136653830.clone()); let pui_native_179=0.0_f64; (pui_native_178==pui_native_179) }} else {pui_native_176} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":116:42
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":117:13
                let v_7469636b65745f5f35393863386432653139666563623032: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":118:7
                { let v_7075695f6e61746976655f313830 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313830.clone(), Value::number(v_696e6465785f5f30646536643164336436666136653830.clone()), Value::string("reason: collection activation cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":119:7
                if { let pui_native_181=!((v_7265616479.read().call)()); if !(pui_native_181) {{ let pui_native_183={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_184=v_7469636b65745f5f35393863386432653139666563623032.clone(); !(pui_native_183==pui_native_184) }} else {pui_native_181} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":119:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":120:7
                if { let pui_native_185=(v_6b696e64.read().call)(v_696e6465785f5f30646536643164336436666136653830.clone()); let pui_native_186=2.0_f64; (pui_native_185==pui_native_186) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":121:15
                    let v_6e6578745f5f65393831633161393530313364346565: f64 = if { let pui_native_187=({ let v_7075695f6e61746976655f313839 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f313839.clone()).get()).num()) }).num(); let pui_native_188=v_696e6465785f5f30646536643164336436666136653830.clone(); (pui_native_187==pui_native_188) } {-(1.0_f64)} else {v_696e6465785f5f30646536643164336436666136653830.clone()};
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":122:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(v_6e6578745f5f65393831633161393530313364346565.clone(), Value::string(""));
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":123:14
                    (v_656d6974.read().call)(Value::string("activate"), v_696e6465785f5f30646536643164336436666136653830.clone(), v_657874656e645f5f34633161383735333230353534306130.clone(), Value::string(""));
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":124:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":126:11
            v_636f6e746578744174.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let call:Rc<dyn Fn(f64, Value, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: Value, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_696e6465785f5f63383336636366396537373937356266: f64 = argument_0;
                let v_62617369735f5f35373233316461656662626465313432: Value = argument_1;
                let v_78: f64 = argument_2;
                let v_79: f64 = argument_3;
                let v_6b6579626f617264: bool = argument_4;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":127:13
                let v_6f776e65725f5f34333637383931663863643364353464 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":128:7
                if { let pui_native_190={ let pui_native_192={ let pui_native_194={ let pui_native_196={ let pui_native_198={ let pui_native_200={ let pui_native_202=!((v_6f776e65725f5f34333637383931663863643364353464.clone()).truthy()); if !(pui_native_202) {!((v_7265616479.read().call)())} else {pui_native_202} }; if !(pui_native_200) {(Value::boolean(!strict_equal(&(v_62617369735f5f35373233316461656662626465313432.clone()), &({ let left = ({ let v_7075695f6e61746976655f323036 = v_6f776e65725f5f34333637383931663863643364353464.clone(); read_props(&(v_7075695f6e61746976655f323036.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_200} }; if !(pui_native_198) {{ let pui_native_207=(v_6b696e64.read().call)(v_696e6465785f5f63383336636366396537373937356266.clone()); let pui_native_208=0.0_f64; (pui_native_207==pui_native_208) }} else {pui_native_198} }; if !(pui_native_196) {!((v_696e6465785f5f63383336636366396537373937356266.clone()).is_finite())} else {pui_native_196} }; if !(pui_native_194) {{ let pui_native_209={ let pui_native_211:f64=v_696e6465785f5f63383336636366396537373937356266.clone(); let pui_native_212:f64=1.0_f64; pui_native_211 % pui_native_212 }; let pui_native_210=0.0_f64; !(pui_native_209==pui_native_210) }} else {pui_native_194} }; if !(pui_native_192) {!((v_78.clone()).is_finite())} else {pui_native_192} }; if !(pui_native_190) {!((v_79.clone()).is_finite())} else {pui_native_190} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":129:102
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":130:13
                let v_7469636b65745f5f66316139366230633865353735653634: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":131:7
                { let v_7075695f6e61746976655f323133 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f323133.clone(), Value::number(v_696e6465785f5f63383336636366396537373937356266.clone()), Value::string("reason: collection context cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":132:7
                if { let pui_native_214={ let pui_native_216=v_7469636b65745f5f66316139366230633865353735653634.clone(); let pui_native_217={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_216==pui_native_217) }; if !(pui_native_214) {!((v_7265616479.read().call)())} else {pui_native_214} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":132:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":133:7
                { let v_7075695f6e61746976655f323138 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323138.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":134:7
                if { let pui_native_219={ let pui_native_221=v_7469636b65745f5f66316139366230633865353735653634.clone(); let pui_native_222={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_221==pui_native_222) }; if !(pui_native_219) {!((v_7265616479.read().call)())} else {pui_native_219} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":134:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":135:7
                { let v_7075695f6e61746976655f323233 = v_6f776e65725f5f34333637383931663863643364353464.clone(); emit(&(v_7075695f6e61746976655f323233.clone()).owner(), Value::string("context"), record(vec![("index", Value::number(v_696e6465785f5f63383336636366396537373937356266.clone())), ("x", Value::number(v_78.clone())), ("y", Value::number(v_79.clone())), ("keyboard", Value::boolean(v_6b6579626f617264.clone())), ("revision", v_62617369735f5f35373233316461656662626465313432.clone())])) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":136:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":138:11
            v_6b65795f5f63636538616136363530633839623964.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let v_73656c656374 = v_73656c656374.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let v_636f6e746578744174 = v_636f6e746578744174.clone(); let call:Rc<dyn Fn(Value, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool| -> bool {
                let v_6e616d655f5f39663033646662643236656166323335: Value = argument_0;
                let v_73686966745f5f35346239656133393965376336626438: bool = argument_1;
                let v_636f6d6d616e645f5f34323363326161366465626334663932: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":139:7
                if { let pui_native_224=!((v_7265616479.read().call)()); if !(pui_native_224) {!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy())} else {pui_native_224} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":139:33
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":140:13
                let v_6d6f64655f5f38336631336234323834663464633562 = { let left = ({ let v_7075695f6e61746976655f323236 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f323236.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":141:7
                if { let pui_native_227={ let pui_native_229=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("filterlist"))))).bool(); if (pui_native_229) {v_636f6d6d616e645f5f34323363326161366465626334663932.clone()} else {pui_native_229} }; if (pui_native_227) {{ let pui_native_233=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("f"))))).bool(); if !(pui_native_233) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F"))))).bool()} else {pui_native_233} }} else {pui_native_227} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":141:81
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f323339 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323339.clone()).get()).num()) }).num(), false, Value::string("focus-filter"));
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":141:135
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":142:7
                if { let pui_native_240=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tracklist"))))).bool(); if (pui_native_240) {{ let pui_native_244=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ContextMenu"))))).bool(); if !(pui_native_244) {{ let pui_native_248=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F10"))))).bool(); if (pui_native_248) {v_73686966745f5f35346239656133393965376336626438.clone()} else {pui_native_248} }} else {pui_native_244} }} else {pui_native_240} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":142:90
                    return (v_636f6e746578744174.read().call)(({ let v_7075695f6e61746976655f323532 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323532.clone()).get()).num()) }).num(), { let left = ({ let v_7075695f6e61746976655f323533 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f323533.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }, 0.0_f64, 0.0_f64, true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":143:7
                if { let pui_native_254=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).bool(); if (pui_native_254) {{ let pui_native_258=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_258) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_258} }} else {pui_native_254} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":143:75
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f323634 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323634.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":143:117
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":144:7
                if { let pui_native_265=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).bool(); if (pui_native_265) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab"))))).bool()} else {pui_native_265} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":145:15
                    let v_5f5f7075694d6f64756c6530: f64 = { let pui_native_271:f64=({ let v_7075695f6e61746976655f323733 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323733.clone()).get()).num()) }).num(); let pui_native_272:f64=if v_73686966745f5f35346239656133393965376336626438.clone() {-(1.0_f64)} else {1.0_f64}; pui_native_271 + pui_native_272 };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":146:9
                    if { let pui_native_274={ let pui_native_276:f64=v_5f5f7075694d6f64756c6530.clone(); let pui_native_277:f64=0.0_f64; pui_native_276 < pui_native_277 }; if !(pui_native_274) {{ let pui_native_278:f64=v_5f5f7075694d6f64756c6530.clone(); let pui_native_279:f64=(v_636f756e74.read().call)(); pui_native_278 >= pui_native_279 }} else {pui_native_274} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":146:42
                        (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f323830 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323830.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":147:14
                        Value::boolean((v_73656c656374.read().call)(v_5f5f7075694d6f64756c6530.clone(), false, true));
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":148:9
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":150:7
                if { let pui_native_281=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).bool(); if (pui_native_281) {{ let pui_native_285={ let pui_native_287={ let pui_native_289={ let pui_native_291=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_291) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_291} }; if !(pui_native_289) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("+"))))).bool()} else {pui_native_289} }; if !(pui_native_287) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("="))))).bool()} else {pui_native_287} }; if !(pui_native_285) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("-"))))).bool()} else {pui_native_285} }} else {pui_native_281} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":150:128
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f333033 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333033.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":150:172
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":151:7
                if { let pui_native_304=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_304) {{ let pui_native_308=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab"))))).bool(); if (pui_native_308) {(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu"))))).bool()} else {pui_native_308} }} else {pui_native_304} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:71
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f333134 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333134.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:113
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":152:7
                if { let pui_native_315={ let pui_native_317=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Enter"))))).bool(); if !(pui_native_317) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string(" "))))).bool()} else {pui_native_317} }; if !(pui_native_315) {{ let pui_native_323=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F2"))))).bool(); if (pui_native_323) {(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table"))))).bool()} else {pui_native_323} }} else {pui_native_315} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":152:84
                    return (v_63686f6f7365.read().call)(({ let v_7075695f6e61746976655f333239 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333239.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":153:7
                if { let pui_native_330=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table"))))).bool(); if (pui_native_330) {{ let pui_native_334={ let pui_native_336=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Delete"))))).bool(); if !(pui_native_336) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_336} }; if !(pui_native_334) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab"))))).bool()} else {pui_native_334} }} else {pui_native_330} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:9
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f333434 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333434.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:53
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":156:7
                if { let pui_native_345={ let pui_native_347=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table"))))).bool(); if (pui_native_347) {v_636f6d6d616e645f5f34323363326161366465626334663932.clone()} else {pui_native_347} }; if (pui_native_345) {{ let pui_native_351={ let pui_native_353={ let pui_native_355=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft"))))).bool(); if !(pui_native_355) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_355} }; if !(pui_native_353) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_353} }; if !(pui_native_351) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_351} }} else {pui_native_345} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":157:9
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f333635 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333635.clone()).get()).num()) }).num(), v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":157:53
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":159:7
                if { let pui_native_366=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu"))))).bool(); if (pui_native_366) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_366} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":160:9
                    if { let pui_native_372:f64=({ let v_7075695f6e61746976655f333734 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f333734.clone()).get()).num()) }).num(); let pui_native_373:f64=0.0_f64; pui_native_372 < pui_native_373 } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":160:33
                        return false;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":161:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(-(1.0_f64), v_6e616d655f5f39663033646662643236656166323335.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":163:7
                if { let pui_native_375=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu"))))).bool(); if (pui_native_375) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_375} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":164:9
                    if { let pui_native_381=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f333833 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333833.clone()).get()).num()) }).num()); let pui_native_382=2.0_f64; !(pui_native_381==pui_native_382) } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":164:39
                        return false;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":165:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(({ let v_7075695f6e61746976655f333834 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333834.clone()).get()).num()) }).num(), v_6e616d655f5f39663033646662643236656166323335.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":167:11
                let v_6e6578745f5f39303365663633396366633434303531: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(-(1.0_f64)));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":168:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Home"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":168:28
                    Value::number({ let value=(v_7365656b.read().call)(0.0_f64, 1.0_f64, (v_636f756e74.read().call)(), false); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":169:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("End"))))).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":169:32
                        Value::number({ let value=(v_7365656b.read().call)({ let pui_native_389:f64=(v_636f756e74.read().call)(); let pui_native_390:f64=1.0_f64; pui_native_389 - pui_native_390 }, -(1.0_f64), (v_636f756e74.read().call)(), false); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":170:12
                        if { let pui_native_391=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_391) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_391} } {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":170:60
                            Value::number({ let value=(v_7365656b.read().call)({ let pui_native_397:f64=({ let v_7075695f6e61746976655f333939 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333939.clone()).get()).num()) }).num(); let pui_native_398:f64=1.0_f64; pui_native_397 - pui_native_398 }, -(1.0_f64), (v_636f756e74.read().call)(), { let pui_native_400=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu"))))).bool(); if !(pui_native_400) {(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tablist"))))).bool()} else {pui_native_400} }); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":171:12
                            if { let pui_native_406=(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown"))))).bool(); if !(pui_native_406) {(Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_406} } {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":171:63
                                Value::number({ let value=(v_7365656b.read().call)({ let pui_native_412:f64=({ let v_7075695f6e61746976655f343134 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343134.clone()).get()).num()) }).num(); let pui_native_413:f64=1.0_f64; pui_native_412 + pui_native_413 }, 1.0_f64, (v_636f756e74.read().call)(), { let pui_native_415=(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu"))))).bool(); if !(pui_native_415) {(Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tablist"))))).bool()} else {pui_native_415} }); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":172:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageUp"))))).bool() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":172:35
                                    Value::number({ let value=(v_7365656b.read().call)((v_6d6178.read().call)(0.0_f64, { let pui_native_423:f64=({ let v_7075695f6e61746976655f343235 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343235.clone()).get()).num()) }).num(); let pui_native_424:f64=if (Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).bool() {4.0_f64} else {20.0_f64}; pui_native_423 - pui_native_424 }), -(1.0_f64), (v_636f756e74.read().call)(), false); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":173:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageDown"))))).bool() {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":173:37
                                        Value::number({ let value=(v_7365656b.read().call)((v_6d696e.read().call)({ let pui_native_430:f64=(v_636f756e74.read().call)(); let pui_native_431:f64=1.0_f64; pui_native_430 - pui_native_431 }, { let pui_native_432:f64=({ let v_7075695f6e61746976655f343334 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343334.clone()).get()).num()) }).num(); let pui_native_433:f64=if (Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).bool() {4.0_f64} else {20.0_f64}; pui_native_432 + pui_native_433 }), 1.0_f64, (v_636f756e74.read().call)(), false); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut()=value.clone(); value });
                                    } else {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":174:12
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":175:7
                if { let pui_native_437={ let pui_native_439:f64={ let value=(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }; let pui_native_440:f64=0.0_f64; pui_native_439 >= pui_native_440 }; if (pui_native_437) {!((v_73656c656374.read().call)({ let value=(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }, v_73686966745f5f35346239656133393965376336626438.clone(), true))} else {pui_native_437} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":175:52
                    return true;
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":176:12
                    if { let pui_native_441={ let pui_native_443={ let pui_native_445={ let pui_native_447:f64={ let value=(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }; let pui_native_448:f64=0.0_f64; pui_native_447 < pui_native_448 }; if (pui_native_445) {!(v_73686966745f5f35346239656133393965376336626438.clone())} else {pui_native_445} }; if (pui_native_443) {{ let pui_native_449:f64=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f343531 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343531.clone()).get()).num()) }).num()); let pui_native_450:f64=0.0_f64; pui_native_449 > pui_native_450 }} else {pui_native_443} }; if (pui_native_441) {!((v_73656c656374.read().call)(({ let v_7075695f6e61746976655f343532 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343532.clone()).get()).num()) }).num(), false, true))} else {pui_native_441} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":176:100
                        return true;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":177:7
                if !((v_7265616479.read().call)()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":177:21
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":178:13
                let v_686f72697a6f6e74616c: bool = (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f343535 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f343535.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f343535.clone()).owner(), false) } }).member("orientation", true)), &(Value::string("horizontal"))))).bool();
                // Source "frontend/proto-ui/src/collection-control.proto.ts":179:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageUp"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":179:32
                    if v_686f72697a6f6e74616c.clone() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":179:48
                        { let v_7075695f6e61746976655f343538 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343538.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":179:128
                        { let v_7075695f6e61746976655f343539 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343539.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":180:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageDown"))))).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":180:39
                        if v_686f72697a6f6e74616c.clone() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":180:55
                            { let v_7075695f6e61746976655f343632 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343632.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":180:134
                            { let v_7075695f6e61746976655f343633 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343633.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":181:12
                        if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Home"))))).bool() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":181:35
                            if v_686f72697a6f6e74616c.clone() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":181:51
                                { let v_7075695f6e61746976655f343636 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343636.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0_f64))])) };
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":181:121
                                { let v_7075695f6e61746976655f343637 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343637.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0_f64))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":182:12
                            if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("End"))))).bool() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":182:34
                                if v_686f72697a6f6e74616c.clone() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":182:50
                                    { let v_7075695f6e61746976655f343730 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343730.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":182:111
                                    { let v_7075695f6e61746976655f343731 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f343731.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":183:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":188:5
            { let v_7075695f6e61746976655f343732 = v_646566.clone(); expose_method(&owner, Value::string("filterKey"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool, bool, Value, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: Value, argument_6: bool| -> bool {
                let v_6e616d655f5f37613536633837626431663532366332: Value = argument_0;
                let v_73686966745f5f38353264393838616565376537356239: bool = argument_1;
                let v_636f6d6d616e645f5f32366138313638353133626133353639: bool = argument_2;
                let v_636f6d706f73696e67: bool = argument_3;
                let v_66726f6d46696c746572: bool = argument_4;
                let v_62617369735f5f38653339326536643161626362663536: Value = argument_5;
                let v_616c74: bool = argument_6;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":189:13
                let v_6f776e65725f5f61633262346635326137313438303332 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":190:7
                if { let pui_native_473={ let pui_native_475={ let pui_native_477={ let pui_native_479={ let pui_native_481=!((v_6f776e65725f5f61633262346635326137313438303332.clone()).truthy()); if !(pui_native_481) {!((v_7265616479.read().call)())} else {pui_native_481} }; if !(pui_native_479) {v_636f6d706f73696e67.clone()} else {pui_native_479} }; if !(pui_native_477) {v_616c74.clone()} else {pui_native_477} }; if !(pui_native_475) {(Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f343835 = v_6f776e65725f5f61633262346635326137313438303332.clone(); read_props(&(v_7075695f6e61746976655f343835.clone()).owner(), false) }).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_475} }; if !(pui_native_473) {(Value::boolean(!strict_equal(&(v_62617369735f5f38653339326536643161626362663536.clone()), &({ let left = ({ let v_7075695f6e61746976655f343838 = v_6f776e65725f5f61633262346635326137313438303332.clone(); read_props(&(v_7075695f6e61746976655f343838.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_473} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":190:142
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":191:7
                if { let pui_native_489=v_636f6d6d616e645f5f32366138313638353133626133353639.clone(); if (pui_native_489) {{ let pui_native_491=(Value::boolean(strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("f"))))).bool(); if !(pui_native_491) {(Value::boolean(strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("F"))))).bool()} else {pui_native_491} }} else {pui_native_489} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":191:54
                    return (v_6b65795f5f63636538616136363530633839623964.read().call)(v_6e616d655f5f37613536633837626431663532366332.clone(), v_73686966745f5f38353264393838616565376537356239.clone(), true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":192:7
                if { let pui_native_497=!(v_66726f6d46696c746572.clone()); if !(pui_native_497) {v_636f6d6d616e645f5f32366138313638353133626133353639.clone()} else {pui_native_497} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":192:35
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":193:7
                if { let pui_native_499={ let pui_native_501={ let pui_native_503=(Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("ArrowUp"))))).bool(); if (pui_native_503) {(Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_503} }; if (pui_native_501) {(Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_501} }; if (pui_native_499) {(Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("Escape"))))).bool()} else {pui_native_499} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":193:96
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":194:7
                return (v_6b65795f5f63636538616136363530633839623964.read().call)(v_6e616d655f5f37613536633837626431663532366332.clone(), v_73686966745f5f38353264393838616565376537356239.clone(), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), arg(&arguments,5), (arg(&arguments,6)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":199:5
            { let v_7075695f6e61746976655f353133 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), ({  let v_637572736f72 = v_637572736f72.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: Value| -> bool {
                let v_696e6465785f5f61613236653065393430303964373333: f64 = argument_0;
                let v_73746172745f5f62326431336666663833346661633834: f64 = argument_1;
                let v_656e645f5f36663731373364653138373330666231: f64 = argument_2;
                let v_76696577706f72745374617274: f64 = argument_3;
                let v_76696577706f7274456e64: f64 = argument_4;
                let v_62617369735f5f37396562613432663433336531656363: Value = argument_5;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":200:13
                let v_6f776e65725f5f65616234323132303862343434376265 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":201:7
                if { let pui_native_514={ let pui_native_516={ let pui_native_518={ let pui_native_520={ let pui_native_522={ let pui_native_524={ let pui_native_526={ let pui_native_528={ let pui_native_530={ let pui_native_532=!((v_6f776e65725f5f65616234323132303862343434376265.clone()).truthy()); if !(pui_native_532) {!((v_7265616479.read().call)())} else {pui_native_532} }; if !(pui_native_530) {(Value::boolean(!strict_equal(&(v_62617369735f5f37396562613432663433336531656363.clone()), &({ let left = ({ let v_7075695f6e61746976655f353336 = v_6f776e65725f5f65616234323132303862343434376265.clone(); read_props(&(v_7075695f6e61746976655f353336.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_530} }; if !(pui_native_528) {{ let pui_native_537=v_696e6465785f5f61613236653065393430303964373333.clone(); let pui_native_538=({ let v_7075695f6e61746976655f353339 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f353339.clone()).get()).num()) }).num(); !(pui_native_537==pui_native_538) }} else {pui_native_528} }; if !(pui_native_526) {{ let pui_native_540=(v_6b696e64.read().call)(v_696e6465785f5f61613236653065393430303964373333.clone()); let pui_native_541=0.0_f64; (pui_native_540==pui_native_541) }} else {pui_native_526} }; if !(pui_native_524) {!((v_73746172745f5f62326431336666663833346661633834.clone()).is_finite())} else {pui_native_524} }; if !(pui_native_522) {!((v_656e645f5f36663731373364653138373330666231.clone()).is_finite())} else {pui_native_522} }; if !(pui_native_520) {!((v_76696577706f72745374617274.clone()).is_finite())} else {pui_native_520} }; if !(pui_native_518) {!((v_76696577706f7274456e64.clone()).is_finite())} else {pui_native_518} }; if !(pui_native_516) {{ let pui_native_542:f64=v_656e645f5f36663731373364653138373330666231.clone(); let pui_native_543:f64=v_73746172745f5f62326431336666663833346661633834.clone(); pui_native_542 <= pui_native_543 }} else {pui_native_516} }; if !(pui_native_514) {{ let pui_native_544:f64=v_76696577706f7274456e64.clone(); let pui_native_545:f64=v_76696577706f72745374617274.clone(); pui_native_544 <= pui_native_545 }} else {pui_native_514} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":203:58
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":204:7
                if { let pui_native_546={ let pui_native_548={ let pui_native_550:f64=v_73746172745f5f62326431336666663833346661633834.clone(); let pui_native_551:f64=v_76696577706f72745374617274.clone(); pui_native_550 >= pui_native_551 }; if (pui_native_548) {{ let pui_native_552:f64=v_656e645f5f36663731373364653138373330666231.clone(); let pui_native_553:f64=v_76696577706f7274456e64.clone(); pui_native_552 <= pui_native_553 }} else {pui_native_548} }; if !(pui_native_546) {{ let pui_native_554={ let pui_native_556:f64=v_73746172745f5f62326431336666663833346661633834.clone(); let pui_native_557:f64=v_76696577706f72745374617274.clone(); pui_native_556 <= pui_native_557 }; if (pui_native_554) {{ let pui_native_558:f64=v_656e645f5f36663731373364653138373330666231.clone(); let pui_native_559:f64=v_76696577706f7274456e64.clone(); pui_native_558 >= pui_native_559 }} else {pui_native_554} }} else {pui_native_546} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":204:109
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:13
                let v_6c656164696e67: f64 = { let pui_native_560:f64=v_73746172745f5f62326431336666663833346661633834.clone(); let pui_native_561:f64=v_76696577706f72745374617274.clone(); pui_native_560 - pui_native_561 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:52
                let v_747261696c696e67: f64 = { let pui_native_562:f64=v_656e645f5f36663731373364653138373330666231.clone(); let pui_native_563:f64=v_76696577706f7274456e64.clone(); pui_native_562 - pui_native_563 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":206:13
                let v_6c656164696e6744697374616e6365: f64 = if { let pui_native_564:f64=v_6c656164696e67.clone(); let pui_native_565:f64=0.0_f64; pui_native_564 < pui_native_565 } {-(v_6c656164696e67.clone())} else {v_6c656164696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":206:71
                let v_747261696c696e6744697374616e6365: f64 = if { let pui_native_566:f64=v_747261696c696e67.clone(); let pui_native_567:f64=0.0_f64; pui_native_566 < pui_native_567 } {-(v_747261696c696e67.clone())} else {v_747261696c696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":207:13
                let v_64656c7461: f64 = if { let pui_native_568:f64=v_6c656164696e6744697374616e6365.clone(); let pui_native_569:f64=v_747261696c696e6744697374616e6365.clone(); pui_native_568 < pui_native_569 } {v_6c656164696e67.clone()} else {v_747261696c696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":208:7
                if (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f353732 = v_6f776e65725f5f65616234323132303862343434376265.clone(); read_props(&(v_7075695f6e61746976655f353732.clone()).owner(), false) }).member("orientation", false)), &(Value::string("horizontal"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":208:59
                    { let v_7075695f6e61746976655f353733 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353733.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(v_64656c7461.clone()))])) };
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":209:12
                    { let v_7075695f6e61746976655f353734 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353734.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(v_64656c7461.clone()))])) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":210:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).num(), arg(&arguments,5)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":212:5
            { let v_7075695f6e61746976655f353735 = v_646566.clone(); expose_method(&owner, Value::string("setItems"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6974656d73 = v_6974656d73.clone(); let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let v_73796e63 = v_73796e63.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6e6578745f5f64396266323461323332323535373463: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":213:7
                if { let pui_native_576={ let pui_native_578={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_578) {{ let pui_native_580:f64=((v_6e6578745f5f64396266323461323332323535373463.clone()).member("length", false)).num(); let pui_native_581:f64=1000000.0_f64; pui_native_580 > pui_native_581 }} else {pui_native_578} }; if !(pui_native_576) {!((v_76616c69644974656d73.read().call)(v_6e6578745f5f64396266323461323332323535373463.clone(), 0.0_f64, ((v_6e6578745f5f64396266323461323332323535373463.clone()).member("length", false)).num()))} else {pui_native_576} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":213:83
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:16
                { let value = v_6e6578745f5f64396266323461323332323535373463.clone(); *(v_6974656d73.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":215:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":215:20
                    v_73796e63.read().call_authored(vec![{ let value = (v_63757272656e74.read()).borrow().clone(); value }]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":216:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":218:5
            { let v_7075695f6e61746976655f353832 = v_646566.clone(); expose_method(&owner, Value::string("choose"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f38656563363332363635383836383033: f64 = argument_0;
                let v_657874656e645f5f34613362383437633732356230383136: bool = argument_1;
                let v_62617369735f5f34373939326232646538653363663566: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":219:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f34373939326232646538653363663566.clone()), &({ let left = ({ let v_7075695f6e61746976655f353835 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f353835.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f353835.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":219:60
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:7
                return (v_63686f6f7365.read().call)(v_696e6465785f5f38656563363332363635383836383033.clone(), v_657874656e645f5f34613362383437633732356230383136.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":222:5
            { let v_7075695f6e61746976655f353836 = v_646566.clone(); expose_method(&owner, Value::string("selectIndex"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f30663930393332303932616130326437: f64 = argument_0;
                let v_657874656e645f5f62646665313032383436333735333831: bool = argument_1;
                let v_62617369735f5f33623766616636313731323030623061: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":223:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f33623766616636313731323030623061.clone()), &({ let left = ({ let v_7075695f6e61746976655f353839 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f353839.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f353839.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":223:60
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":224:7
                return (v_73656c656374.read().call)(v_696e6465785f5f30663930393332303932616130326437.clone(), v_657874656e645f5f62646665313032383436333735333831.clone(), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":228:5
            { let v_7075695f6e61746976655f353930 = v_646566.clone(); expose_method(&owner, Value::string("keyAt"), ({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); let call:Rc<dyn Fn(f64, Value, bool, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value, argument_2: bool, argument_3: bool, argument_4: Value| -> bool {
                let v_696e6465785f5f35316630646638376537343663303363: f64 = argument_0;
                let v_6e616d655f5f32346234353030616138373332366563: Value = argument_1;
                let v_73686966745f5f62303936626531363166323339636132: bool = argument_2;
                let v_636f6d6d616e645f5f31636337376335343432396662636564: bool = argument_3;
                let v_62617369735f5f34646338376562343738383931353262: Value = argument_4;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":229:7
                if { let pui_native_591={ let pui_native_593={ let pui_native_595=!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !(pui_native_595) {(Value::boolean(!strict_equal(&(v_62617369735f5f34646338376562343738383931353262.clone()), &({ let left = ({ let v_7075695f6e61746976655f353939 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f353939.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_595} }; if !(pui_native_593) {!((v_7265616479.read().call)())} else {pui_native_593} }; if !(pui_native_591) {{ let pui_native_600=(v_6b696e64.read().call)(v_696e6465785f5f35316630646638376537343663303363.clone()); let pui_native_601=0.0_f64; (pui_native_600==pui_native_601) }} else {pui_native_591} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":229:104
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:13
                let v_6d6f64655f5f63613732626137346136613864333732 = { let left = ({ let v_7075695f6e61746976655f363032 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f363032.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":231:13
                let v_68616e646c6564: bool = { let pui_native_603={ let pui_native_605={ let pui_native_607={ let pui_native_609={ let pui_native_611={ let pui_native_613={ let pui_native_615={ let pui_native_617={ let pui_native_619={ let pui_native_621={ let pui_native_623={ let pui_native_625={ let pui_native_627={ let pui_native_629={ let pui_native_631=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_631) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_631} }; if !(pui_native_629) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string(" "))))).bool()} else {pui_native_629} }; if !(pui_native_627) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Home"))))).bool()} else {pui_native_627} }; if !(pui_native_625) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("End"))))).bool()} else {pui_native_625} }; if !(pui_native_623) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_623} }; if !(pui_native_621) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_621} }; if !(pui_native_619) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_619} }; if !(pui_native_617) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_617} }; if !(pui_native_615) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("PageUp"))))).bool()} else {pui_native_615} }; if !(pui_native_613) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("PageDown"))))).bool()} else {pui_native_613} }; if !(pui_native_611) {{ let pui_native_655=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Tab"))))).bool(); if (pui_native_655) {{ let pui_native_659={ let pui_native_661=(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("points"))))).bool(); if !(pui_native_661) {(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("table"))))).bool()} else {pui_native_661} }; if !(pui_native_659) {(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("menu"))))).bool()} else {pui_native_659} }} else {pui_native_655} }} else {pui_native_611} }; if !(pui_native_609) {{ let pui_native_669=(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("table"))))).bool(); if (pui_native_669) {{ let pui_native_673={ let pui_native_675=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F2"))))).bool(); if !(pui_native_675) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Delete"))))).bool()} else {pui_native_675} }; if !(pui_native_673) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_673} }} else {pui_native_669} }} else {pui_native_609} }; if !(pui_native_607) {{ let pui_native_683=(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("points"))))).bool(); if (pui_native_683) {{ let pui_native_687={ let pui_native_689=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("+"))))).bool(); if !(pui_native_689) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("="))))).bool()} else {pui_native_689} }; if !(pui_native_687) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("-"))))).bool()} else {pui_native_687} }} else {pui_native_683} }} else {pui_native_607} }; if !(pui_native_605) {{ let pui_native_697={ let pui_native_699=(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("filterlist"))))).bool(); if (pui_native_699) {v_636f6d6d616e645f5f31636337376335343432396662636564.clone()} else {pui_native_699} }; if (pui_native_697) {{ let pui_native_703=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("f"))))).bool(); if !(pui_native_703) {(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F"))))).bool()} else {pui_native_703} }} else {pui_native_697} }} else {pui_native_605} }; if !(pui_native_603) {{ let pui_native_709=(Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("tracklist"))))).bool(); if (pui_native_709) {{ let pui_native_713=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ContextMenu"))))).bool(); if !(pui_native_713) {{ let pui_native_717=(Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F10"))))).bool(); if (pui_native_717) {v_73686966745f5f62303936626531363166323339636132.clone()} else {pui_native_717} }} else {pui_native_713} }} else {pui_native_709} }} else {pui_native_603} };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":238:7
                if !(v_68616e646c6564.clone()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":238:21
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":239:13
                let v_7469636b65745f5f62623365636533376534346261656537: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":240:7
                { let v_7075695f6e61746976655f373231 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f373231.clone(), Value::number(v_696e6465785f5f35316630646638376537343663303363.clone()), Value::string("reason: addressed collection key")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":241:7
                if { let pui_native_722={ let pui_native_724=v_7469636b65745f5f62623365636533376534346261656537.clone(); let pui_native_725={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_724==pui_native_725) }; if !(pui_native_722) {!((v_7265616479.read().call)())} else {pui_native_722} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":241:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":242:7
                (v_656d6974.read().call)(Value::string("select"), v_696e6465785f5f35316630646638376537343663303363.clone(), false, Value::string(""));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":243:7
                if { let pui_native_726={ let pui_native_728={ let pui_native_730=v_7469636b65745f5f62623365636533376534346261656537.clone(); let pui_native_731={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_730==pui_native_731) }; if !(pui_native_728) {!((v_7265616479.read().call)())} else {pui_native_728} }; if !(pui_native_726) {(Value::boolean(!strict_equal(&(v_62617369735f5f34646338376562343738383931353262.clone()), &({ let left = ({ let v_7075695f6e61746976655f373334 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f373334.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f373334.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_726} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":243:92
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":244:7
                return (v_6b65795f5f63636538616136363530633839623964.read().call)(v_6e616d655f5f32346234353030616138373332366563.clone(), v_73686966745f5f62303936626531363166323339636132.clone(), v_636f6d6d616e645f5f31636337376335343432396662636564.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), arg(&arguments,4)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":246:5
            { let v_7075695f6e61746976655f373335 = v_646566.clone(); expose_method(&owner, Value::string("selectAndFocus"), ({  let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f39306136653737656637623231623938: f64 = argument_0;
                let v_657874656e645f5f31613534383230376165646161343561: bool = argument_1;
                let v_62617369735f5f65336261643632616138346237366562: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":247:7
                if { let pui_native_736=!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !(pui_native_736) {(Value::boolean(!strict_equal(&(v_62617369735f5f65336261643632616138346237366562.clone()), &({ let left = ({ let v_7075695f6e61746976655f373430 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f373430.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_736} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":247:71
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":248:7
                if { let pui_native_741=!((v_73656c656374.read().call)(v_696e6465785f5f39306136653737656637623231623938.clone(), v_657874656e645f5f31613534383230376165646161343561.clone(), true)); if !(pui_native_741) {(Value::boolean(!strict_equal(&(v_62617369735f5f65336261643632616138346237366562.clone()), &({ let left = ({ let v_7075695f6e61746976655f373435 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f373435.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f373435.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_741} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":248:92
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":249:7
                { let v_7075695f6e61746976655f373436 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f373436.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":250:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":252:5
            { let v_7075695f6e61746976655f373437 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b65795f5f63636538616136363530633839623964.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":253:5
            { let v_7075695f6e61746976655f373438 = v_646566.clone(); expose_method(&owner, Value::string("contextAt"), (v_636f6e746578744174.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":254:5
            { let v_7075695f6e61746976655f373439 = v_646566.clone(); expose_method(&owner, Value::string("reconcile"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_73796e63 = v_73796e63.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:50
                let v_6f776e65725f5f37393964386233353865383061383463 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:67
                if { let pui_native_750=!((v_6f776e65725f5f37393964386233353865383061383463.clone()).truthy()); if !(pui_native_750) {!((v_7265616479.read().call)())} else {pui_native_750} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":254:91
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:105
                v_73796e63.read().call_authored(vec![v_6f776e65725f5f37393964386233353865383061383463.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:118
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":255:5
            { let v_7075695f6e61746976655f373532 = v_646566.clone(); expose_method(&owner, Value::string("dismiss"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":255:42
                if (v_7265616479.read().call)() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":255:55
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f373533 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f373533.clone()).get()).num()) }).num(), false, Value::string("outside"));
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":256:5
            { let v_7075695f6e61746976655f373534 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":256:44
                if (v_7265616479.read().call)() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":256:57
                    { let v_7075695f6e61746976655f373535 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f373535.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":257:5
            { let v_7075695f6e61746976655f373536 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":258:7
                if (v_6b65795f5f63636538616136363530633839623964.read().call)({ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), { let pui_native_759=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_759) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_759} }) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":258:108
                    { let v_7075695f6e61746976655f373635 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f373635.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":260:11
            v_73796e63.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_7461626c65 = v_7461626c65.clone(); let v_726f7773 = v_726f7773.clone(); let v_636f6c756d6e73 = v_636f6c756d6e73.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f37306636363434393364333530653437: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":261:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":261:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:13
                let v_7469636b65745f5f30646466366662663930636166613963: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:31
                { let value = v_72756e5f5f37306636363434393364333530653437.clone(); *(v_63757272656e74.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":263:13
                let v_70726f7073 = { let v_7075695f6e61746976655f373636 = v_72756e5f5f37306636363434393364333530653437.clone(); read_props(&(v_7075695f6e61746976655f373636.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":264:7
                { let v_7075695f6e61746976655f373637 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f373637.clone(), if (Value::boolean({ let pui_native_768={ let pui_native_770=(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("points"))))).bool(); if !(pui_native_770) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("tracklist"))))).bool()} else {pui_native_770} }; if !(pui_native_768) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_768} })).truthy() { Value::string("listbox") } else { { let left = (v_70726f7073.clone()).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } } }, Value::string("reason: collection role")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":265:7
                if { let pui_native_778={ let pui_native_780={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_781=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_780==pui_native_781) }; if !(pui_native_778) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_778} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":265:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":266:7
                { let v_7075695f6e61746976655f373832 = v_7461626c65.read(); state_set(v_7075695f6e61746976655f373832.clone(), Value::boolean((Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("table"))))).bool()), Value::string("reason: collection table semantics")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":267:7
                if { let pui_native_785={ let pui_native_787={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_788=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_787==pui_native_788) }; if !(pui_native_785) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_785} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":267:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":268:7
                { let v_7075695f6e61746976655f373839 = v_726f7773.read(); state_set(v_7075695f6e61746976655f373839.clone(), Value::number((v_636f756e74.read().call)()), Value::string("reason: collection rows")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":269:7
                if { let pui_native_790={ let pui_native_792={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_793=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_792==pui_native_793) }; if !(pui_native_790) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_790} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":269:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":270:7
                { let v_7075695f6e61746976655f373934 = v_636f6c756d6e73.read(); state_set(v_7075695f6e61746976655f373934.clone(), Value::number(((v_70726f7073.clone()).member("columns", false)).num()), Value::string("reason: collection columns")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":271:7
                if { let pui_native_797={ let pui_native_799={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_800=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_799==pui_native_800) }; if !(pui_native_797) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_797} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":271:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":272:7
                { let v_7075695f6e61746976655f383031 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f383031.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection name")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":273:7
                if { let pui_native_802={ let pui_native_804={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_805=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_804==pui_native_805) }; if !(pui_native_802) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_802} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":273:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":274:7
                { let v_7075695f6e61746976655f383036 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f383036.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection description")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:7
                if { let pui_native_807={ let pui_native_809={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_810=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_809==pui_native_810) }; if !(pui_native_807) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_807} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":275:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":276:7
                { let v_7075695f6e61746976655f383131 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f383131.clone(), Value::boolean(((v_70726f7073.clone()).member("disabled", false)).bool()), Value::string("reason: collection policy")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":277:7
                if { let pui_native_814={ let pui_native_816={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_817=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_816==pui_native_817) }; if !(pui_native_814) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_814} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":277:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":278:7
                { let v_7075695f6e61746976655f383138 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f383138.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f383139 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f383139.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":279:7
                if { let pui_native_820={ let pui_native_822={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_823=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_822==pui_native_823) }; if !(pui_native_820) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_820} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":279:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":280:13
                let v_7265717565737465645f5f38633239316230636561396132643234: f64 = ((v_70726f7073.clone()).member("cursor", false)).num();
                // Source "frontend/proto-ui/src/collection-control.proto.ts":281:13
                let v_6163636570746564: f64 = if { let pui_native_826=(v_7265717565737465645f5f38633239316230636561396132643234.clone()).is_finite(); if (pui_native_826) {{ let pui_native_828={ let pui_native_830:f64=v_7265717565737465645f5f38633239316230636561396132643234.clone(); let pui_native_831:f64=1.0_f64; pui_native_830 % pui_native_831 }; let pui_native_829=0.0_f64; (pui_native_828==pui_native_829) }} else {pui_native_826} } {(v_6d6178.read().call)(0.0_f64, (v_6d696e.read().call)({ let pui_native_832:f64=(v_636f756e74.read().call)(); let pui_native_833:f64=1.0_f64; pui_native_832 - pui_native_833 }, v_7265717565737465645f5f38633239316230636561396132643234.clone()))} else {0.0_f64};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":282:7
                { let v_7075695f6e61746976655f383334 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f383334.clone(), Value::number(if { let pui_native_835:f64=(v_6b696e64.read().call)(v_6163636570746564.clone()); let pui_native_836:f64=0.0_f64; pui_native_835 > pui_native_836 } {v_6163636570746564.clone()} else {(v_6d6178.read().call)(0.0_f64, (v_7365656b.read().call)(0.0_f64, 1.0_f64, (v_636f756e74.read().call)(), false))}), Value::string("reason: accepted collection cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":283:7
                if { let pui_native_837={ let pui_native_839={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_840=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_839==pui_native_840) }; if !(pui_native_837) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_837} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":283:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":284:7
                { let v_7075695f6e61746976655f383431 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f383431.clone(), Value::number(((v_70726f7073.clone()).member("expanded", false)).num()), Value::string("reason: accepted expanded item")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":285:7
                if { let pui_native_844={ let pui_native_846={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_847=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_846==pui_native_847) }; if !(pui_native_844) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_844} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":285:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":289:7
                if { let pui_native_848=(v_7265616479.read().call)(); if (pui_native_848) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_848} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":290:9
                    if { let pui_native_852=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f383534 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383534.clone()).get()).num()) }).num()); let pui_native_853=0.0_f64; (pui_native_852==pui_native_853) } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":290:39
                        (v_656d6974.read().call)(Value::string("select"), -(1.0_f64), false, Value::string("reconcile"));
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":291:14
                        if { let pui_native_855=((v_70726f7073.clone()).member("cursor", false)).num(); let pui_native_856=({ let v_7075695f6e61746976655f383537 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383537.clone()).get()).num()) }).num(); !(pui_native_855==pui_native_856) } {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":291:49
                            (v_656d6974.read().call)(Value::string("select"), ({ let v_7075695f6e61746976655f383538 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383538.clone()).get()).num()) }).num(), false, Value::string("reconcile"));
                        }
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":292:9
                    if { let pui_native_859={ let pui_native_861={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_862=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_861==pui_native_862) }; if !(pui_native_859) {!((v_7265616479.read().call)())} else {pui_native_859} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":292:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":293:9
                    if { let pui_native_863:f64=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f383635 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383635.clone()).get()).num()) }).num()); let pui_native_864:f64=0.0_f64; pui_native_863 > pui_native_864 } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":293:37
                        (v_7265717565737452657665616c.read().call)(({ let v_7075695f6e61746976655f383636 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383636.clone()).get()).num()) }).num());
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":294:9
                    if { let pui_native_867={ let pui_native_869={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_870=v_7469636b65745f5f30646466366662663930636166613963.clone(); !(pui_native_869==pui_native_870) }; if !(pui_native_867) {!((v_7265616479.read().call)())} else {pui_native_867} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":294:43
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":296:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":296:21
                    { let v_7075695f6e61746976655f383731 = v_72756e5f5f37306636363434393364333530653437.clone(); request_update(&(v_7075695f6e61746976655f383731.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":298:5
            { let v_7075695f6e61746976655f383732 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":299:5
            { let v_7075695f6e61746976655f383733 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f64643863366331636131306164353664: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":299:40
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:57
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:74
                    v_73796e63.read().call_authored(vec![v_72756e5f5f64643863366331636131306164353664.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:85
                    if (({ let v_7075695f6e61746976655f383734 = v_72756e5f5f64643863366331636131306164353664.clone(); read_props(&(v_7075695f6e61746976655f383734.clone()).owner(), false) }).member("autoFocus", false)).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":299:116
                        { let v_7075695f6e61746976655f383735 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f383735.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":300:5
            { let v_7075695f6e61746976655f383736 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("mode"), Value::string("label"), Value::string("description"), Value::string("cursor"), Value::string("expanded"), Value::string("revision"), Value::string("disabled"), Value::string("autoFocus"), Value::string("columns"), Value::string("orientation")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":301:5
            { let v_7075695f6e61746976655f383737 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:39
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:48
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:66
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":302:5
            { let v_7075695f6e61746976655f383738 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:43
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:52
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:69
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:87
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":303:5
            { let v_7075695f6e61746976655f383739 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":304:5
            return { let callable={ let pui_native_880={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":304:26
                return { let v_7075695f6e61746976655f383831 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_880.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_880.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> CollectionControlHandle { CollectionControlHandle { owner: self.owner.clone() } }
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
  impl Drop for CollectionControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for CollectionControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for CollectionControl {
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
