// The two user actions: capture the target's input box into a note, and paste a note back.
// Both run on a worker thread: they sleep while the target app processes synthetic keys.

use crate::{clip, platform, store};
use serde::Serialize;
use std::{thread::sleep, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter};

const CLIP_WAIT: Duration = Duration::from_millis(700);

pub fn emit_error(app: &AppHandle, msg: &str) {
    let _ = app.emit("wings://error", msg.to_string());
}

/// What a capture produced: text, or (when the cut yielded no text) an image saved under `image`.
#[derive(Serialize, Clone)]
struct Captured { text: String, image: Option<String> }

/// Select-all + cut in the target, keep what landed on the clipboard, then restore the clipboard.
pub fn capture(app: &AppHandle) {
    if !platform::has_target() {
        return emit_error(app, "还没选择要吸附的窗口");
    }
    if platform::target_is_terminal() {
        return emit_error(app, "终端里不支持保存，只能把便签粘贴进去");
    }
    if !platform::target_is_front() {
        return emit_error(app, "请先点一下目标窗口的输入框，再按快捷键");
    }
    platform::wait_modifiers_released();
    let Ok(mut cb) = arboard::Clipboard::new() else { return emit_error(app, "无法访问剪贴板") };
    let saved = clip::snapshot(&mut cb);
    let _ = cb.clear();

    let ok = platform::send_combo(platform::Key::A)
        .and_then(|_| { sleep(Duration::from_millis(60)); platform::send_combo(platform::Key::X) });
    if let Err(e) = ok {
        clip::restore(&mut cb, saved);
        return emit_error(app, &e);
    }

    let start = Instant::now();
    let mut text = String::new();
    let mut image = None;
    while start.elapsed() < CLIP_WAIT {
        sleep(Duration::from_millis(40));
        if let Ok(t) = cb.get_text() {
            if !t.trim().is_empty() { text = t; break; }
        }
        if let Ok(img) = cb.get_image() {
            image = Some(img.to_owned_img());
            break;
        }
    }
    clip::restore(&mut cb, saved);

    let text = text.trim().to_string();
    if !text.is_empty() {
        let _ = app.emit("wings://captured", Captured { text, image: None });
    } else if let Some(img) = image {
        // The id doubles as the file name; the UI uses it as the note id too.
        let id = format!("{:x}-{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0), std::process::id());
        match store::save_image(app, &id, &img) {
            Ok(()) => { let _ = app.emit("wings://captured", Captured { text: String::new(), image: Some(id) }); }
            Err(_) => emit_error(app, "图片保存失败"),
        }
    } else {
        emit_error(app, "输入框是空的，没有可保存的内容");
    }
}

/// Bring the target forward, put the caret in its input box and paste the note. Never presses Enter.
/// The note stays on the clipboard afterwards, so the user can paste it anywhere else too.
pub fn paste(app: &AppHandle, text: String, image: Option<String>) {
    if !platform::activate_target() {
        return emit_error(app, "没能切换到目标窗口，请手动点一下它再试");
    }
    platform::focus_input();
    let Ok(mut cb) = arboard::Clipboard::new() else { return emit_error(app, "无法访问剪贴板") };
    let written = match image {
        Some(id) => store::load_image(app, &id).and_then(|img| cb.set_image(img).map_err(|e| e.to_string())),
        None => cb.set_text(text).map_err(|e| e.to_string()),
    };
    if written.is_err() {
        return emit_error(app, "无法写入剪贴板");
    }
    sleep(Duration::from_millis(60));
    // Focus guard: re-check right before the keystroke so it never lands in the wrong window.
    if !platform::target_is_front() {
        return emit_error(app, "目标窗口不在最前面，已取消粘贴（内容已复制，可手动粘贴）");
    }
    if let Err(e) = platform::send_combo(platform::Key::V) {
        emit_error(app, &e);
    }
}

/// Empties the target's input box: select all, then Delete. Clicked from the chip, so focus is handed back first.
pub fn clear(app: &AppHandle) {
    if !platform::activate_target() {
        return emit_error(app, "没能切换到目标窗口");
    }
    platform::focus_input();
    let res = platform::send_combo(platform::Key::A)
        .and_then(|_| { sleep(Duration::from_millis(60)); platform::send_combo(platform::Key::Delete) });
    if let Err(e) = res { emit_error(app, &e); }
}
