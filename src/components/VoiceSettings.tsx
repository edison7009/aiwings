// Settings → Voice: the switch, the microphone and a live check of it, and the list of speech-to-text
// accounts (Claude Code / Codex sign-ins found on this machine, the user's own API key, macOS dictation),
// laid out after Coffee CLI's voice settings. Secrets stay in the Rust side; only hints reach this page.
import { useEffect, useState } from 'react';
import { Channel, invoke } from '@tauri-apps/api/core';
import { t } from '../strings';
import { Segmented } from './Segmented';
import { PickMenu } from './PickMenu';
import { cachedVoiceSources, scanVoiceSources, setVoiceDevice, voiceError, setVoiceEnabled, setVoiceSource, useVoiceDevice, useVoiceEnabled, useVoiceSource, type OwnSourceId, type VoiceSource } from '../lib/voice';

const OWN = ['claude', 'codex', 'api', 'macos'];
const sourceName = (s: VoiceSource) => OWN.includes(s.id) ? t.voiceSourceNames[s.id as OwnSourceId] : s.name;

interface ApiView { base: string; model: string; keyHint: string | null }

function MicIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <rect x="9" y="2" width="6" height="12" rx="3" /><path d="M5 10v1a7 7 0 0 0 14 0v-1" /><line x1="12" y1="18" x2="12" y2="22" />
    </svg>
  );
}

/** The user's own OpenAI-compatible endpoint. The saved key is never shown, only its hint. */
function ApiForm({ onSaved, onError }: { onSaved: () => void; onError: (msg: string) => void }) {
  const [view, setView] = useState<ApiView | null>(null);
  const [base, setBase] = useState('');
  const [model, setModel] = useState('');
  const [key, setKey] = useState('');
  useEffect(() => {
    void invoke<ApiView>('voice_api_get').then(v => { setView(v); setBase(v.base); setModel(v.model); });
  }, []);
  if (!view) return null;
  const save = (removeKey = false) => {
    const k = removeKey ? '' : key.trim() || null;
    void invoke('voice_api_set', { base, model, key: k }).then(() => { setKey(''); onSaved(); }, e => onError(String(e)));
  };
  return (
    <div className="voice-api">
      <p className="muted small">{t.voiceApiHint}</p>
      <label className="voice-field"><span>{t.voiceApiBase}</span><input value={base} spellCheck={false} onChange={e => setBase(e.target.value)} /></label>
      <label className="voice-field"><span>{t.voiceApiKey}</span>
        <input type="password" value={key} spellCheck={false} placeholder={view.keyHint ? t.voiceApiKeyKeep(view.keyHint) : 'sk-…'} onChange={e => setKey(e.target.value)} />
      </label>
      <label className="voice-field"><span>{t.voiceApiModel}</span><input value={model} spellCheck={false} onChange={e => setModel(e.target.value)} /></label>
      <div className="voice-api-actions">
        {view.keyHint && <button type="button" className="ghost" onClick={() => save(true)}>{t.voiceApiRemove}</button>}
        <button type="button" className="voice-btn" disabled={!view.keyHint && !key.trim()} onClick={() => save()}>{t.voiceApiSave}</button>
      </div>
    </div>
  );
}

