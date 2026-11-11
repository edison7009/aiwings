// macOS: AXObserver drives docking; CGEvent posts the synthetic shortcuts; NSWorkspace handles activation.

use super::{float_panel, place_bar, Frame, Key};
use crate::TargetInfo;
use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::number::{CFBooleanGetValue, CFBooleanRef};
use core_foundation_sys::runloop::{kCFRunLoopDefaultMode, CFRunLoopAddSource, CFRunLoopGetMain, CFRunLoopRemoveSource, CFRunLoopSourceRef};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication, NSWindow, NSWorkspace};
use std::ffi::c_void;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
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
    fn AXUIElementSetAttributeValue(el: AXUIElementRef, attr: CFStringRef, value: CFTypeRef) -> i32;
    fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFStringRef, out: *mut CFTypeRef) -> i32;
    fn AXValueGetValue(value: CFTypeRef, ty: u32, out: *mut c_void) -> bool;
    fn AXObserverCreate(pid: i32, cb: AXCallback, out: *mut AXObserverRef) -> i32;
    fn AXObserverAddNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef, refcon: *mut c_void) -> i32;
    fn AXObserverRemoveNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef) -> i32;
    fn AXObserverGetRunLoopSource(o: AXObserverRef) -> CFRunLoopSourceRef;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGWarpMouseCursorPosition(p: core_graphics::geometry::CGPoint) -> i32;
}

const APP_NOTES: &[&str] = &[
    "AXMainWindowChanged", "AXFocusedWindowChanged", "AXWindowCreated",
    "AXApplicationActivated", "AXApplicationDeactivated", "AXApplicationHidden", "AXApplicationShown",
];
const WINDOW_NOTES: &[&str] = &["AXMoved", "AXResized", "AXWindowMiniaturized", "AXWindowDeminiaturized", "AXUIElementDestroyed"];
const REATTACH: &[&str] = &["AXMainWindowChanged", "AXFocusedWindowChanged", "AXWindowCreated", "AXUIElementDestroyed"];

struct Dock {
    app_el: AXUIElementRef,
    observer: AXObserverRef,
    /// The followed window (owned +1), or null when the app has none.
    window: AXUIElementRef,
    /// The last text input the user focused in the target (owned +1), or null. Paste puts the caret back here.
    input: AXUIElementRef,
}
// Observer setup and teardown happen on the main thread; AX element reads are thread-safe.
unsafe impl Send for Dock {}

static DOCK: Mutex<Option<Dock>> = Mutex::new(None);
static TARGET_PID: AtomicI32 = AtomicI32::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
static FLOAT: AtomicBool = AtomicBool::new(false);

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

unsafe fn attach_window(d: &mut Dock) {
    if !d.window.is_null() {
        for n in WINDOW_NOTES { AXObserverRemoveNotification(d.observer, d.window, cfs(n).as_concrete_TypeRef()); }
        CFRelease(d.window);
        d.window = null();
    }
    let win = copy_attr(d.app_el, "AXMainWindow").or_else(|| copy_attr(d.app_el, "AXFocusedWindow"));
    if let Some(w) = win {
        for n in WINDOW_NOTES { AXObserverAddNotification(d.observer, w, cfs(n).as_concrete_TypeRef(), std::ptr::null_mut()); }
        d.window = w;
    }
}

/// Show the bar beside the target when the target (or Wings itself) is frontmost and its window is up; hide otherwise.
unsafe fn refresh_locked(d: &Dock) {
    // Hover mode: the bar stays wherever the user put it.
    if FLOAT.load(Ordering::SeqCst) { return; }
    let Some(app) = APP.get() else { return };
    let Some(win) = app.get_webview_window("main") else { return };
    let front = front_pid();
    let pid = TARGET_PID.load(Ordering::SeqCst);
    let ours = std::process::id() as i32;
    let frame = (!d.window.is_null() && !is_minimized(d.window))
        .then(|| Some((read_pair(d.window, "AXPosition", 1)?, read_pair(d.window, "AXSize", 2)?)))
        .flatten();
    match frame {
        Some((pos, size)) if front == pid || front == ours => {
            let s = win.scale_factor().unwrap_or(1.0);
            place_bar(app, Frame { x: pos.a * s, y: pos.b * s, w: size.a * s, h: size.b * s }, s);
            show_no_activate(&win);
        }
        _ => { let _ = win.hide(); }
    }
}

