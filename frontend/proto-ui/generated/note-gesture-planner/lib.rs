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

#[derive(Clone, Debug)]
pub enum Data2 {
    Marquee0,
    DataMove1,
    Resize2,
    Duplicate3,
}

#[derive(Clone, Debug)]
pub enum Data3 {
    SessionBeginMarqueeGesture0,
    HostCapturePointer1,
}

#[derive(Clone, Debug)]
pub enum Data4 {
    SessionSelectIfUnselectedHit0,
    SessionBeginResizeGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data5 {
    SessionToggleHitNote0,
    SessionBeginMoveGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data6 {
    SessionExtendSelectionToHit0,
    SessionBeginMoveGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data7 {
    SessionBeginMoveGesture0,
    HostCapturePointer1,
}

#[derive(Clone, Debug)]
pub enum Data8 {
    SessionSelectOnlyHitNote0,
    SessionBeginMoveGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data9 {
    SessionToggleHitNote0,
    SessionBeginDuplicateGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data10 {
    SessionExtendSelectionToHit0,
    SessionBeginDuplicateGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data11 {
    SessionSelectOnlyHitNote0,
    SessionBeginDuplicateGesture1,
    HostCapturePointer2,
}

#[derive(Clone, Debug)]
pub enum Data12 {
    SessionBeginDuplicateGesture0,
    HostCapturePointer1,
}

#[derive(Clone, Debug)]
pub enum Data13 {
    DataMove0,
    Resize1,
    Duplicate2,
}

#[derive(Clone, Debug)]
pub enum Data14 {
    Resize0,
    Duplicate1,
}

#[derive(Clone, Debug)]
pub enum Data15 {
    SessionClearGesturePreview0,
    HostReleasePointer1,
}

#[derive(Clone, Debug)]
pub enum Data16 {
    SessionCommitMarqueeSelection0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

#[derive(Clone, Debug)]
pub enum Data17 {
    DomainCommitMoveNotes0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

#[derive(Clone, Debug)]
pub enum Data18 {
    DomainCommitResizeNotes0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

#[derive(Clone, Debug)]
pub enum Data19 {
    DomainCommitDuplicateNotes0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

#[derive(Clone, Debug)]
pub enum Data20 {
    SessionReportConflictIfStale0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

#[derive(Clone, Debug)]
pub enum Data21 {
    SessionClickReleaseSelection0,
    SessionClearGesturePreview1,
    HostReleasePointer2,
}

fn pui_function_0(v_6d616368696e65: String, v_7068617365: String, v_6576656e74: String, v_677561726473: Rc<PlanNoteGestureGuards>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    if { let pui_local_17 = { let pui_local_0 = { let pui_local_2 = { let pui_local_4 = { let pui_local_6 = v_6d616368696e65.clone(); let pui_local_7 = "marquee".to_owned(); !(pui_local_6 == pui_local_7) }; if ({ let pui_local_8 = pui_local_4.clone(); pui_local_8 }) { { let pui_local_9 = v_6d616368696e65.clone(); let pui_local_10 = "move".to_owned(); !(pui_local_9 == pui_local_10) } } else { pui_local_4 } }; if ({ let pui_local_11 = pui_local_2.clone(); pui_local_11 }) { { let pui_local_12 = v_6d616368696e65.clone(); let pui_local_13 = "resize".to_owned(); !(pui_local_12 == pui_local_13) } } else { pui_local_2 } }; if ({ let pui_local_14 = pui_local_0.clone(); pui_local_14 }) { { let pui_local_15 = v_6d616368696e65.clone(); let pui_local_16 = "duplicate".to_owned(); !(pui_local_15 == pui_local_16) } } else { pui_local_0 } }; pui_local_17 } {
        return Ok({ let pui_local_18 = v_7068617365.clone(); let pui_local_19 = false; let pui_local_20 = DataArray::new(vec![]); Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_18, consumed: pui_local_19, effects: pui_local_20 }) });
    }
    if { let pui_local_28 = { let pui_local_21 = { let pui_local_23 = v_7068617365.clone(); let pui_local_24 = "idle".to_owned(); (pui_local_23 == pui_local_24) }; if ({ let pui_local_25 = pui_local_21.clone(); pui_local_25 }) { { let pui_local_26 = v_6576656e74.clone(); let pui_local_27 = "PointerDown".to_owned(); (pui_local_26 == pui_local_27) } } else { pui_local_21 } }; pui_local_28 } {
        if { let pui_local_39 = { let pui_local_29 = { let pui_local_31 = { let pui_local_37 = v_6d616368696e65.clone(); if pui_local_37 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_37 == "move".to_owned() { Data2::DataMove1 } else if pui_local_37 == "resize".to_owned() { Data2::Resize2 } else if pui_local_37 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_32 = "marquee".to_owned(); (match pui_local_31 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_32.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_32.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_32.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_32.clone() } }) }; if ({ let pui_local_38 = pui_local_29.clone(); pui_local_38 }) { (v_677561726473.clone()).hit_background.clone() } else { pui_local_29 } }; pui_local_39 } {
            return Ok(pui_function_1("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.BeginMarqueeGesture".to_owned(); Data3::SessionBeginMarqueeGesture0 }, { let _ = "Host.CapturePointer".to_owned(); Data3::HostCapturePointer1 }]))?);
        }
        if { let pui_local_50 = { let pui_local_40 = { let pui_local_42 = { let pui_local_48 = v_6d616368696e65.clone(); if pui_local_48 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_48 == "move".to_owned() { Data2::DataMove1 } else if pui_local_48 == "resize".to_owned() { Data2::Resize2 } else if pui_local_48 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_43 = "resize".to_owned(); (match pui_local_42 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_43.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_43.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_43.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_43.clone() } }) }; if ({ let pui_local_49 = pui_local_40.clone(); pui_local_49 }) { (v_677561726473.clone()).hit_resize_handle.clone() } else { pui_local_40 } }; pui_local_50 } {
            return Ok(pui_function_2("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.SelectIfUnselectedHit".to_owned(); Data4::SessionSelectIfUnselectedHit0 }, { let _ = "Session.BeginResizeGesture".to_owned(); Data4::SessionBeginResizeGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data4::HostCapturePointer2 }]))?);
        }
        if { let pui_local_62 = { let pui_local_51 = { let pui_local_53 = { let pui_local_59 = v_6d616368696e65.clone(); if pui_local_59 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_59 == "move".to_owned() { Data2::DataMove1 } else if pui_local_59 == "resize".to_owned() { Data2::Resize2 } else if pui_local_59 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_54 = "move".to_owned(); (match pui_local_53 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_54.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_54.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_54.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_54.clone() } }) }; if ({ let pui_local_60 = pui_local_51.clone(); pui_local_60 }) { !({ let pui_local_61 = (v_677561726473.clone()).duplicate_modifier.clone(); pui_local_61 }) } else { pui_local_51 } }; pui_local_62 } {
            if { let pui_local_66 = { let pui_local_63 = (v_677561726473.clone()).hit_selected_note.clone(); if ({ let pui_local_65 = pui_local_63.clone(); pui_local_65 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_63 } }; pui_local_66 } {
                return Ok(pui_function_3("idle".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned()]))?);
            }
            if { let pui_local_70 = { let pui_local_67 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_69 = pui_local_67.clone(); pui_local_69 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_67 } }; pui_local_70 } {
                return Ok(pui_function_4("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.ToggleHitNote".to_owned(); Data5::SessionToggleHitNote0 }, { let _ = "Session.BeginMoveGesture".to_owned(); Data5::SessionBeginMoveGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data5::HostCapturePointer2 }]))?);
            }
            if { let pui_local_74 = { let pui_local_71 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_73 = pui_local_71.clone(); pui_local_73 }) { (v_677561726473.clone()).extend_modifier.clone() } else { pui_local_71 } }; pui_local_74 } {
                return Ok(pui_function_5("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.ExtendSelectionToHit".to_owned(); Data6::SessionExtendSelectionToHit0 }, { let _ = "Session.BeginMoveGesture".to_owned(); Data6::SessionBeginMoveGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data6::HostCapturePointer2 }]))?);
            }
            if { let pui_local_75 = (v_677561726473.clone()).hit_selected_note.clone(); pui_local_75 } {
                return Ok(pui_function_6("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.BeginMoveGesture".to_owned(); Data7::SessionBeginMoveGesture0 }, { let _ = "Host.CapturePointer".to_owned(); Data7::HostCapturePointer1 }]))?);
            }
            if { let pui_local_76 = (v_677561726473.clone()).hit_unselected_note.clone(); pui_local_76 } {
                return Ok(pui_function_7("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.SelectOnlyHitNote".to_owned(); Data8::SessionSelectOnlyHitNote0 }, { let _ = "Session.BeginMoveGesture".to_owned(); Data8::SessionBeginMoveGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data8::HostCapturePointer2 }]))?);
            }
        }
        if { let pui_local_87 = { let pui_local_77 = { let pui_local_79 = { let pui_local_85 = v_6d616368696e65.clone(); if pui_local_85 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_85 == "move".to_owned() { Data2::DataMove1 } else if pui_local_85 == "resize".to_owned() { Data2::Resize2 } else if pui_local_85 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_80 = "duplicate".to_owned(); (match pui_local_79 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_80.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_80.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_80.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_80.clone() } }) }; if ({ let pui_local_86 = pui_local_77.clone(); pui_local_86 }) { (v_677561726473.clone()).duplicate_modifier.clone() } else { pui_local_77 } }; pui_local_87 } {
            if { let pui_local_91 = { let pui_local_88 = (v_677561726473.clone()).hit_unselected_note.clone(); if ({ let pui_local_90 = pui_local_88.clone(); pui_local_90 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_88 } }; pui_local_91 } {
                return Ok(pui_function_8("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.ToggleHitNote".to_owned(); Data9::SessionToggleHitNote0 }, { let _ = "Session.BeginDuplicateGesture".to_owned(); Data9::SessionBeginDuplicateGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data9::HostCapturePointer2 }]))?);
            }
            if { let pui_local_95 = { let pui_local_92 = (v_677561726473.clone()).hit_selected_note.clone(); if ({ let pui_local_94 = pui_local_92.clone(); pui_local_94 }) { (v_677561726473.clone()).toggle_modifier.clone() } else { pui_local_92 } }; pui_local_95 } {
                return Ok(pui_function_3("idle".to_owned(), DataArray::new(vec!["Session.ToggleHitNote".to_owned()]))?);
            }
            if { let pui_local_99 = { let pui_local_96 = (v_677561726473.clone()).hit_note_body.clone(); if ({ let pui_local_98 = pui_local_96.clone(); pui_local_98 }) { (v_677561726473.clone()).extend_modifier.clone() } else { pui_local_96 } }; pui_local_99 } {
                return Ok(pui_function_9("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.ExtendSelectionToHit".to_owned(); Data10::SessionExtendSelectionToHit0 }, { let _ = "Session.BeginDuplicateGesture".to_owned(); Data10::SessionBeginDuplicateGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data10::HostCapturePointer2 }]))?);
            }
            if { let pui_local_100 = (v_677561726473.clone()).hit_unselected_note.clone(); pui_local_100 } {
                return Ok(pui_function_10("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.SelectOnlyHitNote".to_owned(); Data11::SessionSelectOnlyHitNote0 }, { let _ = "Session.BeginDuplicateGesture".to_owned(); Data11::SessionBeginDuplicateGesture1 }, { let _ = "Host.CapturePointer".to_owned(); Data11::HostCapturePointer2 }]))?);
            }
            if { let pui_local_101 = (v_677561726473.clone()).hit_selected_note.clone(); pui_local_101 } {
                return Ok(pui_function_11("armed".to_owned(), DataArray::new(vec![{ let _ = "Session.BeginDuplicateGesture".to_owned(); Data12::SessionBeginDuplicateGesture0 }, { let _ = "Host.CapturePointer".to_owned(); Data12::HostCapturePointer1 }]))?);
            }
        }
    }
    let v_616374697665: bool = { let pui_local_102 = { let pui_local_104 = v_7068617365.clone(); let pui_local_105 = "armed".to_owned(); (pui_local_104 == pui_local_105) }; if !({ let pui_local_106 = pui_local_102.clone(); pui_local_106 }) { { let pui_local_107 = v_7068617365.clone(); let pui_local_108 = "dragging".to_owned(); (pui_local_107 == pui_local_108) } } else { pui_local_102 } };
    if { let pui_local_124 = { let pui_local_109 = { let pui_local_111 = v_6576656e74.clone(); let pui_local_112 = "PointerMove".to_owned(); (pui_local_111 == pui_local_112) }; if ({ let pui_local_113 = pui_local_109.clone(); pui_local_113 }) { { let pui_local_114 = { let pui_local_116 = v_7068617365.clone(); let pui_local_117 = "dragging".to_owned(); (pui_local_116 == pui_local_117) }; if !({ let pui_local_118 = pui_local_114.clone(); pui_local_118 }) { { let pui_local_119 = { let pui_local_121 = v_7068617365.clone(); let pui_local_122 = "armed".to_owned(); (pui_local_121 == pui_local_122) }; if ({ let pui_local_123 = pui_local_119.clone(); pui_local_123 }) { (v_677561726473.clone()).pointer_threshold_crossed.clone() } else { pui_local_119 } } } else { pui_local_114 } } } else { pui_local_109 } }; pui_local_124 } {
        if { let pui_local_132 = { let pui_local_125 = { let pui_local_131 = v_6d616368696e65.clone(); if pui_local_131 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_131 == "move".to_owned() { Data2::DataMove1 } else if pui_local_131 == "resize".to_owned() { Data2::Resize2 } else if pui_local_131 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_126 = "marquee".to_owned(); (match pui_local_125 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_126.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_126.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_126.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_126.clone() } }) }; pui_local_132 } {
            return Ok(pui_function_12("dragging".to_owned(), DataArray::new(vec!["Session.UpdateMarqueePreview".to_owned()]))?);
        }
        if { let pui_local_139 = { let pui_local_133 = { let pui_local_138 = v_6d616368696e65.clone(); if pui_local_138 == "move".to_owned() { Data13::DataMove0 } else if pui_local_138 == "resize".to_owned() { Data13::Resize1 } else if pui_local_138 == "duplicate".to_owned() { Data13::Duplicate2 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_134 = "resize".to_owned(); (match pui_local_133 { Data13::DataMove0 => { "move".to_owned() == pui_local_134.clone() }, Data13::Resize1 => { "resize".to_owned() == pui_local_134.clone() }, Data13::Duplicate2 => { "duplicate".to_owned() == pui_local_134.clone() } }) }; pui_local_139 } {
            return Ok(pui_function_13("dragging".to_owned(), DataArray::new(vec!["Session.UpdateResizePreview".to_owned()]))?);
        }
        return Ok(pui_function_14("dragging".to_owned(), DataArray::new(vec!["Session.UpdateMovePreview".to_owned()]))?);
    }
    if { let pui_local_147 = { let pui_local_140 = { let pui_local_142 = v_6576656e74.clone(); let pui_local_143 = "PointerUp".to_owned(); (pui_local_142 == pui_local_143) }; if ({ let pui_local_144 = pui_local_140.clone(); pui_local_144 }) { { let pui_local_145 = v_7068617365.clone(); let pui_local_146 = "dragging".to_owned(); (pui_local_145 == pui_local_146) } } else { pui_local_140 } }; pui_local_147 } {
        if { let pui_local_158 = { let pui_local_148 = { let pui_local_150 = { let pui_local_156 = v_6d616368696e65.clone(); if pui_local_156 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_156 == "move".to_owned() { Data2::DataMove1 } else if pui_local_156 == "resize".to_owned() { Data2::Resize2 } else if pui_local_156 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_151 = "marquee".to_owned(); (match pui_local_150 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_151.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_151.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_151.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_151.clone() } }) }; if ({ let pui_local_157 = pui_local_148.clone(); pui_local_157 }) { (v_677561726473.clone()).base_revision_matches.clone() } else { pui_local_148 } }; pui_local_158 } {
            return Ok(pui_function_15("Session.CommitMarqueeSelection".to_owned())?);
        }
        if { let pui_local_169 = { let pui_local_159 = { let pui_local_161 = { let pui_local_167 = v_6d616368696e65.clone(); if pui_local_167 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_167 == "move".to_owned() { Data2::DataMove1 } else if pui_local_167 == "resize".to_owned() { Data2::Resize2 } else if pui_local_167 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_162 = "marquee".to_owned(); !(match pui_local_161 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_162.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_162.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_162.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_162.clone() } }) }; if ({ let pui_local_168 = pui_local_159.clone(); pui_local_168 }) { (v_677561726473.clone()).base_revision_matches_and_delta_nonzero.clone() } else { pui_local_159 } }; pui_local_169 } {
            if { let pui_local_176 = { let pui_local_170 = { let pui_local_175 = v_6d616368696e65.clone(); if pui_local_175 == "move".to_owned() { Data13::DataMove0 } else if pui_local_175 == "resize".to_owned() { Data13::Resize1 } else if pui_local_175 == "duplicate".to_owned() { Data13::Duplicate2 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_171 = "move".to_owned(); (match pui_local_170 { Data13::DataMove0 => { "move".to_owned() == pui_local_171.clone() }, Data13::Resize1 => { "resize".to_owned() == pui_local_171.clone() }, Data13::Duplicate2 => { "duplicate".to_owned() == pui_local_171.clone() } }) }; pui_local_176 } {
                return Ok(pui_function_17("Domain.CommitMoveNotes".to_owned())?);
            }
            if { let pui_local_182 = { let pui_local_177 = { let pui_local_181 = v_6d616368696e65.clone(); if pui_local_181 == "resize".to_owned() { Data14::Resize0 } else if pui_local_181 == "duplicate".to_owned() { Data14::Duplicate1 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_178 = "resize".to_owned(); (match pui_local_177 { Data14::Resize0 => { "resize".to_owned() == pui_local_178.clone() }, Data14::Duplicate1 => { "duplicate".to_owned() == pui_local_178.clone() } }) }; pui_local_182 } {
                return Ok(pui_function_19("Domain.CommitResizeNotes".to_owned())?);
            }
            return Ok(pui_function_21("Domain.CommitDuplicateNotes".to_owned())?);
        }
        return Ok(pui_function_23("Session.ReportConflictIfStale".to_owned())?);
    }
    if { let pui_local_200 = { let pui_local_183 = { let pui_local_185 = { let pui_local_187 = v_6576656e74.clone(); let pui_local_188 = "PointerUp".to_owned(); (pui_local_187 == pui_local_188) }; if ({ let pui_local_189 = pui_local_185.clone(); pui_local_189 }) { { let pui_local_190 = v_7068617365.clone(); let pui_local_191 = "armed".to_owned(); (pui_local_190 == pui_local_191) } } else { pui_local_185 } }; if ({ let pui_local_192 = pui_local_183.clone(); pui_local_192 }) { { let pui_local_193 = { let pui_local_199 = v_6d616368696e65.clone(); if pui_local_199 == "marquee".to_owned() { Data2::Marquee0 } else if pui_local_199 == "move".to_owned() { Data2::DataMove1 } else if pui_local_199 == "resize".to_owned() { Data2::Resize2 } else if pui_local_199 == "duplicate".to_owned() { Data2::Duplicate3 } else { return Err(DataError::InvalidType("checked union")); } }; let pui_local_194 = "resize".to_owned(); !(match pui_local_193 { Data2::Marquee0 => { "marquee".to_owned() == pui_local_194.clone() }, Data2::DataMove1 => { "move".to_owned() == pui_local_194.clone() }, Data2::Resize2 => { "resize".to_owned() == pui_local_194.clone() }, Data2::Duplicate3 => { "duplicate".to_owned() == pui_local_194.clone() } }) } } else { pui_local_183 } }; pui_local_200 } {
        return Ok(pui_function_25("Session.ClickReleaseSelection".to_owned())?);
    }
    if { let pui_local_216 = { let pui_local_201 = { let pui_local_203 = { let pui_local_205 = v_6576656e74.clone(); let pui_local_206 = "Cancel".to_owned(); (pui_local_205 == pui_local_206) }; if ({ let pui_local_207 = pui_local_203.clone(); pui_local_207 }) { v_616374697665.clone() } else { pui_local_203 } }; if !({ let pui_local_208 = pui_local_201.clone(); pui_local_208 }) { { let pui_local_209 = { let pui_local_211 = v_6576656e74.clone(); let pui_local_212 = "PointerUp".to_owned(); (pui_local_211 == pui_local_212) }; if ({ let pui_local_213 = pui_local_209.clone(); pui_local_213 }) { { let pui_local_214 = v_7068617365.clone(); let pui_local_215 = "armed".to_owned(); (pui_local_214 == pui_local_215) } } else { pui_local_209 } } } else { pui_local_201 } }; pui_local_216 } {
        return Ok(pui_function_27("idle".to_owned(), DataArray::new(vec![{ let _ = "Session.ClearGesturePreview".to_owned(); Data15::SessionClearGesturePreview0 }, { let _ = "Host.ReleasePointer".to_owned(); Data15::HostReleasePointer1 }]))?);
    }
    return Ok({ let pui_local_217 = v_7068617365.clone(); let pui_local_218 = v_616374697665.clone(); let pui_local_219 = DataArray::new(vec![]); Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_217, consumed: pui_local_218, effects: pui_local_219 }) });
}

