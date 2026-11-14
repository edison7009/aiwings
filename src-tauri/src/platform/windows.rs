// Windows: SetWinEventHook drives each bar, owned by its window; SendInput posts the shortcuts, guarded by a foreground check.

use super::{place_bar, Frame, Key};
use crate::TargetInfo;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, CloseHandle, HANDLE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetAncestor, GetForegroundWindow, GetMessageW, GetWindow, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostThreadMessageW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TranslateMessage, GA_ROOTOWNER, GWLP_HWNDPARENT, GW_OWNER,
    MSG, SW_RESTORE, SW_SHOWNOACTIVATE, WINEVENT_OUTOFCONTEXT, WM_QUIT,
};

const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_DESTROY: u32 = 0x8001;
const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;

/// The active window: where saves and pastes go.
static TARGET: AtomicIsize = AtomicIsize::new(0);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
/// Followed windows (hwnd) and the labels of the bars beside them.
static BARS: Mutex<Vec<(isize, String)>> = Mutex::new(Vec::new());

fn target() -> HWND { HWND(TARGET.load(Ordering::SeqCst) as *mut _) }

fn frame_of(h: HWND) -> Option<Frame> {
    let mut r = RECT::default();
    // Extended frame bounds exclude the invisible resize border that GetWindowRect includes.
    unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32) }.ok()?;
    Some(Frame { x: r.left as f64, y: r.top as f64, w: (r.right - r.left) as f64, h: (r.bottom - r.top) as f64 })
}

/// True when the foreground window is the target or one of its popups.
fn target_owns_foreground() -> bool {
    let t = target();
    if t.0.is_null() { return false; }
    let fg = unsafe { GetForegroundWindow() };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(fg, Some(&mut pid)); }
    // Wings is an owned window too, but must never receive the target's shortcuts.
    if pid == unsafe { GetCurrentProcessId() } { return false; }
    fg == t || unsafe { GetAncestor(fg, GA_ROOTOWNER) } == t
}

fn cloaked(h: HWND) -> bool {
    let mut cloaked = 0u32;
    let _ = unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4) };
    cloaked != 0
}

/// Places one bar beside its window. The bar is owned by that window, so Windows keeps it just above it in
/// z-order and minimizes it along with it. Hidden in hover mode, or while the window is minimized, hidden
/// or on another virtual desktop.
fn refresh_one(app: &AppHandle, t: HWND, label: &str) {
    let Some(bar) = app.get_webview_window(label) else { return };
    let up = !crate::bars::FLOAT.load(Ordering::SeqCst) && !crate::bars::HIDDEN.load(Ordering::SeqCst)
        && unsafe { IsWindow(t).as_bool() && IsWindowVisible(t).as_bool() && !IsIconic(t).as_bool() } && !cloaked(t);
    match up.then(|| frame_of(t)).flatten() {
        Some(f) => {
            let scale = bar.scale_factor().unwrap_or(1.0);
            place_bar(&bar, app, f, scale);
            show_no_activate(&bar);
        }
        None => { let _ = bar.hide(); }
    }
}

unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, _event: u32, hwnd: HWND, id_object: i32, _child: i32, _t: u32, _ms: u32) {
    // Only a followed window's own top-level object matters (OBJID_WINDOW = 0), not its carets or children.
    if id_object != 0 { return; }
    let label = BARS.lock().unwrap().iter().find(|(h, _)| *h == hwnd.0 as isize).map(|(_, l)| l.clone());
    if let (Some(label), Some(app)) = (label, APP.get()) { refresh_one(app, hwnd, &label); }
}

