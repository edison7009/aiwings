// Voice input settings, kept on this machine and shared live by Settings and the chip: the master switch
// (on by default), the microphone by name ('' = the system default) and the source ('' = automatic).
// Account secrets stay in the Rust side; only display details reach the page.
import { useSyncExternalStore } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { readPrefs } from './prefs';

export type VoiceSourceId = 'claude' | 'codex' | 'api' | 'macos';
export interface VoiceSource { id: VoiceSourceId; accountId: string | null; issue: 'missing' | 'expired' | 'unset' | null; mode: 'streaming' | 'batch' }
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

export const loadVoiceSources = () => invoke<VoiceSource[]>('voice_sources');

/** Starts a recording; events arrive on `onEvent` until the final text, an error, or a cancel. */
export function startVoice(onEvent: (e: VoiceEvent) => void): Promise<number> {
  const channel = new Channel<VoiceEvent>();
  channel.onmessage = onEvent;
  return invoke<number>('voice_start', { source: voiceSource(), language: voiceLanguage(), device: voiceDevice() || null, onEvent: channel });
}
