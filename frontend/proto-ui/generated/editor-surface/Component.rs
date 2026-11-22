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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("threshold").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("commands").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "threshold" => value.as_f64().is_some_and(f64::is_finite),
        "commands" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "cancelPointer" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "activate" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "doubleActivate" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "navigate" => if !(arguments.len() <= 8 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(6).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(7).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "wheel" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "pinch" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "regionGesture" => if !(arguments.len() <= 12 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(6).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(7).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(8).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(9).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(10).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(11).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "pointGesture" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "curveGesture" => if !(arguments.len() <= 8 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(6).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(7).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "noteGesture" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "phonemeGesture" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "rulerGesture" => if !(arguments.len() <= 8 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(6).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(7).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "motion" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "edgePan" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "rulerSample" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "audioKey" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "commands", default, skip_serializing_if = "Option::is_none")]
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
    #[serde(rename = "scale")]
    pub field_3: f64,
    #[serde(rename = "exponent")]
    pub field_4: f64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData2 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "selection")]
    pub field_1: String,
    #[serde(rename = "snap")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData3 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "snap")]
    pub field_1: bool,
    #[serde(rename = "fine")]
    pub field_2: f64,
    #[serde(rename = "axis")]
    pub field_3: String,
    #[serde(rename = "moveX")]
    pub field_4: bool,
    #[serde(rename = "moveY")]
    pub field_5: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData4 {
    #[serde(rename = "position")]
    pub field_0: f64,
    #[serde(rename = "delta")]
    pub field_1: f64,
    #[serde(rename = "active")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData5 {
    #[serde(rename = "position")]
    pub field_0: f64,
    #[serde(rename = "delta")]
    pub field_1: f64,
    #[serde(rename = "active")]
    pub field_2: bool,
    #[serde(rename = "snap")]
    pub field_3: bool,
    #[serde(rename = "accepted")]
    pub field_4: bool,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct EditorSurfaceHandle { pub owner: OwnerRef }
  impl EditorSurfaceHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<String, Value> { self.state("pointer").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("lease").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<String, Value> { self.state("operation").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<String, Value> { self.state("axis").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_6(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_7(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_9(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_10(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_11(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_12(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_13(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_14(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_15(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_16(&self, argument_0: String, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument")), Value::json(serde_json::to_value(argument_7).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("navigate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("navigate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_17(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("wheel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("wheel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_18(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("pinch").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("pinch", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_19(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: f64, argument_6: f64, argument_7: f64, argument_8: f64, argument_9: f64, argument_10: bool, argument_11: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument")), Value::json(serde_json::to_value(argument_7).expect("portable argument")), Value::json(serde_json::to_value(argument_8).expect("portable argument")), Value::json(serde_json::to_value(argument_9).expect("portable argument")), Value::json(serde_json::to_value(argument_10).expect("portable argument")), Value::json(serde_json::to_value(argument_11).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("regionGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("regionGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_20(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("pointGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("pointGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_21(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument")), Value::json(serde_json::to_value(argument_7).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("curveGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("curveGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_22(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("noteGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("noteGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_23(&self, argument_0: bool, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("phonemeGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("phonemeGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_24(&self, argument_0: bool, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData2, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument")), Value::json(serde_json::to_value(argument_7).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("rulerGesture").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("rulerGesture", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("selection").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_25(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData3, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("motion").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("motion", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()) && result.get("fine").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("axis").map_or(false, |field| field.is_string()) && result.get("moveX").map_or(false, |field| field.is_boolean()) && result.get("moveY").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_26(&self, argument_0: f64, argument_1: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData4, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("edgePan").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("edgePan", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("position").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("delta").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("active").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_27(&self, argument_0: f64, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("rulerSample").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("rulerSample", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("position").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("delta").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("active").map_or(false, |field| field.is_boolean()) && result.get("snap").map_or(false, |field| field.is_boolean()) && result.get("accepted").map_or(false, |field| field.is_boolean()))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_28(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: String, window: &mut Window, cx: &mut App) -> Result<String, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("audioKey").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("audioKey", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_string()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
      pub fn owner(&self) -> OwnerWeak { Rc::downgrade(&self.owner) }
      pub fn state(&self, name: &str) -> Result<Json, Value> { catch_authored(|| { let state=self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed State")); state.get().data() }) }
      pub fn call(&self, name: &str, arguments: Vec<Json>, window: &mut Window, cx: &mut App) -> Result<Json, Value> {
          catch_authored(|| {
          refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
          let callback = self.owner.borrow().exposes.get(name).cloned().unwrap_or_else(|| value_error("unknown exposed method"));
          let arguments: Vec<Value> = arguments.into_iter().map(Value::json).collect(); check_method(name, &arguments);
          let result = invoke(&self.owner, "expose-method", &callback, arguments);
          let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
          if matches!(result, Value::Undefined) { Json::Null } else { result.data() }
          })
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
            let v_706f696e746572: LexicalBinding<Value> = lexical_binding();
            let v_6c65617365: LexicalBinding<Value> = lexical_binding();
            let v_6f7065726174696f6e: LexicalBinding<Value> = lexical_binding();
            let v_61786973: LexicalBinding<Value> = lexical_binding();
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
            let v_6e6578744c65617365: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6672656554696d65: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_66696e697465: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<Value> = lexical_binding();
            let v_6e61766967617465: LexicalBinding<Value> = lexical_binding();
            let v_696e74656e74: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("threshold", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("commands", record(vec![("type", Value::string("string")), ("default", Value::string("none"))]))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":22:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":23:11
            v_706f696e746572.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("pointer"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":24:11
            v_6c65617365.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "discrete", Value::string("lease"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":25:11
            v_6f7065726174696f6e.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("operation"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":26:11
            v_61786973.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("axis"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":27:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":28:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":29:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":30:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":31:5
            { let v_7075695f6e61746976655f38 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f38.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":32:5
            { let v_7075695f6e61746976655f39 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f39.clone()).owner(), Value::string("group")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":33:5
            { let v_7075695f6e61746976655f3130 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f3130.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":34:5
            { let v_7075695f6e61746976655f3131 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3131.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":35:5
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("overflow-hidden".into(), "overflow-hidden".into())])]) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":36:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":37:5
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_state(&owner, Value::string("pointer"), v_706f696e746572.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":38:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_state(&owner, Value::string("lease"), v_6c65617365.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":39:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_state(&owner, Value::string("operation"), v_6f7065726174696f6e.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":40:5
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_state(&owner, Value::string("axis"), v_61786973.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":41:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":42:5
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":43:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":44:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":45:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":46:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":47:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":48:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":49:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":50:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":52:9
            v_6f726967696e58.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":53:9
            v_6f726967696e59.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":54:9
            v_6e6578744c65617365.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":55:9
            v_6672656554696d65.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":56:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f30333433356234313063313666623335: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":56:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30333433356234313063313666623335.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":57:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f37666237353839323836353538303736: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":57:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37666237353839323836353538303736.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":58:11
            v_66696e697465.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_785f5f63326434646266393737343865366539: Value = arg(&arguments, 0);
                let v_795f5f30623262356637343436653939623537: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":58:55
                return { let left = number_is_finite(&(v_785f5f63326434646266393737343865366539.clone())); if left.truthy() { number_is_finite(&(v_795f5f30623262356637343436653939623537.clone())) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":59:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":60:13
                let v_6f776e65725f5f62313631316637316132633337386237 = { let v_7075695f6e61746976655f3232 = v_706f696e746572.read(); (v_7075695f6e61746976655f3232.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:13
                let v_726574697265645f5f36356562316263323133663139313365 = { let v_7075695f6e61746976655f3233 = v_6c65617365.read(); (v_7075695f6e61746976655f3233.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":62:13
                let v_62617369735f5f33356161353337326136303638663134 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":63:13
                let v_6163746976655f5f34393032303735333130643462323031 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3234 = v_7068617365.read(); (v_7075695f6e61746976655f3234.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":64:13
                let v_7469636b65745f5f32323034373566356231373731353262 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":65:7
                { let value = Value::boolean(false); *(v_6672656554696d65.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":66:7
                { let v_7075695f6e61746976655f3235 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3235.clone(), Value::string("idle"), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:29
                    return v_6163746976655f5f34393032303735333130643462323031.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":68:7
                { let v_7075695f6e61746976655f3236 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3236.clone(), Value::string(""), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":69:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":69:29
                    return v_6163746976655f5f34393032303735333130643462323031.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":70:7
                { let v_7075695f6e61746976655f3237 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3237.clone(), Value::number(0.0), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:29
                    return v_6163746976655f5f34393032303735333130643462323031.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":72:7
                { let v_7075695f6e61746976655f3238 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f3238.clone(), Value::string(""), Value::string("reason: editing operation retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:29
                    return v_6163746976655f5f34393032303735333130643462323031.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":74:7
                { let v_7075695f6e61746976655f3239 = v_61786973.read(); state_set(v_7075695f6e61746976655f3239.clone(), Value::string(""), Value::string("reason: editing axis retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:29
                    return v_6163746976655f5f34393032303735333130643462323031.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:7
                if ({ let left = v_6163746976655f5f34393032303735333130643462323031.clone(); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:26
                    { let v_7075695f6e61746976655f3330 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3330.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f62313631316637316132633337386237.clone()), ("revision", v_62617369735f5f33356161353337326136303638663134.clone()), ("lease", v_726574697265645f5f36356562316263323133663139313365.clone())])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:7
                if ({ let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f32323034373566356231373731353262.clone()]); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:35
                    { let v_7075695f6e61746976655f3331 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3331.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":78:7
                return v_6163746976655f5f34393032303735333130643462323031.clone();
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":80:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_6e6578744c65617365 = v_6e6578744c65617365.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f62663234616661353837396138353866: Value = arg(&arguments, 0);
                let v_785f5f64383133333565333661636135393531: Value = arg(&arguments, 1);
                let v_795f5f33313563383339613034316233653833: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3333 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3333.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3334 = v_7068617365.read(); (v_7075695f6e61746976655f3334.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6f776e65725f5f62663234616661353837396138353866.clone()), &(Value::string("")))) } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f64383133333565333661636135393531.clone(), v_795f5f33313563383339613034316233653833.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":82:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":83:13
                let v_7469636b65745f5f33333835316361306564346539366435 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":84:7
                { let value = v_785f5f64383133333565333661636135393531.clone(); *(v_6f726967696e58.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":85:7
                { let value = v_795f5f33313563383339613034316233653833.clone(); *(v_6f726967696e59.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":86:7
                { let value = Value::boolean(false); *(v_6672656554696d65.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":87:7
                { let value = add({ let value = (v_6e6578744c65617365.read()).borrow().clone(); value }, Value::number(1.0)); *(v_6e6578744c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":88:7
                { let v_7075695f6e61746976655f3335 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f3335.clone(), Value::string(""), Value::string("reason: editing operation acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:7
                { let v_7075695f6e61746976655f3336 = v_61786973.read(); state_set(v_7075695f6e61746976655f3336.clone(), Value::string(""), Value::string("reason: editing axis acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":91:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":91:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":92:7
                { let v_7075695f6e61746976655f3337 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3337.clone(), { let value = (v_6e6578744c65617365.read()).borrow().clone(); value }, Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":93:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":93:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":94:7
                { let v_7075695f6e61746976655f3338 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3338.clone(), v_6f776e65725f5f62663234616661353837396138353866.clone(), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:7
                { let v_7075695f6e61746976655f3339 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3339.clone(), Value::string("pressed"), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":97:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":97:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:7
                { let v_7075695f6e61746976655f3430 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3430.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:16
                    { let v_7075695f6e61746976655f3431 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3431.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":101:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33333835316361306564346539366435.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":103:11
            let v_73616d706c65 = {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f61303461633433343066306234646430: Value = arg(&arguments, 0);
                let v_785f5f34626134303339613938336530343564: Value = arg(&arguments, 1);
                let v_795f5f34336438653238386166393133346438: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":104:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3432 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3432.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3433 = v_7068617365.read(); (v_7075695f6e61746976655f3433.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3434 = v_706f696e746572.read(); (v_7075695f6e61746976655f3434.clone()).get() }), &(v_6f776e65725f5f61303461633433343066306234646430.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":104:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f34626134303339613938336530343564.clone(), v_795f5f34336438653238386166393133346438.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:13
                let v_7469636b65745f5f61643335343739323835663232303534 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":107:13
                let v_726177 = { let left = ({ let v_7075695f6e61746976655f3435 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3435.clone()).owner(), false) }).member("threshold", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":108:13
                let v_7468726573686f6c64 = if ({ let left = { let relational_left = v_726177.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { number_is_finite(&(v_726177.clone())) } else { left } }).truthy() { v_726177.clone() } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":109:13
                let v_64785f5f63666161356561616464643963306236 = Value::number((v_785f5f34626134303339613938336530343564.clone()).scalar_number() - ({ let value = (v_6f726967696e58.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":110:13
                let v_64795f5f35373262653939326332306339306166 = Value::number((v_795f5f34336438653238386166393133346438.clone()).scalar_number() - ({ let value = (v_6f726967696e59.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3436 = v_7068617365.read(); (v_7075695f6e61746976655f3436.clone()).get() }), &(Value::string("pressed")))); if left.truthy() { { let relational_left = add(Value::number((v_64785f5f63666161356561616464643963306236.clone()).scalar_number() * (v_64785f5f63666161356561616464643963306236.clone()).scalar_number()), Value::number((v_64795f5f35373262653939326332306339306166.clone()).scalar_number() * (v_64795f5f35373262653939326332306339306166.clone()).scalar_number())); let relational_right = Value::number((v_7468726573686f6c64.clone()).scalar_number() * (v_7468726573686f6c64.clone()).scalar_number()); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:7
                { let v_7075695f6e61746976655f3437 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3437.clone(), Value::string("dragging"), Value::string("reason: editing movement accepted")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f61643335343739323835663232303534.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:16
                    { let v_7075695f6e61746976655f3438 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3438.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":115:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f61643335343739323835663232303534.clone()]);
            })) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":117:5
            { let v_7075695f6e61746976655f3439 = v_646566.clone(); expose_method(&owner, Value::string("sample"), v_73616d706c65.clone()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":118:5
            { let v_7075695f6e61746976655f3530 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f31643565353166323465346534343565: Value = arg(&arguments, 0);
                let v_785f5f64623361326466353161613638333030: Value = arg(&arguments, 1);
                let v_795f5f38633566326234356636356236376565: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3531 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3531.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3532 = v_7068617365.read(); (v_7075695f6e61746976655f3532.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3533 = v_706f696e746572.read(); (v_7075695f6e61746976655f3533.clone()).get() }), &(v_6f776e65725f5f31643565353166323465346534343565.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":120:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f64623361326466353161613638333030.clone(), v_795f5f38633566326234356636356236376565.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":120:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":120:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":121:13
                let v_7469636b65745f5f39343038666661633162343535383233 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":122:7
                { let v_7075695f6e61746976655f3534 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3534.clone(), Value::string("idle"), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:7
                { let v_7075695f6e61746976655f3535 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3535.clone(), Value::string(""), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:7
                { let v_7075695f6e61746976655f3536 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3536.clone(), Value::number(0.0), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":128:7
                { let v_7075695f6e61746976655f3537 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f3537.clone(), Value::string(""), Value::string("reason: editing operation completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":129:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":129:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:7
                { let v_7075695f6e61746976655f3538 = v_61786973.read(); state_set(v_7075695f6e61746976655f3538.clone(), Value::string(""), Value::string("reason: editing axis completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":131:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":131:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":132:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":132:16
                    { let v_7075695f6e61746976655f3539 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3539.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":133:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39343038666661633162343535383233.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":135:5
            { let v_7075695f6e61746976655f3630 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":136:5
            { let v_7075695f6e61746976655f3631 = v_646566.clone(); expose_method(&owner, Value::string("cancelPointer"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f63333934643039386335613737636663: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3632 = v_7068617365.read(); (v_7075695f6e61746976655f3632.clone()).get() }), &(Value::string("idle")))); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3633 = v_706f696e746572.read(); (v_7075695f6e61746976655f3633.clone()).get() }), &(v_6f776e65725f5f63333934643039386335613737636663.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:7
                return v_63616e63656c.read().call_authored(vec![]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":140:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3634 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3634.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:13
                let v_7469636b65745f5f64613531373461646336396530663239 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":143:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":144:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64613531373461646336396530663239.clone()])).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3635 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3635.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":144:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":145:7
                { let v_7075695f6e61746976655f3636 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3636.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":146:7
                return { let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64613531373461646336396530663239.clone()]); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3637 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3637.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":148:5
            { let v_7075695f6e61746976655f3638 = v_646566.clone(); expose_method(&owner, Value::string("activate"), v_6163746976617465.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":149:5
            { let v_7075695f6e61746976655f3639 = v_646566.clone(); expose_method(&owner, Value::string("doubleActivate"), {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_636c69636b735f5f63366266386232326535373735343063: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":149:61
                return { let left = { let left = number_is_finite(&(v_636c69636b735f5f63366266386232326535373735343063.clone())); if left.truthy() { { let relational_left = v_636c69636b735f5f63366266386232326535373735343063.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { v_6163746976617465.read().call_authored(vec![]) } else { left } };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":152:11
            v_6e61766967617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_736f75726365: Value = arg(&arguments, 0);
                let v_64785f5f64656664613233396335633465663636: Value = arg(&arguments, 1);
                let v_64795f5f64386561396464353066386233613466: Value = arg(&arguments, 2);
                let v_7363616c655f5f61666234313430616232363932303564: Value = arg(&arguments, 3);
                let v_636f6e74726f6c5f5f30323938383361393336343965623765: Value = arg(&arguments, 4);
                let v_73686966745f5f39363236346638656637343466643963: Value = arg(&arguments, 5);
                let v_616c745f5f62626237636238623337333062396332: Value = arg(&arguments, 6);
                let v_617272616e67655f5f35636664333865353030656438666336: Value = arg(&arguments, 7);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:13
                let v_6e6f6e655f5f62333839366633626435323833333536 = record(vec![("kind", Value::string("none")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", Value::number(1.0)), ("exponent", Value::number(0.0))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":154:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3730 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3730.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_64785f5f64656664613233396335633465663636.clone(), v_64795f5f64386561396464353066386233613466.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":154:90
                    return v_6e6f6e655f5f62333839366633626435323833333536.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("zoom-in"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:33
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", Value::number(1.25)), ("exponent", Value::number(1.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("zoom-out"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:34
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", Value::number(0.8)), ("exponent", Value::number(1.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("pinch"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:31
                    return if ({ let left = number_is_finite(&(v_7363616c655f5f61666234313430616232363932303564.clone())); if left.truthy() { { let relational_left = v_7363616c655f5f61666234313430616232363932303564.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() { record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", v_7363616c655f5f61666234313430616232363932303564.clone()), ("exponent", Value::number(1.0))]) } else { v_6e6f6e655f5f62333839366633626435323833333536.clone() };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":161:7
                if (Value::boolean(!strict_equal(&(v_736f75726365.clone()), &(Value::string("wheel"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":161:31
                    return v_6e6f6e655f5f62333839366633626435323833333536.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:7
                if (v_636f6e74726f6c5f5f30323938383361393336343965623765.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:20
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", Value::number(1.0015)), ("exponent", v_64795f5f64386561396464353066386233613466.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:7
                if ({ let left = Value::boolean(!(v_617272616e67655f5f35636664333865353030656438666336.clone()).truthy()); if left.truthy() { v_616c745f5f62626237636238623337333062396332.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:28
                    return record(vec![("kind", Value::string("zoom-y")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("scale", Value::number(1.0015)), ("exponent", v_64795f5f64386561396464353066386233613466.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:7
                if (v_73686966745f5f39363236346638656637343466643963.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:18
                    return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(if (Value::boolean(!strict_equal(&(v_64785f5f64656664613233396335633465663636.clone()), &(Value::number(0.0))))).truthy() { v_64785f5f64656664613233396335633465663636.clone() } else { v_64795f5f64386561396464353066386233613466.clone() }).scalar_number())), ("y", Value::number(0.0)), ("scale", Value::number(1.0)), ("exponent", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:7
                if ({ let left = v_617272616e67655f5f35636664333865353030656438666336.clone(); if left.truthy() { Value::boolean(strict_equal(&(v_64785f5f64656664613233396335633465663636.clone()), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:32
                    return v_6e6f6e655f5f62333839366633626435323833333536.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":166:7
                return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(v_64785f5f64656664613233396335633465663636.clone()).scalar_number())), ("y", if (v_617272616e67655f5f35636664333865353030656438666336.clone()).truthy() { Value::number(0.0) } else { Value::number(-(v_64795f5f64386561396464353066386233613466.clone()).scalar_number()) }), ("scale", Value::number(1.0)), ("exponent", Value::number(0.0))]);
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":168:5
            { let v_7075695f6e61746976655f3731 = v_646566.clone(); expose_method(&owner, Value::string("navigate"), v_6e61766967617465.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:5
            { let v_7075695f6e61746976655f3732 = v_646566.clone(); expose_method(&owner, Value::string("wheel"), {  let v_6e61766967617465 = v_6e61766967617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f64623763643630653839356534643633: Value = arg(&arguments, 0);
                let v_64795f5f35663839643739313139383630343862: Value = arg(&arguments, 1);
                let v_636f6e74726f6c5f5f36646461323338663262373332333131: Value = arg(&arguments, 2);
                let v_73686966745f5f32336466623632393666323939313431: Value = arg(&arguments, 3);
                let v_616c745f5f32323733306164353163336537366162: Value = arg(&arguments, 4);
                let v_617272616e67655f5f38303163346339396630343061643865: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:126
                return v_6e61766967617465.read().call_authored(vec![Value::string("wheel"), v_64785f5f64623763643630653839356534643633.clone(), v_64795f5f35663839643739313139383630343862.clone(), Value::number(1.0), v_636f6e74726f6c5f5f36646461323338663262373332333131.clone(), v_73686966745f5f32336466623632393666323939313431.clone(), v_616c745f5f32323733306164353163336537366162.clone(), v_617272616e67655f5f38303163346339396630343061643865.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:5
            { let v_7075695f6e61746976655f3733 = v_646566.clone(); expose_method(&owner, Value::string("pinch"), {  let v_6e61766967617465 = v_6e61766967617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7363616c655f5f30333232663961373264336431313137: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:51
                return v_6e61766967617465.read().call_authored(vec![Value::string("pinch"), Value::number(0.0), Value::number(0.0), v_7363616c655f5f30333232663961373264336431313137.clone(), Value::boolean(false), Value::boolean(false), Value::boolean(false), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":171:11
            v_696e74656e74.initialize({  let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b696e645f5f30636631313433346533626362363533: Value = arg(&arguments, 0);
                let v_73656c656374696f6e: Value = arg(&arguments, 1);
                let v_736e61705f5f63343533643832356362333634373936: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":172:13
                let v_6e6f6e655f5f37653261643834373962623362363232 = record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":173:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3734 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3734.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3735 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f3735.clone()).get() }), &(Value::string("")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f30636631313433346533626362363533.clone()), &(Value::string("none")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":173:116
                    return v_6e6f6e655f5f37653261643834373962623362363232.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":174:13
                let v_7469636b65745f5f62643832356337376665656633303763 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":175:7
                { let v_7075695f6e61746976655f3736 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f3736.clone(), v_6b696e645f5f30636631313433346533626362363533.clone(), Value::string("reason: editing operation chosen")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":176:7
                return if ({ let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62643832356337376665656633303763.clone()]); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3737 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3737.clone()).get() }).truthy()) } else { left } }).truthy() { record(vec![("kind", v_6b696e645f5f30636631313433346533626362363533.clone()), ("selection", v_73656c656374696f6e.clone()), ("snap", v_736e61705f5f63343533643832356362333634373936.clone())]) } else { v_6e6f6e655f5f37653261643834373962623362363232.clone() };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":178:5
            { let v_7075695f6e61746976655f3738 = v_646566.clone(); expose_method(&owner, Value::string("regionGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6869745f5f64623438643364366566306638646566: Value = arg(&arguments, 0);
                let v_72616e6765: Value = arg(&arguments, 1);
                let v_7365636f6e646172795f5f35393633626466666132343461343562: Value = arg(&arguments, 2);
                let v_657874656e64: Value = arg(&arguments, 3);
                let v_73706c6974: Value = arg(&arguments, 4);
                let v_636c69636b735f5f63393538656663666532373361363932: Value = arg(&arguments, 5);
                let v_6c656674: Value = arg(&arguments, 6);
                let v_7269676874: Value = arg(&arguments, 7);
                let v_66616465496e: Value = arg(&arguments, 8);
                let v_666164654f7574: Value = arg(&arguments, 9);
                let v_6661646542616e64: Value = arg(&arguments, 10);
                let v_616c745f5f63636539653739653666646539333137: Value = arg(&arguments, 11);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":180:7
                if ({ let left = Value::boolean(!(v_7365636f6e646172795f5f35393633626466666132343461343562.clone()).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3739 = v_7068617365.read(); (v_7075695f6e61746976655f3739.clone()).get() }), &(Value::string("pressed")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":180:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:7
                if (v_7365636f6e646172795f5f35393633626466666132343461343562.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:22
                    return v_696e74656e74.read().call_authored(vec![if (Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("empty"))))).truthy() { Value::string("lane-menu") } else { Value::string("region-menu") }, if ({ let left = Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { v_657874656e64.clone() } else { left } }).truthy() { Value::string("extend") } else { Value::string("replace") }, Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:7
                if (v_72616e6765.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:18
                    return v_696e74656e74.read().call_authored(vec![Value::string("range"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:7
                if (Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("empty"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:28
                    return v_696e74656e74.read().call_authored(vec![if ({ let relational_left = v_636c69636b735f5f63393538656663666532373361363932.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() { Value::string("create-part") } else { Value::string("seek") }, Value::string("clear"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":184:7
                if ({ let left = Value::boolean(!strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("part")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":184:46
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:7
                if ({ let left = Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { v_657874656e64.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:38
                    return v_696e74656e74.read().call_authored(vec![Value::string("select"), Value::string("extend"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:7
                if ({ let left = Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { v_73706c6974.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:37
                    return v_696e74656e74.read().call_authored(vec![Value::string("split"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:7
                if ({ let relational_left = v_636c69636b735f5f63393538656663666532373361363932.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:24
                    return v_696e74656e74.read().call_authored(vec![if (Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio"))))).truthy() { Value::string("region-menu") } else { Value::string("open-part") }, Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":188:7
                if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { v_6661646542616e64.clone() } else { left } }; if left.truthy() { { let relational_left = v_66616465496e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { { let relational_left = v_66616465496e.clone(); let relational_right = Value::number(7.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":188:69
                    return v_696e74656e74.read().call_authored(vec![Value::string("fade-in"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:7
                if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6869745f5f64623438643364366566306638646566.clone()), &(Value::string("audio")))); if left.truthy() { v_6661646542616e64.clone() } else { left } }; if left.truthy() { { let relational_left = v_666164654f7574.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { { let relational_left = v_666164654f7574.clone(); let relational_right = Value::number(7.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:71
                    return v_696e74656e74.read().call_authored(vec![Value::string("fade-out"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:7
                if ({ let left = { let relational_left = v_6c656674.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_6c656674.clone(); let relational_right = Value::number(7.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:34
                    return v_696e74656e74.read().call_authored(vec![Value::string("resize-left"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:7
                if ({ let left = { let relational_left = v_7269676874.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_7269676874.clone(); let relational_right = Value::number(7.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:36
                    return v_696e74656e74.read().call_authored(vec![Value::string("resize-right"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:7
                return v_696e74656e74.read().call_authored(vec![Value::string("move-region"), Value::string("replace"), Value::boolean(!(v_616c745f5f63636539653739653666646539333137.clone()).truthy())]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":194:5
            { let v_7075695f6e61746976655f3830 = v_646566.clone(); expose_method(&owner, Value::string("pointGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_66616d696c79: Value = arg(&arguments, 0);
                let v_6869745f5f39316261616264663732626433613636: Value = arg(&arguments, 1);
                let v_7365636f6e646172795f5f31343037643366373638326363356463: Value = arg(&arguments, 2);
                let v_636c69636b735f5f32643565623966353633333462313739: Value = arg(&arguments, 3);
                let v_616c745f5f63613433343261666264346538323538: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:7
                if ({ let left = Value::boolean(!(v_7365636f6e646172795f5f31343037643366373638326363356463.clone()).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3831 = v_7068617365.read(); (v_7075695f6e61746976655f3831.clone()).get() }), &(Value::string("pressed")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:7
                if ({ let left = Value::boolean(!strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo")))); if left.truthy() { Value::boolean(!strict_equal(&(v_66616d696c79.clone()), &(Value::string("automation")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:58
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:7
                if (v_7365636f6e646172795f5f31343037643366373638326363356463.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:22
                    return v_696e74656e74.read().call_authored(vec![if ({ let left = Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo")))); if !left.truthy() { v_6869745f5f39316261616264663732626433613636.clone() } else { left } }).truthy() { Value::string("point-menu") } else { Value::string("none") }, Value::string("none"), Value::boolean(!(v_616c745f5f63613433343261666264346538323538.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:7
                if ({ let left = Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo")))); if left.truthy() { { let relational_left = v_636c69636b735f5f32643565623966353633333462313739.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:46
                    return v_696e74656e74.read().call_authored(vec![Value::string("edit-tempo"), Value::string("none"), Value::boolean(!(v_616c745f5f63613433343261666264346538323538.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":199:7
                return v_696e74656e74.read().call_authored(vec![if (Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo"))))).truthy() { if (v_6869745f5f39316261616264663732626433613636.clone()).truthy() { Value::string("move-tempo") } else { Value::string("create-tempo") } } else { if (v_6869745f5f39316261616264663732626433613636.clone()).truthy() { Value::string("move-automation") } else { Value::string("create-automation") } }, Value::string("none"), Value::boolean(!(v_616c745f5f63613433343261666264346538323538.clone()).truthy())]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":201:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); expose_method(&owner, Value::string("curveGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_746f6f6c5f5f66613039386662643236616537313033: Value = arg(&arguments, 0);
                let v_657261736572: Value = arg(&arguments, 1);
                let v_6869745f5f34356164613065653563643537643364: Value = arg(&arguments, 2);
                let v_73656c6563746564: Value = arg(&arguments, 3);
                let v_706f696e7473: Value = arg(&arguments, 4);
                let v_706f7274616d656e746f: Value = arg(&arguments, 5);
                let v_73686966745f5f33323233356131613533636238656238: Value = arg(&arguments, 6);
                let v_616c745f5f62626163633966363565353836663466: Value = arg(&arguments, 7);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":203:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3833 = v_7068617365.read(); (v_7075695f6e61746976655f3833.clone()).get() }), &(Value::string("pressed")))); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3834 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f3834.clone()).get() }), &(Value::string("")))) } else { left } }; if !left.truthy() { { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("select")))); if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("eraser")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("line")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("smooth")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("pitch")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("pencil")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:92
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":205:7
                { let value = v_706f7274616d656e746f.clone(); *(v_6672656554696d65.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:7
                if ({ let left = v_657261736572.clone(); if !left.truthy() { Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("eraser")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:40
                    return v_696e74656e74.read().call_authored(vec![Value::string("curve-erase"), Value::string("none"), Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("select")))); if left.truthy() { v_706f696e7473.clone() } else { left } }; if left.truthy() { v_6869745f5f34356164613065653563643537643364.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:47
                    return v_696e74656e74.read().call_authored(vec![Value::string("curve-point"), if (v_73686966745f5f33323233356131613533636238656238.clone()).truthy() { Value::string("toggle") } else { if (v_73656c6563746564.clone()).truthy() { Value::string("keep") } else { Value::string("replace") } }, { let left = Value::boolean(!(v_616c745f5f62626163633966363565353836663466.clone()).truthy()); if left.truthy() { Value::boolean(!(v_706f7274616d656e746f.clone()).truthy()) } else { left } }]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("select"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:30
                    return v_696e74656e74.read().call_authored(vec![Value::string("curve-select"), Value::string("none"), { let left = Value::boolean(!(v_616c745f5f62626163633966363565353836663466.clone()).truthy()); if left.truthy() { Value::boolean(!(v_706f7274616d656e746f.clone()).truthy()) } else { left } }]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":209:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("line"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":209:28
                    return v_696e74656e74.read().call_authored(vec![Value::string("curve-line"), Value::string("none"), Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":210:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("smooth"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":210:30
                    return v_696e74656e74.read().call_authored(vec![Value::string("curve-smooth"), Value::string("none"), Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":211:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66613039386662643236616537313033.clone()), &(Value::string("pitch"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":211:29
                    return v_696e74656e74.read().call_authored(vec![Value::string("pitch-draw"), Value::string("none"), Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":212:7
                return v_696e74656e74.read().call_authored(vec![Value::string("curve-pencil"), Value::string("none"), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":214:5
            { let v_7075695f6e61746976655f3835 = v_646566.clone(); expose_method(&owner, Value::string("noteGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_746f6f6c5f5f34613937646432333835653338386331: Value = arg(&arguments, 0);
                let v_6869745f5f65333537333564663835623164336632: Value = arg(&arguments, 1);
                let v_746f756368: Value = arg(&arguments, 2);
                let v_6c6f6e675072657373: Value = arg(&arguments, 3);
                let v_6d6f64756c6174696f6e: Value = arg(&arguments, 4);
                let v_616c745f5f62626461353536313764343235323037: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":216:13
                let v_6e6f6e655f5f32303039373733643532383038393963 = record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":217:7
                if ({ let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3836 = v_7068617365.read(); (v_7075695f6e61746976655f3836.clone()).get() }), &(Value::string("pressed")))); if !left.truthy() { { let left = { let left = { let left = Value::boolean(!strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pointer")))); if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pencil")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("eraser")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pitch")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":217:124
                    return v_6e6f6e655f5f32303039373733643532383038393963.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:7
                if (v_6c6f6e675072657373.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:22
                    return if (Value::boolean(strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pitch"))))).truthy() { v_6e6f6e655f5f32303039373733643532383038393963.clone() } else { v_696e74656e74.read().call_authored(vec![if (v_6869745f5f65333537333564663835623164336632.clone()).truthy() { Value::string("note-lyric") } else { Value::string("note-marquee") }, Value::string("none"), Value::boolean(false)]) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pitch"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:29
                    return v_696e74656e74.read().call_authored(vec![Value::string("pitch-draw"), if (v_6869745f5f65333537333564663835623164336632.clone()).truthy() { Value::string("replace") } else { Value::string("none") }, Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":220:7
                if ({ let left = Value::boolean(!(v_746f756368.clone()).truthy()); if left.truthy() { v_6d6f64756c6174696f6e.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":220:33
                    return v_696e74656e74.read().call_authored(vec![Value::string("note-modulation"), Value::string("none"), Value::boolean(false)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":221:7
                if ({ let left = { let left = v_746f756368.clone(); if left.truthy() { Value::boolean(strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pointer")))) } else { left } }; if left.truthy() { Value::boolean(!(v_6869745f5f65333537333564663835623164336632.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":221:48
                    return v_6e6f6e655f5f32303039373733643532383038393963.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("eraser"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:30
                    return if (v_6869745f5f65333537333564663835623164336632.clone()).truthy() { v_696e74656e74.read().call_authored(vec![Value::string("erase-note"), Value::string("none"), Value::boolean(false)]) } else { v_6e6f6e655f5f32303039373733643532383038393963.clone() };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":223:7
                if ({ let left = Value::boolean(strict_equal(&(v_746f6f6c5f5f34613937646432333835653338386331.clone()), &(Value::string("pencil")))); if left.truthy() { Value::boolean(!(v_6869745f5f65333537333564663835623164336632.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":223:38
                    return v_696e74656e74.read().call_authored(vec![Value::string("draw-note"), Value::string("none"), Value::boolean(!(v_616c745f5f62626461353536313764343235323037.clone()).truthy())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":224:7
                return v_696e74656e74.read().call_authored(vec![Value::string("note-edit"), Value::string("none"), Value::boolean(!(v_616c745f5f62626461353536313764343235323037.clone()).truthy())]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":226:5
            { let v_7075695f6e61746976655f3837 = v_646566.clone(); expose_method(&owner, Value::string("phonemeGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_626f756e64617279: Value = arg(&arguments, 0);
                let v_6e6f7465: Value = arg(&arguments, 1);
                let v_6564697461626c65: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":227:7
                if (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3838 = v_7068617365.read(); (v_7075695f6e61746976655f3838.clone()).get() }), &(Value::string("pressed"))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":227:38
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":228:7
                return v_696e74656e74.read().call_authored(vec![if ({ let left = v_626f756e64617279.clone(); if left.truthy() { v_6564697461626c65.clone() } else { left } }).truthy() { Value::string("phoneme-drag") } else { if (v_6e6f7465.clone()).truthy() { Value::string("select-note") } else { Value::string("none") } }, if (v_6e6f7465.clone()).truthy() { Value::string("replace") } else { Value::string("none") }, Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":230:5
            { let v_7075695f6e61746976655f3839 = v_646566.clone(); expose_method(&owner, Value::string("rulerGesture"), {  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7365636f6e646172795f5f35393435313235303834383461343332: Value = arg(&arguments, 0);
                let v_636c69636b735f5f38396630336631396262316562623632: Value = arg(&arguments, 1);
                let v_617272616e67655f5f39323134656437303633363962333235: Value = arg(&arguments, 2);
                let v_7365676d656e74: Value = arg(&arguments, 3);
                let v_6c6f6f705374617274: Value = arg(&arguments, 4);
                let v_6c6f6f70456e64: Value = arg(&arguments, 5);
                let v_73686966745f5f64366634316533633964353333306131: Value = arg(&arguments, 6);
                let v_616c745f5f32336265633231323263653432636361: Value = arg(&arguments, 7);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":232:7
                if ({ let left = Value::boolean(!(v_7365636f6e646172795f5f35393435313235303834383461343332.clone()).truthy()); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3930 = v_7068617365.read(); (v_7075695f6e61746976655f3930.clone()).get() }), &(Value::string("pressed")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":232:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":233:7
                if (v_7365636f6e646172795f5f35393435313235303834383461343332.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":233:22
                    return v_696e74656e74.read().call_authored(vec![Value::string("ruler-menu"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":234:7
                if ({ let left = { let left = { let relational_left = v_636c69636b735f5f38396630336631396262316562623632.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!(v_617272616e67655f5f39323134656437303633363962333235.clone()).truthy()) } else { left } }; if left.truthy() { v_7365676d656e74.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":234:47
                    return v_696e74656e74.read().call_authored(vec![Value::string("edit-segment"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":235:7
                if (v_6c6f6f705374617274.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":235:22
                    return v_696e74656e74.read().call_authored(vec![Value::string("drag-loop-start"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":236:7
                if (v_6c6f6f70456e64.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":236:20
                    return v_696e74656e74.read().call_authored(vec![Value::string("drag-loop-end"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":237:7
                if (v_73686966745f5f64366634316533633964353333306131.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":237:18
                    return v_696e74656e74.read().call_authored(vec![Value::string("set-loop-start"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":238:7
                if (v_616c745f5f32336265633231323263653432636361.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":238:16
                    return v_696e74656e74.read().call_authored(vec![Value::string("set-loop-end"), Value::string("none"), Value::boolean(true)]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":239:7
                return v_696e74656e74.read().call_authored(vec![Value::string("scrub"), Value::string("none"), Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":241:5
            { let v_7075695f6e61746976655f3931 = v_646566.clone(); expose_method(&owner, Value::string("motion"), {  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_785f5f65303661303136303661333664363633: Value = arg(&arguments, 0);
                let v_795f5f33306361396539336536663232383536: Value = arg(&arguments, 1);
                let v_73686966745f5f62393232393736393062613938623438: Value = arg(&arguments, 2);
                let v_616c745f5f30313236383335363537323666383862: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":242:13
                let v_6e6f6e655f5f36336135333731666662613763376365 = record(vec![("kind", Value::string("none")), ("snap", Value::boolean(false)), ("fine", Value::number(1.0)), ("axis", Value::string("")), ("moveX", Value::boolean(false)), ("moveY", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":243:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3932 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3932.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3933 = v_7068617365.read(); (v_7075695f6e61746976655f3933.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3934 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f3934.clone()).get() }), &(Value::string("")))) } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f65303661303136303661333664363633.clone(), v_795f5f33306361396539336536663232383536.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":243:140
                    return v_6e6f6e655f5f36336135333731666662613763376365.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":244:13
                let v_6b696e645f5f36663938353939393031623932306265 = { let v_7075695f6e61746976655f3935 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f3935.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":245:13
                let v_64785f5f61306366333635383530636635343734 = Value::number((v_785f5f65303661303136303661333664363633.clone()).scalar_number() - ({ let value = (v_6f726967696e58.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":245:37
                let v_64795f5f66353666666532363861623965616230 = Value::number((v_795f5f33306361396539336536663232383536.clone()).scalar_number() - ({ let value = (v_6f726967696e59.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":246:13
                let v_6178 = if ({ let relational_left = v_64785f5f61306366333635383530636635343734.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_64785f5f61306366333635383530636635343734.clone()).scalar_number()) } else { v_64785f5f61306366333635383530636635343734.clone() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":246:43
                let v_6179 = if ({ let relational_left = v_64795f5f66353666666532363861623965616230.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_64795f5f66353666666532363861623965616230.clone()).scalar_number()) } else { v_64795f5f66353666666532363861623965616230.clone() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":247:13
                let v_6375727665 = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-point")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-select")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-line")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-pencil")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-smooth")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-erase")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("pitch-draw")))) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":249:13
                let v_7469636b65745f5f34356639616661353638333931383134 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":250:7
                if ({ let left = Value::boolean(!(v_73686966745f5f62393232393736393062613938623438.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_6375727665.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":250:29
                    { let v_7075695f6e61746976655f3936 = v_61786973.read(); state_set(v_7075695f6e61746976655f3936.clone(), Value::string(""), Value::string("reason: editing axis released")) };
                } else {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":251:12
                    if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3937 = v_61786973.read(); (v_7075695f6e61746976655f3937.clone()).get() }), &(Value::string("")))); if left.truthy() { { let left = { let relational_left = v_6178.clone(); let relational_right = Value::number(3.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if !left.truthy() { { let relational_left = v_6179.clone(); let relational_right = Value::number(3.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":251:59
                        { let v_7075695f6e61746976655f3938 = v_61786973.read(); state_set(v_7075695f6e61746976655f3938.clone(), if ({ let relational_left = v_6178.clone(); let relational_right = v_6179.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() { Value::string("x") } else { Value::string("y") }, Value::string("reason: editing axis chosen")) };
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":252:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f34356639616661353638333931383134.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":252:29
                    return v_6e6f6e655f5f36336135333731666662613763376365.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":253:13
                let v_736e61705f5f65303030353431633964666138656664 = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_616c745f5f30313236383335363537323666383862.clone()).truthy()); if left.truthy() { Value::boolean(!({ let value = (v_6672656554696d65.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-line")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-pencil")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-smooth")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("curve-erase")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("pitch-draw")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6b696e645f5f36663938353939393031623932306265.clone()), &(Value::string("phoneme-drag")))) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":254:7
                return record(vec![("kind", v_6b696e645f5f36663938353939393031623932306265.clone()), ("snap", v_736e61705f5f65303030353431633964666138656664.clone()), ("fine", if (v_73686966745f5f62393232393736393062613938623438.clone()).truthy() { Value::number(0.1) } else { Value::number(1.0) }), ("axis", { let v_7075695f6e61746976655f3939 = v_61786973.read(); (v_7075695f6e61746976655f3939.clone()).get() }), ("moveX", { let relational_left = v_6178.clone(); let relational_right = Value::number(3.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }), ("moveY", { let relational_left = v_6179.clone(); let relational_right = Value::number(3.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) })]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":256:5
            { let v_7075695f6e61746976655f313030 = v_646566.clone(); expose_method(&owner, Value::string("edgePan"), {  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_785f5f64626639313133393934663436326334: Value = arg(&arguments, 0);
                let v_7769647468: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":257:13
                let v_6e6f6e655f5f35346265336534636639346234633632 = record(vec![("position", if (number_is_finite(&(v_785f5f64626639313133393934663436326334.clone()))).truthy() { v_785f5f64626639313133393934663436326334.clone() } else { Value::number(0.0) }), ("delta", Value::number(0.0)), ("active", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":258:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f313031 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313031.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313032 = v_7068617365.read(); (v_7075695f6e61746976655f313032.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313033 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f313033.clone()).get() }), &(Value::string("note-edit")))); if left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313034 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f313034.clone()).get() }), &(Value::string("note-marquee")))) } else { left } } } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f64626639313133393934663436326334.clone(), v_7769647468.clone()])).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_7769647468.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":259:120
                    return v_6e6f6e655f5f35346265336534636639346234633632.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":260:7
                if ({ let relational_left = v_785f5f64626639313133393934663436326334.clone(); let relational_right = Value::number((v_7769647468.clone()).scalar_number() - (Value::number(16.0)).scalar_number()); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":260:27
                    return record(vec![("position", if ({ let relational_left = v_785f5f64626639313133393934663436326334.clone(); let relational_right = add(v_7769647468.clone(), Value::number(40.0)); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_785f5f64626639313133393934663436326334.clone() } else { add(v_7769647468.clone(), Value::number(40.0)) }), ("delta", if ({ let relational_left = Value::number((v_785f5f64626639313133393934663436326334.clone()).scalar_number() - (Value::number((v_7769647468.clone()).scalar_number() - (Value::number(16.0)).scalar_number())).scalar_number()); let relational_right = Value::number(40.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number((v_785f5f64626639313133393934663436326334.clone()).scalar_number() - (Value::number((v_7769647468.clone()).scalar_number() - (Value::number(16.0)).scalar_number())).scalar_number()) } else { Value::number(40.0) }), ("active", Value::boolean(true))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":261:7
                if ({ let relational_left = v_785f5f64626639313133393934663436326334.clone(); let relational_right = Value::number(16.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":261:19
                    return record(vec![("position", v_785f5f64626639313133393934663436326334.clone()), ("delta", if ({ let relational_left = Value::number((v_785f5f64626639313133393934663436326334.clone()).scalar_number() - (Value::number(16.0)).scalar_number()); let relational_right = Value::number(-(Value::number(40.0)).scalar_number()); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { Value::number((v_785f5f64626639313133393934663436326334.clone()).scalar_number() - (Value::number(16.0)).scalar_number()) } else { Value::number(-(Value::number(40.0)).scalar_number()) }), ("active", Value::boolean(true))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":262:7
                return v_6e6f6e655f5f35346265336534636639346234633632.clone();
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":264:5
            { let v_7075695f6e61746976655f313035 = v_646566.clone(); expose_method(&owner, Value::string("rulerSample"), {  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_706f736974696f6e: Value = arg(&arguments, 0);
                let v_6f757473696465: Value = arg(&arguments, 1);
                let v_656c6170736564: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":265:13
                let v_6e6f6e655f5f30343133613263393166646263373565 = record(vec![("position", Value::number(0.0)), ("delta", Value::number(0.0)), ("active", Value::boolean(false)), ("snap", Value::boolean(false)), ("accepted", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":266:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f313036 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313036.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313037 = v_7068617365.read(); (v_7075695f6e61746976655f313037.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313038 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f313038.clone()).get() }), &(Value::string("scrub")))) } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_706f736974696f6e.clone(), v_6f757473696465.clone()])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_656c6170736564.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_656c6170736564.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":267:115
                    return v_6e6f6e655f5f30343133613263393166646263373565.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":268:13
                let v_64697374616e6365 = if ({ let relational_left = v_6f757473696465.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(v_6f757473696465.clone()).scalar_number()) } else { v_6f757473696465.clone() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":269:13
                let v_7370656564 = add(Value::number(100.0), Value::number((Value::number(9.0)).scalar_number() * (v_64697374616e6365.clone()).scalar_number()));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":270:13
                let v_76656c6f63697479 = if (Value::boolean(strict_equal(&(v_6f757473696465.clone()), &(Value::number(0.0))))).truthy() { Value::number(0.0) } else { Value::number((if ({ let relational_left = v_6f757473696465.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { Value::number(1.0) }).scalar_number() * (if ({ let relational_left = v_7370656564.clone(); let relational_right = Value::number(2000.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_7370656564.clone() } else { Value::number(2000.0) }).scalar_number()) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":271:7
                return record(vec![("position", v_706f736974696f6e.clone()), ("delta", Value::number((v_76656c6f63697479.clone()).scalar_number() * (if ({ let relational_left = v_656c6170736564.clone(); let relational_right = Value::number(0.05); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_656c6170736564.clone() } else { Value::number(0.05) }).scalar_number())), ("active", Value::boolean(!strict_equal(&(v_6f757473696465.clone()), &(Value::number(0.0))))), ("snap", Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313039 = v_7068617365.read(); (v_7075695f6e61746976655f313039.clone()).get() }), &(Value::string("pressed"))))), ("accepted", Value::boolean(true))]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":276:5
            { let v_7075695f6e61746976655f313130 = v_646566.clone(); expose_method(&owner, Value::string("audioKey"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b6579: Value = arg(&arguments, 0);
                let v_73686966745f5f32396539363862663036336533376430: Value = arg(&arguments, 1);
                let v_7365636f6e646172795f5f63623039393033353534373438393736: Value = arg(&arguments, 2);
                let v_616c745f5f34343732386565383132643739366335: Value = arg(&arguments, 3);
                let v_726570656174: Value = arg(&arguments, 4);
                let v_62617369735f5f37393936393939313339343135633433: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":277:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f313131 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313131.clone()).get() } } else { left } }; if !left.truthy() { v_726570656174.clone() } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f37393936393939313339343135633433.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f313132 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313132.clone()).owner(), false) }).member("commands", false)), &(Value::string("audio")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":278:50
                    return Value::string("");
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":279:11
                let v_636f6d6d616e64 = Rc::new(std::cell::RefCell::new(Value::string("")));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":280:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Backspace")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e646172795f5f63623039393033353534373438393736.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":280:68
                    { let value = if (v_73686966745f5f32396539363862663036336533376430.clone()).truthy() { Value::string("ripple-delete") } else { Value::string("delete") }; *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":281:12
                    if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("i")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("I")))) } else { left } }; if left.truthy() { v_7365636f6e646172795f5f63623039393033353534373438393736.clone() } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":281:59
                        { let value = Value::string("split"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":282:12
                        if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("a")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("A")))) } else { left } }; if left.truthy() { v_7365636f6e646172795f5f63623039393033353534373438393736.clone() } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":282:59
                            { let value = Value::string("select-all"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":283:12
                            if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("j")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("J")))) } else { left } }; if left.truthy() { v_7365636f6e646172795f5f63623039393033353534373438393736.clone() } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":283:59
                                { let value = Value::string("join"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":284:12
                                if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("r")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("R")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e646172795f5f63623039393033353534373438393736.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!(v_616c745f5f34343732386565383132643739366335.clone()).truthy()) } else { left } }).truthy() {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":284:68
                                    { let value = Value::string("range-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":285:12
                                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("v")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("V")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e646172795f5f63623039393033353534373438393736.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!(v_616c745f5f34343732386565383132643739366335.clone()).truthy()) } else { left } }).truthy() {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":285:68
                                        { let value = Value::string("select-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":286:12
                                        if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape"))))).truthy() {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":286:34
                                            { let value = Value::string("escape"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":287:7
                if (Value::boolean(strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string(""))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":287:27
                    return Value::string("");
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":288:13
                let v_7469636b65745f5f39613632356430636666396639363262 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":289:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":290:7
                return if ({ let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39613632356430636666396639363262.clone()]); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313133 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313133.clone()).get() }).truthy()) } else { left } }).truthy() { { let value = (v_636f6d6d616e64.clone()).borrow().clone(); value } } else { Value::string("") };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":292:5
            { let v_7075695f6e61746976655f313134 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7068617365 = v_7068617365.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":293:7
                if ({ let left = Value::boolean(!strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313135 = v_7068617365.read(); (v_7075695f6e61746976655f313135.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":293:61
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":294:7
                { let v_7075695f6e61746976655f313136 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f313136.clone()) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":295:7
                v_63616e63656c.read().call_authored(vec![]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":297:5
            { let v_7075695f6e61746976655f313137 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":298:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f32353836313565636230633237343532: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":299:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":299:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":300:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":301:13
                let v_7469636b65745f5f62363163363136303965306137363363 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":302:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":303:7
                { let value = v_68616e646c655f5f32353836313565636230633237343532.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":304:13
                let v_70726f7073 = { let v_7075695f6e61746976655f313138 = v_68616e646c655f5f32353836313565636230633237343532.clone(); read_props(&(v_7075695f6e61746976655f313138.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":305:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":306:13
                let v_6f776e65725f5f35616334363864353636356332393263 = { let v_7075695f6e61746976655f313139 = v_706f696e746572.read(); (v_7075695f6e61746976655f313139.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":307:13
                let v_726574697265645f5f37373539376365633334303633353038 = { let v_7075695f6e61746976655f313230 = v_6c65617365.read(); (v_7075695f6e61746976655f313230.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":308:13
                let v_62617369735f5f39303665643335346139333531343235 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":309:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313231 = v_7068617365.read(); (v_7075695f6e61746976655f313231.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":310:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":311:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":312:9
                    { let v_7075695f6e61746976655f313232 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313232.clone(), Value::string("idle"), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":313:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":313:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":314:9
                    { let v_7075695f6e61746976655f313233 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f313233.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":315:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":315:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":316:9
                    { let v_7075695f6e61746976655f313234 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f313234.clone(), Value::number(0.0), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":317:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":317:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":318:9
                    { let v_7075695f6e61746976655f313235 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f313235.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":319:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":319:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":320:9
                    { let v_7075695f6e61746976655f313236 = v_61786973.read(); state_set(v_7075695f6e61746976655f313236.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":321:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":321:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":323:7
                { let v_7075695f6e61746976655f313237 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313237.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: editing policy")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":324:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":324:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":325:7
                { let v_7075695f6e61746976655f313238 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313238.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: editing surface label")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":326:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":326:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":327:7
                { let v_7075695f6e61746976655f313239 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313239.clone()).owner(), { let v_7075695f6e61746976655f313330 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313330.clone()).get() }) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":328:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":328:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":329:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":329:25
                    { let v_7075695f6e61746976655f313331 = v_68616e646c655f5f32353836313565636230633237343532.clone(); emit(&(v_7075695f6e61746976655f313331.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f35616334363864353636356332393263.clone()), ("revision", v_62617369735f5f39303665643335346139333531343235.clone()), ("lease", v_726574697265645f5f37373539376365633334303633353038.clone())])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":330:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f62363163363136303965306137363363.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":330:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":331:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":332:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":332:21
                    { let v_7075695f6e61746976655f313332 = v_68616e646c655f5f32353836313565636230633237343532.clone(); request_update(&(v_7075695f6e61746976655f313332.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":334:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":335:13
                let v_68616e646c655f5f38393832306537623934393935383034 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":336:13
                let v_6163746976655f5f38323337656264306436306332396437 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313333 = v_7068617365.read(); (v_7075695f6e61746976655f313333.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":337:13
                let v_6f776e65725f5f62373366316332356631393132336263 = { let v_7075695f6e61746976655f313334 = v_706f696e746572.read(); (v_7075695f6e61746976655f313334.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":338:13
                let v_726574697265645f5f33393534306363356335303663353837 = { let v_7075695f6e61746976655f313335 = v_6c65617365.read(); (v_7075695f6e61746976655f313335.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":339:13
                let v_62617369735f5f66386463336337356431666430363534 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":340:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":341:13
                let v_7469636b65745f5f39333839326330333765626331383464 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":342:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":343:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":344:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":345:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":346:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":346:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":347:7
                { let v_7075695f6e61746976655f313336 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313336.clone(), Value::string("idle"), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":348:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39333839326330333765626331383464.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":348:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":349:7
                { let v_7075695f6e61746976655f313337 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f313337.clone(), Value::string(""), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":350:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39333839326330333765626331383464.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":350:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":351:7
                { let v_7075695f6e61746976655f313338 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f313338.clone(), Value::number(0.0), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":352:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39333839326330333765626331383464.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":352:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":353:7
                { let v_7075695f6e61746976655f313339 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f313339.clone(), Value::string(""), Value::string("reason: editing operation detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":354:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39333839326330333765626331383464.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":354:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":355:7
                { let v_7075695f6e61746976655f313430 = v_61786973.read(); state_set(v_7075695f6e61746976655f313430.clone(), Value::string(""), Value::string("reason: editing axis detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":356:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f39333839326330333765626331383464.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":356:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":357:7
                if ({ let left = v_6163746976655f5f38323337656264306436306332396437.clone(); if left.truthy() { v_68616e646c655f5f38393832306537623934393935383034.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":357:29
                    { let v_7075695f6e61746976655f313431 = v_68616e646c655f5f38393832306537623934393935383034.clone(); emit(&(v_7075695f6e61746976655f313431.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f62373366316332356631393132336263.clone()), ("revision", v_62617369735f5f66386463336337356431666430363534.clone()), ("lease", v_726574697265645f5f33393534306363356335303663353837.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":359:5
            { let v_7075695f6e61746976655f313432 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:5
            { let v_7075695f6e61746976655f313433 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f37663030653336373064323136663532: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f37663030653336373064323136663532.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":361:5
            { let v_7075695f6e61746976655f313434 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("threshold"), Value::string("commands")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":362:5
            { let v_7075695f6e61746976655f313435 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":362:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":363:5
            { let v_7075695f6e61746976655f313436 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":363:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":364:5
            return { let callable={ let pui_native_147={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":364:26
                return { let v_7075695f6e61746976655f313438 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_147.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_147.call)(arg(&arguments,0))})} }; match callable {
      Value::NativeFunction{render,..} => Value::Renderer(render),
      Value::Renderer(render) => Value::Renderer(render),
      Value::Function(callback) => Value::Renderer(Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
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
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| replace_props(&self.owner, raw))?; self.drive(window, cx) }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_focus_observation_needed(&self.owner) && !native::gpui_native_move::move_owner_has_binding(&self.owner) { self._window_activation.take(); return; }
          let id = window.window_handle().window_id();
          if self._window_activation.as_ref().is_some_and(|(bound, _)| *bound == id) { return; }
          self._window_activation.take();
          let owner = self.owner.clone();
          let subscription = cx.observe_window_activation(window, move |view, window, cx| {
              native_host_callback(&owner, "window-activation", || {
                  if owner.borrow().window_id == Some(window.window_handle().window_id()) && !window.is_window_active() {
                      native_window_blur(&owner);
                      view.drive(window, cx).unwrap_or_else(|error| throw_value(error));
                  }
              });
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
          catch_authored(|| {
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); cx.notify(); return; }
          modules_drive(&self.owner, &self.focus, window, cx);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); cx.notify(); return; }
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          drive_queued_focus(&self.owner, window, cx);
          let dirty = std::mem::take(&mut self.owner.borrow_mut().dirty); if dirty { cx.notify(); }
          })
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
