import { useEffect, useRef, useState } from 'react';
import type { Prefs } from '../lib/prefs';
import { t, LANGS } from '../strings';

/** Language picker in the same in-app menu style as lock mode's app menu (the native dropdown looked out of place). */
export function LangMenu({ value, onChange }: { value: Prefs['lang']; onChange: (v: Prefs['lang']) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [open]);

  const options: [Prefs['lang'], string][] = [['auto', t.langAuto], ...LANGS];
  const label = options.find(([code]) => code === value)?.[1] ?? t.langAuto;
  return (
    <div className="app-menu lang-menu" ref={ref}>
      <button type="button" className="app-menu-button" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span className="row-text">{label}</span>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden><path d="M2 3.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
      </button>
      {open && (
        <ul className="app-menu-list" role="listbox">
          {options.map(([code, name]) => (
            <li key={code}>
              <button type="button" role="option" aria-selected={code === value} onClick={() => { onChange(code); setOpen(false); }}>
                <span className="app-menu-check" aria-hidden>{code === value ? '✓' : ''}</span>
                <span className="row-text">{name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
