// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';
import type { PaletteId } from './palettes';
import type { Lang } from '../strings';

export interface Prefs {
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
const DEFAULTS: Prefs = { notesPerApp: false, terminals: false, theme: 'system', palette: 'basalt', chip: true, lang: 'auto' };

export function readPrefs(): Prefs {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    // Older versions' layout (hover / docked) and app lists: Wings now only docks.
    for (const old of ['layout', 'mode', 'follow', 'followKeys', 'lockKeys']) delete raw[old];
    return { ...DEFAULTS, ...raw };
  } catch { return DEFAULTS; }
}

/** Saves the settings (other windows pick them up from the storage event) and hands them to the Rust side. */
export function writePrefs(p: Prefs) {
  try { localStorage.setItem(KEY, JSON.stringify(p)); } catch { /* Best effort. */ }
  sendPrefs(p);
}

export function sendPrefs(p: Prefs) {
  void invoke('set_prefs', { terminals: p.terminals, chip: p.chip });
}

/** Sets light/dark on this window. Every window calls it; the others hear changes via storage events. */
export function applyThemeMode(theme: Prefs['theme'], palette: Prefs['palette']) {
  const root = document.documentElement.dataset;
  if (theme === 'system') delete root.theme; else root.theme = theme;
  if (palette === 'basalt') delete root.palette; else root.palette = palette;
}
