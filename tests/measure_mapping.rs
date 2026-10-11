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
fn detected(rate: u32, pulses: Vec<u64>, epsilon_ms: f64) -> ohmyloop::timing::Preview {
    let end = *pulses.last().unwrap();
    MeasureMap::empty(rate, end + 1)
        .edit(Edit::BuildFromPulses {
            pulses,
            downbeats: vec![],
            first_downbeat: None,
            epsilon_ms,
            start: 0,
            end,
            meter: Meter::common(),
        })
        .unwrap()
}

#[test]
fn batch_grouping_matches_exact_signatures_across_gaps_and_preserves_authored_timing() {
    let mut map = build(0, 14_000, 7, Meter::common());
    let mut eighths = Meter::common();
    eighths.denominator = 8;
    map = notation(&map, 1, eighths, MeasureKind::Full, Fraction::new(2, 1));
    let mut triple = Meter::common();
    triple.numerator = 3;
    map = notation(&map, 2, triple, MeasureKind::Full, Fraction::new(3, 1));
    let mut pickup_meter = Meter::common();
    pickup_meter.pulse = Fraction::new(1, 2);
    map = notation(
        &map,
        0,
        pickup_meter,
        MeasureKind::Pickup,
        Fraction::new(1, 1),
    );
    let mut partial_meter = Meter::common();
    partial_meter.pulse = Fraction::new(2, 1);
    map = notation(
        &map,
        5,
        partial_meter,
        MeasureKind::Incomplete,
        Fraction::new(2, 1),
    );
    map.measures[0].meter.grouping = vec![1, 3];
    map.measures[4].meter.grouping = vec![1, 3];
    map.measures[6].meter.grouping = vec![1, 3];
    map.measures[6].protected = true;
    map = map
        .edit(Edit::SetBeat {
            measure: map.measures[4].id.clone(),
            id: None,
            frame: 8650,
            offset: Fraction::new(1, 1),
        })
        .unwrap()
        .map;
    map = map
        .edit(Edit::Remove {
            id: map.measures[3].id.clone(),
        })
        .unwrap()
        .map;
    let original = map.clone();
    let preview = map
        .edit(Edit::SetGroupingForMeter {
            numerator: 4,
            denominator: 4,
            grouping: vec![2, 2],
        })
        .unwrap();
    let mut expected = original.clone();
    for measure in &mut expected.measures {
        if measure.meter.numerator == 4 && measure.meter.denominator == 4 && !measure.protected {
            measure.meter.grouping = vec![2, 2];
        }
    }
    expected.revision += 1;
    assert_eq!(preview.map, expected);
    assert_eq!(map, original);
    assert_eq!(
        preview.warnings,
        ["3 measures changed; 1 locked measure skipped."]
    );
    assert_eq!(
        preview.view.measures[3]
            .grid
            .iter()
            .find(|p| p.id.is_some())
            .unwrap()
            .frame,
        8650
    );
    for (before, after) in original.measures.iter().zip(&preview.map.measures) {
        for step in 0..=(before.length.value() * 4.0) as u32 {
            let quarter = f64::from(step) / 4.0;
            assert_eq!(
                original.frame_at(before, quarter),
                preview.map.frame_at(after, quarter)
            );
        }
    }
    let before = original.pulse_events().unwrap();
    let after = preview.map.pulse_events().unwrap();
    assert_eq!(
        before.iter().map(|p| p.frame).collect::<Vec<_>>(),
        after.iter().map(|p| p.frame).collect::<Vec<_>>()
    );
    assert_eq!(
        after.iter().find(|p| p.frame == 8650).unwrap().accent,
        ohmyloop::timing::Accent::Beat
    );
    assert_eq!(
        after.iter().find(|p| p.frame == 9100).unwrap().accent,
        ohmyloop::timing::Accent::Group
    );
    let cleared = preview
        .map
        .edit(Edit::SetGroupingForMeter {
            numerator: 4,
            denominator: 4,
            grouping: vec![],
        })
        .unwrap()
        .map;
    expected.revision += 1;
    for measure in &mut expected.measures {
        if measure.meter.numerator == 4 && measure.meter.denominator == 4 && !measure.protected {
            measure.meter.grouping.clear();
        }
    }
    assert_eq!(cleared, expected);
}

