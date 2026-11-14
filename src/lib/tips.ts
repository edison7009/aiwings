// Themed hover tips (`data-tip` + ::after) sit centered under their button. Near the window's edge a
// centered tip would be cut off, so on hover it is shifted back inside through --tip-shift.
const MARGIN = 6;

function fit(e: Event) {
  const el = (e.target as Element | null)?.closest?.<HTMLElement>('[data-tip]');
  if (!el) return;
  el.style.removeProperty('--tip-shift');
  const tip = getComputedStyle(el, '::after');
  // `* { box-sizing }` does not reach ::after, so its width leaves out the padding and border.
  const width = ['width', 'paddingLeft', 'paddingRight', 'borderLeftWidth', 'borderRightWidth']
    .reduce((sum, k) => sum + (parseFloat(tip[k as keyof CSSStyleDeclaration] as string) || 0), 0);
  if (!parseFloat(tip.width) || !el.offsetWidth) return;
  // Rects are in zoomed pixels and the tip's width in CSS pixels; `scale` converts between them.
  const rect = el.getBoundingClientRect();
  const scale = rect.width / el.offsetWidth;
  const page = document.documentElement.getBoundingClientRect();
  const half = (width * scale) / 2;
  const center = rect.left + rect.width / 2;
  const shift = Math.max(0, page.left + MARGIN - (center - half)) + Math.min(0, page.right - MARGIN - (center + half));
  if (shift) el.style.setProperty('--tip-shift', `${shift / scale}px`);
}

export function installTips() {
  document.addEventListener('pointerover', fit);
  document.addEventListener('focusin', fit);
}
