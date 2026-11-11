import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { App } from './App';
import { Chip } from './components/Chip';
import { applyThemeMode, readPrefs } from './lib/prefs';
import { installZoom } from './lib/zoom';
import './styles.css';

const isChip = getCurrentWindow().label === 'chip';
document.documentElement.dataset.window = isChip ? 'chip' : 'main';
{ const p = readPrefs(); applyThemeMode(p.theme, p.palette); };
// The bar's text can be enlarged; the chip keeps its size so it still fits on the input's border.
if (!isChip) installZoom();
// The other window saving prefs (the bar's settings) re-themes this one.
window.addEventListener('storage', e => { if (e.key === 'wings-prefs') { const p = readPrefs(); applyThemeMode(p.theme, p.palette); }; });
createRoot(document.getElementById('root')!).render(<StrictMode>{isChip ? <Chip /> : <App />}</StrictMode>);
