use super::{policy::*, storage};
use std::fs;

fn screen(token: &str, width: f64, height: f64, scale: f64) -> Screen {
    Screen {
        token: token.into(),
        key: Some(token.repeat(64)),
        work: Size { width, height },
        scale,
        origin: Point::default(),
    }
}
fn scene() -> Snapshot {
    Snapshot {
        screens: vec![
            screen("a", 1512.0, 900.0, 2.0),
            screen("b", 2560.0, 1400.0, 1.0),
        ],
        current: "a".into(),
        content: DEFAULT,
        offset: Point { x: 40.0, y: 40.0 },
        decoration: Size {
            width: 0.0,
            height: 28.0,
        },
        mode: Mode::Normal,
        interacting: false,
        user_resizing: false,
        display_epoch: 0,
    }
}
fn apply(snapshot: &mut Snapshot, placement: Placement) {
    snapshot.current = placement.screen;
    snapshot.content = placement.content;
    snapshot.offset = placement.offset;
    if placement.maximize {
        snapshot.mode = Mode::Maximized;
    }
}
fn settle(engine: &mut Engine, mut snapshot: Snapshot, base: u64) -> Snapshot {
    for delta in [0, 400, 900, 1300, 1800, 2300, 2800] {
        if let Some(placement) = engine.step(snapshot.clone(), base + delta, false, false) {
            apply(&mut snapshot, placement);
        }
    }
    snapshot
}

#[test]
fn close_flushes_pending_user_resize_but_maximize_never_becomes_a_normal_preference() {
    let mut engine = Engine::new(Saved::default());
    let mut snapshot = settle(&mut engine, scene(), 0);
    snapshot.content = Size {
        width: 1000.0,
        height: 700.0,
    };
    snapshot.user_resizing = true;
    snapshot.interacting = true;
    assert!(engine.step(snapshot.clone(), 4000, false, true).is_none());
    assert_eq!(engine.saved.last_normal, snapshot.content);
    let normal = snapshot.content;
    snapshot.content = Size {
        width: 900.0,
        height: 680.0,
    };
    engine.step(snapshot.clone(), 4500, false, false);
    snapshot.content = Size {
        width: 1512.0,
        height: 872.0,
    };
    snapshot.mode = Mode::Maximized;
    snapshot.interacting = false;
    snapshot.user_resizing = false;
    settle(&mut engine, snapshot, 5000);
    assert_eq!(engine.saved.last_normal, normal);
    assert_eq!(engine.saved.profiles[&"a".repeat(64)].preferred, normal);
    assert!(engine.saved.last_maximized);
}

#[test]
fn missing_monitor_data_does_not_poison_later_restore_and_lru_survives_restart() {
    let mut saved = Saved::default();
    saved.profiles.insert(
        "a".repeat(64),
        Profile {
            preferred: DEFAULT,
            effective: DEFAULT,
            anchor: Point { x: 0.5, y: 0.5 },
            maximized: false,
            updated: 100,
        },
    );
    let mut engine = Engine::new(saved);
    let mut missing = scene();
    missing.screens.clear();
    assert!(engine.step(missing, 3000, false, false).is_none());
    assert!(!engine.restored);
    assert_eq!(engine.revision, 101);
    settle(&mut engine, scene(), 4000);
    assert!(engine.restored);
    assert_eq!(engine.saved.last_normal, DEFAULT);
}

#[test]
fn fit_is_logical_bounded_and_idempotent_across_scales_and_work_areas() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for (width, height) in [
            (2560.0, 1400.0),
            (1512.0, 900.0),
            (900.0, 1500.0),
            (640.0, 480.0),
        ] {
            let screen = screen("a", width, height, scale);
            let decoration = Size {
                width: 8.0,
                height: 32.0,
            };
            let first = fit(
                &screen,
                Size {
                    width: 4000.0,
                    height: 2400.0,
                },
                decoration,
                Point { x: 0.5, y: 0.5 },
            );
            assert!(first.offset.x >= 0.0 && first.offset.y >= 0.0);
            assert!(first.offset.x + first.content.width + decoration.width <= width);
            assert!(first.offset.y + first.content.height + decoration.height <= height);
            assert!(
                first.minimum.width <= first.content.width
                    && first.minimum.height <= first.content.height
            );
            assert_eq!(
                fit(&screen, first.content, decoration, Point { x: 0.5, y: 0.5 }),
                first
            );
        }
    }
}

#[test]
fn returning_from_a_work_area_below_the_usual_minimum_never_auto_enlarges() {
    let mut engine = Engine::new(Saved::default());
    let mut snapshot = settle(&mut engine, scene(), 0);
    snapshot.screens[0].work = Size {
        width: 640.0,
        height: 480.0,
    };
    snapshot = settle(&mut engine, snapshot, 4000);
    let small = snapshot.content;
    assert!(small.width < MINIMUM.width && small.height < MINIMUM.height);
    snapshot.current = "b".into();
    snapshot.offset = Point { x: 40.0, y: 40.0 };
    snapshot = settle(&mut engine, snapshot, 8000);
    assert_eq!(snapshot.content, small);
    assert_eq!(engine.saved.last_normal, DEFAULT);
}

