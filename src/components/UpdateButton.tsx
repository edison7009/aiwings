import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { t } from '../strings';

interface Progress { status: 'speed_test' | 'downloading' | 'launching' | 'downloaded' | 'error'; percent: number }

const R = 9;
const C = 2 * Math.PI * R;

/** Red download button beside the gear when a newer version is out, as in EchoBird and Coffee CLI:
 *  click it and the ring spins while finding a source, then fills with the download. */
export function UpdateButton({ onError }: { onError: (msg: string) => void }) {
  const [version, setVersion] = useState<string | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);

  useEffect(() => {
    const check = () => void invoke<string | null>('check_update').then(setVersion).catch(() => {});
    check();
    const timer = window.setInterval(check, 6 * 60 * 60 * 1000);
    const sub = listen<Progress>('wings://update-progress', e => setProgress(e.payload));
    return () => { window.clearInterval(timer); void sub.then(f => f()); };
  }, []);

  if (!version) return null;
  const busy = !!progress && progress.status !== 'error' && progress.status !== 'downloaded';
  const spinning = progress?.status === 'speed_test';
  const download = () => {
    if (busy) return;
    setProgress({ status: 'speed_test', percent: 0 });
    void invoke('download_update', { version }).then(
      () => setProgress(null),
      e => { setProgress(null); onError(`${t.updateFailed}${e ? `：${e}` : ''}`); },
    );
  };

  return (
    <button type="button" className="update" aria-label={`${busy ? t.updateDownloading : t.updateDownload} v${version}`} aria-busy={busy} onClick={download}>
      {busy ? (
        <svg width="15" height="15" viewBox="0 0 24 24" className={spinning ? 'spin' : undefined} aria-hidden>
          <circle cx="12" cy="12" r={R} fill="none" stroke="currentColor" strokeOpacity="0.22" strokeWidth="2.6" />
          <circle cx="12" cy="12" r={R} fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" transform="rotate(-90 12 12)"
            strokeDasharray={spinning ? `${C * 0.25} ${C}` : C} strokeDashoffset={spinning ? 0 : C * (1 - (progress?.percent ?? 0) / 100)} />
        </svg>
      ) : (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      )}
    </button>
  );
}
