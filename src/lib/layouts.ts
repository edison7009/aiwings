import type { Prefs } from './prefs';
import { t } from '../strings';

/** One three-way choice, in the header and in settings alike. */
export const LAYOUTS: [Prefs['layout'], string][] = [['follow', t.layoutFollow], ['pin', t.layoutPin], ['float', t.layoutFloat]];
