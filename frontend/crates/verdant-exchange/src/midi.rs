//! Standard MIDI Files 0/1 through midly. Key-release boundaries are preserved;
//! sustain and other synth controllers stay source events rather than guessed
//! singing parameters. SMPTE clocks and independent format-2 songs are refused.
use std::collections::{BTreeMap, BTreeSet};
use midly::{Format as SmfFormat, Header, MetaMessage, MidiMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind};
use serde_json::{Value, json};
use verdant_domain::{Note, Project};
use verdant_domain::model::{TempoPoint, TimeSignature};
use crate::{ExchangeError as E, ExchangeReport, ImportOptions, ExportOptions, Disposition as D, Result};
use crate::common::*;

fn frame(bytes: &[u8], options: &ImportOptions) -> Result<()> {
    if bytes.len() < 14 || &bytes[..4] != b"MThd" || bytes[4..8] != [0, 0, 0, 6] { return Err(E::at("/MThd", "SMF requires a six-byte header")); }
    let format = u16::from_be_bytes([bytes[8], bytes[9]]);
    let count = usize::from(u16::from_be_bytes([bytes[10], bytes[11]]));
    if format > 1 || count == 0 || (format == 0 && count != 1) || count > options.limits.max_tracks { return Err(E::at("/MThd", "unsupported format, track count or independent format-2 sequences")); }
    let mut at = 14usize;
    for _ in 0..count {
        let header = bytes.get(at..at.checked_add(8).ok_or_else(|| E::at("/MTrk", "chunk overflow"))?).ok_or_else(|| E::at("/MTrk", "truncated track header"))?;
        if &header[..4] != b"MTrk" { return Err(E::at("/MTrk", "unexpected SMF chunk")); }
        let len = u32::from_be_bytes(header[4..8].try_into().expect("four-byte length")) as usize;
        at = at.checked_add(8).and_then(|v| v.checked_add(len)).filter(|v| *v <= bytes.len()).ok_or_else(|| E::at("/MTrk", "track exceeds file bytes"))?;
    }
    if at != bytes.len() { return Err(E::at("/MTrk", "trailing chunks or bytes do not match header track count")); } Ok(())
}
fn append_raw<'a>(raw: &mut Vec<(i64, TrackEventKind<'a>)>, tick: i64, kind: TrackEventKind<'a>) { raw.push((tick, kind)); }
fn packet(events: &[(i64, TrackEventKind<'_>)], ppq: u16) -> Result<Vec<u8>> {
    let mut previous = 0i64; let mut track = Vec::with_capacity(events.len() + 1);
    for (tick, kind) in events { let delta = tick - previous;
        if !(0..=0x0fff_ffff).contains(&delta) { return Err(E::at("/midi/events", "SMF delta exceeds 28 bits")); }
        track.push(TrackEvent { delta: (delta as u32).into(), kind: *kind }); previous = *tick;
    }
    track.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) });
    let smf = Smf { header: Header::new(SmfFormat::SingleTrack, Timing::Metrical(ppq.into())), tracks: vec![track] };
    let mut bytes = Vec::new(); smf.write_std(&mut bytes)?; Ok(bytes)
}

