//! Per-monitor physical coordinates stay in this adapter. Policy uses logical
//! content sizes and offsets within one work area, including negative origins.
use super::{policy::*, Signals};
use sha2::{Digest, Sha256};
use std::{
    mem::size_of,
    ptr,
    sync::{atomic::Ordering, Arc},
};
use tauri::WebviewWindow;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::*,
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    UI::{
        HiDpi::*,
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

struct Monitor {
    info: Screen,
    work: RECT,
}

fn screens() -> Vec<Monitor> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let result = &mut *(data as *mut Vec<Monitor>);
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut info.monitorInfo) == 0 {
            return 1;
        }
        let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
        device.cb = size_of::<DISPLAY_DEVICEW>() as u32;
        let key = if EnumDisplayDevicesW(
            info.szDevice.as_ptr(),
            0,
            &mut device,
            EDD_GET_DEVICE_INTERFACE_NAME,
        ) != 0
        {
            let end = device
                .DeviceID
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(device.DeviceID.len());
            (end > 0).then(|| {
                hex::encode(Sha256::digest(
                    String::from_utf16_lossy(&device.DeviceID[..end])
                        .to_lowercase()
                        .as_bytes(),
                ))
            })
        } else {
            None
        };
        let (mut x, mut y) = (96, 96);
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) < 0 || x == 0 {
            x = 96;
        }
        let scale = f64::from(x) / 96.0;
        let work = info.monitorInfo.rcWork;
        result.push(Monitor {
            info: Screen {
                token: format!("{:x}", monitor as usize),
                key,
                scale,
                work: Size {
                    width: f64::from(work.right - work.left) / scale,
                    height: f64::from(work.bottom - work.top) / scale,
                },
                origin: Point {
                    x: f64::from(work.left),
                    y: f64::from(work.top),
                },
            },
            work,
        });
        1
    }
    let mut result: Vec<Monitor> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(collect),
            &mut result as *mut Vec<Monitor> as LPARAM,
        );
    }
    super::remove_ambiguous_keys(result.iter_mut().map(|m| &mut m.info));
    result
}

fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    Some(window.hwnd().ok()?.0 as HWND)
}

unsafe fn arranged(window: HWND, frame: &RECT) -> bool {
    // Available since Windows 10 1903. Resolve it at runtime for older Windows 10.
    let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    if let Some(proc) = GetProcAddress(
        GetModuleHandleW(user32.as_ptr()),
        c"IsWindowArranged".as_ptr().cast(),
    ) {
        let query: unsafe extern "system" fn(HWND) -> i32 = std::mem::transmute(proc);
        return query(window) != 0;
    }
    // Older Windows retains the unsnapped restore rectangle. Only compare sizes:
    // WINDOWPLACEMENT origins use workspace coordinates, unlike GetWindowRect.
    let mut placement: WINDOWPLACEMENT = std::mem::zeroed();
    placement.length = size_of::<WINDOWPLACEMENT>() as u32;
    if GetWindowPlacement(window, &mut placement) == 0 {
        return false;
    }
    let normal = placement.rcNormalPosition;
    ((normal.right - normal.left) - (frame.right - frame.left)).abs() > 2
        || ((normal.bottom - normal.top) - (frame.bottom - frame.top)).abs() > 2
}

