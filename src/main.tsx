import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { App } from './App';
import { Chip } from './components/Chip';
import { applyThemeMode, readPrefs } from './lib/prefs';
import './styles.css';

const isChip = getCurrentWindow().label === 'chip';
document.documentElement.dataset.window = isChip ? 'chip' : 'main';
applyThemeMode(readPrefs().theme);
// The other window saving prefs (the bar's settings) re-themes this one.
window.addEventListener('storage', e => { if (e.key === 'wings-prefs') applyThemeMode(readPrefs().theme); });
createRoot(document.getElementById('root')!).render(<StrictMode>{isChip ? <Chip /> : <App />}</StrictMode>);
