//! AppKit owns point coordinates. No conversion using a possibly stale window backing scale.
use super::{policy::*, Signals};
use block2::RcBlock;
use objc2::{rc::Retained, runtime::ProtocolObject, MainThreadMarker};
use objc2_app_kit::{NSEvent, NSScreen, NSWindow, NSWindowStyleMask};
use objc2_foundation::{
    ns_string, NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize,
};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    ffi::c_void,
    ptr::NonNull,
    sync::{atomic::Ordering, Arc},
};
use tauri::WebviewWindow;

#[repr(C)]
struct UuidBytes {
    bytes: [u8; 16],
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFUUIDGetUUIDBytes(uuid: *const c_void) -> UuidBytes;
    fn CFRelease(object: *const c_void);
}

thread_local! {
    static OBSERVERS: RefCell<Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>> = const { RefCell::new(Vec::new()) };
}

fn native(window: &WebviewWindow) -> Option<&NSWindow> {
    MainThreadMarker::new()?;
    // Tauri retains the NSWindow for the lifetime of this call, on the main thread.
    unsafe { (window.ns_window().ok()? as *const NSWindow).as_ref() }
}

fn id(screen: &NSScreen) -> Option<u32> {
    let value = screen
        .deviceDescription()
        .objectForKey(ns_string!("NSScreenNumber"))?;
    // NSScreenNumber is documented by AppKit as an NSNumber/CGDirectDisplayID.
    Some(unsafe { objc2::msg_send![&*value, unsignedIntValue] })
}

fn key(display: u32) -> Option<String> {
    unsafe {
        let uuid = CGDisplayCreateUUIDFromDisplayID(display);
        if uuid.is_null() {
            return None;
        }
        let bytes = CFUUIDGetUUIDBytes(uuid).bytes;
        CFRelease(uuid);
        Some(hex::encode(Sha256::digest(bytes)))
    }
}

fn screens() -> Option<Vec<(Screen, Retained<NSScreen>)>> {
    let native = NSScreen::screens(MainThreadMarker::new()?);
    let mut result: Vec<_> = native
        .iter()
        .enumerate()
        .map(|(index, screen)| {
            let display = id(&screen);
            let rect = screen.visibleFrame();
            let info = Screen {
                token: display.map_or_else(|| format!("screen-{index}"), |v| v.to_string()),
                key: display.and_then(key),
                work: Size {
                    width: rect.size.width,
                    height: rect.size.height,
                },
                origin: Point {
                    x: rect.origin.x,
                    y: rect.origin.y,
                },
                scale: screen.backingScaleFactor(),
            };
            (info, screen)
        })
        .collect();
    super::remove_ambiguous_keys(result.iter_mut().map(|(screen, _)| screen));
    Some(result)
}

pub fn snapshot(window: &WebviewWindow, signals: &Signals) -> Option<Snapshot> {
    let window = native(window)?;
    let screens = screens()?;
    let current_id = window
        .screen()
        .as_deref()
        .and_then(id)
        .map(|v| v.to_string());
    let (current, screen) = screens
        .iter()
        .find(|(s, _)| Some(&s.token) == current_id.as_ref())
        .or_else(|| screens.first())?;
    let frame = window.frame();
    let content = window.contentRectForFrameRect(frame);
    let work = screen.visibleFrame();
    let offset = Point {
        x: frame.origin.x - work.origin.x,
        y: work.origin.y + work.size.height - frame.origin.y - frame.size.height,
    };
    let mode = if window.isMiniaturized() {
        Mode::Minimized
    } else if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        Mode::Fullscreen
    } else if window.isZoomed() {
        Mode::Maximized
    } else {
        Mode::Normal
    };
    Some(Snapshot {
        current: current.token.clone(),
        screens: screens.into_iter().map(|(s, _)| s).collect(),
        content: Size {
            width: content.size.width,
            height: content.size.height,
        },
        offset,
        decoration: Size {
            width: (frame.size.width - content.size.width).max(0.0),
            height: (frame.size.height - content.size.height).max(0.0),
        },
        mode,
        interacting: NSEvent::pressedMouseButtons() != 0 || window.inLiveResize(),
        user_resizing: window.inLiveResize(),
        display_epoch: signals.display_epoch.load(Ordering::Relaxed),
    })
}

pub fn apply(window: &WebviewWindow, placement: &Placement) -> bool {
    let Some(window) = native(window) else {
        return false;
    };
    let Some(screens) = screens() else {
        return false;
    };
    let Some((_, screen)) = screens.iter().find(|(s, _)| s.token == placement.screen) else {
        return false;
    };
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) || window.isMiniaturized() {
        return false;
    }
    let content = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(placement.content.width, placement.content.height),
    );
    let mut frame = window.frameRectForContentRect(content);
    let work = screen.visibleFrame();
    frame.origin = NSPoint::new(
        work.origin.x + placement.offset.x,
        work.origin.y + work.size.height - placement.offset.y - frame.size.height,
    );
    window.setContentMinSize(NSSize::new(
        placement.minimum.width,
        placement.minimum.height,
    ));
    window.setFrame_display(frame, false);
    if placement.maximize && !window.isZoomed() {
        window.zoom(None);
    }
    true
}

pub fn observe(window: &WebviewWindow, signals: Arc<Signals>) {
    let Some(window) = native(window) else {
        return;
    };
    let center = NSNotificationCenter::defaultCenter();
    // Observers only mark atomics: setFrame may synchronously deliver notifications
    // while the controller holds its mutex. Never call back into it here.
    unsafe {
        for (name, object, resized) in [
            (
                objc2_app_kit::NSApplicationDidChangeScreenParametersNotification,
                None,
                false,
            ),
            (
                objc2_app_kit::NSWindowDidChangeScreenNotification,
                Some(window.as_ref()),
                false,
            ),
            (
                objc2_app_kit::NSWindowDidEndLiveResizeNotification,
                Some(window.as_ref()),
                true,
            ),
        ] {
            let signals = signals.clone();
            let block = RcBlock::new(move |_: NonNull<NSNotification>| {
                if resized {
                    signals.user_resize_ended.store(true, Ordering::Relaxed);
                } else {
                    signals.display_epoch.fetch_add(1, Ordering::Relaxed);
                }
            });
            let observer =
                center.addObserverForName_object_queue_usingBlock(Some(name), object, None, &block);
            OBSERVERS.with(|observers| observers.borrow_mut().push(observer));
        }
    }
}

pub fn stop_observing() {
    OBSERVERS.with(|observers| {
        for observer in observers.borrow_mut().drain(..) {
            unsafe {
                NSNotificationCenter::defaultCenter().removeObserver(observer.as_ref());
            }
        }
    });
}
