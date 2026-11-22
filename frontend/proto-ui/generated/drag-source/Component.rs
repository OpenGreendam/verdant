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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "accept" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 1 && arguments.get(0).map_or(true, |value| matches!(value, Value::Undefined) || (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
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
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct DragSourceHandle { pub owner: OwnerRef }
  impl DragSourceHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_6(&self, window: &mut Window, cx: &mut App) -> Result<f64, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("begin").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("begin", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.as_f64().is_some_and(f64::is_finite)) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_7(&self, argument_0: f64, argument_1: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("accept").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("accept", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_8(&self, argument_0: Option<f64>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![argument_0.map(|value| Value::json(serde_json::to_value(value).expect("portable argument"))).unwrap_or(Value::Undefined)];
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
  pub struct DragSource { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl DragSource {
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
            let v_7068617365: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6e6578744c65617365: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6c65617365: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/drag-source.proto.ts":11:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))]))])) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":13:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":14:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":15:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":16:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":17:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":18:5
            native_builtin_hook(&owner, "asTrigger", || as_trigger(&owner));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":19:5
            { let v_7075695f6e61746976655f34 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f34.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":20:5
            { let v_7075695f6e61746976655f35 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f35.clone()).owner(), Value::string("button")) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":21:5
            { let v_7075695f6e61746976655f36 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f36.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":22:5
            { let v_7075695f6e61746976655f37 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f37.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":23:5
            { let v_7075695f6e61746976655f38 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f38.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-drag-choose"))])) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":24:5
            { let v_7075695f6e61746976655f39 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":25:5
            { let v_7075695f6e61746976655f3130 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":26:5
            { let v_7075695f6e61746976655f3131 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":27:5
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":28:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":29:5
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_event(&owner, Value::string("choose")) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":30:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_event(&owner, Value::string("cancelled")) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":31:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":32:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":33:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":34:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":35:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":36:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":37:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":38:9
            v_6e6578744c65617365.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":39:9
            v_6c65617365.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/drag-source.proto.ts":40:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f66666664303166646466633665343065: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":40:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f66666664303166646466633665343065.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":41:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f35633261306264303961636630623639: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":41:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f35633261306264303961636630623639.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":42:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_65787065637465645f5f32623061376239653163613366653830: Value = { let argument = arg(&arguments, 0); if matches!(&argument, Value::Undefined) { Value::number(0.0) } else { argument } };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":43:7
                if ({ let left = Value::boolean(!strict_equal(&(v_65787065637465645f5f32623061376239653163613366653830.clone()), &(Value::number(0.0)))); if left.truthy() { Value::boolean(!strict_equal(&(v_65787065637465645f5f32623061376239653163613366653830.clone()), &({ let value = (v_6c65617365.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":43:49
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":44:13
                let v_68616e646c655f5f31373933376233343834626636396132 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":45:13
                let v_726574697265645f5f37616634333134323263376230643337 = { let value = (v_6c65617365.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":46:13
                let v_62617369735f5f39376434643734306432343262653065 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":47:13
                let v_7469636b65745f5f62643830653461306433613434383161 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":48:7
                { let value = Value::number(0.0); *(v_6c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":49:7
                { let v_7075695f6e61746976655f3136 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3136.clone(), Value::string("idle"), Value::string("reason: transfer cancelled")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":50:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62643830653461306433613434383161.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":50:29
                    return Value::boolean(!strict_equal(&(v_726574697265645f5f37616634333134323263376230643337.clone()), &(Value::number(0.0))));
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":51:7
                if ({ let left = v_68616e646c655f5f31373933376233343834626636396132.clone(); if left.truthy() { Value::boolean(!strict_equal(&(v_726574697265645f5f37616634333134323263376230643337.clone()), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":51:36
                    { let v_7075695f6e61746976655f3137 = v_68616e646c655f5f31373933376233343834626636396132.clone(); emit(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("cancelled"), record(vec![("lease", v_726574697265645f5f37616634333134323263376230643337.clone()), ("revision", v_62617369735f5f39376434643734306432343262653065.clone())])) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":52:7
                if ({ let left = { let left = v_68616e646c655f5f31373933376233343834626636396132.clone(); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62643830653461306433613434383161.clone()]) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":52:50
                    { let v_7075695f6e61746976655f3138 = v_68616e646c655f5f31373933376233343834626636396132.clone(); request_update(&(v_7075695f6e61746976655f3138.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":53:7
                return Value::boolean(!strict_equal(&(v_726574697265645f5f37616634333134323263376230643337.clone()), &(Value::number(0.0))));
            })) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":55:5
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6e6578744c65617365 = v_6e6578744c65617365.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":56:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3230 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3230.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":56:86
                    return Value::number(0.0);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":57:13
                let v_7469636b65745f5f66323435663930353564613137393365 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":58:7
                { let v_7075695f6e61746976655f3231 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3231.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":59:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66323435663930353564613137393365.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3232 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3232.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":59:60
                    return Value::number(0.0);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":60:7
                { let value = add({ let value = (v_6e6578744c65617365.read()).borrow().clone(); value }, Value::number(1.0)); *(v_6e6578744c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":61:7
                { let value = { let value = (v_6e6578744c65617365.read()).borrow().clone(); value }; *(v_6c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":62:13
                let v_746f6b656e = { let value = (v_6c65617365.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":63:7
                { let v_7075695f6e61746976655f3233 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3233.clone(), Value::string("dragging"), Value::string("reason: transfer acquired")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":64:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66323435663930353564613137393365.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":64:29
                    return Value::number(0.0);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":65:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":65:16
                    { let v_7075695f6e61746976655f3234 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3234.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":66:7
                return if (v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66323435663930353564613137393365.clone()])).truthy() { v_746f6b656e.clone() } else { Value::number(0.0) };
            })) }) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":68:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_method(&owner, Value::string("accept"), {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_65787065637465645f5f33356262646439666564643331383239: Value = arg(&arguments, 0);
                let v_62617369735f5f37353132653065383462643033643162: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":69:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3236 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3236.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3237 = v_7068617365.read(); (v_7075695f6e61746976655f3237.clone()).get() }), &(Value::string("dragging")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_65787065637465645f5f33356262646439666564643331383239.clone()), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_65787065637465645f5f33356262646439666564643331383239.clone()), &({ let value = (v_6c65617365.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f37353132653065383462643033643162.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":70:72
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":71:13
                let v_7469636b65745f5f63373463613533643563313462303536 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":72:7
                { let value = Value::number(0.0); *(v_6c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":73:7
                { let v_7075695f6e61746976655f3238 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3238.clone(), Value::string("idle"), Value::string("reason: transfer consumed")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":74:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f63373463613533643563313462303536.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":74:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":75:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":75:16
                    { let v_7075695f6e61746976655f3239 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3239.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":76:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f63373463613533643563313462303536.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":78:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":79:11
            let v_63686f6f7365 = {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":80:13
                let v_68616e646c655f5f35383666313837333965346232623932 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":81:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f35383666313837333965346232623932.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3331 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3331.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":81:89
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":82:13
                let v_7469636b65745f5f31353164646631343061646664633930 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":83:7
                { let v_7075695f6e61746976655f3332 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3332.clone(), Value::string("idle"), Value::string("reason: transfer chooser")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":84:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31353164646631343061646664633930.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":84:29
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":85:7
                { let v_7075695f6e61746976655f3333 = v_68616e646c655f5f35383666313837333965346232623932.clone(); emit(&(v_7075695f6e61746976655f3333.clone()).owner(), Value::string("choose"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                Value::Undefined
            })) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":87:5
            { let v_7075695f6e61746976655f3334 = v_646566.clone(); listen(&owner, Value::string("press.start"), false, {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6c65617365 = v_6c65617365.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":88:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3335 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3335.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":88:86
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":89:7
                { let v_7075695f6e61746976655f3336 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3336.clone(), Value::string("pressed"), Value::string("reason: transfer pressed")) };
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":91:5
            { let v_7075695f6e61746976655f3337 = v_646566.clone(); listen(&owner, Value::string("press.commit"), false, v_63686f6f7365.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":92:5
            { let v_7075695f6e61746976655f3338 = v_646566.clone(); listen(&owner, Value::string("host:verdant-drag-choose"), false, v_63686f6f7365.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":93:5
            { let v_7075695f6e61746976655f3339 = v_646566.clone(); listen(&owner, Value::string("press.cancel"), false, {  let v_6c65617365 = v_6c65617365.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":93:42
                if (Value::boolean(strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0))))).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":93:59
                    v_63616e63656c.read().call_authored(vec![]);
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":94:5
            { let v_7075695f6e61746976655f3430 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7068617365 = v_7068617365.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f68616e646c65: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":95:7
                if ({ let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape")))); if left.truthy() { { let left = Value::boolean(!strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0)))); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3431 = v_7068617365.read(); (v_7075695f6e61746976655f3431.clone()).get() }), &(Value::string("idle")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":96:9
                    { let v_7075695f6e61746976655f3432 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3432.clone()) };
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":97:9
                    v_63616e63656c.read().call_authored(vec![]);
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":100:5
            { let v_7075695f6e61746976655f3433 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, {  let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":100:37
                return v_63616e63656c.read().call_authored(vec![]);
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":101:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f33653934653235333339356164623437: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":102:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":102:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":103:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":104:13
                let v_7469636b65745f5f63316132333561316361366236376637 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":105:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":106:7
                { let value = v_68616e646c655f5f33653934653235333339356164623437.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":107:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3434 = v_68616e646c655f5f33653934653235333339356164623437.clone(); read_props(&(v_7075695f6e61746976655f3434.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":108:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":109:13
                let v_726574697265645f5f64366265353133333839333265323562 = if ({ let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } }).truthy() { { let value = (v_6c65617365.read()).borrow().clone(); value } } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":110:13
                let v_62617369735f5f37626435313231633463653338663539 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":111:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":112:7
                if (Value::boolean(!strict_equal(&(v_726574697265645f5f64366265353133333839333265323562.clone()), &(Value::number(0.0))))).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":112:26
                    { let value = Value::number(0.0); *(v_6c65617365.read()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":113:7
                { let v_7075695f6e61746976655f3435 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3435.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: transfer label")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":114:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f63316132333561316361366236376637.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":114:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":115:7
                { let v_7075695f6e61746976655f3436 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3436.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: transfer policy")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":116:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f63316132333561316361366236376637.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":116:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":117:7
                { let v_7075695f6e61746976655f3437 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3437.clone()).owner(), { let v_7075695f6e61746976655f3438 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3438.clone()).get() }) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":118:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f63316132333561316361366236376637.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":118:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":119:7
                if (Value::boolean(strict_equal(&({ let value = (v_6c65617365.read()).borrow().clone(); value }), &(Value::number(0.0))))).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":119:24
                    { let v_7075695f6e61746976655f3439 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3439.clone(), Value::string("idle"), Value::string("reason: transfer authority")) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":120:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f63316132333561316361366236376637.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":120:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":121:7
                if (Value::boolean(!strict_equal(&(v_726574697265645f5f64366265353133333839333265323562.clone()), &(Value::number(0.0))))).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":121:26
                    { let v_7075695f6e61746976655f3530 = v_68616e646c655f5f33653934653235333339356164623437.clone(); emit(&(v_7075695f6e61746976655f3530.clone()).owner(), Value::string("cancelled"), record(vec![("lease", v_726574697265645f5f64366265353133333839333265323562.clone()), ("revision", v_62617369735f5f37626435313231633463653338663539.clone())])) };
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":122:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f63316132333561316361366236376637.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":122:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":123:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":124:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":124:21
                    { let v_7075695f6e61746976655f3531 = v_68616e646c655f5f33653934653235333339356164623437.clone(); request_update(&(v_7075695f6e61746976655f3531.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":126:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_6c65617365 = v_6c65617365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":127:13
                let v_68616e646c655f5f34326431653134336231346232643839 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":128:13
                let v_726574697265645f5f61306235366138356330316536663863 = { let value = (v_6c65617365.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":129:13
                let v_62617369735f5f38386130346266626534373436663762 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":130:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":131:13
                let v_7469636b65745f5f65323165663834326336393261343835 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":132:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":133:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":134:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":135:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":136:7
                { let value = Value::number(0.0); *(v_6c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":137:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":137:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":138:7
                { let v_7075695f6e61746976655f3532 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3532.clone(), Value::string("idle"), Value::string("reason: transfer detached")) };
                // Source "frontend/proto-ui/src/drag-source.proto.ts":139:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f65323165663834326336393261343835.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":139:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":140:7
                if ({ let left = v_68616e646c655f5f34326431653134336231346232643839.clone(); if left.truthy() { Value::boolean(!strict_equal(&(v_726574697265645f5f61306235366138356330316536663863.clone()), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/drag-source.proto.ts":140:36
                    { let v_7075695f6e61746976655f3533 = v_68616e646c655f5f34326431653134336231346232643839.clone(); emit(&(v_7075695f6e61746976655f3533.clone()).owner(), Value::string("cancelled"), record(vec![("lease", v_726574697265645f5f61306235366138356330316536663863.clone()), ("revision", v_62617369735f5f38386130346266626534373436663762.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/drag-source.proto.ts":142:5
            { let v_7075695f6e61746976655f3534 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":143:5
            { let v_7075695f6e61746976655f3535 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f37373534376162626362313337373038: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/drag-source.proto.ts":143:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/drag-source.proto.ts":143:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f37373534376162626362313337373038.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":144:5
            { let v_7075695f6e61746976655f3536 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("revision"), Value::string("disabled")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":145:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":145:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":146:5
            { let v_7075695f6e61746976655f3538 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/drag-source.proto.ts":146:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/drag-source.proto.ts":147:5
            return { let callable={ let pui_native_59={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/drag-source.proto.ts":147:26
                return { let v_7075695f6e61746976655f3630 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_59.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_59.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> DragSourceHandle { DragSourceHandle { owner: self.owner.clone() } }
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
  impl Drop for DragSource { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for DragSource { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for DragSource {
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
