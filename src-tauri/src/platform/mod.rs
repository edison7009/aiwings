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
pub enum Key { A, X, V }

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

/// Known AI desktop apps float to the top of the picker; the rest stay alphabetical.
pub fn sort_targets(list: &mut [crate::TargetInfo]) {
    const AI: &[&str] = &["claude", "chatgpt", "grok", "gemini", "deepseek", "kimi", "doubao", "豆包", "perplexity", "copilot", "元宝", "通义"];
    let rank = |t: &crate::TargetInfo| {
        let n = t.name.to_lowercase();
        AI.iter().position(|k| n.contains(k)).unwrap_or(AI.len())
    };
    list.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}
