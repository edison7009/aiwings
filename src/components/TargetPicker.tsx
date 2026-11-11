import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../strings';

export interface Target { id: string; key: string; name: string }

export function TargetPicker({ missing, onPick }: { missing: string | null; onPick: (target: Target) => void }) {
  const [list, setList] = useState<Target[] | null>(null);
  const load = useCallback(() => { void invoke<Target[]>('list_targets').then(setList); }, []);
  useEffect(load, [load]);

  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <span className="brand" data-tauri-drag-region>Wings · 预输入</span>
        <button type="button" className="ghost" onClick={load}>{t.refresh}</button>
      </header>
      <div className="panel-body">
        <h1>{t.pickTitle}</h1>
        <p className="muted">{missing ? t.lastMissing(missing) : t.pickHint}</p>
        {list && list.length === 0 && <p className="muted">{t.pickEmpty}</p>}
        <ul className="targets">
          {list?.map(item => (
            <li key={item.id}>
              <button type="button" onClick={() => onPick(item)}>{item.name || item.key}</button>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
