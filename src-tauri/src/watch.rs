// Background watcher: follows the frontmost AI app (when enabled) and drives the "save to Wings" chip.

use crate::platform::{self, place_chip};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub static FOLLOW: AtomicBool = AtomicBool::new(true);
/// Apps (by key) the user ticked to follow; None means the default, every known AI app.
pub static FOLLOW_KEYS: std::sync::Mutex<Option<Vec<String>>> = std::sync::Mutex::new(None);

fn wants_follow(app: &crate::TargetInfo) -> bool {
    match FOLLOW_KEYS.lock().unwrap().as_ref() {
        Some(keys) => keys.iter().any(|k| *k == app.key),
        None => platform::is_ai_app(app),
    }
}
pub static CHIP: AtomicBool = AtomicBool::new(true);

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_chip: Option<bool> = None;
        // When the input goes away, the chip first flips down and fades; the window hides once that is over.
        let mut hide_at: Option<std::time::Instant> = None;
        let mut last_follow = String::new();
        loop {
            std::thread::sleep(Duration::from_millis(300));
            // No target yet is fine: following is also how a first launch finds its AI app on its own.
            if FOLLOW.load(Ordering::SeqCst) {
                if let Some(front) = platform::front_app() {
                    // Ask the UI to re-dock once per newly focused AI app; docking itself runs on the main thread.
                    if wants_follow(&front) && front.id != platform::current_id() && front.id != last_follow {
                        last_follow = front.id.clone();
                        let _ = app.emit("wings://follow", front);
                    }
                }
            }
            // Always track the focused input (paste uses it to restore the caret); the chip only shows when enabled.
            let input = platform::target_is_front().then(platform::focused_input).flatten().filter(|_| CHIP.load(Ordering::SeqCst) && !platform::target_is_terminal());
            let Some(chip) = app.get_webview_window("chip") else { continue };
            match input {
                Some((frame, has_text)) => {
                    hide_at = None;
                    let scale = chip.scale_factor().unwrap_or(1.0);
                    let s = if cfg!(target_os = "macos") { scale } else { 1.0 };
                    let just_shown = place_chip(&app, platform::Frame { x: frame.x * s, y: frame.y * s, w: frame.w * s, h: frame.h * s }, scale);
                    // A hidden window's page is paused: let it paint once before the flip starts, or the
                    // first frames are dropped and the button looks cut off for a moment.
                    if just_shown { continue; }
                    if last_chip != Some(has_text) {
                        last_chip = Some(has_text);
                        let _ = chip.emit("wings://chip", has_text);
                    }
                }
                None => {
                    if last_chip.is_some() {
                        last_chip = None;
                        let _ = chip.emit("wings://chip", false);
                        hide_at = Some(std::time::Instant::now() + Duration::from_millis(500));
                    }
                    if hide_at.is_some_and(|t| std::time::Instant::now() >= t) {
                        hide_at = None;
                        let _ = chip.hide();
                    }
                }
            }
        }
    });
}