/// One hook thread for all followed windows, restarted whenever the set of processes changes.
/// Serialized, and each restart waits for the new thread's id, so no old thread is left running.
fn restart_hooks() {
    static RESTART: Mutex<()> = Mutex::new(());
    let _one_at_a_time = RESTART.lock().unwrap();
    let tid = HOOK_THREAD.swap(0, Ordering::SeqCst);
    if tid != 0 { unsafe { let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)); } }
    let mut pids: Vec<u32> = BARS.lock().unwrap().iter().map(|(h, _)| {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(HWND(*h as *mut _), Some(&mut pid)); }
        pid
    }).filter(|p| *p != 0).collect();
    pids.sort_unstable();
    pids.dedup();
    if pids.is_empty() { return; }
    let (ready, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || unsafe {
        let mut hooks = Vec::new();
        for pid in pids {
            hooks.push(SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT));
            hooks.push(SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT));
        }
        // The hooks gave this thread its message queue, so a WM_QUIT posted from now on is not lost.
        let _ = ready.send(GetCurrentThreadId());
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        for h in hooks { let _ = UnhookWinEvent(h); }
    });
    if let Ok(tid) = started.recv() { HOOK_THREAD.store(tid, Ordering::SeqCst); }
}

/// Starts following a window for its bar (the window `label`).
pub fn track(app: &AppHandle, t: &TargetInfo, label: &str) {
    let _ = APP.set(app.clone());
    let Ok(raw) = t.id.parse::<isize>() else { return };
    BARS.lock().unwrap().push((raw, label.to_string()));
    if let Some(bar) = app.get_webview_window(label) {
        let _ = bar.set_always_on_top(false);
        // Tauri links its own windows-crate version, so rebuild the handle from the raw pointer.
        if let Ok(h) = bar.hwnd() { unsafe { SetWindowLongPtrW(HWND(h.0 as *mut _), GWLP_HWNDPARENT, raw); } }
    }
    refresh_one(app, HWND(raw as *mut _), label);
    restart_hooks();
}

pub fn untrack(id: &str) {
    let Ok(raw) = id.parse::<isize>() else { return };
    BARS.lock().unwrap().retain(|(h, _)| *h != raw);
    restart_hooks();
}

/// Re-places every bar (after a mode switch).
pub fn refresh_later(app: &AppHandle) {
    let bars = BARS.lock().unwrap().clone();
    for (h, label) in bars { refresh_one(app, HWND(h as *mut _), &label); }
}

/// Makes `t` the window that saves and pastes go to.
pub fn set_active(t: &TargetInfo) {
    let Ok(raw) = t.id.parse::<isize>() else { return };
    super::set_target_key(&t.key);
    TARGET.store(raw, Ordering::SeqCst);
}

pub fn active_id() -> String { TARGET.load(Ordering::SeqCst).to_string() }

pub fn has_target() -> bool { !target().0.is_null() }

pub fn target_is_front() -> bool { target_owns_foreground() }

pub fn activate_target() -> bool {
    let t = target();
    if t.0.is_null() || !unsafe { IsWindow(t).as_bool() } { return false; }
    unsafe {
        if IsIconic(t).as_bool() { let _ = ShowWindow(t, SW_RESTORE); }
        // Wings was just clicked, so it owns the foreground and is allowed to hand it over.
        let _ = SetForegroundWindow(t);
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1000) {
        if target_owns_foreground() { std::thread::sleep(Duration::from_millis(80)); return true; }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

pub fn wait_modifiers_released() {
    // A synthetic Alt-up would pop the target's menu bar, so wait for the real keys instead.
    let keys = [VK_MENU, VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN];
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1500) {
        if keys.iter().all(|k| unsafe { GetAsyncKeyState(k.0 as i32) } as u16 & 0x8000 == 0) { return; }
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, time: 0, dwExtraInfo: 0 } },
    }
}

fn process_is_elevated(pid: u32) -> Option<bool> {
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut token = HANDLE::default();
        let opened = OpenProcessToken(process, TOKEN_QUERY, &mut token);
        let _ = CloseHandle(process);
        opened.ok()?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0;
        let read = GetTokenInformation(token, TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut returned);
        let _ = CloseHandle(token);
        read.ok()?;
        Some(elevation.TokenIsElevated != 0)
    }
}

