import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * macOS sends no mouse-move events to a window of an inactive app, and while the user types in the target
 * Wings is exactly that, so CSS :hover never fires. While the page lacks focus, poll the cursor and mark
 * the note under it, and the button aimed at (a note's delete icon, the close button), with classes the CSS
 * treats like :hover. Put on the whole panel.
 */
export function useHoverWhileInactive() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let busy = false;
    const clear = () => ref.current?.querySelectorAll('.is-hover').forEach(el => el.classList.remove('is-hover'));
    const timer = window.setInterval(() => {
      if (busy || !ref.current) return;
      if (document.hasFocus()) { clear(); return; }
      busy = true;
      void invoke<[number, number] | null>('cursor_in_window').then(pos => {
        busy = false;
        const root = ref.current;
        if (!root) return;
        const hit = pos ? document.elementFromPoint(pos[0], pos[1]) : null;
        const note = hit?.closest<HTMLElement>('.note') ?? null;
        const aim = hit?.closest<HTMLElement>('.note-btn, .win-close') ?? null;
        root.querySelectorAll('.is-hover').forEach(el => { if (el !== note && el !== aim) el.classList.remove('is-hover'); });
        if (note && pos) {
          note.classList.add('is-hover');
          note.style.setProperty('--glow-x', `${Math.round(pos[0] - note.getBoundingClientRect().left)}px`);
        }
        aim?.classList.add('is-hover');
      }, () => { busy = false; });
    }, 60);
    return () => { window.clearInterval(timer); clear(); };
  }, []);
  return ref;
}
