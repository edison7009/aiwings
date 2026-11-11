import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { t } from '../strings';

const FLIP_OUT_MS = 450;

/** "Save to Wings" and "Clear" on the target's input box: flips up from face down when there is text, back down when it goes. */
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
      {/* Clearing by hand is slow; this selects all and deletes in the target's input box. */}
      <button type="button" className={`chip-save chip-clear${shown ? '' : ' is-leaving'}`} aria-label={t.clear} onMouseDown={e => e.preventDefault()} onClick={shown ? () => void invoke('clear_input') : undefined}>
        <span className="chip-face">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6" />
            <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
          </svg>
        </span>
      </button>
    </div>
  );
}
