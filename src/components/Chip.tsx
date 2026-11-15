import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { startVoice, useVoiceEnabled, voiceError, type VoiceEvent } from '../lib/voice';
import { t } from '../strings';

/** This chip's own window: each app has one, and events are sent to the one they concern. */
const win = getCurrentWebviewWindow();

type Phase = 'idle' | 'connecting' | 'recording' | 'processing';

/** Start and stop chimes for recording, as in Coffee CLI. */
const sounds: Record<'start' | 'stop', HTMLAudioElement | null> = { start: null, stop: null };
function chime(which: 'start' | 'stop') {
  const audio = sounds[which] ??= Object.assign(new Audio(`/sounds/voice-${which}.mp3`), { volume: 0.3 });
  audio.currentTime = 0;
  void audio.play().catch(() => {});
}

// Coffee CLI's waveform: dots 2px wide on a 4px step drifting left at 40px/s; each new dot takes the
// current loudness, so speech raises the dots and silence leaves a dotted line.
const WAVE_WIDTH = 64, WAVE_HEIGHT = 14, WAVE_STEP = 4, WAVE_DOT = 2, WAVE_SPEED = 40;

/** Reads the level from a ref in its own animation loop, so nothing re-renders per update. */
function VoiceWave({ level }: { level: React.RefObject<number> }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext('2d');
    if (!element || !context) return;
    const ratio = window.devicePixelRatio || 1;
    element.width = WAVE_WIDTH * ratio;
    element.height = WAVE_HEIGHT * ratio;
    const dots: number[] = new Array(Math.ceil(WAVE_WIDTH / WAVE_STEP) + 1).fill(0);
    let offset = 0, smoothed = 0, last = performance.now(), frame = 0;
    const draw = (now: number) => {
      const elapsed = Math.min(now - last, 100) / 1000;
      last = now;
      // Ease toward the latest level so a 100 ms update does not snap.
      smoothed += (level.current - smoothed) * Math.min(1, elapsed * 12);
      offset += elapsed * WAVE_SPEED;
      while (offset >= WAVE_STEP) { offset -= WAVE_STEP; dots.shift(); dots.push(smoothed); }
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, WAVE_WIDTH, WAVE_HEIGHT);
      context.fillStyle = getComputedStyle(element).color;
      dots.forEach((l, i) => {
        const h = WAVE_DOT + l * (WAVE_HEIGHT - WAVE_DOT);
        context.beginPath();
        context.roundRect(i * WAVE_STEP - offset, (WAVE_HEIGHT - h) / 2, WAVE_DOT, h, WAVE_DOT / 2);
        context.fill();
      });
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [level]);
  return <canvas ref={canvas} className="chip-voice-wave" style={{ width: WAVE_WIDTH, height: WAVE_HEIGHT }} aria-hidden />;
}

/** Shows `msg` where the user is looking (the bar of the app in use, or the hover panel). */
const toast = (msg: string) => void invoke('share_link', { link: null, msg });

/**
 * The draft's actions on the target's input box (save to Wings · voice · collapse/expand the bars), as one solid
 * pill, like Coffee CLI's composer. It stays on the app's input box while the app is in front, focused or not,
 * always bright; with the box empty, saving does nothing. `wings://chip` carries whether the box has text, or
 * null once it is gone.
 * While recording the pill becomes: cancel · waveform · done, with no words (Coffee CLI's look); the text is
 * pasted when done.
 */
