import type { Prefs } from './prefs';
import { t } from '../strings';

/** One two-way choice, in the header and in settings alike, shown as words like every other switch.
 *  A function so the names follow the current language. */
export const layouts = (): [Prefs['layout'], string][] => [
  ['follow', t.layoutFollow],
  ['float', t.layoutFloat],
];
