//! Self-update, same flow as the sister apps (EchoBird, Coffee CLI): the newest version any source reports
//! lights a red download button beside the gear; clicking it downloads the installer for this system into
//! Downloads, from the domestic mirrors (filled by .github/workflows/mirror-release.yml) or GitHub.
//! A mirror is only used when its manifest has exactly the target version, and its file is checked
//! against the manifest's size and SHA-256. Windows then runs the setup wizard and quits; macOS opens the dmg.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const CNB: &str = "https://cnb.cool/echobird/aiwings-releases/-/git/raw/main/";
const GITCODE: &str = "https://api.gitcode.com/api/v5/repos/edison7009/aiwings-releases/raw/";
const GITHUB_LATEST: &str = "https://api.github.com/repos/edison7009/aiwings/releases/latest";

static RUNNING: Mutex<bool> = Mutex::new(false);

#[derive(Deserialize, Clone)]
struct FileEntry { size: u64, sha256: String }

#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    files: HashMap<String, FileEntry>,
}

#[derive(Serialize, Clone)]
struct Progress { status: &'static str, percent: u8 }

fn emit(app: &AppHandle, status: &'static str, percent: u8) {
    let _ = app.emit("wings://update-progress", Progress { status, percent });
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(6)).timeout_read(Duration::from_secs(30)).build()
}

fn cnb_url(name: &str) -> String { format!("{CNB}{name}") }
fn gitcode_url(name: &str) -> String { format!("{GITCODE}{name}?ref=main") }

fn semver(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches('v').split('.');
    let out = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    it.next().is_none().then_some(out)
}

fn mirror_manifest(url: &str) -> Option<Manifest> {
    let m: Manifest = agent().get(url).timeout(Duration::from_secs(5)).call().ok()?.into_json().ok()?;
    semver(&m.version).map(|_| m)
}

fn github_version() -> Option<String> {
    #[derive(Deserialize)]
    struct Release { tag_name: String }
    let r: Release = agent().get(GITHUB_LATEST).timeout(Duration::from_secs(5)).set("User-Agent", "Wings").call().ok()?.into_json().ok()?;
    semver(&r.tag_name).map(|_| r.tag_name.trim_start_matches('v').to_string())
}

/// Installer name for this system, as the Release workflow names it.
fn asset(version: &str) -> Option<String> {
    if cfg!(target_os = "windows") { Some(format!("Wings-{version}-windows-x64-setup.exe")) }
    else if cfg!(target_os = "macos") { Some(format!("Wings-{version}-darwin-universal.dmg")) }
    else { None }
}

/// The newest version any source reports, if it is newer than this build. All sources are asked at once.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Option<String> {
    let current = semver(&app.package_info().version.to_string())?;
    let tasks = [
        std::thread::spawn(|| mirror_manifest(&cnb_url("manifest.json")).map(|m| m.version)),
        std::thread::spawn(|| mirror_manifest(&gitcode_url("manifest.json")).map(|m| m.version)),
        std::thread::spawn(github_version),
    ];
    let newest = tauri::async_runtime::spawn_blocking(move || {
        tasks.into_iter().filter_map(|t| t.join().ok().flatten()).max_by_key(|v| semver(v))
    }).await.ok()??;
    (semver(&newest)? > current).then_some(newest)
}

#[tauri::command]
pub async fn download_update(app: AppHandle, version: String) -> Result<(), String> {
    {
        let mut running = RUNNING.lock().unwrap();
        if *running { return Err("正在下载更新".into()); }
        *running = true;
    }
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || run(&handle, &version)).await.map_err(|e| e.to_string()).and_then(|r| r);
    *RUNNING.lock().unwrap() = false;
    if result.is_err() { emit(&app, "error", 0); }
    result
}

fn run(app: &AppHandle, version: &str) -> Result<(), String> {
    semver(version).ok_or("版本号无效")?;
    let name = asset(version).ok_or("这个系统没有可用的安装包")?;
    emit(app, "speed_test", 0);
    // Mirrors that carry exactly this version, then GitHub (checked against a mirror's digest when one has it).
    let mut sources: Vec<(String, Option<FileEntry>)> = Vec::new();
    for (manifest, file) in [(cnb_url("manifest.json"), cnb_url(&name)), (gitcode_url("manifest.json"), gitcode_url(&name))] {
        if let Some(m) = mirror_manifest(&manifest) {
            if m.version.trim_start_matches('v') == version {
                if let Some(f) = m.files.get(&name) { sources.push((file, Some(f.clone()))); }
            }
        }
    }
    let expected = sources.first().and_then(|s| s.1.clone());
    sources.push((format!("https://github.com/edison7009/aiwings/releases/download/v{version}/{name}"), expected));

    let dir = downloads_dir().ok_or("找不到下载文件夹")?;
    let _ = std::fs::create_dir_all(&dir);
    let dest = dir.join(&name);
    let part = dir.join(format!(".{name}.part"));
    let mut last = "没有可用的下载源".to_string();
    for (url, expect) in sources {
        match fetch(app, &url, expect.as_ref(), &part) {
            Ok(()) => {
                std::fs::rename(&part, &dest).map_err(|e| format!("保存安装包失败：{e}"))?;
                emit(app, "launching", 100);
                return launch(app, &dest);
            }
            Err(e) => { last = e; let _ = std::fs::remove_file(&part); }
        }
    }
    Err(last)
}

fn fetch(app: &AppHandle, url: &str, expect: Option<&FileEntry>, part: &PathBuf) -> Result<(), String> {
    let resp = agent().get(url).set("User-Agent", "Wings").call().map_err(|e| e.to_string())?;
    if resp.content_type().contains("text/html") { return Err("下载源返回的不是安装包".into()); }
    let total = expect.map(|e| e.size).or_else(|| resp.header("content-length").and_then(|v| v.parse().ok()));
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let (mut size, mut shown) = (0u64, 0u8);
    let mut buf = vec![0; 64 * 1024];
    emit(app, "downloading", 0);
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        size += n as u64;
        if let Some(t) = total.filter(|t| *t > 0) {
            let pct = (size * 100 / t).min(99) as u8;
            if pct != shown { shown = pct; emit(app, "downloading", pct); }
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    if size == 0 || total.is_some_and(|t| t != size) { return Err("下载不完整".into()); }
    if let Some(e) = expect {
        if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(&e.sha256) { return Err("安装包校验失败".into()); }
    }
    Ok(())
}

fn downloads_dir() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    Some(PathBuf::from(home).join("Downloads"))
}

fn launch(app: &AppHandle, path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        std::process::Command::new(path).creation_flags(DETACHED_PROCESS).spawn().map_err(|e| format!("打开安装程序失败：{e}"))?;
        std::thread::sleep(Duration::from_millis(600));
        app.exit(0);
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("open").arg(path).spawn().map_err(|e| format!("打开安装包失败：{e}"))?;
        emit(app, "downloaded", 100);
    }
    Ok(())
}

/// Opens a web page in the default browser (the website link in settings).
#[tauri::command]
pub fn open_url(url: String) {
    if !url.starts_with("https://") { return; }
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(target_os = "windows"))]
    let _ = std::process::Command::new("open").arg(&url).spawn();
}
