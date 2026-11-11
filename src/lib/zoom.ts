// Text size for the bar: ⌘+ / ⌘- / ⌘0 (Ctrl on Windows and Linux), remembered across launches.
const KEY = 'wings-zoom';
const STEPS = [0.8, 0.9, 1, 1.1, 1.25, 1.4, 1.6];

function read(): number {
  try { const z = Number(localStorage.getItem(KEY)); return STEPS.includes(z) ? z : 1; } catch { return 1; }
}

function apply(z: number) {
  // CSS zoom scales text and layout together, in WebKit and WebView2 alike.
  document.documentElement.style.setProperty('zoom', String(z));
  try { localStorage.setItem(KEY, String(z)); } catch { /* Best effort. */ }
}

let z = 1;
const listeners = new Set<(z: number) => void>();

export const ZOOM_STEPS = STEPS;
export function getZoom() { return z; }
export function setZoom(next: number) {
  if (next === z) return;
  z = next; apply(z); listeners.forEach(f => f(z));
}
export function onZoom(f: (z: number) => void) { listeners.add(f); return () => { listeners.delete(f); }; }
export function stepZoom(dir: -1 | 1) {
  const i = STEPS.indexOf(z);
  setZoom(STEPS[Math.min(Math.max(i + dir, 0), STEPS.length - 1)]);
}

export function installZoom() {
  z = read();
  apply(z);
  window.addEventListener('keydown', e => {
    if (!(/Mac/.test(navigator.userAgent) ? e.metaKey : e.ctrlKey) || e.altKey) return;
    if (e.key === '=' || e.key === '+') stepZoom(1);
    else if (e.key === '-' || e.key === '_') stepZoom(-1);
    else if (e.key === '0') setZoom(1);
    else return;
    e.preventDefault();
  });
}
