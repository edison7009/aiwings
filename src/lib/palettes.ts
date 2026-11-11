// Colour palettes for the settings tiles (background, card and accent of each half). Keep in sync with the [data-palette] blocks at the end of styles.css.
export const PALETTES = [
  { id: 'basalt', name: '玄武岩', day: { bg: '#f4f2ee', card: '#ffffff', accent: '#007aff' }, night: { bg: '#1d1c1a', card: '#2b2a27', accent: '#007aff' } },
  { id: 'wings', name: 'Wings 粉', day: { bg: '#fbf3f7', card: '#ffffff', accent: '#e0367d' }, night: { bg: '#231a20', card: '#2d2229', accent: '#f84890' } },
  { id: 'sakura', name: '樱花', day: { bg: '#f6f1f3', card: '#ffffff', accent: '#773b4d' }, night: { bg: '#262024', card: '#2f292d', accent: '#dab2be' } },
  { id: 'amber', name: '琥珀', day: { bg: '#f7f3ec', card: '#ffffff', accent: '#7f5c34' }, night: { bg: '#272219', card: '#302a20', accent: '#d2b28e' } },
  { id: 'mint', name: '薄荷', day: { bg: '#eff4f1', card: '#ffffff', accent: '#437059' }, night: { bg: '#1f2723', card: '#27302b', accent: '#b0c9bc' } },
  { id: 'glacier', name: '冰川', day: { bg: '#eff3f7', card: '#ffffff', accent: '#34567f' }, night: { bg: '#20252d', card: '#283039', accent: '#adc3dd' } },
  { id: 'lavender', name: '薰衣草', day: { bg: '#f4f1f6', card: '#ffffff', accent: '#5f3d75' }, night: { bg: '#25212b', card: '#2d2933', accent: '#c7b3d4' } },
] as const;

export type PaletteId = typeof PALETTES[number]['id'];
