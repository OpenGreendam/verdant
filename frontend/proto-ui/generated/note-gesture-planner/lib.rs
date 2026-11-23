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
pub struct PlanNoteGestureGuards {
    identity: Rc<()>,
    /// Source field "hit_background".
    pub hit_background: bool,
    /// Source field "hit_selected_note".
    pub hit_selected_note: bool,
    /// Source field "hit_unselected_note".
    pub hit_unselected_note: bool,
    /// Source field "hit_note_body".
    pub hit_note_body: bool,
    /// Source field "hit_resize_handle".
    pub hit_resize_handle: bool,
    /// Source field "toggle_modifier".
    pub toggle_modifier: bool,
    /// Source field "extend_modifier".
    pub extend_modifier: bool,
    /// Source field "duplicate_modifier".
    pub duplicate_modifier: bool,
    /// Source field "pointer_threshold_crossed".
    pub pointer_threshold_crossed: bool,
    /// Source field "base_revision_matches".
    pub base_revision_matches: bool,
    /// Source field "base_revision_matches_and_delta_nonzero".
    pub base_revision_matches_and_delta_nonzero: bool,
}
impl PlanNoteGestureGuards {
    pub fn new(hit_background: bool, hit_selected_note: bool, hit_unselected_note: bool, hit_note_body: bool, hit_resize_handle: bool, toggle_modifier: bool, extend_modifier: bool, duplicate_modifier: bool, pointer_threshold_crossed: bool, base_revision_matches: bool, base_revision_matches_and_delta_nonzero: bool) -> Rc<Self> {
        Rc::new(Self { identity: Rc::new(()), hit_background, hit_selected_note, hit_unselected_note, hit_note_body, hit_resize_handle, toggle_modifier, extend_modifier, duplicate_modifier, pointer_threshold_crossed, base_revision_matches, base_revision_matches_and_delta_nonzero })
    }
}

#[derive(Debug)]
pub struct PlanNoteGestureResult {
    identity: Rc<()>,
    /// Source field "nextPhase".
    pub next_phase: String,
    /// Source field "consumed".
    pub consumed: bool,
    /// Source field "effects".
    pub effects: DataArray<String>,
}
impl PlanNoteGestureResult {
    pub fn new(next_phase: String, consumed: bool, effects: DataArray<String>) -> Rc<Self> {
        Rc::new(Self { identity: Rc::new(()), next_phase, consumed, effects })
    }
}

