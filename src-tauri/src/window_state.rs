use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::config::{self, WindowGeometry};
use crate::state::AppState;

pub fn save(window: &WebviewWindow) {
    let mini = window
        .state::<AppState>()
        .mini_restore
        .lock()
        .ok()
        .and_then(|restore| restore.as_ref().map(|r| (r.size, r.maximized)));
    let maximized = match mini {
        Some((_, maximized)) => maximized,
        None => window.is_maximized().unwrap_or(false),
    };
    let normal = mini.is_none()
        && !maximized
        && !window.is_minimized().unwrap_or(false)
        && window.is_visible().unwrap_or(false);

    let current = match (window.outer_position(), window.inner_size()) {
        (Ok(position), Ok(size)) => Some(WindowGeometry {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized,
        }),
        _ => None,
    };
    let geometry = match (normal, config::load_window_geometry(), current) {
        (true, _, Some(current)) => current,
        (false, Some(previous), _) => WindowGeometry { maximized, ..previous },
        (false, None, Some(current)) => match mini {
            Some((size, _)) => WindowGeometry {
                width: size.width,
                height: size.height,
                ..current
            },
            None => current,
        },
        _ => return,
    };
    let _ = config::save_window_geometry(geometry);
}

pub fn restore(window: &WebviewWindow) {
    let Some(geometry) = config::load_window_geometry() else {
        return;
    };
    if geometry.width > 0 && geometry.height > 0 {
        let _ = window.set_size(PhysicalSize::new(geometry.width, geometry.height));
    }
    if is_on_screen(window, &geometry) {
        let _ = window.set_position(PhysicalPosition::new(geometry.x, geometry.y));
    }
    if geometry.maximized {
        let _ = window.maximize();
    }
}

fn is_on_screen(window: &WebviewWindow, geometry: &WindowGeometry) -> bool {
    let (x, y) = (geometry.x + 100, geometry.y + 20);
    window
        .available_monitors()
        .map(|monitors| {
            monitors.iter().any(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                x >= origin.x
                    && y >= origin.y
                    && x < origin.x + size.width as i32
                    && y < origin.y + size.height as i32
            })
        })
        .unwrap_or(false)
}
