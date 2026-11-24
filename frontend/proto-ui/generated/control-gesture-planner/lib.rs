// Generated checked typed data functions. No setup, renderer, owner, dynamic object map or IR interpreter.
#![allow(dead_code, unused_variables, unused_mut, unused_imports, unreachable_code)]

use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataError {
    InvalidType(&'static str),
    NonFiniteNumber,
    InvalidString,
    NullishProperty,
}
impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for DataError {}

/// The source language's undefined value, distinct from null and an omitted optional field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Undefined;


/// Typed immutable array. Clones preserve the authored array's reference identity.
#[derive(Clone, Debug)]
pub struct DataArray<T> { items: Rc<Vec<T>>, identity: Rc<()> }
impl<T> DataArray<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items: Rc::new(items), identity: Rc::new(()) } }
    pub fn as_slice(&self) -> &[T] { self.items.as_slice() }
}
impl<T> From<Vec<T>> for DataArray<T> { fn from(items: Vec<T>) -> Self { Self::new(items) } }
impl<T> std::ops::Deref for DataArray<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { self.as_slice() }
}




#[derive(Debug)]
pub struct PlanControlGestureGuards {
    identity: Rc<()>,
    /// Source field "hit_control".
    pub hit_control: bool,
    /// Source field "threshold_crossed".
    pub threshold_crossed: bool,
    /// Source field "reset_modifier".
    pub reset_modifier: bool,
    /// Source field "double_click".
    pub double_click: bool,
}
impl PlanControlGestureGuards {
    pub fn new(hit_control: bool, threshold_crossed: bool, reset_modifier: bool, double_click: bool) -> Rc<Self> {
        Rc::new(Self { identity: Rc::new(()), hit_control, threshold_crossed, reset_modifier, double_click })
    }
}

#[derive(Debug)]
pub struct PlanControlGestureResult {
    identity: Rc<()>,
    /// Source field "nextPhase".
    pub next_phase: String,
    /// Source field "consumed".
    pub consumed: bool,
    /// Source field "effects".
    pub effects: DataArray<String>,
}
impl PlanControlGestureResult {
    pub fn new(next_phase: String, consumed: bool, effects: DataArray<String>) -> Rc<Self> {
        Rc::new(Self { identity: Rc::new(()), next_phase, consumed, effects })
    }
}

#[derive(Clone, Debug)]
pub enum Data2 {
    Alternative0(f64),
    Alternative1,
}

#[derive(Clone, Debug)]
pub enum Data3 {
    Alternative0,
    Alternative1,
}

