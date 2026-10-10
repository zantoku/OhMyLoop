use ohmyloop::timing::{Edit, Fraction, MeasureKind, MeasureMap, Meter, ShiftDirection};

fn uniform(count: u32, meter: Meter) -> MeasureMap {
    let end = u64::from(count) * 4000;
    MeasureMap::empty(1000, end + 1000)
        .edit(Edit::Build {
            start: 0,
            end,
            count,
            meter,
        })
        .unwrap()
        .map
}
fn shift(map: &MeasureMap, index: usize, pulses: u32, direction: ShiftDirection) -> MeasureMap {
    map.edit(Edit::Shift {
        id: map.measures[index].id.clone(),
        pulses,
        direction,
    })
    .unwrap()
    .map
}
fn pulse_frames(map: &MeasureMap) -> Vec<u64> {
    map.pulse_events()
        .unwrap()
        .iter()
        .map(|p| p.frame)
        .collect()
}
fn at(map: &MeasureMap, quarter: f64) -> u64 {
    let mut start = 0.0;
    for m in &map.measures {
        let end = start + m.length.value();
        if quarter <= end {
            return map.frame_at(m, quarter - start);
        }
        start = end;
    }
    panic!("Quarter outside fixture");
}

#[test]
fn right_shift_realigns_measure_17_expands_16_and_keeps_all_pulse_times() {
    let map = uniform(20, Meter::common());
    let original = map.clone();
    let shifted = shift(&map, 16, 2, ShiftDirection::Right);
    let view = shifted.view();
    assert_eq!(view.measures[16].start, 66000);
    assert_eq!(view.measures[15].start, 60000);
    assert_eq!(shifted.measures[15].meter.numerator, 6);
    assert_eq!(shifted.measures[15].kind, MeasureKind::Full);
    assert_eq!(shifted.measures[19].length, Fraction::new(2, 1));
    assert_eq!(shifted.measures[19].kind, MeasureKind::Incomplete);
    assert_eq!(view.measures[19].end, 80000);
    assert_eq!(pulse_frames(&shifted), pulse_frames(&map));
    assert!(shifted.measures.iter().all(|m| m.beats.is_empty()));
    assert_eq!(
        shifted.measures.iter().map(|m| &m.id).collect::<Vec<_>>(),
        map.measures.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
    assert_eq!(shifted.measures[..15], map.measures[..15]);
    assert_eq!(shifted.revision, map.revision + 1);
    assert_eq!(shifted.frames, map.frames);
    assert_eq!(map, original);
    shifted.validate().unwrap();
}

#[test]
fn left_shift_shortens_preceding_measure_and_expands_the_last_meter() {
    let map = uniform(20, Meter::common());
    let shifted = shift(&map, 16, 2, ShiftDirection::Left);
    assert_eq!(shifted.view().measures[16].start, 62000);
    assert_eq!(shifted.measures[15].meter.numerator, 4);
    assert_eq!(shifted.measures[15].length, Fraction::new(2, 1));
    assert_eq!(shifted.measures[15].kind, MeasureKind::Incomplete);
    assert_eq!(shifted.measures[19].meter.numerator, 6);
    assert_eq!(shifted.measures[19].length, Fraction::new(6, 1));
    assert_eq!(shifted.measures[19].kind, MeasureKind::Full);
    assert_eq!(shifted.view().measures[19].end, 80000);
    assert_eq!(pulse_frames(&shifted), pulse_frames(&map));
    assert!(
        shifted
            .measures
            .iter()
            .all(|m| m.length.value() <= m.meter.length().value())
    );
}

#[test]
fn shifts_repartition_rubato_without_a_fixed_seconds_delta_or_lost_anchor_ids() {
    let mut map = uniform(4, Meter::common());
    for (i, frame) in [(0, 3000), (1, 9000)] {
        map = map
            .edit(Edit::MoveBoundary {
                id: map.measures[i].end.clone(),
                frame,
            })
            .unwrap()
            .map;
    }
    for (i, frame, offset) in [
        (1, 3600, Fraction::new(1, 1)),
        (1, 6500, Fraction::new(2, 1)),
        (1, 7500, Fraction::new(3, 1)),
        (2, 9300, Fraction::new(1, 2)),
        (3, 14500, Fraction::new(2, 1)),
    ] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[i].id.clone(),
                id: None,
                frame,
                offset,
            })
            .unwrap()
            .map;
    }
    let shifted = shift(&map, 1, 2, ShiftDirection::Right);
    assert_eq!(shifted.view().measures[1].start, 6500);
    assert_eq!(shifted.view().measures[3].start, 14500);
    assert_ne!(
        shifted.view().measures[1].start - map.view().measures[1].start,
        shifted.view().measures[2].start - map.view().measures[2].start
    );
    let starts = [0.0, 6.0, 10.0, 14.0];
    for (i, m) in shifted.measures.iter().enumerate() {
        for k in 0..=16 {
            let q = m.length.value() * f64::from(k) / 16.0;
            assert!(shifted.frame_at(m, q).abs_diff(at(&map, starts[i] + q)) <= 1);
        }
    }
    for frame in [3600, 7500, 9300] {
        let old = map
            .measures
            .iter()
            .flat_map(|m| &m.beats)
            .find(|b| b.frame == frame)
            .unwrap();
        let new = shifted
            .measures
            .iter()
            .flat_map(|m| &m.beats)
            .find(|b| b.id == old.id)
            .unwrap();
        assert_eq!(new.frame, frame);
    }
    assert!(shifted.measures[1].beats.iter().any(|b| b.frame == 9000));
    assert!(shifted.measures[0].beats.iter().any(|b| b.frame == 3000));
    let restored_timing = shift(&shifted, 1, 2, ShiftDirection::Left);
    assert_eq!(restored_timing.boundaries, map.boundaries);
    restored_timing.validate().unwrap();
}