#[test]
fn batch_grouping_counts_changes_and_does_not_create_clicks_between_pulses() {
    let mut map = build(
        0,
        10_500,
        3,
        Meter {
            numerator: 7,
            denominator: 8,
            pulse: Fraction::new(3, 2),
            grouping: vec![],
        },
    );
    map.measures[0].meter.grouping = vec![2, 2, 3];
    map.measures[2].protected = true;
    let before = map.pulse_events().unwrap();
    let preview = map
        .edit(Edit::SetGroupingForMeter {
            numerator: 7,
            denominator: 8,
            grouping: vec![2, 2, 3],
        })
        .unwrap();
    assert_eq!(
        preview.warnings,
        ["1 measure changed; 1 locked measure skipped."]
    );
    assert_eq!(preview.map.measures[0], map.measures[0]);
    assert_eq!(preview.map.measures[2], map.measures[2]);
    // Group starts at 1 and 2 quarter notes do not align with the 1.5-quarter pulse.
    assert_eq!(preview.map.pulse_events().unwrap(), before);
    assert_eq!(
        preview.view.measures[1]
            .grid
            .iter()
            .map(|p| p.quarter)
            .collect::<Vec<_>>(),
        [0.0, 1.0, 2.0]
    );
}

#[test]
fn batch_grouping_rejects_invalid_input_and_no_changes_atomically() {
    let original = build(0, 8000, 2, Meter::common());
    for (numerator, denominator, grouping) in [
        (4, 4, vec![2, 3]),
        (4, 4, vec![0, 4]),
        (4, 4, vec![u32::MAX, 1]),
        (4, 4, vec![1; 100]),
        (0, 4, vec![]),
        (100, 4, vec![]),
        (4, 3, vec![]),
        (4, 4, vec![]),
        (7, 8, vec![2, 2, 3]),
    ] {
        let map = original.clone();
        assert!(
            map.edit(Edit::SetGroupingForMeter {
                numerator,
                denominator,
                grouping
            })
            .is_err()
        );
        assert_eq!(map, original);
    }
    let mut locked = original.clone();
    for measure in &mut locked.measures {
        measure.protected = true;
    }
    assert!(
        locked
            .edit(Edit::SetGroupingForMeter {
                numerator: 4,
                denominator: 4,
                grouping: vec![2, 2]
            })
            .is_err()
    );
    assert!(
        serde_json::from_str::<Edit>(
            r#"{"type":"set_grouping_for_meter","numerator":4,"denominator":4,"grouping":[-1,5]}"#
        )
        .is_err()
    );
}

#[test]
fn detected_fit_keeps_boundaries_but_omits_jitter_anchors_at_any_source_rate() {
    for rate in [1000, 44_100, 48_000] {
        let r = u64::from(rate);
        let pulses = vec![
            0,
            r + r / 50,
            2 * r - r / 100,
            3 * r + r / 100,
            4 * r,
            5 * r + r / 100,
            6 * r,
        ];
        for epsilon_ms in [30.0, 5.0, 0.0] {
            let preview = detected(rate, pulses.clone(), epsilon_ms);
            let summary = preview.draft.unwrap();
            assert_eq!(summary.epsilon_ms, epsilon_ms);
            assert_eq!(
                summary.anchor_count,
                preview
                    .map
                    .measures
                    .iter()
                    .map(|m| m.beats.len())
                    .sum::<usize>()
            );
            if epsilon_ms == 30.0 {
                assert_eq!(summary.anchor_count, 0);
            } else {
                assert!(summary.anchor_count > 0);
            }
            assert_eq!(preview.view.measures[0].end, 4 * r);
            assert_eq!(preview.view.measures[1].end, 6 * r);
            assert_eq!(preview.map.measures[1].kind, MeasureKind::Incomplete);
            assert_eq!(preview.map.measures[1].length, Fraction::new(2, 1));
            for (i, pulse) in pulses.iter().enumerate().take(pulses.len() - 1) {
                let measure = &preview.map.measures[i / 4];
                let rendered = preview.map.frame_at(measure, (i % 4) as f64);
                assert!(rendered.abs_diff(*pulse) as f64 <= epsilon_ms * f64::from(rate) / 1000.0);
            }
        }
    }
    // Exact collinear detections also need no internal anchors at zero epsilon.
    assert_eq!(
        detected(1000, vec![0, 1000, 2000, 3000, 4000], 0.0)
            .draft
            .unwrap()
            .anchor_count,
        0
    );
}