fn pui_function_0(v_7068617365: String, v_6576656e74: String, v_677561726473: Rc<PlanControlGestureGuards>) -> Result<Rc<PlanControlGestureResult>, DataError> {
    if { let pui_local_7 = { let pui_local_0 = { let pui_local_2 = v_7068617365.clone(); let pui_local_3 = "idle".to_owned(); (pui_local_2 == pui_local_3) }; if ({ let pui_local_4 = pui_local_0.clone(); pui_local_4 }) { { let pui_local_5 = v_6576656e74.clone(); let pui_local_6 = "PointerDown".to_owned(); (pui_local_5 == pui_local_6) } } else { pui_local_0 } }; pui_local_7 } {
        if { let pui_local_9 = !({ let pui_local_8 = (v_677561726473.clone()).hit_control.clone(); pui_local_8 }); pui_local_9 } {
            return Ok({ let pui_local_10 = "idle".to_owned(); let pui_local_11 = false; let pui_local_12 = DataArray::new(vec![]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_10, consumed: pui_local_11, effects: pui_local_12 }) });
        }
        if { let pui_local_16 = { let pui_local_13 = (v_677561726473.clone()).double_click.clone(); if !({ let pui_local_15 = pui_local_13.clone(); pui_local_15 }) { (v_677561726473.clone()).reset_modifier.clone() } else { pui_local_13 } }; pui_local_16 } {
            return Ok({ let pui_local_17 = "idle".to_owned(); let pui_local_18 = true; let pui_local_19 = DataArray::new(vec!["Control.Select".to_owned(), "Control.Reset".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_17, consumed: pui_local_18, effects: pui_local_19 }) });
        }
        return Ok({ let pui_local_20 = "armed".to_owned(); let pui_local_21 = true; let pui_local_22 = DataArray::new(vec!["Control.Select".to_owned(), "Control.Begin".to_owned(), "Host.CapturePointer".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_20, consumed: pui_local_21, effects: pui_local_22 }) });
    }
    if { let pui_local_30 = { let pui_local_23 = { let pui_local_25 = v_7068617365.clone(); let pui_local_26 = "armed".to_owned(); (pui_local_25 == pui_local_26) }; if ({ let pui_local_27 = pui_local_23.clone(); pui_local_27 }) { { let pui_local_28 = v_6576656e74.clone(); let pui_local_29 = "PointerMove".to_owned(); (pui_local_28 == pui_local_29) } } else { pui_local_23 } }; pui_local_30 } {
        if { let pui_local_32 = !({ let pui_local_31 = (v_677561726473.clone()).threshold_crossed.clone(); pui_local_31 }); pui_local_32 } {
            return Ok({ let pui_local_33 = "armed".to_owned(); let pui_local_34 = true; let pui_local_35 = DataArray::new(vec![]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_33, consumed: pui_local_34, effects: pui_local_35 }) });
        }
        return Ok({ let pui_local_36 = "dragging".to_owned(); let pui_local_37 = true; let pui_local_38 = DataArray::new(vec!["Control.Preview".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_36, consumed: pui_local_37, effects: pui_local_38 }) });
    }
    if { let pui_local_46 = { let pui_local_39 = { let pui_local_41 = v_7068617365.clone(); let pui_local_42 = "dragging".to_owned(); (pui_local_41 == pui_local_42) }; if ({ let pui_local_43 = pui_local_39.clone(); pui_local_43 }) { { let pui_local_44 = v_6576656e74.clone(); let pui_local_45 = "PointerMove".to_owned(); (pui_local_44 == pui_local_45) } } else { pui_local_39 } }; pui_local_46 } {
        return Ok({ let pui_local_47 = "dragging".to_owned(); let pui_local_48 = true; let pui_local_49 = DataArray::new(vec!["Control.Preview".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_47, consumed: pui_local_48, effects: pui_local_49 }) });
    }
    if { let pui_local_57 = { let pui_local_50 = { let pui_local_52 = v_7068617365.clone(); let pui_local_53 = "dragging".to_owned(); (pui_local_52 == pui_local_53) }; if ({ let pui_local_54 = pui_local_50.clone(); pui_local_54 }) { { let pui_local_55 = v_6576656e74.clone(); let pui_local_56 = "PointerUp".to_owned(); (pui_local_55 == pui_local_56) } } else { pui_local_50 } }; pui_local_57 } {
        return Ok({ let pui_local_58 = "idle".to_owned(); let pui_local_59 = true; let pui_local_60 = DataArray::new(vec!["Control.Commit".to_owned(), "Host.ReleasePointer".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_58, consumed: pui_local_59, effects: pui_local_60 }) });
    }
    if { let pui_local_68 = { let pui_local_61 = { let pui_local_63 = v_7068617365.clone(); let pui_local_64 = "armed".to_owned(); (pui_local_63 == pui_local_64) }; if ({ let pui_local_65 = pui_local_61.clone(); pui_local_65 }) { { let pui_local_66 = v_6576656e74.clone(); let pui_local_67 = "PointerUp".to_owned(); (pui_local_66 == pui_local_67) } } else { pui_local_61 } }; pui_local_68 } {
        return Ok({ let pui_local_69 = "idle".to_owned(); let pui_local_70 = true; let pui_local_71 = DataArray::new(vec!["Control.Discard".to_owned(), "Host.ReleasePointer".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_69, consumed: pui_local_70, effects: pui_local_71 }) });
    }
    if { let pui_local_84 = { let pui_local_72 = { let pui_local_74 = { let pui_local_76 = v_7068617365.clone(); let pui_local_77 = "armed".to_owned(); (pui_local_76 == pui_local_77) }; if !({ let pui_local_78 = pui_local_74.clone(); pui_local_78 }) { { let pui_local_79 = v_7068617365.clone(); let pui_local_80 = "dragging".to_owned(); (pui_local_79 == pui_local_80) } } else { pui_local_74 } }; if ({ let pui_local_81 = pui_local_72.clone(); pui_local_81 }) { { let pui_local_82 = v_6576656e74.clone(); let pui_local_83 = "Cancel".to_owned(); (pui_local_82 == pui_local_83) } } else { pui_local_72 } }; pui_local_84 } {
        return Ok({ let pui_local_85 = "idle".to_owned(); let pui_local_86 = true; let pui_local_87 = DataArray::new(vec!["Control.Discard".to_owned(), "Host.ReleasePointer".to_owned()]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_85, consumed: pui_local_86, effects: pui_local_87 }) });
    }
    return Ok({ let pui_local_88 = v_7068617365.clone(); let pui_local_89 = false; let pui_local_90 = DataArray::new(vec![]); Rc::new(PlanControlGestureResult { identity: Rc::new(()), next_phase: pui_local_88, consumed: pui_local_89, effects: pui_local_90 }) });
}

