import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Prefs } from '../lib/prefs';
import type { Target } from './TargetPicker';
import { t } from '../strings';
import { Segmented } from './Segmented';

interface Props {
  prefs: Prefs;
  shortcut: string;
  target: Target;
  onChange: (p: Prefs) => void;
  onPickTarget: (t: Target) => void;
  onBack: () => void;
}

export function Settings({ prefs, shortcut, target, onChange, onPickTarget, onBack }: Props) {
  const set = <K extends keyof Prefs>(k: K, v: Prefs[K]) => onChange({ ...prefs, [k]: v });
  // Open apps come and go, so the list reloads whenever the menu is about to open.
  const [apps, setApps] = useState<Target[]>([]);
  const loadApps = useCallback(() => { void invoke<Target[]>('list_targets').then(setApps); }, []);
  useEffect(loadApps, [loadApps]);
  const options = apps.some(a => a.id === target.id) ? apps : [target, ...apps];
  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <button type="button" className="ghost" onClick={onBack}>‹ {t.back}</button>
        <span className="brand" data-tauri-drag-region>{t.settings}</span>
      </header>
      <div className="panel-body settings">
        {/* Follow first: when off, the menu below picks the one app Wings stays with. */}
        <section>
          <Segmented value={prefs.follow ? 'on' : 'off'} options={[['on', t.followOn], ['off', t.followOff]]} onChange={v => set('follow', v === 'on')} />
          {!prefs.follow && (
            <select className="app-select" value={target.id} onMouseDown={loadApps} onFocus={loadApps}
              onChange={e => { const hit = options.find(a => a.id === e.target.value); if (hit) onPickTarget(hit); }}>
              {options.map(a => <option key={a.id} value={a.id}>{a.name || a.key}</option>)}
            </select>
          )}
        </section>
        <section>
          <label>{t.chipLabel}</label>
          <Segmented value={prefs.chip ? 'on' : 'off'} options={[['on', t.show], ['off', t.hide]]} onChange={v => set('chip', v === 'on')} />
        </section>
        <section>
          <label>{t.opacity} <span className="muted">{Math.round(prefs.opacity * 100)}%</span></label>
          <input type="range" min={30} max={100} value={Math.round(prefs.opacity * 100)} onChange={e => set('opacity', Number(e.target.value) / 100)} />
        </section>
        <section>
          <label>{t.shortcut}</label>
          <div className="row"><kbd>{shortcut}</kbd></div>
        </section>
      </div>
    </>
  );
}