unsafe extern "C" fn on_ax(_o: AXObserverRef, _el: AXUIElementRef, n: CFStringRef, _r: *mut c_void) {
    let name = CFString::wrap_under_get_rule(n).to_string();
    let mut guard = DOCK.lock().unwrap();
    let Some(d) = guard.as_mut() else { return };
    if REATTACH.contains(&name.as_str()) { attach_window(d); }
    refresh_locked(d);
}

unsafe fn teardown(d: Dock) {
    let src = AXObserverGetRunLoopSource(d.observer);
    CFRunLoopRemoveSource(CFRunLoopGetMain(), src, kCFRunLoopDefaultMode);
    if !d.window.is_null() { CFRelease(d.window); }
    if !d.input.is_null() { CFRelease(d.input); }
    CFRelease(d.observer);
    CFRelease(d.app_el);
}

pub fn dock(app: AppHandle, id: &str) -> Result<(), String> {
    let _ = APP.set(app.clone());
    let pid: i32 = id.parse().map_err(|_| "无效的目标".to_string())?;
    if !accessibility_trusted(false) { return Err("需要先授予辅助功能权限".into()); }
    let mut guard = DOCK.lock().unwrap();
    unsafe {
        if let Some(old) = guard.take() { teardown(old); }
        let app_el = AXUIElementCreateApplication(pid);
        // Electron/Chromium apps (Claude, ChatGPT) only build their AX tree when asked to.
        AXUIElementSetAttributeValue(app_el, cfs("AXManualAccessibility").as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        let mut observer: AXObserverRef = null();
        if AXObserverCreate(pid, on_ax, &mut observer) != 0 || observer.is_null() {
            CFRelease(app_el);
            return Err("无法监听这个应用的窗口".into());
        }
        for n in APP_NOTES { AXObserverAddNotification(observer, app_el, cfs(n).as_concrete_TypeRef(), std::ptr::null_mut()); }
        CFRunLoopAddSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(observer), kCFRunLoopDefaultMode);
        let mut d = Dock { app_el, observer, window: null(), input: null() };
        attach_window(&mut d);
        TARGET_PID.store(pid, Ordering::SeqCst);
        refresh_locked(&d);
        *guard = Some(d);
    }
    Ok(())
}

pub fn undock(app: &AppHandle) {
    TARGET_PID.store(0, Ordering::SeqCst);
    if let Some(d) = DOCK.lock().unwrap().take() { unsafe { teardown(d) } }
    float_panel(app);
}

pub fn refresh_later(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        // Let the new frontmost app settle before reading it.
        std::thread::sleep(Duration::from_millis(150));
        let _ = app.run_on_main_thread(|| {
            if let Some(d) = DOCK.lock().unwrap().as_ref() { unsafe { refresh_locked(d) } }
        });
    });
}

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

pub fn set_float(app: &AppHandle, float: bool) {
    FLOAT.store(float, Ordering::SeqCst);
    let Some(win) = app.get_webview_window("main") else { return };
    if float { show_no_activate(&win); } else { refresh_later(app); }
}

pub fn current_id() -> String { TARGET_PID.load(Ordering::SeqCst).to_string() }

pub fn front_app() -> Option<TargetInfo> {
    let a = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(TargetInfo {
        id: a.processIdentifier().to_string(),
        key: a.bundleIdentifier()?.to_string(),
        name: a.localizedName().map(|n| n.to_string()).unwrap_or_default(),
    })
}

