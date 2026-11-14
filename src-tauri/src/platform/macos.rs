// macOS: one AXObserver per followed app drives its bar; CGEvent posts the synthetic shortcuts; NSWorkspace handles activation.

use super::{place_bar, Frame, Key};
use crate::TargetInfo;
use core_foundation::base::{CFEqual, CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::number::{CFBooleanGetValue, CFBooleanRef};
use core_foundation_sys::runloop::{kCFRunLoopDefaultMode, CFRunLoopAddSource, CFRunLoopGetMain, CFRunLoopRemoveSource, CFRunLoopSourceRef};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication, NSWindow, NSWindowOrderingMode, NSWorkspace};
use std::ffi::c_void;
use std::ptr::null;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

type AXUIElementRef = CFTypeRef;
type AXObserverRef = CFTypeRef;
type AXCallback = unsafe extern "C" fn(AXObserverRef, AXUIElementRef, CFStringRef, *mut c_void);

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementSetMessagingTimeout(el: AXUIElementRef, seconds: f32) -> i32;
    fn AXUIElementSetAttributeValue(el: AXUIElementRef, attr: CFStringRef, value: CFTypeRef) -> i32;
    fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFStringRef, out: *mut CFTypeRef) -> i32;
    fn AXValueGetValue(value: CFTypeRef, ty: u32, out: *mut c_void) -> bool;
    fn AXObserverCreate(pid: i32, cb: AXCallback, out: *mut AXObserverRef) -> i32;
    fn AXObserverAddNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef, refcon: *mut c_void) -> i32;
    fn AXObserverRemoveNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef) -> i32;
    fn AXObserverGetRunLoopSource(o: AXObserverRef) -> CFRunLoopSourceRef;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGWarpMouseCursorPosition(p: core_graphics::geometry::CGPoint) -> i32;
    /// Private but long-stable (window managers rely on it): the window-server id of an AX window.
    fn _AXUIElementGetWindow(el: AXUIElementRef, out: *mut u32) -> i32;
}

const APP_NOTES: &[&str] = &[
    "AXMainWindowChanged", "AXFocusedWindowChanged", "AXWindowCreated",
    "AXApplicationActivated", "AXApplicationDeactivated", "AXApplicationHidden", "AXApplicationShown",
];
const WINDOW_NOTES: &[&str] = &["AXMoved", "AXResized", "AXWindowMiniaturized", "AXWindowDeminiaturized", "AXUIElementDestroyed"];
const REATTACH: &[&str] = &["AXMainWindowChanged", "AXFocusedWindowChanged", "AXWindowCreated", "AXUIElementDestroyed"];

/// One followed app: its observer, and the window its bar sits beside.
struct Watched {
    pid: i32,
    app_el: AXUIElementRef,
    observer: AXObserverRef,
    /// The followed window (owned +1), or null when the app has none.
    window: AXUIElementRef,
    /// Label of the bar window beside it.
    label: String,
}
// Observer setup, teardown and refreshes happen on the main thread; AX element reads are thread-safe.
unsafe impl Send for Watched {}

/// An app saves and pastes have gone to, with the input box last used in it.
struct Active {
    pid: i32,
    app_el: AXUIElementRef,
    /// The last text input the user focused in it (owned +1), or null. Paste puts the caret back here.
    input: AXUIElementRef,
}
unsafe impl Send for Active {}

static WATCHED: Mutex<Vec<Watched>> = Mutex::new(Vec::new());
/// Every app made active so far (closed ones are dropped by `untrack`), so each keeps its own remembered
/// input box: switching to another app and back must not forget where to paste. TARGET_PID picks the current one.
static ACTIVE: Mutex<Vec<Active>> = Mutex::new(Vec::new());
static TARGET_PID: AtomicI32 = AtomicI32::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();

fn cfs(s: &str) -> CFString { CFString::new(s) }

unsafe fn copy_attr(el: AXUIElementRef, attr: &str) -> Option<CFTypeRef> {
    let mut out: CFTypeRef = null();
    let name = cfs(attr);
    (AXUIElementCopyAttributeValue(el, name.as_concrete_TypeRef(), &mut out) == 0 && !out.is_null()).then_some(out)
}

#[repr(C)]
#[derive(Default)]
struct Pair { a: f64, b: f64 }

