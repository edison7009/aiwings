// Colour palettes for the settings tiles (background, card and accent of each half). Keep in sync with the [data-palette] blocks at the end of styles.css.
export const PALETTES = [
  // The default's tile is neutral stone grey: its blue accent belongs to the buttons, not to the palette.
  { id: 'basalt', day: { bg: '#f4f2ee', card: '#ffffff', accent: '#9c978d' }, night: { bg: '#1d1c1a', card: '#2b2a27', accent: '#6b6860' } },
  { id: 'wings', day: { bg: '#fbf3f7', card: '#ffffff', accent: '#e0367d' }, night: { bg: '#231a20', card: '#2d2229', accent: '#f84890' } },
  { id: 'sakura', day: { bg: '#f6f1f3', card: '#ffffff', accent: '#773b4d' }, night: { bg: '#262024', card: '#2f292d', accent: '#dab2be' } },
  { id: 'amber', day: { bg: '#f7f3ec', card: '#ffffff', accent: '#7f5c34' }, night: { bg: '#272219', card: '#302a20', accent: '#d2b28e' } },
  { id: 'mint', day: { bg: '#eff4f1', card: '#ffffff', accent: '#437059' }, night: { bg: '#1f2723', card: '#27302b', accent: '#b0c9bc' } },
  { id: 'glacier', day: { bg: '#eff3f7', card: '#ffffff', accent: '#34567f' }, night: { bg: '#20252d', card: '#283039', accent: '#adc3dd' } },
  { id: 'lavender', day: { bg: '#f4f1f6', card: '#ffffff', accent: '#5f3d75' }, night: { bg: '#25212b', card: '#2d2933', accent: '#c7b3d4' } },
  { id: 'copper', day: { bg: '#f8f2ee', card: '#ffffff', accent: '#844425' }, night: { bg: '#23150e', card: '#2d1d15', accent: '#de9673' } },
  { id: 'obsidian', day: { bg: '#eaecee', card: '#ffffff', accent: '#3c4a5d' }, night: { bg: '#0a0a0a', card: '#171717', accent: '#b0b0b0' } },
] as const;

export type PaletteId = typeof PALETTES[number]['id'];
