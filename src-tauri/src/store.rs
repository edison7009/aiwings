// Notes persistence: one JSON file in the app data dir, written whole on every change.
// The Rust side owns the list: every window adds and removes through it and gets the new list
// broadcast as `wings://notes`, so several bars never overwrite each other.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub created_at: u64,
    /// Image notes: id of the PNG under images/ (text is then empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Key of the app the note was saved from, lowercased; absent for older notes and the welcome note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

/// Loaded on first use; None until then, or while the file cannot be read (so a bad read never overwrites it).
static NOTES: Mutex<Option<Vec<Note>>> = Mutex::new(None);

fn notes_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("notes.json"))
}

/// A broken file starts an empty list rather than blocking the app; entries that do not parse are skipped.
fn parse(data: &str) -> Vec<Note> {
    serde_json::from_str::<Vec<serde_json::Value>>(data)
        .map(|list| list.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
        .unwrap_or_default()
}

fn read(app: &AppHandle) -> Result<Vec<Note>, String> {
    match fs::read_to_string(notes_path(app)?) {
        Ok(s) => Ok(parse(&s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn write(app: &AppHandle, notes: &[Note]) -> Result<(), String> {
    let path = notes_path(app)?;
    let data = serde_json::to_string(notes).map_err(|e| e.to_string())?;
    // Write to a sibling temp file then rename, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Applies a change, writes the file and tells every window. Fails only when the file cannot be read.
/// A failed write keeps the change in memory (a captured input is already cut, so dropping it would lose
/// the text); the next change writes it again.
fn change(app: &AppHandle, f: impl FnOnce(&mut Vec<Note>)) -> Result<Vec<Note>, String> {
    let mut guard = NOTES.lock().unwrap();
    if guard.is_none() { *guard = Some(read(app)?); }
    let notes = guard.as_mut().unwrap();
    f(notes);
    let list = notes.clone();
    // Written under the lock, so two quick changes cannot land on disk in the wrong order.
    let _ = write(app, &list);
    drop(guard);
    let _ = app.emit("wings://notes", &list);
    Ok(list)
}

/// Unique enough for one user's notes, and safe as an image file name.
pub fn new_id() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    format!("{:x}-{:x}-{:x}", nanos, std::process::id(), SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Appends a note at the bottom (send order: oldest at the top). An image note's id is its image id.
pub fn push(app: &AppHandle, text: String, image: Option<String>, source: Option<String>) -> Result<Vec<Note>, String> {
    let id = image.clone().unwrap_or_else(new_id);
    let note = Note { id, text, created_at: now_ms(), image, app: source };
    change(app, |notes| notes.push(note))
}

#[tauri::command]
pub fn load_notes(app: AppHandle) -> Result<Vec<Note>, String> {
    let mut guard = NOTES.lock().unwrap();
    if guard.is_none() { *guard = Some(read(&app)?); }
    Ok(guard.clone().unwrap_or_default())
}

/// Notes added from the UI (the welcome note) belong to no app.
#[tauri::command]
pub fn add_note(app: AppHandle, text: String) -> Result<Vec<Note>, String> {
    push(&app, text, None, None)
}

/// Removes a note, and its image file if it had one.
#[tauri::command]
pub fn remove_note(app: AppHandle, id: String) -> Result<Vec<Note>, String> {
    let mut gone = None;
    let list = change(&app, |notes| {
        if let Some(i) = notes.iter().position(|n| n.id == id) { gone = Some(notes.remove(i)); }
    })?;
    if let Some(image) = gone.and_then(|n| n.image) { let _ = remove_image(&app, &image); }
    Ok(list)
}

// ─── Image notes: one PNG per note under <app data>/images/<id>.png ────────────

fn image_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    // Ids are UUIDs we generate; refuse anything else so a crafted id cannot escape the folder.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("invalid image id".into());
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("images");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("{id}.png")))
}

pub fn save_image(app: &AppHandle, id: &str, img: &arboard::ImageData) -> Result<(), String> {
    let file = fs::File::create(image_path(app, id)?).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width as u32, img.height as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&img.bytes).map_err(|e| e.to_string())
}

pub fn load_image(app: &AppHandle, id: &str) -> Result<arboard::ImageData<'static>, String> {
    let file = fs::File::open(image_path(app, id)?).map_err(|e| e.to_string())?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    Ok(arboard::ImageData { width: info.width as usize, height: info.height as usize, bytes: buf.into() })
}

/// The stored PNG as a data URL, for the note's thumbnail.
#[tauri::command]
pub fn image_data_url(app: AppHandle, id: String) -> Result<String, String> {
    use base64::Engine;
    let bytes = fs::read(image_path(&app, &id)?).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Drops an image that never made it into a note.
pub fn discard_image(app: &AppHandle, id: &str) { let _ = remove_image(app, id); }

fn remove_image(app: &AppHandle, id: &str) -> Result<(), String> {
    match fs::remove_file(image_path(app, id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_files_load_and_bad_entries_are_skipped() {
        let old = r#"[
            {"id":"a","text":"hi","createdAt":1700000000000},
            {"id":"b","text":"","createdAt":1700000000001,"image":"b"},
            {"text":"no id"},
            {"id":"c","text":"from claude","createdAt":1,"app":"com.anthropic.claudefordesktop"}
        ]"#;
        let notes = parse(old);
        assert_eq!(notes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        assert_eq!(notes[1].image.as_deref(), Some("b"));
        assert!(notes[0].app.is_none());
        assert_eq!(notes[2].app.as_deref(), Some("com.anthropic.claudefordesktop"));
        assert!(parse("not json").is_empty());
    }

    #[test]
    fn missing_fields_stay_out_of_the_file() {
        let note = Note { id: "a".into(), text: "hi".into(), created_at: 5, image: None, app: None };
        assert_eq!(serde_json::to_string(&note).unwrap(), r#"{"id":"a","text":"hi","createdAt":5}"#);
    }

    #[test]
    fn ids_are_unique_and_safe_file_names() {
        let (a, b) = (new_id(), new_id());
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }
}
