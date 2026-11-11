// Background watcher: follows the frontmost AI app (when enabled) and drives the "save to Wings" chip.

use crate::platform::{self, place_chip};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub static FOLLOW: AtomicBool = AtomicBool::new(false);
pub static CHIP: AtomicBool = AtomicBool::new(true);

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_chip: Option<bool> = None;
        let mut last_follow = String::new();
        loop {
            std::thread::sleep(Duration::from_millis(300));
            if FOLLOW.load(Ordering::SeqCst) && platform::has_target() {
                if let Some(front) = platform::front_app() {
                    // Ask the UI to re-dock once per newly focused AI app; docking itself runs on the main thread.
                    if platform::is_ai_app(&front) && front.id != platform::current_id() && front.id != last_follow {
                        last_follow = front.id.clone();
                        let _ = app.emit("wings://follow", front);
                    }
                }
            }
            // Always track the focused input (paste uses it to restore the caret); the chip only shows when enabled.
            let input = platform::target_is_front().then(platform::focused_input).flatten().filter(|_| CHIP.load(Ordering::SeqCst));
            let Some(chip) = app.get_webview_window("chip") else { continue };
            match input {
                Some((frame, has_text)) => {
                    let scale = chip.scale_factor().unwrap_or(1.0);
                    let s = if cfg!(target_os = "macos") { scale } else { 1.0 };
                    place_chip(&app, platform::Frame { x: frame.x * s, y: frame.y * s, w: frame.w * s, h: frame.h * s }, scale);
                    if last_chip != Some(has_text) {
                        last_chip = Some(has_text);
                        let _ = chip.emit("wings://chip", has_text);
                    }
                }
                None => {
                    if last_chip.is_some() {
                        last_chip = None;
                        let _ = chip.emit("wings://chip", false);
                        let _ = chip.hide();
                    }
                }
            }
        }
    });
}
