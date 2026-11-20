//! Controller-reference allocation before native spectral/pitch MLPG.
//! This retains the originally selected interior singing sustain state and all
//! five native statistical states. State indices are not certified phonemes.
use anyhow::{ensure,Context,Result};
use serde_json::json;
use super::{PreparedSource,selected_array};

pub const POLICY_ID:&str="en-aa-terminal-sustain-native-release-context-v3";

pub(super) fn protect_terminal_nucleus(source:&mut PreparedSource,authored_language:u32,
    continuation_segments:usize,terminal_note_frames:usize)->Result<()> {
    let policy=&source.details["state_duration_allocation"]["final_policy"];
    let chosen=policy["sustain_state_index_0based"].as_u64().map(|value|value as usize);
    if authored_language!=2 || policy["native_key"]!="en:aa" || continuation_segments<2
        || policy["mode"]!="singing" || !chosen.is_some_and(|value|(1..=3).contains(&value)) {
        return Ok(());
    }
    let chosen=chosen.expect("validated selected sustain state");
    ensure!(source.phone_id==Some(174),"English aa reference policy requires native current phone 174");
    let old=&source.state_frames;
    ensure!(old.len()==5 && old.iter().all(|&count|count>=1) && old.iter().sum::<usize>()==source.frames,
        "native preparation state allocation is inconsistent");
    ensure!((1..=source.frames).contains(&terminal_note_frames),"terminal note must have a positive native frame budget");
    if !source.authored_voiced {return Ok(());}
    let first=source.frames-terminal_note_frames;
    let sustain_first=old[..chosen].iter().sum::<usize>();
    let sustain_end=old[..=chosen].iter().sum::<usize>();
    let intersection=sustain_end.min(source.frames).saturating_sub(first.max(sustain_first));
    if intersection>0 {return Ok(());}
    let visible_tail_budget=terminal_note_frames/2;
    let tail_budget=old[chosen+1..].iter().sum::<usize>();
    let lookahead=tail_budget.checked_sub(visible_tail_budget).filter(|&value|value>0)
        .context("terminal repair has no native release context to retain")?;
    let context_frames=source.frames.checked_add(lookahead).context("native release context frame overflow")?;
    let means=selected_array(&source.selected,"state_duration_means",&[5])?;
    let variances=selected_array(&source.selected,"state_duration_variances",&[5])?;
    let mut updated=old.clone();
    updated[chosen]=updated[chosen].checked_add(lookahead).context("native sustain context frame overflow")?;
    ensure!(updated.iter().sum::<usize>()==context_frames && updated.iter().all(|&count|count>=1)
        && updated[..chosen]==old[..chosen] && updated[chosen+1..]==old[chosen+1..],
        "terminal nucleus allocation changed declared context clock or other native state durations");
    let new_end=updated[..=chosen].iter().sum::<usize>();
    let after_intersection=new_end.min(source.frames).saturating_sub(first.max(sustain_first));
    ensure!(after_intersection>=terminal_note_frames-visible_tail_budget,
        "terminal continuation lacks its declared sustain-state majority");
    let evidence=json!({"policy":POLICY_ID,"applied":true,"authored_language_id":2,
        "native_key":"en:aa","native_phone_id":174,"source_frames":source.frames,
        "native_context_frames":context_frames,"release_lookahead_frames":lookahead,
        "emitted_source_frame_indices":[0,source.frames],
        "terminal_note_range":[first,source.frames],"continuation_segments":continuation_segments,
        "selected_sustain_state":chosen,"old_state_counts":old,"new_state_counts":updated,
        "old_terminal_sustain_intersection_frames":intersection,"new_terminal_sustain_intersection_frames":after_intersection,
        "visible_tail_frame_budget":visible_tail_budget,"tail_frame_budget":tail_budget,
        "transferred_to_sustain_frames":0,"additional_context_frames_in_sustain":lookahead,
        "all_original_native_release_durations_retained":true,
        "selected_duration_means":means,"selected_duration_variances":variances,
        "selected_statistical_state_records_unchanged":true,"all_five_states_retained":true,
        "emitted_frames_changed":false,"native_context_frames_changed":true,
        "required_next_step":"rerun native spectral and pitch MLPG on complete context, then retain emitted source frame indices",
        "phonetic_state_labels_certified":false});
    source.state_frames=updated;
    source.details["state_frames"]=json!(source.state_frames);
    source.details["state_duration_allocation"]["counts"]=json!(source.state_frames);
    source.details["state_duration_allocation"]["count_sum"]=json!(context_frames);
    source.details["state_duration_allocation"]["final_policy"]["terminal_nucleus_reference"]=evidence.clone();
    source.details["singing_state_duration"]["terminal_nucleus_reference"]=evidence.clone();
    source.details["terminal_nucleus_reference"]=evidence;
    // `frames` remains the emitted score budget. The complete five-state
    // context is expanded and passed through both original MLPG operations,
    // then the extra release indices are removed. No waveform is copied.
    Ok(())
}

