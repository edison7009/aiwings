import { useEffect, useState } from 'react';
import type { Prefs } from '../lib/prefs';
import { PALETTES, type PaletteId } from '../lib/palettes';
import { t } from '../strings';

const dark = window.matchMedia('(prefers-color-scheme: dark)');

/** A row of round swatches, each the palette's background with its accent; the name shows as a hover tip. */
export function PaletteSwatches({ value, theme, onChange }: { value: PaletteId; theme: Prefs['theme']; onChange: (v: PaletteId) => void }) {
  const [systemDark, setSystemDark] = useState(dark.matches);
  useEffect(() => {
    const on = () => setSystemDark(dark.matches);
    dark.addEventListener('change', on);
    return () => dark.removeEventListener('change', on);
  }, []);
  const night = theme === 'dark' || (theme === 'system' && systemDark);
  return (
    <div className="swatches" role="radiogroup" aria-label={t.palette}>
      {PALETTES.map(p => {
        const [bg, accent] = night ? p.night : p.day;
        return (
          <button key={p.id} type="button" role="radio" aria-checked={p.id === value} aria-label={p.name} data-tip={p.name}
            className={p.id === value ? 'swatch active' : 'swatch'} style={{ '--sw-bg': bg, '--sw-accent': accent } as React.CSSProperties}
            onClick={() => onChange(p.id)} />
        );
      })}
    </div>
  );
}