pub fn send_combo(key: Key) -> Result<(), String> {
    // Focus guard before every keystroke: Windows focus is easy to lose mid-sequence.
    if !target_owns_foreground() { return Err("目标窗口不在最前面，已取消".into()); }
    let mut foreground_pid = 0;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid)); }
    if process_is_elevated(foreground_pid) == Some(true)
        && process_is_elevated(unsafe { GetCurrentProcessId() }) == Some(false) {
        let is_powershell = unsafe { info_of(target()) }.is_some_and(|t|
            matches!(t.key.as_str(), "powershell.exe" | "pwsh.exe")
                || (super::is_terminal(&t.key) && t.name.to_ascii_lowercase().contains("powershell")));
        if is_powershell {
            return Err("发送到 PowerShell 需要管理员权限，右键 Wings 以管理员身份运行或右键「属性」→「兼容性」→ 勾选「以管理员身份运行此程序」".into());
        }
        return Err("目标以管理员身份运行，当前 Wings 权限不足。请用普通权限打开目标，或手动以管理员身份启动 Wings".into());
    }
    let vk = VIRTUAL_KEY(match key { Key::A => 0x41, Key::X => 0x58, Key::V => 0x56, Key::Delete => 0x08 });
    let inputs: Vec<INPUT> = if matches!(key, Key::Delete) {
        vec![key_input(vk, false), key_input(vk, true)]
    } else {
        vec![key_input(VK_CONTROL, false), key_input(vk, false), key_input(vk, true), key_input(VK_CONTROL, true)]
    };
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if (sent as usize) < inputs.len() {
        // SendInput's return value alone does not identify UIPI as the failure cause.
        return Err("模拟输入未成功，请检查目标窗口和权限".into());
    }
    Ok(())
}

pub fn accessibility_trusted(_prompt: bool) -> bool { true }

pub fn open_accessibility_settings() {}

fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(path.rsplit('\\').next()?.to_lowercase())
    }
}

unsafe extern "system" fn collect(h: HWND, lp: LPARAM) -> BOOL {
    let out = &mut *(lp.0 as *mut Vec<TargetInfo>);
    if let Some(t) = info_of(h) { out.push(t); }
    true.into()
}

unsafe fn info_of(h: HWND) -> Option<TargetInfo> {
    if !IsWindowVisible(h).as_bool() || !GetWindow(h, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true) { return None; }
    let mut cloaked = 0u32;
    let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
    if cloaked != 0 { return None; }
    let len = GetWindowTextLengthW(h);
    if len == 0 { return None; }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(h, &mut buf);
    let title = String::from_utf16_lossy(&buf[..n as usize]);
    let mut pid = 0u32;
    GetWindowThreadProcessId(h, Some(&mut pid));
    if pid == GetCurrentProcessId() { return None; }
    let exe = exe_name(pid)?;
    if exe == "explorer.exe" { return None; }
    Some(TargetInfo { id: (h.0 as isize).to_string(), key: exe, name: title })
}

pub fn list_targets() -> Vec<TargetInfo> {
    let mut out: Vec<TargetInfo> = Vec::new();
    unsafe { let _ = EnumWindows(Some(collect), LPARAM(&mut out as *mut _ as isize)); }
    crate::platform::sort_targets(&mut out);
    out
}

pub fn show_no_activate(win: &tauri::WebviewWindow) {
    if let Ok(h) = win.hwnd() { unsafe { let _ = ShowWindow(HWND(h.0 as *mut _), SW_SHOWNOACTIVATE); } }
}

pub fn front_app() -> Option<TargetInfo> {
    unsafe { info_of(GetAncestor(GetForegroundWindow(), GA_ROOTOWNER)) }
}

enum InputRequest {
    Inspect(std::sync::mpsc::Sender<Option<(Frame, bool)>>),
    Focus(std::sync::mpsc::Sender<bool>),
}