pub fn snapshot(window: &WebviewWindow, signals: &Signals) -> Option<Snapshot> {
    let hwnd = hwnd(window)?;
    let screens = screens();
    unsafe {
        let token = format!(
            "{:x}",
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) as usize
        );
        let current = screens
            .iter()
            .find(|m| m.info.token == token)
            .or_else(|| screens.first())?;
        let (mut frame, mut client): (RECT, RECT) = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut frame) == 0 || GetClientRect(hwnd, &mut client) == 0 {
            return None;
        }
        let scale = current.info.scale;
        let content = Size {
            width: f64::from(client.right - client.left) / scale,
            height: f64::from(client.bottom - client.top) / scale,
        };
        let mut gui: GUITHREADINFO = std::mem::zeroed();
        gui.cbSize = size_of::<GUITHREADINFO>() as u32;
        let moving = GetGUIThreadInfo(GetWindowThreadProcessId(hwnd, ptr::null_mut()), &mut gui)
            != 0
            && gui.flags & GUI_INMOVESIZE != 0;
        let mode = if IsIconic(hwnd) != 0 {
            Mode::Minimized
        } else if window.is_fullscreen().unwrap_or(false) {
            Mode::Fullscreen
        } else if IsZoomed(hwnd) != 0 {
            Mode::Maximized
        } else if !moving && arranged(hwnd, &frame) {
            Mode::Arranged
        } else {
            Mode::Normal
        };
        let offset = Point {
            x: f64::from(frame.left - current.work.left) / scale,
            y: f64::from(frame.top - current.work.top) / scale,
        };
        Some(Snapshot {
            current: current.info.token.clone(),
            screens: screens.into_iter().map(|m| m.info).collect(),
            content,
            offset,
            decoration: Size {
                width: (f64::from(frame.right - frame.left) / scale - content.width).max(0.0),
                height: (f64::from(frame.bottom - frame.top) / scale - content.height).max(0.0),
            },
            mode,
            interacting: moving || GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0,
            user_resizing: signals.resize_active.load(Ordering::Relaxed),
            display_epoch: signals.display_epoch.load(Ordering::Relaxed),
        })
    }
}

pub fn apply(window: &WebviewWindow, placement: &Placement) -> bool {
    let Some(hwnd) = hwnd(window) else {
        return false;
    };
    let screens = screens();
    let Some(target) = screens.iter().find(|m| m.info.token == placement.screen) else {
        return false;
    };
    let scale = target.info.scale;
    unsafe {
        if IsIconic(hwnd) != 0 || window.is_fullscreen().unwrap_or(false) {
            return false;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: (placement.content.width * scale).round() as i32,
            bottom: (placement.content.height * scale).round() as i32,
        };
        if AdjustWindowRectExForDpi(
            &mut rect,
            GetWindowLongPtrW(hwnd, GWL_STYLE) as u32,
            0,
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32,
            (scale * 96.0).round() as u32,
        ) == 0
        {
            return false;
        }
        // Release an old larger-screen minimum before moving onto a very small work area.
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(1.0, 1.0)));
        let success = SetWindowPos(
            hwnd,
            ptr::null_mut(),
            target.work.left + (placement.offset.x * scale).round() as i32,
            target.work.top + (placement.offset.y * scale).round() as i32,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) != 0;
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(
            placement.minimum.width,
            placement.minimum.height,
        )));
        if success && placement.maximize {
            ShowWindow(hwnd, SW_MAXIMIZE);
        }
        success
    }
}

const SUBCLASS_ID: usize = 0x4d444745;
unsafe extern "system" fn observe_message(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let signals = &*(data as *const Arc<Signals>);
    match message {
        WM_ENTERSIZEMOVE => signals.resize_active.store(false, Ordering::Relaxed),
        WM_SIZING => signals.resize_active.store(true, Ordering::Relaxed),
        WM_EXITSIZEMOVE => {
            if signals.resize_active.swap(false, Ordering::Relaxed) {
                signals.user_resize_ended.store(true, Ordering::Relaxed);
            }
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            signals.display_epoch.fetch_add(1, Ordering::Relaxed);
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(observe_message), id);
            drop(Box::from_raw(data as *mut Arc<Signals>));
        }
        _ => {}
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}

pub fn observe(window: &WebviewWindow, signals: Arc<Signals>) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    let data = Box::into_raw(Box::new(signals));
    unsafe {
        if SetWindowSubclass(hwnd, Some(observe_message), SUBCLASS_ID, data as usize) == 0 {
            drop(Box::from_raw(data));
            eprintln!("WINDOW_GEOMETRY_OBSERVER_FAILED");
        }
    }
}

// WM_NCDESTROY removes the observer and releases its Arc on the owning UI thread.
pub fn stop_observing() {}
