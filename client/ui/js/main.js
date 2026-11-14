// Boot, the events from the Rust side, and every action the window can take. The state is in
// state.js and the drawing is in render.js.

import {
  S, THEMES, VIRTUAL_BASE, chatReady, currentEntries, currentPerson, isDmChannel, isLoggedIn,
  personTargets, selectedName, selectedServer, virtualTarget, canEdit, canDelete,
} from './state.js';
import {
  body, titlebar, authBanner, resizeEdges, dialogs, menuPopup, pickerPopup, channelMenuItems, userMenuItems,
  messageMenuItems, railMenuItems, serverMenuItems,
} from './render.js';
import { errMsg } from './util.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const win = window.__TAURI__.window.getCurrentWindow();
const appEl = document.getElementById('app');

const DEFAULT_ADDR = '127.0.0.1:7878';
const DM_TICK_MS = 5000;
const RESIZE = { n: 'North', s: 'South', e: 'East', w: 'West', nw: 'NorthWest', ne: 'NorthEast', sw: 'SouthWest', se: 'SouthEast' };
/** Buttons that open a popup. A click on them is not a click away from the popup. */
const OPENERS = new Set(['more', 'picker', 'channel-menu', 'server-menu', 'user-menu']);

let dmTimer = null;
let saveTimer = null;
let focusId = null;

// ----- rendering -----

function render() {
  const focus = captureFocus();
  const scroll = captureScroll();
  appEl.innerHTML = `${titlebar()}${authBanner()}<div class="app-body">${body()}</div>${resizeEdges()}${dialogs()}${menuPopup()}${pickerPopup()}`;
  restoreScroll(scroll);
  restoreFocus(focus);
  document.documentElement.dataset.theme = S.prefs.theme;
}

function captureFocus() {
  const el = document.activeElement;
  if (!el || !el.id || !(el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) return null;
  return { id: el.id, start: el.selectionStart, end: el.selectionEnd };
}

function restoreFocus(saved) {
  const id = focusId ?? saved?.id;
  if (!id) return;
  const el = document.getElementById(id);
  if (!el) return;
  el.focus();
  try {
    if (focusId) {
      el.setSelectionRange(el.value.length, el.value.length);
    } else if (saved) {
      el.setSelectionRange(saved.start, saved.end);
    }
  } catch {
    // Some inputs (such as password fields in some engines) refuse a selection; that is fine.
  }
  focusId = null;
}

function captureScroll() {
  const el = document.getElementById('messages');
  if (!el) return null;
  return {
    key: el.dataset.scroll,
    top: el.scrollTop,
    atBottom: el.scrollHeight - el.scrollTop - el.clientHeight < 40,
  };
}

function restoreScroll(saved) {
  const el = document.getElementById('messages');
  if (!el) return;
  if (!saved || saved.key !== el.dataset.scroll || saved.atBottom) {
    el.scrollTop = el.scrollHeight;
  } else {
    el.scrollTop = saved.top;
  }
}

// ----- preferences on disk -----

function saveSoon() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    const server = selectedServer();
    const prefs = {
      servers: S.prefs.servers,
      theme: S.prefs.theme,
      skinTone: S.prefs.skinTone,
      session: S.user && S.remember ? { name: S.user, server: server?.addr ?? null } : null,
      layout: S.layout,
      lastChannels: S.lastChannels,
    };
    invoke('prefs_save', { prefs }).catch((e) => console.error('could not save settings', e));
  }, 300);
}

function applyTheme() {
  document.documentElement.dataset.theme = S.prefs.theme;
}

// ----- state helpers -----

function resetServerView(srv) {
  S.server = {
    name: srv?.name ?? '',
    description: '',
    icon: srv?.icon ?? null,
    public: false,
    channels: [],
    channel: (srv && S.lastChannels[srv.addr]) || 'general',
    dms: [],
    online: [],
    offline: [],
    owner: null,
    ownerId: null,
    userId: S.userId,
  };
  S.messages = {};
  S.unread = new Set();
  S.readMarks = {};
  S.search = '';
  S.editing = null;
  S.pendingDelete = null;
  S.input = '';
  S.status = '';
}

function freshMain() {
  return { id: null, addr: null, seq: S.main.seq, connecting: false, buffer: [], error: null, welcomed: false, status: '' };
}

function setPath(path, value) {
  const parts = path.split('.');
  let target = S;
  for (const key of parts.slice(0, -1)) target = target[key];
  target[parts[parts.length - 1]] = value;
}

