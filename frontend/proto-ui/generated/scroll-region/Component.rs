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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("scope").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("axes").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revealIndex").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("revealRevision").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "autoFocus" => value.is_boolean(),
        "scope" => value.is_boolean(),
        "axes" => value.is_string(),
        "revealIndex" => value.as_f64().is_some_and(f64::is_finite),
        "revealRevision" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "key" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "scope", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "axes", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<String>,
    #[serde(rename = "revealIndex", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<f64>,
    #[serde(rename = "revealRevision", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<String>,
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
    pub fn method_6(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: f64, argument_5: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_73636f7065: LexicalBinding<Value> = lexical_binding();
            let v_7363726f6c6c: LexicalBinding<Value> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_61786573: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_72657665616c496e646578: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_72657665616c5265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b6579: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":12:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("document"))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("scope", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("axes", record(vec![("type", Value::string("string")), ("default", Value::string("vertical"))])), ("revealIndex", record(vec![("type", Value::string("number")), ("default", Value::number(-(Value::number(1.0)).scalar_number()))])), ("revealRevision", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
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
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:53
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":24:68
            v_61786573.initialize(Rc::new(std::cell::RefCell::new(Value::string("vertical"))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":25:9
            v_72657665616c496e646578.initialize(Rc::new(std::cell::RefCell::new(Value::number(-(Value::number(1.0)).scalar_number()))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":25:31
            v_72657665616c5265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":26:11
            v_6b6579.initialize({  let v_73636f7065 = v_73636f7065.clone(); let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_61786573 = v_61786573.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d65: Value = arg(&arguments, 0);
                let v_7368696674: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":27:7
                if ({ let left = Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":27:34
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Tab"))))).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:29
                    if (Value::boolean(!({ let v_7075695f6e61746976655f3135 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3135.clone()).owner()) }).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:52
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:66
                    if (v_7368696674.clone()).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:77
                        { let v_7075695f6e61746976655f3136 = v_73636f7065.read(); focus_navigate(&(v_7075695f6e61746976655f3136.clone()).owner(), false, "prev", Value::Undefined) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:101
                        { let v_7075695f6e61746976655f3137 = v_73636f7065.read(); focus_navigate(&(v_7075695f6e61746976655f3137.clone()).owner(), false, "next", Value::Undefined) };
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":28:120
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":29:13
                let v_686f72697a6f6e74616c = { let left = Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("both")))); if left.truthy() { v_7368696674.clone() } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":30:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowRight")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("vertical")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":31:9
                    { let v_7075695f6e61746976655f3138 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3138.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowLeft"))))).truthy() { Value::number(-(Value::number(40.0)).scalar_number()) } else { Value::number(40.0) })])) };
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":31:101
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowUp"))))).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:33
                    if (v_686f72697a6f6e74616c.clone()).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:49
                        { let v_7075695f6e61746976655f3139 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3139.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(-(Value::number(40.0)).scalar_number()))])) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":33:118
                        { let v_7075695f6e61746976655f3230 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3230.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(-(Value::number(40.0)).scalar_number()))])) };
                    }
                } else {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":34:12
                    if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("ArrowDown"))))).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":34:40
                        if (v_686f72697a6f6e74616c.clone()).truthy() {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":34:56
                            { let v_7075695f6e61746976655f3231 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3231.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", Value::number(40.0))])) };
                        } else {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":34:124
                            { let v_7075695f6e61746976655f3232 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3232.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", Value::number(40.0))])) };
                        }
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:12
                        if ({ let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageUp")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string(" ")))); if left.truthy() { v_7368696674.clone() } else { left } } } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:64
                            if (v_686f72697a6f6e74616c.clone()).truthy() {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:80
                                { let v_7075695f6e61746976655f3233 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3233.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("before"))])) };
                            } else {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":35:160
                                { let v_7075695f6e61746976655f3234 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3234.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("before"))])) };
                            }
                        } else {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:12
                            if ({ let left = Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("PageDown")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string(" ")))) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:55
                                if (v_686f72697a6f6e74616c.clone()).truthy() {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:71
                                    { let v_7075695f6e61746976655f3235 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3235.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("horizontal")), ("direction", Value::string("after"))])) };
                                } else {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":36:150
                                    { let v_7075695f6e61746976655f3236 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3236.clone()).owner(), record(vec![("kind", Value::string("page")), ("axis", Value::string("vertical")), ("direction", Value::string("after"))])) };
                                }
                            } else {
                                // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:12
                                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Home"))))).truthy() {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:35
                                    if (v_686f72697a6f6e74616c.clone()).truthy() {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:51
                                        { let v_7075695f6e61746976655f3237 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3237.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("horizontal")), ("position", Value::number(0.0))])) };
                                    } else {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":37:121
                                        { let v_7075695f6e61746976655f3238 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3238.clone()).owner(), record(vec![("kind", Value::string("to")), ("axis", Value::string("vertical")), ("position", Value::number(0.0))])) };
                                    }
                                } else {
                                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:12
                                    if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("End"))))).truthy() {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:34
                                        if (v_686f72697a6f6e74616c.clone()).truthy() {
                                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:50
                                            { let v_7075695f6e61746976655f3239 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3239.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("horizontal"))])) };
                                        } else {
                                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":38:111
                                            { let v_7075695f6e61746976655f3330 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3330.clone()).owner(), record(vec![("kind", Value::string("to-end")), ("axis", Value::string("vertical"))])) };
                                        }
                                    } else {
                                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":39:12
                                        return Value::boolean(false);
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":40:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":42:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b6579.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":45:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_method(&owner, Value::string("revealGeometry"), {  let v_7363726f6c6c = v_7363726f6c6c.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_61786573 = v_61786573.clone(); let v_72657665616c496e646578 = v_72657665616c496e646578.clone(); let v_72657665616c5265766973696f6e = v_72657665616c5265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e646578: Value = arg(&arguments, 0);
                let v_7374617274: Value = arg(&arguments, 1);
                let v_656e64: Value = arg(&arguments, 2);
                let v_76696577706f72745374617274: Value = arg(&arguments, 3);
                let v_76696577706f7274456e64: Value = arg(&arguments, 4);
                let v_6261736973: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":46:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6261736973.clone()), &({ let value = (v_72657665616c5265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_696e646578.clone()), &({ let value = (v_72657665616c496e646578.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_696e646578.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_696e646578.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(Value::number((v_696e646578.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_7374617274.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_656e64.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f72745374617274.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_76696577706f7274456e64.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_656e64.clone(); let relational_right = v_7374617274.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let relational_left = v_76696577706f7274456e64.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":49:57
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":52:7
                if ({ let left = { let relational_left = v_7374617274.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }; if left.truthy() { { let relational_left = v_656e64.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":52:57
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":53:13
                let v_64656c7461 = if ({ let relational_left = v_7374617274.clone(); let relational_right = v_76696577706f72745374617274.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { Value::number((v_7374617274.clone()).scalar_number() - (v_76696577706f72745374617274.clone()).scalar_number()) } else { if ({ let relational_left = v_656e64.clone(); let relational_right = v_76696577706f7274456e64.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { Value::number((v_656e64.clone()).scalar_number() - (v_76696577706f7274456e64.clone()).scalar_number()) } else { Value::number(0.0) } };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":54:7
                if (Value::boolean(!strict_equal(&(v_64656c7461.clone()), &(Value::number(0.0))))).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":55:9
                    if (Value::boolean(strict_equal(&({ let value = (v_61786573.read()).borrow().clone(); value }), &(Value::string("horizontal"))))).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":55:36
                        { let v_7075695f6e61746976655f3333 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3333.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("horizontal")), ("delta", v_64656c7461.clone())])) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":56:14
                        { let v_7075695f6e61746976655f3334 = v_7363726f6c6c.read(); scroll_request(&(v_7075695f6e61746976655f3334.clone()).owner(), record(vec![("kind", Value::string("by")), ("axis", Value::string("vertical")), ("delta", v_64656c7461.clone())])) };
                    }
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":58:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":60:5
            { let v_7075695f6e61746976655f3335 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":60:44
                if ({ let left = { let value = (v_6174746163686564.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":60:71
                    { let v_7075695f6e61746976655f3336 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3336.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":61:5
            { let v_7075695f6e61746976655f3337 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":61:49
                if (v_6b6579.read().call_authored(vec![{ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true))))])).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":61:100
                    { let v_7075695f6e61746976655f3338 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3338.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":62:11
            v_73796e63.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_666f637573 = v_666f637573.clone(); let v_73636f7065 = v_73636f7065.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_61786573 = v_61786573.clone(); let v_72657665616c496e646578 = v_72657665616c496e646578.clone(); let v_72657665616c5265766973696f6e = v_72657665616c5265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f61616563316231343338313938313832: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":63:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":63:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":63:35
                let v_7469636b65745f5f32383838333030653036313561366236 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":63:59
                let v_70726f7073 = { let v_7075695f6e61746976655f3339 = v_72756e5f5f61616563316231343338313938313832.clone(); read_props(&(v_7075695f6e61746976655f3339.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":64:7
                { let value = if ({ let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("axes", false)), &(Value::string("horizontal")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("axes", false)), &(Value::string("both")))) } else { left } }).truthy() { (v_70726f7073.clone()).member("axes", false) } else { Value::string("vertical") }; *(v_61786573.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":65:7
                { let value = { let left = (v_70726f7073.clone()).member("revealIndex", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }; *(v_72657665616c496e646578.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":65:46
                { let value = { let left = (v_70726f7073.clone()).member("revealRevision", false); if left.nullish() { Value::string("") } else { left } }; *(v_72657665616c5265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":66:7
                { let v_7075695f6e61746976655f3430 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3430.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: scroll region name")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":66:67
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32383838333030653036313561366236.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":66:101
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:7
                { let v_7075695f6e61746976655f3431 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f3431.clone(), { let left = (v_70726f7073.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: scroll region description")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:86
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32383838333030653036313561366236.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":67:120
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":68:7
                { let v_7075695f6e61746976655f3432 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f3432.clone(), { let left = (v_70726f7073.clone()).member("role", false); if left.nullish() { Value::string("document") } else { left } }, Value::string("reason: scroll region role")) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":68:73
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32383838333030653036313561366236.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":68:107
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":69:7
                { let v_7075695f6e61746976655f3433 = v_666f637573.read(); focus_nav(&(v_7075695f6e61746976655f3433.clone()).owner(), if (Value::boolean(strict_equal(&((v_70726f7073.clone()).member("scope", false)), &(Value::boolean(false))))).truthy() { Value::string("none") } else { Value::string("auto") }) };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":69:75
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32383838333030653036313561366236.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":69:109
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":70:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":71:9
                    if ({ let left = Value::boolean(!strict_equal(&((v_70726f7073.clone()).member("scope", false)), &(Value::boolean(false)))); if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3434 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3434.clone()).owner()) }).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":71:57
                        { let v_7075695f6e61746976655f3435 = v_73636f7065.read(); focus_activate(&(v_7075695f6e61746976655f3435.clone()).owner(), Value::Undefined) };
                    } else {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":72:14
                        if ({ let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("scope", false)), &(Value::boolean(false)))); if left.truthy() { { let v_7075695f6e61746976655f3436 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3436.clone()).owner()) } } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/scroll-region.proto.ts":72:61
                            { let v_7075695f6e61746976655f3437 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f3437.clone()).owner(), Value::Undefined) };
                        }
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":73:9
                    if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32383838333030653036313561366236.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/scroll-region.proto.ts":73:43
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":74:9
                    { let v_7075695f6e61746976655f3438 = v_72756e5f5f61616563316231343338313938313832.clone(); request_update(&(v_7075695f6e61746976655f3438.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":77:5
            { let v_7075695f6e61746976655f3439 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":78:5
            { let v_7075695f6e61746976655f3530 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description"), Value::string("role"), Value::string("autoFocus"), Value::string("scope"), Value::string("axes"), Value::string("revealIndex"), Value::string("revealRevision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":79:5
            { let v_7075695f6e61746976655f3531 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f35616266636538363963613335626130: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:29
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:46
                v_73796e63.read().call_authored(vec![v_72756e5f5f35616266636538363963613335626130.clone()]);
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":80:63
                let v_7469636b65745f5f62396433653764316335353164633130 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":81:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f62396433653764316335353164633130.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":81:54
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":82:7
                if (({ let v_7075695f6e61746976655f3532 = v_72756e5f5f35616266636538363963613335626130.clone(); read_props(&(v_7075695f6e61746976655f3532.clone()).owner(), false) }).member("autoFocus", false)).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":82:38
                    { let v_7075695f6e61746976655f3533 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3533.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:5
            { let v_7075695f6e61746976655f3534 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_73636f7065 = v_73636f7065.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:39
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:48
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:66
                if ({ let v_7075695f6e61746976655f3535 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3535.clone()).owner()) }).truthy() {
                    // Source "frontend/proto-ui/src/scroll-region.proto.ts":84:88
                    { let v_7075695f6e61746976655f3536 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f3536.clone()).owner(), Value::Undefined) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:43
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:52
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":85:70
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":86:5
            { let v_7075695f6e61746976655f3538 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("min-h-0".into(), "min-h-".into())])]) };
            // Source "frontend/proto-ui/src/scroll-region.proto.ts":87:5
            return { let callable={ let pui_native_59={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/scroll-region.proto.ts":87:24
                return { let v_7075695f6e61746976655f3630 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_59.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_59.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> ScrollRegionHandle { ScrollRegionHandle { owner: self.owner.clone() } }
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
          self.owner.borrow_mut().dirty = false;
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
