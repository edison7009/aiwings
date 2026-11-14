mod actions;
mod clip;
mod platform;
mod store;
mod update;
mod watch;

use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Default save shortcut; the user can pick another single modifier + single key in settings.
pub const SHORTCUT: &str = "Alt+S";
static CURRENT_SHORTCUT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn register_shortcut(app: &AppHandle, combo: &str) -> Result<(), String> {
    let handle = app.clone();
    app.global_shortcut().on_shortcut(combo, move |_, _, ev| {
        if ev.state == ShortcutState::Pressed {
            let h = handle.clone();
            std::thread::spawn(move || actions::capture(&h));
        }
    }).map_err(|e| e.to_string())
}

/// "Alt+S" as the user sees it: ⌥S on macOS, Ctrl+S / Win+S style on Windows.
fn label_for(combo: &str) -> String {
    if !cfg!(target_os = "macos") { return combo.replace("Control", "Ctrl").replace("Super", "Win"); }
    combo.split('+').map(|part| match part {
        "Alt" => "⌥", "Control" => "⌃", "Shift" => "⇧", "Super" => "⌘", key => key,
    }.to_string()).collect()
}

/// Swaps the save shortcut. Accepts exactly one modifier and one key; if the new combination is taken
/// (by the system or another app) the old one is put back and an error explains why.
#[tauri::command]
fn set_shortcut(app: AppHandle, combo: String) -> Result<String, String> {
    let parts: Vec<&str> = combo.split('+').collect();
    let valid = parts.len() == 2 && ["Alt", "Control", "Shift", "Super"].contains(&parts[0]) && !parts[1].is_empty();
    if !valid { return Err("请用一个修饰键加一个键，比如 ⌥S".into()); }
    // The platform's command key (⌘ on macOS, Ctrl on Windows) would shadow Copy, Paste, Save… in every app.
    let command_key = if cfg!(target_os = "macos") { "Super" } else { "Control" };
    if parts[0] == command_key {
        return Err(if cfg!(target_os = "macos") { "⌘ 组合会覆盖各软件自带的快捷键（如 ⌘C），请用 ⌥ 或 ⌃" } else { "Ctrl 组合会覆盖各软件自带的快捷键（如 Ctrl+C），请用 Alt" }.into());
    }
    let mut current = CURRENT_SHORTCUT.lock().unwrap();
    if *current == combo { return Ok(label_for(&combo)); }
    let _ = app.global_shortcut().unregister(current.as_str());
    if register_shortcut(&app, &combo).is_err() {
        let _ = register_shortcut(&app, &current);
        return Err("这个组合已被系统或其他软件占用，请换一个".into());
    }
    *current = combo.clone();
    Ok(label_for(&combo))
}

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

/// Apps for the settings menu: AI apps installed on this machine, then any other open app (WeChat, say).
/// `ai` marks the ones followed by default; `save` the ones Wings can also save from (the rest: send and follow).
#[derive(Serialize, Clone)]
struct MenuApp { #[serde(flatten)] app: TargetInfo, ai: bool, save: bool }

#[tauri::command]
fn list_menu_apps() -> Vec<MenuApp> {
    let mut out: Vec<MenuApp> = Vec::new();
    for app in platform::installed_ai_apps().into_iter().chain(platform::list_targets()) {
        if platform::is_helper(&app) { continue; }
        if let Some(m) = out.iter_mut().find(|m| m.app.key == app.key) {
            if m.app.id.is_empty() { m.app.id = app.id; }
            continue;
        }
        let (ai, save) = (platform::follows_by_default(&app), platform::supports_save(&app));
        out.push(MenuApp { app, ai, save });
    }
    out
}

/// Opens (if needed) an installed AI app and returns it ready to dock. Async: launching can take seconds.
#[tauri::command]
async fn open_ai_app(key: String) -> Result<TargetInfo, String> {
    tauri::async_runtime::spawn_blocking(move || platform::launch_app(&key))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "没能打开这个应用".to_string())
}

#[tauri::command]
fn set_target(app: AppHandle, id: String, key: String) -> Result<(), String> {
    platform::dock(app, &id)?;
    platform::set_target_key(&key);
    Ok(())
}

#[tauri::command]
fn clear_target(app: AppHandle) {
    platform::undock(&app);
}