unsafe fn read_pair(el: AXUIElementRef, attr: &str, ty: u32) -> Option<Pair> {
    let v = copy_attr(el, attr)?;
    let mut p = Pair::default();
    let ok = AXValueGetValue(v, ty, &mut p as *mut Pair as *mut c_void);
    CFRelease(v);
    ok.then_some(p)
}

unsafe fn is_minimized(win: AXUIElementRef) -> bool {
    match copy_attr(win, "AXMinimized") {
        Some(v) => { let m = CFBooleanGetValue(v as CFBooleanRef); CFRelease(v); m }
        None => false,
    }
}

fn front_pid() -> i32 {
    NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| a.processIdentifier()).unwrap_or(0)
}

/// An app's AX element. Electron/Chromium apps only build their AX tree when asked to.
unsafe fn app_element(pid: i32) -> AXUIElementRef {
    let app_el = AXUIElementCreateApplication(pid);
    // A hung app would otherwise block each AX read for the 6 s default, and the main thread with it.
    AXUIElementSetMessagingTimeout(app_el, 0.5);
    // Claude honours AXManualAccessibility; the ChatGPT app rejects it as unsupported (-25205) and needs
    // AXEnhancedUserInterface instead.
    let yes = CFBoolean::true_value();
    if AXUIElementSetAttributeValue(app_el, cfs("AXManualAccessibility").as_concrete_TypeRef(), yes.as_CFTypeRef()) != 0 {
        AXUIElementSetAttributeValue(app_el, cfs("AXEnhancedUserInterface").as_concrete_TypeRef(), yes.as_CFTypeRef());
    }
    app_el
}

fn refcon(pid: i32) -> *mut c_void { pid as isize as *mut c_void }

unsafe fn attach_window(w: &mut Watched) {
    if !w.window.is_null() {
        for n in WINDOW_NOTES { AXObserverRemoveNotification(w.observer, w.window, cfs(n).as_concrete_TypeRef()); }
        CFRelease(w.window);
        w.window = null();
    }
    if let Some(win) = copy_attr(w.app_el, "AXMainWindow").or_else(|| copy_attr(w.app_el, "AXFocusedWindow")) {
        for n in WINDOW_NOTES { AXObserverAddNotification(w.observer, win, cfs(n).as_concrete_TypeRef(), refcon(w.pid)); }
        w.window = win;
    }
}

fn app_hidden(pid: i32) -> bool {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid).is_some_and(|a| a.isHidden())
}

/// Places one bar beside its app's window and orders it just above that window, so whatever covers the
/// app covers the bar too, and the bar never floats over other apps. Hidden in hover mode, while the app is
/// hidden, or while its window is minimized. Main thread only (AppKit).
unsafe fn refresh_one(app: &AppHandle, w: &Watched) {
    let Some(bar) = app.get_webview_window(&w.label) else { return };
    let up = !crate::bars::FLOAT.load(Ordering::SeqCst) && !w.window.is_null() && !is_minimized(w.window) && !app_hidden(w.pid);
    let frame = up.then(|| Some((read_pair(w.window, "AXPosition", 1)?, read_pair(w.window, "AXSize", 2)?))).flatten();
    let Some((pos, size)) = frame else { let _ = bar.hide(); return };
    let s = bar.scale_factor().unwrap_or(1.0);
    place_bar(&bar, app, Frame { x: pos.a * s, y: pos.b * s, w: size.a * s, h: size.b * s }, s);
    let mut wid = 0u32;
    let known = _AXUIElementGetWindow(w.window, &mut wid) == 0 && wid != 0;
    if let Ok(ns) = bar.ns_window() {
        let ns = &*(ns as *const NSWindow);
        if known { ns.orderWindow_relativeTo(NSWindowOrderingMode::Above, wid as isize); } else { ns.orderFrontRegardless(); }
    }
}

unsafe extern "C" fn on_ax(_o: AXObserverRef, _el: AXUIElementRef, n: CFStringRef, refcon: *mut c_void) {
    let name = CFString::wrap_under_get_rule(n).to_string();
    let pid = refcon as isize as i32;
    let Some(app) = APP.get() else { return };
    let mut watched = WATCHED.lock().unwrap();
    let Some(w) = watched.iter_mut().find(|w| w.pid == pid) else { return };
    if REATTACH.contains(&name.as_str()) { attach_window(w); }
    refresh_one(app, w);
}

