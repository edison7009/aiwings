// User settings, kept in localStorage and mirrored to the Rust side.
import { invoke } from '@tauri-apps/api/core';

export interface Prefs {
  /** dock: glued to the target's right edge. float: stays where the user drags it. */
  mode: 'dock' | 'float';
  /** Re-dock onto whichever AI app the user switches to. */
  follow: boolean;
  /** Show the "save to Wings" chip on the target's input box. */
  chip: boolean;
  /** Background opacity, 0.3–1. */
  opacity: number;
}

const KEY = 'wings-prefs';
const DEFAULTS: Prefs = { mode: 'dock', follow: false, chip: true, opacity: 0.92 };

export function readPrefs(): Prefs {
  try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') }; } catch { return DEFAULTS; }
}

export function writePrefs(p: Prefs) {
  try { localStorage.setItem(KEY, JSON.stringify(p)); } catch { /* Best effort. */ }
  void invoke('set_prefs', { float: p.mode === 'float', follow: p.follow, chip: p.chip });
}
