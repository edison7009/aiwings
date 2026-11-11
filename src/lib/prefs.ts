// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';

export interface Prefs {
  /**
   * follow: docked beside the AI app in use, moving to whichever AI app the user switches to.
   * lock: the same, but only among the apps the user chose (one or more).
   * float: stays wherever the user drags it; notes paste into the AI app used last.
   */
  layout: 'follow' | 'lock' | 'float';
  /** Apps (by key) chosen for lock mode; never empty while locked. */
  lockKeys: string[];
  /** Light, dark, or follow the system. */
  theme: 'system' | 'light' | 'dark';
  /** Show the "save to Wings" chip on the target's input box. */
  chip: boolean;
}

const KEY = 'wings-prefs';
const DEFAULTS: Prefs = { layout: 'follow', lockKeys: [], theme: 'system', chip: true };

export function readPrefs(): Prefs {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    // Older versions kept dock/float and follow as two separate switches.
    if (!raw.layout && (raw.mode || raw.follow !== undefined)) raw.layout = raw.mode === 'float' ? 'float' : raw.follow === false ? 'pin' : 'follow';
    delete raw.mode; delete raw.follow;
    // Older versions had "pin" (one app) and a follow list. Both become lock mode with the same apps.
    if (raw.layout === 'pin') {
      raw.layout = 'lock';
      const last = rememberedKey();
      raw.lockKeys = last ? [last] : [];
    } else if (raw.layout === 'follow' && Array.isArray(raw.followKeys) && raw.followKeys.length > 0) {
      raw.layout = 'lock';
      raw.lockKeys = raw.followKeys;
    }
    delete raw.followKeys;
    return { ...DEFAULTS, ...raw };
  } catch { return DEFAULTS; }
}

function rememberedKey(): string | null {
  try { return JSON.parse(localStorage.getItem('wings-target') ?? 'null')?.key ?? null; } catch { return null; }
}

/** Switches layout. Entering lock mode with nothing chosen locks onto the current app. */
export function withLayout(p: Prefs, layout: Prefs['layout'], currentKey: string | null): Prefs {
  const lockKeys = layout === 'lock' && p.lockKeys.length === 0 && currentKey ? [currentKey] : p.lockKeys;
  return { ...p, layout, lockKeys };
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
  void invoke('set_prefs', { float: p.layout === 'float', follow: true, followKeys: p.layout === 'lock' && p.lockKeys.length > 0 ? p.lockKeys : null, chip: p.chip, floatRect: readFloatRect() });
}

/** Sets light/dark on this window. Both windows (bar and chip) call it; the chip hears changes via storage events. */
export function applyThemeMode(theme: Prefs['theme']) {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}
