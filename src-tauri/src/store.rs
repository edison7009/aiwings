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