unsafe fn teardown(w: Watched) {
    let src = AXObserverGetRunLoopSource(w.observer);
    CFRunLoopRemoveSource(CFRunLoopGetMain(), src, kCFRunLoopDefaultMode);
    if !w.window.is_null() { CFRelease(w.window); }
    CFRelease(w.observer);
    CFRelease(w.app_el);
}

/// Starts following an app for its bar (the window `label`).
pub fn track(app: &AppHandle, t: &TargetInfo, label: &str) {
    let _ = APP.set(app.clone());
    let Ok(pid) = t.id.parse::<i32>() else { return };
    let (label, handle) = (label.to_string(), app.clone());
    let _ = app.run_on_main_thread(move || unsafe {
        let app_el = app_element(pid);
        let mut observer: AXObserverRef = null();
        if AXObserverCreate(pid, on_ax, &mut observer) != 0 || observer.is_null() {
            CFRelease(app_el);
            return;
        }
        for n in APP_NOTES { AXObserverAddNotification(observer, app_el, cfs(n).as_concrete_TypeRef(), refcon(pid)); }
        CFRunLoopAddSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(observer), kCFRunLoopDefaultMode);
        let mut w = Watched { pid, app_el, observer, window: null(), label };
        attach_window(&mut w);
        refresh_one(&handle, &w);
        WATCHED.lock().unwrap().push(w);
    });
}

pub fn untrack(id: &str) {
    let Ok(pid) = id.parse::<i32>() else { return };
    let Some(app) = APP.get() else { return };
    let _ = app.run_on_main_thread(move || {
        let mut watched = WATCHED.lock().unwrap();
        if let Some(i) = watched.iter().position(|w| w.pid == pid) { unsafe { teardown(watched.remove(i)) } }
    });
    let mut all = ACTIVE.lock().unwrap();
    if let Some(i) = all.iter().position(|a| a.pid == pid) {
        let a = all.remove(i);
        unsafe { if !a.input.is_null() { CFRelease(a.input); } CFRelease(a.app_el); }
    }
}

/// Re-places every bar: after a mode switch, and when Wings itself loses focus (clicking a bar can bring it
/// above other apps' windows).
pub fn refresh_later(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        // Let the new frontmost app settle before reading it.
        std::thread::sleep(Duration::from_millis(150));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            for w in WATCHED.lock().unwrap().iter() { unsafe { refresh_one(&handle, w) } }
        });
    });
}

/// Makes `t` the app that saves and pastes go to.
pub fn set_active(t: &TargetInfo) {
    let Ok(pid) = t.id.parse::<i32>() else { return };
    super::set_target_key(&t.key);
    let mut all = ACTIVE.lock().unwrap();
    if !all.iter().any(|a| a.pid == pid) {
        all.push(Active { pid, app_el: unsafe { app_element(pid) }, input: null() });
    }
    TARGET_PID.store(pid, Ordering::SeqCst);
}

/// The current app's entry.
fn current(all: &mut [Active]) -> Option<&mut Active> {
    let pid = TARGET_PID.load(Ordering::SeqCst);
    all.iter_mut().find(|a| a.pid == pid)
}

pub fn active_id() -> String { TARGET_PID.load(Ordering::SeqCst).to_string() }

pub fn has_target() -> bool { TARGET_PID.load(Ordering::SeqCst) != 0 }

pub fn target_is_front() -> bool {
    let pid = TARGET_PID.load(Ordering::SeqCst);
    pid != 0 && front_pid() == pid
}

pub fn activate_target() -> bool {
    let pid = TARGET_PID.load(Ordering::SeqCst);
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else { return false };
    #[allow(deprecated)]
    app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1000) {
        if target_is_front() { std::thread::sleep(Duration::from_millis(80)); return true; }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

pub fn wait_modifiers_released() {
    // Option/Shift/Control/Command still held from the hotkey would combine with our keys.
    const MASK: u64 = 0x80000 | 0x20000 | 0x40000 | 0x100000;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1000) {
        if unsafe { CGEventSourceFlagsState(1) } & MASK == 0 { return; }
        std::thread::sleep(Duration::from_millis(15));
    }
}

