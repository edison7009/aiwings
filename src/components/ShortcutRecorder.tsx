import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../strings';

const SHORTCUT_KEY = 'wings-shortcut';

/** The saved combination in Tauri's format ("Alt+S"), or null for the default. */
export function savedShortcut(): string | null {
  try { return localStorage.getItem(SHORTCUT_KEY); } catch { return null; }
}

/** Turns a key press into "Modifier+Key": exactly one of ⌥ ⌃ ⌘ plus a letter, digit or F-key. Shift alone
 *  is refused: Shift+letter is how capitals are typed. */
function comboOf(e: KeyboardEvent): string | null {
  const mods = [e.altKey && 'Alt', e.ctrlKey && 'Control', e.metaKey && 'Super', e.shiftKey && 'Shift'].filter(Boolean) as string[];
  if (mods.length !== 1 || mods[0] === 'Shift') return null;
  const m = /^(?:Key([A-Z])|Digit([0-9])|(F[0-9]{1,2}))$/.exec(e.code);
  const key = m?.[1] ?? m?.[2] ?? m?.[3];
  return key ? `${mods[0]}+${key}` : null;
}

/** Click, then press the new combination. Esc cancels. */
export function ShortcutRecorder({ label, onChange, onError }: { label: string; onChange: (label: string) => void; onError: (msg: string) => void }) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === 'Escape') { setRecording(false); return; }
      if (['Alt', 'Control', 'Meta', 'Shift'].includes(e.key)) return; // still holding just the modifier
      const combo = comboOf(e);
      if (!combo) { onError(t.shortcutRule); return; }
      setRecording(false);
      void invoke<string>('set_shortcut', { combo }).then(next => {
        try { localStorage.setItem(SHORTCUT_KEY, combo); } catch { /* Best effort. */ }
        onChange(next);
      }, err => onError(String(err)));
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [recording, onChange, onError]);

  return (
    <button type="button" className={`row shortcut-row${recording ? ' is-recording' : ''}`} onClick={() => setRecording(!recording)}>
      <kbd>{recording ? t.shortcutPress : label}</kbd>
      <span className="muted small-inline">{recording ? t.shortcutCancel : t.shortcutChange}</span>
    </button>
  );
}