/// The target's focused text input: the frame (points) of the box drawn around it, and whether it holds any text.
/// Reads only the character count, never the text itself.
pub fn focused_input() -> Option<(Frame, bool)> {
    let mut guard = DOCK.lock().unwrap();
    let d = guard.as_mut()?;
    unsafe {
        let el = copy_attr(d.app_el, "AXFocusedUIElement")?;
        let role = copy_attr(el, "AXRole").map(|r| CFString::wrap_under_create_rule(r as CFStringRef).to_string());
        let editable = matches!(role.as_deref(), Some("AXTextArea" | "AXTextField" | "AXComboBox"));
        let frame = if editable { frame_of(el) } else { None };
        if editable && el != d.input {
            // Remember it, so a paste can put the caret back after focus wandered off.
            if !d.input.is_null() { CFRelease(d.input); }
            d.input = CFRetain(el);
        }
        let count = copy_attr(el, "AXNumberOfCharacters")
            .and_then(|v| core_foundation::number::CFNumber::wrap_under_create_rule(v as _).to_i64())
            .unwrap_or(0);
        let frame = frame.map(|f| input_box(el, f));
        CFRelease(el);
        Some((frame?, count > 0))
    }
}

unsafe fn frame_of(el: AXUIElementRef) -> Option<Frame> {
    let (pos, size) = (read_pair(el, "AXPosition", 1)?, read_pair(el, "AXSize", 2)?);
    Some(Frame { x: pos.a, y: pos.b, w: size.a, h: size.b })
}

/// Web apps (Claude, ChatGPT) report only the editable text region; the rounded box the user sees is
/// an ancestor that grows with it. Climb to the outermost ancestor that still hugs the text region,
/// so the chip rides the box's top edge as the box grows. See docs/input-box.md.
unsafe fn input_box(el: AXUIElementRef, text: Frame) -> Frame {
    let mut best = text;
    let mut cur = el;
    let mut owned: Vec<CFTypeRef> = Vec::new();
    for _ in 0..6 {
        let Some(parent) = copy_attr(cur, "AXParent") else { break };
        owned.push(parent);
        cur = parent;
        let Some(f) = frame_of(parent) else { continue };
        let hugs = f.y <= text.y + 1.0
            && f.x <= text.x + 1.0
            && f.x + f.w >= text.x + text.w - 1.0
            && f.w <= text.w + 160.0
            // Toolbar row under the text, at most.
            && f.y + f.h <= text.y + text.h + 140.0
            // Attachments (images, files) sit above the text inside the box, so the top may rise a lot;
            // the height cap is what stops the climb at the box instead of the whole conversation column.
            && f.h <= text.h + 420.0;
        if !hugs { break; }
        best = f;
    }
    for o in owned { CFRelease(o); }
    best
}

unsafe fn focused_is_editable(app_el: AXUIElementRef) -> bool {
    let Some(el) = copy_attr(app_el, "AXFocusedUIElement") else { return false };
    let role = copy_attr(el, "AXRole").map(|r| CFString::wrap_under_create_rule(r as CFStringRef).to_string());
    CFRelease(el);
    matches!(role.as_deref(), Some("AXTextArea" | "AXTextField" | "AXComboBox"))
}

/// Makes sure the caret is in the target's input box before a paste. Clicking Wings often leaves the
/// target active but with nothing focused, and Cmd+V then goes nowhere. Asks the remembered input to
/// take focus; if the app ignores that, clicks into it and puts the pointer back.
pub fn focus_input() {
    let guard = DOCK.lock().unwrap();
    let Some(d) = guard.as_ref() else { return };
    unsafe {
        if focused_is_editable(d.app_el) || d.input.is_null() { return; }
        AXUIElementSetAttributeValue(d.input, cfs("AXFocused").as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
        std::thread::sleep(Duration::from_millis(80));
        if focused_is_editable(d.app_el) { return; }
        let Some(f) = frame_of(d.input) else { return };
        let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
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
    }
}
