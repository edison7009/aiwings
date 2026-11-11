import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { withLayout, type Prefs } from '../lib/prefs';
import type { Target } from './TargetPicker';
import { t } from '../strings';
import { LangMenu } from './LangMenu';
import { Segmented } from './Segmented';
import { layouts } from '../lib/layouts';
import { AppMenu } from './AppMenu';
import { ShortcutRecorder } from './ShortcutRecorder';
import { TextSize } from './TextSize';
import { PaletteSwatches } from './PaletteSwatches';

interface Props {
  prefs: Prefs;
  shortcut: string;
  target: Target;
  onChange: (p: Prefs) => void;
  onError: (msg: string) => void;
  onShortcut: (label: string) => void;
  onBack: () => void;
}

export function Settings({ prefs, shortcut, target, onChange, onError, onShortcut, onBack }: Props) {
  const [version, setVersion] = useState('');
  useEffect(() => { void getVersion().then(setVersion).catch(() => {}); }, []);
  const set = <K extends keyof Prefs>(k: K, v: Prefs[K]) => onChange({ ...prefs, [k]: v });
  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <span className="brand" data-tauri-drag-region>{t.settings}</span>
      </header>
      <div className="panel-body settings">
        {/* Layout first. Only lock mode asks which apps; follow and hover need no setup. */}
        <section>
          <Segmented value={prefs.layout} options={layouts()} onChange={v => onChange(withLayout(prefs, v, target.key))} />
          <p className="muted small hint">{t.layoutHint[prefs.layout]}</p>
          {prefs.layout === 'lock' && <AppMenu keys={prefs.lockKeys} onChangeKeys={keys => set('lockKeys', keys)} />}
        </section>
        <section>
          <label>{t.chipLabel}</label>
          <Segmented value={prefs.chip ? 'on' : 'off'} options={[['on', t.show], ['off', t.hide]]} onChange={v => set('chip', v === 'on')} />
        </section>
        <section>
          <label>{t.theme}</label>
          <Segmented value={prefs.theme} options={[['system', t.themeSystem], ['light', t.themeLight], ['dark', t.themeDark]]} onChange={v => set('theme', v)} />
          <PaletteSwatches value={prefs.palette} theme={prefs.theme} onChange={v => set('palette', v)} />
        </section>
        <section>
          <label>{t.textSize}</label>
          <TextSize />
        </section>
        <section>
          <label>{t.language}</label>
          <LangMenu value={prefs.lang} onChange={v => set('lang', v)} />
        </section>
        <section>
          <label>{t.shortcut}</label>
          <ShortcutRecorder label={shortcut} onChange={onShortcut} onError={onError} />
        </section>
      </div>
      <footer className="settings-foot">
        <span className="settings-about">
          {version ? `v${version}` : ''}
          {version && <span aria-hidden> · </span>}
          <button type="button" onClick={() => void invoke('open_url', { url: 'https://aiwings.app' })}>aiwings.app</button>
        </span>
        {/* Round back button in the gear's spot: settings opens and closes from the same place. */}
        <button type="button" className="gear settings-back" aria-label={t.back} onClick={onBack}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
          </svg>
        </button>
      </footer>
    </>
  );
}
