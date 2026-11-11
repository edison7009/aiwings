// Windows: SetWinEventHook drives docking; SendInput posts the shortcuts, guarded by a foreground check.

use super::{float_panel, place_bar, Frame, Key};
use crate::TargetInfo;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
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

const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_DESTROY: u32 = 0x8001;
const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;

static TARGET: AtomicIsize = AtomicIsize::new(0);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
static FLOAT: AtomicBool = AtomicBool::new(false);

fn target() -> HWND { HWND(TARGET.load(Ordering::SeqCst) as *mut _) }

fn wings_hwnd(app: &AppHandle) -> Option<HWND> {
    // Tauri links its own windows-crate version, so rebuild the handle from the raw pointer.
    app.get_webview_window("main")?.hwnd().ok().map(|h| HWND(h.0 as *mut _))
}

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
    fg == t || unsafe { GetAncestor(fg, GA_ROOTOWNER) } == t
}

fn refresh() {
    // Hover mode: the bar stays wherever the user put it.
    if FLOAT.load(Ordering::SeqCst) { return; }
    let Some(app) = APP.get() else { return };
    let Some(win) = app.get_webview_window("main") else { return };
    let t = target();
    let alive = !t.0.is_null() && unsafe { IsWindow(t).as_bool() && !IsIconic(t).as_bool() };
    let fg = unsafe { GetForegroundWindow() };
    let ours = wings_hwnd(app);
    let front = target_owns_foreground() || Some(fg) == ours;
    match (alive && front).then(|| frame_of(t)).flatten() {
        Some(f) => {
            let scale = win.scale_factor().unwrap_or(1.0);
            place_bar(app, f, scale);
            show_no_activate(&win);
        }
        None => { let _ = win.hide(); }
    }
}

unsafe extern "system" fn on_event(_h: HWINEVENTHOOK, event: u32, hwnd: HWND, id_object: i32, _child: i32, _t: u32, _ms: u32) {
    // Location/minimize/destroy only matter for the target's own top-level window (OBJID_WINDOW = 0).
    if event != EVENT_SYSTEM_FOREGROUND && (hwnd != target() || id_object != 0) { return; }
    if event == EVENT_OBJECT_DESTROY { TARGET.store(0, Ordering::SeqCst); }
    refresh();
}

fn stop_hooks() {
    let tid = HOOK_THREAD.swap(0, Ordering::SeqCst);
    if tid != 0 { unsafe { let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)); } }
}

pub fn dock(app: AppHandle, id: &str) -> Result<(), String> {
    let _ = APP.set(app.clone());
    let raw: isize = id.parse().map_err(|_| "无效的目标".to_string())?;
    let t = HWND(raw as *mut _);
    if !unsafe { IsWindow(t).as_bool() } { return Err("这个窗口已经关闭了".into()); }
    stop_hooks();
    TARGET.store(raw, Ordering::SeqCst);
    if !FLOAT.load(Ordering::SeqCst) { own_by_target(&app, true); }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(t, Some(&mut pid)); }
    std::thread::spawn(move || unsafe {
        HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
        let fg = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT);
        let mm = SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT);
        let obj = SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT);
        refresh();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        for h in [fg, mm, obj] { let _ = UnhookWinEvent(h); }
    });
    Ok(())
}

pub fn undock(app: &AppHandle) {
    stop_hooks();
    TARGET.store(0, Ordering::SeqCst);
    own_by_target(app, false);
    float_panel(app);
}

pub fn refresh_later(_app: &AppHandle) {
    // The global foreground hook already reports every focus change.
}

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

pub fn send_combo(key: Key) -> Result<(), String> {
    // Focus guard before every keystroke: Windows focus is easy to lose mid-sequence.
    if !target_owns_foreground() { return Err("目标窗口不在最前面，已取消".into()); }
    let vk = VIRTUAL_KEY(match key { Key::A => 0x41, Key::X => 0x58, Key::V => 0x56, Key::Delete => 0x08 });
    let inputs: Vec<INPUT> = if matches!(key, Key::Delete) {
        vec![key_input(vk, false), key_input(vk, true)]
    } else {
        vec![key_input(VK_CONTROL, false), key_input(vk, false), key_input(vk, true), key_input(VK_CONTROL, true)]
    };
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if (sent as usize) < inputs.len() {
        // UIPI: an elevated target silently rejects input from a normal process.
        return Err("目标应用以管理员身份运行，Wings 无法向它输入。请用普通方式打开它".into());
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

/// Owned by the target, Wings sits just above it in z-order; unowned, it floats above everything.
fn own_by_target(app: &AppHandle, owned: bool) {
    let Some(w) = app.get_webview_window("main") else { return };
    let _ = w.set_always_on_top(!owned);
    if let Some(h) = wings_hwnd(app) {
        unsafe { SetWindowLongPtrW(h, GWLP_HWNDPARENT, if owned { TARGET.load(Ordering::SeqCst) } else { 0 }); }
    }
}

pub fn show_no_activate(win: &tauri::WebviewWindow) {
    if let Ok(h) = win.hwnd() { unsafe { let _ = ShowWindow(HWND(h.0 as *mut _), SW_SHOWNOACTIVATE); } }
}

pub fn set_float(app: &AppHandle, float: bool) {
    FLOAT.store(float, Ordering::SeqCst);
    let owned = !float && has_target();
    own_by_target(app, owned);
    if let Some(w) = app.get_webview_window("main") { show_no_activate(&w); }
    if !float { refresh(); }
}

pub fn current_id() -> String { (TARGET.load(Ordering::SeqCst)).to_string() }

pub fn front_app() -> Option<TargetInfo> {
    unsafe { info_of(GetAncestor(GetForegroundWindow(), GA_ROOTOWNER)) }
}

/// Input box detection is not adapted on Windows yet (UI Automation goes here).
pub fn focused_input() -> Option<(Frame, bool)> { None }

/// Not adapted on Windows yet: the paste goes wherever the target's caret already is.
pub fn focus_input() {}
