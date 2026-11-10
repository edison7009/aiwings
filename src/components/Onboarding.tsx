import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../strings';

/** macOS first launch: walks the user through granting Accessibility, then continues on its own. */
export function Onboarding({ onGranted }: { onGranted: () => void }) {
  const [asked, setAsked] = useState(false);

  useEffect(() => {
    const id = window.setInterval(() => {
      void invoke<boolean>('permission_granted', { prompt: false }).then(ok => { if (ok) onGranted(); });
    }, 1000);
    return () => window.clearInterval(id);
  }, [onGranted]);

  const open = () => {
    // The prompting check also adds Wings to the Accessibility list, so the user only flips a switch.
    void invoke('permission_granted', { prompt: true });
    void invoke('open_permission_settings');
    setAsked(true);
  };

  return (
    <>
      <header className="panel-head" data-tauri-drag-region>
        <span className="brand" data-tauri-drag-region>Wings · 候话</span>
      </header>
      <div className="panel-body onboarding">
        <h1>{t.permTitle}</h1>
        <p className="muted">{t.permBody}</p>
        <ol>
          <li>{t.permStep1}</li>
          <li>{t.permStep2}</li>
          <li>{t.permStep3}</li>
        </ol>
        <button type="button" className="primary" onClick={open}>{t.permButton}</button>
        {asked && <p className="muted center">{t.permWaiting}</p>}
      </div>
    </>
  );
}
