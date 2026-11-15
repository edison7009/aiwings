//! Voice input for the chip on the target's input box: the microphone streams to a speech-to-text
//! service and the recognized text is pasted into the box. Ported from Coffee CLI's voice input:
//! Claude Code and Codex reuse the sign-in their own tool stored on this machine (read only, never
//! refreshed, never sent to the page); macOS uses Apple Speech; "api" is an OpenAI-compatible
//! transcription endpoint with the user's own key, sent as one recording once it ends.
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::{AtomicBool, AtomicU64, Ordering}, LazyLock, Mutex},
    time::Duration,
};
use tauri::{ipc::Channel, AppHandle, Manager};
use tokio::{sync::mpsc, time::Instant};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};
#[cfg(target_os = "macos")]
mod apple;

pub const SAMPLE_RATE: u32 = 16_000;
/// 100 ms of audio per frame.
const CHUNK: usize = SAMPLE_RATE as usize / 10;

/// A recording is running: the chip holds still (and stays shown) until it is over.
pub static ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VoiceEvent {
    /// The service accepted the stream; speech from here on is transcribed.
    Ready,
    /// Recording ended; the batch service is processing it.
    Processing,
    /// Everything heard so far in this recording. `done` marks the service's final text.
    Text { text: String, done: bool },
    Error { message: String },
    /// Loudness of the last 100 ms of microphone audio, 0–1.
    Level { level: f32 },
}

pub enum Control { Finish, Cancel }

static SESSIONS: LazyLock<Mutex<HashMap<u64, mpsc::UnboundedSender<Control>>>> = LazyLock::new(Default::default);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

// ---- Local sign-ins (Claude Code, Codex) ----

struct Account { token: String, account_id: Option<String> }

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// `$KEY` when set, otherwise `~/fallback`.
fn config_dir(key: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty()).map(PathBuf::from).or_else(|| Some(home()?.join(fallback)))
}

/// Claude Code keeps its sign-in in the login keychain on macOS.
#[cfg(target_os = "macos")]
fn claude_keychain() -> Option<String> {
    let output = std::process::Command::new("security").args(["find-generic-password", "-s", "Claude Code-credentials", "-w"]).output().ok()?;
    output.status.success().then(|| String::from_utf8(output.stdout).ok()).flatten()
}
#[cfg(not(target_os = "macos"))]
fn claude_keychain() -> Option<String> { None }

fn claude_stored() -> Option<String> {
    claude_keychain().or_else(|| std::fs::read_to_string(config_dir("CLAUDE_CONFIG_DIR", ".claude")?.join(".credentials.json")).ok())
}

fn claude_entry(text: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(text).ok()?;
    parsed.get("claudeAiOauth").or_else(|| parsed.get("claude.ai_oauth")).cloned()
}

/// Claude Code's stored sign-in, or Err("expired") when it ran out (the tool renews it only while it runs).
fn claude_account() -> Result<Account, &'static str> {
    let entry = claude_stored().as_deref().and_then(claude_entry).ok_or("missing")?;
    let token = entry["accessToken"].as_str().filter(|t| !t.is_empty()).ok_or("missing")?;
    if entry["expiresAt"].as_i64().is_some_and(|at| at <= now_ms()) { return Err("expired"); }
    // The account's id is in Claude Code's settings file, next to the credentials' folder.
    let account_id = std::env::var("CLAUDE_CONFIG_DIR").ok().filter(|p| !p.trim().is_empty())
        .map(|p| PathBuf::from(p).join(".claude.json")).or_else(|| Some(home()?.join(".claude.json")))
        .and_then(|path| serde_json::from_slice::<Value>(&std::fs::read(path).ok()?).ok())
        .and_then(|v| v["oauthAccount"]["accountUuid"].as_str().filter(|id| !id.is_empty()).map(str::to_owned));
    Ok(Account { token: token.to_string(), account_id })
}

/// Codex's ChatGPT sign-in (an API-key sign-in has no dictation).
fn codex_account() -> Result<Account, &'static str> {
    let text = std::fs::read_to_string(config_dir("CODEX_HOME", ".codex").ok_or("missing")?.join("auth.json")).map_err(|_| "missing")?;
    let parsed: Value = serde_json::from_str(&text).map_err(|_| "missing")?;
    if parsed["auth_mode"] != "chatgpt" { return Err("missing"); }
    let tokens = &parsed["tokens"];
    Ok(Account {
        token: tokens["access_token"].as_str().filter(|t| !t.is_empty()).ok_or("missing")?.to_string(),
        account_id: tokens["account_id"].as_str().filter(|id| !id.is_empty()).map(str::to_owned),
    })
}

