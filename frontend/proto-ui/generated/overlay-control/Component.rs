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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("description").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("role").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("open").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("defaultChoice").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("cancelChoice").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("choices").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "description" => value.is_string(),
        "role" => value.is_string(),
        "revision" => value.is_string(),
        "open" => value.is_boolean(),
        "disabled" => value.is_boolean(),
        "defaultChoice" => value.as_f64().is_some_and(f64::is_finite),
        "cancelChoice" => value.as_f64().is_some_and(f64::is_finite),
        "choices" => value.as_f64().is_some_and(f64::is_finite), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "choose" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "key" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<String>,
    #[serde(rename = "open", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<bool>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "defaultChoice", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<f64>,
    #[serde(rename = "cancelChoice", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<f64>,
    #[serde(rename = "choices", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<f64>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct OverlayControlHandle { pub owner: OwnerRef }
  impl OverlayControlHandle {
      pub fn state_0(&self) -> Result<bool, Value> { self.state("open").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<bool, Value> { self.state("scopeActive").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_5(&self, argument_0: f64, argument_1: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("choose").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("choose", &arguments);
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
        let callback = self.owner.borrow().exposes.get("key").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("key", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_7(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
  pub struct OverlayControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl OverlayControl {
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
            let v_6f70656e: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_73636f7065: LexicalBinding<Value> = lexical_binding();
            let v_6f7665726c6179: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265616479: LexicalBinding<Value> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<Value> = lexical_binding();
            let v_6b6579: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("description", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("role", record(vec![("type", Value::string("string")), ("default", Value::string("dialog"))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("open", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("defaultChoice", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("cancelChoice", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("choices", record(vec![("type", Value::string("number")), ("default", Value::number(2.0))]))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":22:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":22:56
            v_6465736372697074696f6e.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("description"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":23:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("dialog"), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":23:60
            v_6f70656e.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("open"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":24:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":24:63
            let v_6d6f64616c = { let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("modal"), Value::boolean(true), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":25:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":25:34
            { let v_7075695f6e61746976655f37 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f37.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":26:11
            v_73636f7065.initialize(native_builtin_hook(&owner, "asFocusScope", || focus_setup(&owner, "focus-scope")));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":27:5
            { let v_7075695f6e61746976655f38 = v_73636f7065.read(); focus_setup_config(&(v_7075695f6e61746976655f38.clone()).owner(), "focus-scope", record(vec![("trap", Value::boolean(true)), ("loop", Value::boolean(true)), ("navigation", Value::string("tab")), ("entry", Value::string("first")), ("emptyPolicy", Value::string("container")), ("restore", Value::string("previous"))])) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":28:11
            v_6f7665726c6179.initialize(native_builtin_hook(&owner, "asOverlay", || overlay_setup(&owner)));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":29:5
            { let v_7075695f6e61746976655f39 = v_6f7665726c6179.read(); overlay_configure(&(v_7075695f6e61746976655f39.clone()).owner(), record(vec![("defaultOpen", Value::boolean(true)), ("modal", Value::boolean(true)), ("portal", Value::boolean(false)), ("anchored", Value::boolean(false)), ("closeOnEscape", Value::boolean(false)), ("closeOnOutsidePress", Value::boolean(false)), ("closeOnFocusOutside", Value::boolean(false)), ("entry", Value::string("manual")), ("restore", Value::string("none")), ("layerRole", Value::string("dialog-content"))]), false) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":32:5
            { let v_7075695f6e61746976655f3130 = v_6f7665726c6179.read(); overlay_keep_mounted(&(v_7075695f6e61746976655f3130.clone()).owner()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":33:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":33:34
            { let v_7075695f6e61746976655f3131 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3131.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":33:51
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3132.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":33:69
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3133.clone()).owner(), "description", v_6465736372697074696f6e.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":34:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("modal"), v_6d6f64616c.clone(), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":34:33
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_state(&owner, Value::string("open"), v_6f70656e.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":35:37
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("scopeActive"), (v_73636f7065.read()).member("active", false)) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":36:52
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_event(&owner, Value::string("response")) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":38:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":39:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":39:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":39:53
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":40:11
            v_7265616479.initialize({  let v_6f70656e = v_6f70656e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":40:34
                return { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let v_7075695f6e61746976655f3231 = v_6f70656e.read(); (v_7075695f6e61746976655f3231.clone()).get() } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3232 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3232.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":41:11
            v_63686f6f7365.initialize({  let v_72756e = v_72756e.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e646578: Value = arg(&arguments, 0);
                let v_6261736973: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":42:7
                if ({ let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":42:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":43:13
                let v_6f776e65725f5f31313364326362633862363663326164 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":44:13
                let v_70726f70735f5f34343864666632623264333631653633 = { let v_7075695f6e61746976655f3233 = v_6f776e65725f5f31313364326362633862363663326164.clone(); read_props(&(v_7075695f6e61746976655f3233.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":45:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&(v_6261736973.clone()), &({ let left = (v_70726f70735f5f34343864666632623264333631653633.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }))); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_696e646578.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(Value::number((v_696e646578.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if !left.truthy() { { let relational_left = v_696e646578.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let relational_left = v_696e646578.clone(); let relational_right = { let left = (v_70726f70735f5f34343864666632623264333631653633.clone()).member("choices", false); if left.nullish() { Value::number(2.0) } else { left } }; Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":45:137
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":46:7
                { let v_7075695f6e61746976655f3234 = v_6f776e65725f5f31313364326362633862363663326164.clone(); emit(&(v_7075695f6e61746976655f3234.clone()).owner(), Value::string("response"), record(vec![("index", v_696e646578.clone()), ("revision", v_6261736973.clone())])) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":47:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":49:11
            v_6b6579.initialize({  let v_72756e = v_72756e.clone(); let v_7265616479 = v_7265616479.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d65: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":50:7
                if ({ let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":50:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":51:13
                let v_70726f70735f5f63326361373438316134356463633239 = { let v_7075695f6e61746976655f3235 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3235.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":52:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Escape"))))).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":52:30
                    return v_63686f6f7365.read().call_authored(vec![{ let left = (v_70726f70735f5f63326361373438316134356463633239.clone()).member("cancelChoice", false); if left.nullish() { Value::number(0.0) } else { left } }, { let left = (v_70726f70735f5f63326361373438316134356463633239.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }]);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":53:7
                if (Value::boolean(strict_equal(&(v_6e616d65.clone()), &(Value::string("Enter"))))).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":53:29
                    return v_63686f6f7365.read().call_authored(vec![{ let left = (v_70726f70735f5f63326361373438316134356463633239.clone()).member("defaultChoice", false); if left.nullish() { Value::number(1.0) } else { left } }, { let left = (v_70726f70735f5f63326361373438316134356463633239.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } }]);
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":54:7
                return Value::boolean(false);
            })) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":56:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_method(&owner, Value::string("choose"), v_63686f6f7365.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":56:42
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b6579.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":57:5
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":57:44
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":57:57
                    { let v_7075695f6e61746976655f3239 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3239.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":58:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b6579 = v_6b6579.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f63757272656e74: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":59:7
                if (v_6b6579.read().call_authored(vec![{ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }])).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":59:33
                    { let v_7075695f6e61746976655f3331 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3331.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":61:11
            v_73796e63.initialize({  let v_6c6162656c = v_6c6162656c.clone(); let v_6465736372697074696f6e = v_6465736372697074696f6e.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6f70656e = v_6f70656e.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_73636f7065 = v_73636f7065.clone(); let v_6f7665726c6179 = v_6f7665726c6179.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f32643534346533373138323333326131: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":62:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":62:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":63:7
                { let value = v_6f776e65725f5f32643534346533373138323333326131.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":63:26
                let v_7469636b6574 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":63:50
                let v_70726f70735f5f39313266313337383736313037343732 = { let v_7075695f6e61746976655f3332 = v_6f776e65725f5f32643534346533373138323333326131.clone(); read_props(&(v_7075695f6e61746976655f3332.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":64:7
                { let v_7075695f6e61746976655f3333 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3333.clone(), { let left = (v_70726f70735f5f39313266313337383736313037343732.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: overlay name")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":65:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":65:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":66:7
                { let v_7075695f6e61746976655f3334 = v_6465736372697074696f6e.read(); state_set(v_7075695f6e61746976655f3334.clone(), { let left = (v_70726f70735f5f39313266313337383736313037343732.clone()).member("description", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: overlay description")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":67:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":67:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":68:7
                { let v_7075695f6e61746976655f3335 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f3335.clone(), { let left = (v_70726f70735f5f39313266313337383736313037343732.clone()).member("role", false); if left.nullish() { Value::string("dialog") } else { left } }, Value::string("reason: overlay role")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":69:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":69:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":70:7
                { let v_7075695f6e61746976655f3336 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3336.clone(), { let left = (v_70726f70735f5f39313266313337383736313037343732.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: overlay policy")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":71:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":71:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":72:7
                { let v_7075695f6e61746976655f3337 = v_6f70656e.read(); state_set(v_7075695f6e61746976655f3337.clone(), { let left = (v_70726f70735f5f39313266313337383736313037343732.clone()).member("open", false); if left.nullish() { Value::boolean(true) } else { left } }, Value::string("reason: accepted overlay visibility")) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":73:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":73:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":74:7
                { let v_7075695f6e61746976655f3338 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3338.clone()).owner(), { let left = Value::boolean(!({ let v_7075695f6e61746976655f3339 = v_6f70656e.read(); (v_7075695f6e61746976655f3339.clone()).get() }).truthy()); if !left.truthy() { { let v_7075695f6e61746976655f3430 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3430.clone()).get() } } else { left } }) };
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":75:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":75:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":76:7
                if ({ let v_7075695f6e61746976655f3431 = v_6f70656e.read(); (v_7075695f6e61746976655f3431.clone()).get() }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":76:23
                    { let v_7075695f6e61746976655f3432 = v_6f7665726c6179.read(); overlay_intent(&(v_7075695f6e61746976655f3432.clone()).owner(), "openOverlay", Value::string("controlled.sync")) };
                } else {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":76:68
                    { let v_7075695f6e61746976655f3433 = v_6f7665726c6179.read(); overlay_intent(&(v_7075695f6e61746976655f3433.clone()).owner(), "close", Value::string("controlled.sync")) };
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":77:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":77:54
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":78:7
                if ({ let left = { let v_7075695f6e61746976655f3434 = v_6f70656e.read(); (v_7075695f6e61746976655f3434.clone()).get() }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3435 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3435.clone()).owner()) }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":78:44
                    { let v_7075695f6e61746976655f3436 = v_73636f7065.read(); focus_activate(&(v_7075695f6e61746976655f3436.clone()).owner(), Value::Undefined) };
                } else {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":79:12
                    if ({ let left = Value::boolean(!({ let v_7075695f6e61746976655f3437 = v_6f70656e.read(); (v_7075695f6e61746976655f3437.clone()).get() }).truthy()); if left.truthy() { { let v_7075695f6e61746976655f3438 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3438.clone()).owner()) } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/overlay-control.proto.ts":79:49
                        { let v_7075695f6e61746976655f3439 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f3439.clone()).owner(), Value::Undefined) };
                    }
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":80:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b6574.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":80:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":81:7
                { let v_7075695f6e61746976655f3530 = v_6f776e65725f5f32643534346533373138323333326131.clone(); request_update(&(v_7075695f6e61746976655f3530.clone()).owner()) };
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":83:5
            { let v_7075695f6e61746976655f3531 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:5
            { let v_7075695f6e61746976655f3532 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f36313963333462633631353162326563: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:42
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:59
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":84:76
                    v_73796e63.read().call_authored(vec![v_6f776e65725f5f36313963333462633631353162326563.clone()]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":85:5
            { let v_7075695f6e61746976655f3533 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("description"), Value::string("role"), Value::string("open"), Value::string("disabled"), Value::string("defaultChoice"), Value::string("cancelChoice"), Value::string("choices"), Value::string("revision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:5
            { let v_7075695f6e61746976655f3534 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_73636f7065 = v_73636f7065.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:39
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:48
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:66
                if ({ let v_7075695f6e61746976655f3535 = v_73636f7065.read(); focus_is_active(&(v_7075695f6e61746976655f3535.clone()).owner()) }).truthy() {
                    // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:88
                    { let v_7075695f6e61746976655f3536 = v_73636f7065.read(); focus_deactivate(&(v_7075695f6e61746976655f3536.clone()).owner(), Value::Undefined) };
                }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":86:108
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:43
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:52
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:69
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":87:87
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":88:5
            { let v_7075695f6e61746976655f3538 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/overlay-control.proto.ts":89:5
            return { let callable={ let pui_native_59={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/overlay-control.proto.ts":89:24
                return { let v_7075695f6e61746976655f3630 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_59.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_59.call)(arg(&arguments,0))})} }; match callable {
      Value::NativeFunction{render,..} => Value::Renderer(render),
      Value::Renderer(render) => Value::Renderer(render),
      Value::Function(callback) => Value::Renderer(Rc::new(move |arguments:Vec<Value>| { panic!("checked native renderer entry has no scalar callback alternative") })),
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
      pub fn handle(&self) -> OverlayControlHandle { OverlayControlHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| replace_props(&self.owner, raw))?; self.drive(window, cx) }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_window_observation_needed(&self.owner) { self._window_activation.take(); return; }
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
      fn notify_after_turn(&self, cx: &mut Context<Self>) {
          let view = cx.entity().downgrade();
          // GPUI suppresses Entity observer effects while a window is drawing,
          // including its focus/blur phase. Keep authored state synchronous,
          // then publish the host change after that physical draw lease ends.
          // A weak target cannot keep a detached component alive for delivery.
          cx.defer(move |cx| { let _ = view.update(cx, |_, cx| cx.notify()); });
      }
      pub fn drive(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          catch_authored(|| {
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); self.notify_after_turn(cx); return; }
          modules_drive(&self.owner, &self.focus, window, cx);
          if self.owner.borrow().disposed { self._entry_readiness.take(); self._window_activation.take(); self._focus_subscriptions.clear(); self._focus_lost.take(); self._global_key_capture.take(); self.notify_after_turn(cx); return; }
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          drive_queued_focus(&self.owner, window, cx);
          let dirty = std::mem::take(&mut self.owner.borrow_mut().dirty); if dirty { self.notify_after_turn(cx); }
          })
      }
  }
  impl Drop for OverlayControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for OverlayControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for OverlayControl {
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
          // Rendering projects the current source; only drive may consume its
          // pending observer signal. A parent native Render can call an exposed
          // method before this child is laid out and before deferred drive runs.
          let snapshot = self.owner.borrow().projection.clone().expect("live native projection");
          self.focus = self.focus.clone().tab_stop(native_tab_participation(&self.owner));
          let element = paint_native_root(&snapshot, &self.owner, &self.focus, format!("compiled-root-{}", Rc::as_ptr(&self.owner) as usize), drive);
          if let Some(focus) = &self.window_input { window_input_scope(element, focus, &self.owner) } else { element }
          }).unwrap_or_else(|| gpui::div().into_any_element())
      }
  }
  
//# sourceMappingURL=Component.rs.map
