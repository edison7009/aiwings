// Theme hook-up point. Today every app gets the neutral Wings theme, which follows the system
// light/dark mode through the CSS tokens in styles.css. To match a docked app's own palette later,
// return overrides for its key (bundle id on macOS, exe name on Windows) from themeFor().

/** CSS custom properties a theme may override; anything left out keeps the stylesheet default. */
export interface ThemeTokens {
  /** Main accent (buttons, links). */
  '--accent'?: string;
  /** Pointer-following glow behind a hovered note. */
  '--hover-glow'?: string;
  /** Note card background and its hovered state. */
  '--card'?: string;
  '--card-hover'?: string;
}

const APP_THEMES: Record<string, ThemeTokens> = {
  // Example for later: 'com.anthropic.claudefordesktop': { '--accent': '#d97757' },
};

export function themeFor(appKey: string | null): ThemeTokens {
  return (appKey && APP_THEMES[appKey]) || {};
}

/** Applies a theme on the root element, clearing tokens a previous theme set. */
export function applyTheme(tokens: ThemeTokens) {
  const root = document.documentElement.style;
  for (const name of ['--accent', '--hover-glow', '--card', '--card-hover'] as const) {
    const value = tokens[name];
    if (value) root.setProperty(name, value); else root.removeProperty(name);
  }
}
