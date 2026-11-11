// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';

export interface Prefs {
  /**
   * follow: docked beside the app in use, moving to whichever ticked app the user switches to.
   * pin: docked beside one chosen app only; hidden when the user is elsewhere.
   * float: stays wherever the user drags it; notes paste into the ticked app used last.
   */
  layout: 'follow' | 'pin' | 'float';
  /** Apps (by key) ticked to follow; null = the default, AI apps and Windows terminals. */
  followKeys: string[] | null;
  /** Light, dark, or follow the system. */
  theme: 'system' | 'light' | 'dark';
  /** Show the "save to Wings" chip on the target's input box. */
  chip: boolean;
}

const KEY = 'wings-prefs';
const DEFAULTS: Prefs = { layout: 'follow', followKeys: null, theme: 'system', chip: true };

export function readPrefs(): Prefs {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    // Older versions kept dock/float and follow as two separate switches.
    if (!raw.layout && (raw.mode || raw.follow !== undefined)) raw.layout = raw.mode === 'float' ? 'float' : raw.follow === false ? 'pin' : 'follow';
    delete raw.mode; delete raw.follow;
    return { ...DEFAULTS, ...raw };
  } catch { return DEFAULTS; }
}

const FLOAT_RECT_KEY = 'wings-float-rect';

/** Last hover frame in physical pixels [x, y, width, height]; hover mode returns there. */
export function saveFloatRect(rect: [number, number, number, number]) {
  try { localStorage.setItem(FLOAT_RECT_KEY, JSON.stringify(rect)); } catch { /* Best effort. */ }
}

function readFloatRect(): [number, number, number, number] | null {
  try {
    const r: unknown = JSON.parse(localStorage.getItem(FLOAT_RECT_KEY) ?? 'null');
    return Array.isArray(r) && r.length === 4 && r.every(n => typeof n === 'number') ? r as [number, number, number, number] : null;
  } catch { return null; }
}

export function writePrefs(p: Prefs) {
  try { localStorage.setItem(KEY, JSON.stringify(p)); } catch { /* Best effort. */ }
  void invoke('set_prefs', { float: p.layout === 'float', follow: p.layout !== 'pin', followKeys: p.followKeys, chip: p.chip, floatRect: readFloatRect() });
}

/** Sets light/dark on this window. Both windows (bar and chip) call it; the chip hears changes via storage events. */
export function applyThemeMode(theme: Prefs['theme']) {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}
