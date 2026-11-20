use verdant_backend::f0::{control_track, validate};
use verdant_backend::post::{TimeMap, dyn_envelope_with_time_map};
use verdant_domain::model::GainPoint;
use verdant_domain::{Note, TempoMap};

#[test]
fn non_grid_phrase_end_has_an_exact_unvoiced_sentinel() {
    let tempo = TempoMap::constant(140.0);
    let note = Note::new(0, 960, 60);
    let end = tempo.seconds_at(960.0);
    let track = control_track(&[&note], &tempo, 0.0, end);
    validate(&track).unwrap();
    assert_eq!(track.frame_times_seconds.last(), Some(&end));
    assert_eq!(track.uv.last(), Some(&true));
    assert_eq!(track.f0_hz.last(), Some(&0.0));
    assert!(track.frame_times_seconds[track.uv.len() - 2] < end);
    assert!(!track.uv[track.uv.len() - 2]);
}

#[test]
fn internal_gap_boundaries_are_preserved_even_between_grid_points() {
    let tempo = TempoMap::constant(140.0);
    let a = Note::new(0, 960, 60);
    let b = Note::new(968, 960, 64);
    let start_b = tempo.seconds_at(968.0);
    let end_a = tempo.seconds_at(960.0);
    let track = control_track(&[&a, &b], &tempo, 0.0, tempo.seconds_at(1928.0));
    validate(&track).unwrap();
    let i = track.frame_times_seconds.iter().position(|t| *t == end_a).unwrap();
    let j = track.frame_times_seconds.iter().position(|t| *t == start_b).unwrap();
    assert!(track.uv[i]);
    assert_eq!(track.f0_hz[i], 0.0);
    assert!(!track.uv[j]);
    assert!(track.f0_hz[j] > 300.0);
}

#[test]
fn collapsed_gap_is_right_continuous_and_has_no_endpoint_extension() {
    let map = TimeMap::new(vec![0.0, 0.105, 0.105, 0.205], vec![0.0, 0.1025, 0.105, 0.205]).unwrap();
    assert!((map.authored_at(0.104).unwrap() - 0.104 / 0.105 * 0.1025).abs() < 1e-12);
    assert_eq!(map.authored_at(0.105), Some(0.105));
    assert_eq!(map.authored_at(-0.001), None);
    assert_eq!(map.authored_at(0.205), None);
    assert_eq!(map.authored_at(f64::NAN), None);
    assert!(TimeMap::new(vec![0.0, 0.1, 0.09], vec![0.0, 0.1, 0.2]).is_err());
    assert!(TimeMap::new(vec![0.0, 0.1], vec![0.0]).is_err());
}

#[test]
fn dyn_boundary_uses_actual_audio_time_after_quantization() {
    let tempo = TempoMap::constant(140.0);
    let a = Note::new(0, 960, 60);
    let mut b = Note::new(960, 960, 62);
    b.curves.gain_millidecibel_points = vec![GainPoint { id: "gain".into(), tick: 0, millidecibels: -6000, unknown: Default::default() }];
    let beat = tempo.seconds_at(960.0);
    let map = TimeMap::new(vec![0., 0.430, 0.855], vec![0., beat, 2. * beat]).unwrap();
    let mapped = dyn_envelope_with_time_map(&[&a, &b], &tempo, 0., 1000, 855, Some(&map)).unwrap();
    assert_eq!(mapped[429], 1.0); // second note has not started in rendered audio
    assert!((mapped[430] - 0.5011872).abs() < 1e-6);
}

#[test]
fn nonzero_phrase_origin_keeps_exact_note_and_gap_boundaries() {
    let tempo = TempoMap::constant(140.0);
    let a = Note::new(18, 1000, 60);
    let mut b = Note::new(1018, 960, 64);
    b.portamento_ms = Some(0); // This regression isolates right-continuous step membership.
    let origin = tempo.seconds_at(18.0);
    let boundary = tempo.seconds_at(1018.0) - origin;
    assert!(origin + boundary < tempo.seconds_at(1018.0)); // regression trigger
    let track = control_track(&[&a, &b], &tempo, origin, tempo.seconds_at(1978.0) - origin);
    let index = track.frame_times_seconds.iter().position(|t| *t == boundary).unwrap();
    assert!(!track.uv[index]);
    assert!((track.f0_hz[index] - 329.628).abs() < 0.001);
    let later = Note::new(1026, 960, 64);
    let gap = control_track(&[&a, &later], &tempo, origin, tempo.seconds_at(1986.0) - origin);
    let index = gap.frame_times_seconds.iter().position(|t| *t == boundary).unwrap();
    assert!(gap.uv[index]);
}

#[test]
fn mapped_dyn_is_right_continuous_with_nonzero_phrase_origin() {
    let tempo = TempoMap::constant(140.0);
    let a = Note::new(18, 1000, 60);
    let mut b = Note::new(1018, 960, 64);
    b.curves.gain_millidecibel_points = vec![GainPoint { id: "gain".into(), tick: 0, millidecibels: -6000, unknown: Default::default() }];
    let origin = tempo.seconds_at(18.0);
    let boundary = tempo.seconds_at(1018.0) - origin;
    let end = tempo.seconds_at(1978.0) - origin;
    let map = TimeMap::new(vec![0., 0.445, 0.875], vec![0., boundary, end]).unwrap();
    let gain = dyn_envelope_with_time_map(&[&a, &b], &tempo, origin, 1000, 875, Some(&map)).unwrap();
    assert_eq!(gain[444], 1.0);
    assert!((gain[445] - 0.5011872).abs() < 1e-6);
}

#[test]
fn mapped_gain_mutes_the_resampling_tail_even_without_dyn() {
    let tempo = TempoMap::constant(120.0);
    let mut note = Note::new(0, 960, 60);
    let map = TimeMap::new(vec![0., 0.105], vec![0., 0.105]).unwrap();
    // 1680 samples at 16k -> 4631 at 44.1k -> 5041 at 48k.
    let plain = dyn_envelope_with_time_map(&[&note], &tempo, 0., 48000, 5041, Some(&map)).unwrap();
    assert_eq!(plain[5039], 1.0);
    assert_eq!(plain[5040], 0.0);
    note.curves.gain_millidecibel_points =
        vec![GainPoint { id: "gain".into(), tick: 0, millidecibels: -60000, unknown: Default::default() }];
    let quiet = dyn_envelope_with_time_map(&[&note], &tempo, 0., 48000, 5041, Some(&map)).unwrap();
    assert!((quiet[5039] - 0.001).abs() < 1e-9);
    assert_eq!(quiet[5040], 0.0);
}
