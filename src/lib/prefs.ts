// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';
import type { PaletteId } from './palettes';
import type { Lang } from '../strings';

export interface Prefs {
  /**
   * follow ("吸附"): a bar beside each open AI app, staying put; with none open, the hover panel shows instead.
   * float ("悬停"): one panel wherever the user drags it; notes paste into the AI app used last.
   */
  layout: 'follow' | 'float';
  /** Each bar lists only the notes saved from its own app (older notes show in every bar). */
  notesPerApp: boolean;
  /** Terminals get bars too: paste only, nothing can be saved from them. */
  terminals: boolean;
  /** Light, dark, or follow the system. */
  theme: 'system' | 'light' | 'dark';
  /** Colour family; the theme above picks its day or night half. */
  palette: PaletteId;
  /** Show the "save to Wings" chip on the target's input box. */
  chip: boolean;
  /** UI language; 'auto' follows the system. */
  lang: Lang | 'auto';
}

const KEY = 'wings-prefs';
const DEFAULTS: Prefs = { layout: 'follow', notesPerApp: false, terminals: false, theme: 'system', palette: 'basalt', chip: true, lang: 'auto' };

export function readPrefs(): Prefs {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    // Older versions: dock/float switches, "pin", a follow list and "lock" (chosen apps) all become docking,
    // which now gives every open AI app its own bar.
    if (!raw.layout && raw.mode === 'float') raw.layout = 'float';
    if (raw.layout !== 'float') raw.layout = 'follow';
    for (const old of ['mode', 'follow', 'followKeys', 'lockKeys']) delete raw[old];
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

/** Saves the settings (other windows pick them up from the storage event) and hands them to the Rust side. */
export function writePrefs(p: Prefs) {
  try { localStorage.setItem(KEY, JSON.stringify(p)); } catch { /* Best effort. */ }
  sendPrefs(p);
}

export function sendPrefs(p: Prefs) {
  void invoke('set_prefs', { float: p.layout === 'float', terminals: p.terminals, chip: p.chip, floatRect: readFloatRect() });
}

/** Sets light/dark on this window. Every window calls it; the others hear changes via storage events. */
export function applyThemeMode(theme: Prefs['theme'], palette: Prefs['palette']) {
  const root = document.documentElement.dataset;
  if (theme === 'system') delete root.theme; else root.theme = theme;
  if (palette === 'basalt') delete root.palette; else root.palette = palette;
}
