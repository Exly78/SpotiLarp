//! Previous / Play-Pause / Next buttons under SpotiLarp's taskbar thumbnail
//! preview on Windows (an idea from Spotifast). Presses reach the UI as
//! "media-control" events, like the tray menu's.
//!
//! Windows only takes the buttons once the window has a taskbar button,
//! which it announces with the "TaskbarButtonCreated" message (again after
//! Explorer restarts, or the window comes back from the tray), so a window
//! subclass waits for it. Everything here runs on the main thread, which owns
//! the window. Its state is only ever `try_borrow`ed: a panic inside the
//! window procedure would abort, and Windows may call it re-entrantly.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;

use tauri::{AppHandle, Emitter};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, SetWindowSubclass, TaskbarList, THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON,
    THB_TOOLTIP, THUMBBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, RegisterWindowMessageW, HICON, ICONINFO, WM_COMMAND};

const SUBCLASS_ID: usize = 0x5350_4c52; // "SPLR"
const PREVIOUS: u32 = 1;
const PLAY_PAUSE: u32 = 2;
const NEXT: u32 = 3;
const SUPERSAMPLE: u32 = 4;

/// Shapes in a unit square: convex polygons, clockwise.
type Shape = &'static [&'static [(f32, f32)]];
const PREVIOUS_SHAPE: Shape = &[&[(0.18, 0.2), (0.3, 0.2), (0.3, 0.8), (0.18, 0.8)], &[(0.8, 0.2), (0.8, 0.8), (0.34, 0.5)]];
const NEXT_SHAPE: Shape = &[&[(0.2, 0.2), (0.66, 0.5), (0.2, 0.8)], &[(0.7, 0.2), (0.82, 0.2), (0.82, 0.8), (0.7, 0.8)]];
const PLAY_SHAPE: Shape = &[&[(0.28, 0.17), (0.84, 0.5), (0.28, 0.83)]];
const PAUSE_SHAPE: Shape = &[
    &[(0.24, 0.18), (0.42, 0.18), (0.42, 0.82), (0.24, 0.82)],
    &[(0.58, 0.18), (0.76, 0.18), (0.76, 0.82), (0.58, 0.82)],
];

struct Icons {
    previous: HICON,
    play: HICON,
    pause: HICON,
    next: HICON,
}

struct Bar {
    hwnd: HWND,
    taskbar: Option<ITaskbarList3>,
    icons: Option<Icons>,
    playing: bool,
}

thread_local! {
    static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();

/// Hooks the main window; call on the main thread before it's first shown.
pub fn attach(app: &AppHandle, hwnd: isize) {
    let _ = APP.set(app.clone());
    let message = unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) };
    if message == 0 {
        return;
    }
    let _ = TASKBAR_CREATED.set(message);
    let hwnd = HWND(hwnd as *mut c_void);
    BAR.with(|bar| {
        if let Ok(mut bar) = bar.try_borrow_mut() {
            *bar = Some(Bar { hwnd, taskbar: None, icons: None, playing: false });
        }
    });
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0);
    }
}

