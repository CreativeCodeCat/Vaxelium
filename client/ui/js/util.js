// Small helpers shared by the views: escaping, colours, times, emoji paths and icons.

export function esc(text) {
  return String(text ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

/** The message from an error returned by a command (a string, or {message}). */
export function errMsg(e) {
  if (typeof e === 'string') return e;
  if (e && typeof e.message === 'string') return e.message;
  return String(e);
}

/** A stable colour for a name, so the same person always looks the same (a CSS variable). */
export function nameColor(name) {
  let hash = 5381;
  for (const b of new TextEncoder().encode(name)) {
    hash = (Math.imul(hash, 33) ^ b) >>> 0;
  }
  return `var(--avatar-${hash % 8})`;
}

/** The first letter of a name, upper case. */
export function initial(name) {
  const ch = Array.from(name || '')[0];
  return ch ? ch.toUpperCase() : '?';
}

/** Path to the Twemoji artwork for an emoji (see ui/emoji), or null when it has none. */
export function emojiSrc(emoji) {
  const code = Array.from(emoji)
    .map((c) => c.codePointAt(0))
    .filter((cp) => cp !== 0xfe0f)
    .map((cp) => cp.toString(16))
    .join('-');
  return `emoji/${code}.svg`;
}

/** An emoji as an image, falling back to its text when no artwork exists. */
export function emojiHtml(emoji, size = 20) {
  return `<img class="emoji" width="${size}" height="${size}" alt="${esc(emoji)}" src="${esc(emojiSrc(emoji))}" data-fallback="${esc(emoji)}">`;
}

// ----- times -----

const pad = (n) => String(n).padStart(2, '0');

export function dayKey(ts) {
  const d = new Date(ts * 1000);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

/** Short time, e.g. "14:05". */
export function timeLabel(ts) {
  const d = new Date(ts * 1000);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function sameDay(a, b) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function yesterday() {
  const d = new Date();
  d.setDate(d.getDate() - 1);
  return d;
}

/** Full stamp next to an author, e.g. "Today at 14:05". */
export function stampLabel(ts) {
  const d = new Date(ts * 1000);
  const time = timeLabel(ts);
  if (sameDay(d, new Date())) return `Today at ${time}`;
  if (sameDay(d, yesterday())) return `Yesterday at ${time}`;
  return `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${d.getFullYear()} ${time}`;
}

/** Date divider label, e.g. "Today", "Yesterday", or "Thursday, November 12, 2026". */
export function dayLabel(ts) {
  const d = new Date(ts * 1000);
  if (sameDay(d, new Date())) return 'Today';
  if (sameDay(d, yesterday())) return 'Yesterday';
  return d.toLocaleDateString(undefined, { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
}

// ----- icons (16 px line icons, drawn with currentColor) -----

const svg = (body) =>
  `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const ICON = {
  hash: svg('<rect x="3" y="5" width="18" height="13" rx="4"/><path d="M10 9l-1 6M15 9l-1 6"/>'),
  bell: svg('<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4z"/><path d="M10 21h4"/>'),
  pin: svg('<path d="M9 3h6l-1 6 3 3v2H7v-2l3-3z"/><path d="M12 14v7"/>'),
  members: svg('<circle cx="9" cy="8" r="3"/><path d="M3 20c0-3.3 2.7-5 6-5s6 1.7 6 5"/><circle cx="17" cy="9" r="2.5"/><path d="M16 15c3 0 5 1.5 5 4"/>'),
  search: svg('<circle cx="11" cy="11" r="6"/><path d="M16 16l5 5"/>'),
  mic: svg('<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>'),
  headset: svg('<path d="M4 14v-2a8 8 0 0 1 16 0v2"/><rect x="3" y="14" width="4" height="6" rx="2"/><rect x="17" y="14" width="4" height="6" rx="2"/>'),
  gear: svg('<path d="M18.81 10.37 L21.43 10.81 L21.43 13.19 L18.81 13.63 L17.97 15.66 L19.51 17.82 L17.82 19.51 L15.66 17.97 L13.63 18.81 L13.19 21.43 L10.81 21.43 L10.37 18.81 L8.34 17.97 L6.18 19.51 L4.49 17.82 L6.03 15.66 L5.19 13.63 L2.57 13.19 L2.57 10.81 L5.19 10.37 L6.03 8.34 L4.49 6.18 L6.18 4.49 L8.34 6.03 L10.37 5.19 L10.81 2.57 L13.19 2.57 L13.63 5.19 L15.66 6.03 L17.82 4.49 L19.51 6.18 L17.97 8.34 Z"/><circle cx="12" cy="12" r="3.0"/>'),
  plus: svg('<path d="M12 5v14M5 12h14"/>'),
  gift: svg('<rect x="3" y="8" width="18" height="13" rx="1"/><path d="M3 12h18M12 8v13M12 8c-2-4-6-4-6-1s6 1 6 1zM12 8c2-4 6-4 6-1s-6 1-6 1z"/>'),
  sticker: svg('<rect x="4" y="4" width="16" height="16" rx="4"/><circle cx="9" cy="10" r="1" fill="currentColor"/><circle cx="15" cy="10" r="1" fill="currentColor"/>'),
  emoji: svg('<circle cx="12" cy="12" r="9"/><circle cx="9" cy="10" r="1" fill="currentColor"/><circle cx="15" cy="10" r="1" fill="currentColor"/><path d="M8.5 14.5c1 1.5 2.2 2 3.5 2s2.5-.5 3.5-2"/>'),
  apps: svg('<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><rect x="14" y="14" width="6" height="6" rx="1"/>'),
  chevronDown: svg('<path d="M6 9l6 6 6-6"/>'),
  chevronRight: svg('<path d="M9 6l6 6-6 6"/>'),
  explore: svg('<circle cx="12" cy="12" r="9"/><path d="M15.5 8.5l-2 5-5 2 2-5z"/>'),
  pencil: svg('<path d="M4 20l4-1 11-11-3-3L5 16z"/><path d="M14 6l3 3"/>'),
  trash: svg('<path d="M4 7h16M10 7V4h4v3M6 7l1 13h10l1-13"/>'),
  close: svg('<path d="M6 6l12 12M18 6L6 18"/>'),
  minimize: svg('<path d="M5 12h14"/>'),
  maximize: svg('<rect x="5" y="5" width="14" height="14" rx="1"/>'),
  restore: svg('<rect x="8" y="4" width="12" height="12" rx="1"/><path d="M4 8v11h11"/>'),
  user: svg('<circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 3.6-6 8-6s8 2 8 6"/>'),
  attach: svg('<path d="M21 11l-8.5 8.5a5 5 0 0 1-7-7L14 4a3.5 3.5 0 0 1 5 5l-8.5 8.5a2 2 0 0 1-3-3L15 7"/>'),
  at: svg('<circle cx="12" cy="12" r="4"/><path d="M16 12v1.5a2.5 2.5 0 0 0 5 0V12a9 9 0 1 0-3.5 7.1"/>'),
  sliders: svg('<path d="M4 6h10M18 6h2M4 12h2M10 12h10M4 18h10M18 18h2"/><circle cx="16" cy="6" r="2"/><circle cx="8" cy="12" r="2"/><circle cx="16" cy="18" r="2"/>'),
  smile: svg('<circle cx="12" cy="12" r="9"/><path d="M8 14.5c1 1.5 2.5 2 4 2s3-.5 4-2"/>'),
};
