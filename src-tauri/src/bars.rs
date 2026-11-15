// Docked bars: one window beside each open AI app (and terminal, when the user turns terminals on).
// Each bar stays put beside its own app instead of one bar jumping between apps.
// The "main" window only shows the permission steps before Wings may dock. Bars can be collapsed all at
// once (the × on a bar, the arrow on the chip); a target too wide to leave room beside it collapses its
// own bar until it narrows again.

use crate::{platform, TargetInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Terminals get bars too (paste only: Wings cannot save from them).
pub static TERMINALS: AtomicBool = AtomicBool::new(false);
/// Collapsed: every bar stays hidden until the chip's arrow or the tray brings them back. Remembered across launches.
pub static HIDDEN: AtomicBool = AtomicBool::new(false);
/// Apps whose bar the user expanded while the app was too wide for one beside it: their bar shows over the
/// app's right edge instead of collapsing. Cleared on the next collapse.
static SHOW_WIDE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Whether the bar window `label` was expanded by hand beside a too-wide app.
pub fn shows_wide(label: &str) -> bool {
    label.strip_prefix("bar-").is_some_and(|id| SHOW_WIDE.lock().unwrap().iter().any(|w| w == id))
}

/// The app each bar sits beside. The bar's window label is `label(&target.id)`.
static BARS: Mutex<Vec<TargetInfo>> = Mutex::new(Vec::new());

pub fn label(id: &str) -> String { format!("bar-{id}") }
/// The window of the chip on that app's input box.
pub fn chip_label(id: &str) -> String { format!("chip-{id}") }

/// The app a bar or chip window belongs to.
pub fn target_of(label: &str) -> Option<TargetInfo> {
    BARS.lock().unwrap().iter().find(|t| self::label(&t.id) == label || chip_label(&t.id) == label).cloned()
}

/// Every followed app.
pub fn targets() -> Vec<TargetInfo> { BARS.lock().unwrap().clone() }

pub fn wants(t: &TargetInfo) -> bool {
    !platform::is_helper(t) && platform::follows(t, TERMINALS.load(Ordering::SeqCst))
}

/// One target per app (its main window). A window already followed keeps its bar while it is still open,
/// so a second window of the same app coming forward does not move the bar.
fn choose(found: Vec<TargetInfo>, current: &[TargetInfo]) -> Vec<TargetInfo> {
    let mut out: Vec<TargetInfo> = Vec::new();
    for t in &found {
        if out.iter().any(|o| o.key == t.key) { continue; }
        let kept = current.iter().find(|c| c.key == t.key && found.iter().any(|f| f.id == c.id));
        out.push(kept.cloned().unwrap_or_else(|| t.clone()));
    }
    out
}

/// Opens a bar for each newly found app and closes the bars of apps that are gone.
pub fn sync(app: &AppHandle) {
    let found: Vec<TargetInfo> = platform::list_targets().into_iter().filter(wants).collect();
    let (added, removed) = {
        let mut bars = BARS.lock().unwrap();
        let next = choose(found, &bars);
        let added: Vec<TargetInfo> = next.iter().filter(|n| !bars.iter().any(|b| b.id == n.id)).cloned().collect();
        let removed: Vec<TargetInfo> = bars.iter().filter(|b| !next.iter().any(|n| n.id == b.id)).cloned().collect();
        *bars = next;
        (added, removed)
    };
    for t in removed {
        platform::untrack(&t.id);
        if let Some(w) = app.get_webview_window(&label(&t.id)) { let _ = w.destroy(); }
        let chip = chip_label(&t.id);
        if let Some(w) = app.get_webview_window(&chip) { let _ = w.destroy(); }
        platform::forget_chip(&chip);
        crate::watch::forget_chip(&chip);
        SHOW_WIDE.lock().unwrap().retain(|w| w != &t.id);
    }
    for t in added {
        let label = label(&t.id);
        let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
            .title("Wings")
            .inner_size(platform::BAR_WIDTH, 520.0)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .skip_taskbar(true)
            .focused(false)
            .accept_first_mouse(true)
            // Wings is never the active app while the user types elsewhere; without this the page is throttled.
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
            .visible(false)
            .build();
        match built {
            Ok(_) => { platform::track(app, &t, &label); build_chip(app, &t.id); }
            Err(_) => BARS.lock().unwrap().retain(|b| b.id != t.id),
        }
    }
}

/// The app's chip: hidden until its input box has been used, then it stays on it.
fn build_chip(app: &AppHandle, id: &str) {
    let _ = WebviewWindowBuilder::new(app, chip_label(id), WebviewUrl::App("index.html".into()))
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
        .build();
}

fn labels() -> Vec<String> { BARS.lock().unwrap().iter().map(|t| label(&t.id)).collect() }

/// The main window shows the permission steps, and only until permission is granted.
pub fn update_main(app: &AppHandle, trusted: bool) {
    let Some(main) = app.get_webview_window("main") else { return };
    let shown = main.is_visible().unwrap_or(false);
    if !trusted && !shown { platform::show_no_activate(&main); }
    if trusted && shown { platform::hide(&main); }
}

/// Where an error from a save or paste is shown: the bar of the app it happened in, if it is up;
/// otherwise the main window (seen only before permission is granted).
pub fn error_window(app: &AppHandle) -> String {
    let active = platform::active_id();
    let bar = label(&active);
    if bar_up(app, &active) { bar } else { "main".into() }
}

/// Whether the bar beside `id` is on screen.
pub fn bar_up(app: &AppHandle, id: &str) -> bool {
    BARS.lock().unwrap().iter().any(|t| t.id == id)
        && app.get_webview_window(&label(id)).is_some_and(|w| w.is_visible().unwrap_or(false))
}

fn collapsed_flag(app: &AppHandle) -> Option<std::path::PathBuf> { Some(app.path().app_data_dir().ok()?.join("bars-collapsed")) }

/// Restores the collapsed state of the last session.
pub fn load_collapsed(app: &AppHandle) {
    HIDDEN.store(collapsed_flag(app).is_some_and(|p| p.exists()), Ordering::SeqCst);
}

/// Collapses (`true`) or expands every bar, and remembers it.
pub fn set_collapsed(app: &AppHandle, collapsed: bool) {
    HIDDEN.store(collapsed, Ordering::SeqCst);
    if collapsed { SHOW_WIDE.lock().unwrap().clear(); }
    if let Some(path) = collapsed_flag(app) {
        if collapsed {
            if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
            let _ = std::fs::write(&path, b"");
        } else { let _ = std::fs::remove_file(&path); }
    }
    if collapsed { for l in labels() { if let Some(w) = app.get_webview_window(&l) { platform::hide(&w); } } }
    platform::refresh_later(app);
}

/// A chip's arrow: collapses the bars when its app's bar is up, otherwise brings them back,
/// even beside a target too wide for one.
pub fn toggle(app: &AppHandle, id: &str) {
    if bar_up(app, id) { set_collapsed(app, true); return; }
    SHOW_WIDE.lock().unwrap().push(id.to_string());
    set_collapsed(app, false);
}

/// From the tray or a second launch: expand the bars (or show the permission steps).
pub fn show(app: &AppHandle) {
    set_collapsed(app, false);
    if !platform::accessibility_trusted(false) {
        if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(id: &str, key: &str) -> TargetInfo { TargetInfo { id: id.into(), key: key.into(), name: key.into() } }

    #[test]
    fn one_bar_per_app_and_the_followed_window_keeps_it() {
        let found = vec![t("2", "claude.exe"), t("1", "claude.exe"), t("3", "codex.exe")];
        let fresh = choose(found.clone(), &[]);
        assert_eq!(fresh.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["2", "3"]);
        let kept = choose(found.clone(), &[t("1", "claude.exe")]);
        assert_eq!(kept.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["1", "3"]);
        // The followed window closed: the app's other window takes over.
        let moved = choose(vec![t("2", "claude.exe")], &[t("1", "claude.exe")]);
        assert_eq!(moved[0].id, "2");
    }
}
