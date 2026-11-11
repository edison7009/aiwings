import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Target } from './TargetPicker';
import { t } from '../strings';

interface MenuApp extends Target { ai: boolean; save: boolean }

interface Props { keys: string[]; onChangeKeys: (keys: string[]) => void }

/**
 * Lock mode's app menu, opening below its button (the macOS native menu would cover it).
 * Lists AI apps installed here, then any other open app, so someone can also lock onto WeChat, say.
 * A blue "supports save" marks apps Wings can save from; the others are send and follow only.
 * Tick one or more; the last tick cannot be removed, since locking onto nothing would follow nothing.
 */
export function AppMenu({ keys, onChangeKeys }: Props) {
  const [open, setOpen] = useState(false);
  const [apps, setApps] = useState<MenuApp[]>([]);
  const ref = useRef<HTMLDivElement>(null);

  const load = useCallback(() => { void invoke<MenuApp[]>('list_menu_apps').then(setApps); }, []);
  useEffect(load, [load]);

  // A press anywhere else closes the menu.
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [open]);

  const isTicked = (app: MenuApp) => keys.includes(app.key);

  // The menu stays open so several apps can be ticked in a row.
  const choose = (app: MenuApp) => {
    if (isTicked(app)) { if (keys.length > 1) onChangeKeys(keys.filter(k => k !== app.key)); }
    else onChangeKeys([...keys, app.key]);
  };

  const label = apps.filter(isTicked).map(a => a.name).join('、') || t.lockCount(keys.length);

  return (
    <div className="app-menu" ref={ref}>
      <button type="button" className="app-menu-button" aria-expanded={open} onClick={() => { if (!open) load(); setOpen(!open); }}>
        <span className="row-text">{label}</span>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden><path d="M2 3.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
      </button>
      {open && (
        <ul className="app-menu-list" role="listbox" aria-multiselectable>
          {apps.length === 0 && <li className="app-menu-empty">{t.noApps}</li>}
          {apps.map(app => (
            <li key={app.key}>
              <button type="button" role="option" aria-selected={isTicked(app)} onClick={() => choose(app)}>
                <span className="app-menu-check" aria-hidden>{isTicked(app) ? '✓' : ''}</span>
                <span className="row-text">{app.name}</span>
                {app.save && <span className="save-tag">{t.supportsSave}</span>}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