#[test]
fn detected_fit_threshold_is_strict_and_rechecks_new_interpolation_segments() {
    let pulses = vec![0, 1025, 2000, 3000, 4000];
    assert_eq!(
        detected(1000, pulses.clone(), 25.0)
            .draft
            .unwrap()
            .anchor_count,
        0
    );
    let below = detected(1000, pulses, 24.0);
    assert_eq!(below.map.measures[0].beats.len(), 1);
    assert_eq!(below.map.measures[0].beats[0].frame, 1025);
    // The second pulse is within epsilon of even spacing, but not of the
    // interpolation introduced by correcting the first pulse. It needs an
    // anchor too to bound the final map's error, rather than only initial error.
    let preview = detected(1000, vec![0, 140, 190, 310, 400], 25.0);
    assert_eq!(
        preview.map.measures[0]
            .beats
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [140, 190]
    );
    for (q, frame) in [0, 140, 190, 310, 400].into_iter().enumerate() {
        assert!(
            preview
                .map
                .frame_at(&preview.map.measures[0], q as f64)
                .abs_diff(frame)
                <= 25
        );
    }
}

#[test]
fn detected_fit_zero_epsilon_respects_fractional_and_compound_pulse_units() {
    for (meter, intervals) in [
        (
            Meter {
                numerator: 7,
                denominator: 8,
                pulse: Fraction::new(1, 2),
                grouping: vec![2, 2, 3],
            },
            7,
        ),
        (
            Meter {
                pulse: Fraction::new(2, 3),
                ..Meter::common()
            },
            6,
        ),
        (
            Meter {
                numerator: 6,
                denominator: 8,
                pulse: Fraction::new(3, 2),
                grouping: vec![],
            },
            2,
        ),
    ] {
        let pulses: Vec<_> = (0..=intervals)
            .map(|i| 1000 + (10003 * i + intervals / 2) / intervals)
            .collect();
        let map = MeasureMap::empty(48_000, 20_000)
            .edit(Edit::BuildFromPulses {
                pulses: pulses.clone(),
                downbeats: vec![],
                first_downbeat: None,
                epsilon_ms: 0.0,
                start: 1000,
                end: 11003,
                meter: meter.clone(),
            })
            .unwrap()
            .map;
        for (index, frame) in pulses.into_iter().enumerate() {
            let quarter = index as f64 * f64::from(meter.pulse.num) / f64::from(meter.pulse.den);
            assert_eq!(map.frame_at(&map.measures[0], quarter), frame);
        }
    }
}

