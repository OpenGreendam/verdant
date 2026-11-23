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
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265616479: LexicalBinding<Value> = lexical_binding();
            let v_6974656d73: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b696e64: LexicalBinding<Value> = lexical_binding();
            let v_636f756e74: LexicalBinding<Value> = lexical_binding();
            let v_6d696e: LexicalBinding<Value> = lexical_binding();
            let v_6d6178: LexicalBinding<Value> = lexical_binding();
            let v_66696e64: LexicalBinding<Value> = lexical_binding();
            let v_7365656b: LexicalBinding<Value> = lexical_binding();
            let v_76616c69644974656d73: LexicalBinding<Value> = lexical_binding();
            let v_656d6974: LexicalBinding<Value> = lexical_binding();
            let v_7265717565737452657665616c: LexicalBinding<Value> = lexical_binding();
            let v_6368616e6765457870616e73696f6e: LexicalBinding<Value> = lexical_binding();
            let v_73656c656374: LexicalBinding<Value> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<Value> = lexical_binding();
            let v_636f6e746578744174: LexicalBinding<Value> = lexical_binding();
            let v_6b65795f5f63636538616136363530633839623964: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/collection-control.proto.ts":14:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("mode", record(vec![("type", Value::string("string")), ("default", Value::string("listbox"))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("cursor", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("orientation", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("expanded", record(vec![("type", Value::string("number")), ("default", Value::number(-(Value::number(1.0)).scalar_number()))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("columns", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))]))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":22:11
            v_637572736f72.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cursor"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":23:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "discrete", Value::string("expanded"), Value::number(-(Value::number(1.0)).scalar_number()), Value::Undefined) });
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
            v_726f7773.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("rows"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:11
            v_636f6c756d6e73.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "discrete", Value::string("columns"), Value::number(0.0), Value::Undefined) });
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
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":48:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":49:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":50:11
            v_7265616479.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":50:34
                return { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e74.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3332 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3332.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":51:9
            v_6974656d73.initialize(Rc::new(std::cell::RefCell::new(Value::Array(Rc::new(vec![])))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":52:11
            v_6b696e64.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f36363464363464383135616264373833: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":52:45
                return if ({ let left = { let left = { let left = number_is_finite(&(v_696e6465785f5f36363464363464383135616264373833.clone())); if left.truthy() { Value::boolean(strict_equal(&(Value::number((v_696e6465785f5f36363464363464383135616264373833.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if left.truthy() { { let relational_left = v_696e6465785f5f36363464363464383135616264373833.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { { let relational_left = v_696e6465785f5f36363464363464383135616264373833.clone(); let relational_right = ({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() { { let left = { let object = { let value = (v_6974656d73.read()).borrow().clone(); value }; object.caught_member(&property_key(&(v_696e6465785f5f36363464363464383135616264373833.clone())), false) }; if left.nullish() { Value::number(0.0) } else { left } } } else { Value::number(0.0) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":53:11
            v_636f756e74.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":53:33
                return ({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":54:11
            v_6d696e.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f66613437356230386530336138663863: Value = arg(&arguments, 0);
                let v_625f5f63366162623939393938333937383534: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":54:51
                return if ({ let relational_left = v_615f5f66613437356230386530336138663863.clone(); let relational_right = v_625f5f63366162623939393938333937383534.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_615f5f66613437356230386530336138663863.clone() } else { v_625f5f63366162623939393938333937383534.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":55:11
            v_6d6178.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f34616131306637366338373831393632: Value = arg(&arguments, 0);
                let v_625f5f61393064373237633066333932316664: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":55:51
                return if ({ let relational_left = v_615f5f34616131306637366338373831393632.clone(); let relational_right = v_625f5f61393064373237633066333932316664.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_615f5f34616131306637366338373831393632.clone() } else { v_625f5f61393064373237633066333932316664.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":56:11
            v_66696e64.initialize({  let v_6b696e64 = v_6b696e64.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_73746172745f5f32326334346537333664336138613034: Value = arg(&arguments, 0);
                let v_656e645f5f62383832313466396664336637333765: Value = arg(&arguments, 1);
                let v_666f7277617264: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":57:7
                if ({ let relational_left = v_656e645f5f62383832313466396664336637333765.clone(); let relational_right = v_73746172745f5f32326334346537333664336138613034.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":57:25
                    return Value::number(-(Value::number(1.0)).scalar_number());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":58:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f62383832313466396664336637333765.clone()).scalar_number() - (v_73746172745f5f32326334346537333664336138613034.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":58:30
                    return if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_73746172745f5f32326334346537333664336138613034.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_73746172745f5f32326334346537333664336138613034.clone() } else { Value::number(-(Value::number(1.0)).scalar_number()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":59:13
                let v_73756d5f5f33303961306230316337396238383531 = add(v_73746172745f5f32326334346537333664336138613034.clone(), v_656e645f5f62383832313466396664336637333765.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":60:13
                let v_6d6964646c655f5f37386164383235636239616665353633 = Value::number((Value::number((v_73756d5f5f33303961306230316337396238383531.clone()).scalar_number() - (Value::number((v_73756d5f5f33303961306230316337396238383531.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":61:13
                let v_63616e6469646174655f5f63303864656465393134663333643331 = if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_73746172745f5f32326334346537333664336138613034.clone(), v_6d6964646c655f5f37386164383235636239616665353633.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f37386164383235636239616665353633.clone(), v_656e645f5f62383832313466396664336637333765.clone(), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":62:7
                if ({ let relational_left = v_63616e6469646174655f5f63303864656465393134663333643331.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":62:27
                    return v_63616e6469646174655f5f63303864656465393134663333643331.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":63:7
                return if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f37386164383235636239616665353633.clone(), v_656e645f5f62383832313466396664336637333765.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_73746172745f5f32326334346537333664336138613034.clone(), v_6d6964646c655f5f37386164383235636239616665353633.clone(), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":65:11
            v_7365656b.initialize({  let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7265717565737465645f5f62643933396431636635323762306436: Value = arg(&arguments, 0);
                let v_646972656374696f6e: Value = arg(&arguments, 1);
                let v_5f72656d61696e696e67: Value = arg(&arguments, 2);
                let v_77726170: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":66:13
                let v_696e6465785f5f33396633613562393662616462336531 = v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), v_6d6178.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), v_7265717565737465645f5f62643933396431636635323762306436.clone()])]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":67:13
                let v_63616e6469646174655f5f61653531383666613035326231303664 = if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f33396633613562393662616462336531.clone()]), v_636f756e74.read().call_authored(vec![]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f33396633613562393662616462336531.clone(), Value::number(1.0))]), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":68:7
                if ({ let left = { let relational_left = v_63616e6469646174655f5f61653531383666613035326231303664.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if !left.truthy() { Value::boolean(!(v_77726170.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":68:36
                    return v_63616e6469646174655f5f61653531383666613035326231303664.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":69:7
                return if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f33396633613562393662616462336531.clone()]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f33396633613562393662616462336531.clone(), Value::number(1.0))]), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":71:11
            v_76616c69644974656d73.initialize({  let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76616c756573: Value = arg(&arguments, 0);
                let v_73746172745f5f62396333303062343363623338616636: Value = arg(&arguments, 1);
                let v_656e645f5f37653636623161383836346433353835: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":72:7
                if ({ let relational_left = v_656e645f5f37653636623161383836346433353835.clone(); let relational_right = v_73746172745f5f62396333303062343363623338616636.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":72:25
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":73:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f37653636623161383836346433353835.clone()).scalar_number() - (v_73746172745f5f62396333303062343363623338616636.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":73:30
                    return { let left = { let left = Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62396333303062343363623338616636.clone())), false) }), &(Value::number(0.0)))); if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62396333303062343363623338616636.clone())), false) }), &(Value::number(1.0)))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62396333303062343363623338616636.clone())), false) }), &(Value::number(2.0)))) } else { left } };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":74:13
                let v_73756d5f5f37653963393564326132353162383639 = add(v_73746172745f5f62396333303062343363623338616636.clone(), v_656e645f5f37653636623161383836346433353835.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":75:13
                let v_6d6964646c655f5f33303439393264393338623235383761 = Value::number((Value::number((v_73756d5f5f37653963393564326132353162383639.clone()).scalar_number() - (Value::number((v_73756d5f5f37653963393564326132353162383639.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":76:7
                return { let left = v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_73746172745f5f62396333303062343363623338616636.clone(), v_6d6964646c655f5f33303439393264393338623235383761.clone()]); if left.truthy() { v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_6d6964646c655f5f33303439393264393338623235383761.clone(), v_656e645f5f37653636623161383836346433353835.clone()]) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":78:11
            v_656d6974.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f38393536316239363236373961313664: Value = arg(&arguments, 0);
                let v_696e6465785f5f65316365343832353735633961623635: Value = arg(&arguments, 1);
                let v_657874656e645f5f36353566626264643163366330363164: Value = arg(&arguments, 2);
                let v_6b65795f5f37383635303832363661313933636137: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":79:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":79:20
                    { let v_7075695f6e61746976655f3333 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3333.clone()).owner(), v_6e616d655f5f38393536316239363236373961313664.clone(), record(vec![("index", v_696e6465785f5f65316365343832353735633961623635.clone()), ("extend", v_657874656e645f5f36353566626264643163366330363164.clone()), ("key", v_6b65795f5f37383635303832363661313933636137.clone()), ("revision", { let left = ({ let v_7075695f6e61746976655f3334 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3334.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":81:11
            v_7265717565737452657665616c.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f33393531626561343663653166626431: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":82:13
                let v_6f776e65725f5f61353730616338633666353139383837 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":83:7
                if ({ let left = Value::boolean(!(v_6f776e65725f5f61353730616338633666353139383837.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":83:31
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":84:13
                let v_6d6f64655f5f32386364333135663730633139386234 = ({ let v_7075695f6e61746976655f3335 = v_6f776e65725f5f61353730616338633666353139383837.clone(); read_props(&(v_7075695f6e61746976655f3335.clone()).owner(), false) }).member("mode", false);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":85:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f32386364333135663730633139386234.clone()), &(Value::string("filterlist")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f32386364333135663730633139386234.clone()), &(Value::string("tracklist")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":85:58
                    v_656d6974.read().call_authored(vec![Value::string("command"), v_696e6465785f5f33393531626561343663653166626431.clone(), Value::boolean(false), Value::string("reveal")]);
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":87:11
            v_6368616e6765457870616e73696f6e.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f31643832333866323937646431306436: Value = arg(&arguments, 0);
                let v_6e616d655f5f34323666653430356465353638646632: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":88:7
                if (Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":88:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":89:13
                let v_7469636b65745f5f35396236643266323430666663633037 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":90:7
                if ({ let left = { let relational_left = v_6e6578745f5f31643832333866323937646431306436.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if left.truthy() { { let relational_left = { let v_7075695f6e61746976655f3336 = v_657870616e646564.read(); (v_7075695f6e61746976655f3336.clone()).get() }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":91:9
                    { let v_7075695f6e61746976655f3337 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3337.clone(), { let v_7075695f6e61746976655f3338 = v_657870616e646564.read(); (v_7075695f6e61746976655f3338.clone()).get() }, Value::string("reason: closing submenu restores parent")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":92:9
                    if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f35396236643266323430666663633037.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":92:43
                        return Value::boolean(false);
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":94:7
                { let v_7075695f6e61746976655f3339 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3339.clone(), v_6e6578745f5f31643832333866323937646431306436.clone(), Value::string("reason: submenu expansion")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":95:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f35396236643266323430666663633037.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":95:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":96:7
                v_656d6974.read().call_authored(vec![Value::string("expand"), v_6e6578745f5f31643832333866323937646431306436.clone(), Value::boolean(false), v_6e616d655f5f34323666653430356465353638646632.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":97:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_7469636b65745f5f35396236643266323430666663633037.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":97:40
                    { let v_7075695f6e61746976655f3430 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3430.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":98:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":100:11
            v_73656c656374.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f30396366383834383461353234623464: Value = arg(&arguments, 0);
                let v_657874656e645f5f65343537633465636263636362333362: Value = arg(&arguments, 1);
                let v_6578706c69636974: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":101:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f30396366383834383461353234623464.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":101:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":104:7
                if ({ let left = { let left = Value::boolean(!(v_6578706c69636974.clone()).truthy()); if left.truthy() { Value::boolean(strict_equal(&(v_696e6465785f5f30396366383834383461353234623464.clone()), &({ let v_7075695f6e61746976655f3431 = v_637572736f72.read(); (v_7075695f6e61746976655f3431.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(!(v_657874656e645f5f65343537633465636263636362333362.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":104:59
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":105:13
                let v_7469636b65745f5f37626238383031343734313532396437 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":106:7
                { let v_7075695f6e61746976655f3432 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3432.clone(), v_696e6465785f5f30396366383834383461353234623464.clone(), Value::string("reason: collection selection")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":107:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f37626238383031343734313532396437.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":107:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":108:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f30396366383834383461353234623464.clone(), v_657874656e645f5f65343537633465636263636362333362.clone(), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":109:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f37626238383031343734313532396437.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":109:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":110:7
                v_7265717565737452657665616c.read().call_authored(vec![v_696e6465785f5f30396366383834383461353234623464.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":111:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f37626238383031343734313532396437.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":111:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":112:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_7469636b65745f5f37626238383031343734313532396437.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":112:40
                    { let v_7075695f6e61746976655f3433 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3433.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":113:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":115:11
            v_63686f6f7365.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f30646536643164336436666136653830: Value = arg(&arguments, 0);
                let v_657874656e645f5f34633161383735333230353534306130: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":116:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f30646536643164336436666136653830.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":116:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":117:13
                let v_7469636b65745f5f35393863386432653139666563623032 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":118:7
                { let v_7075695f6e61746976655f3434 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3434.clone(), v_696e6465785f5f30646536643164336436666136653830.clone(), Value::string("reason: collection activation cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":119:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f35393863386432653139666563623032.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":119:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":120:7
                if (Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f30646536643164336436666136653830.clone()])), &(Value::number(2.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":121:15
                    let v_6e6578745f5f65393831633161393530313364346565 = if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3435 = v_657870616e646564.read(); (v_7075695f6e61746976655f3435.clone()).get() }), &(v_696e6465785f5f30646536643164336436666136653830.clone())))).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { v_696e6465785f5f30646536643164336436666136653830.clone() };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":122:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![v_6e6578745f5f65393831633161393530313364346565.clone(), Value::string("")]);
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":123:14
                    v_656d6974.read().call_authored(vec![Value::string("activate"), v_696e6465785f5f30646536643164336436666136653830.clone(), v_657874656e645f5f34633161383735333230353534306130.clone(), Value::string("")]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":124:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":126:11
            v_636f6e746578744174.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f63383336636366396537373937356266: Value = arg(&arguments, 0);
                let v_62617369735f5f35373233316461656662626465313432: Value = arg(&arguments, 1);
                let v_78: Value = arg(&arguments, 2);
                let v_79: Value = arg(&arguments, 3);
                let v_6b6579626f617264: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":127:13
                let v_6f776e65725f5f34333637383931663863643364353464 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":128:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f34333637383931663863643364353464.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f35373233316461656662626465313432.clone()), &({ let left = ({ let v_7075695f6e61746976655f3436 = v_6f776e65725f5f34333637383931663863643364353464.clone(); read_props(&(v_7075695f6e61746976655f3436.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f63383336636366396537373937356266.clone()])), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_696e6465785f5f63383336636366396537373937356266.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(Value::number((v_696e6465785f5f63383336636366396537373937356266.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_78.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_79.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":129:102
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":130:13
                let v_7469636b65745f5f66316139366230633865353735653634 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":131:7
                { let v_7075695f6e61746976655f3437 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3437.clone(), v_696e6465785f5f63383336636366396537373937356266.clone(), Value::string("reason: collection context cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":132:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f66316139366230633865353735653634.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":132:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":133:7
                { let v_7075695f6e61746976655f3438 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3438.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":134:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f66316139366230633865353735653634.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":134:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":135:7
                { let v_7075695f6e61746976655f3439 = v_6f776e65725f5f34333637383931663863643364353464.clone(); emit(&(v_7075695f6e61746976655f3439.clone()).owner(), Value::string("context"), record(vec![("index", v_696e6465785f5f63383336636366396537373937356266.clone()), ("x", v_78.clone()), ("y", v_79.clone()), ("keyboard", v_6b6579626f617264.clone()), ("revision", v_62617369735f5f35373233316461656662626465313432.clone())])) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":136:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":138:11
            v_6b65795f5f63636538616136363530633839623964.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let v_73656c656374 = v_73656c656374.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let v_636f6e746578744174 = v_636f6e746578744174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f39663033646662643236656166323335: Value = arg(&arguments, 0);
                let v_73686966745f5f35346239656133393965376336626438: Value = arg(&arguments, 1);
                let v_636f6d6d616e645f5f34323363326161366465626334663932: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":139:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":139:33
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":140:13
                let v_6d6f64655f5f38336631336234323834663464633562 = { let left = ({ let v_7075695f6e61746976655f3530 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3530.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":141:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("filterlist")))); if left.truthy() { v_636f6d6d616e645f5f34323363326161366465626334663932.clone() } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":141:81
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3531 = v_637572736f72.read(); (v_7075695f6e61746976655f3531.clone()).get() }, Value::boolean(false), Value::string("focus-filter")]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":141:135
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":142:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tracklist")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F10")))); if left.truthy() { v_73686966745f5f35346239656133393965376336626438.clone() } else { left } } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":142:90
                    return v_636f6e746578744174.read().call_authored(vec![{ let v_7075695f6e61746976655f3532 = v_637572736f72.read(); (v_7075695f6e61746976655f3532.clone()).get() }, { let left = ({ let v_7075695f6e61746976655f3533 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3533.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }, Value::number(0.0), Value::number(0.0), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":143:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Enter")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":143:75
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3534 = v_637572736f72.read(); (v_7075695f6e61746976655f3534.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":143:117
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":144:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":145:15
                    let v_5f5f7075694d6f64756c6530 = add({ let v_7075695f6e61746976655f3535 = v_637572736f72.read(); (v_7075695f6e61746976655f3535.clone()).get() }, if (v_73686966745f5f35346239656133393965376336626438.clone()).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { Value::number(1.0) });
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":146:9
                    if ({ let left = { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if !left.truthy() { { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = v_636f756e74.read().call_authored(vec![]); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":146:42
                        v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3536 = v_637572736f72.read(); (v_7075695f6e61746976655f3536.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":147:14
                        v_73656c656374.read().call_authored(vec![v_5f5f7075694d6f64756c6530.clone(), Value::boolean(false), Value::boolean(true)]);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":148:9
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":150:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points")))); if left.truthy() { { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("+")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("=")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("-")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":150:128
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3537 = v_637572736f72.read(); (v_7075695f6e61746976655f3537.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":150:172
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":151:7
                if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Escape")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:71
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3538 = v_637572736f72.read(); (v_7075695f6e61746976655f3538.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:113
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":152:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Enter")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string(" ")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("F2")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":152:84
                    return v_63686f6f7365.read().call_authored(vec![{ let v_7075695f6e61746976655f3539 = v_637572736f72.read(); (v_7075695f6e61746976655f3539.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":153:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Backspace")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Tab")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3630 = v_637572736f72.read(); (v_7075695f6e61746976655f3630.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":156:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("table")))); if left.truthy() { v_636f6d6d616e645f5f34323363326161366465626334663932.clone() } else { left } }; if left.truthy() { { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":157:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3631 = v_637572736f72.read(); (v_7075695f6e61746976655f3631.clone()).get() }, v_73686966745f5f35346239656133393965376336626438.clone(), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":157:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":159:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":160:9
                    if ({ let relational_left = { let v_7075695f6e61746976655f3632 = v_657870616e646564.read(); (v_7075695f6e61746976655f3632.clone()).get() }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":160:33
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":161:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), v_6e616d655f5f39663033646662643236656166323335.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":163:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":164:9
                    if (Value::boolean(!strict_equal(&(v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f3633 = v_637572736f72.read(); (v_7075695f6e61746976655f3633.clone()).get() }])), &(Value::number(2.0))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":164:39
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":165:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![{ let v_7075695f6e61746976655f3634 = v_637572736f72.read(); (v_7075695f6e61746976655f3634.clone()).get() }, v_6e616d655f5f39663033646662643236656166323335.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":167:11
                let v_6e6578745f5f39303365663633396366633434303531 = Rc::new(std::cell::RefCell::new(Value::number(-(Value::number(1.0)).scalar_number())));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":168:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Home"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":168:28
                    { let value = v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":169:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("End"))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":169:32
                        { let value = v_7365656b.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":170:12
                        if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":170:60
                            { let value = v_7365656b.read().call_authored(vec![Value::number(({ let v_7075695f6e61746976655f3635 = v_637572736f72.read(); (v_7075695f6e61746976655f3635.clone()).get() }).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":171:12
                            if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowDown")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":171:63
                                { let value = v_7365656b.read().call_authored(vec![add({ let v_7075695f6e61746976655f3636 = v_637572736f72.read(); (v_7075695f6e61746976655f3636.clone()).get() }, Value::number(1.0)), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":172:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageUp"))))).truthy() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":172:35
                                    { let value = v_7365656b.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), Value::number(({ let v_7075695f6e61746976655f3637 = v_637572736f72.read(); (v_7075695f6e61746976655f3637.clone()).get() }).scalar_number() - (if (Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) }).scalar_number())]), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":173:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageDown"))))).truthy() {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":173:37
                                        { let value = v_7365656b.read().call_authored(vec![v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), add({ let v_7075695f6e61746976655f3638 = v_637572736f72.read(); (v_7075695f6e61746976655f3638.clone()).get() }, if (Value::boolean(strict_equal(&(v_6d6f64655f5f38336631336234323834663464633562.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) })]), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f39303365663633396366633434303531.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":174:12
                                        return Value::boolean(false);
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":175:7
                if ({ let left = { let relational_left = { let value = (v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!(v_73656c656374.read().call_authored(vec![{ let value = (v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }, v_73686966745f5f35346239656133393965376336626438.clone(), Value::boolean(true)])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":175:52
                    return Value::boolean(true);
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":176:12
                    if ({ let left = { let left = { let left = { let relational_left = { let value = (v_6e6578745f5f39303365663633396366633434303531.clone()).borrow().clone(); value }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!(v_73686966745f5f35346239656133393965376336626438.clone()).truthy()) } else { left } }; if left.truthy() { { let relational_left = v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f3639 = v_637572736f72.read(); (v_7075695f6e61746976655f3639.clone()).get() }]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }; if left.truthy() { Value::boolean(!(v_73656c656374.read().call_authored(vec![{ let v_7075695f6e61746976655f3730 = v_637572736f72.read(); (v_7075695f6e61746976655f3730.clone()).get() }, Value::boolean(false), Value::boolean(true)])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":176:100
                        return Value::boolean(true);
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":177:7
                if (Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":177:21
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":178:13
                let v_686f72697a6f6e74616c = Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3731 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3731.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3731.clone()).owner(), false) } }).member("orientation", true)), &(Value::string("horizontal"))));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":179:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageUp"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":179:32
                    if (v_686f72697a6f6e74616c.clone()).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":179:48
                        { let v_7075695f6e61746976655f3732 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3732.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":179:128
                        { let v_7075695f6e61746976655f3733 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3733.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":180:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("PageDown"))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":180:39
                        if (v_686f72697a6f6e74616c.clone()).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":180:55
                            { let v_7075695f6e61746976655f3734 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3734.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":180:134
                            { let v_7075695f6e61746976655f3735 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3735.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":181:12
                        if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("Home"))))).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":181:35
                            if (v_686f72697a6f6e74616c.clone()).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":181:51
                                { let v_7075695f6e61746976655f3736 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3736.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0))])) };
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":181:121
                                { let v_7075695f6e61746976655f3737 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3737.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":182:12
                            if (Value::boolean(strict_equal(&(v_6e616d655f5f39663033646662643236656166323335.clone()), &(Value::string("End"))))).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":182:34
                                if (v_686f72697a6f6e74616c.clone()).truthy() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":182:50
                                    { let v_7075695f6e61746976655f3738 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3738.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":182:111
                                    { let v_7075695f6e61746976655f3739 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3739.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":183:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":188:5
            { let v_7075695f6e61746976655f3830 = v_646566.clone(); expose_method(&owner, Value::string("filterKey"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f37613536633837626431663532366332: Value = arg(&arguments, 0);
                let v_73686966745f5f38353264393838616565376537356239: Value = arg(&arguments, 1);
                let v_636f6d6d616e645f5f32366138313638353133626133353639: Value = arg(&arguments, 2);
                let v_636f6d706f73696e67: Value = arg(&arguments, 3);
                let v_66726f6d46696c746572: Value = arg(&arguments, 4);
                let v_62617369735f5f38653339326536643161626362663536: Value = arg(&arguments, 5);
                let v_616c74: Value = arg(&arguments, 6);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":189:13
                let v_6f776e65725f5f61633262346635326137313438303332 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":190:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f61633262346635326137313438303332.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { v_636f6d706f73696e67.clone() } else { left } }; if !left.truthy() { v_616c74.clone() } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f3831 = v_6f776e65725f5f61633262346635326137313438303332.clone(); read_props(&(v_7075695f6e61746976655f3831.clone()).owner(), false) }).member("mode", false)), &(Value::string("filterlist")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f38653339326536643161626362663536.clone()), &({ let left = ({ let v_7075695f6e61746976655f3832 = v_6f776e65725f5f61633262346635326137313438303332.clone(); read_props(&(v_7075695f6e61746976655f3832.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":190:142
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":191:7
                if ({ let left = v_636f6d6d616e645f5f32366138313638353133626133353639.clone(); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":191:54
                    return v_6b65795f5f63636538616136363530633839623964.read().call_authored(vec![v_6e616d655f5f37613536633837626431663532366332.clone(), v_73686966745f5f38353264393838616565376537356239.clone(), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":192:7
                if ({ let left = Value::boolean(!(v_66726f6d46696c746572.clone()).truthy()); if !left.truthy() { v_636f6d6d616e645f5f32366138313638353133626133353639.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":192:35
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":193:7
                if ({ let left = { let left = { let left = Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("ArrowUp")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("Enter")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f37613536633837626431663532366332.clone()), &(Value::string("Escape")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":193:96
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":194:7
                return v_6b65795f5f63636538616136363530633839623964.read().call_authored(vec![v_6e616d655f5f37613536633837626431663532366332.clone(), v_73686966745f5f38353264393838616565376537356239.clone(), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":199:5
            { let v_7075695f6e61746976655f3833 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f61613236653065393430303964373333: Value = arg(&arguments, 0);
                let v_73746172745f5f62326431336666663833346661633834: Value = arg(&arguments, 1);
                let v_656e645f5f36663731373364653138373330666231: Value = arg(&arguments, 2);
                let v_76696577706f72745374617274: Value = arg(&arguments, 3);
                let v_76696577706f7274456e64: Value = arg(&arguments, 4);
                let v_62617369735f5f37396562613432663433336531656363: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":200:13
                let v_6f776e65725f5f65616234323132303862343434376265 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":201:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f65616234323132303862343434376265.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f37396562613432663433336531656363.clone()), &({ let left = ({ let v_7075695f6e61746976655f3834 = v_6f776e65725f5f65616234323132303862343434376265.clone(); read_props(&(v_7075695f6e61746976655f3834.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_696e6465785f5f61613236653065393430303964373333.clone()), &({ let v_7075695f6e61746976655f3835 = v_637572736f72.read(); (v_7075695f6e61746976655f3835.clone()).get() }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f61613236653065393430303964373333.clone()])), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_73746172745f5f62326431336666663833346661633834.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_656e645f5f36663731373364653138373330666231.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f72745374617274.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f7274456e64.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_656e645f5f36663731373364653138373330666231.clone(); let relational_right = v_73746172745f5f62326431336666663833346661633834.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let relational_left = v_76696577706f7274456e64.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":203:58
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":204:7
                if ({ let left = { let left = { let relational_left = v_73746172745f5f62326431336666663833346661633834.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_656e645f5f36663731373364653138373330666231.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let left = { let relational_left = v_73746172745f5f62326431336666663833346661633834.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_656e645f5f36663731373364653138373330666231.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":204:109
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:13
                let v_6c656164696e67 = Value::number((v_73746172745f5f62326431336666663833346661633834.clone()).scalar_number() - (v_76696577706f72745374617274.clone()).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:52
                let v_747261696c696e67 = Value::number((v_656e645f5f36663731373364653138373330666231.clone()).scalar_number() - (v_76696577706f7274456e64.clone()).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":206:13
                let v_6c656164696e6744697374616e6365 = if ({ let relational_left = v_6c656164696e67.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_6c656164696e67.clone()).scalar_number()) } else { v_6c656164696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":206:71
                let v_747261696c696e6744697374616e6365 = if ({ let relational_left = v_747261696c696e67.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_747261696c696e67.clone()).scalar_number()) } else { v_747261696c696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":207:13
                let v_64656c7461 = if ({ let relational_left = v_6c656164696e6744697374616e6365.clone(); let relational_right = v_747261696c696e6744697374616e6365.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_6c656164696e67.clone() } else { v_747261696c696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":208:7
                if (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3836 = v_6f776e65725f5f65616234323132303862343434376265.clone(); read_props(&(v_7075695f6e61746976655f3836.clone()).owner(), false) }).member("orientation", false)), &(Value::string("horizontal"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":208:59
                    { let v_7075695f6e61746976655f3837 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3837.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", v_64656c7461.clone())])) };
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":209:12
                    { let v_7075695f6e61746976655f3838 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3838.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", v_64656c7461.clone())])) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":210:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":212:5
            { let v_7075695f6e61746976655f3839 = v_646566.clone(); expose_method(&owner, Value::string("setItems"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6974656d73 = v_6974656d73.clone(); let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f64396266323461323332323535373463: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":213:7
                if ({ let left = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { { let relational_left = (v_6e6578745f5f64396266323461323332323535373463.clone()).member("length", false); let relational_right = Value::number(1000000.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }; if !left.truthy() { Value::boolean(!(v_76616c69644974656d73.read().call_authored(vec![v_6e6578745f5f64396266323461323332323535373463.clone(), Value::number(0.0), (v_6e6578745f5f64396266323461323332323535373463.clone()).member("length", false)])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":213:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:16
                { let value = v_6e6578745f5f64396266323461323332323535373463.clone(); *(v_6974656d73.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":215:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":215:20
                    v_73796e63.read().call_authored(vec![{ let value = (v_63757272656e74.read()).borrow().clone(); value }]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":216:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":218:5
            { let v_7075695f6e61746976655f3930 = v_646566.clone(); expose_method(&owner, Value::string("choose"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f38656563363332363635383836383033: Value = arg(&arguments, 0);
                let v_657874656e645f5f34613362383437633732356230383136: Value = arg(&arguments, 1);
                let v_62617369735f5f34373939326232646538653363663566: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":219:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f34373939326232646538653363663566.clone()), &({ let left = ({ let v_7075695f6e61746976655f3931 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3931.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3931.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":219:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:7
                return v_63686f6f7365.read().call_authored(vec![v_696e6465785f5f38656563363332363635383836383033.clone(), v_657874656e645f5f34613362383437633732356230383136.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":222:5
            { let v_7075695f6e61746976655f3932 = v_646566.clone(); expose_method(&owner, Value::string("selectIndex"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f30663930393332303932616130326437: Value = arg(&arguments, 0);
                let v_657874656e645f5f62646665313032383436333735333831: Value = arg(&arguments, 1);
                let v_62617369735f5f33623766616636313731323030623061: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":223:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f33623766616636313731323030623061.clone()), &({ let left = ({ let v_7075695f6e61746976655f3933 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3933.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3933.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":223:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":224:7
                return v_73656c656374.read().call_authored(vec![v_696e6465785f5f30663930393332303932616130326437.clone(), v_657874656e645f5f62646665313032383436333735333831.clone(), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":228:5
            { let v_7075695f6e61746976655f3934 = v_646566.clone(); expose_method(&owner, Value::string("keyAt"), {  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f35316630646638376537343663303363: Value = arg(&arguments, 0);
                let v_6e616d655f5f32346234353030616138373332366563: Value = arg(&arguments, 1);
                let v_73686966745f5f62303936626531363166323339636132: Value = arg(&arguments, 2);
                let v_636f6d6d616e645f5f31636337376335343432396662636564: Value = arg(&arguments, 3);
                let v_62617369735f5f34646338376562343738383931353262: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":229:7
                if ({ let left = { let left = { let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f34646338376562343738383931353262.clone()), &({ let left = ({ let v_7075695f6e61746976655f3935 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3935.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f35316630646638376537343663303363.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":229:104
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:13
                let v_6d6f64655f5f63613732626137346136613864333732 = { let left = ({ let v_7075695f6e61746976655f3936 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3936.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":231:13
                let v_68616e646c6564 = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Enter")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string(" ")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Home")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("End")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowLeft")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowRight")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ArrowDown")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("PageUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("PageDown")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Tab")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("points")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("table")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("menu")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("table")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F2")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Delete")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("Backspace")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("points")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("+")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("=")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("-")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("filterlist")))); if left.truthy() { v_636f6d6d616e645f5f31636337376335343432396662636564.clone() } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f63613732626137346136613864333732.clone()), &(Value::string("tracklist")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32346234353030616138373332366563.clone()), &(Value::string("F10")))); if left.truthy() { v_73686966745f5f62303936626531363166323339636132.clone() } else { left } } } else { left } } } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":238:7
                if (Value::boolean(!(v_68616e646c6564.clone()).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":238:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":239:13
                let v_7469636b65745f5f62623365636533376534346261656537 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":240:7
                { let v_7075695f6e61746976655f3937 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3937.clone(), v_696e6465785f5f35316630646638376537343663303363.clone(), Value::string("reason: addressed collection key")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":241:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f62623365636533376534346261656537.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":241:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":242:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f35316630646638376537343663303363.clone(), Value::boolean(false), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":243:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f62623365636533376534346261656537.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f34646338376562343738383931353262.clone()), &({ let left = ({ let v_7075695f6e61746976655f3938 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3938.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3938.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":243:92
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":244:7
                return v_6b65795f5f63636538616136363530633839623964.read().call_authored(vec![v_6e616d655f5f32346234353030616138373332366563.clone(), v_73686966745f5f62303936626531363166323339636132.clone(), v_636f6d6d616e645f5f31636337376335343432396662636564.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":246:5
            { let v_7075695f6e61746976655f3939 = v_646566.clone(); expose_method(&owner, Value::string("selectAndFocus"), {  let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f39306136653737656637623231623938: Value = arg(&arguments, 0);
                let v_657874656e645f5f31613534383230376165646161343561: Value = arg(&arguments, 1);
                let v_62617369735f5f65336261643632616138346237366562: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":247:7
                if ({ let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f65336261643632616138346237366562.clone()), &({ let left = ({ let v_7075695f6e61746976655f313030 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313030.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":247:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":248:7
                if ({ let left = Value::boolean(!(v_73656c656374.read().call_authored(vec![v_696e6465785f5f39306136653737656637623231623938.clone(), v_657874656e645f5f31613534383230376165646161343561.clone(), Value::boolean(true)])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f65336261643632616138346237366562.clone()), &({ let left = ({ let v_7075695f6e61746976655f313031 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f313031.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f313031.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":248:92
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":249:7
                { let v_7075695f6e61746976655f313032 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313032.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":250:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":252:5
            { let v_7075695f6e61746976655f313033 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b65795f5f63636538616136363530633839623964.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":253:5
            { let v_7075695f6e61746976655f313034 = v_646566.clone(); expose_method(&owner, Value::string("contextAt"), v_636f6e746578744174.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":254:5
            { let v_7075695f6e61746976655f313035 = v_646566.clone(); expose_method(&owner, Value::string("reconcile"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:50
                let v_6f776e65725f5f37393964386233353865383061383463 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:67
                if ({ let left = Value::boolean(!(v_6f776e65725f5f37393964386233353865383061383463.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":254:91
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:105
                v_73796e63.read().call_authored(vec![v_6f776e65725f5f37393964386233353865383061383463.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:118
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":255:5
            { let v_7075695f6e61746976655f313036 = v_646566.clone(); expose_method(&owner, Value::string("dismiss"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":255:42
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":255:55
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f313037 = v_637572736f72.read(); (v_7075695f6e61746976655f313037.clone()).get() }, Value::boolean(false), Value::string("outside")]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":256:5
            { let v_7075695f6e61746976655f313038 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":256:44
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":256:57
                    { let v_7075695f6e61746976655f313039 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313039.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":257:5
            { let v_7075695f6e61746976655f313130 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b65795f5f63636538616136363530633839623964 = v_6b65795f5f63636538616136363530633839623964.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":258:7
                if (v_6b65795f5f63636538616136363530633839623964.read().call_authored(vec![{ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true)))), { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true)))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true)))) } else { left } }])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":258:108
                    { let v_7075695f6e61746976655f313131 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f313131.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":260:11
            v_73796e63.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_7461626c65 = v_7461626c65.clone(); let v_726f7773 = v_726f7773.clone(); let v_636f6c756d6e73 = v_636f6c756d6e73.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f37306636363434393364333530653437: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":261:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":261:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:13
                let v_7469636b65745f5f30646466366662663930636166613963 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:31
                { let value = v_72756e5f5f37306636363434393364333530653437.clone(); *(v_63757272656e74.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":263:13
                let v_70726f7073 = { let v_7075695f6e61746976655f313132 = v_72756e5f5f37306636363434393364333530653437.clone(); read_props(&(v_7075695f6e61746976655f313132.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":264:7
                { let v_7075695f6e61746976655f313133 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f313133.clone(), if ({ let left = { let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("points")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("tracklist")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist")))) } else { left } }).truthy() { Value::string("listbox") } else { { let left = (v_70726f7073.clone()).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } } }, Value::string("reason: collection role")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":265:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":265:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":266:7
                { let v_7075695f6e61746976655f313134 = v_7461626c65.read(); state_set(v_7075695f6e61746976655f313134.clone(), Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("table")))), Value::string("reason: collection table semantics")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":267:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":267:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":268:7
                { let v_7075695f6e61746976655f313135 = v_726f7773.read(); state_set(v_7075695f6e61746976655f313135.clone(), v_636f756e74.read().call_authored(vec![]), Value::string("reason: collection rows")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":269:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":269:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":270:7
                { let v_7075695f6e61746976655f313136 = v_636f6c756d6e73.read(); state_set(v_7075695f6e61746976655f313136.clone(), { let left = (v_70726f7073.clone()).member("columns", false); if left.nullish() { Value::number(0.0) } else { left } }, Value::string("reason: collection columns")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":271:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":271:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":272:7
                { let v_7075695f6e61746976655f313137 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313137.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection name")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":273:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":273:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":274:7
                { let v_7075695f6e61746976655f313138 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f313138.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection description")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":275:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":276:7
                { let v_7075695f6e61746976655f313139 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313139.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: collection policy")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":277:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":277:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":278:7
                { let v_7075695f6e61746976655f313230 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313230.clone()).owner(), { let v_7075695f6e61746976655f313231 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313231.clone()).get() }) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":279:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":279:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":280:13
                let v_7265717565737465645f5f38633239316230636561396132643234 = { let left = (v_70726f7073.clone()).member("cursor", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":281:13
                let v_6163636570746564 = if ({ let left = number_is_finite(&(v_7265717565737465645f5f38633239316230636561396132643234.clone())); if left.truthy() { Value::boolean(strict_equal(&(Value::number((v_7265717565737465645f5f38633239316230636561396132643234.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }).truthy() { v_6d6178.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), v_7265717565737465645f5f38633239316230636561396132643234.clone()])]) } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":282:7
                { let v_7075695f6e61746976655f313232 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313232.clone(), if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_6163636570746564.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_6163636570746564.clone() } else { v_6d6178.read().call_authored(vec![Value::number(0.0), v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)])]) }, Value::string("reason: accepted collection cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":283:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":283:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":284:7
                { let v_7075695f6e61746976655f313233 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f313233.clone(), { let left = (v_70726f7073.clone()).member("expanded", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }, Value::string("reason: accepted expanded item")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":285:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":285:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":289:7
                if ({ let left = v_7265616479.read().call_authored(vec![]); if left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":290:9
                    if (Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f313234 = v_637572736f72.read(); (v_7075695f6e61746976655f313234.clone()).get() }])), &(Value::number(0.0))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":290:39
                        v_656d6974.read().call_authored(vec![Value::string("select"), Value::number(-(Value::number(1.0)).scalar_number()), Value::boolean(false), Value::string("reconcile")]);
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":291:14
                        if (Value::boolean(!strict_equal(&((v_70726f7073.clone()).member("cursor", false)), &({ let v_7075695f6e61746976655f313235 = v_637572736f72.read(); (v_7075695f6e61746976655f313235.clone()).get() })))).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":291:49
                            v_656d6974.read().call_authored(vec![Value::string("select"), { let v_7075695f6e61746976655f313236 = v_637572736f72.read(); (v_7075695f6e61746976655f313236.clone()).get() }, Value::boolean(false), Value::string("reconcile")]);
                        }
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":292:9
                    if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":292:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":293:9
                    if ({ let relational_left = v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f313237 = v_637572736f72.read(); (v_7075695f6e61746976655f313237.clone()).get() }]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":293:37
                        v_7265717565737452657665616c.read().call_authored(vec![{ let v_7075695f6e61746976655f313238 = v_637572736f72.read(); (v_7075695f6e61746976655f313238.clone()).get() }]);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":294:9
                    if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30646466366662663930636166613963.clone()))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":294:43
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":296:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":296:21
                    { let v_7075695f6e61746976655f313239 = v_72756e5f5f37306636363434393364333530653437.clone(); request_update(&(v_7075695f6e61746976655f313239.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":298:5
            { let v_7075695f6e61746976655f313330 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":299:5
            { let v_7075695f6e61746976655f313331 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f64643863366331636131306164353664: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":299:40
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:57
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:74
                    v_73796e63.read().call_authored(vec![v_72756e5f5f64643863366331636131306164353664.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":299:85
                    if (({ let v_7075695f6e61746976655f313332 = v_72756e5f5f64643863366331636131306164353664.clone(); read_props(&(v_7075695f6e61746976655f313332.clone()).owner(), false) }).member("autoFocus", false)).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":299:116
                        { let v_7075695f6e61746976655f313333 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313333.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":300:5
            { let v_7075695f6e61746976655f313334 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("mode"), Value::string("label"), Value::string("description"), Value::string("cursor"), Value::string("expanded"), Value::string("revision"), Value::string("disabled"), Value::string("autoFocus"), Value::string("columns"), Value::string("orientation")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":301:5
            { let v_7075695f6e61746976655f313335 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:39
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:48
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:66
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":302:5
            { let v_7075695f6e61746976655f313336 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:43
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:52
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:69
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:87
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":303:5
            { let v_7075695f6e61746976655f313337 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":304:5
            return { let callable={ let pui_native_138={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":304:26
                return { let v_7075695f6e61746976655f313339 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_138.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_138.call)(arg(&arguments,0))})} }; match callable {
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