// UIA elements stay on one long-lived MTA thread, including SetFocus and identity checks.
// Bounded requests keep a stalled accessibility provider from building an unbounded queue.
fn input_worker() -> &'static std::sync::mpsc::SyncSender<InputRequest> {
    static WORKER: OnceLock<std::sync::mpsc::SyncSender<InputRequest>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || unsafe {
            use windows::Win32::System::Com::*;
            use windows::Win32::UI::Accessibility::*;
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() { return; }
            let uia: Result<IUIAutomation, _> = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);
            if let Ok(uia) = uia {
                // One remembered composer per window, so switching apps and back still finds where to paste.
                let mut remembered: Vec<(isize, IUIAutomationElement)> = Vec::new();
                for request in rx {
                    match request {
                        InputRequest::Inspect(reply) => {
                            let result = inspect_input(&uia).map(|(element, frame, has_text)| {
                                let hwnd = TARGET.load(Ordering::SeqCst);
                                remembered.retain(|(h, _)| *h != hwnd && IsWindow(HWND(*h as *mut _)).as_bool());
                                remembered.push((hwnd, element));
                                (frame, has_text)
                            });
                            let _ = reply.send(result);
                        }
                        InputRequest::Focus(reply) => {
                            let ok = (|| {
                                if !target_owns_foreground() { return None; }
                                let hwnd = TARGET.load(Ordering::SeqCst);
                                let (_, el) = remembered.iter().find(|(h, _)| *h == hwnd)?;
                                if !belongs_to_target(&uia, el) { return None; }
                                // Never replace the remembered composer with an arbitrary editor's caret.
                                input_value(el)?;
                                el.SetFocus().ok()?;
                                std::thread::sleep(Duration::from_millis(60));
                                let focused = uia.GetFocusedElement().ok()?;
                                uia.CompareElements(el, &focused).ok()?.as_bool().then_some(())
                            })().is_some();
                            let _ = reply.send(ok);
                        }
                    }
                }
            }
            CoUninitialize();
        });
        tx
    })
}

unsafe fn belongs_to_target(uia: &windows::Win32::UI::Accessibility::IUIAutomation,
    el: &windows::Win32::UI::Accessibility::IUIAutomationElement) -> bool {
    let Ok(walker) = uia.ControlViewWalker() else { return false };
    let mut cur = el.clone();
    for _ in 0..32 {
        if cur.CurrentNativeWindowHandle().ok() == Some(target()) { return true; }
        let Ok(parent) = walker.GetParentElement(&cur) else { return false };
        cur = parent;
    }
    false
}

unsafe fn input_value(el: &windows::Win32::UI::Accessibility::IUIAutomationElement) -> Option<String> {
    use windows::Win32::UI::Accessibility::*;
    if !el.CurrentIsEnabled().ok()?.as_bool() || el.CurrentIsOffscreen().ok()?.as_bool()
        || el.CurrentIsPassword().ok()?.as_bool() { return None; }
    let kind = el.CurrentControlType().ok()?;
    let hints = format!("{} {} {}", el.CurrentName().ok()?, el.CurrentAutomationId().ok()?, el.CurrentHelpText().ok()?);
    let key = super::TARGET_KEY.lock().unwrap().clone();
    let named_chat = super::apps::is_chat_input_for_app(&key, true, &hints);
    let minimax = key == "minimax code.exe";
    let rich_group = kind == UIA_GroupControlTypeId && minimax && named_chat;
    if kind != UIA_EditControlTypeId && !(kind == UIA_ComboBoxControlTypeId && named_chat) && !rich_group { return None; }
    if !super::apps::is_chat_input_for_app(&key, super::target_is_ide(), &hints) { return None; }
    if let Ok(value) = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
        if value.CurrentIsReadOnly().ok()?.as_bool() { return None; }
        return Some(value.CurrentValue().ok()?.to_string());
    }
    // MiniMax's named rich-text group exposes text rather than ValuePattern.
    // Do not use this path for generic groups, document bodies, or read-only text.
    if !rich_group || !el.CurrentIsKeyboardFocusable().ok()?.as_bool() { return None; }
    let pattern: IUIAutomationTextPattern = el.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
    let range = pattern.DocumentRange().ok()?;
    if bool::try_from(&range.GetAttributeValue(UIA_IsReadOnlyAttributeId).ok()?).ok()? { return None; }
    Some(range.GetText(-1).ok()?.to_string())
}