#[test]
fn detected_fit_defaults_and_invalid_epsilon_are_validated_at_the_boundary() {
    let old = serde_json::json!({"type":"build_from_pulses","pulses":[0,1020,2000,3000,4000],"start":0,"end":4000,
        "meter":Meter::common()});
    let map = MeasureMap::empty(1000, 5000);
    let preview = map.edit(serde_json::from_value(old).unwrap()).unwrap();
    assert_eq!(
        preview.draft.unwrap().epsilon_ms,
        ohmyloop::timing::DEFAULT_BEAT_EPSILON_MS
    );
    assert!(preview.map.measures[0].beats.is_empty());
    for epsilon_ms in [-1.0, 1000.01, f64::NAN, f64::INFINITY] {
        assert!(
            map.edit(Edit::BuildFromPulses {
                pulses: vec![0, 1000, 2000, 3000, 4000],
                downbeats: vec![],
                first_downbeat: None,
                epsilon_ms,
                start: 0,
                end: 4000,
                meter: Meter::common(),
            })
            .is_err()
        );
        assert!(map.measures.is_empty());
    }
}
#[test]
fn join_derives_common_denominator_meter_and_preserves_piecewise_timing() {
    let map = build(0, 8000, 2, Meter::common());
    let mut triple = Meter::common();
    triple.numerator = 3;
    triple.grouping = vec![1, 2];
    let mut map = notation(&map, 0, triple, MeasureKind::Full, Fraction::new(3, 1));
    for (i, frame, offset) in [(0, 1000, 1), (1, 6000, 2)] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[i].id.clone(),
                id: None,
                frame,
                offset: Fraction::new(offset, 1),
            })
            .unwrap()
            .map;
    }
    let original = map.clone();
    let joined = map
        .edit(Edit::Merge {
            id: map.measures[0].id.clone(),
        })
        .unwrap()
        .map;
    assert_eq!(joined.measures.len(), 1);
    assert_eq!(joined.measures[0].id, map.measures[0].id);
    assert_eq!(joined.measures[0].meter.numerator, 7);
    assert_eq!(joined.measures[0].meter.denominator, 4);
    assert_eq!(joined.measures[0].meter.grouping, [1, 2, 4]);
    assert_eq!(joined.measures[0].kind, MeasureKind::Full);
    assert_eq!(joined.measures[0].length, Fraction::new(7, 1));
    assert_eq!(
        (
            joined.view().measures[0].start,
            joined.view().measures[0].end
        ),
        (0, 8000)
    );
    assert_eq!(joined.frame_at(&joined.measures[0], 1.0), 1000);
    assert_eq!(joined.frame_at(&joined.measures[0], 3.0), 4000);
    assert_eq!(joined.frame_at(&joined.measures[0], 5.0), 6000);
    assert_eq!(joined.measures[0].beats[0].id, map.measures[0].beats[0].id);
    assert_eq!(joined.measures[0].beats[2].id, map.measures[1].beats[0].id);
    assert_eq!(map, original);
    let mismatch = notation(
        &map,
        1,
        Meter {
            numerator: 4,
            denominator: 8,
            ..Meter::common()
        },
        MeasureKind::Full,
        Fraction::new(2, 1),
    );
    assert!(
        mismatch
            .edit(Edit::Merge {
                id: mismatch.measures[0].id.clone()
            })
            .is_err()
    );
    let locked = map
        .edit(Edit::SetProtected {
            id: map.measures[1].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    assert!(
        locked
            .edit(Edit::Merge {
                id: locked.measures[0].id.clone()
            })
            .is_err()
    );
    let too_large = build(
        0,
        8000,
        2,
        Meter {
            numerator: 99,
            ..Meter::common()
        },
    );
    assert!(
        too_large
            .edit(Edit::Merge {
                id: too_large.measures[0].id.clone()
            })
            .is_err()
    );
    let gap = build(0, 12000, 3, Meter::common());
    let gap = gap
        .edit(Edit::Remove {
            id: gap.measures[1].id.clone(),
        })
        .unwrap()
        .map;
    assert!(
        gap.edit(Edit::Merge {
            id: gap.measures[0].id.clone()
        })
        .is_err()
    );
}

#[test]
fn numbered_split_uses_fixed_and_interpolated_beats_and_preserves_both_partial_meters() {
    let mut map = build(0, 8000, 2, Meter::common());
    for (frame, offset) in [(500, 1), (1500, 2), (3500, 3)] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[0].id.clone(),
                id: None,
                frame,
                offset: Fraction::new(offset, 1),
            })
            .unwrap()
            .map;
    }
    let point = &map.view().measures[0].split_points[1];
    assert_eq!((point.beat, point.frame, point.anchor), (3, 1500, true));
    let split = map
        .edit(Edit::SplitAtBeat {
            id: map.measures[0].id.clone(),
            beat: 3,
        })
        .unwrap()
        .map;
    assert_eq!(split.measures.len(), 3);
    assert_eq!(split.measures[2], map.measures[1]);
    for m in &split.measures[..2] {
        assert_eq!(m.meter, map.measures[0].meter);
        assert_eq!(m.kind, MeasureKind::Incomplete);
        assert_eq!(m.length, Fraction::new(2, 1));
    }
    assert_eq!(split.view().measures[0].end, 1500);
    assert_eq!(split.measures[0].beats[0], map.measures[0].beats[0]);
    assert_eq!(split.measures[1].beats[0].id, map.measures[0].beats[2].id);
    assert_eq!(split.measures[1].beats[0].offset, Fraction::new(1, 1));
    let joined = split
        .edit(Edit::Merge {
            id: split.measures[0].id.clone(),
        })
        .unwrap()
        .map;
    assert_eq!(joined.measures[0].meter.numerator, 8);
    assert_eq!(joined.measures[0].kind, MeasureKind::Incomplete);
    assert_eq!(joined.measures[0].length, Fraction::new(4, 1));
    let uniform = build(0, 8000, 2, Meter::common());
    assert!(!uniform.view().measures[0].split_points[2].anchor);
    let split = uniform
        .edit(Edit::SplitAtBeat {
            id: uniform.measures[0].id.clone(),
            beat: 4,
        })
        .unwrap()
        .map;
    assert_eq!(split.view().measures[0].end, 3000);
    assert_eq!(split.measures[0].length, Fraction::new(3, 1));
    assert_eq!(split.measures[1].length, Fraction::new(1, 1));
    for beat in [0, 1, 5, 513] {
        assert!(
            uniform
                .edit(Edit::SplitAtBeat {
                    id: uniform.measures[0].id.clone(),
                    beat
                })
                .is_err()
        );
    }
}

