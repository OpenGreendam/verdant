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
        catch_authored(|| {
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
        })
    }
    pub fn method_6(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
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
        })
    }
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
  pub struct PerformanceKey { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl PerformanceKey {
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
            let v_70726573736564: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6e6f7465: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6d6f6d656e74617279: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646972656374: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_747970696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6c617463686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_7075626c697368: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_6b65794e6f7465: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/performance-key.proto.ts":10:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("note", record(vec![("type", Value::string("number")), ("default", Value::number(60.0))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("momentary", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
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
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":35:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":36:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":37:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":38:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":39:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":40:9
            v_6e6f7465.initialize(Rc::new(std::cell::RefCell::new(Value::number(60.0))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":41:9
            v_6d6f6d656e74617279.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":42:9
            v_646972656374.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":43:9
            v_747970696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":44:9
            v_6c617463686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/performance-key.proto.ts":45:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f61656265653532386161386631653432: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":45:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f61656265653532386161386631653432.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":46:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f30396132343164313532356439633262: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":46:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30396132343164313532356439633262.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":47:11
            v_7075626c697368.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f36363362323361383031366238646236: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":48:13
                let v_68616e646c655f5f63306435313436306134323836613634 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":49:7
                if ({ let left = Value::boolean(!(v_68616e646c655f5f63306435313436306134323836613634.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36363362323361383031366238646236.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":49:40
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":50:13
                let v_6e6578745f5f63643462306137376239333430333437 = { let left = { let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3136 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3136.clone()).get() }).truthy()) } else { left } }; if left.truthy() { { let left = { let left = { let value = (v_646972656374.read()).borrow().clone(); value }; if !left.truthy() { { let value = (v_747970696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let value = (v_6c617463686564.read()).borrow().clone(); value } } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":51:7
                if (Value::boolean(strict_equal(&(v_6e6578745f5f63643462306137376239333430333437.clone()), &({ let v_7075695f6e61746976655f3137 = v_70726573736564.read(); (v_7075695f6e61746976655f3137.clone()).get() })))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":51:35
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":52:13
                let v_62617369735f5f31356237306138386330646263646261 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":53:7
                { let v_7075695f6e61746976655f3138 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3138.clone(), v_6e6578745f5f63643462306137376239333430333437.clone(), Value::string("reason: performance key contacts")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":54:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36363362323361383031366238646236.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":54:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":55:7
                { let v_7075695f6e61746976655f3139 = v_68616e646c655f5f63306435313436306134323836613634.clone(); emit(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", v_6e6578745f5f63643462306137376239333430333437.clone()), ("revision", v_62617369735f5f31356237306138386330646263646261.clone())])) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":56:7
                if ({ let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36363362323361383031366238646236.clone()]); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":56:40
                    { let v_7075695f6e61746976655f3230 = v_68616e646c655f5f63306435313436306134323836613634.clone(); request_update(&(v_7075695f6e61746976655f3230.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":57:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36363362323361383031366238646236.clone()]);
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":59:11
            v_63616e63656c.initialize({  let v_65706f6368 = v_65706f6368.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":60:13
                let v_7469636b65745f5f64386631316666346530383134633439 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":61:7
                { let value = Value::boolean(false); *(v_646972656374.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":62:7
                { let value = Value::boolean(false); *(v_747970696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":63:7
                { let value = Value::boolean(false); *(v_6c617463686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":64:7
                return v_7075626c697368.read().call_authored(vec![v_7469636b65745f5f64386631316666346530383134633439.clone()]);
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":66:11
            v_6b65794e6f7465.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b65795f5f31333033353530396366366338333030: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":67:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("a"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":67:24
                    return Value::number(60.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":68:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("w"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":68:24
                    return Value::number(61.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":69:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("s"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":69:24
                    return Value::number(62.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":70:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("e"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":70:24
                    return Value::number(63.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":71:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("d"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":71:24
                    return Value::number(64.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":72:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("f"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":72:24
                    return Value::number(65.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":73:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("t"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":73:24
                    return Value::number(66.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":74:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("g"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":74:24
                    return Value::number(67.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":75:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("y"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":75:24
                    return Value::number(68.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":76:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("h"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":76:24
                    return Value::number(69.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":77:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("u"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":77:24
                    return Value::number(70.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":78:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("j"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":78:24
                    return Value::number(71.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":79:7
                if (Value::boolean(strict_equal(&(v_6b65795f5f31333033353530396366366338333030.clone()), &(Value::string("k"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":79:24
                    return Value::number(72.0);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":80:7
                return Value::number(-(Value::number(1.0)).scalar_number());
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":82:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_method(&owner, Value::string("keyDown"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_6b65794e6f7465 = v_6b65794e6f7465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b65795f5f62613233303533333230636632616261: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":83:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3232 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3232.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6b65794e6f7465.read().call_authored(vec![v_6b65795f5f62613233303533333230636632616261.clone()])), &({ let value = (v_6e6f7465.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":83:96
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":84:7
                { let value = Value::boolean(true); *(v_747970696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":85:7
                v_7075626c697368.read().call_authored(vec![{ let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":86:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":88:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_method(&owner, Value::string("keyUp"), {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_6b65794e6f7465 = v_6b65794e6f7465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b65795f5f61666335643130323631636438623161: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":89:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6b65794e6f7465.read().call_authored(vec![v_6b65795f5f61666335643130323631636438623161.clone()])), &({ let value = (v_6e6f7465.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":89:78
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":90:7
                { let value = Value::boolean(false); *(v_747970696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":91:7
                v_7075626c697368.read().call_authored(vec![{ let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":92:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":94:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":95:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_646972656374 = v_646972656374.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":96:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3236 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3236.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":96:71
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":97:7
                { let value = Value::boolean(true); *(v_646972656374.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":98:7
                v_7075626c697368.read().call_authored(vec![{ let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":100:11
            let v_72656c65617365446972656374 = {  let v_65706f6368 = v_65706f6368.clone(); let v_646972656374 = v_646972656374.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":100:35
                { let value = Value::boolean(false); *(v_646972656374.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":100:51
                v_7075626c697368.read().call_authored(vec![{ let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
                Value::Undefined
            })) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":101:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); listen(&owner, Value::string("press.end"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":102:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":103:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); listen(&owner, Value::string("pointer.cancel"), false, v_72656c65617365446972656374.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":104:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":105:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); listen(&owner, Value::string("host:verdant-note-toggle"), false, {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":106:13
                let v_68616e646c655f5f30623936326134363639393434666562 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":107:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f30623936326134363639393434666562.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3332 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3332.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":107:74
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":108:7
                if ({ let value = (v_6d6f6d656e74617279.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":108:24
                    { let v_7075695f6e61746976655f3333 = v_68616e646c655f5f30623936326134363639393434666562.clone(); emit(&(v_7075695f6e61746976655f3333.clone()).owner(), Value::string("trigger"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":108:69
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":109:7
                { let value = Value::boolean(!({ let value = (v_6c617463686564.read()).borrow().clone(); value }).truthy()); *(v_6c617463686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":110:7
                v_7075626c697368.read().call_authored(vec![{ let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":112:5
            { let v_7075695f6e61746976655f3334 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":113:7
                if (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape"))))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":114:9
                    { let v_7075695f6e61746976655f3335 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3335.clone()) };
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":115:9
                    v_63616e63656c.read().call_authored(vec![]);
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":118:11
            v_73796e63.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6e6f7465 = v_6e6f7465.clone(); let v_6d6f6d656e74617279 = v_6d6f6d656e74617279.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f33323531313266386434666564316237: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":119:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":119:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":120:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":121:13
                let v_7469636b65745f5f37623630646265383932663334343861 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":122:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":123:7
                { let value = v_68616e646c655f5f33323531313266386434666564316237.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":124:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3336 = v_68616e646c655f5f33323531313266386434666564316237.clone(); read_props(&(v_7075695f6e61746976655f3336.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":125:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":126:13
                let v_6e6578744e6f7465 = { let left = (v_70726f7073.clone()).member("note", false); if left.nullish() { Value::number(60.0) } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":127:13
                let v_72656c65617365 = { let left = { let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_6e6578744e6f7465.clone()), &({ let value = (v_6e6f7465.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":128:13
                let v_776173507265737365645f5f65363166356164623037306135663432 = { let v_7075695f6e61746976655f3337 = v_70726573736564.read(); (v_7075695f6e61746976655f3337.clone()).get() };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":129:13
                let v_62617369735f5f63623934333639363239643438663062 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":130:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":131:7
                { let value = v_6e6578744e6f7465.clone(); *(v_6e6f7465.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":132:7
                { let value = { let left = (v_70726f7073.clone()).member("momentary", false); if left.nullish() { Value::boolean(false) } else { left } }; *(v_6d6f6d656e74617279.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":133:7
                if (v_72656c65617365.clone()).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":133:22
                    { let value = Value::boolean(false); *(v_646972656374.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":133:38
                    { let value = Value::boolean(false); *(v_747970696e67.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":133:54
                    { let value = Value::boolean(false); *(v_6c617463686564.read()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":134:7
                { let v_7075695f6e61746976655f3338 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3338.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: performance key name")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":135:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f37623630646265383932663334343861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":135:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":136:7
                { let v_7075695f6e61746976655f3339 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3339.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: performance key policy")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":137:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f37623630646265383932663334343861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":137:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":138:7
                { let v_7075695f6e61746976655f3430 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3430.clone()).owner(), { let v_7075695f6e61746976655f3431 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3431.clone()).get() }) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":139:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f37623630646265383932663334343861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":139:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":140:13
                let v_6e6578745f5f35346133643435633863653933333132 = { let left = { let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3432 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3432.clone()).get() }).truthy()) } else { left } }; if left.truthy() { { let left = { let left = { let value = (v_646972656374.read()).borrow().clone(); value }; if !left.truthy() { { let value = (v_747970696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let value = (v_6c617463686564.read()).borrow().clone(); value } } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":141:7
                { let v_7075695f6e61746976655f3433 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3433.clone(), v_6e6578745f5f35346133643435633863653933333132.clone(), Value::string("reason: performance key authority")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":142:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f37623630646265383932663334343861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":142:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":143:7
                if (Value::boolean(!strict_equal(&(v_776173507265737365645f5f65363166356164623037306135663432.clone()), &(v_6e6578745f5f35346133643435633863653933333132.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":143:32
                    { let v_7075695f6e61746976655f3434 = v_68616e646c655f5f33323531313266386434666564316237.clone(); emit(&(v_7075695f6e61746976655f3434.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", v_6e6578745f5f35346133643435633863653933333132.clone()), ("revision", v_62617369735f5f63623934333639363239643438663062.clone())])) };
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":144:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f37623630646265383932663334343861.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":144:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":145:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":146:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":146:21
                    { let v_7075695f6e61746976655f3435 = v_68616e646c655f5f33323531313266386434666564316237.clone(); request_update(&(v_7075695f6e61746976655f3435.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":148:11
            v_726574697265.initialize({  let v_70726573736564 = v_70726573736564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_646972656374 = v_646972656374.clone(); let v_747970696e67 = v_747970696e67.clone(); let v_6c617463686564 = v_6c617463686564.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":149:13
                let v_68616e646c655f5f31643532313166363564323637666262 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":150:13
                let v_776173507265737365645f5f66356563626465363339343165356266 = { let v_7075695f6e61746976655f3436 = v_70726573736564.read(); (v_7075695f6e61746976655f3436.clone()).get() };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":151:13
                let v_62617369735f5f63616234356566323264343830353832 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":152:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":153:13
                let v_7469636b65745f5f33396665633965343065333031663963 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":154:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":155:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":156:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":157:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":158:7
                { let value = Value::boolean(false); *(v_646972656374.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":159:7
                { let value = Value::boolean(false); *(v_747970696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":160:7
                { let value = Value::boolean(false); *(v_6c617463686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":161:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":161:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":162:7
                { let v_7075695f6e61746976655f3437 = v_70726573736564.read(); state_set(v_7075695f6e61746976655f3437.clone(), Value::boolean(false), Value::string("reason: performance key detached")) };
                // Source "frontend/proto-ui/src/performance-key.proto.ts":163:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f33396665633965343065333031663963.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":163:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":164:7
                if ({ let left = v_776173507265737365645f5f66356563626465363339343165356266.clone(); if left.truthy() { v_68616e646c655f5f31643532313166363564323637666262.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/performance-key.proto.ts":164:33
                    { let v_7075695f6e61746976655f3438 = v_68616e646c655f5f31643532313166363564323637666262.clone(); emit(&(v_7075695f6e61746976655f3438.clone()).owner(), Value::string("pressedChange"), record(vec![("pressed", Value::boolean(false)), ("revision", v_62617369735f5f63616234356566323264343830353832.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/performance-key.proto.ts":166:5
            { let v_7075695f6e61746976655f3439 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":167:5
            { let v_7075695f6e61746976655f3530 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f34656661326665653934393836626137: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/performance-key.proto.ts":167:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/performance-key.proto.ts":167:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f34656661326665653934393836626137.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":168:5
            { let v_7075695f6e61746976655f3531 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("note"), Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("momentary")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":169:5
            { let v_7075695f6e61746976655f3532 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":169:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":170:5
            { let v_7075695f6e61746976655f3533 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/performance-key.proto.ts":170:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/performance-key.proto.ts":171:5
            return { let callable={ let pui_native_54={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/performance-key.proto.ts":171:26
                return { let v_7075695f6e61746976655f3535 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_54.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_54.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> PerformanceKeyHandle { PerformanceKeyHandle { owner: self.owner.clone() } }
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
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
