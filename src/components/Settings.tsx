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
      {/* Back sits where the gear was, so opening and leaving settings is one spot. */}
      <footer className="settings-foot">
        <button type="button" onClick={onBack}>‹ {t.back}</button>
        <span className="settings-version">{version ? `v${version}` : ''}</span>
        <button type="button" onClick={() => void invoke('open_url', { url: 'https://aiwings.app' })}>
          aiwings.app
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" /><polyline points="15 3 21 3 21 9" /><line x1="10" y1="14" x2="21" y2="3" />
          </svg>
        </button>
      </footer>
    </>
  );
}