export function VoiceSettings({ onError }: { onError: (msg: string) => void }) {
  const enabled = useVoiceEnabled();
  const device = useVoiceDevice();
  const source = useVoiceSource();
  const [sources, setSources] = useState<VoiceSource[] | null>(cachedVoiceSources);
  const [scanning, setScanning] = useState(false);
  const [editApi, setEditApi] = useState(false);
  const [mics, setMics] = useState<{ devices: string[]; default: string | null } | null>(null);
  const [testing, setTesting] = useState(false);
  const [level, setLevel] = useState(0);

  // "Refresh": signing in or out happens in the tools themselves, so look again. Without a saved scan,
  // the first look happens on its own.
  const scan = async () => {
    setScanning(true);
    try { setSources(await scanVoiceSources()); } catch (e) { onError(voiceError(String(e))); }
    finally { setScanning(false); }
  };
  useEffect(() => { if (enabled && !cachedVoiceSources()) void scan(); }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    let live = true;
    const load = () => void invoke<{ devices: string[]; default: string | null }>('voice_devices').then(list => { if (live) setMics(list); }, () => {});
    load();
    // A headset plugged in or out shows up the next time the window gets focus.
    window.addEventListener('focus', load);
    return () => { live = false; window.removeEventListener('focus', load); };
  }, [enabled]);

  // The check runs while on and restarts on the chosen microphone; it stops on leaving the page.
  useEffect(() => {
    if (!testing || !enabled) return;
    let id: number | null = null, stopped = false;
    const channel = new Channel<number>();
    channel.onmessage = l => { if (!stopped) setLevel(l); };
    void invoke<number>('voice_mic_test_start', { device: device || null, onLevel: channel }).then(started => {
      id = started;
      if (stopped) void invoke('voice_mic_test_stop', { id: started });
    }, () => { if (!stopped) { setTesting(false); onError(t.voiceErrors.no_microphone); } });
    return () => { stopped = true; setLevel(0); if (id != null) void invoke('voice_mic_test_stop', { id }); };
  }, [testing, device, enabled, onError]);

  const micOptions: [string, string][] = [['', mics?.default ? t.micDefaultNamed(mics.default) : t.micDefault], ...(mics?.devices ?? []).map(n => [n, n] as [string, string])];
  // A saved microphone that is unplugged stays listed, so the choice is not silently lost.
  if (device && !micOptions.some(([v]) => v === device)) micOptions.push([device, t.micMissing(device)]);

  const status = (s: VoiceSource) => {
    if (s.issue === 'expired') return t.voiceExpired;
    if (s.issue === 'busy') return t.voiceBusy;
    if (s.issue === 'unset') return t.voiceUnset;
    if (s.issue === 'missing') return s.id === 'macos' ? t.voiceUnavailable : t.voiceMissing;
    if (s.id === 'macos') return t.voiceSystem;
    return s.accountId ? t.voiceAccount(s.accountId) : t.voiceSignedIn;
  };
  // An unusable source can still be picked when it only needs a key, so the form opens straight away.
  const pick = (s: VoiceSource) => {
    if (s.id === 'api' && s.issue) { setEditApi(true); return; }
    if (s.issue) return;
    // Doubao: its session is copied into Wings' own store first (Coffee CLI's way).
    if (s.id === 'doubao' || s.id === 'doubaowork') {
      void invoke('voice_select_source', { source: s.id }).then(() => setVoiceSource(s.id), e => onError(voiceError(String(e))));
      return;
    }
    setVoiceSource(s.id);
  };

  return (
    <>
      <section>
        <label>{t.voiceInput}</label>
        <Segmented value={enabled ? 'on' : 'off'} options={[['on', t.on], ['off', t.off]]} onChange={v => { setVoiceEnabled(v === 'on'); if (v === 'off') setTesting(false); }} />
        <p className="muted small hint">{t.voiceInputHint}</p>
      </section>
      {enabled && <>
        <section>
          <label>{t.microphone}</label>
          <PickMenu className="app-menu--overlay" value={device} options={micOptions} onChange={setVoiceDevice} />
          <div className="voice-test">
            <button type="button" className={`voice-btn${testing ? ' is-on' : ''}`} aria-pressed={testing} onClick={() => setTesting(!testing)}>
              <MicIcon />{testing ? t.micTestStop : t.micTest}
            </button>
            <span className="voice-meter" aria-hidden><span style={{ width: `${Math.round(level * 100)}%` }} /></span>
          </div>
        </section>
        <section>
          <div className="voice-head">
            <span>{t.voiceSourceLabel}</span>
            <button type="button" className="ghost" disabled={scanning} aria-busy={scanning} onClick={() => void scan()}>{scanning ? t.voiceRefreshing : t.voiceRefresh}</button>
          </div>
          <div className="voice-sources" role="group" aria-label={t.voiceSourceLabel}>
            <button type="button" className="voice-source" aria-pressed={source === ''} onClick={() => setVoiceSource('')}>
              <span className="voice-source-name">{t.voiceAuto}</span>
              <span className="voice-source-status">{t.voiceAutoHint}</span>
            </button>
            {(sources ?? []).map(s => (
              <div key={s.id} className="voice-source-wrap">
                <button type="button" className="voice-source" aria-pressed={source === s.id} data-issue={s.issue ?? undefined} onClick={() => pick(s)}>
                  <span className="voice-source-name">{sourceName(s)}<span className="voice-source-mode">{s.transcriptionMode === 'batch' ? t.voiceBatch : t.voiceStreaming}</span></span>
                  <span className="voice-source-status">{status(s)}</span>
                </button>
                {s.id === 'api' && <button type="button" className="ghost voice-source-edit" aria-expanded={editApi} onClick={() => setEditApi(!editApi)}>{t.voiceApiEdit}</button>}
              </div>
            ))}
          </div>
          {editApi && <ApiForm onError={onError} onSaved={() => { setEditApi(false); void scan(); }} />}
          {sources && source && !sources.some(s => s.id === source && !s.issue) && <p className="muted small hint">{t.voiceSourceGone}</p>}
        </section>
      </>}
    </>
  );
}