// ---- The user's own transcription API ----

#[derive(Serialize, Deserialize, Clone, Default)]
struct ApiConfig { base: String, key: String, model: String }

const API_BASE: &str = "https://api.openai.com/v1";
const API_MODEL: &str = "whisper-1";

fn api_path(app: &AppHandle) -> Option<PathBuf> { Some(app.path().app_data_dir().ok()?.join("voice-api.json")) }

fn load_api(app: &AppHandle) -> Option<ApiConfig> {
    let config: ApiConfig = serde_json::from_slice(&std::fs::read(api_path(app)?).ok()?).ok()?;
    (!config.key.is_empty()).then_some(config)
}

/// "sk-…abcd": enough to tell keys apart, never the key itself.
fn key_hint(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 { return "…".into(); }
    format!("{}…{}", chars[..3].iter().collect::<String>(), chars[chars.len() - 4..].iter().collect::<String>())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiView { base: String, model: String, key_hint: Option<String> }

/// The saved endpoint for Settings; the key only as a hint.
#[tauri::command]
pub fn voice_api_get(app: AppHandle) -> ApiView {
    let saved = api_path(&app).and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice::<ApiConfig>(&b).ok()).unwrap_or_default();
    ApiView {
        base: if saved.base.is_empty() { API_BASE.into() } else { saved.base },
        model: if saved.model.is_empty() { API_MODEL.into() } else { saved.model },
        key_hint: (!saved.key.is_empty()).then(|| key_hint(&saved.key)),
    }
}

/// Saves the endpoint. `key`: None keeps the saved key, an empty string removes it.
#[tauri::command]
pub fn voice_api_set(app: AppHandle, base: String, model: String, key: Option<String>) -> Result<(), String> {
    let path = api_path(&app).ok_or("无法保存设置")?;
    let old = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice::<ApiConfig>(&b).ok()).unwrap_or_default();
    let config = ApiConfig {
        base: base.trim().trim_end_matches('/').to_string(),
        model: model.trim().to_string(),
        key: key.map(|k| k.trim().to_string()).unwrap_or(old.key),
    };
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir).map_err(|_| "无法保存设置")?; }
    std::fs::write(&path, serde_json::to_vec(&config).map_err(|e| e.to_string())?).map_err(|_| "无法保存设置".into())
}

// ---- Sources ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSource {
    id: &'static str,
    account_id: Option<String>,
    /// Why it cannot be used now: "missing" (not signed in), "expired", "unset" (no key yet).
    issue: Option<&'static str>,
    /// "streaming": words appear while speaking; "batch": all at once after the recording.
    mode: &'static str,
}

/// Every source Wings knows, each with whether it can be used right now. Tokens never cross IPC.
#[tauri::command]
pub async fn voice_sources(app: AppHandle) -> Vec<VoiceSource> {
    tauri::async_runtime::spawn_blocking(move || {
        let local = |id: &'static str, account: Result<Account, &'static str>| match account {
            Ok(a) => VoiceSource { id, account_id: a.account_id, issue: None, mode: "streaming" },
            Err(issue) => VoiceSource { id, account_id: None, issue: Some(issue), mode: "streaming" },
        };
        let api = load_api(&app);
        #[allow(unused_mut)]
        let mut list = vec![
            local("claude", claude_account()),
            local("codex", codex_account()),
            VoiceSource { id: "api", account_id: api.as_ref().map(|c| key_hint(&c.key)), issue: api.is_none().then_some("unset"), mode: "batch" },
        ];
        #[cfg(target_os = "macos")]
        list.push(VoiceSource { id: "macos", account_id: None, issue: (!apple::available()).then_some("missing"), mode: "streaming" });
        list
    }).await.unwrap_or_default()
}

