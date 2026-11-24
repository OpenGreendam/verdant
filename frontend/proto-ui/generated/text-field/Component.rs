// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  #[cfg(panic = "abort")] compile_error!("Authored exception lowering requires panic=unwind");
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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selectAllOnMount").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("submitOnEnter").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("cancelOnEscape").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("advanceOnTab").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("commitOnBlur").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("searchNavigation").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("numeric").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("numericMin").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericMax").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericStep").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericDefault").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("value").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("defaultValue").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("readOnly").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("placeholder").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("required").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("name").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("autoComplete").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("minLength").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxLength").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("inputMode").map_or(true, |prop| prop.is_null() || (prop.as_str() == Some("none") || prop.as_str() == Some("text") || prop.as_str() == Some("tel") || prop.as_str() == Some("url") || prop.as_str() == Some("email") || prop.as_str() == Some("numeric") || prop.as_str() == Some("decimal") || prop.as_str() == Some("search"))) && value.get("enterKeyHint").map_or(true, |prop| prop.is_null() || (prop.as_str() == Some("enter") || prop.as_str() == Some("done") || prop.as_str() == Some("go") || prop.as_str() == Some("next") || prop.as_str() == Some("previous") || prop.as_str() == Some("search") || prop.as_str() == Some("send"))) && value.get("ariaLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("labelledBy").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("describedBy").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "autoFocus" => value.is_boolean(),
        "selectAllOnMount" => value.is_boolean(),
        "submitOnEnter" => value.is_boolean(),
        "cancelOnEscape" => value.is_boolean(),
        "advanceOnTab" => value.is_boolean(),
        "commitOnBlur" => value.is_boolean(),
        "searchNavigation" => value.is_boolean(),
        "numeric" => value.is_boolean(),
        "numericMin" => value.as_f64().is_some_and(f64::is_finite),
        "numericMax" => value.as_f64().is_some_and(f64::is_finite),
        "numericStep" => value.as_f64().is_some_and(f64::is_finite),
        "numericDefault" => value.as_f64().is_some_and(f64::is_finite),
        "value" => value.is_string(),
        "defaultValue" => value.is_string(),
        "disabled" => value.is_boolean(),
        "readOnly" => value.is_boolean(),
        "placeholder" => value.is_string(),
        "required" => value.is_boolean(),
        "name" => value.is_string(),
        "autoComplete" => value.is_string(),
        "minLength" => value.as_f64().is_some_and(f64::is_finite),
        "maxLength" => value.as_f64().is_some_and(f64::is_finite),
        "inputMode" => (value.as_str() == Some("none") || value.as_str() == Some("text") || value.as_str() == Some("tel") || value.as_str() == Some("url") || value.as_str() == Some("email") || value.as_str() == Some("numeric") || value.as_str() == Some("decimal") || value.as_str() == Some("search")),
        "enterKeyHint" => (value.as_str() == Some("enter") || value.as_str() == Some("done") || value.as_str() == Some("go") || value.as_str() == Some("next") || value.as_str() == Some("previous") || value.as_str() == Some("search") || value.as_str() == Some("send")),
        "ariaLabel" => value.is_string(),
        "labelledBy" => value.is_string(),
        "describedBy" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "focusSelf" => if !(arguments.len() <= 1 && arguments.get(0).map_or(true, |value| matches!(value, Value::Undefined) || (value).try_data().is_some_and(|data| (data.is_object() && data.get("reason").map_or(true, |field| (field.as_str() == Some("programmatic") || field.as_str() == Some("keyboard") || field.as_str() == Some("pointer"))) && data.get("preventScroll").map_or(true, |field| field.is_boolean()))))) { value_type_error("invalid exposed method arguments") },
        "blurSelf" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "resetValue" => if !(arguments.len() <= 1 && arguments.get(0).map_or(true, |value| matches!(value, Value::Undefined) || (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "selectRange" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "selectAll" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "requestSubmit" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "requestCancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "requestAdvance" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "stepText" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData1 {
    Alternative0(String),
    Alternative1(String),
    Alternative2(String),
    Alternative3(String),
    Alternative4(String),
    Alternative5(String),
    Alternative6(String),
    Alternative7(String),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData2 {
    Alternative0(String),
    Alternative1(String),
    Alternative2(String),
    Alternative3(String),
    Alternative4(String),
    Alternative5(String),
    Alternative6(String),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<bool>,
    #[serde(rename = "selectAllOnMount", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<bool>,
    #[serde(rename = "submitOnEnter", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<bool>,
    #[serde(rename = "cancelOnEscape", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<bool>,
    #[serde(rename = "advanceOnTab", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "commitOnBlur", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "searchNavigation", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "numeric", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "numericMin", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
    #[serde(rename = "numericMax", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<f64>,
    #[serde(rename = "numericStep", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<f64>,
    #[serde(rename = "numericDefault", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<f64>,
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<String>,
    #[serde(rename = "defaultValue", default, skip_serializing_if = "Option::is_none")]
    pub field_13: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_14: Option<bool>,
    #[serde(rename = "readOnly", default, skip_serializing_if = "Option::is_none")]
    pub field_15: Option<bool>,
    #[serde(rename = "placeholder", default, skip_serializing_if = "Option::is_none")]
    pub field_16: Option<String>,
    #[serde(rename = "required", default, skip_serializing_if = "Option::is_none")]
    pub field_17: Option<bool>,
    #[serde(rename = "name", default, skip_serializing_if = "Option::is_none")]
    pub field_18: Option<String>,
    #[serde(rename = "autoComplete", default, skip_serializing_if = "Option::is_none")]
    pub field_19: Option<String>,
    #[serde(rename = "minLength", default, skip_serializing_if = "Option::is_none")]
    pub field_20: Option<f64>,
    #[serde(rename = "maxLength", default, skip_serializing_if = "Option::is_none")]
    pub field_21: Option<f64>,
    #[serde(rename = "inputMode", default, skip_serializing_if = "Option::is_none")]
    pub field_22: Option<GeneratedData1>,
    #[serde(rename = "enterKeyHint", default, skip_serializing_if = "Option::is_none")]
    pub field_23: Option<GeneratedData2>,
    #[serde(rename = "ariaLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_24: Option<String>,
    #[serde(rename = "labelledBy", default, skip_serializing_if = "Option::is_none")]
    pub field_25: Option<String>,
    #[serde(rename = "describedBy", default, skip_serializing_if = "Option::is_none")]
    pub field_26: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum GeneratedData4 {
    Alternative0(String),
    Alternative1(String),
    Alternative2(String),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData3 {
    #[serde(rename = "reason", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<GeneratedData4>,
    #[serde(rename = "preventScroll", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<bool>,
}
  
  pub type GeneratedProps = GeneratedData0;
  fn authored_hook_0(definition: Value, input: Value) -> Value {
    let owner = definition.owner();
    authored_hook(&owner, "as-input-editor", "once", input, { let owner = owner.clone();  Value::Function(Rc::new(move |arguments: Vec<Value>| {
        let v_6465665f5f61326333376636353034653363313533: Value = arg(&arguments, 0);
        let v_636f6e74726f6c: LexicalBinding<Value> = lexical_binding();
        let v_666f63757361626c65: LexicalBinding<Value> = lexical_binding();
        let v_76616c75655f5f35333132623533343531383633613830: LexicalBinding<Value> = lexical_binding();
        let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
        let v_726561644f6e6c79: LexicalBinding<Value> = lexical_binding();
        let v_636f6d706f73696e67: LexicalBinding<Value> = lexical_binding();
        let v_617269614c6162656c: LexicalBinding<Value> = lexical_binding();
        let v_6c6162656c6c65644279: LexicalBinding<Value> = lexical_binding();
        let v_6465736372696265644279: LexicalBinding<Value> = lexical_binding();
        let v_63757272656e7456696577: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
        let v_7669657745706f63685f5f66393766306532316130323536623461: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
        let v_73796e63: LexicalBinding<Value> = lexical_binding();
        let v_656d6974436f6d706f736974696f6e: LexicalBinding<Value> = lexical_binding();
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":30:9
        let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":31:3
        { let v_7075695f6e61746976655f30 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f313e = record(vec![("value", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("defaultValue", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("readOnly", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("placeholder", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("required", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("name", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("autoComplete", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("minLength", record(vec![("type", Value::string("number")), ("empty", Value::string("fallback"))])), ("maxLength", record(vec![("type", Value::string("number")), ("empty", Value::string("fallback"))])), ("inputMode", record(vec![("type", Value::string("enum")), ("empty", Value::string("fallback")), ("options", Value::Array(Rc::new(vec![Value::string("none"), Value::string("text"), Value::string("tel"), Value::string("url"), Value::string("email"), Value::string("numeric"), Value::string("decimal"), Value::string("search")])))])), ("enterKeyHint", record(vec![("type", Value::string("enum")), ("empty", Value::string("fallback")), ("options", Value::Array(Rc::new(vec![Value::string("enter"), Value::string("done"), Value::string("go"), Value::string("next"), Value::string("previous"), Value::string("search"), Value::string("send")])))])), ("ariaLabel", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("labelledBy", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("describedBy", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))]))]); let v_7075695f6e61746976655f32 = props_define(&owner, v_3c636170747572653a7075695f6e61746976655f313e.clone()); capture_effect(&owner, "props", record(vec![("op", Value::string("define")), ("specMap", v_3c636170747572653a7075695f6e61746976655f313e.clone())]));    v_7075695f6e61746976655f32 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":56:3
        { let v_7075695f6e61746976655f33 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f343e = record(vec![("defaultValue", Value::string("")), ("disabled", Value::boolean(false)), ("readOnly", Value::boolean(false)), ("placeholder", Value::string("")), ("required", Value::boolean(false)), ("name", Value::string("")), ("autoComplete", Value::string("")), ("minLength", Value::number(-(1.0_f64))), ("maxLength", Value::number(-(1.0_f64))), ("ariaLabel", Value::string("")), ("labelledBy", Value::string("")), ("describedBy", Value::string(""))]); let v_7075695f6e61746976655f35 = set_defaults(&owner, v_3c636170747572653a7075695f6e61746976655f343e.clone()); capture_effect(&owner, "props", record(vec![("op", Value::string("setDefaults")), ("partial", v_3c636170747572653a7075695f6e61746976655f343e.clone())]));    v_7075695f6e61746976655f35 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":71:9
        v_636f6e74726f6c.initialize(native_builtin_hook(&owner, "asTextControl", || text_setup(&owner)));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":72:9
        v_666f63757361626c65.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":73:3
        { let v_7075695f6e61746976655f36 = v_666f63757361626c65.read(); focus_setup_config(&(v_7075695f6e61746976655f36.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":74:9
        v_76616c75655f5f35333132623533343531383633613830.initialize({ let v_7075695f6e61746976655f37 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f383e = Value::string("value"); let v_3c636170747572653a7075695f6e61746976655f393e = Value::string(""); let v_7075695f6e61746976655f3130 = create_state(&owner, "string", v_3c636170747572653a7075695f6e61746976655f383e.clone(), v_3c636170747572653a7075695f6e61746976655f393e.clone(), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3130.clone());    v_7075695f6e61746976655f3130 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":75:9
        v_64697361626c6564.initialize({ let v_7075695f6e61746976655f3131 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f31323e = Value::string("disabled"); let v_3c636170747572653a7075695f6e61746976655f31333e = Value::boolean(false); let v_7075695f6e61746976655f3134 = create_state(&owner, "bool", v_3c636170747572653a7075695f6e61746976655f31323e.clone(), Value::boolean((v_3c636170747572653a7075695f6e61746976655f31333e.clone()).bool()), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3134.clone());    v_7075695f6e61746976655f3134 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":76:9
        v_726561644f6e6c79.initialize({ let v_7075695f6e61746976655f3135 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f31363e = Value::string("readOnly"); let v_3c636170747572653a7075695f6e61746976655f31373e = Value::boolean(false); let v_7075695f6e61746976655f3138 = create_state(&owner, "bool", v_3c636170747572653a7075695f6e61746976655f31363e.clone(), Value::boolean((v_3c636170747572653a7075695f6e61746976655f31373e.clone()).bool()), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3138.clone());    v_7075695f6e61746976655f3138 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":77:9
        v_636f6d706f73696e67.initialize({ let v_7075695f6e61746976655f3139 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f32303e = Value::string("composing"); let v_3c636170747572653a7075695f6e61746976655f32313e = Value::boolean(false); let v_7075695f6e61746976655f3232 = create_state(&owner, "bool", v_3c636170747572653a7075695f6e61746976655f32303e.clone(), Value::boolean((v_3c636170747572653a7075695f6e61746976655f32313e.clone()).bool()), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3232.clone());    v_7075695f6e61746976655f3232 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":78:9
        v_617269614c6162656c.initialize({ let v_7075695f6e61746976655f3233 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f32343e = Value::string("inputAriaLabel"); let v_3c636170747572653a7075695f6e61746976655f32353e = Value::string(""); let v_7075695f6e61746976655f3236 = create_state(&owner, "string", v_3c636170747572653a7075695f6e61746976655f32343e.clone(), v_3c636170747572653a7075695f6e61746976655f32353e.clone(), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3236.clone());    v_7075695f6e61746976655f3236 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":79:9
        v_6c6162656c6c65644279.initialize({ let v_7075695f6e61746976655f3237 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f32383e = Value::string("inputLabelledBy"); let v_3c636170747572653a7075695f6e61746976655f32393e = Value::string(""); let v_7075695f6e61746976655f3330 = create_state(&owner, "string", v_3c636170747572653a7075695f6e61746976655f32383e.clone(), v_3c636170747572653a7075695f6e61746976655f32393e.clone(), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3330.clone());    v_7075695f6e61746976655f3330 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":80:9
        v_6465736372696265644279.initialize({ let v_7075695f6e61746976655f3331 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f33323e = Value::string("inputDescribedBy"); let v_3c636170747572653a7075695f6e61746976655f33333e = Value::string(""); let v_7075695f6e61746976655f3334 = create_state(&owner, "string", v_3c636170747572653a7075695f6e61746976655f33323e.clone(), v_3c636170747572653a7075695f6e61746976655f33333e.clone(), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f3334.clone());    v_7075695f6e61746976655f3334 } });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":81:9
        let v_666f6375736564 = (v_666f63757361626c65.read()).member("focused", false);
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":82:9
        let v_666f63757356697369626c65 = (v_666f63757361626c65.read()).member("focusVisible", false);
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":83:7
        v_63757272656e7456696577.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":84:7
        v_7669657745706f63685f5f66393766306532316130323536623461.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":86:3
        { let v_7075695f6e61746976655f3335 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f33363e = Value::string("value"); let v_3c636170747572653a7075695f6e61746976655f33373e = v_76616c75655f5f35333132623533343531383633613830.read(); let v_7075695f6e61746976655f3338 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f33363e.clone(), v_3c636170747572653a7075695f6e61746976655f33373e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f33363e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f33373e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f33373e.clone()); v_7075695f6e61746976655f3338 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":87:3
        { let v_7075695f6e61746976655f3339 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f34303e = Value::string("disabled"); let v_3c636170747572653a7075695f6e61746976655f34313e = v_64697361626c6564.read(); let v_7075695f6e61746976655f3432 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f34303e.clone(), v_3c636170747572653a7075695f6e61746976655f34313e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f34303e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f34313e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f34313e.clone()); v_7075695f6e61746976655f3432 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":88:3
        { let v_7075695f6e61746976655f3433 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f34343e = Value::string("readOnly"); let v_3c636170747572653a7075695f6e61746976655f34353e = v_726561644f6e6c79.read(); let v_7075695f6e61746976655f3436 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f34343e.clone(), v_3c636170747572653a7075695f6e61746976655f34353e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f34343e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f34353e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f34353e.clone()); v_7075695f6e61746976655f3436 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":89:3
        { let v_7075695f6e61746976655f3437 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f34383e = Value::string("focused"); let v_3c636170747572653a7075695f6e61746976655f34393e = v_666f6375736564.clone(); let v_7075695f6e61746976655f3530 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f34383e.clone(), v_3c636170747572653a7075695f6e61746976655f34393e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f34383e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f34393e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f34393e.clone()); v_7075695f6e61746976655f3530 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":90:3
        { let v_7075695f6e61746976655f3531 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f35323e = Value::string("focusVisible"); let v_3c636170747572653a7075695f6e61746976655f35333e = v_666f63757356697369626c65.clone(); let v_7075695f6e61746976655f3534 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f35323e.clone(), v_3c636170747572653a7075695f6e61746976655f35333e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f35323e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f35333e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f35333e.clone()); v_7075695f6e61746976655f3534 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":91:3
        { let v_7075695f6e61746976655f3535 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f35363e = Value::string("composing"); let v_3c636170747572653a7075695f6e61746976655f35373e = v_636f6d706f73696e67.read(); let v_7075695f6e61746976655f3538 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f35363e.clone(), v_3c636170747572653a7075695f6e61746976655f35373e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f35363e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f35373e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f35373e.clone()); v_7075695f6e61746976655f3538 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":92:3
        { let v_7075695f6e61746976655f3539 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f36323e = Value::string("focusSelf"); let v_3c636170747572653a7075695f6e61746976655f36333e = {  let v_666f63757361626c65 = v_666f63757361626c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_6f7074696f6e73: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":93:5
            if !(({ let v_7075695f6e61746976655f3630 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3630.clone()).get()).bool()) }).bool()) {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":93:26
                { let v_7075695f6e61746976655f3631 = v_666f63757361626c65.read(); focus_request(&(v_7075695f6e61746976655f3631.clone()).owner(), v_6f7074696f6e73.clone(), false) };
            }
            Value::Undefined
        })) }; let v_7075695f6e61746976655f3634 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f36323e.clone(), v_3c636170747572653a7075695f6e61746976655f36333e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f36323e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f36333e.clone())]));    v_7075695f6e61746976655f3634 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":95:3
        { let v_7075695f6e61746976655f3635 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f36373e = Value::string("blurSelf"); let v_3c636170747572653a7075695f6e61746976655f36383e = {  let v_666f63757361626c65 = v_666f63757361626c65.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":95:39
            return { let v_7075695f6e61746976655f3636 = v_666f63757361626c65.read(); focus_blur(&(v_7075695f6e61746976655f3636.clone()).owner()) };
        })) }; let v_7075695f6e61746976655f3639 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f36373e.clone(), v_3c636170747572653a7075695f6e61746976655f36383e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f36373e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f36383e.clone())]));    v_7075695f6e61746976655f3639 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":96:3
        { let v_7075695f6e61746976655f3730 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3130363e = Value::string("resetValue"); let v_3c636170747572653a7075695f6e61746976655f3130373e = ({  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
            let v_6e6578745f5f66663730326563316261626232386433: Value = argument_0;
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":97:11
            let v_72756e5f5f36613733663336306433363665653839 = { let value = (v_63757272656e7456696577.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":98:5
            if { let pui_native_71=!((v_72756e5f5f36613733663336306433363665653839.clone()).truthy()); if !(pui_native_71) {{ let pui_native_73=(Value::boolean(!strict_equal(&(v_6e6578745f5f66663730326563316261626232386433.clone()), &(Value::Undefined)))).bool(); if (pui_native_73) {(Value::boolean(!strict_equal(&(typeof_value(&(v_6e6578745f5f66663730326563316261626232386433.clone()))), &(Value::string("string"))))).bool()} else {pui_native_73} }} else {pui_native_71} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":98:67
                return false;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":99:11
            let v_65706f63685f5f32663363346131656430346666306533: f64 = { let cell=v_7669657745706f63685f5f66393766306532316130323536623461.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":100:11
            let v_6163636570746564: bool = ({ let v_7075695f6e61746976655f3739 = v_636f6e74726f6c.read(); Value::boolean((text_reset_value(&(v_7075695f6e61746976655f3739.clone()).owner(), v_6e6578745f5f66663730326563316261626232386433.clone())).bool()) }).bool();
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":101:5
            if { let pui_native_80=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f36613733663336306433363665653839.clone())))).bool(); if !(pui_native_80) {{ let pui_native_84={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_85=v_65706f63685f5f32663363346131656430346666306533.clone(); !(pui_native_84==pui_native_85) }} else {pui_native_80} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":101:53
                return false;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":102:5
            { let v_7075695f6e61746976655f3836 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3836.clone(), Value::boolean({ let pui_native_87=({ let v_7075695f6e61746976655f3839 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3839.clone()).owner()) }).member("composing", true); if pui_native_87.nullish() {false} else {(pui_native_87).bool()} }), Value::string("reason: input reset composition")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":103:5
            if { let pui_native_90=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f36613733663336306433363665653839.clone())))).bool(); if !(pui_native_90) {{ let pui_native_94={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_95=v_65706f63685f5f32663363346131656430346666306533.clone(); !(pui_native_94==pui_native_95) }} else {pui_native_90} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":103:53
                return false;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":106:5
            { let v_7075695f6e61746976655f3936 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f3936.clone(), { let left = ({ let v_7075695f6e61746976655f3937 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3937.clone()).owner()) }).member("value", true); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input reset value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":107:5
            return { let pui_native_98={ let pui_native_100=(Value::boolean(strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f36613733663336306433363665653839.clone())))).bool(); if (pui_native_100) {{ let pui_native_104={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_105=v_65706f63685f5f32663363346131656430346666306533.clone(); (pui_native_104==pui_native_105) }} else {pui_native_100} }; if (pui_native_98) {v_6163636570746564.clone()} else {pui_native_98} };
        }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f313038 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3130363e.clone(), v_3c636170747572653a7075695f6e61746976655f3130373e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3130363e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3130373e.clone())]));    v_7075695f6e61746976655f313038 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":109:3
        { let v_7075695f6e61746976655f313039 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3131303e = Value::string("valueChange"); let v_3c636170747572653a7075695f6e61746976655f3131313e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f313132 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3131303e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3131303e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3131313e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3131303e.clone());  v_7075695f6e61746976655f313132 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":110:3
        { let v_7075695f6e61746976655f313133 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3131343e = Value::string("change"); let v_3c636170747572653a7075695f6e61746976655f3131353e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f313136 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3131343e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3131343e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3131353e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3131343e.clone());  v_7075695f6e61746976655f313136 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":111:3
        { let v_7075695f6e61746976655f313137 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3131383e = Value::string("compositionStart"); let v_3c636170747572653a7075695f6e61746976655f3131393e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f313230 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3131383e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3131383e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3131393e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3131383e.clone());  v_7075695f6e61746976655f313230 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":112:3
        { let v_7075695f6e61746976655f313231 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3132323e = Value::string("compositionUpdate"); let v_3c636170747572653a7075695f6e61746976655f3132333e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f313234 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3132323e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3132323e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3132333e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3132323e.clone());  v_7075695f6e61746976655f313234 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":113:3
        { let v_7075695f6e61746976655f313235 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3132363e = Value::string("compositionEnd"); let v_3c636170747572653a7075695f6e61746976655f3132373e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f313238 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3132363e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3132363e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3132373e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3132363e.clone());  v_7075695f6e61746976655f313238 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":115:3
        { let v_7075695f6e61746976655f313239 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f313239.clone()).owner(), Value::string("textbox")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":116:3
        { let v_7075695f6e61746976655f313330 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f313330.clone()).owner(), "name", v_617269614c6162656c.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":117:3
        { let v_7075695f6e61746976655f313331 = v_61636365737369626c65.clone(); { let v_3c636170747572653a7075695f6e61746976655f3133323e = Value::string("disabled"); let v_3c636170747572653a7075695f6e61746976655f3133333e = v_64697361626c6564.read(); let v_7075695f6e61746976655f313334 = accessible_state(&(v_7075695f6e61746976655f313331.clone()).owner(), v_3c636170747572653a7075695f6e61746976655f3133323e.clone(), v_3c636170747572653a7075695f6e61746976655f3133333e.clone(), Value::Undefined); capture_effect(&owner, "state", record(vec![("op", Value::string("a11y.state")), ("key", v_3c636170747572653a7075695f6e61746976655f3133323e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f3133333e.clone()), ("options", Value::Undefined)]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f3133333e.clone()); v_7075695f6e61746976655f313334 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":118:3
        { let v_7075695f6e61746976655f313335 = v_61636365737369626c65.clone(); { let v_3c636170747572653a7075695f6e61746976655f3133363e = Value::string("readOnly"); let v_3c636170747572653a7075695f6e61746976655f3133373e = v_726561644f6e6c79.read(); let v_7075695f6e61746976655f313338 = accessible_state(&(v_7075695f6e61746976655f313335.clone()).owner(), v_3c636170747572653a7075695f6e61746976655f3133363e.clone(), v_3c636170747572653a7075695f6e61746976655f3133373e.clone(), Value::Undefined); capture_effect(&owner, "state", record(vec![("op", Value::string("a11y.state")), ("key", v_3c636170747572653a7075695f6e61746976655f3133363e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f3133373e.clone()), ("options", Value::Undefined)]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f3133373e.clone()); v_7075695f6e61746976655f313338 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":119:3
        { let v_7075695f6e61746976655f313339 = v_61636365737369626c65.clone(); accessible_relation(&(v_7075695f6e61746976655f313339.clone()).owner(), Value::string("labelledBy"), record(vec![("target", v_6c6162656c6c65644279.read())])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":120:3
        { let v_7075695f6e61746976655f313430 = v_61636365737369626c65.clone(); accessible_relation(&(v_7075695f6e61746976655f313430.clone()).owner(), Value::string("describedBy"), record(vec![("target", v_6465736372696265644279.read())])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":122:9
        v_73796e63.initialize({  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_666f63757361626c65 = v_666f63757361626c65.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726561644f6e6c79 = v_726561644f6e6c79.clone(); let v_617269614c6162656c = v_617269614c6162656c.clone(); let v_6c6162656c6c65644279 = v_6c6162656c6c65644279.clone(); let v_6465736372696265644279 = v_6465736372696265644279.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_70726f70735f5f34383235376530636234353738623236: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":123:11
            let v_636f6e74726f6c6c6564: bool = (Value::boolean(strict_equal(&(typeof_value(&((v_70726f70735f5f34383235376530636234353738623236.clone()).member("value", false)))), &(Value::string("string"))))).bool();
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":124:11
            let v_6e65787444697361626c6564: bool = { let pui_native_143=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("disabled", false); if pui_native_143.nullish() {false} else {(pui_native_143).bool()} };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":125:5
            { let v_7075695f6e61746976655f313435 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313435.clone(), Value::boolean(v_6e65787444697361626c6564.clone()), Value::string("reason: input sync disabled")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":126:5
            { let v_7075695f6e61746976655f313436 = v_726561644f6e6c79.read(); state_set(v_7075695f6e61746976655f313436.clone(), Value::boolean({ let pui_native_147=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("readOnly", false); if pui_native_147.nullish() {false} else {(pui_native_147).bool()} }), Value::string("reason: input sync readonly")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":127:5
            { let v_7075695f6e61746976655f313439 = v_617269614c6162656c.read(); state_set(v_7075695f6e61746976655f313439.clone(), { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("ariaLabel", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync aria label")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":128:5
            { let v_7075695f6e61746976655f313530 = v_6c6162656c6c65644279.read(); state_set(v_7075695f6e61746976655f313530.clone(), { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("labelledBy", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync labelledby")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":129:5
            { let v_7075695f6e61746976655f313531 = v_6465736372696265644279.read(); state_set(v_7075695f6e61746976655f313531.clone(), { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("describedBy", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync describedby")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":130:5
            { let v_7075695f6e61746976655f313532 = v_666f63757361626c65.read(); focus_disabled(&(v_7075695f6e61746976655f313532.clone()).owner(), Value::boolean(v_6e65787444697361626c6564.clone())) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":131:5
            { let v_7075695f6e61746976655f313533 = v_636f6e74726f6c.read(); text_sync(&(v_7075695f6e61746976655f313533.clone()).owner(), record(vec![("valueMode", if (Value::boolean(v_636f6e74726f6c6c6564.clone())).truthy() { Value::string("controlled") } else { Value::string("uncontrolled") }), ("value", if (Value::boolean(v_636f6e74726f6c6c6564.clone())).truthy() { (v_70726f70735f5f34383235376530636234353738623236.clone()).member("value", false) } else { Value::Undefined }), ("defaultValue", { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("defaultValue", false); if left.nullish() { Value::string("") } else { left } }), ("disabled", Value::boolean(v_6e65787444697361626c6564.clone())), ("readOnly", Value::boolean({ let pui_native_154=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("readOnly", false); if pui_native_154.nullish() {false} else {(pui_native_154).bool()} })), ("placeholder", { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("placeholder", false); if left.nullish() { Value::string("") } else { left } }), ("required", Value::boolean({ let pui_native_156=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("required", false); if pui_native_156.nullish() {false} else {(pui_native_156).bool()} })), ("name", { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("name", false); if left.nullish() { Value::string("") } else { left } }), ("autoComplete", { let left = (v_70726f70735f5f34383235376530636234353738623236.clone()).member("autoComplete", false); if left.nullish() { Value::string("") } else { left } }), ("minLength", Value::number({ let pui_native_158=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("minLength", false); if pui_native_158.nullish() {-(1.0_f64)} else {(pui_native_158).num()} })), ("maxLength", Value::number({ let pui_native_160=(v_70726f70735f5f34383235376530636234353738623236.clone()).member("maxLength", false); if pui_native_160.nullish() {-(1.0_f64)} else {(pui_native_160).num()} })), ("inputMode", (v_70726f70735f5f34383235376530636234353738623236.clone()).member("inputMode", false)), ("enterKeyHint", (v_70726f70735f5f34383235376530636234353738623236.clone()).member("enterKeyHint", false))])) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":146:5
            { let v_7075695f6e61746976655f313632 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f313632.clone(), { let left = ({ let v_7075695f6e61746976655f313633 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f313633.clone()).owner()) }).member("value", true); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync value")) };
            Value::Undefined
        })) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":148:3
        { let v_7075695f6e61746976655f313634 = v_6465665f5f61326333376636353034653363313533.clone(); register_life(&owner, "created", {  let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f32313164343061326431393661613333: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":148:36
            return v_73796e63.read().call_authored(vec![{ let v_7075695f6e61746976655f313635 = v_72756e5f5f32313164343061326431393661613333.clone(); read_props(&(v_7075695f6e61746976655f313635.clone()).owner(), false) }]);
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":149:3
        { let v_7075695f6e61746976655f313636 = v_6465665f5f61326333376636353034653363313533.clone(); register_life(&owner, "mounted", {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f38343335633434383964613932626137: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":150:5
            { let value = v_72756e5f5f38343335633434383964613932626137.clone(); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":151:11
            let v_65706f63685f5f38376137376563643334396136363038: f64 = { let cell=v_7669657745706f63685f5f66393766306532316130323536623461.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":154:5
            { let v_7075695f6e61746976655f313637 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f313637.clone(), Value::boolean({ let pui_native_168=({ let v_7075695f6e61746976655f313730 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f313730.clone()).owner()) }).member("composing", true); if pui_native_168.nullish() {false} else {(pui_native_168).bool()} }), Value::string("reason: input mounted composition")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":155:5
            if { let pui_native_171=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f38343335633434383964613932626137.clone())))).bool(); if !(pui_native_171) {{ let pui_native_175={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_176=v_65706f63685f5f38376137376563643334396136363038.clone(); !(pui_native_175==pui_native_176) }} else {pui_native_171} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":155:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":156:5
            { let v_7075695f6e61746976655f313737 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f313737.clone(), { let left = ({ let v_7075695f6e61746976655f313738 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f313738.clone()).owner()) }).member("value", true); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input mounted value")) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":158:3
        { let v_7075695f6e61746976655f313739 = v_6465665f5f61326333376636353034653363313533.clone(); register_life(&owner, "unmounted", {  let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":159:5
            { let value = Value::Json(Json::Null); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":160:5
            Value::number({ let cell=v_7669657745706f63685f5f66393766306532316130323536623461.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; previous });
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":162:5
            { let v_7075695f6e61746976655f313830 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f313830.clone(), Value::boolean(false), Value::string("reason: input view composition retired")) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":164:3
        { let v_7075695f6e61746976655f313831 = v_6465665f5f61326333376636353034653363313533.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":165:5
            { let value = Value::Json(Json::Null); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":166:5
            Value::number({ let cell=v_7669657745706f63685f5f66393766306532316130323536623461.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; previous });
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":168:3
        { let v_7075695f6e61746976655f313832 = v_6465665f5f61326333376636353034653363313533.clone(); { let v_3c636170747572653a7075695f6e61746976655f3138333e = Value::Array(Rc::new(vec![Value::string("value"), Value::string("defaultValue"), Value::string("disabled"), Value::string("readOnly"), Value::string("placeholder"), Value::string("required"), Value::string("name"), Value::string("autoComplete"), Value::string("minLength"), Value::string("maxLength"), Value::string("inputMode"), Value::string("enterKeyHint"), Value::string("ariaLabel"), Value::string("labelledBy"), Value::string("describedBy")])); let v_3c636170747572653a7075695f6e61746976655f3138343e = {  let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_5f72756e: Value = arg(&arguments, 0);
            let v_6e6578745f5f37346334353638383464343836646438: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":186:21
            return v_73796e63.read().call_authored(vec![v_6e6578745f5f37346334353638383464343836646438.clone()]);
        })) }; let v_7075695f6e61746976655f313835 = watch_props(&owner, Some(v_3c636170747572653a7075695f6e61746976655f3138333e.clone()), false, v_3c636170747572653a7075695f6e61746976655f3138343e.clone()); capture_effect(&owner, "props", record(vec![("op", Value::string("watch")), ("keys", v_3c636170747572653a7075695f6e61746976655f3138333e.clone()), ("off", v_7075695f6e61746976655f313835.clone())])); capture_disposer(&owner, "props", v_7075695f6e61746976655f313835.clone());   v_7075695f6e61746976655f313835 } };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":189:3
        { let v_7075695f6e61746976655f313836 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f313836.clone()).owner(), Value::string("input"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f61373164663264393265316231303835: Value = arg(&arguments, 0);
            let v_6576656e745f5f35653364623861346232613634383063: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":190:11
            let v_65706f63685f5f37663837316133616262613063353931: f64 = { let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":191:5
            { let v_7075695f6e61746976655f313837 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f313837.clone(), { let left = ({ let v_7075695f6e61746976655f313838 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f313838.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f35653364623861346232613634383063.clone()).member("value", false) } else { left } }, Value::string("reason: input value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":192:5
            if { let pui_native_189=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f61373164663264393265316231303835.clone())))).bool(); if !(pui_native_189) {{ let pui_native_193={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_194=v_65706f63685f5f37663837316133616262613063353931.clone(); !(pui_native_193==pui_native_194) }} else {pui_native_189} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":192:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":193:5
            { let v_7075695f6e61746976655f313935 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f313935.clone(), Value::boolean(((v_6576656e745f5f35653364623861346232613634383063.clone()).member("composing", false)).bool()), Value::string("reason: input composing")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":194:5
            if { let pui_native_196=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f61373164663264393265316231303835.clone())))).bool(); if !(pui_native_196) {{ let pui_native_200={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_201=v_65706f63685f5f37663837316133616262613063353931.clone(); !(pui_native_200==pui_native_201) }} else {pui_native_196} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":194:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":195:11
            let v_64657461696c5f5f36366331366262366166636338326466 = freeze(record(vec![("value", (v_6576656e745f5f35653364623861346232613634383063.clone()).member("value", false)), ("composing", Value::boolean(((v_6576656e745f5f35653364623861346232613634383063.clone()).member("composing", false)).bool())), ("data", (v_6576656e745f5f35653364623861346232613634383063.clone()).member("data", false)), ("inputType", (v_6576656e745f5f35653364623861346232613634383063.clone()).member("inputType", false))]));
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":201:5
            { let v_7075695f6e61746976655f323032 = v_72756e5f5f61373164663264393265316231303835.clone(); emit(&(v_7075695f6e61746976655f323032.clone()).owner(), Value::string("valueChange"), v_64657461696c5f5f36366331366262366166636338326466.clone()) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":203:3
        { let v_7075695f6e61746976655f323033 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f323033.clone()).owner(), Value::string("change"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f32356264343734363433636265316539: Value = arg(&arguments, 0);
            let v_6576656e745f5f39633662303030393339343633633464: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":204:11
            let v_65706f63685f5f62323430376134323639636538363764: f64 = { let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":205:5
            { let v_7075695f6e61746976655f323034 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f323034.clone(), { let left = ({ let v_7075695f6e61746976655f323035 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f323035.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f39633662303030393339343633633464.clone()).member("value", false) } else { left } }, Value::string("reason: input change value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":206:5
            if { let pui_native_206=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f32356264343734363433636265316539.clone())))).bool(); if !(pui_native_206) {{ let pui_native_210={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_211=v_65706f63685f5f62323430376134323639636538363764.clone(); !(pui_native_210==pui_native_211) }} else {pui_native_206} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":206:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":207:5
            { let v_7075695f6e61746976655f323132 = v_72756e5f5f32356264343734363433636265316539.clone(); emit(&(v_7075695f6e61746976655f323132.clone()).owner(), Value::string("change"), freeze(record(vec![("value", (v_6576656e745f5f39633662303030393339343633633464.clone()).member("value", false))]))) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":209:9
        v_656d6974436f6d706f736974696f6e.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f34393035303033313436323564626138: Value = arg(&arguments, 0);
            let v_6576656e744e616d65: Value = arg(&arguments, 1);
            let v_6576656e745f5f32316262663663636430643236343765: Value = arg(&arguments, 2);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":214:11
            let v_64657461696c5f5f34346438343638393031663664353133 = freeze(record(vec![("value", (v_6576656e745f5f32316262663663636430643236343765.clone()).member("value", false)), ("data", (v_6576656e745f5f32316262663663636430643236343765.clone()).member("data", false))]));
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":215:5
            { let v_7075695f6e61746976655f323133 = v_72756e5f5f34393035303033313436323564626138.clone(); emit(&(v_7075695f6e61746976655f323133.clone()).owner(), v_6576656e744e616d65.clone(), v_64657461696c5f5f34346438343638393031663664353133.clone()) };
            Value::Undefined
        })) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":217:3
        { let v_7075695f6e61746976655f323134 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f323134.clone()).owner(), Value::string("compositionstart"), {  let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f66656362663861656165373632666162: Value = arg(&arguments, 0);
            let v_6576656e745f5f64383734366134333265633237613961: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":218:11
            let v_65706f63685f5f63613362303863633734643037343863: f64 = { let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":219:5
            { let v_7075695f6e61746976655f323135 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f323135.clone(), Value::boolean(true), Value::string("reason: input composition start")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":220:5
            if { let pui_native_216=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f66656362663861656165373632666162.clone())))).bool(); if !(pui_native_216) {{ let pui_native_220={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_221=v_65706f63685f5f63613362303863633734643037343863.clone(); !(pui_native_220==pui_native_221) }} else {pui_native_216} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":220:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":221:5
            v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f66656362663861656165373632666162.clone(), Value::string("compositionStart"), v_6576656e745f5f64383734366134333265633237613961.clone()]);
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":223:3
        { let v_7075695f6e61746976655f323232 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f323232.clone()).owner(), Value::string("compositionupdate"), {  let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f62353236386666346163633537653764: Value = arg(&arguments, 0);
            let v_6576656e745f5f32396136376233633937303131313461: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":223:51
            return v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f62353236386666346163633537653764.clone(), Value::string("compositionUpdate"), v_6576656e745f5f32396136376233633937303131313461.clone()]);
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":224:3
        { let v_7075695f6e61746976655f323233 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f323233.clone()).owner(), Value::string("compositionend"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f35333132623533343531383633613830 = v_76616c75655f5f35333132623533343531383633613830.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f66393766306532316130323536623461 = v_7669657745706f63685f5f66393766306532316130323536623461.clone(); let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f35643966376562356630613434366336: Value = arg(&arguments, 0);
            let v_6576656e745f5f63366162373461376233303565346139: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":225:11
            let v_65706f63685f5f39326335356335616363653037383336: f64 = { let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":226:5
            { let v_7075695f6e61746976655f323234 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f323234.clone(), Value::boolean(false), Value::string("reason: input composition end")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":227:5
            if { let pui_native_225=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f35643966376562356630613434366336.clone())))).bool(); if !(pui_native_225) {{ let pui_native_229={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_230=v_65706f63685f5f39326335356335616363653037383336.clone(); !(pui_native_229==pui_native_230) }} else {pui_native_225} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":227:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":228:5
            { let v_7075695f6e61746976655f323331 = v_76616c75655f5f35333132623533343531383633613830.read(); state_set(v_7075695f6e61746976655f323331.clone(), { let left = ({ let v_7075695f6e61746976655f323332 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f323332.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f63366162373461376233303565346139.clone()).member("value", false) } else { left } }, Value::string("reason: input composition end value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":229:5
            if { let pui_native_233=(Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f35643966376562356630613434366336.clone())))).bool(); if !(pui_native_233) {{ let pui_native_237={ let value=(v_7669657745706f63685f5f66393766306532316130323536623461.read()).borrow().clone(); value }; let pui_native_238=v_65706f63685f5f39326335356335616363653037383336.clone(); !(pui_native_237==pui_native_238) }} else {pui_native_233} } {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":229:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":230:5
            v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f35643966376562356630613434366336.clone(), Value::string("compositionEnd"), v_6576656e745f5f63366162373461376233303565346139.clone()]);
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":232:3
        return { let callable={   Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":232:16
            return Value::Json(Json::Null);
        })) }; match callable {
      Value::NativeFunction{identity,render} => capture_renderer(Value::NativeFunction{identity,render:render.clone()},render),
      Value::Renderer(adapter) => Value::Renderer(adapter),
      Value::Function(callback) => capture_renderer(Value::Function(callback.clone()),Rc::new(move |arguments:Vec<Value>| { { let pui_native_239 = callback(arguments); if matches!(&pui_native_239, Value::Json(Json::Null)) { native::gpui_native_owner::NativeSnapshot::empty() } else { native::gpui_native_owner::NativeSnapshot::invalid(pui_native_239) } } })),
      _ => panic!("checked setup result must be a callable")
    } };
    })) }, Value::Undefined, Value::Undefined)
}
  #[derive(Clone)]
  pub struct TextFieldHandle { pub owner: OwnerRef }
  impl TextFieldHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("value").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("readOnly").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("composing").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_6(&self, argument_0: Option<GeneratedData3>, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![argument_0.map(|value| Value::json(serde_json::to_value(value).expect("portable argument"))).unwrap_or(Value::Undefined)];
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
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("blurSelf").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("blurSelf", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_null()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_8(&self, argument_0: Option<String>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![argument_0.map(|value| Value::json(serde_json::to_value(value).expect("portable argument"))).unwrap_or(Value::Undefined)];
        let callback = self.owner.borrow().exposes.get("resetValue").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("resetValue", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn state_14(&self) -> Result<f64, Value> { self.state("viewEpoch").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_22(&self, argument_0: f64, argument_1: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("selectRange").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectRange", &arguments);
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
        let callback = self.owner.borrow().exposes.get("selectAll").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectAll", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_24(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("requestSubmit").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestSubmit", &arguments);
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
        let callback = self.owner.borrow().exposes.get("requestCancel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestCancel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_26(&self, argument_0: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("requestAdvance").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestAdvance", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_27(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("stepText").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("stepText", &arguments);
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
  pub struct TextField { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl TextField {
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
          text_declaration(&owner, false);
          configure_native_atoms(&owner_ref, false, false);
          refresh_native_atoms(&owner_ref, window, cx);
          let _lexical_owner = enter_lexical_owner(&owner);
          let renderer = { let owner = owner.clone();  Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_6465665f5f35333637373936326264623532343666: Value = arg(&arguments, 0);
            let v_7374617465: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_7669657745706f63685f5f65636632393634613739333761306432: LexicalBinding<Value> = lexical_binding();
            let v_72756e5f5f35306463623264633165633462396466: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6d6f756e746564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_616c6c6f776564: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_72657175657374: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool) -> bool>> = lexical_binding();
            let v_73656c65637452616e6765: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64, f64) -> bool>> = lexical_binding();
            let v_7265706c6163654472616674: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value) -> bool>> = lexical_binding();
            let v_7374657054657874: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> bool>> = lexical_binding();
            // Source "frontend/proto-ui/src/text-field.proto.ts":26:11
            let v_696e68657269746564 = authored_hook_0(definition_capability(&owner), Value::Undefined);
            // Source "frontend/proto-ui/src/text-field.proto.ts":27:11
            v_7374617465.initialize((v_696e68657269746564.clone()).member("stateHandles", false));
            // Source "frontend/proto-ui/src/text-field.proto.ts":28:5
            if !((v_7374617465.read()).truthy()) {
                // Source "frontend/proto-ui/src/text-field.proto.ts":28:17
                throw_value(record(vec![("name", Value::string("Error")), ("message", (Value::string("Input root state projection is required")).error_message_value())]));
            }
            // Source "frontend/proto-ui/src/text-field.proto.ts":29:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/text-field.proto.ts":30:11
            v_7669657745706f63685f5f65636632393634613739333761306432.initialize({ let v_7075695f6e61746976655f323430 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3234313e = Value::string("viewEpoch"); let v_3c636170747572653a7075695f6e61746976655f3234323e = Value::number(0.0_f64); let v_7075695f6e61746976655f323433 = create_state(&owner, "discrete", v_3c636170747572653a7075695f6e61746976655f3234313e.clone(), Value::number((v_3c636170747572653a7075695f6e61746976655f3234323e.clone()).num()), Value::Undefined); capture_effect(&owner, "state", v_7075695f6e61746976655f323433.clone());    v_7075695f6e61746976655f323433 } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":31:5
            { let v_7075695f6e61746976655f323434 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3234353e = Value::string("viewEpoch"); let v_3c636170747572653a7075695f6e61746976655f3234363e = v_7669657745706f63685f5f65636632393634613739333761306432.read(); let v_7075695f6e61746976655f323437 = expose_state(&owner, v_3c636170747572653a7075695f6e61746976655f3234353e.clone(), v_3c636170747572653a7075695f6e61746976655f3234363e.clone()); capture_effect(&owner, "state", record(vec![("op", Value::string("expose.state")), ("key", v_3c636170747572653a7075695f6e61746976655f3234353e.clone()), ("handle", v_3c636170747572653a7075695f6e61746976655f3234363e.clone())]));   capture_introduced_state(&owner, v_3c636170747572653a7075695f6e61746976655f3234363e.clone()); v_7075695f6e61746976655f323437 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":32:5
            { let v_7075695f6e61746976655f323438 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3234393e = record(vec![("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selectAllOnMount", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("submitOnEnter", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("cancelOnEscape", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("advanceOnTab", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("commitOnBlur", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("searchNavigation", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("numeric", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("numericMin", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("numericMax", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("numericStep", record(vec![("type", Value::string("number")), ("default", Value::number(1.0_f64))])), ("numericDefault", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))]))]); let v_7075695f6e61746976655f323530 = props_define(&owner, v_3c636170747572653a7075695f6e61746976655f3234393e.clone()); capture_effect(&owner, "props", record(vec![("op", Value::string("define")), ("specMap", v_3c636170747572653a7075695f6e61746976655f3234393e.clone())]));    v_7075695f6e61746976655f323530 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":46:5
            { let v_7075695f6e61746976655f323531 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3235323e = Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-w-0".into(), "min-w-".into())]); let v_7075695f6e61746976655f323533 = style_use(&owner, vec![v_3c636170747572653a7075695f6e61746976655f3235323e.clone()]); capture_effect(&owner, "feedback", v_7075695f6e61746976655f323533.clone()); capture_disposer(&owner, "feedback", v_7075695f6e61746976655f323533.clone());   v_7075695f6e61746976655f323533 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":47:5
            { let v_7075695f6e61746976655f323534 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3235353e = Value::string("submit"); let v_3c636170747572653a7075695f6e61746976655f3235363e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323537 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3235353e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3235353e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3235363e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3235353e.clone());  v_7075695f6e61746976655f323537 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":48:5
            { let v_7075695f6e61746976655f323538 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3235393e = Value::string("cancel"); let v_3c636170747572653a7075695f6e61746976655f3236303e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323631 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3235393e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3235393e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3236303e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3235393e.clone());  v_7075695f6e61746976655f323631 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":49:5
            { let v_7075695f6e61746976655f323632 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3236333e = Value::string("advance"); let v_3c636170747572653a7075695f6e61746976655f3236343e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323635 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3236333e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3236333e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3236343e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3236333e.clone());  v_7075695f6e61746976655f323635 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":50:5
            { let v_7075695f6e61746976655f323636 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3236373e = Value::string("blurCommit"); let v_3c636170747572653a7075695f6e61746976655f3236383e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323639 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3236373e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3236373e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3236383e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3236373e.clone());  v_7075695f6e61746976655f323639 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":51:5
            { let v_7075695f6e61746976655f323730 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3237313e = Value::string("selectionRequest"); let v_3c636170747572653a7075695f6e61746976655f3237323e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323733 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3237313e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3237313e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3237323e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3237313e.clone());  v_7075695f6e61746976655f323733 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":52:5
            { let v_7075695f6e61746976655f323734 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3237353e = Value::string("replacementRequest"); let v_3c636170747572653a7075695f6e61746976655f3237363e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323737 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3237353e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3237353e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3237363e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3237353e.clone());  v_7075695f6e61746976655f323737 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":53:5
            { let v_7075695f6e61746976655f323738 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3237393e = Value::string("navigationRequest"); let v_3c636170747572653a7075695f6e61746976655f3238303e = record(vec![("payload", Value::string("json"))]); let v_7075695f6e61746976655f323831 = expose_event(&owner, v_3c636170747572653a7075695f6e61746976655f3237393e.clone()); capture_effect(&owner, "event", record(vec![("op", Value::string("expose.event")), ("key", v_3c636170747572653a7075695f6e61746976655f3237393e.clone()), ("spec", v_3c636170747572653a7075695f6e61746976655f3238303e.clone())]));  capture_event_key(&owner, v_3c636170747572653a7075695f6e61746976655f3237393e.clone());  v_7075695f6e61746976655f323831 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":54:9
            v_72756e5f5f35306463623264633165633462396466.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/text-field.proto.ts":55:9
            v_6d6f756e746564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/text-field.proto.ts":56:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/text-field.proto.ts":57:11
            v_616c6c6f776564.initialize({  let v_7374617465 = v_7374617465.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/text-field.proto.ts":57:36
                return { let pui_native_282={ let pui_native_284={ let pui_native_286={ let value=(v_6d6f756e746564.read()).borrow().clone(); value }; if (pui_native_286) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_286} }; if (pui_native_284) {!(({ let v_7075695f6e61746976655f323838 = (v_7374617465.read()).member("disabled", false); Value::boolean(((v_7075695f6e61746976655f323838.clone()).get()).bool()) }).bool())} else {pui_native_284} }; if (pui_native_282) {!(({ let v_7075695f6e61746976655f323839 = (v_7374617465.read()).member("composing", false); Value::boolean(((v_7075695f6e61746976655f323839.clone()).get()).bool()) }).bool())} else {pui_native_282} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":58:11
            v_72657175657374.initialize({  let v_7374617465 = v_7374617465.clone(); let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); let call:Rc<dyn Fn(Value, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool| -> bool {
                let v_6b696e64: Value = argument_0;
                let v_666f72776172645f5f63363732666563376464363033376232: bool = argument_1;
                // Source "frontend/proto-ui/src/text-field.proto.ts":59:13
                let v_63757272656e745f5f33353632396561656266306637373766 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":60:7
                if { let pui_native_290=!((v_63757272656e745f5f33353632396561656266306637373766.clone()).truthy()); if !(pui_native_290) {!((v_616c6c6f776564.read().call)())} else {pui_native_290} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":60:35
                    return false;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":61:13
                let v_64657461696c5f5f36306630663866376435353464653666 = record(vec![("value", { let v_7075695f6e61746976655f323932 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f323932.clone()).get() }), ("forward", Value::boolean(v_666f72776172645f5f63363732666563376464363033376232.clone())), ("viewEpoch", Value::number(({ let v_7075695f6e61746976655f323933 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f323933.clone()).get()).num()) }).num()))]);
                // Source "frontend/proto-ui/src/text-field.proto.ts":62:7
                if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("submit"))))).bool() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":62:30
                    { let v_7075695f6e61746976655f323936 = v_63757272656e745f5f33353632396561656266306637373766.clone(); emit(&(v_7075695f6e61746976655f323936.clone()).owner(), Value::string("submit"), v_64657461696c5f5f36306630663866376435353464653666.clone()) };
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":63:12
                    if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("cancel"))))).bool() {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":63:35
                        { let v_7075695f6e61746976655f323939 = v_63757272656e745f5f33353632396561656266306637373766.clone(); emit(&(v_7075695f6e61746976655f323939.clone()).owner(), Value::string("cancel"), v_64657461696c5f5f36306630663866376435353464653666.clone()) };
                    } else {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":64:12
                        if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("advance"))))).bool() {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":64:36
                            { let v_7075695f6e61746976655f333032 = v_63757272656e745f5f33353632396561656266306637373766.clone(); emit(&(v_7075695f6e61746976655f333032.clone()).owner(), Value::string("advance"), v_64657461696c5f5f36306630663866376435353464653666.clone()) };
                        } else {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":65:12
                            if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("blurCommit"))))).bool() {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":65:39
                                { let v_7075695f6e61746976655f333035 = v_63757272656e745f5f33353632396561656266306637373766.clone(); emit(&(v_7075695f6e61746976655f333035.clone()).owner(), Value::string("blurCommit"), v_64657461696c5f5f36306630663866376435353464653666.clone()) };
                            } else {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":66:12
                                return false;
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":67:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":71:11
            v_73656c65637452616e6765.initialize({  let v_7374617465 = v_7374617465.clone(); let v_666f637573 = v_666f637573.clone(); let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); let call:Rc<dyn Fn(f64, f64)->bool>=Rc::new(move |argument_0: f64, argument_1: f64| -> bool {
                let v_7374617274: f64 = argument_0;
                let v_656e64: f64 = argument_1;
                // Source "frontend/proto-ui/src/text-field.proto.ts":72:13
                let v_63757272656e745f5f38373939626134346665653036663939 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":73:7
                if { let pui_native_306={ let pui_native_308={ let pui_native_310=!((v_63757272656e745f5f38373939626134346665653036663939.clone()).truthy()); if !(pui_native_310) {!((v_616c6c6f776564.read().call)())} else {pui_native_310} }; if !(pui_native_308) {!((v_7374617274.clone()).is_finite())} else {pui_native_308} }; if !(pui_native_306) {!((v_656e64.clone()).is_finite())} else {pui_native_306} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":73:87
                    return false;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":74:13
                let v_6c656e677468: f64 = (({ let v_7075695f6e61746976655f333132 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f333132.clone()).get() }).member("length", false)).num();
                // Source "frontend/proto-ui/src/text-field.proto.ts":75:11
                let v_66726f6d: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new((Value::number((Value::number(v_7374617274.clone())).scalar_number().floor())).num()));
                // Source "frontend/proto-ui/src/text-field.proto.ts":76:11
                let v_746f: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new((Value::number((Value::number(v_656e64.clone())).scalar_number().floor())).num()));
                // Source "frontend/proto-ui/src/text-field.proto.ts":77:7
                if { let pui_native_313:f64={ let value=(v_66726f6d.clone()).borrow().clone(); value }; let pui_native_314:f64=0.0_f64; pui_native_313 < pui_native_314 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":77:21
                    Value::number({ let value=0.0_f64; *(v_66726f6d.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":78:7
                if { let pui_native_315:f64={ let value=(v_746f.clone()).borrow().clone(); value }; let pui_native_316:f64={ let value=(v_66726f6d.clone()).borrow().clone(); value }; pui_native_315 < pui_native_316 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":78:22
                    Value::number({ let value={ let value=(v_66726f6d.clone()).borrow().clone(); value }; *(v_746f.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":79:7
                if { let pui_native_317:f64={ let value=(v_66726f6d.clone()).borrow().clone(); value }; let pui_native_318:f64=v_6c656e677468.clone(); pui_native_317 > pui_native_318 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":79:26
                    Value::number({ let value=v_6c656e677468.clone(); *(v_66726f6d.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":80:7
                if { let pui_native_319:f64={ let value=(v_746f.clone()).borrow().clone(); value }; let pui_native_320:f64=v_6c656e677468.clone(); pui_native_319 > pui_native_320 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":80:24
                    Value::number({ let value=v_6c656e677468.clone(); *(v_746f.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":81:13
                let v_65706f63685f5f63636364666365346435623161626565: f64 = ({ let v_7075695f6e61746976655f333231 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f333231.clone()).get()).num()) }).num();
                // Source "frontend/proto-ui/src/text-field.proto.ts":82:7
                { let v_7075695f6e61746976655f333232 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f333232.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":83:7
                if { let pui_native_323=!((v_616c6c6f776564.read().call)()); if !(pui_native_323) {{ let pui_native_325=({ let v_7075695f6e61746976655f333237 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f333237.clone()).get()).num()) }).num(); let pui_native_326=v_65706f63685f5f63636364666365346435623161626565.clone(); !(pui_native_325==pui_native_326) }} else {pui_native_323} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":83:52
                    return false;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":84:7
                { let v_7075695f6e61746976655f333238 = v_63757272656e745f5f38373939626134346665653036663939.clone(); emit(&(v_7075695f6e61746976655f333238.clone()).owner(), Value::string("selectionRequest"), record(vec![("start", Value::number({ let value=(v_66726f6d.clone()).borrow().clone(); value })), ("end", Value::number({ let value=(v_746f.clone()).borrow().clone(); value })), ("viewEpoch", Value::number(v_65706f63685f5f63636364666365346435623161626565.clone()))])) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":85:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":87:5
            { let v_7075695f6e61746976655f333239 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3333303e = Value::string("selectRange"); let v_3c636170747572653a7075695f6e61746976655f3333313e = (v_73656c65637452616e6765.read()).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333332 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3333303e.clone(), v_3c636170747572653a7075695f6e61746976655f3333313e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3333303e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3333313e.clone())]));    v_7075695f6e61746976655f333332 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":88:5
            { let v_7075695f6e61746976655f333333 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3333353e = Value::string("selectAll"); let v_3c636170747572653a7075695f6e61746976655f3333363e = ({  let v_7374617465 = v_7374617465.clone(); let v_73656c65637452616e6765 = v_73656c65637452616e6765.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/text-field.proto.ts":88:42
                return (v_73656c65637452616e6765.read().call)(0.0_f64, (({ let v_7075695f6e61746976655f333334 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f333334.clone()).get() }).member("length", false)).num());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333337 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3333353e.clone(), v_3c636170747572653a7075695f6e61746976655f3333363e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3333353e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3333363e.clone())]));    v_7075695f6e61746976655f333337 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":89:5
            { let v_7075695f6e61746976655f333338 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3333393e = Value::string("requestSubmit"); let v_3c636170747572653a7075695f6e61746976655f3334303e = ({  let v_72657175657374 = v_72657175657374.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/text-field.proto.ts":89:46
                return (v_72657175657374.read().call)(Value::string("submit"), true);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333431 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3333393e.clone(), v_3c636170747572653a7075695f6e61746976655f3334303e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3333393e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3334303e.clone())]));    v_7075695f6e61746976655f333431 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":90:5
            { let v_7075695f6e61746976655f333432 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3334333e = Value::string("requestCancel"); let v_3c636170747572653a7075695f6e61746976655f3334343e = ({  let v_72657175657374 = v_72657175657374.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/text-field.proto.ts":90:46
                return (v_72657175657374.read().call)(Value::string("cancel"), false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333435 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3334333e.clone(), v_3c636170747572653a7075695f6e61746976655f3334343e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3334333e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3334343e.clone())]));    v_7075695f6e61746976655f333435 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":91:5
            { let v_7075695f6e61746976655f333436 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3334373e = Value::string("requestAdvance"); let v_3c636170747572653a7075695f6e61746976655f3334383e = ({  let v_72657175657374 = v_72657175657374.clone(); let call:Rc<dyn Fn(bool)->bool>=Rc::new(move |argument_0: bool| -> bool {
                let v_666f72776172645f5f34386362353865363766636239666565: bool = argument_0;
                // Source "frontend/proto-ui/src/text-field.proto.ts":91:63
                return (v_72657175657374.read().call)(Value::string("advance"), v_666f72776172645f5f34386362353865363766636239666565.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333439 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3334373e.clone(), v_3c636170747572653a7075695f6e61746976655f3334383e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3334373e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3334383e.clone())]));    v_7075695f6e61746976655f333439 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":92:11
            v_7265706c6163654472616674.initialize({  let v_7374617465 = v_7374617465.clone(); let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_76616c75655f5f31363432363734353431336264383230: Value = argument_0;
                // Source "frontend/proto-ui/src/text-field.proto.ts":93:13
                let v_63757272656e745f5f36393332356639343264653737386162 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":94:7
                if { let pui_native_350={ let pui_native_352=!((v_63757272656e745f5f36393332356639343264653737386162.clone()).truthy()); if !(pui_native_352) {!((v_616c6c6f776564.read().call)())} else {pui_native_352} }; if !(pui_native_350) {({ let v_7075695f6e61746976655f333534 = (v_7374617465.read()).member("readOnly", false); Value::boolean(((v_7075695f6e61746976655f333534.clone()).get()).bool()) }).bool()} else {pui_native_350} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":94:59
                    return false;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":95:7
                { let v_7075695f6e61746976655f333535 = v_63757272656e745f5f36393332356639343264653737386162.clone(); emit(&(v_7075695f6e61746976655f333535.clone()).owner(), Value::string("replacementRequest"), record(vec![("value", v_76616c75655f5f31363432363734353431336264383230.clone()), ("start", Value::number(0.0_f64)), ("end", Value::number(((v_76616c75655f5f31363432363734353431336264383230.clone()).member("length", false)).num())), ("viewEpoch", Value::number(({ let v_7075695f6e61746976655f333536 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f333536.clone()).get()).num()) }).num()))])) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":96:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":98:11
            v_7374657054657874.initialize({  let v_7374617465 = v_7374617465.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); let v_7265706c6163654472616674 = v_7265706c6163654472616674.clone(); let call:Rc<dyn Fn(f64)->bool>=Rc::new(move |argument_0: f64| -> bool {
                let v_646972656374696f6e: f64 = argument_0;
                // Source "frontend/proto-ui/src/text-field.proto.ts":99:13
                let v_63757272656e745f5f39353861663337393637356566303764 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":100:7
                if { let pui_native_357={ let pui_native_359=!((v_63757272656e745f5f39353861663337393637356566303764.clone()).truthy()); if !(pui_native_359) {!((v_616c6c6f776564.read().call)())} else {pui_native_359} }; if !(pui_native_357) {!((({ let v_7075695f6e61746976655f333631 = v_63757272656e745f5f39353861663337393637356566303764.clone(); read_props(&(v_7075695f6e61746976655f333631.clone()).owner(), false) }).member("numeric", false)).bool())} else {pui_native_357} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":100:67
                    return false;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":101:13
                let v_70726f70735f5f30383162313739353533393637353535 = { let v_7075695f6e61746976655f333632 = v_63757272656e745f5f39353861663337393637356566303764.clone(); read_props(&(v_7075695f6e61746976655f333632.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":102:13
                let v_74657874 = { let v_7075695f6e61746976655f333633 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f333633.clone()).get() };
                // Source "frontend/proto-ui/src/text-field.proto.ts":103:11
                let v_76616c75655f5f34656137363761636534323032373432: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(if { let pui_native_364=((v_74657874.clone()).member("length", false)).num(); let pui_native_365=0.0_f64; (pui_native_364==pui_native_365) } {((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericDefault", false)).num()} else {(Value::number((v_74657874.clone()).scalar_number())).num()}));
                // Source "frontend/proto-ui/src/text-field.proto.ts":104:7
                if !(({ let value=(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow().clone(); value }).is_finite()) {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":104:36
                    Value::number({ let value=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericDefault", false)).num(); *(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":105:7
                Value::number({ let value={ let pui_native_370:f64={ let value=(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow().clone(); value }; let pui_native_371:f64={ let pui_native_372:f64=v_646972656374696f6e.clone(); let pui_native_373:f64=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericStep", false)).num(); pui_native_372 * pui_native_373 }; pui_native_370 + pui_native_371 }; *(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/text-field.proto.ts":106:7
                if { let pui_native_376:f64={ let value=(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow().clone(); value }; let pui_native_377:f64=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericMin", false)).num(); pui_native_376 < pui_native_377 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":106:44
                    Value::number({ let value=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericMin", false)).num(); *(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":107:7
                if { let pui_native_382:f64={ let value=(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow().clone(); value }; let pui_native_383:f64=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericMax", false)).num(); pui_native_382 > pui_native_383 } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":107:44
                    Value::number({ let value=((v_70726f70735f5f30383162313739353533393637353535.clone()).member("numericMax", false)).num(); *(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":108:7
                return (v_7265706c6163654472616674.read().call)((Value::number({ let value=(v_76616c75655f5f34656137363761636534323032373432.clone()).borrow().clone(); value })).string_value());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/text-field.proto.ts":110:5
            { let v_7075695f6e61746976655f333838 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3338393e = Value::string("stepText"); let v_3c636170747572653a7075695f6e61746976655f3339303e = (v_7374657054657874.read()).ordinary.clone().expect("checked native data callable"); let v_7075695f6e61746976655f333931 = expose_method(&owner, v_3c636170747572653a7075695f6e61746976655f3338393e.clone(), v_3c636170747572653a7075695f6e61746976655f3339303e.clone()); capture_effect(&owner, "context", record(vec![("op", Value::string("expose.method")), ("key", v_3c636170747572653a7075695f6e61746976655f3338393e.clone()), ("fn", v_3c636170747572653a7075695f6e61746976655f3339303e.clone())]));    v_7075695f6e61746976655f333931 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":111:5
            { let v_7075695f6e61746976655f333932 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3439303e = Value::string("key.down"); let v_3c636170747572653a7075695f6e61746976655f3439313e = {  let v_7374617465 = v_7374617465.clone(); let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_72657175657374 = v_72657175657374.clone(); let v_7265706c6163654472616674 = v_7265706c6163654472616674.clone(); let v_7374657054657874 = v_7374657054657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e74: Value = arg(&arguments, 0);
                let v_6576656e745f5f33353031303463336330653765346230: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/text-field.proto.ts":112:13
                let v_63757272656e745f5f61613565306636366665343037666165 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":113:7
                if { let pui_native_393={ let pui_native_395={ let pui_native_397=!((v_63757272656e745f5f61613565306636366665343037666165.clone()).truthy()); if !(pui_native_397) {!({ let value=(v_6d6f756e746564.read()).borrow().clone(); value })} else {pui_native_397} }; if !(pui_native_395) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_395} }; if !(pui_native_393) {({ let v_7075695f6e61746976655f333939 = (v_7374617465.read()).member("disabled", false); Value::boolean(((v_7075695f6e61746976655f333939.clone()).get()).bool()) }).bool()} else {pui_native_393} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":113:69
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":114:13
                let v_6b6579 = (v_6576656e745f5f33353031303463336330653765346230.clone()).member("key", false);
                // Source "frontend/proto-ui/src/text-field.proto.ts":115:7
                if ({ let v_7075695f6e61746976655f343030 = (v_7374617465.read()).member("composing", false); Value::boolean(((v_7075695f6e61746976655f343030.clone()).get()).bool()) }).bool() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":118:9
                    if { let pui_native_401={ let pui_native_403=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Enter"))))).bool(); if !(pui_native_403) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape"))))).bool()} else {pui_native_403} }; if !(pui_native_401) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Tab"))))).bool()} else {pui_native_401} } {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":118:67
                        { let v_7075695f6e61746976655f343131 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343131.clone()) };
                    }
                    // Source "frontend/proto-ui/src/text-field.proto.ts":119:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":121:13
                let v_70726f70735f5f32373435383161613533636433393466 = { let v_7075695f6e61746976655f343132 = v_63757272656e745f5f61613565306636366665343037666165.clone(); read_props(&(v_7075695f6e61746976655f343132.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":122:13
                let v_636f6d6d616e64: bool = { let pui_native_413=(Value::boolean(strict_equal(&((v_6576656e745f5f33353031303463336330653765346230.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_413) {(Value::boolean(strict_equal(&((v_6576656e745f5f33353031303463336330653765346230.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_413} };
                // Source "frontend/proto-ui/src/text-field.proto.ts":126:7
                if { let pui_native_419={ let pui_native_421=((v_70726f70735f5f32373435383161613533636433393466.clone()).member("searchNavigation", false)).bool(); if (pui_native_421) {!(((v_6576656e745f5f33353031303463336330653765346230.clone()).member("altKey", false)).truthy())} else {pui_native_421} }; if (pui_native_419) {{ let pui_native_423={ let pui_native_425=!(v_636f6d6d616e64.clone()); if (pui_native_425) {{ let pui_native_427={ let pui_native_429={ let pui_native_431=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_431) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_431} }; if !(pui_native_429) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Enter"))))).bool()} else {pui_native_429} }; if !(pui_native_427) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape"))))).bool()} else {pui_native_427} }} else {pui_native_425} }; if !(pui_native_423) {{ let pui_native_441={ let pui_native_443=v_636f6d6d616e64.clone(); if (pui_native_443) {!(((v_6576656e745f5f33353031303463336330653765346230.clone()).member("shiftKey", false)).truthy())} else {pui_native_443} }; if (pui_native_441) {{ let pui_native_445=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("f"))))).bool(); if !(pui_native_445) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("F"))))).bool()} else {pui_native_445} }} else {pui_native_441} }} else {pui_native_423} }} else {pui_native_419} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":129:9
                    { let v_7075695f6e61746976655f343531 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343531.clone()) };
                    // Source "frontend/proto-ui/src/text-field.proto.ts":130:9
                    { let v_7075695f6e61746976655f343532 = v_63757272656e745f5f61613565306636366665343037666165.clone(); emit(&(v_7075695f6e61746976655f343532.clone()).owner(), Value::string("navigationRequest"), record(vec![("key", { let left = v_6b6579.clone(); if left.nullish() { Value::string("") } else { left } }), ("value", { let v_7075695f6e61746976655f343533 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f343533.clone()).get() }), ("shift", Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f33353031303463336330653765346230.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool())), ("command", Value::boolean(v_636f6d6d616e64.clone())), ("viewEpoch", Value::number(({ let v_7075695f6e61746976655f343536 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f343536.clone()).get()).num()) }).num()))])) };
                    // Source "frontend/proto-ui/src/text-field.proto.ts":134:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":136:7
                if ({ let left = Value::boolean(v_636f6d6d616e64.clone()); if !left.truthy() { (v_6576656e745f5f33353031303463336330653765346230.clone()).member("altKey", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":136:36
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":137:7
                if { let pui_native_457=((v_70726f70735f5f32373435383161613533636433393466.clone()).member("numeric", false)).bool(); if (pui_native_457) {{ let pui_native_459=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_459) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_459} }} else {pui_native_457} } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":138:9
                    { let v_7075695f6e61746976655f343635 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343635.clone()) };
                    // Source "frontend/proto-ui/src/text-field.proto.ts":139:9
                    Value::boolean((v_7374657054657874.read().call)(if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp"))))).bool() {1.0_f64} else {-(1.0_f64)}));
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":140:14
                    if { let pui_native_468=((v_70726f70735f5f32373435383161613533636433393466.clone()).member("numeric", false)).bool(); if (pui_native_468) {(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Delete"))))).bool()} else {pui_native_468} } {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":141:9
                        { let v_7075695f6e61746976655f343732 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343732.clone()) };
                        // Source "frontend/proto-ui/src/text-field.proto.ts":142:9
                        Value::boolean((v_7265706c6163654472616674.read().call)(Value::string("")));
                    } else {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":143:14
                        if { let pui_native_473=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Enter"))))).bool(); if (pui_native_473) {((v_70726f70735f5f32373435383161613533636433393466.clone()).member("submitOnEnter", false)).bool()} else {pui_native_473} } {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":144:9
                            { let v_7075695f6e61746976655f343737 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343737.clone()) };
                            // Source "frontend/proto-ui/src/text-field.proto.ts":145:9
                            Value::boolean((v_72657175657374.read().call)(Value::string("submit"), true));
                        } else {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":146:14
                            if { let pui_native_478=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape"))))).bool(); if (pui_native_478) {((v_70726f70735f5f32373435383161613533636433393466.clone()).member("cancelOnEscape", false)).bool()} else {pui_native_478} } {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":147:9
                                { let v_7075695f6e61746976655f343832 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343832.clone()) };
                                // Source "frontend/proto-ui/src/text-field.proto.ts":148:9
                                Value::boolean((v_72657175657374.read().call)(Value::string("cancel"), false));
                            } else {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":149:14
                                if { let pui_native_483=(Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Tab"))))).bool(); if (pui_native_483) {((v_70726f70735f5f32373435383161613533636433393466.clone()).member("advanceOnTab", false)).bool()} else {pui_native_483} } {
                                    // Source "frontend/proto-ui/src/text-field.proto.ts":150:9
                                    { let v_7075695f6e61746976655f343837 = v_6576656e745f5f33353031303463336330653765346230.clone(); prevent(v_7075695f6e61746976655f343837.clone()) };
                                    // Source "frontend/proto-ui/src/text-field.proto.ts":151:9
                                    Value::boolean((v_72657175657374.read().call)(Value::string("advance"), (Value::boolean(!strict_equal(&((v_6576656e745f5f33353031303463336330653765346230.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool()));
                                }
                            }
                        }
                    }
                }
                Value::Undefined
            })) }; let v_7075695f6e61746976655f343932 = listen(&owner, v_3c636170747572653a7075695f6e61746976655f3439303e.clone(), false, v_3c636170747572653a7075695f6e61746976655f3439313e.clone(), Value::Undefined); capture_effect(&owner, "event", record(vec![("off", v_7075695f6e61746976655f343932.clone())])); capture_disposer(&owner, "event", v_7075695f6e61746976655f343932.clone());   v_7075695f6e61746976655f343932 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":154:5
            { let v_7075695f6e61746976655f343933 = v_6465665f5f35333637373936326264623532343666.clone(); { let v_3c636170747572653a7075695f6e61746976655f3439353e = Value::string("host:blur"); let v_3c636170747572653a7075695f6e61746976655f3439363e = {  let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_72657175657374 = v_72657175657374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":155:13
                let v_63757272656e745f5f34323935333164613239353436353731 = { let value = (v_72756e5f5f35306463623264633165633462396466.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":156:7
                if ({ let left = v_63757272656e745f5f34323935333164613239353436353731.clone(); if left.truthy() { Value::boolean((({ let v_7075695f6e61746976655f343934 = v_63757272656e745f5f34323935333164613239353436353731.clone(); read_props(&(v_7075695f6e61746976655f343934.clone()).owner(), false) }).member("commitOnBlur", false)).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":156:56
                    Value::boolean((v_72657175657374.read().call)(Value::string("blurCommit"), true));
                }
                Value::Undefined
            })) }; let v_7075695f6e61746976655f343937 = listen(&owner, v_3c636170747572653a7075695f6e61746976655f3439353e.clone(), false, v_3c636170747572653a7075695f6e61746976655f3439363e.clone(), Value::Undefined); capture_effect(&owner, "event", record(vec![("off", v_7075695f6e61746976655f343937.clone())])); capture_disposer(&owner, "event", v_7075695f6e61746976655f343937.clone());   v_7075695f6e61746976655f343937 } };
            // Source "frontend/proto-ui/src/text-field.proto.ts":158:5
            { let v_7075695f6e61746976655f343938 = v_6465665f5f35333637373936326264623532343666.clone(); register_life(&owner, "mounted", {  let v_7374617465 = v_7374617465.clone(); let v_666f637573 = v_666f637573.clone(); let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73656c65637452616e6765 = v_73656c65637452616e6765.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f34613431386431663362653630323835: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/text-field.proto.ts":159:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":159:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":160:7
                { let value = v_63757272656e745f5f34613431386431663362653630323835.clone(); *(v_72756e5f5f35306463623264633165633462396466.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":161:7
                Value::boolean({ let value=true; *(v_6d6f756e746564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/text-field.proto.ts":162:7
                { let v_7075695f6e61746976655f343939 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); state_set(v_7075695f6e61746976655f343939.clone(), Value::number({ let pui_native_500:f64=({ let v_7075695f6e61746976655f353032 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f353032.clone()).get()).num()) }).num(); let pui_native_501:f64=1.0_f64; pui_native_500 + pui_native_501 }), Value::string("reason: text view mounted")) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":163:7
                if (({ let v_7075695f6e61746976655f353033 = v_63757272656e745f5f34613431386431663362653630323835.clone(); read_props(&(v_7075695f6e61746976655f353033.clone()).owner(), false) }).member("selectAllOnMount", false)).bool() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":163:49
                    Value::boolean((v_73656c65637452616e6765.read().call)(0.0_f64, (({ let v_7075695f6e61746976655f353034 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f353034.clone()).get() }).member("length", false)).num()));
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":164:12
                    if { let pui_native_505=(({ let v_7075695f6e61746976655f353037 = v_63757272656e745f5f34613431386431663362653630323835.clone(); read_props(&(v_7075695f6e61746976655f353037.clone()).owner(), false) }).member("autoFocus", false)).bool(); if (pui_native_505) {!(({ let v_7075695f6e61746976655f353038 = (v_7374617465.read()).member("disabled", false); Value::boolean(((v_7075695f6e61746976655f353038.clone()).get()).bool()) }).bool())} else {pui_native_505} } {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":164:72
                        { let v_7075695f6e61746976655f353039 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f353039.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":166:5
            { let v_7075695f6e61746976655f353130 = v_6465665f5f35333637373936326264623532343666.clone(); register_life(&owner, "unmounted", {  let v_7669657745706f63685f5f65636632393634613739333761306432 = v_7669657745706f63685f5f65636632393634613739333761306432.clone(); let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":167:7
                Value::boolean({ let value=false; *(v_6d6f756e746564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/text-field.proto.ts":168:7
                { let value = Value::Json(Json::Null); *(v_72756e5f5f35306463623264633165633462396466.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":169:7
                { let v_7075695f6e61746976655f353131 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); state_set(v_7075695f6e61746976655f353131.clone(), Value::number({ let pui_native_512:f64=({ let v_7075695f6e61746976655f353134 = v_7669657745706f63685f5f65636632393634613739333761306432.read(); Value::number(((v_7075695f6e61746976655f353134.clone()).get()).num()) }).num(); let pui_native_513:f64=1.0_f64; pui_native_512 + pui_native_513 }), Value::string("reason: text view retired")) };
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":171:5
            { let v_7075695f6e61746976655f353135 = v_6465665f5f35333637373936326264623532343666.clone(); register_life(&owner, "before-dispose", {  let v_72756e5f5f35306463623264633165633462396466 = v_72756e5f5f35306463623264633165633462396466.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":172:7
                Value::boolean({ let value=false; *(v_6d6f756e746564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/text-field.proto.ts":173:7
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/text-field.proto.ts":174:7
                { let value = Value::Json(Json::Null); *(v_72756e5f5f35306463623264633165633462396466.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":176:5
            return { let callable=(v_696e68657269746564.clone()).member("render", false); match callable {
      Value::NativeFunction{identity,render} => capture_renderer(Value::NativeFunction{identity,render:render.clone()},render),
      Value::Renderer(adapter) => Value::Renderer(adapter),
      Value::Undefined => Value::Undefined,
      Value::Function(callback) => capture_renderer(Value::Function(callback.clone()),Rc::new(move |arguments:Vec<Value>| { { let pui_native_516 = callback(arguments); if matches!(&pui_native_516, Value::Json(Json::Null)) { native::gpui_native_owner::NativeSnapshot::empty() } else { native::gpui_native_owner::NativeSnapshot::invalid(pui_native_516) } } })),
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
      pub fn handle(&self) -> TextFieldHandle { TextFieldHandle { owner: self.owner.clone() } }
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
  impl Drop for TextField { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for TextField { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for TextField {
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