unsafe fn inspect_input(uia: &windows::Win32::UI::Accessibility::IUIAutomation)
    -> Option<(windows::Win32::UI::Accessibility::IUIAutomationElement, Frame, bool)> {
    if !target_owns_foreground() { return None; }
    let el = uia.GetFocusedElement().ok()?;
    if !belongs_to_target(uia, &el) { return None; }
    let text = input_value(&el)?;
    let r = el.CurrentBoundingRectangle().ok()?;
    if r.right <= r.left || r.bottom <= r.top { return None; }
    let scale = APP.get().and_then(|a| a.get_webview_window("main"))
        .and_then(|w| w.scale_factor().ok()).unwrap_or(1.0);
    let prefer_inner = super::target_prefers_inner_box();
    let text_frame = Frame { x: r.left as f64, y: r.top as f64,
        w: (r.right - r.left) as f64, h: (r.bottom - r.top) as f64 };
    let key = super::TARGET_KEY.lock().unwrap().clone();
    let clipped_input = matches!(key.as_str(), "qwenworkcn.exe" | "kimi.exe");
    let min_padding = if super::apps::is_qoder(&key) { 2.0 } else { 8.0 };
    let mut best = r;
    let walker = uia.RawViewWalker().ok()?;
    let mut cur = el.clone();
    for _ in 0..6 {
        let Ok(parent) = walker.GetParentElement(&cur) else { break };
        cur = parent.clone();
        let Ok(f) = parent.CurrentBoundingRectangle() else { continue };
        if f.right <= f.left || f.bottom <= f.top { continue; }
        // Qwen/Kimi have a clipping wrapper slightly narrower than the editable child.
        // It is not the painted composer; keep climbing rather than stopping here.
        if clipped_input && f.left == r.left && f.top == r.top && f.bottom == r.bottom
            && f.right < r.right && r.right - f.right <= (16.0 * scale).ceil() as i32 { continue; }
        let width_slack = super::apps::input_width_slack(&key,
            (r.bottom - r.top) as f64, (f.bottom - f.top) as f64, scale);
        if f.left > r.left + 1 || f.top > r.top + 1 || f.right < r.right - 1
            || f.bottom < r.bottom - 1 || f.right - f.left > r.right - r.left + (width_slack * scale) as i32
            || f.bottom > r.bottom + (140.0 * scale) as i32
            || f.bottom - f.top > r.bottom - r.top + (420.0 * scale) as i32 { break; }
        best = f;
        let candidate = Frame { x: f.left as f64, y: f.top as f64,
            w: (f.right - f.left) as f64, h: (f.bottom - f.top) as f64 };
        if prefer_inner && super::input_box_has_toolbar(text_frame, candidate, scale, min_padding) { break; }
    }
    Some((el, Frame { x: best.left as f64, y: best.top as f64,
        w: (best.right - best.left) as f64, h: (best.bottom - best.top) as f64 }, super::apps::has_input_text(&text, None)))
}

pub fn focused_input() -> Option<(Frame, bool)> {
    let (tx, rx) = std::sync::mpsc::channel();
    input_worker().try_send(InputRequest::Inspect(tx)).ok()?;
    rx.recv_timeout(Duration::from_millis(600)).ok().flatten()
}

pub fn focus_input() -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    if input_worker().try_send(InputRequest::Focus(tx)).is_err() { return false; }
    rx.recv_timeout(Duration::from_millis(600)).unwrap_or(false)
}

#[cfg(test)]
mod token_tests {
    use super::*;

    #[test]
    fn own_elevation_token_is_readable() {
        assert!(process_is_elevated(unsafe { GetCurrentProcessId() }).is_some());
    }
}