enum Input { Claude(Account), Codex(Account), Api(ApiConfig), #[cfg(target_os = "macos")] Apple }

/// The chosen source, or with "" (automatic) the first usable one.
fn resolve(app: &AppHandle, source: &str) -> Result<Input, String> {
    let one = |id: &str| -> Result<Input, String> {
        match id {
            "claude" => claude_account().map(Input::Claude).map_err(|_| "signed_out".into()),
            "codex" => codex_account().map(Input::Codex).map_err(|_| "signed_out".into()),
            "api" => load_api(app).map(Input::Api).ok_or_else(|| "api_unset".into()),
            #[cfg(target_os = "macos")]
            "macos" => if apple::available() { Ok(Input::Apple) } else { Err("speech_unavailable".into()) },
            _ => Err("no_source".into()),
        }
    };
    if !source.is_empty() { return one(source); }
    ["claude", "codex", "api", "macos"].into_iter().find_map(|id| one(id).ok()).ok_or_else(|| "no_source".into())
}

#[tauri::command]
pub async fn voice_start(app: AppHandle, source: String, language: String, device: Option<String>, on_event: Channel<VoiceEvent>) -> Result<u64, String> {
    let handle = app.clone();
    let input = tauri::async_runtime::spawn_blocking(move || resolve(&handle, &source)).await.map_err(|e| e.to_string())??;
    let (audio_tx, audio_rx) = mpsc::unbounded_channel();
    let capture = capture::start(audio_tx, device)?;
    let (control_tx, control_rx) = mpsc::unbounded_channel();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    SESSIONS.lock().unwrap().insert(id, control_tx);
    ACTIVE.store(true, Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        let emit = |event: VoiceEvent| { let _ = on_event.send(event); };
        let stop = || capture.stop();
        let result = match input {
            Input::Claude(account) => run_claude(&account.token, &language, audio_rx, control_rx, &emit, stop).await,
            Input::Codex(account) => run_codex(&account.token, audio_rx, control_rx, &emit, stop).await,
            Input::Api(config) => run_api(config, &language, audio_rx, control_rx, &emit, stop).await,
            #[cfg(target_os = "macos")]
            Input::Apple => apple::run(&language, audio_rx, control_rx, &emit, stop).await,
        };
        capture.stop();
        let mut sessions = SESSIONS.lock().unwrap();
        sessions.remove(&id);
        if sessions.is_empty() { ACTIVE.store(false, Ordering::SeqCst); }
        drop(sessions);
        if let Err(message) = result { emit(VoiceEvent::Error { message }); }
    });
    Ok(id)
}

fn control(id: u64, command: Control) {
    if let Some(sender) = SESSIONS.lock().unwrap().get(&id) { let _ = sender.send(command); }
}

/// Stop listening and wait for the service's final text.
#[tauri::command]
pub fn voice_finish(id: u64) { control(id, Control::Finish); }

/// Stop listening and drop the recording.
#[tauri::command]
pub fn voice_cancel(id: u64) { control(id, Control::Cancel); }

#[derive(Serialize)]
pub struct Microphones { devices: Vec<String>, default: Option<String> }

/// Input devices by name, and which one the system uses by default.
#[tauri::command]
pub async fn voice_devices() -> Microphones {
    tauri::async_runtime::spawn_blocking(capture::devices).await.unwrap_or(Microphones { devices: Vec::new(), default: None })
}

static TESTS: LazyLock<Mutex<HashMap<u64, std::sync::Arc<capture::Capture>>>> = LazyLock::new(Default::default);

/// Microphone check for Settings: streams the loudness of the chosen device, nothing leaves the machine.
#[tauri::command]
pub async fn voice_mic_test_start(device: Option<String>, on_level: Channel<f32>) -> Result<u64, String> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let capture = std::sync::Arc::new(capture::start(tx, device)?);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    TESTS.lock().unwrap().insert(id, capture);
    tauri::async_runtime::spawn(async move {
        while let Some(chunk) = rx.recv().await { if on_level.send(level(&chunk)).is_err() { break; } }
        if let Some(capture) = TESTS.lock().unwrap().remove(&id) { capture.stop(); }
    });
    Ok(id)
}

#[tauri::command]
pub async fn voice_mic_test_stop(id: u64) {
    if let Some(capture) = TESTS.lock().unwrap().remove(&id) { capture.stop(); }
}

/// Concatenate recognized pieces: a space between two Latin words, nothing between CJK text.
pub fn join(parts: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.trim();
        if part.is_empty() { continue; }
        let wide = |c: char| (c as u32) >= 0x2E80;
        if let (Some(last), Some(first)) = (out.chars().last(), part.chars().next()) {
            if !wide(last) && !wide(first) { out.push(' '); }
        }
        out.push_str(part);
    }
    out
}