fn pui_function_1(v_6e6578745068617365: String, v_65666665637473: DataArray<Data3>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_220 = v_6e6578745068617365.clone(); let pui_local_221 = true; let pui_local_222 = { let pui_local_223 = v_65666665637473.clone(); DataArray { identity: pui_local_223.identity.clone(), items: Rc::new(pui_local_223.iter().cloned().map(|value| Ok(match value { Data3::SessionBeginMarqueeGesture0 => { "Session.BeginMarqueeGesture".to_owned() }, Data3::HostCapturePointer1 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_220, consumed: pui_local_221, effects: pui_local_222 }) });
}

fn pui_function_2(v_6e6578745068617365: String, v_65666665637473: DataArray<Data4>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_226 = v_6e6578745068617365.clone(); let pui_local_227 = true; let pui_local_228 = { let pui_local_229 = v_65666665637473.clone(); DataArray { identity: pui_local_229.identity.clone(), items: Rc::new(pui_local_229.iter().cloned().map(|value| Ok(match value { Data4::SessionSelectIfUnselectedHit0 => { "Session.SelectIfUnselectedHit".to_owned() }, Data4::SessionBeginResizeGesture1 => { "Session.BeginResizeGesture".to_owned() }, Data4::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_226, consumed: pui_local_227, effects: pui_local_228 }) });
}

fn pui_function_3(v_6e6578745068617365: String, v_65666665637473: DataArray<String>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_233 = v_6e6578745068617365.clone(); let pui_local_234 = true; let pui_local_235 = { let pui_local_236 = v_65666665637473.clone(); DataArray { identity: pui_local_236.identity.clone(), items: Rc::new(pui_local_236.iter().cloned().map(|value| Ok(value)).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_233, consumed: pui_local_234, effects: pui_local_235 }) });
}

fn pui_function_4(v_6e6578745068617365: String, v_65666665637473: DataArray<Data5>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_237 = v_6e6578745068617365.clone(); let pui_local_238 = true; let pui_local_239 = { let pui_local_240 = v_65666665637473.clone(); DataArray { identity: pui_local_240.identity.clone(), items: Rc::new(pui_local_240.iter().cloned().map(|value| Ok(match value { Data5::SessionToggleHitNote0 => { "Session.ToggleHitNote".to_owned() }, Data5::SessionBeginMoveGesture1 => { "Session.BeginMoveGesture".to_owned() }, Data5::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_237, consumed: pui_local_238, effects: pui_local_239 }) });
}

fn pui_function_5(v_6e6578745068617365: String, v_65666665637473: DataArray<Data6>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_244 = v_6e6578745068617365.clone(); let pui_local_245 = true; let pui_local_246 = { let pui_local_247 = v_65666665637473.clone(); DataArray { identity: pui_local_247.identity.clone(), items: Rc::new(pui_local_247.iter().cloned().map(|value| Ok(match value { Data6::SessionExtendSelectionToHit0 => { "Session.ExtendSelectionToHit".to_owned() }, Data6::SessionBeginMoveGesture1 => { "Session.BeginMoveGesture".to_owned() }, Data6::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_244, consumed: pui_local_245, effects: pui_local_246 }) });
}

fn pui_function_6(v_6e6578745068617365: String, v_65666665637473: DataArray<Data7>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_251 = v_6e6578745068617365.clone(); let pui_local_252 = true; let pui_local_253 = { let pui_local_254 = v_65666665637473.clone(); DataArray { identity: pui_local_254.identity.clone(), items: Rc::new(pui_local_254.iter().cloned().map(|value| Ok(match value { Data7::SessionBeginMoveGesture0 => { "Session.BeginMoveGesture".to_owned() }, Data7::HostCapturePointer1 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_251, consumed: pui_local_252, effects: pui_local_253 }) });
}

fn pui_function_7(v_6e6578745068617365: String, v_65666665637473: DataArray<Data8>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_257 = v_6e6578745068617365.clone(); let pui_local_258 = true; let pui_local_259 = { let pui_local_260 = v_65666665637473.clone(); DataArray { identity: pui_local_260.identity.clone(), items: Rc::new(pui_local_260.iter().cloned().map(|value| Ok(match value { Data8::SessionSelectOnlyHitNote0 => { "Session.SelectOnlyHitNote".to_owned() }, Data8::SessionBeginMoveGesture1 => { "Session.BeginMoveGesture".to_owned() }, Data8::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_257, consumed: pui_local_258, effects: pui_local_259 }) });
}

fn pui_function_8(v_6e6578745068617365: String, v_65666665637473: DataArray<Data9>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_264 = v_6e6578745068617365.clone(); let pui_local_265 = true; let pui_local_266 = { let pui_local_267 = v_65666665637473.clone(); DataArray { identity: pui_local_267.identity.clone(), items: Rc::new(pui_local_267.iter().cloned().map(|value| Ok(match value { Data9::SessionToggleHitNote0 => { "Session.ToggleHitNote".to_owned() }, Data9::SessionBeginDuplicateGesture1 => { "Session.BeginDuplicateGesture".to_owned() }, Data9::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_264, consumed: pui_local_265, effects: pui_local_266 }) });
}

fn pui_function_9(v_6e6578745068617365: String, v_65666665637473: DataArray<Data10>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_271 = v_6e6578745068617365.clone(); let pui_local_272 = true; let pui_local_273 = { let pui_local_274 = v_65666665637473.clone(); DataArray { identity: pui_local_274.identity.clone(), items: Rc::new(pui_local_274.iter().cloned().map(|value| Ok(match value { Data10::SessionExtendSelectionToHit0 => { "Session.ExtendSelectionToHit".to_owned() }, Data10::SessionBeginDuplicateGesture1 => { "Session.BeginDuplicateGesture".to_owned() }, Data10::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_271, consumed: pui_local_272, effects: pui_local_273 }) });
}

fn pui_function_10(v_6e6578745068617365: String, v_65666665637473: DataArray<Data11>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_278 = v_6e6578745068617365.clone(); let pui_local_279 = true; let pui_local_280 = { let pui_local_281 = v_65666665637473.clone(); DataArray { identity: pui_local_281.identity.clone(), items: Rc::new(pui_local_281.iter().cloned().map(|value| Ok(match value { Data11::SessionSelectOnlyHitNote0 => { "Session.SelectOnlyHitNote".to_owned() }, Data11::SessionBeginDuplicateGesture1 => { "Session.BeginDuplicateGesture".to_owned() }, Data11::HostCapturePointer2 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_278, consumed: pui_local_279, effects: pui_local_280 }) });
}

fn pui_function_11(v_6e6578745068617365: String, v_65666665637473: DataArray<Data12>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_285 = v_6e6578745068617365.clone(); let pui_local_286 = true; let pui_local_287 = { let pui_local_288 = v_65666665637473.clone(); DataArray { identity: pui_local_288.identity.clone(), items: Rc::new(pui_local_288.iter().cloned().map(|value| Ok(match value { Data12::SessionBeginDuplicateGesture0 => { "Session.BeginDuplicateGesture".to_owned() }, Data12::HostCapturePointer1 => { "Host.CapturePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_285, consumed: pui_local_286, effects: pui_local_287 }) });
}

fn pui_function_12(v_6e6578745068617365: String, v_65666665637473: DataArray<String>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_291 = v_6e6578745068617365.clone(); let pui_local_292 = true; let pui_local_293 = { let pui_local_294 = v_65666665637473.clone(); DataArray { identity: pui_local_294.identity.clone(), items: Rc::new(pui_local_294.iter().cloned().map(|value| Ok(value)).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_291, consumed: pui_local_292, effects: pui_local_293 }) });
}

fn pui_function_13(v_6e6578745068617365: String, v_65666665637473: DataArray<String>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_295 = v_6e6578745068617365.clone(); let pui_local_296 = true; let pui_local_297 = { let pui_local_298 = v_65666665637473.clone(); DataArray { identity: pui_local_298.identity.clone(), items: Rc::new(pui_local_298.iter().cloned().map(|value| Ok(value)).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_295, consumed: pui_local_296, effects: pui_local_297 }) });
}

fn pui_function_14(v_6e6578745068617365: String, v_65666665637473: DataArray<String>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_299 = v_6e6578745068617365.clone(); let pui_local_300 = true; let pui_local_301 = { let pui_local_302 = v_65666665637473.clone(); DataArray { identity: pui_local_302.identity.clone(), items: Rc::new(pui_local_302.iter().cloned().map(|value| Ok(value)).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_299, consumed: pui_local_300, effects: pui_local_301 }) });
}

fn pui_function_15(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_16("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data16::SessionCommitMarqueeSelection0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data16::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data16::HostReleasePointer2 }]))?);
}

fn pui_function_16(v_6e6578745068617365: String, v_65666665637473: DataArray<Data16>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_303 = v_6e6578745068617365.clone(); let pui_local_304 = true; let pui_local_305 = { let pui_local_306 = v_65666665637473.clone(); DataArray { identity: pui_local_306.identity.clone(), items: Rc::new(pui_local_306.iter().cloned().map(|value| Ok(match value { Data16::SessionCommitMarqueeSelection0 => { "Session.CommitMarqueeSelection".to_owned() }, Data16::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data16::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_303, consumed: pui_local_304, effects: pui_local_305 }) });
}

fn pui_function_17(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_18("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data17::DomainCommitMoveNotes0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data17::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data17::HostReleasePointer2 }]))?);
}

fn pui_function_18(v_6e6578745068617365: String, v_65666665637473: DataArray<Data17>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_310 = v_6e6578745068617365.clone(); let pui_local_311 = true; let pui_local_312 = { let pui_local_313 = v_65666665637473.clone(); DataArray { identity: pui_local_313.identity.clone(), items: Rc::new(pui_local_313.iter().cloned().map(|value| Ok(match value { Data17::DomainCommitMoveNotes0 => { "Domain.CommitMoveNotes".to_owned() }, Data17::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data17::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_310, consumed: pui_local_311, effects: pui_local_312 }) });
}

fn pui_function_19(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_20("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data18::DomainCommitResizeNotes0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data18::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data18::HostReleasePointer2 }]))?);
}

fn pui_function_20(v_6e6578745068617365: String, v_65666665637473: DataArray<Data18>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_317 = v_6e6578745068617365.clone(); let pui_local_318 = true; let pui_local_319 = { let pui_local_320 = v_65666665637473.clone(); DataArray { identity: pui_local_320.identity.clone(), items: Rc::new(pui_local_320.iter().cloned().map(|value| Ok(match value { Data18::DomainCommitResizeNotes0 => { "Domain.CommitResizeNotes".to_owned() }, Data18::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data18::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_317, consumed: pui_local_318, effects: pui_local_319 }) });
}

fn pui_function_21(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_22("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data19::DomainCommitDuplicateNotes0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data19::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data19::HostReleasePointer2 }]))?);
}

fn pui_function_22(v_6e6578745068617365: String, v_65666665637473: DataArray<Data19>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_324 = v_6e6578745068617365.clone(); let pui_local_325 = true; let pui_local_326 = { let pui_local_327 = v_65666665637473.clone(); DataArray { identity: pui_local_327.identity.clone(), items: Rc::new(pui_local_327.iter().cloned().map(|value| Ok(match value { Data19::DomainCommitDuplicateNotes0 => { "Domain.CommitDuplicateNotes".to_owned() }, Data19::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data19::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_324, consumed: pui_local_325, effects: pui_local_326 }) });
}

fn pui_function_23(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_24("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data20::SessionReportConflictIfStale0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data20::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data20::HostReleasePointer2 }]))?);
}

fn pui_function_24(v_6e6578745068617365: String, v_65666665637473: DataArray<Data20>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_331 = v_6e6578745068617365.clone(); let pui_local_332 = true; let pui_local_333 = { let pui_local_334 = v_65666665637473.clone(); DataArray { identity: pui_local_334.identity.clone(), items: Rc::new(pui_local_334.iter().cloned().map(|value| Ok(match value { Data20::SessionReportConflictIfStale0 => { "Session.ReportConflictIfStale".to_owned() }, Data20::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data20::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_331, consumed: pui_local_332, effects: pui_local_333 }) });
}

fn pui_function_25(v_656666656374: String) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok(pui_function_26("idle".to_owned(), DataArray::new(vec![{ let _ = v_656666656374.clone(); Data21::SessionClickReleaseSelection0 }, { let _ = "Session.ClearGesturePreview".to_owned(); Data21::SessionClearGesturePreview1 }, { let _ = "Host.ReleasePointer".to_owned(); Data21::HostReleasePointer2 }]))?);
}

fn pui_function_26(v_6e6578745068617365: String, v_65666665637473: DataArray<Data21>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_338 = v_6e6578745068617365.clone(); let pui_local_339 = true; let pui_local_340 = { let pui_local_341 = v_65666665637473.clone(); DataArray { identity: pui_local_341.identity.clone(), items: Rc::new(pui_local_341.iter().cloned().map(|value| Ok(match value { Data21::SessionClickReleaseSelection0 => { "Session.ClickReleaseSelection".to_owned() }, Data21::SessionClearGesturePreview1 => { "Session.ClearGesturePreview".to_owned() }, Data21::HostReleasePointer2 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_338, consumed: pui_local_339, effects: pui_local_340 }) });
}

fn pui_function_27(v_6e6578745068617365: String, v_65666665637473: DataArray<Data15>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    return Ok({ let pui_local_345 = v_6e6578745068617365.clone(); let pui_local_346 = true; let pui_local_347 = { let pui_local_348 = v_65666665637473.clone(); DataArray { identity: pui_local_348.identity.clone(), items: Rc::new(pui_local_348.iter().cloned().map(|value| Ok(match value { Data15::SessionClearGesturePreview0 => { "Session.ClearGesturePreview".to_owned() }, Data15::HostReleasePointer1 => { "Host.ReleasePointer".to_owned() } })).collect::<Result<Vec<_>, DataError>>()?) } }; Rc::new(PlanNoteGestureResult { identity: Rc::new(()), next_phase: pui_local_345, consumed: pui_local_346, effects: pui_local_347 }) });
}

/// Calls selected pure source function "planNoteGesture"; component setup is not executed.
pub fn plan_note_gesture(argument_0: String, argument_1: String, argument_2: String, argument_3: Rc<PlanNoteGestureGuards>) -> Result<Rc<PlanNoteGestureResult>, DataError> {
    
    
    
    
    let result = pui_function_0(argument_0, argument_1, argument_2, argument_3)?;
    
    Ok(result)
}