fn pui_function_1(v_7374617274: f64, v_64656c7461: f64, v_74726176656c: f64, v_66696e65: bool) -> Result<f64, DataError> {
    let v_7370616e: Data2 = if { let pui_local_93 = { let pui_local_91 = v_74726176656c.clone(); let pui_local_92 = 1.0f64; (pui_local_91) > (pui_local_92) }; pui_local_93 } { Data2::Alternative0(v_74726176656c.clone()) } else { Data2::Alternative0(1.0f64) };
    let v_72617465: Data3 = if { let pui_local_94 = v_66696e65.clone(); pui_local_94 } { { let _ = 0.1f64; Data3::Alternative0 } } else { { let _ = 1.0f64; Data3::Alternative1 } };
    let v_6e657874: f64 = { let pui_local_95 = v_7374617274.clone(); let pui_local_96 = { let pui_local_97 = { let pui_local_101 = v_64656c7461.clone(); let pui_local_102 = v_7370616e.clone(); (pui_local_101) / (match pui_local_102 { Data2::Alternative0(pui_local_103) => { pui_local_103 }, Data2::Alternative1 => { 1.0f64 } }) }; let pui_local_98 = v_72617465.clone(); (pui_local_97) * (match pui_local_98 { Data3::Alternative0 => { 0.1f64 }, Data3::Alternative1 => { 1.0f64 } }) }; (pui_local_95) + (pui_local_96) };
    if { let pui_local_107 = { let pui_local_105 = v_6e657874.clone(); let pui_local_106 = 0.0f64; (pui_local_105) < (pui_local_106) }; pui_local_107 } {
        return Ok(0.0f64);
    }
    if { let pui_local_110 = { let pui_local_108 = v_6e657874.clone(); let pui_local_109 = 1.0f64; (pui_local_108) > (pui_local_109) }; pui_local_110 } {
        return Ok(1.0f64);
    }
    return Ok(v_6e657874.clone());
}

/// Calls selected pure source function "planControlGesture"; component setup is not executed.
pub fn plan_control_gesture(argument_0: String, argument_1: String, argument_2: Rc<PlanControlGestureGuards>) -> Result<Rc<PlanControlGestureResult>, DataError> {
    
    
    
    let result = pui_function_0(argument_0, argument_1, argument_2)?;
    
    Ok(result)
}

/// Calls selected pure source function "dragNormalized"; component setup is not executed.
pub fn drag_normalized(argument_0: f64, argument_1: f64, argument_2: f64, argument_3: bool) -> Result<f64, DataError> {
    { let pui_local_123 = (argument_0).clone(); if !pui_local_123.is_finite() { return Err(DataError::NonFiniteNumber); } }
    { let pui_local_124 = (argument_1).clone(); if !pui_local_124.is_finite() { return Err(DataError::NonFiniteNumber); } }
    { let pui_local_125 = (argument_2).clone(); if !pui_local_125.is_finite() { return Err(DataError::NonFiniteNumber); } }
    
    let result = pui_function_1(argument_0, argument_1, argument_2, argument_3)?;
    { let pui_local_127 = (result).clone(); if !pui_local_127.is_finite() { return Err(DataError::NonFiniteNumber); } }
    Ok(result)
}