export function Chip() {
  const [hasText, setHasText] = useState<boolean | null>(null);
  const voiceOn = useVoiceEnabled();
  const [phase, setPhase] = useState<Phase>('idle');
  const level = useRef(0);
  const session = useRef<{ id: number | null; cancelled: boolean; finishing?: boolean } | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  // A recording started by an earlier load of this page (it reloads on a language change) has nothing left
  // to stop it: end it.
  useEffect(() => { void invoke('voice_cancel_all'); }, []);
  // Whether the current app's bar is up: the arrow then collapses the bars, otherwise it brings them back.
  const [barUp, setBarUp] = useState(true);
  useEffect(() => {
    const un = win.listen<boolean>('wings://bar', e => setBarUp(e.payload));
    void un.then(() => invoke<boolean>('bar_state')).then(setBarUp, () => {});
    return () => { void un.then(f => f()); };
  }, []);
  useEffect(() => {
    // Listen first, then ask: an event sent while this page was still loading is not lost.
    const un = win.listen<boolean | null>('wings://chip', e => setHasText(e.payload));
    void un.then(() => invoke<boolean | null>('chip_state')).then(setHasText, () => {});
    return () => { void un.then(f => f()); };
  }, []);
  // The window is as wide as the pill (it is anchored on the right), so it covers no more of the input box.
  // The recording pill grows with its text, so measure again whenever that changes.
  useLayoutEffect(() => {
    if (ref.current) void invoke('set_chip_width', { width: ref.current.offsetWidth + 2 });
  }, [phase, voiceOn]);

  const enabled = !!hasText;
  const reset = () => { session.current = null; setPhase('idle'); level.current = 0; };
  const onVoice = (e: VoiceEvent) => {
    if (!session.current || session.current.cancelled) return;
    switch (e.kind) {
      case 'ready': chime('start'); setPhase('recording'); break;
      // The custom API also finishes on its own at its length limit; chime then too.
      case 'processing': if (!session.current.finishing) chime('stop'); setPhase('processing'); break;
      case 'level': level.current = e.level; break;
      case 'error': reset(); toast(voiceError(e.message)); break;
      case 'text':
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
    chime('stop');
    current.finishing = true;
    level.current = 0;
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
    return (
      <div ref={ref} className={`chip-ground chip-voice is-${phase}`}>
        <button type="button" className="chip-action chip-icon" aria-label={t.voiceCancel} onMouseDown={keepVoice} onClick={cancel}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" aria-hidden><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
        <span className="chip-voice-body" onMouseDown={keepVoice}><VoiceWave level={level} /></span>
        <button type="button" className="chip-action chip-icon" aria-label={t.voiceDone} aria-disabled={phase === 'processing'} onMouseDown={keepVoice} onClick={phase === 'processing' ? undefined : finish}>
          {phase === 'processing' ? (
            <svg className="chip-spin" viewBox="0 0 16 16" fill="none" aria-hidden><circle cx="8" cy="8" r="5.5" stroke="currentColor" strokeWidth="1.8" opacity="0.35" /><path d="M8 2.5 A5.5 5.5 0 0 1 13.5 8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" /></svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden><polyline points="20 6 9 17 4 12" /></svg>
          )}
        </button>
      </div>
    );
  }

  return (
    <div ref={ref} onMouseDown={press} className={`chip-ground${hasText === null ? ' is-leaving' : ''}`}>
      <button type="button" className="chip-action" aria-disabled={!enabled} onMouseDown={keep} onClick={enabled ? () => void invoke('capture_now') : undefined}>{t.saveToWings}</button>
      {/* Speaking works on an empty box too, so the microphone is never dimmed. */}
      {voiceOn && (
        <button type="button" className="chip-action chip-icon" aria-label={t.voiceStart} onMouseDown={keepVoice} onClick={record}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <rect x="9" y="2" width="6" height="12" rx="3" /><path d="M5 10v1a7 7 0 0 0 14 0v-1" /><line x1="12" y1="18" x2="12" y2="22" />
          </svg>
        </button>
      )}
      {/* Collapse or expand the bars; like the microphone it works on an empty box too. */}
      <button type="button" className="chip-action chip-icon" aria-label={barUp ? t.closeTip : t.barExpand} onMouseDown={keepVoice} onClick={() => void invoke('toggle_bars')}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          {/* ‹ while the bars are up, › once collapsed. */}
          <polyline points={barUp ? '15 6 9 12 15 18' : '9 6 15 12 9 18'} />
        </svg>
      </button>
    </div>
  );
}
