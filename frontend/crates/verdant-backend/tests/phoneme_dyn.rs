use verdant_backend::post::{TimeMap, dyn_envelope_for_phrase, dyn_envelope_with_time_map};
use verdant_backend::wire::{PhonemeTimingNote, PhonemeTimingTrace};
use verdant_domain::model::GainPoint;
use verdant_domain::{Note, TempoMap};

fn gain(note: &mut Note, points: &[(i64, i64)]) {
    note.curves.gain_millidecibel_points = points.iter().enumerate().map(|(i, &(tick, millidecibels))| GainPoint {
        id: format!("gain-{i}").as_str().into(), tick, millidecibels, unknown: Default::default(),
    }).collect();
}

fn anchor(note: &Note, origin: f64, first: usize, prefix: f64, trim: f64) -> PhonemeTimingNote {
    let tempo = TempoMap::constant(120.0);
    PhonemeTimingNote {
        note_id: note.id.as_str().into(),
        note_onset_seconds: tempo.seconds_at(note.start_tick as f64) - origin,
        note_end_seconds: tempo.seconds_at(note.end_tick() as f64) - origin,
        first_segment_index: first, nucleus_segment_index: Some(first + usize::from(prefix > 0.0)),
        nucleus_delay_seconds: 0.0,
        end_segment_index: first + 1 + usize::from(prefix > 0.0), prefix_seconds: prefix, tail_trim_seconds: trim,
    }
}

fn fixture() -> (TempoMap, Note, Note, PhonemeTimingTrace) {
    let tempo = TempoMap::constant(120.0);
    let mut a = Note::new(0, 1920, 60);
    let mut b = Note::new(1920, 1920, 64);
    gain(&mut a, &[(0, -60000)]);
    gain(&mut b, &[(0, -6000), (960, -18000)]);
    let timing = PhonemeTimingTrace { method: "vowel-anchor-v1".into(), notes: vec![
        anchor(&a, 0.0, 0, 0.0, 0.1), anchor(&b, 0.0, 1, 0.1, 0.0),
    ] };
    (tempo, a, b, timing)
}

#[test]
fn prefix_uses_its_own_note_not_the_muted_previous_note() {
    let (tempo, a, b, timing) = fixture();
    let map = TimeMap::new(vec![0., 2.], vec![0., 2.]).unwrap();
    let env = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2001, Some(&map), Some(&timing)).unwrap().unwrap();
    assert!((env[899] - 0.001).abs() < 1e-8);
    assert!((env[900] - 0.5011872).abs() < 1e-6, "next note owns its 100 ms prefix");
    assert_eq!(env[900], env[999], "prefix holds the first DYN value");
    assert!((env[1250] - 0.25118864).abs() < 1e-6, "curve keeps original score ticks after onset");
    assert_eq!(env[2000], 0., "no resampling endpoint escape");
}

#[test]
fn quiet_current_note_also_quiets_its_prefix() {
    let (tempo, mut a, mut b, timing) = fixture();
    a.curves.gain_millidecibel_points.clear();
    gain(&mut b, &[(0, -60000)]);
    let map = TimeMap::new(vec![0., 2.], vec![0., 2.]).unwrap();
    let env = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, Some(&map), Some(&timing)).unwrap().unwrap();
    assert_eq!(env[899], 1.);
    for &value in &env[900..] { assert!((value - 0.001).abs() < 1e-8); }
}

#[test]
fn worker_phrase_split_does_not_change_prefix_dyn() {
    let (tempo, a, b, timing) = fixture();
    let map = TimeMap::new(vec![0., 2.], vec![0., 2.]).unwrap();
    let together = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, Some(&map), Some(&timing)).unwrap().unwrap();
    let split = PhonemeTimingTrace { method: timing.method.clone(), notes: vec![anchor(&b, 0.9, 0, 0.1, 0.0)] };
    let split_map = TimeMap::new(vec![0., 1.1], vec![0., 1.1]).unwrap();
    let separate = dyn_envelope_for_phrase(&[&b], &tempo, 0.9, 1000, 1100, Some(&split_map), Some(&split)).unwrap().unwrap();
    for (i, (&joined, &split)) in together[900..].iter().zip(&separate).enumerate() {
        assert!((joined - split).abs() < 1e-7, "sample {i} differs across a worker phrase split");
    }
}

#[test]
fn phoneme_ownership_follows_the_verified_time_map() {
    let (tempo, a, b, timing) = fixture();
    let map = TimeMap::new(vec![0., 0.895, 0.995, 1.995], vec![0., 0.9, 1., 2.]).unwrap();
    let env = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 1996, Some(&map), Some(&timing)).unwrap().unwrap();
    assert!((env[894] - 0.001).abs() < 1e-8);
    assert!((env[895] - 0.5011872).abs() < 1e-6);
    assert_eq!(env[895], env[994]);
    assert!((env[1245] - 0.25118864).abs() < 1e-6);
    assert_eq!(env[1995], 0.);
}

#[test]
fn no_dyn_still_mutes_the_verified_endpoint() {
    let (tempo, mut a, mut b, timing) = fixture();
    a.curves.gain_millidecibel_points.clear();
    b.curves.gain_millidecibel_points.clear();
    let map = TimeMap::new(vec![0., 2.], vec![0., 2.]).unwrap();
    assert!(dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, Some(&map), Some(&timing)).unwrap().is_none());
    let env = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2001, Some(&map), Some(&timing)).unwrap().unwrap();
    assert!(env[..2000].iter().all(|&value| value == 1.));
    assert_eq!(env[2000], 0.);
}

#[test]
fn missing_mode_delegates_to_legacy_without_sample_changes() {
    let (tempo, a, b, _) = fixture();
    let map = TimeMap::new(vec![0., 0.995, 1.995], vec![0., 1., 2.]).unwrap();
    for time_map in [None, Some(&map)] {
        let legacy = dyn_envelope_with_time_map(&[&a, &b], &tempo, 0., 44100, 88100, time_map);
        let routed = dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 44100, 88100, time_map, None).unwrap();
        assert_eq!(legacy, routed);
    }
}

#[test]
fn invalid_explicit_ownership_fails_instead_of_falling_back() {
    let (tempo, a, b, timing) = fixture();
    let map = TimeMap::new(vec![0., 2.], vec![0., 2.]).unwrap();
    assert!(dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, None, Some(&timing)).is_err());
    assert!(dyn_envelope_for_phrase(&[&b], &tempo, 0., 1000, 2000, Some(&map), Some(&timing)).is_err());
    let mut invalid = timing.clone();
    invalid.notes[0].tail_trim_seconds = 0.; // Overlaps the next note's prefix.
    assert!(dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, Some(&map), Some(&invalid)).is_err());
    invalid = timing.clone();
    invalid.notes[1].note_onset_seconds += 0.01;
    assert!(dyn_envelope_for_phrase(&[&a, &b], &tempo, 0., 1000, 2000, Some(&map), Some(&invalid)).is_err());
}
