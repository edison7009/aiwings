// Docked bars: one window beside each open AI app (and terminal, when the user turns terminals on).
// Each bar stays put beside its own app instead of one bar jumping between apps.
// The "main" window is the hover panel: used in hover mode, before permission is granted, and on its own
// in docked mode whenever no followed app has a window up ("smart hover").

use crate::{platform, TargetInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Hover mode: bars are hidden and the main panel floats.
pub static FLOAT: AtomicBool = AtomicBool::new(false);
/// Terminals get bars too (paste only: Wings cannot save from them).
pub static TERMINALS: AtomicBool = AtomicBool::new(false);
/// Put away with the close button: every bar and the hover panel stay hidden until the tray brings them back.
pub static HIDDEN: AtomicBool = AtomicBool::new(false);

/// The app each bar sits beside. The bar's window label is `label(&target.id)`.
static BARS: Mutex<Vec<TargetInfo>> = Mutex::new(Vec::new());

pub fn label(id: &str) -> String { format!("bar-{id}") }

/// The app a bar window belongs to.
pub fn target_of(label: &str) -> Option<TargetInfo> {
    BARS.lock().unwrap().iter().find(|t| self::label(&t.id) == label).cloned()
}

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
            Ok(_) => platform::track(app, &t, &label),
            Err(_) => BARS.lock().unwrap().retain(|b| b.id != t.id),
        }
    }
}

fn labels() -> Vec<String> { BARS.lock().unwrap().iter().map(|t| label(&t.id)).collect() }

fn any_bar_visible(app: &AppHandle) -> bool {
    labels().iter().any(|l| app.get_webview_window(l).is_some_and(|w| w.is_visible().unwrap_or(false)))
}

/// Shows the hover panel when hovering, before permission, or (docked) once no bar has been up for a second;
/// hides it as soon as a bar is up. The delay keeps it from flashing while windows open and close.
pub fn update_main(app: &AppHandle, trusted: bool) {
    static SINCE: Mutex<Option<Instant>> = Mutex::new(None);
    let Some(main) = app.get_webview_window("main") else { return };
    let docked_idle = !FLOAT.load(Ordering::SeqCst) && trusted && !any_bar_visible(app);
    let want = !HIDDEN.load(Ordering::SeqCst) && (!trusted || FLOAT.load(Ordering::SeqCst) || docked_idle);
    // Read the window itself: the close button and the tray also show and hide it.
    let shown = main.is_visible().unwrap_or(false);
    let mut since = SINCE.lock().unwrap();
    if want == shown { *since = None; return; }
    if want && docked_idle {
        let start = *since.get_or_insert_with(Instant::now);
        if start.elapsed() < Duration::from_secs(1) { return; }
    }
    *since = None;
    if want { platform::show_no_activate(&main); } else { let _ = main.hide(); }
}

/// Where an error from a save or paste is shown: the bar of the app it happened in, if it is up;
/// otherwise the hover panel.
pub fn error_window(app: &AppHandle) -> String {
    let active = platform::active_id();
    let bar = label(&active);
    let up = !FLOAT.load(Ordering::SeqCst)
        && BARS.lock().unwrap().iter().any(|t| t.id == active)
        && app.get_webview_window(&bar).is_some_and(|w| w.is_visible().unwrap_or(false));
    if up { bar } else { "main".into() }
}

/// The close button: puts every bar and the hover panel away. Wings keeps running in the tray.
pub fn hide_all(app: &AppHandle) {
    HIDDEN.store(true, Ordering::SeqCst);
    for l in labels() { if let Some(w) = app.get_webview_window(&l) { let _ = w.hide(); } }
    if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
}

/// Brings Wings into view from the tray: the hover panel when that is what is showing, else the bars.
pub fn show(app: &AppHandle) {
    let was_hidden = HIDDEN.swap(false, Ordering::SeqCst);
    // Docked: the bars come back beside their apps; with none on screen the panel follows within a second.
    if FLOAT.load(Ordering::SeqCst) || (!was_hidden && !any_bar_visible(app)) {
        if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
    }
    platform::refresh_later(app);
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
