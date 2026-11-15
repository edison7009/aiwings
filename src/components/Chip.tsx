import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { shareLink } from '../lib/share';
import { t } from '../strings';

/**
 * The draft's actions on the target's input box (save to Wings · share link · clear), as one solid pill,
 * like Coffee CLI's composer. It is there whenever an input box is focused; while the box is empty it
 * darkens and does nothing. `wings://chip` carries whether the box has text, or null once it is gone.
 */
export function Chip() {
  const [hasText, setHasText] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Listen first, then ask: an event sent while this page was still loading is not lost.
    const un = listen<boolean | null>('wings://chip', e => setHasText(e.payload));
    void un.then(() => invoke<boolean | null>('chip_state')).then(setHasText, () => {});
    return () => { void un.then(f => f()); };
  }, []);
  // The window is as wide as the pill (it is anchored on the right), so it covers no more of the input box.
  useLayoutEffect(() => {
    if (ref.current) void invoke('set_chip_width', { width: ref.current.offsetWidth + 2 });
  }, []);

  const enabled = !!hasText && !busy;
  const share = async () => {
    setBusy(true);
    try {
      const text = await invoke<string | null>('input_text');
      if (!text) return;
      const link = await shareLink(text);
      await invoke('share_link', link ? { link, msg: t.linkCopied } : { link: null, msg: t.shareTooLong });
    } finally { setBusy(false); }
  };
  const keep = (e: React.MouseEvent) => e.preventDefault();
  // aria-disabled, not disabled: a disabled button swallows the mouse-down this needs.
  // Pressing anywhere on the pill makes Wings active for a moment; the pill holds still meanwhile, and a
  // press on the dark pill gives the caret straight back to the input box.
  const press = () => void invoke('chip_pressed', { refocus: !enabled });

  return (
    <div ref={ref} onMouseDown={press} className={`chip-ground${hasText === null ? ' is-leaving' : ''}`} data-enabled={enabled || undefined}>
      <button type="button" className="chip-action" aria-disabled={!enabled} onMouseDown={keep} onClick={enabled ? () => void invoke('capture_now') : undefined}>{t.saveToWings}</button>
      <button type="button" className="chip-action chip-icon" aria-disabled={!enabled} aria-label={t.shareLink} onMouseDown={keep} onClick={enabled ? () => void share() : undefined}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M4 12v7a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-7" /><polyline points="16 6 12 2 8 6" /><line x1="12" y1="2" x2="12" y2="15" />
        </svg>
      </button>
      {/* Clearing by hand is slow; this selects all and deletes in the target's input box. */}
      <button type="button" className="chip-action chip-icon" aria-disabled={!enabled} aria-label={t.clear} onMouseDown={keep} onClick={enabled ? () => void invoke('clear_input') : undefined}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <polyline points="3 6 5 6 21 6" />
          <path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6" />
          <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
        </svg>
      </button>
    </div>
  );
}
