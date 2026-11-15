// Background watcher: keeps one bar and one chip per open AI app, tracks the app in use, and drives the chips.

use crate::platform::{self, place_chip, Frame};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub static CHIP: AtomicBool = AtomicBool::new(true);
/// What each chip shows (by window label): whether its input has text, or None when there is no input. Kept
/// so a chip page that loads after the event went out can ask for it.
pub static CHIP_STATES: Mutex<Vec<(String, Option<bool>)>> = Mutex::new(Vec::new());

/// When a chip was last pressed. Pressing it makes Wings the active app for a moment, so the target's
/// input looks gone; the chips keep their state until the action has handed focus back.
pub static PRESSED_AT: Mutex<Option<Instant>> = Mutex::new(None);
const PRESS_HOLD: Duration = Duration::from_millis(1500);

/// Forgets a closed app's chip.
pub fn forget_chip(label: &str) { CHIP_STATES.lock().unwrap().retain(|(l, _)| l != label); }

fn tell_chip(app: &AppHandle, label: &str, state: Option<bool>) {
    let mut all = CHIP_STATES.lock().unwrap();
    all.retain(|(l, _)| l != label);
    all.push((label.to_string(), state));
    let _ = app.emit_to(label, "wings://chip", state);
}

/// One app's chip as the watcher last left it.
#[derive(Default)]
struct Slot {
    /// What the page was last told: text or not, None while there is no input.
    state: Option<bool>,
    /// Whether the app's bar was up, for the arrow.
    bar: Option<bool>,
    /// The app's window frame when the chip was last placed on its input: when another app is in use and
    /// this window moves, the chip moves with it.
    anchor: Option<Frame>,
    /// When the input went away the chip first fades; the window hides once that is over.
    hide_at: Option<Instant>,
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        // Each followed app has its own chip. The app in use moves its chip with its input box; the others
        // keep theirs where they were (nothing changes in an app while it is not used), following only
        // their window, so switching between apps never moves a chip around.
        let mut slots: Vec<(String, Slot)> = Vec::new();
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
            // The followed app in front is where saves and pastes go.
            if trusted {
                if let Some(front) = platform::front_app() {
                    if crate::bars::wants(&front) && front.id != platform::active_id() { platform::set_active(&front); }
                }
            }
            crate::bars::update_main(&app, trusted);
            let targets = crate::bars::targets();
            slots.retain(|(id, _)| targets.iter().any(|t| &t.id == id));
            // A press, or a recording, keeps every chip where it is until the action is done.
            let hold = PRESSED_AT.lock().unwrap().is_some_and(|t| t.elapsed() < PRESS_HOLD)
                || crate::voice::ACTIVE.load(Ordering::SeqCst);
            let active = platform::active_id();
            let chips_on = CHIP.load(Ordering::SeqCst);
            for t in &targets {
                let label = crate::bars::chip_label(&t.id);
                let Some(chip) = app.get_webview_window(&label) else { continue };
                if !slots.iter().any(|(id, _)| id == &t.id) { slots.push((t.id.clone(), Slot::default())); }
                let slot = &mut slots.iter_mut().find(|(id, _)| id == &t.id).unwrap().1;
                // The arrow points the way the bars will go: in when this app's bar is up, out when not.
                let bar = crate::bars::bar_up(&app, &t.id);
                if slot.bar != Some(bar) {
                    slot.bar = Some(bar);
                    let _ = app.emit_to(label.as_str(), "wings://bar", bar);
                }
                if hold { continue; }
                // Another app is in use and this chip has nothing to show: no need to ask about its window.
                if t.id != active && !(chips_on && slot.state.is_some()) {
                    if chip.is_visible().unwrap_or(false) { platform::hide(&chip); }
                    continue;
                }
                let window = platform::window_frame(&t.id);
                let scale = chip.scale_factor().unwrap_or(1.0);
                // Accessibility frames are in points on macOS, physical pixels on Windows.
                let s = if cfg!(target_os = "macos") { scale } else { 1.0 };
                if t.id == active {
                    // Terminals need docking and paste only.
                    let input = (window.is_some() && chips_on && !platform::target_input_unsupported() && !platform::target_is_terminal())
                        .then(platform::focused_input).flatten();
                    match input {
                        Some((frame, has_text)) => {
                            slot.hide_at = None;
                            slot.anchor = window;
                            let just_shown = place_chip(&chip, Frame { x: frame.x * s, y: frame.y * s, w: frame.w * s, h: frame.h * s }, scale);
                            platform::layer_chip(&chip, &t.id, just_shown);
                            // A hidden window's page is paused: let it paint once before the flip starts, or the
                            // first frames are dropped and the button looks cut off for a moment.
                            if just_shown { continue; }
                            if slot.state != Some(has_text) {
                                slot.state = Some(has_text);
                                tell_chip(&app, &label, Some(has_text));
                            }
                        }
                        None => {
                            if slot.state.is_some() {
                                slot.state = None;
                                slot.anchor = None;
                                tell_chip(&app, &label, None);
                                slot.hide_at = Some(Instant::now() + Duration::from_millis(500));
                            }
                            if slot.hide_at.is_some_and(|t| Instant::now() >= t) {
                                slot.hide_at = None;
                                platform::hide(&chip);
                            }
                        }
                    }
                    continue;
                }
                // Another app is in use: this chip stays, following its window, hidden while that window is
                // minimized or hidden.
                let shown = chip.is_visible().unwrap_or(false);
                match (window, slot.anchor) {
                    (Some(now), Some(then)) => {
                        if now.x != then.x || now.y != then.y {
                            platform::shift_chip(&chip, (now.x - then.x) * s, (now.y - then.y) * s);
                            slot.anchor = Some(now);
                        } else if !shown {
                            platform::show_no_activate(&chip);
                        }
                        platform::layer_chip(&chip, &t.id, !shown);
                    }
                    _ => if shown { platform::hide(&chip); },
                }
            }
        }
    });
}
