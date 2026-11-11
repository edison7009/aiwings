import type { Prefs } from '../lib/prefs';
import type { Target } from './TargetPicker';
import { t } from '../strings';
import { Segmented } from './Segmented';
import { AppMenu } from './AppMenu';

interface Props {
  prefs: Prefs;
  shortcut: string;
  target: Target;
  onChange: (p: Prefs) => void;
  onPickTarget: (t: Target) => void;
  onError: (msg: string) => void;
  onBack: () => void;
}

export function Settings({ prefs, shortcut, target, onChange, onPickTarget, onError, onBack }: Props) {
  const set = <K extends keyof Prefs>(k: K, v: Prefs[K]) => onChange({ ...prefs, [k]: v });
  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <button type="button" className="ghost" onClick={onBack}>‹ {t.back}</button>
        <span className="brand" data-tauri-drag-region>{t.settings}</span>
      </header>
      <div className="panel-body settings">
        {/* Follow first. Under it, ticks choose which apps to follow, or (when off) the one app to stay with. */}
        <section>
          <Segmented value={prefs.follow ? 'on' : 'off'} options={[['on', t.followOn], ['off', t.followOff]]} onChange={v => set('follow', v === 'on')} />
          <p className="muted small hint">{prefs.follow ? t.followOnHint : t.followOffHint}</p>
          {prefs.follow
            ? <AppMenu mode="multi" followKeys={prefs.followKeys} onChangeKeys={keys => set('followKeys', keys)} />
            : <AppMenu mode="single" current={target} onPick={onPickTarget} onError={onError} />}
        </section>
        <section>
          <label>{t.chipLabel}</label>
          <Segmented value={prefs.chip ? 'on' : 'off'} options={[['on', t.show], ['off', t.hide]]} onChange={v => set('chip', v === 'on')} />
        </section>
        <section>
          <label>{t.shortcut}</label>
          <div className="row"><kbd>{shortcut}</kbd></div>
        </section>
      </div>
    </>
  );
}
