import type { Note } from '../lib/notes';
import { t } from '../strings';

interface Props {
  notes: Note[];
  shortcut: string;
  onPaste: (note: Note) => void;
  onRemove: (id: string) => void;
}

/** A click pastes a note into the target's input box; × removes it. */
export function NoteList({ notes, shortcut, onPaste, onRemove }: Props) {
  return (
    <div className="panel-body notes">
      {notes.length === 0 && (
        <div className="empty">
          <p>{t.emptyTitle}</p>
          <p className="muted">{t.emptyBody(shortcut)}</p>
        </div>
      )}
      {notes.map(note => (
        <div key={note.id} className="note">
          <button type="button" className="note-body" onClick={() => onPaste(note)}>{note.text}</button>
          <button type="button" className="note-delete" aria-label={t.delete} onClick={() => onRemove(note.id)}>×</button>
        </div>
      ))}
    </div>
  );
}
