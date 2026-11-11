// Note model and persistence. Notes are a flat list in send order: oldest at the top.
import { invoke } from '@tauri-apps/api/core';

export interface Note {
  id: string;
  text: string;
  createdAt: number;
  /** Image notes: id of the PNG kept by the Rust side (text is then empty). */
  image?: string;
}

const SEEDED_KEY = 'wings-seeded';

export function makeNote(text: string, image?: string): Note {
  return image ? { id: image, text: '', createdAt: Date.now(), image } : { id: crypto.randomUUID(), text, createdAt: Date.now() };
}

export async function loadNotes(welcome: string): Promise<Note[]> {
  let notes: Note[] = [];
  try {
    const parsed: unknown = JSON.parse(await invoke<string>('load_notes'));
    if (Array.isArray(parsed)) notes = parsed.filter((n): n is Note => typeof n?.id === 'string' && typeof n?.text === 'string');
  } catch { /* A broken file starts an empty list rather than blocking the app. */ }
  let seeded = false;
  try { seeded = localStorage.getItem(SEEDED_KEY) === '1'; } catch { /* Best effort. */ }
  if (!seeded) {
    try { localStorage.setItem(SEEDED_KEY, '1'); } catch { /* Best effort. */ }
    if (notes.length === 0) notes = [makeNote(welcome)];
  }
  return notes;
}

export function saveNotes(notes: Note[]) {
  void invoke('save_notes', { data: JSON.stringify(notes) }).catch(() => { /* Next change retries the write. */ });
}