/// Loudness of a chunk: RMS in dBFS mapped from −50…−20 dB onto 0–1.
pub fn level(samples: &[i16]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let power = samples.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / samples.len() as f64;
    if power == 0.0 { return 0.0; }
    let db = 20.0 * (power.sqrt() / 32768.0).log10();
    ((db + 50.0) / 30.0).clamp(0.0, 1.0) as f32
}

fn pcm_bytes(samples: &[i16]) -> Vec<u8> { samples.iter().flat_map(|s| s.to_le_bytes()).collect() }

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect(request: tokio_tungstenite::tungstenite::handshake::client::Request) -> Result<Socket, String> {
    let connecting = tokio_tungstenite::connect_async(request);
    match tokio::time::timeout(Duration::from_secs(10), connecting).await {
        Ok(Ok((socket, _))) => Ok(socket),
        Ok(Err(tokio_tungstenite::tungstenite::Error::Http(response))) => Err(format!("http_{}", response.status().as_u16())),
        Ok(Err(error)) => Err(format!("connect: {error}")),
        Err(_) => Err("connect: timed out".into()),
    }
}

fn header(value: &str) -> Result<HeaderValue, String> { HeaderValue::from_str(value).map_err(|e| e.to_string()) }

/// Claude's partial text is the whole current utterance; an endpoint settles it.
#[derive(Default)]
struct ClaudeTranscript { settled: Vec<String>, current: String }

impl ClaudeTranscript {
    /// Returns true when the message changed the text.
    fn apply(&mut self, message: &Value) -> Result<bool, String> {
        match message["type"].as_str().unwrap_or("") {
            "TranscriptInterim" | "TranscriptText" => {
                let text = message["data"].as_str().unwrap_or("");
                if text.is_empty() || text == self.current { return Ok(false); }
                self.current = text.to_string();
                Ok(true)
            }
            "TranscriptEndpoint" => { self.settle(); Ok(false) }
            "TranscriptError" => Err(message["description"].as_str().or(message["error_code"].as_str()).unwrap_or("transcription error").to_string()),
            "error" => Err(message["message"].as_str().unwrap_or("transcription error").to_string()),
            _ => Ok(false),
        }
    }
    fn settle(&mut self) { if !self.current.is_empty() { self.settled.push(std::mem::take(&mut self.current)); } }
    fn text(&self) -> String { join(self.settled.iter().cloned().chain([self.current.clone()])) }
}

