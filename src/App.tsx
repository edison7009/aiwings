import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Onboarding } from './components/Onboarding';
import { TargetPicker, type Target } from './components/TargetPicker';
import { NoteList } from './components/NoteList';
import { loadNotes, makeNote, saveNotes, type Note } from './lib/notes';
import { t } from './strings';

const TARGET_KEY = 'wings-target';

interface Remembered { key: string; name: string }

function readRemembered(): Remembered | null {
  try { const raw = localStorage.getItem(TARGET_KEY); return raw ? JSON.parse(raw) : null; } catch { return null; }
}

export function App() {
  const [trusted, setTrusted] = useState<boolean | null>(null);
  const [target, setTarget] = useState<Target | null>(null);
  const [missing, setMissing] = useState<string | null>(null);
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [shortcut, setShortcut] = useState('Alt+S');
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const flash = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2600);
  }, []);

  useEffect(() => {
    void invoke<string>('shortcut_label').then(label => {
      setShortcut(label);
      void loadNotes(t.welcome(label)).then(setNotes);
    });
    void invoke<boolean>('permission_granted', { prompt: false }).then(setTrusted);
  }, []);

  const update = useCallback((fn: (prev: Note[]) => Note[]) => {
    setNotes(prev => { const next = fn(prev ?? []); saveNotes(next); return next; });
  }, []);

  useEffect(() => {
    const unCaptured = listen<string>('wings://captured', e => update(prev => [...prev, makeNote(e.payload)]));
    const unError = listen<string>('wings://error', e => flash(e.payload));
    return () => { void unCaptured.then(f => f()); void unError.then(f => f()); };
  }, [update, flash]);

  const choose = useCallback(async (next: Target) => {
    try {
      await invoke('set_target', { id: next.id });
      setTarget(next);
      setMissing(null);
      try { localStorage.setItem(TARGET_KEY, JSON.stringify({ key: next.key, name: next.name })); } catch { /* Best effort. */ }
    } catch (e) { flash(String(e)); }
  }, [flash]);

  // Re-dock the remembered app once permission is in place.
  useEffect(() => {
    if (!trusted || target) return;
    const last = readRemembered();
    if (!last) return;
    void invoke<Target[]>('list_targets').then(list => {
      const hit = list.find(item => item.key === last.key);
      if (hit) void choose(hit); else setMissing(last.name);
    });
  }, [trusted, target, choose]);

  const switchTarget = () => { void invoke('clear_target'); setTarget(null); };

  let body;
  if (trusted === null || notes === null) body = null;
  else if (!trusted) body = <Onboarding onGranted={() => setTrusted(true)} />;
  else if (!target) body = <TargetPicker missing={missing} onPick={choose} />;
  else body = (
    <NoteList
      notes={notes}
      shortcut={shortcut}
      targetName={target.name}
      onSwitch={switchTarget}
      onPaste={note => { void invoke('paste_note', { text: note.text }); flash(t.pasted); }}
      onRemove={id => update(prev => prev.filter(n => n.id !== id))}
    />
  );

  return (
    <div className="shell">
      {body}
      {toast && <div className="toast" role="status">{toast}</div>}
    </div>
  );
}
