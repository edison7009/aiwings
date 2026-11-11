import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Target } from './TargetPicker';
import { t } from '../strings';

interface MenuApp extends Target { ai: boolean; save: boolean }

type Props =
  | { mode: 'single'; current: Target; onPick: (target: Target) => void; onError: (msg: string) => void }
  | { mode: 'multi'; followKeys: string[] | null; onChangeKeys: (keys: string[]) => void };

/**
 * App menu under the follow switch, opening below its button (the macOS native menu would cover it).
 * Lists AI apps installed here, then any other open app, so someone can also follow WeChat, say.
 * A blue "supports save" marks apps Wings can save from; the others are send and follow only.
 * single: tick the one app Wings stays with (launched first if it is not open).
 * multi: tick every app Wings follows; by default, all AI apps.
 */
export function AppMenu(props: Props) {
  const [open, setOpen] = useState(false);
  const [apps, setApps] = useState<MenuApp[]>([]);
  const [opening, setOpening] = useState<string | null>(null);
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

  const followed = props.mode === 'multi' ? (props.followKeys ?? apps.filter(a => a.ai).map(a => a.key)) : [];
  const isTicked = (app: MenuApp) => props.mode === 'single' ? app.key === props.current.key : followed.includes(app.key);

  const choose = async (app: MenuApp) => {
    if (props.mode === 'multi') {
      // The menu stays open so several apps can be ticked in a row.
      props.onChangeKeys(followed.includes(app.key) ? followed.filter(k => k !== app.key) : [...followed, app.key]);
      return;
    }
    setOpen(false);
    if (app.key === props.current.key) return;
    setOpening(app.key);
    try { props.onPick(await invoke<Target>('open_ai_app', { key: app.key })); }
    catch (e) { props.onError(String(e)); }
    finally { setOpening(null); }
  };

  const label = opening
    ? t.opening(apps.find(a => a.key === opening)?.name ?? '')
    : props.mode === 'single'
      ? props.current.name
      : followed.length === 0 ? t.followNone : apps.filter(isTicked).map(a => a.name).join('、') || t.followCount(followed.length);

  return (
    <div className="app-menu" ref={ref}>
      <button type="button" className="app-menu-button" aria-expanded={open} disabled={!!opening} onClick={() => { if (!open) load(); setOpen(!open); }}>
        <span className="row-text">{label}</span>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden><path d="M2 3.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
      </button>
      {open && (
        <ul className="app-menu-list" role="listbox" aria-multiselectable={props.mode === 'multi'}>
          {apps.length === 0 && <li className="app-menu-empty">{t.noApps}</li>}
          {apps.map(app => (
            <li key={app.key}>
              <button type="button" role="option" aria-selected={isTicked(app)} onClick={() => void choose(app)}>
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
