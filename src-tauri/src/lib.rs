mod actions;
mod clip;
mod platform;
mod store;
mod watch;

use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub const SHORTCUT: &str = "Alt+S";

#[derive(Serialize, Clone)]
pub struct TargetInfo {
    /// Session-scoped id used to dock (pid on macOS, hwnd on Windows).
    pub id: String,
    /// Stable key remembered across launches (bundle id on macOS, exe name on Windows).
    pub key: String,
    pub name: String,
}

#[tauri::command]
fn list_targets() -> Vec<TargetInfo> {
    platform::list_targets()
}

#[tauri::command]
fn set_target(app: AppHandle, id: String) -> Result<(), String> {
    platform::dock(app, &id)
}

#[tauri::command]
fn clear_target(app: AppHandle) {
    platform::undock(&app);
}

#[tauri::command]
fn paste_note(app: AppHandle, text: String) {
    std::thread::spawn(move || actions::paste(&app, text));
}

/// Clicked from the chip: the click moved focus to Wings, so hand it back before cutting.
#[tauri::command]
fn capture_now(app: AppHandle) {
    std::thread::spawn(move || {
        if platform::activate_target() { actions::capture(&app); } else { actions::emit_error(&app, "没能切换到目标窗口"); }
    });
}

#[tauri::command]
fn set_prefs(app: AppHandle, float: bool, follow: bool, chip: bool) {
    watch::FOLLOW.store(follow, Ordering::SeqCst);
    watch::CHIP.store(chip, Ordering::SeqCst);
    platform::set_float(&app, float);
}

#[tauri::command]
fn permission_granted(prompt: bool) -> bool {
    platform::accessibility_trusted(prompt)
}

#[tauri::command]
fn open_permission_settings() {
    platform::open_accessibility_settings();
}

#[tauri::command]
fn shortcut_label() -> String {
    if cfg!(target_os = "macos") { "⌥S".into() } else { SHORTCUT.into() }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            app.global_shortcut().on_shortcut(SHORTCUT, move |_, _, ev| {
                if ev.state == ShortcutState::Pressed {
                    let h = handle.clone();
                    std::thread::spawn(move || actions::capture(&h));
                }
            })?;
            // The "save to Wings" chip that sits on the target's input box; hidden until an input is focused.
            WebviewWindowBuilder::new(app, "chip", WebviewUrl::App("index.html".into()))
                .title("Wings")
                .inner_size(platform::CHIP_W, platform::CHIP_H)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .focusable(false)
                .visible(false)
                .build()?;
            watch::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|win, ev| {
            // Leaving Wings for a third app must hide the docked bar; the target sends no event for that.
            if let WindowEvent::Focused(false) = ev { platform::refresh_later(win.app_handle()); }
        })
        .invoke_handler(tauri::generate_handler![
            list_targets, set_target, clear_target, paste_note, capture_now, set_prefs,
            permission_granted, open_permission_settings, shortcut_label,
            store::load_notes, store::save_notes
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wings");
}
