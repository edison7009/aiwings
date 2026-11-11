// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';

export interface Prefs {
  /** dock: glued to the target's right edge. float: stays where the user drags it. */
  mode: 'dock' | 'float';
  /** Re-dock onto whichever followed app the user switches to. */
  follow: boolean;
  /** Apps (by key) ticked to follow; null = the default, every AI app. */
  followKeys: string[] | null;
  /** Show the "save to Wings" chip on the target's input box. */
  chip: boolean;
}

const KEY = 'wings-prefs';
const DEFAULTS: Prefs = { mode: 'dock', follow: true, followKeys: null, chip: true };

export function readPrefs(): Prefs {
  try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') }; } catch { return DEFAULTS; }
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
  void invoke('set_prefs', { float: p.mode === 'float', follow: p.follow, followKeys: p.followKeys, chip: p.chip, floatRect: readFloatRect() });
}
