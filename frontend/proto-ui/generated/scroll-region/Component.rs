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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("trapFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("tabStop").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("axes").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revealIndex").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("revealRevision").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "autoFocus" => value.is_boolean(),
        "trapFocus" => value.is_boolean(),
        "tabStop" => value.is_boolean(),
        "axes" => value.is_string(),
        "revealIndex" => value.as_f64().is_some_and(f64::is_finite),
        "revealRevision" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "key" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "keyInput" => if !(arguments.len() <= 4 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "revealGeometry" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<bool>,
    #[serde(rename = "trapFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "tabStop", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "axes", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<String>,
    #[serde(rename = "revealIndex", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<f64>,
    #[serde(rename = "revealRevision", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct ScrollRegionHandle { pub owner: OwnerRef }
  impl ScrollRegionHandle {
      pub fn state_0(&self) -> Result<f64, Value> { self.state("horizontalPosition").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("position").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("atEnd").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("scopeActive").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_5(&self, argument_0: String, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("key").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("key", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_6(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyInput").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyInput", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_7(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("revealGeometry").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("revealGeometry", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_8(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
  pub struct ScrollRegion { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl ScrollRegion {
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
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_73636f7065: LexicalBinding<Value> = lexical_binding();
            let v_7363726f6c6c: LexicalBinding<Value> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<bool>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_61786573: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_72657665616c496e646578: LexicalBinding<Rc<std::cell::RefCell<f64>>> = lexical_binding();
            let v_72657665616c5265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b6579: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool) -> bool>> = lexical_binding();
            let v_6b6579496e707574: LexicalBinding<native::gpui_native_owner::NativeCallable<dyn Fn(Value, bool, bool, bool) -> bool>> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":12:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("document"))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("trapFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("tabStop", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("axes", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("revealIndex", record(vec![("type", Value::string("number")), ("default", Value::number(-(1.0_f64)))])), ("revealRevision", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":15:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":15:56
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":16:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("document"), Value::Undefined) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":17:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":17:34
            { let v_7075695f6e61746976655f34 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f34.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":18:11
            v_73636f7065.initialize(native_builtin_hook(&owner, "asFocusScope", || focus_setup(&owner, "focus-scope")));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":18:35
            { let v_7075695f6e61746976655f35 = v_73636f7065.read(); focus_setup_config(&(v_7075695f6e61746976655f35.clone()).owner(), "focus-scope", record(vec![("trap", Value::boolean(true)), ("loop", Value::boolean(true)), ("navigation", Value::string("tab")), ("entry", Value::string("manual")), ("restore", Value::string("none"))])) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":19:11
            v_7363726f6c6c.initialize(native_builtin_hook(&owner, "asScrollSurface", || scroll_setup(&owner)));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":19:39
            { let v_7075695f6e61746976655f36 = v_7363726f6c6c.read(); scroll_configure(&(v_7075695f6e61746976655f36.clone()).owner(), record(vec![("axes", Value::string("both")), ("projection", Value::string("system"))])) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":20:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":20:34
            { let v_7075695f6e61746976655f37 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f37.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":20:51
            { let v_7075695f6e61746976655f38 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f38.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":20:69
            { let v_7075695f6e61746976655f39 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f39.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":21:5
            { let v_7075695f6e61746976655f3130 = v_646566.clone(); expose_state(&owner, Value::string("horizontalPosition"), ((v_7363726f6c6c.read()).member("horizontal", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":22:5
            { let v_7075695f6e61746976655f3131 = v_646566.clone(); expose_state(&owner, Value::string("position"), ((v_7363726f6c6c.read()).member("vertical", false)).member("position", false)) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":22:61
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); expose_state(&owner, Value::string("atEnd"), ((v_7363726f6c6c.read()).member("vertical", false)).member("atEnd", false)) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":23:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":23:49
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_state(&owner, Value::string("scopeActive"), (v_73636f7065.read()).member("active", false)) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(false)));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:53
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(0.0_f64)));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:68
            v_61786573.initialize(Rc::new(std::cell::RefCell::new(Value::string("vertical"))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":25:9
            v_72657665616c496e646578.initialize(Rc::new(std::cell::RefCell::new(-(1.0_f64))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":25:31
            v_72657665616c5265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":26:11
            v_6b6579.initialize({  let v_73636f7065 = v_73636f7065.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_61786573 = v_61786573.clone(); let call:Rc<dyn Fn(Value, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool| -> bool {
                let v_6e616d655f5f37373832613636303363333634346562: Value = argument_0;
                let v_73686966745f5f66353537663061656365373439323033: bool = argument_1;
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":27:7
                if { let pui_native_15=!({ let value=(v_6174746163686564.read()).borrow().clone(); value }); if !(pui_native_15) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_15} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":27:34
                    return false;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("Tab"))))).bool() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:29
                    if !(({ let v_7075695f6e61746976655f3139 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f3139.clone()).owner())).bool()) }).bool()) {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:52
                        return false;
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:66
                    if v_73686966745f5f66353537663061656365373439323033.clone() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:77
                        { let v_7075695f6e61746976655f3230 = v_73636f7065.read(); focus_navigate(&(v_7075695f6e61746976655f3230.clone()).owner(), false, "prev", Value::Undefined) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:101
                        { let v_7075695f6e61746976655f3231 = v_73636f7065.read(); focus_navigate(&(v_7075695f6e61746976655f3231.clone()).owner(), false, "next", Value::Undefined) };
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:120
                    return true;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":30:7
                if { let pui_native_22=(Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal"))))).bool(); if (pui_native_22) {{ let pui_native_26=(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowUp"))))).bool(); if !(pui_native_26) {(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowDown"))))).bool()} else {pui_native_26} }} else {pui_native_22} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":30:82
                    return false;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":31:13
                let v_686f72697a6f6e74616c: bool = { let pui_native_32=(Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal"))))).bool(); if !(pui_native_32) {{ let pui_native_36=(Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("both"))))).bool(); if (pui_native_36) {v_73686966745f5f66353537663061656365373439323033.clone()} else {pui_native_36} }} else {pui_native_32} };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":32:7
                if { let pui_native_40={ let pui_native_42=(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowLeft"))))).bool(); if !(pui_native_42) {(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowRight"))))).bool()} else {pui_native_42} }; if (pui_native_40) {(Value::boolean(!strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("vertical"))))).bool()} else {pui_native_40} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:9
                    { let v_7075695f6e61746976655f3530 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3530.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowLeft"))))).bool() {-(40.0_f64)} else {40.0_f64}))])) };
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:101
                    return true;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowUp"))))).bool() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:33
                    if v_686f72697a6f6e74616c.clone() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:49
                        { let v_7075695f6e61746976655f3535 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3535.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(-(40.0_f64)))])) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:118
                        { let v_7075695f6e61746976655f3536 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3536.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(-(40.0_f64)))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("ArrowDown"))))).bool() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:40
                        if v_686f72697a6f6e74616c.clone() {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:56
                            { let v_7075695f6e61746976655f3539 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3539.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(40.0_f64))])) };
                        } else {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:124
                            { let v_7075695f6e61746976655f3630 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3630.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(40.0_f64))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:12
                        if { let pui_native_61=(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("PageUp"))))).bool(); if !(pui_native_61) {{ let pui_native_65=(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string(" "))))).bool(); if (pui_native_65) {v_73686966745f5f66353537663061656365373439323033.clone()} else {pui_native_65} }} else {pui_native_61} } {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:64
                            if v_686f72697a6f6e74616c.clone() {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:80
                                { let v_7075695f6e61746976655f3639 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3639.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                            } else {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:160
                                { let v_7075695f6e61746976655f3730 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3730.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:12
                            if { let pui_native_71=(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("PageDown"))))).bool(); if !(pui_native_71) {(Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string(" "))))).bool()} else {pui_native_71} } {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:55
                                if v_686f72697a6f6e74616c.clone() {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:71
                                    { let v_7075695f6e61746976655f3737 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3737.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:150
                                    { let v_7075695f6e61746976655f3738 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3738.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                                }
                            } else {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":39:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("Home"))))).bool() {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":39:35
                                    if v_686f72697a6f6e74616c.clone() {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":39:51
                                        { let v_7075695f6e61746976655f3831 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3831.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0_f64))])) };
                                    } else {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":39:121
                                        { let v_7075695f6e61746976655f3832 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3832.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0_f64))])) };
                                    }
                                } else {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":40:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f37373832613636303363333634346562.clone()), &(Value::string("End"))))).bool() {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":40:34
                                        if v_686f72697a6f6e74616c.clone() {
                                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":40:50
                                            { let v_7075695f6e61746976655f3835 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3835.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                        } else {
                                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":40:111
                                            { let v_7075695f6e61746976655f3836 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3836.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                        }
                                    } else {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":41:12
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":42:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":44:11
            v_6b6579496e707574.initialize({  let v_6b6579 = v_6b6579.clone(); let call:Rc<dyn Fn(Value, bool, bool, bool)->bool>=Rc::new(move |argument_0: Value, argument_1: bool, argument_2: bool, argument_3: bool| -> bool {
                let v_6e616d655f5f36386638646331396164333065313162: Value = argument_0;
                let v_73686966745f5f62646664393066313264653663363563: bool = argument_1;
                let v_636f6d6d616e64: bool = argument_2;
                let v_616c74: bool = argument_3;
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":45:7
                if { let pui_native_87=v_636f6d6d616e64.clone(); if !(pui_native_87) {v_616c74.clone()} else {pui_native_87} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":45:27
                    return false;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":46:7
                return (v_6b6579.read().call)(v_6e616d655f5f36386638646331396164333065313162.clone(), v_73686966745f5f62646664393066313264653663363563.clone());
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call(arg(&arguments,0), (arg(&arguments,1)).bool(), (arg(&arguments,2)).bool(), (arg(&arguments,3)).bool()))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":48:5
            { let v_7075695f6e61746976655f3839 = v_646566.clone(); expose_method(&owner, Value::string("key"), (v_6b6579.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":49:5
            { let v_7075695f6e61746976655f3930 = v_646566.clone(); expose_method(&owner, Value::string("keyInput"), (v_6b6579496e707574.read()).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":52:5
            { let v_7075695f6e61746976655f3931 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), ({  let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_61786573 = v_61786573.clone(); let v_72657665616c496e646578 = v_72657665616c496e646578.clone(); let v_72657665616c5265766973696f6e = v_72657665616c5265766973696f6e.clone(); let call:Rc<dyn Fn(f64, f64, f64, f64, f64, Value)->bool>=Rc::new(move |argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: Value| -> bool {
                let v_696e646578: f64 = argument_0;
                let v_7374617274: f64 = argument_1;
                let v_656e64: f64 = argument_2;
                let v_76696577706f72745374617274: f64 = argument_3;
                let v_76696577706f7274456e64: f64 = argument_4;
                let v_6261736973: Value = argument_5;
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":53:7
                if { let pui_native_92={ let pui_native_94={ let pui_native_96={ let pui_native_98={ let pui_native_100={ let pui_native_102={ let pui_native_104={ let pui_native_106={ let pui_native_108={ let pui_native_110={ let pui_native_112={ let pui_native_114=!({ let value=(v_6174746163686564.read()).borrow().clone(); value }); if !(pui_native_114) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_114} }; if !(pui_native_112) {(Value::boolean(!strict_equal(&(v_6261736973.clone()), &({ let value = (v_72657665616c5265766973696f6e.read()).borrow().clone(); value })))).bool()} else {pui_native_112} }; if !(pui_native_110) {{ let pui_native_118=v_696e646578.clone(); let pui_native_119={ let value=(v_72657665616c496e646578.read()).borrow().clone(); value }; !(pui_native_118==pui_native_119) }} else {pui_native_110} }; if !(pui_native_108) {!((v_696e646578.clone()).is_finite())} else {pui_native_108} }; if !(pui_native_106) {{ let pui_native_120:f64=v_696e646578.clone(); let pui_native_121:f64=0.0_f64; pui_native_120 < pui_native_121 }} else {pui_native_106} }; if !(pui_native_104) {{ let pui_native_122={ let pui_native_124:f64=v_696e646578.clone(); let pui_native_125:f64=1.0_f64; pui_native_124 % pui_native_125 }; let pui_native_123=0.0_f64; !(pui_native_122==pui_native_123) }} else {pui_native_104} }; if !(pui_native_102) {!((v_7374617274.clone()).is_finite())} else {pui_native_102} }; if !(pui_native_100) {!((v_656e64.clone()).is_finite())} else {pui_native_100} }; if !(pui_native_98) {!((v_76696577706f72745374617274.clone()).is_finite())} else {pui_native_98} }; if !(pui_native_96) {!((v_76696577706f7274456e64.clone()).is_finite())} else {pui_native_96} }; if !(pui_native_94) {{ let pui_native_126:f64=v_656e64.clone(); let pui_native_127:f64=v_7374617274.clone(); pui_native_126 < pui_native_127 }} else {pui_native_94} }; if !(pui_native_92) {{ let pui_native_128:f64=v_76696577706f7274456e64.clone(); let pui_native_129:f64=v_76696577706f72745374617274.clone(); pui_native_128 <= pui_native_129 }} else {pui_native_92} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":56:57
                    return false;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":59:7
                if { let pui_native_130={ let pui_native_132:f64=v_7374617274.clone(); let pui_native_133:f64=v_76696577706f72745374617274.clone(); pui_native_132 <= pui_native_133 }; if (pui_native_130) {{ let pui_native_134:f64=v_656e64.clone(); let pui_native_135:f64=v_76696577706f7274456e64.clone(); pui_native_134 >= pui_native_135 }} else {pui_native_130} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":59:57
                    return true;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":60:13
                let v_64656c7461: f64 = if { let pui_native_136:f64=v_7374617274.clone(); let pui_native_137:f64=v_76696577706f72745374617274.clone(); pui_native_136 < pui_native_137 } {{ let pui_native_138:f64=v_7374617274.clone(); let pui_native_139:f64=v_76696577706f72745374617274.clone(); pui_native_138 - pui_native_139 }} else {if { let pui_native_140:f64=v_656e64.clone(); let pui_native_141:f64=v_76696577706f7274456e64.clone(); pui_native_140 > pui_native_141 } {{ let pui_native_142:f64=v_656e64.clone(); let pui_native_143:f64=v_76696577706f7274456e64.clone(); pui_native_142 - pui_native_143 }} else {0.0_f64}};
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":61:7
                if { let pui_native_144=v_64656c7461.clone(); let pui_native_145=0.0_f64; !(pui_native_144==pui_native_145) } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":62:9
                    if (Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal"))))).bool() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":62:36
                        { let v_7075695f6e61746976655f313438 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f313438.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(v_64656c7461.clone()))])) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":63:14
                        { let v_7075695f6e61746976655f313439 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f313439.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(v_64656c7461.clone()))])) };
                    }
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":65:7
                return true;
            }); {
      let ordinary_call=call.clone();
      let ordinary=Value::Function(Rc::new(move |arguments:Vec<Value>| {Value::boolean(ordinary_call((arg(&arguments,0)).num(), (arg(&arguments,1)).num(), (arg(&arguments,2)).num(), (arg(&arguments,3)).num(), (arg(&arguments,4)).num(), arg(&arguments,5)))}));
      native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:Some(ordinary) }
    } }).ordinary.clone().expect("checked native data callable")) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:5
            { let v_7075695f6e61746976655f313530 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:44
                if { let pui_native_151={ let value=(v_6174746163686564.read()).borrow().clone(); value }; if (pui_native_151) {!({ let value=(v_646973706f736564.read()).borrow().clone(); value })} else {pui_native_151} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:71
                    { let v_7075695f6e61746976655f313533 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f313533.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":68:5
            { let v_7075695f6e61746976655f313534 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_666f637573 = v_666f637573.clone(); let v_6b6579496e707574 = v_6b6579496e707574.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":71:7
                if !(({ let v_7075695f6e61746976655f313535 = (v_666f637573.read()).member("focused", false); Value::boolean(((v_7075695f6e61746976655f313535.clone()).get()).bool()) }).bool()) {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":71:33
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":72:7
                if (v_6b6579496e707574.read().call)({ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true))))).bool(), { let pui_native_158=(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true))))).bool(); if !(pui_native_158) {(Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true))))).bool()} else {pui_native_158} }, (Value::boolean(strict_equal(&((v_6576656e74.clone()).member("altKey", false)), &(Value::boolean(true))))).bool()) {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":72:136
                    { let v_7075695f6e61746976655f313636 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f313636.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":74:11
            v_73796e63.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_73636f7065 = v_73636f7065.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_61786573 = v_61786573.clone(); let v_72657665616c496e646578 = v_72657665616c496e646578.clone(); let v_72657665616c5265766973696f6e = v_72657665616c5265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f38643036613563396531323732636461: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":75:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":75:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":75:35
                let v_7469636b65745f5f35353534336439323533613764666164: f64 = { let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":75:59
                let v_70726f7073 = { let v_7075695f6e61746976655f313637 = v_72756e5f5f38643036613563396531323732636461.clone(); read_props(&(v_7075695f6e61746976655f313637.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":76:7
                { let value = if (Value::boolean({ let pui_native_168=(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("axes", false)), &(Value::string("horizontal"))))).bool(); if !(pui_native_168) {(Value::boolean(strict_equal(&((v_70726f7073.clone()).member("axes", false)), &(Value::string("both"))))).bool()} else {pui_native_168} })).truthy() { (v_70726f7073.clone()).member("axes", false) } else { Value::string("vertical") }; *(v_61786573.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":77:7
                if (Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal"))))).bool() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":77:34
                    { let v_7075695f6e61746976655f313736 = v_7363726f6c6c.read(); scroll_sync(&(v_7075695f6e61746976655f313736.clone()).owner(), record(vec![("axes", Value::string("horizontal"))])) };
                } else {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":78:12
                    if (Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("both"))))).bool() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":78:33
                        { let v_7075695f6e61746976655f313739 = v_7363726f6c6c.read(); scroll_sync(&(v_7075695f6e61746976655f313739.clone()).owner(), record(vec![("axes", Value::string("both"))])) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":79:12
                        { let v_7075695f6e61746976655f313830 = v_7363726f6c6c.read(); scroll_sync(&(v_7075695f6e61746976655f313830.clone()).owner(), record(vec![("axes", Value::string("vertical"))])) };
                    }
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:7
                if { let pui_native_181={ let pui_native_183={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_184=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_183==pui_native_184) }; if !(pui_native_181) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_181} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":81:7
                Value::number({ let value=((v_70726f7073.clone()).member("revealIndex", false)).num(); *(v_72657665616c496e646578.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":81:46
                { let value = { let left = (v_70726f7073.clone()).member("revealRevision", false); if left.nullish() { Value::string("") } else { left } }; *(v_72657665616c5265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":82:7
                { let v_7075695f6e61746976655f313837 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f313837.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: scroll region name")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":82:67
                if { let pui_native_188={ let pui_native_190={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_191=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_190==pui_native_191) }; if !(pui_native_188) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_188} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":82:101
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":83:7
                { let v_7075695f6e61746976655f313932 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f313932.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: scroll region description")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":83:86
                if { let pui_native_193={ let pui_native_195={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_196=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_195==pui_native_196) }; if !(pui_native_193) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_193} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":83:120
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:7
                { let v_7075695f6e61746976655f313937 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f313937.clone(), { let left = (v_70726f7073.clone()).member("role", false); if left.nullish() { Value::string("document") } else { left } }, Value::string("reason: scroll region role")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:73
                if { let pui_native_198={ let pui_native_200={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_201=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_200==pui_native_201) }; if !(pui_native_198) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_198} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:107
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:7
                { let v_7075695f6e61746976655f323032 = v_666f637573.read(); focus_nav(&(v_7075695f6e61746976655f323032.clone()).owner(), if (Value::boolean({ let pui_native_203=((v_70726f7073.clone()).member("tabStop", false)).bool(); let pui_native_204=false; (pui_native_203==pui_native_204) })).truthy() { Value::string("none") } else { Value::string("auto") }) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:77
                if { let pui_native_205={ let pui_native_207={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_208=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_207==pui_native_208) }; if !(pui_native_205) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_205} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:111
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":86:7
                if { let value=(v_6174746163686564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":87:9
                    if { let pui_native_209={ let pui_native_211=((v_70726f7073.clone()).member("trapFocus", false)).bool(); let pui_native_212=true; (pui_native_211==pui_native_212) }; if (pui_native_209) {!(({ let v_7075695f6e61746976655f323133 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f323133.clone()).owner())).bool()) }).bool())} else {pui_native_209} } {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":87:60
                        { let v_7075695f6e61746976655f323134 = v_73636f7065.read(); focus_activate(&(v_7075695f6e61746976655f323134.clone()).owner(), Value::Undefined) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":88:14
                        if { let pui_native_215={ let pui_native_217=((v_70726f7073.clone()).member("trapFocus", false)).bool(); let pui_native_218=true; !(pui_native_217==pui_native_218) }; if (pui_native_215) {({ let v_7075695f6e61746976655f323139 = v_73636f7065.read(); Value::boolean((focus_is_active(&(v_7075695f6e61746976655f323139.clone()).owner())).bool()) }).bool()} else {pui_native_215} } {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":88:64
                            { let v_7075695f6e61746976655f323230 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f323230.clone()).owner(), Value::Undefined) };
                        }
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":89:9
                    if { let pui_native_221={ let pui_native_223={ let value=(v_65706f6368.read()).borrow().clone(); value }; let pui_native_224=v_7469636b65745f5f35353534336439323533613764666164.clone(); !(pui_native_223==pui_native_224) }; if !(pui_native_221) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_221} } {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":89:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":90:9
                    { let v_7075695f6e61746976655f323235 = v_72756e5f5f38643036613563396531323732636461.clone(); request_update(&(v_7075695f6e61746976655f323235.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":93:5
            { let v_7075695f6e61746976655f323236 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":94:5
            { let v_7075695f6e61746976655f323237 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description"), Value::string("role"), Value::string("autoFocus"), Value::string("trapFocus"), Value::string("tabStop"), Value::string("axes"), Value::string("revealIndex"), Value::string("revealRevision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":95:5
            { let v_7075695f6e61746976655f323238 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f37313733623537613635643266643864: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":96:7
                if { let value=(v_646973706f736564.read()).borrow().clone(); value } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":96:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":96:29
                Value::boolean({ let value=true; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":96:46
                v_73796e63.read().call_authored(vec![v_72756e5f5f37313733623537613635643266643864.clone()]);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":96:63
                let v_7469636b65745f5f61613164373736396431633634373361: f64 = { let value=(v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":97:7
                if { let pui_native_229={ let pui_native_231={ let pui_native_233=v_7469636b65745f5f61613164373736396431633634373361.clone(); let pui_native_234={ let value=(v_65706f6368.read()).borrow().clone(); value }; !(pui_native_233==pui_native_234) }; if !(pui_native_231) {!({ let value=(v_6174746163686564.read()).borrow().clone(); value })} else {pui_native_231} }; if !(pui_native_229) {{ let value=(v_646973706f736564.read()).borrow().clone(); value }} else {pui_native_229} } {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":97:54
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":98:7
                if (({ let v_7075695f6e61746976655f323335 = v_72756e5f5f37313733623537613635643266643864.clone(); read_props(&(v_7075695f6e61746976655f323335.clone()).owner(), false) }).member("autoFocus", false)).bool() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":98:38
                    { let v_7075695f6e61746976655f323336 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f323336.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":101:5
            { let v_7075695f6e61746976655f323337 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_73636f7065 = v_73636f7065.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":101:39
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":101:48
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":101:66
                { let v_7075695f6e61746976655f323338 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f323338.clone()).owner(), Value::Undefined) };
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":102:5
            { let v_7075695f6e61746976655f323339 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_73636f7065 = v_73636f7065.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":102:43
                Value::number({ let cell=v_65706f6368.read(); let previous=cell.borrow().clone(); let value=previous + 1.0; *cell.borrow_mut()=value; value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":102:52
                Value::boolean({ let value=false; *(v_6174746163686564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":102:70
                Value::boolean({ let value=true; *(v_646973706f736564.read()).borrow_mut()=value.clone(); value });
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":102:87
                { let v_7075695f6e61746976655f323430 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f323430.clone()).owner(), Value::Undefined) };
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":103:5
            { let v_7075695f6e61746976655f323431 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-h-0".into(), "min-h-".into())])]) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":104:5
            return { let callable={ let pui_native_242={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":104:24
                return { let v_7075695f6e61746976655f323433 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call, ordinary:None } }; Value::NativeFunction{identity:pui_native_242.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_242.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ScrollRegionHandle { ScrollRegionHandle { owner: self.owner.clone() } }
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
  impl Drop for ScrollRegion { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for ScrollRegion { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for ScrollRegion {
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
