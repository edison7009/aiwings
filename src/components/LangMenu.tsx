import type { Prefs } from '../lib/prefs';
import { t, LANGS } from '../strings';
import { PickMenu } from './PickMenu';

/** Language picker: "follow the system", then each language in its own words. */
export function LangMenu({ value, onChange }: { value: Prefs['lang']; onChange: (v: Prefs['lang']) => void }) {
  return <PickMenu className="lang-menu" value={value} options={[['auto', t.langAuto], ...LANGS]} onChange={onChange} />;
}