#[test]
fn selected_compound_pulse_controls_distance_and_extended_meter_grouping() {
    let meter = Meter {
        numerator: 6,
        denominator: 8,
        pulse: Fraction::new(3, 2),
        grouping: vec![3, 3],
    };
    let map = uniform(4, meter);
    for direction in [ShiftDirection::Left, ShiftDirection::Right] {
        let shifted = shift(&map, 1, 1, direction);
        let grown = match direction {
            ShiftDirection::Right => &shifted.measures[0],
            ShiftDirection::Left => &shifted.measures[3],
        };
        assert_eq!(grown.meter.numerator, 9);
        assert_eq!(grown.meter.denominator, 8);
        assert_eq!(grown.meter.grouping, [3, 3, 3]);
        assert_eq!(grown.length, Fraction::new(9, 2));
        assert_eq!(pulse_frames(&shifted), pulse_frames(&map));
        shifted.validate().unwrap();
    }
}

#[test]
fn partial_and_changed_meter_spans_preserve_internal_extents_and_valid_pickups() {
    let mut map = uniform(4, Meter::common());
    map = map
        .edit(Edit::SetNotation {
            id: map.measures[0].id.clone(),
            meter: Meter::common(),
            kind: MeasureKind::Pickup,
            length: Fraction::new(3, 1),
            following: false,
        })
        .unwrap()
        .map;
    map = map
        .edit(Edit::SetNotation {
            id: map.measures[2].id.clone(),
            meter: Meter {
                numerator: 3,
                ..Meter::common()
            },
            kind: MeasureKind::Incomplete,
            length: Fraction::new(1, 1),
            following: false,
        })
        .unwrap()
        .map;
    let shifted = shift(&map, 1, 1, ShiftDirection::Left);
    assert_eq!(shifted.measures[0].kind, MeasureKind::Pickup);
    assert_eq!(shifted.measures[0].length, Fraction::new(2, 1));
    assert_eq!(shifted.measures[0].offset, Fraction::new(2, 1));
    assert_eq!(shifted.measures[2].meter, map.measures[2].meter);
    assert_eq!(shifted.measures[2].length, map.measures[2].length);
    assert_eq!(shifted.measures[2].kind, MeasureKind::Incomplete);
    assert_eq!(shifted.measures[3].meter.numerator, 5);
    shifted.validate().unwrap();
}

