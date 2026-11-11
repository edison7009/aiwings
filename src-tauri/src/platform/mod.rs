// Platform layer: target discovery, docking, focus control and synthetic keys.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize};

#[derive(Clone, Copy)]
/// A, X, V go with Cmd/Ctrl; Delete (Backspace) is sent bare.
pub enum Key { A, X, V, Delete }

pub const BAR_WIDTH: f64 = 280.0;

/// Target window frame in physical pixels, top-left origin.
#[derive(Clone, Copy, Debug)]
pub struct Frame { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }

/// Places the bar just outside the target's right edge; if that runs off the screen, inside it.
pub fn place_bar(app: &AppHandle, f: Frame, scale: f64) {
    let Some(win) = app.get_webview_window("main") else { return };
    let bar_w = BAR_WIDTH * scale;
    let mut x = f.x + f.w;
    if let Ok(Some(m)) = app.monitor_from_point(f.x + f.w / 2.0, f.y + f.h / 2.0) {
        let right = (m.position().x as f64) + m.size().width as f64;
        if x + bar_w > right { x = f.x + f.w - bar_w; }
    }
    let _ = win.set_size(PhysicalSize::new(bar_w.round() as u32, f.h.max(200.0).round() as u32));
    let _ = win.set_position(PhysicalPosition::new(x.round() as i32, f.y.round() as i32));
}

/// Back to a free-floating panel for the picker.
pub fn float_panel(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_size(LogicalSize::new(BAR_WIDTH, 520.0));
        let _ = win.center();
        let _ = win.show();
    }
}

const AI_APPS: &[&str] = &["claude", "chatgpt", "codex", "grok", "gemini", "deepseek", "zcode", "kimi", "doubao", "豆包", "perplexity", "copilot", "元宝", "通义"];

fn ai_rank(t: &crate::TargetInfo) -> usize {
    let n = format!("{} {}", t.name, t.key).to_lowercase();
    AI_APPS.iter().position(|k| n.contains(k)).unwrap_or(AI_APPS.len())
}

/// Whether an app is one of the AI apps Wings follows when "follow" is on.
pub fn is_ai_app(t: &crate::TargetInfo) -> bool { ai_rank(t) < AI_APPS.len() }

/// Known AI desktop apps float to the top of the picker; the rest stay alphabetical.
pub fn sort_targets(list: &mut [crate::TargetInfo]) {
    list.sort_by(|a, b| ai_rank(a).cmp(&ai_rank(b)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

/// Places the "save to Wings" chip astride the input box's top border, left of the box's
/// right-hand buttons (send/stop, mic) so it never sits right above them.
pub fn place_chip(app: &AppHandle, input: Frame, scale: f64) {
    let Some(chip) = app.get_webview_window("chip") else { return };
    let (w, h) = (CHIP_W * scale, CHIP_H * scale);
    let _ = chip.set_size(PhysicalSize::new(w.round() as u32, h.round() as u32));
    // The button's centre is the window's vertical centre, so half the window above the line centres it on the border.
    let x = input.x + input.w - w - CHIP_RIGHT_INSET * scale;
    let _ = chip.set_position(PhysicalPosition::new(x.round() as i32, (input.y - h / 2.0).round() as i32));
    show_no_activate(&chip);
}

pub const CHIP_W: f64 = 140.0;
pub const CHIP_H: f64 = 34.0;
/// Room kept clear for the box's own buttons at its right end.
pub const CHIP_RIGHT_INSET: f64 = 72.0;

/// Hover returns to where (and how big) it was last hovering, or on first use keeps the docked
/// width and trims the height. It is resizable by the edges; docked, the target sets the size again.
/// `last` is the remembered hover frame in physical pixels: x, y, width, height.
pub fn shape_for_mode(app: &AppHandle, float: bool, last: Option<[f64; 4]>) {
    let Some(win) = app.get_webview_window("main") else { return };
    let _ = win.set_resizable(float);
    if !float { return; }
    let _ = win.set_min_size(Some(LogicalSize::new(200.0, 240.0)));
    // Only if that spot is still on a screen (a monitor may have been unplugged since).
    if let Some([x, y, w, h]) = last {
        if let Ok(Some(_)) = app.monitor_from_point(x + w / 2.0, y + 20.0) {
            let _ = win.set_size(PhysicalSize::new(w.round() as u32, h.round() as u32));
            let _ = win.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
            return;
        }
    }
    let scale = win.scale_factor().unwrap_or(1.0);
    if let Ok(size) = win.inner_size() {
        let h = (size.height as f64).min(FLOAT_HEIGHT * scale);
        let _ = win.set_size(PhysicalSize::new(size.width, h.round() as u32));
    }
}

/// Default hover height; shorter than a docked bar, which matches the whole target window.
pub const FLOAT_HEIGHT: f64 = 460.0;
