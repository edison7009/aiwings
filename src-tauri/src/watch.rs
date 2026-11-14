// Background watcher: keeps one bar per open AI app, tracks the app in use, and drives the "save to Wings" chip.

use crate::platform::{self, place_chip};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub static CHIP: AtomicBool = AtomicBool::new(true);

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_chip: Option<bool> = None;
        // When the input goes away, the chip first flips down and fades; the window hides once that is over.
        let mut hide_at: Option<std::time::Instant> = None;
        let mut tick: u32 = 0;
        let mut trusted = false;
        loop {
            std::thread::sleep(Duration::from_millis(300));
            // Permission and the list of open apps change slowly: look about once a second.
            if tick % 3 == 0 {
                trusted = platform::accessibility_trusted(false);
                if trusted { crate::bars::sync(&app); }
            }
            tick = tick.wrapping_add(1);
            // The followed app in front is where saves (and the hover panel's pastes) go.
            if trusted {
                if let Some(front) = platform::front_app() {
                    if crate::bars::wants(&front) && front.id != platform::active_id() { platform::set_active(&front); }
                }
            }
            crate::bars::update_main(&app, trusted);
            // Track editable-app inputs for caret restoration; terminals need docking and paste only.
            let input = (platform::target_is_front() && !platform::target_input_unsupported() && !platform::target_is_terminal())
                .then(platform::focused_input).flatten().filter(|_| CHIP.load(Ordering::SeqCst));
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
