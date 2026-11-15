import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { shareLink } from '../lib/share';
import { startVoice, useVoiceEnabled, type VoiceEvent } from '../lib/voice';
import { t } from '../strings';

type Phase = 'idle' | 'connecting' | 'recording' | 'processing';

/** Shows `msg` where the user is looking (the bar of the app in use, or the hover panel). */
const toast = (msg: string) => void invoke('share_link', { link: null, msg });

/** The service's error code as words; anything unknown is shown as it came. */
function voiceError(message: string) {
  const known = (t.voiceErrors as Record<string, string>)[message];
  return known ?? t.voiceFailed(message);
}

/**
 * The draft's actions on the target's input box (save to Wings · share link · clear · voice), as one solid
 * pill, like Coffee CLI's composer. It is there whenever an input box is focused; while the box is empty it
 * darkens and only the microphone works. `wings://chip` carries whether the box has text, or null once it is gone.
 * While recording the pill becomes: cancel · what was heard so far · done; the text is pasted when done.
 */
export function Chip() {
  const [hasText, setHasText] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const voiceOn = useVoiceEnabled();
  const [phase, setPhase] = useState<Phase>('idle');
  const [heard, setHeard] = useState('');
  const [level, setLevel] = useState(0);
  const session = useRef<{ id: number | null; cancelled: boolean } | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Listen first, then ask: an event sent while this page was still loading is not lost.
    const un = listen<boolean | null>('wings://chip', e => setHasText(e.payload));
    void un.then(() => invoke<boolean | null>('chip_state')).then(setHasText, () => {});
    return () => { void un.then(f => f()); };
  }, []);
  // The window is as wide as the pill (it is anchored on the right), so it covers no more of the input box.
  // The recording pill grows with its text, so measure again whenever that changes.
  useLayoutEffect(() => {
    if (ref.current) void invoke('set_chip_width', { width: ref.current.offsetWidth + 2 });
  }, [phase, heard, voiceOn]);

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

  const reset = () => { session.current = null; setPhase('idle'); setHeard(''); setLevel(0); };
  const onVoice = (e: VoiceEvent) => {
    if (!session.current || session.current.cancelled) return;
    switch (e.kind) {
      case 'ready': setPhase('recording'); break;
      case 'processing': setPhase('processing'); break;
      case 'level': setLevel(e.level); break;
      case 'error': reset(); toast(voiceError(e.message)); break;
      case 'text':
        setHeard(e.text);
        if (!e.done) break;
        reset();
        if (e.text.trim()) void invoke('paste_note', { text: e.text.trim(), image: null });
        else toast(t.voiceNothing);
    }
  };
  const record = () => {
    const current = { id: null as number | null, cancelled: false };
    session.current = current;
    setPhase('connecting');
    startVoice(onVoice).then(id => {
      current.id = id;
      if (current.cancelled) void invoke('voice_cancel', { id });
    }, (err: unknown) => { if (session.current === current) { reset(); toast(voiceError(String(err))); } });
  };
  const finish = () => {
    const current = session.current;
    if (!current) return;
    // Nothing heard yet (still connecting): there is nothing to wait for.
    if (current.id == null || phase === 'connecting') { cancel(); return; }
    setPhase('processing');
    void invoke('voice_finish', { id: current.id });
  };
  const cancel = () => {
    const current = session.current;
    if (current) { current.cancelled = true; if (current.id != null) void invoke('voice_cancel', { id: current.id }); }
    reset();
  };

  const keep = (e: React.MouseEvent) => e.preventDefault();
  // The voice buttons always give the caret back to the input box: that is where the text goes.
  const keepVoice = (e: React.MouseEvent) => { e.preventDefault(); e.stopPropagation(); void invoke('chip_pressed', { refocus: true }); };
  // aria-disabled, not disabled: a disabled button swallows the mouse-down this needs.
  // Pressing anywhere on the pill makes Wings active for a moment; the pill holds still meanwhile, and a
  // press on the dark pill gives the caret straight back to the input box.
  const press = () => void invoke('chip_pressed', { refocus: !enabled });

  if (phase !== 'idle') {
    const status = phase === 'connecting' ? t.voiceConnecting : phase === 'processing' ? t.voiceProcessing : heard ? `…${heard.slice(-16)}` : t.voiceListening;
    return (
      <div ref={ref} className="chip-ground chip-voice" data-enabled>
        <button type="button" className="chip-action chip-icon" aria-label={t.voiceCancel} onMouseDown={keepVoice} onClick={cancel}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" aria-hidden><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
        <span className="chip-voice-text" onMouseDown={keepVoice}>
          <span className="chip-voice-level" style={{ '--level': phase === 'recording' ? level : 0 } as React.CSSProperties} aria-hidden><i /><i /><i /></span>
          {status}
        </span>
        <button type="button" className="chip-action chip-icon" aria-label={t.voiceDone} aria-disabled={phase === 'processing'} onMouseDown={keepVoice} onClick={phase === 'processing' ? undefined : finish}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden><polyline points="20 6 9 17 4 12" /></svg>
        </button>
      </div>
    );
  }

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
      {/* Speaking works on an empty box too, so the microphone is never dimmed. */}
      {voiceOn && (
        <button type="button" className="chip-action chip-icon chip-mic" aria-label={t.voiceStart} onMouseDown={keepVoice} onClick={record}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <rect x="9" y="2" width="6" height="12" rx="3" /><path d="M5 10v1a7 7 0 0 0 14 0v-1" /><line x1="12" y1="18" x2="12" y2="22" />
          </svg>
        </button>
      )}
    </div>
  );
}
