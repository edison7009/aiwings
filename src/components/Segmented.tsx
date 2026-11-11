import type { ReactNode } from 'react';

/**
 * Switch with a sliding thumb. An option with an icon shows only the icon; its name appears as a
 * themed tip on hover (never a native title=), so the row still fits when the text is enlarged.
 */
export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: [T, string, ReactNode?][]; onChange: (v: T) => void }) {
  const index = Math.max(0, options.findIndex(([v]) => v === value));
  return (
    <div className="segmented" role="tablist" style={{ '--seg-index': index, '--seg-count': options.length } as React.CSSProperties}>
      <span className="segmented-thumb" aria-hidden />
      {options.map(([v, label, icon]) => (
        <button key={v} type="button" role="tab" aria-selected={v === value} className={`${v === value ? 'active' : ''}${icon ? ' icon' : ''}`}
          aria-label={icon ? label : undefined} data-tip={icon ? label : undefined} onClick={() => onChange(v)}>
          {icon ?? label}
        </button>
      ))}
    </div>
  );
}
