import type { Prefs } from '../lib/prefs';
import { PALETTES, type PaletteId } from '../lib/palettes';
import { t } from '../strings';

type Half = { bg: string; card: string; accent: string };


/** Nine palette tiles, the name as a hover tip; following the system shows the day half fading into the night half. */
export function PaletteSwatches({ value, theme, onChange }: { value: PaletteId; theme: Prefs['theme']; onChange: (v: PaletteId) => void }) {
  return (
    <div className="swatches" role="radiogroup" aria-label={t.palette}>
      {PALETTES.map(p => {
        // Flat: the background colour plus an accent dot; following the system splits the tile day | night.
        const half: Half = theme === 'dark' ? p.night : p.day;
        const fill = theme === 'system' ? `linear-gradient(90deg, ${p.day.bg} 50%, ${p.night.bg} 50%)` : half.bg;
        const active = p.id === value;
        return (
          <button key={p.id} type="button" role="radio" aria-checked={active} aria-label={p.name} data-tip={p.name} className={active ? 'swatch active' : 'swatch'} onClick={() => onChange(p.id)}>
            <span className="swatch-tile" style={{ background: fill }}>
              <span className="swatch-dot" style={{ background: half.accent }} />
              {active && <svg className="swatch-check" width="15" height="15" viewBox="0 0 16 16" aria-hidden><circle cx="8" cy="8" r="8" fill="var(--accent)" /><path d="M4.8 8.2l2.2 2.2 4.2-4.4" fill="none" stroke="var(--accent-text)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg>}
            </span>
          </button>
        );
      })}
    </div>
  );
}
