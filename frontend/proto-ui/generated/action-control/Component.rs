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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selected").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("selectable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("tabStop").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("contextMenu").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("contextOnly").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("position").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("count").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("expandable").map_or(true, |prop| prop.is_null() || prop.is_boolean()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "disabled" => value.is_boolean(),
        "selected" => value.is_boolean(),
        "selectable" => value.is_boolean(),
        "tabStop" => value.is_boolean(),
        "contextMenu" => value.is_boolean(),
        "contextOnly" => value.is_boolean(),
        "revision" => value.is_string(),
        "position" => value.as_f64().is_some_and(f64::is_finite),
        "count" => value.as_f64().is_some_and(f64::is_finite),
        "expanded" => value.is_boolean(),
        "expandable" => value.is_boolean(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "requestActivate" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "contextMenu", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "contextOnly", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<String>,
    #[serde(rename = "position", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<f64>,
    #[serde(rename = "count", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<f64>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<bool>,
    #[serde(rename = "expandable", default, skip_serializing_if = "Option::is_none")]
    pub field_13: Option<bool>,
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
    pub fn method_9(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_10(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        catch_authored(|| {
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
  pub struct ActionControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl ActionControl {
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
            let v_636c656172: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/action-control.proto.ts":13:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("button"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selected", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("selectable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("tabStop", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("contextMenu", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("contextOnly", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("position", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("count", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("expanded", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("expandable", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":23:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":24:11
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":25:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("button"), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":26:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":27:11
            v_73656c6563746564.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "bool", Value::string("selected"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":28:11
            v_746f67676c65.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("toggle"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":29:11
            v_636865636b6564.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "bool", Value::string("checkedRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":30:11
            v_73656c65637461626c65.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "bool", Value::string("selectableRole"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":31:11
            v_70726573736564.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "bool", Value::string("pressed"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":32:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f3130 = v_646566.clone(); create_state(&owner, "bool", Value::string("expanded"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":32:63
            v_657870616e6461626c65.initialize({ let v_7075695f6e61746976655f3131 = v_646566.clone(); create_state(&owner, "bool", Value::string("expandable"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":33:11
            v_686f7665726564.initialize({ let v_7075695f6e61746976655f3132 = v_646566.clone(); create_state(&owner, "bool", Value::string("hovered"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":34:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/action-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3133 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3133.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":36:5
            native_builtin_hook(&owner, "asTrigger", || as_trigger(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":37:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/action-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3134.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":38:22
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3135.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":38:40
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3136.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":39:5
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":40:5
            { let v_7075695f6e61746976655f3138 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("expanded"), v_657870616e646564.read(), record(vec![("when", v_657870616e6461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":41:5
            { let v_7075695f6e61746976655f3139 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("pressed"), v_73656c6563746564.read(), record(vec![("when", v_746f67676c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":42:5
            { let v_7075695f6e61746976655f3230 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3230.clone()).owner(), Value::string("checked"), v_73656c6563746564.read(), record(vec![("when", v_636865636b6564.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":43:5
            { let v_7075695f6e61746976655f3231 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3231.clone()).owner(), Value::string("selected"), v_73656c6563746564.read(), record(vec![("when", v_73656c65637461626c65.read())])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":44:5
            { let v_7075695f6e61746976655f3232 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3232.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-action-activate"))])) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":45:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":45:45
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("selected"), v_73656c6563746564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":46:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("pressed"), v_70726573736564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":46:43
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("hovered"), v_686f7665726564.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":47:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":47:49
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":48:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":49:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":50:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("navigate")) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":51:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":52:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":53:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":54:9
            v_67656e65726174696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/action-control.proto.ts":55:11
            v_636c656172.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_686f7665726564 = v_686f7665726564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":56:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":56:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":57:13
                let v_7469636b65745f5f31353366316431333835386232363062 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":58:7
                { let v_7075695f6e61746976655f3332 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3332.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":59:7
                if ({ let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }), &(v_7469636b65745f5f31353366316431333835386232363062.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":59:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":60:7
                { let v_7075695f6e61746976655f3333 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f3333.clone(), Value::boolean(false), Value::string("reason: action retired")) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":62:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f65343663646361386530656438396330: Value = arg(&arguments, 0);
                let v_6f726967696e: Value = arg(&arguments, 1);
                let v_6b6579: Value = arg(&arguments, 2);
                let v_73686966744b6579: Value = arg(&arguments, 3);
                let v_6374726c4b6579: Value = arg(&arguments, 4);
                let v_6d6574614b6579: Value = arg(&arguments, 5);
                let v_616c744b6579: Value = arg(&arguments, 6);
                // Source "frontend/proto-ui/src/action-control.proto.ts":63:7
                if ({ let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3334 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3334.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":63:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":64:7
                if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f65343663646361386530656438396330.clone()), &(Value::string("context")))); if left.truthy() { Value::boolean(!(({ let v_7075695f6e61746976655f3335 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3335.clone()).owner(), false) }).member("contextMenu", false)).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":64:63
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":65:13
                let v_7469636b65745f5f37323166333730356663396534386538 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":66:13
                let v_7265766973696f6e5f5f64303264363865356434393064336263 = { let left = ({ let v_7075695f6e61746976655f3336 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3336.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/action-control.proto.ts":67:13
                let v_6e6578745f5f36653363326432653664353963626566 = if (Value::boolean(strict_equal(&(v_6e616d655f5f65343663646361386530656438396330.clone()), &(Value::string("context"))))).truthy() { { let left = ({ let v_7075695f6e61746976655f3337 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3337.clone()).owner(), false) }).member("selected", false); if left.nullish() { Value::boolean(false) } else { left } } } else { Value::boolean(!({ let left = ({ let v_7075695f6e61746976655f3338 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3338.clone()).owner(), false) }).member("selected", false); if left.nullish() { Value::boolean(false) } else { left } }).truthy()) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":68:7
                { let v_7075695f6e61746976655f3339 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3339.clone(), Value::boolean(false), Value::string("reason: action committed")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":69:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3430 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3430.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_7469636b65745f5f37323166333730356663396534386538.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":69:85
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":71:7
                { let v_7075695f6e61746976655f3431 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3431.clone()).owner(), v_6e616d655f5f65343663646361386530656438396330.clone(), record(vec![("selected", v_6e6578745f5f36653363326432653664353963626566.clone()), ("revision", v_7265766973696f6e5f5f64303264363865356434393064336263.clone()), ("origin", v_6f726967696e.clone()), ("key", v_6b6579.clone()), ("shiftKey", v_73686966744b6579.clone()), ("ctrlKey", v_6374726c4b6579.clone()), ("metaKey", v_6d6574614b6579.clone()), ("altKey", v_616c744b6579.clone())])) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":72:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":74:11
            v_73796e63.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_73656c6563746564 = v_73656c6563746564.clone(); let v_746f67676c65 = v_746f67676c65.clone(); let v_636865636b6564 = v_636865636b6564.clone(); let v_73656c65637461626c65 = v_73656c65637461626c65.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_657870616e6461626c65 = v_657870616e6461626c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_636c656172 = v_636c656172.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f63303962373339663362633037343138: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":75:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":75:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":76:13
                let v_7469636b65745f5f38373331373031353666373366626266 = { let cell = v_67656e65726174696f6e.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/action-control.proto.ts":77:7
                { let value = v_63757272656e745f5f63303962373339663362633037343138.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":78:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3432 = v_63757272656e745f5f63303962373339663362633037343138.clone(); read_props(&(v_7075695f6e61746976655f3432.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":79:7
                { let v_7075695f6e61746976655f3433 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3433.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: action policy")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":80:7
                if (Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value })))).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":80:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":81:7
                { let v_7075695f6e61746976655f3434 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3434.clone()).owner(), { let v_7075695f6e61746976655f3435 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3435.clone()).get() }) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":82:7
                if (Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value })))).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":82:34
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":83:13
                let v_70726576696f7573 = { let v_7075695f6e61746976655f3436 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3436.clone()).get() };
                // Source "frontend/proto-ui/src/action-control.proto.ts":84:7
                { let v_7075695f6e61746976655f3437 = v_73656c6563746564.read(); state_set(v_7075695f6e61746976655f3437.clone(), { let left = (v_70726f7073.clone()).member("selected", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: accepted selection")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":85:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":85:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":86:7
                { let v_7075695f6e61746976655f3438 = v_666f637573.read(); focus_nav(&(v_7075695f6e61746976655f3438.clone()).owner(), if ({ let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("tabStop", false)), &(Value::boolean(false)))); if !left.truthy() { { let left = { let relational_left = { let left = (v_70726f7073.clone()).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3439 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3439.clone()).get() }).truthy()) } else { left } } } else { left } }).truthy() { Value::string("none") } else { Value::string("auto") }) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":87:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":87:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":88:7
                if ({ let left = { let left = { let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!(v_70726576696f7573.clone()).truthy()) } else { left } }; if left.truthy() { { let v_7075695f6e61746976655f3530 = v_73656c6563746564.read(); (v_7075695f6e61746976655f3530.clone()).get() } } else { left } }; if left.truthy() { { let relational_left = { let left = (v_70726f7073.clone()).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":88:78
                    { let v_7075695f6e61746976655f3531 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3531.clone()).owner(), Value::Undefined, false) };
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":89:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":89:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":90:7
                { let v_7075695f6e61746976655f3532 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f3532.clone(), { let left = (v_70726f7073.clone()).member("role", false); if left.nullish() { Value::string("button") } else { left } }, Value::string("reason: action role")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":91:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":91:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":92:7
                { let v_7075695f6e61746976655f3533 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3533.clone(), { let left = (v_70726f7073.clone()).member("expanded", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: accepted expanded state")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":93:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":93:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":94:7
                { let v_7075695f6e61746976655f3534 = v_657870616e6461626c65.read(); state_set(v_7075695f6e61746976655f3534.clone(), { let left = (v_70726f7073.clone()).member("expandable", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: expansion semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":95:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":95:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":96:7
                { let v_7075695f6e61746976655f3535 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3535.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action name")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":97:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":97:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":98:7
                { let v_7075695f6e61746976655f3536 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f3536.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: action description")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":99:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":99:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":100:7
                { let v_7075695f6e61746976655f3537 = v_636865636b6564.read(); state_set(v_7075695f6e61746976655f3537.clone(), { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("switch")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("checkbox")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("menuitemcheckbox")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("menuitemradio")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("radio")))) } else { left } }, Value::string("reason: checked semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":101:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":101:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":102:7
                { let v_7075695f6e61746976655f3538 = v_73656c65637461626c65.read(); state_set(v_7075695f6e61746976655f3538.clone(), { let left = { let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("tab")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("option")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("role", false)), &(Value::string("row")))) } else { left } }, Value::string("reason: selected semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":103:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":103:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":104:7
                { let v_7075695f6e61746976655f3539 = v_746f67676c65.read(); state_set(v_7075695f6e61746976655f3539.clone(), { let left = { let left = { let left = (v_70726f7073.clone()).member("selectable", false); if left.nullish() { Value::boolean(false) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3630 = v_636865636b6564.read(); (v_7075695f6e61746976655f3630.clone()).get() }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3631 = v_73656c65637461626c65.read(); (v_7075695f6e61746976655f3631.clone()).get() }).truthy()) } else { left } }, Value::string("reason: toggle semantics")) };
                // Source "frontend/proto-ui/src/action-control.proto.ts":105:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f38373331373031353666373366626266.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":105:46
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":106:7
                if ({ let v_7075695f6e61746976655f3632 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3632.clone()).get() }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":106:27
                    v_636c656172.read().call_authored(vec![]);
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":107:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":107:21
                    { let v_7075695f6e61746976655f3633 = v_63757272656e745f5f63303962373339663362633037343138.clone(); request_update(&(v_7075695f6e61746976655f3633.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/action-control.proto.ts":109:5
            { let v_7075695f6e61746976655f3634 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":110:5
            { let v_7075695f6e61746976655f3635 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_63757272656e745f5f66393363303166623535663730363366: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/action-control.proto.ts":110:44
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":110:61
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":110:78
                    v_73796e63.read().call_authored(vec![v_63757272656e745f5f66393363303166623535663730363366.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":111:5
            { let v_7075695f6e61746976655f3636 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description"), Value::string("role"), Value::string("disabled"), Value::string("selected"), Value::string("selectable"), Value::string("tabStop"), Value::string("contextMenu"), Value::string("contextOnly"), Value::string("revision"), Value::string("position"), Value::string("count"), Value::string("expanded"), Value::string("expandable")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":112:5
            { let v_7075695f6e61746976655f3637 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_70726573736564 = v_70726573736564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":113:7
                if ({ let left = { let left = Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3638 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3638.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":113:52
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":114:7
                { let v_7075695f6e61746976655f3639 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3639.clone(), Value::boolean(true), Value::string("reason: action press")) };
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":116:5
            { let v_7075695f6e61746976655f3730 = v_646566.clone(); listen(&owner, Value::string("pointer.enter"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_686f7665726564 = v_686f7665726564.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":116:43
                if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3731 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3731.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":116:76
                    { let v_7075695f6e61746976655f3732 = v_686f7665726564.read(); state_set(v_7075695f6e61746976655f3732.clone(), Value::boolean(true), Value::string("reason: action enter")) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":117:5
            { let v_7075695f6e61746976655f3733 = v_646566.clone(); listen(&owner, Value::string("pointer.leave"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":118:5
            { let v_7075695f6e61746976655f3734 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":119:5
            { let v_7075695f6e61746976655f3735 = v_646566.clone(); listen(&owner, Value::string("pointer.cancel"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":120:5
            { let v_7075695f6e61746976655f3736 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_636c656172.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":121:5
            { let v_7075695f6e61746976655f3737 = v_646566.clone(); listen(&owner, Value::string("press.commit"), false, {  let v_72756e = v_72756e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f33326631383732636264666662333563: Value = arg(&arguments, 0);
                let v_6576656e745f5f33666566633330323261656266336638: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":122:13
                let v_6e616d655f5f33383331663666356330633433623464 = if ({ let left = (v_6576656e745f5f33666566633330323261656266336638.clone()).member("key", false); if left.truthy() { ({ let v_7075695f6e61746976655f3738 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3738.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3738.clone()).owner(), false) } }).member("contextOnly", true) } else { left } }).truthy() { Value::string("context") } else { Value::string("activate") };
                // Source "frontend/proto-ui/src/action-control.proto.ts":123:7
                if (v_6163746976617465.read().call_authored(vec![v_6e616d655f5f33383331663666356330633433623464.clone(), if ((v_6576656e745f5f33666566633330323261656266336638.clone()).member("key", false)).truthy() { Value::string("keyboard") } else { Value::string("press") }, { let left = (v_6576656e745f5f33666566633330323261656266336638.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f33666566633330323261656266336638.clone()).member("shiftKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f33666566633330323261656266336638.clone()).member("ctrlKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f33666566633330323261656266336638.clone()).member("metaKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f33666566633330323261656266336638.clone()).member("altKey", false)), &(Value::boolean(true))))])).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":125:9
                    { let v_7075695f6e61746976655f3739 = v_6576656e745f5f33666566633330323261656266336638.clone(); prevent(v_7075695f6e61746976655f3739.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":128:5
            { let v_7075695f6e61746976655f3830 = v_646566.clone(); listen(&owner, Value::string("host:verdant-action-activate"), false, {  let v_72756e = v_72756e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":128:56
                return v_6163746976617465.read().call_authored(vec![if (({ let v_7075695f6e61746976655f3831 = { let value = (v_72756e.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3831.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3831.clone()).owner(), false) } }).member("contextOnly", true)).truthy() { Value::string("context") } else { Value::string("activate") }, Value::string("assistive"), Value::string(""), Value::boolean(false), Value::boolean(false), Value::boolean(false), Value::boolean(false)]);
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":129:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); listen(&owner, Value::string("context.menu"), false, {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f39396534363038343036306132393461: Value = arg(&arguments, 0);
                let v_6576656e745f5f65633431303831353636383835393363: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":130:7
                if (v_6163746976617465.read().call_authored(vec![Value::string("context"), Value::string("press"), Value::string(""), Value::boolean(strict_equal(&((v_6576656e745f5f65633431303831353636383835393363.clone()).member("shiftKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f65633431303831353636383835393363.clone()).member("ctrlKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f65633431303831353636383835393363.clone()).member("metaKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f65633431303831353636383835393363.clone()).member("altKey", false)), &(Value::boolean(true))))])).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":131:57
                    { let v_7075695f6e61746976655f3833 = v_6576656e745f5f65633431303831353636383835393363.clone(); prevent(v_7075695f6e61746976655f3833.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":133:5
            { let v_7075695f6e61746976655f3834 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_726f6c65 = v_726f6c65.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e745f5f63323835363439303162656564666365: Value = arg(&arguments, 0);
                let v_6576656e745f5f32613662383336383662336564363738: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/action-control.proto.ts":134:7
                if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("F10")))); if left.truthy() { (v_6576656e745f5f32613662383336383662336564363738.clone()).member("shiftKey", false) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("ctrlKey", false)).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("metaKey", false)).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("altKey", false)).truthy()) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":135:9
                    if (v_6163746976617465.read().call_authored(vec![Value::string("context"), Value::string("keyboard"), { let left = (v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("shiftKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("ctrlKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("metaKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("altKey", false)), &(Value::boolean(true))))])).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":136:59
                        { let v_7075695f6e61746976655f3835 = v_6576656e745f5f32613662383336383662336564363738.clone(); prevent(v_7075695f6e61746976655f3835.clone()) };
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":137:9
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":139:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3836 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3836.clone()).get() }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("ctrlKey", false)).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("metaKey", false)).truthy()) } else { left } }; if left.truthy() { Value::boolean(!((v_6576656e745f5f32613662383336383662336564363738.clone()).member("altKey", false)).truthy()) } else { left } }; if left.truthy() { { let relational_left = { let left = ({ let v_7075695f6e61746976655f3837 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3837.clone()).owner(), false) }).member("count", false); if left.nullish() { Value::number(0.0) } else { left } }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":140:15
                    let v_636f756e74 = { let left = ({ let v_7075695f6e61746976655f3838 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3838.clone()).owner(), false) }).member("count", false); if left.nullish() { Value::number(0.0) } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":141:15
                    let v_706f736974696f6e = { let left = ({ let v_7075695f6e61746976655f3839 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3839.clone()).owner(), false) }).member("position", false); if left.nullish() { Value::number(0.0) } else { left } };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":142:13
                    let v_6e6578745f5f65353430383963323332353632353434 = Rc::new(std::cell::RefCell::new(v_706f736974696f6e.clone()));
                    // Source "frontend/proto-ui/src/action-control.proto.ts":143:9
                    if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ArrowUp")))) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":143:67
                        { let value = Value::number((Value::number((add(v_706f736974696f6e.clone(), v_636f756e74.clone())).scalar_number() - (Value::number(1.0)).scalar_number())).scalar_number() % (v_636f756e74.clone()).scalar_number()); *(v_6e6578745f5f65353430383963323332353632353434.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":144:14
                        if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ArrowRight")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ArrowDown")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":144:75
                            { let value = Value::number((add(v_706f736974696f6e.clone(), Value::number(1.0))).scalar_number() % (v_636f756e74.clone()).scalar_number()); *(v_6e6578745f5f65353430383963323332353632353434.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":145:14
                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("Home"))))).truthy() {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":145:40
                                { let value = Value::number(0.0); *(v_6e6578745f5f65353430383963323332353632353434.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/action-control.proto.ts":146:14
                                if (Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("End"))))).truthy() {
                                    // Source "frontend/proto-ui/src/action-control.proto.ts":146:39
                                    { let value = Value::number((v_636f756e74.clone()).scalar_number() - (Value::number(1.0)).scalar_number()); *(v_6e6578745f5f65353430383963323332353632353434.clone()).borrow_mut() = value; }
                                }
                            }
                        }
                    }
                    // Source "frontend/proto-ui/src/action-control.proto.ts":147:9
                    if (Value::boolean(!strict_equal(&({ let value = (v_6e6578745f5f65353430383963323332353632353434.clone()).borrow().clone(); value }), &(v_706f736974696f6e.clone())))).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":148:17
                        let v_6f776e6572 = { let value = (v_72756e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":149:17
                        let v_7469636b65745f5f37323665386135623062386139646462 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":150:17
                        let v_7265766973696f6e5f5f30633239613234313039643033653166 = { let left = ({ let v_7075695f6e61746976655f3930 = v_6f776e6572.clone(); read_props(&(v_7075695f6e61746976655f3930.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":151:11
                        { let v_7075695f6e61746976655f3931 = v_6576656e745f5f32613662383336383662336564363738.clone(); prevent(v_7075695f6e61746976655f3931.clone()) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":152:11
                        if ({ let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }), &(v_7469636b65745f5f37323665386135623062386139646462.clone()))); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3932 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3932.clone()).get() } } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/action-control.proto.ts":152:81
                            return Value::Undefined;
                        }
                        // Source "frontend/proto-ui/src/action-control.proto.ts":153:11
                        { let v_7075695f6e61746976655f3933 = v_6f776e6572.clone(); emit(&(v_7075695f6e61746976655f3933.clone()).owner(), Value::string("navigate"), record(vec![("index", { let value = (v_6e6578745f5f65353430383963323332353632353434.clone()).borrow().clone(); value }), ("revision", v_7265766973696f6e5f5f30633239613234313039643033653166.clone())])) };
                        // Source "frontend/proto-ui/src/action-control.proto.ts":154:11
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":157:7
                if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string(" ")))); if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3934 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3934.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":157:49
                    { let v_7075695f6e61746976655f3935 = v_6576656e745f5f32613662383336383662336564363738.clone(); prevent(v_7075695f6e61746976655f3935.clone()) };
                }
                // Source "frontend/proto-ui/src/action-control.proto.ts":158:7
                if ({ let left = { let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3936 = v_726f6c65.read(); (v_7075695f6e61746976655f3936.clone()).get() }), &(Value::string("combobox")))); if left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false)), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3937 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3937.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":159:15
                    let v_7469636b65745f5f34306635333563626366633935346234 = { let value = (v_67656e65726174696f6e.read()).borrow().clone(); value };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":160:9
                    { let v_7075695f6e61746976655f3938 = v_6576656e745f5f32613662383336383662336564363738.clone(); prevent(v_7075695f6e61746976655f3938.clone()) };
                    // Source "frontend/proto-ui/src/action-control.proto.ts":161:9
                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_7469636b65745f5f34306635333563626366633935346234.clone()), &({ let value = (v_67656e65726174696f6e.read()).borrow().clone(); value }))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3939 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3939.clone()).get() }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/action-control.proto.ts":162:11
                        v_6163746976617465.read().call_authored(vec![Value::string("activate"), Value::string("keyboard"), { let left = (v_6576656e745f5f32613662383336383662336564363738.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("shiftKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("ctrlKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("metaKey", false)), &(Value::boolean(true)))), Value::boolean(strict_equal(&((v_6576656e745f5f32613662383336383662336564363738.clone()).member("altKey", false)), &(Value::boolean(true))))]);
                    }
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":167:5
            { let v_7075695f6e61746976655f313030 = v_646566.clone(); expose_method(&owner, Value::string("requestActivate"), {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":167:48
                return v_6163746976617465.read().call_authored(vec![Value::string("activate"), Value::string("programmatic"), Value::string(""), Value::boolean(false), Value::boolean(false), Value::boolean(false), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":168:5
            { let v_7075695f6e61746976655f313031 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":168:44
                if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f313032 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313032.clone()).get() }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/action-control.proto.ts":168:77
                    { let v_7075695f6e61746976655f313033 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313033.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":169:5
            { let v_7075695f6e61746976655f313034 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); let v_636c656172 = v_636c656172.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":169:39
                { let cell = v_67656e65726174696f6e.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/action-control.proto.ts":169:53
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":169:71
                v_636c656172.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/action-control.proto.ts":169:80
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":170:5
            { let v_7075695f6e61746976655f313035 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_67656e65726174696f6e = v_67656e65726174696f6e.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/action-control.proto.ts":170:43
                { let cell = v_67656e65726174696f6e.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/action-control.proto.ts":170:57
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":170:74
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/action-control.proto.ts":170:92
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":173:5
            { let v_7075695f6e61746976655f313036 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("absolute".into(), "absolute".into()), ("inset-0".into(), "inset-0".into()), ("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/action-control.proto.ts":174:5
            return { let callable={ let pui_native_107={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/action-control.proto.ts":174:26
                return { let v_7075695f6e61746976655f313038 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_107.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_107.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ActionControlHandle { ActionControlHandle { owner: self.owner.clone() } }
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