fn pui_function_0(v_6d616368696e65: String, v_7068617365: String, v_6576656e74: String, v_677561726473: Rc<PlanNoteGestureGuards>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    if { let pui_local_17 = { let pui_local_0 = { let pui_local_2 = { let pui_local_4 = { let pui_local_6 = v_6d616368696e65.clone(); let pui_local_7 = "marquee".to_owned(); !(pui_local_6 == pui_local_7) }; if ({ let pui_local_8 = pui_local_4.clone(); pui_local_8 }) { { let pui_local_9 = v_6d616368696e65.clone(); let pui_local_10 = "move".to_owned(); !(pui_local_9 == pui_local_10) } } else { pui_local_4 } }; if ({ let pui_local_11 = pui_local_2.clone(); pui_local_11 }) { { let pui_local_12 = v_6d616368696e65.clone(); let pui_local_13 = "resize".to_owned(); !(pui_local_12 == pui_local_13) } } else { pui_local_2 } }; if ({ let pui_local_14 = pui_local_0.clone(); pui_local_14 }) { { let pui_local_15 = v_6d616368696e65.clone(); let pui_local_16 = "duplicate".to_owned(); !(pui_local_15 == pui_local_16) } } else { pui_local_0 } }; pui_local_17 } {
        return Ok({ let pui_local_18 = v_7068617365.clone(); let pui_local_19 = false; let pui_local_20 = DataArray::new(vec![]); Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_18, consumed: pui_local_19, effects: pui_local_20 }) });
    }
    if { let pui_local_28 = { let pui_local_21 = { let pui_local_23 = v_7068617365.clone(); let pui_local_24 = "idle".to_owned(); (pui_local_23 == pui_local_24) }; if ({ let pui_local_25 = pui_local_21.clone(); pui_local_25 }) { { let pui_local_26 = v_6576656e74.clone(); let pui_local_27 = "PointerDown".to_owned(); (pui_local_26 == pui_local_27) } } else { pui_local_21 } }; pui_local_28 } {
        if { let pui_local_34 = { let pui_local_29 = { let pui_local_31 = v_6d616368696e65.clone(); let pui_local_32 = "marquee".to_owned(); (pui_local_31 == pui_local_32) }; if ({ let pui_local_33 = pui_local_29.clone(); pui_local_33 }) { (v_677561726473.clone()).hit_background.clone() } else { pui_local_29 } }; pui_local_34 } {
            return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.BeginMarqueeGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
        }
        if { let pui_local_40 = { let pui_local_35 = { let pui_local_37 = v_6d616368696e65.clone(); let pui_local_38 = "resize".to_owned(); (pui_local_37 == pui_local_38) }; if ({ let pui_local_39 = pui_local_35.clone(); pui_local_39 }) { (v_677561726473.clone()).hit_resize_handle.clone() } else { pui_local_35 } }; pui_local_40 } {
            return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.SelectIfUnselectedHit".to_owned(), "Session.BeginResizeGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
        }
        if { let pui_local_47 = { let pui_local_41 = { let pui_local_43 = v_6d616368696e65.clone(); let pui_local_44 = "move".to_owned(); (pui_local_43 == pui_local_44) }; if ({ let pui_local_45 = pui_local_41.clone(); pui_local_45 }) { !({ let pui_local_46 = (v_677561726473.clone()).duplicate_modifier.clone(); pui_local_46 }) } else { pui_local_41 } }; pui_local_47 } {
            if { let pui_local_51 = { let pui_local_48 = (v_677561726473.clone()).hit_selected_note.clone(); if ({ let pui_local_50 = pui_local_48.clone(); pui_local_50 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_48 } }; pui_local_51 } {
                return Ok(pui_function_1("idle".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned()]))?);
            }
            if { let pui_local_55 = { let pui_local_52 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_54 = pui_local_52.clone(); pui_local_54 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_52 } }; pui_local_55 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned(), "Session.BeginMoveGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_59 = { let pui_local_56 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_58 = pui_local_56.clone(); pui_local_58 }) { (v_677561726473.clone()).extend_modifier.clone() } else { pui_local_56 } }; pui_local_59 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.ExtendSelectionToHit".to_owned(), "Session.BeginMoveGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_60 = (v_677561726473.clone()).hit_selected_note.clone(); pui_local_60 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.BeginMoveGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_61 = (v_677561726473.clone()).hit_unselected_note.clone(); pui_local_61 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.SelectOnlyHitNote".to_owned(), "Session.BeginMoveGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
        }
        if { let pui_local_67 = { let pui_local_62 = { let pui_local_64 = v_6d616368696e65.clone(); let pui_local_65 = "duplicate".to_owned(); (pui_local_64 == pui_local_65) }; if ({ let pui_local_66 = pui_local_62.clone(); pui_local_66 }) { (v_677561726473.clone()).duplicate_modifier.clone() } else { pui_local_62 } }; pui_local_67 } {
            if { let pui_local_71 = { let pui_local_68 = (v_677561726473.clone()).hit_unselected_note.clone(); if ({ let pui_local_70 = pui_local_68.clone(); pui_local_70 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_68 } }; pui_local_71 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned(), "Session.BeginDuplicateGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_75 = { let pui_local_72 = (v_677561726473.clone()).hit_selected_note.clone(); if ({ let pui_local_74 = pui_local_72.clone(); pui_local_74 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_72 } }; pui_local_75 } {
                return Ok(pui_function_1("idle".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned()]))?);
            }
            if { let pui_local_79 = { let pui_local_76 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_78 = pui_local_76.clone(); pui_local_78 }) { (v_677561726473.clone()).extend_modifier.clone() } else { pui_local_76 } }; pui_local_79 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.ExtendSelectionToHit".to_owned(), "Session.BeginDuplicateGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_80 = (v_677561726473.clone()).hit_unselected_note.clone(); pui_local_80 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.SelectOnlyHitNote".to_owned(), "Session.BeginDuplicateGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
            if { let pui_local_81 = (v_677561726473.clone()).hit_selected_note.clone(); pui_local_81 } {
                return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec!["Session.BeginDuplicateGesture".to_owned(), "Host.CapturePointer".to_owned()]))?);
            }
        }
    }
    let v_616374697665: bool = { let pui_local_82 = { let pui_local_84 = v_7068617365.clone(); let pui_local_85 = "armed".to_owned(); (pui_local_84 == pui_local_85) }; if !({ let pui_local_86 = pui_local_82.clone(); pui_local_86 }) { { let pui_local_87 = v_7068617365.clone(); let pui_local_88 = "dragging".to_owned(); (pui_local_87 == pui_local_88) } } else { pui_local_82 } };
    if { let pui_local_104 = { let pui_local_89 = { let pui_local_91 = v_6576656e74.clone(); let pui_local_92 = "PointerMove".to_owned(); (pui_local_91 == pui_local_92) }; if ({ let pui_local_93 = pui_local_89.clone(); pui_local_93 }) { { let pui_local_94 = { let pui_local_96 = v_7068617365.clone(); let pui_local_97 = "dragging".to_owned(); (pui_local_96 == pui_local_97) }; if !({ let pui_local_98 = pui_local_94.clone(); pui_local_98 }) { { let pui_local_99 = { let pui_local_101 = v_7068617365.clone(); let pui_local_102 = "armed".to_owned(); (pui_local_101 == pui_local_102) }; if ({ let pui_local_103 = pui_local_99.clone(); pui_local_103 }) { (v_677561726473.clone()).pointer_threshold_crossed.clone() } else { pui_local_99 } } } else { pui_local_94 } } } else { pui_local_89 } }; pui_local_104 } {
        if { let pui_local_107 = { let pui_local_105 = v_6d616368696e65.clone(); let pui_local_106 = "marquee".to_owned(); (pui_local_105 == pui_local_106) }; pui_local_107 } {
            return Ok(pui_function_1("dragging".to_owned(), DataArray::new(vec!["Session.UpdateMarqueePreview".to_owned()]))?);
        }
        if { let pui_local_110 = { let pui_local_108 = v_6d616368696e65.clone(); let pui_local_109 = "resize".to_owned(); (pui_local_108 == pui_local_109) }; pui_local_110 } {
            return Ok(pui_function_1("dragging".to_owned(), DataArray::new(vec!["Session.UpdateResizePreview".to_owned()]))?);
        }
        return Ok(pui_function_1("dragging".to_owned(), DataArray::new(vec!["Session.UpdateMovePreview".to_owned()]))?);
    }
    if { let pui_local_118 = { let pui_local_111 = { let pui_local_113 = v_6576656e74.clone(); let pui_local_114 = "PointerUp".to_owned(); (pui_local_113 == pui_local_114) }; if ({ let pui_local_115 = pui_local_111.clone(); pui_local_115 }) { { let pui_local_116 = v_7068617365.clone(); let pui_local_117 = "dragging".to_owned(); (pui_local_116 == pui_local_117) } } else { pui_local_111 } }; pui_local_118 } {
        if { let pui_local_124 = { let pui_local_119 = { let pui_local_121 = v_6d616368696e65.clone(); let pui_local_122 = "marquee".to_owned(); (pui_local_121 == pui_local_122) }; if ({ let pui_local_123 = pui_local_119.clone(); pui_local_123 }) { (v_677561726473.clone()).base_revision_matches.clone() } else { pui_local_119 } }; pui_local_124 } {
            return Ok(pui_function_2("Session.CommitMarqueeSelection".to_owned())?);
        }
        if { let pui_local_130 = { let pui_local_125 = { let pui_local_127 = v_6d616368696e65.clone(); let pui_local_128 = "marquee".to_owned(); !(pui_local_127 == pui_local_128) }; if ({ let pui_local_129 = pui_local_125.clone(); pui_local_129 }) { (v_677561726473.clone()).base_revision_matches_and_delta_nonzero.clone() } else { pui_local_125 } }; pui_local_130 } {
            if { let pui_local_133 = { let pui_local_131 = v_6d616368696e65.clone(); let pui_local_132 = "move".to_owned(); (pui_local_131 == pui_local_132) }; pui_local_133 } {
                return Ok(pui_function_2("Domain.CommitMoveNotes".to_owned())?);
            }
            if { let pui_local_136 = { let pui_local_134 = v_6d616368696e65.clone(); let pui_local_135 = "resize".to_owned(); (pui_local_134 == pui_local_135) }; pui_local_136 } {
                return Ok(pui_function_2("Domain.CommitResizeNotes".to_owned())?);
            }
            return Ok(pui_function_2("Domain.CommitDuplicateNotes".to_owned())?);
        }
        return Ok(pui_function_2("Session.ReportConflictIfStale".to_owned())?);
    }
    if { let pui_local_149 = { let pui_local_137 = { let pui_local_139 = { let pui_local_141 = v_6576656e74.clone(); let pui_local_142 = "PointerUp".to_owned(); (pui_local_141 == pui_local_142) }; if ({ let pui_local_143 = pui_local_139.clone(); pui_local_143 }) { { let pui_local_144 = v_7068617365.clone(); let pui_local_145 = "armed".to_owned(); (pui_local_144 == pui_local_145) } } else { pui_local_139 } }; if ({ let pui_local_146 = pui_local_137.clone(); pui_local_146 }) { { let pui_local_147 = v_6d616368696e65.clone(); let pui_local_148 = "resize".to_owned(); !(pui_local_147 == pui_local_148) } } else { pui_local_137 } }; pui_local_149 } {
        return Ok(pui_function_2("Session.ClickReleaseSelection".to_owned())?);
    }
    if { let pui_local_165 = { let pui_local_150 = { let pui_local_152 = { let pui_local_154 = v_6576656e74.clone(); let pui_local_155 = "Cancel".to_owned(); (pui_local_154 == pui_local_155) }; if ({ let pui_local_156 = pui_local_152.clone(); pui_local_156 }) { v_616374697665.clone() } else { pui_local_152 } }; if !({ let pui_local_157 = pui_local_150.clone(); pui_local_157 }) { { let pui_local_158 = { let pui_local_160 = v_6576656e74.clone(); let pui_local_161 = "PointerUp".to_owned(); (pui_local_160 == pui_local_161) }; if ({ let pui_local_162 = pui_local_158.clone(); pui_local_162 }) { { let pui_local_163 = v_7068617365.clone(); let pui_local_164 = "armed".to_owned(); (pui_local_163 == pui_local_164) } } else { pui_local_158 } } } else { pui_local_150 } }; pui_local_165 } {
        return Ok(pui_function_1("idle".to_owned(), DataArray::new(vec!["Session.ClearGesturePreview".to_owned(), "Host.ReleasePointer".to_owned()]))?);
    }
    return Ok({ let pui_local_166 = v_7068617365.clone(); let pui_local_167 = v_616374697665.clone(); let pui_local_168 = DataArray::new(vec![]); Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_166, consumed: pui_local_167, effects: pui_local_168 }) });
}

fn pui_function_1(v_6e6578745068617365: String, v_65666665637473: DataArray<String>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_169 = v_6e6578745068617365.clone(); let pui_local_170 = true; let pui_local_171 = v_65666665637473.clone(); Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_169, consumed: pui_local_170, effects: pui_local_171 }) });
}

fn pui_function_2(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_1("idle".to_owned(), DataArray::new(vec![v_656666656374.clone(), "Session.ClearGesturePreview".to_owned(), "Host.ReleasePointer".to_owned()]))?);
}

/// Calls selected pure source function "planNoteGesture"; component setup is not executed.
pub fn plan_note_gesture(argument_0: String, argument_1: String, argument_2: String, argument_3: Rc<PlanNoteGestureGuards>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    
    
    
    
    let result = pui_function_0(argument_0, argument_1, argument_2, argument_3)?;
    
    Ok(result)
}
