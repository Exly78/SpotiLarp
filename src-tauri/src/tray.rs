use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};

const TRAY_ID: &str = "main";
const MAX_TOOLTIP_CHARS: usize = 120;

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn create(app: &App) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "Play / Pause").build(app)?;
    let next = MenuItemBuilder::with_id("next", "Next").build(app)?;
    let previous = MenuItemBuilder::with_id("previous", "Previous").build(app)?;
    let show = MenuItemBuilder::with_id("show", "Show SpotiLarp").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&toggle, &next, &previous])
        .item(&PredefinedMenuItem::separator(app)?)
        .items(&[&show, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("SpotiLarp")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            action @ ("toggle" | "next" | "previous") => {
                let _ = app.emit("media-control", serde_json::json!({ "action": action }));
            }
            "show" => show_main_window(app),
            "quit" => {
                if let Some(window) = app.get_webview_window("main") {
                    crate::window_state::save(&window);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn set_tooltip(app: &AppHandle, text: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let text: String = text.chars().take(MAX_TOOLTIP_CHARS).collect();
        let _ = tray.set_tooltip(Some(text));
    }
}
