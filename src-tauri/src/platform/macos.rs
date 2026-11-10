// macOS: AXObserver drives docking; CGEvent posts the synthetic shortcuts; NSWorkspace handles activation.

use super::{float_panel, place_bar, Frame, Key};
use crate::TargetInfo;
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
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
    fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFStringRef, out: *mut CFTypeRef) -> i32;
    fn AXValueGetValue(value: CFTypeRef, ty: u32, out: *mut c_void) -> bool;
    fn AXObserverCreate(pid: i32, cb: AXCallback, out: *mut AXObserverRef) -> i32;
    fn AXObserverAddNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef, refcon: *mut c_void) -> i32;
    fn AXObserverRemoveNotification(o: AXObserverRef, el: AXUIElementRef, n: CFStringRef) -> i32;
    fn AXObserverGetRunLoopSource(o: AXObserverRef) -> CFRunLoopSourceRef;
    fn CGEventSourceFlagsState(state: i32) -> u64;
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
}
// Only ever touched on the main thread; the Mutex is for the static.
unsafe impl Send for Dock {}

static DOCK: Mutex<Option<Dock>> = Mutex::new(None);
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
            // Show without activating Wings, so the target keeps keyboard focus.
            if let Ok(ns) = win.ns_window() {
                (*(ns as *const NSWindow)).orderFrontRegardless();
            }
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
        let mut observer: AXObserverRef = null();
        if AXObserverCreate(pid, on_ax, &mut observer) != 0 || observer.is_null() {
            CFRelease(app_el);
            return Err("无法监听这个应用的窗口".into());
        }
        for n in APP_NOTES { AXObserverAddNotification(observer, app_el, cfs(n).as_concrete_TypeRef(), std::ptr::null_mut()); }
        CFRunLoopAddSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(observer), kCFRunLoopDefaultMode);
        let mut d = Dock { app_el, observer, window: null() };
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
    let code: u16 = match key { Key::A => 0, Key::X => 7, Key::V => 9 };
    let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| "无法模拟按键")?;
    for down in [true, false] {
        let ev = CGEvent::new_keyboard_event(src.clone(), code, down).map_err(|_| "无法模拟按键")?;
        ev.set_flags(CGEventFlags::CGEventFlagCommand);
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