#[test]
fn numbered_splits_use_regular_pulses_despite_grouping_and_pickup_offsets() {
    let meter = Meter {
        numerator: 7,
        denominator: 8,
        pulse: Fraction::new(1, 2),
        grouping: vec![2, 2, 3],
    };
    let map = build(0, 7000, 2, meter.clone());
    assert!(!map.view().measures[0].grid.iter().any(|p| p.label == "4"));
    let split = map
        .edit(Edit::SplitAtBeat {
            id: map.measures[0].id.clone(),
            beat: 4,
        })
        .unwrap()
        .map;
    assert_eq!(split.view().measures[0].end, 1500);
    assert_eq!(split.measures[0].meter, meter);
    assert_eq!(split.measures[0].length, Fraction::new(3, 2));
    let pickup = build(0, 8000, 2, Meter::common());
    let pickup = notation(
        &pickup,
        0,
        Meter::common(),
        MeasureKind::Pickup,
        Fraction::new(2, 1),
    );
    assert_eq!(pickup.view().measures[0].split_points[0].beat, 4);
    let split = pickup
        .edit(Edit::SplitAtBeat {
            id: pickup.measures[0].id.clone(),
            beat: 4,
        })
        .unwrap()
        .map;
    assert_eq!(split.view().measures[0].end, 2000);
    for m in &split.measures[..2] {
        assert_eq!(m.kind, MeasureKind::Incomplete);
        assert_eq!(m.offset, Fraction::new(0, 1));
    }
    let locked = map
        .edit(Edit::SetProtected {
            id: map.measures[0].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    assert!(
        locked
            .edit(Edit::SplitAtBeat {
                id: locked.measures[0].id.clone(),
                beat: 4
            })
            .is_err()
    );
}

#[test]
fn free_time_reclaim_creates_only_inside_gaps_and_never_replaces_existing_measures() {
    let map = build(1000, 9000, 2, Meter::common());
    let map = map
        .edit(Edit::Remove {
            id: map.measures[0].id.clone(),
        })
        .unwrap()
        .map;
    let original = map.clone();
    let reclaimed = map
        .edit(Edit::Reclaim {
            start: 0,
            end: 5000,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    assert_eq!(reclaimed.measures[1], map.measures[0]);
    assert_eq!(
        (
            reclaimed.view().measures[0].start,
            reclaimed.view().measures[0].end
        ),
        (0, 5000)
    );
    assert_eq!(reclaimed.measures[0].kind, MeasureKind::Full);
    assert!(reclaimed.measures[0].beats.is_empty());
    assert_eq!(reclaimed.pulse_events().unwrap()[1].frame, 1250);
    assert_eq!(map, original);
    for (start, end) in [(4000, 6000), (5000, 9000), (0, 0), (9000, 20001)] {
        assert!(
            map.edit(Edit::Reclaim {
                start,
                end,
                meter: Meter::common()
            })
            .is_err()
        );
        assert_eq!(map, original);
    }
    let empty = MeasureMap::empty(1000, 20000);
    let reclaimed = empty
        .edit(Edit::Reclaim {
            start: 0,
            end: 20000,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    assert_eq!(reclaimed.measures.len(), 1);
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
    assert!(moving.measures[0].beats.is_empty());
    assert_eq!(moving.frame_at(&moving.measures[0], 1.0), 2250);
    assert_eq!(moving.position(6000).unwrap().label, "2");
    assert!(moving.position(9000).is_none());
    assert!(map.position(999).is_none());
}
#[test]
fn resizing_shared_boundary_respaces_both_measures_and_the_click_map() {
    let mut map = build(0, 12_000, 3, Meter::common());
    for (i, frame, offset) in [(0, 3000, 1), (1, 5000, 3), (2, 9000, 1)] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[i].id.clone(),
                id: None,
                frame,
                offset: Fraction::new(offset, 1),
            })
            .unwrap()
            .map;
    }
    let before = map.clone();
    let resized = map
        .edit(Edit::MoveBoundary {
            id: map.measures[1].start.clone(),
            frame: 2000,
        })
        .unwrap()
        .map;
    assert!(resized.measures[0].beats.is_empty());
    assert!(resized.measures[1].beats.is_empty());
    assert_eq!(resized.measures[2], before.measures[2]);
    assert_eq!(resized.next_id, before.next_id);
    assert_eq!(resized.revision, before.revision + 1);
    for (i, frames) in [
        (0, vec![0, 500, 1000, 1500]),
        (1, vec![2000, 3500, 5000, 6500]),
    ] {
        assert_eq!(
            resized.view().measures[i]
                .grid
                .iter()
                .map(|p| p.frame)
                .collect::<Vec<_>>(),
            frames
        );
        for (q, frame) in frames.iter().enumerate() {
            assert_eq!(resized.frame_at(&resized.measures[i], q as f64), *frame);
        }
    }
    assert_eq!(
        resized
            .pulse_events()
            .unwrap()
            .iter()
            .take(8)
            .map(|p| p.frame)
            .collect::<Vec<_>>(),
        [0, 500, 1000, 1500, 2000, 3500, 5000, 6500]
    );
    assert_eq!(map, before);
    // A pointer release at the original frame must not discard corrections.
    let unchanged = map
        .edit(Edit::MoveBoundary {
            id: map.measures[1].start.clone(),
            frame: 4000,
        })
        .unwrap()
        .map;
    assert_eq!(unchanged.measures, map.measures);
    assert_eq!(unchanged.boundaries, map.boundaries);
    // Protection on either side blocks the whole atomic resize.
    for i in 0..2 {
        let protected = map
            .edit(Edit::SetProtected {
                id: map.measures[i].id.clone(),
                protected: true,
            })
            .unwrap()
            .map;
        assert!(
            protected
                .edit(Edit::MoveBoundary {
                    id: map.measures[1].start.clone(),
                    frame: 2000
                })
                .is_err()
        );
        assert_eq!(protected.measures[i].beats, map.measures[i].beats);
    }
    for frame in [0, 8000, 13_000] {
        assert!(
            map.edit(Edit::MoveBoundary {
                id: map.measures[1].start.clone(),
                frame
            })
            .is_err()
        );
        assert_eq!(map, before);
    }
}

#[test]
fn resizing_pickup_and_partial_with_changed_meter_uses_actual_notated_extent() {
    let map = build(1000, 9000, 2, Meter::common());
    let map = notation(
        &map,
        0,
        Meter::common(),
        MeasureKind::Pickup,
        Fraction::new(1, 1),
    );
    let compound = Meter {
        numerator: 6,
        denominator: 8,
        pulse: Fraction::new(1, 2),
        grouping: vec![],
    };
    let mut map = notation(
        &map,
        1,
        compound,
        MeasureKind::Incomplete,
        Fraction::new(3, 2),
    );
    for (i, frame) in [(0, 2000), (1, 5500)] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[i].id.clone(),
                id: None,
                frame,
                offset: Fraction::new(1, 2),
            })
            .unwrap()
            .map;
    }
    let resized = map
        .edit(Edit::MoveBoundary {
            id: map.measures[1].start.clone(),
            frame: 6000,
        })
        .unwrap()
        .map;
    assert_eq!(resized.measures[0].offset, Fraction::new(3, 1));
    assert_eq!(resized.measures[0].length, Fraction::new(1, 1));
    assert_eq!(resized.frame_at(&resized.measures[0], 0.5), 3500);
    assert_eq!(
        resized.view().measures[1]
            .grid
            .iter()
            .map(|p| p.frame)
            .collect::<Vec<_>>(),
        [6000, 7000, 8000]
    );
    assert_eq!(resized.position(6000).unwrap().label, "1");
}

