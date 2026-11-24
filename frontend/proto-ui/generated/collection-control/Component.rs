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
        "chooseSelection" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
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
    pub fn method_15(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: String, argument_6: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_16(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_17(&self, argument_0: Vec<f64>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_18(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_19(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_20(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_21(&self, argument_0: f64, argument_1: String, argument_2: bool, argument_3: bool, argument_4: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_22(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_23(&self, argument_0: String, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_24(&self, argument_0: f64, argument_1: String, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_25(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_26(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
    pub fn method_27(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
            let v_63686f6f736553656c656374696f6e: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool, bool) -> bool>> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, bool) -> bool>> = lexical_binding();
            let v_636f6e746578744174: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, Value, f64, f64, bool) -> bool>> = lexical_binding();
            let v_6b65795f5f33313237306232626462353633303563: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool, bool) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/collection-control.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("mode", record(vec![("type", Value::string("string")), ("default", Value::string("listbox"))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("cursor", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("orientation", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("expanded", record(vec![("type", Value::string("number")), ("default", Value::number(-(1.0_f64)))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("columns", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))]))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":23:11
            v_637572736f72.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cursor"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":24:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "discrete", Value::string("expanded"), Value::number(-(1.0_f64)), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":25:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":26:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("listbox"), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":27:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":28:11
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:11
            v_7461626c65.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("table"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:11
            v_726f7773.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("rows"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:11
            v_636f6c756d6e73.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "discrete", Value::string("columns"), Value::number(0.0_f64), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:34
            { let v_7075695f6e61746976655f3130 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:11
            v_7363726f6c6c.initialize(native_builtin_hook(&owner, "asScrollSurface", || scroll_setup(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:39
            { let v_7075695f6e61746976655f3131 = v_7363726f6c6c.read(); scroll_configure(&(v_7075695f6e61746976655f3131.clone()).owner(), record(vec![("axes", Value::string("both")), ("projection", Value::string("system"))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:34
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3132.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:51
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3133.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:69
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3134.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:100
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("rowCount"), v_726f7773.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:52
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("columnCount"), v_636f6c756d6e73.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("scrollPosition"), ((v_7363726f6c6c.read()).member("vertical", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:67
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("scrollAtEnd"), ((v_7363726f6c6c.read()).member("vertical", false)).member("atEnd", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("horizontalPosition"), ((v_7363726f6c6c.read()).member("horizontal", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_state(&owner, Value::string("mode"), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("cursor"), v_637572736f72.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:41
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("expanded"), v_657870616e646564.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":40:49
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_event(&owner, Value::string("select")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_event(&owner, Value::string("selection")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("expand")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("close")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":48:9
            v_63757272656e74.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":49:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":50:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":51:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":52:11
            v_7265616479.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":52:34
                return { let pui_native_33={ let pui_native_35={ let pui_native_37=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e74.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_37) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_37} }; if (pui_native_35) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_35} }; if (pui_native_33) {!(({ let v_7075695f6e61746976655f3431 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3431.clone()).get()).bool()) }).bool())} else {pui_native_33} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":53:9
            v_6974656d73.initialize(Rc::new(std::cell::RefCell::new(Value::Array(Rc::new(vec![])))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":54:11
            v_6b696e64.initialize({  let v_6974656d73 = v_6974656d73.clone(); let call:Rc<dyn Fn(f64)->f64>=Rc::new(move |argument_0: f64| -> f64 {
                let v_696e6465785f5f30326361363239303864656461346262: f64 = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":54:45
                return if { let pui_native_42={ let pui_native_44={ let pui_native_46=(v_696e6465785f5f30326361363239303864656461346262.clone()).is_finite(); if (pui_native_46) {{ let pui_native_48={ let pui_native_50:f64=v_696e6465785f5f30326361363239303864656461346262.clone(); let pui_native_51:f64=1.0_f64; pui_native_50 % pui_native_51 }; let pui_native_49=0.0_f64; (pui_native_48==pui_native_49) }} else {pui_native_46} }; if (pui_native_44) {{ let pui_native_52:f64=v_696e6465785f5f30326361363239303864656461346262.clone(); let pui_native_53:f64=0.0_f64; pui_native_52 >= pui_native_53 }} else {pui_native_44} }; if (pui_native_42) {{ let pui_native_54:f64=v_696e6465785f5f30326361363239303864656461346262.clone(); let pui_native_55:f64=(({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false)).num(); pui_native_54 < pui_native_55 }} else {pui_native_42} } {{ let pui_native_56={ let object = { let value = (v_6974656d73.read()).borrow().clone(); value }; object.caught_member(&property_key(&(Value::number(v_696e6465785f5f30326361363239303864656461346262.clone()))), false) }; if pui_native_56.nullish() {0.0_f64} else {(pui_native_56).num()} }} else {0.0_f64};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":55:11
            v_636f756e74.initialize({  let v_6974656d73 = v_6974656d73.clone(); let call:Rc<dyn Fn()->f64>=Rc::new(move || -> f64 {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":55:33
                return (({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false)).num();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":56:11
            v_6d696e.initialize({   let call:Rc<dyn Fn(f64, f64)->f64>=Rc::new(move |argument_0: f64, argument_1: f64| -> f64 {
                let v_615f5f39653863383131636238333037643035: f64 = argument_0;
                let v_625f5f30656530353632313232636661633539: f64 = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":56:51
                return if { let pui_native_58:f64=v_615f5f39653863383131636238333037643035.clone(); let pui_native_59:f64=v_625f5f30656530353632313232636661633539.clone(); pui_native_58 < pui_native_59 } {v_615f5f39653863383131636238333037643035.clone()} else {v_625f5f30656530353632313232636661633539.clone()};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":57:11
            v_6d6178.initialize({   let call:Rc<dyn Fn(f64, f64)->f64>=Rc::new(move |argument_0: f64, argument_1: f64| -> f64 {
                let v_615f5f37663438653432373064303334353031: f64 = argument_0;
                let v_625f5f61376638623836616232353965653661: f64 = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":57:51
                return if { let pui_native_60:f64=v_615f5f37663438653432373064303334353031.clone(); let pui_native_61:f64=v_625f5f61376638623836616232353965653661.clone(); pui_native_60 > pui_native_61 } {v_615f5f37663438653432373064303334353031.clone()} else {v_625f5f61376638623836616232353965653661.clone()};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":58:11
            v_66696e64.initialize({  let v_6b696e64 = v_6b696e64.clone(); let v_66696e64 = v_66696e64.clone(); let call:Rc<dyn Fn(f64, f64, bool)->f64>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool| -> f64 {
                let v_73746172745f5f65363461663763383436653762373438: f64 = argument_0;
                let v_656e645f5f34643862316366323434343963653635: f64 = argument_1;
                let v_666f7277617264: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":59:7
                if { let pui_native_62:f64=v_656e645f5f34643862316366323434343963653635.clone(); let pui_native_63:f64=v_73746172745f5f65363461663763383436653762373438.clone(); pui_native_62 <= pui_native_63 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":59:25
                    return -(1.0_f64);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":60:7
                if { let pui_native_64={ let pui_native_66:f64=v_656e645f5f34643862316366323434343963653635.clone(); let pui_native_67:f64=v_73746172745f5f65363461663763383436653762373438.clone(); pui_native_66 - pui_native_67 }; let pui_native_65=1.0_f64; (pui_native_64==pui_native_65) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":60:30
                    return if { let pui_native_68:f64=(v_6b696e64.read().call)(v_73746172745f5f65363461663763383436653762373438.clone()); let pui_native_69:f64=0.0_f64; pui_native_68 > pui_native_69 } {v_73746172745f5f65363461663763383436653762373438.clone()} else {-(1.0_f64)};
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":61:13
                let v_73756d5f5f62643666613462663866383831326633: f64 = { let pui_native_70:f64=v_73746172745f5f65363461663763383436653762373438.clone(); let pui_native_71:f64=v_656e645f5f34643862316366323434343963653635.clone(); pui_native_70 + pui_native_71 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":62:13
                let v_6d6964646c655f5f33616663653530323739303465383063: f64 = { let pui_native_72:f64={ let pui_native_74:f64=v_73756d5f5f62643666613462663866383831326633.clone(); let pui_native_75:f64={ let pui_native_76:f64=v_73756d5f5f62643666613462663866383831326633.clone(); let pui_native_77:f64=2.0_f64; pui_native_76 % pui_native_77 }; pui_native_74 - pui_native_75 }; let pui_native_73:f64=2.0_f64; pui_native_72 / pui_native_73 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":63:13
                let v_63616e6469646174655f5f35626330663639323239636665636233: f64 = if v_666f7277617264.clone() {(v_66696e64.read().call)(v_73746172745f5f65363461663763383436653762373438.clone(), v_6d6964646c655f5f33616663653530323739303465383063.clone(), true)} else {(v_66696e64.read().call)(v_6d6964646c655f5f33616663653530323739303465383063.clone(), v_656e645f5f34643862316366323434343963653635.clone(), false)};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":64:7
                if { let pui_native_78:f64=v_63616e6469646174655f5f35626330663639323239636665636233.clone(); let pui_native_79:f64=0.0_f64; pui_native_78 >= pui_native_79 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":64:27
                    return v_63616e6469646174655f5f35626330663639323239636665636233.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":65:7
                return if v_666f7277617264.clone() {(v_66696e64.read().call)(v_6d6964646c655f5f33616663653530323739303465383063.clone(), v_656e645f5f34643862316366323434343963653635.clone(), true)} else {(v_66696e64.read().call)(v_73746172745f5f65363461663763383436653762373438.clone(), v_6d6964646c655f5f33616663653530323739303465383063.clone(), false)};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":67:11
            v_7365656b.initialize({  let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_66696e64 = v_66696e64.clone(); let call:Rc<dyn Fn(f64, f64, f64, bool)->f64>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: bool| -> f64 {
                let v_7265717565737465645f5f61616433346264636266643464333035: f64 = argument_0;
                let v_646972656374696f6e: f64 = argument_1;
                let v_5f72656d61696e696e67: f64 = argument_2;
                let v_77726170: bool = argument_3;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":68:13
                let v_696e6465785f5f39656537326561323230346138323830: f64 = (v_6d696e.read().call)((v_636f756e74.read().call)(), (v_6d6178.read().call)(-(1.0_f64), v_7265717565737465645f5f61616433346264636266643464333035.clone()));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":69:13
                let v_63616e6469646174655f5f33656162343237653962653835613630: f64 = if { let pui_native_80:f64=v_646972656374696f6e.clone(); let pui_native_81:f64=0.0_f64; pui_native_80 > pui_native_81 } {(v_66696e64.read().call)((v_6d6178.read().call)(0.0_f64, v_696e6465785f5f39656537326561323230346138323830.clone()), (v_636f756e74.read().call)(), true)} else {(v_66696e64.read().call)(0.0_f64, (v_6d696e.read().call)((v_636f756e74.read().call)(), { let pui_native_82:f64=v_696e6465785f5f39656537326561323230346138323830.clone(); let pui_native_83:f64=1.0_f64; pui_native_82 + pui_native_83 }), false)};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":70:7
                if { let pui_native_84={ let pui_native_86:f64=v_63616e6469646174655f5f33656162343237653962653835613630.clone(); let pui_native_87:f64=0.0_f64; pui_native_86 >= pui_native_87 }; if !(pui_native_84) {!(v_77726170.clone())} else {pui_native_84} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":70:36
                    return v_63616e6469646174655f5f33656162343237653962653835613630.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":71:7
                return if { let pui_native_88:f64=v_646972656374696f6e.clone(); let pui_native_89:f64=0.0_f64; pui_native_88 > pui_native_89 } {(v_66696e64.read().call)(0.0_f64, (v_6d6178.read().call)(0.0_f64, v_696e6465785f5f39656537326561323230346138323830.clone()), true)} else {(v_66696e64.read().call)((v_6d696e.read().call)((v_636f756e74.read().call)(), { let pui_native_90:f64=v_696e6465785f5f39656537326561323230346138323830.clone(); let pui_native_91:f64=1.0_f64; pui_native_90 + pui_native_91 }), (v_636f756e74.read().call)(), false)};
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":73:11
            v_76616c69644974656d73.initialize({  let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_76616c756573: Value = argument_0;
                let v_73746172745f5f36343232323034393930313237636130: f64 = argument_1;
                let v_656e645f5f64636536656461623931356462646437: f64 = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":74:7
                if { let pui_native_92:f64=v_656e645f5f64636536656461623931356462646437.clone(); let pui_native_93:f64=v_73746172745f5f36343232323034393930313237636130.clone(); pui_native_92 <= pui_native_93 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":74:25
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":75:7
                if { let pui_native_94={ let pui_native_96:f64=v_656e645f5f64636536656461623931356462646437.clone(); let pui_native_97:f64=v_73746172745f5f36343232323034393930313237636130.clone(); pui_native_96 - pui_native_97 }; let pui_native_95=1.0_f64; (pui_native_94==pui_native_95) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":75:30
                    return { let pui_native_98={ let pui_native_100={ let pui_native_102={ let pui_native_104={ let pui_native_106={ let pui_native_108=(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(0.0_f64))))).bool(); if !(pui_native_108) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(1.0_f64))))).bool()} else {pui_native_108} }; if !(pui_native_106) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(2.0_f64))))).bool()} else {pui_native_106} }; if !(pui_native_104) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(3.0_f64))))).bool()} else {pui_native_104} }; if !(pui_native_102) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(4.0_f64))))).bool()} else {pui_native_102} }; if !(pui_native_100) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(5.0_f64))))).bool()} else {pui_native_100} }; if !(pui_native_98) {(Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(Value::number(v_73746172745f5f36343232323034393930313237636130.clone()))), false) }), &(Value::number(6.0_f64))))).bool()} else {pui_native_98} };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":77:13
                let v_73756d5f5f66613865633461633333666333663764: f64 = { let pui_native_124:f64=v_73746172745f5f36343232323034393930313237636130.clone(); let pui_native_125:f64=v_656e645f5f64636536656461623931356462646437.clone(); pui_native_124 + pui_native_125 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":78:13
                let v_6d6964646c655f5f62343638626333396465303765643130: f64 = { let pui_native_126:f64={ let pui_native_128:f64=v_73756d5f5f66613865633461633333666333663764.clone(); let pui_native_129:f64={ let pui_native_130:f64=v_73756d5f5f66613865633461633333666333663764.clone(); let pui_native_131:f64=2.0_f64; pui_native_130 % pui_native_131 }; pui_native_128 - pui_native_129 }; let pui_native_127:f64=2.0_f64; pui_native_126 / pui_native_127 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":79:7
                return { let pui_native_132=(v_76616c69644974656d73.read().call)(v_76616c756573.clone(), v_73746172745f5f36343232323034393930313237636130.clone(), v_6d6964646c655f5f62343638626333396465303765643130.clone()); if (pui_native_132) {(v_76616c69644974656d73.read().call)(v_76616c756573.clone(), v_6d6964646c655f5f62343638626333396465303765643130.clone(), v_656e645f5f64636536656461623931356462646437.clone())} else {pui_native_132} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":81:11
            v_656d6974.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let call:Rc<dyn Fn(Value, f64, bool, Value)->Value>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: bool, argument_3: Value| -> Value {
                let v_6e616d655f5f35336262333530306235323331356336: Value = argument_0;
                let v_696e6465785f5f31326130336230313666393137636639: f64 = argument_1;
                let v_657874656e645f5f38393636633337646532396662333765: bool = argument_2;
                let v_6b65795f5f34636165336263666265636437323461: Value = argument_3;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":82:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":82:20
                    { let v_7075695f6e61746976655f313334 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313334.clone()).owner(), v_6e616d655f5f35336262333530306235323331356336.clone(), record(vec![("index", Value::number(v_696e6465785f5f31326130336230313666393137636639.clone())), ("extend", Value::boolean(v_657874656e645f5f38393636633337646532396662333765.clone())), ("key", v_6b65795f5f34636165336263666265636437323461.clone()), ("revision", { let left = ({ let v_7075695f6e61746976655f313335 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313335.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), arg(&arguments,3))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":84:11
            v_7265717565737452657665616c.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); let call:Rc<dyn Fn(f64)->Value>=Rc::new(move |argument_0: f64| -> Value {
                let v_696e6465785f5f66386363393537336563326437386536: f64 = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":85:13
                let v_6f776e65725f5f31663336303039656461623133353137 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":86:7
                if { let pui_native_136=!((v_6f776e65725f5f31663336303039656461623133353137.clone()).truthy()); if !(pui_native_136) {!((v_7265616479.read().call)())} else {pui_native_136} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":86:31
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":87:13
                let v_6d6f64655f5f63363134663533656339333364653533 = ({ let v_7075695f6e61746976655f313338 = v_6f776e65725f5f31663336303039656461623133353137.clone(); read_props(&(v_7075695f6e61746976655f313338.clone()).owner(), false) }).member("mode", false);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":88:7
                if { let pui_native_139=(Value::boolean(strict_equal(&(v_6d6f64655f5f63363134663533656339333364653533.clone()), &(Value::string("filterlist"))))).bool(); if !(pui_native_139) {(Value::boolean(strict_equal(&(v_6d6f64655f5f63363134663533656339333364653533.clone()), &(Value::string("tracklist"))))).bool()} else {pui_native_139} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":88:58
                    (v_656d6974.read().call)(Value::string("command"), v_696e6465785f5f66386363393537336563326437386536.clone(), false, Value::string("reveal"));
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":90:11
            v_6368616e6765457870616e73696f6e.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); let call:Rc<dyn Fn(f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value| -> bool {
                let v_6e6578745f5f36383532316164336366343239326163: f64 = argument_0;
                let v_6e616d655f5f33323266343836396635356664363235: Value = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":91:7
                if !((v_7265616479.read().call)()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":91:21
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":92:13
                let v_7469636b65745f5f32326532643233646135653764623839: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":93:7
                if { let pui_native_145={ let pui_native_147:f64=v_6e6578745f5f36383532316164336366343239326163.clone(); let pui_native_148:f64=0.0_f64; pui_native_147 < pui_native_148 }; if (pui_native_145) {{ let pui_native_149:f64=({ let v_7075695f6e61746976655f313531 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f313531.clone()).get()).num()) }).num(); let pui_native_150:f64=0.0_f64; pui_native_149 >= pui_native_150 }} else {pui_native_145} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":94:9
                    { let v_7075695f6e61746976655f313532 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313532.clone(), Value::number(({ let v_7075695f6e61746976655f313533 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f313533.clone()).get()).num()) }).num()), Value::string("reason: closing submenu restores parent")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":95:9
                    if { let pui_native_154={ let pui_native_156=v_7469636b65745f5f32326532643233646135653764623839.clone(); let pui_native_157={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_156==pui_native_157) }; if !(pui_native_154) {!((v_7265616479.read().call)())} else {pui_native_154} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":95:43
                        return false;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":97:7
                { let v_7075695f6e61746976655f313538 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f313538.clone(), Value::number(v_6e6578745f5f36383532316164336366343239326163.clone()), Value::string("reason: submenu expansion")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":98:7
                if { let pui_native_159={ let pui_native_161=v_7469636b65745f5f32326532643233646135653764623839.clone(); let pui_native_162={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_161==pui_native_162) }; if !(pui_native_159) {!((v_7265616479.read().call)())} else {pui_native_159} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":98:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":99:7
                (v_656d6974.read().call)(Value::string("expand"), v_6e6578745f5f36383532316164336366343239326163.clone(), false, v_6e616d655f5f33323266343836396635356664363235.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":100:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let pui_native_163=v_7469636b65745f5f32326532643233646135653764623839.clone(); let pui_native_164={ let value=(v_65706f6368.read()).borrow().clone(); value }; (pui_native_163==pui_native_164) }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":100:40
                    { let v_7075695f6e61746976655f313635 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f313635.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":101:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":103:11
            v_73656c656374.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); let call:Rc<dyn Fn(f64, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool| -> bool {
                let v_696e6465785f5f38633865653562626535643637343361: f64 = argument_0;
                let v_657874656e645f5f61613031393634316633326332393539: bool = argument_1;
                let v_6578706c69636974: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":104:7
                if { let pui_native_166=!((v_7265616479.read().call)()); if !(pui_native_166) {{ let pui_native_168=(v_6b696e64.read().call)(v_696e6465785f5f38633865653562626535643637343361.clone()); let pui_native_169=0.0_f64; (pui_native_168==pui_native_169) }} else {pui_native_166} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":104:42
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":107:7
                if { let pui_native_170={ let pui_native_172=!(v_6578706c69636974.clone()); if (pui_native_172) {{ let pui_native_174=v_696e6465785f5f38633865653562626535643637343361.clone(); let pui_native_175=({ let v_7075695f6e61746976655f313736 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f313736.clone()).get()).num()) }).num(); (pui_native_174==pui_native_175) }} else {pui_native_172} }; if (pui_native_170) {!(v_657874656e645f5f61613031393634316633326332393539.clone())} else {pui_native_170} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":107:59
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":108:13
                let v_7469636b65745f5f38306132353465333530633565643963: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":109:7
                { let v_7075695f6e61746976655f313737 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f313737.clone(), Value::number(v_696e6465785f5f38633865653562626535643637343361.clone()), Value::string("reason: collection selection")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":110:7
                if { let pui_native_178={ let pui_native_180=v_7469636b65745f5f38306132353465333530633565643963.clone(); let pui_native_181={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_180==pui_native_181) }; if !(pui_native_178) {!((v_7265616479.read().call)())} else {pui_native_178} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":110:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":111:7
                (v_656d6974.read().call)(Value::string("select"), v_696e6465785f5f38633865653562626535643637343361.clone(), v_657874656e645f5f61613031393634316633326332393539.clone(), Value::string(""));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":112:7
                if { let pui_native_182={ let pui_native_184=v_7469636b65745f5f38306132353465333530633565643963.clone(); let pui_native_185={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_184==pui_native_185) }; if !(pui_native_182) {!((v_7265616479.read().call)())} else {pui_native_182} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":112:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":113:7
                (v_7265717565737452657665616c.read().call)(v_696e6465785f5f38633865653562626535643637343361.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":114:7
                if { let pui_native_186={ let pui_native_188=v_7469636b65745f5f38306132353465333530633565643963.clone(); let pui_native_189={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_188==pui_native_189) }; if !(pui_native_186) {!((v_7265616479.read().call)())} else {pui_native_186} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":114:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":115:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let pui_native_190=v_7469636b65745f5f38306132353465333530633565643963.clone(); let pui_native_191={ let value=(v_65706f6368.read()).borrow().clone(); value }; (pui_native_190==pui_native_191) }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":115:40
                    { let v_7075695f6e61746976655f313932 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f313932.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":116:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":118:11
            v_63686f6f736553656c656374696f6e.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let call:Rc<dyn Fn(f64, bool, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: bool| -> bool {
                let v_696e6465785f5f31343739633462363464653965383661: f64 = argument_0;
                let v_73656c65637465645f5f31333562346438646631343839373036: bool = argument_1;
                let v_657874656e645f5f37356664363364653334383062316439: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":119:13
                let v_6f776e65725f5f65613837353135313139303235333533 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":120:13
                let v_6974656d: f64 = (v_6b696e64.read().call)(v_696e6465785f5f31343739633462363464653965383661.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":121:7
                if { let pui_native_193={ let pui_native_195={ let pui_native_197={ let pui_native_199={ let pui_native_201=!((v_6f776e65725f5f65613837353135313139303235333533.clone()).truthy()); if !(pui_native_201) {!((v_7265616479.read().call)())} else {pui_native_201} }; if !(pui_native_199) {(Value::boolean(!strict_equal(&(typeof_value(&(Value::boolean(v_73656c65637465645f5f31333562346438646631343839373036.clone())))), &(Value::string("boolean"))))).bool()} else {pui_native_199} }; if !(pui_native_197) {{ let pui_native_205:f64=v_6974656d.clone(); let pui_native_206:f64=3.0_f64; pui_native_205 < pui_native_206 }} else {pui_native_197} }; if !(pui_native_195) {{ let pui_native_207:f64=v_6974656d.clone(); let pui_native_208:f64=6.0_f64; pui_native_207 > pui_native_208 }} else {pui_native_195} }; if !(pui_native_193) {{ let pui_native_209={ let pui_native_211:f64=v_6974656d.clone(); let pui_native_212:f64=5.0_f64; pui_native_211 >= pui_native_212 }; if (pui_native_209) {!(v_73656c65637465645f5f31333562346438646631343839373036.clone())} else {pui_native_209} }} else {pui_native_193} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":122:38
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":123:13
                let v_7469636b65745f5f36356130626238663864643166633465: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":124:7
                { let v_7075695f6e61746976655f323133 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f323133.clone(), Value::number(v_696e6465785f5f31343739633462363464653965383661.clone()), Value::string("reason: collection selection proposal cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":125:7
                if { let pui_native_214=!((v_7265616479.read().call)()); if !(pui_native_214) {{ let pui_native_216={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_217=v_7469636b65745f5f36356130626238663864643166633465.clone(); !(pui_native_216==pui_native_217) }} else {pui_native_214} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":125:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":126:7
                { let v_7075695f6e61746976655f323138 = v_6f776e65725f5f65613837353135313139303235333533.clone(); emit(&(v_7075695f6e61746976655f323138.clone()).owner(), Value::string("selection"), record(vec![("index", Value::number(v_696e6465785f5f31343739633462363464653965383661.clone())), ("selected", Value::boolean(v_73656c65637465645f5f31333562346438646631343839373036.clone())), ("extend", Value::boolean(v_657874656e645f5f37356664363364653334383062316439.clone())), ("revision", { let left = ({ let v_7075695f6e61746976655f323139 = v_6f776e65725f5f65613837353135313139303235333533.clone(); read_props(&(v_7075695f6e61746976655f323139.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":127:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":129:11
            v_63686f6f7365.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let v_63686f6f736553656c656374696f6e = v_63686f6f736553656c656374696f6e.clone(); let call:Rc<dyn Fn(f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: bool| -> bool {
                let v_696e6465785f5f39626438336462336535376636363435: f64 = argument_0;
                let v_657874656e645f5f33363439326338383833356263323765: bool = argument_1;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":130:7
                if { let pui_native_220=!((v_7265616479.read().call)()); if !(pui_native_220) {{ let pui_native_222=(v_6b696e64.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone()); let pui_native_223=0.0_f64; (pui_native_222==pui_native_223) }} else {pui_native_220} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":130:42
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":131:7
                if { let pui_native_224:f64=(v_6b696e64.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone()); let pui_native_225:f64=3.0_f64; pui_native_224 >= pui_native_225 } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":131:29
                    return (v_63686f6f736553656c656374696f6e.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone(), { let pui_native_226={ let pui_native_228=(v_6b696e64.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone()); let pui_native_229=3.0_f64; (pui_native_228==pui_native_229) }; if !(pui_native_226) {{ let pui_native_230:f64=(v_6b696e64.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone()); let pui_native_231:f64=5.0_f64; pui_native_230 >= pui_native_231 }} else {pui_native_226} }, v_657874656e645f5f33363439326338383833356263323765.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":132:13
                let v_7469636b65745f5f35616631333862653238373638616633: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":133:7
                { let v_7075695f6e61746976655f323332 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f323332.clone(), Value::number(v_696e6465785f5f39626438336462336535376636363435.clone()), Value::string("reason: collection activation cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":134:7
                if { let pui_native_233=!((v_7265616479.read().call)()); if !(pui_native_233) {{ let pui_native_235={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_236=v_7469636b65745f5f35616631333862653238373638616633.clone(); !(pui_native_235==pui_native_236) }} else {pui_native_233} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":134:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":135:7
                if { let pui_native_237=(v_6b696e64.read().call)(v_696e6465785f5f39626438336462336535376636363435.clone()); let pui_native_238=2.0_f64; (pui_native_237==pui_native_238) } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":136:15
                    let v_6e6578745f5f64613663373939346534653436316533: f64 = if { let pui_native_239=({ let v_7075695f6e61746976655f323431 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f323431.clone()).get()).num()) }).num(); let pui_native_240=v_696e6465785f5f39626438336462336535376636363435.clone(); (pui_native_239==pui_native_240) } {-(1.0_f64)} else {v_696e6465785f5f39626438336462336535376636363435.clone()};
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":137:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(v_6e6578745f5f64613663373939346534653436316533.clone(), Value::string(""));
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":138:14
                    (v_656d6974.read().call)(Value::string("activate"), v_696e6465785f5f39626438336462336535376636363435.clone(), v_657874656e645f5f33363439326338383833356263323765.clone(), Value::string(""));
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":139:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":141:11
            v_636f6e746578744174.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let call:Rc<dyn Fn(f64, Value, f64, f64, bool)->bool>=Rc::new(move |argument_0: f64, argument_1: Value, argument_2: f64, argument_3: f64, argument_4: bool| -> bool {
                let v_696e6465785f5f33333339333663376664333136656566: f64 = argument_0;
                let v_62617369735f5f30333362633738336136636135633865: Value = argument_1;
                let v_78: f64 = argument_2;
                let v_79: f64 = argument_3;
                let v_6b6579626f617264: bool = argument_4;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":142:13
                let v_6f776e65725f5f61653333326132633836326564373361 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":143:7
                if { let pui_native_242={ let pui_native_244={ let pui_native_246={ let pui_native_248={ let pui_native_250={ let pui_native_252={ let pui_native_254=!((v_6f776e65725f5f61653333326132633836326564373361.clone()).truthy()); if !(pui_native_254) {!((v_7265616479.read().call)())} else {pui_native_254} }; if !(pui_native_252) {(Value::boolean(!strict_equal(&(v_62617369735f5f30333362633738336136636135633865.clone()), &({ let left = ({ let v_7075695f6e61746976655f323538 = v_6f776e65725f5f61653333326132633836326564373361.clone(); read_props(&(v_7075695f6e61746976655f323538.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_252} }; if !(pui_native_250) {{ let pui_native_259=(v_6b696e64.read().call)(v_696e6465785f5f33333339333663376664333136656566.clone()); let pui_native_260=0.0_f64; (pui_native_259==pui_native_260) }} else {pui_native_250} }; if !(pui_native_248) {!((v_696e6465785f5f33333339333663376664333136656566.clone()).is_finite())} else {pui_native_248} }; if !(pui_native_246) {{ let pui_native_261={ let pui_native_263:f64=v_696e6465785f5f33333339333663376664333136656566.clone(); let pui_native_264:f64=1.0_f64; pui_native_263 % pui_native_264 }; let pui_native_262=0.0_f64; !(pui_native_261==pui_native_262) }} else {pui_native_246} }; if !(pui_native_244) {!((v_78.clone()).is_finite())} else {pui_native_244} }; if !(pui_native_242) {!((v_79.clone()).is_finite())} else {pui_native_242} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":144:102
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":145:13
                let v_7469636b65745f5f62663366613139396230613862636432: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":146:7
                { let v_7075695f6e61746976655f323635 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f323635.clone(), Value::number(v_696e6465785f5f33333339333663376664333136656566.clone()), Value::string("reason: collection context cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":147:7
                if { let pui_native_266={ let pui_native_268=v_7469636b65745f5f62663366613139396230613862636432.clone(); let pui_native_269={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_268==pui_native_269) }; if !(pui_native_266) {!((v_7265616479.read().call)())} else {pui_native_266} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":147:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":148:7
                { let v_7075695f6e61746976655f323730 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323730.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":149:7
                if { let pui_native_271={ let pui_native_273=v_7469636b65745f5f62663366613139396230613862636432.clone(); let pui_native_274={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_273==pui_native_274) }; if !(pui_native_271) {!((v_7265616479.read().call)())} else {pui_native_271} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":149:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":150:7
                { let v_7075695f6e61746976655f323735 = v_6f776e65725f5f61653333326132633836326564373361.clone(); emit(&(v_7075695f6e61746976655f323735.clone()).owner(), Value::string("context"), record(vec![("index", Value::number(v_696e6465785f5f33333339333663376664333136656566.clone())), ("x", Value::number(v_78.clone())), ("y", Value::number(v_79.clone())), ("keyboard", Value::boolean(v_6b6579626f617264.clone())), ("revision", v_62617369735f5f30333362633738336136636135633865.clone())])) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":151:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":153:11
            v_6b65795f5f33313237306232626462353633303563.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let v_73656c656374 = v_73656c656374.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let v_636f6e746578744174 = v_636f6e746578744174.clone(); let call:Rc<dyn Fn(Value, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool| -> bool {
                let v_6e616d655f5f31653531353938353539333938623565: Value = argument_0;
                let v_73686966745f5f35633837616463336165313061656534: bool = argument_1;
                let v_636f6d6d616e645f5f34326465393634336530653437646465: bool = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":154:7
                if { let pui_native_276=!((v_7265616479.read().call)()); if !(pui_native_276) {!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy())} else {pui_native_276} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":154:33
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":155:13
                let v_6d6f64655f5f66313262663165643863663539623463 = { let left = ({ let v_7075695f6e61746976655f323738 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f323738.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":156:7
                if { let pui_native_279={ let pui_native_281=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("filterlist"))))).bool(); if (pui_native_281) {v_636f6d6d616e645f5f34326465393634336530653437646465.clone()} else {pui_native_281} }; if (pui_native_279) {{ let pui_native_285=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("f"))))).bool(); if !(pui_native_285) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("F"))))).bool()} else {pui_native_285} }} else {pui_native_279} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":156:81
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f323931 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f323931.clone()).get()).num()) }).num(), false, Value::string("focus-filter"));
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":156:135
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":157:7
                if { let pui_native_292=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("tracklist"))))).bool(); if (pui_native_292) {{ let pui_native_296=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ContextMenu"))))).bool(); if !(pui_native_296) {{ let pui_native_300=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("F10"))))).bool(); if (pui_native_300) {v_73686966745f5f35633837616463336165313061656534.clone()} else {pui_native_300} }} else {pui_native_296} }} else {pui_native_292} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":157:90
                    return (v_636f6e746578744174.read().call)(({ let v_7075695f6e61746976655f333034 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333034.clone()).get()).num()) }).num(), { let left = ({ let v_7075695f6e61746976655f333035 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f333035.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }, 0.0_f64, 0.0_f64, true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":158:7
                if { let pui_native_306=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("points"))))).bool(); if (pui_native_306) {{ let pui_native_310=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_310) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_310} }} else {pui_native_306} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":158:75
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f333136 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333136.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":158:117
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":159:7
                if { let pui_native_317=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("points"))))).bool(); if (pui_native_317) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Tab"))))).bool()} else {pui_native_317} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":160:15
                    let v_5f5f7075694d6f64756c6530: f64 = { let pui_native_323:f64=({ let v_7075695f6e61746976655f333235 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333235.clone()).get()).num()) }).num(); let pui_native_324:f64=if v_73686966745f5f35633837616463336165313061656534.clone() {-(1.0_f64)} else {1.0_f64}; pui_native_323 + pui_native_324 };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":161:9
                    if { let pui_native_326={ let pui_native_328:f64=v_5f5f7075694d6f64756c6530.clone(); let pui_native_329:f64=0.0_f64; pui_native_328 < pui_native_329 }; if !(pui_native_326) {{ let pui_native_330:f64=v_5f5f7075694d6f64756c6530.clone(); let pui_native_331:f64=(v_636f756e74.read().call)(); pui_native_330 >= pui_native_331 }} else {pui_native_326} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":161:42
                        (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f333332 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333332.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":162:14
                        Value::boolean((v_73656c656374.read().call)(v_5f5f7075694d6f64756c6530.clone(), false, true));
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":163:9
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":165:7
                if { let pui_native_333=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("points"))))).bool(); if (pui_native_333) {{ let pui_native_337={ let pui_native_339={ let pui_native_341={ let pui_native_343=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_343) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_343} }; if !(pui_native_341) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("+"))))).bool()} else {pui_native_341} }; if !(pui_native_339) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("="))))).bool()} else {pui_native_339} }; if !(pui_native_337) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("-"))))).bool()} else {pui_native_337} }} else {pui_native_333} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":165:128
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f333535 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333535.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":165:172
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":166:7
                if { let pui_native_356=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_356) {{ let pui_native_360=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Tab"))))).bool(); if (pui_native_360) {(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("menu"))))).bool()} else {pui_native_360} }} else {pui_native_356} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":166:71
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f333636 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333636.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":166:113
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":167:7
                if { let pui_native_367={ let pui_native_369=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Enter"))))).bool(); if !(pui_native_369) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string(" "))))).bool()} else {pui_native_369} }; if !(pui_native_367) {{ let pui_native_375=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("F2"))))).bool(); if (pui_native_375) {(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("table"))))).bool()} else {pui_native_375} }} else {pui_native_367} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":167:84
                    return (v_63686f6f7365.read().call)(({ let v_7075695f6e61746976655f333831 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333831.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":168:7
                if { let pui_native_382=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("table"))))).bool(); if (pui_native_382) {{ let pui_native_386={ let pui_native_388=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Delete"))))).bool(); if !(pui_native_388) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_388} }; if !(pui_native_386) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Tab"))))).bool()} else {pui_native_386} }} else {pui_native_382} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":169:9
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f333936 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f333936.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":169:53
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":171:7
                if { let pui_native_397={ let pui_native_399=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("table"))))).bool(); if (pui_native_399) {v_636f6d6d616e645f5f34326465393634336530653437646465.clone()} else {pui_native_399} }; if (pui_native_397) {{ let pui_native_403={ let pui_native_405={ let pui_native_407=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowLeft"))))).bool(); if !(pui_native_407) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_407} }; if !(pui_native_405) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_405} }; if !(pui_native_403) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_403} }} else {pui_native_397} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":172:9
                    (v_656d6974.read().call)(Value::string("command"), ({ let v_7075695f6e61746976655f343137 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343137.clone()).get()).num()) }).num(), v_73686966745f5f35633837616463336165313061656534.clone(), v_6e616d655f5f31653531353938353539333938623565.clone());
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":172:53
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":174:7
                if { let pui_native_418=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("menu"))))).bool(); if (pui_native_418) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_418} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":175:9
                    if { let pui_native_424:f64=({ let v_7075695f6e61746976655f343236 = v_657870616e646564.read(); Value::number(((v_7075695f6e61746976655f343236.clone()).get()).num()) }).num(); let pui_native_425:f64=0.0_f64; pui_native_424 < pui_native_425 } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":175:33
                        return false;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":176:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(-(1.0_f64), v_6e616d655f5f31653531353938353539333938623565.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":178:7
                if { let pui_native_427=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("menu"))))).bool(); if (pui_native_427) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_427} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":179:9
                    if { let pui_native_433=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f343335 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343335.clone()).get()).num()) }).num()); let pui_native_434=2.0_f64; !(pui_native_433==pui_native_434) } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":179:39
                        return false;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":180:9
                    return (v_6368616e6765457870616e73696f6e.read().call)(({ let v_7075695f6e61746976655f343336 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343336.clone()).get()).num()) }).num(), v_6e616d655f5f31653531353938353539333938623565.clone());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":182:11
                let v_6e6578745f5f64336238353432623434393663643234: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(-(1.0_f64)));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":183:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Home"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":183:28
                    Value::number({ let value=(v_7365656b.read().call)(0.0_f64, 1.0_f64, (v_636f756e74.read().call)(), false); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":184:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("End"))))).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":184:32
                        Value::number({ let value=(v_7365656b.read().call)({ let pui_native_441:f64=(v_636f756e74.read().call)(); let pui_native_442:f64=1.0_f64; pui_native_441 - pui_native_442 }, -(1.0_f64), (v_636f756e74.read().call)(), false); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":185:12
                        if { let pui_native_443=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_443) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_443} } {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":185:60
                            Value::number({ let value=(v_7365656b.read().call)({ let pui_native_449:f64=({ let v_7075695f6e61746976655f343531 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343531.clone()).get()).num()) }).num(); let pui_native_450:f64=1.0_f64; pui_native_449 - pui_native_450 }, -(1.0_f64), (v_636f756e74.read().call)(), { let pui_native_452=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("menu"))))).bool(); if !(pui_native_452) {(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("tablist"))))).bool()} else {pui_native_452} }); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":186:12
                            if { let pui_native_458=(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowDown"))))).bool(); if !(pui_native_458) {(Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_458} } {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":186:63
                                Value::number({ let value=(v_7365656b.read().call)({ let pui_native_464:f64=({ let v_7075695f6e61746976655f343636 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343636.clone()).get()).num()) }).num(); let pui_native_465:f64=1.0_f64; pui_native_464 + pui_native_465 }, 1.0_f64, (v_636f756e74.read().call)(), { let pui_native_467=(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("menu"))))).bool(); if !(pui_native_467) {(Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("tablist"))))).bool()} else {pui_native_467} }); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":187:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("PageUp"))))).bool() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":187:35
                                    Value::number({ let value=(v_7365656b.read().call)((v_6d6178.read().call)(0.0_f64, { let pui_native_475:f64=({ let v_7075695f6e61746976655f343737 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343737.clone()).get()).num()) }).num(); let pui_native_476:f64=if (Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("points"))))).bool() {4.0_f64} else {20.0_f64}; pui_native_475 - pui_native_476 }), -(1.0_f64), (v_636f756e74.read().call)(), false); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":188:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("PageDown"))))).bool() {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":188:37
                                        Value::number({ let value=(v_7365656b.read().call)((v_6d696e.read().call)({ let pui_native_482:f64=(v_636f756e74.read().call)(); let pui_native_483:f64=1.0_f64; pui_native_482 - pui_native_483 }, { let pui_native_484:f64=({ let v_7075695f6e61746976655f343836 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f343836.clone()).get()).num()) }).num(); let pui_native_485:f64=if (Value::boolean(strict_equal(&(v_6d6f64655f5f66313262663165643863663539623463.clone()), &(Value::string("points"))))).bool() {4.0_f64} else {20.0_f64}; pui_native_484 + pui_native_485 }), 1.0_f64, (v_636f756e74.read().call)(), false); *(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow_mut()=value.clone(); value });
                                    } else {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":189:12
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":190:7
                if { let pui_native_489={ let pui_native_491:f64={ let value=(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow().clone(); value }; let pui_native_492:f64=0.0_f64; pui_native_491 >= pui_native_492 }; if (pui_native_489) {!((v_73656c656374.read().call)({ let value=(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow().clone(); value }, v_73686966745f5f35633837616463336165313061656534.clone(), true))} else {pui_native_489} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":190:52
                    return true;
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":191:12
                    if { let pui_native_493={ let pui_native_495={ let pui_native_497={ let pui_native_499:f64={ let value=(v_6e6578745f5f64336238353432623434393663643234.clone()).borrow().clone(); value }; let pui_native_500:f64=0.0_f64; pui_native_499 < pui_native_500 }; if (pui_native_497) {!(v_73686966745f5f35633837616463336165313061656534.clone())} else {pui_native_497} }; if (pui_native_495) {{ let pui_native_501:f64=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f353033 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f353033.clone()).get()).num()) }).num()); let pui_native_502:f64=0.0_f64; pui_native_501 > pui_native_502 }} else {pui_native_495} }; if (pui_native_493) {!((v_73656c656374.read().call)(({ let v_7075695f6e61746976655f353034 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f353034.clone()).get()).num()) }).num(), false, true))} else {pui_native_493} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":191:100
                        return true;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":192:7
                if !((v_7265616479.read().call)()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":192:21
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":193:13
                let v_686f72697a6f6e74616c: bool = (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f353037 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f353037.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f353037.clone()).owner(), false) } }).member("orientation", true)), &(Value::string("horizontal"))))).bool();
                // Source "frontend/proto-ui/src/collection-control.proto.ts":194:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("PageUp"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":194:32
                    if v_686f72697a6f6e74616c.clone() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":194:48
                        { let v_7075695f6e61746976655f353130 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353130.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":194:128
                        { let v_7075695f6e61746976655f353131 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353131.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":195:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("PageDown"))))).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":195:39
                        if v_686f72697a6f6e74616c.clone() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":195:55
                            { let v_7075695f6e61746976655f353134 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353134.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":195:134
                            { let v_7075695f6e61746976655f353135 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353135.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":196:12
                        if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("Home"))))).bool() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":196:35
                            if v_686f72697a6f6e74616c.clone() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":196:51
                                { let v_7075695f6e61746976655f353138 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353138.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0_f64))])) };
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":196:121
                                { let v_7075695f6e61746976655f353139 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353139.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0_f64))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":197:12
                            if (Value::boolean(strict_equal(&(v_6e616d655f5f31653531353938353539333938623565.clone()), &(Value::string("End"))))).bool() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":197:34
                                if v_686f72697a6f6e74616c.clone() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":197:50
                                    { let v_7075695f6e61746976655f353232 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353232.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":197:111
                                    { let v_7075695f6e61746976655f353233 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f353233.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":198:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":203:5
            { let v_7075695f6e61746976655f353234 = v_646566.clone(); expose_method(&owner, Value::string("filterKey"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b65795f5f33313237306232626462353633303563 = v_6b65795f5f33313237306232626462353633303563.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool, bool, Value, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: Value, argument_6: bool| -> bool {
                let v_6e616d655f5f35313562393863663438366465383033: Value = argument_0;
                let v_73686966745f5f39636336623530663866663564313331: bool = argument_1;
                let v_636f6d6d616e645f5f39383435636662666562333333323461: bool = argument_2;
                let v_636f6d706f73696e67: bool = argument_3;
                let v_66726f6d46696c746572: bool = argument_4;
                let v_62617369735f5f38633034643537623763396630633563: Value = argument_5;
                let v_616c74: bool = argument_6;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":204:13
                let v_6f776e65725f5f38363135333531313564633763333935 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:7
                if { let pui_native_525={ let pui_native_527={ let pui_native_529={ let pui_native_531={ let pui_native_533=!((v_6f776e65725f5f38363135333531313564633763333935.clone()).truthy()); if !(pui_native_533) {!((v_7265616479.read().call)())} else {pui_native_533} }; if !(pui_native_531) {v_636f6d706f73696e67.clone()} else {pui_native_531} }; if !(pui_native_529) {v_616c74.clone()} else {pui_native_529} }; if !(pui_native_527) {(Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f353337 = v_6f776e65725f5f38363135333531313564633763333935.clone(); read_props(&(v_7075695f6e61746976655f353337.clone()).owner(), false) }).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_527} }; if !(pui_native_525) {(Value::boolean(!strict_equal(&(v_62617369735f5f38633034643537623763396630633563.clone()), &({ let left = ({ let v_7075695f6e61746976655f353430 = v_6f776e65725f5f38363135333531313564633763333935.clone(); read_props(&(v_7075695f6e61746976655f353430.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_525} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":205:142
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":206:7
                if { let pui_native_541=v_636f6d6d616e645f5f39383435636662666562333333323461.clone(); if (pui_native_541) {{ let pui_native_543=(Value::boolean(strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("f"))))).bool(); if !(pui_native_543) {(Value::boolean(strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("F"))))).bool()} else {pui_native_543} }} else {pui_native_541} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":206:54
                    return (v_6b65795f5f33313237306232626462353633303563.read().call)(v_6e616d655f5f35313562393863663438366465383033.clone(), v_73686966745f5f39636336623530663866663564313331.clone(), true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":207:7
                if { let pui_native_549=!(v_66726f6d46696c746572.clone()); if !(pui_native_549) {v_636f6d6d616e645f5f39383435636662666562333333323461.clone()} else {pui_native_549} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":207:35
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":208:7
                if { let pui_native_551={ let pui_native_553={ let pui_native_555=(Value::boolean(!strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("ArrowUp"))))).bool(); if (pui_native_555) {(Value::boolean(!strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_555} }; if (pui_native_553) {(Value::boolean(!strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_553} }; if (pui_native_551) {(Value::boolean(!strict_equal(&(v_6e616d655f5f35313562393863663438366465383033.clone()), &(Value::string("Escape"))))).bool()} else {pui_native_551} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":208:96
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":209:7
                return (v_6b65795f5f33313237306232626462353633303563.read().call)(v_6e616d655f5f35313562393863663438366465383033.clone(), v_73686966745f5f39636336623530663866663564313331.clone(), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), arg(&arguments,5), (arg(&arguments,6)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":214:5
            { let v_7075695f6e61746976655f353635 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), ({  let v_637572736f72 = v_637572736f72.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: Value| -> bool {
                let v_696e6465785f5f36383839346430386638393134633839: f64 = argument_0;
                let v_73746172745f5f64323338316466323134393535373330: f64 = argument_1;
                let v_656e645f5f65656532326337653131343830303532: f64 = argument_2;
                let v_76696577706f72745374617274: f64 = argument_3;
                let v_76696577706f7274456e64: f64 = argument_4;
                let v_62617369735f5f30663665323131313637666239356533: Value = argument_5;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":215:13
                let v_6f776e65725f5f32333364623864326432333337633637 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":216:7
                if { let pui_native_566={ let pui_native_568={ let pui_native_570={ let pui_native_572={ let pui_native_574={ let pui_native_576={ let pui_native_578={ let pui_native_580={ let pui_native_582={ let pui_native_584=!((v_6f776e65725f5f32333364623864326432333337633637.clone()).truthy()); if !(pui_native_584) {!((v_7265616479.read().call)())} else {pui_native_584} }; if !(pui_native_582) {(Value::boolean(!strict_equal(&(v_62617369735f5f30663665323131313637666239356533.clone()), &({ let left = ({ let v_7075695f6e61746976655f353838 = v_6f776e65725f5f32333364623864326432333337633637.clone(); read_props(&(v_7075695f6e61746976655f353838.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_582} }; if !(pui_native_580) {{ let pui_native_589=v_696e6465785f5f36383839346430386638393134633839.clone(); let pui_native_590=({ let v_7075695f6e61746976655f353931 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f353931.clone()).get()).num()) }).num(); !(pui_native_589==pui_native_590) }} else {pui_native_580} }; if !(pui_native_578) {{ let pui_native_592=(v_6b696e64.read().call)(v_696e6465785f5f36383839346430386638393134633839.clone()); let pui_native_593=0.0_f64; (pui_native_592==pui_native_593) }} else {pui_native_578} }; if !(pui_native_576) {!((v_73746172745f5f64323338316466323134393535373330.clone()).is_finite())} else {pui_native_576} }; if !(pui_native_574) {!((v_656e645f5f65656532326337653131343830303532.clone()).is_finite())} else {pui_native_574} }; if !(pui_native_572) {!((v_76696577706f72745374617274.clone()).is_finite())} else {pui_native_572} }; if !(pui_native_570) {!((v_76696577706f7274456e64.clone()).is_finite())} else {pui_native_570} }; if !(pui_native_568) {{ let pui_native_594:f64=v_656e645f5f65656532326337653131343830303532.clone(); let pui_native_595:f64=v_73746172745f5f64323338316466323134393535373330.clone(); pui_native_594 <= pui_native_595 }} else {pui_native_568} }; if !(pui_native_566) {{ let pui_native_596:f64=v_76696577706f7274456e64.clone(); let pui_native_597:f64=v_76696577706f72745374617274.clone(); pui_native_596 <= pui_native_597 }} else {pui_native_566} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":218:58
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":219:7
                if { let pui_native_598={ let pui_native_600={ let pui_native_602:f64=v_73746172745f5f64323338316466323134393535373330.clone(); let pui_native_603:f64=v_76696577706f72745374617274.clone(); pui_native_602 >= pui_native_603 }; if (pui_native_600) {{ let pui_native_604:f64=v_656e645f5f65656532326337653131343830303532.clone(); let pui_native_605:f64=v_76696577706f7274456e64.clone(); pui_native_604 <= pui_native_605 }} else {pui_native_600} }; if !(pui_native_598) {{ let pui_native_606={ let pui_native_608:f64=v_73746172745f5f64323338316466323134393535373330.clone(); let pui_native_609:f64=v_76696577706f72745374617274.clone(); pui_native_608 <= pui_native_609 }; if (pui_native_606) {{ let pui_native_610:f64=v_656e645f5f65656532326337653131343830303532.clone(); let pui_native_611:f64=v_76696577706f7274456e64.clone(); pui_native_610 >= pui_native_611 }} else {pui_native_606} }} else {pui_native_598} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":219:109
                    return true;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:13
                let v_6c656164696e67: f64 = { let pui_native_612:f64=v_73746172745f5f64323338316466323134393535373330.clone(); let pui_native_613:f64=v_76696577706f72745374617274.clone(); pui_native_612 - pui_native_613 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:52
                let v_747261696c696e67: f64 = { let pui_native_614:f64=v_656e645f5f65656532326337653131343830303532.clone(); let pui_native_615:f64=v_76696577706f7274456e64.clone(); pui_native_614 - pui_native_615 };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":221:13
                let v_6c656164696e6744697374616e6365: f64 = if { let pui_native_616:f64=v_6c656164696e67.clone(); let pui_native_617:f64=0.0_f64; pui_native_616 < pui_native_617 } {-(v_6c656164696e67.clone())} else {v_6c656164696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":221:71
                let v_747261696c696e6744697374616e6365: f64 = if { let pui_native_618:f64=v_747261696c696e67.clone(); let pui_native_619:f64=0.0_f64; pui_native_618 < pui_native_619 } {-(v_747261696c696e67.clone())} else {v_747261696c696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":222:13
                let v_64656c7461: f64 = if { let pui_native_620:f64=v_6c656164696e6744697374616e6365.clone(); let pui_native_621:f64=v_747261696c696e6744697374616e6365.clone(); pui_native_620 < pui_native_621 } {v_6c656164696e67.clone()} else {v_747261696c696e67.clone()};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":223:7
                if (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f363234 = v_6f776e65725f5f32333364623864326432333337633637.clone(); read_props(&(v_7075695f6e61746976655f363234.clone()).owner(), false) }).member("orientation", false)), &(Value::string("horizontal"))))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":223:59
                    { let v_7075695f6e61746976655f363235 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f363235.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(v_64656c7461.clone()))])) };
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":224:12
                    { let v_7075695f6e61746976655f363236 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f363236.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(v_64656c7461.clone()))])) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":225:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).num(), arg(&arguments,5)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":227:5
            { let v_7075695f6e61746976655f363237 = v_646566.clone(); expose_method(&owner, Value::string("setItems"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6974656d73 = v_6974656d73.clone(); let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let v_73796e63 = v_73796e63.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6e6578745f5f61623766313036376639343963313730: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":228:7
                if { let pui_native_628={ let pui_native_630={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_630) {{ let pui_native_632:f64=((v_6e6578745f5f61623766313036376639343963313730.clone()).member("length", false)).num(); let pui_native_633:f64=1000000.0_f64; pui_native_632 > pui_native_633 }} else {pui_native_630} }; if !(pui_native_628) {!((v_76616c69644974656d73.read().call)(v_6e6578745f5f61623766313036376639343963313730.clone(), 0.0_f64, ((v_6e6578745f5f61623766313036376639343963313730.clone()).member("length", false)).num()))} else {pui_native_628} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":228:83
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":229:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":229:16
                { let value = v_6e6578745f5f61623766313036376639343963313730.clone(); *(v_6974656d73.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":230:20
                    v_73796e63.read().call_authored(vec![{ let value = (v_63757272656e74.read()).borrow().clone(); value }]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":231:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":233:5
            { let v_7075695f6e61746976655f363334 = v_646566.clone(); expose_method(&owner, Value::string("choose"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f63373335353762396230376133326239: f64 = argument_0;
                let v_657874656e645f5f31343835386662343037383364393130: bool = argument_1;
                let v_62617369735f5f35336436323234623133653633333930: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":234:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f35336436323234623133653633333930.clone()), &({ let left = ({ let v_7075695f6e61746976655f363337 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f363337.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f363337.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":234:60
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":235:7
                return (v_63686f6f7365.read().call)(v_696e6465785f5f63373335353762396230376133326239.clone(), v_657874656e645f5f31343835386662343037383364393130.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":239:5
            { let v_7075695f6e61746976655f363338 = v_646566.clone(); expose_method(&owner, Value::string("chooseSelection"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f736553656c656374696f6e = v_63686f6f736553656c656374696f6e.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f38383736393263333739366462303863: f64 = argument_0;
                let v_73656c65637465645f5f37643037343962616537333861383163: bool = argument_1;
                let v_62617369735f5f61303436336632326438363633306432: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":240:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f61303436336632326438363633306432.clone()), &({ let left = ({ let v_7075695f6e61746976655f363431 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f363431.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f363431.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":240:60
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":241:7
                return (v_63686f6f736553656c656374696f6e.read().call)(v_696e6465785f5f38383736393263333739366462303863.clone(), v_73656c65637465645f5f37643037343962616537333861383163.clone(), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":243:5
            { let v_7075695f6e61746976655f363432 = v_646566.clone(); expose_method(&owner, Value::string("selectIndex"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f34626263393463626262386339393665: f64 = argument_0;
                let v_657874656e645f5f35373134396231636566643333613333: bool = argument_1;
                let v_62617369735f5f65613038396661303830636637356238: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":244:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f65613038396661303830636637356238.clone()), &({ let left = ({ let v_7075695f6e61746976655f363435 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f363435.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f363435.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":244:60
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":245:7
                return (v_73656c656374.read().call)(v_696e6465785f5f34626263393463626262386339393665.clone(), v_657874656e645f5f35373134396231636566643333613333.clone(), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":249:5
            { let v_7075695f6e61746976655f363436 = v_646566.clone(); expose_method(&owner, Value::string("keyAt"), ({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6b65795f5f33313237306232626462353633303563 = v_6b65795f5f33313237306232626462353633303563.clone(); let call:Rc<dyn Fn(f64, Value, bool, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: Value, argument_2: bool, argument_3: bool, argument_4: Value| -> bool {
                let v_696e6465785f5f39613964663633353332353461393130: f64 = argument_0;
                let v_6e616d655f5f36373230366437326532366632303463: Value = argument_1;
                let v_73686966745f5f63376633613532323338623461616631: bool = argument_2;
                let v_636f6d6d616e645f5f31343436643066333261653738303530: bool = argument_3;
                let v_62617369735f5f65393361336431303638333466646339: Value = argument_4;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":250:7
                if { let pui_native_647={ let pui_native_649={ let pui_native_651=!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !(pui_native_651) {(Value::boolean(!strict_equal(&(v_62617369735f5f65393361336431303638333466646339.clone()), &({ let left = ({ let v_7075695f6e61746976655f363535 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f363535.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_651} }; if !(pui_native_649) {!((v_7265616479.read().call)())} else {pui_native_649} }; if !(pui_native_647) {{ let pui_native_656=(v_6b696e64.read().call)(v_696e6465785f5f39613964663633353332353461393130.clone()); let pui_native_657=0.0_f64; (pui_native_656==pui_native_657) }} else {pui_native_647} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":250:104
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":251:13
                let v_6d6f64655f5f35356138323238306565323364336539 = { let left = ({ let v_7075695f6e61746976655f363538 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f363538.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":252:13
                let v_68616e646c6564: bool = { let pui_native_659={ let pui_native_661={ let pui_native_663={ let pui_native_665={ let pui_native_667={ let pui_native_669={ let pui_native_671={ let pui_native_673={ let pui_native_675={ let pui_native_677={ let pui_native_679={ let pui_native_681={ let pui_native_683={ let pui_native_685={ let pui_native_687=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Escape"))))).bool(); if !(pui_native_687) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_687} }; if !(pui_native_685) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string(" "))))).bool()} else {pui_native_685} }; if !(pui_native_683) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Home"))))).bool()} else {pui_native_683} }; if !(pui_native_681) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("End"))))).bool()} else {pui_native_681} }; if !(pui_native_679) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("ArrowLeft"))))).bool()} else {pui_native_679} }; if !(pui_native_677) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_677} }; if !(pui_native_675) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("ArrowUp"))))).bool()} else {pui_native_675} }; if !(pui_native_673) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_673} }; if !(pui_native_671) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("PageUp"))))).bool()} else {pui_native_671} }; if !(pui_native_669) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("PageDown"))))).bool()} else {pui_native_669} }; if !(pui_native_667) {{ let pui_native_711=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Tab"))))).bool(); if (pui_native_711) {{ let pui_native_715={ let pui_native_717=(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("points"))))).bool(); if !(pui_native_717) {(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("table"))))).bool()} else {pui_native_717} }; if !(pui_native_715) {(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("menu"))))).bool()} else {pui_native_715} }} else {pui_native_711} }} else {pui_native_667} }; if !(pui_native_665) {{ let pui_native_725=(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("table"))))).bool(); if (pui_native_725) {{ let pui_native_729={ let pui_native_731=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("F2"))))).bool(); if !(pui_native_731) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Delete"))))).bool()} else {pui_native_731} }; if !(pui_native_729) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("Backspace"))))).bool()} else {pui_native_729} }} else {pui_native_725} }} else {pui_native_665} }; if !(pui_native_663) {{ let pui_native_739=(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("points"))))).bool(); if (pui_native_739) {{ let pui_native_743={ let pui_native_745=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("+"))))).bool(); if !(pui_native_745) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("="))))).bool()} else {pui_native_745} }; if !(pui_native_743) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("-"))))).bool()} else {pui_native_743} }} else {pui_native_739} }} else {pui_native_663} }; if !(pui_native_661) {{ let pui_native_753={ let pui_native_755=(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("filterlist"))))).bool(); if (pui_native_755) {v_636f6d6d616e645f5f31343436643066333261653738303530.clone()} else {pui_native_755} }; if (pui_native_753) {{ let pui_native_759=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("f"))))).bool(); if !(pui_native_759) {(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("F"))))).bool()} else {pui_native_759} }} else {pui_native_753} }} else {pui_native_661} }; if !(pui_native_659) {{ let pui_native_765=(Value::boolean(strict_equal(&(v_6d6f64655f5f35356138323238306565323364336539.clone()), &(Value::string("tracklist"))))).bool(); if (pui_native_765) {{ let pui_native_769=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("ContextMenu"))))).bool(); if !(pui_native_769) {{ let pui_native_773=(Value::boolean(strict_equal(&(v_6e616d655f5f36373230366437326532366632303463.clone()), &(Value::string("F10"))))).bool(); if (pui_native_773) {v_73686966745f5f63376633613532323338623461616631.clone()} else {pui_native_773} }} else {pui_native_769} }} else {pui_native_765} }} else {pui_native_659} };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":259:7
                if !(v_68616e646c6564.clone()) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":259:21
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":260:13
                let v_7469636b65745f5f36376634363361343934346338336435: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":261:7
                { let v_7075695f6e61746976655f373737 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f373737.clone(), Value::number(v_696e6465785f5f39613964663633353332353461393130.clone()), Value::string("reason: addressed collection key")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":262:7
                if { let pui_native_778={ let pui_native_780=v_7469636b65745f5f36376634363361343934346338336435.clone(); let pui_native_781={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_780==pui_native_781) }; if !(pui_native_778) {!((v_7265616479.read().call)())} else {pui_native_778} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":262:41
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":263:7
                (v_656d6974.read().call)(Value::string("select"), v_696e6465785f5f39613964663633353332353461393130.clone(), false, Value::string(""));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":264:7
                if { let pui_native_782={ let pui_native_784={ let pui_native_786=v_7469636b65745f5f36376634363361343934346338336435.clone(); let pui_native_787={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_786==pui_native_787) }; if !(pui_native_784) {!((v_7265616479.read().call)())} else {pui_native_784} }; if !(pui_native_782) {(Value::boolean(!strict_equal(&(v_62617369735f5f65393361336431303638333466646339.clone()), &({ let left = ({ let v_7075695f6e61746976655f373930 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f373930.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f373930.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_782} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":264:92
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":265:7
                return (v_6b65795f5f33313237306232626462353633303563.read().call)(v_6e616d655f5f36373230366437326532366632303463.clone(), v_73686966745f5f63376633613532323338623461616631.clone(), v_636f6d6d616e645f5f31343436643066333261653738303530.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), arg(&arguments,1), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), arg(&arguments,4)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":267:5
            { let v_7075695f6e61746976655f373931 = v_646566.clone(); expose_method(&owner, Value::string("selectAndFocus"), ({  let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); let call:Rc<dyn Fn(f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: bool, argument_2: Value| -> bool {
                let v_696e6465785f5f31373832316333373836656561396135: f64 = argument_0;
                let v_657874656e645f5f38346337363031353938303939346237: bool = argument_1;
                let v_62617369735f5f66343336333961366362623061353935: Value = argument_2;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":268:7
                if { let pui_native_792=!(({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !(pui_native_792) {(Value::boolean(!strict_equal(&(v_62617369735f5f66343336333961366362623061353935.clone()), &({ let left = ({ let v_7075695f6e61746976655f373936 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f373936.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_792} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":268:71
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":269:7
                if { let pui_native_797=!((v_73656c656374.read().call)(v_696e6465785f5f31373832316333373836656561396135.clone(), v_657874656e645f5f38346337363031353938303939346237.clone(), true)); if !(pui_native_797) {(Value::boolean(!strict_equal(&(v_62617369735f5f66343336333961366362623061353935.clone()), &({ let left = ({ let v_7075695f6e61746976655f383031 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f383031.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f383031.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_797} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":269:92
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":270:7
                { let v_7075695f6e61746976655f383032 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f383032.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":271:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).bool(), arg(&arguments,2)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":273:5
            { let v_7075695f6e61746976655f383033 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b65795f5f33313237306232626462353633303563.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":274:5
            { let v_7075695f6e61746976655f383034 = v_646566.clone(); expose_method(&owner, Value::string("contextAt"), (v_636f6e746578744174.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":275:5
            { let v_7075695f6e61746976655f383035 = v_646566.clone(); expose_method(&owner, Value::string("reconcile"), ({  let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_73796e63 = v_73796e63.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:50
                let v_6f776e65725f5f39383566666437346264316139323338 = { let value = (v_63757272656e74.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:67
                if { let pui_native_806=!((v_6f776e65725f5f39383566666437346264316139323338.clone()).truthy()); if !(pui_native_806) {!((v_7265616479.read().call)())} else {pui_native_806} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":275:91
                    return false;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:105
                v_73796e63.read().call_authored(vec![v_6f776e65725f5f39383566666437346264316139323338.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":275:118
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":276:5
            { let v_7075695f6e61746976655f383038 = v_646566.clone(); expose_method(&owner, Value::string("dismiss"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":276:42
                if (v_7265616479.read().call)() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":276:55
                    (v_656d6974.read().call)(Value::string("close"), ({ let v_7075695f6e61746976655f383039 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f383039.clone()).get()).num()) }).num(), false, Value::string("outside"));
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":277:5
            { let v_7075695f6e61746976655f383130 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":277:44
                if (v_7265616479.read().call)() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":277:57
                    { let v_7075695f6e61746976655f383131 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f383131.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":278:5
            { let v_7075695f6e61746976655f383132 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b65795f5f33313237306232626462353633303563 = v_6b65795f5f33313237306232626462353633303563.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":279:7
                if (v_6b65795f5f33313237306232626462353633303563.read().call)({ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), { let pui_native_815=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_815) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_815} }) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":279:108
                    { let v_7075695f6e61746976655f383231 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f383231.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":281:11
            v_73796e63.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_7461626c65 = v_7461626c65.clone(); let v_726f7773 = v_726f7773.clone(); let v_636f6c756d6e73 = v_636f6c756d6e73.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_7265717565737452657665616c = v_7265717565737452657665616c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f33613732633933653133343230653934: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":282:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":282:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":283:13
                let v_7469636b65745f5f65363232633838623764666431383734: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":283:31
                { let value = v_72756e5f5f33613732633933653133343230653934.clone(); *(v_63757272656e74.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":284:13
                let v_70726f7073 = { let v_7075695f6e61746976655f383232 = v_72756e5f5f33613732633933653133343230653934.clone(); read_props(&(v_7075695f6e61746976655f383232.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":285:7
                { let v_7075695f6e61746976655f383233 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f383233.clone(), if (Value::boolean({ let pui_native_824={ let pui_native_826=(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("points"))))).bool(); if !(pui_native_826) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("tracklist"))))).bool()} else {pui_native_826} }; if !(pui_native_824) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_824} })).truthy() { Value::string("listbox") } else { { let left = (v_70726f7073.clone()).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } } }, Value::string("reason: collection role")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":286:7
                if { let pui_native_834={ let pui_native_836={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_837=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_836==pui_native_837) }; if !(pui_native_834) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_834} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":286:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":287:7
                { let v_7075695f6e61746976655f383338 = v_7461626c65.read(); state_set(v_7075695f6e61746976655f383338.clone(), Value::boolean((Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("table"))))).bool()), Value::string("reason: collection table semantics")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":288:7
                if { let pui_native_841={ let pui_native_843={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_844=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_843==pui_native_844) }; if !(pui_native_841) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_841} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":288:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":289:7
                { let v_7075695f6e61746976655f383435 = v_726f7773.read(); state_set(v_7075695f6e61746976655f383435.clone(), Value::number((v_636f756e74.read().call)()), Value::string("reason: collection rows")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":290:7
                if { let pui_native_846={ let pui_native_848={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_849=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_848==pui_native_849) }; if !(pui_native_846) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_846} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":290:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":291:7
                { let v_7075695f6e61746976655f383530 = v_636f6c756d6e73.read(); state_set(v_7075695f6e61746976655f383530.clone(), Value::number(((v_70726f7073.clone()).member("columns", false)).num()), Value::string("reason: collection columns")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":292:7
                if { let pui_native_853={ let pui_native_855={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_856=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_855==pui_native_856) }; if !(pui_native_853) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_853} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":292:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":293:7
                { let v_7075695f6e61746976655f383537 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f383537.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection name")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":294:7
                if { let pui_native_858={ let pui_native_860={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_861=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_860==pui_native_861) }; if !(pui_native_858) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_858} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":294:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":295:7
                { let v_7075695f6e61746976655f383632 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f383632.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection description")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":296:7
                if { let pui_native_863={ let pui_native_865={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_866=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_865==pui_native_866) }; if !(pui_native_863) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_863} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":296:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":297:7
                { let v_7075695f6e61746976655f383637 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f383637.clone(), Value::boolean(((v_70726f7073.clone()).member("disabled", false)).bool()), Value::string("reason: collection policy")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":298:7
                if { let pui_native_870={ let pui_native_872={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_873=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_872==pui_native_873) }; if !(pui_native_870) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_870} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":298:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":299:7
                { let v_7075695f6e61746976655f383734 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f383734.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f383735 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f383735.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":300:7
                if { let pui_native_876={ let pui_native_878={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_879=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_878==pui_native_879) }; if !(pui_native_876) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_876} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":300:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":301:13
                let v_7265717565737465645f5f37363535373865336536303032633938: f64 = ((v_70726f7073.clone()).member("cursor", false)).num();
                // Source "frontend/proto-ui/src/collection-control.proto.ts":302:13
                let v_6163636570746564: f64 = if { let pui_native_882=(v_7265717565737465645f5f37363535373865336536303032633938.clone()).is_finite(); if (pui_native_882) {{ let pui_native_884={ let pui_native_886:f64=v_7265717565737465645f5f37363535373865336536303032633938.clone(); let pui_native_887:f64=1.0_f64; pui_native_886 % pui_native_887 }; let pui_native_885=0.0_f64; (pui_native_884==pui_native_885) }} else {pui_native_882} } {(v_6d6178.read().call)(0.0_f64, (v_6d696e.read().call)({ let pui_native_888:f64=(v_636f756e74.read().call)(); let pui_native_889:f64=1.0_f64; pui_native_888 - pui_native_889 }, v_7265717565737465645f5f37363535373865336536303032633938.clone()))} else {0.0_f64};
                // Source "frontend/proto-ui/src/collection-control.proto.ts":303:7
                { let v_7075695f6e61746976655f383930 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f383930.clone(), Value::number(if { let pui_native_891:f64=(v_6b696e64.read().call)(v_6163636570746564.clone()); let pui_native_892:f64=0.0_f64; pui_native_891 > pui_native_892 } {v_6163636570746564.clone()} else {(v_6d6178.read().call)(0.0_f64, (v_7365656b.read().call)(0.0_f64, 1.0_f64, (v_636f756e74.read().call)(), false))}), Value::string("reason: accepted collection cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":304:7
                if { let pui_native_893={ let pui_native_895={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_896=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_895==pui_native_896) }; if !(pui_native_893) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_893} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":304:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":305:7
                { let v_7075695f6e61746976655f383937 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f383937.clone(), Value::number(((v_70726f7073.clone()).member("expanded", false)).num()), Value::string("reason: accepted expanded item")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":306:7
                if { let pui_native_900={ let pui_native_902={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_903=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_902==pui_native_903) }; if !(pui_native_900) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_900} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":306:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":310:7
                if { let pui_native_904=(v_7265616479.read().call)(); if (pui_native_904) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("filterlist"))))).bool()} else {pui_native_904} } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":311:9
                    if { let pui_native_908=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f393130 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f393130.clone()).get()).num()) }).num()); let pui_native_909=0.0_f64; (pui_native_908==pui_native_909) } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":311:39
                        (v_656d6974.read().call)(Value::string("select"), -(1.0_f64), false, Value::string("reconcile"));
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":312:14
                        if { let pui_native_911=((v_70726f7073.clone()).member("cursor", false)).num(); let pui_native_912=({ let v_7075695f6e61746976655f393133 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f393133.clone()).get()).num()) }).num(); !(pui_native_911==pui_native_912) } {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":312:49
                            (v_656d6974.read().call)(Value::string("select"), ({ let v_7075695f6e61746976655f393134 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f393134.clone()).get()).num()) }).num(), false, Value::string("reconcile"));
                        }
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":313:9
                    if { let pui_native_915={ let pui_native_917={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_918=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_917==pui_native_918) }; if !(pui_native_915) {!((v_7265616479.read().call)())} else {pui_native_915} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":313:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":314:9
                    if { let pui_native_919:f64=(v_6b696e64.read().call)(({ let v_7075695f6e61746976655f393231 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f393231.clone()).get()).num()) }).num()); let pui_native_920:f64=0.0_f64; pui_native_919 > pui_native_920 } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":314:37
                        (v_7265717565737452657665616c.read().call)(({ let v_7075695f6e61746976655f393232 = v_637572736f72.read(); Value::number(((v_7075695f6e61746976655f393232.clone()).get()).num()) }).num());
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":315:9
                    if { let pui_native_923={ let pui_native_925={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_926=v_7469636b65745f5f65363232633838623764666431383734.clone(); !(pui_native_925==pui_native_926) }; if !(pui_native_923) {!((v_7265616479.read().call)())} else {pui_native_923} } {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":315:43
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":317:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":317:21
                    { let v_7075695f6e61746976655f393237 = v_72756e5f5f33613732633933653133343230653934.clone(); request_update(&(v_7075695f6e61746976655f393237.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":319:5
            { let v_7075695f6e61746976655f393238 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":320:5
            { let v_7075695f6e61746976655f393239 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f65363438313039316238653232326131: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":320:40
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":320:57
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":320:74
                    v_73796e63.read().call_authored(vec![v_72756e5f5f65363438313039316238653232326131.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":320:85
                    if (({ let v_7075695f6e61746976655f393330 = v_72756e5f5f65363438313039316238653232326131.clone(); read_props(&(v_7075695f6e61746976655f393330.clone()).owner(), false) }).member("autoFocus", false)).bool() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":320:116
                        { let v_7075695f6e61746976655f393331 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f393331.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":321:5
            { let v_7075695f6e61746976655f393332 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("mode"), Value::string("label"), Value::string("description"), Value::string("cursor"), Value::string("expanded"), Value::string("revision"), Value::string("disabled"), Value::string("autoFocus"), Value::string("columns"), Value::string("orientation")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":322:5
            { let v_7075695f6e61746976655f393333 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":322:39
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":322:48
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":322:66
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":323:5
            { let v_7075695f6e61746976655f393334 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":323:43
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":323:52
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":323:69
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/collection-control.proto.ts":323:87
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":324:5
            { let v_7075695f6e61746976655f393335 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":325:5
            return { let callable={ let pui_native_936={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":325:26
                return { let v_7075695f6e61746976655f393337 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_936.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_936.call)(arg(&arguments,0))})} }; match callable {
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
