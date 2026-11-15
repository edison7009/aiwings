// Share links, ported from Coffee CLI's prompt links: an https link to aiwings.app/p that shows the prompt
// with a Copy button. The prompt rides in the fragment (#z= deflated, #t= plain), which browsers never send
// to the server, so the site only decodes it in the page and stores nothing. website/p.html reads it.

export const SHARE_PAGE = 'https://aiwings.app/p';
/** Same limit as the page and Coffee CLI, counted in characters. */
export const MAX_SHARE_CHARS = 8000;

// Zero-width, bidi, invisible operators, tag characters and stray controls; the emoji joiner and
// variation selectors stay because emoji need them. Same list as website/p.html.
// eslint-disable-next-line no-control-regex, no-misleading-character-class
const HIDDEN = /[\u00AD\u034F\u061C\u115F\u1160\u17B4\u17B5\u180B-\u180F\u200B\u200C\u200E\u200F\u202A-\u202E\u2060-\u206F\u3164\uFEFF\uFFA0\uFFF0-\uFFFB\u{1D173}-\u{1D17A}\u{E0000}-\u{E0FFF}\u0000-\u0008\u000B-\u001F\u007F-\u009F]/gu;

function clean(text: string): string {
  return text.replace(/\r\n/g, '\n').replace(HIDDEN, '').trim();
}

/** Whether a text can be shared: some visible text, within the link limit. */
export function canShare(text: string): boolean {
  const c = clean(text);
  return !!c && [...c].length <= MAX_SHARE_CHARS;
}

function toBase64Url(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function compress(text: string): Promise<string | null> {
  if (typeof CompressionStream === 'undefined') return null;
  const stream = new Blob([new TextEncoder().encode(text)]).stream().pipeThrough(new CompressionStream('deflate-raw'));
  return toBase64Url(new Uint8Array(await new Response(stream).arrayBuffer()));
}

/** The share link for a text, or null when it can't be shared. */
export async function shareLink(text: string): Promise<string | null> {
  if (!canShare(text)) return null;
  const c = clean(text);
  const packed = await compress(c).catch(() => null);
  return `${SHARE_PAGE}#${packed ? `z=${packed}` : `t=${encodeURIComponent(c)}`}`;
}