pub(crate) fn import(bytes: &[u8], options: &ImportOptions, report: &mut ExchangeReport) -> Result<Project> {
    frame(bytes, options)?;
    let smf = Smf::parse(bytes).map_err(|e| E::at("/midi", e.to_string()))?;
    let ppq = match smf.header.timing { Timing::Metrical(v) if v.as_int() > 0 => i64::from(v.as_int()), _ => return Err(E::at("/MThd/division", "SMPTE/zero timing cannot supply an exact musical PPQ map")) };
    report.source_ppq = ppq;
    let mut project = source_project(ppq); project.tempo_map.clear(); project.time_signatures.clear();
    let mut tempos = BTreeMap::<i64, u32>::new(); let mut meters = BTreeMap::<i64, (u8, u8, u8, u8)>::new(); let mut event_count = 0usize; let mut note_count=0usize;
    for (ti, events) in smf.tracks.iter().enumerate() {
        let mut tick = 0i64; let mut track_name = format!("MIDI {}", ti + 1);
        let mut channels = BTreeMap::<u8, Vec<Note>>::new();
        let mut held = BTreeMap::<(u8, u8), (i64, u8)>::new();
        let mut raw = BTreeMap::<u8, Vec<(i64, TrackEventKind<'_>)>>::new();
        let mut meta = Vec::new(); let mut lyrics = BTreeMap::<i64, Vec<&[u8]>>::new();
        let mut bends = BTreeMap::<u8, Vec<(i64, f64)>>::new();
        let mut pitch_range = [2.0f64; 16]; let mut pitch_value = [0i16; 16];
        let mut rpn = [(127u8, 127u8); 16]; let mut ended = false;
        for event in events {
            event_count = event_count.checked_add(1).ok_or_else(|| E::at("/midi", "event count overflow"))?;
            if event_count > options.limits.max_events { return Err(E::at("/midi", "event limit exceeded")); }
            if ended { return Err(E::at(format!("/tracks/{ti}"), "events after EndOfTrack")); }
            tick = tick.checked_add(i64::from(event.delta.as_int())).filter(|v| *v <= MAX_TICK).ok_or_else(|| E::at("/midi", "absolute tick overflow"))?;
            match event.kind {
                TrackEventKind::Meta(MetaMessage::EndOfTrack) => ended = true,
                TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                    match std::str::from_utf8(name) { Ok(s) => track_name = s.to_string(), Err(_) => { append_raw(&mut meta, tick, event.kind); report.issue(format!("/tracks/{ti}/name"), D::RetainedUnsupported, hex::encode(name), "Non-UTF-8 MIDI track name retained as original bytes."); } }
                }
                TrackEventKind::Meta(MetaMessage::Tempo(v)) => {
                    let value = v.as_int(); if value == 0 { return Err(E::at("/tempo", "zero MIDI microseconds per quarter")); }
                    if tempos.insert(tick, value).is_some() { return Err(E::at("/tempo", "duplicate/conflicting tempo events at one tick")); }
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, clocks, thirty_seconds)) => {
                    if n == 0 || n > 64 || d > 6 { return Err(E::at("/meter", "invalid MIDI time signature")); }
                    if meters.insert(tick, (n, d, clocks, thirty_seconds)).is_some() { return Err(E::at("/meter", "duplicate time signature at one tick")); }
                    // MIDI clock/metronome and 32nd-note fields are metadata.
                    append_raw(&mut meta, tick, event.kind);
                }
                TrackEventKind::Meta(MetaMessage::Lyric(value)) => { lyrics.entry(tick).or_default().push(value); }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int(); let index = usize::from(ch); channels.entry(ch).or_default();
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() != 0 => {
                            let key = key.as_int();
                            if held.insert((ch, key), (tick, vel.as_int())).is_some() { return Err(E::at(format!("/tracks/{ti}/channel/{ch}"), "overlapping NoteOn for the same key has ambiguous NoteOff pairing")); }
                        }
                        MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                            let (start, velocity) = held.remove(&(ch, key.as_int())).ok_or_else(|| E::at(format!("/tracks/{ti}/channel/{ch}"), "unmatched NoteOff"))?;
                            let mut n = note(start, tick - start, i64::from(key.as_int()), String::new(), "/midi/notes")?;
                            set_velocity(&mut n, i64::from(velocity), "/midi/velocity")?; n.unknown.insert("midi_channel".into(), ch.into());
                            let release = match message { MidiMessage::NoteOff { vel, .. } => vel.as_int(), _ => 0 };
                            n.unknown.insert("midi_release_velocity".into(), release.into());
                            note_count=note_count.checked_add(1).ok_or_else(||E::at("/notes","note count overflow"))?;if note_count>options.limits.max_notes{return Err(E::at("/notes","note limit exceeded"));}
                            channels.get_mut(&ch).expect("channel exists").push(n);
                        }
                        MidiMessage::PitchBend { bend } => {
                            append_raw(raw.entry(ch).or_default(), tick, event.kind);
                            pitch_value[index] = bend.as_int();
                            let value = f64::from(bend.as_int()) / 8192.0 * pitch_range[index] * 100_000.0;
                            let nodes = bends.entry(ch).or_default();
                            if nodes.last().is_some_and(|p| p.0 == tick) { nodes.pop(); } nodes.push((tick, value));
                        }
                        MidiMessage::Controller { controller, value } => {
                            let (cc, value) = (controller.as_int(), value.as_int());
                            match cc {
                                101 => rpn[index].0 = value, 100 => rpn[index].1 = value,
                                6 if rpn[index] == (0, 0) => { pitch_range[index] = f64::from(value) + pitch_range[index].fract(); },
                                38 if rpn[index] == (0, 0) => { pitch_range[index] = pitch_range[index].floor() + f64::from(value) / 100.0; }, _ => {},
                            }
                            if matches!(cc, 6 | 38) && rpn[index] == (0, 0) {
                                let value = f64::from(pitch_value[index]) / 8192.0 * pitch_range[index] * 100_000.0;
                                let nodes = bends.entry(ch).or_default(); if nodes.last().is_some_and(|p| p.0 == tick) { nodes.pop(); } nodes.push((tick, value));
                            }
                            append_raw(raw.entry(ch).or_default(), tick, event.kind);
                            report.issue(format!("/tracks/{ti}/channels/{ch}/cc/{cc}/{tick}"), D::RetainedUnsupported, value,
                                "MIDI controller retained as a source event; note key-release boundaries do not guess pedal/synth semantics.");
                        }
                        _ => { append_raw(raw.entry(ch).or_default(), tick, event.kind); report.issue(format!("/tracks/{ti}/channels/{ch}/events/{tick}"), D::RetainedUnsupported, format!("{message:?}"), "MIDI channel event has no singing-editor parameter equivalent."); }
                    }
                }
                _ => { append_raw(&mut meta, tick, event.kind); report.issue(format!("/tracks/{ti}/meta/{tick}"), D::RetainedUnsupported, format!("{:?}", event.kind), "Non-score MIDI event retained for compatible SMF roundtrip."); }
            }
        }
        if !ended || !held.is_empty() { return Err(E::at(format!("/tracks/{ti}"), "missing EndOfTrack or unterminated notes")); }
        for (at, values) in lyrics {
            let candidates: Vec<_> = channels.iter().flat_map(|(ch, notes)| notes.iter().enumerate().filter(move |(_, n)| n.start_tick == at).map(move |(i, _)| (*ch, i))).collect();
            if values.len() == 1 && candidates.len() == 1 && std::str::from_utf8(values[0]).is_ok() {
                let (ch, i) = candidates[0]; channels.get_mut(&ch).expect("channel")[i].lyric = std::str::from_utf8(values[0]).expect("validated UTF-8").into();
            } else {
                for value in values { append_raw(&mut meta, at, TrackEventKind::Meta(MetaMessage::Lyric(value))); }
                report.issue(format!("/tracks/{ti}/lyrics/{at}"), D::RetainedUnsupported, candidates.len(), "Lyric does not identify exactly one UTF-8 note; source events retained without guessing a note.");
            }
        }
        let mut all_notes = Vec::new();
        for (channel, mut notes) in channels {
            if let Some(nodes) = bends.get(&channel) { apply_curve(&mut notes, nodes, Curve::Pitch, false, "/midi/pitch_bend")?; }
            all_notes.extend(notes);
        }
        all_notes.sort_by_key(|n| (n.start_tick, n.midi_note));
        let mut track = source_track(track_name); track.notes = all_notes;
        track.unknown.insert("midi_source_track".into(), ti.into());
        if options.preserve_extensions {
            let mut retained: Vec<_> = raw.into_values().flatten().collect(); retained.extend(meta);
            retained.sort_by_key(|e| e.0);
            track.unknown.insert("exchange_midi".into(), json!({"ppq":ppq,"smf_hex":hex::encode(packet(&retained, ppq as u16)?),"curve_snapshot":curve_snapshot(track.notes.iter(),0)}));
        }
        project.tracks.push(track);
    }
    if !tempos.contains_key(&0) { tempos.insert(0, 500_000); }
    for (tick, us) in tempos {
        let bpm = 60_000_000.0 / f64::from(us); let micro = micro_bpm(bpm, "/tempo", report)?;
        let mut unknown = BTreeMap::new(); unknown.insert("midi_microseconds_per_quarter".into(), us.into());
        project.tempo_map.push(TempoPoint { tick, micro_bpm: micro, unknown });
    }
    if !meters.contains_key(&0) { meters.insert(0, (4, 2, 24, 8)); }
    for (tick, (n, d, clocks, thirty_seconds)) in meters {
        let unknown = BTreeMap::from([("midi_clocks_per_click".into(),clocks.into()),("midi_notated_32nds".into(),thirty_seconds.into())]);
        project.time_signatures.push(TimeSignature { tick, numerator: u32::from(n), denominator: 1u32 << d, unknown });
    }
    // Use an exact common multiple when the source meter's beat would have a
    // fractional tick. Every source boundary is multiplied by the same factor.
    let mut factor = 1i64;
    for m in &project.time_signatures { let denominator = i64::from(m.denominator); while ppq.checked_mul(factor).and_then(|v| v.checked_mul(4)).ok_or_else(|| E::at("/ppq", "PPQ overflow"))? % denominator != 0 { factor = factor.checked_mul(2).ok_or_else(|| E::at("/ppq", "PPQ overflow"))?; } }
    if factor > 1 {
        project.ppq *= factor;
        for point in &mut project.tempo_map { point.tick = scale(point.tick, ppq, project.ppq, "/tempo")?; }
        for point in &mut project.time_signatures { point.tick = scale(point.tick, ppq, project.ppq, "/meter")?; }
        for t in &mut project.tracks { for n in &mut t.notes { n.start_tick = scale(n.start_tick, ppq, project.ppq, "/notes")?; n.duration_tick = scale(n.duration_tick, ppq, project.ppq, "/notes")?; for p in &mut n.curves.pitch_millicent_points { p.tick *= factor; } } }
        report.issue("/ppq", D::Metadata, ppq, format!("All ticks and PPQ multiplied by {factor} to represent meter beats exactly."));
    }
    project.extensions.insert("exchange_midi".into(),json!({"source_ppq":ppq,"format":if smf.header.format==SmfFormat::SingleTrack {0}else{1}}));
    // The snapshot uses the promoted common scale; retained SMF events keep
    // their original PPQ and are scaled exactly at export.
    for track in &mut project.tracks { let snapshot=curve_snapshot(track.notes.iter(),0);
        if let Some(meta)=track.unknown.get_mut("exchange_midi").and_then(Value::as_object_mut) { meta.insert("curve_snapshot".into(),snapshot); }
    }
    Ok(project)
}

