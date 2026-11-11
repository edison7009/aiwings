import type { Prefs } from '../lib/prefs';
import { PALETTES, type PaletteId } from '../lib/palettes';
import { t } from '../strings';

type Half = { bg: string; card: string; accent: string };


/** Nine palette tiles, the name as a hover tip; following the system shows the day half fading into the night half. */
const tint = (h: Half) => `color-mix(in srgb, ${h.accent} 30%, ${h.card})`;

export function PaletteSwatches({ value, theme, onChange }: { value: PaletteId; theme: Prefs['theme']; onChange: (v: PaletteId) => void }) {
  return (
    <div className="swatches" role="radiogroup" aria-label={t.palette}>
      {PALETTES.map(p => {
        // EchoBird's tiles: a soft diagonal from the background to a tint of the accent; following the
        // system runs day into night.
        const half: Half = theme === 'dark' ? p.night : p.day;
        const fill = theme === 'system'
          ? `linear-gradient(135deg, ${p.day.bg} 0%, ${tint(p.day)} 35%, ${tint(p.night)} 65%, ${p.night.bg} 100%)`
          : `linear-gradient(135deg, ${half.bg} 0%, ${half.card} 45%, ${tint(half)} 100%)`;
        const active = p.id === value;
        return (
          <button key={p.id} type="button" role="radio" aria-checked={active} aria-label={t.palettes[p.id]} className={active ? 'swatch active' : 'swatch'} onClick={() => onChange(p.id)}>
            <span className="swatch-tile" style={{ backgroundImage: fill }}>
              {/* Always blue: palettes colour the background and text, not this mark. */}
              {active && <svg className="swatch-check" width="15" height="15" viewBox="0 0 16 16" aria-hidden><circle cx="8" cy="8" r="8" fill="#3b82f6" /><path d="M4.8 8.2l2.2 2.2 4.2-4.4" fill="none" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg>}
            </span>
            <span className="swatch-name">{t.palettes[p.id]}</span>
          </button>
        );
      })}
    </div>
  );
}
