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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("threshold").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("commands").map_or(true, |prop| prop.is_null() || prop.is_string()) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "label" => value.is_string(),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "threshold" => value.as_f64().is_some_and(f64::is_finite),
        "commands" => value.is_string(), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "begin" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "sample" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "finish" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "cancel" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "cancelPointer" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "activate" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "doubleActivate" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "wheel" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "pinch" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite)))) { value_type_error("invalid exposed method arguments") },
        "audioKey" => if !(arguments.len() <= 6 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(5).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
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
    #[serde(rename = "threshold", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "commands", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData1 {
    #[serde(rename = "kind")]
    pub field_0: String,
    #[serde(rename = "x")]
    pub field_1: f64,
    #[serde(rename = "y")]
    pub field_2: f64,
    #[serde(rename = "amount")]
    pub field_3: f64,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct EditorSurfaceHandle { pub owner: OwnerRef }
  impl EditorSurfaceHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("phase").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<String, Value> { self.state("pointer").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("lease").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("disabled").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_5(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_7(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("begin").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("begin", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_8(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("sample").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("sample", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_9(&self, argument_0: String, argument_1: f64, argument_2: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("finish").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("finish", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_10(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
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
    pub fn method_11(&self, argument_0: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("cancelPointer").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("cancelPointer", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_12(&self, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("activate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("activate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_13(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("doubleActivate").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("doubleActivate", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_14(&self, argument_0: f64, argument_1: f64, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: bool, window: &mut Window, cx: &mut App) -> Result<GeneratedData1, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("wheel").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("wheel", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !((result.is_object() && result.get("kind").map_or(false, |field| field.is_string()) && result.get("x").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("y").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)) && result.get("amount").map_or(false, |field| field.as_f64().is_some_and(f64::is_finite)))) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_15(&self, argument_0: f64, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("pinch").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("pinch", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        })
    }
    pub fn method_16(&self, argument_0: String, argument_1: bool, argument_2: bool, argument_3: bool, argument_4: bool, argument_5: String, window: &mut Window, cx: &mut App) -> Result<String, Value> {
        catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument")), Value::json(serde_json::to_value(argument_5).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("audioKey").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("audioKey", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_string()) { value_type_error("exposed method returned invalid portable data") }
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
  pub struct EditorSurface { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl EditorSurface {
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
            let v_706f696e746572: LexicalBinding<Value> = lexical_binding();
            let v_6c65617365: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_72756e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_617574686f7269747945706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_73796e63696e67: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265766973696f6e: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6f726967696e58: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6f726967696e59: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6e6578744c65617365: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_617574686f7269747943757272656e74: LexicalBinding<Value> = lexical_binding();
            let v_66696e697465: LexicalBinding<Value> = lexical_binding();
            let v_63616e63656c: LexicalBinding<Value> = lexical_binding();
            let v_6163746976617465: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            let v_726574697265: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":15:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("threshold", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("commands", record(vec![("type", Value::string("string")), ("default", Value::string("none"))]))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":22:11
            v_7068617365.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "string", Value::string("phase"), Value::string("idle"), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":23:11
            v_706f696e746572.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "string", Value::string("pointer"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":24:11
            v_6c65617365.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "discrete", Value::string("lease"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":25:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":26:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":27:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":28:11
            let v_61636365737369626c65 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":29:5
            { let v_7075695f6e61746976655f36 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f36.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":30:5
            { let v_7075695f6e61746976655f37 = v_61636365737369626c65.clone(); accessible_role(&(v_7075695f6e61746976655f37.clone()).owner(), Value::string("group")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":31:5
            { let v_7075695f6e61746976655f38 = v_61636365737369626c65.clone(); accessible_text(&(v_7075695f6e61746976655f38.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":32:5
            { let v_7075695f6e61746976655f39 = v_61636365737369626c65.clone(); accessible_state(&(v_7075695f6e61746976655f39.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":33:5
            { let v_7075695f6e61746976655f3130 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into()), ("overflow-hidden".into(), "overflow-hidden".into())])]) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":34:5
            { let v_7075695f6e61746976655f3131 = v_646566.clone(); expose_state(&owner, Value::string("phase"), v_7068617365.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":35:5
            { let v_7075695f6e61746976655f3132 = v_646566.clone(); expose_state(&owner, Value::string("pointer"), v_706f696e746572.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":36:5
            { let v_7075695f6e61746976655f3133 = v_646566.clone(); expose_state(&owner, Value::string("lease"), v_6c65617365.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":37:5
            { let v_7075695f6e61746976655f3134 = v_646566.clone(); expose_state(&owner, Value::string("disabled"), v_64697361626c6564.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":38:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":39:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":40:5
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_event(&owner, Value::string("interactionCancel")) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":41:9
            v_72756e.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":42:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":43:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":44:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":45:9
            v_617574686f7269747945706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":46:9
            v_73796e63696e67.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":47:9
            v_7265766973696f6e.initialize(Rc::new(std::cell::RefCell::new(Value::string(""))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":48:9
            v_6f726967696e58.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":49:9
            v_6f726967696e59.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":50:9
            v_6e6578744c65617365.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:11
            v_63757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f64343633393139643630666162326232: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":51:50
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f64343633393139643630666162326232.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":52:11
            v_617574686f7269747943757272656e74.initialize({  let v_72756e = v_72756e.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7469636b65745f5f37376634313633363830643164353263: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":52:59
                return { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f37376634313633363830643164353263.clone()))) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":53:11
            v_66696e697465.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_785f5f33353733343937343066643066643762: Value = arg(&arguments, 0);
                let v_795f5f66643761376363643030363132386530: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":53:55
                return { let left = number_is_finite(&(v_785f5f33353733343937343066643066643762.clone())); if left.truthy() { number_is_finite(&(v_795f5f66643761376363643030363132386530.clone())) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":54:11
            v_63616e63656c.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_72756e = v_72756e.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":55:13
                let v_6f776e65725f5f64333666666564653836383739643966 = { let v_7075695f6e61746976655f3138 = v_706f696e746572.read(); (v_7075695f6e61746976655f3138.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":56:13
                let v_726574697265645f5f61646165363831616263663062666630 = { let v_7075695f6e61746976655f3139 = v_6c65617365.read(); (v_7075695f6e61746976655f3139.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":57:13
                let v_62617369735f5f62336161643863656161373936313963 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":58:13
                let v_6163746976655f5f62323637326237643032646133333133 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3230 = v_7068617365.read(); (v_7075695f6e61746976655f3230.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":59:13
                let v_7469636b65745f5f33396331303331373137363430636465 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":60:7
                { let v_7075695f6e61746976655f3231 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3231.clone(), Value::string("idle"), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33396331303331373137363430636465.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":61:29
                    return v_6163746976655f5f62323637326237643032646133333133.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":62:7
                { let v_7075695f6e61746976655f3232 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3232.clone(), Value::string(""), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":63:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33396331303331373137363430636465.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":63:29
                    return v_6163746976655f5f62323637326237643032646133333133.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":64:7
                { let v_7075695f6e61746976655f3233 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3233.clone(), Value::number(0.0), Value::string("reason: editing contact retired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":65:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33396331303331373137363430636465.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":65:29
                    return v_6163746976655f5f62323637326237643032646133333133.clone();
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":66:7
                if ({ let left = v_6163746976655f5f62323637326237643032646133333133.clone(); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":66:26
                    { let v_7075695f6e61746976655f3234 = { let value = (v_72756e.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3234.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f64333666666564653836383739643966.clone()), ("revision", v_62617369735f5f62336161643863656161373936313963.clone()), ("lease", v_726574697265645f5f61646165363831616263663062666630.clone())])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:7
                if ({ let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33396331303331373137363430636465.clone()]); if left.truthy() { { let value = (v_72756e.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":67:35
                    { let v_7075695f6e61746976655f3235 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3235.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":68:7
                return v_6163746976655f5f62323637326237643032646133333133.clone();
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":70:5
            { let v_7075695f6e61746976655f3236 = v_646566.clone(); expose_method(&owner, Value::string("begin"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_6e6578744c65617365 = v_6e6578744c65617365.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f31373864353639363862663235353731: Value = arg(&arguments, 0);
                let v_785f5f34393364396438643135353830356364: Value = arg(&arguments, 1);
                let v_795f5f39643263323164346636636639626430: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":71:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3237 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3237.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3238 = v_7068617365.read(); (v_7075695f6e61746976655f3238.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6f776e65725f5f31373864353639363862663235353731.clone()), &(Value::string("")))) } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f34393364396438643135353830356364.clone(), v_795f5f39643263323164346636636639626430.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":72:43
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":73:13
                let v_7469636b65745f5f62313435346532313035363366313436 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":74:7
                { let value = v_785f5f34393364396438643135353830356364.clone(); *(v_6f726967696e58.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":75:7
                { let value = v_795f5f39643263323164346636636639626430.clone(); *(v_6f726967696e59.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":76:7
                { let value = add({ let value = (v_6e6578744c65617365.read()).borrow().clone(); value }, Value::number(1.0)); *(v_6e6578744c65617365.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":77:7
                { let v_7075695f6e61746976655f3239 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3239.clone(), { let value = (v_6e6578744c65617365.read()).borrow().clone(); value }, Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":78:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62313435346532313035363366313436.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":78:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":79:7
                { let v_7075695f6e61746976655f3330 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3330.clone(), v_6f776e65725f5f31373864353639363862663235353731.clone(), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":80:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62313435346532313035363366313436.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":80:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":81:7
                { let v_7075695f6e61746976655f3331 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3331.clone(), Value::string("pressed"), Value::string("reason: editing contact acquired")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":82:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62313435346532313035363366313436.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":82:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":83:7
                { let v_7075695f6e61746976655f3332 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3332.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":84:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62313435346532313035363366313436.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":84:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":85:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":85:16
                    { let v_7075695f6e61746976655f3333 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3333.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":86:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f62313435346532313035363366313436.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":88:11
            let v_73616d706c65 = {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_6f726967696e58 = v_6f726967696e58.clone(); let v_6f726967696e59 = v_6f726967696e59.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f36383461616638313365623762656534: Value = arg(&arguments, 0);
                let v_785f5f30356561323666653262616363303365: Value = arg(&arguments, 1);
                let v_795f5f66333835663661663261376237333330: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3334 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3334.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3335 = v_7068617365.read(); (v_7075695f6e61746976655f3335.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3336 = v_706f696e746572.read(); (v_7075695f6e61746976655f3336.clone()).get() }), &(v_6f776e65725f5f36383461616638313365623762656534.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":89:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f30356561323666653262616363303365.clone(), v_795f5f66333835663661663261376237333330.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":90:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":91:13
                let v_7469636b65745f5f36333261633165396437306137383662 = { let value = (v_65706f6368.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":92:13
                let v_726177 = { let left = ({ let v_7075695f6e61746976655f3337 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3337.clone()).owner(), false) }).member("threshold", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":93:13
                let v_7468726573686f6c64 = if ({ let left = { let relational_left = v_726177.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }; if left.truthy() { number_is_finite(&(v_726177.clone())) } else { left } }).truthy() { v_726177.clone() } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":94:13
                let v_64785f5f64646430346564623239326262386237 = Value::number((v_785f5f30356561323666653262616363303365.clone()).scalar_number() - ({ let value = (v_6f726967696e58.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":95:13
                let v_64795f5f39653833643938643135646664356636 = Value::number((v_795f5f66333835663661663261376237333330.clone()).scalar_number() - ({ let value = (v_6f726967696e59.read()).borrow().clone(); value }).scalar_number());
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3338 = v_7068617365.read(); (v_7075695f6e61746976655f3338.clone()).get() }), &(Value::string("pressed")))); if left.truthy() { { let relational_left = add(Value::number((v_64785f5f64646430346564623239326262386237.clone()).scalar_number() * (v_64785f5f64646430346564623239326262386237.clone()).scalar_number()), Value::number((v_64795f5f39653833643938643135646664356636.clone()).scalar_number() * (v_64795f5f39653833643938643135646664356636.clone()).scalar_number())); let relational_right = Value::number((v_7468726573686f6c64.clone()).scalar_number() * (v_7468726573686f6c64.clone()).scalar_number()); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":96:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":97:7
                { let v_7075695f6e61746976655f3339 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3339.clone(), Value::string("dragging"), Value::string("reason: editing movement accepted")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36333261633165396437306137383662.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":98:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":99:16
                    { let v_7075695f6e61746976655f3430 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3430.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":100:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f36333261633165396437306137383662.clone()]);
            })) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":102:5
            { let v_7075695f6e61746976655f3431 = v_646566.clone(); expose_method(&owner, Value::string("sample"), v_73616d706c65.clone()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":103:5
            { let v_7075695f6e61746976655f3432 = v_646566.clone(); expose_method(&owner, Value::string("finish"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_66696e697465 = v_66696e697465.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f65386236616634396637383630613164: Value = arg(&arguments, 0);
                let v_785f5f35373961333462323030346332353634: Value = arg(&arguments, 1);
                let v_795f5f36393130393765343030613035313366: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":104:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3433 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3433.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3434 = v_7068617365.read(); (v_7075695f6e61746976655f3434.clone()).get() }), &(Value::string("idle")))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3435 = v_706f696e746572.read(); (v_7075695f6e61746976655f3435.clone()).get() }), &(v_6f776e65725f5f65386236616634396637383630613164.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":104:124
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:7
                if (Value::boolean(!(v_66696e697465.read().call_authored(vec![v_785f5f35373961333462323030346332353634.clone(), v_795f5f36393130393765343030613035313366.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:28
                    v_63616e63656c.read().call_authored(vec![]);
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":105:38
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":106:13
                let v_7469636b65745f5f31383761306233383538633361373261 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":107:7
                { let v_7075695f6e61746976655f3436 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3436.clone(), Value::string("idle"), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":108:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31383761306233383538633361373261.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":108:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":109:7
                { let v_7075695f6e61746976655f3437 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3437.clone(), Value::string(""), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":110:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31383761306233383538633361373261.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":110:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":111:7
                { let v_7075695f6e61746976655f3438 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3438.clone(), Value::number(0.0), Value::string("reason: editing contact completed")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:7
                if (Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31383761306233383538633361373261.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":112:29
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:7
                if ({ let value = (v_72756e.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":113:16
                    { let v_7075695f6e61746976655f3439 = { let value = (v_72756e.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3439.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":114:7
                return v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f31383761306233383538633361373261.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":116:5
            { let v_7075695f6e61746976655f3530 = v_646566.clone(); expose_method(&owner, Value::string("cancel"), v_63616e63656c.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":117:5
            { let v_7075695f6e61746976655f3531 = v_646566.clone(); expose_method(&owner, Value::string("cancelPointer"), {  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f776e65725f5f30613565383533363962613664643632: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":118:7
                if ({ let left = Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3532 = v_7068617365.read(); (v_7075695f6e61746976655f3532.clone()).get() }), &(Value::string("idle")))); if !left.truthy() { Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3533 = v_706f696e746572.read(); (v_7075695f6e61746976655f3533.clone()).get() }), &(v_6f776e65725f5f30613565383533363962613664643632.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":118:62
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":119:7
                return v_63616e63656c.read().call_authored(vec![]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":121:11
            v_6163746976617465.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":122:7
                if ({ let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3534 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3534.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":122:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":123:13
                let v_7469636b65745f5f66616164663964353862373061353564 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":124:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:7
                if ({ let left = { let left = { let left = Value::boolean(!(v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66616164663964353862373061353564.clone()])).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3535 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3535.clone()).get() } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":125:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":126:7
                { let v_7075695f6e61746976655f3536 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3536.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":127:7
                return { let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f66616164663964353862373061353564.clone()]); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3537 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3537.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":129:5
            { let v_7075695f6e61746976655f3538 = v_646566.clone(); expose_method(&owner, Value::string("activate"), v_6163746976617465.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:5
            { let v_7075695f6e61746976655f3539 = v_646566.clone(); expose_method(&owner, Value::string("doubleActivate"), {  let v_6163746976617465 = v_6163746976617465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_636c69636b73: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":130:61
                return { let left = { let left = number_is_finite(&(v_636c69636b73.clone())); if left.truthy() { { let relational_left = v_636c69636b73.clone(); let relational_right = Value::number(2.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { v_6163746976617465.read().call_authored(vec![]) } else { left } };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":133:5
            { let v_7075695f6e61746976655f3630 = v_646566.clone(); expose_method(&owner, Value::string("wheel"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_66696e697465 = v_66696e697465.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_64785f5f64386233303330653361336461393133: Value = arg(&arguments, 0);
                let v_64795f5f37346361393737666532326530316230: Value = arg(&arguments, 1);
                let v_636f6e74726f6c: Value = arg(&arguments, 2);
                let v_73686966745f5f30373861313232363638303361656630: Value = arg(&arguments, 3);
                let v_616c745f5f64386438303165643432386363336639: Value = arg(&arguments, 4);
                let v_617272616e6765: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":134:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3631 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3631.clone()).get() } } else { left } }; if !left.truthy() { Value::boolean(!(v_66696e697465.read().call_authored(vec![v_64785f5f64386233303330653361336461393133.clone(), v_64795f5f37346361393737666532326530316230.clone()])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":134:90
                    return record(vec![("kind", Value::string("none")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":135:7
                if (v_636f6e74726f6c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":135:20
                    return record(vec![("kind", Value::string("zoom-x")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", v_64795f5f37346361393737666532326530316230.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":136:7
                if ({ let left = Value::boolean(!(v_617272616e6765.clone()).truthy()); if left.truthy() { v_616c745f5f64386438303165643432386363336639.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":136:28
                    return record(vec![("kind", Value::string("zoom-y")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", v_64795f5f37346361393737666532326530316230.clone())]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:7
                if (v_73686966745f5f30373861313232363638303361656630.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":137:18
                    return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(if (Value::boolean(!strict_equal(&(v_64785f5f64386233303330653361336461393133.clone()), &(Value::number(0.0))))).truthy() { v_64785f5f64386233303330653361336461393133.clone() } else { v_64795f5f37346361393737666532326530316230.clone() }).scalar_number())), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:7
                if ({ let left = v_617272616e6765.clone(); if left.truthy() { Value::boolean(strict_equal(&(v_64785f5f64386233303330653361336461393133.clone()), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":138:32
                    return record(vec![("kind", Value::string("none")), ("x", Value::number(0.0)), ("y", Value::number(0.0)), ("amount", Value::number(0.0))]);
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":139:7
                return record(vec![("kind", Value::string("pan")), ("x", Value::number(-(v_64785f5f64386233303330653361336461393133.clone()).scalar_number())), ("y", if (v_617272616e6765.clone()).truthy() { Value::number(0.0) } else { Value::number(-(v_64795f5f37346361393737666532326530316230.clone()).scalar_number()) }), ("amount", Value::number(0.0))]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":141:5
            { let v_7075695f6e61746976655f3632 = v_646566.clone(); expose_method(&owner, Value::string("pinch"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7363616c65: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":142:7
                return { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_72756e.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3633 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3633.clone()).get() }).truthy()) } else { left } }; if left.truthy() { number_is_finite(&(v_7363616c65.clone())) } else { left } }; if left.truthy() { { let relational_left = v_7363616c65.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":146:5
            { let v_7075695f6e61746976655f3634 = v_646566.clone(); expose_method(&owner, Value::string("audioKey"), {  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6b6579: Value = arg(&arguments, 0);
                let v_73686966745f5f32386635616163373163393264333531: Value = arg(&arguments, 1);
                let v_7365636f6e64617279: Value = arg(&arguments, 2);
                let v_616c745f5f33306231633637383133623663646636: Value = arg(&arguments, 3);
                let v_726570656174: Value = arg(&arguments, 4);
                let v_62617369735f5f35633438646231656563643966633736: Value = arg(&arguments, 5);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":147:7
                if ({ let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(!({ let value = (v_72756e.read()).borrow().clone(); value }).truthy()); if !left.truthy() { { let value = (v_73796e63696e67.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { Value::boolean(!({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy()) } else { left } }; if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }; if !left.truthy() { { let v_7075695f6e61746976655f3635 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3635.clone()).get() } } else { left } }; if !left.truthy() { v_726570656174.clone() } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f35633438646231656563643966633736.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(({ let v_7075695f6e61746976655f3636 = { let value = (v_72756e.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3636.clone()).owner(), false) }).member("commands", false)), &(Value::string("audio")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":148:50
                    return Value::string("");
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":149:11
                let v_636f6d6d616e64 = Rc::new(std::cell::RefCell::new(Value::string("")));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":150:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Backspace")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":150:68
                    { let value = if (v_73686966745f5f32386635616163373163393264333531.clone()).truthy() { Value::string("ripple-delete") } else { Value::string("delete") }; *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":151:12
                    if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("i")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("I")))) } else { left } }; if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":151:59
                        { let value = Value::string("split"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":152:12
                        if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("a")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("A")))) } else { left } }; if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":152:59
                            { let value = Value::string("select-all"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:12
                            if ({ let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("j")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("J")))) } else { left } }; if left.truthy() { v_7365636f6e64617279.clone() } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":153:59
                                { let value = Value::string("join"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/editor-surface.proto.ts":154:12
                                if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("r")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("R")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!(v_616c745f5f33306231633637383133623663646636.clone()).truthy()) } else { left } }).truthy() {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":154:68
                                    { let value = Value::string("range-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":155:12
                                    if ({ let left = { let left = { let left = Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("v")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("V")))) } else { left } }; if left.truthy() { Value::boolean(!(v_7365636f6e64617279.clone()).truthy()) } else { left } }; if left.truthy() { Value::boolean(!(v_616c745f5f33306231633637383133623663646636.clone()).truthy()) } else { left } }).truthy() {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":155:68
                                        { let value = Value::string("select-tool"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":156:12
                                        if (Value::boolean(strict_equal(&(v_6b6579.clone()), &(Value::string("Escape"))))).truthy() {
                                            // Source "frontend/proto-ui/src/editor-surface.proto.ts":156:34
                                            { let value = Value::string("escape"); *(v_636f6d6d616e64.clone()).borrow_mut() = value; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:7
                if (Value::boolean(strict_equal(&({ let value = (v_636f6d6d616e64.clone()).borrow().clone(); value }), &(Value::string(""))))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":157:27
                    return Value::string("");
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":158:13
                let v_7469636b65745f5f33643039383866643934643334363337 = add({ let value = (v_65706f6368.read()).borrow().clone(); value }, Value::number(1.0));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":159:7
                v_63616e63656c.read().call_authored(vec![]);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":160:7
                return if ({ let left = { let left = { let left = v_63757272656e74.read().call_authored(vec![v_7469636b65745f5f33643039383866643934643334363337.clone()]); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_73796e63696e67.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3637 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3637.clone()).get() }).truthy()) } else { left } }).truthy() { { let value = (v_636f6d6d616e64.clone()).borrow().clone(); value } } else { Value::string("") };
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":162:5
            { let v_7075695f6e61746976655f3638 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_7068617365 = v_7068617365.clone(); let v_63616e63656c = v_63616e63656c.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:7
                if ({ let left = Value::boolean(!strict_equal(&((v_6576656e74.clone()).member("key", false)), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3639 = v_7068617365.read(); (v_7075695f6e61746976655f3639.clone()).get() }), &(Value::string("idle")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":163:61
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":164:7
                { let v_7075695f6e61746976655f3730 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3730.clone()) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":165:7
                v_63616e63656c.read().call_authored(vec![]);
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":167:5
            { let v_7075695f6e61746976655f3731 = v_646566.clone(); listen(&owner, Value::string("host:blur"), false, v_63616e63656c.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":168:11
            v_73796e63.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_666f637573 = v_666f637573.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); let v_617574686f7269747943757272656e74 = v_617574686f7269747943757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f33646139353637303665313962313535: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":169:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":170:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":171:13
                let v_7469636b65745f5f32663165323566373234376137643836 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":172:7
                { let value = Value::boolean(true); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":173:7
                { let value = v_68616e646c655f5f33646139353637303665313962313535.clone(); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":174:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3732 = v_68616e646c655f5f33646139353637303665313962313535.clone(); read_props(&(v_7075695f6e61746976655f3732.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":175:13
                let v_6e6578745265766973696f6e = { let left = (v_70726f7073.clone()).member("revision", false); if left.nullish() { Value::string("") } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":176:13
                let v_6f776e65725f5f32623530333530616137333939353937 = { let v_7075695f6e61746976655f3733 = v_706f696e746572.read(); (v_7075695f6e61746976655f3733.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":177:13
                let v_726574697265645f5f62326430323763316330346666303137 = { let v_7075695f6e61746976655f3734 = v_6c65617365.read(); (v_7075695f6e61746976655f3734.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":178:13
                let v_62617369735f5f38356164656530366239396234333437 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":179:13
                let v_73686f756c6443616e63656c = { let left = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3735 = v_7068617365.read(); (v_7075695f6e61746976655f3735.clone()).get() }), &(Value::string("idle")))); if left.truthy() { { let left = Value::boolean(!strict_equal(&(v_6e6578745265766973696f6e.clone()), &({ let value = (v_7265766973696f6e.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("disabled", false)), &(Value::boolean(true)))) } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":180:7
                { let value = v_6e6578745265766973696f6e.clone(); *(v_7265766973696f6e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":181:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":182:9
                    { let v_7075695f6e61746976655f3736 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3736.clone(), Value::string("idle"), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":183:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":184:9
                    { let v_7075695f6e61746976655f3737 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3737.clone(), Value::string(""), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":185:40
                        return Value::Undefined;
                    }
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":186:9
                    { let v_7075695f6e61746976655f3738 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3738.clone(), Value::number(0.0), Value::string("reason: editing authority changed")) };
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:9
                    if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                        // Source "frontend/proto-ui/src/editor-surface.proto.ts":187:40
                        return Value::Undefined;
                    }
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":189:7
                { let v_7075695f6e61746976655f3739 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3739.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: editing policy")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":190:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":191:7
                { let v_7075695f6e61746976655f3830 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3830.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: editing surface label")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":192:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":193:7
                { let v_7075695f6e61746976655f3831 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3831.clone()).owner(), { let v_7075695f6e61746976655f3832 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3832.clone()).get() }) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":194:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":194:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:7
                if (v_73686f756c6443616e63656c.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":195:25
                    { let v_7075695f6e61746976655f3833 = v_68616e646c655f5f33646139353637303665313962313535.clone(); emit(&(v_7075695f6e61746976655f3833.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f32623530333530616137333939353937.clone()), ("revision", v_62617369735f5f38356164656530366239396234333437.clone()), ("lease", v_726574697265645f5f62326430323763316330346666303137.clone())])) };
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:7
                if (Value::boolean(!(v_617574686f7269747943757272656e74.read().call_authored(vec![v_7469636b65745f5f32663165323566373234376137643836.clone()])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":196:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":197:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":198:21
                    { let v_7075695f6e61746976655f3834 = v_68616e646c655f5f33646139353637303665313962313535.clone(); request_update(&(v_7075695f6e61746976655f3834.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":200:11
            v_726574697265.initialize({  let v_7068617365 = v_7068617365.clone(); let v_706f696e746572 = v_706f696e746572.clone(); let v_6c65617365 = v_6c65617365.clone(); let v_72756e = v_72756e.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_617574686f7269747945706f6368 = v_617574686f7269747945706f6368.clone(); let v_73796e63696e67 = v_73796e63696e67.clone(); let v_7265766973696f6e = v_7265766973696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7065726d616e656e74: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":201:13
                let v_68616e646c655f5f36343135623333373535333766313535 = { let value = (v_72756e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":202:13
                let v_6163746976655f5f30623866366135633138643665393365 = Value::boolean(!strict_equal(&({ let v_7075695f6e61746976655f3835 = v_7068617365.read(); (v_7075695f6e61746976655f3835.clone()).get() }), &(Value::string("idle"))));
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":203:13
                let v_6f776e65725f5f38646161313464373264336463343537 = { let v_7075695f6e61746976655f3836 = v_706f696e746572.read(); (v_7075695f6e61746976655f3836.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":204:13
                let v_726574697265645f5f63383765386130343638376264623161 = { let v_7075695f6e61746976655f3837 = v_6c65617365.read(); (v_7075695f6e61746976655f3837.clone()).get() };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":205:13
                let v_62617369735f5f62393965656432373639313431343361 = { let value = (v_7265766973696f6e.read()).borrow().clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":206:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":207:13
                let v_7469636b65745f5f34376138646431373632303162646534 = { let cell = v_617574686f7269747945706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":208:7
                { let value = Value::boolean(false); *(v_73796e63696e67.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":209:7
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":210:7
                { let value = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { v_7065726d616e656e74.clone() } else { left } }; *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":211:7
                { let value = Value::Json(Json::Null); *(v_72756e.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":212:7
                if (v_7065726d616e656e74.clone()).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":212:22
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":213:7
                { let v_7075695f6e61746976655f3838 = v_7068617365.read(); state_set(v_7075695f6e61746976655f3838.clone(), Value::string("idle"), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":214:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34376138646431373632303162646534.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":214:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":215:7
                { let v_7075695f6e61746976655f3839 = v_706f696e746572.read(); state_set(v_7075695f6e61746976655f3839.clone(), Value::string(""), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":216:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34376138646431373632303162646534.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":216:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":217:7
                { let v_7075695f6e61746976655f3930 = v_6c65617365.read(); state_set(v_7075695f6e61746976655f3930.clone(), Value::number(0.0), Value::string("reason: editing surface detached")) };
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:7
                if (Value::boolean(!strict_equal(&({ let value = (v_617574686f7269747945706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f34376138646431373632303162646534.clone())))).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":218:38
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:7
                if ({ let left = v_6163746976655f5f30623866366135633138643665393365.clone(); if left.truthy() { v_68616e646c655f5f36343135623333373535333766313535.clone() } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/editor-surface.proto.ts":219:29
                    { let v_7075695f6e61746976655f3931 = v_68616e646c655f5f36343135623333373535333766313535.clone(); emit(&(v_7075695f6e61746976655f3931.clone()).owner(), Value::string("interactionCancel"), record(vec![("pointer", v_6f776e65725f5f38646161313464373264336463343537.clone()), ("revision", v_62617369735f5f62393965656432373639313431343361.clone()), ("lease", v_726574697265645f5f63383765386130343638376264623161.clone())])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":221:5
            { let v_7075695f6e61746976655f3932 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:5
            { let v_7075695f6e61746976655f3933 = v_646566.clone(); register_life(&owner, "mounted", {  let v_6174746163686564 = v_6174746163686564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_68616e646c655f5f38616335373664366131336265323236: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:43
                { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":222:60
                v_73796e63.read().call_authored(vec![v_68616e646c655f5f38616335373664366131336265323236.clone()]);
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":223:5
            { let v_7075695f6e61746976655f3934 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("label"), Value::string("revision"), Value::string("disabled"), Value::string("threshold"), Value::string("commands")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":224:5
            { let v_7075695f6e61746976655f3935 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":224:37
                return v_726574697265.read().call_authored(vec![Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":225:5
            { let v_7075695f6e61746976655f3936 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_726574697265 = v_726574697265.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":225:41
                return v_726574697265.read().call_authored(vec![Value::boolean(true)]);
            })) }) };
            // Source "frontend/proto-ui/src/editor-surface.proto.ts":226:5
            return { let callable={ let pui_native_97={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/editor-surface.proto.ts":226:26
                return { let v_7075695f6e61746976655f3938 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_97.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_97.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> EditorSurfaceHandle { EditorSurfaceHandle { owner: self.owner.clone() } }
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
  impl Drop for EditorSurface { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for EditorSurface { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for EditorSurface {
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