function friendlyDisconnect(reason) {
  const r = reason.toLowerCase();
  if (r.includes('refused')) return "The server isn't running or can't be reached. Check the address and try again.";
  if (r.includes('timed out') || r.includes('timeout') || r.includes('did not answer')) {
    return 'The server stopped responding. Check your connection and try again.';
  }
  if (r.includes('certificate')) return "The server's security certificate has changed. See the message on the login screen.";
  if (['close_notify', 'unexpected-eof', 'closed', 'reset'].some((k) => r.includes(k))) {
    return 'The server closed the connection, perhaps because it restarted. Try again in a moment.';
  }
  return 'Something interrupted the connection. Try again.';
}

// ----- sending to the servers -----

function sendMain(msg) {
  return invoke('send_main', { msg }).catch(() => {});
}

function sendDm(addr, msg) {
  return invoke('send_dm', { addr, msg }).catch(() => {});
}

function join(channel) {
  const name = channel.trim().replace(/^#/, '');
  if (!name || name === S.server.channel) return;
  sendMain({ type: 'join', channel: name });
}

/** Sends a change to one message (edit, delete or react). Direct message ids go to their own server. */
function messageAction(id, msg) {
  if (id < VIRTUAL_BASE) {
    sendMain({ ...msg, id });
    return;
  }
  const target = virtualTarget(id);
  if (!target) return;
  sendDm(target.addr, { ...msg, id: target.realId });
  sendDm(target.addr, { type: 'fetch_dm', channel: target.channel });
}

function fetchPerson(otherId) {
  for (const target of personTargets(otherId)) {
    sendDm(target.addr, { type: 'fetch_dm', channel: target.channel });
  }
}

function listDirectMessagesEverywhere() {
  for (const addr of Object.keys(S.dmLinks)) sendDm(addr, { type: 'list_direct_messages' });
}

// ----- connections -----

async function connectSelected() {
  const srv = selectedServer();
  if (!srv || !isLoggedIn()) return;
  const seq = S.main.seq + 1;
  resetServerView(srv);
  S.main = { ...freshMain(), addr: srv.addr, seq, connecting: true, status: 'Connecting...' };
  render();
  try {
    const id = await invoke('connect_main', { addr: srv.addr });
    if (S.main.seq !== seq) return;
    S.main.id = id;
    S.main.connecting = false;
    const buffered = S.main.buffer.filter((e) => e.id === id);
    S.main.buffer = [];
    for (const e of buffered) handleMain(e.msg);
  } catch (e) {
    if (S.main.seq !== seq) return;
    S.main.connecting = false;
    S.main.error = `Could not connect to ${srv.name}: ${errMsg(e)}`;
  }
  render();
}

function selectServer(index) {
  S.home = false;
  S.page = null;
  if (index < 0 || index >= S.prefs.servers.length) return;
  if (S.selected === index) {
    // Still connected here, but a direct message may be open: go back to this server's channel.
    if (S.main.id !== null && isDmChannel(S.server.channel)) {
      join(S.lastChannels[S.main.addr] ?? 'general');
    }
    return;
  }
  S.selected = index;
  saveSoon();
  connectSelected();
}

function disconnectMain() {
  invoke('disconnect_main').catch(() => {});
  S.main = { ...freshMain(), seq: S.main.seq + 1 };
  resetServerView(null);
}

function removeServer(index) {
  const srv = S.prefs.servers[index];
  if (!srv) return;
  if (S.selected === index) {
    disconnectMain();
    S.selected = null;
  } else if (S.selected !== null && S.selected > index) {
    S.selected -= 1;
  }
  invoke('dm_disconnect', { addr: srv.addr }).catch(() => {});
  invoke('trust_forget', { kind: 'server', addr: srv.addr }).catch(() => {});
  delete S.dmLinks[srv.addr];
  S.prefs.servers.splice(index, 1);
  saveSoon();
  dmTick();
}

function dmTick() {
  if (!isLoggedIn()) return;
  invoke('dm_connect_all', { addrs: S.prefs.servers.map((s) => s.addr) }).catch(() => {});
}

function startDmTimer() {
  clearInterval(dmTimer);
  dmTick();
  dmTimer = setInterval(dmTick, DM_TICK_MS);
}

function stopDmTimer() {
  clearInterval(dmTimer);
  dmTimer = null;
}

// ----- events from the servers -----

function handleMain(msg) {
  switch (msg.type) {
    case 'welcome': {
      S.main.welcomed = true;
      S.main.error = null;
      S.userId = msg.user_id;
      S.user = msg.name;
      S.server.userId = msg.user_id;
      S.server.public = msg.server_public;
      S.server.description = msg.server_description;
      S.server.icon = msg.server_icon ?? null;
      setServerProfile(msg.server_name);
      // The rail draws the icon from the saved server list, so keep that copy in step too.
      if (S.selected !== null) S.prefs.servers[S.selected].icon = S.server.icon;
      saveSoon();
      S.status = `Connected as ${msg.name}`;
      sendMain({ type: 'list_channels' });
      sendMain({ type: 'join', channel: S.server.channel });
      if (S.pendingDm && S.pendingDm.addr === S.main.addr) {
        sendMain({ type: 'join', channel: S.pendingDm.channel });
      }
      S.pendingDm = null;
      break;
    }
    case 'joined': {
      const previous = S.server.channel;
      if (msg.channel !== previous && S.messages[previous]) {
        S.readMarks[previous] = S.messages[previous].length;
      }
      S.unread.delete(msg.channel);
      S.messages[msg.channel] = msg.history.map((m) => ({ kind: 'chat', m }));
      S.server.channel = msg.channel;
      if (!isDmChannel(msg.channel) && S.main.addr) {
        S.lastChannels[S.main.addr] = msg.channel;
        saveSoon();
      }
      focusId = 'composer';
      sendMain({ type: 'list_channels' });
      break;
    }
    case 'message': {
      const m = msg.message;
      (S.messages[m.channel] ??= []).push({ kind: 'chat', m });
      break;
    }
    case 'reactions': {
      const m = findMessage(msg.channel, msg.id);
      if (m) m.reactions = msg.reactions;
      break;
    }
    case 'message_deleted': {
      const list = S.messages[msg.channel];
      if (list) S.messages[msg.channel] = list.filter((e) => !(e.kind === 'chat' && e.m.id === msg.id));
      if (S.editing && S.editing.id === msg.id) S.editing = null;
      break;
    }
    case 'message_edited': {
      const m = findMessage(msg.channel, msg.id);
      if (m) {
        m.text = msg.text;
        m.edited = true;
      }
      break;
    }
    case 'activity': {
      if (msg.channel.startsWith('dm-') && !S.server.dms.some((d) => d.channel === msg.channel)) {
        sendMain({ type: 'list_direct_messages' });
      }
      if (msg.channel !== S.server.channel) S.unread.add(msg.channel);
      break;
    }
    case 'server_updated':
      S.server.description = msg.description;
      setServerProfile(msg.name);
      break;
    case 'server_icon':
      S.server.icon = msg.png_base64 ?? null;
      if (S.selected !== null) S.prefs.servers[S.selected].icon = S.server.icon;
      saveSoon();
      break;
    case 'channels':
      S.server.channels = msg.channels;
      break;
    case 'direct_messages':
      S.server.dms = msg.list;
      break;
    case 'presence': {
      const owner = msg.users.find((u) => u.owner);
      S.server.online = msg.users.filter((u) => u.online).map((u) => u.name);
      S.server.offline = msg.users.filter((u) => !u.online).map((u) => u.name);
      S.server.owner = owner ? owner.name : null;
      S.server.ownerId = owner ? owner.id : null;
      break;
    }
    case 'system': {
      (S.messages[msg.channel] ??= []).push({ kind: 'system', text: msg.text, at: Date.now() / 1000 });
      break;
    }
    case 'error':
      if (S.main.welcomed) S.status = `Error: ${msg.text}`;
      else S.main.error = msg.text;
      break;
    default:
      // Info, Peers and DM history do not arrive on the main connection.
      break;
  }
}

function setServerProfile(name) {
  S.server.name = name;
  if (S.selected !== null) {
    S.prefs.servers[S.selected].name = name;
    saveSoon();
  }
}

function findMessage(channel, id) {
  const list = S.messages[channel];
  const entry = list?.find((e) => e.kind === 'chat' && e.m.id === id);
  return entry ? entry.m : null;
}

function handleDm(addr, msg) {
  const link = S.dmLinks[addr];
  if (!link) return;
  switch (msg.type) {
    case 'welcome':
      link.serverName = msg.server_name;
      break;
    case 'direct_messages':
      link.dms = msg.list;
      break;
    case 'dm_history':
      link.history[msg.channel] = msg.messages;
      break;
    case 'activity':
      if (msg.channel.startsWith('dm-')) {
        link.unread.add(msg.channel);
        sendDm(addr, { type: 'list_direct_messages' });
        sendDm(addr, { type: 'fetch_dm', channel: msg.channel });
      }
      break;
    default:
      break;
  }
}

function newLink(id) {
  return { id, serverName: '', dms: [], unread: new Set(), history: {} };
}

function onNet({ link, addr, id, msg }) {
  if (link === 'main') {
    if (S.main.id === null && S.main.connecting) {
      // The connection's id is not known until the connect call returns; keep the messages for then.
      S.main.buffer.push({ id, msg });
      return;
    }
    if (id === S.main.id) handleMain(msg);
  } else {
    if (!S.dmLinks[addr]) S.dmLinks[addr] = newLink(id);
    if (S.dmLinks[addr].id === id) handleDm(addr, msg);
  }
  render();
}

function onNetClosed({ link, addr, id, reason }) {
  if (link === 'main') {
    if (id !== S.main.id) return;
    const name = selectedName() ?? 'the server';
    const friendly = friendlyDisconnect(reason);
    const wasWelcomed = S.main.welcomed;
    S.main = { ...freshMain(), seq: S.main.seq };
    resetServerView(selectedServer());
    if (wasWelcomed) S.main.error = `Lost the connection to ${name}. ${friendly}`;
    else if (!S.main.error) S.main.error = `Couldn't join ${name}. ${friendly}`;
  } else if (S.dmLinks[addr]?.id === id) {
    delete S.dmLinks[addr];
  }
  render();
}

// ----- actions -----

function openDiscover() {
  S.page = 'discover';
  const via = selectedServer();
  S.discover = { via: via?.addr ?? null, found: [], error: null, expanded: new Set(), searching: !!via };
  if (!via) {
    S.discover.error = 'Add a server first. Searching starts from a server you know.';
    return;
  }
  invoke('discover', { via: via.addr }).catch((e) => {
    S.discover.error = errMsg(e);
    render();
  });
}

function openPerson(otherId) {
  const targets = personTargets(otherId);
  let newest = null;
  for (const t of targets) if (!newest || t.last > newest.last) newest = t;
  if (!newest) return;
  S.home = true;
  openDmOn(newest.addr, newest.channel);
}

function openDmOn(addr, channel) {
  const link = S.dmLinks[addr];
  if (link) link.unread.delete(channel);
  const person = link?.dms.find((d) => d.channel === channel)?.other_id;
  if (person) fetchPerson(person);
  if (S.main.id !== null && S.main.addr === addr) {
    join(channel);
    return;
  }
  const index = S.prefs.servers.findIndex((s) => s.addr === addr);
  if (index < 0) return;
  S.pendingDm = { addr, channel };
  S.home = false;
  S.selected = index;
  saveSoon();
  connectSelected();
}

function sendInput() {
  const text = S.input.trim();
  S.input = '';
  if (!text) return;
  if (text.startsWith('/join ')) join(text.slice(6));
  else sendMain({ type: 'say', text });
}

function addServerSubmit() {
  let addr = S.addServer.addr.trim();
  if (addr === '') addr = DEFAULT_ADDR;
  else if (!addr.includes(':')) addr = `${addr}:7878`;
  if (S.prefs.servers.some((s) => s.addr === addr)) {
    S.addServer.error = 'That server is already in your list.';
    return;
  }
  S.prefs.servers.push({ name: addr, addr, icon: null });
  S.addServer = { open: false, addr: '', error: null };
  saveSoon();
  selectServer(S.prefs.servers.length - 1);
  dmTick();
}

function discoverAdd(addr) {
  const found = S.discover.found.find((f) => f.addr === addr);
  if (!found || S.prefs.servers.some((s) => s.addr === addr)) return;
  S.prefs.servers.push({ name: found.name, addr: found.addr, icon: found.icon });
  saveSoon();
  dmTick();
}

function openMenu(x, y, items, key, width) {
  const height = items.length * 32 + 16;
  S.menu = {
    key,
    x: Math.max(4, Math.min(x, window.innerWidth - (width ?? 200) - 8)),
    y: Math.max(4, Math.min(y, window.innerHeight - height - 8)),
    items,
    width,
  };
  S.picker = null;
}

function rectMenu(el, items, key, width) {
  const r = el.getBoundingClientRect();
  openMenu(width ? r.left + 8 : r.left, r.bottom + 4, items, key, width);
}

async function doLogin() {
  const L = S.login;
  try {
    const user = await invoke('login', {
      email: L.email,
      password: L.password,
      name: L.register ? L.name : null,
      remember: S.remember,
    });
    S.user = user;
    S.authDown = false;
    S.login = { ...L, password: '', name: '', error: null };
    startDmTimer();
    saveSoon();
  } catch (e) {
    S.login.error = errMsg(e);
    S.authDown = !!e?.auth_down;
  }
}

async function doLogout() {
  stopDmTimer();
  await invoke('logout').catch(() => {});
  S.user = null;
  S.userId = '';
  S.selected = null;
  S.home = false;
  S.page = null;
  S.dmLinks = {};
  S.main = freshMain();
  resetServerView(null);
  S.login.password = '';
  S.account = { ...S.account, email: null, username: null, editing: null, draft: { name: null, username: null, email: null, theme: null } };
  saveSoon();
}

function loadProfile() {
  return invoke('profile')
    .then((p) => {
      S.user = p.name;
      S.account.username = p.username;
      S.account.email = p.email ?? null;
      S.account.emailError = null;
    })
    .catch((e) => {
      S.account.emailError = errMsg(e);
    });
}

function applyProfile(p) {
  S.user = p.name;
  S.account.username = p.username;
  S.account.email = p.email ?? null;
  saveSoon();
}

async function saveAccountChanges() {
  const A = S.account;
  A.saveError = null;
  if (A.draft.name !== null) {
    if (A.draft.name !== S.user) {
      try {
        applyProfile(await invoke('update_profile', { name: A.draft.name, username: null }));
      } catch (e) {
        A.saveError = errMsg(e);
        return;
      }
    }
    A.draft.name = null;
  }
  if (A.draft.username !== null) {
    if (A.draft.username !== A.username) {
      try {
        applyProfile(await invoke('update_profile', { name: null, username: A.draft.username }));
      } catch (e) {
        A.saveError = errMsg(e);
        return;
      }
    }
    A.draft.username = null;
  }
  if (A.draft.email !== null) {
    const [newEmail, password] = A.draft.email;
    try {
      A.email = await invoke('change_email', { password, newEmail });
      A.draft.email = null;
      A.emailPassword = '';
    } catch (e) {
      A.saveError = errMsg(e);
      return;
    }
  }
  if (A.draft.theme !== null) {
    S.prefs.theme = A.draft.theme;
    applyTheme();
    A.draft.theme = null;
    saveSoon();
  }
}

function resetAccountDrafts() {
  S.account.draft = { name: null, username: null, email: null, theme: null };
  S.account.saveError = null;
}

function serverFormFromState() {
  S.serverForm = { name: selectedName() ?? '', description: S.server.description, iconError: null };
}

function saveServer() {
  const name = S.serverForm.name.trim();
  if (!name) return;
  sendMain({ type: 'update_server', name, description: S.serverForm.description.trim() });
}

async function pickIcon() {
  S.serverForm.iconError = null;
  try {
    const png = await invoke('pick_server_icon');
    if (png) sendMain({ type: 'set_server_icon', png_base64: png });
  } catch (e) {
    S.serverForm.iconError = errMsg(e);
  }
}

function accountEditSubmit(field) {
  const A = S.account;
  if (field === 'name') {
    A.draft.name = A.nameInput.trim();
  } else if (field === 'username') {
    A.draft.username = A.usernameInput.trim().toLowerCase();
  } else {
    A.draft.email = [A.emailInput.trim(), A.emailPassword];
  }
  A.editing = null;
  A.formError = null;
}

/** Runs the action named by a button, menu item or key. */
async function act(name, el) {
  const d = el.dataset;
  switch (name) {
    case 'soon':
      return;
    case 'win-min': return win.minimize();
    case 'win-max': return win.toggleMaximize();
    case 'win-close': return win.close();
    case 'auth-banner-close':
      S.authBannerClosed = true;
      return;
    case 'login-toggle':
      S.login.register = !S.login.register;
      S.login.error = null;
      return;
    case 'trust-auth':
      await invoke('trust_forget_auth').catch(() => {});
      S.login.error = null;
      return;
    case 'trust-server':
      await invoke('trust_forget', { kind: 'server', addr: S.main.addr }).catch(() => {});
      return connectSelected();
    case 'retry-server':
      return connectSelected();
    case 'logout':
      return doLogout();
    case 'select-server':
      return selectServer(Number(d.i));
    case 'remove-server':
      S.menu = null;
      return removeServer(Number(d.i));
    case 'add-server-open':
      S.addServer = { open: true, addr: '', error: null };
      focusId = 'add-addr';
      return;
    case 'add-server-close':
      S.addServer.open = false;
      return;
    case 'discover-from-add':
      S.addServer.open = false;
      return openDiscover();
    case 'discover-open':
      return openDiscover();
    case 'discover-close':
      S.page = null;
      return;
    case 'discover-toggle': {
      const set = S.discover.expanded;
      if (set.has(d.addr)) set.delete(d.addr);
      else set.add(d.addr);
      return;
    }
    case 'discover-add':
      return discoverAdd(d.addr);
    case 'home-open':
      S.home = true;
      S.page = null;
      listDirectMessagesEverywhere();
      return;
    case 'open-person':
      return openPerson(d.id);
    case 'channel-open':
      if (d.name !== S.server.channel) join(d.name);
      return;
    case 'channel-menu':
      return rectMenu(el, channelMenuItems(d.name, d.unread === '1'), `channel-${d.name}`);
    case 'create-open':
      S.createOpen = true;
      S.menu = null;
      focusId = 'new-channel';
      return;
    case 'create-channel': {
      const name = S.newChannel;
      S.newChannel = '';
      S.createOpen = false;
      return join(name);
    }
    case 'toggle-channels':
      S.channelsCollapsed = !S.channelsCollapsed;
      return;
    case 'toggle-members':
      S.membersHidden = !S.membersHidden;
      S.menu = null;
      return;
    case 'server-menu':
      if (!chatReady()) return;
      return rectMenu(el, serverMenuItems(), 'server', el.offsetWidth - 16);
    case 'mark-read':
      S.unread.delete(d.name);
      delete S.readMarks[d.name];
      S.menu = null;
      return;
    case 'copy':
      S.menu = null;
      return navigator.clipboard?.writeText(d.text).catch(() => {});
    case 'mention':
      S.input += `@${d.name} `;
      S.menu = null;
      focusId = 'composer';
      return;
    case 'message-user':
      S.menu = null;
      S.home = true;
      return sendMain({ type: 'open_dm', name: d.name });
    case 'user-menu': {
      const canMsg = d.canMsg === '1';
      return openMenu(...menuPositionFor(el), userMenuItems(d.name, canMsg), `user-${d.name}`);
    }
    case 'react':
      S.menu = null;
      S.picker = null;
      return messageAction(Number(d.id), { type: 'react', emoji: d.emoji });
    case 'picker': {
      const id = Number(d.id);
      if (S.picker && S.picker.id === id) {
        S.picker = null;
        return;
      }
      const r = el.getBoundingClientRect();
      S.menu = null;
      S.picker = { id, x: Math.max(4, Math.min(r.left - 280, window.innerWidth - 360)), y: Math.min(r.bottom + 4, window.innerHeight - 420) };
      S.pickerQuery = '';
      return;
    }
    case 'picker-tone':
      S.prefs.skinTone = Number(d.tone);
      saveSoon();
      return;
    case 'more': {
      const id = Number(d.id);
      const m = findAnyMessage(id);
      if (!m) return;
      const key = `more-${id}`;
      if (S.menu && S.menu.key === key) {
        S.menu = null;
        return;
      }
      return rectMenu(el, messageMenuItems(m), key);
    }
    case 'edit-start': {
      const m = findAnyMessage(Number(d.id));
      S.menu = null;
      if (!m) return;
      S.editing = { id: m.id, text: m.text };
      focusId = 'edit-box';
      return;
    }
    case 'edit-save': {
      if (!S.editing) return;
      const { id, text } = S.editing;
      S.editing = null;
      return messageAction(id, { type: 'edit', text });
    }
    case 'edit-cancel':
      S.editing = null;
      return;
    case 'delete-ask':
      S.pendingDelete = Number(d.id);
      S.menu = null;
      return;
    case 'delete-confirm': {
      const id = S.pendingDelete;
      S.pendingDelete = null;
      if (id !== null) messageAction(id, { type: 'delete' });
      return;
    }
    case 'delete-cancel':
      S.pendingDelete = null;
      return;
    case 'server-settings-open':
      S.menu = null;
      serverFormFromState();
      S.page = 'server';
      return;
    case 'server-settings-close':
      S.page = null;
      return;
    case 'server-reset':
      serverFormFromState();
      return;
    case 'server-save':
      return saveServer();
    case 'icon-pick':
      return pickIcon();
    case 'icon-remove':
      S.serverForm.iconError = null;
      return sendMain({ type: 'set_server_icon', png_base64: null });
    case 'user-settings-open':
      S.page = 'user';
      S.account.editing = null;
      return loadProfile();
    case 'user-settings-close':
      S.page = null;
      return;
    case 'account-edit': {
      const A = S.account;
      A.editing = d.field;
      A.formError = null;
      if (d.field === 'name') A.nameInput = A.draft.name ?? S.user;
      if (d.field === 'username') A.usernameInput = A.draft.username ?? A.username ?? '';
      if (d.field === 'email') {
        A.emailInput = A.draft.email ? A.draft.email[0] : '';
        A.emailPassword = '';
      }
      focusId = d.field === 'name' ? 'acc-name' : d.field === 'username' ? 'acc-username' : 'acc-email';
      return;
    }
    case 'account-edit-cancel':
      S.account.editing = null;
      S.account.formError = null;
      return;
    case 'account-reset':
      resetAccountDrafts();
      return;
    case 'account-save':
      return saveAccountChanges();
    case 'dm-server':
      return;
    default:
      console.warn('no action for', name);
  }
}

/** Where a popup opens next to the element that was clicked. */
function menuPositionFor(el) {
  const r = el.getBoundingClientRect();
  return [r.left, r.bottom + 4];
}

function findAnyMessage(id) {
  if (id < VIRTUAL_BASE) {
    return currentEntries().find((e) => e.kind === 'chat' && e.m.id === id)?.m ?? null;
  }
  return currentEntries().find((e) => e.kind === 'chat' && e.m.id === id)?.m ?? null;
}

// ----- input, keys and pointer -----

function onClick(event) {
  const el = event.target.closest('[data-act]');
  const inPopup = !!event.target.closest('.popup');
  if (!inPopup && (!el || !OPENERS.has(el.dataset.act))) {
    if (S.menu || S.picker) {
      // A click away from a popup closes it. Other clicks leave the window alone, so the
      // focused input is not replaced.
      S.menu = null;
      S.picker = null;
      render();
    }
  }
  if (!el) return;
  // A click on a row's gear or name counts as its own action, not the row's.
  act(el.dataset.act, el).then(() => render(), (e) => {
    console.error(e);
    render();
  });
}

function onContextMenu(event) {
  const el = event.target.closest('[data-ctx]');
  if (!el) return;
  event.preventDefault();
  const d = el.dataset;
  const x = event.clientX;
  const y = event.clientY;
  switch (d.ctx) {
    case 'channel':
      openMenu(x, y, channelMenuItems(d.nameCtx, el.dataset.unread === '1'), `channel-${d.nameCtx}`);
      break;
    case 'user':
      openMenu(x, y, userMenuItems(d.nameCtx, d.canMsg === '1'), `user-${d.nameCtx}`);
      break;
    case 'rail':
      openMenu(x, y, railMenuItems(Number(d.iCtx)), `rail-${d.iCtx}`);
      break;
    case 'message': {
      const m = findAnyMessage(Number(d.idCtx));
      if (!m || m.id === 0 || !(canEdit(m) || canDelete(m))) break;
      openMenu(x, y, messageMenuItems(m), `context-${m.id}`);
      break;
    }
    default:
      break;
  }
  render();
}

function onInput(event) {
  const el = event.target;
  if (!el.dataset || !el.dataset.model) return;
  let value = el.value;
  if (el.dataset.lower) {
    value = value.toLowerCase();
    el.value = value;
  }
  setPath(el.dataset.model, value);
  if (el.dataset.rerender) render();
}

function onChange(event) {
  // Text fields also fire `change` when they lose focus. Re-rendering then would replace the
  // field the user just moved to, so only checkboxes and drop-down lists are handled here.
  const el = event.target;
  if (el.dataset.check === 'remember') {
    S.remember = el.checked;
    saveSoon();
  } else if (el.dataset.change === 'theme') {
    const theme = el.value;
    S.account.draft.theme = theme === S.prefs.theme ? null : theme;
  } else if (el.dataset.change === 'dm-server') {
    const target = personTargets(currentPerson() ?? '')[Number(el.value)];
    if (target) openDmOn(target.addr, target.channel);
  } else {
    return;
  }
  render();
}

function onSubmit(event) {
  event.preventDefault();
  const form = event.target;
  switch (form.dataset.form) {
    case 'login':
      doLogin().then(() => render());
      break;
    case 'send':
      sendInput();
      break;
    case 'add-server':
      addServerSubmit();
      break;
    case 'account-edit':
      accountEditSubmit(form.dataset.field);
      break;
    default:
      break;
  }
  render();
}

function onKeydown(event) {
  const el = event.target;
  if (event.key === 'Enter' && el.dataset && el.dataset.enter) {
    event.preventDefault();
    const name = el.dataset.enter;
    if (name === 'edit-save') act('edit-save', el).then(render);
    else if (name === 'create-channel') act('create-channel', el).then(render);
    else if (name === 'server-save') act('server-save', el).then(render);
    return;
  }
  if (event.key === 'Escape') {
    if (el.dataset && el.dataset.escape) {
      act(el.dataset.escape, el).then(render);
      return;
    }
    if (S.menu || S.picker) {
      S.menu = null;
      S.picker = null;
    } else if (S.editing) {
      S.editing = null;
    }
    else if (S.pendingDelete !== null) S.pendingDelete = null;
    else if (S.addServer.open) S.addServer.open = false;
    else if (S.account.editing) S.account.editing = null;
    else if (S.page === 'discover' || S.page === 'user' || S.page === 'server') S.page = null;
    else return;
    render();
  }
}

const RESIZE_CURSOR_EDGES = new Set(Object.keys(RESIZE));

function onMouseDown(event) {
  const split = event.target.dataset?.split;
  if (split && event.button === 0) {
    event.preventDefault();
    startSplitDrag(event, split, event.target.dataset.right === '1');
    return;
  }
  const edge = event.target.dataset?.resize;
  if (edge && RESIZE_CURSOR_EDGES.has(edge) && event.button === 0) {
    win.startResizeDragging(RESIZE[edge]).catch(() => {});
  }
}

// ----- resizing the side panels -----

const PANEL_MIN = 250;
const PANEL_MAX = 420;
let split = null;

function startSplitDrag(event, key, right) {
  const panel = document.querySelector(`[data-panel="${key}"]`);
  if (!panel) return;
  split = { key, right, startX: event.clientX, startWidth: S.layout[key], width: S.layout[key], panel, handle: event.target };
  event.target.classList.add('dragging');
  document.body.style.cursor = 'col-resize';
}

function onSplitMove(event) {
  if (!split) return;
  const dx = event.clientX - split.startX;
  // The member list grows to the left, the other panels to the right.
  const delta = split.right ? -dx : dx;
  split.width = Math.max(PANEL_MIN, Math.min(PANEL_MAX, split.startWidth + delta));
  split.panel.style.width = `${split.width}px`;
}

function onSplitEnd() {
  if (!split) return;
  S.layout[split.key] = split.width;
  split.handle.classList.remove('dragging');
  document.body.style.cursor = '';
  split = null;
  saveSoon();
}

// ----- startup -----

async function listenToServer() {
  await listen('net', (e) => onNet(e.payload));
  await listen('net-closed', (e) => onNetClosed(e.payload));
  await listen('dm-link', (e) => {
    const { addr, id, up } = e.payload;
    if (up) {
      if (!S.dmLinks[addr] || S.dmLinks[addr].id !== id) S.dmLinks[addr] = newLink(id);
    } else if (S.dmLinks[addr]?.id === id) {
      delete S.dmLinks[addr];
    }
    render();
  });
  await listen('discovered', (e) => {
    const found = e.payload;
    if (!S.discover.found.some((f) => f.addr === found.addr)) S.discover.found.push(found);
    S.discover.searching = false;
    render();
  });
}

async function boot() {
  document.addEventListener('click', onClick);
  document.addEventListener('contextmenu', onContextMenu);
  document.addEventListener('input', onInput);
  document.addEventListener('change', onChange);
  document.addEventListener('submit', onSubmit);
  document.addEventListener('keydown', onKeydown);
  document.addEventListener('mousedown', onMouseDown);
  document.addEventListener('mousemove', onSplitMove);
  document.addEventListener('mouseup', onSplitEnd);
  await listenToServer();

  const [prefs, catalog] = await Promise.all([invoke('prefs_load'), invoke('emoji_catalog')]);
  if (prefs) {
    S.prefs.servers = prefs.servers ?? [];
    S.prefs.theme = THEMES.some(([id]) => id === prefs.theme) ? prefs.theme : 'breeze-dark';
    S.prefs.skinTone = prefs.skinTone ?? 0;
    S.prefs.session = prefs.session ?? null;
    S.lastChannels = prefs.lastChannels ?? {};
    for (const key of Object.keys(S.layout)) {
      const saved = prefs.layout?.[key];
      if (typeof saved === 'number') S.layout[key] = Math.max(PANEL_MIN, Math.min(PANEL_MAX, saved));
    }
  }
  S.emoji.list = catalog.emoji;
  S.emoji.skin = Object.fromEntries(catalog.skin);
  applyTheme();
  render();

  const saved = S.prefs.session;
  if (saved) {
    try {
      const r = await invoke('restore_session', { saved: { name: saved.name } });
      if (r.user) {
        S.user = r.user;
        S.authDown = r.auth_down;
        S.notice = r.notice ?? null;
        S.remember = true;
        startDmTimer();
        const index = S.prefs.servers.findIndex((s) => s.addr === saved.server);
        if (index >= 0) selectServer(index);
      } else {
        S.prefs.session = null;
        S.login.error = r.error ?? null;
        saveSoon();
      }
    } catch (e) {
      S.login.error = errMsg(e);
      S.authDown = !!e?.auth_down;
    }
    render();
  }
}

boot().catch((e) => {
  console.error(e);
  appEl.textContent = `Could not start Vaxelium: ${errMsg(e)}`;
});