#[test]
fn invalid_shift_limits_protection_gaps_and_meter_overflow_are_atomic() {
    let map = uniform(4, Meter::common());
    let original = map.clone();
    for (index, pulses, direction) in [
        (0, 1, ShiftDirection::Right),
        (1, 0, ShiftDirection::Left),
        (1, 65, ShiftDirection::Right),
    ] {
        assert!(
            map.edit(Edit::Shift {
                id: map.measures[index].id.clone(),
                pulses,
                direction
            })
            .is_err()
        );
        assert_eq!(map, original);
    }
    for index in 0..4 {
        let locked = map
            .edit(Edit::SetProtected {
                id: map.measures[index].id.clone(),
                protected: true,
            })
            .unwrap()
            .map;
        assert!(
            locked
                .edit(Edit::Shift {
                    id: locked.measures[1].id.clone(),
                    pulses: 1,
                    direction: ShiftDirection::Left
                })
                .is_err()
        );
    }
    let outside = map
        .edit(Edit::SetProtected {
            id: map.measures[0].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    assert_eq!(
        shift(&outside, 2, 1, ShiftDirection::Right).measures[0],
        outside.measures[0]
    );
    let gap = map
        .edit(Edit::Remove {
            id: map.measures[2].id.clone(),
        })
        .unwrap()
        .map;
    assert!(
        gap.edit(Edit::Shift {
            id: gap.measures[1].id.clone(),
            pulses: 1,
            direction: ShiftDirection::Right
        })
        .is_err()
    );
    let huge = uniform(
        3,
        Meter {
            numerator: 99,
            ..Meter::common()
        },
    );
    for direction in [ShiftDirection::Right, ShiftDirection::Left] {
        assert!(
            huge.edit(Edit::Shift {
                id: huge.measures[1].id.clone(),
                pulses: 2,
                direction
            })
            .is_err()
        );
    }
}

#[test]
fn right_shift_absorbs_an_exactly_consumed_partial_tail() {
    let original = uniform(4, Meter::common());
    let map = original
        .edit(Edit::SetNotation {
            id: original.measures[3].id.clone(),
            meter: Meter::common(),
            kind: MeasureKind::Incomplete,
            length: Fraction::new(2, 1),
            following: false,
        })
        .unwrap()
        .map;
    let shifted = shift(&map, 1, 2, ShiftDirection::Right);
    assert_eq!(shifted.measures.len(), 3);
    assert_eq!(shifted.measures[0].meter.numerator, 6);
    assert_eq!(shifted.measures[2].id, map.measures[2].id);
    assert_eq!(shifted.measures[2].end, map.measures[3].end);
    assert_eq!(shifted.measures[2].kind, MeasureKind::Full);
    assert_eq!(shifted.view().measures[2].end, 16000);
    for quarter in [0.0, 2.0, 6.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0] {
        assert!(at(&shifted, quarter).abs_diff(at(&map, quarter)) <= 1);
    }
    assert!(shifted.measures[2].beats.iter().any(|b| b.frame == 12000));
    shifted.validate().unwrap();
}

#[test]
fn right_shift_absorbs_multiple_tail_measures_and_can_leave_one_measure() {
    let map = uniform(5, Meter::common());
    let shifted = shift(&map, 1, 9, ShiftDirection::Right);
    assert_eq!(shifted.measures.len(), 3);
    assert_eq!(shifted.measures[0].meter.numerator, 13);
    assert_eq!(shifted.measures[2].id, map.measures[2].id);
    assert_eq!(shifted.measures[2].length, Fraction::new(3, 1));
    assert_eq!(pulse_frames(&shifted), pulse_frames(&map));
    let collapsed = shift(&map, 1, 64, ShiftDirection::Right);
    assert_eq!(collapsed.measures.len(), 1);
    assert_eq!(collapsed.measures[0].id, map.measures[0].id);
    assert_eq!(collapsed.measures[0].meter.numerator, 20);
    assert_eq!(collapsed.measures[0].kind, MeasureKind::Full);
    assert_eq!(
        collapsed.boundaries,
        vec![map.boundaries[0].clone(), map.boundaries[5].clone()]
    );
    assert_eq!(pulse_frames(&collapsed), pulse_frames(&map));
}

#[test]
fn left_shift_absorbs_preceding_measures_and_extends_last_without_overfull_bars() {
    let map = uniform(5, Meter::common());
    let exact = shift(&map, 2, 4, ShiftDirection::Left);
    assert_eq!(exact.measures.len(), 4);
    assert_eq!(exact.measures[0], map.measures[0]);
    assert_eq!(exact.measures[1].id, map.measures[2].id);
    assert_eq!(exact.measures[1].start, map.measures[1].start);
    assert_eq!(exact.measures[3].meter.numerator, 8);
    let shifted = shift(&map, 3, 9, ShiftDirection::Left);
    assert_eq!(shifted.measures.len(), 3);
    assert_eq!(shifted.measures[0].id, map.measures[2].id);
    assert_eq!(shifted.measures[0].length, Fraction::new(3, 1));
    assert_eq!(shifted.measures[1].id, map.measures[3].id);
    assert_eq!(shifted.measures[2].meter.numerator, 13);
    assert_eq!(shifted.view().measures[0].start, 0);
    assert_eq!(shifted.view().measures[2].end, 20000);
    assert_eq!(pulse_frames(&shifted), pulse_frames(&map));
    let collapsed = shift(&map, 2, 64, ShiftDirection::Left);
    assert_eq!(collapsed.measures.len(), 1);
    assert_eq!(collapsed.measures[0].id, map.measures[4].id);
    assert_eq!(collapsed.measures[0].meter.numerator, 20);
    assert_eq!(pulse_frames(&collapsed), pulse_frames(&map));
    collapsed.validate().unwrap();
}

#[test]
fn absorption_preserves_anchors_and_rejects_protected_consumed_measures_or_earlier_gaps() {
    let mut map = uniform(5, Meter::common());
    map = map
        .edit(Edit::SetBeat {
            measure: map.measures[4].id.clone(),
            id: None,
            offset: Fraction::new(3, 2),
            frame: 17500,
        })
        .unwrap()
        .map;
    let anchor = map.measures[4].beats[0].clone();
    let shifted = shift(&map, 1, 9, ShiftDirection::Right);
    assert!(
        shifted
            .measures
            .iter()
            .flat_map(|m| &m.beats)
            .any(|b| b.id == anchor.id && b.frame == anchor.frame)
    );
    for k in 0..=80 {
        let q = f64::from(k) / 4.0;
        assert!(at(&shifted, q).abs_diff(at(&map, q)) <= 1);
    }
    for index in [0, 1, 4] {
        let locked = map
            .edit(Edit::SetProtected {
                id: map.measures[index].id.clone(),
                protected: true,
            })
            .unwrap()
            .map;
        let before = locked.clone();
        assert!(
            locked
                .edit(Edit::Shift {
                    id: locked.measures[3].id.clone(),
                    pulses: 9,
                    direction: ShiftDirection::Left
                })
                .is_err()
        );
        assert_eq!(locked, before);
    }
    let gap = map
        .edit(Edit::Remove {
            id: map.measures[1].id.clone(),
        })
        .unwrap()
        .map;
    assert!(
        gap.edit(Edit::Shift {
            id: gap.measures[2].id.clone(),
            pulses: 9,
            direction: ShiftDirection::Left
        })
        .is_err()
    );
}