#[test]
fn height_only_constraint_preserves_width_and_negative_origin_does_not_change_size() {
    let mut screen = screen("a", 1800.0, 800.0, 2.0);
    screen.origin = Point {
        x: -2560.0,
        y: -1440.0,
    };
    let fitted = fit(
        &screen,
        Size {
            width: 1200.0,
            height: 1500.0,
        },
        Size {
            width: 0.0,
            height: 28.0,
        },
        Point::default(),
    );
    assert_eq!(fitted.content.width, 1200.0);
    assert_eq!(fitted.content.height, 748.0);
}

#[test]
fn restore_selects_the_matching_screen_and_does_not_accumulate_dpi_drift() {
    let mut saved = Saved {
        last_monitor: Some("b".repeat(64)),
        last_normal: Size {
            width: 1800.0,
            height: 1100.0,
        },
        ..Saved::default()
    };
    for iteration in 0..20 {
        let mut engine = Engine::new(saved);
        let mut current = scene();
        current.screens[1].scale = if iteration % 2 == 0 { 1.0 } else { 2.0 };
        let restored = settle(&mut engine, current, 0);
        assert_eq!(restored.current, "b");
        assert_eq!(
            restored.content,
            Size {
                width: 1800.0,
                height: 1100.0
            }
        );
        saved = engine.saved;
    }
}

#[test]
fn disconnected_external_display_restores_builtin_profile_and_keeps_external_preference() {
    let mut saved = Saved {
        last_monitor: Some("b".repeat(64)),
        last_normal: Size {
            width: 2200.0,
            height: 1300.0,
        },
        ..Saved::default()
    };
    for (key, size) in [("a", DEFAULT), ("b", saved.last_normal)] {
        saved.profiles.insert(
            key.repeat(64),
            Profile {
                preferred: size,
                effective: size,
                anchor: Point { x: 0.5, y: 0.5 },
                maximized: false,
                updated: 0,
            },
        );
    }
    let mut engine = Engine::new(saved);
    let mut current = scene();
    current.screens.pop();
    let restored = settle(&mut engine, current, 0);
    assert_eq!(restored.content, DEFAULT);
    assert_eq!(
        engine.saved.profiles[&"b".repeat(64)].preferred.width,
        2200.0
    );
}

#[test]
fn cross_screen_shrink_does_not_pollute_preferences_or_expand_on_return() {
    let large = Size {
        width: 2200.0,
        height: 1250.0,
    };
    let saved = Saved {
        last_monitor: Some("b".repeat(64)),
        last_normal: large,
        ..Saved::default()
    };
    let mut engine = Engine::new(saved);
    let mut current = settle(&mut engine, scene(), 0);
    current.current = "a".into();
    current.interacting = true;
    assert!(engine.step(current.clone(), 4000, false, false).is_none());
    assert_eq!(current.content, large);
    current.interacting = false;
    let smaller = settle(&mut engine, current, 4500);
    assert!(smaller.content.width < large.width && smaller.content.height < large.height);
    assert_eq!(engine.saved.profiles[&"b".repeat(64)].preferred, large);
    assert_eq!(engine.saved.last_normal, large);
    let mut returned = smaller.clone();
    returned.current = "b".into();
    let returned = settle(&mut engine, returned, 8000);
    assert_eq!(returned.content, smaller.content);
    assert_eq!(engine.saved.profiles[&"b".repeat(64)].preferred, large);
    let mut next = Engine::new(engine.saved);
    assert_eq!(settle(&mut next, scene(), 0).content, large);
}

#[test]
fn only_confirmed_user_resizes_change_the_preferred_size() {
    let mut engine = Engine::new(Saved::default());
    let mut current = settle(&mut engine, scene(), 0);
    current.content = Size {
        width: 1000.0,
        height: 700.0,
    };
    // Unknown/programmatic resize is an effective geometry update only.
    current = settle(&mut engine, current, 4000);
    assert_eq!(engine.saved.profiles[&"a".repeat(64)].preferred, DEFAULT);
    current.content = Size {
        width: 1100.0,
        height: 720.0,
    };
    engine.step(current.clone(), 8000, true, false);
    settle(&mut engine, current, 8400);
    assert_eq!(
        engine.saved.profiles[&"a".repeat(64)].preferred,
        Size {
            width: 1100.0,
            height: 720.0
        }
    );
}

#[test]
fn topology_changes_cancel_resize_intent_and_shrink_after_drag_finishes() {
    let mut engine = Engine::new(Saved::default());
    let mut current = settle(&mut engine, scene(), 0);
    current.user_resizing = true;
    current.interacting = true;
    engine.step(current.clone(), 4000, false, false);
    current.screens[0].work = Size {
        width: 800.0,
        height: 600.0,
    };
    current.display_epoch += 1;
    assert!(engine.step(current.clone(), 5000, true, false).is_none());
    current.interacting = false;
    current.user_resizing = false;
    let fitted = settle(&mut engine, current, 6000);
    assert!(fitted.content.height < MINIMUM.height);
    assert_eq!(engine.saved.profiles[&"a".repeat(64)].preferred, DEFAULT);
}

