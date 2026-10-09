use ohmyloop::{
    project::{Document, Store},
    timing::{Edit, Fraction, MeasureKind, MeasureMap, Meter},
};
fn build(start: u64, end: u64, count: u32, meter: Meter) -> MeasureMap {
    MeasureMap::empty(1000, 20_000)
        .edit(Edit::Build {
            start,
            end,
            count,
            meter,
        })
        .unwrap()
        .map
}
fn notation(
    map: &MeasureMap,
    index: usize,
    meter: Meter,
    kind: MeasureKind,
    length: Fraction,
) -> MeasureMap {
    map.edit(Edit::SetNotation {
        id: map.measures[index].id.clone(),
        meter,
        kind,
        length,
        following: false,
    })
    .unwrap()
    .map
}
#[test]
fn compound_and_additive_meter_grids_use_notated_pulses() {
    let compound = Meter {
        numerator: 6,
        denominator: 8,
        pulse: Fraction::new(3, 2),
        grouping: vec![],
    };
    let map = build(1000, 5000, 2, compound);
    assert_eq!(
        map.view().measures[0]
            .grid
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [1000, 2000]
    );
    assert_eq!(map.position(2000).unwrap().beat, 2.0);
    let odd = Meter {
        numerator: 7,
        denominator: 8,
        pulse: Fraction::new(1, 2),
        grouping: vec![2, 2, 3],
    };
    let map = build(0, 3500, 1, odd);
    assert_eq!(
        map.view().measures[0]
            .grid
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [0, 1000, 2000]
    );
    assert_eq!(map.measures[0].length, Fraction::new(7, 2));
}
#[test]
fn anchors_follow_rubato_and_round_trip_without_a_global_tempo() {
    let mut map = build(1000, 9000, 2, Meter::common());
    map = map
        .edit(Edit::SetBeat {
            measure: map.measures[0].id.clone(),
            id: None,
            frame: 1600,
            offset: Fraction::new(1, 1),
        })
        .unwrap()
        .map;
    map = map
        .edit(Edit::SetBeat {
            measure: map.measures[0].id.clone(),
            id: None,
            frame: 4200,
            offset: Fraction::new(3, 1),
        })
        .unwrap()
        .map;
    for frame in [1000, 1600, 2407, 4200, 4999] {
        let q = map.quarter_at(&map.measures[0], frame);
        assert_eq!(map.frame_at(&map.measures[0], q), frame);
    }
    assert_eq!(map.frame_at(&map.measures[0], 1.0), 1600);
    let moving = map
        .edit(Edit::MoveBoundary {
            id: map.measures[0].end.clone(),
            frame: 6000,
        })
        .unwrap()
        .map;
    assert_eq!(moving.view().measures[1].end, 9000);
    assert_eq!(moving.measures[0].beats, map.measures[0].beats);
    assert_eq!(moving.position(6000).unwrap().label, "2");
    assert!(moving.position(9000).is_none());
    assert!(map.position(999).is_none());
}
#[test]
fn pickup_partial_and_meter_change_preserve_source_coordinates() {
    let map = build(500, 12_500, 3, Meter::common());
    let map = notation(
        &map,
        0,
        Meter::common(),
        MeasureKind::Pickup,
        Fraction::new(1, 1),
    );
    let three = Meter {
        numerator: 3,
        ..Meter::common()
    };
    let map = notation(
        &map,
        1,
        three.clone(),
        MeasureKind::Full,
        Fraction::new(3, 1),
    );
    let map = notation(&map, 2, three, MeasureKind::Incomplete, Fraction::new(3, 2));
    let view = map.view();
    assert_eq!(
        view.measures
            .iter()
            .map(|m| m.label.as_str())
            .collect::<Vec<_>>(),
        ["Pickup", "1", "2"]
    );
    assert_eq!(view.measures[0].start, 500);
    assert_eq!(map.measures[0].offset, Fraction::new(3, 1));
    assert_eq!(map.position(500).unwrap().beat, 4.0);
    assert_eq!(view.measures[2].end, 12_500);
    assert!(map.position(12_500).is_none());
}
#[test]
fn invalid_edits_and_protected_neighbours_leave_original_untouched() {
    let map = build(0, 8000, 2, Meter::common());
    let original = map.clone();
    assert!(
        map.edit(Edit::MoveBoundary {
            id: map.measures[0].end.clone(),
            frame: 0
        })
        .is_err()
    );
    assert!(
        map.edit(Edit::SetBeat {
            measure: map.measures[0].id.clone(),
            id: None,
            frame: 2000,
            offset: Fraction { num: 1, den: 0 }
        })
        .is_err()
    );
    assert!(
        map.edit(Edit::Build {
            start: 100,
            end: 5000,
            count: 2,
            meter: Meter::common()
        })
        .is_err()
    );
    assert_eq!(map, original);
    let map = map
        .edit(Edit::SetProtected {
            id: map.measures[1].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    assert!(
        map.edit(Edit::MoveBoundary {
            id: map.measures[0].end.clone(),
            frame: 4500
        })
        .is_err()
    );
    assert!(
        map.edit(Edit::Build {
            start: 0,
            end: 8000,
            count: 4,
            meter: Meter::common()
        })
        .is_err()
    );
}
#[test]
fn split_merge_and_free_time_have_explicit_semantics() {
    let map = build(0, 8000, 2, Meter::common());
    let map = map
        .edit(Edit::Split {
            id: map.measures[0].id.clone(),
            frame: 1500,
            left: Fraction::new(1, 1),
        })
        .unwrap()
        .map;
    assert_eq!(map.measures[0].kind, MeasureKind::Incomplete);
    assert_eq!(map.measures[1].length, Fraction::new(3, 1));
    let map = map
        .edit(Edit::Merge {
            id: map.measures[0].id.clone(),
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    assert_eq!(map.measures[0].kind, MeasureKind::Full);
    assert_eq!(map.measures[0].beats[0].frame, 1500);
    let map = map
        .edit(Edit::Remove {
            id: map.measures[0].id.clone(),
        })
        .unwrap()
        .map;
    assert!(map.position(1500).is_none());
    assert_eq!(map.position(4000).unwrap().label, "1");
    map.validate().unwrap();
}
#[test]
fn selected_range_rebuild_retains_other_ids_and_respace_keeps_edges() {
    let map = build(0, 12_000, 3, Meter::common());
    let outside = map.measures[2].clone();
    let map = map
        .edit(Edit::Build {
            start: 0,
            end: 8000,
            count: 4,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    assert_eq!(map.measures.last().unwrap(), &outside);
    let map = map
        .edit(Edit::MoveBoundary {
            id: map.measures[0].end.clone(),
            frame: 1500,
        })
        .unwrap()
        .map;
    let first = map.measures[0].id.clone();
    let last = map.measures[3].id.clone();
    let map = map.edit(Edit::Respace { first, last }).unwrap().map;
    assert_eq!(map.view().measures[0].end, 2000);
    assert_eq!(map.view().measures[3].end, 8000);
    assert_eq!(map.measures.last().unwrap(), &outside);
}
#[test]
fn persistence_migrates_v2_and_retains_invalid_original_bytes() {
    let directory = std::env::temp_dir().join(format!("ohmyloop-map-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let store = Store::new(directory.clone()).unwrap();
    let mut old = serde_json::to_value(Document::new("fixture".into())).unwrap();
    old["version"] = 2.into();
    old.as_object_mut().unwrap().remove("measure_map");
    old["controls"]["speed"] = 0.75.into();
    old["last_position"] = 1.234567.into();
    let bytes = serde_json::to_vec(&old).unwrap();
    std::fs::write(directory.join("fixture.json"), &bytes).unwrap();
    let mut migrated = store.load("fixture", 20.0).unwrap();
    assert_eq!(migrated.version, 3);
    assert_eq!(migrated.controls.speed, 0.75);
    assert_eq!(migrated.last_position, 1.234567);
    assert_eq!(
        std::fs::read(directory.join("fixture.json")).unwrap(),
        bytes
    );
    migrated.measure_map = Some(build(500, 8000, 3, Meter::common()));
    store.save(&migrated, 20.0).unwrap();
    assert_eq!(store.load("fixture", 20.0).unwrap(), migrated);
    let invalid = br#"{"version":4,"song_key":"fixture"}"#;
    std::fs::write(directory.join("fixture.json"), invalid).unwrap();
    assert!(store.load("fixture", 20.0).is_err());
    assert_eq!(
        std::fs::read(directory.join("fixture.json")).unwrap(),
        invalid
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn detected_pulses_preserve_drift_and_partial_tail_without_extrapolation() {
    let pulses = vec![1000, 1500, 2100, 2700, 3400, 4200, 5000];
    let map = MeasureMap::empty(1000, 20_000)
        .edit(Edit::BuildFromPulses {
            pulses: pulses.clone(),
            start: 500,
            end: 10_000,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    let view = map.view();
    assert_eq!(view.measures.len(), 2);
    assert_eq!(view.measures[0].start, 1000);
    assert_eq!(view.measures[0].end, 3400);
    assert_eq!(view.measures[1].end, 5000);
    assert_eq!(view.measures[1].kind, MeasureKind::Incomplete);
    assert_eq!(view.measures[1].length, Fraction::new(2, 1));
    assert!(map.position(6000).is_none());
    assert_eq!(map.frame_at(&map.measures[0], 2.0), 2100);
    let odd = Meter {
        numerator: 7,
        denominator: 8,
        pulse: Fraction::new(3, 2),
        grouping: vec![],
    };
    assert!(
        MeasureMap::empty(1000, 20_000)
            .edit(Edit::BuildFromPulses {
                pulses,
                start: 0,
                end: 10_000,
                meter: odd
            })
            .is_err()
    );
}
