import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { t } from '../strings';

const FLIP_OUT_MS = 450;

/** "Save to Wings" on the target's input box: flips up from face down when there is text, back down when it goes. */
export function Chip() {
  const [shown, setShown] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const [prevShown, setPrevShown] = useState(shown);
  if (prevShown !== shown) { setPrevShown(shown); setLeaving(!shown); }

  useEffect(() => {
    const un = listen<boolean>('wings://chip', e => setShown(e.payload));
    return () => { void un.then(f => f()); };
  }, []);
  useEffect(() => {
    if (!leaving) return;
    const timer = window.setTimeout(() => setLeaving(false), FLIP_OUT_MS);
    return () => window.clearTimeout(timer);
  }, [leaving]);

  if (!shown && !leaving) return null;
  return (
    <div className="chip-ground">
      <button type="button" className={`chip-save${shown ? '' : ' is-leaving'}`} onMouseDown={e => e.preventDefault()} onClick={shown ? () => void invoke('capture_now') : undefined}>
        <span className="chip-face">{t.saveToWings}</span>
      </button>
    </div>
  );
}
