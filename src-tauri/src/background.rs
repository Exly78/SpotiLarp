//! Using less memory while the window is out of sight. A music player spends
//! most of its time minimized or in the tray, so then WebView2 is asked to
//! stop drawing and trim what it holds, and the page is told so it can let go
//! of its interface. The page keeps running: it decides what plays next.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::state::AppState;

pub const BACKGROUND_EVENT: &str = "app-background";
pub const CLOCK_EVENT: &str = "clock-tick";
const CLOCK_INTERVAL: Duration = Duration::from_secs(1);

/// Whether the window is minimized or hidden in the tray.
pub fn is_hidden(app: &AppHandle) -> bool {
    app.state::<AppState>().background.load(Ordering::SeqCst)
}

/// Call after anything that may have hidden, minimized, shown or restored the window.
pub fn update(window: &WebviewWindow) {
    if window.label() != "main" {
        return;
    }
    let hidden = !window.is_visible().unwrap_or(true) || window.is_minimized().unwrap_or(false);
    let state = window.state::<AppState>();
    if state.background.swap(hidden, Ordering::SeqCst) == hidden {
        return;
    }
    #[cfg(target_os = "windows")]
    set_webview_resources(window, hidden);
    let _ = window.emit(BACKGROUND_EVENT, hidden);
}

#[cfg(target_os = "windows")]
fn set_webview_resources(window: &WebviewWindow, hidden: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows_core::Interface;

    let _ = window.with_webview(move |webview| unsafe {
        let controller = webview.controller();
        // A minimized window's page would otherwise keep drawing for nobody.
        let _ = controller.SetIsVisible(!hidden);
        let Ok(core) = controller.CoreWebView2() else {
            return;
        };
        // Needs WebView2 Runtime 114 or newer; older ones just keep their memory.
        if let Ok(core) = core.cast::<ICoreWebView2_19>() {
            let level = if hidden {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            let _ = core.SetMemoryUsageTargetLevel(level);
        }
    });
}

/// A once-a-second tick for the page while it needs one (the sleep timer).
/// A hidden page's own timers can be held back by up to a minute; these
/// arrive on time.
pub fn set_clock(app: &AppHandle, active: bool) {
    let state = app.state::<AppState>();
    // Each start gets a number; a clock stops once a newer one starts or it's turned off.
    let generation = state.clock_generation.fetch_add(1, Ordering::SeqCst) + 1;
    if !active {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(CLOCK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if app.state::<AppState>().clock_generation.load(Ordering::SeqCst) != generation {
                break;
            }
            let _ = app.emit(CLOCK_EVENT, ());
        }
    });
}