pub(crate) fn export(project: &Project, options: &ExportOptions, report: &mut ExchangeReport) -> Result<Vec<u8>> {
    let ppq = options.midi_ppq.map(i64::from).unwrap_or_else(|| if project.ppq <= 32767 { project.ppq } else { 960 });
    bounded(ppq, 1, 32767, "/options/midi_ppq")?;
    let mut conductor = Vec::<(i64, u8, TrackEventKind<'_>)>::new();
    for p in &project.tempo_map {
        let exact = 60_000_000_000_000.0 / p.micro_bpm as f64; let rounded = exact.round();
        if !(1.0..=16_777_215.0).contains(&rounded) { return Err(E::at("/tempo_map", "tempo cannot fit MIDI's 24-bit microseconds")); }
        let original = p.unknown.get("midi_microseconds_per_quarter").and_then(Value::as_u64).filter(|v| *v > 0 && *v <= 0xff_ffff && (60_000_000_000_000.0 / *v as f64).round() as u64 == p.micro_bpm);
        let us = original.unwrap_or(rounded as u64);
        if original.is_none() && exact != rounded { report.issue(format!("/tempo_map/{}", p.tick), D::Quantized, p.micro_bpm, "SMF stores integer microseconds per quarter note."); }
        conductor.push((scale(p.tick, project.ppq, ppq, "/tempo")?, 0, TrackEventKind::Meta(MetaMessage::Tempo((us as u32).into()))));
    }
    for m in &project.time_signatures {
        if m.numerator > 255 || m.denominator > 128 { return Err(E::at("/meter", "meter does not fit MIDI byte fields")); }
        conductor.push((scale(m.tick, project.ppq, ppq, "/meter")?, 1, TrackEventKind::Meta(MetaMessage::TimeSignature(m.numerator as u8, m.denominator.trailing_zeros() as u8,
            m.unknown.get("midi_clocks_per_click").and_then(Value::as_u64).filter(|v|*v<=255).unwrap_or(24) as u8,
            m.unknown.get("midi_notated_32nds").and_then(Value::as_u64).filter(|v|*v<=255).unwrap_or(8) as u8))));
    }
    let selected: Vec<_> = selected(project, options).collect();
    if selected.len() >= u16::MAX as usize { return Err(E::at("/tracks", "SMF track count overflow")); }
    let retained_bytes: Vec<Option<Vec<u8>>> = selected.iter().map(|(_, track)| {
        if !options.preserve_extensions { return Ok(None); }
        track.unknown.get("exchange_midi").and_then(|v| v.get("smf_hex")).and_then(Value::as_str).map(|s| {
            if s.len() > options.limits.max_file_bytes.saturating_mul(2) { return Err(E::at("/exchange_midi", "retained event byte limit")); }
            hex::decode(s).map_err(|_| E::at("/exchange_midi", "invalid retained SMF hex"))
        }).transpose()
    }).collect::<Result<_>>()?;
    let mut tracks = Vec::new();
    let mut conductor = Some(conductor);
    for (index, (ti, track)) in selected.iter().enumerate() {
        if track.gain_millidecibels!=0||track.pan_permille!=0||track.muted||track.solo {report.issue(format!("/tracks/{ti}/mixer"),D::Unsupported,json!({"gain":track.gain_millidecibels,"pan":track.pan_permille,"muted":track.muted,"solo":track.solo}),"Native track mixer state has no exact singing-independent SMF representation; it is not relabeled as General MIDI controllers.");}
        let mut events = conductor.take().unwrap_or_default();
        events.push((0, 0, TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes()))));
        let retain_pitch = options.preserve_extensions && retained_bytes[index].is_some() &&
            track.unknown.get("exchange_midi").and_then(|v|v.get("curve_snapshot")) == Some(&curve_snapshot(track.notes.iter(),0));
        compatible_channel_notes(track)?;
        let default_channel = track.unknown.get("midi_channel").and_then(Value::as_u64).unwrap_or(0);
        bounded(default_channel as i64, 0, 15, "/midi_channel")?;
        let mut pitch = BTreeMap::<u8, BTreeMap<i64, i64>>::new();
        for (ni, n) in track.notes.iter().enumerate() {
            let ch = n.unknown.get("midi_channel").and_then(Value::as_u64).unwrap_or(default_channel);
            bounded(ch as i64, 0, 15, "/notes/midi_channel")?;
            let ch = ch as u8; let at = scale(n.start_tick, project.ppq, ppq, "/notes/start")?; let end = scale(n.end_tick(), project.ppq, ppq, "/notes/end")?;
            let vel = velocity(n)?;
            if vel == 0 { return Err(E::at("/notes/midi_velocity", "SMF NoteOn velocity zero is NoteOff and cannot encode a sounding note")); }
            events.push((at, 4, TrackEventKind::Midi { channel: ch.into(), message: MidiMessage::NoteOn { key: (n.midi_note as u8).into(), vel: vel.into() } }));
            events.push((end, 1, TrackEventKind::Midi { channel: ch.into(), message: MidiMessage::NoteOff { key: (n.midi_note as u8).into(), vel: (n.unknown.get("midi_release_velocity").and_then(Value::as_u64).filter(|v| *v <= 127 && options.preserve_extensions).unwrap_or(0) as u8).into() } }));
            if !n.lyric.is_empty() { events.push((at, 3, TrackEventKind::Meta(MetaMessage::Lyric(n.lyric.as_bytes())))); }
            if !n.pronunciation.phonemes.is_empty() { report.issue(format!("/tracks/{ti}/notes/{ni}/pronunciation"), D::Unsupported, json!(n.pronunciation), "Standard MIDI lyric events have no phoneme override field."); }
            if !n.curves.gain_millidecibel_points.is_empty() || !n.curves.expression_points.is_empty() { report.issue(format!("/tracks/{ti}/notes/{ni}/curves"), D::Unsupported, json!(n.curves), "Singing dynamics and timbre controls are not guessed as General MIDI controllers."); }
            let points = pitch.entry(ch).or_default();
            let values: Vec<_> = if n.curves.pitch_millicent_points.is_empty() { vec![(0, i64::from(n.detune_millicents))] }
                else { n.curves.pitch_millicent_points.iter().map(|p| Ok((p.tick, p.millicents.checked_add(i64::from(n.detune_millicents)).ok_or_else(|| E::at("/pitch", "pitch overflow"))?))).collect::<Result<_>>()? };
            for (relative, value) in values {
                let tick = scale(n.start_tick + relative, project.ppq, ppq, "/pitch")?;
                // At an adjacent note boundary the new note controls the bend.
                if relative == n.duration_tick { continue; }
                if let Some(old) = points.insert(tick, value) { if old != value { return Err(E::at("/pitch", "simultaneous notes require incompatible channel-wide pitch bends")); } }
            }
            if n.curves.interpolation("PIT") != verdant_domain::editing::Interpolation::Hold && n.curves.pitch_millicent_points.len() > 1 {
                report.issue(format!("/tracks/{ti}/notes/{ni}/curves/pitch/interpolation"), D::Unsupported, json!(n.curves.interpolation("PIT")), "SMF bends are discrete events; only authored controller nodes export, without guessed interpolation density.");
            }
        }
        if let Some(bytes) = &retained_bytes[index] {
            let raw = Smf::parse(bytes).map_err(|e| E::at("/exchange_midi", e.to_string()))?;
            let from = match raw.header.timing { Timing::Metrical(v) if v.as_int() > 0 => i64::from(v.as_int()), _ => return Err(E::at("/exchange_midi", "invalid retained timing")) };
            let mut count = 0usize;
            for raw_track in raw.tracks { let mut tick = 0i64;
                for event in raw_track {
                    count += 1; if count > options.limits.max_events { return Err(E::at("/exchange_midi", "retained event limit")); }
                    tick = tick.checked_add(i64::from(event.delta.as_int())).ok_or_else(|| E::at("/exchange_midi", "tick overflow"))?;
                    let keep = match event.kind {
                        TrackEventKind::Meta(MetaMessage::EndOfTrack | MetaMessage::Tempo(_) | MetaMessage::TimeSignature(..) | MetaMessage::TrackName(_)) => false,
                        TrackEventKind::Midi { message: MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }, .. } => false,
                        TrackEventKind::Midi { message: MidiMessage::PitchBend { .. }, .. } => retain_pitch,
                        TrackEventKind::Midi { message: MidiMessage::Controller { controller, .. }, .. } if matches!(controller.as_int(), 100 | 101 | 6 | 38) => retain_pitch,
                        _ => true,
                    };
                    if keep { events.push((scale(tick, from, ppq, "/exchange_midi")?, 2, event.kind)); }
                }
            }
        }
        for (ch, points) in pitch.into_iter().filter(|_| !retain_pitch) {
            let max = points.values().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            let range = (max / 100_000 + 1).max(2);
            if range > 127 { return Err(E::at("/pitch", "pitch bend exceeds MIDI RPN sensitivity range")); }
            for (cc, value) in [(101, 0), (100, 0), (6, range as u8), (38, 0), (101, 127), (100, 127)] {
                events.push((0, 0, TrackEventKind::Midi { channel: ch.into(), message: MidiMessage::Controller { controller: cc.into(), value: value.into() } }));
            }
            for (tick, value) in points {
                let raw = (value as f64 / (range as f64 * 100_000.0) * 8192.0).round().clamp(-8192.0, 8191.0) as i16;
                if (f64::from(raw) / 8192.0 * range as f64 * 100_000.0).round() as i64 != value { report.issue(format!("/tracks/{ti}/pitch/{tick}"), D::Quantized, value, "MIDI bend has 14-bit resolution."); }
                events.push((tick, 2, TrackEventKind::Midi { channel: ch.into(), message: MidiMessage::PitchBend { bend: PitchBend::from_int(raw) } }));
            }
        }
        for part in parts(project, track)? { if project.extensions.contains_key(verdant_domain::parts::KEY) { report.issue(format!("/tracks/{ti}/parts/{}", part.id), D::Unsupported, part.name, "SMF carries a continuous track, without editable part identities."); } }
        tracks.push(finish(events)?);
    }
    if tracks.is_empty() { tracks.push(finish(conductor.take().unwrap_or_default())?); }
    let format = if tracks.len() == 1 { SmfFormat::SingleTrack } else { SmfFormat::Parallel };
    let smf = Smf { header: Header::new(format, Timing::Metrical((ppq as u16).into())), tracks };
    let mut bytes = Vec::new(); smf.write_std(&mut bytes)?; Ok(bytes)
}
fn finish<'a>(mut events: Vec<(i64, u8, TrackEventKind<'a>)>) -> Result<Vec<TrackEvent<'a>>> {
    events.sort_by_key(|e| (e.0, e.1)); let mut previous = 0; let mut result = Vec::with_capacity(events.len() + 1);
    for (tick, _, kind) in events { let delta = tick - previous;
        if !(0..=0x0fff_ffff).contains(&delta) { return Err(E::at("/midi", "event delta exceeds SMF's 28-bit limit")); }
        result.push(TrackEvent { delta: (delta as u32).into(), kind }); previous = tick;
    }
    result.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) }); Ok(result)
}