#[test]
fn resizing_outer_boundaries_and_gap_edges_only_respaces_the_attached_measure() {
    let mut map = build(0, 12_000, 3, Meter::common());
    for (i, frame) in [(0, 1000), (2, 9000)] {
        map = map
            .edit(Edit::SetBeat {
                measure: map.measures[i].id.clone(),
                id: None,
                frame,
                offset: Fraction::new(1, 1),
            })
            .unwrap()
            .map;
    }
    map = map
        .edit(Edit::Remove {
            id: map.measures[1].id.clone(),
        })
        .unwrap()
        .map;
    for (id, frame, changed) in [
        (map.measures[0].start.clone(), 500, 0),
        (map.measures[0].end.clone(), 4500, 0),
        (map.measures[1].start.clone(), 8500, 1),
        (map.measures[1].end.clone(), 12_500, 1),
    ] {
        let resized = map.edit(Edit::MoveBoundary { id, frame }).unwrap().map;
        assert!(resized.measures[changed].beats.is_empty());
        assert_eq!(resized.measures[1 - changed], map.measures[1 - changed]);
        resized.validate().unwrap();
    }
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
        })
        .unwrap()
        .map;
    assert_eq!(map.measures[0].kind, MeasureKind::Incomplete);
    assert_eq!(map.measures[0].meter.numerator, 8);
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
    assert_eq!(migrated.version, 4);
    assert_eq!(migrated.controls.speed, 0.75);
    assert_eq!(migrated.last_position, 1.234567);
    assert_eq!(
        std::fs::read(directory.join("fixture.json")).unwrap(),
        bytes
    );
    migrated.measure_map = Some(
        build(500, 8000, 3, Meter::common())
            .edit(Edit::SetGroupingForMeter {
                numerator: 4,
                denominator: 4,
                grouping: vec![2, 2],
            })
            .unwrap()
            .map,
    );
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
            epsilon_ms: 0.0,
            first_downbeat: None,
            downbeats: vec![],
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
                epsilon_ms: 0.0,
                first_downbeat: None,
                downbeats: vec![],
                pulses,
                start: 0,
                end: 10_000,
                meter: odd
            })
            .is_err()
    );
}

