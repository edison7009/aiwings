import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Onboarding } from './components/Onboarding';
import { UpdateButton } from './components/UpdateButton';
import { NoteList } from './components/NoteList';
import { Settings } from './components/Settings';
import { savedShortcut } from './components/ShortcutRecorder';
import { Segmented } from './components/Segmented';
import { loadNotes, removeNote, type Note } from './lib/notes';
import { layouts } from './lib/layouts';
import { applyThemeMode, readPrefs, saveFloatRect, sendPrefs, writePrefs, type Prefs } from './lib/prefs';
import { applyTheme, themeFor } from './lib/theme';
import { useHoverWhileInactive } from './lib/hover';
import { t, tr, setLang } from './strings';

const IS_MAC = /Mac/.test(navigator.userAgent);
const PASTE_KEY = IS_MAC ? '⌘V' : 'Ctrl+V';
const win = getCurrentWebviewWindow();
/** "main" is the hover panel; every other window is the bar beside one app. */
const IS_PANEL = win.label === 'main';

/** The app a bar sits beside. `terminal`: paste only, nothing is saved from it. */
interface BarTarget { id: string; key: string; name: string; terminal: boolean }

function GearIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}

/** Puts Wings away to the tray; it does not quit. macOS: a red dot on the left, as in the system's title bars;
 *  Windows: an × on the right. */
function CloseButton() {
  return (
    <button type="button" className={`win-close${IS_MAC ? ' win-close--mac' : ''}`} aria-label={t.closeTip} onClick={() => void invoke('hide_wings')}>
      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth={IS_MAC ? 2 : 1.3} strokeLinecap="round" aria-hidden>
        <path d="M3 3l6 6M9 3l-6 6" />
      </svg>
    </button>
  );
}

export function App() {
  // Bars only exist once permission is in place; the panel asks for it.
  const [trusted, setTrusted] = useState<boolean | null>(IS_PANEL ? null : true);
  const [target, setTarget] = useState<BarTarget | null>(null);
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [shortcut, setShortcut] = useState('Alt+S');
  const [prefs, setPrefs] = useState<Prefs>(readPrefs);
  const [view, setView] = useState<'notes' | 'settings'>('notes');
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);
  const panelRef = useHoverWhileInactive();

  const flash = useCallback((msg: string, duration = 2600) => {
    setToast(tr(msg));
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), duration);
  }, []);

  const changePrefs = useCallback((next: Prefs) => { setLang(next.lang); setPrefs(next); writePrefs(next); applyThemeMode(next.theme, next.palette); }, []);

  useEffect(() => {
    // Re-apply a saved custom shortcut first (the panel only: one window is enough); if it can no longer be
    // registered, the default stays.
    const saved = IS_PANEL ? savedShortcut() : null;
    const label = saved ? invoke<string>('set_shortcut', { combo: saved }).catch(() => invoke<string>('shortcut_label')) : invoke<string>('shortcut_label');
    void label.then(label => {
      setShortcut(label);
      // Only the panel seeds the welcome note, so several bars starting at once cannot add it twice.
      void (IS_PANEL ? loadNotes(t.welcome(label)) : invoke<Note[]>('load_notes')).then(setNotes, () => setNotes([]));
    });
    if (IS_PANEL) {
      void invoke<boolean>('permission_granted', { prompt: false }).then(setTrusted);
      sendPrefs(readPrefs());
    } else {
      void invoke<BarTarget | null>('bar_target').then(setTarget);
    }
  }, []);

  useEffect(() => {
    const subs = [
      listen<Note[]>('wings://notes', e => setNotes(e.payload)),
      // Errors are sent to the one window they concern.
      win.listen<string>('wings://error', e => flash(e.payload, 8000)),
    ];
    // Settings changed in another window: follow along.
    const onStorage = (e: StorageEvent) => {
      if (e.key !== 'wings-prefs') return;
      const next = readPrefs();
      setLang(next.lang);
      setPrefs(next);
    };
    window.addEventListener('storage', onStorage);
    return () => { subs.forEach(s => void s.then(f => f())); window.removeEventListener('storage', onStorage); };
  }, [flash]);

  useEffect(() => {
    void invoke('set_tray_labels', { show: t.trayShow, quit: t.trayQuit });
  }, [prefs.lang]);

  useEffect(() => { applyTheme(themeFor(target?.key ?? null)); }, [target]);

  // The panel remembers where it is and how big, so it comes back there (also on the next launch).
  useEffect(() => {
    if (!IS_PANEL) return;
    let timer: number | undefined;
    const save = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        void Promise.all([win.outerPosition(), win.innerSize()]).then(([p, z]) => saveFloatRect([p.x, p.y, z.width, z.height]));
      }, 300);
    };
    const subs = [win.onMoved(save), win.onResized(save)];
    return () => { window.clearTimeout(timer); subs.forEach(s => void s.then(f => f())); };
  }, []);

  // A bar lists only its own app's notes when the user asked for that; notes from before (no app) show everywhere.
  // Terminals cannot save, so their bar always shows every note.
  const ownKey = target && !target.terminal && prefs.notesPerApp ? target.key.toLowerCase() : null;
  const shown = notes && ownKey ? notes.filter(n => !n.app || n.app === ownKey) : notes;

  let body;
  if (trusted === null || shown === null || (!IS_PANEL && !target)) body = null;
  else if (!trusted) body = <Onboarding onGranted={() => setTrusted(true)} />;
  else if (view === 'settings') body = (
    <Settings prefs={prefs} shortcut={shortcut} onChange={changePrefs} onError={flash} onShortcut={setShortcut} onBack={() => setView('notes')} />
  );
  else body = (
    <>
      {/* The panel's header drags it anywhere; a bar follows its app instead. */}
      {/* Close and the layout switch, mirrored per platform: macOS keeps window buttons on the left, Windows on the right. */}
      <header className={`panel-head${IS_MAC ? ' panel-head--mac' : ''}`} data-tauri-drag-region={IS_PANEL || undefined}>
        <Segmented value={prefs.layout} options={layouts()} onChange={layout => changePrefs({ ...prefs, layout })} />
        <CloseButton />
      </header>
      <NoteList
        notes={shown}
        shortcut={shortcut}
        onPaste={note => { void invoke('paste_note', { text: note.text, image: note.image ?? null }); flash(t.pasted(PASTE_KEY)); }}
        onRemove={removeNote}
      />
      <button type="button" className="gear" aria-label={t.settings} onClick={() => setView('settings')}><GearIcon /></button>
    </>
  );

  return (
    <div className={`shell${IS_PANEL ? ' shell--float' : ' shell--docked'}`}>
      <div className="panel" ref={panelRef}>
        {body}
        {/* Mounted for the window's lifetime so a running download keeps its progress while settings is open. */}
        {trusted && shown !== null && <UpdateButton hidden={view === 'settings'} onError={flash} />}
      </div>
      {toast && <div className="toast" role="status">{toast}</div>}
    </div>
  );
}
