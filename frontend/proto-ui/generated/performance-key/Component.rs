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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("note").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("momentary").map_or(true, |prop| prop.is_null() || prop.is_boolean()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "note" => value.as_f64().is_some_and(f64::is_finite),
        "label" => value.is_string(),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "momentary" => value.is_boolean(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "keyDown" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "keyUp" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "note", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<f64>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<bool>,
    #[serde(rename = "momentary", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct PerformanceKeyHandle { pub owner: OwnerRef }
  impl PerformanceKeyHandle {
      pub fn state_0(&self) -> Result<bool, Value> { self.state("pressed").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_5(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyDown").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyDown", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_6(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyUp").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyUp", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
  pub struct PerformanceKey { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl PerformanceKey {
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
            let v_70726573736564: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6e6f7465: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_6d6f6d656e74617279: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646972656374: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_747970696e67: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_6c617463686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_7075626c697368: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(f64) -> bool>> = lexical_binding();
            let v_63616e63656c: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn() -> bool>> = lexical_binding();
            let v_6b65794e6f7465: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value) -> f64>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(bool) -> Value>> = lexical_binding();
            // Source "frontend/proto-ui/src/performance-key.proto.ts":10:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("note", record(vec![("type", Value::string("number")), ("default", Value::number(60.0_f64))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("momentary", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":15:11
            v_70726573736564.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "bool", Value::string("pressed"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":16:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":17:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":18:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":19:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":20:5
            native_builtin_hook(&owner, "asTrigger", || as_trigger(&owner));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":21:5
            { let v_7075695f6e61746976655f34 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f34.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":22:5
            { let v_7075695f6e61746976655f35 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f35.clone()).owner(), Value::string("button")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":23:5
            { let v_7075695f6e61746976655f36 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f36.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":24:5
            { let v_7075695f6e61746976655f37 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f37.clone()).owner(), Value::string("pressed"), v_70726573736564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":25:5
            { let v_7075695f6e61746976655f38 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f38.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":26:5
            { let v_7075695f6e61746976655f39 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f39.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-note-toggle"))])) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":27:5
            { let v_7075695f6e61746976655f3130 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":28:5
            { let v_7075695f6e61746976655f3131 = v_646566.clone(); expose_state(&owner, Value::string("pressed"), v_70726573736564.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":29:5
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":30:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":31:5
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_event(&owner, Value::string("pressedChange")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":32:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_event(&owner, Value::string("trigger")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":33:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":34:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":35:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":36:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":37:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":38:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":39:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":40:9
            v_6e6f7465.initialize(Rc::new(std::cell::RefCell::new(60.0_f64)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":41:9
            v_6d6f6d656e74617279.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":42:9
            v_646972656374.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":43:9
            v_747970696e67.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":44:9
            v_6c617463686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":47:11
            v_7075626c697368.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let call:Rc<dyn Fn(f64)->bool>=Rc::new(move |argument_0: f64| -> bool {
                let v_7469636b65745f5f36363362323361383031366238646236: f64 = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":48:13
                let v_68616e646c655f5f63306435313436306134323836613634 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":49:7
                if { let pui_native_16=!((v_68616e646c655f5f63306435313436306134323836613634.clone()).truthy()); if !(pui_native_16) {!({ let pui_native_18={ let pui_native_20=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_20) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_20} }; if (pui_native_18) {{ let pui_native_24={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_25=v_7469636b65745f5f36363362323361383031366238646236.clone(); (pui_native_24==pui_native_25) }} else {pui_native_18} })} else {pui_native_16} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":49:40
                    return false;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":50:13
                let v_6e6578745f5f63643462306137376239333430333437: bool = { let pui_native_26={ let pui_native_28={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_28) {!(({ let v_7075695f6e61746976655f3330 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f3330.clone()).get()).bool()) }).bool())} else {pui_native_28} }; if (pui_native_26) {{ let pui_native_31={ let pui_native_33={ let value=(v_646972656374.read()).borrow().clone(); value }; if !(pui_native_33) {{ let value=(v_747970696e67.read()).borrow().clone(); value }} else {pui_native_33} }; if !(pui_native_31) {{ let value=(v_6c617463686564.read()).borrow().clone(); value }} else {pui_native_31} }} else {pui_native_26} };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":51:7
                if { let pui_native_35=v_6e6578745f5f63643462306137376239333430333437.clone(); let pui_native_36=({ let v_7075695f6e61746976655f3337 = v_70726573736564.read(); Value::boolean(((v_7075695f6e61746976655f3337.clone()).get()).bool()) }).bool(); (pui_native_35==pui_native_36) } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":51:35
                    return false;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":52:13
                let v_62617369735f5f31356237306138386330646263646261 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":53:7
                { let v_7075695f6e61746976655f3338 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3338.clone(), Value::boolean(v_6e6578745f5f63643462306137376239333430333437.clone()), Value::string("reason: performance key contacts")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":54:7
                if !({ let pui_native_39={ let pui_native_41=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_41) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_41} }; if (pui_native_39) {{ let pui_native_45={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_46=v_7469636b65745f5f36363362323361383031366238646236.clone(); (pui_native_45==pui_native_46) }} else {pui_native_39} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":54:29
                    return false;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":55:7
                { let v_7075695f6e61746976655f3437 = v_68616e646c655f5f63306435313436306134323836613634.clone(); emit(&(v_7075695f6e61746976655f3437.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", Value::boolean(v_6e6578745f5f63643462306137376239333430333437.clone())), ("revision", v_62617369735f5f31356237306138386330646263646261.clone())])) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":56:7
                if { let pui_native_48={ let pui_native_50={ let pui_native_52=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_52) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_52} }; if (pui_native_50) {{ let pui_native_56={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_57=v_7469636b65745f5f36363362323361383031366238646236.clone(); (pui_native_56==pui_native_57) }} else {pui_native_50} }; if (pui_native_48) {{ let value=(v_6174746163686564.read()).borrow().clone(); value }} else {pui_native_48} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":56:40
                    { let v_7075695f6e61746976655f3538 = v_68616e646c655f5f63306435313436306134323836613634.clone(); request_update(&(v_7075695f6e61746976655f3538.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":57:7
                return { let pui_native_59={ let pui_native_61=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_61) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_61} }; if (pui_native_59) {{ let pui_native_65={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_66=v_7469636b65745f5f36363362323361383031366238646236.clone(); (pui_native_65==pui_native_66) }} else {pui_native_59} };
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":59:11
            v_63616e63656c.initialize({  let v_65706f6368 = v_65706f6368.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let call:Rc<dyn Fn()->bool>=Rc::new(move || -> bool {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":60:13
                let v_7469636b65745f5f64386631316666346530383134633439: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":61:7
                Value::boolean({ let value=false; *(v_646972656374.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":62:7
                Value::boolean({ let value=false; *(v_747970696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":63:7
                Value::boolean({ let value=false; *(v_6c617463686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":64:7
                return (v_7075626c697368.read().call)(v_7469636b65745f5f64386631316666346530383134633439.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":66:11
            v_6b65794e6f7465.initialize({   let call:Rc<dyn Fn(Value)->f64>=Rc::new(move |argument_0: Value| -> f64 {
                let v_6b65795f5f31333033353530396366366338333030: Value = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":67:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("a"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":67:24
                    return 60.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":68:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("w"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":68:24
                    return 61.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":69:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("s"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":69:24
                    return 62.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":70:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("e"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":70:24
                    return 63.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":71:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("d"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":71:24
                    return 64.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":72:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("f"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":72:24
                    return 65.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":73:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("t"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":73:24
                    return 66.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":74:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("g"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":74:24
                    return 67.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":75:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("y"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":75:24
                    return 68.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":76:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("h"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":76:24
                    return 69.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":77:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("u"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":77:24
                    return 70.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":78:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("j"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":78:24
                    return 71.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":79:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("k"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":79:24
                    return 72.0_f64;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":80:7
                return -(1.0_f64);
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::number(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":82:5
            { let v_7075695f6e61746976655f3933 = v_646566.clone(); expose_method(&owner, Value::string("keyDown"), ({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_6b65794e6f7465 = v_6b65794e6f7465.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6b65795f5f62613233303533333230636632616261: Value = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":83:7
                if { let pui_native_94={ let pui_native_96={ let pui_native_98={ let pui_native_100={ let pui_native_102=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_102) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_102} }; if !(pui_native_100) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_100} }; if !(pui_native_98) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_98} }; if !(pui_native_96) {({ let v_7075695f6e61746976655f313034 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313034.clone()).get()).bool()) }).bool()} else {pui_native_96} }; if !(pui_native_94) {{ let pui_native_105=(v_6b65794e6f7465.read().call)(v_6b65795f5f62613233303533333230636632616261.clone()); let pui_native_106={ let value=(v_6e6f7465.read()).borrow().clone(); value }; !(pui_native_105==pui_native_106) }} else {pui_native_94} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":83:96
                    return false;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":84:13
                let v_7265706561746564: bool = { let value=(v_747970696e67.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":85:7
                Value::boolean({ let value=true; *(v_747970696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":86:13
                let v_7469636b65745f5f62666339643562653436666339646339: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":87:7
                Value::boolean((v_7075626c697368.read().call)(v_7469636b65745f5f62666339643562653436666339646339.clone()));
                // Source "frontend/proto-ui/src/performance-key.proto.ts":88:7
                if ({ let left = Value::boolean({ let pui_native_107={ let pui_native_109={ let value=(v_6d6f6d656e74617279.read()).borrow().clone(); value }; if (pui_native_109) {!(v_7265706561746564.clone())} else {pui_native_109} }; if (pui_native_107) {{ let pui_native_111={ let pui_native_113=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_113) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_113} }; if (pui_native_111) {{ let pui_native_117={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_118=v_7469636b65745f5f62666339643562653436666339646339.clone(); (pui_native_117==pui_native_118) }} else {pui_native_111} }} else {pui_native_107} }); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":88:61
                    { let v_7075695f6e61746976655f313139 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313139.clone()).owner(), Value::string("trigger"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":89:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":91:5
            { let v_7075695f6e61746976655f313230 = v_646566.clone(); expose_method(&owner, Value::string("keyUp"), ({  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_6b65794e6f7465 = v_6b65794e6f7465.clone(); let call:Rc<dyn Fn(Value)->bool>=Rc::new(move |argument_0: Value| -> bool {
                let v_6b65795f5f61373336393063656332346663383037: Value = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":92:7
                if { let pui_native_121={ let pui_native_123={ let pui_native_125={ let pui_native_127=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_127) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_127} }; if !(pui_native_125) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_125} }; if !(pui_native_123) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_123} }; if !(pui_native_121) {{ let pui_native_129=(v_6b65794e6f7465.read().call)(v_6b65795f5f61373336393063656332346663383037.clone()); let pui_native_130={ let value=(v_6e6f7465.read()).borrow().clone(); value }; !(pui_native_129==pui_native_130) }} else {pui_native_121} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":92:78
                    return false;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":93:7
                Value::boolean({ let value=false; *(v_747970696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":94:7
                Value::boolean((v_7075626c697368.read().call)({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value }));
                // Source "frontend/proto-ui/src/performance-key.proto.ts":95:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":97:5
            { let v_7075695f6e61746976655f313331 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":98:5
            { let v_7075695f6e61746976655f313332 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_646972656374 = v_646972656374.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":99:7
                if { let pui_native_133={ let pui_native_135={ let pui_native_137={ let pui_native_139=!(({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !(pui_native_139) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_139} }; if !(pui_native_137) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_137} }; if !(pui_native_135) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_135} }; if !(pui_native_133) {({ let v_7075695f6e61746976655f313431 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313431.clone()).get()).bool()) }).bool()} else {pui_native_133} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":99:71
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":100:7
                Value::boolean({ let value=true; *(v_646972656374.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":101:13
                let v_7469636b65745f5f36613738373434376130383033653132: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":102:7
                Value::boolean((v_7075626c697368.read().call)(v_7469636b65745f5f36613738373434376130383033653132.clone()));
                // Source "frontend/proto-ui/src/performance-key.proto.ts":103:7
                if ({ let left = Value::boolean({ let pui_native_142={ let value=(v_6d6f6d656e74617279.read()).borrow().clone(); value }; if (pui_native_142) {{ let pui_native_144={ let pui_native_146=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_146) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_146} }; if (pui_native_144) {{ let pui_native_150={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_151=v_7469636b65745f5f36613738373434376130383033653132.clone(); (pui_native_150==pui_native_151) }} else {pui_native_144} }} else {pui_native_142} }); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":103:48
                    { let v_7075695f6e61746976655f313532 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f313532.clone()).owner(), Value::string("trigger"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":105:11
            let v_72656c65617365446972656374 = {  let v_65706f6368 = v_65706f6368.clone(); let v_646972656374 = v_646972656374.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":105:35
                Value::boolean({ let value=false; *(v_646972656374.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":105:51
                Value::boolean((v_7075626c697368.read().call)({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value }));
                Value::Undefined
            })) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":106:5
            { let v_7075695f6e61746976655f313533 = v_646566.clone(); listen(&owner, Value::string("press.end"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":107:5
            { let v_7075695f6e61746976655f313534 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":108:5
            { let v_7075695f6e61746976655f313535 = v_646566.clone(); listen(&owner, Value::string("pointer.cancel"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":109:5
            { let v_7075695f6e61746976655f313536 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, (v_63616e63656c.read()).ordinary.clone().expect("checked native data callable"), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":110:5
            { let v_7075695f6e61746976655f313537 = v_646566.clone(); listen(&owner, Value::string("host:verdant-note-toggle"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":111:13
                let v_68616e646c655f5f39636164383032306661616238393939 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":112:7
                if { let pui_native_158={ let pui_native_160={ let pui_native_162={ let pui_native_164=!((v_68616e646c655f5f39636164383032306661616238393939.clone()).truthy()); if !(pui_native_164) {{ let value=(v_73796e63696e67.read()).borrow().clone(); value }} else {pui_native_164} }; if !(pui_native_162) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_162} }; if !(pui_native_160) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_160} }; if !(pui_native_158) {({ let v_7075695f6e61746976655f313636 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f313636.clone()).get()).bool()) }).bool()} else {pui_native_158} } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":112:74
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":113:7
                if { let value=(v_6d6f6d656e74617279.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":113:24
                    { let v_7075695f6e61746976655f313637 = v_68616e646c655f5f39636164383032306661616238393939.clone(); emit(&(v_7075695f6e61746976655f313637.clone()).owner(), Value::string("trigger"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":113:69
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":114:7
                Value::boolean({ let value=!({ let value=(v_6c617463686564.read()).borrow().clone(); value }); *(v_6c617463686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":115:7
                Value::boolean((v_7075626c697368.read().call)({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value }));
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":117:5
            { let v_7075695f6e61746976655f313638 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":118:7
                if (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape"))))).bool() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":119:9
                    { let v_7075695f6e61746976655f313731 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f313731.clone()) };
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":120:9
                    Value::boolean((v_63616e63656c.read().call)());
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":123:11
            v_73796e63.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f33323464323035333565366331653736: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":124:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":124:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":125:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":126:13
                let v_7469636b65745f5f36653966326136363536313830626139: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":127:7
                Value::boolean({ let value=true; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":128:7
                { let value = v_68616e646c655f5f33323464323035333565366331653736.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":129:13
                let v_70726f7073 = { let v_7075695f6e61746976655f313732 = v_68616e646c655f5f33323464323035333565366331653736.clone(); read_props(&(v_7075695f6e61746976655f313732.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":130:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":131:13
                let v_6e6578744e6f7465: f64 = ((v_70726f7073.clone()).member("note", false)).num();
                // Source "frontend/proto-ui/src/performance-key.proto.ts":132:13
                let v_72656c65617365: bool = { let pui_native_175={ let pui_native_177={ let pui_native_179=(Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value })))).bool(); if !(pui_native_179) {{ let pui_native_183=v_6e6578744e6f7465.clone(); let pui_native_184={ let value=(v_6e6f7465.read()).borrow().clone(); value }; !(pui_native_183==pui_native_184) }} else {pui_native_179} }; if !(pui_native_177) {{ let pui_native_185=((v_70726f7073.clone()).member("disabled", false)).bool(); let pui_native_186=true; (pui_native_185==pui_native_186) }} else {pui_native_177} }; if !(pui_native_175) {{ let pui_native_187=((v_70726f7073.clone()).member("momentary", false)).bool(); let pui_native_188={ let value=(v_6d6f6d656e74617279.read()).borrow().clone(); value }; !(pui_native_187==pui_native_188) }} else {pui_native_175} };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":133:13
                let v_776173507265737365645f5f38336138313035376435343237303764: bool = ({ let v_7075695f6e61746976655f313931 = v_70726573736564.read(); Value::boolean(((v_7075695f6e61746976655f313931.clone()).get()).bool()) }).bool();
                // Source "frontend/proto-ui/src/performance-key.proto.ts":134:13
                let v_62617369735f5f63376530373836313864613330313533 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":135:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":136:7
                Value::number({ let value=v_6e6578744e6f7465.clone(); *(v_6e6f7465.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":137:7
                Value::boolean({ let value=((v_70726f7073.clone()).member("momentary", false)).bool(); *(v_6d6f6d656e74617279.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":138:7
                if v_72656c65617365.clone() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":138:22
                    Value::boolean({ let value=false; *(v_646972656374.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":138:38
                    Value::boolean({ let value=false; *(v_747970696e67.read()).borrow_mut()=value.clone(); value });
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":138:54
                    Value::boolean({ let value=false; *(v_6c617463686564.read()).borrow_mut()=value.clone(); value });
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":139:7
                { let v_7075695f6e61746976655f313934 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313934.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: performance key name")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":140:7
                if !({ let pui_native_195={ let pui_native_197=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_197) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_197} }; if (pui_native_195) {{ let pui_native_201={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_202=v_7469636b65745f5f36653966326136363536313830626139.clone(); (pui_native_201==pui_native_202) }} else {pui_native_195} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":140:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":141:7
                { let v_7075695f6e61746976655f323033 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f323033.clone(), Value::boolean(((v_70726f7073.clone()).member("disabled", false)).bool()), Value::string("reason: performance key policy")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":142:7
                if !({ let pui_native_206={ let pui_native_208=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_208) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_208} }; if (pui_native_206) {{ let pui_native_212={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_213=v_7469636b65745f5f36653966326136363536313830626139.clone(); (pui_native_212==pui_native_213) }} else {pui_native_206} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":142:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":143:7
                { let v_7075695f6e61746976655f323134 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f323134.clone()).owner(), Value::boolean(({ let v_7075695f6e61746976655f323135 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323135.clone()).get()).bool()) }).bool())) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":144:7
                if !({ let pui_native_216={ let pui_native_218=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_218) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_218} }; if (pui_native_216) {{ let pui_native_222={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_223=v_7469636b65745f5f36653966326136363536313830626139.clone(); (pui_native_222==pui_native_223) }} else {pui_native_216} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":144:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":145:13
                let v_6e6578745f5f62376631643232616239383438626338: bool = { let pui_native_224={ let pui_native_226={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_226) {!(({ let v_7075695f6e61746976655f323238 = v_64697361626c6564.read(); Value::boolean(((v_7075695f6e61746976655f323238.clone()).get()).bool()) }).bool())} else {pui_native_226} }; if (pui_native_224) {{ let pui_native_229={ let pui_native_231={ let value=(v_646972656374.read()).borrow().clone(); value }; if !(pui_native_231) {{ let value=(v_747970696e67.read()).borrow().clone(); value }} else {pui_native_231} }; if !(pui_native_229) {{ let value=(v_6c617463686564.read()).borrow().clone(); value }} else {pui_native_229} }} else {pui_native_224} };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":146:7
                { let v_7075695f6e61746976655f323333 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f323333.clone(), Value::boolean(v_6e6578745f5f62376631643232616239383438626338.clone()), Value::string("reason: performance key authority")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":147:7
                if !({ let pui_native_234={ let pui_native_236=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_236) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_236} }; if (pui_native_234) {{ let pui_native_240={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_241=v_7469636b65745f5f36653966326136363536313830626139.clone(); (pui_native_240==pui_native_241) }} else {pui_native_234} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":147:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":148:7
                if { let pui_native_242=v_776173507265737365645f5f38336138313035376435343237303764.clone(); let pui_native_243=v_6e6578745f5f62376631643232616239383438626338.clone(); !(pui_native_242==pui_native_243) } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":148:32
                    { let v_7075695f6e61746976655f323434 = v_68616e646c655f5f33323464323035333565366331653736.clone(); emit(&(v_7075695f6e61746976655f323434.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", Value::boolean(v_6e6578745f5f62376631643232616239383438626338.clone())), ("revision", v_62617369735f5f63376530373836313864613330313533.clone())])) };
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":149:7
                if !({ let pui_native_245={ let pui_native_247=(Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null))))).bool(); if (pui_native_247) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_247} }; if (pui_native_245) {{ let pui_native_251={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_252=v_7469636b65745f5f36653966326136363536313830626139.clone(); (pui_native_251==pui_native_252) }} else {pui_native_245} }) {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":149:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":150:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":151:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":151:21
                    { let v_7075695f6e61746976655f323533 = v_68616e646c655f5f33323464323035333565366331653736.clone(); request_update(&(v_7075695f6e61746976655f323533.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":153:11
            v_726574697265.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let call:Rc<dyn Fn(bool)->Value>=Rc::new(move |argument_0: bool| -> Value {
                let v_7065726d616e656e74: bool = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":154:13
                let v_68616e646c655f5f61393430623239666131366436313234 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":155:13
                let v_776173507265737365645f5f34383036303562646661373332663738: bool = ({ let v_7075695f6e61746976655f323534 = v_70726573736564.read(); Value::boolean(((v_7075695f6e61746976655f323534.clone()).get()).bool()) }).bool();
                // Source "frontend/proto-ui/src/performance-key.proto.ts":156:13
                let v_62617369735f5f61356432616531333066663139303835 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":157:7
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":158:13
                let v_7469636b65745f5f30636332666435383430346139333464: f64 = { let cell=v_617574686f7269747945706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":159:7
                Value::boolean({ let value=false; *(v_73796e63696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":160:7
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":161:7
                Value::boolean({ let value={ let pui_native_255={ let value=(v_646973706f736564.read()).borrow().clone(); value }; if !(pui_native_255) {v_7065726d616e656e74.clone()} else {pui_native_255} }; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":162:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":163:7
                Value::boolean({ let value=false; *(v_646972656374.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":164:7
                Value::boolean({ let value=false; *(v_747970696e67.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":165:7
                Value::boolean({ let value=false; *(v_6c617463686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":166:7
                if v_7065726d616e656e74.clone() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":166:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":167:7
                { let v_7075695f6e61746976655f323537 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f323537.clone(), Value::boolean(false), Value::string("reason: performance key detached")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":168:7
                if { let pui_native_258={ let value=(v_617574686f7269747945706f6368.read()).borrow().clone(); value }; let pui_native_259=v_7469636b65745f5f30636332666435383430346139333464.clone(); !(pui_native_258==pui_native_259) } {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":168:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":169:7
                if ({ let left = Value::boolean(v_776173507265737365645f5f34383036303562646661373332663738.clone()); if left.truthy() { v_68616e646c655f5f61393430623239666131366436313234.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":169:33
                    { let v_7075695f6e61746976655f323630 = v_68616e646c655f5f61393430623239666131366436313234.clone(); emit(&(v_7075695f6e61746976655f323630.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", Value::boolean(false)), ("revision", v_62617369735f5f61356432616531333066663139303835.clone())])) };
                }
                Value::Undefined
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {ordinary_call((arg(&arguments,0)).bool())}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":171:5
            { let v_7075695f6e61746976655f323631 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":172:5
            { let v_7075695f6e61746976655f323632 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f63373431386366396535363233653364: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":172:43
                Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/performance-key.proto.ts":172:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f63373431386366396535363233653364.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":173:5
            { let v_7075695f6e61746976655f323633 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("note"), Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("momentary")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":174:5
            { let v_7075695f6e61746976655f323634 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":174:37
                return (v_726574697265.read().call)(false);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":175:5
            { let v_7075695f6e61746976655f323635 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":175:41
                return (v_726574697265.read().call)(true);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":176:5
            return { let callable={ let pui_native_266={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":176:26
                return { let v_7075695f6e61746976655f323637 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_266.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_266.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> PerformanceKeyHandle { PerformanceKeyHandle { owner: self.owner.clone() } }
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
  impl Drop for PerformanceKey { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for PerformanceKey { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for PerformanceKey {
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