async fn run_claude(
    token: &str, language: &str, mut audio: mpsc::UnboundedReceiver<Vec<i16>>, mut control: mpsc::UnboundedReceiver<Control>,
    emit: &(dyn Fn(VoiceEvent) + Send + Sync), stop_capture: impl Fn() + Send,
) -> Result<(), String> {
    let language = if language.trim().is_empty() { "en" } else { language };
    let query = format!("encoding=linear16&sample_rate={SAMPLE_RATE}&channels=1&endpointing_ms=300&utterance_end_ms=1000&language={language}&use_conversation_engine=true");
    let mut request = format!("wss://api.anthropic.com/api/ws/speech_to_text/voice_stream?{query}").into_client_request().map_err(|e| e.to_string())?;
    let headers = request.headers_mut();
    headers.insert("Authorization", header(&format!("Bearer {token}"))?);
    headers.insert("x-app", header("cli")?);
    headers.insert("User-Agent", header(concat!("wings/", env!("CARGO_PKG_VERSION")))?);
    let mut socket = tokio::select! {
        socket = connect(request) => socket?,
        _ = control.recv() => return Ok(()),
    };
    let keep_alive = || Message::Text(r#"{"type":"KeepAlive"}"#.into());
    socket.send(keep_alive()).await.map_err(|e| e.to_string())?;
    emit(VoiceEvent::Ready);
    let mut transcript = ClaudeTranscript::default();
    let mut ticker = tokio::time::interval(Duration::from_secs(8));
    ticker.tick().await;
    // After CloseStream: done at the next endpoint, 1.5 s of silence from the server, or 5 s at most.
    let mut closing: Option<(Instant, Instant)> = None;
    loop {
        let deadline = closing.map(|(hard, idle)| hard.min(idle)).unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));
        tokio::select! {
            chunk = audio.recv(), if closing.is_none() => if let Some(chunk) = chunk {
                emit(VoiceEvent::Level { level: level(&chunk) });
                socket.send(Message::Binary(pcm_bytes(&chunk).into())).await.map_err(|e| e.to_string())?;
            },
            command = control.recv(), if closing.is_none() => match command {
                Some(Control::Finish) => {
                    stop_capture();
                    while let Ok(chunk) = audio.try_recv() { let _ = socket.send(Message::Binary(pcm_bytes(&chunk).into())).await; }
                    socket.send(Message::Text(r#"{"type":"CloseStream"}"#.into())).await.map_err(|e| e.to_string())?;
                    let now = Instant::now();
                    closing = Some((now + Duration::from_secs(5), now + Duration::from_millis(1500)));
                }
                Some(Control::Cancel) | None => { let _ = socket.close(None).await; return Ok(()); }
            },
            _ = ticker.tick(), if closing.is_none() => { socket.send(keep_alive()).await.map_err(|e| e.to_string())?; }
            _ = tokio::time::sleep_until(deadline), if closing.is_some() => break,
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };
                    if let Some((_, idle)) = closing.as_mut() { *idle = Instant::now() + Duration::from_millis(1500); }
                    if transcript.apply(&value)? { emit(VoiceEvent::Text { text: transcript.text(), done: false }); }
                    if closing.is_some() && value["type"] == "TranscriptEndpoint" { break; }
                }
                Some(Ok(Message::Close(frame))) if closing.is_none() => {
                    return Err(format!("closed: {}", frame.map(|f| f.reason.to_string()).unwrap_or_default()));
                }
                None if closing.is_none() => return Err("closed".into()),
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(error)) => return Err(error.to_string()),
                _ => {}
            },
        }
    }
    let _ = socket.close(None).await;
    transcript.settle();
    emit(VoiceEvent::Text { text: transcript.text(), done: true });
    Ok(())
}

/// Codex streams per-utterance segments by revision; a final replaces its segment.
#[derive(Default)]
struct CodexTranscript { order: Vec<String>, entries: HashMap<String, (i64, String, bool)> }

impl CodexTranscript {
    fn apply(&mut self, message: &Value) -> Result<bool, String> {
        let kind = message["type"].as_str().unwrap_or("");
        match kind {
            "transcript.segment" | "transcript.final" => {
                let Some(id) = message["utterance_id"].as_str() else { return Ok(false) };
                let revision = message["revision"].as_i64().unwrap_or(0);
                let text = message["text"].as_str().unwrap_or("").to_string();
                let is_final = kind == "transcript.final";
                if !self.entries.contains_key(id) { self.order.push(id.to_string()); }
                let entry = self.entries.entry(id.to_string()).or_insert((-1, String::new(), false));
                if entry.2 && !is_final || revision < entry.0 && entry.2 == is_final { return Ok(false); }
                *entry = (revision, text, is_final);
                Ok(true)
            }
            "transcript.failed" => Err(message["error"]["message"].as_str().unwrap_or("transcription failed").to_string()),
            "session.error" if message["fatal"] == true => Err(message["error"]["message"].as_str().unwrap_or("session error").to_string()),
            _ => Ok(false),
        }
    }
    fn text(&self) -> String { join(self.order.iter().filter_map(|id| self.entries.get(id)).map(|(_, text, _)| text.clone())) }
}