#[test]
fn fullscreen_minimized_and_arranged_sizes_do_not_replace_normal_state() {
    let mut engine = Engine::new(Saved::default());
    let current = settle(&mut engine, scene(), 0);
    let saved = engine.saved.clone();
    for (index, mode) in [Mode::Fullscreen, Mode::Minimized, Mode::Arranged]
        .into_iter()
        .enumerate()
    {
        let mut temporary = current.clone();
        temporary.mode = mode;
        temporary.content = Size {
            width: 2000.0,
            height: 1300.0,
        };
        settle(&mut engine, temporary, 4000 + index as u64 * 4000);
        assert_eq!(engine.saved, saved);
    }
}

#[test]
fn maximize_retains_normal_size_and_restores_on_remaining_monitor() {
    let saved = Saved {
        last_monitor: Some("b".repeat(64)),
        last_maximized: true,
        ..Saved::default()
    };
    let mut engine = Engine::new(saved);
    let mut current = scene();
    current.screens.pop();
    let restored = settle(&mut engine, current, 0);
    assert_eq!(restored.mode, Mode::Maximized);
    assert_eq!(engine.saved.last_normal, DEFAULT);
    assert!(engine.saved.last_maximized);
}

#[test]
fn missing_or_duplicate_monitor_identity_is_not_used_for_exact_restore() {
    let mut current = scene();
    current.screens[1].key = current.screens[0].key.clone();
    super::remove_ambiguous_keys(current.screens.iter_mut());
    assert!(current.screens.iter().all(|s| s.key.is_none()));
    let mut engine = Engine::new(Saved {
        last_monitor: Some("a".repeat(64)),
        ..Saved::default()
    });
    settle(&mut engine, current, 0);
    assert!(engine.saved.last_monitor.is_none());
    assert!(engine.saved.profiles.is_empty());
}

#[test]
fn corrupt_future_and_legacy_state_fall_back_without_touching_unrelated_files() {
    let root = std::env::temp_dir().join(format!("md-geometry-{}", uuid::Uuid::new_v4()));
    assert_eq!(storage::load(&root), Saved::default());
    fs::create_dir_all(&root).unwrap();
    let old = r#"{"main":{"width":4800,"height":3200}}"#;
    fs::write(root.join(".window-state.json"), old).unwrap();
    fs::write(
        root.join("ui-preferences.json"),
        "synthetic-other-preference",
    )
    .unwrap();
    assert_eq!(storage::load(&root), Saved::default());
    for input in [
        "invalid".to_owned(),
        serde_json::to_string(&Saved {
            schema_version: 99,
            ..Saved::default()
        })
        .unwrap(),
        serde_json::to_string(&Saved {
            last_normal: Size {
                width: 0.0,
                height: 700.0,
            },
            ..Saved::default()
        })
        .unwrap(),
        "x".repeat(256 * 1024 + 1),
    ] {
        fs::write(root.join("window-geometry-v1.json"), input).unwrap();
        assert_eq!(storage::load(&root), Saved::default());
    }
    let saved = Saved {
        last_normal: Size {
            width: 1100.0,
            height: 710.0,
        },
        ..Saved::default()
    };
    storage::save(&root, &saved).unwrap();
    assert_eq!(storage::load(&root), saved);
    assert_eq!(
        fs::read_to_string(root.join(".window-state.json")).unwrap(),
        old
    );
    assert_eq!(
        fs::read_to_string(root.join(".window-state.pre-logical.json")).unwrap(),
        old
    );
    assert_eq!(
        fs::read_to_string(root.join("ui-preferences.json")).unwrap(),
        "synthetic-other-preference"
    );
    let blocked = root.join("not-a-folder");
    fs::write(&blocked, "synthetic").unwrap();
    assert!(storage::save(&blocked, &saved).is_err());
    assert_eq!(
        fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count(),
        0
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn delayed_write_cannot_overwrite_the_newer_close_flush() {
    let root = std::env::temp_dir().join(format!("md-geometry-writes-{}", uuid::Uuid::new_v4()));
    let service = super::Service {
        engine: std::sync::Mutex::new(Engine::new(Saved::default())),
        root: Some(root.clone()),
        started: std::time::Instant::now(),
        signals: Default::default(),
        pending: Default::default(),
        writer: std::sync::Mutex::new(0),
    };
    let recent = Saved {
        last_normal: Size {
            width: 1100.0,
            height: 710.0,
        },
        ..Saved::default()
    };
    service.persist(3, &recent);
    service.persist(2, &Saved::default());
    assert_eq!(storage::load(&root), recent);
    fs::remove_dir_all(root).unwrap();
}
