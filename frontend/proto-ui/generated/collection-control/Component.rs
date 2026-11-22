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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("mode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("cursor").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("orientation").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("columns").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "mode" => value.is_string(),
        "label" => value.is_string(),
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
    #[serde(rename = "cursor", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "orientation", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<String>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<f64>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "columns", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
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
            let v_637572736f72: LexicalBinding<Value> = lexical_binding();
            let v_657870616e646564: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
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
            let v_73656c656374: LexicalBinding<Value> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<Value> = lexical_binding();
            let v_636f6e746578744174: LexicalBinding<Value> = lexical_binding();
            let v_6b65795f5f30633866646432663039343166323131: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/collection-control.proto.ts":14:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("mode", record(vec![("type", Value::string("string")), ("default", Value::string("listbox"))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("cursor", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("orientation", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("expanded", record(vec![("type", Value::string("number")), ("default", Value::number(-(Value::number(1.0)).scalar_number()))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("columns", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))]))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":21:11
            v_637572736f72.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cursor"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":22:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "discrete", Value::string("expanded"), Value::number(-(Value::number(1.0)).scalar_number()), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":23:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":24:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("listbox"), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":25:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":26:11
            v_7461626c65.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("table"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":27:11
            v_726f7773.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "discrete", Value::string("rows"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":28:11
            v_636f6c756d6e73.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("columns"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:34
            { let v_7075695f6e61746976655f39 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f39.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:11
            v_7363726f6c6c.initialize(native_builtin_hook(&owner, "asScrollSurface", || scroll_setup(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:39
            { let v_7075695f6e61746976655f3130 = v_7363726f6c6c.read(); scroll_configure(&(v_7075695f6e61746976655f3130.clone()).owner(), record(vec![("axes", Value::string("both")), ("projection", Value::string("system"))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:34
            { let v_7075695f6e61746976655f3131 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3131.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:51
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3132.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:69
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("rowCount"), v_726f7773.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:52
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("columnCount"), v_636f6c756d6e73.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_state(&owner, Value::string("scrollPosition"), ((v_7363726f6c6c.read()).member("vertical", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:67
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_state(&owner, Value::string("scrollAtEnd"), ((v_7363726f6c6c.read()).member("vertical", false)).member("atEnd", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("horizontalPosition"), ((v_7363726f6c6c.read()).member("horizontal", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("mode"), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("cursor"), v_637572736f72.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:41
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("expanded"), v_657870616e646564.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":37:49
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_event(&owner, Value::string("select")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_event(&owner, Value::string("expand")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_event(&owner, Value::string("close")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":44:9
            v_63757272656e74.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":45:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":46:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":47:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":48:11
            v_7265616479.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":48:34
                return { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e74.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3330 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3330.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":49:9
            v_6974656d73.initialize(Rc::new(std::cell::RefCell::new(Value::Array(Rc::new(vec![])))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":50:11
            v_6b696e64.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f66653165626661386138373530346139: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":50:45
                return { let left = { let object = { let value = (v_6974656d73.read()).borrow().clone(); value }; object.caught_member(&property_key(&(v_696e6465785f5f66653165626661386138373530346139.clone())), false) }; if left.nullish() { Value::number(0.0) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":51:11
            v_636f756e74.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":51:33
                return ({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":52:11
            v_6d696e.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f36653966336566383862363732653863: Value = arg(&arguments, 0);
                let v_625f5f35616330633535356536613230306237: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":52:51
                return if ({ let relational_left = v_615f5f36653966336566383862363732653863.clone(); let relational_right = v_625f5f35616330633535356536613230306237.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_615f5f36653966336566383862363732653863.clone() } else { v_625f5f35616330633535356536613230306237.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":53:11
            v_6d6178.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f61646331323933626235623962383831: Value = arg(&arguments, 0);
                let v_625f5f63363437646239303362366233333739: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":53:51
                return if ({ let relational_left = v_615f5f61646331323933626235623962383831.clone(); let relational_right = v_625f5f63363437646239303362366233333739.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_615f5f61646331323933626235623962383831.clone() } else { v_625f5f63363437646239303362366233333739.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":54:11
            v_66696e64.initialize({  let v_6b696e64 = v_6b696e64.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_73746172745f5f36353239396566323230333135343634: Value = arg(&arguments, 0);
                let v_656e645f5f65623363646233383039313734366530: Value = arg(&arguments, 1);
                let v_666f7277617264: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":55:7
                if ({ let relational_left = v_656e645f5f65623363646233383039313734366530.clone(); let relational_right = v_73746172745f5f36353239396566323230333135343634.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":55:25
                    return Value::number(-(Value::number(1.0)).scalar_number());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":56:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f65623363646233383039313734366530.clone()).scalar_number() - (v_73746172745f5f36353239396566323230333135343634.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":56:30
                    return if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_73746172745f5f36353239396566323230333135343634.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_73746172745f5f36353239396566323230333135343634.clone() } else { Value::number(-(Value::number(1.0)).scalar_number()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":57:13
                let v_73756d5f5f31646438633537633361396132346663 = add(v_73746172745f5f36353239396566323230333135343634.clone(), v_656e645f5f65623363646233383039313734366530.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":58:13
                let v_6d6964646c655f5f63666162373030303034353036363530 = Value::number((Value::number((v_73756d5f5f31646438633537633361396132346663.clone()).scalar_number() - (Value::number((v_73756d5f5f31646438633537633361396132346663.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":59:13
                let v_63616e6469646174655f5f32313361336663636430643038353162 = if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_73746172745f5f36353239396566323230333135343634.clone(), v_6d6964646c655f5f63666162373030303034353036363530.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f63666162373030303034353036363530.clone(), v_656e645f5f65623363646233383039313734366530.clone(), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":60:7
                if ({ let relational_left = v_63616e6469646174655f5f32313361336663636430643038353162.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":60:27
                    return v_63616e6469646174655f5f32313361336663636430643038353162.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":61:7
                return if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f63666162373030303034353036363530.clone(), v_656e645f5f65623363646233383039313734366530.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_73746172745f5f36353239396566323230333135343634.clone(), v_6d6964646c655f5f63666162373030303034353036363530.clone(), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":63:11
            v_7365656b.initialize({  let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_726571756573746564: Value = arg(&arguments, 0);
                let v_646972656374696f6e: Value = arg(&arguments, 1);
                let v_5f72656d61696e696e67: Value = arg(&arguments, 2);
                let v_77726170: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":64:13
                let v_696e6465785f5f39363064626363623231316531616535 = v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), v_6d6178.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), v_726571756573746564.clone()])]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":65:13
                let v_63616e6469646174655f5f31663565306335626263623334323931 = if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f39363064626363623231316531616535.clone()]), v_636f756e74.read().call_authored(vec![]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f39363064626363623231316531616535.clone(), Value::number(1.0))]), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":66:7
                if ({ let left = { let relational_left = v_63616e6469646174655f5f31663565306335626263623334323931.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if !left.truthy() { Value::boolean(!(v_77726170.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":66:36
                    return v_63616e6469646174655f5f31663565306335626263623334323931.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":67:7
                return if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f39363064626363623231316531616535.clone()]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f39363064626363623231316531616535.clone(), Value::number(1.0))]), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":69:11
            v_76616c69644974656d73.initialize({  let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76616c756573: Value = arg(&arguments, 0);
                let v_73746172745f5f62343436383630313961396335613930: Value = arg(&arguments, 1);
                let v_656e645f5f33383662376437636264393930643563: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":70:7
                if ({ let relational_left = v_656e645f5f33383662376437636264393930643563.clone(); let relational_right = v_73746172745f5f62343436383630313961396335613930.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":70:25
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":71:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f33383662376437636264393930643563.clone()).scalar_number() - (v_73746172745f5f62343436383630313961396335613930.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":71:30
                    return { let left = { let left = Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62343436383630313961396335613930.clone())), false) }), &(Value::number(0.0)))); if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62343436383630313961396335613930.clone())), false) }), &(Value::number(1.0)))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62343436383630313961396335613930.clone())), false) }), &(Value::number(2.0)))) } else { left } };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":72:13
                let v_73756d5f5f35663037613361313637386236313636 = add(v_73746172745f5f62343436383630313961396335613930.clone(), v_656e645f5f33383662376437636264393930643563.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":73:13
                let v_6d6964646c655f5f30353535383632333764393232326664 = Value::number((Value::number((v_73756d5f5f35663037613361313637386236313636.clone()).scalar_number() - (Value::number((v_73756d5f5f35663037613361313637386236313636.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":74:7
                return { let left = v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_73746172745f5f62343436383630313961396335613930.clone(), v_6d6964646c655f5f30353535383632333764393232326664.clone()]); if left.truthy() { v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_6d6964646c655f5f30353535383632333764393232326664.clone(), v_656e645f5f33383662376437636264393930643563.clone()]) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":76:11
            v_656d6974.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f65366233343937663035313961303965: Value = arg(&arguments, 0);
                let v_696e6465785f5f61663239313534343762383631333566: Value = arg(&arguments, 1);
                let v_657874656e645f5f64353066666233366231356433393534: Value = arg(&arguments, 2);
                let v_6b65795f5f35333764613564666435393230373662: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":77:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":77:20
                    { let v_7075695f6e61746976655f3331 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3331.clone()).owner(), v_6e616d655f5f65366233343937663035313961303965.clone(), record(vec![("index", v_696e6465785f5f61663239313534343762383631333566.clone()), ("extend", v_657874656e645f5f64353066666233366231356433393534.clone()), ("key", v_6b65795f5f35333764613564666435393230373662.clone()), ("revision", { let left = ({ let v_7075695f6e61746976655f3332 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3332.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":79:11
            v_7265717565737452657665616c.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f31663233636663313663663464386364: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":80:13
                let v_6f776e65725f5f39643830616330376332393265373739 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":81:7
                if ({ let left = Value::boolean(!(v_6f776e65725f5f39643830616330376332393265373739.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":81:31
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":82:13
                let v_6d6f64655f5f35356636316330643939356265656630 = ({ let v_7075695f6e61746976655f3333 = v_6f776e65725f5f39643830616330376332393265373739.clone(); read_props(&(v_7075695f6e61746976655f3333.clone()).owner(), false) }).member("mode", false);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":83:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f35356636316330643939356265656630.clone()), &(Value::string("filterlist")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f35356636316330643939356265656630.clone()), &(Value::string("tracklist")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":83:58
                    v_656d6974.read().call_authored(vec![Value::string("command"), v_696e6465785f5f31663233636663313663663464386364.clone(), Value::boolean(false), Value::string("reveal")]);
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":85:11
            v_73656c656374.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f38353039306265616663623766326431: Value = arg(&arguments, 0);
                let v_657874656e645f5f37326332333662666366393631623162: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":86:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f38353039306265616663623766326431.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":86:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":87:7
                if ({ let left = Value::boolean(strict_equal(&(v_696e6465785f5f38353039306265616663623766326431.clone()), &({ let v_7075695f6e61746976655f3334 = v_637572736f72.read(); (v_7075695f6e61746976655f3334.clone()).get() }))); if left.truthy() { Value::boolean(!(v_657874656e645f5f37326332333662666366393631623162.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":87:46
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":88:13
                let v_7469636b65745f5f36333432366438353761396234343532 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":89:7
                { let v_7075695f6e61746976655f3335 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3335.clone(), v_696e6465785f5f38353039306265616663623766326431.clone(), Value::string("reason: collection selection")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":90:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f36333432366438353761396234343532.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":90:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":91:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f38353039306265616663623766326431.clone(), v_657874656e645f5f37326332333662666366393631623162.clone(), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":92:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f36333432366438353761396234343532.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":92:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":93:7
                v_7265717565737452657665616c.read().call_authored(vec![v_696e6465785f5f38353039306265616663623766326431.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":94:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f36333432366438353761396234343532.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":94:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":95:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_7469636b65745f5f36333432366438353761396234343532.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":95:40
                    { let v_7075695f6e61746976655f3336 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3336.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":96:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":98:11
            v_63686f6f7365.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f65333230333638366538356262666661: Value = arg(&arguments, 0);
                let v_657874656e645f5f64626562643333366135633937393632: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":99:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f65333230333638366538356262666661.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":99:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":100:13
                let v_7469636b65745f5f39393734383163353330336539653965 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":101:7
                { let v_7075695f6e61746976655f3337 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3337.clone(), v_696e6465785f5f65333230333638366538356262666661.clone(), Value::string("reason: collection activation cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":102:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39393734383163353330336539653965.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":102:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":103:7
                if (Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f65333230333638366538356262666661.clone()])), &(Value::number(2.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":104:15
                    let v_6e6578745f5f37303165653736613762376534656662 = if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3338 = v_657870616e646564.read(); (v_7075695f6e61746976655f3338.clone()).get() }), &(v_696e6465785f5f65333230333638366538356262666661.clone())))).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { v_696e6465785f5f65333230333638366538356262666661.clone() };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":105:9
                    { let v_7075695f6e61746976655f3339 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3339.clone(), v_6e6578745f5f37303165653736613762376534656662.clone(), Value::string("reason: submenu expansion")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":106:9
                    if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f39393734383163353330336539653965.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":106:43
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":107:9
                    v_656d6974.read().call_authored(vec![Value::string("expand"), v_6e6578745f5f37303165653736613762376534656662.clone(), Value::boolean(false), Value::string("")]);
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":108:14
                    v_656d6974.read().call_authored(vec![Value::string("activate"), v_696e6465785f5f65333230333638366538356262666661.clone(), v_657874656e645f5f64626562643333366135633937393632.clone(), Value::string("")]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":109:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":111:11
            v_636f6e746578744174.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f37666333333635326439396563356138: Value = arg(&arguments, 0);
                let v_62617369735f5f31396232633263663964333761636662: Value = arg(&arguments, 1);
                let v_78: Value = arg(&arguments, 2);
                let v_79: Value = arg(&arguments, 3);
                let v_6b6579626f617264: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":112:13
                let v_6f776e65725f5f38306433613263353330653339336166 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":113:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f38306433613263353330653339336166.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f31396232633263663964333761636662.clone()), &({ let left = ({ let v_7075695f6e61746976655f3430 = v_6f776e65725f5f38306433613263353330653339336166.clone(); read_props(&(v_7075695f6e61746976655f3430.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f37666333333635326439396563356138.clone()])), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_696e6465785f5f37666333333635326439396563356138.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(Value::number((v_696e6465785f5f37666333333635326439396563356138.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_78.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_79.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":114:102
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":115:13
                let v_7469636b65745f5f63663734663235656631623262353165 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":116:7
                { let v_7075695f6e61746976655f3431 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3431.clone(), v_696e6465785f5f37666333333635326439396563356138.clone(), Value::string("reason: collection context cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":117:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f63663734663235656631623262353165.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":117:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":118:7
                { let v_7075695f6e61746976655f3432 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3432.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":119:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f63663734663235656631623262353165.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":119:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":120:7
                { let v_7075695f6e61746976655f3433 = v_6f776e65725f5f38306433613263353330653339336166.clone(); emit(&(v_7075695f6e61746976655f3433.clone()).owner(), Value::string("context"), record(vec![("index", v_696e6465785f5f37666333333635326439396563356138.clone()), ("x", v_78.clone()), ("y", v_79.clone()), ("keyboard", v_6b6579626f617264.clone()), ("revision", v_62617369735f5f31396232633263663964333761636662.clone())])) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":121:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":123:11
            v_6b65795f5f30633866646432663039343166323131.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_73656c656374 = v_73656c656374.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let v_636f6e746578744174 = v_636f6e746578744174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f34363035653461353162643431343231: Value = arg(&arguments, 0);
                let v_73686966745f5f66396534343965346638393930623436: Value = arg(&arguments, 1);
                let v_636f6d6d616e645f5f33323964353433643535333665323139: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":124:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":124:33
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":125:13
                let v_6d6f64655f5f38613839386363316264316362373466 = { let left = ({ let v_7075695f6e61746976655f3434 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3434.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":126:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("filterlist")))); if left.truthy() { v_636f6d6d616e645f5f33323964353433643535333665323139.clone() } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":126:81
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3435 = v_637572736f72.read(); (v_7075695f6e61746976655f3435.clone()).get() }, Value::boolean(false), Value::string("focus-filter")]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":126:135
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":127:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("tracklist")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("F10")))); if left.truthy() { v_73686966745f5f66396534343965346638393930623436.clone() } else { left } } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":127:90
                    return v_636f6e746578744174.read().call_authored(vec![{ let v_7075695f6e61746976655f3436 = v_637572736f72.read(); (v_7075695f6e61746976655f3436.clone()).get() }, { let left = ({ let v_7075695f6e61746976655f3437 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3437.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }, Value::number(0.0), Value::number(0.0), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":128:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("points")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Enter")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":128:75
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3438 = v_637572736f72.read(); (v_7075695f6e61746976655f3438.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":128:117
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":129:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("points")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Tab")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":130:15
                    let v_5f5f7075694d6f64756c6530 = add({ let v_7075695f6e61746976655f3439 = v_637572736f72.read(); (v_7075695f6e61746976655f3439.clone()).get() }, if (v_73686966745f5f66396534343965346638393930623436.clone()).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { Value::number(1.0) });
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":131:9
                    if ({ let left = { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if !left.truthy() { { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = v_636f756e74.read().call_authored(vec![]); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":131:42
                        v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3530 = v_637572736f72.read(); (v_7075695f6e61746976655f3530.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":132:14
                        v_73656c656374.read().call_authored(vec![v_5f5f7075694d6f64756c6530.clone(), Value::boolean(false)]);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":133:9
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":135:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("points")))); if left.truthy() { { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowDown")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("+")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("=")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("-")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":135:128
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3531 = v_637572736f72.read(); (v_7075695f6e61746976655f3531.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":135:172
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":136:7
                if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Escape")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Tab")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("menu")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":136:71
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3532 = v_637572736f72.read(); (v_7075695f6e61746976655f3532.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":136:113
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":137:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Enter")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string(" ")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("F2")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("table")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":137:84
                    return v_63686f6f7365.read().call_authored(vec![{ let v_7075695f6e61746976655f3533 = v_637572736f72.read(); (v_7075695f6e61746976655f3533.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":138:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("table")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Backspace")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Tab")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":139:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3534 = v_637572736f72.read(); (v_7075695f6e61746976655f3534.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":139:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":141:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("table")))); if left.truthy() { v_636f6d6d616e645f5f33323964353433643535333665323139.clone() } else { left } }; if left.truthy() { { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowRight")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowDown")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":142:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3535 = v_637572736f72.read(); (v_7075695f6e61746976655f3535.clone()).get() }, v_73686966745f5f66396534343965346638393930623436.clone(), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":142:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":144:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":145:9
                    if ({ let relational_left = { let v_7075695f6e61746976655f3536 = v_657870616e646564.read(); (v_7075695f6e61746976655f3536.clone()).get() }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":145:33
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":146:15
                    let v_706172656e74 = { let v_7075695f6e61746976655f3537 = v_657870616e646564.read(); (v_7075695f6e61746976655f3537.clone()).get() };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":146:40
                    { let v_7075695f6e61746976655f3538 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3538.clone(), Value::number(-(Value::number(1.0)).scalar_number()), Value::string("reason: submenu close")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":147:9
                    v_656d6974.read().call_authored(vec![Value::string("expand"), Value::number(-(Value::number(1.0)).scalar_number()), Value::boolean(false), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":147:42
                    v_73656c656374.read().call_authored(vec![v_706172656e74.clone(), Value::boolean(false)]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":147:65
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":149:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":150:9
                    if (Value::boolean(!strict_equal(&(v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f3539 = v_637572736f72.read(); (v_7075695f6e61746976655f3539.clone()).get() }])), &(Value::number(2.0))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":150:39
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:9
                    { let v_7075695f6e61746976655f3630 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3630.clone(), { let v_7075695f6e61746976655f3631 = v_637572736f72.read(); (v_7075695f6e61746976655f3631.clone()).get() }, Value::string("reason: submenu open")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:61
                    v_656d6974.read().call_authored(vec![Value::string("expand"), { let v_7075695f6e61746976655f3632 = v_637572736f72.read(); (v_7075695f6e61746976655f3632.clone()).get() }, Value::boolean(false), v_6e616d655f5f34363035653461353162643431343231.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":151:104
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":153:11
                let v_6e6578745f5f33316364633465366236663364373633 = Rc::new(std::cell::RefCell::new(Value::number(-(Value::number(1.0)).scalar_number())));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":154:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Home"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:28
                    { let value = v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":155:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("End"))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":155:32
                        { let value = v_7365656b.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":156:12
                        if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":156:60
                            { let value = v_7365656b.read().call_authored(vec![Value::number(({ let v_7075695f6e61746976655f3633 = v_637572736f72.read(); (v_7075695f6e61746976655f3633.clone()).get() }).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":157:12
                            if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowDown")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":157:63
                                { let value = v_7365656b.read().call_authored(vec![add({ let v_7075695f6e61746976655f3634 = v_637572736f72.read(); (v_7075695f6e61746976655f3634.clone()).get() }, Value::number(1.0)), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":158:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("PageUp"))))).truthy() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":158:35
                                    { let value = v_7365656b.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), Value::number(({ let v_7075695f6e61746976655f3635 = v_637572736f72.read(); (v_7075695f6e61746976655f3635.clone()).get() }).scalar_number() - (if (Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) }).scalar_number())]), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":159:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("PageDown"))))).truthy() {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":159:37
                                        { let value = v_7365656b.read().call_authored(vec![v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), add({ let v_7075695f6e61746976655f3636 = v_637572736f72.read(); (v_7075695f6e61746976655f3636.clone()).get() }, if (Value::boolean(strict_equal(&(v_6d6f64655f5f38613839386363316264316362373466.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) })]), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f33316364633465366236663364373633.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":160:12
                                        return Value::boolean(false);
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":161:7
                if ({ let left = { let relational_left = { let value = (v_6e6578745f5f33316364633465366236663364373633.clone()).borrow().clone(); value }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!(v_73656c656374.read().call_authored(vec![{ let value = (v_6e6578745f5f33316364633465366236663364373633.clone()).borrow().clone(); value }, v_73686966745f5f66396534343965346638393930623436.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":161:46
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":162:7
                if (Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":162:21
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":163:13
                let v_686f72697a6f6e74616c = Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3637 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3637.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3637.clone()).owner(), false) } }).member("orientation", true)), &(Value::string("horizontal"))));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":164:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("PageUp"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":164:32
                    if (v_686f72697a6f6e74616c.clone()).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":164:48
                        { let v_7075695f6e61746976655f3638 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3638.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":164:128
                        { let v_7075695f6e61746976655f3639 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3639.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":165:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("PageDown"))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":165:39
                        if (v_686f72697a6f6e74616c.clone()).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":165:55
                            { let v_7075695f6e61746976655f3730 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3730.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":165:134
                            { let v_7075695f6e61746976655f3731 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3731.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":166:12
                        if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("Home"))))).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":166:35
                            if (v_686f72697a6f6e74616c.clone()).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":166:51
                                { let v_7075695f6e61746976655f3732 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3732.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0))])) };
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":166:121
                                { let v_7075695f6e61746976655f3733 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3733.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":167:12
                            if (Value::boolean(strict_equal(&(v_6e616d655f5f34363035653461353162643431343231.clone()), &(Value::string("End"))))).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":167:34
                                if (v_686f72697a6f6e74616c.clone()).truthy() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":167:50
                                    { let v_7075695f6e61746976655f3734 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3734.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":167:111
                                    { let v_7075695f6e61746976655f3735 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3735.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":168:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":173:5
            { let v_7075695f6e61746976655f3736 = v_646566.clone(); expose_method(&owner, Value::string("filterKey"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b65795f5f30633866646432663039343166323131 = v_6b65795f5f30633866646432663039343166323131.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f33643662646565323261363661653566: Value = arg(&arguments, 0);
                let v_73686966745f5f36323136343439306364363665623962: Value = arg(&arguments, 1);
                let v_636f6d6d616e645f5f62663166346137653266656365336166: Value = arg(&arguments, 2);
                let v_636f6d706f73696e67: Value = arg(&arguments, 3);
                let v_66726f6d46696c746572: Value = arg(&arguments, 4);
                let v_62617369735f5f32386436643434653933393333363665: Value = arg(&arguments, 5);
                let v_616c74: Value = arg(&arguments, 6);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":174:13
                let v_6f776e65725f5f35623830376465313439396131393462 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":175:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f35623830376465313439396131393462.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { v_636f6d706f73696e67.clone() } else { left } }; if !left.truthy() { v_616c74.clone() } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f3737 = v_6f776e65725f5f35623830376465313439396131393462.clone(); read_props(&(v_7075695f6e61746976655f3737.clone()).owner(), false) }).member("mode", false)), &(Value::string("filterlist")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f32386436643434653933393333363665.clone()), &({ let left = ({ let v_7075695f6e61746976655f3738 = v_6f776e65725f5f35623830376465313439396131393462.clone(); read_props(&(v_7075695f6e61746976655f3738.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":175:142
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":176:7
                if ({ let left = v_636f6d6d616e645f5f62663166346137653266656365336166.clone(); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":176:54
                    return v_6b65795f5f30633866646432663039343166323131.read().call_authored(vec![v_6e616d655f5f33643662646565323261363661653566.clone(), v_73686966745f5f36323136343439306364363665623962.clone(), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":177:7
                if ({ let left = Value::boolean(!(v_66726f6d46696c746572.clone()).truthy()); if !left.truthy() { v_636f6d6d616e645f5f62663166346137653266656365336166.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":177:35
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":178:7
                if ({ let left = { let left = { let left = Value::boolean(!strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("ArrowUp")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("Enter")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f33643662646565323261363661653566.clone()), &(Value::string("Escape")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":178:96
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":179:7
                return v_6b65795f5f30633866646432663039343166323131.read().call_authored(vec![v_6e616d655f5f33643662646565323261363661653566.clone(), v_73686966745f5f36323136343439306364363665623962.clone(), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":184:5
            { let v_7075695f6e61746976655f3739 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f34646362656166336465626136656238: Value = arg(&arguments, 0);
                let v_73746172745f5f37646366623534643336633266373362: Value = arg(&arguments, 1);
                let v_656e645f5f64663539653134653463333134663236: Value = arg(&arguments, 2);
                let v_76696577706f72745374617274: Value = arg(&arguments, 3);
                let v_76696577706f7274456e64: Value = arg(&arguments, 4);
                let v_62617369735f5f32313766623038313338616233653533: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":185:13
                let v_6f776e65725f5f32366133643039303534653065626561 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":186:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f32366133643039303534653065626561.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f32313766623038313338616233653533.clone()), &({ let left = ({ let v_7075695f6e61746976655f3830 = v_6f776e65725f5f32366133643039303534653065626561.clone(); read_props(&(v_7075695f6e61746976655f3830.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_696e6465785f5f34646362656166336465626136656238.clone()), &({ let v_7075695f6e61746976655f3831 = v_637572736f72.read(); (v_7075695f6e61746976655f3831.clone()).get() }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f34646362656166336465626136656238.clone()])), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_73746172745f5f37646366623534643336633266373362.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_656e645f5f64663539653134653463333134663236.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f72745374617274.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f7274456e64.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_656e645f5f64663539653134653463333134663236.clone(); let relational_right = v_73746172745f5f37646366623534643336633266373362.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let relational_left = v_76696577706f7274456e64.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":188:58
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":189:7
                if ({ let left = { let left = { let relational_left = v_73746172745f5f37646366623534643336633266373362.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_656e645f5f64663539653134653463333134663236.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let left = { let relational_left = v_73746172745f5f37646366623534643336633266373362.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_656e645f5f64663539653134653463333134663236.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":189:109
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":190:13
                let v_6c656164696e67 = Value::number((v_73746172745f5f37646366623534643336633266373362.clone()).scalar_number() - (v_76696577706f72745374617274.clone()).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":190:52
                let v_747261696c696e67 = Value::number((v_656e645f5f64663539653134653463333134663236.clone()).scalar_number() - (v_76696577706f7274456e64.clone()).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":191:13
                let v_6c656164696e6744697374616e6365 = if ({ let relational_left = v_6c656164696e67.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_6c656164696e67.clone()).scalar_number()) } else { v_6c656164696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":191:71
                let v_747261696c696e6744697374616e6365 = if ({ let relational_left = v_747261696c696e67.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_747261696c696e67.clone()).scalar_number()) } else { v_747261696c696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":192:13
                let v_64656c7461 = if ({ let relational_left = v_6c656164696e6744697374616e6365.clone(); let relational_right = v_747261696c696e6744697374616e6365.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_6c656164696e67.clone() } else { v_747261696c696e67.clone() };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":193:7
                if (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f3832 = v_6f776e65725f5f32366133643039303534653065626561.clone(); read_props(&(v_7075695f6e61746976655f3832.clone()).owner(), false) }).member("orientation", false)), &(Value::string("horizontal"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":193:59
                    { let v_7075695f6e61746976655f3833 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3833.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", v_64656c7461.clone())])) };
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":194:12
                    { let v_7075695f6e61746976655f3834 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3834.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", v_64656c7461.clone())])) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":195:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":197:5
            { let v_7075695f6e61746976655f3835 = v_646566.clone(); expose_method(&owner, Value::string("setItems"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6974656d73 = v_6974656d73.clone(); let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f32633933663637623364656436303136: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":198:7
                if ({ let left = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { { let relational_left = (v_6e6578745f5f32633933663637623364656436303136.clone()).member("length", false); let relational_right = Value::number(1000000.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }; if !left.truthy() { Value::boolean(!(v_76616c69644974656d73.read().call_authored(vec![v_6e6578745f5f32633933663637623364656436303136.clone(), Value::number(0.0), (v_6e6578745f5f32633933663637623364656436303136.clone()).member("length", false)])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":198:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":199:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":199:16
                { let value = v_6e6578745f5f32633933663637623364656436303136.clone(); *(v_6974656d73.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":200:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":200:20
                    v_73796e63.read().call_authored(vec![{ let value = (v_63757272656e74.read()).borrow().clone(); value }]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":201:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":203:5
            { let v_7075695f6e61746976655f3836 = v_646566.clone(); expose_method(&owner, Value::string("choose"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f36366239323535306639313430636233: Value = arg(&arguments, 0);
                let v_657874656e645f5f64313734373739336163356439373062: Value = arg(&arguments, 1);
                let v_62617369735f5f39623661643031613630356536303534: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":204:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f39623661643031613630356536303534.clone()), &({ let left = ({ let v_7075695f6e61746976655f3837 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3837.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3837.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":204:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:7
                return v_63686f6f7365.read().call_authored(vec![v_696e6465785f5f36366239323535306639313430636233.clone(), v_657874656e645f5f64313734373739336163356439373062.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":207:5
            { let v_7075695f6e61746976655f3838 = v_646566.clone(); expose_method(&owner, Value::string("selectIndex"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f39653031373339376530633364323964: Value = arg(&arguments, 0);
                let v_657874656e645f5f32333432616663346630616438666232: Value = arg(&arguments, 1);
                let v_62617369735f5f34623533323234343365636234326265: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":208:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f34623533323234343365636234326265.clone()), &({ let left = ({ let v_7075695f6e61746976655f3839 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3839.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3839.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":208:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":209:7
                return v_73656c656374.read().call_authored(vec![v_696e6465785f5f39653031373339376530633364323964.clone(), v_657874656e645f5f32333432616663346630616438666232.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":213:5
            { let v_7075695f6e61746976655f3930 = v_646566.clone(); expose_method(&owner, Value::string("keyAt"), {  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6b65795f5f30633866646432663039343166323131 = v_6b65795f5f30633866646432663039343166323131.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f33376332343664663538323135663233: Value = arg(&arguments, 0);
                let v_6e616d655f5f62636630663530636232353030613432: Value = arg(&arguments, 1);
                let v_73686966745f5f30396135303961363235363163346431: Value = arg(&arguments, 2);
                let v_636f6d6d616e645f5f36653533636362313165363139633662: Value = arg(&arguments, 3);
                let v_62617369735f5f39313439383963623366323530313666: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:7
                if ({ let left = { let left = { let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f39313439383963623366323530313666.clone()), &({ let left = ({ let v_7075695f6e61746976655f3931 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3931.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f33376332343664663538323135663233.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":214:104
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":215:13
                let v_7469636b65745f5f37326137313234393139383539633864 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":216:7
                { let v_7075695f6e61746976655f3932 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3932.clone(), v_696e6465785f5f33376332343664663538323135663233.clone(), Value::string("reason: addressed collection key")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":217:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f37326137313234393139383539633864.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":217:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":218:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f33376332343664663538323135663233.clone(), Value::boolean(false), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":219:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f37326137313234393139383539633864.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f39313439383963623366323530313666.clone()), &({ let left = ({ let v_7075695f6e61746976655f3933 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3933.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3933.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":219:92
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:7
                return v_6b65795f5f30633866646432663039343166323131.read().call_authored(vec![v_6e616d655f5f62636630663530636232353030613432.clone(), v_73686966745f5f30396135303961363235363163346431.clone(), v_636f6d6d616e645f5f36653533636362313165363139633662.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":222:5
            { let v_7075695f6e61746976655f3934 = v_646566.clone(); expose_method(&owner, Value::string("selectAndFocus"), {  let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f31343936343536626532373736303361: Value = arg(&arguments, 0);
                let v_657874656e645f5f61623539303732306639393137336437: Value = arg(&arguments, 1);
                let v_62617369735f5f61633865326331373164396261643432: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":223:7
                if ({ let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f61633865326331373164396261643432.clone()), &({ let left = ({ let v_7075695f6e61746976655f3935 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3935.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":223:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":224:7
                if ({ let left = Value::boolean(!(v_73656c656374.read().call_authored(vec![v_696e6465785f5f31343936343536626532373736303361.clone(), v_657874656e645f5f61623539303732306639393137336437.clone()])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f61633865326331373164396261643432.clone()), &({ let left = ({ let v_7075695f6e61746976655f3936 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3936.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3936.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":224:86
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":225:7
                { let v_7075695f6e61746976655f3937 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3937.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":226:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":228:5
            { let v_7075695f6e61746976655f3938 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b65795f5f30633866646432663039343166323131.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":229:5
            { let v_7075695f6e61746976655f3939 = v_646566.clone(); expose_method(&owner, Value::string("contextAt"), v_636f6e746578744174.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":230:5
            { let v_7075695f6e61746976655f313030 = v_646566.clone(); expose_method(&owner, Value::string("reconcile"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:50
                let v_6f776e65725f5f30373637303663643337383164663230 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:67
                if ({ let left = Value::boolean(!(v_6f776e65725f5f30373637303663643337383164663230.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":230:91
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:105
                v_73796e63.read().call_authored(vec![v_6f776e65725f5f30373637303663643337383164663230.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:118
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":231:5
            { let v_7075695f6e61746976655f313031 = v_646566.clone(); expose_method(&owner, Value::string("dismiss"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":231:42
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":231:55
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f313032 = v_637572736f72.read(); (v_7075695f6e61746976655f313032.clone()).get() }, Value::boolean(false), Value::string("outside")]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":232:5
            { let v_7075695f6e61746976655f313033 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":232:44
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":232:57
                    { let v_7075695f6e61746976655f313034 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313034.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":233:5
            { let v_7075695f6e61746976655f313035 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b65795f5f30633866646432663039343166323131 = v_6b65795f5f30633866646432663039343166323131.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":234:7
                if (v_6b65795f5f30633866646432663039343166323131.read().call_authored(vec![{ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true)))), { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true)))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true)))) } else { left } }])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":234:108
                    { let v_7075695f6e61746976655f313036 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f313036.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":236:11
            v_73796e63.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_7461626c65 = v_7461626c65.clone(); let v_726f7773 = v_726f7773.clone(); let v_636f6c756d6e73 = v_636f6c756d6e73.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f65643836353530333561643335373639: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":237:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":237:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":238:13
                let v_7469636b65745f5f37363765333739666635326434393335 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":238:31
                { let value = v_72756e5f5f65643836353530333561643335373639.clone(); *(v_63757272656e74.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":239:13
                let v_70726f7073 = { let v_7075695f6e61746976655f313037 = v_72756e5f5f65643836353530333561643335373639.clone(); read_props(&(v_7075695f6e61746976655f313037.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":240:7
                { let v_7075695f6e61746976655f313038 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f313038.clone(), if ({ let left = { let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("points")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("tracklist")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist")))) } else { left } }).truthy() { Value::string("listbox") } else { { let left = (v_70726f7073.clone()).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } } }, Value::string("reason: collection role")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":241:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":241:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":242:7
                { let v_7075695f6e61746976655f313039 = v_7461626c65.read(); state_set(v_7075695f6e61746976655f313039.clone(), Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("table")))), Value::string("reason: collection table semantics")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":243:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":243:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":244:7
                { let v_7075695f6e61746976655f313130 = v_726f7773.read(); state_set(v_7075695f6e61746976655f313130.clone(), v_636f756e74.read().call_authored(vec![]), Value::string("reason: collection rows")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":245:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":245:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":246:7
                { let v_7075695f6e61746976655f313131 = v_636f6c756d6e73.read(); state_set(v_7075695f6e61746976655f313131.clone(), { let left = (v_70726f7073.clone()).member("columns", false); if left.nullish() { Value::number(0.0) } else { left } }, Value::string("reason: collection columns")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":247:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":247:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":248:7
                { let v_7075695f6e61746976655f313132 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313132.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection name")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":249:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":249:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":250:7
                { let v_7075695f6e61746976655f313133 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313133.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: collection policy")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":251:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":251:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":252:7
                { let v_7075695f6e61746976655f313134 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313134.clone()).owner(), { let v_7075695f6e61746976655f313135 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313135.clone()).get() }) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":253:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":253:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":254:13
                let v_6163636570746564 = v_6d6178.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), { let left = (v_70726f7073.clone()).member("cursor", false); if left.nullish() { Value::number(0.0) } else { left } }])]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":255:7
                { let v_7075695f6e61746976655f313136 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313136.clone(), if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_6163636570746564.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_6163636570746564.clone() } else { v_6d6178.read().call_authored(vec![Value::number(0.0), v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)])]) }, Value::string("reason: accepted collection cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":256:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":256:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":257:7
                { let v_7075695f6e61746976655f313137 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f313137.clone(), { let left = (v_70726f7073.clone()).member("expanded", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }, Value::string("reason: accepted expanded item")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":258:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":258:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:7
                if ({ let left = v_7265616479.read().call_authored(vec![]); if left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":263:9
                    if (Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f313138 = v_637572736f72.read(); (v_7075695f6e61746976655f313138.clone()).get() }])), &(Value::number(0.0))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":263:39
                        v_656d6974.read().call_authored(vec![Value::string("select"), Value::number(-(Value::number(1.0)).scalar_number()), Value::boolean(false), Value::string("reconcile")]);
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":264:14
                        if (Value::boolean(!strict_equal(&((v_70726f7073.clone()).member("cursor", false)), &({ let v_7075695f6e61746976655f313139 = v_637572736f72.read(); (v_7075695f6e61746976655f313139.clone()).get() })))).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":264:49
                            v_656d6974.read().call_authored(vec![Value::string("select"), { let v_7075695f6e61746976655f313230 = v_637572736f72.read(); (v_7075695f6e61746976655f313230.clone()).get() }, Value::boolean(false), Value::string("reconcile")]);
                        }
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":265:9
                    if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":265:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":266:9
                    if ({ let relational_left = v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f313231 = v_637572736f72.read(); (v_7075695f6e61746976655f313231.clone()).get() }]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":266:37
                        v_7265717565737452657665616c.read().call_authored(vec![{ let v_7075695f6e61746976655f313232 = v_637572736f72.read(); (v_7075695f6e61746976655f313232.clone()).get() }]);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":267:9
                    if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37363765333739666635326434393335.clone()))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":267:43
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":269:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":269:21
                    { let v_7075695f6e61746976655f313233 = v_72756e5f5f65643836353530333561643335373639.clone(); request_update(&(v_7075695f6e61746976655f313233.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":271:5
            { let v_7075695f6e61746976655f313234 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":272:5
            { let v_7075695f6e61746976655f313235 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f35623034643662393533383033313138: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":272:40
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":272:57
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":272:74
                    v_73796e63.read().call_authored(vec![v_72756e5f5f35623034643662393533383033313138.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":272:85
                    if (({ let v_7075695f6e61746976655f313236 = v_72756e5f5f35623034643662393533383033313138.clone(); read_props(&(v_7075695f6e61746976655f313236.clone()).owner(), false) }).member("autoFocus", false)).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":272:116
                        { let v_7075695f6e61746976655f313237 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313237.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":273:5
            { let v_7075695f6e61746976655f313238 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("mode"), Value::string("label"), Value::string("cursor"), Value::string("expanded"), Value::string("revision"), Value::string("disabled"), Value::string("autoFocus"), Value::string("columns"), Value::string("orientation")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":274:5
            { let v_7075695f6e61746976655f313239 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":274:39
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":274:48
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":274:66
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":275:5
            { let v_7075695f6e61746976655f313330 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:43
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:52
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:69
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:87
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":276:5
            { let v_7075695f6e61746976655f313331 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":277:5
            return { let callable={ let pui_native_132={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":277:26
                return { let v_7075695f6e61746976655f313333 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_132.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_132.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> CollectionControlHandle { CollectionControlHandle { owner: self.owner.clone() } }
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
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
