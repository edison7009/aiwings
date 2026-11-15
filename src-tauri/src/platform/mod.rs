// Platform layer: target discovery, docking, focus control and synthetic keys.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

use std::sync::Mutex;
use tauri::{AppHandle, PhysicalPosition, PhysicalSize};

#[derive(Clone, Copy)]
/// Sent with Cmd/Ctrl: select all, cut, paste.
pub enum Key { A, X, V }

// These home screens wrap the composer in a larger project/header surface.
// Stop at the first padded ancestor that also contains the toolbar below the editor.
pub fn target_prefers_inner_box() -> bool {
    let key = TARGET_KEY.lock().unwrap();
    key.contains("zcode") || key.contains("opencode") || key.contains("antigravity")
        || key.contains("kilo") || apps::is_qoder(&key)
}

pub fn input_box_has_toolbar(text: Frame, candidate: Frame, scale: f64, min_padding: f64) -> bool {
    candidate.w >= text.w + min_padding * scale
        && candidate.y + candidate.h >= text.y + text.h + 24.0 * scale
}

pub const BAR_WIDTH: f64 = 280.0;

/// Target window frame in physical pixels, top-left origin.
#[derive(Clone, Copy, Debug)]
pub struct Frame { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }

/// Left edge of the bar: just outside the target's right edge, or None when that runs off the screen
/// (a maximized window, or an app widening to open its built-in browser): the bar then collapses.
/// Expanded by hand anyway (`wide`), it goes inside the target's right edge while the target fits the
/// screen, and otherwise follows it out rather than cover the app's content.
fn bar_x(f: Frame, bar_w: f64, screen_right: Option<f64>, scale: f64, wide: bool) -> Option<f64> {
    // Tuck 1pt under the target's edge so no sliver of desktop shows between the two windows.
    let outside = f.x + f.w - scale;
    match screen_right {
        Some(right) if outside + bar_w > right => {
            if !wide { return None; }
            Some(if f.x + f.w <= right + 2.0 * scale { f.x + f.w - bar_w } else { outside })
        }
        _ => Some(outside),
    }
}

/// Places a bar beside its target's right edge (see `bar_x`). False when there is no room and the bar
/// should be hidden.
pub fn place_bar(win: &tauri::WebviewWindow, app: &AppHandle, f: Frame, scale: f64) -> bool {
    let bar_w = BAR_WIDTH * scale;
    let right = app.monitor_from_point(f.x + f.w / 2.0, f.y + f.h / 2.0).ok().flatten()
        .map(|m| (m.position().x as f64) + m.size().width as f64);
    let Some(x) = bar_x(f, bar_w, right, scale, crate::bars::shows_wide(win.label())) else { return false };
    let _ = win.set_size(PhysicalSize::new(bar_w.round() as u32, f.h.max(200.0).round() as u32));
    let _ = win.set_position(PhysicalPosition::new(x.round() as i32, f.y.round() as i32));
    true
}

mod apps;

fn ai_rank(t: &crate::TargetInfo) -> usize { apps::rank(&t.name, &t.key) }

/// Whether an app is one of the AI apps Wings docks to.
pub fn is_ai_app(t: &crate::TargetInfo) -> bool { apps::is_ai(&t.name, &t.key) }

/// AI apps always get a bar; terminals (docking and paste only) when the user turned them on.
pub fn follows(t: &crate::TargetInfo, terminals: bool) -> bool {
    is_ai_app(t) || (terminals && is_terminal(&t.key))
}

pub fn target_is_ide() -> bool { apps::is_ide("", &TARGET_KEY.lock().unwrap()) }

