// The app's state, and the lookups the views need. Everything the page shows lives here; the
// session token and the connections live in the Rust side.

/** Virtual message ids in the merged direct message timeline (see VIRTUAL_BASE in the old client). */
export const VIRTUAL_BASE = 1_000_000_000_000;
export const GROUP_GAP_SECS = 7 * 60;
export const MIN_PASSWORD_LEN = 8;

export const THEMES = [
  ['breeze-dark', 'Breeze Dark'],
  ['breeze-light', 'Breeze Light'],
  ['catppuccin-mocha', 'Catppuccin Mocha'],
  ['catppuccin-latte', 'Catppuccin Latte'],
];

export const S = {
  prefs: { servers: [], theme: 'breeze-dark', skinTone: 0, session: null },
  emoji: { list: [], skin: {} },

  // Account
  user: null,
  userId: '',
  authDown: false,
  authBannerClosed: false,
  notice: null,
  remember: true,
  login: { register: false, name: '', email: '', password: '', error: null, trustCert: false },

  // Servers
  selected: null,
  home: false,
  // null, or 'discover', 'user' (user settings), 'server' (server settings)
  page: null,
  addServer: { open: false, addr: '', error: null },
  discover: { via: null, found: [], error: null, expanded: new Set(), searching: false },
  account: {
    email: null,
    emailError: null,
    username: null,
    editing: null, // 'name' | 'username' | 'email'
    nameInput: '',
    usernameInput: '',
    emailInput: '',
    emailPassword: '',
    formError: null,
    draft: { name: null, username: null, email: null, theme: null },
    saveError: null,
  },
  serverForm: { name: '', description: '', iconError: null },

  // The open server's connection
  main: { id: null, addr: null, seq: 0, connecting: false, buffer: [], error: null, welcomed: false, status: '' },
  server: {
    name: '',
    description: '',
    icon: null,
    public: false,
    channels: [],
    channel: 'general',
    dms: [],
    online: [],
    offline: [],
    owner: null,
    ownerId: null,
    userId: '',
  },
  messages: {}, // channel -> entries: {kind:'chat', m} | {kind:'system', text, at}
  unread: new Set(),
  readMarks: {}, // channel -> index of the first message after the last visit
  search: '',
  editing: null, // {id, text}
  pendingDelete: null,
  input: '',
  newChannel: '',
  createOpen: false,
  membersHidden: false,
  channelsCollapsed: false,
  // Popups: {x, y, items} for the context menu, and {id} for the emoji picker
  menu: null,
  picker: null,
  pickerQuery: '',
  pendingDm: null, // {addr, channel}

  // Other servers' direct messages: addr -> {id, serverName, dms, unread:Set, history}
  dmLinks: {},
  // The last text channel open on each server, by address, so going back to it returns there.
  lastChannels: {},
  // Widths of the side panels, in pixels. Set by dragging their edges and saved between runs.
  layout: { channels: 250, dms: 250, members: 250 },
  // Views scroll positions
  scroll: {},
  status: '',
};

export const isLoggedIn = () => S.user !== null;

/** Joined the open server and ready to show its channels. */
export const chatReady = () => S.main.id !== null && S.main.welcomed;

export const isDmChannel = (channel) => channel.startsWith('dm-');

/** The name of a direct message's other member, from this server's list or another server's. */
function dmName(channel) {
  const own = S.server.dms.find((d) => d.channel === channel);
  if (own) return own.name;
  for (const link of Object.values(S.dmLinks)) {
    const found = link.dms.find((d) => d.channel === channel);
    if (found) return found.name;
  }
  return null;
}

/** How the open channel is named: "#name", or "@ name" for a direct message. */
export function channelLabel(channel = S.server.channel) {
  if (isDmChannel(channel)) return `@ ${dmName(channel) ?? 'Direct message'}`;
  return `#${channel}`;
}

export function selectedServer() {
  return S.selected === null ? null : S.prefs.servers[S.selected] ?? null;
}

export function selectedName() {
  return selectedServer()?.name ?? null;
}

export function isOwner() {
  return S.server.ownerId !== null && S.server.ownerId === S.userId;
}

/** The person whose conversation is open on the Direct messages page, by hidden account id. */
export function currentPerson() {
  if (S.main.id === null || !isDmChannel(S.server.channel)) return null;
  const channel = S.server.channel;
  const own = S.server.dms.find((d) => d.channel === channel);
  if (own) return own.other_id;
  const link = S.dmLinks[S.main.addr];
  const found = link?.dms.find((d) => d.channel === channel);
  return found ? found.other_id : null;
}

/** Every conversation with this person, on every server, in a stable order. */
export function personTargets(otherId) {
  const out = [];
  for (const [addr, link] of Object.entries(S.dmLinks)) {
    for (const dm of link.dms) {
      if (dm.other_id === otherId) {
        out.push({ addr, channel: dm.channel, server: serverLabel(addr, link.serverName), last: dm.last_message });
      }
    }
  }
  out.sort((a, b) => (a.addr + a.channel).localeCompare(b.addr + b.channel));
  return out;
}

function serverLabel(addr, serverName) {
  if (serverName) return serverName;
  return S.prefs.servers.find((s) => s.addr === addr)?.name ?? addr;
}

/** The Direct messages list: one row per person across every server, most recent first. */
export function persons() {
  const rows = new Map();
  for (const link of Object.values(S.dmLinks)) {
    for (const dm of link.dms) {
      let row = rows.get(dm.other_id);
      if (!row) {
        row = { id: dm.other_id, name: dm.name, last: 0, unread: false };
        rows.set(dm.other_id, row);
      }
      if (dm.last_message >= row.last) {
        row.last = dm.last_message;
        row.name = dm.name;
      }
      if (link.unread.has(dm.channel)) row.unread = true;
    }
  }
  return [...rows.values()].sort((a, b) => b.last - a.last || a.name.localeCompare(b.name));
}

export function personName(otherId) {
  return persons().find((p) => p.id === otherId)?.name ?? null;
}

/** The messages with one person from every server, merged by time. Ids say which server (see VIRTUAL_BASE). */
export function dmTimeline() {
  const person = currentPerson();
  if (person === null) return [];
  const entries = [];
  personTargets(person).forEach((target, index) => {
    const link = S.dmLinks[target.addr];
    const history = link?.history[target.channel];
    if (!history) return;
    for (const message of history) {
      const m = { ...message };
      if (m.id !== 0) m.id += (index + 1) * VIRTUAL_BASE;
      entries.push({ kind: 'chat', m });
    }
  });
  entries.sort((a, b) => entryTime(a) - entryTime(b));
  return entries;
}

export function entryTime(entry) {
  return entry.kind === 'chat' ? entry.m.timestamp : entry.at;
}

/** Which server a DM message id belongs to, and its real id there. Null for ordinary messages. */
export function virtualTarget(id) {
  if (id < VIRTUAL_BASE) return null;
  const person = currentPerson();
  if (person === null) return null;
  const index = Math.floor(id / VIRTUAL_BASE) - 1;
  const target = personTargets(person)[index];
  return target ? { ...target, realId: id % VIRTUAL_BASE } : null;
}

export function canEdit(m) {
  return m.author_id === S.userId;
}

export function canDelete(m) {
  return canEdit(m) || isOwner();
}

/** The entries shown for the open view (channel messages, or the merged direct message timeline). */
export function currentEntries() {
  if (currentPerson() !== null) return dmTimeline();
  return S.messages[S.server.channel] ?? [];
}
