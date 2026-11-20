use super::{bar_navigation_tick, bar_target_tick, position_label};
use verdant_domain::{MeterMap, Project, TempoMap};
use verdant_domain::model::TimeSignature;

fn imported_project(change_tick: i64) -> Project {
    let mut project = Project::new_empty();
    project.ppq = 480;
    project.time_signatures = vec![
        TimeSignature { tick: 0, numerator: 4, denominator: 4, unknown: Default::default() },
        TimeSignature { tick: change_tick, numerator: 3, denominator: 4, unknown: Default::default() },
    ];
    // Exercise the same stored/imported array consumed by Editor::meter().
    verdant_domain::io::parse_project(&serde_json::to_string(&project).unwrap()).unwrap().0
}

#[test]
fn imported_meter_change_aligns_ruler_lcd_and_bar_navigation_at_480_ppq() {
    let project = imported_project(3840);
    let meter = MeterMap::new(&project.time_signatures, project.ppq).unwrap();
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    let boundaries = [0, 1920, 3840, 5280, 6720];
    assert_eq!(meter.bars(0, 6721).collect::<Vec<_>>(), boundaries);
    for (index, &tick) in boundaries.iter().enumerate() {
        assert_eq!(meter.tick_at_bar(index as i64 + 1), Some(tick));
        assert_eq!(bar_target_tick(&meter, &(index + 1).to_string()), Ok(tick));
        assert_eq!(position_label(&meter, tick as f64), format!("{}  1  1  1", index + 1));
    }
    assert_eq!(position_label(&meter, 3360.0), "2  4  1  1");
    assert_eq!(position_label(&meter, 4800.0), "3  3  1  1");
    assert_eq!(meter.signature_at(4800).numerator, 3);
    for pair in boundaries.windows(2) {
        assert_eq!(bar_navigation_tick(&meter, pair[0] as f64, true), pair[1]);
        assert_eq!(bar_navigation_tick(&meter, pair[1] as f64, false), pair[0]);
    }
    // Seeking derives ticks from audio seconds; a fractional playhead must not
    // round over a boundary before deciding which bar is next or previous.
    for boundary in [1920, 3840, 5280] {
        let before = tempo.tick_at(tempo.seconds_at(boundary as f64 - 0.25));
        let after = tempo.tick_at(tempo.seconds_at(boundary as f64 + 0.25));
        assert_eq!(bar_navigation_tick(&meter, before, true), boundary);
        assert_eq!(bar_navigation_tick(&meter, after, false), boundary);
    }
    assert_eq!(bar_navigation_tick(&meter, 0.0, false), 0);
    assert_eq!(meter.tick_at_bar(0), None);
    assert_eq!(meter.tick_at_bar(-1), None);
    assert_eq!(meter.tick_at_bar(i64::MAX), None);
    assert_eq!(bar_target_tick(&meter, " 4 "), Ok(5280));
    for invalid in ["0", "-1", "2.5", "", "9223372036854775807", "9223372036854775808"] {
        assert!(bar_target_tick(&meter, invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn partial_measure_change_and_denominator_share_the_same_bar_numbering() {
    let mut project = imported_project(3360);
    project.time_signatures.push(TimeSignature {
        tick: 6240, numerator: 6, denominator: 8, unknown: Default::default(),
    });
    let meter = MeterMap::new(&project.time_signatures, project.ppq).unwrap();
    let boundaries = [0, 1920, 3360, 4800, 6240, 7680];
    assert_eq!(meter.bars(0, 7681).collect::<Vec<_>>(), boundaries);
    for (index, &tick) in boundaries.iter().enumerate() {
        assert_eq!(meter.tick_at_bar(index as i64 + 1), Some(tick));
        assert_eq!(position_label(&meter, tick as f64), format!("{}  1  1  1", index + 1));
    }
    assert_eq!(bar_navigation_tick(&meter, 1920.0, true), 3360);
    assert_eq!(bar_navigation_tick(&meter, 3360.0, false), 1920);
    assert_eq!(position_label(&meter, 6480.0), "5  2  1  1");
    assert_eq!(meter.beat_ticks_at(6480), 240);
    assert_eq!(meter.signature_at(6480).denominator, 8);
}
