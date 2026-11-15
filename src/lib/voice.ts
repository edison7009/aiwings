// Voice input settings, kept on this machine and shared live by Settings and the chip: the master switch
// (on by default), the microphone by name ('' = the system default) and the source ('' = automatic).
// Account secrets stay in the Rust side; only display details reach the page.
import { useSyncExternalStore } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { readPrefs } from './prefs';
import { t } from '../strings';

/** Wings' own sources (always listed, named in the UI language); other apps' sign-ins come with their own name. */
export type OwnSourceId = 'claude' | 'codex' | 'api' | 'macos';
export interface VoiceSource {
  id: string;
  name: string;
  accountId: string | null;
  /** Why it cannot be used now; "busy": the app is running and holds its session. */
  issue: 'missing' | 'expired' | 'unset' | 'busy' | null;
  /** "batch": all at once after the recording; streaming when absent. */
  transcriptionMode?: 'batch';
}
export type VoiceEvent =
  | { kind: 'ready' } | { kind: 'processing' }
  | { kind: 'text'; text: string; done: boolean }
  | { kind: 'error'; message: string }
  | { kind: 'level'; level: number };

const ENABLED = 'wings-voice-enabled';
const DEVICE = 'wings-voice-device';
const SOURCE = 'wings-voice-source';
const listeners = new Set<() => void>();
const read = (key: string) => { try { return localStorage.getItem(key); } catch { return null; } };
const write = (key: string, value: string) => { try { localStorage.setItem(key, value); } catch { /* Best effort. */ } listeners.forEach(l => l()); };
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  // Settings lives in another window than the chip.
  const storage = (e: StorageEvent) => { if (e.key === null || e.key === ENABLED || e.key === DEVICE || e.key === SOURCE) listener(); };
  window.addEventListener('storage', storage);
  return () => { listeners.delete(listener); window.removeEventListener('storage', storage); };
};

export const voiceEnabled = () => read(ENABLED) !== 'false';
export const voiceDevice = () => read(DEVICE) ?? '';
export const voiceSource = () => read(SOURCE) ?? '';
export const setVoiceEnabled = (on: boolean) => write(ENABLED, String(on));
export const setVoiceDevice = (name: string) => write(DEVICE, name);
export const setVoiceSource = (id: string) => write(SOURCE, id);
export const useVoiceEnabled = () => useSyncExternalStore(subscribe, voiceEnabled);
export const useVoiceDevice = () => useSyncExternalStore(subscribe, voiceDevice);
export const useVoiceSource = () => useSyncExternalStore(subscribe, voiceSource);

/** The UI language, or the system's when that is automatic. Traditional Chinese keeps its region (Apple's
 *  dictation needs it); the Rust side gives the online services the plain code. */
export function voiceLanguage(): string {
  const lang = readPrefs().lang;
  const tag = (lang === 'auto' ? navigator.language || 'en' : lang).toLowerCase();
  if (tag.startsWith('zh')) return /tw|hk|mo|hant/.test(tag) ? 'zh-TW' : 'zh';
  return tag.split('-')[0];
}

/** A speech error code as words; anything unknown is shown as it came. */
export function voiceError(message: string): string {
  const known = (t.voiceErrors as Record<string, string>)[message];
  return known ?? t.voiceFailed(message);
}

// The last scan, kept on this machine: scanning reads other apps' sign-ins (spawning some of them, asking
// the keychain), so it runs only the first time and on Refresh, as in Coffee CLI.
const SOURCES = 'wings-voice-sources';
export function cachedVoiceSources(): VoiceSource[] | null {
  try {
    const list: unknown = JSON.parse(localStorage.getItem(SOURCES) ?? 'null');
    return Array.isArray(list) && list.every(s => s && typeof s.id === 'string' && typeof s.name === 'string') ? list as VoiceSource[] : null;
  } catch { return null; }
}
export async function scanVoiceSources(): Promise<VoiceSource[]> {
  const list = await invoke<VoiceSource[]>('voice_sources');
  try { localStorage.setItem(SOURCES, JSON.stringify(list)); } catch { /* Best effort. */ }
  return list;
}

/** Starts a recording; events arrive on `onEvent` until the final text, an error, or a cancel. Automatic
 *  mode also tries the other apps' sign-ins the last scan found usable. */
export function startVoice(onEvent: (e: VoiceEvent) => void): Promise<number> {
  const channel = new Channel<VoiceEvent>();
  channel.onmessage = onEvent;
  const found = (cachedVoiceSources() ?? []).filter(s => !s.issue).map(s => s.id);
  return invoke<number>('voice_start', { source: voiceSource(), found, language: voiceLanguage(), device: voiceDevice() || null, onEvent: channel });
}
