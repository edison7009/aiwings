import type { Prefs } from '../lib/prefs';
import type { Target } from './TargetPicker';
import { t } from '../strings';
import { Segmented } from './Segmented';
import { LAYOUTS } from '../lib/layouts';
import { AppMenu } from './AppMenu';
import { ShortcutRecorder } from './ShortcutRecorder';

interface Props {
  prefs: Prefs;
  shortcut: string;
  target: Target;
  onChange: (p: Prefs) => void;
  onPickTarget: (t: Target) => void;
  onError: (msg: string) => void;
  onShortcut: (label: string) => void;
  onBack: () => void;
}

export function Settings({ prefs, shortcut, target, onChange, onPickTarget, onError, onShortcut, onBack }: Props) {
  const set = <K extends keyof Prefs>(k: K, v: Prefs[K]) => onChange({ ...prefs, [k]: v });
  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <button type="button" className="ghost" onClick={onBack}>‹ {t.back}</button>
        <span className="brand" data-tauri-drag-region>{t.settings}</span>
      </header>
      <div className="panel-body settings">
        {/* Layout first. Under it, ticks choose the apps to follow, or (pinned) the one app to stay with. */}
        <section>
          <Segmented value={prefs.layout} options={LAYOUTS} onChange={v => set('layout', v)} />
          <p className="muted small hint">{t.layoutHint[prefs.layout]}</p>
          {prefs.layout !== 'pin'
            ? <AppMenu mode="multi" followKeys={prefs.followKeys} onChangeKeys={keys => set('followKeys', keys)} />
            : <AppMenu mode="single" current={target} onPick={onPickTarget} onError={onError} />}
        </section>
        <section>
          <label>{t.chipLabel}</label>
          <Segmented value={prefs.chip ? 'on' : 'off'} options={[['on', t.show], ['off', t.hide]]} onChange={v => set('chip', v === 'on')} />
        </section>
        <section>
          <label>{t.theme}</label>
          <Segmented value={prefs.theme} options={[['system', t.themeSystem], ['light', t.themeLight], ['dark', t.themeDark]]} onChange={v => set('theme', v)} />
        </section>
        <section>
          <label>{t.shortcut}</label>
          <ShortcutRecorder label={shortcut} onChange={onShortcut} onError={onError} />
        </section>
      </div>
    </>
  );
}
