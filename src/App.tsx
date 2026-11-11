import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Onboarding } from './components/Onboarding';
import { TargetPicker, type Target } from './components/TargetPicker';
import { NoteList } from './components/NoteList';
import { Settings } from './components/Settings';
import { savedShortcut } from './components/ShortcutRecorder';
import { Segmented } from './components/Segmented';
import { loadNotes, makeNote, saveNotes, type Note } from './lib/notes';
import { readPrefs, saveFloatRect, writePrefs, type Prefs } from './lib/prefs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { applyTheme, themeFor } from './lib/theme';
import { t } from './strings';

const TARGET_KEY = 'wings-target';
const PASTE_KEY = /Mac/.test(navigator.userAgent) ? '⌘V' : 'Ctrl+V';

interface Remembered { key: string; name: string }

function readRemembered(): Remembered | null {
  try { const raw = localStorage.getItem(TARGET_KEY); return raw ? JSON.parse(raw) : null; } catch { return null; }
}

function GearIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}

export function App() {
  const [trusted, setTrusted] = useState<boolean | null>(null);
  const [target, setTarget] = useState<Target | null>(null);
  const [missing, setMissing] = useState<string | null>(null);
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [shortcut, setShortcut] = useState('Alt+S');
  const [prefs, setPrefs] = useState<Prefs>(readPrefs);
  const [view, setView] = useState<'notes' | 'settings'>('notes');
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);
  const prefsRef = useRef(prefs);
  const trustedRef = useRef(false);
  prefsRef.current = prefs;
  trustedRef.current = trusted === true;

  const flash = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2600);
  }, []);

  const changePrefs = useCallback((next: Prefs) => { setPrefs(next); writePrefs(next); }, []);

  useEffect(() => {
    // Re-apply a saved custom shortcut first; if it can no longer be registered, the default stays.
    const saved = savedShortcut();
    const label = saved ? invoke<string>('set_shortcut', { combo: saved }).catch(() => invoke<string>('shortcut_label')) : invoke<string>('shortcut_label');
    void label.then(label => {
      setShortcut(label);
      void loadNotes(t.welcome(label)).then(setNotes);
    });
    void invoke<boolean>('permission_granted', { prompt: false }).then(setTrusted);
  }, []);

  const update = useCallback((fn: (prev: Note[]) => Note[]) => {
    setNotes(prev => { const next = fn(prev ?? []); saveNotes(next); return next; });
  }, []);

  const choose = useCallback(async (next: Target) => {
    try {
      await invoke('set_target', { id: next.id, key: next.key });
      setTarget(next);
      setMissing(null);
      writePrefs(prefsRef.current);
      try { localStorage.setItem(TARGET_KEY, JSON.stringify({ key: next.key, name: next.name })); } catch { /* Best effort. */ }
    } catch (e) { flash(String(e)); }
  }, [flash]);

  useEffect(() => {
    const subs = [
      listen<{ text: string; image: string | null }>('wings://captured', e => update(prev => [...prev, makeNote(e.payload.text, e.payload.image ?? undefined)])),
      listen<string>('wings://error', e => flash(e.payload)),
      listen<Target>('wings://follow', e => { if (prefsRef.current.follow && trustedRef.current) void choose(e.payload); }),
    ];
    return () => { subs.forEach(s => void s.then(f => f())); };
  }, [update, flash, choose]);

  // Hand the saved settings (which apps to follow, among others) to the Rust side as soon as it can act on them.
  useEffect(() => { if (trusted) writePrefs(prefsRef.current); }, [trusted]);

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

  useEffect(() => { applyTheme(themeFor(target?.key ?? null)); }, [target]);

  // While hovering, remember where the bar is and how big, so the next switch to hover (or the next launch) returns there.
  const floating = !!target && prefs.mode === 'float';
  useEffect(() => {
    if (!floating) return;
    const win = getCurrentWindow();
    let timer: number | undefined;
    const save = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        void Promise.all([win.outerPosition(), win.innerSize()]).then(([p, z]) => saveFloatRect([p.x, p.y, z.width, z.height]));
      }, 300);
    };
    const subs = [win.onMoved(save), win.onResized(save)];
    return () => { window.clearTimeout(timer); subs.forEach(s => void s.then(f => f())); };
  }, [floating]);


  let body;
  if (trusted === null || notes === null) body = null;
  else if (!trusted) body = <Onboarding onGranted={() => setTrusted(true)} />;
  else if (!target) body = <TargetPicker missing={missing} onPick={choose} />;
  else if (view === 'settings') body = (
    <Settings prefs={prefs} shortcut={shortcut} target={target} onChange={changePrefs} onPickTarget={next => void choose(next)} onError={flash} onShortcut={setShortcut} onBack={() => setView('notes')} />
  );
  else body = (
    <>
      {/* Hover mode lets the header drag the bar anywhere; docked, it follows the target instead. */}
      <header className="panel-head" data-tauri-drag-region={prefs.mode === 'float' || undefined}>
        <span className="brand" data-tauri-drag-region={prefs.mode === 'float' || undefined}>{target.name}</span>
        <Segmented value={prefs.mode} options={[['dock', t.dock], ['float', t.float]]} onChange={mode => changePrefs({ ...prefs, mode })} />
      </header>
      <NoteList
        notes={notes}
        shortcut={shortcut}
        onPaste={note => { void invoke('paste_note', { text: note.text, image: note.image ?? null }); flash(t.pasted(PASTE_KEY)); }}
        onRemove={id => update(prev => {
          const gone = prev.find(n => n.id === id);
          if (gone?.image) void invoke('delete_image', { id: gone.image });
          return prev.filter(n => n.id !== id);
        })}
      />
      <button type="button" className="gear" aria-label={t.settings} onClick={() => setView('settings')}><GearIcon /></button>
    </>
  );

  return (
    <div className={`shell${target ? (prefs.mode === 'dock' ? ' shell--docked' : ' shell--float') : ''}`}>
      <div className="panel">{body}</div>
      {toast && <div className="toast" role="status">{toast}</div>}
    </div>
  );
}
