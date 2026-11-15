mod actions;
mod bars;
mod clip;
mod platform;
mod store;
mod update;
mod voice;
mod watch;

use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
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

/// The app a bar window sits beside; None for the hover panel. `terminal`: paste only, nothing is saved from it.
#[derive(Serialize, Clone)]
struct BarTarget { #[serde(flatten)] app: TargetInfo, terminal: bool }

#[tauri::command]
fn bar_target(window: tauri::WebviewWindow) -> Option<BarTarget> {
    bars::target_of(window.label()).map(|app| BarTarget { terminal: platform::is_terminal(&app.key), app })
}

/// A bar pastes into its own app; the hover panel into the app used last.
#[tauri::command]
fn paste_note(app: AppHandle, window: tauri::WebviewWindow, text: String, image: Option<String>) {
    if let Some(t) = bars::target_of(window.label()) { platform::set_active(&t); }
    std::thread::spawn(move || actions::paste(&app, text, image));
}

/// Clicked from a chip: the click moved focus to Wings, so hand it back (to that chip's app) before cutting.
#[tauri::command]
fn capture_now(app: AppHandle, window: tauri::WebviewWindow) {
    if let Some(t) = bars::target_of(window.label()) { platform::set_active(&t); }
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

/// Puts a share link on the clipboard (when given) and shows `msg` where the user is looking:
/// the bar of the app in use, or the hover panel.
#[tauri::command]
fn share_link(app: AppHandle, link: Option<String>, msg: Option<String>) {
    if let Some(link) = link {
        let copied = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(link));
        if copied.is_err() { return actions::emit_error(&app, "无法写入剪贴板"); }
    }
    if let Some(msg) = msg {
        let _ = app.emit_to(bars::error_window(&app).as_str(), "wings://toast", msg);
    }
}

/// A chip was pressed (mouse down): its app becomes the one saves and pastes go to. Its buttons hand
/// focus back themselves; with `refocus`, give the caret back to that app's input here.
#[tauri::command]
fn chip_pressed(window: tauri::WebviewWindow, refocus: bool) {
    *watch::PRESSED_AT.lock().unwrap() = Some(std::time::Instant::now());
    if let Some(t) = bars::target_of(window.label()) { platform::set_active(&t); }
    if refocus {
        std::thread::spawn(|| { if platform::activate_target() { platform::focus_input(); } });
    }
}

/// A chip's current state, for a chip page that missed the last `wings://chip` event while loading.
#[tauri::command]
fn chip_state(window: tauri::WebviewWindow) -> Option<bool> {
    watch::CHIP_STATES.lock().unwrap().iter().find(|(l, _)| l == window.label()).and_then(|(_, s)| *s)
}

/// A chip page's pill width, so its window covers no more of the input box than the buttons do.
#[tauri::command]
fn set_chip_width(window: tauri::WebviewWindow, width: f64) {
    let width = width.clamp(60.0, 400.0);
    platform::set_chip_width(window.label(), width);
    // Resize a shown chip right away, keeping its right edge in place: while recording the watcher holds
    // it still and would not.
    if !window.is_visible().unwrap_or(false) { return; }
    let (Ok(pos), Ok(size), Ok(scale)) = (window.outer_position(), window.outer_size(), window.scale_factor()) else { return };
    let w = (width * scale).round() as i32;
    let _ = window.set_size(tauri::PhysicalSize::new(w as u32, size.height));
    let _ = window.set_position(tauri::PhysicalPosition::new(pos.x + size.width as i32 - w, pos.y));
}

/// The tray menu's items, kept so the UI can relabel them on a language change.
struct TrayItems { show: tauri::menu::MenuItem<tauri::Wry>, quit: tauri::menu::MenuItem<tauri::Wry> }

#[tauri::command]
fn set_tray_labels(app: AppHandle, show: String, quit: String) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.show.set_text(show);
        let _ = items.quit.set_text(quit);
    }
}

/// Tray (menu bar on macOS) icon: the W on its white tile. Left click brings Wings into view;
/// the right-click menu is the way to quit.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let show = MenuItem::with_id(app, "show", "显示 Wings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Wings", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::with_id("wings")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .tooltip("Wings")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "quit" => app.exit(0),
            "show" => bars::show(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                bars::show(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(TrayItems { show, quit });
    Ok(())
}

/// `terminals`: terminals get bars too. `chip`: show the chip on the target's input box.
#[tauri::command]
fn set_prefs(terminals: bool, chip: bool) {
    watch::CHIP.store(chip, Ordering::SeqCst);
    bars::TERMINALS.store(terminals, Ordering::SeqCst);
}

/// The × on a bar: collapses every bar (Wings keeps running; the chip's arrow or the tray brings them back).
#[tauri::command]
fn hide_wings(app: AppHandle) { bars::set_collapsed(&app, true); }

/// A chip's arrow: collapse or expand the bars.
#[tauri::command]
fn toggle_bars(app: AppHandle, window: tauri::WebviewWindow) {
    if let Some(t) = bars::target_of(window.label()) { bars::toggle(&app, &t.id); }
}

/// Whether a chip's app has its bar on screen, for a chip page that missed the last `wings://bar` event.
#[tauri::command]
fn bar_state(app: AppHandle, window: tauri::WebviewWindow) -> bool {
    bars::target_of(window.label()).is_some_and(|t| bars::bar_up(&app, &t.id))
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
            bars::show(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // Docked beside other apps, Wings has no window chrome or Dock icon: the tray is the one way to quit.
            build_tray(app)?;
            bars::load_collapsed(app.handle());
            register_shortcut(app.handle(), SHORTCUT)?;
            *CURRENT_SHORTCUT.lock().unwrap() = SHORTCUT.to_string();
            watch::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|win, ev| {
            // Clicking a bar can lift it above other apps' windows: put the bars back once Wings loses focus.
            if let WindowEvent::Focused(false) = ev { platform::refresh_later(win.app_handle()); }
        })
        .invoke_handler(tauri::generate_handler![
            bar_target, hide_wings, set_tray_labels, paste_note, capture_now, share_link, set_chip_width, chip_state, chip_pressed, set_prefs, toggle_bars, bar_state, cursor_in_window,
            permission_granted, open_permission_settings, shortcut_label, set_shortcut,
            update::check_update, update::download_update, update::open_url,
            store::load_notes, store::add_note, store::remove_note, store::image_data_url,
            voice::voice_sources, voice::voice_start, voice::voice_finish, voice::voice_cancel, voice::voice_cancel_all, voice::voice_devices,
            voice::voice_mic_test_start, voice::voice_mic_test_stop, voice::voice_api_get, voice::voice_api_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wings");
}
