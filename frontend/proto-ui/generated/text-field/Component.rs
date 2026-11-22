// Direct checked-source GPUI component. Supporting files are ordinary editable Rust.
  #[cfg(panic = "abort")] compile_error!("Authored exception lowering requires panic=unwind");
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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("value").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("defaultValue").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("readOnly").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("placeholder").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("required").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("name").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("autoComplete").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("minLength").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxLength").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("inputMode").map_or(true, |prop| prop.is_null() || (prop.as_str() == Some("none") || prop.as_str() == Some("text") || prop.as_str() == Some("tel") || prop.as_str() == Some("url") || prop.as_str() == Some("email") || prop.as_str() == Some("numeric") || prop.as_str() == Some("decimal") || prop.as_str() == Some("search"))) && value.get("enterKeyHint").map_or(true, |prop| prop.is_null() || (prop.as_str() == Some("enter") || prop.as_str() == Some("done") || prop.as_str() == Some("go") || prop.as_str() == Some("next") || prop.as_str() == Some("previous") || prop.as_str() == Some("search") || prop.as_str() == Some("send"))) && value.get("ariaLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("labelledBy").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("describedBy").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selectAllOnMount").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("submitOnEnter").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("cancelOnEscape").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("advanceOnTab").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("commitOnBlur").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("numeric").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("numericMin").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericMax").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericStep").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("numericDefault").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "value" => value.is_string(),
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
        "describedBy" => value.is_string(),
        "autoFocus" => value.is_boolean(),
        "selectAllOnMount" => value.is_boolean(),
        "submitOnEnter" => value.is_boolean(),
        "cancelOnEscape" => value.is_boolean(),
        "advanceOnTab" => value.is_boolean(),
        "commitOnBlur" => value.is_boolean(),
        "numeric" => value.is_boolean(),
        "numericMin" => value.as_f64().is_some_and(f64::is_finite),
        "numericMax" => value.as_f64().is_some_and(f64::is_finite),
        "numericStep" => value.as_f64().is_some_and(f64::is_finite),
        "numericDefault" => value.as_f64().is_some_and(f64::is_finite), _ => false } }
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
    #[serde(rename = "value", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "defaultValue", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<bool>,
    #[serde(rename = "readOnly", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<bool>,
    #[serde(rename = "placeholder", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<String>,
    #[serde(rename = "required", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "name", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<String>,
    #[serde(rename = "autoComplete", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<String>,
    #[serde(rename = "minLength", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
    #[serde(rename = "maxLength", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<f64>,
    #[serde(rename = "inputMode", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<GeneratedData1>,
    #[serde(rename = "enterKeyHint", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<GeneratedData2>,
    #[serde(rename = "ariaLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<String>,
    #[serde(rename = "labelledBy", default, skip_serializing_if = "Option::is_none")]
    pub field_13: Option<String>,
    #[serde(rename = "describedBy", default, skip_serializing_if = "Option::is_none")]
    pub field_14: Option<String>,
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_15: Option<bool>,
    #[serde(rename = "selectAllOnMount", default, skip_serializing_if = "Option::is_none")]
    pub field_16: Option<bool>,
    #[serde(rename = "submitOnEnter", default, skip_serializing_if = "Option::is_none")]
    pub field_17: Option<bool>,
    #[serde(rename = "cancelOnEscape", default, skip_serializing_if = "Option::is_none")]
    pub field_18: Option<bool>,
    #[serde(rename = "advanceOnTab", default, skip_serializing_if = "Option::is_none")]
    pub field_19: Option<bool>,
    #[serde(rename = "commitOnBlur", default, skip_serializing_if = "Option::is_none")]
    pub field_20: Option<bool>,
    #[serde(rename = "numeric", default, skip_serializing_if = "Option::is_none")]
    pub field_21: Option<bool>,
    #[serde(rename = "numericMin", default, skip_serializing_if = "Option::is_none")]
    pub field_22: Option<f64>,
    #[serde(rename = "numericMax", default, skip_serializing_if = "Option::is_none")]
    pub field_23: Option<f64>,
    #[serde(rename = "numericStep", default, skip_serializing_if = "Option::is_none")]
    pub field_24: Option<f64>,
    #[serde(rename = "numericDefault", default, skip_serializing_if = "Option::is_none")]
    pub field_25: Option<f64>,
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
        let v_6465665f5f39336639653237316461616433393465: Value = arg(&arguments, 0);
        let v_636f6e74726f6c: LexicalBinding<Value> = lexical_binding();
        let v_666f63757361626c65: LexicalBinding<Value> = lexical_binding();
        let v_76616c75655f5f38326232636139303264376561653766: LexicalBinding<Value> = lexical_binding();
        let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
        let v_726561644f6e6c79: LexicalBinding<Value> = lexical_binding();
        let v_636f6d706f73696e67: LexicalBinding<Value> = lexical_binding();
        let v_617269614c6162656c: LexicalBinding<Value> = lexical_binding();
        let v_6c6162656c6c65644279: LexicalBinding<Value> = lexical_binding();
        let v_6465736372696265644279: LexicalBinding<Value> = lexical_binding();
        let v_63757272656e7456696577: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
        let v_7669657745706f63685f5f31323962326666396330396536663933: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
        let v_73796e63: LexicalBinding<Value> = lexical_binding();
        let v_656d6974436f6d706f736974696f6e: LexicalBinding<Value> = lexical_binding();
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":24:9
        let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":25:3
        { let v_7075695f6e61746976655f30 = v_6465665f5f39336639653237316461616433393465.clone(); props_define(&owner, record(vec![("value", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("defaultValue", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("readOnly", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("placeholder", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("required", record(vec![("type", Value::string("boolean")), ("empty", Value::string("fallback"))])), ("name", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("autoComplete", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("minLength", record(vec![("type", Value::string("number")), ("empty", Value::string("fallback"))])), ("maxLength", record(vec![("type", Value::string("number")), ("empty", Value::string("fallback"))])), ("inputMode", record(vec![("type", Value::string("enum")), ("empty", Value::string("fallback")), ("options", Value::Array(Rc::new(vec![Value::string("none"), Value::string("text"), Value::string("tel"), Value::string("url"), Value::string("email"), Value::string("numeric"), Value::string("decimal"), Value::string("search")])))])), ("enterKeyHint", record(vec![("type", Value::string("enum")), ("empty", Value::string("fallback")), ("options", Value::Array(Rc::new(vec![Value::string("enter"), Value::string("done"), Value::string("go"), Value::string("next"), Value::string("previous"), Value::string("search"), Value::string("send")])))])), ("ariaLabel", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("labelledBy", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))])), ("describedBy", record(vec![("type", Value::string("string")), ("empty", Value::string("fallback"))]))])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":50:3
        { let v_7075695f6e61746976655f31 = v_6465665f5f39336639653237316461616433393465.clone(); set_defaults(&owner, record(vec![("defaultValue", Value::string("")), ("disabled", Value::boolean(false)), ("readOnly", Value::boolean(false)), ("placeholder", Value::string("")), ("required", Value::boolean(false)), ("name", Value::string("")), ("autoComplete", Value::string("")), ("minLength", Value::number(-(Value::number(1.0)).scalar_number())), ("maxLength", Value::number(-(Value::number(1.0)).scalar_number())), ("ariaLabel", Value::string("")), ("labelledBy", Value::string("")), ("describedBy", Value::string(""))])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":65:9
        v_636f6e74726f6c.initialize(native_builtin_hook(&owner, "asTextControl", || text_setup(&owner)));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":66:9
        v_666f63757361626c65.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":67:3
        { let v_7075695f6e61746976655f32 = v_666f63757361626c65.read(); focus_setup_config(&(v_7075695f6e61746976655f32.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":68:9
        v_76616c75655f5f38326232636139303264376561653766.initialize({ let v_7075695f6e61746976655f33 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "string", Value::string("value"), Value::string(""), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":69:9
        v_64697361626c6564.initialize({ let v_7075695f6e61746976655f34 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":70:9
        v_726561644f6e6c79.initialize({ let v_7075695f6e61746976655f35 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "bool", Value::string("readOnly"), Value::boolean(false), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":71:9
        v_636f6d706f73696e67.initialize({ let v_7075695f6e61746976655f36 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "bool", Value::string("composing"), Value::boolean(false), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":72:9
        v_617269614c6162656c.initialize({ let v_7075695f6e61746976655f37 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "string", Value::string("inputAriaLabel"), Value::string(""), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":73:9
        v_6c6162656c6c65644279.initialize({ let v_7075695f6e61746976655f38 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "string", Value::string("inputLabelledBy"), Value::string(""), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":74:9
        v_6465736372696265644279.initialize({ let v_7075695f6e61746976655f39 = v_6465665f5f39336639653237316461616433393465.clone(); create_state(&owner, "string", Value::string("inputDescribedBy"), Value::string(""), Value::Undefined) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":75:9
        let v_666f6375736564 = (v_666f63757361626c65.read()).member("focused", false);
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":76:9
        let v_666f63757356697369626c65 = (v_666f63757361626c65.read()).member("focusVisible", false);
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":77:7
        v_63757272656e7456696577.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":78:7
        v_7669657745706f63685f5f31323962326666396330396536663933.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":80:3
        { let v_7075695f6e61746976655f3130 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("value"), v_76616c75655f5f38326232636139303264376561653766.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":81:3
        { let v_7075695f6e61746976655f3131 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":82:3
        { let v_7075695f6e61746976655f3132 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("readOnly"), v_726561644f6e6c79.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":83:3
        { let v_7075695f6e61746976655f3133 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("focused"), v_666f6375736564.clone()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":84:3
        { let v_7075695f6e61746976655f3134 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("focusVisible"), v_666f63757356697369626c65.clone()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":85:3
        { let v_7075695f6e61746976655f3135 = v_6465665f5f39336639653237316461616433393465.clone(); expose_state(&owner, Value::string("composing"), v_636f6d706f73696e67.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":86:3
        { let v_7075695f6e61746976655f3136 = v_6465665f5f39336639653237316461616433393465.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f63757361626c65 = v_666f63757361626c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_6f7074696f6e73: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":87:5
            if (Value::boolean(!({ let v_7075695f6e61746976655f3137 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3137.clone()).get() }).truthy())).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":87:26
                { let v_7075695f6e61746976655f3138 = v_666f63757361626c65.read(); focus_request(&(v_7075695f6e61746976655f3138.clone()).owner(), v_6f7074696f6e73.clone(), false) };
            }
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":89:3
        { let v_7075695f6e61746976655f3139 = v_6465665f5f39336639653237316461616433393465.clone(); expose_method(&owner, Value::string("blurSelf"), {  let v_666f63757361626c65 = v_666f63757361626c65.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":89:39
            return { let v_7075695f6e61746976655f3230 = v_666f63757361626c65.read(); focus_blur(&(v_7075695f6e61746976655f3230.clone()).owner()) };
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":90:3
        { let v_7075695f6e61746976655f3231 = v_6465665f5f39336639653237316461616433393465.clone(); expose_method(&owner, Value::string("resetValue"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f38326232636139303264376561653766 = v_76616c75655f5f38326232636139303264376561653766.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_6e6578745f5f65363161343336373863353438376434: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":91:11
            let v_72756e5f5f64306337323334663838326665663939 = { let value = (v_63757272656e7456696577.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":92:5
            if (Value::boolean(!(v_72756e5f5f64306337323334663838326665663939.clone()).truthy())).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":92:15
                return Value::boolean(false);
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":93:11
            let v_65706f63685f5f33323137353863646331646265643265 = { let cell = v_7669657745706f63685f5f31323962326666396330396536663933.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":94:11
            let v_6163636570746564 = { let v_7075695f6e61746976655f3232 = v_636f6e74726f6c.read(); text_reset_value(&(v_7075695f6e61746976655f3232.clone()).owner(), v_6e6578745f5f65363161343336373863353438376434.clone()) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":95:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f64306337323334663838326665663939.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f33323137353863646331646265643265.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":95:53
                return Value::boolean(false);
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":96:11
            let v_736e617073686f74 = { let v_7075695f6e61746976655f3233 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3233.clone()).owner()) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":97:5
            { let v_7075695f6e61746976655f3234 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3234.clone(), { let left = (v_736e617073686f74.clone()).member("composing", true); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: input reset composition")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":98:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f64306337323334663838326665663939.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f33323137353863646331646265643265.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":98:53
                return Value::boolean(false);
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":99:5
            { let v_7075695f6e61746976655f3235 = v_76616c75655f5f38326232636139303264376561653766.read(); state_set(v_7075695f6e61746976655f3235.clone(), { let left = (v_736e617073686f74.clone()).member("value", true); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input reset value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":100:5
            return { let left = { let left = Value::boolean(strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f64306337323334663838326665663939.clone()))); if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f33323137353863646331646265643265.clone()))) } else { left } }; if left.truthy() { v_6163636570746564.clone() } else { left } };
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":102:3
        { let v_7075695f6e61746976655f3236 = v_6465665f5f39336639653237316461616433393465.clone(); expose_event(&owner, Value::string("valueChange")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":103:3
        { let v_7075695f6e61746976655f3237 = v_6465665f5f39336639653237316461616433393465.clone(); expose_event(&owner, Value::string("change")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":104:3
        { let v_7075695f6e61746976655f3238 = v_6465665f5f39336639653237316461616433393465.clone(); expose_event(&owner, Value::string("compositionStart")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":105:3
        { let v_7075695f6e61746976655f3239 = v_6465665f5f39336639653237316461616433393465.clone(); expose_event(&owner, Value::string("compositionUpdate")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":106:3
        { let v_7075695f6e61746976655f3330 = v_6465665f5f39336639653237316461616433393465.clone(); expose_event(&owner, Value::string("compositionEnd")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":108:3
        { let v_7075695f6e61746976655f3331 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f3331.clone()).owner(), Value::string("textbox")) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":109:3
        { let v_7075695f6e61746976655f3332 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f3332.clone()).owner(), "name", v_617269614c6162656c.read()) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":110:3
        { let v_7075695f6e61746976655f3333 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3333.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":111:3
        { let v_7075695f6e61746976655f3334 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f3334.clone()).owner(), Value::string("readOnly"), v_726561644f6e6c79.read(), Value::Undefined) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":112:3
        { let v_7075695f6e61746976655f3335 = v_61636365737369626c65.clone(); accessible_relation(&(v_7075695f6e61746976655f3335.clone()).owner(), Value::string("labelledBy"), record(vec![("target", v_6c6162656c6c65644279.read())])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":113:3
        { let v_7075695f6e61746976655f3336 = v_61636365737369626c65.clone(); accessible_relation(&(v_7075695f6e61746976655f3336.clone()).owner(), Value::string("describedBy"), record(vec![("target", v_6465736372696265644279.read())])) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":115:9
        v_73796e63.initialize({  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_666f63757361626c65 = v_666f63757361626c65.clone(); let v_76616c75655f5f38326232636139303264376561653766 = v_76616c75655f5f38326232636139303264376561653766.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726561644f6e6c79 = v_726561644f6e6c79.clone(); let v_617269614c6162656c = v_617269614c6162656c.clone(); let v_6c6162656c6c65644279 = v_6c6162656c6c65644279.clone(); let v_6465736372696265644279 = v_6465736372696265644279.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_70726f70735f5f62613937656537623634356135613032: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":116:11
            let v_636f6e74726f6c6c6564 = Value::boolean(strict_equal(&(typeof_value(&((v_70726f70735f5f62613937656537623634356135613032.clone()).member("value", false)))), &(Value::string("string"))));
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":117:11
            let v_6e65787444697361626c6564 = { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":118:5
            { let v_7075695f6e61746976655f3337 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3337.clone(), v_6e65787444697361626c6564.clone(), Value::string("reason: input sync disabled")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":119:5
            { let v_7075695f6e61746976655f3338 = v_726561644f6e6c79.read(); state_set(v_7075695f6e61746976655f3338.clone(), { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("readOnly", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: input sync readonly")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":120:5
            { let v_7075695f6e61746976655f3339 = v_617269614c6162656c.read(); state_set(v_7075695f6e61746976655f3339.clone(), { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("ariaLabel", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync aria label")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":121:5
            { let v_7075695f6e61746976655f3430 = v_6c6162656c6c65644279.read(); state_set(v_7075695f6e61746976655f3430.clone(), { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("labelledBy", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync labelledby")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":122:5
            { let v_7075695f6e61746976655f3431 = v_6465736372696265644279.read(); state_set(v_7075695f6e61746976655f3431.clone(), { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("describedBy", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync describedby")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":123:5
            { let v_7075695f6e61746976655f3432 = v_666f63757361626c65.read(); focus_disabled(&(v_7075695f6e61746976655f3432.clone()).owner(), v_6e65787444697361626c6564.clone()) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":124:5
            { let v_7075695f6e61746976655f3433 = v_636f6e74726f6c.read(); text_sync(&(v_7075695f6e61746976655f3433.clone()).owner(), record(vec![("valueMode", if (v_636f6e74726f6c6c6564.clone()).truthy() { Value::string("controlled") } else { Value::string("uncontrolled") }), ("value", if (v_636f6e74726f6c6c6564.clone()).truthy() { (v_70726f70735f5f62613937656537623634356135613032.clone()).member("value", false) } else { Value::Undefined }), ("defaultValue", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("defaultValue", false); if left.nullish() { Value::string("") } else { left } }), ("disabled", v_6e65787444697361626c6564.clone()), ("readOnly", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("readOnly", false); if left.nullish() { Value::boolean(false) } else { left } }), ("placeholder", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("placeholder", false); if left.nullish() { Value::string("") } else { left } }), ("required", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("required", false); if left.nullish() { Value::boolean(false) } else { left } }), ("name", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("name", false); if left.nullish() { Value::string("") } else { left } }), ("autoComplete", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("autoComplete", false); if left.nullish() { Value::string("") } else { left } }), ("minLength", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("minLength", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }), ("maxLength", { let left = (v_70726f70735f5f62613937656537623634356135613032.clone()).member("maxLength", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }), ("inputMode", (v_70726f70735f5f62613937656537623634356135613032.clone()).member("inputMode", false)), ("enterKeyHint", (v_70726f70735f5f62613937656537623634356135613032.clone()).member("enterKeyHint", false))])) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":139:5
            { let v_7075695f6e61746976655f3434 = v_76616c75655f5f38326232636139303264376561653766.read(); state_set(v_7075695f6e61746976655f3434.clone(), { let left = ({ let v_7075695f6e61746976655f3435 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3435.clone()).owner()) }).member("value", true); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: input sync value")) };
            Value::Undefined
        })) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":141:3
        { let v_7075695f6e61746976655f3436 = v_6465665f5f39336639653237316461616433393465.clone(); register_life(&owner, "created", {  let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f39343064353734306634626636363837: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":141:36
            return v_73796e63.read().call_authored(vec![{ let v_7075695f6e61746976655f3437 = v_72756e5f5f39343064353734306634626636363837.clone(); read_props(&(v_7075695f6e61746976655f3437.clone()).owner(), false) }]);
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":142:3
        { let v_7075695f6e61746976655f3438 = v_6465665f5f39336639653237316461616433393465.clone(); register_life(&owner, "mounted", {  let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f63383937396534626334396237616165: Value = arg(&arguments, 0);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":143:5
            { let value = v_72756e5f5f63383937396534626334396237616165.clone(); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":145:3
        { let v_7075695f6e61746976655f3439 = v_6465665f5f39336639653237316461616433393465.clone(); register_life(&owner, "unmounted", {  let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":146:5
            { let value = Value::Json(Json::Null); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":147:5
            { let cell = v_7669657745706f63685f5f31323962326666396330396536663933.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":149:5
            { let v_7075695f6e61746976655f3530 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3530.clone(), Value::boolean(false), Value::string("reason: input view composition retired")) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":151:3
        { let v_7075695f6e61746976655f3531 = v_6465665f5f39336639653237316461616433393465.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":152:5
            { let value = Value::Json(Json::Null); *(v_63757272656e7456696577.read()).borrow_mut() = value; }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":153:5
            { let cell = v_7669657745706f63685f5f31323962326666396330396536663933.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":155:3
        { let v_7075695f6e61746976655f3532 = v_6465665f5f39336639653237316461616433393465.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("value"), Value::string("defaultValue"), Value::string("disabled"), Value::string("readOnly"), Value::string("placeholder"), Value::string("required"), Value::string("name"), Value::string("autoComplete"), Value::string("minLength"), Value::string("maxLength"), Value::string("inputMode"), Value::string("enterKeyHint"), Value::string("ariaLabel"), Value::string("labelledBy"), Value::string("describedBy")]))), false, {  let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_5f72756e: Value = arg(&arguments, 0);
            let v_6e6578745f5f36633032353830383633613531623461: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":173:21
            return v_73796e63.read().call_authored(vec![v_6e6578745f5f36633032353830383633613531623461.clone()]);
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":176:3
        { let v_7075695f6e61746976655f3533 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f3533.clone()).owner(), Value::string("input"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f38326232636139303264376561653766 = v_76616c75655f5f38326232636139303264376561653766.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f34383262393463663964323166353530: Value = arg(&arguments, 0);
            let v_6576656e745f5f31323033626564316637313133313265: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":177:11
            let v_65706f63685f5f65373136363032306565623432323262 = { let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":178:5
            { let v_7075695f6e61746976655f3534 = v_76616c75655f5f38326232636139303264376561653766.read(); state_set(v_7075695f6e61746976655f3534.clone(), { let left = ({ let v_7075695f6e61746976655f3535 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3535.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f31323033626564316637313133313265.clone()).member("value", false) } else { left } }, Value::string("reason: input value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":179:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f34383262393463663964323166353530.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f65373136363032306565623432323262.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":179:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":180:5
            { let v_7075695f6e61746976655f3536 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3536.clone(), (v_6576656e745f5f31323033626564316637313133313265.clone()).member("composing", false), Value::string("reason: input composing")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":181:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f34383262393463663964323166353530.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f65373136363032306565623432323262.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":181:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":182:11
            let v_64657461696c5f5f36363131626434646664356336323938 = freeze(record(vec![("value", (v_6576656e745f5f31323033626564316637313133313265.clone()).member("value", false)), ("composing", (v_6576656e745f5f31323033626564316637313133313265.clone()).member("composing", false)), ("data", (v_6576656e745f5f31323033626564316637313133313265.clone()).member("data", false)), ("inputType", (v_6576656e745f5f31323033626564316637313133313265.clone()).member("inputType", false))]));
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":188:5
            { let v_7075695f6e61746976655f3537 = v_72756e5f5f34383262393463663964323166353530.clone(); emit(&(v_7075695f6e61746976655f3537.clone()).owner(), Value::string("valueChange"), v_64657461696c5f5f36363131626434646664356336323938.clone()) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":190:3
        { let v_7075695f6e61746976655f3538 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f3538.clone()).owner(), Value::string("change"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f38326232636139303264376561653766 = v_76616c75655f5f38326232636139303264376561653766.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f35383130363330643639663737393465: Value = arg(&arguments, 0);
            let v_6576656e745f5f65306133363232343365616664393732: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":191:11
            let v_65706f63685f5f35396531653439333031636466663762 = { let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":192:5
            { let v_7075695f6e61746976655f3539 = v_76616c75655f5f38326232636139303264376561653766.read(); state_set(v_7075695f6e61746976655f3539.clone(), { let left = ({ let v_7075695f6e61746976655f3630 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3630.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f65306133363232343365616664393732.clone()).member("value", false) } else { left } }, Value::string("reason: input change value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":193:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f35383130363330643639663737393465.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f35396531653439333031636466663762.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":193:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":194:5
            { let v_7075695f6e61746976655f3631 = v_72756e5f5f35383130363330643639663737393465.clone(); emit(&(v_7075695f6e61746976655f3631.clone()).owner(), Value::string("change"), freeze(record(vec![("value", (v_6576656e745f5f65306133363232343365616664393732.clone()).member("value", false))]))) };
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":196:9
        v_656d6974436f6d706f736974696f6e.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f62643737336162333337313237376463: Value = arg(&arguments, 0);
            let v_6576656e744e616d65: Value = arg(&arguments, 1);
            let v_6576656e745f5f36373639343362356439643962366534: Value = arg(&arguments, 2);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":201:11
            let v_64657461696c5f5f34663366393738633436633163373235 = freeze(record(vec![("value", (v_6576656e745f5f36373639343362356439643962366534.clone()).member("value", false)), ("data", (v_6576656e745f5f36373639343362356439643962366534.clone()).member("data", false))]));
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":202:5
            { let v_7075695f6e61746976655f3632 = v_72756e5f5f62643737336162333337313237376463.clone(); emit(&(v_7075695f6e61746976655f3632.clone()).owner(), v_6576656e744e616d65.clone(), v_64657461696c5f5f34663366393738633436633163373235.clone()) };
            Value::Undefined
        })) });
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":204:3
        { let v_7075695f6e61746976655f3633 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f3633.clone()).owner(), Value::string("compositionstart"), {  let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f34383734316261356665303563663437: Value = arg(&arguments, 0);
            let v_6576656e745f5f33376236653761306164323339303930: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":205:11
            let v_65706f63685f5f34303365376566376535613163383938 = { let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":206:5
            { let v_7075695f6e61746976655f3634 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3634.clone(), Value::boolean(true), Value::string("reason: input composition start")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":207:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f34383734316261356665303563663437.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f34303365376566376535613163383938.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":207:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":208:5
            v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f34383734316261356665303563663437.clone(), Value::string("compositionStart"), v_6576656e745f5f33376236653761306164323339303930.clone()]);
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":210:3
        { let v_7075695f6e61746976655f3635 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f3635.clone()).owner(), Value::string("compositionupdate"), {  let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f33363439663934383538623633616335: Value = arg(&arguments, 0);
            let v_6576656e745f5f39623734373965313530636433333635: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":210:51
            return v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f33363439663934383538623633616335.clone(), Value::string("compositionUpdate"), v_6576656e745f5f39623734373965313530636433333635.clone()]);
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":211:3
        { let v_7075695f6e61746976655f3636 = v_636f6e74726f6c.read(); text_on(&(v_7075695f6e61746976655f3636.clone()).owner(), Value::string("compositionend"), {  let v_636f6e74726f6c = v_636f6e74726f6c.clone(); let v_76616c75655f5f38326232636139303264376561653766 = v_76616c75655f5f38326232636139303264376561653766.clone(); let v_636f6d706f73696e67 = v_636f6d706f73696e67.clone(); let v_63757272656e7456696577 = v_63757272656e7456696577.clone(); let v_7669657745706f63685f5f31323962326666396330396536663933 = v_7669657745706f63685f5f31323962326666396330396536663933.clone(); let v_656d6974436f6d706f736974696f6e = v_656d6974436f6d706f736974696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_72756e5f5f64316436303966343966343538613666: Value = arg(&arguments, 0);
            let v_6576656e745f5f35626561343661343363326464653635: Value = arg(&arguments, 1);
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":212:11
            let v_65706f63685f5f66393039303161353430613164306134 = { let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":213:5
            { let v_7075695f6e61746976655f3637 = v_636f6d706f73696e67.read(); state_set(v_7075695f6e61746976655f3637.clone(), Value::boolean(false), Value::string("reason: input composition end")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":214:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f64316436303966343966343538613666.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f66393039303161353430613164306134.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":214:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":215:5
            { let v_7075695f6e61746976655f3638 = v_76616c75655f5f38326232636139303264376561653766.read(); state_set(v_7075695f6e61746976655f3638.clone(), { let left = ({ let v_7075695f6e61746976655f3639 = v_636f6e74726f6c.read(); text_snapshot(&(v_7075695f6e61746976655f3639.clone()).owner()) }).member("value", true); if left.nullish() { (v_6576656e745f5f35626561343661343363326464653635.clone()).member("value", false) } else { left } }, Value::string("reason: input composition end value")) };
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":216:5
            if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e7456696577.read()).borrow().clone(); value }), &(v_72756e5f5f64316436303966343966343538613666.clone()))); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_7669657745706f63685f5f31323962326666396330396536663933.read()).borrow().clone(); value }), &(v_65706f63685f5f66393039303161353430613164306134.clone()))) } else { left } }).truthy() {
                // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":216:53
                return Value::Undefined;
            }
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":217:5
            v_656d6974436f6d706f736974696f6e.read().call_authored(vec![v_72756e5f5f64316436303966343966343538613666.clone(), Value::string("compositionEnd"), v_6576656e745f5f35626561343661343363326464653635.clone()]);
            Value::Undefined
        })) }) };
        // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":219:3
        return {   Value::Function(Rc::new(move |_: Vec<Value>| {
            // Source "frontend/proto-ui/src/upstream-input/root.proto.ts":219:16
            return Value::Json(Json::Null);
        })) };
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
        catch_authored(|| {
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
        })
    }
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_8(&self, argument_0: Option<String>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn state_14(&self) -> Result<f64, Value> { self.state("viewEpoch").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_21(&self, argument_0: f64, argument_1: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_22(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_23(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_24(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_25(&self, argument_0: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_26(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
  pub struct TextField { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl TextField {
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
          text_declaration(&owner, false);
          configure_native_atoms(&owner_ref, false, false);
          refresh_native_atoms(&owner_ref, window, cx);
          let _lexical_owner = enter_lexical_owner(&owner);
          let renderer = { let owner = owner.clone();  Value::Function(Rc::new(move |arguments: Vec<Value>| {
            let v_6465665f5f35313135366635356438373238343336: Value = arg(&arguments, 0);
            let v_7374617465: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_7669657745706f63685f5f38393261616630326134373465313530: LexicalBinding<Value> = lexical_binding();
            let v_72756e5f5f36346432306239616339396665303663: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6d6f756e746564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_616c6c6f776564: LexicalBinding<Value> = lexical_binding();
            let v_72657175657374: LexicalBinding<Value> = lexical_binding();
            let v_73656c65637452616e6765: LexicalBinding<Value> = lexical_binding();
            let v_7265706c6163654472616674: LexicalBinding<Value> = lexical_binding();
            let v_7374657054657874: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/text-field.proto.ts":25:11
            let v_696e68657269746564 = authored_hook_0(capability(&owner), Value::Undefined);
            // Source "frontend/proto-ui/src/text-field.proto.ts":26:11
            v_7374617465.initialize((v_696e68657269746564.clone()).member("stateHandles", false));
            // Source "frontend/proto-ui/src/text-field.proto.ts":27:5
            if (Value::boolean(!(v_7374617465.read()).truthy())).truthy() {
                // Source "frontend/proto-ui/src/text-field.proto.ts":27:17
                throw_value(record(vec![("name", Value::string("Error")), ("message", Value::string(&(Value::string("Input root state projection is required")).error_message()))]));
            }
            // Source "frontend/proto-ui/src/text-field.proto.ts":28:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/text-field.proto.ts":29:11
            v_7669657745706f63685f5f38393261616630326134373465313530.initialize({ let v_7075695f6e61746976655f3730 = v_6465665f5f35313135366635356438373238343336.clone(); create_state(&owner, "discrete", Value::string("viewEpoch"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":30:5
            { let v_7075695f6e61746976655f3731 = v_6465665f5f35313135366635356438373238343336.clone(); expose_state(&owner, Value::string("viewEpoch"), v_7669657745706f63685f5f38393261616630326134373465313530.read()) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":31:5
            { let v_7075695f6e61746976655f3732 = v_6465665f5f35313135366635356438373238343336.clone(); props_define(&owner, record(vec![("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selectAllOnMount", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("submitOnEnter", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("cancelOnEscape", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("advanceOnTab", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("commitOnBlur", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("numeric", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("numericMin", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("numericMax", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("numericStep", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("numericDefault", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))]))])) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":44:5
            { let v_7075695f6e61746976655f3733 = v_6465665f5f35313135366635356438373238343336.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-w-0".into(), "min-w-".into())])]) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":45:5
            { let v_7075695f6e61746976655f3734 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("submit")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":46:5
            { let v_7075695f6e61746976655f3735 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("cancel")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":47:5
            { let v_7075695f6e61746976655f3736 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("advance")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":48:5
            { let v_7075695f6e61746976655f3737 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("blurCommit")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":49:5
            { let v_7075695f6e61746976655f3738 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("selectionRequest")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":50:5
            { let v_7075695f6e61746976655f3739 = v_6465665f5f35313135366635356438373238343336.clone(); expose_event(&owner, Value::string("replacementRequest")) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":51:9
            v_72756e5f5f36346432306239616339396665303663.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/text-field.proto.ts":52:9
            v_6d6f756e746564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/text-field.proto.ts":53:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/text-field.proto.ts":54:11
            v_616c6c6f776564.initialize({  let v_7374617465 = v_7374617465.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":54:36
                return { let left = { let left = { let left = { let value = (v_6d6f756e746564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3830 = (v_7374617465.read()).member("disabled", false); (v_7075695f6e61746976655f3830.clone()).get() }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3831 = (v_7374617465.read()).member("composing", false); (v_7075695f6e61746976655f3831.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":55:11
            v_72657175657374.initialize({  let v_7374617465 = v_7374617465.clone(); let v_7669657745706f63685f5f38393261616630326134373465313530 = v_7669657745706f63685f5f38393261616630326134373465313530.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b696e64: Value = arg(&arguments, 0);
                let v_666f72776172645f5f32353339336230623532343236613433: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/text-field.proto.ts":56:13
                let v_63757272656e745f5f31386237653664373137306234653334 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":57:7
                if ({ let left = Value::boolean(!(v_63757272656e745f5f31386237653664373137306234653334.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_616c6c6f776564.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":57:35
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":58:13
                let v_64657461696c5f5f39386164643966363563653965623536 = record(vec![("value", { let v_7075695f6e61746976655f3832 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f3832.clone()).get() }), ("forward", v_666f72776172645f5f32353339336230623532343236613433.clone()), ("viewEpoch", { let v_7075695f6e61746976655f3833 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f3833.clone()).get() })]);
                // Source "frontend/proto-ui/src/text-field.proto.ts":59:7
                if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("submit"))))).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":59:30
                    { let v_7075695f6e61746976655f3834 = v_63757272656e745f5f31386237653664373137306234653334.clone(); emit(&(v_7075695f6e61746976655f3834.clone()).owner(), Value::string("submit"), v_64657461696c5f5f39386164643966363563653965623536.clone()) };
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":60:12
                    if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("cancel"))))).truthy() {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":60:35
                        { let v_7075695f6e61746976655f3835 = v_63757272656e745f5f31386237653664373137306234653334.clone(); emit(&(v_7075695f6e61746976655f3835.clone()).owner(), Value::string("cancel"), v_64657461696c5f5f39386164643966363563653965623536.clone()) };
                    } else {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":61:12
                        if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("advance"))))).truthy() {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":61:36
                            { let v_7075695f6e61746976655f3836 = v_63757272656e745f5f31386237653664373137306234653334.clone(); emit(&(v_7075695f6e61746976655f3836.clone()).owner(), Value::string("advance"), v_64657461696c5f5f39386164643966363563653965623536.clone()) };
                        } else {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":62:12
                            if (Value::boolean(strict_equal(&(v_6b696e64.clone()), &(Value::string("blurCommit"))))).truthy() {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":62:39
                                { let v_7075695f6e61746976655f3837 = v_63757272656e745f5f31386237653664373137306234653334.clone(); emit(&(v_7075695f6e61746976655f3837.clone()).owner(), Value::string("blurCommit"), v_64657461696c5f5f39386164643966363563653965623536.clone()) };
                            } else {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":63:12
                                return Value::boolean(false);
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":64:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":68:11
            v_73656c65637452616e6765.initialize({  let v_7374617465 = v_7374617465.clone(); let v_666f637573 = v_666f637573.clone(); let v_7669657745706f63685f5f38393261616630326134373465313530 = v_7669657745706f63685f5f38393261616630326134373465313530.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7374617274: Value = arg(&arguments, 0);
                let v_656e64: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/text-field.proto.ts":69:13
                let v_63757272656e745f5f37626163626139663133376134383065 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":70:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e745f5f37626163626139663133376134383065.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_616c6c6f776564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_7374617274.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_656e64.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":70:87
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":71:13
                let v_6c656e677468 = ({ let v_7075695f6e61746976655f3838 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f3838.clone()).get() }).member("length", false);
                // Source "frontend/proto-ui/src/text-field.proto.ts":72:11
                let v_66726f6d = Rc::new(std::cell::RefCell::new(Value::number((v_7374617274.clone()).num().floor())));
                // Source "frontend/proto-ui/src/text-field.proto.ts":73:11
                let v_746f = Rc::new(std::cell::RefCell::new(Value::number((v_656e64.clone()).num().floor())));
                // Source "frontend/proto-ui/src/text-field.proto.ts":74:7
                if ({ let relational_left = { let value = (v_66726f6d.clone()).borrow().clone(); value }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":74:21
                    { let value = Value::number(0.0); *(v_66726f6d.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":75:7
                if ({ let relational_left = { let value = (v_746f.clone()).borrow().clone(); value }; let relational_right = { let value = (v_66726f6d.clone()).borrow().clone(); value }; Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":75:22
                    { let value = { let value = (v_66726f6d.clone()).borrow().clone(); value }; *(v_746f.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":76:7
                if ({ let relational_left = { let value = (v_66726f6d.clone()).borrow().clone(); value }; let relational_right = v_6c656e677468.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":76:26
                    { let value = v_6c656e677468.clone(); *(v_66726f6d.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":77:7
                if ({ let relational_left = { let value = (v_746f.clone()).borrow().clone(); value }; let relational_right = v_6c656e677468.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":77:24
                    { let value = v_6c656e677468.clone(); *(v_746f.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":78:13
                let v_65706f63685f5f33353735343431326666626363363534 = { let v_7075695f6e61746976655f3839 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f3839.clone()).get() };
                // Source "frontend/proto-ui/src/text-field.proto.ts":79:7
                { let v_7075695f6e61746976655f3930 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3930.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":80:7
                if ({ let left = Value::boolean(!(v_616c6c6f776564.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3931 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f3931.clone()).get() }), &(v_65706f63685f5f33353735343431326666626363363534.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":80:52
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":81:7
                { let v_7075695f6e61746976655f3932 = v_63757272656e745f5f37626163626139663133376134383065.clone(); emit(&(v_7075695f6e61746976655f3932.clone()).owner(), Value::string("selectionRequest"), record(vec![("start", { let value = (v_66726f6d.clone()).borrow().clone(); value }), ("end", { let value = (v_746f.clone()).borrow().clone(); value }), ("viewEpoch", v_65706f63685f5f33353735343431326666626363363534.clone())])) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":82:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":84:5
            { let v_7075695f6e61746976655f3933 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("selectRange"), v_73656c65637452616e6765.read()) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":85:5
            { let v_7075695f6e61746976655f3934 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("selectAll"), {  let v_7374617465 = v_7374617465.clone(); let v_73656c65637452616e6765 = v_73656c65637452616e6765.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":85:42
                return v_73656c65637452616e6765.read().call_authored(vec![Value::number(0.0), ({ let v_7075695f6e61746976655f3935 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f3935.clone()).get() }).member("length", false)]);
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":86:5
            { let v_7075695f6e61746976655f3936 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("requestSubmit"), {  let v_72657175657374 = v_72657175657374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":86:46
                return v_72657175657374.read().call_authored(vec![Value::string("submit"), Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":87:5
            { let v_7075695f6e61746976655f3937 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("requestCancel"), {  let v_72657175657374 = v_72657175657374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":87:46
                return v_72657175657374.read().call_authored(vec![Value::string("cancel"), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":88:5
            { let v_7075695f6e61746976655f3938 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("requestAdvance"), {  let v_72657175657374 = v_72657175657374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_666f72776172645f5f65633536383434616262623633376232: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/text-field.proto.ts":88:63
                return v_72657175657374.read().call_authored(vec![Value::string("advance"), v_666f72776172645f5f65633536383434616262623633376232.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":89:11
            v_7265706c6163654472616674.initialize({  let v_7374617465 = v_7374617465.clone(); let v_7669657745706f63685f5f38393261616630326134373465313530 = v_7669657745706f63685f5f38393261616630326134373465313530.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76616c75655f5f33656633613839653264656633646630: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/text-field.proto.ts":90:13
                let v_63757272656e745f5f36383565636133383737663131333339 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":91:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e745f5f36383565636133383737663131333339.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_616c6c6f776564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3939 = (v_7374617465.read()).member("readOnly", false); (v_7075695f6e61746976655f3939.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":91:59
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":92:7
                { let v_7075695f6e61746976655f313030 = v_63757272656e745f5f36383565636133383737663131333339.clone(); emit(&(v_7075695f6e61746976655f313030.clone()).owner(), Value::string("replacementRequest"), record(vec![("value", v_76616c75655f5f33656633613839653264656633646630.clone()), ("start", Value::number(0.0)), ("end", (v_76616c75655f5f33656633613839653264656633646630.clone()).member("length", false)), ("viewEpoch", { let v_7075695f6e61746976655f313031 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f313031.clone()).get() })])) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":93:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":95:11
            v_7374657054657874.initialize({  let v_7374617465 = v_7374617465.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_616c6c6f776564 = v_616c6c6f776564.clone(); let v_7265706c6163654472616674 = v_7265706c6163654472616674.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_646972656374696f6e: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/text-field.proto.ts":96:13
                let v_63757272656e745f5f65326166373635613865646666643562 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":97:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e745f5f65326166373635613865646666643562.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_616c6c6f776564.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f313032 = v_63757272656e745f5f65326166373635613865646666643562.clone(); read_props(&(v_7075695f6e61746976655f313032.clone()).owner(), false) }).member("numeric", false)).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":97:67
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":98:13
                let v_70726f70735f5f66336531343936393936626632616233 = { let v_7075695f6e61746976655f313033 = v_63757272656e745f5f65326166373635613865646666643562.clone(); read_props(&(v_7075695f6e61746976655f313033.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":99:13
                let v_74657874 = { let v_7075695f6e61746976655f313034 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f313034.clone()).get() };
                // Source "frontend/proto-ui/src/text-field.proto.ts":100:11
                let v_76616c75655f5f32366633383130393865373733353532 = Rc::new(std::cell::RefCell::new(if (Value::boolean(strict_equal(&((v_74657874.clone()).member("length", false)), &(Value::number(0.0))))).truthy() { { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericDefault", false); if left.nullish() { Value::number(0.0) } else { left } } } else { Value::number((v_74657874.clone()).scalar_number()) }));
                // Source "frontend/proto-ui/src/text-field.proto.ts":101:7
                if (Value::boolean(!(number_is_finite(&({ let value = (v_76616c75655f5f32366633383130393865373733353532.clone()).borrow().clone(); value }))).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":101:36
                    { let value = { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericDefault", false); if left.nullish() { Value::number(0.0) } else { left } }; *(v_76616c75655f5f32366633383130393865373733353532.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":102:7
                { let value = add({ let value = (v_76616c75655f5f32366633383130393865373733353532.clone()).borrow().clone(); value }, Value::number((v_646972656374696f6e.clone()).scalar_number() * ({ let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericStep", false); if left.nullish() { Value::number(1.0) } else { left } }).scalar_number())); *(v_76616c75655f5f32366633383130393865373733353532.clone()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":103:7
                if ({ let relational_left = { let value = (v_76616c75655f5f32366633383130393865373733353532.clone()).borrow().clone(); value }; let relational_right = { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericMin", false); if left.nullish() { Value::number(0.0) } else { left } }; Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":103:44
                    { let value = { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericMin", false); if left.nullish() { Value::number(0.0) } else { left } }; *(v_76616c75655f5f32366633383130393865373733353532.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":104:7
                if ({ let relational_left = { let value = (v_76616c75655f5f32366633383130393865373733353532.clone()).borrow().clone(); value }; let relational_right = { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericMax", false); if left.nullish() { Value::number(1.0) } else { left } }; Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":104:44
                    { let value = { let left = (v_70726f70735f5f66336531343936393936626632616233.clone()).member("numericMax", false); if left.nullish() { Value::number(1.0) } else { left } }; *(v_76616c75655f5f32366633383130393865373733353532.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":105:7
                return v_7265706c6163654472616674.read().call_authored(vec![Value::string(&({ let value = (v_76616c75655f5f32366633383130393865373733353532.clone()).borrow().clone(); value }).scalar_text())]);
            })) });
            // Source "frontend/proto-ui/src/text-field.proto.ts":107:5
            { let v_7075695f6e61746976655f313035 = v_6465665f5f35313135366635356438373238343336.clone(); expose_method(&owner, Value::string("stepText"), v_7374657054657874.read()) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":108:5
            { let v_7075695f6e61746976655f313036 = v_6465665f5f35313135366635356438373238343336.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7374617465 = v_7374617465.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_72657175657374 = v_72657175657374.clone(); let v_7265706c6163654472616674 = v_7265706c6163654472616674.clone(); let v_7374657054657874 = v_7374657054657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e74: Value = arg(&arguments, 0);
                let v_6576656e745f5f63346532323966663537306639346437: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/text-field.proto.ts":109:13
                let v_63757272656e745f5f61343336646133343536336633356466 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":110:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e745f5f61343336646133343536336633356466.clone()).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6d6f756e746564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f313037 = (v_7374617465.read()).member("disabled", false); (v_7075695f6e61746976655f313037.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":110:69
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":111:13
                let v_6b6579 = (v_6576656e745f5f63346532323966663537306639346437.clone()).member("key", false);
                // Source "frontend/proto-ui/src/text-field.proto.ts":112:7
                if ({ let v_7075695f6e61746976655f313038 = (v_7374617465.read()).member("composing", false); (v_7075695f6e61746976655f313038.clone()).get() }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":115:9
                    if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Enter")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Tab")))) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":115:67
                        { let v_7075695f6e61746976655f313039 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313039.clone()) };
                    }
                    // Source "frontend/proto-ui/src/text-field.proto.ts":116:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":118:7
                if ({ let left = { let left = (v_6576656e745f5f63346532323966663537306639346437.clone()).member("ctrlKey", false); if !left.truthy() { (v_6576656e745f5f63346532323966663537306639346437.clone()).member("metaKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f63346532323966663537306639346437.clone()).member("altKey", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":118:59
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":119:13
                let v_70726f70735f5f36313661373439316365623732326466 = { let v_7075695f6e61746976655f313130 = v_63757272656e745f5f61343336646133343536336633356466.clone(); read_props(&(v_7075695f6e61746976655f313130.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":120:7
                if ({ let left = (v_70726f70735f5f36313661373439316365623732326466.clone()).member("numeric", false); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowDown")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":121:9
                    { let v_7075695f6e61746976655f313131 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313131.clone()) };
                    // Source "frontend/proto-ui/src/text-field.proto.ts":122:9
                    v_7374657054657874.read().call_authored(vec![if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("ArrowUp"))))).truthy() { Value::number(1.0) } else { Value::number(-(Value::number(1.0)).scalar_number()) }]);
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":123:14
                    if ({ let left = (v_70726f70735f5f36313661373439316365623732326466.clone()).member("numeric", false); if left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Delete")))) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":124:9
                        { let v_7075695f6e61746976655f313132 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313132.clone()) };
                        // Source "frontend/proto-ui/src/text-field.proto.ts":125:9
                        v_7265706c6163654472616674.read().call_authored(vec![Value::string("")]);
                    } else {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":126:14
                        if ({ let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Enter")))); if left.truthy() { (v_70726f70735f5f36313661373439316365623732326466.clone()).member("submitOnEnter", false) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":127:9
                            { let v_7075695f6e61746976655f313133 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313133.clone()) };
                            // Source "frontend/proto-ui/src/text-field.proto.ts":128:9
                            v_72657175657374.read().call_authored(vec![Value::string("submit"), Value::boolean(true)]);
                        } else {
                            // Source "frontend/proto-ui/src/text-field.proto.ts":129:14
                            if ({ let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape")))); if left.truthy() { (v_70726f70735f5f36313661373439316365623732326466.clone()).member("cancelOnEscape", false) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":130:9
                                { let v_7075695f6e61746976655f313134 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313134.clone()) };
                                // Source "frontend/proto-ui/src/text-field.proto.ts":131:9
                                v_72657175657374.read().call_authored(vec![Value::string("cancel"), Value::boolean(false)]);
                            } else {
                                // Source "frontend/proto-ui/src/text-field.proto.ts":132:14
                                if ({ let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Tab")))); if left.truthy() { (v_70726f70735f5f36313661373439316365623732326466.clone()).member("advanceOnTab", false) } else { left } }).truthy() {
                                    // Source "frontend/proto-ui/src/text-field.proto.ts":133:9
                                    { let v_7075695f6e61746976655f313135 = v_6576656e745f5f63346532323966663537306639346437.clone(); prevent(v_7075695f6e61746976655f313135.clone()) };
                                    // Source "frontend/proto-ui/src/text-field.proto.ts":134:9
                                    v_72657175657374.read().call_authored(vec![Value::string("advance"), Value::boolean(!strict_equal(&((v_6576656e745f5f63346532323966663537306639346437.clone()).member("shiftKey", false)), &(Value::boolean(true))))]);
                                }
                            }
                        }
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":137:5
            { let v_7075695f6e61746976655f313136 = v_6465665f5f35313135366635356438373238343336.clone(); listen(&owner, Value::string("host:blur"), false, {  let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_72657175657374 = v_72657175657374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":138:13
                let v_63757272656e745f5f33336562306132326461356232623839 = { let value = (v_72756e5f5f36346432306239616339396665303663.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/text-field.proto.ts":139:7
                if ({ let left = v_63757272656e745f5f33336562306132326461356232623839.clone(); if left.truthy() { ({ let v_7075695f6e61746976655f313137 = v_63757272656e745f5f33336562306132326461356232623839.clone(); read_props(&(v_7075695f6e61746976655f313137.clone()).owner(), false) }).member("commitOnBlur", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":139:56
                    v_72657175657374.read().call_authored(vec![Value::string("blurCommit"), Value::boolean(true)]);
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":141:5
            { let v_7075695f6e61746976655f313138 = v_6465665f5f35313135366635356438373238343336.clone(); register_life(&owner, "mounted", {  let v_7374617465 = v_7374617465.clone(); let v_666f637573 = v_666f637573.clone(); let v_7669657745706f63685f5f38393261616630326134373465313530 = v_7669657745706f63685f5f38393261616630326134373465313530.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73656c65637452616e6765 = v_73656c65637452616e6765.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f64353465646563653464333066353664: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/text-field.proto.ts":142:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":142:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/text-field.proto.ts":143:7
                { let value = v_63757272656e745f5f64353465646563653464333066353664.clone(); *(v_72756e5f5f36346432306239616339396665303663.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":144:7
                { let value = Value::boolean(true); *(v_6d6f756e746564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":145:7
                { let v_7075695f6e61746976655f313139 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); state_set(v_7075695f6e61746976655f313139.clone(), add({ let v_7075695f6e61746976655f313230 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f313230.clone()).get() }, Value::number(1.0)), Value::string("reason: text view mounted")) };
                // Source "frontend/proto-ui/src/text-field.proto.ts":146:7
                if (({ let v_7075695f6e61746976655f313231 = v_63757272656e745f5f64353465646563653464333066353664.clone(); read_props(&(v_7075695f6e61746976655f313231.clone()).owner(), false) }).member("selectAllOnMount", false)).truthy() {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":146:49
                    v_73656c65637452616e6765.read().call_authored(vec![Value::number(0.0), ({ let v_7075695f6e61746976655f313232 = (v_7374617465.read()).member("value", false); (v_7075695f6e61746976655f313232.clone()).get() }).member("length", false)]);
                } else {
                    // Source "frontend/proto-ui/src/text-field.proto.ts":147:12
                    if ({ let left = ({ let v_7075695f6e61746976655f313233 = v_63757272656e745f5f64353465646563653464333066353664.clone(); read_props(&(v_7075695f6e61746976655f313233.clone()).owner(), false) }).member("autoFocus", false); if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313234 = (v_7374617465.read()).member("disabled", false); (v_7075695f6e61746976655f313234.clone()).get() }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/text-field.proto.ts":147:72
                        { let v_7075695f6e61746976655f313235 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313235.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":149:5
            { let v_7075695f6e61746976655f313236 = v_6465665f5f35313135366635356438373238343336.clone(); register_life(&owner, "unmounted", {  let v_7669657745706f63685f5f38393261616630326134373465313530 = v_7669657745706f63685f5f38393261616630326134373465313530.clone(); let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":150:7
                { let value = Value::boolean(false); *(v_6d6f756e746564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":151:7
                { let value = Value::Json(Json::Null); *(v_72756e5f5f36346432306239616339396665303663.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":152:7
                { let v_7075695f6e61746976655f313237 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); state_set(v_7075695f6e61746976655f313237.clone(), add({ let v_7075695f6e61746976655f313238 = v_7669657745706f63685f5f38393261616630326134373465313530.read(); (v_7075695f6e61746976655f313238.clone()).get() }, Value::number(1.0)), Value::string("reason: text view retired")) };
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":154:5
            { let v_7075695f6e61746976655f313239 = v_6465665f5f35313135366635356438373238343336.clone(); register_life(&owner, "before-dispose", {  let v_72756e5f5f36346432306239616339396665303663 = v_72756e5f5f36346432306239616339396665303663.clone(); let v_6d6f756e746564 = v_6d6f756e746564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/text-field.proto.ts":155:7
                { let value = Value::boolean(false); *(v_6d6f756e746564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":156:7
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/text-field.proto.ts":157:7
                { let value = Value::Json(Json::Null); *(v_72756e5f5f36346432306239616339396665303663.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/text-field.proto.ts":159:5
            return { let callable=(v_696e68657269746564.clone()).member("render", false); match callable {
      Value::NativeFunction{render,..} => Value::Renderer(render),
      Value::Renderer(render) => Value::Renderer(render),
      Value::Function(callback) => Value::Renderer(Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
      _ => panic!("checked setup result must be a callable")
    } };
        })) }.call_authored(vec![capability(&owner)]);
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
      pub fn handle(&self) -> TextFieldHandle { TextFieldHandle { owner: self.owner.clone() } }
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
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