/// Known AI desktop apps float to the top of the picker; the rest stay alphabetical.
pub fn sort_targets(list: &mut [crate::TargetInfo]) {
    list.sort_by(|a, b| ai_rank(a).cmp(&ai_rank(b)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

/// Each app's chip window: its pill's measured width (logical px), reported by its page so the window covers
/// no more of the input box than the buttons do (CHIP_W until measured), and where it was last put.
static CHIP_WIDTHS: Mutex<Vec<(String, f64)>> = Mutex::new(Vec::new());
static PLACED: Mutex<Vec<(String, (i32, i32, u32, u32))>> = Mutex::new(Vec::new());

pub fn chip_width(label: &str) -> f64 {
    CHIP_WIDTHS.lock().unwrap().iter().find(|(l, _)| l == label).map_or(CHIP_W, |(_, w)| *w)
}

pub fn set_chip_width(label: &str, width: f64) {
    let mut all = CHIP_WIDTHS.lock().unwrap();
    all.retain(|(l, _)| l != label);
    all.push((label.to_string(), width));
}

/// Forgets a closed app's chip.
pub fn forget_chip(label: &str) {
    CHIP_WIDTHS.lock().unwrap().retain(|(l, _)| l != label);
    PLACED.lock().unwrap().retain(|(l, _)| l != label);
    forget_chip_layer(label);
}

/// Places a chip astride the input box's top border, left of the box's mic and send/stop buttons,
/// reserving two icon widths. Only for the app in use (the inset depends on it).
/// Returns true when the chip window was hidden and has just been shown.
/// Only touches the window when its spot changes or it was hidden: re-setting the frame and re-ordering
/// it every tick makes the button stutter.
pub fn place_chip(chip: &tauri::WebviewWindow, input: Frame, scale: f64) -> bool {
    let (w, h) = (chip_width(chip.label()) * scale, CHIP_H * scale);
    // The button's centre is the window's vertical centre, so half the window above the line centres it on the border.
    // Preserve Grok Bot's verified 96-point inset independently of the shared default.
    let key = TARGET_KEY.lock().unwrap();
    let inset = if key.contains("grok bot") || key.contains("grokbot") || key.contains("grok-bot") {
        96.0
    } else { CHIP_RIGHT_INSET };
    drop(key);
    let x = input.x + input.w - w - inset * scale;
    let spot = (x.round() as i32, (input.y - h / 2.0).round() as i32, w.round() as u32, h.round() as u32);
    put_chip(chip, spot)
}

/// Moves a chip by a distance in physical pixels: its app's window moved while another app was in use.
pub fn shift_chip(chip: &tauri::WebviewWindow, dx: f64, dy: f64) -> bool {
    let placed = PLACED.lock().unwrap().iter().find(|(l, _)| l == chip.label()).map(|(_, p)| *p);
    let Some(p) = placed else { return false };
    put_chip(chip, ((p.0 as f64 + dx).round() as i32, (p.1 as f64 + dy).round() as i32, p.2, p.3))
}

fn put_chip(chip: &tauri::WebviewWindow, spot: (i32, i32, u32, u32)) -> bool {
    let mut all = PLACED.lock().unwrap();
    let placed = all.iter().find(|(l, _)| l == chip.label()).map(|(_, p)| *p);
    if placed.is_none_or(|p| (p.2, p.3) != (spot.2, spot.3)) {
        let _ = chip.set_size(PhysicalSize::new(spot.2, spot.3));
    }
    if placed.is_none_or(|p| (p.0, p.1) != (spot.0, spot.1)) {
        let _ = chip.set_position(PhysicalPosition::new(spot.0, spot.1));
    }
    let was_hidden = !chip.is_visible().unwrap_or(false);
    if was_hidden { show_no_activate(chip); }
    all.retain(|(l, _)| l != chip.label());
    all.push((chip.label().to_string(), spot));
    was_hidden
}

pub const CHIP_W: f64 = 140.0;
pub const CHIP_H: f64 = 34.0;
/// Room kept clear for the box's own buttons at its right end.
pub const CHIP_RIGHT_INSET: f64 = 64.0;

/// Terminals: Wings can follow them and paste into them, but cannot save from them. Select-all grabs the
/// whole scrollback and cut does nothing, because the line being typed belongs to the program running inside.
const TERMINALS: &[&str] = &[
    "com.apple.terminal", "com.googlecode.iterm2", "dev.warp.warp", "com.mitchellh.ghostty", "com.github.wez.wezterm",
    "org.alacritty", "net.kovidgoyal.kitty", "co.zeit.hyper", "org.tabby",
    "windowsterminal.exe", "openconsole.exe", "cmd.exe", "powershell.exe", "pwsh.exe", "wezterm-gui.exe", "alacritty.exe", "conhost.exe",
];

static TARGET_KEY: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn set_target_key(key: &str) { *TARGET_KEY.lock().unwrap() = key.to_lowercase(); }

/// The active app's key (the one saves and pastes go to), lowercased.
pub fn target_key() -> String { TARGET_KEY.lock().unwrap().clone() }

pub fn is_terminal(key: &str) -> bool {
    let key = key.to_lowercase();
    TERMINALS.iter().any(|t| key.starts_with(t))
}

pub fn target_is_terminal() -> bool { is_terminal(&TARGET_KEY.lock().unwrap()) }

// Confirmed unsupported Windows inputs; do not keep probing them.
fn input_unsupported(key: &str) -> bool {
    cfg!(windows) && matches!(key.to_lowercase().as_str(), "doubao.exe" | "doubaowork.exe" | "trae cn.exe")
}

pub fn target_input_unsupported() -> bool { input_unsupported(&TARGET_KEY.lock().unwrap()) }

/// Apps eligible for saving; the focused input must also pass runtime accessibility checks.
#[cfg(test)]
fn supports_save(t: &crate::TargetInfo) -> bool { is_ai_app(t) && !is_terminal(&t.key) && !input_unsupported(&t.key) }

/// Helper bundles that ship inside or beside an app (URL handlers, updaters) are not apps to pick.
pub fn is_helper(t: &crate::TargetInfo) -> bool {
    let n = t.name.to_lowercase();
    ["handler", "helper", "updater", "uninstall", "crash"].iter().any(|w| n.contains(w))
}

#[cfg(test)]
mod input_box_tests {
    use super::*;

    #[test]
    fn macos_terminals_follow_only_when_on_and_never_save() {
        for key in ["com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable", "com.mitchellh.ghostty"] {
            let terminal = crate::TargetInfo { id: "123".into(), key: key.into(), name: "Terminal".into() };
            assert!(follows(&terminal, true) && !follows(&terminal, false), "{key}");
            assert!(!supports_save(&terminal), "{key}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_terminals_follow_without_save() {
        for key in ["WindowsTerminal.exe", "powershell.exe", "pwsh.exe", "cmd.exe", "OpenConsole.exe", "conhost.exe"] {
            let terminal = crate::TargetInfo { id: "123".into(), key: key.into(), name: "PowerShell".into() };
            assert!(is_terminal(key), "{key}");
            assert!(follows(&terminal, true) && !follows(&terminal, false), "{key}");
            assert!(!supports_save(&terminal), "{key}");
        }
        let document = crate::TargetInfo { id: "456".into(), key: "notepad.exe".into(), name: "PowerShell notes".into() };
        assert!(!follows(&document, true));
    }

    #[test]
    fn bar_collapses_without_room_unless_expanded_by_hand() {
        let f = |x: f64, w: f64| Frame { x, y: 0.0, w, h: 800.0 };
        // Room on the right: outside, tucked 1pt under the edge.
        assert_eq!(bar_x(f(100.0, 1000.0), 280.0, Some(1920.0), 1.0, false), Some(1099.0));
        // Maximized, or widened past the screen: no room, collapsed.
        assert_eq!(bar_x(f(0.0, 1920.0), 280.0, Some(1920.0), 1.0, false), None);
        assert_eq!(bar_x(f(600.0, 1800.0), 280.0, Some(1920.0), 1.0, false), None);
        // Expanded by hand: inside a maximized window; an overflowing one is followed out, covering nothing.
        assert_eq!(bar_x(f(0.0, 1920.0), 280.0, Some(1920.0), 1.0, true), Some(1640.0));
        assert_eq!(bar_x(f(600.0, 1800.0), 280.0, Some(1920.0), 1.0, true), Some(2399.0));
        // Maximized on a second monitor to the right, scaled: collapsed, or inside when expanded by hand.
        assert_eq!(bar_x(f(1920.0, 2880.0), 560.0, Some(4800.0), 2.0, false), None);
        assert_eq!(bar_x(f(1920.0, 2880.0), 560.0, Some(4800.0), 2.0, true), Some(4240.0));
        assert_eq!(bar_x(f(1920.0, 1000.0), 560.0, None, 2.0, false), Some(2918.0));
    }

    #[test]
    fn inner_composer_requires_padding_and_toolbar() {
        let text = Frame { x: 30.0, y: 200.0, w: 640.0, h: 40.0 };
        assert!(!input_box_has_toolbar(text, text, 1.0, 8.0));
        let padded_text = Frame { x: 18.0, y: 188.0, w: 664.0, h: 64.0 };
        assert!(!input_box_has_toolbar(text, padded_text, 1.0, 8.0));
        let composer = Frame { h: 106.0, ..padded_text };
        assert!(input_box_has_toolbar(text, composer, 1.0, 8.0));
        // Qoder's text control fills the box except for its 1px border on each side.
        let qoder = Frame { x: 29.0, y: 199.0, w: 642.0, h: 120.0 };
        assert!(input_box_has_toolbar(text, qoder, 1.0, 2.0));
        assert!(!input_box_has_toolbar(text, qoder, 1.0, 8.0));
        let scale = |f: Frame| Frame { x: f.x * 1.5, y: f.y * 1.5, w: f.w * 1.5, h: f.h * 1.5 };
        assert!(input_box_has_toolbar(scale(text), scale(composer), 1.5, 8.0));
        assert!(!input_box_has_toolbar(scale(text), scale(padded_text), 1.5, 8.0));
    }
}
