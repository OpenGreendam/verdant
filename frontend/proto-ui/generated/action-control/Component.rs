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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selected").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selectable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("position").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("count").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("expandable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("submitOnEnter").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("contextMenu").map_or(true, |prop| prop.is_null() || prop.is_boolean()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "disabled" => value.is_boolean(),
        "selected" => value.is_boolean(),
        "selectable" => value.is_boolean(),
        "revision" => value.is_string(),
        "position" => value.as_f64().is_some_and(f64::is_finite),
        "count" => value.as_f64().is_some_and(f64::is_finite),
        "expanded" => value.is_boolean(),
        "expandable" => value.is_boolean(),
        "submitOnEnter" => value.is_boolean(),
        "contextMenu" => value.is_boolean(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "requestActivate" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "requestContext" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "focusSelf" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "description", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "role", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<bool>,
    #[serde(rename = "selected", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "selectable", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<String>,
    #[serde(rename = "position", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<f64>,
    #[serde(rename = "count", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<bool>,
    #[serde(rename = "expandable", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<bool>,
    #[serde(rename = "submitOnEnter", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<bool>,
    #[serde(rename = "contextMenu", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<bool>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct ActionControlHandle { pub owner: OwnerRef }
  impl ActionControlHandle {
      pub fn state_0(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("selected").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("pressed").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("hovered").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_10(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("requestActivate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestActivate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_11(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("requestContext").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("requestContext", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_12(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
  pub struct ActionControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl ActionControl {
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
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_6465736372697074696f6e: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_73656c6563746564: LexicalBinding<Value> = lexical_binding();
            let v_746f67676c65: LexicalBinding<Value> = lexical_binding();
            let v_636865636b6564: LexicalBinding<Value> = lexical_binding();
            let v_73656c65637461626c65: LexicalBinding<Value> = lexical_binding();
            let v_70726573736564: LexicalBinding<Value> = lexical_binding();
            let v_657870616e646564: LexicalBinding<Value> = lexical_binding();
            let v_657870616e6461626c65: LexicalBinding<Value> = lexical_binding();
            let v_686f7665726564: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_67656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7465787447656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_707265737347656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265746972655072657373: LexicalBinding<Value> = lexical_binding();
            let v_636c656172: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<Value> = lexical_binding();
            let v_636f6e74657874: LexicalBinding<Value> = lexical_binding();
            let v_73796e6354657874: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/action-control.proto.ts":13:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("button"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selected", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selectable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("position", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("count", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("expanded", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("expandable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("submitOnEnter", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("contextMenu", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":20:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":21:11
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":22:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("button"), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":23:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":24:11
            v_73656c6563746564.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "bool", Value::string("selected"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":25:11
            v_746f67676c65.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("toggle"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":26:11
            v_636865636b6564.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("checkedRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":27:11
            v_73656c65637461626c65.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "bool", Value::string("selectableRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":28:11
            v_70726573736564.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "bool", Value::string("pressed"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":29:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f3130 = v_646566.clone(); create_state(&owner, "bool", Value::string("expanded"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":29:63
            v_657870616e6461626c65.initialize({ let v_7075695f6e61746976655f3131 = v_646566.clone(); create_state(&owner, "bool", Value::string("expandable"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":30:11
            v_686f7665726564.initialize({ let v_7075695f6e61746976655f3132 = v_646566.clone(); create_state(&owner, "bool", Value::string("hovered"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":31:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":32:5
            { let v_7075695f6e61746976655f3133 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3133.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":33:5
            native_builtin_hook(&owner, "asTrigger", || as_trigger(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":34:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3134.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":35:22
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3135.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":35:40
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3136.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3138 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("expanded"), v_657870616e646564.read(), record(vec![("when", v_657870616e6461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3139 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("pressed"), v_73656c6563746564.read(), record(vec![("when", v_746f67676c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3230 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3230.clone()).owner(), Value::string("checked"), v_73656c6563746564.read(), record(vec![("when", v_636865636b6564.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3231 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3231.clone()).owner(), Value::string("selected"), v_73656c6563746564.read(), record(vec![("when", v_73656c65637461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3232 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3232.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-action-activate"))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":42:45
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("selected"), v_73656c6563746564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("pressed"), v_70726573736564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":43:43
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("hovered"), v_686f7665726564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":44:49
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("navigate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("submit")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":48:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":49:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":50:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":51:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":52:9
            v_67656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":53:9
            v_7465787447656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":56:9
            v_707265737347656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::number(-(Value::number(1.0)).scalar_number()))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":57:11
            v_7265746972655072657373.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":58:7
                { let value = Value::number(-(Value::number(1.0)).scalar_number()); *(v_707265737347656e65726174696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":59:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":59:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":60:7
                { let v_7075695f6e61746976655f3333 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3333.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":62:11
            v_636c656172.initialize({  let v_686f7665726564 = v_686f7665726564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":63:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":63:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":64:13
                let v_7469636b65745f5f35353966386333396332383230623631 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":65:7
                v_7265746972655072657373.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":66:7
                if ({ let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }), &(v_7469636b65745f5f35353966386333396332383230623631.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":66:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":67:7
                { let v_7075695f6e61746976655f3334 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f3334.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":69:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":70:7
                if ({ let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3335 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3335.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":70:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":71:13
                let v_7469636b65745f5f66336535303336663465633037633566 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":72:13
                let v_7265766973696f6e5f5f32353234366265383766313038663563 = { let left = ({ let v_7075695f6e61746976655f3336 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3336.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/action-control.proto.ts":73:13
                let v_6e6578745f5f61333734613833326465366237396263 = Value::boolean(!({ let left = ({ let v_7075695f6e61746976655f3337 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3337.clone()).owner(), false) }).member("selected", false); if left.nullish() { Value::boolean(false) } else { left } }).truthy());
                // Source "frontend/proto-ui/src/action-control.proto.ts":74:7
                { let v_7075695f6e61746976655f3338 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3338.clone(), Value::boolean(false), Value::string("reason: action committed")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":75:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3339 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3339.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_7469636b65745f5f66336535303336663465633037633566.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":75:85
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":77:7
                { let v_7075695f6e61746976655f3430 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3430.clone()).owner(), Value::string("activate"), record(vec![("selected", v_6e6578745f5f61333734613833326465366237396263.clone()), ("revision", v_7265766973696f6e5f5f32353234366265383766313038663563.clone())])) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":78:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":80:11
            v_636f6e74657874.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_78: Value = arg(&arguments, 0);
                let v_79: Value = arg(&arguments, 1);
                let v_6b6579626f617264: Value = arg(&arguments, 2);
                let v_6261736973: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/action-control.proto.ts":81:13
                let v_6f776e65725f5f63653964636162353132636136636536 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":82:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_6f776e65725f5f63653964636162353132636136636536.clone()).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3431 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3431.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f3432 = v_6f776e65725f5f63653964636162353132636136636536.clone(); read_props(&(v_7075695f6e61746976655f3432.clone()).owner(), false) }).member("contextMenu", false)).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6261736973.clone()), &({ let left = ({ let v_7075695f6e61746976655f3433 = v_6f776e65725f5f63653964636162353132636136636536.clone(); read_props(&(v_7075695f6e61746976655f3433.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_78.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_79.clone()))).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":83:104
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":84:13
                let v_7469636b65745f5f31623931386632373063336663346336 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":84:34
                { let v_7075695f6e61746976655f3434 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3434.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":85:7
                if ({ let left = { let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f31623931386632373063336663346336.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3435 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3435.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":85:77
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":86:7
                { let v_7075695f6e61746976655f3436 = v_6f776e65725f5f63653964636162353132636136636536.clone(); emit(&(v_7075695f6e61746976655f3436.clone()).owner(), Value::string("context"), record(vec![("x", v_78.clone()), ("y", v_79.clone()), ("keyboard", v_6b6579626f617264.clone()), ("revision", v_6261736973.clone())])) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":87:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":91:11
            v_73796e6354657874.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_7465787447656e65726174696f6e = v_7465787447656e65726174696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f63643062646664633465353830633038: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":92:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":92:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":93:13
                let v_7469636b65745f5f65633561653264373634626237376537 = { let cell = v_7465787447656e65726174696f6e.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":94:13
                let v_706f6c696379 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":95:13
                let v_70726f70735f5f36653832363461336435626566633431 = { let v_7075695f6e61746976655f3437 = v_63757272656e745f5f63643062646664633465353830633038.clone(); read_props(&(v_7075695f6e61746976655f3437.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":96:7
                { let v_7075695f6e61746976655f3438 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3438.clone(), { let left = (v_70726f70735f5f36653832363461336435626566633431.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action name")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":97:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f65633561653264373634626237376537.clone()), &({ let value = (v_7465787447656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_706f6c696379.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":97:75
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":98:7
                { let v_7075695f6e61746976655f3439 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f3439.clone(), { let left = (v_70726f70735f5f36653832363461336435626566633431.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action description")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":99:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f65633561653264373634626237376537.clone()), &({ let value = (v_7465787447656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_706f6c696379.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":99:75
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":100:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":100:21
                    { let v_7075695f6e61746976655f3530 = v_63757272656e745f5f63643062646664633465353830633038.clone(); request_update(&(v_7075695f6e61746976655f3530.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":102:11
            v_73796e63.initialize({  let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_73656c6563746564 = v_73656c6563746564.clone(); let v_746f67676c65 = v_746f67676c65.clone(); let v_636865636b6564 = v_636865636b6564.clone(); let v_73656c65637461626c65 = v_73656c65637461626c65.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_657870616e6461626c65 = v_657870616e6461626c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_636c656172 = v_636c656172.clone(); let v_73796e6354657874 = v_73796e6354657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f36653164323132373632653031303134: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":103:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":103:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":104:13
                let v_7469636b65745f5f64323530326234373762623565393663 = { let cell = v_67656e65726174696f6e.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":105:7
                { let value = v_63757272656e745f5f36653164323132373632653031303134.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":106:13
                let v_70726f70735f5f39333439643761323663643833306561 = { let v_7075695f6e61746976655f3531 = v_63757272656e745f5f36653164323132373632653031303134.clone(); read_props(&(v_7075695f6e61746976655f3531.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":107:7
                { let value = Value::number(-(Value::number(1.0)).scalar_number()); *(v_707265737347656e65726174696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":108:7
                { let v_7075695f6e61746976655f3532 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3532.clone(), Value::boolean(false), Value::string("reason: action authority changed")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":109:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":109:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":110:7
                { let v_7075695f6e61746976655f3533 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3533.clone(), { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: action policy")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":111:7
                if (Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value })))).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":111:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":112:7
                { let v_7075695f6e61746976655f3534 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3534.clone()).owner(), { let v_7075695f6e61746976655f3535 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3535.clone()).get() }) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":113:7
                if (Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value })))).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":113:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":114:13
                let v_70726576696f7573 = { let v_7075695f6e61746976655f3536 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3536.clone()).get() };
                // Source "frontend/proto-ui/src/action-control.proto.ts":115:7
                { let v_7075695f6e61746976655f3537 = v_73656c6563746564.read(); state_set(v_7075695f6e61746976655f3537.clone(), { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("selected", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: accepted selection")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":116:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":116:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":117:7
                { let v_7075695f6e61746976655f3538 = v_666f637573.read(); focus_nav(&(v_7075695f6e61746976655f3538.clone()).owner(), if ({ let left = { let relational_left = { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3539 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3539.clone()).get() }).truthy()) } else { left } }).truthy() { Value::string("none") } else { Value::string("auto") }) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":118:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":118:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":119:7
                if ({ let left = { let left = { let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!(v_70726576696f7573.clone()).truthy()) } else { left } }; if left.truthy() { { let v_7075695f6e61746976655f3630 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3630.clone()).get() } } else { left } }; if left.truthy() { { let relational_left = { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":119:78
                    { let v_7075695f6e61746976655f3631 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3631.clone()).owner(), Value::Undefined, false) };
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":120:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":120:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":121:7
                { let v_7075695f6e61746976655f3632 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f3632.clone(), { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false); if left.nullish() { Value::string("button") } else { left } }, Value::string("reason: action role")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":122:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":122:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":123:7
                { let v_7075695f6e61746976655f3633 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3633.clone(), { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("expanded", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: accepted expanded state")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":124:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":124:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":125:7
                { let v_7075695f6e61746976655f3634 = v_657870616e6461626c65.read(); state_set(v_7075695f6e61746976655f3634.clone(), { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("expandable", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: expansion semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":126:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":126:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":127:7
                v_73796e6354657874.read().call_authored(vec![v_63757272656e745f5f36653164323132373632653031303134.clone()]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":128:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":128:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":129:7
                { let v_7075695f6e61746976655f3635 = v_636865636b6564.read(); state_set(v_7075695f6e61746976655f3635.clone(), { let left = { let left = { let left = Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("switch")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("checkbox")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("menuitemcheckbox")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("radio")))) } else { left } }, Value::string("reason: checked semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":130:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":130:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":131:7
                { let v_7075695f6e61746976655f3636 = v_73656c65637461626c65.read(); state_set(v_7075695f6e61746976655f3636.clone(), { let left = { let left = { let left = Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("tab")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("option")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("row")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f70735f5f39333439643761323663643833306561.clone()).member("role", false)), &(Value::string("menuitemradio")))) } else { left } }, Value::string("reason: selected semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":132:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":132:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":133:7
                { let v_7075695f6e61746976655f3637 = v_746f67676c65.read(); state_set(v_7075695f6e61746976655f3637.clone(), { let left = { let left = { let left = (v_70726f70735f5f39333439643761323663643833306561.clone()).member("selectable", false); if left.nullish() { Value::boolean(false) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3638 = v_636865636b6564.read(); (v_7075695f6e61746976655f3638.clone()).get() }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3639 = v_73656c65637461626c65.read(); (v_7075695f6e61746976655f3639.clone()).get() }).truthy()) } else { left } }, Value::string("reason: toggle semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":134:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f64323530326234373762623565393663.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":134:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":135:7
                if ({ let v_7075695f6e61746976655f3730 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3730.clone()).get() }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":135:27
                    v_636c656172.read().call_authored(vec![]);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":136:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":136:21
                    { let v_7075695f6e61746976655f3731 = v_63757272656e745f5f36653164323132373632653031303134.clone(); request_update(&(v_7075695f6e61746976655f3731.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":138:5
            { let v_7075695f6e61746976655f3732 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":139:5
            { let v_7075695f6e61746976655f3733 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f65643137323666333861303133323838: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":139:44
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":139:61
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":139:78
                    v_73796e63.read().call_authored(vec![v_63757272656e745f5f65643137323666333861303133323838.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":140:5
            { let v_7075695f6e61746976655f3734 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("role"), Value::string("disabled"), Value::string("selected"), Value::string("selectable"), Value::string("revision"), Value::string("position"), Value::string("count"), Value::string("expanded"), Value::string("expandable"), Value::string("submitOnEnter"), Value::string("contextMenu")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":141:5
            { let v_7075695f6e61746976655f3735 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description")]))), false, v_73796e6354657874.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":142:5
            { let v_7075695f6e61746976655f3736 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":143:7
                if ({ let left = { let left = Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3737 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3737.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":143:52
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":144:7
                { let value = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }; *(v_707265737347656e65726174696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":145:7
                { let v_7075695f6e61746976655f3738 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3738.clone(), Value::boolean(true), Value::string("reason: action press")) };
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":147:5
            { let v_7075695f6e61746976655f3739 = v_646566.clone(); listen(&owner, Value::string("pointer.enter"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_686f7665726564 = v_686f7665726564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":147:43
                if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3830 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3830.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":147:76
                    { let v_7075695f6e61746976655f3831 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f3831.clone(), Value::boolean(true), Value::string("reason: action enter")) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":148:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); listen(&owner, Value::string("pointer.leave"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":149:5
            { let v_7075695f6e61746976655f3833 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, v_7265746972655072657373.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":150:5
            { let v_7075695f6e61746976655f3834 = v_646566.clone(); listen(&owner, Value::string("pointer.cancel"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":153:5
            { let v_7075695f6e61746976655f3835 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_7265746972655072657373.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":154:5
            { let v_7075695f6e61746976655f3836 = v_646566.clone(); listen(&owner, Value::string("press.commit"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f65336438376465363933313363356235: Value = arg(&arguments, 0);
                let v_6576656e745f5f33643765326661326539666138303136: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":155:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3837 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3837.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_707265737347656e65726174696f6e.read()).borrow().clone(); value }), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":155:94
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":156:13
                let v_7469636b65745f5f66393865393562373336353233616434 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":157:7
                { let v_7075695f6e61746976655f3838 = v_6576656e745f5f33643765326661326539666138303136.clone(); prevent(v_7075695f6e61746976655f3838.clone()) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":158:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_7469636b65745f5f66393865393562373336353233616434.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_707265737347656e65726174696f6e.read()).borrow().clone(); value }), &(v_7469636b65745f5f66393865393562373336353233616434.clone()))) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3839 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3839.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":158:108
                    v_6163746976617465.read().call_authored(vec![]);
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":160:5
            { let v_7075695f6e61746976655f3930 = v_646566.clone(); listen(&owner, Value::string("key.up"), false, {  let v_70726573736564 = v_70726573736564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f38306366356234346264313330363236: Value = arg(&arguments, 0);
                let v_6576656e745f5f32666336616431383865636161383938: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":161:7
                if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32666336616431383865636161383938.clone()).member("key", false)), &(Value::string("Enter")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f32666336616431383865636161383938.clone()).member("key", false)), &(Value::string(" ")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":162:9
                    { let value = Value::number(-(Value::number(1.0)).scalar_number()); *(v_707265737347656e65726174696f6e.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":163:9
                    if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":163:36
                        { let v_7075695f6e61746976655f3931 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3931.clone(), Value::boolean(false), Value::string("reason: action key released")) };
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":166:5
            { let v_7075695f6e61746976655f3932 = v_646566.clone(); listen(&owner, Value::string("host:verdant-action-activate"), false, v_6163746976617465.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":167:5
            { let v_7075695f6e61746976655f3933 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); let v_636f6e74657874 = v_636f6e74657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f63626666636265613234333037663566: Value = arg(&arguments, 0);
                let v_6576656e745f5f64313337656163363261336566636632: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":168:7
                if ({ let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3934 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3934.clone()).get() }).truthy()) } else { left } }; if left.truthy() { ({ let v_7075695f6e61746976655f3935 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3935.clone()).owner(), false) }).member("contextMenu", false) } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("F10")))); if left.truthy() { (v_6576656e745f5f64313337656163363261336566636632.clone()).member("shiftKey", false) } else { left } } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":170:15
                    let v_6f776e65725f5f37376639363162636235646532333962 = { let value = (v_72756e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":170:34
                    let v_7469636b65745f5f30663332386538356632313361396263 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":170:61
                    let v_7265766973696f6e5f5f64636164356635346137663734323830 = { let left = ({ let v_7075695f6e61746976655f3936 = v_6f776e65725f5f37376639363162636235646532333962.clone(); read_props(&(v_7075695f6e61746976655f3936.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":171:9
                    { let v_7075695f6e61746976655f3937 = v_6576656e745f5f64313337656163363261336566636632.clone(); prevent(v_7075695f6e61746976655f3937.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":172:9
                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_7469636b65745f5f30663332386538356632313361396263.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3938 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3938.clone()).get() }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":172:80
                        v_636f6e74657874.read().call_authored(vec![Value::number(0.0), Value::number(0.0), Value::boolean(true), v_7265766973696f6e5f5f64636164356635346137663734323830.clone()]);
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":173:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":175:7
                if ({ let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3939 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3939.clone()).get() }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("Enter")))) } else { left } }; if left.truthy() { ({ let v_7075695f6e61746976655f313030 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313030.clone()).owner(), false) }).member("submitOnEnter", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":176:15
                    let v_6f776e65725f5f37613261656134393733363730376536 = { let value = (v_72756e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":176:34
                    let v_7469636b65745f5f66346633383333336634343464373435 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":176:61
                    let v_7265766973696f6e5f5f32323965393266366634623633346538 = { let left = ({ let v_7075695f6e61746976655f313031 = v_6f776e65725f5f37613261656134393733363730376536.clone(); read_props(&(v_7075695f6e61746976655f313031.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":177:9
                    { let v_7075695f6e61746976655f313032 = v_6576656e745f5f64313337656163363261336566636632.clone(); prevent(v_7075695f6e61746976655f313032.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":178:9
                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_7469636b65745f5f66346633383333336634343464373435.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313033 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313033.clone()).get() }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":178:80
                        { let v_7075695f6e61746976655f313034 = v_6f776e65725f5f37613261656134393733363730376536.clone(); emit(&(v_7075695f6e61746976655f313034.clone()).owner(), Value::string("submit"), record(vec![("revision", v_7265766973696f6e5f5f32323965393266366634623633346538.clone())])) };
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":179:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":181:7
                if ({ let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313035 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313035.clone()).get() }).truthy()) } else { left } }; if left.truthy() { { let relational_left = { let left = ({ let v_7075695f6e61746976655f313036 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313036.clone()).owner(), false) }).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":182:15
                    let v_636f756e74 = { let left = ({ let v_7075695f6e61746976655f313037 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313037.clone()).owner(), false) }).member("count", false); if left.nullish() { Value::number(0.0) } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":183:15
                    let v_706f736974696f6e = { let left = ({ let v_7075695f6e61746976655f313038 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f313038.clone()).owner(), false) }).member("position", false); if left.nullish() { Value::number(0.0) } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":184:13
                    let v_6e6578745f5f31313261616535656130366464643766 = Rc::new(std::cell::RefCell::new(v_706f736974696f6e.clone()));
                    // Source "frontend/proto-ui/src/action-control.proto.ts":185:9
                    if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ArrowUp")))) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":185:67
                        { let value = Value::number((Value::number((add(v_706f736974696f6e.clone(), v_636f756e74.clone())).scalar_number() - (Value::number(1.0)).scalar_number())).scalar_number() % (v_636f756e74.clone()).scalar_number()); *(v_6e6578745f5f31313261616535656130366464643766.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":186:14
                        if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ArrowRight")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ArrowDown")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":186:75
                            { let value = Value::number((add(v_706f736974696f6e.clone(), Value::number(1.0))).scalar_number() % (v_636f756e74.clone()).scalar_number()); *(v_6e6578745f5f31313261616535656130366464643766.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":187:14
                            if (Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("Home"))))).truthy() {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":187:40
                                { let value = Value::number(0.0); *(v_6e6578745f5f31313261616535656130366464643766.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":188:14
                                if (Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("End"))))).truthy() {
                                    // Source "frontend/proto-ui/src/action-control.proto.ts":188:39
                                    { let value = Value::number((v_636f756e74.clone()).scalar_number() - (Value::number(1.0)).scalar_number()); *(v_6e6578745f5f31313261616535656130366464643766.clone()).borrow_mut() = value; }
                                }
                            }
                        }
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":189:9
                    if (Value::boolean(!strict_equal(&({ let value = (v_6e6578745f5f31313261616535656130366464643766.clone()).borrow().clone(); value }), &(v_706f736974696f6e.clone())))).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":190:17
                        let v_6f776e65725f5f37656135663636366633356266613961 = { let value = (v_72756e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":191:17
                        let v_7469636b65745f5f30633562646536323864643738373839 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":192:17
                        let v_7265766973696f6e5f5f38666564653237366564616562343633 = { let left = ({ let v_7075695f6e61746976655f313039 = v_6f776e65725f5f37656135663636366633356266613961.clone(); read_props(&(v_7075695f6e61746976655f313039.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":193:11
                        { let v_7075695f6e61746976655f313130 = v_6576656e745f5f64313337656163363261336566636632.clone(); prevent(v_7075695f6e61746976655f313130.clone()) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":194:11
                        if ({ let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }), &(v_7469636b65745f5f30633562646536323864643738373839.clone()))); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f313131 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313131.clone()).get() } } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":194:81
                            return Value::Undefined;
                        }
                        // Source "frontend/proto-ui/src/action-control.proto.ts":195:11
                        { let v_7075695f6e61746976655f313132 = v_6f776e65725f5f37656135663636366633356266613961.clone(); emit(&(v_7075695f6e61746976655f313132.clone()).owner(), Value::string("navigate"), record(vec![("index", { let value = (v_6e6578745f5f31313261616535656130366464643766.clone()).borrow().clone(); value }), ("revision", v_7265766973696f6e5f5f38666564653237366564616562343633.clone())])) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":196:11
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":199:7
                if ({ let left = { let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313133 = v_726f6c65.read(); (v_7075695f6e61746976655f313133.clone()).get() }), &(Value::string("combobox")))); if left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f64313337656163363261336566636632.clone()).member("key", false)), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313134 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313134.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":200:15
                    let v_7469636b65745f5f63663261346334306434303162353366 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":201:9
                    { let v_7075695f6e61746976655f313135 = v_6576656e745f5f64313337656163363261336566636632.clone(); prevent(v_7075695f6e61746976655f313135.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":202:9
                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_7469636b65745f5f63663261346334306434303162353366.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313136 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313136.clone()).get() }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":202:80
                        v_6163746976617465.read().call_authored(vec![]);
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":205:5
            { let v_7075695f6e61746976655f313137 = v_646566.clone(); expose_method(&owner, Value::string("requestActivate"), v_6163746976617465.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":206:5
            { let v_7075695f6e61746976655f313138 = v_646566.clone(); expose_method(&owner, Value::string("requestContext"), v_636f6e74657874.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":207:5
            { let v_7075695f6e61746976655f313139 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":207:44
                if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313230 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313230.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":207:77
                    { let v_7075695f6e61746976655f313231 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313231.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":208:5
            { let v_7075695f6e61746976655f313232 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_636c656172 = v_636c656172.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":208:39
                { let cell = v_67656e65726174696f6e.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/action-control.proto.ts":208:53
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":208:71
                v_636c656172.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":208:80
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":209:5
            { let v_7075695f6e61746976655f313233 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":209:43
                { let cell = v_67656e65726174696f6e.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/action-control.proto.ts":209:57
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":209:74
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":209:92
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":212:5
            { let v_7075695f6e61746976655f313234 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("absolute".into(), "absolute".into()), ("inset-0".into(), "inset-0".into()), ("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":213:5
            return { let callable={ let pui_native_125={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/action-control.proto.ts":213:26
                return { let v_7075695f6e61746976655f313236 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_125.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_125.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ActionControlHandle { ActionControlHandle { owner: self.owner.clone() } }
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
  impl Drop for ActionControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for ActionControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for ActionControl {
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
