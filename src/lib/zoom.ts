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

export function installZoom() {
  let z = read();
  apply(z);
  window.addEventListener('keydown', e => {
    if (!(/Mac/.test(navigator.userAgent) ? e.metaKey : e.ctrlKey) || e.altKey) return;
    const i = STEPS.indexOf(z);
    let next = z;
    if (e.key === '=' || e.key === '+') next = STEPS[Math.min(i + 1, STEPS.length - 1)];
    else if (e.key === '-' || e.key === '_') next = STEPS[Math.max(i - 1, 0)];
    else if (e.key === '0') next = 1;
    else return;
    e.preventDefault();
    if (next !== z) { z = next; apply(z); }
  });
}