#[tauri::command]
fn paste_note(app: AppHandle, text: String, image: Option<String>) {
    std::thread::spawn(move || actions::paste(&app, text, image));
}

/// Clicked from the chip: the click moved focus to Wings, so hand it back before cutting.
#[tauri::command]
fn capture_now(app: AppHandle) {
    if platform::target_input_unsupported() {
        return actions::emit_error(&app, "此应用暂不支持输入框操作");
    }
    std::thread::spawn(move || {
        if !platform::activate_target() { return actions::emit_error(&app, "没能切换到目标窗口"); }
        // Like paste and clear: without the caret in the input, select-all would grab the whole page.
        if !platform::focus_input() {
            return actions::emit_error(&app, "没能定位聊天输入框，请先点一下输入框再试");
        }
        actions::capture(&app);
    });
}

#[tauri::command]
fn clear_input(app: AppHandle) {
    std::thread::spawn(move || actions::clear(&app));
}

/// The tray menu's items, kept so the UI can relabel them (docked app, language changes).
struct TrayItems { target: tauri::menu::MenuItem<tauri::Wry>, show: tauri::menu::MenuItem<tauri::Wry>, quit: tauri::menu::MenuItem<tauri::Wry> }

/// Labels come from the UI, in its language. `docked` enables the first line, which brings both windows forward.
#[tauri::command]
fn set_tray_labels(app: AppHandle, target: String, docked: bool, show: String, quit: String) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.target.set_text(target);
        let _ = items.target.set_enabled(docked);
        let _ = items.show.set_text(show);
        let _ = items.quit.set_text(quit);
    }
}

/// Tray (menu bar on macOS) icon: the W on its white tile. Left click brings the bar back;
/// the right-click menu shows the docked app and is the way to quit.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let target = MenuItem::with_id(app, "target", "Wings", false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "显示 Wings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Wings", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&target, &PredefinedMenuItem::separator(app)?, &show, &quit])?;
    TrayIconBuilder::with_id("wings")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .tooltip("Wings")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "quit" => app.exit(0),
            "show" => show_bar(app),
            "target" => { show_bar(app); platform::activate_target(); }
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_bar(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(TrayItems { target, show, quit });
    Ok(())
}

fn show_bar(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn set_prefs(app: AppHandle, float: bool, follow: bool, follow_keys: Option<Vec<String>>, chip: bool, float_rect: Option<[f64; 4]>) {
    watch::FOLLOW.store(follow, Ordering::SeqCst);
    *watch::FOLLOW_KEYS.lock().unwrap() = follow_keys;
    watch::CHIP.store(chip, Ordering::SeqCst);
    // Prefs are re-sent on every change (opacity drags too); only an actual mode switch reshapes the window.
    static FLOATING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if FLOATING.swap(float, Ordering::SeqCst) != float { platform::shape_for_mode(&app, float, float_rect); }
    platform::set_float(&app, float);
}

/// Cursor position inside the window, in CSS pixels. macOS sends no mouse-move events to a window of an
/// inactive app, so the UI polls this to keep hover effects working while the target has focus.
#[tauri::command]
fn cursor_in_window(window: tauri::WebviewWindow) -> Option<(f64, f64)> {
    let c = window.cursor_position().ok()?;
    let p = window.inner_position().ok()?;
    let s = window.scale_factor().ok()?;
    Some(((c.x - p.x as f64) / s, (c.y - p.y as f64) / s))
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
    label_for(&CURRENT_SHORTCUT.lock().unwrap())
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
            // Docked beside other apps, Wings has no window chrome or Dock icon: the tray is the one way to quit.
            build_tray(app)?;
            register_shortcut(app.handle(), SHORTCUT)?;
            *CURRENT_SHORTCUT.lock().unwrap() = SHORTCUT.to_string();
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
                .accept_first_mouse(true)
                // Wings is never the active app while the user types elsewhere; without this, macOS 14+
                // throttles the page and animations stutter.
                .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
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
            list_targets, list_menu_apps, set_tray_labels, open_ai_app, set_target, clear_target, paste_note, capture_now, clear_input, set_prefs, cursor_in_window,
            permission_granted, open_permission_settings, shortcut_label, set_shortcut,
            update::check_update, update::download_update, update::open_url,
            store::load_notes, store::add_note, store::remove_note, store::image_data_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wings");
}
