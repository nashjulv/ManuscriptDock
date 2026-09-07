//! Rust-owned desktop geometry. No IPC, user-supplied paths or WebView permissions.
#[cfg(target_os = "macos")]
mod macos;
mod policy;
mod storage;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use fallback as platform;

use policy::{Engine, Saved, Screen};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{App, AppHandle, Manager, WebviewWindow};

#[derive(Default)]
pub struct Signals {
    display_epoch: AtomicU64,
    user_resize_ended: AtomicBool,
    #[cfg(target_os = "windows")]
    resize_active: AtomicBool,
    queued: AtomicBool,
    stopped: AtomicBool,
}

struct Service {
    engine: Mutex<Engine>,
    root: Option<PathBuf>,
    started: Instant,
    signals: Arc<Signals>,
    pending: Mutex<Option<(u64, Saved)>>,
    writer: Mutex<u64>,
}

fn remove_ambiguous_keys<'a>(screens: impl Iterator<Item = &'a mut Screen>) {
    let mut screens: Vec<_> = screens.collect();
    let mut counts = HashMap::new();
    for screen in &screens {
        if let Some(key) = &screen.key {
            *counts.entry(key.clone()).or_insert(0) += 1;
        }
    }
    for screen in &mut screens {
        if screen
            .key
            .as_ref()
            .is_some_and(|key| counts.get(key).copied().unwrap_or(0) > 1)
        {
            screen.key = None;
        }
    }
}

impl Service {
    fn persist(&self, revision: u64, saved: &Saved) {
        let Some(root) = &self.root else {
            return;
        };
        // Also used by close/quit: an older queued write can never replace a newer flush.
        let Ok(mut last_written) = self.writer.lock() else {
            return;
        };
        if revision <= *last_written {
            return;
        }
        match storage::save(root, saved) {
            Ok(()) => *last_written = revision,
            Err(_) => eprintln!("WINDOW_GEOMETRY_SAVE_FAILED"),
        }
    }

    fn pump(&self, window: &WebviewWindow, closing: bool) {
        let now = self.started.elapsed().as_millis() as u64;
        let snapshot = platform::snapshot(window, &self.signals);
        let Ok(mut engine) = self.engine.lock() else {
            return;
        };
        let ended = self
            .signals
            .user_resize_ended
            .swap(false, Ordering::Relaxed);
        let placement = snapshot.and_then(|s| engine.step(s, now, ended, closing));
        if let Some(placement) = placement {
            if !platform::apply(window, &placement) {
                eprintln!("WINDOW_GEOMETRY_APPLY_FAILED");
            }
        }
        if !closing && !engine.shown && (engine.ready_to_show(now) || now >= 2000) {
            // Bounded fallback: missing monitor information must never hide the app forever.
            match window.show() {
                Ok(()) => engine.shown = true,
                Err(_) => eprintln!("WINDOW_GEOMETRY_SHOW_FAILED"),
            }
        }
        if engine.restored {
            let revision = engine.revision;
            let saved = engine.saved.clone();
            drop(engine);
            if closing {
                self.persist(revision, &saved);
            } else if let Ok(mut pending) = self.pending.lock() {
                *pending = Some((revision, saved));
            }
        }
    }
}

fn schedule(app: &AppHandle, service: &Arc<Service>) {
    if service.signals.stopped.load(Ordering::Relaxed)
        || service.signals.queued.swap(true, Ordering::Relaxed)
    {
        return;
    }
    let handle = app.clone();
    let task = service.clone();
    if app
        .run_on_main_thread(move || {
            if let Some(window) = handle.get_webview_window("main") {
                task.pump(&window, false);
            }
            task.signals.queued.store(false, Ordering::Relaxed);
        })
        .is_err()
    {
        service.signals.queued.store(false, Ordering::Relaxed);
    }
}

pub fn install(app: &mut App) {
    let root = app.path().app_config_dir().ok();
    let saved = root.as_deref().map(storage::load).unwrap_or_default();
    let service = Arc::new(Service {
        engine: Mutex::new(Engine::new(saved)),
        root,
        started: Instant::now(),
        signals: Arc::default(),
        pending: Mutex::new(None),
        writer: Mutex::new(0),
    });
    if let Some(window) = app.get_webview_window("main") {
        platform::observe(&window, service.signals.clone());
    }
    app.manage(service.clone());
    let handle = app.handle().clone();
    // Notifications/events provide prompt updates; a bounded, low-rate rescan also
    // catches resume and topology changes not surfaced by Tauri on every platform.
    std::thread::spawn(move || {
        while !service.signals.stopped.load(Ordering::Relaxed) {
            schedule(&handle, &service);
            let pending = service.pending.lock().ok().and_then(|mut p| p.take());
            if let Some((revision, saved)) = pending {
                service.persist(revision, &saved);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

pub fn event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
        flush(window.app_handle());
    } else if let Some(service) = window.app_handle().try_state::<Arc<Service>>() {
        schedule(window.app_handle(), &service);
    }
}

pub fn flush(app: &AppHandle) {
    if let (Some(service), Some(window)) = (
        app.try_state::<Arc<Service>>(),
        app.get_webview_window("main"),
    ) {
        service.pump(&window, true);
    }
}

pub fn stop(app: &AppHandle) {
    flush(app);
    if let Some(service) = app.try_state::<Arc<Service>>() {
        service.signals.stopped.store(true, Ordering::Relaxed);
        if let Ok(engine) = service.engine.lock() {
            service.persist(engine.revision, &engine.saved);
        }
    }
    platform::stop_observing();
}

#[cfg(test)]
mod tests;
