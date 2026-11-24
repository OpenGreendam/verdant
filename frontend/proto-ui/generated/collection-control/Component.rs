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
  fn accepts_props(value: &Json) -> bool { value.is_object() && value.get("mode").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("label").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("cursor").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("expanded").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) && value.get("revision").map_or(true, |prop| prop.is_null() || prop.is_string()) && value.get("disabled").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("autoFocus").map_or(true, |prop| prop.is_null() || prop.is_boolean()) && value.get("columns").map_or(true, |prop| prop.is_null() || prop.as_f64().is_some_and(f64::is_finite)) }
  fn prop_valid(key: &str, value: &Json) -> bool { match key { "mode" => value.is_string(),
        "label" => value.is_string(),
        "cursor" => value.as_f64().is_some_and(f64::is_finite),
        "expanded" => value.as_f64().is_some_and(f64::is_finite),
        "revision" => value.is_string(),
        "disabled" => value.is_boolean(),
        "autoFocus" => value.is_boolean(),
        "columns" => value.as_f64().is_some_and(f64::is_finite), _ => false } }
  fn check_method(name: &str, arguments: &[Value]) { match name {
          "setItems" => if !(arguments.len() <= 1 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_array().is_some_and(|items| items.iter().all(|item| item.as_f64().is_some_and(f64::is_finite)))))) { value_type_error("invalid exposed method arguments") },
        "choose" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "selectIndex" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "keyAt" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "selectAndFocus" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.as_f64().is_some_and(f64::is_finite))) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "key" => if !(arguments.len() <= 3 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean()))) { value_type_error("invalid exposed method arguments") },
        "keyFrom" => if !(arguments.len() <= 5 && arguments.get(0).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(1).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string())) && arguments.get(2).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(3).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_boolean())) && arguments.get(4).map_or(false, |value| (value).try_data().is_some_and(|data| data.is_string()))) { value_type_error("invalid exposed method arguments") },
        "dismiss" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
        "focusSelf" => if !(arguments.len() <= 0 && true) { value_type_error("invalid exposed method arguments") },
          _ => value_error("unknown exposed method"),
  } }
  
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedData0 {
    #[serde(rename = "mode", default, skip_serializing_if = "Option::is_none")]
    pub field_0: Option<String>,
    #[serde(rename = "label", default, skip_serializing_if = "Option::is_none")]
    pub field_1: Option<String>,
    #[serde(rename = "cursor", default, skip_serializing_if = "Option::is_none")]
    pub field_2: Option<f64>,
    #[serde(rename = "expanded", default, skip_serializing_if = "Option::is_none")]
    pub field_3: Option<f64>,
    #[serde(rename = "revision", default, skip_serializing_if = "Option::is_none")]
    pub field_4: Option<String>,
    #[serde(rename = "disabled", default, skip_serializing_if = "Option::is_none")]
    pub field_5: Option<bool>,
    #[serde(rename = "autoFocus", default, skip_serializing_if = "Option::is_none")]
    pub field_6: Option<bool>,
    #[serde(rename = "columns", default, skip_serializing_if = "Option::is_none")]
    pub field_7: Option<f64>,
}
  
  pub type GeneratedProps = GeneratedData0;
  
  #[derive(Clone)]
  pub struct CollectionControlHandle { pub owner: OwnerRef }
  impl CollectionControlHandle {
      pub fn state_0(&self) -> Result<String, Value> { self.state("mode").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_1(&self) -> Result<f64, Value> { self.state("cursor").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_2(&self) -> Result<f64, Value> { self.state("expanded").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_3(&self) -> Result<bool, Value> { self.state("focused").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn state_4(&self) -> Result<bool, Value> { self.state("focusVisible").map(|value| serde_json::from_value(value).expect("checked exposed State")) }
    pub fn method_10(&self, argument_0: Vec<f64>, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("setItems").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("setItems", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_11(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("choose").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("choose", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_12(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("selectIndex").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectIndex", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_13(&self, argument_0: f64, argument_1: String, argument_2: bool, argument_3: bool, argument_4: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyAt").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyAt", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_14(&self, argument_0: f64, argument_1: bool, argument_2: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("selectAndFocus").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("selectAndFocus", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_15(&self, argument_0: String, argument_1: bool, argument_2: bool, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument"))];
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
    pub fn method_16(&self, argument_0: String, argument_1: String, argument_2: bool, argument_3: bool, argument_4: String, window: &mut Window, cx: &mut App) -> Result<bool, Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![Value::json(serde_json::to_value(argument_0).expect("portable argument")), Value::json(serde_json::to_value(argument_1).expect("portable argument")), Value::json(serde_json::to_value(argument_2).expect("portable argument")), Value::json(serde_json::to_value(argument_3).expect("portable argument")), Value::json(serde_json::to_value(argument_4).expect("portable argument"))];
        let callback = self.owner.borrow().exposes.get("keyFrom").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("keyFrom", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_boolean()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_17(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
        let outcome = catch_authored(|| {
        refresh_native_atoms(&self.owner, window, cx);
          #[cfg(feature = "experimental-focus-tree")] refresh_native_entry(&self.owner, window);
        let arguments = vec![];
        let callback = self.owner.borrow().exposes.get("dismiss").cloned().unwrap_or_else(|| value_error("unknown exposed method"));
        check_method("dismiss", &arguments);
        let result = invoke(&self.owner, "expose-method", &callback, arguments);
        let drive = self.owner.borrow().native_drive.clone(); if let Some(drive) = drive { drive(window, cx); }
        let result = if matches!(result, Value::Undefined) { Json::Null } else { result.data() };
        if !(result.is_null()) { value_type_error("exposed method returned invalid portable data") }
        serde_json::from_value(result).expect("checked exposed method result")
        });
        if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
        outcome
    }
    pub fn method_18(&self, window: &mut Window, cx: &mut App) -> Result<(), Value> {
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
  pub struct CollectionControl { pub owner: OwnerRef, focus: FocusHandle, window_input: Option<FocusHandle>, _focus_subscriptions: Vec<Subscription>, _window_activation: Option<(gpui::WindowId, Subscription)>, _focus_lost: Option<Subscription>, _global_key_capture: Option<Subscription>, _entry_readiness: Option<(gpui::WindowId, Subscription)> }
  impl CollectionControl {
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
            let v_637572736f72: LexicalBinding<Value> = lexical_binding();
            let v_657870616e646564: LexicalBinding<Value> = lexical_binding();
            let v_64697361626c6564: LexicalBinding<Value> = lexical_binding();
            let v_726f6c65: LexicalBinding<Value> = lexical_binding();
            let v_6c6162656c: LexicalBinding<Value> = lexical_binding();
            let v_7461626c65: LexicalBinding<Value> = lexical_binding();
            let v_726f7773: LexicalBinding<Value> = lexical_binding();
            let v_636f6c756d6e73: LexicalBinding<Value> = lexical_binding();
            let v_666f637573: LexicalBinding<Value> = lexical_binding();
            let v_63757272656e74: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6174746163686564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_646973706f736564: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_65706f6368: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_7265616479: LexicalBinding<Value> = lexical_binding();
            let v_6974656d73: LexicalBinding<Rc<std::cell::RefCell<Value>>> = lexical_binding();
            let v_6b696e64: LexicalBinding<Value> = lexical_binding();
            let v_636f756e74: LexicalBinding<Value> = lexical_binding();
            let v_6d696e: LexicalBinding<Value> = lexical_binding();
            let v_6d6178: LexicalBinding<Value> = lexical_binding();
            let v_66696e64: LexicalBinding<Value> = lexical_binding();
            let v_7365656b: LexicalBinding<Value> = lexical_binding();
            let v_76616c69644974656d73: LexicalBinding<Value> = lexical_binding();
            let v_656d6974: LexicalBinding<Value> = lexical_binding();
            let v_6368616e6765457870616e73696f6e: LexicalBinding<Value> = lexical_binding();
            let v_73656c656374: LexicalBinding<Value> = lexical_binding();
            let v_63686f6f7365: LexicalBinding<Value> = lexical_binding();
            let v_6b65795f5f33653136666361613234643562356135: LexicalBinding<Value> = lexical_binding();
            let v_73796e63: LexicalBinding<Value> = lexical_binding();
            // Source "frontend/proto-ui/src/collection-control.proto.ts":14:5
            { let v_7075695f6e61746976655f30 = v_646566.clone(); props_define(&owner, record(vec![("mode", record(vec![("type", Value::string("string")), ("default", Value::string("listbox"))])), ("label", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("cursor", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))])), ("expanded", record(vec![("type", Value::string("number")), ("default", Value::number(-(Value::number(1.0)).scalar_number()))])), ("revision", record(vec![("type", Value::string("string")), ("default", Value::string(""))])), ("disabled", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("autoFocus", record(vec![("type", Value::string("boolean")), ("default", Value::boolean(false))])), ("columns", record(vec![("type", Value::string("number")), ("default", Value::number(0.0))]))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":20:11
            v_637572736f72.initialize({ let v_7075695f6e61746976655f31 = v_646566.clone(); create_state(&owner, "discrete", Value::string("cursor"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":21:11
            v_657870616e646564.initialize({ let v_7075695f6e61746976655f32 = v_646566.clone(); create_state(&owner, "discrete", Value::string("expanded"), Value::number(-(Value::number(1.0)).scalar_number()), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":22:11
            v_64697361626c6564.initialize({ let v_7075695f6e61746976655f33 = v_646566.clone(); create_state(&owner, "bool", Value::string("disabled"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":23:11
            v_726f6c65.initialize({ let v_7075695f6e61746976655f34 = v_646566.clone(); create_state(&owner, "string", Value::string("role"), Value::string("listbox"), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":24:11
            v_6c6162656c.initialize({ let v_7075695f6e61746976655f35 = v_646566.clone(); create_state(&owner, "string", Value::string("label"), Value::string(""), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":25:11
            v_7461626c65.initialize({ let v_7075695f6e61746976655f36 = v_646566.clone(); create_state(&owner, "bool", Value::string("table"), Value::boolean(false), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":26:11
            v_726f7773.initialize({ let v_7075695f6e61746976655f37 = v_646566.clone(); create_state(&owner, "discrete", Value::string("rows"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":27:11
            v_636f6c756d6e73.initialize({ let v_7075695f6e61746976655f38 = v_646566.clone(); create_state(&owner, "discrete", Value::string("columns"), Value::number(0.0), Value::Undefined) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":28:11
            v_666f637573.initialize(native_builtin_hook(&owner, "asFocusable", || as_focusable(&owner)));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":28:34
            { let v_7075695f6e61746976655f39 = v_666f637573.read(); focus_setup_config(&(v_7075695f6e61746976655f39.clone()).owner(), "focus", record(vec![("disabled", Value::boolean(false))])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:11
            let v_61313179 = native_builtin_hook(&owner, "asAccessible", || as_accessible(&owner));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:34
            { let v_7075695f6e61746976655f3130 = v_61313179.clone(); accessible_role(&(v_7075695f6e61746976655f3130.clone()).owner(), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:51
            { let v_7075695f6e61746976655f3131 = v_61313179.clone(); accessible_text(&(v_7075695f6e61746976655f3131.clone()).owner(), "name", v_6c6162656c.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":29:69
            { let v_7075695f6e61746976655f3132 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3132.clone()).owner(), Value::string("disabled"), v_64697361626c6564.read(), Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:5
            { let v_7075695f6e61746976655f3133 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3133.clone()).owner(), Value::string("rowCount"), v_726f7773.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":30:52
            { let v_7075695f6e61746976655f3134 = v_61313179.clone(); accessible_state(&(v_7075695f6e61746976655f3134.clone()).owner(), Value::string("columnCount"), v_636f6c756d6e73.read(), record(vec![("when", v_7461626c65.read())])) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":31:5
            { let v_7075695f6e61746976655f3135 = v_646566.clone(); expose_state(&owner, Value::string("mode"), v_726f6c65.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:5
            { let v_7075695f6e61746976655f3136 = v_646566.clone(); expose_state(&owner, Value::string("cursor"), v_637572736f72.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":32:41
            { let v_7075695f6e61746976655f3137 = v_646566.clone(); expose_state(&owner, Value::string("expanded"), v_657870616e646564.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:5
            { let v_7075695f6e61746976655f3138 = v_646566.clone(); expose_state(&owner, Value::string("focused"), (v_666f637573.read()).member("focused", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":33:49
            { let v_7075695f6e61746976655f3139 = v_646566.clone(); expose_state(&owner, Value::string("focusVisible"), (v_666f637573.read()).member("focusVisible", false)) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":34:5
            { let v_7075695f6e61746976655f3230 = v_646566.clone(); expose_event(&owner, Value::string("select")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":35:5
            { let v_7075695f6e61746976655f3231 = v_646566.clone(); expose_event(&owner, Value::string("activate")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":36:5
            { let v_7075695f6e61746976655f3232 = v_646566.clone(); expose_event(&owner, Value::string("expand")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":37:5
            { let v_7075695f6e61746976655f3233 = v_646566.clone(); expose_event(&owner, Value::string("close")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":38:5
            { let v_7075695f6e61746976655f3234 = v_646566.clone(); expose_event(&owner, Value::string("command")) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":39:9
            v_63757272656e74.initialize(Rc::new(std::cell::RefCell::new(Value::Json(Json::Null))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":40:9
            v_6174746163686564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":41:9
            v_646973706f736564.initialize(Rc::new(std::cell::RefCell::new(Value::boolean(false))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":42:9
            v_65706f6368.initialize(Rc::new(std::cell::RefCell::new(Value::number(0.0))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":43:11
            v_7265616479.initialize({  let v_64697361626c6564 = v_64697361626c6564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":43:34
                return { let left = { let left = { let left = Value::boolean(!strict_equal(&({ let value = (v_63757272656e74.read()).borrow().clone(); value }), &(Value::Json(Json::Null)))); if left.truthy() { { let value = (v_6174746163686564.read()).borrow().clone(); value } } else { left } }; if left.truthy() { Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy()) } else { left } }; if left.truthy() { Value::boolean(!({ let v_7075695f6e61746976655f3235 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3235.clone()).get() }).truthy()) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":44:9
            v_6974656d73.initialize(Rc::new(std::cell::RefCell::new(Value::Array(Rc::new(vec![])))));
            // Source "frontend/proto-ui/src/collection-control.proto.ts":45:11
            v_6b696e64.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f33383330343633613538613831386464: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":45:45
                return if ({ let left = { let left = { let left = number_is_finite(&(v_696e6465785f5f33383330343633613538613831386464.clone())); if left.truthy() { Value::boolean(strict_equal(&(Value::number((v_696e6465785f5f33383330343633613538613831386464.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }; if left.truthy() { { let relational_left = v_696e6465785f5f33383330343633613538613831386464.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }; if left.truthy() { { let relational_left = v_696e6465785f5f33383330343633613538613831386464.clone(); let relational_right = ({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) } } else { left } }).truthy() { { let left = { let object = { let value = (v_6974656d73.read()).borrow().clone(); value }; object.caught_member(&property_key(&(v_696e6465785f5f33383330343633613538613831386464.clone())), false) }; if left.nullish() { Value::number(0.0) } else { left } } } else { Value::number(0.0) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":46:11
            v_636f756e74.initialize({  let v_6974656d73 = v_6974656d73.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":46:33
                return ({ let value = (v_6974656d73.read()).borrow().clone(); value }).member("length", false);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":47:11
            v_6d696e.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f35333736356334313532333061663562: Value = arg(&arguments, 0);
                let v_625f5f39363838376461653131663362616436: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":47:51
                return if ({ let relational_left = v_615f5f35333736356334313532333061663562.clone(); let relational_right = v_625f5f39363838376461653131663362616436.clone(); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() { v_615f5f35333736356334313532333061663562.clone() } else { v_625f5f39363838376461653131663362616436.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":48:11
            v_6d6178.initialize({   Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_615f5f65646262376536636366353837663030: Value = arg(&arguments, 0);
                let v_625f5f63633234613765333966393631643963: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":48:51
                return if ({ let relational_left = v_615f5f65646262376536636366353837663030.clone(); let relational_right = v_625f5f63633234613765333966393631643963.clone(); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_615f5f65646262376536636366353837663030.clone() } else { v_625f5f63633234613765333966393631643963.clone() };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":49:11
            v_66696e64.initialize({  let v_6b696e64 = v_6b696e64.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_73746172745f5f62306433326562386337326661303031: Value = arg(&arguments, 0);
                let v_656e645f5f62353862613033616530363838633732: Value = arg(&arguments, 1);
                let v_666f7277617264: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":50:7
                if ({ let relational_left = v_656e645f5f62353862613033616530363838633732.clone(); let relational_right = v_73746172745f5f62306433326562386337326661303031.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":50:25
                    return Value::number(-(Value::number(1.0)).scalar_number());
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":51:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f62353862613033616530363838633732.clone()).scalar_number() - (v_73746172745f5f62306433326562386337326661303031.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":51:30
                    return if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_73746172745f5f62306433326562386337326661303031.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_73746172745f5f62306433326562386337326661303031.clone() } else { Value::number(-(Value::number(1.0)).scalar_number()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":52:13
                let v_73756d5f5f33373136643437336363316630643961 = add(v_73746172745f5f62306433326562386337326661303031.clone(), v_656e645f5f62353862613033616530363838633732.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":53:13
                let v_6d6964646c655f5f31663863353466303065616564636361 = Value::number((Value::number((v_73756d5f5f33373136643437336363316630643961.clone()).scalar_number() - (Value::number((v_73756d5f5f33373136643437336363316630643961.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":54:13
                let v_63616e6469646174655f5f65623534323566613061343935386436 = if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_73746172745f5f62306433326562386337326661303031.clone(), v_6d6964646c655f5f31663863353466303065616564636361.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f31663863353466303065616564636361.clone(), v_656e645f5f62353862613033616530363838633732.clone(), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":55:7
                if ({ let relational_left = v_63616e6469646174655f5f65623534323566613061343935386436.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":55:27
                    return v_63616e6469646174655f5f65623534323566613061343935386436.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":56:7
                return if (v_666f7277617264.clone()).truthy() { v_66696e64.read().call_authored(vec![v_6d6964646c655f5f31663863353466303065616564636361.clone(), v_656e645f5f62353862613033616530363838633732.clone(), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_73746172745f5f62306433326562386337326661303031.clone(), v_6d6964646c655f5f31663863353466303065616564636361.clone(), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":58:11
            v_7365656b.initialize({  let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_66696e64 = v_66696e64.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_7265717565737465645f5f63376561623461316434636264313733: Value = arg(&arguments, 0);
                let v_646972656374696f6e: Value = arg(&arguments, 1);
                let v_5f72656d61696e696e67: Value = arg(&arguments, 2);
                let v_77726170: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":59:13
                let v_696e6465785f5f37666539343561303966363566613938 = v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), v_6d6178.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), v_7265717565737465645f5f63376561623461316434636264313733.clone()])]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":60:13
                let v_63616e6469646174655f5f39653863383131636238333037643035 = if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f37666539343561303966363566613938.clone()]), v_636f756e74.read().call_authored(vec![]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f37666539343561303966363566613938.clone(), Value::number(1.0))]), Value::boolean(false)]) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":61:7
                if ({ let left = { let relational_left = v_63616e6469646174655f5f39653863383131636238333037643035.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }; if !left.truthy() { Value::boolean(!(v_77726170.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":61:36
                    return v_63616e6469646174655f5f39653863383131636238333037643035.clone();
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":62:7
                return if ({ let relational_left = v_646972656374696f6e.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_66696e64.read().call_authored(vec![Value::number(0.0), v_6d6178.read().call_authored(vec![Value::number(0.0), v_696e6465785f5f37666539343561303966363566613938.clone()]), Value::boolean(true)]) } else { v_66696e64.read().call_authored(vec![v_6d696e.read().call_authored(vec![v_636f756e74.read().call_authored(vec![]), add(v_696e6465785f5f37666539343561303966363566613938.clone(), Value::number(1.0))]), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]) };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":64:11
            v_76616c69644974656d73.initialize({  let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_76616c756573: Value = arg(&arguments, 0);
                let v_73746172745f5f62663139373830333066376239633933: Value = arg(&arguments, 1);
                let v_656e645f5f34633332353136383534353137613231: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":65:7
                if ({ let relational_left = v_656e645f5f34633332353136383534353137613231.clone(); let relational_right = v_73746172745f5f62663139373830333066376239633933.clone(); Value::boolean(relational_left.scalar_number() <= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":65:25
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":66:7
                if (Value::boolean(strict_equal(&(Value::number((v_656e645f5f34633332353136383534353137613231.clone()).scalar_number() - (v_73746172745f5f62663139373830333066376239633933.clone()).scalar_number())), &(Value::number(1.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":66:30
                    return { let left = { let left = Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62663139373830333066376239633933.clone())), false) }), &(Value::number(0.0)))); if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62663139373830333066376239633933.clone())), false) }), &(Value::number(1.0)))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&({ let object = v_76616c756573.clone(); object.caught_member(&property_key(&(v_73746172745f5f62663139373830333066376239633933.clone())), false) }), &(Value::number(2.0)))) } else { left } };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":67:13
                let v_73756d5f5f38656230633036656565666561323233 = add(v_73746172745f5f62663139373830333066376239633933.clone(), v_656e645f5f34633332353136383534353137613231.clone());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":68:13
                let v_6d6964646c655f5f37653761336366663232346464316563 = Value::number((Value::number((v_73756d5f5f38656230633036656565666561323233.clone()).scalar_number() - (Value::number((v_73756d5f5f38656230633036656565666561323233.clone()).scalar_number() % (Value::number(2.0)).scalar_number())).scalar_number())).scalar_number() / (Value::number(2.0)).scalar_number());
                // Source "frontend/proto-ui/src/collection-control.proto.ts":69:7
                return { let left = v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_73746172745f5f62663139373830333066376239633933.clone(), v_6d6964646c655f5f37653761336366663232346464316563.clone()]); if left.truthy() { v_76616c69644974656d73.read().call_authored(vec![v_76616c756573.clone(), v_6d6964646c655f5f37653761336366663232346464316563.clone(), v_656e645f5f34633332353136383534353137613231.clone()]) } else { left } };
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":71:11
            v_656d6974.initialize({  let v_63757272656e74 = v_63757272656e74.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f62363830376563343236323666336162: Value = arg(&arguments, 0);
                let v_696e6465785f5f63393831313364353732373933313338: Value = arg(&arguments, 1);
                let v_657874656e645f5f65656466613864626430393536646663: Value = arg(&arguments, 2);
                let v_6b65795f5f33656165303463353266666136373330: Value = arg(&arguments, 3);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":72:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":72:20
                    { let v_7075695f6e61746976655f3236 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; emit(&(v_7075695f6e61746976655f3236.clone()).owner(), v_6e616d655f5f62363830376563343236323666336162.clone(), record(vec![("index", v_696e6465785f5f63393831313364353732373933313338.clone()), ("extend", v_657874656e645f5f65656466613864626430393536646663.clone()), ("key", v_6b65795f5f33656165303463353266666136373330.clone()), ("revision", { let left = ({ let v_7075695f6e61746976655f3237 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3237.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } })])) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":74:11
            v_6368616e6765457870616e73696f6e.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f30356566353935643338396461386432: Value = arg(&arguments, 0);
                let v_6e616d655f5f38313961633134336362313039376235: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":75:7
                if (Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":75:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":76:13
                let v_7469636b65745f5f35366637313833646330326533326563 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":77:7
                if ({ let left = { let relational_left = v_6e6578745f5f30356566353935643338396461386432.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if left.truthy() { { let relational_left = { let v_7075695f6e61746976655f3238 = v_657870616e646564.read(); (v_7075695f6e61746976655f3238.clone()).get() }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":78:9
                    { let v_7075695f6e61746976655f3239 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3239.clone(), { let v_7075695f6e61746976655f3330 = v_657870616e646564.read(); (v_7075695f6e61746976655f3330.clone()).get() }, Value::string("reason: closing submenu restores parent")) };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":79:9
                    if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f35366637313833646330326533326563.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":79:43
                        return Value::boolean(false);
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":81:7
                { let v_7075695f6e61746976655f3331 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3331.clone(), v_6e6578745f5f30356566353935643338396461386432.clone(), Value::string("reason: submenu expansion")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":82:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f35366637313833646330326533326563.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":82:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":83:7
                v_656d6974.read().call_authored(vec![Value::string("expand"), v_6e6578745f5f30356566353935643338396461386432.clone(), Value::boolean(false), v_6e616d655f5f38313961633134336362313039376235.clone()]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":84:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_7469636b65745f5f35366637313833646330326533326563.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":84:40
                    { let v_7075695f6e61746976655f3332 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3332.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":85:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":87:11
            v_73656c656374.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f66636662326563386334386331643335: Value = arg(&arguments, 0);
                let v_657874656e645f5f64313431663165323866663262373866: Value = arg(&arguments, 1);
                let v_6578706c69636974: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":88:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f66636662326563386334386331643335.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":88:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":91:7
                if ({ let left = { let left = Value::boolean(!(v_6578706c69636974.clone()).truthy()); if left.truthy() { Value::boolean(strict_equal(&(v_696e6465785f5f66636662326563386334386331643335.clone()), &({ let v_7075695f6e61746976655f3333 = v_637572736f72.read(); (v_7075695f6e61746976655f3333.clone()).get() }))) } else { left } }; if left.truthy() { Value::boolean(!(v_657874656e645f5f64313431663165323866663262373866.clone()).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":91:59
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":92:13
                let v_7469636b65745f5f31633732336131366165353631356664 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":93:7
                { let v_7075695f6e61746976655f3334 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3334.clone(), v_696e6465785f5f66636662326563386334386331643335.clone(), Value::string("reason: collection selection")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":94:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f31633732336131366165353631356664.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":94:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":95:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f66636662326563386334386331643335.clone(), v_657874656e645f5f64313431663165323866663262373866.clone(), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":96:7
                if ({ let left = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if left.truthy() { Value::boolean(strict_equal(&(v_7469636b65745f5f31633732336131366165353631356664.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":96:40
                    { let v_7075695f6e61746976655f3335 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; request_update(&(v_7075695f6e61746976655f3335.clone()).owner()) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":97:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":99:11
            v_63686f6f7365.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f31303933366663333430356362356538: Value = arg(&arguments, 0);
                let v_657874656e645f5f36653734353665663531393662383762: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":100:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f31303933366663333430356362356538.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":100:42
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":101:13
                let v_7469636b65745f5f31313034623130643764656138376162 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":102:7
                { let v_7075695f6e61746976655f3336 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3336.clone(), v_696e6465785f5f31303933366663333430356362356538.clone(), Value::string("reason: collection activation cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":103:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f31313034623130643764656138376162.clone()))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":103:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":104:7
                if (Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f31303933366663333430356362356538.clone()])), &(Value::number(2.0))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":105:15
                    let v_6e6578745f5f39386434313764643562303565303935 = if (Value::boolean(strict_equal(&({ let v_7075695f6e61746976655f3337 = v_657870616e646564.read(); (v_7075695f6e61746976655f3337.clone()).get() }), &(v_696e6465785f5f31303933366663333430356362356538.clone())))).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { v_696e6465785f5f31303933366663333430356362356538.clone() };
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":106:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![v_6e6578745f5f39386434313764643562303565303935.clone(), Value::string("")]);
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":107:14
                    v_656d6974.read().call_authored(vec![Value::string("activate"), v_696e6465785f5f31303933366663333430356362356538.clone(), v_657874656e645f5f36653734353665663531393662383762.clone(), Value::string("")]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":108:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":110:11
            v_6b65795f5f33653136666361613234643562356135.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); let v_656d6974 = v_656d6974.clone(); let v_6368616e6765457870616e73696f6e = v_6368616e6765457870616e73696f6e.clone(); let v_73656c656374 = v_73656c656374.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e616d655f5f66316363656631636166656366613938: Value = arg(&arguments, 0);
                let v_73686966745f5f30306631393732336363613366373633: Value = arg(&arguments, 1);
                let v_636f6d6d616e645f5f63393436336161386137383466613761: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":111:7
                if ({ let left = Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()); if !left.truthy() { Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":111:33
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":112:13
                let v_6d6f64655f5f65306330663134393432653465666439 = { let left = ({ let v_7075695f6e61746976655f3338 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3338.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":113:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("searchlist")))); if left.truthy() { v_636f6d6d616e645f5f63393436336161386137383466613761.clone() } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":114:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3339 = v_637572736f72.read(); (v_7075695f6e61746976655f3339.clone()).get() }, Value::boolean(false), Value::string("Find")]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":114:55
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":116:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("points")))); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Enter")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":116:75
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3430 = v_637572736f72.read(); (v_7075695f6e61746976655f3430.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":116:117
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":117:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("points")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Tab")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":118:15
                    let v_5f5f7075694d6f64756c6530 = add({ let v_7075695f6e61746976655f3431 = v_637572736f72.read(); (v_7075695f6e61746976655f3431.clone()).get() }, if (v_73686966745f5f30306631393732336363613366373633.clone()).truthy() { Value::number(-(Value::number(1.0)).scalar_number()) } else { Value::number(1.0) });
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":119:9
                    if ({ let left = { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }; if !left.truthy() { { let relational_left = v_5f5f7075694d6f64756c6530.clone(); let relational_right = v_636f756e74.read().call_authored(vec![]); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":119:42
                        v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3432 = v_637572736f72.read(); (v_7075695f6e61746976655f3432.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":120:14
                        v_73656c656374.read().call_authored(vec![v_5f5f7075694d6f64756c6530.clone(), Value::boolean(false), Value::boolean(true)]);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":121:9
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":123:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("points")))); if left.truthy() { { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowDown")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("+")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("=")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("-")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":123:128
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3433 = v_637572736f72.read(); (v_7075695f6e61746976655f3433.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":123:172
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":124:7
                if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Escape")))); if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Tab")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("menu")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":124:71
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3434 = v_637572736f72.read(); (v_7075695f6e61746976655f3434.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":124:113
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":125:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Enter")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string(" ")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("F2")))); if left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("table")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":125:84
                    return v_63686f6f7365.read().call_authored(vec![{ let v_7075695f6e61746976655f3435 = v_637572736f72.read(); (v_7075695f6e61746976655f3435.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":126:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("table")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Delete")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Backspace")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Tab")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":127:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3436 = v_637572736f72.read(); (v_7075695f6e61746976655f3436.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":127:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":129:7
                if ({ let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("table")))); if left.truthy() { v_636f6d6d616e645f5f63393436336161386137383466613761.clone() } else { left } }; if left.truthy() { { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowLeft")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowRight")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowDown")))) } else { left } } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":130:9
                    v_656d6974.read().call_authored(vec![Value::string("command"), { let v_7075695f6e61746976655f3437 = v_637572736f72.read(); (v_7075695f6e61746976655f3437.clone()).get() }, v_73686966745f5f30306631393732336363613366373633.clone(), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":130:53
                    return Value::boolean(true);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":132:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":133:9
                    if ({ let relational_left = { let v_7075695f6e61746976655f3438 = v_657870616e646564.read(); (v_7075695f6e61746976655f3438.clone()).get() }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() < relational_right.scalar_number()) }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":133:33
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":134:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![Value::number(-(Value::number(1.0)).scalar_number()), v_6e616d655f5f66316363656631636166656366613938.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":136:7
                if ({ let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("menu")))); if left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":137:9
                    if (Value::boolean(!strict_equal(&(v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f3439 = v_637572736f72.read(); (v_7075695f6e61746976655f3439.clone()).get() }])), &(Value::number(2.0))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":137:39
                        return Value::boolean(false);
                    }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":138:9
                    return v_6368616e6765457870616e73696f6e.read().call_authored(vec![{ let v_7075695f6e61746976655f3530 = v_637572736f72.read(); (v_7075695f6e61746976655f3530.clone()).get() }, v_6e616d655f5f66316363656631636166656366613938.clone()]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":140:11
                let v_6e6578745f5f63333064316432366164346565353263 = Rc::new(std::cell::RefCell::new(Value::number(-(Value::number(1.0)).scalar_number())));
                // Source "frontend/proto-ui/src/collection-control.proto.ts":141:7
                if (Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("Home"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":141:28
                    { let value = v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":142:12
                    if (Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("End"))))).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":142:32
                        { let value = v_7365656b.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                    } else {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":143:12
                        if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowUp")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowLeft")))) } else { left } }).truthy() {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":143:60
                            { let value = v_7365656b.read().call_authored(vec![Value::number(({ let v_7075695f6e61746976655f3531 = v_637572736f72.read(); (v_7075695f6e61746976655f3531.clone()).get() }).scalar_number() - (Value::number(1.0)).scalar_number()), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                        } else {
                            // Source "frontend/proto-ui/src/collection-control.proto.ts":144:12
                            if ({ let left = Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowDown")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("ArrowRight")))) } else { left } }).truthy() {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":144:63
                                { let value = v_7365656b.read().call_authored(vec![add({ let v_7075695f6e61746976655f3532 = v_637572736f72.read(); (v_7075695f6e61746976655f3532.clone()).get() }, Value::number(1.0)), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("menu")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("tablist")))) } else { left } }]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                            } else {
                                // Source "frontend/proto-ui/src/collection-control.proto.ts":145:12
                                if (Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("PageUp"))))).truthy() {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":145:35
                                    { let value = v_7365656b.read().call_authored(vec![v_6d6178.read().call_authored(vec![Value::number(0.0), Value::number(({ let v_7075695f6e61746976655f3533 = v_637572736f72.read(); (v_7075695f6e61746976655f3533.clone()).get() }).scalar_number() - (if (Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) }).scalar_number())]), Value::number(-(Value::number(1.0)).scalar_number()), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                                } else {
                                    // Source "frontend/proto-ui/src/collection-control.proto.ts":146:12
                                    if (Value::boolean(strict_equal(&(v_6e616d655f5f66316363656631636166656366613938.clone()), &(Value::string("PageDown"))))).truthy() {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":146:37
                                        { let value = v_7365656b.read().call_authored(vec![v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), add({ let v_7075695f6e61746976655f3534 = v_637572736f72.read(); (v_7075695f6e61746976655f3534.clone()).get() }, if (Value::boolean(strict_equal(&(v_6d6f64655f5f65306330663134393432653465666439.clone()), &(Value::string("points"))))).truthy() { Value::number(4.0) } else { Value::number(20.0) })]), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)]); *(v_6e6578745f5f63333064316432366164346565353263.clone()).borrow_mut() = value; }
                                    } else {
                                        // Source "frontend/proto-ui/src/collection-control.proto.ts":147:12
                                        return Value::boolean(false);
                                    }
                                }
                            }
                        }
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":148:7
                if ({ let relational_left = { let value = (v_6e6578745f5f63333064316432366164346565353263.clone()).borrow().clone(); value }; let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() >= relational_right.scalar_number()) }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":148:22
                    v_73656c656374.read().call_authored(vec![{ let value = (v_6e6578745f5f63333064316432366164346565353263.clone()).borrow().clone(); value }, v_73686966745f5f30306631393732336363613366373633.clone(), Value::boolean(true)]);
                } else {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":149:12
                    if ({ let left = Value::boolean(!(v_73686966745f5f30306631393732336363613366373633.clone()).truthy()); if left.truthy() { { let relational_left = v_6b696e64.read().call_authored(vec![{ let v_7075695f6e61746976655f3535 = v_637572736f72.read(); (v_7075695f6e61746976655f3535.clone()).get() }]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":149:50
                        v_73656c656374.read().call_authored(vec![{ let v_7075695f6e61746976655f3536 = v_637572736f72.read(); (v_7075695f6e61746976655f3536.clone()).get() }, Value::boolean(false), Value::boolean(true)]);
                    }
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":150:7
                return Value::boolean(true);
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":152:5
            { let v_7075695f6e61746976655f3537 = v_646566.clone(); expose_method(&owner, Value::string("setItems"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6974656d73 = v_6974656d73.clone(); let v_76616c69644974656d73 = v_76616c69644974656d73.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6e6578745f5f63336462363239303163653033336462: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":153:7
                if ({ let left = { let left = { let value = (v_646973706f736564.read()).borrow().clone(); value }; if !left.truthy() { { let relational_left = (v_6e6578745f5f63336462363239303163653033336462.clone()).member("length", false); let relational_right = Value::number(1000000.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) } } else { left } }; if !left.truthy() { Value::boolean(!(v_76616c69644974656d73.read().call_authored(vec![v_6e6578745f5f63336462363239303163653033336462.clone(), Value::number(0.0), (v_6e6578745f5f63336462363239303163653033336462.clone()).member("length", false)])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":153:83
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":154:7
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":154:16
                { let value = v_6e6578745f5f63336462363239303163653033336462.clone(); *(v_6974656d73.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":155:7
                if ({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":155:20
                    v_73796e63.read().call_authored(vec![{ let value = (v_63757272656e74.read()).borrow().clone(); value }]);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":156:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":158:5
            { let v_7075695f6e61746976655f3538 = v_646566.clone(); expose_method(&owner, Value::string("choose"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_63686f6f7365 = v_63686f6f7365.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f66353737353231626163366539343165: Value = arg(&arguments, 0);
                let v_657874656e645f5f65396264633562613335356664396136: Value = arg(&arguments, 1);
                let v_62617369735f5f38363834363564666435393761333634: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":159:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f38363834363564666435393761333634.clone()), &({ let left = ({ let v_7075695f6e61746976655f3539 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3539.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3539.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":159:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":160:7
                return v_63686f6f7365.read().call_authored(vec![v_696e6465785f5f66353737353231626163366539343165.clone(), v_657874656e645f5f65396264633562613335356664396136.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":162:5
            { let v_7075695f6e61746976655f3630 = v_646566.clone(); expose_method(&owner, Value::string("selectIndex"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f63353036383464353233643634356363: Value = arg(&arguments, 0);
                let v_657874656e645f5f39663461633861653036333862643166: Value = arg(&arguments, 1);
                let v_62617369735f5f62633662396166316630393466636466: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":163:7
                if (Value::boolean(!strict_equal(&(v_62617369735f5f62633662396166316630393466636466.clone()), &({ let left = ({ let v_7075695f6e61746976655f3631 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3631.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3631.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } })))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":163:60
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":164:7
                return v_73656c656374.read().call_authored(vec![v_696e6465785f5f63353036383464353233643634356363.clone(), v_657874656e645f5f39663461633861653036333862643166.clone(), Value::boolean(false)]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":168:5
            { let v_7075695f6e61746976655f3632 = v_646566.clone(); expose_method(&owner, Value::string("keyAt"), {  let v_637572736f72 = v_637572736f72.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_7265616479 = v_7265616479.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_656d6974 = v_656d6974.clone(); let v_6b65795f5f33653136666361613234643562356135 = v_6b65795f5f33653136666361613234643562356135.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f64306165326638646339316538393238: Value = arg(&arguments, 0);
                let v_6e616d655f5f37633037336565656331383435386633: Value = arg(&arguments, 1);
                let v_73686966745f5f36656666326561356338393366363434: Value = arg(&arguments, 2);
                let v_636f6d6d616e645f5f66616363653966383038376163323264: Value = arg(&arguments, 3);
                let v_62617369735f5f30313765626163333033336265303963: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":169:7
                if ({ let left = { let left = { let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f30313765626163333033336265303963.clone()), &({ let left = ({ let v_7075695f6e61746976655f3633 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3633.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }; if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6b696e64.read().call_authored(vec![v_696e6465785f5f64306165326638646339316538393238.clone()])), &(Value::number(0.0)))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":169:104
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":170:13
                let v_6d6f64655f5f66353666336236626638373938313438 = { let left = ({ let v_7075695f6e61746976655f3634 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3634.clone()).owner(), false) }).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":171:13
                let v_68616e646c6564 = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Escape")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Enter")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string(" ")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Home")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("End")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("ArrowLeft")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("ArrowRight")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("ArrowUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("ArrowDown")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("PageUp")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("PageDown")))) } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Tab")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("points")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("table")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("menu")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("table")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("F2")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Delete")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("Backspace")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("points")))); if left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("+")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("=")))) } else { left } }; if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("-")))) } else { left } } } else { left } } } else { left } }; if !left.truthy() { { let left = { let left = Value::boolean(strict_equal(&(v_6d6f64655f5f66353666336236626638373938313438.clone()), &(Value::string("searchlist")))); if left.truthy() { v_636f6d6d616e645f5f66616363653966383038376163323264.clone() } else { left } }; if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f37633037336565656331383435386633.clone()), &(Value::string("F")))) } else { left } } } else { left } } } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":177:7
                if (Value::boolean(!(v_68616e646c6564.clone()).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":177:21
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":178:13
                let v_7469636b65745f5f66386464366565353362663562323165 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":179:7
                { let v_7075695f6e61746976655f3635 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3635.clone(), v_696e6465785f5f64306165326638646339316538393238.clone(), Value::string("reason: addressed collection key")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":180:7
                if ({ let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f66386464366565353362663562323165.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":180:41
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":181:7
                v_656d6974.read().call_authored(vec![Value::string("select"), v_696e6465785f5f64306165326638646339316538393238.clone(), Value::boolean(false), Value::string("")]);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":182:7
                if ({ let left = { let left = Value::boolean(!strict_equal(&(v_7469636b65745f5f66386464366565353362663562323165.clone()), &({ let value = (v_65706f6368.read()).borrow().clone(); value }))); if !left.truthy() { Value::boolean(!(v_7265616479.read().call_authored(vec![])).truthy()) } else { left } }; if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f30313765626163333033336265303963.clone()), &({ let left = ({ let v_7075695f6e61746976655f3636 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3636.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3636.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":182:92
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":183:7
                return v_6b65795f5f33653136666361613234643562356135.read().call_authored(vec![v_6e616d655f5f37633037336565656331383435386633.clone(), v_73686966745f5f36656666326561356338393366363434.clone(), v_636f6d6d616e645f5f66616363653966383038376163323264.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":185:5
            { let v_7075695f6e61746976655f3637 = v_646566.clone(); expose_method(&owner, Value::string("selectAndFocus"), {  let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_73656c656374 = v_73656c656374.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_696e6465785f5f66353962396636653339316335646135: Value = arg(&arguments, 0);
                let v_657874656e645f5f66353030356433613765343738326139: Value = arg(&arguments, 1);
                let v_62617369735f5f35373963656437353235666236363134: Value = arg(&arguments, 2);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":186:7
                if ({ let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f35373963656437353235666236363134.clone()), &({ let left = ({ let v_7075695f6e61746976655f3638 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3638.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":186:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":187:7
                if ({ let left = Value::boolean(!(v_73656c656374.read().call_authored(vec![v_696e6465785f5f66353962396636653339316335646135.clone(), v_657874656e645f5f66353030356433613765343738326139.clone(), Value::boolean(true)])).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f35373963656437353235666236363134.clone()), &({ let left = ({ let v_7075695f6e61746976655f3639 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; if v_7075695f6e61746976655f3639.nullish() { Value::Undefined } else { read_props(&(v_7075695f6e61746976655f3639.clone()).owner(), false) } }).member("revision", true); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":187:92
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":188:7
                { let v_7075695f6e61746976655f3730 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3730.clone()).owner(), Value::Undefined, false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":189:7
                return Value::boolean(true);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":191:5
            { let v_7075695f6e61746976655f3731 = v_646566.clone(); expose_method(&owner, Value::string("key"), v_6b65795f5f33653136666361613234643562356135.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":195:5
            { let v_7075695f6e61746976655f3732 = v_646566.clone(); expose_method(&owner, Value::string("keyFrom"), {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6b65795f5f33653136666361613234643562356135 = v_6b65795f5f33653136666361613234643562356135.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_6f726967696e: Value = arg(&arguments, 0);
                let v_6e616d655f5f32326433386338663766393631363137: Value = arg(&arguments, 1);
                let v_73686966745f5f31656661333433333861393737353465: Value = arg(&arguments, 2);
                let v_636f6d6d616e645f5f38363831366138306238346364386536: Value = arg(&arguments, 3);
                let v_62617369735f5f65633266346237343965623538623863: Value = arg(&arguments, 4);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":196:7
                if ({ let left = Value::boolean(!({ let value = (v_63757272656e74.read()).borrow().clone(); value }).truthy()); if !left.truthy() { Value::boolean(!strict_equal(&(v_62617369735f5f65633266346237343965623538623863.clone()), &({ let left = ({ let v_7075695f6e61746976655f3733 = { let value = (v_63757272656e74.read()).borrow().clone(); value }; read_props(&(v_7075695f6e61746976655f3733.clone()).owner(), false) }).member("revision", false); if left.nullish() { Value::string("") } else { left } }))) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":196:71
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":197:7
                if (Value::boolean(strict_equal(&(v_6f726967696e.clone()), &(Value::string("form"))))).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":197:30
                    return if (Value::boolean(strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("Escape"))))).truthy() { v_6b65795f5f33653136666361613234643562356135.read().call_authored(vec![v_6e616d655f5f32326433386338663766393631363137.clone(), v_73686966745f5f31656661333433333861393737353465.clone(), v_636f6d6d616e645f5f38363831366138306238346364386536.clone()]) } else { Value::boolean(false) };
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":198:7
                if ({ let left = { let left = { let left = { let left = { let left = Value::boolean(strict_equal(&(v_6f726967696e.clone()), &(Value::string("search")))); if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("ArrowUp")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("ArrowDown")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("Enter")))) } else { left } }; if left.truthy() { Value::boolean(!strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("Escape")))) } else { left } }; if left.truthy() { Value::boolean(!({ let left = v_636f6d6d616e645f5f38363831366138306238346364386536.clone(); if left.truthy() { { let left = Value::boolean(strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("f")))); if !left.truthy() { Value::boolean(strict_equal(&(v_6e616d655f5f32326433386338663766393631363137.clone()), &(Value::string("F")))) } else { left } } } else { left } }).truthy()) } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":199:58
                    return Value::boolean(false);
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":200:7
                return v_6b65795f5f33653136666361613234643562356135.read().call_authored(vec![v_6e616d655f5f32326433386338663766393631363137.clone(), v_73686966745f5f31656661333433333861393737353465.clone(), v_636f6d6d616e645f5f38363831366138306238346364386536.clone()]);
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":202:5
            { let v_7075695f6e61746976655f3734 = v_646566.clone(); expose_method(&owner, Value::string("dismiss"), {  let v_637572736f72 = v_637572736f72.clone(); let v_7265616479 = v_7265616479.clone(); let v_656d6974 = v_656d6974.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":202:42
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":202:55
                    v_656d6974.read().call_authored(vec![Value::string("close"), { let v_7075695f6e61746976655f3735 = v_637572736f72.read(); (v_7075695f6e61746976655f3735.clone()).get() }, Value::boolean(false), Value::string("outside")]);
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":203:5
            { let v_7075695f6e61746976655f3736 = v_646566.clone(); expose_method(&owner, Value::string("focusSelf"), {  let v_666f637573 = v_666f637573.clone(); let v_7265616479 = v_7265616479.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":203:44
                if (v_7265616479.read().call_authored(vec![])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":203:57
                    { let v_7075695f6e61746976655f3737 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3737.clone()).owner(), Value::Undefined, false) };
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":204:5
            { let v_7075695f6e61746976655f3738 = v_646566.clone(); listen(&owner, Value::string("key.down"), false, {  let v_6b65795f5f33653136666361613234643562356135 = v_6b65795f5f33653136666361613234643562356135.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_5f72756e: Value = arg(&arguments, 0);
                let v_6576656e74: Value = arg(&arguments, 1);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":205:7
                if (v_6b65795f5f33653136666361613234643562356135.read().call_authored(vec![{ let left = (v_6576656e74.clone()).member("key", false); if left.nullish() { Value::string("") } else { left } }, Value::boolean(strict_equal(&((v_6576656e74.clone()).member("shiftKey", false)), &(Value::boolean(true)))), { let left = Value::boolean(strict_equal(&((v_6576656e74.clone()).member("ctrlKey", false)), &(Value::boolean(true)))); if !left.truthy() { Value::boolean(strict_equal(&((v_6576656e74.clone()).member("metaKey", false)), &(Value::boolean(true)))) } else { left } }])).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":205:108
                    { let v_7075695f6e61746976655f3739 = v_6576656e74.clone(); prevent(v_7075695f6e61746976655f3739.clone()) };
                }
                Value::Undefined
            })) }, Value::Undefined) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":207:11
            v_73796e63.initialize({  let v_637572736f72 = v_637572736f72.clone(); let v_657870616e646564 = v_657870616e646564.clone(); let v_64697361626c6564 = v_64697361626c6564.clone(); let v_726f6c65 = v_726f6c65.clone(); let v_6c6162656c = v_6c6162656c.clone(); let v_7461626c65 = v_7461626c65.clone(); let v_726f7773 = v_726f7773.clone(); let v_636f6c756d6e73 = v_636f6c756d6e73.clone(); let v_666f637573 = v_666f637573.clone(); let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); let v_6b696e64 = v_6b696e64.clone(); let v_636f756e74 = v_636f756e74.clone(); let v_6d696e = v_6d696e.clone(); let v_6d6178 = v_6d6178.clone(); let v_7365656b = v_7365656b.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f39656636646635343466353536393832: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":208:7
                if ({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":208:21
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":209:13
                let v_7469636b65745f5f63383763303339616431316635383666 = { let cell = v_65706f6368.read(); let previous = cell.borrow().clone(); let value = Value::number(previous.scalar_number() + 1.0); *cell.borrow_mut() = value.clone(); value };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":209:31
                { let value = v_72756e5f5f39656636646635343466353536393832.clone(); *(v_63757272656e74.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":210:13
                let v_70726f7073 = { let v_7075695f6e61746976655f3830 = v_72756e5f5f39656636646635343466353536393832.clone(); read_props(&(v_7075695f6e61746976655f3830.clone()).owner(), false) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":211:7
                { let v_7075695f6e61746976655f3831 = v_726f6c65.read(); state_set(v_7075695f6e61746976655f3831.clone(), if ({ let left = Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("points")))); if !left.truthy() { Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("searchlist")))) } else { left } }).truthy() { Value::string("listbox") } else { { let left = (v_70726f7073.clone()).member("mode", false); if left.nullish() { Value::string("listbox") } else { left } } }, Value::string("reason: collection role")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":212:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":212:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":213:7
                { let v_7075695f6e61746976655f3832 = v_7461626c65.read(); state_set(v_7075695f6e61746976655f3832.clone(), Value::boolean(strict_equal(&((v_70726f7073.clone()).member("mode", false)), &(Value::string("table")))), Value::string("reason: collection table semantics")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":214:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":214:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":215:7
                { let v_7075695f6e61746976655f3833 = v_726f7773.read(); state_set(v_7075695f6e61746976655f3833.clone(), v_636f756e74.read().call_authored(vec![]), Value::string("reason: collection rows")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":216:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":216:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":217:7
                { let v_7075695f6e61746976655f3834 = v_636f6c756d6e73.read(); state_set(v_7075695f6e61746976655f3834.clone(), { let left = (v_70726f7073.clone()).member("columns", false); if left.nullish() { Value::number(0.0) } else { left } }, Value::string("reason: collection columns")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":218:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":218:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":219:7
                { let v_7075695f6e61746976655f3835 = v_6c6162656c.read(); state_set(v_7075695f6e61746976655f3835.clone(), { let left = (v_70726f7073.clone()).member("label", false); if left.nullish() { Value::string("") } else { left } }, Value::string("reason: collection name")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":220:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":220:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":221:7
                { let v_7075695f6e61746976655f3836 = v_64697361626c6564.read(); state_set(v_7075695f6e61746976655f3836.clone(), { let left = (v_70726f7073.clone()).member("disabled", false); if left.nullish() { Value::boolean(false) } else { left } }, Value::string("reason: collection policy")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":222:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":222:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":223:7
                { let v_7075695f6e61746976655f3837 = v_666f637573.read(); focus_disabled(&(v_7075695f6e61746976655f3837.clone()).owner(), { let v_7075695f6e61746976655f3838 = v_64697361626c6564.read(); (v_7075695f6e61746976655f3838.clone()).get() }) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":224:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":224:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":225:13
                let v_7265717565737465645f5f66663965366438396166386135326663 = { let left = (v_70726f7073.clone()).member("cursor", false); if left.nullish() { Value::number(0.0) } else { left } };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":226:13
                let v_6163636570746564 = if ({ let left = number_is_finite(&(v_7265717565737465645f5f66663965366438396166386135326663.clone())); if left.truthy() { Value::boolean(strict_equal(&(Value::number((v_7265717565737465645f5f66663965366438396166386135326663.clone()).scalar_number() % (Value::number(1.0)).scalar_number())), &(Value::number(0.0)))) } else { left } }).truthy() { v_6d6178.read().call_authored(vec![Value::number(0.0), v_6d696e.read().call_authored(vec![Value::number((v_636f756e74.read().call_authored(vec![])).scalar_number() - (Value::number(1.0)).scalar_number()), v_7265717565737465645f5f66663965366438396166386135326663.clone()])]) } else { Value::number(0.0) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":227:7
                { let v_7075695f6e61746976655f3839 = v_637572736f72.read(); state_set(v_7075695f6e61746976655f3839.clone(), if ({ let relational_left = v_6b696e64.read().call_authored(vec![v_6163636570746564.clone()]); let relational_right = Value::number(0.0); Value::boolean(relational_left.scalar_number() > relational_right.scalar_number()) }).truthy() { v_6163636570746564.clone() } else { v_6d6178.read().call_authored(vec![Value::number(0.0), v_7365656b.read().call_authored(vec![Value::number(0.0), Value::number(1.0), v_636f756e74.read().call_authored(vec![]), Value::boolean(false)])]) }, Value::string("reason: accepted collection cursor")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":228:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":228:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":229:7
                { let v_7075695f6e61746976655f3930 = v_657870616e646564.read(); state_set(v_7075695f6e61746976655f3930.clone(), { let left = (v_70726f7073.clone()).member("expanded", false); if left.nullish() { Value::number(-(Value::number(1.0)).scalar_number()) } else { left } }, Value::string("reason: accepted expanded item")) };
                // Source "frontend/proto-ui/src/collection-control.proto.ts":230:7
                if ({ let left = Value::boolean(!strict_equal(&({ let value = (v_65706f6368.read()).borrow().clone(); value }), &(v_7469636b65745f5f63383763303339616431316635383666.clone()))); if !left.truthy() { { let value = (v_646973706f736564.read()).borrow().clone(); value } } else { left } }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":230:41
                    return Value::Undefined;
                }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":231:7
                if ({ let value = (v_6174746163686564.read()).borrow().clone(); value }).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":231:21
                    { let v_7075695f6e61746976655f3931 = v_72756e5f5f39656636646635343466353536393832.clone(); request_update(&(v_7075695f6e61746976655f3931.clone()).owner()) };
                }
                Value::Undefined
            })) });
            // Source "frontend/proto-ui/src/collection-control.proto.ts":233:5
            { let v_7075695f6e61746976655f3932 = v_646566.clone(); register_life(&owner, "created", v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":234:5
            { let v_7075695f6e61746976655f3933 = v_646566.clone(); register_life(&owner, "mounted", {  let v_666f637573 = v_666f637573.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_73796e63 = v_73796e63.clone(); Value::Function(Rc::new(move |arguments: Vec<Value>| {
                let v_72756e5f5f35363539376232613461383236383766: Value = arg(&arguments, 0);
                // Source "frontend/proto-ui/src/collection-control.proto.ts":234:40
                if (Value::boolean(!({ let value = (v_646973706f736564.read()).borrow().clone(); value }).truthy())).truthy() {
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":234:57
                    { let value = Value::boolean(true); *(v_6174746163686564.read()).borrow_mut() = value; }
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":234:74
                    v_73796e63.read().call_authored(vec![v_72756e5f5f35363539376232613461383236383766.clone()]);
                    // Source "frontend/proto-ui/src/collection-control.proto.ts":234:85
                    if (({ let v_7075695f6e61746976655f3934 = v_72756e5f5f35363539376232613461383236383766.clone(); read_props(&(v_7075695f6e61746976655f3934.clone()).owner(), false) }).member("autoFocus", false)).truthy() {
                        // Source "frontend/proto-ui/src/collection-control.proto.ts":234:116
                        { let v_7075695f6e61746976655f3935 = v_666f637573.read(); focus_request(&(v_7075695f6e61746976655f3935.clone()).owner(), Value::Undefined, false) };
                    }
                }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":235:5
            { let v_7075695f6e61746976655f3936 = v_646566.clone(); watch_props(&owner, Some(Value::Array(Rc::new(vec![Value::string("mode"), Value::string("label"), Value::string("cursor"), Value::string("expanded"), Value::string("revision"), Value::string("disabled"), Value::string("autoFocus"), Value::string("columns")]))), false, v_73796e63.read()) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":236:5
            { let v_7075695f6e61746976655f3937 = v_646566.clone(); register_life(&owner, "unmounted", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":236:39
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":236:48
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":236:66
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":237:5
            { let v_7075695f6e61746976655f3938 = v_646566.clone(); register_life(&owner, "before-dispose", {  let v_63757272656e74 = v_63757272656e74.clone(); let v_6174746163686564 = v_6174746163686564.clone(); let v_646973706f736564 = v_646973706f736564.clone(); let v_65706f6368 = v_65706f6368.clone(); Value::Function(Rc::new(move |_: Vec<Value>| {
                // Source "frontend/proto-ui/src/collection-control.proto.ts":237:43
                { let cell = v_65706f6368.read(); let next = cell.borrow().scalar_number() + 1.0; *cell.borrow_mut() = Value::number(next); }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":237:52
                { let value = Value::boolean(true); *(v_646973706f736564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":237:69
                { let value = Value::boolean(false); *(v_6174746163686564.read()).borrow_mut() = value; }
                // Source "frontend/proto-ui/src/collection-control.proto.ts":237:87
                { let value = Value::Json(Json::Null); *(v_63757272656e74.read()).borrow_mut() = value; }
                Value::Undefined
            })) }) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":238:5
            { let v_7075695f6e61746976655f3939 = v_646566.clone(); style_use(&owner, vec![Value::Style(vec![("w-full".into(), "w-".into()), ("h-full".into(), "h-".into())])]) };
            // Source "frontend/proto-ui/src/collection-control.proto.ts":239:5
            return { let callable={ let pui_native_100={   let call:Rc<dyn Fn(Value)->native::gpui_native_owner::NativeSnapshot>=Rc::new(move |argument_0: Value| -> native::gpui_native_owner::NativeSnapshot {
                let v_72656e6465726572: Value = argument_0;
                // Source "frontend/proto-ui/src/collection-control.proto.ts":239:26
                return { let v_7075695f6e61746976655f313031 = v_72656e6465726572.clone(); native::gpui_native_owner::NativeSnapshot::slot() };
            }); native::gpui_native_owner::NativeCallable { identity:Rc::new(()), call } }; Value::NativeFunction{identity:pui_native_100.identity.clone(),render:Rc::new(move |arguments:Vec<Value>| {(pui_native_100.call)(arg(&arguments,0))})} }; match callable {
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
      pub fn handle(&self) -> CollectionControlHandle { CollectionControlHandle { owner: self.owner.clone() } }
      pub fn set_props(&mut self, raw: Json, window: &mut Window, cx: &mut Context<Self>) -> Result<(), Value> {
          let outcome = catch_authored(|| replace_props(&self.owner, raw));
          if outcome.is_err() { drive_pending_native_events(&self.owner, window, cx); }
          outcome?; self.drive(window, cx)
      }
      pub fn set_slot(&mut self, slot: Vec<native::gpui_native_owner::NativeSnapshot>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { ensure(&Rc::downgrade(&self.owner)); assert_native_slot_children(&slot); let mut owner = self.owner.borrow_mut(); owner.slot = slot; owner.dirty = true; cx.notify(); }) }
      pub fn set_parent(&mut self, parent: Option<OwnerWeak>, cx: &mut Context<Self>) -> Result<(), Value> { catch_authored(|| { if let Err(error)=set_native_parent(&self.owner, parent.unwrap_or_default()){throw_error("NATIVE_PARENT_INVALID", error)} cx.notify(); }) }
      fn bind_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
          if !native_window_observation_needed(&self.owner) { self._window_activation.take(); return; }
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
  impl Drop for CollectionControl { fn drop(&mut self) { native_host_callback(&self.owner, "drop", || Owner::dispose(&self.owner)); } }
  impl gpui::Focusable for CollectionControl { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
  impl Render for CollectionControl {
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
