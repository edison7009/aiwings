import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Note } from '../lib/notes';
import { t } from '../strings';

interface Props {
  notes: Note[];
  shortcut: string;
  onPaste: (note: Note) => void;
  onRemove: (id: string) => void;
}

function DeleteIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6" />
      <path d="M10 11v6" /><path d="M14 11v6" />
      <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

/**
 * Notes were already written in the target's own input box, so here they are read-only:
 * up to three lines each, a click pastes one back, and hovering slides in a trash icon
 * over the end of the text without moving anything.
 */
export function NoteList({ notes, shortcut, onPaste, onRemove }: Props) {
  const listRef = useHoverWhileInactive();
  return (
    <div ref={listRef} className="panel-body notes">
      {notes.length === 0 && (
        <div className="empty">
          <p>{t.emptyTitle}</p>
          <p className="muted">{t.emptyBody(shortcut)}</p>
        </div>
      )}
      {notes.map(note => (
        <div
          key={note.id}
          className="note"
          // The hover glow follows the pointer through a CSS variable, no re-render.
          onPointerMove={e => e.currentTarget.style.setProperty('--glow-x', `${Math.round(e.clientX - e.currentTarget.getBoundingClientRect().left)}px`)}
        >
          <button type="button" className="note-text" onClick={() => onPaste(note)}>{note.image ? <NoteImage id={note.image} /> : <span className="note-clamp">{note.text}</span>}</button>
          <span className="note-quick">
            <button type="button" className="note-btn note-delete" aria-label={t.delete} onClick={() => onRemove(note.id)}><DeleteIcon /></button>
          </span>
        </div>
      ))}
    </div>
  );
}

/**
 * macOS sends no mouse-move events to a window of an inactive app, and while the user types in the target
 * Wings is exactly that, so CSS :hover never fires. While the page lacks focus, poll the cursor and mark
 * the note under it (and the delete icon, when aimed at) with classes the CSS treats like :hover.
 */
function useHoverWhileInactive() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let busy = false;
    const clear = () => ref.current?.querySelectorAll('.is-hover').forEach(el => el.classList.remove('is-hover'));
    const timer = window.setInterval(() => {
      if (busy || !ref.current) return;
      if (document.hasFocus()) { clear(); return; }
      busy = true;
      void invoke<[number, number] | null>('cursor_in_window').then(pos => {
        busy = false;
        const root = ref.current;
        if (!root) return;
        const hit = pos ? document.elementFromPoint(pos[0], pos[1]) : null;
        const note = hit?.closest<HTMLElement>('.note') ?? null;
        const aim = hit?.closest<HTMLElement>('.note-btn') ?? null;
        root.querySelectorAll('.is-hover').forEach(el => { if (el !== note && el !== aim) el.classList.remove('is-hover'); });
        if (note && pos) {
          note.classList.add('is-hover');
          note.style.setProperty('--glow-x', `${Math.round(pos[0] - note.getBoundingClientRect().left)}px`);
        }
        aim?.classList.add('is-hover');
      }, () => { busy = false; });
    }, 60);
    return () => { window.clearInterval(timer); clear(); };
  }, []);
  return ref;
}

// Thumbnails load once per session; the PNG itself stays with the Rust side.
const imageCache = new Map<string, string>();

function NoteImage({ id }: { id: string }) {
  const [src, setSrc] = useState(() => imageCache.get(id) ?? null);
  useEffect(() => {
    if (src) return;
    void invoke<string>('image_data_url', { id }).then(url => { imageCache.set(id, url); setSrc(url); }, () => {});
  }, [id, src]);
  return src ? <img className="note-image" src={src} alt="" draggable={false} /> : <span className="note-image note-image--loading" />;
}