async fn run_codex(
    token: &str, mut audio: mpsc::UnboundedReceiver<Vec<i16>>, mut control: mpsc::UnboundedReceiver<Control>,
    emit: &(dyn Fn(VoiceEvent) + Send + Sync), stop_capture: impl Fn() + Send,
) -> Result<(), String> {
    let mut request = "wss://chatgpt.com/backend-api/dictation/stream?client_type=desktop&dictation_surface=composer"
        .into_client_request().map_err(|e| e.to_string())?;
    request.headers_mut().insert("Sec-WebSocket-Protocol", header(&format!("chatgpt-dictation, openai-bearer.{token}, codex-desktop"))?);
    let mut socket = tokio::select! {
        socket = connect(request) => socket?,
        _ = control.recv() => return Ok(()),
    };
    let start = json!({
        "type": "session.start",
        "config": {
            "input_audio_format": "pcm16", "sample_rate_hz": SAMPLE_RATE, "num_channels": 1,
            "max_buffer_size_bytes": 4_194_304, "max_utterance_duration_ms": 30_000, "session_ttl_ms": 300_000,
            "provider_mode": "streaming_sse", "transcript_delivery_mode": "segment",
            "vad": { "type": "server_vad", "threshold": 0.5, "prefix_padding_ms": 300, "silence_duration_ms": 500 },
        },
    });
    socket.send(Message::Text(start.to_string().into())).await.map_err(|e| e.to_string())?;
    let append = |samples: &[i16]| {
        use base64::Engine;
        Message::Text(json!({ "type": "audio.append", "audio": base64::engine::general_purpose::STANDARD.encode(pcm_bytes(samples)) }).to_string().into())
    };
    let mut started = false;
    let mut pending: Vec<Vec<i16>> = Vec::new();
    let mut transcript = CodexTranscript::default();
    let mut closing: Option<Instant> = None;
    loop {
        let deadline = closing.unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));
        tokio::select! {
            chunk = audio.recv(), if closing.is_none() => if let Some(chunk) = chunk {
                if started { emit(VoiceEvent::Level { level: level(&chunk) }); }
                if started { socket.send(append(&chunk)).await.map_err(|e| e.to_string())?; } else { pending.push(chunk); }
            },
            command = control.recv(), if closing.is_none() => match command {
                Some(Control::Finish) => {
                    stop_capture();
                    if !started { let _ = socket.close(None).await; emit(VoiceEvent::Text { text: String::new(), done: true }); return Ok(()); }
                    while let Ok(chunk) = audio.try_recv() { let _ = socket.send(append(&chunk)).await; }
                    // A quarter second of silence lets the server's VAD close the last utterance.
                    let _ = socket.send(append(&vec![0i16; SAMPLE_RATE as usize / 4])).await;
                    socket.send(Message::Text(r#"{"type":"session.close"}"#.into())).await.map_err(|e| e.to_string())?;
                    closing = Some(Instant::now() + Duration::from_secs(8));
                }
                Some(Control::Cancel) | None => { let _ = socket.close(None).await; return Ok(()); }
            },
            _ = tokio::time::sleep_until(deadline), if closing.is_some() => break,
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };
                    match value["type"].as_str().unwrap_or("") {
                        "session.started" => {
                            started = true;
                            emit(VoiceEvent::Ready);
                            for chunk in pending.drain(..) { socket.send(append(&chunk)).await.map_err(|e| e.to_string())?; }
                        }
                        "session.updated" if value["session"]["status"] == "closed" => {
                            if closing.is_some() { break; }
                            return Err("closed".into());
                        }
                        _ => if transcript.apply(&value)? { emit(VoiceEvent::Text { text: transcript.text(), done: false }); },
                    }
                }
                Some(Ok(Message::Close(frame))) if closing.is_none() => {
                    return Err(format!("closed: {}", frame.map(|f| f.reason.to_string()).unwrap_or_default()));
                }
                None if closing.is_none() => return Err("closed".into()),
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(error)) => return Err(error.to_string()),
                _ => {}
            },
        }
    }
    let _ = socket.close(None).await;
    emit(VoiceEvent::Text { text: transcript.text(), done: true });
    Ok(())
}

/// The user's OpenAI-compatible endpoint: record everything, then send it as one WAV file.
async fn run_api(
    config: ApiConfig, language: &str, mut audio: mpsc::UnboundedReceiver<Vec<i16>>, mut control: mpsc::UnboundedReceiver<Control>,
    emit: &(dyn Fn(VoiceEvent) + Send + Sync), stop_capture: impl Fn() + Send,
) -> Result<(), String> {
    emit(VoiceEvent::Ready);
    let mut samples: Vec<i16> = Vec::new();
    loop {
        tokio::select! {
            chunk = audio.recv() => match chunk {
                Some(chunk) => { emit(VoiceEvent::Level { level: level(&chunk) }); samples.extend(chunk); }
                None => return Err("microphone_closed".into()),
            },
            command = control.recv() => match command {
                Some(Control::Finish) => {
                    stop_capture();
                    while let Ok(chunk) = audio.try_recv() { samples.extend(chunk); }
                    break;
                }
                Some(Control::Cancel) | None => return Ok(()),
            },
        }
    }
    // Under a quarter second is a slip of the finger, not speech.
    if samples.len() < SAMPLE_RATE as usize / 4 {
        emit(VoiceEvent::Text { text: String::new(), done: true });
        return Ok(());
    }
    emit(VoiceEvent::Processing);
    let language = language.to_string();
    let upload = tauri::async_runtime::spawn_blocking(move || transcribe(&config, &language, &wav(&samples)));
    let text = tokio::select! {
        result = upload => result.map_err(|e| e.to_string())??,
        // Cancelled while the service works on it.
        _ = control.recv() => return Ok(()),
    };
    emit(VoiceEvent::Text { text, done: true });
    Ok(())
}

