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
pub struct GeneratedData2 {
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
#[serde(untagged)]
pub enum GeneratedData4 {
    Alternative0(f64),
    Alternative1(f64),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData3 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "x")]
    pub field_1: f64,
    #[serde(rename = "y")]
    pub field_2: GeneratedData4,
    #[serde(rename = "scale")]
    pub field_3: f64,
    #[serde(rename = "exponent")]
    pub field_4: f64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData1 {
    Alternative0(GeneratedData2),
    Alternative1(GeneratedData3),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData5 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "selection")]
    pub field_1: String,
    #[serde(rename = "snap")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData7 {
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
#[serde(untagged)]
pub enum GeneratedData9 {
    Alternative0(f64),
    Alternative1(f64),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData8 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "snap")]
    pub field_1: bool,
    #[serde(rename = "fine")]
    pub field_2: GeneratedData9,
    #[serde(rename = "axis")]
    pub field_3: String,
    #[serde(rename = "moveX")]
    pub field_4: bool,
    #[serde(rename = "moveY")]
    pub field_5: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData6 {
    Alternative0(GeneratedData7),
    Alternative1(GeneratedData8),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData12 {
    Alternative0(f64),
    Alternative1(f64),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData11 {
    #[serde(rename = "position")]
    pub field_0: GeneratedData12,
    #[serde(rename = "delta")]
    pub field_1: f64,
    #[serde(rename = "active")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData14 {
    Alternative0(f64),
    Alternative1(f64),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData13 {
    #[serde(rename = "position")]
    pub field_0: f64,
    #[serde(rename = "delta")]
    pub field_1: GeneratedData14,
    #[serde(rename = "active")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData15 {
    #[serde(rename = "position")]
    pub field_0: f64,
    #[serde(rename = "delta")]
    pub field_1: f64,
    #[serde(rename = "active")]
    pub field_2: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData10 {
    Alternative0(GeneratedData11),
    Alternative1(GeneratedData13),
    Alternative2(GeneratedData15),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData16 {
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
    pub fn method_10(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_11(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_12(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_13(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_14(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_15(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_16(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_17(&self, argument_0: String, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument")), Value::json(serde_json::to_value(argument_6).expect("portable argument")), Value::json(serde_json::to_value(argument_7).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("navigate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("navigate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))) || (result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| (field.as_f64() == Some(0.0_f64) || field.as_f64().is_some_and(f64::is_finite))) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_18(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("wheel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("wheel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))) || (result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| (field.as_f64() == Some(0.0_f64) || field.as_f64().is_some_and(f64::is_finite))) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_19(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("pinch").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("pinch", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))) || (result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| (field.as_f64() == Some(0.0_f64) || field.as_f64().is_some_and(f64::is_finite))) && result.get("scale").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("exponent").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite))))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_20(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: f64, argument_6: f64, argument_7: f64, argument_8: f64, argument_9: f64, argument_10: bool, argument_11: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_21(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_22(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_23(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_24(&self, argument_0: bool, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_25(&self, argument_0: bool, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData5, Value> {
        let outcome = catch_authored(|| {
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
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_26(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData6, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("motion").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("motion", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()) && result.get("fine").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("axis").map_or(false, |field| field.is_string()) && result.get("moveX").map_or(false, |field| field.is_boolean()) && result.get("moveY").map_or(false, |field| field.is_boolean())) || (result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("snap").map_or(false, |field| field.is_boolean()) && result.get("fine").map_or(false, |field| (field.as_f64() == Some(0.1_f64) || field.as_f64() == Some(1.0_f64))) && result.get("axis").map_or(false, |field| field.is_string()) && result.get("moveX").map_or(false, |field| field.is_boolean()) && result.get("moveY").map_or(false, |field| field.is_boolean())))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_27(&self, argument_0: f64, argument_1: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData10, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("edgePan").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("edgePan", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(((result.is_object() && result.get("position").map_or(false, |field| (field.as_f64().is_some_and(f64::is_finite) || field.as_f64() == Some(0.0_f64))) && result.get("delta").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("active").map_or(false, |field| field.is_boolean())) || (result.is_object() && result.get("position").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("delta").map_or(false, |field| (field.as_f64().is_some_and(f64::is_finite) || field.as_f64() == Some(40.0_f64))) && result.get("active").map_or(false, |field| field.is_boolean())) || (result.is_object() && result.get("position").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("delta").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("active").map_or(false, |field| field.is_boolean())))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_28(&self, argument_0: f64, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<GeneratedData16, Value> {
        let outcome = catch_authored(|| {
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
            let v_706f696e746572: LexicalBinding<Value> = lexical_binding();
            let v_6c65617365: LexicalBinding<Value> = lexical_binding();
            let v_6f7065726174696f6e: LexicalBinding<Value> = lexical_binding();
            let v_61786973: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6f726967696e58: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6f726967696e59: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6e6578744c65617365: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6672656554696d65: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_63616e63656c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_6163746976617465: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_6e61766967617465: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, f64, f64, f64, bool, bool, bool, bool) -> Value>> = lexical_binding();
            let v_696e74656e74: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, Value, bool) -> Value>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(bool) -> Value>> = lexical_binding();
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("threshold", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("commandMode", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":22:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":23:11
            v_706f696e746572.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("pointer"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":24:11
            v_6c65617365.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "discrete", Value::string("lease"), Value::number(0.0_f64), Value::Undefined) });
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
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":45:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":46:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":47:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":48:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":49:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":50:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":52:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":53:9
            v_6f726967696e58.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":54:9
            v_6f726967696e59.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":55:9
            v_6e6578744c65617365.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":56:9
            v_6672656554696d65.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":60:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:13
                let v_6f776e65725f5f64663364313333643931313764343166 = { let v_7075695f6e61746976655f3233 = v_706f696e746572.read(); (v_7075695f6e61746976655f3233.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":62:13
                let v_726574697265645f5f35616138636131663663366362353437: f64 = ({ let v_7075695f6e61746976655f3234 = v_6c65617365.read(); Value::number(((v_7075695f6e61746976655f3234.clone()).get()).num()) }).num();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":63:13
                let v_62617369735f5f35323332616333306165323863666637 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":64:13
                let v_6163746976655f5f34336230336364386466633235396163: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3237 = v_7068617365.read(); (v_7075695f6e61746976655f3237.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":65:13
                let v_7469636b65745f5f30613035396363663834363864643136: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":66:7
                Value::boolean({ let value=false; *(v_6672656554696d65.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:7
                { let v_7075695f6e61746976655f3238 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3238.clone(), Value::string("idle"), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":68:7
                if !({ let pui_native_29={ let pui_native_31=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_31) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_31} }; if (pui_native_29) {{ let pui_native_35={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_36=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_35==pui_native_36) }} else {pui_native_29} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":68:29
                    return v_6163746976655f5f34336230336364386466633235396163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":69:7
                { let v_7075695f6e61746976655f3337 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3337.clone(), Value::string(""), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":70:7
                if !({ let pui_native_38={ let pui_native_40=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_40) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_40} }; if (pui_native_38) {{ let pui_native_44={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_45=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_44==pui_native_45) }} else {pui_native_38} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":70:29
                    return v_6163746976655f5f34336230336364386466633235396163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:7
                { let v_7075695f6e61746976655f3436 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3436.clone(), Value::number(0.0_f64), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":72:7
                if !({ let pui_native_47={ let pui_native_49=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_49) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_49} }; if (pui_native_47) {{ let pui_native_53={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_54=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_53==pui_native_54) }} else {pui_native_47} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":72:29
                    return v_6163746976655f5f34336230336364386466633235396163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:7
                { let v_7075695f6e61746976655f3535 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f3535.clone(), Value::string(""), Value::string("reason: editing operation retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":74:7
                if !({ let pui_native_56={ let pui_native_58=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_58) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_58} }; if (pui_native_56) {{ let pui_native_62={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_63=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_62==pui_native_63) }} else {pui_native_56} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":74:29
                    return v_6163746976655f5f34336230336364386466633235396163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:7
                { let v_7075695f6e61746976655f3634 = v_61786973.read(); state_set(v_7075695f6e61746976655f3634.clone(), Value::string(""), Value::string("reason: editing axis retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:7
                if !({ let pui_native_65={ let pui_native_67=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_67) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_67} }; if (pui_native_65) {{ let pui_native_71={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_72=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_71==pui_native_72) }} else {pui_native_65} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:29
                    return v_6163746976655f5f34336230336364386466633235396163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:7
                if ({ let left = Value::boolean(v_6163746976655f5f34336230336364386466633235396163.clone()); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:26
                    { let v_7075695f6e61746976655f3733 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3733.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f64663364313333643931313764343166.clone()), ("revision", v_62617369735f5f35323332616333306165323863666637.clone()), ("lease", Value::number(v_726574697265645f5f35616138636131663663366362353437.clone()))])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":78:7
                if ({ let left = Value::boolean({ let pui_native_74={ let pui_native_76=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_76) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_76} }; if (pui_native_74) {{ let pui_native_80={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_81=v_7469636b65745f5f30613035396363663834363864643136.clone(); (pui_native_80==pui_native_81) }} else {pui_native_74} }); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":78:35
                    { let v_7075695f6e61746976655f3832 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3832.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":79:7
                return v_6163746976655f5f34336230336364386466633235396163.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:5
            { let v_7075695f6e61746976655f3833 = v_646566.clone(); expose_method(&owner, Value::string("begin"), ({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_6e6578744c65617365 = v_6e6578744c65617365.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_6f776e65725f5f30643663326434653938326630613930: Value = argument_0;
                let v_785f5f34303635316334386261393266303239: f64 = argument_1;
                let v_795f5f38633366386435643837616362353633: f64 = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":82:7
                if { let pui_native_84={ let pui_native_86={ let pui_native_88={ let pui_native_90={ let pui_native_92={ let pui_native_94={ let pui_native_96=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_96) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_96} }; if !(pui_native_94) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_94} }; if !(pui_native_92) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_92} }; if !(pui_native_90) {({ let v_7075695f6e61746976655f3938 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3938.clone()).get()).bool()) }).bool()} else {pui_native_90} }; if !(pui_native_88) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313031 = v_7068617365.read(); (v_7075695f6e61746976655f313031.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_88} }; if !(pui_native_86) {(Value::boolean(strict_equal(&(v_6f776e65725f5f30643663326434653938326630613930.clone()), &(Value::string(""))))).bool()} else {pui_native_86} }; if !(pui_native_84) {!({ let pui_native_104=(v_785f5f34303635316334386261393266303239.clone()).is_finite(); if (pui_native_104) {(v_795f5f38633366386435643837616362353633.clone()).is_finite()} else {pui_native_104} })} else {pui_native_84} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":83:43
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":84:13
                let v_7469636b65745f5f62623264363863613466623830393230: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":85:7
                Value::number({ let value=v_785f5f34303635316334386261393266303239.clone(); *(v_6f726967696e58.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":86:7
                Value::number({ let value=v_795f5f38633366386435643837616362353633.clone(); *(v_6f726967696e59.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":87:7
                Value::boolean({ let value=false; *(v_6672656554696d65.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":88:7
                Value::number({ let value={ let pui_native_106:f64={ let value=(v_6e6578744c65617365.read()).borrow().clone(); value }; let pui_native_107:f64=1.0_f64; pui_native_106 + pui_native_107 }; *(v_6e6578744c65617365.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:7
                { let v_7075695f6e61746976655f313038 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f313038.clone(), Value::string(""), Value::string("reason: editing operation acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:7
                if !({ let pui_native_109={ let pui_native_111=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_111) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_111} }; if (pui_native_109) {{ let pui_native_115={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_116=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_115==pui_native_116) }} else {pui_native_109} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":91:7
                { let v_7075695f6e61746976655f313137 = v_61786973.read(); state_set(v_7075695f6e61746976655f313137.clone(), Value::string(""), Value::string("reason: editing axis acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":92:7
                if !({ let pui_native_118={ let pui_native_120=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_120) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_120} }; if (pui_native_118) {{ let pui_native_124={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_125=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_124==pui_native_125) }} else {pui_native_118} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":92:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":93:7
                { let v_7075695f6e61746976655f313236 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f313236.clone(), Value::number({ let value=(v_6e6578744c65617365.read()).borrow().clone(); value }), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":94:7
                if !({ let pui_native_127={ let pui_native_129=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_129) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_129} }; if (pui_native_127) {{ let pui_native_133={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_134=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_133==pui_native_134) }} else {pui_native_127} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":94:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:7
                { let v_7075695f6e61746976655f313335 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f313335.clone(), v_6f776e65725f5f30643663326434653938326630613930.clone(), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:7
                if !({ let pui_native_136={ let pui_native_138=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_138) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_138} }; if (pui_native_136) {{ let pui_native_142={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_143=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_142==pui_native_143) }} else {pui_native_136} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":97:7
                { let v_7075695f6e61746976655f313434 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313434.clone(), Value::string("pressed"), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:7
                if !({ let pui_native_145={ let pui_native_147=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_147) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_147} }; if (pui_native_145) {{ let pui_native_151={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_152=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_151==pui_native_152) }} else {pui_native_145} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:7
                { let v_7075695f6e61746976655f313533 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313533.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:7
                if !({ let pui_native_154={ let pui_native_156=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_156) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_156} }; if (pui_native_154) {{ let pui_native_160={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_161=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_160==pui_native_161) }} else {pui_native_154} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":101:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":101:16
                    { let v_7075695f6e61746976655f313632 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f313632.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":102:7
                return { let pui_native_163={ let pui_native_165=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_165) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_165} }; if (pui_native_163) {{ let pui_native_169={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_170=v_7469636b65745f5f62623264363863613466623830393230.clone(); (pui_native_169==pui_native_170) }} else {pui_native_163} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":104:11
            let v_73616d706c65 = {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_6f776e65725f5f66376139366331666635633333326239: Value = argument_0;
                let v_785f5f62386138636566353236613561366230: f64 = argument_1;
                let v_795f5f33333334376664356537663566326337: f64 = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:7
                if { let pui_native_171={ let pui_native_173={ let pui_native_175={ let pui_native_177={ let pui_native_179={ let pui_native_181=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_181) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_181} }; if !(pui_native_179) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_179} }; if !(pui_native_177) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_177} }; if !(pui_native_175) {({ let v_7075695f6e61746976655f313833 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313833.clone()).get()).bool()) }).bool()} else {pui_native_175} }; if !(pui_native_173) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313836 = v_7068617365.read(); (v_7075695f6e61746976655f313836.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_173} }; if !(pui_native_171) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313839 = v_706f696e746572.read(); (v_7075695f6e61746976655f313839.clone()).get() }), &(v_6f776e65725f5f66376139366331666635633333326239.clone())))).bool()} else {pui_native_171} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:124
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:7
                if !({ let pui_native_190=(v_785f5f62386138636566353236613561366230.clone()).is_finite(); if (pui_native_190) {(v_795f5f33333334376664356537663566326337.clone()).is_finite()} else {pui_native_190} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:28
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:38
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":107:13
                let v_7469636b65745f5f62373037613163653363333637643236: f64 = { let value=(v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":108:13
                let v_726177: f64 = (({ let v_7075695f6e61746976655f313934 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313934.clone()).owner(), false) }).member("threshold", false)).num();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":109:13
                let v_7468726573686f6c64: f64 = if { let pui_native_195={ let pui_native_197:f64=v_726177.clone(); let pui_native_198:f64=0.0_f64; pui_native_197 > pui_native_198 }; if (pui_native_195) {(v_726177.clone()).is_finite()} else {pui_native_195} } {v_726177.clone()} else {0.0_f64};
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":110:13
                let v_64785f5f33653062396439623337396436626235: f64 = { let pui_native_199:f64=v_785f5f62386138636566353236613561366230.clone(); let pui_native_200:f64={ let value=(v_6f726967696e58.read()).borrow().clone(); value }; pui_native_199 - pui_native_200 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:13
                let v_64795f5f63636436333132383261633434386233: f64 = { let pui_native_201:f64=v_795f5f33333334376664356537663566326337.clone(); let pui_native_202:f64={ let value=(v_6f726967696e59.read()).borrow().clone(); value }; pui_native_201 - pui_native_202 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:7
                if { let pui_native_203=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f323037 = v_7068617365.read(); (v_7075695f6e61746976655f323037.clone()).get() }), &(Value::string("pressed"))))).bool(); if (pui_native_203) {{ let pui_native_208:f64={ let pui_native_210:f64={ let pui_native_212:f64=v_64785f5f33653062396439623337396436626235.clone(); let pui_native_213:f64=v_64785f5f33653062396439623337396436626235.clone(); pui_native_212 * pui_native_213 }; let pui_native_211:f64={ let pui_native_214:f64=v_64795f5f63636436333132383261633434386233.clone(); let pui_native_215:f64=v_64795f5f63636436333132383261633434386233.clone(); pui_native_214 * pui_native_215 }; pui_native_210 + pui_native_211 }; let pui_native_209:f64={ let pui_native_216:f64=v_7468726573686f6c64.clone(); let pui_native_217:f64=v_7468726573686f6c64.clone(); pui_native_216 * pui_native_217 }; pui_native_208 < pui_native_209 }} else {pui_native_203} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:83
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:7
                { let v_7075695f6e61746976655f323138 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323138.clone(), Value::string("dragging"), Value::string("reason: editing movement accepted")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:7
                if !({ let pui_native_219={ let pui_native_221=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_221) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_221} }; if (pui_native_219) {{ let pui_native_225={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_226=v_7469636b65745f5f62373037613163653363333637643236.clone(); (pui_native_225==pui_native_226) }} else {pui_native_219} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":115:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":115:16
                    { let v_7075695f6e61746976655f323237 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f323237.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":116:7
                return { let pui_native_228={ let pui_native_230=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_230) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_230} }; if (pui_native_228) {{ let pui_native_234={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_235=v_7469636b65745f5f62373037613163653363333637643236.clone(); (pui_native_234==pui_native_235) }} else {pui_native_228} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":118:5
            { let v_7075695f6e61746976655f323336 = v_646566.clone(); expose_method(&owner, Value::string("sample"), (v_73616d706c65.clone()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:5
            { let v_7075695f6e61746976655f323337 = v_646566.clone(); expose_method(&owner, Value::string("finish"), ({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn(Value, f64, f64)->bool>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64| -> bool {
                let v_6f776e65725f5f37666232653538663035326633396431: Value = argument_0;
                let v_785f5f31363538303563383934373266306566: f64 = argument_1;
                let v_795f5f35663164643335366666616135326434: f64 = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":120:7
                if { let pui_native_238={ let pui_native_240={ let pui_native_242={ let pui_native_244={ let pui_native_246={ let pui_native_248=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_248) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_248} }; if !(pui_native_246) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_246} }; if !(pui_native_244) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_244} }; if !(pui_native_242) {({ let v_7075695f6e61746976655f323530 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323530.clone()).get()).bool()) }).bool()} else {pui_native_242} }; if !(pui_native_240) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f323533 = v_7068617365.read(); (v_7075695f6e61746976655f323533.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_240} }; if !(pui_native_238) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f323536 = v_706f696e746572.read(); (v_7075695f6e61746976655f323536.clone()).get() }), &(v_6f776e65725f5f37666232653538663035326633396431.clone())))).bool()} else {pui_native_238} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":120:124
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":121:7
                if !({ let pui_native_257=(v_785f5f31363538303563383934373266306566.clone()).is_finite(); if (pui_native_257) {(v_795f5f35663164643335366666616135326434.clone()).is_finite()} else {pui_native_257} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":121:28
                    Value::boolean((v_63616e63656c.read().call)());
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":121:38
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":122:13
                let v_7469636b65745f5f35363730363832663064643631343365: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:7
                { let v_7075695f6e61746976655f323539 = v_7068617365.read(); state_set(v_7075695f6e61746976655f323539.clone(), Value::string("idle"), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:7
                if !({ let pui_native_260={ let pui_native_262=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_262) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_262} }; if (pui_native_260) {{ let pui_native_266={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_267=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_266==pui_native_267) }} else {pui_native_260} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:7
                { let v_7075695f6e61746976655f323638 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f323638.clone(), Value::string(""), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:7
                if !({ let pui_native_269={ let pui_native_271=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_271) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_271} }; if (pui_native_269) {{ let pui_native_275={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_276=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_275==pui_native_276) }} else {pui_native_269} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:7
                { let v_7075695f6e61746976655f323737 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f323737.clone(), Value::number(0.0_f64), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":128:7
                if !({ let pui_native_278={ let pui_native_280=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_280) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_280} }; if (pui_native_278) {{ let pui_native_284={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_285=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_284==pui_native_285) }} else {pui_native_278} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":128:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":129:7
                { let v_7075695f6e61746976655f323836 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f323836.clone(), Value::string(""), Value::string("reason: editing operation completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:7
                if !({ let pui_native_287={ let pui_native_289=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_289) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_289} }; if (pui_native_287) {{ let pui_native_293={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_294=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_293==pui_native_294) }} else {pui_native_287} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":131:7
                { let v_7075695f6e61746976655f323935 = v_61786973.read(); state_set(v_7075695f6e61746976655f323935.clone(), Value::string(""), Value::string("reason: editing axis completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":132:7
                if !({ let pui_native_296={ let pui_native_298=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_298) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_298} }; if (pui_native_296) {{ let pui_native_302={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_303=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_302==pui_native_303) }} else {pui_native_296} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":132:29
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":133:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":133:16
                    { let v_7075695f6e61746976655f333034 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f333034.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":134:7
                return { let pui_native_305={ let pui_native_307=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_307) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_307} }; if (pui_native_305) {{ let pui_native_311={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_312=v_7469636b65745f5f35363730363832663064643631343365.clone(); (pui_native_311==pui_native_312) }} else {pui_native_305} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":136:5
            { let v_7075695f6e61746976655f333133 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:5
            { let v_7075695f6e61746976655f333134 = v_646566.clone(); expose_method(&owner, Value::string("cancelPointer"), ({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6f776e65725f5f36626234346264666563383762343634: Value = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:7
                if { let pui_native_315=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f333139 = v_7068617365.read(); (v_7075695f6e61746976655f333139.clone()).get() }), &(Value::string("idle"))))).bool(); if !(pui_native_315) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f333232 = v_706f696e746572.read(); (v_7075695f6e61746976655f333232.clone()).get() }), &(v_6f776e65725f5f36626234346264666563383762343634.clone())))).bool()} else {pui_native_315} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:62
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":139:7
                return (v_63616e63656c.read().call)();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63616e63656c = v_63616e63656c.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:7
                if { let pui_native_323={ let pui_native_325={ let pui_native_327={ let pui_native_329=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_329) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_329} }; if !(pui_native_327) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_327} }; if !(pui_native_325) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_325} }; if !(pui_native_323) {({ let v_7075695f6e61746976655f333331 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333331.clone()).get()).bool()) }).bool()} else {pui_native_323} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:71
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":143:13
                let v_7469636b65745f5f65396664353138613532613162316664: f64 = { let pui_native_332:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_333:f64=1.0_f64; pui_native_332 + pui_native_333 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":144:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":145:7
                if { let pui_native_334={ let pui_native_336={ let pui_native_338=!({ let pui_native_340={ let pui_native_342=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_342) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_342} }; if (pui_native_340) {{ let pui_native_346={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_347=v_7469636b65745f5f65396664353138613532613162316664.clone(); (pui_native_346==pui_native_347) }} else {pui_native_340} }); if !(pui_native_338) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_338} }; if !(pui_native_336) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_336} }; if !(pui_native_334) {({ let v_7075695f6e61746976655f333438 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333438.clone()).get()).bool()) }).bool()} else {pui_native_334} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":145:71
                    return false;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":146:7
                { let v_7075695f6e61746976655f333439 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f333439.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":147:7
                return { let pui_native_350={ let pui_native_352={ let pui_native_354={ let pui_native_356={ let pui_native_358=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_358) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_358} }; if (pui_native_356) {{ let pui_native_362={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_363=v_7469636b65745f5f65396664353138613532613162316664.clone(); (pui_native_362==pui_native_363) }} else {pui_native_356} }; if (pui_native_354) {!({ let value=(v_73796e63696e67.read()).borrow().clone(); value })} else {pui_native_354} }; if (pui_native_352) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_352} }; if (pui_native_350) {!(({ let v_7075695f6e61746976655f333634 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333634.clone()).get()).bool()) }).bool())} else {pui_native_350} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":149:5
            { let v_7075695f6e61746976655f333635 = v_646566.clone(); expose_method(&owner, Value::string("activate"), (v_6163746976617465.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":150:5
            { let v_7075695f6e61746976655f333636 = v_646566.clone(); expose_method(&owner, Value::string("doubleActivate"), ({  let v_6163746976617465 = v_6163746976617465.clone(); let call:Rc<dyn Fn(f64)->bool>=Rc::new(move |argument_0: f64| -> bool {
                let v_636c69636b735f5f66393165376266336366616561393664: f64 = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":150:61
                return { let pui_native_367={ let pui_native_369=(v_636c69636b735f5f66393165376266336366616561393664.clone()).is_finite(); if (pui_native_369) {{ let pui_native_371:f64=v_636c69636b735f5f66393165376266336366616561393664.clone(); let pui_native_372:f64=2.0_f64; pui_native_371 >= pui_native_372 }} else {pui_native_369} }; if (pui_native_367) {(v_6163746976617465.read().call)()} else {pui_native_367} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:11
            v_6e61766967617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn(Value, f64, f64, f64, bool, bool, bool, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool| -> Value {
                let v_736f75726365: Value = argument_0;
                let v_64785f5f30316638323735666436613634353366: f64 = argument_1;
                let v_64795f5f66653066333265616633366630633261: f64 = argument_2;
                let v_7363616c655f5f35633438393132666536366335393837: f64 = argument_3;
                let v_636f6e74726f6c5f5f30613636643035636665643764636434: bool = argument_4;
                let v_73686966745f5f61313536343834336232343731376437: bool = argument_5;
                let v_616c745f5f33333733643434363136623862303339: bool = argument_6;
                let v_617272616e67655f5f64636365366436323963373035326634: bool = argument_7;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":154:13
                let v_6e6f6e655f5f35633438646231656563643966633736 = record(vec![("kind", Value::string("none")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(1.0_f64)), ("exponent", Value::number(0.0_f64))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":155:7
                if { let pui_native_373={ let pui_native_375={ let pui_native_377={ let pui_native_379={ let pui_native_381=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_381) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_381} }; if !(pui_native_379) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_379} }; if !(pui_native_377) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_377} }; if !(pui_native_375) {({ let v_7075695f6e61746976655f333833 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333833.clone()).get()).bool()) }).bool()} else {pui_native_375} }; if !(pui_native_373) {!({ let pui_native_384=(v_64785f5f30316638323735666436613634353366.clone()).is_finite(); if (pui_native_384) {(v_64795f5f66653066333265616633366630633261.clone()).is_finite()} else {pui_native_384} })} else {pui_native_373} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":155:90
                    return v_6e6f6e655f5f35633438646231656563643966633736.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("zoom-in"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:33
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(1.25_f64)), ("exponent", Value::number(1.0_f64))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("zoom-out"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:34
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(0.8_f64)), ("exponent", Value::number(1.0_f64))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":160:7
                if (Value::boolean(strict_equal(&(v_736f75726365.clone()), &(Value::string("pinch"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":160:31
                    return if (Value::boolean({ let pui_native_392=(v_7363616c655f5f35633438393132666536366335393837.clone()).is_finite(); if (pui_native_392) {{ let pui_native_394:f64=v_7363616c655f5f35633438393132666536366335393837.clone(); let pui_native_395:f64=0.0_f64; pui_native_394 > pui_native_395 }} else {pui_native_392} })).truthy() { record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(v_7363616c655f5f35633438393132666536366335393837.clone())), ("exponent", Value::number(1.0_f64))]) } else { v_6e6f6e655f5f35633438646231656563643966633736.clone() };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:7
                if (Value::boolean(!strict_equal(&(v_736f75726365.clone()), &(Value::string("wheel"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:31
                    return v_6e6f6e655f5f35633438646231656563643966633736.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:7
                if v_636f6e74726f6c5f5f30613636643035636665643764636434.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:20
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(1.0015_f64)), ("exponent", Value::number(v_64795f5f66653066333265616633366630633261.clone()))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:7
                if { let pui_native_398=!(v_617272616e67655f5f64636365366436323963373035326634.clone()); if (pui_native_398) {v_616c745f5f33333733643434363136623862303339.clone()} else {pui_native_398} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:28
                    return record(vec![("kind", Value::string("zoom-y")), ("x", Value::number(0.0_f64)), ("y", Value::number(0.0_f64)), ("scale", Value::number(1.0015_f64)), ("exponent", Value::number(v_64795f5f66653066333265616633366630633261.clone()))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:7
                if v_73686966745f5f61313536343834336232343731376437.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:18
                    return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(if { let pui_native_400=v_64785f5f30316638323735666436613634353366.clone(); let pui_native_401=0.0_f64; !(pui_native_400==pui_native_401) } {v_64785f5f30316638323735666436613634353366.clone()} else {v_64795f5f66653066333265616633366630633261.clone()}))), ("y", Value::number(0.0_f64)), ("scale", Value::number(1.0_f64)), ("exponent", Value::number(0.0_f64))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":166:7
                if { let pui_native_402=v_617272616e67655f5f64636365366436323963373035326634.clone(); if (pui_native_402) {{ let pui_native_404=v_64785f5f30316638323735666436613634353366.clone(); let pui_native_405=0.0_f64; (pui_native_404==pui_native_405) }} else {pui_native_402} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":166:32
                    return v_6e6f6e655f5f35633438646231656563643966633736.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":167:7
                return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(v_64785f5f30316638323735666436613634353366.clone()))), ("y", Value::number(if v_617272616e67655f5f64636365366436323963373035326634.clone() {0.0_f64} else {-(v_64795f5f66653066333265616633366630633261.clone())})), ("scale", Value::number(1.0_f64)), ("exponent", Value::number(0.0_f64))]);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool(), (arg(&arguments,6)).bool(), (arg(&arguments,7)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:5
            { let v_7075695f6e61746976655f343036 = v_646566.clone(); expose_method(&owner, Value::string("navigate"), (v_6e61766967617465.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:5
            { let v_7075695f6e61746976655f343037 = v_646566.clone(); expose_method(&owner, Value::string("wheel"), ({  let v_6e61766967617465 = v_6e61766967617465.clone(); let call:Rc<dyn Fn(f64, f64, bool, bool, bool, bool)->Value>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool| -> Value {
                let v_64785f5f34306661396466303666343032353164: f64 = argument_0;
                let v_64795f5f65623965616134386533623961623363: f64 = argument_1;
                let v_636f6e74726f6c5f5f33336265343631646636303862353664: bool = argument_2;
                let v_73686966745f5f38386135636534353131386164313833: bool = argument_3;
                let v_616c745f5f39333833316535653631363065643632: bool = argument_4;
                let v_617272616e67655f5f32653538333262366531306366346561: bool = argument_5;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:126
                return (v_6e61766967617465.read().call)(Value::string("wheel"), v_64785f5f34306661396466303666343032353164.clone(), v_64795f5f65623965616134386533623961623363.clone(), 1.0_f64, v_636f6e74726f6c5f5f33336265343631646636303862353664.clone(), v_73686966745f5f38386135636534353131386164313833.clone(), v_616c745f5f39333833316535653631363065643632.clone(), v_617272616e67655f5f32653538333262366531306366346561.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":171:5
            { let v_7075695f6e61746976655f343038 = v_646566.clone(); expose_method(&owner, Value::string("pinch"), ({  let v_6e61766967617465 = v_6e61766967617465.clone(); let call:Rc<dyn Fn(f64)->Value>=Rc::new(move |argument_0: f64| -> Value {
                let v_7363616c655f5f31613230626234626532393163303437: f64 = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":171:51
                return (v_6e61766967617465.read().call)(Value::string("pinch"), 0.0_f64, 0.0_f64, v_7363616c655f5f31613230626234626532393163303437.clone(), false, false, false, false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":172:11
            v_696e74656e74.initialize({  let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn(Value, Value, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: Value, argument_2: bool| -> Value {
                let v_6b696e645f5f39346339633937383261616431353464: Value = argument_0;
                let v_73656c656374696f6e: Value = argument_1;
                let v_736e61705f5f32393536653135623363666337306630: bool = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":173:13
                let v_6e6f6e655f5f66653664356234376531666338633330 = record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":174:7
                if { let pui_native_409={ let pui_native_411={ let pui_native_413={ let pui_native_415={ let pui_native_417={ let pui_native_419=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_419) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_419} }; if !(pui_native_417) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_417} }; if !(pui_native_415) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_415} }; if !(pui_native_413) {({ let v_7075695f6e61746976655f343231 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343231.clone()).get()).bool()) }).bool()} else {pui_native_413} }; if !(pui_native_411) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f343234 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f343234.clone()).get() }), &(Value::string(""))))).bool()} else {pui_native_411} }; if !(pui_native_409) {(Value::boolean(strict_equal(&(v_6b696e645f5f39346339633937383261616431353464.clone()), &(Value::string("none"))))).bool()} else {pui_native_409} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":174:116
                    return v_6e6f6e655f5f66653664356234376531666338633330.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":175:13
                let v_7469636b65745f5f35636133616639383037383865666164: f64 = { let value=(v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":176:7
                { let v_7075695f6e61746976655f343237 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f343237.clone(), v_6b696e645f5f39346339633937383261616431353464.clone(), Value::string("reason: editing operation chosen")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":177:7
                return if (Value::boolean({ let pui_native_428={ let pui_native_430={ let pui_native_432={ let pui_native_434={ let pui_native_436=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_436) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_436} }; if (pui_native_434) {{ let pui_native_440={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_441=v_7469636b65745f5f35636133616639383037383865666164.clone(); (pui_native_440==pui_native_441) }} else {pui_native_434} }; if (pui_native_432) {!({ let value=(v_73796e63696e67.read()).borrow().clone(); value })} else {pui_native_432} }; if (pui_native_430) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_430} }; if (pui_native_428) {!(({ let v_7075695f6e61746976655f343432 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343432.clone()).get()).bool()) }).bool())} else {pui_native_428} })).truthy() { record(vec![("kind", v_6b696e645f5f39346339633937383261616431353464.clone()), ("selection", v_73656c656374696f6e.clone()), ("snap", Value::boolean(v_736e61705f5f32393536653135623363666337306630.clone()))]) } else { v_6e6f6e655f5f66653664356234376531666338633330.clone() };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), arg(&arguments,1), (arg(&arguments,2)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":179:5
            { let v_7075695f6e61746976655f343433 = v_646566.clone(); expose_method(&owner, Value::string("regionGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool, bool, f64, f64, f64, f64, f64, bool, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: f64, argument_6: f64, argument_7: f64, argument_8: f64, argument_9: f64, argument_10: bool, argument_11: bool| -> Value {
                let v_6869745f5f65353761353939333765393838633331: Value = argument_0;
                let v_72616e6765: bool = argument_1;
                let v_7365636f6e646172795f5f64313539626232323239633634383263: bool = argument_2;
                let v_657874656e64: bool = argument_3;
                let v_73706c6974: bool = argument_4;
                let v_636c69636b735f5f39303534393739653435373762343737: f64 = argument_5;
                let v_6c656674: f64 = argument_6;
                let v_7269676874: f64 = argument_7;
                let v_66616465496e: f64 = argument_8;
                let v_666164654f7574: f64 = argument_9;
                let v_6661646542616e64: bool = argument_10;
                let v_616c745f5f37376137623037383434393966346662: bool = argument_11;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:7
                if { let pui_native_444=!(v_7365636f6e646172795f5f64313539626232323239633634383263.clone()); if (pui_native_444) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f343438 = v_7068617365.read(); (v_7075695f6e61746976655f343438.clone()).get() }), &(Value::string("pressed"))))).bool()} else {pui_native_444} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:7
                if v_7365636f6e646172795f5f64313539626232323239633634383263.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:22
                    return (v_696e74656e74.read().call)(if (Value::boolean((Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("empty"))))).bool())).truthy() { Value::string("lane-menu") } else { Value::string("region-menu") }, if (Value::boolean({ let pui_native_451=(Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_451) {v_657874656e64.clone()} else {pui_native_451} })).truthy() { Value::string("extend") } else { Value::string("replace") }, !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:7
                if v_72616e6765.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:18
                    return (v_696e74656e74.read().call)(Value::string("range"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":184:7
                if (Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("empty"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":184:28
                    return (v_696e74656e74.read().call)(if (Value::boolean({ let pui_native_457:f64=v_636c69636b735f5f39303534393739653435373762343737.clone(); let pui_native_458:f64=2.0_f64; pui_native_457 >= pui_native_458 })).truthy() { Value::string("create-part") } else { Value::string("seek") }, Value::string("clear"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:7
                if { let pui_native_459=(Value::boolean(!strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_459) {(Value::boolean(!strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("part"))))).bool()} else {pui_native_459} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:46
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:7
                if { let pui_native_465=(Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_465) {v_657874656e64.clone()} else {pui_native_465} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:38
                    return (v_696e74656e74.read().call)(Value::string("select"), Value::string("extend"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:7
                if { let pui_native_469=(Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_469) {v_73706c6974.clone()} else {pui_native_469} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:37
                    return (v_696e74656e74.read().call)(Value::string("split"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":188:7
                if { let pui_native_473:f64=v_636c69636b735f5f39303534393739653435373762343737.clone(); let pui_native_474:f64=2.0_f64; pui_native_473 >= pui_native_474 } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":188:24
                    return (v_696e74656e74.read().call)(if (Value::boolean((Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool())).truthy() { Value::string("region-menu") } else { Value::string("open-part") }, Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:7
                if { let pui_native_477={ let pui_native_479={ let pui_native_481=(Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_481) {v_6661646542616e64.clone()} else {pui_native_481} }; if (pui_native_479) {{ let pui_native_485:f64=v_66616465496e.clone(); let pui_native_486:f64=0.0_f64; pui_native_485 >= pui_native_486 }} else {pui_native_479} }; if (pui_native_477) {{ let pui_native_487:f64=v_66616465496e.clone(); let pui_native_488:f64=7.0_f64; pui_native_487 < pui_native_488 }} else {pui_native_477} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:69
                    return (v_696e74656e74.read().call)(Value::string("fade-in"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:7
                if { let pui_native_489={ let pui_native_491={ let pui_native_493=(Value::boolean(strict_equal(&(v_6869745f5f65353761353939333765393838633331.clone()), &(Value::string("audio"))))).bool(); if (pui_native_493) {v_6661646542616e64.clone()} else {pui_native_493} }; if (pui_native_491) {{ let pui_native_497:f64=v_666164654f7574.clone(); let pui_native_498:f64=0.0_f64; pui_native_497 >= pui_native_498 }} else {pui_native_491} }; if (pui_native_489) {{ let pui_native_499:f64=v_666164654f7574.clone(); let pui_native_500:f64=7.0_f64; pui_native_499 < pui_native_500 }} else {pui_native_489} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:71
                    return (v_696e74656e74.read().call)(Value::string("fade-out"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:7
                if { let pui_native_501={ let pui_native_503:f64=v_6c656674.clone(); let pui_native_504:f64=0.0_f64; pui_native_503 >= pui_native_504 }; if (pui_native_501) {{ let pui_native_505:f64=v_6c656674.clone(); let pui_native_506:f64=7.0_f64; pui_native_505 < pui_native_506 }} else {pui_native_501} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:34
                    return (v_696e74656e74.read().call)(Value::string("resize-left"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:7
                if { let pui_native_507={ let pui_native_509:f64=v_7269676874.clone(); let pui_native_510:f64=0.0_f64; pui_native_509 >= pui_native_510 }; if (pui_native_507) {{ let pui_native_511:f64=v_7269676874.clone(); let pui_native_512:f64=7.0_f64; pui_native_511 < pui_native_512 }} else {pui_native_507} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:36
                    return (v_696e74656e74.read().call)(Value::string("resize-right"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":193:7
                return (v_696e74656e74.read().call)(Value::string("move-region"), Value::string("replace"), !(v_616c745f5f37376137623037383434393966346662.clone()));
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).num(), (arg(&arguments,6)).num(), (arg(&arguments,7)).num(), (arg(&arguments,8)).num(), (arg(&arguments,9)).num(), (arg(&arguments,10)).bool(), (arg(&arguments,11)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:5
            { let v_7075695f6e61746976655f353133 = v_646566.clone(); expose_method(&owner, Value::string("pointGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(Value, bool, bool, f64, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: f64, argument_4: bool| -> Value {
                let v_66616d696c79: Value = argument_0;
                let v_6869745f5f37313064373866376234396536623033: bool = argument_1;
                let v_7365636f6e646172795f5f32663363396436306664666463356363: bool = argument_2;
                let v_636c69636b735f5f32646463656531393831363037663361: f64 = argument_3;
                let v_616c745f5f61333536373965323633393164313936: bool = argument_4;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:7
                if { let pui_native_514=!(v_7365636f6e646172795f5f32663363396436306664666463356363.clone()); if (pui_native_514) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f353138 = v_7068617365.read(); (v_7075695f6e61746976655f353138.clone()).get() }), &(Value::string("pressed"))))).bool()} else {pui_native_514} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:7
                if { let pui_native_519=(Value::boolean(!strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo"))))).bool(); if (pui_native_519) {(Value::boolean(!strict_equal(&(v_66616d696c79.clone()), &(Value::string("automation"))))).bool()} else {pui_native_519} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:58
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:7
                if v_7365636f6e646172795f5f32663363396436306664666463356363.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:22
                    return (v_696e74656e74.read().call)(if (Value::boolean({ let pui_native_525=(Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo"))))).bool(); if !(pui_native_525) {v_6869745f5f37313064373866376234396536623033.clone()} else {pui_native_525} })).truthy() { Value::string("point-menu") } else { Value::string("none") }, Value::string("none"), !(v_616c745f5f61333536373965323633393164313936.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":199:7
                if { let pui_native_529=(Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo"))))).bool(); if (pui_native_529) {{ let pui_native_533:f64=v_636c69636b735f5f32646463656531393831363037663361.clone(); let pui_native_534:f64=2.0_f64; pui_native_533 >= pui_native_534 }} else {pui_native_529} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":199:46
                    return (v_696e74656e74.read().call)(Value::string("edit-tempo"), Value::string("none"), !(v_616c745f5f61333536373965323633393164313936.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":200:7
                return (v_696e74656e74.read().call)(if (Value::boolean((Value::boolean(strict_equal(&(v_66616d696c79.clone()), &(Value::string("tempo"))))).bool())).truthy() { if (Value::boolean(v_6869745f5f37313064373866376234396536623033.clone())).truthy() { Value::string("move-tempo") } else { Value::string("create-tempo") } } else { if (Value::boolean(v_6869745f5f37313064373866376234396536623033.clone())).truthy() { Value::string("move-automation") } else { Value::string("create-automation") } }, Value::string("none"), !(v_616c745f5f61333536373965323633393164313936.clone()));
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).num(), (arg(&arguments,4)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":202:5
            { let v_7075695f6e61746976655f353337 = v_646566.clone(); expose_method(&owner, Value::string("curveGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool, bool, bool, bool, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool| -> Value {
                let v_746f6f6c5f5f66373334666339663839383439616134: Value = argument_0;
                let v_657261736572: bool = argument_1;
                let v_6869745f5f38303534613737643865313462366463: bool = argument_2;
                let v_73656c6563746564: bool = argument_3;
                let v_706f696e7473: bool = argument_4;
                let v_706f7274616d656e746f: bool = argument_5;
                let v_73686966745f5f36383736626630666365616232626466: bool = argument_6;
                let v_616c745f5f64623031336134373631393139653639: bool = argument_7;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:7
                if { let pui_native_538={ let pui_native_540=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f353434 = v_7068617365.read(); (v_7075695f6e61746976655f353434.clone()).get() }), &(Value::string("pressed"))))).bool(); if !(pui_native_540) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f353437 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f353437.clone()).get() }), &(Value::string(""))))).bool()} else {pui_native_540} }; if !(pui_native_538) {{ let pui_native_548={ let pui_native_550={ let pui_native_552={ let pui_native_554={ let pui_native_556=(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("select"))))).bool(); if (pui_native_556) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("eraser"))))).bool()} else {pui_native_556} }; if (pui_native_554) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("line"))))).bool()} else {pui_native_554} }; if (pui_native_552) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("smooth"))))).bool()} else {pui_native_552} }; if (pui_native_550) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("pitch"))))).bool()} else {pui_native_550} }; if (pui_native_548) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("pencil"))))).bool()} else {pui_native_548} }} else {pui_native_538} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":205:92
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:7
                Value::boolean({ let value=v_706f7274616d656e746f.clone(); *(v_6672656554696d65.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:7
                if { let pui_native_570=v_657261736572.clone(); if !(pui_native_570) {(Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("eraser"))))).bool()} else {pui_native_570} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:40
                    return (v_696e74656e74.read().call)(Value::string("curve-erase"), Value::string("none"), false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:7
                if { let pui_native_574={ let pui_native_576=(Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("select"))))).bool(); if (pui_native_576) {v_706f696e7473.clone()} else {pui_native_576} }; if (pui_native_574) {v_6869745f5f38303534613737643865313462366463.clone()} else {pui_native_574} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:47
                    return (v_696e74656e74.read().call)(Value::string("curve-point"), if (Value::boolean(v_73686966745f5f36383736626630666365616232626466.clone())).truthy() { Value::string("toggle") } else { if (Value::boolean(v_73656c6563746564.clone())).truthy() { Value::string("keep") } else { Value::string("replace") } }, { let pui_native_580=!(v_616c745f5f64623031336134373631393139653639.clone()); if (pui_native_580) {!(v_706f7274616d656e746f.clone())} else {pui_native_580} });
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":209:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("select"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":209:30
                    return (v_696e74656e74.read().call)(Value::string("curve-select"), Value::string("none"), { let pui_native_584=!(v_616c745f5f64623031336134373631393139653639.clone()); if (pui_native_584) {!(v_706f7274616d656e746f.clone())} else {pui_native_584} });
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":210:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("line"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":210:28
                    return (v_696e74656e74.read().call)(Value::string("curve-line"), Value::string("none"), false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":211:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("smooth"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":211:30
                    return (v_696e74656e74.read().call)(Value::string("curve-smooth"), Value::string("none"), false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":212:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f66373334666339663839383439616134.clone()), &(Value::string("pitch"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":212:29
                    return (v_696e74656e74.read().call)(Value::string("pitch-draw"), Value::string("none"), false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":213:7
                return (v_696e74656e74.read().call)(Value::string("curve-pencil"), Value::string("none"), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool(), (arg(&arguments,6)).bool(), (arg(&arguments,7)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":215:5
            { let v_7075695f6e61746976655f353932 = v_646566.clone(); expose_method(&owner, Value::string("noteGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool, bool, bool)->Value>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool| -> Value {
                let v_746f6f6c5f5f31653933623466383461613664316439: Value = argument_0;
                let v_6869745f5f36323433646539613531376134326261: bool = argument_1;
                let v_746f756368: bool = argument_2;
                let v_6c6f6e675072657373: bool = argument_3;
                let v_6d6f64756c6174696f6e: bool = argument_4;
                let v_616c745f5f36623932303236393265636663386362: bool = argument_5;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":217:13
                let v_6e6f6e655f5f36323461396633656138353336373734 = record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:7
                if { let pui_native_593=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f353937 = v_7068617365.read(); (v_7075695f6e61746976655f353937.clone()).get() }), &(Value::string("pressed"))))).bool(); if !(pui_native_593) {{ let pui_native_598={ let pui_native_600={ let pui_native_602=(Value::boolean(!strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pointer"))))).bool(); if (pui_native_602) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pencil"))))).bool()} else {pui_native_602} }; if (pui_native_600) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("eraser"))))).bool()} else {pui_native_600} }; if (pui_native_598) {(Value::boolean(!strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pitch"))))).bool()} else {pui_native_598} }} else {pui_native_593} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:124
                    return v_6e6f6e655f5f36323461396633656138353336373734.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:7
                if v_6c6f6e675072657373.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:22
                    return if (Value::boolean((Value::boolean(strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pitch"))))).bool())).truthy() { v_6e6f6e655f5f36323461396633656138353336373734.clone() } else { (v_696e74656e74.read().call)(if (Value::boolean(v_6869745f5f36323433646539613531376134326261.clone())).truthy() { Value::string("note-lyric") } else { Value::string("note-marquee") }, Value::string("none"), false) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":220:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pitch"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":220:29
                    return (v_696e74656e74.read().call)(Value::string("pitch-draw"), if (Value::boolean(v_6869745f5f36323433646539613531376134326261.clone())).truthy() { Value::string("replace") } else { Value::string("none") }, false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":221:7
                if { let pui_native_616=!(v_746f756368.clone()); if (pui_native_616) {v_6d6f64756c6174696f6e.clone()} else {pui_native_616} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":221:33
                    return (v_696e74656e74.read().call)(Value::string("note-modulation"), Value::string("none"), false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:7
                if { let pui_native_618={ let pui_native_620=v_746f756368.clone(); if (pui_native_620) {(Value::boolean(strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pointer"))))).bool()} else {pui_native_620} }; if (pui_native_618) {!(v_6869745f5f36323433646539613531376134326261.clone())} else {pui_native_618} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:48
                    return v_6e6f6e655f5f36323461396633656138353336373734.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":223:7
                if (Value::boolean(strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("eraser"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":223:30
                    return if (Value::boolean(v_6869745f5f36323433646539613531376134326261.clone())).truthy() { (v_696e74656e74.read().call)(Value::string("erase-note"), Value::string("none"), false) } else { v_6e6f6e655f5f36323461396633656138353336373734.clone() };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":224:7
                if { let pui_native_626=(Value::boolean(strict_equal(&(v_746f6f6c5f5f31653933623466383461613664316439.clone()), &(Value::string("pencil"))))).bool(); if (pui_native_626) {!(v_6869745f5f36323433646539613531376134326261.clone())} else {pui_native_626} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":224:38
                    return (v_696e74656e74.read().call)(Value::string("draw-note"), Value::string("none"), !(v_616c745f5f36623932303236393265636663386362.clone()));
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":225:7
                return (v_696e74656e74.read().call)(Value::string("note-edit"), Value::string("none"), !(v_616c745f5f36623932303236393265636663386362.clone()));
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":227:5
            { let v_7075695f6e61746976655f363330 = v_646566.clone(); expose_method(&owner, Value::string("phonemeGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(bool, bool, bool)->Value>=Rc::new(move |argument_0: bool, argument_1: bool, argument_2: bool| -> Value {
                let v_626f756e64617279: bool = argument_0;
                let v_6e6f7465: bool = argument_1;
                let v_6564697461626c65: bool = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":228:7
                if (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f363333 = v_7068617365.read(); (v_7075695f6e61746976655f363333.clone()).get() }), &(Value::string("pressed"))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":228:38
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":229:7
                return (v_696e74656e74.read().call)(if (Value::boolean({ let pui_native_634=v_626f756e64617279.clone(); if (pui_native_634) {v_6564697461626c65.clone()} else {pui_native_634} })).truthy() { Value::string("phoneme-drag") } else { if (Value::boolean(v_6e6f7465.clone())).truthy() { Value::string("select-note") } else { Value::string("none") } }, if (Value::boolean(v_6e6f7465.clone())).truthy() { Value::string("replace") } else { Value::string("none") }, false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool(), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":231:5
            { let v_7075695f6e61746976655f363336 = v_646566.clone(); expose_method(&owner, Value::string("rulerGesture"), ({  let v_7068617365 = v_7068617365.clone(); let v_696e74656e74 = v_696e74656e74.clone(); let call:Rc<dyn Fn(bool, f64, bool, bool, bool, bool, bool, bool)->Value>=Rc::new(move |argument_0: bool, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool, argument_7: bool| -> Value {
                let v_7365636f6e646172795f5f66613930663962646238666163303162: bool = argument_0;
                let v_636c69636b735f5f37356235393230393364313464646664: f64 = argument_1;
                let v_617272616e67655f5f61636438313732653764666464346166: bool = argument_2;
                let v_7365676d656e74: bool = argument_3;
                let v_6c6f6f705374617274: bool = argument_4;
                let v_6c6f6f70456e64: bool = argument_5;
                let v_73686966745f5f35663730616631363633343435656561: bool = argument_6;
                let v_616c745f5f34326336653833373438313232346664: bool = argument_7;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":233:7
                if { let pui_native_637=!(v_7365636f6e646172795f5f66613930663962646238666163303162.clone()); if (pui_native_637) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f363431 = v_7068617365.read(); (v_7075695f6e61746976655f363431.clone()).get() }), &(Value::string("pressed"))))).bool()} else {pui_native_637} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":233:52
                    return record(vec![("kind", Value::string("none")), ("selection", Value::string("none")), ("snap", Value::boolean(false))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":234:7
                if v_7365636f6e646172795f5f66613930663962646238666163303162.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":234:22
                    return (v_696e74656e74.read().call)(Value::string("ruler-menu"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":235:7
                if { let pui_native_642={ let pui_native_644={ let pui_native_646:f64=v_636c69636b735f5f37356235393230393364313464646664.clone(); let pui_native_647:f64=2.0_f64; pui_native_646 >= pui_native_647 }; if (pui_native_644) {!(v_617272616e67655f5f61636438313732653764666464346166.clone())} else {pui_native_644} }; if (pui_native_642) {v_7365676d656e74.clone()} else {pui_native_642} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":235:47
                    return (v_696e74656e74.read().call)(Value::string("edit-segment"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":236:7
                if v_6c6f6f705374617274.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":236:22
                    return (v_696e74656e74.read().call)(Value::string("drag-loop-start"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":237:7
                if v_6c6f6f70456e64.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":237:20
                    return (v_696e74656e74.read().call)(Value::string("drag-loop-end"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":238:7
                if v_73686966745f5f35663730616631363633343435656561.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":238:18
                    return (v_696e74656e74.read().call)(Value::string("set-loop-start"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":239:7
                if v_616c745f5f34326336653833373438313232346664.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":239:16
                    return (v_696e74656e74.read().call)(Value::string("set-loop-end"), Value::string("none"), true);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":240:7
                return (v_696e74656e74.read().call)(Value::string("scrub"), Value::string("none"), true);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool(), (arg(&arguments,6)).bool(), (arg(&arguments,7)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":242:5
            { let v_7075695f6e61746976655f363438 = v_646566.clone(); expose_method(&owner, Value::string("motion"), ({  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_6672656554696d65 = v_6672656554696d65.clone(); let call:Rc<dyn Fn(f64, f64, bool, bool)->Value>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool| -> Value {
                let v_785f5f31366261636432343139373633626465: f64 = argument_0;
                let v_795f5f36613039303731636234613834653638: f64 = argument_1;
                let v_73686966745f5f33336366313562376334333738313134: bool = argument_2;
                let v_616c745f5f38626235316262653134313935313261: bool = argument_3;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":243:13
                let v_6e6f6e655f5f65326238323137623665373738613935 = record(vec![("kind", Value::string("none")), ("snap", Value::boolean(false)), ("fine", Value::number(1.0_f64)), ("axis", Value::string("")), ("moveX", Value::boolean(false)), ("moveY", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":244:7
                if { let pui_native_649={ let pui_native_651={ let pui_native_653={ let pui_native_655={ let pui_native_657={ let pui_native_659={ let pui_native_661=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_661) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_661} }; if !(pui_native_659) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_659} }; if !(pui_native_657) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_657} }; if !(pui_native_655) {({ let v_7075695f6e61746976655f363633 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f363633.clone()).get()).bool()) }).bool()} else {pui_native_655} }; if !(pui_native_653) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f363636 = v_7068617365.read(); (v_7075695f6e61746976655f363636.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_653} }; if !(pui_native_651) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f363639 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f363639.clone()).get() }), &(Value::string(""))))).bool()} else {pui_native_651} }; if !(pui_native_649) {!({ let pui_native_670=(v_785f5f31366261636432343139373633626465.clone()).is_finite(); if (pui_native_670) {(v_795f5f36613039303731636234613834653638.clone()).is_finite()} else {pui_native_670} })} else {pui_native_649} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":244:140
                    return v_6e6f6e655f5f65326238323137623665373738613935.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":245:13
                let v_6b696e645f5f39393466393136626132646266626239 = { let v_7075695f6e61746976655f363732 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f363732.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":246:13
                let v_64785f5f32633666346566333736666234303663: f64 = { let pui_native_673:f64=v_785f5f31366261636432343139373633626465.clone(); let pui_native_674:f64={ let value=(v_6f726967696e58.read()).borrow().clone(); value }; pui_native_673 - pui_native_674 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":246:37
                let v_64795f5f39386330653763633337313330353631: f64 = { let pui_native_675:f64=v_795f5f36613039303731636234613834653638.clone(); let pui_native_676:f64={ let value=(v_6f726967696e59.read()).borrow().clone(); value }; pui_native_675 - pui_native_676 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":247:13
                let v_6178: f64 = if { let pui_native_677:f64=v_64785f5f32633666346566333736666234303663.clone(); let pui_native_678:f64=0.0_f64; pui_native_677 < pui_native_678 } {-(v_64785f5f32633666346566333736666234303663.clone())} else {v_64785f5f32633666346566333736666234303663.clone()};
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":247:43
                let v_6179: f64 = if { let pui_native_679:f64=v_64795f5f39386330653763633337313330353631.clone(); let pui_native_680:f64=0.0_f64; pui_native_679 < pui_native_680 } {-(v_64795f5f39386330653763633337313330353631.clone())} else {v_64795f5f39386330653763633337313330353631.clone()};
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":248:13
                let v_6375727665: bool = { let pui_native_681={ let pui_native_683={ let pui_native_685={ let pui_native_687={ let pui_native_689={ let pui_native_691=(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-point"))))).bool(); if !(pui_native_691) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-select"))))).bool()} else {pui_native_691} }; if !(pui_native_689) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-line"))))).bool()} else {pui_native_689} }; if !(pui_native_687) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-pencil"))))).bool()} else {pui_native_687} }; if !(pui_native_685) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-smooth"))))).bool()} else {pui_native_685} }; if !(pui_native_683) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-erase"))))).bool()} else {pui_native_683} }; if !(pui_native_681) {(Value::boolean(strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("pitch-draw"))))).bool()} else {pui_native_681} };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":250:13
                let v_7469636b65745f5f63343634363633353235363836626333: f64 = { let value=(v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":251:7
                if { let pui_native_707=!(v_73686966745f5f33336366313562376334333738313134.clone()); if !(pui_native_707) {!(v_6375727665.clone())} else {pui_native_707} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":251:29
                    { let v_7075695f6e61746976655f373039 = v_61786973.read(); state_set(v_7075695f6e61746976655f373039.clone(), Value::string(""), Value::string("reason: editing axis released")) };
                } else {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":252:12
                    if { let pui_native_710=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f373134 = v_61786973.read(); (v_7075695f6e61746976655f373134.clone()).get() }), &(Value::string(""))))).bool(); if (pui_native_710) {{ let pui_native_715={ let pui_native_717:f64=v_6178.clone(); let pui_native_718:f64=3.0_f64; pui_native_717 >= pui_native_718 }; if !(pui_native_715) {{ let pui_native_719:f64=v_6179.clone(); let pui_native_720:f64=3.0_f64; pui_native_719 >= pui_native_720 }} else {pui_native_715} }} else {pui_native_710} } {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":252:59
                        { let v_7075695f6e61746976655f373231 = v_61786973.read(); state_set(v_7075695f6e61746976655f373231.clone(), if (Value::boolean({ let pui_native_722:f64=v_6178.clone(); let pui_native_723:f64=v_6179.clone(); pui_native_722 >= pui_native_723 })).truthy() { Value::string("x") } else { Value::string("y") }, Value::string("reason: editing axis chosen")) };
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":253:7
                if !({ let pui_native_724={ let pui_native_726=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_726) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_726} }; if (pui_native_724) {{ let pui_native_730={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_731=v_7469636b65745f5f63343634363633353235363836626333.clone(); (pui_native_730==pui_native_731) }} else {pui_native_724} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":253:29
                    return v_6e6f6e655f5f65326238323137623665373738613935.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":254:13
                let v_736e61705f5f30663164373736643561326538663136: bool = { let pui_native_732={ let pui_native_734={ let pui_native_736={ let pui_native_738={ let pui_native_740={ let pui_native_742={ let pui_native_744=!(v_616c745f5f38626235316262653134313935313261.clone()); if (pui_native_744) {!({ let value=(v_6672656554696d65.read()).borrow().clone(); value })} else {pui_native_744} }; if (pui_native_742) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-line"))))).bool()} else {pui_native_742} }; if (pui_native_740) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-pencil"))))).bool()} else {pui_native_740} }; if (pui_native_738) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-smooth"))))).bool()} else {pui_native_738} }; if (pui_native_736) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("curve-erase"))))).bool()} else {pui_native_736} }; if (pui_native_734) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("pitch-draw"))))).bool()} else {pui_native_734} }; if (pui_native_732) {(Value::boolean(!strict_equal(&(v_6b696e645f5f39393466393136626132646266626239.clone()), &(Value::string("phoneme-drag"))))).bool()} else {pui_native_732} };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":255:7
                return record(vec![("kind", v_6b696e645f5f39393466393136626132646266626239.clone()), ("snap", Value::boolean(v_736e61705f5f30663164373736643561326538663136.clone())), ("fine", Value::number(if v_73686966745f5f33336366313562376334333738313134.clone() {0.1_f64} else {1.0_f64})), ("axis", { let v_7075695f6e61746976655f373538 = v_61786973.read(); (v_7075695f6e61746976655f373538.clone()).get() }), ("moveX", Value::boolean({ let pui_native_759:f64=v_6178.clone(); let pui_native_760:f64=3.0_f64; pui_native_759 >= pui_native_760 })), ("moveY", Value::boolean({ let pui_native_761:f64=v_6179.clone(); let pui_native_762:f64=3.0_f64; pui_native_761 >= pui_native_762 }))]);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":257:5
            { let v_7075695f6e61746976655f373633 = v_646566.clone(); expose_method(&owner, Value::string("edgePan"), ({  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn(f64, f64)->Value>=Rc::new(move |argument_0: f64, argument_1: f64| -> Value {
                let v_785f5f65363833316662396166373963613635: f64 = argument_0;
                let v_7769647468: f64 = argument_1;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":258:13
                let v_6e6f6e655f5f61653330363663303638663434323565 = record(vec![("position", Value::number(if (v_785f5f65363833316662396166373963613635.clone()).is_finite() {v_785f5f65363833316662396166373963613635.clone()} else {0.0_f64})), ("delta", Value::number(0.0_f64)), ("active", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":259:7
                if { let pui_native_764={ let pui_native_766={ let pui_native_768={ let pui_native_770={ let pui_native_772={ let pui_native_774={ let pui_native_776={ let pui_native_778=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_778) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_778} }; if !(pui_native_776) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_776} }; if !(pui_native_774) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_774} }; if !(pui_native_772) {({ let v_7075695f6e61746976655f373830 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f373830.clone()).get()).bool()) }).bool()} else {pui_native_772} }; if !(pui_native_770) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f373833 = v_7068617365.read(); (v_7075695f6e61746976655f373833.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_770} }; if !(pui_native_768) {{ let pui_native_784=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373838 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f373838.clone()).get() }), &(Value::string("note-edit"))))).bool(); if (pui_native_784) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f373931 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f373931.clone()).get() }), &(Value::string("note-marquee"))))).bool()} else {pui_native_784} }} else {pui_native_768} }; if !(pui_native_766) {!({ let pui_native_792=(v_785f5f65363833316662396166373963613635.clone()).is_finite(); if (pui_native_792) {(v_7769647468.clone()).is_finite()} else {pui_native_792} })} else {pui_native_766} }; if !(pui_native_764) {{ let pui_native_794:f64=v_7769647468.clone(); let pui_native_795:f64=0.0_f64; pui_native_794 <= pui_native_795 }} else {pui_native_764} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":260:120
                    return v_6e6f6e655f5f61653330363663303638663434323565.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":261:7
                if { let pui_native_796:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_797:f64={ let pui_native_798:f64=v_7769647468.clone(); let pui_native_799:f64=16.0_f64; pui_native_798 - pui_native_799 }; pui_native_796 > pui_native_797 } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":261:27
                    return record(vec![("position", Value::number(if { let pui_native_800:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_801:f64={ let pui_native_802:f64=v_7769647468.clone(); let pui_native_803:f64=40.0_f64; pui_native_802 + pui_native_803 }; pui_native_800 < pui_native_801 } {v_785f5f65363833316662396166373963613635.clone()} else {{ let pui_native_804:f64=v_7769647468.clone(); let pui_native_805:f64=40.0_f64; pui_native_804 + pui_native_805 }})), ("delta", Value::number(if { let pui_native_806:f64={ let pui_native_808:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_809:f64={ let pui_native_810:f64=v_7769647468.clone(); let pui_native_811:f64=16.0_f64; pui_native_810 - pui_native_811 }; pui_native_808 - pui_native_809 }; let pui_native_807:f64=40.0_f64; pui_native_806 < pui_native_807 } {{ let pui_native_812:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_813:f64={ let pui_native_814:f64=v_7769647468.clone(); let pui_native_815:f64=16.0_f64; pui_native_814 - pui_native_815 }; pui_native_812 - pui_native_813 }} else {40.0_f64})), ("active", Value::boolean(true))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":262:7
                if { let pui_native_816:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_817:f64=16.0_f64; pui_native_816 < pui_native_817 } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":262:19
                    return record(vec![("position", Value::number(v_785f5f65363833316662396166373963613635.clone())), ("delta", Value::number(if { let pui_native_818:f64={ let pui_native_820:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_821:f64=16.0_f64; pui_native_820 - pui_native_821 }; let pui_native_819:f64=-(40.0_f64); pui_native_818 > pui_native_819 } {{ let pui_native_822:f64=v_785f5f65363833316662396166373963613635.clone(); let pui_native_823:f64=16.0_f64; pui_native_822 - pui_native_823 }} else {-(40.0_f64)})), ("active", Value::boolean(true))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":263:7
                return v_6e6f6e655f5f61653330363663303638663434323565.clone();
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":265:5
            { let v_7075695f6e61746976655f383234 = v_646566.clone(); expose_method(&owner, Value::string("rulerSample"), ({  let v_7068617365 = v_7068617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let call:Rc<dyn Fn(f64, f64, f64)->Value>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64| -> Value {
                let v_706f736974696f6e: f64 = argument_0;
                let v_6f757473696465: f64 = argument_1;
                let v_656c6170736564: f64 = argument_2;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":266:13
                let v_6e6f6e655f5f37393963373132393065323036666163 = record(vec![("position", Value::number(0.0_f64)), ("delta", Value::number(0.0_f64)), ("active", Value::boolean(false)), ("snap", Value::boolean(false)), ("accepted", Value::boolean(false))]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":267:7
                if { let pui_native_825={ let pui_native_827={ let pui_native_829={ let pui_native_831={ let pui_native_833={ let pui_native_835={ let pui_native_837={ let pui_native_839={ let pui_native_841=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_841) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_841} }; if !(pui_native_839) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_839} }; if !(pui_native_837) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_837} }; if !(pui_native_835) {({ let v_7075695f6e61746976655f383433 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f383433.clone()).get()).bool()) }).bool()} else {pui_native_835} }; if !(pui_native_833) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f383436 = v_7068617365.read(); (v_7075695f6e61746976655f383436.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_833} }; if !(pui_native_831) {(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f383439 = v_6f7065726174696f6e.read(); (v_7075695f6e61746976655f383439.clone()).get() }), &(Value::string("scrub"))))).bool()} else {pui_native_831} }; if !(pui_native_829) {!({ let pui_native_850=(v_706f736974696f6e.clone()).is_finite(); if (pui_native_850) {(v_6f757473696465.clone()).is_finite()} else {pui_native_850} })} else {pui_native_829} }; if !(pui_native_827) {!((v_656c6170736564.clone()).is_finite())} else {pui_native_827} }; if !(pui_native_825) {{ let pui_native_852:f64=v_656c6170736564.clone(); let pui_native_853:f64=0.0_f64; pui_native_852 < pui_native_853 }} else {pui_native_825} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":268:115
                    return v_6e6f6e655f5f37393963373132393065323036666163.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":269:13
                let v_64697374616e6365: f64 = if { let pui_native_854:f64=v_6f757473696465.clone(); let pui_native_855:f64=0.0_f64; pui_native_854 < pui_native_855 } {-(v_6f757473696465.clone())} else {v_6f757473696465.clone()};
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":270:13
                let v_7370656564: f64 = { let pui_native_856:f64=100.0_f64; let pui_native_857:f64={ let pui_native_858:f64=9.0_f64; let pui_native_859:f64=v_64697374616e6365.clone(); pui_native_858 * pui_native_859 }; pui_native_856 + pui_native_857 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":271:13
                let v_76656c6f63697479: f64 = if { let pui_native_860=v_6f757473696465.clone(); let pui_native_861=0.0_f64; (pui_native_860==pui_native_861) } {0.0_f64} else {{ let pui_native_862:f64=if { let pui_native_864:f64=v_6f757473696465.clone(); let pui_native_865:f64=0.0_f64; pui_native_864 < pui_native_865 } {-(1.0_f64)} else {1.0_f64}; let pui_native_863:f64=if { let pui_native_866:f64=v_7370656564.clone(); let pui_native_867:f64=2000.0_f64; pui_native_866 < pui_native_867 } {v_7370656564.clone()} else {2000.0_f64}; pui_native_862 * pui_native_863 }};
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":272:7
                return record(vec![("position", Value::number(v_706f736974696f6e.clone())), ("delta", Value::number({ let pui_native_868:f64=v_76656c6f63697479.clone(); let pui_native_869:f64=if { let pui_native_870:f64=v_656c6170736564.clone(); let pui_native_871:f64=0.05_f64; pui_native_870 < pui_native_871 } {v_656c6170736564.clone()} else {0.05_f64}; pui_native_868 * pui_native_869 })), ("active", Value::boolean({ let pui_native_872=v_6f757473696465.clone(); let pui_native_873=0.0_f64; !(pui_native_872==pui_native_873) })), ("snap", Value::boolean((Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f383736 = v_7068617365.read(); (v_7075695f6e61746976655f383736.clone()).get() }), &(Value::string("pressed"))))).bool())), ("accepted", Value::boolean(true))]);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":275:5
            { let v_7075695f6e61746976655f383737 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":276:13
                let v_68616e646c655f5f35383865363066613061343365313339 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":277:7
                if ({ let left = Value::boolean({ let pui_native_878={ let pui_native_880={ let pui_native_882={ let pui_native_884=!((v_68616e646c655f5f35383865363066613061343365313339.clone()).truthy()); if !(pui_native_884) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_884} }; if !(pui_native_882) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_882} }; if !(pui_native_880) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_880} }; if !(pui_native_878) {({ let v_7075695f6e61746976655f383836 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f383836.clone()).get()).bool()) }).bool()} else {pui_native_878} }); if !left.truthy() { (v_6576656e74.clone()).member("repeat", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":277:90
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":278:13
                let v_617564696f: bool = (Value::boolean(strict_equal(&(({ let v_7075695f6e61746976655f383839 = v_68616e646c655f5f35383865363066613061343365313339.clone(); read_props(&(v_7075695f6e61746976655f383839.clone()).owner(), false) }).member("commandMode", false)), &(Value::string("audio"))))).bool();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":279:13
                let v_7365636f6e646172795f5f63356336623533393731333432656630 = { let left = (v_6576656e74.clone()).member("ctrlKey", false); if !left.truthy() { (v_6576656e74.clone()).member("metaKey", false) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":280:11
                let v_636f6d6d616e64 = Rc::new(std::cell::RefCell::new(Value::string("")));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":281:7
                if v_617564696f.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":282:9
                    if { let pui_native_890={ let pui_native_892=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Delete"))))).bool(); if !(pui_native_892) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Backspace"))))).bool()} else {pui_native_892} }; if (pui_native_890) {!((v_7365636f6e646172795f5f63356336623533393731333432656630.clone()).truthy())} else {pui_native_890} } {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":282:82
                        { let value = if ((v_6576656e74.clone()).member("shiftKey", false)).truthy() { Value::string("ripple-delete") } else { Value::string("delete") }; *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":283:14
                        if ({ let left = Value::boolean({ let pui_native_898=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("i"))))).bool(); if !(pui_native_898) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("I"))))).bool()} else {pui_native_898} }); if left.truthy() { v_7365636f6e646172795f5f63356336623533393731333432656630.clone() } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":283:73
                            { let value = Value::string("split"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":284:14
                            if ({ let left = Value::boolean({ let pui_native_904=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("a"))))).bool(); if !(pui_native_904) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("A"))))).bool()} else {pui_native_904} }); if left.truthy() { v_7365636f6e646172795f5f63356336623533393731333432656630.clone() } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":284:73
                                { let value = Value::string("select-all"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":285:14
                                if ({ let left = Value::boolean({ let pui_native_910=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("j"))))).bool(); if !(pui_native_910) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("J"))))).bool()} else {pui_native_910} }); if left.truthy() { v_7365636f6e646172795f5f63356336623533393731333432656630.clone() } else { left } }).truthy() {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":285:73
                                    { let value = Value::string("join"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":286:14
                                    if { let pui_native_916={ let pui_native_918={ let pui_native_920=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("r"))))).bool(); if !(pui_native_920) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("R"))))).bool()} else {pui_native_920} }; if (pui_native_918) {!((v_7365636f6e646172795f5f63356336623533393731333432656630.clone()).truthy())} else {pui_native_918} }; if (pui_native_916) {!(((v_6576656e74.clone()).member("altKey", false)).truthy())} else {pui_native_916} } {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":286:91
                                        { let value = Value::string("range-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":287:14
                                        if { let pui_native_926={ let pui_native_928={ let pui_native_930=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("v"))))).bool(); if !(pui_native_930) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("V"))))).bool()} else {pui_native_930} }; if (pui_native_928) {!((v_7365636f6e646172795f5f63356336623533393731333432656630.clone()).truthy())} else {pui_native_928} }; if (pui_native_926) {!(((v_6576656e74.clone()).member("altKey", false)).truthy())} else {pui_native_926} } {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":287:91
                                            { let value = Value::string("select-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                        } else {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":288:14
                                            if (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape"))))).bool() {
                                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":288:42
                                                { let value = Value::string("clear-range"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":290:7
                if { let pui_native_938=(Value::boolean(strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string(""))))).bool(); if (pui_native_938) {{ let pui_native_942=(Value::boolean(!strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape"))))).bool(); if !(pui_native_942) {(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f393438 = v_7068617365.read(); (v_7075695f6e61746976655f393438.clone()).get() }), &(Value::string("idle"))))).bool()} else {pui_native_942} }} else {pui_native_938} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":290:81
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":291:13
                let v_7469636b65745f5f64666432643133633134313165313564: f64 = { let pui_native_949:f64={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_950:f64=1.0_f64; pui_native_949 + pui_native_950 };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":292:7
                Value::boolean((v_63616e63656c.read().call)());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":293:7
                if { let pui_native_951={ let pui_native_953={ let pui_native_955=!({ let pui_native_957={ let pui_native_959=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_959) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_959} }; if (pui_native_957) {{ let pui_native_963={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_964=v_7469636b65745f5f64666432643133633134313165313564.clone(); (pui_native_963==pui_native_964) }} else {pui_native_957} }); if !(pui_native_955) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_955} }; if !(pui_native_953) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_953} }; if !(pui_native_951) {({ let v_7075695f6e61746976655f393635 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f393635.clone()).get()).bool()) }).bool()} else {pui_native_951} } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":293:71
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":294:7
                { let v_7075695f6e61746976655f393636 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f393636.clone()) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":295:7
                if (Value::boolean(!strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string(""))))).bool() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":295:27
                    { let v_7075695f6e61746976655f393639 = v_68616e646c655f5f35383865363066613061343365313339.clone(); emit(&(v_7075695f6e61746976655f393639.clone()).owner(), Value::string("command"), record(vec![("command", { let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":297:5
            { let v_7075695f6e61746976655f393730 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":298:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f64363534643263373965623830363131: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":299:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":299:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":300:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":301:13
                let v_7469636b65745f5f61353139613562636434303330666235: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":302:7
                Value::boolean({ let value=true; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":303:7
                { let value = v_68616e646c655f5f64363534643263373965623830363131.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":304:13
                let v_70726f7073 = { let v_7075695f6e61746976655f393731 = v_68616e646c655f5f64363534643263373965623830363131.clone(); read_props(&(v_7075695f6e61746976655f393731.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":305:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":306:13
                let v_6f776e65725f5f35353461666465323831633063356466 = { let v_7075695f6e61746976655f393732 = v_706f696e746572.read(); (v_7075695f6e61746976655f393732.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":307:13
                let v_726574697265645f5f38623538353065313061373666646230: f64 = ({ let v_7075695f6e61746976655f393733 = v_6c65617365.read(); Value::number(((v_7075695f6e61746976655f393733.clone()).get()).num()) }).num();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":308:13
                let v_62617369735f5f33613932343134643762386364343133 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":309:13
                let v_73686f756c6443616e63656c: bool = { let pui_native_974=(Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f393738 = v_7068617365.read(); (v_7075695f6e61746976655f393738.clone()).get() }), &(Value::string("idle"))))).bool(); if (pui_native_974) {{ let pui_native_979=(Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool(); if !(pui_native_979) {{ let pui_native_983=((v_70726f7073.clone()).member("disabled", false)).bool(); let pui_native_984=true; (pui_native_983==pui_native_984) }} else {pui_native_979} }} else {pui_native_974} };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":310:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":311:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":312:9
                    { let v_7075695f6e61746976655f393835 = v_7068617365.read(); state_set(v_7075695f6e61746976655f393835.clone(), Value::string("idle"), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":313:9
                    if !({ let pui_native_986={ let pui_native_988=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_988) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_988} }; if (pui_native_986) {{ let pui_native_992={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_993=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_992==pui_native_993) }} else {pui_native_986} }) {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":313:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":314:9
                    { let v_7075695f6e61746976655f393934 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f393934.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":315:9
                    if !({ let pui_native_995={ let pui_native_997=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_997) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_997} }; if (pui_native_995) {{ let pui_native_1001={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1002=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1001==pui_native_1002) }} else {pui_native_995} }) {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":315:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":316:9
                    { let v_7075695f6e61746976655f31303033 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f31303033.clone(), Value::number(0.0_f64), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":317:9
                    if !({ let pui_native_1004={ let pui_native_1006=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1006) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1006} }; if (pui_native_1004) {{ let pui_native_1010={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1011=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1010==pui_native_1011) }} else {pui_native_1004} }) {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":317:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":318:9
                    { let v_7075695f6e61746976655f31303132 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f31303132.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":319:9
                    if !({ let pui_native_1013={ let pui_native_1015=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1015) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1015} }; if (pui_native_1013) {{ let pui_native_1019={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1020=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1019==pui_native_1020) }} else {pui_native_1013} }) {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":319:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":320:9
                    { let v_7075695f6e61746976655f31303231 = v_61786973.read(); state_set(v_7075695f6e61746976655f31303231.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":321:9
                    if !({ let pui_native_1022={ let pui_native_1024=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1024) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1024} }; if (pui_native_1022) {{ let pui_native_1028={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1029=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1028==pui_native_1029) }} else {pui_native_1022} }) {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":321:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":323:7
                { let v_7075695f6e61746976655f31303330 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f31303330.clone(), Value::boolean(((v_70726f7073.clone()).member("disabled", false)).bool()), Value::string("reason: editing policy")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":324:7
                if !({ let pui_native_1033={ let pui_native_1035=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1035) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1035} }; if (pui_native_1033) {{ let pui_native_1039={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1040=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1039==pui_native_1040) }} else {pui_native_1033} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":324:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":325:7
                { let v_7075695f6e61746976655f31303431 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f31303431.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: editing surface label")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":326:7
                if !({ let pui_native_1042={ let pui_native_1044=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1044) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1044} }; if (pui_native_1042) {{ let pui_native_1048={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1049=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1048==pui_native_1049) }} else {pui_native_1042} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":326:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":327:7
                { let v_7075695f6e61746976655f31303530 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f31303530.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f31303531 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f31303531.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":328:7
                if !({ let pui_native_1052={ let pui_native_1054=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1054) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1054} }; if (pui_native_1052) {{ let pui_native_1058={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1059=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1058==pui_native_1059) }} else {pui_native_1052} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":328:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":329:7
                if v_73686f756c6443616e63656c.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":329:25
                    { let v_7075695f6e61746976655f31303630 = v_68616e646c655f5f64363534643263373965623830363131.clone(); emit(&(v_7075695f6e61746976655f31303630.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f35353461666465323831633063356466.clone()), ("revision", v_62617369735f5f33613932343134643762386364343133.clone()), ("lease", Value::number(v_726574697265645f5f38623538353065313061373666646230.clone()))])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":330:7
                if !({ let pui_native_1061={ let pui_native_1063=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_1063) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_1063} }; if (pui_native_1061) {{ let pui_native_1067={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1068=v_7469636b65745f5f61353139613562636434303330666235.clone(); (pui_native_1067==pui_native_1068) }} else {pui_native_1061} }) {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":330:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":331:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":332:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":332:21
                    { let v_7075695f6e61746976655f31303639 = v_68616e646c655f5f64363534643263373965623830363131.clone(); request_update(&(v_7075695f6e61746976655f31303639.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":334:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_6f7065726174696f6e = v_6f7065726174696f6e.clone(); let v_61786973 = v_61786973.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let call:Rc<dyn Fn(bool)->Value>=Rc::new(move |argument_0: bool| -> Value {
                let v_7065726d616e656e74: bool = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":335:13
                let v_68616e646c655f5f30316237643461656439396237663436 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":336:13
                let v_6163746976655f5f34343763626563653539613766643364: bool = (Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f31303732 = v_7068617365.read(); (v_7075695f6e61746976655f31303732.clone()).get() }), &(Value::string("idle"))))).bool();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":337:13
                let v_6f776e65725f5f65346161633866303765363763393637 = { let v_7075695f6e61746976655f31303733 = v_706f696e746572.read(); (v_7075695f6e61746976655f31303733.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":338:13
                let v_726574697265645f5f66333133356536636236366439313866: f64 = ({ let v_7075695f6e61746976655f31303734 = v_6c65617365.read(); Value::number(((v_7075695f6e61746976655f31303734.clone()).get()).num()) }).num();
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":339:13
                let v_62617369735f5f37386636346261326638393063633566 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":340:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":341:13
                let v_7469636b65745f5f32333532313738633666363639323766: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":342:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":343:7
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":344:7
                Value::boolean({ let value={ let pui_native_1075={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_1075) {v_7065726d616e656e74.clone()} else {pui_native_1075} }; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":345:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":346:7
                if v_7065726d616e656e74.clone() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":346:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":347:7
                { let v_7075695f6e61746976655f31303737 = v_7068617365.read(); state_set(v_7075695f6e61746976655f31303737.clone(), Value::string("idle"), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":348:7
                if { let pui_native_1078={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1079=v_7469636b65745f5f32333532313738633666363639323766.clone(); !(pui_native_1078==pui_native_1079) } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":348:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":349:7
                { let v_7075695f6e61746976655f31303830 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f31303830.clone(), Value::string(""), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":350:7
                if { let pui_native_1081={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1082=v_7469636b65745f5f32333532313738633666363639323766.clone(); !(pui_native_1081==pui_native_1082) } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":350:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":351:7
                { let v_7075695f6e61746976655f31303833 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f31303833.clone(), Value::number(0.0_f64), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":352:7
                if { let pui_native_1084={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1085=v_7469636b65745f5f32333532313738633666363639323766.clone(); !(pui_native_1084==pui_native_1085) } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":352:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":353:7
                { let v_7075695f6e61746976655f31303836 = v_6f7065726174696f6e.read(); state_set(v_7075695f6e61746976655f31303836.clone(), Value::string(""), Value::string("reason: editing operation detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":354:7
                if { let pui_native_1087={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1088=v_7469636b65745f5f32333532313738633666363639323766.clone(); !(pui_native_1087==pui_native_1088) } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":354:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":355:7
                { let v_7075695f6e61746976655f31303839 = v_61786973.read(); state_set(v_7075695f6e61746976655f31303839.clone(), Value::string(""), Value::string("reason: editing axis detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":356:7
                if { let pui_native_1090={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_1091=v_7469636b65745f5f32333532313738633666363639323766.clone(); !(pui_native_1090==pui_native_1091) } {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":356:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":357:7
                if ({ let left = Value::boolean(v_6163746976655f5f34343763626563653539613766643364.clone()); if left.truthy() { v_68616e646c655f5f30316237643461656439396237663436.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":357:29
                    { let v_7075695f6e61746976655f31303932 = v_68616e646c655f5f30316237643461656439396237663436.clone(); emit(&(v_7075695f6e61746976655f31303932.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f65346161633866303765363763393637.clone()), ("revision", v_62617369735f5f37386636346261326638393063633566.clone()), ("lease", Value::number(v_726574697265645f5f66333133356536636236366439313866.clone()))])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":359:5
            { let v_7075695f6e61746976655f31303933 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:5
            { let v_7075695f6e61746976655f31303934 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f63313461663432303466363832313565: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:43
                Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":360:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f63313461663432303466363832313565.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":361:5
            { let v_7075695f6e61746976655f31303935 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("threshold"), Value::string("commandMode")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":362:5
            { let v_7075695f6e61746976655f31303936 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":362:37
                return (v_726574697265.read().call)(false);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":363:5
            { let v_7075695f6e61746976655f31303937 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":363:41
                return (v_726574697265.read().call)(true);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":364:5
            return { let callable={ let pui_native_1098={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":364:26
                return { let v_7075695f6e61746976655f31303939 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_1098.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_1098.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> EditorSurfaceHandle { EditorSurfaceHandle { owner: self.owner.clone() } }
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
