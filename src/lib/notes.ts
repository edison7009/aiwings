// Note model. Notes are a flat list in send order: oldest at the top.
// The Rust side owns the list: windows add and remove through it and receive every new list as `wings://notes`.
import { invoke } from '@tauri-apps/api/core';

export interface Note {
  id: string;
  text: string;
  createdAt: number;
  /** Image notes: id of the PNG kept by the Rust side (text is then empty). */
  image?: string;
  /** Key of the app the note was saved from, lowercased; absent for older notes and the welcome note. */
  app?: string;
}

const SEEDED_KEY = 'wings-seeded';

export async function loadNotes(welcome: string): Promise<Note[]> {
  let notes: Note[] = [];
  try { notes = await invoke<Note[]>('load_notes'); } catch { /* Unreadable file: show nothing rather than overwrite it. */ }
  let seeded = false;
  try { seeded = localStorage.getItem(SEEDED_KEY) === '1'; } catch { /* Best effort. */ }
  if (!seeded) {
    try { localStorage.setItem(SEEDED_KEY, '1'); } catch { /* Best effort. */ }
    if (notes.length === 0) notes = await invoke<Note[]>('add_note', { text: welcome }).catch(() => notes);
  }
  return notes;
}

export function removeNote(id: string) {
  void invoke('remove_note', { id }).catch(() => { /* The list stays as it was. */ });
}
