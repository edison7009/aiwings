import type { Prefs } from '../lib/prefs';
import { t } from '../strings';
import { Segmented } from './Segmented';

interface Props {
  prefs: Prefs;
  shortcut: string;
  targetName: string;
  onChange: (p: Prefs) => void;
  onSwitchTarget: () => void;
  onBack: () => void;
}

export function Settings({ prefs, shortcut, targetName, onChange, onSwitchTarget, onBack }: Props) {
  const set = <K extends keyof Prefs>(k: K, v: Prefs[K]) => onChange({ ...prefs, [k]: v });
  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <button type="button" className="ghost" onClick={onBack}>‹ {t.back}</button>
        <span className="brand" data-tauri-drag-region>{t.settings}</span>
      </header>
      <div className="panel-body settings">
        <section>
          <label>{t.target}</label>
          <div className="row">
            <span className="row-text">{targetName}</span>
            <button type="button" className="ghost" onClick={onSwitchTarget}>{t.change}</button>
          </div>
        </section>
        <section>
          <label>{t.followLabel}</label>
          <Segmented value={prefs.follow ? 'on' : 'off'} options={[['off', t.followOff], ['on', t.followOn]]} onChange={v => set('follow', v === 'on')} />
          <p className="muted small">{prefs.follow ? t.followOnHint : t.followOffHint}</p>
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
