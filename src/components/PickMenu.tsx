import { useEffect, useRef, useState } from 'react';

/** A pick-one menu that opens in the flow below its button (the native dropdown looked out of place). */
export function PickMenu<T extends string>({ value, options, onChange, className }: { value: T; options: [T, string][]; onChange: (v: T) => void; className?: string }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [open]);
  const label = options.find(([v]) => v === value)?.[1] ?? options[0]?.[1] ?? '';
  return (
    <div className={`app-menu${className ? ` ${className}` : ''}`} ref={ref}>
      <button type="button" className="app-menu-button" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span className="row-text">{label}</span>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden><path d="M2 3.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
      </button>
      {open && (
        <ul className="app-menu-list" role="listbox">
          {options.map(([v, name]) => (
            <li key={v}>
              <button type="button" role="option" aria-selected={v === value} onClick={() => { onChange(v); setOpen(false); }}>
                <span className="app-menu-check" aria-hidden>{v === value ? '✓' : ''}</span>
                <span className="row-text">{name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
