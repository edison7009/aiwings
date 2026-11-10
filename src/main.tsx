import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { App } from './App';
import { Chip } from './components/Chip';
import './styles.css';

const isChip = getCurrentWindow().label === 'chip';
document.documentElement.dataset.window = isChip ? 'chip' : 'main';
createRoot(document.getElementById('root')!).render(<StrictMode>{isChip ? <Chip /> : <App />}</StrictMode>);
