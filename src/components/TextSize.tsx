import { useEffect, useState } from 'react';
import { t } from '../strings';
import { getZoom, onZoom, setZoom, stepZoom, ZOOM_STEPS } from '../lib/zoom';

const MOD = /Mac/.test(navigator.userAgent) ? '⌘' : 'Ctrl';

/** − 100% + in settings; the same steps as the keyboard shortcuts. Clicking the percentage resets. */
export function TextSize() {
  const [z, setZ] = useState(getZoom());
  useEffect(() => onZoom(setZ), []);
  return (
    <div className="row text-size">
      <button type="button" className="ghost" aria-label={t.smaller} data-tip={`${t.smaller} ${MOD}-`} disabled={z === ZOOM_STEPS[0]} onClick={() => stepZoom(-1)}>−</button>
      <button type="button" className="ghost text-size-value" aria-label={t.resetSize} data-tip={`${t.resetSize} ${MOD}0`} onClick={() => setZoom(1)}>{Math.round(z * 100)}%</button>
      <button type="button" className="ghost" aria-label={t.larger} data-tip={`${t.larger} ${MOD}+`} disabled={z === ZOOM_STEPS[ZOOM_STEPS.length - 1]} onClick={() => stepZoom(1)}>+</button>
    </div>
  );
}
