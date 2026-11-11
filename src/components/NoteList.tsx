import { useState } from 'react';
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

function SendIcon() {
  return <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden><polygon points="5 3 21 12 5 21" /></svg>;
}

/**
 * Notes as in Coffee CLI's pre-input board: one line each; a click opens or closes the whole text.
 * Collapsed, hovering slides delete and send in over the end of the line without moving anything;
 * open, they sit in a row under the text. Send pastes into the target's input box.
 */
export function NoteList({ notes, shortcut, onPaste, onRemove }: Props) {
  const [openIds, setOpenIds] = useState<Set<string>>(() => new Set());
  const toggle = (id: string) => setOpenIds(prev => { const next = new Set(prev); if (!next.delete(id)) next.add(id); return next; });

  return (
    <div className="panel-body notes">
      {notes.length === 0 && (
        <div className="empty">
          <p>{t.emptyTitle}</p>
          <p className="muted">{t.emptyBody(shortcut)}</p>
        </div>
      )}
      {notes.map(note => {
        const open = openIds.has(note.id);
        const del = <button type="button" className="note-btn note-delete" aria-label={t.delete} onClick={() => onRemove(note.id)}><DeleteIcon /></button>;
        const send = <button type="button" className="note-btn note-send" aria-label={t.send} onClick={() => onPaste(note)}><SendIcon /></button>;
        return (
          <div
            key={note.id}
            className={`note${open ? '' : ' is-collapsed'}`}
            // The hover glow follows the pointer through a CSS variable, no re-render.
            onPointerMove={e => e.currentTarget.style.setProperty('--glow-x', `${Math.round(e.clientX - e.currentTarget.getBoundingClientRect().left)}px`)}
          >
            <button type="button" className="note-text" aria-expanded={open} onClick={() => toggle(note.id)}>{note.text}</button>
            {open
              ? <div className="note-actions">{del}<span className="note-gap" />{send}</div>
              : <span className="note-quick">{del}{send}</span>}
          </div>
        );
      })}
    </div>
  );
}