#[test]
fn reanalysis_replaces_a_selected_old_draft_with_unobserved_edges_left_free() {
    let old = build(0, 8000, 2, Meter::common());
    let preview = old
        .edit(Edit::BuildFromPulses {
            epsilon_ms: 0.0,
            first_downbeat: None,
            pulses: vec![1000, 1500, 2100, 2700, 3400, 4200, 5000],
            downbeats: vec![1000, 4200],
            start: 0,
            end: 8000,
            meter: Meter::common(),
        })
        .unwrap();
    assert_eq!(preview.map.revision, old.revision + 1);
    assert!(preview.map.position(500).is_none());
    assert!(preview.map.position(7000).is_none());
    assert!(!preview.warnings.is_empty());
    let protected = old
        .edit(Edit::SetProtected {
            id: old.measures[0].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    assert!(
        protected
            .edit(Edit::BuildFromPulses {
                epsilon_ms: 0.0,
                first_downbeat: None,
                pulses: vec![1000, 1500, 2100, 2700, 3400, 4200, 5000],
                downbeats: vec![1000],
                start: 0,
                end: 8000,
                meter: Meter::common()
            })
            .is_err()
    );
}
#[test]
fn numbering_and_tempo_are_derived_without_changing_source_bounds() {
    let map = build(0, 8000, 2, Meter::common());
    let changed = map.edit(Edit::SetOrigin { first_number: 17 }).unwrap().map;
    assert_eq!(changed.boundaries, map.boundaries);
    assert_eq!(changed.view().measures[0].label, "17");
    assert_eq!(changed.view().measures[0].tempo_bpm, 60.0);
    let changed = changed
        .edit(Edit::SetBeat {
            measure: changed.measures[0].id.clone(),
            id: None,
            frame: 500,
            offset: Fraction::new(1, 1),
        })
        .unwrap()
        .map;
    assert_eq!(changed.position(250).unwrap().tempo_bpm, 120.0);
    assert!((changed.position(1000).unwrap().tempo_bpm - 60.0 * 3.0 / 3.5).abs() < 1e-8);
}

#[test]
fn following_meter_changes_preserve_explicit_partial_measures() {
    let map = build(0, 8000, 2, Meter::common());
    let map = map
        .edit(Edit::SetNotation {
            id: map.measures[1].id.clone(),
            meter: Meter::common(),
            kind: MeasureKind::Incomplete,
            length: Fraction::new(1, 1),
            following: false,
        })
        .unwrap()
        .map;
    let mut triple = Meter::common();
    triple.numerator = 3;
    let changed = map
        .edit(Edit::SetNotation {
            id: map.measures[0].id.clone(),
            meter: triple,
            kind: MeasureKind::Full,
            length: Fraction::new(3, 1),
            following: true,
        })
        .unwrap()
        .map;
    assert_eq!(changed.boundaries, map.boundaries);
    assert_eq!(changed.measures[1].kind, MeasureKind::Incomplete);
    assert_eq!(changed.measures[1].length, Fraction::new(1, 1));
    assert_eq!(changed.measures[1].meter.numerator, 3);
}

#[test]
fn detected_draft_derives_count_and_snaps_first_downbeat_without_even_spacing() {
    let old = build(0, 8000, 2, Meter::common());
    let pulses = vec![1000, 1500, 2100, 2700, 3400, 4200, 5000, 5700, 6200, 7000];
    let make = |meter| {
        old.edit(Edit::BuildFromPulses {
            epsilon_ms: 0.0,
            pulses: pulses.clone(),
            downbeats: vec![2700, 5700],
            first_downbeat: Some(2500),
            start: 0,
            end: 8000,
            meter,
        })
        .unwrap()
    };
    let preview = make(Meter::common());
    let summary = preview.draft.as_ref().unwrap();
    assert_eq!(
        (
            summary.pulse_count,
            summary.pulses_per_measure,
            summary.full_measures,
            summary.partial_pulses,
            summary.measure_count
        ),
        (7, 4, 1, 2, 2)
    );
    assert_eq!((summary.first_frame, summary.last_frame), (2700, 7000));
    assert_eq!(
        preview
            .map
            .boundaries
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [2700, 5700, 7000]
    );
    assert_eq!(
        preview.map.measures[0]
            .beats
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [3400, 5000]
    );
    assert!(preview.map.position(1000).is_none());
    let compound = make(Meter {
        numerator: 6,
        denominator: 8,
        pulse: Fraction::new(3, 2),
        grouping: vec![],
    });
    assert_eq!(compound.draft.unwrap().measure_count, 3);
    assert_eq!(
        compound
            .map
            .boundaries
            .iter()
            .map(|b| b.frame)
            .collect::<Vec<_>>(),
        [2700, 4200, 5700, 7000]
    );
    assert_eq!(old.measures.len(), 2);
}
#[test]
fn explicit_clear_removes_protected_annotations_and_preserves_source_and_id_counter() {
    let map = build(0, 8000, 2, Meter::common());
    let map = map
        .edit(Edit::SetProtected {
            id: map.measures[0].id.clone(),
            protected: true,
        })
        .unwrap()
        .map;
    let map = map.edit(Edit::SetOrigin { first_number: 17 }).unwrap().map;
    let cleared = map.edit(Edit::Clear).unwrap();
    assert!(cleared.map.measures.is_empty() && cleared.map.boundaries.is_empty());
    assert_eq!(
        (cleared.map.rate, cleared.map.frames, cleared.map.next_id),
        (map.rate, map.frames, map.next_id)
    );
    assert_eq!(cleared.map.first_number, 1);
    assert_eq!(cleared.map.revision, map.revision + 1);
    assert!(cleared.view.measures.is_empty());
    assert!(cleared.map.position(1000).is_none());
    assert_eq!(map.measures.len(), 2);
    assert!(map.measures[0].protected);
}
