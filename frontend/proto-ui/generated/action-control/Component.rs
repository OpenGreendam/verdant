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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selected").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selectable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("tabStop").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("contextOnly").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("position").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("count").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("expandable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("submitOnEnter").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("contextMenu").map_or(true, |prop| prop.is_null() || prop.is_boolean()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "disabled" => value.is_boolean(),
        "selected" => value.is_boolean(),
        "selectable" => value.is_boolean(),
        "tabStop" => value.is_boolean(),
        "contextOnly" => value.is_boolean(),
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
    #[serde(rename = "tabStop", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "contextOnly", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<String>,
    #[serde(rename = "position", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<f64>,
    #[serde(rename = "count", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<f64>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<bool>,
    #[serde(rename = "expandable", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<bool>,
    #[serde(rename = "submitOnEnter", default, skip_serializing_if = "Option::is_none")]
    pub field_13: Option<bool>,
    #[serde(rename = "contextMenu", default, skip_serializing_if = "Option::is_none")]
    pub field_14: Option<bool>,
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
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_67656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7465787447656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_707265737347656e65726174696f6e: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_656e7465725375626d6974746564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_7265746972655072657373: LexicalBinding<Value> = lexical_binding();
            let v_636c656172: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, Value, Value, bool, bool, bool, bool) -> bool>> = lexical_binding();
            let v_73796e6354657874: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/action-control.proto.ts":13:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("button"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selected", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selectable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("tabStop", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("contextOnly", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("position", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("count", record(vec![("type", Value::string("number")), ("default", Value::number(0.0_f64))])), ("expanded", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("expandable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("submitOnEnter", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("contextMenu", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":21:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":22:11
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":23:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("button"), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":24:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":25:11
            v_73656c6563746564.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "bool", Value::string("selected"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":26:11
            v_746f67676c65.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("toggle"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":27:11
            v_636865636b6564.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("checkedRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":28:11
            v_73656c65637461626c65.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "bool", Value::string("selectableRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":29:11
            v_70726573736564.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "bool", Value::string("pressed"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":30:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f3130 = v_646566.clone(); create_state(&owner, "bool", Value::string("expanded"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":30:63
            v_657870616e6461626c65.initialize({ let v_7075695f6e61746976655f3131 = v_646566.clone(); create_state(&owner, "bool", Value::string("expandable"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":31:11
            v_686f7665726564.initialize({ let v_7075695f6e61746976655f3132 = v_646566.clone(); create_state(&owner, "bool", Value::string("hovered"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":32:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":33:5
            { let v_7075695f6e61746976655f3133 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3133.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":34:5
            native_builtin_hook(&owner, "asTrigger", || as_trigger(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":35:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3134.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":36:22
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3135.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":36:40
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3136.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3138 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("expanded"), v_657870616e646564.read(), record(vec![("when", v_657870616e6461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3139 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("pressed"), v_73656c6563746564.read(), record(vec![("when", v_746f67676c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3230 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3230.clone()).owner(), Value::string("checked"), v_73656c6563746564.read(), record(vec![("when", v_636865636b6564.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3231 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3231.clone()).owner(), Value::string("selected"), v_73656c6563746564.read(), record(vec![("when", v_73656c65637461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3232 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3232.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-action-activate"))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":43:45
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("selected"), v_73656c6563746564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("pressed"), v_70726573736564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":44:43
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("hovered"), v_686f7665726564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":45:49
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("navigate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":48:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("submit")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":49:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":50:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":51:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":52:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":53:9
            v_67656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":54:9
            v_7465787447656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":57:9
            v_707265737347656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(-(1.0_f64))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":59:9
            v_656e7465725375626d6974746564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":60:11
            v_7265746972655072657373.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":61:7
                Value::number({ let value=-(1.0_f64); *(v_707265737347656e65726174696f6e.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":62:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":62:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":63:7
                { let v_7075695f6e61746976655f3333 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3333.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":65:11
            v_636c656172.initialize({  let v_686f7665726564 = v_686f7665726564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":66:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":66:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":67:13
                let v_7469636b65745f5f36653363326432653664353963626566: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":68:7
                v_7265746972655072657373.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":69:7
                if { let pui_native_34={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_34) {{ let pui_native_36={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; let pui_native_37=v_7469636b65745f5f36653363326432653664353963626566.clone(); !(pui_native_36==pui_native_37) }} else {pui_native_34} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":69:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":70:7
                { let v_7075695f6e61746976655f3338 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f3338.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":72:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); let call:Rc<dyn Fn(Value, Value, Value, bool, bool, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: Value, argument_2: Value, argument_3: bool, argument_4: bool, argument_5: bool, argument_6: bool| -> bool {
                let v_6e616d655f5f39383331323330656633643539333464: Value = argument_0;
                let v_6f726967696e: Value = argument_1;
                let v_6b6579: Value = argument_2;
                let v_73686966744b6579: bool = argument_3;
                let v_6374726c4b6579: bool = argument_4;
                let v_6d6574614b6579: bool = argument_5;
                let v_616c744b6579: bool = argument_6;
                // Source "frontend/proto-ui/src/action-control.proto.ts":73:7
                if { let pui_native_39={ let pui_native_41={ let pui_native_43=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_43) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_43} }; if !(pui_native_41) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_41} }; if !(pui_native_39) {({ let v_7075695f6e61746976655f3435 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3435.clone()).get()).bool()) }).bool()} else {pui_native_39} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":73:60
                    return false;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":74:13
                let v_7469636b65745f5f38313533346661333832633863653634: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":75:13
                let v_7265766973696f6e5f5f66646635653733363037666537363730 = { let left = ({ let v_7075695f6e61746976655f3436 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3436.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/action-control.proto.ts":76:7
                if { let pui_native_47=(Value::boolean(strict_equal(&(v_6e616d655f5f39383331323330656633643539333464.clone()), &(Value::string("context"))))).bool(); if (pui_native_47) {!((({ let v_7075695f6e61746976655f3531 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3531.clone()).owner(), false) }).member("contextMenu", false)).bool())} else {pui_native_47} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":76:63
                    return false;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":77:13
                let v_70726f70735f5f30613265646334343938646230323433 = { let v_7075695f6e61746976655f3532 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3532.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":78:13
                let v_6163636570746564: bool = ((v_70726f70735f5f30613265646334343938646230323433.clone()).member("selected", false)).bool();
                // Source "frontend/proto-ui/src/action-control.proto.ts":79:13
                let v_6578636c7573697665: bool = { let pui_native_55={ let pui_native_57=(Value::boolean(strict_equal(&((v_70726f70735f5f30613265646334343938646230323433.clone()).member("role", false)), &(Value::string("radio"))))).bool(); if !(pui_native_57) {(Value::boolean(strict_equal(&((v_70726f70735f5f30613265646334343938646230323433.clone()).member("role", false)), &(Value::string("menuitemradio"))))).bool()} else {pui_native_57} }; if !(pui_native_55) {(Value::boolean(strict_equal(&((v_70726f70735f5f30613265646334343938646230323433.clone()).member("role", false)), &(Value::string("tab"))))).bool()} else {pui_native_55} };
                // Source "frontend/proto-ui/src/action-control.proto.ts":82:13
                let v_6e6578745f5f31366635656339353831623832626665: bool = if (Value::boolean(strict_equal(&(v_6e616d655f5f39383331323330656633643539333464.clone()), &(Value::string("context"))))).bool() {v_6163636570746564.clone()} else {if v_6578636c7573697665.clone() {true} else {!(v_6163636570746564.clone())}};
                // Source "frontend/proto-ui/src/action-control.proto.ts":83:7
                v_7265746972655072657373.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":84:7
                if { let pui_native_67={ let pui_native_69={ let pui_native_71={ let pui_native_73=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_73) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_73} }; if !(pui_native_71) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_71} }; if !(pui_native_69) {({ let v_7075695f6e61746976655f3735 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3735.clone()).get()).bool()) }).bool()} else {pui_native_69} }; if !(pui_native_67) {{ let pui_native_76=v_7469636b65745f5f38313533346661333832633863653634.clone(); let pui_native_77={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_76==pui_native_77) }} else {pui_native_67} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":84:85
                    return false;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":86:7
                { let v_7075695f6e61746976655f3738 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3738.clone()).owner(), v_6e616d655f5f39383331323330656633643539333464.clone(), record(vec![("selected", Value::boolean(v_6e6578745f5f31366635656339353831623832626665.clone())), ("revision", v_7265766973696f6e5f5f66646635653733363037666537363730.clone()), ("origin", v_6f726967696e.clone()), ("key", v_6b6579.clone()), ("shiftKey", Value::boolean(v_73686966744b6579.clone())), ("ctrlKey", Value::boolean(v_6374726c4b6579.clone())), ("metaKey", Value::boolean(v_6d6574614b6579.clone())), ("altKey", Value::boolean(v_616c744b6579.clone()))])) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":87:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), arg(&arguments,1), arg(&arguments,2), (arg(&arguments,3)).bool(), (arg(&arguments,4)).bool(), (arg(&arguments,5)).bool(), (arg(&arguments,6)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/action-control.proto.ts":89:11
            let v_636f6e74657874 = {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let call:Rc<dyn Fn(f64, f64, bool, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: bool, argument_3: Value| -> bool {
                let v_78: f64 = argument_0;
                let v_79: f64 = argument_1;
                let v_6b6579626f617264: bool = argument_2;
                let v_6261736973: Value = argument_3;
                // Source "frontend/proto-ui/src/action-control.proto.ts":90:13
                let v_6f776e65725f5f64373466623961393766663863353331 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":91:7
                if { let pui_native_79={ let pui_native_81={ let pui_native_83={ let pui_native_85={ let pui_native_87={ let pui_native_89={ let pui_native_91=!((v_6f776e65725f5f64373466623961393766663863353331.clone()).truthy()); if !(pui_native_91) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_91} }; if !(pui_native_89) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_89} }; if !(pui_native_87) {({ let v_7075695f6e61746976655f3933 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3933.clone()).get()).bool()) }).bool()} else {pui_native_87} }; if !(pui_native_85) {!((({ let v_7075695f6e61746976655f3934 = v_6f776e65725f5f64373466623961393766663863353331.clone(); read_props(&(v_7075695f6e61746976655f3934.clone()).owner(), false) }).member("contextMenu", false)).bool())} else {pui_native_85} }; if !(pui_native_83) {(Value::boolean(!strict_equal(&(v_6261736973.clone()), &({ let left = ({ let v_7075695f6e61746976655f3937 = v_6f776e65725f5f64373466623961393766663863353331.clone(); read_props(&(v_7075695f6e61746976655f3937.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })))).bool()} else {pui_native_83} }; if !(pui_native_81) {!((v_78.clone()).is_finite())} else {pui_native_81} }; if !(pui_native_79) {!((v_79.clone()).is_finite())} else {pui_native_79} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":92:104
                    return false;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":93:13
                let v_7469636b65745f5f35343037633838383661313330386538: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":93:34
                { let v_7075695f6e61746976655f3938 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3938.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":94:7
                if { let pui_native_99={ let pui_native_101={ let pui_native_103={ let pui_native_105=v_7469636b65745f5f35343037633838383661313330386538.clone(); let pui_native_106={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_105==pui_native_106) }; if !(pui_native_103) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_103} }; if !(pui_native_101) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_101} }; if !(pui_native_99) {({ let v_7075695f6e61746976655f313037 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313037.clone()).get()).bool()) }).bool()} else {pui_native_99} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":94:77
                    return false;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":95:7
                { let v_7075695f6e61746976655f313038 = v_6f776e65725f5f64373466623961393766663863353331.clone(); emit(&(v_7075695f6e61746976655f313038.clone()).owner(), Value::string("context"), record(vec![("x", Value::number(v_78.clone())), ("y", Value::number(v_79.clone())), ("keyboard", Value::boolean(v_6b6579626f617264.clone())), ("selected", Value::boolean((({ let v_7075695f6e61746976655f313131 = v_6f776e65725f5f64373466623961393766663863353331.clone(); read_props(&(v_7075695f6e61746976655f313131.clone()).owner(), false) }).member("selected", false)).bool())), ("revision", v_6261736973.clone()), ("origin", if (Value::boolean(v_6b6579626f617264.clone())).truthy() { Value::string("keyboard") } else { Value::string("press") }), ("key", if (Value::boolean(v_6b6579626f617264.clone())).truthy() { Value::string("ContextMenu") } else { Value::string("") }), ("shiftKey", Value::boolean(false)), ("ctrlKey", Value::boolean(false)), ("metaKey", Value::boolean(false)), ("altKey", Value::boolean(false))])) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":96:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).bool(), arg(&arguments,3)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } };
            // Source "frontend/proto-ui/src/action-control.proto.ts":100:11
            v_73796e6354657874.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_7465787447656e65726174696f6e = v_7465787447656e65726174696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f65333530383632616561646465663262: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":101:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":101:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":102:13
                let v_7469636b65745f5f36633363343766656263376232306431: f64 = { let cell=v_7465787447656e65726174696f6e.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":103:13
                let v_706f6c696379: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":104:13
                let v_70726f70735f5f37663734326134306233653635663833 = { let v_7075695f6e61746976655f313132 = v_63757272656e745f5f65333530383632616561646465663262.clone(); read_props(&(v_7075695f6e61746976655f313132.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":105:7
                { let v_7075695f6e61746976655f313133 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313133.clone(), { let left = (v_70726f70735f5f37663734326134306233653635663833.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action name")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":106:7
                if { let pui_native_114={ let pui_native_116={ let pui_native_118=v_7469636b65745f5f36633363343766656263376232306431.clone(); let pui_native_119={ let value=(v_7465787447656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_118==pui_native_119) }; if !(pui_native_116) {{ let pui_native_120=v_706f6c696379.clone(); let pui_native_121={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_120==pui_native_121) }} else {pui_native_116} }; if !(pui_native_114) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_114} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":106:75
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":107:7
                { let v_7075695f6e61746976655f313232 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f313232.clone(), { let left = (v_70726f70735f5f37663734326134306233653635663833.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action description")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":108:7
                if { let pui_native_123={ let pui_native_125={ let pui_native_127=v_7469636b65745f5f36633363343766656263376232306431.clone(); let pui_native_128={ let value=(v_7465787447656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_127==pui_native_128) }; if !(pui_native_125) {{ let pui_native_129=v_706f6c696379.clone(); let pui_native_130={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_129==pui_native_130) }} else {pui_native_125} }; if !(pui_native_123) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_123} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":108:75
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":109:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":109:21
                    { let v_7075695f6e61746976655f313331 = v_63757272656e745f5f65333530383632616561646465663262.clone(); request_update(&(v_7075695f6e61746976655f313331.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":111:11
            v_73796e63.initialize({  let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_73656c6563746564 = v_73656c6563746564.clone(); let v_746f67676c65 = v_746f67676c65.clone(); let v_636865636b6564 = v_636865636b6564.clone(); let v_73656c65637461626c65 = v_73656c65637461626c65.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_657870616e6461626c65 = v_657870616e6461626c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_636c656172 = v_636c656172.clone(); let v_73796e6354657874 = v_73796e6354657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f37386535613665363764323739396532: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":112:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":112:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":113:13
                let v_7469636b65745f5f33343035633535663930373039643266: f64 = { let cell=v_67656e65726174696f6e.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":114:7
                { let value = v_63757272656e745f5f37386535613665363764323739396532.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":115:13
                let v_70726f70735f5f33663235343335393332653036383761 = { let v_7075695f6e61746976655f313332 = v_63757272656e745f5f37386535613665363764323739396532.clone(); read_props(&(v_7075695f6e61746976655f313332.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":116:7
                Value::number({ let value=-(1.0_f64); *(v_707265737347656e65726174696f6e.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":117:7
                { let v_7075695f6e61746976655f313333 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f313333.clone(), Value::boolean(false), Value::string("reason: action authority changed")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":118:7
                if { let pui_native_134={ let pui_native_136=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_137={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_136==pui_native_137) }; if !(pui_native_134) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_134} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":118:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":119:7
                { let v_7075695f6e61746976655f313338 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f313338.clone(), Value::boolean(((v_70726f70735f5f33663235343335393332653036383761.clone()).member("disabled", false)).bool()), Value::string("reason: action policy")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":120:7
                if { let pui_native_141=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_142={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_141==pui_native_142) } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":120:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":121:7
                { let v_7075695f6e61746976655f313433 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313433.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f313434 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313434.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":122:7
                if { let pui_native_145=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_146={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_145==pui_native_146) } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":122:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":123:13
                let v_70726576696f7573: bool = ({ let v_7075695f6e61746976655f313437 = v_73656c6563746564.read(); Value::boolean(((v_7075695f6e61746976655f313437.clone()).get()).bool()) }).bool();
                // Source "frontend/proto-ui/src/action-control.proto.ts":124:7
                { let v_7075695f6e61746976655f313438 = v_73656c6563746564.read(); state_set(v_7075695f6e61746976655f313438.clone(), Value::boolean(((v_70726f70735f5f33663235343335393332653036383761.clone()).member("selected", false)).bool()), Value::string("reason: accepted selection")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":125:7
                if { let pui_native_151={ let pui_native_153=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_154={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_153==pui_native_154) }; if !(pui_native_151) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_151} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":125:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":126:7
                { let v_7075695f6e61746976655f313535 = v_666f637573.read(); focus_nav(&(v_7075695f6e61746976655f313535.clone()).owner(), if (Value::boolean({ let pui_native_156={ let pui_native_158=((v_70726f70735f5f33663235343335393332653036383761.clone()).member("tabStop", false)).bool(); let pui_native_159=false; (pui_native_158==pui_native_159) }; if !(pui_native_156) {{ let pui_native_160={ let pui_native_162:f64=((v_70726f70735f5f33663235343335393332653036383761.clone()).member("count", false)).num(); let pui_native_163:f64=0.0_f64; pui_native_162 > pui_native_163 }; if (pui_native_160) {!(({ let v_7075695f6e61746976655f313636 = v_73656c6563746564.read(); Value::boolean(((v_7075695f6e61746976655f313636.clone()).get()).bool()) }).bool())} else {pui_native_160} }} else {pui_native_156} })).truthy() { Value::string("none") } else { Value::string("auto") }) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":127:7
                if { let pui_native_167={ let pui_native_169=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_170={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_169==pui_native_170) }; if !(pui_native_167) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_167} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":127:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":128:7
                if { let pui_native_171={ let pui_native_173={ let pui_native_175={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_175) {!(v_70726576696f7573.clone())} else {pui_native_175} }; if (pui_native_173) {({ let v_7075695f6e61746976655f313737 = v_73656c6563746564.read(); Value::boolean(((v_7075695f6e61746976655f313737.clone()).get()).bool()) }).bool()} else {pui_native_173} }; if (pui_native_171) {{ let pui_native_178:f64=((v_70726f70735f5f33663235343335393332653036383761.clone()).member("count", false)).num(); let pui_native_179:f64=0.0_f64; pui_native_178 > pui_native_179 }} else {pui_native_171} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":128:78
                    { let v_7075695f6e61746976655f313832 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313832.clone()).owner(), Value::Undefined, false) };
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":129:7
                if { let pui_native_183={ let pui_native_185=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_186={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_185==pui_native_186) }; if !(pui_native_183) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_183} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":129:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":130:7
                { let v_7075695f6e61746976655f313837 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f313837.clone(), { let left = (v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false); if left.nullish() { Value::string("button") } else { left } }, Value::string("reason: action role")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":131:7
                if { let pui_native_188={ let pui_native_190=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_191={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_190==pui_native_191) }; if !(pui_native_188) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_188} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":131:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":132:7
                { let v_7075695f6e61746976655f313932 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f313932.clone(), Value::boolean(((v_70726f70735f5f33663235343335393332653036383761.clone()).member("expanded", false)).bool()), Value::string("reason: accepted expanded state")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":133:7
                if { let pui_native_195={ let pui_native_197=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_198={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_197==pui_native_198) }; if !(pui_native_195) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_195} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":133:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":134:7
                { let v_7075695f6e61746976655f313939 = v_657870616e6461626c65.read(); state_set(v_7075695f6e61746976655f313939.clone(), Value::boolean(((v_70726f70735f5f33663235343335393332653036383761.clone()).member("expandable", false)).bool()), Value::string("reason: expansion semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":135:7
                if { let pui_native_202={ let pui_native_204=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_205={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_204==pui_native_205) }; if !(pui_native_202) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_202} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":135:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":136:7
                v_73796e6354657874.read().call_authored(vec![v_63757272656e745f5f37386535613665363764323739396532.clone()]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":137:7
                if { let pui_native_206={ let pui_native_208=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_209={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_208==pui_native_209) }; if !(pui_native_206) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_206} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":137:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":138:7
                { let v_7075695f6e61746976655f323130 = v_636865636b6564.read(); state_set(v_7075695f6e61746976655f323130.clone(), Value::boolean({ let pui_native_211={ let pui_native_213={ let pui_native_215={ let pui_native_217=(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("switch"))))).bool(); if !(pui_native_217) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("checkbox"))))).bool()} else {pui_native_217} }; if !(pui_native_215) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("menuitemcheckbox"))))).bool()} else {pui_native_215} }; if !(pui_native_213) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("menuitemradio"))))).bool()} else {pui_native_213} }; if !(pui_native_211) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("radio"))))).bool()} else {pui_native_211} }), Value::string("reason: checked semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":139:7
                if { let pui_native_229={ let pui_native_231=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_232={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_231==pui_native_232) }; if !(pui_native_229) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_229} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":139:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":140:7
                { let v_7075695f6e61746976655f323333 = v_73656c65637461626c65.read(); state_set(v_7075695f6e61746976655f323333.clone(), Value::boolean({ let pui_native_234={ let pui_native_236=(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("tab"))))).bool(); if !(pui_native_236) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("option"))))).bool()} else {pui_native_236} }; if !(pui_native_234) {(Value::boolean(strict_equal(&((v_70726f70735f5f33663235343335393332653036383761.clone()).member("role", false)), &(Value::string("row"))))).bool()} else {pui_native_234} }), Value::string("reason: selected semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":141:7
                if { let pui_native_244={ let pui_native_246=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_247={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_246==pui_native_247) }; if !(pui_native_244) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_244} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":141:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":142:7
                { let v_7075695f6e61746976655f323438 = v_746f67676c65.read(); state_set(v_7075695f6e61746976655f323438.clone(), Value::boolean({ let pui_native_249={ let pui_native_251=((v_70726f70735f5f33663235343335393332653036383761.clone()).member("selectable", false)).bool(); if (pui_native_251) {!(({ let v_7075695f6e61746976655f323535 = v_636865636b6564.read(); Value::boolean(((v_7075695f6e61746976655f323535.clone()).get()).bool()) }).bool())} else {pui_native_251} }; if (pui_native_249) {!(({ let v_7075695f6e61746976655f323536 = v_73656c65637461626c65.read(); Value::boolean(((v_7075695f6e61746976655f323536.clone()).get()).bool()) }).bool())} else {pui_native_249} }), Value::string("reason: toggle semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":143:7
                if { let pui_native_257={ let pui_native_259=v_7469636b65745f5f33343035633535663930373039643266.clone(); let pui_native_260={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_259==pui_native_260) }; if !(pui_native_257) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_257} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":143:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":144:7
                if ({ let v_7075695f6e61746976655f323631 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323631.clone()).get()).bool()) }).bool() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":144:27
                    v_636c656172.read().call_authored(vec![]);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":145:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":145:21
                    { let v_7075695f6e61746976655f323632 = v_63757272656e745f5f37386535613665363764323739396532.clone(); request_update(&(v_7075695f6e61746976655f323632.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":147:5
            { let v_7075695f6e61746976655f323633 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":148:5
            { let v_7075695f6e61746976655f323634 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f31303162386363376364363731626434: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":148:44
                if !({ let value=(v_646973706f736564.read()).borrow().clone(); value }) {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":148:61
                    Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/action-control.proto.ts":148:78
                    v_73796e63.read().call_authored(vec![v_63757272656e745f5f31303162386363376364363731626434.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":149:5
            { let v_7075695f6e61746976655f323635 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("role"), Value::string("disabled"), Value::string("selected"), Value::string("selectable"), Value::string("revision"), Value::string("position"), Value::string("count"), Value::string("expanded"), Value::string("expandable"), Value::string("submitOnEnter"), Value::string("contextMenu"), Value::string("contextOnly"), Value::string("tabStop")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":150:5
            { let v_7075695f6e61746976655f323636 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description")]))), false, v_73796e6354657874.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":151:5
            { let v_7075695f6e61746976655f323637 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f61366332653935353961643336353265: Value = arg(&arguments, 0);
                let v_6576656e745f5f37623264376230383337303036636565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":152:7
                if { let pui_native_268={ let pui_native_270={ let pui_native_272=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_272) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_272} }; if !(pui_native_270) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_270} }; if !(pui_native_268) {({ let v_7075695f6e61746976655f323734 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323734.clone()).get()).bool()) }).bool()} else {pui_native_268} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":152:60
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":153:7
                if { let pui_native_275=(Value::boolean(strict_equal(&((v_6576656e745f5f37623264376230383337303036636565.clone()).member("key", false)), &(Value::string("Enter"))))).bool(); if (pui_native_275) {{ let pui_native_279={ let value=(v_656e7465725375626d6974746564.read()).borrow().clone(); value }; if !(pui_native_279) {(({ let v_7075695f6e61746976655f323831 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f323831.clone()).owner(), false) }).member("submitOnEnter", false)).bool()} else {pui_native_279} }} else {pui_native_275} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":153:89
                    v_7265746972655072657373.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/action-control.proto.ts":153:104
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":154:7
                Value::number({ let value={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; *(v_707265737347656e65726174696f6e.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":155:7
                { let v_7075695f6e61746976655f323832 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f323832.clone(), Value::boolean(true), Value::string("reason: action press")) };
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":157:5
            { let v_7075695f6e61746976655f323833 = v_646566.clone(); listen(&owner, Value::string("pointer.enter"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_686f7665726564 = v_686f7665726564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":157:43
                if { let pui_native_284={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_284) {!(({ let v_7075695f6e61746976655f323836 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323836.clone()).get()).bool()) }).bool())} else {pui_native_284} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":157:76
                    { let v_7075695f6e61746976655f323837 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f323837.clone(), Value::boolean(true), Value::string("reason: action enter")) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":158:5
            { let v_7075695f6e61746976655f323838 = v_646566.clone(); listen(&owner, Value::string("pointer.leave"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":159:5
            { let v_7075695f6e61746976655f323839 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, v_7265746972655072657373.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":160:5
            { let v_7075695f6e61746976655f323930 = v_646566.clone(); listen(&owner, Value::string("pointer.cancel"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":163:5
            { let v_7075695f6e61746976655f323931 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_7265746972655072657373.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":164:5
            { let v_7075695f6e61746976655f323932 = v_646566.clone(); listen(&owner, Value::string("press.commit"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f32313037396636333366333937653763: Value = arg(&arguments, 0);
                let v_6576656e745f5f61636231386538393462386265323761: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":165:7
                if { let pui_native_293={ let pui_native_295={ let pui_native_297=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_297) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_297} }; if !(pui_native_295) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_295} }; if !(pui_native_293) {({ let v_7075695f6e61746976655f323939 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323939.clone()).get()).bool()) }).bool()} else {pui_native_293} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":165:60
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":166:7
                if { let pui_native_300=(Value::boolean(strict_equal(&((v_6576656e745f5f61636231386538393462386265323761.clone()).member("key", false)), &(Value::string("Enter"))))).bool(); if (pui_native_300) {{ let pui_native_304={ let value=(v_656e7465725375626d6974746564.read()).borrow().clone(); value }; if !(pui_native_304) {(({ let v_7075695f6e61746976655f333036 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f333036.clone()).owner(), false) }).member("submitOnEnter", false)).bool()} else {pui_native_304} }} else {pui_native_300} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":167:9
                    v_7265746972655072657373.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/action-control.proto.ts":167:24
                    { let v_7075695f6e61746976655f333037 = v_6576656e745f5f61636231386538393462386265323761.clone(); prevent(v_7075695f6e61746976655f333037.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":167:72
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":169:7
                if { let pui_native_308={ let value=(v_707265737347656e65726174696f6e.read()).borrow().clone(); value }; let pui_native_309={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_308==pui_native_309) } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":169:43
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":170:13
                let v_7469636b65745f5f38353065646332323332636564333134: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":171:13
                let v_6e616d655f5f38303539313132333036343137356134 = if ({ let left = (v_6576656e745f5f61636231386538393462386265323761.clone()).member("key", false); if left.truthy() { Value::boolean((({ let v_7075695f6e61746976655f333130 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f333130.clone()).owner(), false) }).member("contextOnly", false)).bool()) } else { left } }).truthy() { Value::string("context") } else { Value::string("activate") };
                // Source "frontend/proto-ui/src/action-control.proto.ts":172:7
                { let v_7075695f6e61746976655f333131 = v_6576656e745f5f61636231386538393462386265323761.clone(); prevent(v_7075695f6e61746976655f333131.clone()) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":173:7
                if { let pui_native_312={ let pui_native_314={ let pui_native_316={ let pui_native_318={ let pui_native_320=v_7469636b65745f5f38353065646332323332636564333134.clone(); let pui_native_321={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; (pui_native_320==pui_native_321) }; if (pui_native_318) {{ let pui_native_322={ let value=(v_707265737347656e65726174696f6e.read()).borrow().clone(); value }; let pui_native_323=v_7469636b65745f5f38353065646332323332636564333134.clone(); (pui_native_322==pui_native_323) }} else {pui_native_318} }; if (pui_native_316) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_316} }; if (pui_native_314) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_314} }; if (pui_native_312) {!(({ let v_7075695f6e61746976655f333234 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333234.clone()).get()).bool()) }).bool())} else {pui_native_312} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":174:9
                    Value::boolean((v_6163746976617465.read().call)(v_6e616d655f5f38303539313132333036343137356134.clone(), if ((v_6576656e745f5f61636231386538393462386265323761.clone()).member("key", false)).truthy() { Value::string("keyboard") } else { Value::string("press") }, { let left = (v_6576656e745f5f61636231386538393462386265323761.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e745f5f61636231386538393462386265323761.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f61636231386538393462386265323761.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f61636231386538393462386265323761.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f61636231386538393462386265323761.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()));
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":178:5
            { let v_7075695f6e61746976655f333333 = v_646566.clone(); listen(&owner, Value::string("key.up"), false, {  let v_70726573736564 = v_70726573736564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_707265737347656e65726174696f6e = v_707265737347656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f62316139663263636363383138313130: Value = arg(&arguments, 0);
                let v_6576656e745f5f30393931386233323531366136613531: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":179:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f30393931386233323531366136613531.clone()).member("key", false)), &(Value::string("Enter"))))).bool() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":179:34
                    Value::boolean({ let value=false; *(v_656e7465725375626d6974746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":180:7
                if { let pui_native_336=(Value::boolean(strict_equal(&((v_6576656e745f5f30393931386233323531366136613531.clone()).member("key", false)), &(Value::string("Enter"))))).bool(); if !(pui_native_336) {(Value::boolean(strict_equal(&((v_6576656e745f5f30393931386233323531366136613531.clone()).member("key", false)), &(Value::string(" "))))).bool()} else {pui_native_336} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":181:9
                    Value::number({ let value=-(1.0_f64); *(v_707265737347656e65726174696f6e.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/action-control.proto.ts":182:9
                    if { let pui_native_342={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_342) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_342} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":182:36
                        { let v_7075695f6e61746976655f333434 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f333434.clone(), Value::boolean(false), Value::string("reason: action key released")) };
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":185:5
            { let v_7075695f6e61746976655f333435 = v_646566.clone(); listen(&owner, Value::string("host:verdant-action-activate"), false, ({  let v_72756e = v_72756e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/action-control.proto.ts":185:56
                return (v_6163746976617465.read().call)(if (({ let v_7075695f6e61746976655f333436 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f333436.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f333436.clone()).owner(), false) } }).member("contextOnly", true)).truthy() { Value::string("context") } else { Value::string("activate") }, Value::string("assistive"), Value::string(""), false, false, false, false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":186:5
            { let v_7075695f6e61746976655f333437 = v_646566.clone(); listen(&owner, Value::string("context.menu"), false, {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f63666361646666336433343531346532: Value = arg(&arguments, 0);
                let v_6576656e745f5f33633039626238613038623063393565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":187:7
                if (v_6163746976617465.read().call)(Value::string("context"), Value::string("press"), Value::string(""), (Value::boolean(strict_equal(&((v_6576656e745f5f33633039626238613038623063393565.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f33633039626238613038623063393565.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f33633039626238613038623063393565.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f33633039626238613038623063393565.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()) {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":188:57
                    { let v_7075695f6e61746976655f333536 = v_6576656e745f5f33633039626238613038623063393565.clone(); prevent(v_7075695f6e61746976655f333536.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":190:5
            { let v_7075695f6e61746976655f333537 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); let v_7265746972655072657373 = v_7265746972655072657373.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f61656437363566323038313862353139: Value = arg(&arguments, 0);
                let v_6576656e745f5f36643133363061346632656430323436: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":191:7
                if { let pui_native_358=(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("Enter"))))).bool(); if (pui_native_358) {!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("repeat", false)).truthy())} else {pui_native_358} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":191:51
                    Value::boolean({ let value=false; *(v_656e7465725375626d6974746564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":192:7
                if ({ let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let value=(v_6174746163686564.read()).borrow().clone(); value }) } else { left } }; if left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f333632 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333632.clone()).get()).bool()) }).bool())) } else { left } }; if left.truthy() { Value::boolean((({ let v_7075695f6e61746976655f333633 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f333633.clone()).owner(), false) }).member("contextMenu", false)).bool()) } else { left } }; if left.truthy() { { let left = Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ContextMenu"))))).bool()); if !left.truthy() { { let left = { let left = { let left = { let left = Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("F10"))))).bool()); if left.truthy() { (v_6576656e745f5f36643133363061346632656430323436.clone()).member("shiftKey", false) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("ctrlKey", false)).truthy())) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("metaKey", false)).truthy())) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("altKey", false)).truthy())) } else { left } } } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":194:15
                    let v_7469636b65745f5f39663738616230396435346335343764: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":195:9
                    { let v_7075695f6e61746976655f333638 = v_6576656e745f5f36643133363061346632656430323436.clone(); prevent(v_7075695f6e61746976655f333638.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":196:9
                    if { let pui_native_369={ let pui_native_371={ let pui_native_373={ let pui_native_375=v_7469636b65745f5f39663738616230396435346335343764.clone(); let pui_native_376={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; (pui_native_375==pui_native_376) }; if (pui_native_373) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_373} }; if (pui_native_371) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_371} }; if (pui_native_369) {!(({ let v_7075695f6e61746976655f333737 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333737.clone()).get()).bool()) }).bool())} else {pui_native_369} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":197:11
                        Value::boolean((v_6163746976617465.read().call)(Value::string("context"), Value::string("keyboard"), { let left = (v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()));
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":200:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":202:7
                if ({ let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let value=(v_6174746163686564.read()).borrow().clone(); value }) } else { left } }; if left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f333836 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f333836.clone()).get()).bool()) }).bool())) } else { left } }; if left.truthy() { Value::boolean((Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("Enter"))))).bool()) } else { left } }; if left.truthy() { Value::boolean((({ let v_7075695f6e61746976655f333839 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f333839.clone()).owner(), false) }).member("submitOnEnter", false)).bool()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":203:15
                    let v_6f776e65725f5f36613861316465376366386463313764 = { let value = (v_72756e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":203:34
                    let v_7469636b65745f5f62343237356633626333306466313064: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":203:61
                    let v_7265766973696f6e5f5f39613737316334653963353866663931 = { let left = ({ let v_7075695f6e61746976655f333930 = v_6f776e65725f5f36613861316465376366386463313764.clone(); read_props(&(v_7075695f6e61746976655f333930.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":204:9
                    Value::boolean({ let value=true; *(v_656e7465725375626d6974746564.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/action-control.proto.ts":205:9
                    { let v_7075695f6e61746976655f333931 = v_6576656e745f5f36643133363061346632656430323436.clone(); prevent(v_7075695f6e61746976655f333931.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":206:9
                    if { let pui_native_392={ let pui_native_394={ let pui_native_396={ let pui_native_398=v_7469636b65745f5f62343237356633626333306466313064.clone(); let pui_native_399={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; !(pui_native_398==pui_native_399) }; if !(pui_native_396) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_396} }; if !(pui_native_394) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_394} }; if !(pui_native_392) {({ let v_7075695f6e61746976655f343030 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343030.clone()).get()).bool()) }).bool()} else {pui_native_392} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":206:79
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":207:9
                    v_7265746972655072657373.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/action-control.proto.ts":208:9
                    if { let pui_native_401={ let pui_native_403={ let pui_native_405={ let pui_native_407=v_7469636b65745f5f62343237356633626333306466313064.clone(); let pui_native_408={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; (pui_native_407==pui_native_408) }; if (pui_native_405) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_405} }; if (pui_native_403) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_403} }; if (pui_native_401) {!(({ let v_7075695f6e61746976655f343039 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343039.clone()).get()).bool()) }).bool())} else {pui_native_401} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":208:80
                        { let v_7075695f6e61746976655f343130 = v_6f776e65725f5f36613861316465376366386463313764.clone(); emit(&(v_7075695f6e61746976655f343130.clone()).owner(), Value::string("submit"), record(vec![("revision", v_7265766973696f6e5f5f39613737316334653963353866663931.clone())])) };
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":209:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":211:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { Value::boolean({ let value=(v_6174746163686564.read()).borrow().clone(); value }) } else { left } }; if left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f343131 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343131.clone()).get()).bool()) }).bool())) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("ctrlKey", false)).truthy())) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("metaKey", false)).truthy())) } else { left } }; if left.truthy() { Value::boolean(!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("altKey", false)).truthy())) } else { left } }; if left.truthy() { Value::boolean({ let pui_native_412:f64=(({ let v_7075695f6e61746976655f343136 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f343136.clone()).owner(), false) }).member("count", false)).num(); let pui_native_413:f64=0.0_f64; pui_native_412 > pui_native_413 }) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":212:15
                    let v_636f756e74: f64 = (({ let v_7075695f6e61746976655f343139 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f343139.clone()).owner(), false) }).member("count", false)).num();
                    // Source "frontend/proto-ui/src/action-control.proto.ts":213:15
                    let v_706f736974696f6e: f64 = (({ let v_7075695f6e61746976655f343232 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f343232.clone()).owner(), false) }).member("position", false)).num();
                    // Source "frontend/proto-ui/src/action-control.proto.ts":214:13
                    let v_6e6578745f5f35653536346337316565353932366634: Rc<std::cell::RefCell<f64>> = Rc::new(std::cell::RefCell::new(v_706f736974696f6e.clone()));
                    // Source "frontend/proto-ui/src/action-control.proto.ts":215:9
                    if { let pui_native_423=(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ArrowLeft"))))).bool(); if !(pui_native_423) {(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ArrowUp"))))).bool()} else {pui_native_423} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":215:67
                        Value::number({ let value={ let pui_native_429:f64={ let pui_native_431:f64={ let pui_native_433:f64=v_706f736974696f6e.clone(); let pui_native_434:f64=v_636f756e74.clone(); pui_native_433 + pui_native_434 }; let pui_native_432:f64=1.0_f64; pui_native_431 - pui_native_432 }; let pui_native_430:f64=v_636f756e74.clone(); pui_native_429 % pui_native_430 }; *(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow_mut()=value.clone(); value });
                    } else {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":216:14
                        if { let pui_native_435=(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ArrowRight"))))).bool(); if !(pui_native_435) {(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ArrowDown"))))).bool()} else {pui_native_435} } {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":216:75
                            Value::number({ let value={ let pui_native_441:f64={ let pui_native_443:f64=v_706f736974696f6e.clone(); let pui_native_444:f64=1.0_f64; pui_native_443 + pui_native_444 }; let pui_native_442:f64=v_636f756e74.clone(); pui_native_441 % pui_native_442 }; *(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow_mut()=value.clone(); value });
                        } else {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":217:14
                            if (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("Home"))))).bool() {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":217:40
                                Value::number({ let value=0.0_f64; *(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow_mut()=value.clone(); value });
                            } else {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":218:14
                                if (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("End"))))).bool() {
                                    // Source "frontend/proto-ui/src/action-control.proto.ts":218:39
                                    Value::number({ let value={ let pui_native_449:f64=v_636f756e74.clone(); let pui_native_450:f64=1.0_f64; pui_native_449 - pui_native_450 }; *(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow_mut()=value.clone(); value });
                                }
                            }
                        }
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":219:9
                    if { let pui_native_451={ let value=(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow().clone(); value }; let pui_native_452=v_706f736974696f6e.clone(); !(pui_native_451==pui_native_452) } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":220:17
                        let v_6f776e65725f5f39383533356139376462356638343230 = { let value = (v_72756e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":221:17
                        let v_7469636b65745f5f39386136303636363130643461663131: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":222:17
                        let v_7265766973696f6e5f5f33303139663763633835313033363366 = { let left = ({ let v_7075695f6e61746976655f343533 = v_6f776e65725f5f39383533356139376462356638343230.clone(); read_props(&(v_7075695f6e61746976655f343533.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":223:11
                        { let v_7075695f6e61746976655f343534 = v_6576656e745f5f36643133363061346632656430323436.clone(); prevent(v_7075695f6e61746976655f343534.clone()) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":224:11
                        if { let pui_native_455={ let pui_native_457={ let pui_native_459={ let pui_native_461={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; let pui_native_462=v_7469636b65745f5f39386136303636363130643461663131.clone(); !(pui_native_461==pui_native_462) }; if !(pui_native_459) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_459} }; if !(pui_native_457) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_457} }; if !(pui_native_455) {({ let v_7075695f6e61746976655f343633 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343633.clone()).get()).bool()) }).bool()} else {pui_native_455} } {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":224:81
                            return Value::Undefined;
                        }
                        // Source "frontend/proto-ui/src/action-control.proto.ts":225:11
                        { let v_7075695f6e61746976655f343634 = v_6f776e65725f5f39383533356139376462356638343230.clone(); emit(&(v_7075695f6e61746976655f343634.clone()).owner(), Value::string("navigate"), record(vec![("index", Value::number({ let value=(v_6e6578745f5f35653536346337316565353932366634.clone()).borrow().clone(); value })), ("revision", v_7265766973696f6e5f5f33303139663763633835313033363366.clone())])) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":226:11
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":229:7
                if { let pui_native_465={ let pui_native_467={ let pui_native_469={ let pui_native_471={ let pui_native_473=(Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f343737 = v_726f6c65.read(); (v_7075695f6e61746976655f343737.clone()).get() }), &(Value::string("combobox"))))).bool(); if (pui_native_473) {(Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false)), &(Value::string("ArrowDown"))))).bool()} else {pui_native_473} }; if (pui_native_471) {!(({ let v_7075695f6e61746976655f343830 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343830.clone()).get()).bool()) }).bool())} else {pui_native_471} }; if (pui_native_469) {!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("ctrlKey", false)).truthy())} else {pui_native_469} }; if (pui_native_467) {!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("metaKey", false)).truthy())} else {pui_native_467} }; if (pui_native_465) {!(((v_6576656e745f5f36643133363061346632656430323436.clone()).member("altKey", false)).truthy())} else {pui_native_465} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":230:15
                    let v_7469636b65745f5f66383962373831633434363663633433: f64 = { let value=(v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":231:9
                    { let v_7075695f6e61746976655f343831 = v_6576656e745f5f36643133363061346632656430323436.clone(); prevent(v_7075695f6e61746976655f343831.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":232:9
                    if { let pui_native_482={ let pui_native_484={ let pui_native_486={ let pui_native_488=v_7469636b65745f5f66383962373831633434363663633433.clone(); let pui_native_489={ let value=(v_67656e65726174696f6e.read()).borrow().clone(); value }; (pui_native_488==pui_native_489) }; if (pui_native_486) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_486} }; if (pui_native_484) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_484} }; if (pui_native_482) {!(({ let v_7075695f6e61746976655f343930 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f343930.clone()).get()).bool()) }).bool())} else {pui_native_482} } {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":232:80
                        Value::boolean((v_6163746976617465.read().call)(Value::string("activate"), Value::string("keyboard"), { let left = (v_6576656e745f5f36643133363061346632656430323436.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool(), (Value::boolean(strict_equal(&((v_6576656e745f5f36643133363061346632656430323436.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()));
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":235:5
            { let v_7075695f6e61746976655f343939 = v_646566.clone(); expose_method(&owner, Value::string("requestActivate"), ({  let v_6163746976617465 = v_6163746976617465.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/action-control.proto.ts":235:48
                return (v_6163746976617465.read().call)(Value::string("activate"), Value::string("programmatic"), Value::string(""), false, false, false, false);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":236:5
            { let v_7075695f6e61746976655f353030 = v_646566.clone(); expose_method(&owner, Value::string("requestContext"), (v_636f6e74657874.clone()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":237:5
            { let v_7075695f6e61746976655f353031 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":237:44
                if { let pui_native_502={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_502) {!(({ let v_7075695f6e61746976655f353034 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f353034.clone()).get()).bool()) }).bool())} else {pui_native_502} } {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":237:77
                    { let v_7075695f6e61746976655f353035 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f353035.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":238:5
            { let v_7075695f6e61746976655f353036 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); let v_636c656172 = v_636c656172.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":238:39
                Value::number({ let cell=v_67656e65726174696f6e.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":238:53
                Value::boolean({ let value=false; *(v_656e7465725375626d6974746564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":238:77
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":238:95
                v_636c656172.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":238:104
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":239:5
            { let v_7075695f6e61746976655f353037 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_656e7465725375626d6974746564 = v_656e7465725375626d6974746564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":239:43
                Value::number({ let cell=v_67656e65726174696f6e.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":239:57
                Value::boolean({ let value=false; *(v_656e7465725375626d6974746564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":239:81
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":239:98
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/action-control.proto.ts":239:116
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":242:5
            { let v_7075695f6e61746976655f353038 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("absolute".into(), "absolute".into()), ("inset-0".into(), "inset-0".into()), ("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":243:5
            return { let callable={ let pui_native_509={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/action-control.proto.ts":243:26
                return { let v_7075695f6e61746976655f353130 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_509.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_509.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ActionControlHandle { ActionControlHandle { owner: self.owner.clone() } }
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