/// Switches the middle button between Play and Pause.
pub fn set_playing(app: &AppHandle, playing: bool) {
    let _ = app.run_on_main_thread(move || {
        BAR.with(|bar| {
            if let Some(bar) = bar.try_borrow_mut().ok().as_deref_mut().and_then(Option::as_mut) {
                if bar.playing != playing {
                    bar.playing = playing;
                    bar.update();
                }
            }
        });
    });
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if Some(&message) == TASKBAR_CREATED.get() {
        BAR.with(|bar| {
            if let Some(bar) = bar.try_borrow_mut().ok().as_deref_mut().and_then(Option::as_mut) {
                bar.install();
            }
        });
    } else if message == WM_COMMAND && (wparam.0 >> 16) as u32 & 0xffff == THBN_CLICKED {
        let action = match (wparam.0 & 0xffff) as u32 {
            PREVIOUS => Some("previous"),
            PLAY_PAUSE => Some("toggle"),
            NEXT => Some("next"),
            _ => None,
        };
        if let (Some(action), Some(app)) = (action, APP.get()) {
            let _ = app.emit("media-control", serde_json::json!({ "action": action }));
            return LRESULT(0);
        }
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

impl Bar {
    /// Puts the buttons on a (new) taskbar button.
    fn install(&mut self) {
        // Drawn for the taskbar's current theme and the window's DPI each time
        // a button appears; a few small icons, kept for the app's lifetime.
        self.icons = make_icons(self.hwnd);
        let taskbar = match unsafe { CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) } {
            Ok(taskbar) => taskbar,
            Err(_) => return,
        };
        if unsafe { taskbar.HrInit() }.is_err() {
            return;
        }
        let buttons = self.buttons();
        let added = unsafe { taskbar.ThumbBarAddButtons(self.hwnd, &buttons) };
        // A taskbar button that already has them takes an update instead.
        if added.is_err() {
            let _ = unsafe { taskbar.ThumbBarUpdateButtons(self.hwnd, &buttons) };
        }
        self.taskbar = Some(taskbar);
    }

    fn update(&self) {
        if let Some(taskbar) = &self.taskbar {
            let _ = unsafe { taskbar.ThumbBarUpdateButtons(self.hwnd, &self.buttons()) };
        }
    }

    fn buttons(&self) -> [THUMBBUTTON; 3] {
        let icon = |pick: fn(&Icons) -> HICON| self.icons.as_ref().map(pick).unwrap_or_default();
        let play_pause = if self.playing { icon(|i| i.pause) } else { icon(|i| i.play) };
        [
            button(PREVIOUS, icon(|i| i.previous), "Previous"),
            button(PLAY_PAUSE, play_pause, if self.playing { "Pause" } else { "Play" }),
            button(NEXT, icon(|i| i.next), "Next"),
        ]
    }
}

fn button(id: u32, icon: HICON, tip: &str) -> THUMBBUTTON {
    let mut tooltip = [0u16; 260];
    for (slot, unit) in tooltip.iter_mut().zip(tip.encode_utf16().take(259)) {
        *slot = unit;
    }
    THUMBBUTTON {
        dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
        iId: id,
        hIcon: icon,
        szTip: tooltip,
        dwFlags: THBF_ENABLED,
        ..Default::default()
    }
}

fn make_icons(hwnd: HWND) -> Option<Icons> {
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let size = (16 * dpi / 96).clamp(16, 64);
    // White glyphs on the dark taskbar, near-black on a light one.
    let color = if light_taskbar() { [0x1a, 0x1a, 0x1a] } else { [0xff, 0xff, 0xff] };
    Some(Icons {
        previous: icon(size, PREVIOUS_SHAPE, color)?,
        play: icon(size, PLAY_SHAPE, color)?,
        pause: icon(size, PAUSE_SHAPE, color)?,
        next: icon(size, NEXT_SHAPE, color)?,
    })
}

fn light_taskbar() -> bool {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    result.is_ok() && value == 1
}

/// An anti-aliased glyph as a 32-bit icon.
fn icon(size: u32, shape: Shape, [r, g, b]: [u8; 3]) -> Option<HICON> {
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size as i32,
        // Negative: rows run top to bottom.
        biHeight: -(size as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let info = BITMAPINFO { bmiHeader: header, ..Default::default() };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let color = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }.ok()?;
    if bits.is_null() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(color.0));
        }
        return None;
    }
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize) };
    for y in 0..size {
        for x in 0..size {
            let alpha = (coverage(shape, x, y, size) * 255.0).round() as u8;
            let at = ((y * size + x) * 4) as usize;
            pixels[at..at + 4].copy_from_slice(&[b, g, r, alpha]);
        }
    }
    let mask = unsafe { CreateBitmap(size as i32, size as i32, 1, 1, None) };
    let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
    let icon = unsafe { CreateIconIndirect(&info) }.ok();
    // The icon keeps copies of both bitmaps.
    unsafe {
        let _ = DeleteObject(HGDIOBJ(mask.0));
        let _ = DeleteObject(HGDIOBJ(color.0));
    }
    icon
}

/// How much of pixel (x, y) the shape covers, from a grid of samples.
fn coverage(shape: Shape, x: u32, y: u32, size: u32) -> f32 {
    let mut inside = 0;
    for sy in 0..SUPERSAMPLE {
        for sx in 0..SUPERSAMPLE {
            let px = (x as f32 + (sx as f32 + 0.5) / SUPERSAMPLE as f32) / size as f32;
            let py = (y as f32 + (sy as f32 + 0.5) / SUPERSAMPLE as f32) / size as f32;
            if shape.iter().any(|polygon| contains(polygon, px, py)) {
                inside += 1;
            }
        }
    }
    inside as f32 / (SUPERSAMPLE * SUPERSAMPLE) as f32
}

/// Whether a convex polygon (either winding) contains a point.
fn contains(polygon: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut sign = 0.0f32;
    for (i, &(ax, ay)) in polygon.iter().enumerate() {
        let (bx, by) = polygon[(i + 1) % polygon.len()];
        let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        if cross != 0.0 {
            if sign != 0.0 && cross.signum() != sign {
                return false;
            }
            sign = cross.signum();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cover_their_middle_and_not_their_corners() {
        for shape in [PREVIOUS_SHAPE, NEXT_SHAPE, PLAY_SHAPE] {
            assert_eq!(coverage(shape, 0, 0, 16), 0.0);
            assert_eq!(coverage(shape, 15, 15, 16), 0.0);
        }
        assert_eq!(coverage(PLAY_SHAPE, 7, 8, 16), 1.0);
        assert_eq!(coverage(PAUSE_SHAPE, 7, 8, 16), 0.0);
        assert_eq!(coverage(PAUSE_SHAPE, 5, 8, 16), 1.0);
    }
}
