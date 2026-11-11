import type { Prefs } from '../lib/prefs';
import { PALETTES, type PaletteId } from '../lib/palettes';
import { t } from '../strings';

type Half = { bg: string; card: string; accent: string };

/** A diagonal sweep from the background through the card to a touch of the accent, as EchoBird draws its themes. */
const sweep = (h: Half) => `linear-gradient(135deg, ${h.bg} 0%, ${h.card} 48%, color-mix(in srgb, ${h.accent} 55%, ${h.card}) 100%)`;

/** Nine palette tiles, the name as a hover tip; following the system shows the day half fading into the night half. */
export function PaletteSwatches({ value, theme, onChange }: { value: PaletteId; theme: Prefs['theme']; onChange: (v: PaletteId) => void }) {
  return (
    <div className="swatches" role="radiogroup" aria-label={t.palette}>
      {PALETTES.map(p => {
        const fill = theme === 'system'
          ? `linear-gradient(135deg, ${p.day.bg} 0%, ${p.day.card} 30%, color-mix(in srgb, ${p.day.accent} 45%, ${p.day.card}) 48%, color-mix(in srgb, ${p.night.accent} 45%, ${p.night.card}) 52%, ${p.night.card} 70%, ${p.night.bg} 100%)`
          : sweep(theme === 'dark' ? p.night : p.day);
        const active = p.id === value;
        return (
          <button key={p.id} type="button" role="radio" aria-checked={active} aria-label={p.name} data-tip={p.name} className={active ? 'swatch active' : 'swatch'} onClick={() => onChange(p.id)}>
            <span className="swatch-tile" style={{ backgroundImage: fill }}>
              {active && <svg className="swatch-check" width="15" height="15" viewBox="0 0 16 16" aria-hidden><circle cx="8" cy="8" r="8" fill="var(--accent)" /><path d="M4.8 8.2l2.2 2.2 4.2-4.4" fill="none" stroke="var(--accent-text)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg>}
            </span>
          </button>
        );
      })}
    </div>
  );
}