/// Channel bends are shared by every active key. Reject a score that would
/// otherwise silently retune an overlapping note, or pair NoteOff ambiguously.
fn compatible_channel_notes(track: &verdant_domain::Track) -> Result<()> {
    let mut channels = BTreeMap::<u64, Vec<&Note>>::new();
    for n in &track.notes { let ch=n.unknown.get("midi_channel").and_then(Value::as_u64).unwrap_or_else(||track.unknown.get("midi_channel").and_then(Value::as_u64).unwrap_or(0)); channels.entry(ch).or_default().push(n); }
    for notes in channels.values_mut() { notes.sort_by_key(|n|n.start_tick); let mut active:Vec<&Note>=Vec::new();
        for n in notes { active.retain(|old|old.end_tick()>n.start_tick);
            for old in &active { if old.midi_note==n.midi_note { return Err(E::at("/notes","overlapping notes on the same MIDI channel/key have ambiguous NoteOff pairing")); }
                let from=n.start_tick.max(old.start_tick);let to=n.end_tick().min(old.end_tick());
                let mut times=BTreeSet::from([from]);
                for note in [*old,*n] { for p in &note.curves.pitch_millicent_points { let at=note.start_tick+p.tick;if at>=from&&at<to{times.insert(at);} } }
                let at=|note:&Note,t:i64|->Result<i64>{let offset=note.curves.pitch_millicent_points.iter().take_while(|p|p.tick<=t-note.start_tick).last().map_or(0,|p|p.millicents);offset.checked_add(i64::from(note.detune_millicents)).ok_or_else(||E::at("/pitch","pitch overflow"))};
                for t in times { if at(old,t)?!=at(n,t)?{return Err(E::at("/pitch","overlapping notes require incompatible channel-wide pitch bends; use distinct MIDI channels"));} }
            }active.push(n);
        }
    }Ok(())
}
