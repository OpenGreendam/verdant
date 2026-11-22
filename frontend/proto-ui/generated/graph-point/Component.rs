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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("x").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("y").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("minX").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxX").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("minY").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("maxY").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("hasX").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("hasY").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("xLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("yLabel").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "x" => value.as_f64().is_some_and(f64::is_finite),
        "y" => value.as_f64().is_some_and(f64::is_finite),
        "minX" => value.as_f64().is_some_and(f64::is_finite),
        "maxX" => value.as_f64().is_some_and(f64::is_finite),
        "minY" => value.as_f64().is_some_and(f64::is_finite),
        "maxY" => value.as_f64().is_some_and(f64::is_finite),
        "hasX" => value.is_boolean(),
        "hasY" => value.is_boolean(),
        "label" => value.is_string(),
        "xLabel" => value.is_string(),
        "yLabel" => value.is_string(),
        "disabled" => value.is_boolean(),
        "revision" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 2 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "openContext" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "x", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<f64>,
    #[serde(rename = "y", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<f64>,
    #[serde(rename = "minX", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "maxX", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "minY", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<f64>,
    #[serde(rename = "maxY", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<f64>,
    #[serde(rename = "hasX", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "hasY", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<bool>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_8: Option<String>,
    #[serde(rename = "xLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_9: Option<String>,
    #[serde(rename = "yLabel", default, skip_serializing_if = "Option::is_none")]
    pub field_10: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_11: Option<bool>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_12: Option<String>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct GraphPointHandle { pub owner: OwnerRef }
  impl GraphPointHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("x").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("y").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<String, Value> { self.state("axis").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_6(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_12(&self, argument_0: bool, argument_1: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("begin").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("begin", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_13(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("sample").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("sample", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_14(&self, argument_0: f64, argument_1: f64, argument_2: f64, argument_3: f64, argument_4: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("finish").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("finish", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_15(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_16(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("openContext").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("openContext", &arguments);
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
  pub struct GraphPoint { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl GraphPoint {
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
            let v_7068617365: LexicalBinding<Value> = lexical_binding();
            let v_78: LexicalBinding<Value> = lexical_binding();
            let v_79: LexicalBinding<Value> = lexical_binding();
            let v_61786973: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654e6f77: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654d696e: LexicalBinding<Value> = lexical_binding();
            let v_76616c75654d6178: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_616363657074656458: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_616363657074656459: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6c6f7758: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6869676858: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6c6f7759: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6869676859: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_68617358: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_68617359: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_626173654c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_784c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_794c6162656c: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_636c616d70: LexicalBinding<Value> = lexical_binding();
            let v_70726f6a656374696f6e: LexicalBinding<Value> = lexical_binding();
            let v_7075626c697368: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_70726f706f7365: LexicalBinding<Value> = lexical_binding();
            let v_73616d706c654174: LexicalBinding<Value> = lexical_binding();
            let v_61646a757374: LexicalBinding<Value> = lexical_binding();
            let v_73776974636841786973: LexicalBinding<Value> = lexical_binding();
            let v_636f6e74657874: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/graph-point.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("x", record(vec![("type", Value::string("number")), ("default", Value::number(0.5))])), ("y", record(vec![("type", Value::string("number")), ("default", Value::number(0.5))])), ("minX", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("maxX", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("minY", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("maxY", record(vec![("type", Value::string("number")), ("default", Value::number(1.0))])), ("hasX", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("hasY", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(true))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("xLabel", record(vec![("type", Value::string("string")), ("default", Value::string("横轴"))])), ("yLabel", record(vec![("type", Value::string("string")), ("default", Value::string("纵轴"))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))]))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":23:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":24:11
            v_78.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "range", Value::string("x"), Value::number(0.5), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":25:11
            v_79.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "range", Value::string("y"), Value::number(0.5), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":26:11
            v_61786973.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("axis"), Value::string("y"), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":27:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":28:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":29:11
            v_76616c75654e6f77.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "range", Value::string("valueNow"), Value::number(0.5), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":30:11
            v_76616c75654d696e.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "range", Value::string("valueMin"), Value::number(0.0), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":31:11
            v_76616c75654d6178.initialize({ let v_7075695f6e61746976655f39 = v_646566.clone(); create_state(&owner, "range", Value::string("valueMax"), Value::number(1.0), record(vec![("min", Value::number(0.0)), ("max", Value::number(1.0))])) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":32:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":33:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":34:5
            { let v_7075695f6e61746976655f3130 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f3130.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":35:5
            { let v_7075695f6e61746976655f3131 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3131.clone()).owner(), Value::string("slider")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":36:5
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3132.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":37:5
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("valueNow"), v_76616c75654e6f77.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":38:5
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("valueMin"), v_76616c75654d696e.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":39:5
            { let v_7075695f6e61746976655f3135 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3135.clone()).owner(), Value::string("valueMax"), v_76616c75654d6178.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":40:5
            { let v_7075695f6e61746976655f3136 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3136.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":41:5
            { let v_7075695f6e61746976655f3137 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3137.clone()).owner(), Value::string("increment"), record(vec![("event", Value::string("host:verdant-point-increment"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":42:5
            { let v_7075695f6e61746976655f3138 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3138.clone()).owner(), Value::string("decrement"), record(vec![("event", Value::string("host:verdant-point-decrement"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":43:5
            { let v_7075695f6e61746976655f3139 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3139.clone()).owner(), Value::string("setValue"), record(vec![("event", Value::string("host:verdant-point-set-value"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":46:5
            { let v_7075695f6e61746976655f3230 = v_61313179.clone(); accessible_action_spec(&(v_7075695f6e61746976655f3230.clone()).owner(), Value::string("activate"), record(vec![("event", Value::string("host:verdant-point-axis"))])) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":47:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("rounded-full".into(), "rounded".into()), ("select-none".into(), "content-selection affordance".into())])]) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:39
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_state(&owner, Value::string("x"), v_78.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":48:65
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_state(&owner, Value::string("y"), v_79.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":49:5
            { let v_7075695f6e61746976655f3235 = v_646566.clone(); expose_state(&owner, Value::string("axis"), v_61786973.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":49:37
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":50:5
            { let v_7075695f6e61746976655f3237 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":50:49
            { let v_7075695f6e61746976655f3238 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":51:5
            { let v_7075695f6e61746976655f3239 = v_646566.clone(); expose_event(&owner, Value::string("valueCommit")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":52:5
            { let v_7075695f6e61746976655f3330 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":53:5
            { let v_7075695f6e61746976655f3331 = v_646566.clone(); expose_event(&owner, Value::string("selected")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":54:5
            { let v_7075695f6e61746976655f3332 = v_646566.clone(); expose_event(&owner, Value::string("context")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":55:5
            { let v_7075695f6e61746976655f3333 = v_646566.clone(); expose_event(&owner, Value::string("exact")) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":56:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:31
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":57:53
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":58:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":58:24
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:28
            v_616363657074656458.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.5))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":59:49
            v_616363657074656459.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.5))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:9
            v_6c6f7758.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:23
            v_6869676858.initialize(Rc::new(std::cell::RefCell::new(Value::number(1.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:38
            v_6c6f7759.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":60:52
            v_6869676859.initialize(Rc::new(std::cell::RefCell::new(Value::number(1.0))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:9
            v_68617358.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(true))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:26
            v_68617359.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(true))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:43
            v_626173654c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:63
            v_784c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":61:80
            v_794c6162656c.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/graph-point.proto.ts":62:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f65333332656536633964366262333235: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":62:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f65333332656536633964366262333235.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":63:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f30326531643235343862383331626637: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":63:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f30326531643235343862383331626637.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":64:11
            v_636c616d70.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76: Value = arg(&arguments, 0);
                let v_6c6f77: Value = arg(&arguments, 1);
                let v_68696768: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":65:7
                if ({ let left = Value::boolean(!(number_is_finite(&(v_76.clone()))).truthy()); if !left.truthy() { { let relational_left = v_76.clone(); let relational_right = v_6c6f77.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":65:43
                    return v_6c6f77.clone();
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":66:7
                if ({ let relational_left = v_76.clone(); let relational_right = v_68696768.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":66:21
                    return v_68696768.clone();
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":67:7
                return v_76.clone();
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":69:11
            v_70726f6a656374696f6e.initialize({  let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_61786973 = v_61786973.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_76616c75654e6f77 = v_76616c75654e6f77.clone(); let v_76616c75654d696e = v_76616c75654d696e.clone(); let v_76616c75654d6178 = v_76616c75654d6178.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_626173654c6162656c = v_626173654c6162656c.clone(); let v_784c6162656c = v_784c6162656c.clone(); let v_794c6162656c = v_794c6162656c.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f39613664623137653666373731363463: LexicalBinding<Value> = lexical_binding();
                let v_617574686f72697479: LexicalBinding<Value> = lexical_binding();
                v_7469636b65745f5f39613664623137653666373731363463.initialize(arg(&arguments, 0));
                v_617574686f72697479.initialize(arg(&arguments, 1));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":70:13
                let v_76616c6964 = {  let v_63757272656e74 = v_63757272656e74.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); let v_7469636b65745f5f39613664623137653666373731363463 = v_7469636b65745f5f39613664623137653666373731363463.clone(); let v_617574686f72697479 = v_617574686f72697479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":70:36
                    return if (v_617574686f72697479.read()).truthy() { v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f39613664623137653666373731363463.read()]) } else { v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f39613664623137653666373731363463.read()]) };
                })) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":71:7
                if (Value::boolean(!(v_76616c6964.call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":71:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":72:13
                let v_697358 = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3334 = v_61786973.read(); (v_7075695f6e61746976655f3334.clone()).get() }), &(Value::string("x"))));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":73:7
                { let v_7075695f6e61746976655f3335 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3335.clone(), add(add(add({ let value = (v_626173654c6162656c.read()).borrow().clone(); value }, Value::string("，")), if (v_697358.clone()).truthy() { { let value = (v_784c6162656c.read()).borrow().clone(); value } } else { { let value = (v_794c6162656c.read()).borrow().clone(); value } }), Value::string("归一化值；方向键调整，空格切换坐标轴")), Value::string("reason: graph axis label")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":74:7
                if (Value::boolean(!(v_76616c6964.call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":74:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":75:7
                { let v_7075695f6e61746976655f3336 = v_76616c75654e6f77.read(); state_set(v_7075695f6e61746976655f3336.clone(), if (v_697358.clone()).truthy() { { let v_7075695f6e61746976655f3337 = v_78.read(); (v_7075695f6e61746976655f3337.clone()).get() } } else { { let v_7075695f6e61746976655f3338 = v_79.read(); (v_7075695f6e61746976655f3338.clone()).get() } }, Value::string("reason: graph axis value")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":76:7
                if (Value::boolean(!(v_76616c6964.call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":76:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":77:7
                { let v_7075695f6e61746976655f3339 = v_76616c75654d696e.read(); state_set(v_7075695f6e61746976655f3339.clone(), if (v_697358.clone()).truthy() { { let value = (v_6c6f7758.read()).borrow().clone(); value } } else { { let value = (v_6c6f7759.read()).borrow().clone(); value } }, Value::string("reason: graph axis minimum")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":78:7
                if (Value::boolean(!(v_76616c6964.call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":78:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":79:7
                { let v_7075695f6e61746976655f3430 = v_76616c75654d6178.read(); state_set(v_7075695f6e61746976655f3430.clone(), if (v_697358.clone()).truthy() { { let value = (v_6869676858.read()).borrow().clone(); value } } else { { let value = (v_6869676859.read()).borrow().clone(); value } }, Value::string("reason: graph axis maximum")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":80:7
                return v_76616c6964.call_authored(vec![]);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":82:11
            v_7075626c697368.initialize({  let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e657874585f5f38353066356236383363623837343936: Value = arg(&arguments, 0);
                let v_6e657874595f5f37396132303731326231333461663932: Value = arg(&arguments, 1);
                let v_7469636b65745f5f64616337656537363366653033333330: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":83:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64616337656537363366653033333330.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":83:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":84:7
                { let v_7075695f6e61746976655f3431 = v_78.read(); state_set(v_7075695f6e61746976655f3431.clone(), v_636c616d70.read().call_authored(vec![v_6e657874585f5f38353066356236383363623837343936.clone(), { let value = (v_6c6f7758.read()).borrow().clone(); value }, { let value = (v_6869676858.read()).borrow().clone(); value }]), Value::string("reason: graph horizontal preview")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":85:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64616337656537363366653033333330.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":85:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":86:7
                { let v_7075695f6e61746976655f3432 = v_79.read(); state_set(v_7075695f6e61746976655f3432.clone(), v_636c616d70.read().call_authored(vec![v_6e657874595f5f37396132303731326231333461663932.clone(), { let value = (v_6c6f7759.read()).borrow().clone(); value }, { let value = (v_6869676859.read()).borrow().clone(); value }]), Value::string("reason: graph vertical preview")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":87:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64616337656537363366653033333330.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":87:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":88:7
                if (Value::boolean(!(v_70726f6a656374696f6e.read().call_authored(vec![v_7469636b65745f5f64616337656537363366653033333330.clone(), Value::boolean(false)])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":88:39
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":89:7
                if ({ let left = { let value = (v_72756e.read()).borrow().clone(); value }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":89:28
                    { let v_7075695f6e61746976655f3433 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3433.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":90:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64616337656537363366653033333330.clone()]);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":92:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:13
                let v_68616e646c655f5f35316535346639386539306362633834 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:33
                let v_7469636b65745f5f64643534393136613163326361633234 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:57
                let v_6163746976655f5f63383862373135386338353065346635 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3434 = v_7068617365.read(); (v_7075695f6e61746976655f3434.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":93:96
                let v_62617369735f5f65333231376239656639393033383039 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":94:7
                if (Value::boolean(!(v_68616e646c655f5f35316535346639386539306362633834.clone()).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":94:20
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":95:7
                { let v_7075695f6e61746976655f3435 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3435.clone(), Value::string("idle"), Value::string("reason: graph interaction cancelled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":96:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f64643534393136613163326361633234.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f64643534393136613163326361633234.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":96:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":97:7
                if (v_6163746976655f5f63383862373135386338353065346635.clone()).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":97:19
                    { let v_7075695f6e61746976655f3436 = v_68616e646c655f5f35316535346639386539306362633834.clone(); emit(&(v_7075695f6e61746976655f3436.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f65333231376239656639393033383039.clone())])) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":98:7
                return v_6163746976655f5f63383862373135386338353065346635.clone();
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":100:11
            v_70726f706f7365.initialize({  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_7075626c697368 = v_7075626c697368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e657874585f5f35663131336333386262623937303635: Value = arg(&arguments, 0);
                let v_6e657874595f5f34616631633833323036343966323235: Value = arg(&arguments, 1);
                let v_72657365745f5f30326163666164373033323462616161: Value = arg(&arguments, 2);
                let v_7469636b65745f5f62626435373366343634663931653936: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":101:13
                let v_68616e646c655f5f35363332633365303633366633323631 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":102:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f35363332633365303633366633323631.clone()).truthy()); if !left.truthy() { Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62626435373366343634663931653936.clone()])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3437 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3437.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":102:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:13
                let v_70726f706f73616c58 = v_636c616d70.read().call_authored(vec![v_6e657874585f5f35663131336333386262623937303635.clone(), { let value = (v_6c6f7758.read()).borrow().clone(); value }, { let value = (v_6869676858.read()).borrow().clone(); value }]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:58
                let v_70726f706f73616c59 = v_636c616d70.read().call_authored(vec![v_6e657874595f5f34616631633833323036343966323235.clone(), { let value = (v_6c6f7759.read()).borrow().clone(); value }, { let value = (v_6869676859.read()).borrow().clone(); value }]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":103:103
                let v_62617369735f5f31363533653063616435373835376630 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":104:7
                { let v_7075695f6e61746976655f3438 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3438.clone(), Value::string("idle"), Value::string("reason: graph proposal settled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":105:7
                if ({ let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62626435373366343634663931653936.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_7075626c697368.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f62626435373366343634663931653936.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":105:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":106:7
                { let v_7075695f6e61746976655f3439 = v_68616e646c655f5f35363332633365303633366633323631.clone(); emit(&(v_7075695f6e61746976655f3439.clone()).owner(), Value::string("valueCommit"), record(vec![("x", v_70726f706f73616c58.clone()), ("y", v_70726f706f73616c59.clone()), ("reset", v_72657365745f5f30326163666164373033323462616161.clone()), ("revision", v_62617369735f5f31363533653063616435373835376630.clone())])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":107:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":109:5
            { let v_7075695f6e61746976655f3530 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72657365745f5f36633433323537306263393436313837: Value = arg(&arguments, 0);
                let v_646f75626c65: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":110:13
                let v_68616e646c655f5f37306631373339333162636531396530 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":111:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f37306631373339333162636531396530.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3531 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3531.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3532 = v_7068617365.read(); (v_7075695f6e61746976655f3532.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":111:100
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":112:13
                let v_7469636b65745f5f30333331633531306434306464373066 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":113:7
                { let v_7075695f6e61746976655f3533 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3533.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":114:7
                if ({ let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f30333331633531306434306464373066.clone()])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3534 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3534.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":114:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":115:7
                { let v_7075695f6e61746976655f3535 = v_68616e646c655f5f37306631373339333162636531396530.clone(); emit(&(v_7075695f6e61746976655f3535.clone()).owner(), Value::string("selected"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":116:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f30333331633531306434306464373066.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":116:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":117:7
                if ({ let left = v_72657365745f5f36633433323537306263393436313837.clone(); if !left.truthy() { v_646f75626c65.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":117:28
                    return v_70726f706f7365.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, Value::boolean(true), v_7469636b65745f5f30333331633531306434306464373066.clone()]);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":118:7
                { let v_7075695f6e61746976655f3536 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3536.clone(), Value::string("pressed"), Value::string("reason: graph interaction acquired")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":119:7
                return v_7075626c697368.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f30333331633531306434306464373066.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":121:11
            v_73616d706c654174.initialize({  let v_7068617365 = v_7068617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f61643062663761373730396630373164: Value = arg(&arguments, 0);
                let v_64795f5f62316539373130383130306661343733: Value = arg(&arguments, 1);
                let v_74726176656c585f5f36633564306563356265393734653166: Value = arg(&arguments, 2);
                let v_74726176656c595f5f31656638393139313732386633366330: Value = arg(&arguments, 3);
                let v_66696e655f5f66653631613432663939663866323034: Value = arg(&arguments, 4);
                let v_7469636b65745f5f65333335666338323038326165343565: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":122:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65333335666338323038326165343565.clone()])).truthy()) } else { left } }; if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3537 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3537.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3538 = v_7068617365.read(); (v_7075695f6e61746976655f3538.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":122:105
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":123:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!(number_is_finite(&(v_64785f5f61643062663761373730396630373164.clone()))).truthy()); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_64795f5f62316539373130383130306661343733.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_74726176656c585f5f36633564306563356265393734653166.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!(number_is_finite(&(v_74726176656c595f5f31656638393139313732386633366330.clone()))).truthy()) } else { left } }; if !left.truthy() { { let relational_left = v_74726176656c585f5f36633564306563356265393734653166.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }; if !left.truthy() { { let relational_left = v_74726176656c595f5f31656638393139313732386633366330.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":124:44
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":124:54
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":125:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3539 = v_7068617365.read(); (v_7075695f6e61746976655f3539.clone()).get() }), &(Value::string("pressed")))); if left.truthy() { { let relational_left = add(Value::number((v_64785f5f61643062663761373730396630373164.clone()).scalar_number() * (v_64785f5f61643062663761373730396630373164.clone()).scalar_number()), Value::number((v_64795f5f62316539373130383130306661343733.clone()).scalar_number() * (v_64795f5f62316539373130383130306661343733.clone()).scalar_number())); let relational_right = Value::number(9.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":125:63
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":126:7
                { let v_7075695f6e61746976655f3630 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3630.clone(), Value::string("dragging"), Value::string("reason: graph movement admitted")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":127:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f65333335666338323038326165343565.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":127:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":128:13
                let v_7363616c65 = if (v_66696e655f5f66653631613432663939663866323034.clone()).truthy() { Value::number(0.1) } else { Value::number(1.0) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":129:7
                return v_7075626c697368.read().call_authored(vec![if ({ let value = (v_68617358.read()).borrow().clone(); value }).truthy() { add({ let value = (v_616363657074656458.read()).borrow().clone(); value }, Value::number((Value::number((v_64785f5f61643062663761373730396630373164.clone()).scalar_number() / (v_74726176656c585f5f36633564306563356265393734653166.clone()).scalar_number())).scalar_number() * (v_7363616c65.clone()).scalar_number())) } else { { let value = (v_616363657074656458.read()).borrow().clone(); value } }, if ({ let value = (v_68617359.read()).borrow().clone(); value }).truthy() { Value::number(({ let value = (v_616363657074656459.read()).borrow().clone(); value }).scalar_number() - (Value::number((Value::number((v_64795f5f62316539373130383130306661343733.clone()).scalar_number() / (v_74726176656c595f5f31656638393139313732386633366330.clone()).scalar_number())).scalar_number() * (v_7363616c65.clone()).scalar_number())).scalar_number()) } else { { let value = (v_616363657074656459.read()).borrow().clone(); value } }, v_7469636b65745f5f65333335666338323038326165343565.clone()]);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":132:5
            { let v_7075695f6e61746976655f3631 = v_646566.clone(); expose_method(&owner, Value::string("sample"), {  let v_65706f6368 = v_65706f6368.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f31626565333434643663636134383465: Value = arg(&arguments, 0);
                let v_64795f5f35353933383361636434396233636566: Value = arg(&arguments, 1);
                let v_74726176656c585f5f30393535343037306537373834386364: Value = arg(&arguments, 2);
                let v_74726176656c595f5f62343234393066646666386164646332: Value = arg(&arguments, 3);
                let v_66696e655f5f33663937616266303832323738393861: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":133:7
                return v_73616d706c654174.read().call_authored(vec![v_64785f5f31626565333434643663636134383465.clone(), v_64795f5f35353933383361636434396233636566.clone(), v_74726176656c585f5f30393535343037306537373834386364.clone(), v_74726176656c595f5f62343234393066646666386164646332.clone(), v_66696e655f5f33663937616266303832323738393861.clone(), { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value }]);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":135:5
            { let v_7075695f6e61746976655f3632 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7075626c697368 = v_7075626c697368.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_73616d706c654174 = v_73616d706c654174.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f33303864303661313062303232353432: Value = arg(&arguments, 0);
                let v_64795f5f63313866303337356134653163633863: Value = arg(&arguments, 1);
                let v_74726176656c585f5f65343262616338663139333330666462: Value = arg(&arguments, 2);
                let v_74726176656c595f5f31323664333035333237326634383364: Value = arg(&arguments, 3);
                let v_66696e655f5f34636138626561666430346133626162: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":136:13
                let v_7469636b65745f5f33663766353463383461646332313334 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":137:7
                if ({ let left = Value::boolean(!(v_73616d706c654174.read().call_authored(vec![v_64785f5f33303864303661313062303232353432.clone(), v_64795f5f63313866303337356134653163633863.clone(), v_74726176656c585f5f65343262616338663139333330666462.clone(), v_74726176656c595f5f31323664333035333237326634383364.clone(), v_66696e655f5f34636138626561666430346133626162.clone(), v_7469636b65745f5f33663766353463383461646332313334.clone()])).truthy()); if !left.truthy() { Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33663766353463383461646332313334.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":137:82
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":138:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3633 = v_7068617365.read(); (v_7075695f6e61746976655f3633.clone()).get() }), &(Value::string("dragging"))))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":138:39
                    return v_70726f706f7365.read().call_authored(vec![{ let v_7075695f6e61746976655f3634 = v_78.read(); (v_7075695f6e61746976655f3634.clone()).get() }, { let v_7075695f6e61746976655f3635 = v_79.read(); (v_7075695f6e61746976655f3635.clone()).get() }, Value::boolean(false), v_7469636b65745f5f33663766353463383461646332313334.clone()]);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":139:7
                { let v_7075695f6e61746976655f3636 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3636.clone(), Value::string("idle"), Value::string("reason: graph click settled")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":140:7
                return v_7075626c697368.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, v_7469636b65745f5f33663766353463383461646332313334.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":142:5
            { let v_7075695f6e61746976655f3637 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":143:11
            v_61646a757374.initialize({  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7768696368: Value = arg(&arguments, 0);
                let v_616d6f756e745f5f36656230343132633832303666666530: Value = arg(&arguments, 1);
                let v_656e64706f696e74: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":144:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3638 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3638.clone()).get() } } else { left } }; if !left.truthy() { if (Value::boolean(strict_equal(&(v_7768696368.clone()), &(Value::string("x"))))).truthy() { Value::boolean(!({ let value = (v_68617358.read()).borrow().clone(); value }).truthy()) } else { Value::boolean(!({ let value = (v_68617359.read()).borrow().clone(); value }).truthy()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":144:106
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":145:13
                let v_7469636b65745f5f30666561623462613635346537656164 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":146:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":147:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f30666561623462613635346537656164.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":147:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":148:7
                { let v_7075695f6e61746976655f3639 = v_61786973.read(); state_set(v_7075695f6e61746976655f3639.clone(), v_7768696368.clone(), Value::string("reason: graph keyboard axis")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":149:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f30666561623462613635346537656164.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":149:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":150:11
                let v_6e657874585f5f34393034393834616436313863653131 = Rc::new(std::cell::RefCell::new({ let value = (v_616363657074656458.read()).borrow().clone(); value }));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":150:34
                let v_6e657874595f5f31643537333562343439386133333261 = Rc::new(std::cell::RefCell::new({ let value = (v_616363657074656459.read()).borrow().clone(); value }));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":151:7
                if (Value::boolean(strict_equal(&(v_7768696368.clone()), &(Value::string("x"))))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":151:26
                    { let value = if ({ let relational_left = v_656e64706f696e74.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { { let value = (v_6c6f7758.read()).borrow().clone(); value } } else { if ({ let relational_left = v_656e64706f696e74.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { { let value = (v_6869676858.read()).borrow().clone(); value } } else { add({ let value = (v_616363657074656458.read()).borrow().clone(); value }, v_616d6f756e745f5f36656230343132633832303666666530.clone()) } }; *(v_6e657874585f5f34393034393834616436313863653131.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":152:12
                    { let value = if ({ let relational_left = v_656e64706f696e74.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { { let value = (v_6c6f7759.read()).borrow().clone(); value } } else { if ({ let relational_left = v_656e64706f696e74.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { { let value = (v_6869676859.read()).borrow().clone(); value } } else { add({ let value = (v_616363657074656459.read()).borrow().clone(); value }, v_616d6f756e745f5f36656230343132633832303666666530.clone()) } }; *(v_6e657874595f5f31643537333562343439386133333261.clone()).borrow_mut() = value; }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":153:7
                return v_70726f706f7365.read().call_authored(vec![{ let value = (v_6e657874585f5f34393034393834616436313863653131.clone()).borrow().clone(); value }, { let value = (v_6e657874595f5f31643537333562343439386133333261.clone()).borrow().clone(); value }, Value::boolean(false), v_7469636b65745f5f30666561623462613635346537656164.clone()]);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":155:11
            v_73776974636841786973.initialize({  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":156:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3730 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3730.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_68617358.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_68617359.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":156:89
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":157:13
                let v_7469636b65745f5f33386563616130383536636330386434 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":158:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":159:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33386563616130383536636330386434.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":159:29
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":160:7
                { let v_7075695f6e61746976655f3731 = v_61786973.read(); state_set(v_7075695f6e61746976655f3731.clone(), if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3732 = v_61786973.read(); (v_7075695f6e61746976655f3732.clone()).get() }), &(Value::string("x"))))).truthy() { Value::string("y") } else { Value::string("x") }, Value::string("reason: graph assistive axis")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":161:7
                if (Value::boolean(!(v_70726f6a656374696f6e.read().call_authored(vec![v_7469636b65745f5f33386563616130383536636330386434.clone(), Value::boolean(false)])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":161:39
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":162:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":162:16
                    { let v_7075695f6e61746976655f3733 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3733.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":164:11
            v_636f6e74657874.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":165:13
                let v_68616e646c655f5f62383562316436633662666266386533 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":166:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!(v_68616e646c655f5f62383562316436633662666266386533.clone()).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3734 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3734.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":166:74
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":167:13
                let v_7469636b65745f5f31326534323735393738663635386431 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":167:33
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":168:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31326534323735393738663635386431.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":168:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":169:7
                { let v_7075695f6e61746976655f3735 = v_68616e646c655f5f62383562316436633662666266386533.clone(); emit(&(v_7075695f6e61746976655f3735.clone()).owner(), Value::string("context"), record(vec![("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":170:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":172:5
            { let v_7075695f6e61746976655f3736 = v_646566.clone(); expose_method(&owner, Value::string("openContext"), v_636f6e74657874.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":173:5
            { let v_7075695f6e61746976655f3737 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-axis"), false, v_73776974636841786973.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":174:5
            { let v_7075695f6e61746976655f3738 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-increment"), false, {  let v_61786973 = v_61786973.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":174:56
                return v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3739 = v_61786973.read(); (v_7075695f6e61746976655f3739.clone()).get() }, Value::number(0.01), Value::number(0.0)]);
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":175:5
            { let v_7075695f6e61746976655f3830 = v_646566.clone(); listen(&owner, Value::string("host:verdant-point-decrement"), false, {  let v_61786973 = v_61786973.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":175:56
                return v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3831 = v_61786973.read(); (v_7075695f6e61746976655f3831.clone()).get() }, Value::number(-(Value::number(0.01)).scalar_number()), Value::number(0.0)]);
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":176:5
            { let v_7075695f6e61746976655f3832 = v_646566.clone(); listen_numeric_value(&owner, Value::string("host:verdant-point-set-value"), false, {  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_61646a757374 = v_61646a757374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f68616e646c655f5f39326665653738363662653264303032: Value = arg(&arguments, 0);
                let v_6f7061717565: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":177:13
                let v_6576656e745f5f39353038373361366333343161373536 = v_6f7061717565.clone();
                // Source "frontend/proto-ui/src/graph-point.proto.ts":177:63
                let v_6e657874 = (v_6576656e745f5f39353038373361366333343161373536.clone()).member("value", false);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":178:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&(typeof_value(&(v_6e657874.clone()))), &(Value::string("number")))); if !left.truthy() { Value::boolean(!(number_is_finite(&(v_6e657874.clone()))).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3833 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3833.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":178:113
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":179:7
                v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3834 = v_61786973.read(); (v_7075695f6e61746976655f3834.clone()).get() }, Value::number((v_6e657874.clone()).scalar_number() - (if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3835 = v_61786973.read(); (v_7075695f6e61746976655f3835.clone()).get() }), &(Value::string("x"))))).truthy() { { let value = (v_616363657074656458.read()).borrow().clone(); value } } else { { let value = (v_616363657074656459.read()).borrow().clone(); value } }).scalar_number()), Value::number(0.0)]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":181:5
            { let v_7075695f6e61746976655f3836 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); let v_70726f706f7365 = v_70726f706f7365.clone(); let v_61646a757374 = v_61646a757374.clone(); let v_73776974636841786973 = v_73776974636841786973.clone(); let v_636f6e74657874 = v_636f6e74657874.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f68616e646c655f5f61353161313432366533313466393632: Value = arg(&arguments, 0);
                let v_6576656e745f5f32396130326466303338363232373465: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":182:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Escape"))))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:37
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:47
                    { let v_7075695f6e61746976655f3837 = v_6576656e745f5f32396130326466303338363232373465.clone(); prevent(v_7075695f6e61746976655f3837.clone()) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":182:95
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":183:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3838 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3838.clone()).get() } } else { left } }; if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("ctrlKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("metaKey", false) } else { left } }; if !left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("altKey", false) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":183:121
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":184:13
                let v_616d6f756e745f5f39613631363733393039313330373161 = if ((v_6576656e745f5f32396130326466303338363232373465.clone()).member("shiftKey", false)).truthy() { Value::number(0.001) } else { Value::number(0.01) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":185:11
                let v_6163636570746564 = Rc::new(std::cell::RefCell::new(Value::boolean(false)));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":186:7
                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowLeft"))))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":186:38
                    { let value = v_61646a757374.read().call_authored(vec![if ({ let value = (v_68617358.read()).borrow().clone(); value }).truthy() { Value::string("x") } else { Value::string("y") }, Value::number(-(v_616d6f756e745f5f39613631363733393039313330373161.clone()).scalar_number()), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":187:12
                    if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowRight"))))).truthy() {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":187:44
                        { let value = v_61646a757374.read().call_authored(vec![if ({ let value = (v_68617358.read()).borrow().clone(); value }).truthy() { Value::string("x") } else { Value::string("y") }, v_616d6f756e745f5f39613631363733393039313330373161.clone(), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":188:12
                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowUp"))))).truthy() {
                            // Source "frontend/proto-ui/src/graph-point.proto.ts":188:41
                            { let value = v_61646a757374.read().call_authored(vec![if ({ let value = (v_68617359.read()).borrow().clone(); value }).truthy() { Value::string("y") } else { Value::string("x") }, v_616d6f756e745f5f39613631363733393039313330373161.clone(), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/graph-point.proto.ts":189:12
                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ArrowDown"))))).truthy() {
                                // Source "frontend/proto-ui/src/graph-point.proto.ts":189:43
                                { let value = v_61646a757374.read().call_authored(vec![if ({ let value = (v_68617359.read()).borrow().clone(); value }).truthy() { Value::string("y") } else { Value::string("x") }, Value::number(-(v_616d6f756e745f5f39613631363733393039313330373161.clone()).scalar_number()), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/graph-point.proto.ts":190:12
                                if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Home"))))).truthy() {
                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":190:38
                                    { let value = v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3839 = v_61786973.read(); (v_7075695f6e61746976655f3839.clone()).get() }, Value::number(0.0), Value::number(-(Value::number(1.0)).scalar_number())]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":191:12
                                    if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("End"))))).truthy() {
                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":191:37
                                        { let value = v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3930 = v_61786973.read(); (v_7075695f6e61746976655f3930.clone()).get() }, Value::number(0.0), Value::number(1.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":192:12
                                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("PageUp"))))).truthy() {
                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":192:40
                                            { let value = v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3931 = v_61786973.read(); (v_7075695f6e61746976655f3931.clone()).get() }, Value::number((v_616d6f756e745f5f39613631363733393039313330373161.clone()).scalar_number() * (Value::number(10.0)).scalar_number()), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                        } else {
                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":193:12
                                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("PageDown"))))).truthy() {
                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":193:42
                                                { let value = v_61646a757374.read().call_authored(vec![{ let v_7075695f6e61746976655f3932 = v_61786973.read(); (v_7075695f6e61746976655f3932.clone()).get() }, Value::number((Value::number(-(v_616d6f756e745f5f39613631363733393039313330373161.clone()).scalar_number())).scalar_number() * (Value::number(10.0)).scalar_number()), Value::number(0.0)]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                            } else {
                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":194:12
                                                if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string(" ")))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Space")))) } else { left } }).truthy() {
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":194:62
                                                    v_73776974636841786973.read().call_authored(vec![]);
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":194:76
                                                    { let value = Value::boolean(true); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                                } else {
                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":195:12
                                                    if ({ let left = Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("ContextMenu")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("F10")))); if left.truthy() { (v_6576656e745f5f32396130326466303338363232373465.clone()).member("shiftKey", false) } else { left } } } else { left } }).truthy() {
                                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":195:88
                                                        { let value = v_636f6e74657874.read().call_authored(vec![]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                                    } else {
                                                        // Source "frontend/proto-ui/src/graph-point.proto.ts":196:12
                                                        if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Backspace"))))).truthy() {
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:51
                                                            let v_7469636b65745f5f33393030366661303466663761336330 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:71
                                                            v_63616e63656c.read().call_authored(vec![]);
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":196:81
                                                            { let value = v_70726f706f7365.read().call_authored(vec![{ let value = (v_616363657074656458.read()).borrow().clone(); value }, { let value = (v_616363657074656459.read()).borrow().clone(); value }, Value::boolean(true), v_7469636b65745f5f33393030366661303466663761336330.clone()]); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                                        } else {
                                                            // Source "frontend/proto-ui/src/graph-point.proto.ts":197:12
                                                            if (Value::boolean(strict_equal(&((v_6576656e745f5f32396130326466303338363232373465.clone()).member("key", false)), &(Value::string("Enter"))))).truthy() {
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:47
                                                                let v_68616e646c655f5f32376631623165366561616538366165 = { let value = (v_72756e.read()).borrow().clone(); value };
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:67
                                                                let v_7469636b65745f5f34313665366265366431623635396536 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:87
                                                                v_63616e63656c.read().call_authored(vec![]);
                                                                // Source "frontend/proto-ui/src/graph-point.proto.ts":197:97
                                                                if ({ let left = v_68616e646c655f5f32376631623165366561616538366165.clone(); if left.truthy() { v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f34313665366265366431623635396536.clone()]) } else { left } }).truthy() {
                                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":197:130
                                                                    { let v_7075695f6e61746976655f3933 = v_68616e646c655f5f32376631623165366561616538366165.clone(); emit(&(v_7075695f6e61746976655f3933.clone()).owner(), Value::string("exact"), record(vec![("axis", { let v_7075695f6e61746976655f3934 = v_61786973.read(); (v_7075695f6e61746976655f3934.clone()).get() }), ("revision", { let value = (v_7265766973696f6e.read()).borrow().clone(); value })])) };
                                                                    // Source "frontend/proto-ui/src/graph-point.proto.ts":197:191
                                                                    { let value = Value::boolean(true); *(v_6163636570746564.clone()).borrow_mut() = value; }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":198:7
                if ({ let value = (v_6163636570746564.clone()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":198:21
                    { let v_7075695f6e61746976655f3935 = v_6576656e745f5f32396130326466303338363232373465.clone(); prevent(v_7075695f6e61746976655f3935.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":200:5
            { let v_7075695f6e61746976655f3936 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":201:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_61786973 = v_61786973.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); let v_6c6f7758 = v_6c6f7758.clone(); let v_6869676858 = v_6869676858.clone(); let v_6c6f7759 = v_6c6f7759.clone(); let v_6869676859 = v_6869676859.clone(); let v_68617358 = v_68617358.clone(); let v_68617359 = v_68617359.clone(); let v_626173654c6162656c = v_626173654c6162656c.clone(); let v_784c6162656c = v_784c6162656c.clone(); let v_794c6162656c = v_794c6162656c.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); let v_636c616d70 = v_636c616d70.clone(); let v_70726f6a656374696f6e = v_70726f6a656374696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f61323336653831363963633263646137: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":202:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":202:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:22
                let v_7469636b65745f5f34383863353335333164346263333537 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:49
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":203:65
                { let value = v_68616e646c655f5f61323336653831363963633263646137.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":204:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3937 = v_68616e646c655f5f61323336653831363963633263646137.clone(); read_props(&(v_7075695f6e61746976655f3937.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":204:47
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":204:90
                let v_70726576696f75735265766973696f6e = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":205:13
                let v_6e657874585f5f33666236613030636635303763366666 = v_636c616d70.read().call_authored(vec![{ let left = (v_70726f7073.clone()).member("x", false); if left.nullish() { Value::number(0.5) } else { left } }, { let left = (v_70726f7073.clone()).member("minX", false); if left.nullish() { Value::number(0.0) } else { left } }, { let left = (v_70726f7073.clone()).member("maxX", false); if left.nullish() { Value::number(1.0) } else { left } }]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":206:13
                let v_6e657874595f5f37383036376538613932613437323437 = v_636c616d70.read().call_authored(vec![{ let left = (v_70726f7073.clone()).member("y", false); if left.nullish() { Value::number(0.5) } else { left } }, { let left = (v_70726f7073.clone()).member("minY", false); if left.nullish() { Value::number(0.0) } else { left } }, { let left = (v_70726f7073.clone()).member("maxY", false); if left.nullish() { Value::number(1.0) } else { left } }]);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":207:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3938 = v_7068617365.read(); (v_7075695f6e61746976655f3938.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = { let left = { let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!strict_equal(&(v_6e657874585f5f33666236613030636635303763366666.clone()), &({ let value = (v_616363657074656458.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_6e657874595f5f37383036376538613932613437323437.clone()), &({ let value = (v_616363657074656459.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":208:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":208:32
                { let value = v_6e657874585f5f33666236613030636635303763366666.clone(); *(v_616363657074656458.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":208:51
                { let value = v_6e657874595f5f37383036376538613932613437323437.clone(); *(v_616363657074656459.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:7
                { let value = { let left = (v_70726f7073.clone()).member("minX", false); if left.nullish() { Value::number(0.0) } else { left } }; *(v_6c6f7758.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:31
                { let value = { let left = (v_70726f7073.clone()).member("maxX", false); if left.nullish() { Value::number(1.0) } else { left } }; *(v_6869676858.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:56
                { let value = { let left = (v_70726f7073.clone()).member("minY", false); if left.nullish() { Value::number(0.0) } else { left } }; *(v_6c6f7759.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":209:80
                { let value = { let left = (v_70726f7073.clone()).member("maxY", false); if left.nullish() { Value::number(1.0) } else { left } }; *(v_6869676859.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:7
                { let value = { let left = (v_70726f7073.clone()).member("hasX", false); if left.nullish() { Value::boolean(true) } else { left } }; *(v_68617358.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":210:34
                { let value = { let left = (v_70726f7073.clone()).member("hasY", false); if left.nullish() { Value::boolean(true) } else { left } }; *(v_68617359.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":211:7
                { let value = { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }; *(v_626173654c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":211:38
                { let value = { let left = (v_70726f7073.clone()).member("xLabel", false); if left.nullish() { Value::string("横轴") } else { left } }; *(v_784c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":211:69
                { let value = { let left = (v_70726f7073.clone()).member("yLabel", false); if left.nullish() { Value::string("纵轴") } else { left } }; *(v_794c6162656c.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":212:7
                { let v_7075695f6e61746976655f3939 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3939.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: graph policy")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":213:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":213:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":214:7
                { let v_7075695f6e61746976655f313030 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f313030.clone()).owner(), { let v_7075695f6e61746976655f313031 = v_64697361626c6564.read(); (v_7075695f6e61746976655f313031.clone()).get() }) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":215:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":215:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":216:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":216:25
                    { let v_7075695f6e61746976655f313032 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313032.clone(), Value::string("idle"), Value::string("reason: graph authority changed")) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":217:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":217:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":218:7
                if (Value::boolean(!({ let value = (v_68617359.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":218:18
                    { let v_7075695f6e61746976655f313033 = v_61786973.read(); state_set(v_7075695f6e61746976655f313033.clone(), Value::string("x"), Value::string("reason: graph available axis")) };
                } else {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":219:12
                    if (Value::boolean(!({ let value = (v_68617358.read()).borrow().clone(); value }).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":219:23
                        { let v_7075695f6e61746976655f313034 = v_61786973.read(); state_set(v_7075695f6e61746976655f313034.clone(), Value::string("y"), Value::string("reason: graph available axis")) };
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":220:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":220:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":221:7
                if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f313035 = v_7068617365.read(); (v_7075695f6e61746976655f313035.clone()).get() }), &(Value::string("idle"))))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":222:9
                    { let v_7075695f6e61746976655f313036 = v_78.read(); state_set(v_7075695f6e61746976655f313036.clone(), { let value = (v_616363657074656458.read()).borrow().clone(); value }, Value::string("reason: graph horizontal authority")) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":223:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":223:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":224:9
                    { let v_7075695f6e61746976655f313037 = v_79.read(); state_set(v_7075695f6e61746976655f313037.clone(), { let value = (v_616363657074656459.read()).borrow().clone(); value }, Value::string("reason: graph vertical authority")) };
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":225:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/graph-point.proto.ts":225:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":227:7
                if (Value::boolean(!(v_70726f6a656374696f6e.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone(), Value::boolean(true)])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":227:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":228:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":228:25
                    { let v_7075695f6e61746976655f313038 = v_68616e646c655f5f61323336653831363963633263646137.clone(); emit(&(v_7075695f6e61746976655f313038.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_70726576696f75735265766973696f6e.clone())])) };
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":229:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f34383863353335333164346263333537.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":229:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":230:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":230:24
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":230:38
                    { let v_7075695f6e61746976655f313039 = v_68616e646c655f5f61323336653831363963633263646137.clone(); request_update(&(v_7075695f6e61746976655f313039.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":232:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_78 = v_78.clone(); let v_79 = v_79.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_616363657074656458 = v_616363657074656458.clone(); let v_616363657074656459 = v_616363657074656459.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":233:13
                let v_68616e646c655f5f34343339643365653937306535303363 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":233:33
                let v_6163746976655f5f35616365636433346433306630383638 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f313130 = v_7068617365.read(); (v_7075695f6e61746976655f313130.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/graph-point.proto.ts":233:72
                let v_62617369735f5f35343238663939653330303563323866 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:22
                let v_7469636b65745f5f32663937306530613935663139653363 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:49
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:67
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:101
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":234:113
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":235:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":235:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":236:7
                { let v_7075695f6e61746976655f313131 = v_7068617365.read(); state_set(v_7075695f6e61746976655f313131.clone(), Value::string("idle"), Value::string("reason: graph point detached")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":237:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32663937306530613935663139653363.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":237:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":238:7
                { let v_7075695f6e61746976655f313132 = v_78.read(); state_set(v_7075695f6e61746976655f313132.clone(), { let value = (v_616363657074656458.read()).borrow().clone(); value }, Value::string("reason: graph point detached horizontal")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":239:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32663937306530613935663139653363.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":239:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":240:7
                { let v_7075695f6e61746976655f313133 = v_79.read(); state_set(v_7075695f6e61746976655f313133.clone(), { let value = (v_616363657074656459.read()).borrow().clone(); value }, Value::string("reason: graph point detached vertical")) };
                // Source "frontend/proto-ui/src/graph-point.proto.ts":241:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f32663937306530613935663139653363.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":241:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":242:7
                if ({ let left = v_6163746976655f5f35616365636433346433306630383638.clone(); if left.truthy() { v_68616e646c655f5f34343339643365653937306535303363.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/graph-point.proto.ts":242:29
                    { let v_7075695f6e61746976655f313134 = v_68616e646c655f5f34343339643365653937306535303363.clone(); emit(&(v_7075695f6e61746976655f313134.clone()).owner(), Value::string("interactionCancel"), record(vec![("revision", v_62617369735f5f35343238663939653330303563323866.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/graph-point.proto.ts":244:5
            { let v_7075695f6e61746976655f313135 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":245:5
            { let v_7075695f6e61746976655f313136 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f65306166303930373066653433646431: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/graph-point.proto.ts":245:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/graph-point.proto.ts":245:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f65306166303930373066653433646431.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":246:5
            { let v_7075695f6e61746976655f313137 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("x"), Value::string("y"), Value::string("minX"), Value::string("maxX"), Value::string("minY"), Value::string("maxY"), Value::string("hasX"), Value::string("hasY"), Value::string("label"), Value::string("xLabel"), Value::string("yLabel"), Value::string("disabled"), Value::string("revision")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":247:5
            { let v_7075695f6e61746976655f313138 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":247:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":248:5
            { let v_7075695f6e61746976655f313139 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/graph-point.proto.ts":248:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/graph-point.proto.ts":249:5
            return { let callable={ let pui_native_120={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/graph-point.proto.ts":249:26
                return { let v_7075695f6e61746976655f313231 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_120.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_120.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> GraphPointHandle { GraphPointHandle { owner: self.owner.clone() } }
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
  impl Drop for GraphPoint { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for GraphPoint { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for GraphPoint {
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
