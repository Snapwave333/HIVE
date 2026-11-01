use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItemBuilder::with_id("open", "Open H.I.V.E.").build(app)?;
    let pause_item = MenuItemBuilder::with_id("pause", "Pause Agency").build(app)?;
    let resume_item = MenuItemBuilder::with_id("resume", "Resume Agency").build(app)?;
    let mute_item = MenuItemBuilder::with_id("toggle_mute", "Toggle Audio Mute (Strict: Muted)").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Stop & Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_item)
        .separator()
        .item(&pause_item)
        .item(&resume_item)
        .separator()
        .item(&mute_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .tooltip("H.I.V.E. Autonomous Agency")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            match event.id().as_ref() {
                "open" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.maximize();
                        let _ = window.set_focus();
                    }
                }
                "pause" => {
                    let state = app_handle.state::<crate::AppState>();
                    state.daemon.pause();
                }
                "resume" => {
                    let state = app_handle.state::<crate::AppState>();
                    state.daemon.resume();
                }
                "toggle_mute" => {
                    let state = app_handle.state::<crate::AppState>();
                    let is_muted = state.toggle_audio_mute();
                    log::info!("Audio mute toggled to: {}", is_muted);
                }
                "quit" => {
                    let state = app_handle.state::<crate::AppState>();
                    state.daemon.stop();
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.maximize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