pub fn send_combo(key: Key) -> Result<(), String> {
    if !unsafe { AXIsProcessTrusted() } { return Err("需要先授予辅助功能权限".into()); }
    if !target_is_front() { return Err("目标窗口不在最前面，已取消".into()); }
    let code: u16 = match key { Key::A => 0, Key::X => 7, Key::V => 9, Key::Delete => 51 };
    let flags = if matches!(key, Key::Delete) { CGEventFlags::CGEventFlagNull } else { CGEventFlags::CGEventFlagCommand };
    let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| "无法模拟按键")?;
    for down in [true, false] {
        let ev = CGEvent::new_keyboard_event(src.clone(), code, down).map_err(|_| "无法模拟按键")?;
        ev.set_flags(flags);
        ev.post(CGEventTapLocation::HID);
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub fn accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt { return AXIsProcessTrusted(); }
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let opts = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef() as CFTypeRef)
    }
}

pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

pub fn list_targets() -> Vec<TargetInfo> {
    let ours = std::process::id() as i32;
    let apps = NSWorkspace::sharedWorkspace().runningApplications();
    let mut out: Vec<TargetInfo> = apps.iter()
        .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular && a.processIdentifier() != ours)
        .filter_map(|a| Some(TargetInfo {
            id: a.processIdentifier().to_string(),
            key: a.bundleIdentifier()?.to_string(),
            name: a.localizedName().map(|n| n.to_string()).unwrap_or_default(),
        }))
        .filter(|t| t.key != "com.apple.finder")
        .collect();
    crate::platform::sort_targets(&mut out);
    out
}

/// Shows a window without activating Wings, so the target keeps keyboard focus.
/// Hops to the main thread: AppKit/WebKit abort the process when a window is ordered from a background thread.
pub fn show_no_activate(win: &tauri::WebviewWindow) {
    let w = win.clone();
    let _ = win.run_on_main_thread(move || {
        if let Ok(ns) = w.ns_window() {
            unsafe { (*(ns as *const NSWindow)).orderFrontRegardless(); }
        }
    });
}

pub fn front_app() -> Option<TargetInfo> {
    let a = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(TargetInfo {
        id: a.processIdentifier().to_string(),
        key: a.bundleIdentifier()?.to_string(),
        name: a.localizedName().map(|n| n.to_string()).unwrap_or_default(),
    })
}

/// The target's focused text input: the frame (points) of the box drawn around it, and whether it holds any text.
/// The text is looked at only to tell empty from not; it is never kept or sent anywhere.
pub fn focused_input() -> Option<(Frame, bool)> {
    let mut all = ACTIVE.lock().unwrap();
    let d = current(&mut all)?;
    unsafe {
        let el = copy_attr(d.app_el, "AXFocusedUIElement")?;
        let role = copy_attr(el, "AXRole").map(|r| CFString::wrap_under_create_rule(r as CFStringRef).to_string());
        let editable = editable_input(el, role.as_deref());
        let frame = if editable { frame_of(el) } else { None };
        if editable && el != d.input {
            // Remember it, so a paste can put the caret back after focus wandered off.
            if !d.input.is_null() { CFRelease(d.input); }
            d.input = CFRetain(el);
        }
        let has_text = has_real_text(el);
        let frame = frame.map(|f| input_box(el, f));
        CFRelease(el);
        Some((frame?, has_text))
    }
}

unsafe fn frame_of(el: AXUIElementRef) -> Option<Frame> {
    let (pos, size) = (read_pair(el, "AXPosition", 1)?, read_pair(el, "AXSize", 2)?);
    Some(Frame { x: pos.a, y: pos.b, w: size.a, h: size.b })
}

/// Web apps (Claude, ChatGPT) report only the editable text region; the rounded box the user sees is
/// an ancestor that grows with it. Climb to the outermost ancestor that still hugs the text region,
/// so the chip rides the box's top edge as the box grows. Details are in the internal input-box notes.
unsafe fn input_box(el: AXUIElementRef, text: Frame) -> Frame {
    let prefer_inner = super::target_prefers_inner_box();
    let key = super::TARGET_KEY.lock().unwrap().clone();
    let min_padding = if super::apps::is_qoder(&super::TARGET_KEY.lock().unwrap()) { 2.0 } else { 8.0 };
    let mut best = text;
    let mut cur = el;
    let mut owned: Vec<CFTypeRef> = Vec::new();
    for _ in 0..6 {
        let Some(parent) = copy_attr(cur, "AXParent") else { break };
        owned.push(parent);
        cur = parent;
        let Some(f) = frame_of(parent) else { continue };
        let width_slack = super::apps::input_width_slack(&key, text.h, f.h, 1.0);
        let hugs = f.y <= text.y + 1.0
            && f.x <= text.x + 1.0
            && f.x + f.w >= text.x + text.w - 1.0
            && f.w <= text.w + width_slack
            // Toolbar row under the text, at most.
            && f.y + f.h <= text.y + text.h + 140.0
            // Attachments (images, files) sit above the text inside the box, so the top may rise a lot;
            // the height cap is what stops the climb at the box instead of the whole conversation column.
            && f.h <= text.h + 420.0;
        if !hugs { break; }
        best = f;
        if prefer_inner && super::input_box_has_toolbar(text, f, 1.0, min_padding) { break; }
    }
    for o in owned { CFRelease(o); }
    best
}

