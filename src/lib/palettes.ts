// Colour palettes for the settings swatches. Keep in sync with the [data-palette] blocks at the end of styles.css.
export const PALETTES = [
  { id: 'basalt', name: '玄武岩', day: ['#f4f2ee', '#007aff'], night: ['#1d1c1a', '#007aff'] },
  { id: 'wings', name: 'Wings 粉', day: ['#fbf3f7', '#e0367d'], night: ['#231a20', '#f84890'] },
  { id: 'sakura', name: '樱花', day: ['#f6f1f3', '#773b4d'], night: ['#262024', '#dab2be'] },
  { id: 'amber', name: '琥珀', day: ['#f7f3ec', '#7f5c34'], night: ['#272219', '#d2b28e'] },
  { id: 'mint', name: '薄荷', day: ['#eff4f1', '#437059'], night: ['#1f2723', '#b0c9bc'] },
  { id: 'glacier', name: '冰川', day: ['#eff3f7', '#34567f'], night: ['#20252d', '#adc3dd'] },
  { id: 'lavender', name: '薰衣草', day: ['#f4f1f6', '#5f3d75'], night: ['#25212b', '#c7b3d4'] },
] as const;

export type PaletteId = typeof PALETTES[number]['id'];
