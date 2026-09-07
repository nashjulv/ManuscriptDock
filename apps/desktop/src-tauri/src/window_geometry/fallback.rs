//! Conservative fallback for platforms without a supported native interaction adapter.
//! Safe restoration remains available; uncertain resize events never replace preferences.
use super::{policy::*, Signals};
use std::sync::{atomic::Ordering, Arc};
use tauri::WebviewWindow;

fn screens(window: &WebviewWindow) -> Option<Vec<Screen>> {
    Some(
        window
            .available_monitors()
            .ok()?
            .iter()
            .map(|m| {
                let area = m.work_area();
                let scale = m.scale_factor();
                Screen {
                    token: format!("{}:{}", m.position().x, m.position().y),
                    key: None,
                    scale,
                    work: Size {
                        width: f64::from(area.size.width) / scale,
                        height: f64::from(area.size.height) / scale,
                    },
                    origin: Point {
                        x: f64::from(area.position.x),
                        y: f64::from(area.position.y),
                    },
                }
            })
            .collect(),
    )
}

pub fn snapshot(window: &WebviewWindow, signals: &Signals) -> Option<Snapshot> {
    let monitor = window.current_monitor().ok()??;
    let scale = monitor.scale_factor();
    let position = window.outer_position().ok()?;
    let outer = window.outer_size().ok()?;
    let inner = window.inner_size().ok()?;
    let area = monitor.work_area();
    Some(Snapshot {
        screens: screens(window)?,
        current: format!("{}:{}", monitor.position().x, monitor.position().y),
        content: Size {
            width: f64::from(inner.width) / scale,
            height: f64::from(inner.height) / scale,
        },
        offset: Point {
            x: f64::from(position.x - area.position.x) / scale,
            y: f64::from(position.y - area.position.y) / scale,
        },
        decoration: Size {
            width: f64::from(outer.width.saturating_sub(inner.width)) / scale,
            height: f64::from(outer.height.saturating_sub(inner.height)) / scale,
        },
        mode: if window.is_fullscreen().ok()? {
            Mode::Fullscreen
        } else if window.is_minimized().ok()? {
            Mode::Minimized
        } else if window.is_maximized().ok()? {
            Mode::Maximized
        } else {
            Mode::Normal
        },
        // No reliable native drag-end signal here: avoid runtime repositioning.
        interacting: window.is_visible().unwrap_or(true),
        user_resizing: false,
        display_epoch: signals.display_epoch.load(Ordering::Relaxed),
    })
}

pub fn apply(window: &WebviewWindow, placement: &Placement) -> bool {
    let Some(screens) = screens(window) else {
        return false;
    };
    let Some(screen) = screens.iter().find(|s| s.token == placement.screen) else {
        return false;
    };
    let _ = window.set_min_size(Some(tauri::LogicalSize::new(
        placement.minimum.width,
        placement.minimum.height,
    )));
    if window
        .set_size(tauri::PhysicalSize::new(
            (placement.content.width * screen.scale).round() as u32,
            (placement.content.height * screen.scale).round() as u32,
        ))
        .is_err()
    {
        return false;
    }
    if window
        .set_position(tauri::PhysicalPosition::new(
            (screen.origin.x + placement.offset.x * screen.scale).round() as i32,
            (screen.origin.y + placement.offset.y * screen.scale).round() as i32,
        ))
        .is_err()
    {
        return false;
    }
    !placement.maximize || window.maximize().is_ok()
}

pub fn observe(_: &WebviewWindow, _: Arc<Signals>) {}
pub fn stop_observing() {}