unsafe fn editable_input(el: AXUIElementRef, role: Option<&str>) -> bool {
    if !matches!(role, Some("AXTextArea" | "AXTextField" | "AXComboBox")) { return false; }
    if let Some(v) = copy_attr(el, "AXSubrole") {
        let cf = core_foundation::base::CFType::wrap_under_create_rule(v);
        if cf.downcast::<CFString>().is_some_and(|s| s.to_string() == "AXSecureTextField") { return false; }
    }
    let mut hints = String::new();
    for attr in ["AXDescription", "AXTitle", "AXHelp", "AXIdentifier", "AXPlaceholderValue"] {
        if let Some(v) = copy_attr(el, attr) {
            let cf = core_foundation::base::CFType::wrap_under_create_rule(v);
            if let Some(s) = cf.downcast::<CFString>() { hints.push_str(&s.to_string()); hints.push(' '); }
        }
    }
    let key = super::TARGET_KEY.lock().unwrap().clone();
    super::apps::is_chat_input_for_app(&key, super::target_is_ide(), &hints)
}

unsafe fn input_is_focused(d: &Active) -> bool {
    let Some(el) = copy_attr(d.app_el, "AXFocusedUIElement") else { return false };
    let same = !d.input.is_null() && CFEqual(el, d.input) != 0;
    CFRelease(el);
    same
}

/// Makes sure the caret is in the target's input box before a paste. Clicking Wings often leaves the
/// target active but with nothing focused, and Cmd+V then goes nowhere. Asks the remembered input to
/// take focus; if the app ignores that, clicks into it and puts the pointer back.
pub fn focus_input() -> bool {
    let mut all = ACTIVE.lock().unwrap();
    let Some(d) = current(&mut all) else { return false };
    let d = &*d;
    unsafe {
        if !target_is_front() || d.input.is_null() { return false; }
        if input_is_focused(d) { return true; }
        AXUIElementSetAttributeValue(d.input, cfs("AXFocused").as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        std::thread::sleep(Duration::from_millis(80));
        if input_is_focused(d) { return true; }
        let Some(f) = frame_of(d.input) else { return false };
        let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return false };
        let back = CGEvent::new(src.clone()).map(|e| e.location()).ok();
        // Near the end of the first line: inside the text, past most of it.
        let at = core_graphics::geometry::CGPoint::new(f.x + (f.w - 12.0).max(4.0), f.y + f.h.min(24.0) / 2.0);
        use core_graphics::event::{CGEventType, CGMouseButton};
        for ty in [CGEventType::LeftMouseDown, CGEventType::LeftMouseUp] {
            if let Ok(e) = CGEvent::new_mouse_event(src.clone(), ty, at, CGMouseButton::Left) { e.post(CGEventTapLocation::HID); }
            std::thread::sleep(Duration::from_millis(20));
        }
        if let Some(p) = back { CGWarpMouseCursorPosition(p); }
        std::thread::sleep(Duration::from_millis(60));
        input_is_focused(d)
    }
}

/// Whether an input holds text the user typed. Chromium inputs report a character count of 1 or more
/// even when empty (a placeholder, or the editor's hidden line break), so the count alone is not enough:
/// check the value itself, ignoring whitespace and a value that is just the placeholder.
unsafe fn has_real_text(el: AXUIElementRef) -> bool {
    let read = |attr: &str| copy_attr(el, attr).and_then(|v| {
        let cf = core_foundation::base::CFType::wrap_under_create_rule(v);
        cf.downcast::<CFString>().map(|s| s.to_string())
    });
    match read("AXValue") {
        Some(value) => {
            super::apps::has_input_text(&value, read("AXPlaceholderValue").as_deref())
        }
        None => copy_attr(el, "AXNumberOfCharacters")
            .and_then(|v| core_foundation::number::CFNumber::wrap_under_create_rule(v as _).to_i64())
            .is_some_and(|n| n > 0),
    }
}