/// Concatenate complete native contexts without changing emitted source or
/// following-phone clocks. This also handles a protected source inside a run.
pub(super) fn context_clock(sources:&[(usize,usize)],emitted_frames:usize)
    ->Result<(usize,Vec<usize>)> {
    let mut retained=Vec::with_capacity(emitted_frames);
    let mut offset=0usize;
    for &(emitted,context) in sources {
        ensure!(emitted>0 && context>=emitted,"native context is shorter than its emitted source");
        let emitted_end=offset.checked_add(emitted).context("native emitted frame overflow")?;
        retained.extend(offset..emitted_end);
        offset=offset.checked_add(context).context("native context frame overflow")?;
    }
    ensure!(retained.len()==emitted_frames,"native context changed the emitted run clock");
    Ok((offset,retained))
}

pub(super) fn retain_rows<T:Clone>(values:Vec<T>,width:usize,context_frames:usize,
    retained:&[usize])->Result<Vec<T>> {
    ensure!(width>0 && context_frames.checked_mul(width)==Some(values.len()),
        "native context array shape mismatch");
    ensure!(retained.iter().all(|&frame|frame<context_frames)
        && retained.windows(2).all(|pair|pair[0]<pair[1]),"invalid native retained frame map");
    if retained.len()==context_frames {return Ok(values);}
    let mut output=Vec::with_capacity(retained.len().checked_mul(width).context("native retained shape overflow")?);
    for &frame in retained {output.extend_from_slice(&values[frame*width..(frame+1)*width]);}
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arrays,NumericArray,SelectedPhone};

    fn source()->PreparedSource {
        let counts=vec![5,3,109,4,4];
        let policy=json!({"native_key":"en:aa","mode":"singing","sustain_state_index_0based":2});
        let mut arrays=Arrays::new();
        arrays.insert("state_duration_means".into(),NumericArray::new(vec![5],"float64",vec![5.,3.,7.,4.,4.]).unwrap());
        arrays.insert("state_duration_variances".into(),NumericArray::new(vec![5],"float64",vec![1.;5]).unwrap());
        PreparedSource {frames:125,state_frames:counts.clone(),selected:SelectedPhone{arrays,metadata:json!({})},
            phone_id:Some(174),language:Some(3),native_key:Some("en:aa".into()),phonetic_class:"vowel".into(),
            phonemic_length:1,authored_voiced:true,details:json!({"state_frames":counts,
                "state_duration_allocation":{"counts":counts,"count_sum":125,"final_policy":policy},
                "singing_state_duration":policy})}
    }

    #[test]
    fn terminal_reference_retains_all_release_durations_and_emitted_clock()->Result<()> {
        let mut source=source();
        let selected=serde_json::to_value(&source.selected.arrays)?;
        protect_terminal_nucleus(&mut source,2,2,5)?;
        assert_eq!(source.state_frames,[5,3,115,4,4]);
        assert_eq!(source.state_frames.iter().sum::<usize>(),131);
        assert_eq!(source.frames,125);
        assert_eq!(serde_json::to_value(&source.selected.arrays)?,selected);
        assert_eq!(source.details["terminal_nucleus_reference"]["new_terminal_sustain_intersection_frames"],3);
        Ok(())
    }

    #[test]
    fn ordinary_notes_other_languages_and_existing_sustain_are_unchanged()->Result<()> {
        for (language,segments,last_frames) in [(1,2,5),(3,2,5),(2,1,5),(2,2,20)] {
            let mut source=source();let old=source.state_frames.clone();let details=source.details.clone();
            protect_terminal_nucleus(&mut source,language,segments,last_frames)?;
            assert_eq!(source.state_frames,old);assert_eq!(source.details,details);
        }
        let mut source=source();source.authored_voiced=false;let old=source.state_frames.clone();
        protect_terminal_nucleus(&mut source,2,2,5)?;assert_eq!(source.state_frames,old);
        Ok(())
    }

    #[test]
    fn release_context_is_removed_before_following_source_without_clock_shift()->Result<()> {
        let (frames,retained)=context_clock(&[(5,8),(3,3),(4,6)],12)?;
        assert_eq!(frames,17);
        assert_eq!(retained,[0,1,2,3,4,8,9,10,11,12,13,14]);
        let full:Vec<i32>=(0..34).collect();
        assert_eq!(retain_rows(full,2,frames,&retained)?,
            [0,1,2,3,4,5,6,7,8,9,16,17,18,19,20,21,22,23,24,25,26,27,28,29]);
        assert!(context_clock(&[(5,4)],5).is_err());
        assert!(context_clock(&[(5,8)],4).is_err());
        assert!(retain_rows(vec![1,2,3],1,3,&[0,3]).is_err());
        assert!(retain_rows(vec![1,2,3],1,3,&[1,0]).is_err());
        Ok(())
    }
}
