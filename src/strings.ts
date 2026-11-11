// UI copy. `t` holds the current language's strings and is swapped in place when the language changes,
// so every `t.x` read at render time follows. Simplified Chinese (src/i18n/zh.ts) is the source.
import { zh, type Strings } from './i18n/zh';
import { zhTW } from './i18n/zhTW';
import { en } from './i18n/en';
import { ja } from './i18n/ja';
import { ko } from './i18n/ko';
import { es } from './i18n/es';
import { fr } from './i18n/fr';
import { de } from './i18n/de';
import { pt } from './i18n/pt';
import { ru } from './i18n/ru';

const LOCALES = { zh, 'zh-TW': zhTW, en, ja, ko, es, fr, de, pt, ru } satisfies Record<string, Strings>;
export type Lang = keyof typeof LOCALES;

/** The language picker's list, each in its own language. */
export const LANGS: [Lang, string][] = [
  ['zh', '简体中文'], ['zh-TW', '繁體中文'], ['en', 'English'], ['ja', '日本語'], ['ko', '한국어'],
  ['es', 'Español'], ['fr', 'Français'], ['de', 'Deutsch'], ['pt', 'Português'], ['ru', 'Русский'],
];

/** The system's language, or English when Wings has no translation for it. */
function systemLang(): Lang {
  const tag = (navigator.language || 'en').toLowerCase();
  if (tag.startsWith('zh')) return /tw|hk|mo|hant/.test(tag) ? 'zh-TW' : 'zh';
  const base = tag.split('-')[0];
  return base in LOCALES ? base as Lang : 'en';
}

export const t: Strings = { ...zh };
let errorPattern: RegExp | null = null;

/** Switches every string to `lang` ('auto' follows the system). */
export function setLang(lang: Lang | 'auto') {
  const code = lang === 'auto' ? systemLang() : lang;
  Object.assign(t, LOCALES[code] ?? zh);
  document.documentElement.lang = code === 'zh' ? 'zh-CN' : code;
  const keys = Object.keys(t.errors).sort((a, b) => b.length - a.length);
  errorPattern = keys.length ? new RegExp(keys.map(k => k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'g') : null;
}

/** Messages from the Rust side are Chinese; this swaps each known piece for the current language's words. */
export function tr(msg: string): string {
  return errorPattern ? msg.replace(errorPattern, m => t.errors[m] ?? m) : msg;
}

// Set before the first render, from the saved setting (read directly: prefs.ts imports this module).
try { setLang(JSON.parse(localStorage.getItem('wings-prefs') ?? '{}').lang ?? 'auto'); } catch { setLang('auto'); }
