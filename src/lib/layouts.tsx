import type { ReactNode } from 'react';
import type { Prefs } from './prefs';
import { t } from '../strings';

const svg = { width: 15, height: 15, viewBox: '0 0 16 16', fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinecap: 'round', strokeLinejoin: 'round', 'aria-hidden': true } as const;

/** Follow: an app window with Wings docked as a solid strip on its right. */
const FollowIcon = <svg {...svg}><rect x="1.75" y="2.75" width="9" height="10.5" rx="1.5" /><rect x="12.25" y="2.75" width="2.5" height="10.5" rx="1" fill="currentColor" stroke="none" /></svg>;
/** Lock: a padlock, it stays with the chosen apps. */
const LockIcon = <svg {...svg}><rect x="3" y="7" width="10" height="7" rx="1.5" /><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" /></svg>;
/** Hover: a small window floating over a larger one. */
const FloatIcon = <svg {...svg}><rect x="2" y="2.5" width="12" height="9" rx="1.5" strokeDasharray="2 1.6" /><rect x="7" y="7.5" width="7" height="6" rx="1.2" fill="var(--card)" /></svg>;

/** One three-way choice, in the header and in settings alike: icons, with the name as a hover tip. */
export const LAYOUTS: [Prefs['layout'], string, ReactNode][] = [
  ['follow', t.layoutFollow, FollowIcon],
  ['lock', t.layoutLock, LockIcon],
  ['float', t.layoutFloat, FloatIcon],
];