/// 16 kHz mono PCM16 as a WAV file.
fn wav(samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    out.extend(pcm_bytes(samples));
    out
}

/// POST {base}/audio/transcriptions (multipart), as OpenAI, Groq, SiliconFlow and others accept it.
fn transcribe(config: &ApiConfig, language: &str, audio: &[u8]) -> Result<String, String> {
    let boundary = format!("wings{:x}", now_ms());
    let mut body = Vec::with_capacity(audio.len() + 512);
    let model = if config.model.is_empty() { API_MODEL } else { &config.model };
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
    };
    field("model", model);
    field("response_format", "json");
    if !language.is_empty() { field("language", language); }
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"voice.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
    body.extend_from_slice(audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let base = if config.base.is_empty() { API_BASE } else { &config.base };
    let response = ureq::post(&format!("{base}/audio/transcriptions"))
        .timeout(Duration::from_secs(60))
        .set("Authorization", &format!("Bearer {}", config.key))
        .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
        .send_bytes(&body);
    let value: Value = match response {
        Ok(response) => response.into_json().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(code, response)) => {
            let detail = response.into_json::<Value>().ok()
                .and_then(|v| v["error"]["message"].as_str().or(v["message"].as_str()).map(str::to_owned));
            return Err(match detail { Some(d) => format!("http_{code}: {d}"), None => format!("http_{code}") });
        }
        Err(error) => return Err(format!("connect: {error}")),
    };
    Ok(value["text"].as_str().unwrap_or("").trim().to_string())
}

/// Mono 16 kHz PCM16 from any input format: average the channels, then resample linearly.
pub struct Resampler { step: f64, position: f64, previous: f32, out: Vec<i16> }

impl Resampler {
    pub fn new(input_rate: u32) -> Self { Self { step: input_rate as f64 / SAMPLE_RATE as f64, position: 0.0, previous: 0.0, out: Vec::with_capacity(CHUNK) } }

