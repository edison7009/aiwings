// Notes persistence: one JSON file in the app data dir, written whole on every change.

use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

fn notes_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("notes.json"))
}

#[tauri::command]
pub fn load_notes(app: AppHandle) -> Result<String, String> {
    match fs::read_to_string(notes_path(&app)?) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("[]".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn save_notes(app: AppHandle, data: String) -> Result<(), String> {
    let path = notes_path(&app)?;
    // Write to a sibling temp file then rename, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
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

#[tauri::command]
pub fn delete_image(app: AppHandle, id: String) -> Result<(), String> {
    match fs::remove_file(image_path(&app, &id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}