    /// Feeds mono samples; returns whole 100 ms chunks.
    pub fn push(&mut self, mono: impl IntoIterator<Item = f32>) -> Vec<Vec<i16>> {
        let mut chunks = Vec::new();
        for sample in mono {
            // `position` is where the next output sample falls, relative to `previous` (0) and `sample` (1).
            while self.position <= 1.0 {
                let value = self.previous + (sample - self.previous) * self.position as f32;
                self.out.push((value.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                if self.out.len() == CHUNK { chunks.push(std::mem::replace(&mut self.out, Vec::with_capacity(CHUNK))); }
                self.position += self.step;
            }
            self.position -= 1.0;
            self.previous = sample;
        }
        chunks
    }

    pub fn flush(&mut self) -> Option<Vec<i16>> { (!self.out.is_empty()).then(|| std::mem::take(&mut self.out)) }
}

mod capture {
    use super::Resampler;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{mpsc as std_mpsc, Arc, Mutex};
    use tokio::sync::mpsc;

    /// The capture thread owns the stream (not `Send` on every platform); dropping the sender stops it.
    pub struct Capture { stop: Mutex<Option<(std_mpsc::Sender<()>, std_mpsc::Receiver<()>)>> }

    impl Capture {
        pub fn stop(&self) {
            let mut stop = self.stop.lock().unwrap();
            if let Some((signal, finished)) = stop.take() {
                drop(signal);
                // Finish drains the audio queue immediately after stop returns.
                let _ = finished.recv();
            }
        }
    }

    pub fn devices() -> super::Microphones {
        let host = cpal::default_host();
        #[allow(deprecated)]
        let devices = host.input_devices().map(|list| list.filter_map(|d| d.name().ok()).collect()).unwrap_or_default();
        #[allow(deprecated)]
        let default = host.default_input_device().and_then(|d| d.name().ok());
        super::Microphones { devices, default }
    }

    /// The named microphone when it is still connected, otherwise the system default.
    fn device(name: Option<&str>) -> Option<cpal::Device> {
        let host = cpal::default_host();
        #[allow(deprecated)]
        let named = name.filter(|n| !n.is_empty()).and_then(|name| host.input_devices().ok()?.find(|d| d.name().ok().as_deref() == Some(name)));
        named.or_else(|| host.default_input_device())
    }

    pub fn start(tx: mpsc::UnboundedSender<Vec<i16>>, name: Option<String>) -> Result<Capture, String> {
        let (stop_tx, stop_rx) = std_mpsc::channel::<()>();
        let (finished_tx, finished_rx) = std_mpsc::channel::<()>();
        let (ready_tx, ready_rx) = std_mpsc::channel::<Result<(), String>>();
        std::thread::Builder::new().name("voice-capture".into()).spawn(move || {
            let (stream, resampler) = match open(tx.clone(), name.as_deref()) {
                Ok(pair) => { let _ = ready_tx.send(Ok(())); pair }
                Err(error) => { let _ = ready_tx.send(Err(error)); return; }
            };
            let _ = stop_rx.recv();
            drop(stream);
            let rest = resampler.lock().unwrap().flush();
            if let Some(rest) = rest { let _ = tx.send(rest); }
            let _ = finished_tx.send(());
        }).map_err(|e| e.to_string())?;
        ready_rx.recv().map_err(|_| "microphone: capture thread ended".to_string())??;
        Ok(Capture { stop: Mutex::new(Some((stop_tx, finished_rx))) })
    }

    fn open(tx: mpsc::UnboundedSender<Vec<i16>>, name: Option<&str>) -> Result<(cpal::Stream, Arc<Mutex<Resampler>>), String> {
        let device = device(name).ok_or("no_microphone")?;
        let config = device.default_input_config().map_err(|e| format!("microphone: {e}"))?;
        let channels = config.channels().max(1) as usize;
        let resampler = Arc::new(Mutex::new(Resampler::new(config.sample_rate())));
        let format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        fn build<T: cpal::SizedSample>(device: &cpal::Device, config: &cpal::StreamConfig, channels: usize, resampler: Arc<Mutex<Resampler>>, tx: mpsc::UnboundedSender<Vec<i16>>) -> Result<cpal::Stream, cpal::BuildStreamError>
        where f32: cpal::FromSample<T> {
            device.build_input_stream(config, move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mono = data.chunks(channels).map(|frame| frame.iter().map(|s| f32::from_sample_(*s)).sum::<f32>() / channels as f32);
                for chunk in resampler.lock().unwrap().push(mono) { let _ = tx.send(chunk); }
            }, |error| eprintln!("voice capture: {error}"), None)
        }
        use cpal::FromSample;
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, &stream_config, channels, resampler.clone(), tx),
            cpal::SampleFormat::I16 => build::<i16>(&device, &stream_config, channels, resampler.clone(), tx),
            cpal::SampleFormat::U16 => build::<u16>(&device, &stream_config, channels, resampler.clone(), tx),
            cpal::SampleFormat::I32 => build::<i32>(&device, &stream_config, channels, resampler.clone(), tx),
            other => return Err(format!("microphone: unsupported sample format {other}")),
        }.map_err(|e| format!("microphone: {e}"))?;
        stream.play().map_err(|e| format!("microphone: {e}"))?;
        Ok((stream, resampler))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_latin_with_spaces_and_cjk_without() {
        assert_eq!(join(["hello".into(), "world".into()]), "hello world");
        assert_eq!(join(["你好".into(), "世界".into()]), "你好世界");
    }

    #[test]
    fn wav_header_is_44_bytes() {
        let out = wav(&[0, 1, 2]);
        assert_eq!(out.len(), 44 + 6);
        assert_eq!(&out[0..4], b"RIFF");
    }

    #[test]
    fn key_hint_never_shows_the_key() {
        assert_eq!(key_hint("sk-abcdefghijklmnop"), "sk-…mnop");
        assert_eq!(key_hint("short"), "…");
    }
}
