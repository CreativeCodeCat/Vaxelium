// The views. Each function returns HTML for one part of the window. Buttons and menu items carry
// a `data-act` name and their arguments as data attributes; app.js handles them.

import { S, THEMES, chatReady, channelLabel, currentEntries, currentPerson, canDelete, canEdit,
  entryTime, isDmChannel, isLoggedIn, isOwner, persons, personName, personTargets, selectedName,
  selectedServer, GROUP_GAP_SECS } from './state.js';
import { ICON, dayKey, dayLabel, emojiHtml, esc, initial, nameColor, stampLabel, timeLabel } from './util.js';

export const QUICK_REACTIONS = ['👍', '❤️', '😂'];
const TONE_NAMES = ['Default', 'Light skin tone', 'Medium-light skin tone', 'Medium skin tone',
  'Medium-dark skin tone', 'Dark skin tone'];
const CERT_CHANGED = 'certificate has changed';

/** A draggable edge between a side panel and the centre column. `right` is for the member list. */
function splitter(key, right = false) {
  return `<div class="splitter" data-split="${key}" data-right="${right ? 1 : 0}" title="Drag to resize"></div>`;
}

/** Attributes for an element: {addr: 'x', id: 3} becomes data-addr="x" data-id="3". */
function data(args = {}) {
  return Object.entries(args)
    .map(([k, v]) => ` data-${k}="${esc(v)}"`)
    .join('');
}

/** A button that runs `act` with the given arguments. */
function act(name, args, inner, cls = '', extra = '') {
  return `<button type="button" class="${cls}" data-act="${name}"${data(args)}${extra}>${inner}</button>`;
}

/** An icon button with a tooltip. */
function iconBtn(name, args, icon, title, cls = '') {
  return `<button type="button" class="icon-btn ${cls}" data-act="${name}"${data(args)} title="${esc(title)}" aria-label="${esc(title)}">${icon}</button>`;
}

/** A placeholder for a feature that is not built yet. */
function soonBtn(icon, title, cls = '') {
  return `<button type="button" class="icon-btn placeholder ${cls}" data-act="soon" title="${esc(title)}" aria-label="${esc(title)}">${icon}</button>`;
}

/** Shows a server's uploaded icon, or a coloured square with its first letter. */
function serverIcon(server, cls, size) {
  if (server.icon) {
    return `<img class="${cls}" alt="" src="data:image/png;base64,${server.icon}">`;
  }
  return `<div class="${cls}" style="background:${nameColor(server.name)};font-size:${size}px">${esc(initial(server.name))}</div>`;
}

function avatar(name, size, status) {
  const dot = status ? `<span class="status" style="width:${Math.round(size * 0.34)}px;height:${Math.round(size * 0.34)}px"></span>` : '';
  return `<div class="avatar" style="width:${size}px;height:${size}px;font-size:${Math.round(size * 0.45)}px;background:${nameColor(name)}">${esc(initial(name))}${dot}</div>`;
}

// ----- the window -----

export function titlebar() {
  const title = (() => {
    if (!isLoggedIn()) return 'Vaxelium';
    if (!chatReady()) return selectedName() ?? 'Vaxelium';
    if (S.home) return channelLabel();
    return `${channelLabel()}  ·  ${selectedName()}`;
  })();
  return `
    <header class="titlebar" data-tauri-drag-region>
      <div class="title" data-tauri-drag-region>${esc(title)}</div>
      <div class="win-buttons">
        <button type="button" data-act="win-min" title="Minimize" aria-label="Minimize">${ICON.minimize}</button>
        <button type="button" data-act="win-max" title="Maximize" aria-label="Maximize">${ICON.maximize}</button>
        <button type="button" class="close" data-act="win-close" title="Close" aria-label="Close">${ICON.close}</button>
      </div>
    </header>`;
}

export function authBanner() {
  if (!S.authDown || S.authBannerClosed) return '';
  return `
    <div class="auth-banner" role="alert">
      <span>The auth service cannot be reached. You can't log in or create accounts until it is back.
        Servers still accept your saved login for up to 24 hours.</span>
      <button type="button" data-act="auth-banner-close" aria-label="Close">${ICON.close}</button>
    </div>`;
}

export function resizeEdges() {
  return ['n', 's', 'e', 'w', 'nw', 'ne', 'sw', 'se']
    .map((d) => `<div class="resize-edge ${d}" data-resize="${d}"></div>`)
    .join('');
}

export function body() {
  if (!isLoggedIn()) return loginPage();
  if (S.page === 'discover') return discoverPage();
  if (S.page === 'user') return userSettingsPage();
  if (S.page === 'server' && chatReady()) return serverSettingsPage();
  return `${railHtml()}${S.home && chatReady() ? homeView() : chatReady() ? chatView() : serverStatusPage()}`;
}

// ----- login -----

function loginPage() {
  const L = S.login;
  const submit = L.register ? 'Register' : 'Log in';
  const nameField = L.register ? `
      <div class="field">
        <span class="label">Display name</span>
        <input type="text" id="login-name" data-model="login.name" value="${esc(L.name)}" placeholder="your name" autocomplete="name">
      </div>` : '';
  const certButton = L.error && L.error.includes(CERT_CHANGED)
    ? `<div class="actions"><button type="button" class="btn primary wide" data-act="trust-auth">Trust the new certificate</button></div>` : '';
  return `
    <div class="center-page">
      <form class="card" data-form="login" autocomplete="off">
        <h1>${L.register ? 'Create an account' : 'Welcome back!'}</h1>
        <p class="sub">One account works on every Vaxelium server.</p>
        ${nameField}
        <div class="field">
          <span class="label">Email</span>
          <input type="text" id="login-email" data-model="login.email" value="${esc(L.email)}" placeholder="you@example.com" autocomplete="username">
        </div>
        <div class="field">
          <span class="label">Password</span>
          <input type="password" id="login-password" data-model="login.password" value="${esc(L.password)}" placeholder="password" autocomplete="current-password">
        </div>
        <label class="check"><input type="checkbox" data-check="remember" ${S.remember ? 'checked' : ''}> Keep me logged in on this computer</label>
        <button type="submit" class="btn primary wide">${submit}</button>
        <div class="switch">
          <span>${L.register ? 'Already have an account?' : 'Need an account?'}</span>
          <button type="button" class="link" data-act="login-toggle">${L.register ? 'Log in' : 'Register'}</button>
        </div>
        ${L.error ? `<p class="error">${esc(L.error)}</p>` : ''}
        ${certButton}
      </form>
    </div>`;
}

/** The page shown when logged in with no server open. */
function serverStatusPage() {
  const servers = S.prefs.servers;
  let body;
  if (servers.length === 0) {
    body = `
      <h1>Add a server</h1>
      <p class="sub">Enter the address of a Vaxelium server to start chatting.</p>
      <button type="button" class="btn primary wide" data-act="add-server-open">Add a server</button>`;
  } else if (S.main.connecting || (S.main.id !== null && !S.main.welcomed && !S.main.error)) {
    body = `
      <h1>Joining ${esc(selectedName() ?? 'server')}...</h1>
      <p class="sub">Checking your account with the server.</p>`;
  } else if (S.main.error) {
    const cert = S.main.error.includes(CERT_CHANGED);
    body = `
      <h1>Couldn't join the server</h1>
      <p class="error">${esc(S.main.error)}</p>
      <div class="actions">${cert
        ? act('trust-server', {}, 'Trust the new certificate', 'btn primary wide')
        : (S.selected !== null ? act('retry-server', {}, 'Try again', 'btn primary wide') : '')}</div>`;
  } else {
    body = `
      <h1>Pick a server</h1>
      <p class="sub">Choose one from the icons on the left, or add a new one with +.</p>`;
  }
  return `
    <div class="center-page">
      <div class="card">
        ${body}
        <div class="signed-in">
          <span>Signed in as ${esc(S.user)}</span>
          <button type="button" class="link" data-act="logout">Log out</button>
        </div>
      </div>
    </div>`;
}

// ----- server rail -----

function railHtml() {
  const servers = S.prefs.servers.map((server, i) => {
    const active = S.selected === i && !S.home ? 'active' : '';
    const label = esc(server.name);
    return `<button type="button" class="rail-btn ${active}" data-act="select-server" data-i="${i}" data-ctx="rail" data-i-ctx="${i}" title="${label}" aria-label="${label}">${serverIcon(server, 'icon', 20)}</button>`;
  }).join('');
  const sep = `<div class="rail-sep"></div>`;
  return `
    <nav class="rail" aria-label="Servers">
      <button type="button" class="rail-btn dm ${S.home ? 'active' : ''}" data-act="home-open" title="Direct messages" aria-label="Direct messages"><img src="assets/dm.svg" width="24" height="24" alt=""></button>
      ${sep}
      ${servers}
      <button type="button" class="rail-btn add" data-act="add-server-open" title="Add a server" aria-label="Add a server">+</button>
      <button type="button" class="rail-btn discover" data-act="discover-open" title="Find servers" aria-label="Find servers">${ICON.explore.replace('width="16" height="16"', 'width="22" height="22"')}</button>
    </nav>`;
}

// ----- channel view -----

export function chatView() {
  return `
    ${sidebarHtml()}${splitter('channels')}
    <section class="center">
      ${chatHeader()}
      ${messagesHtml()}
      ${composerHtml()}
    </section>
    ${S.membersHidden ? '' : splitter('members', true) + membersHtml()}`;
}

function sidebarHtml() {
  const server = S.server;
  const name = esc(selectedName() ?? 'Vaxelium');
  const channels = server.channels
    .filter((c) => !S.channelsCollapsed || c === server.channel)
    .map((c) => {
      const selected = c === server.channel;
      const unread = S.unread.has(c);
      return `
        <div class="row ${selected ? 'selected' : ''} ${unread ? 'unread' : ''}" data-act="channel-open" data-name="${esc(c)}" data-ctx="channel" data-name-ctx="${esc(c)}" data-unread="${unread ? 1 : 0}">
          <span class="label-text"># ${esc(c)}</span>
          ${unread && !selected ? '<span class="dot"></span>' : ''}
          <button type="button" class="gear" data-act="channel-menu" data-name="${esc(c)}" data-unread="${unread ? 1 : 0}" aria-label="Channel menu">${ICON.gear}</button>
        </div>`;
    }).join('');
  const create = S.createOpen ? `
    <div class="create-box">
      <input type="text" id="new-channel" data-model="newChannel" data-enter="create-channel" value="${esc(S.newChannel)}" placeholder="Create channel" autocomplete="off">
    </div>` : '';
  return `
    <aside class="sidebar" data-panel="channels" style="width:${S.layout.channels}px">
      <div class="server-header" data-act="server-menu" role="button" tabindex="0">
        ${server.public ? `<img src="assets/house.svg" width="14" height="14" alt="" title="Known by other servers">` : ''}
        <span class="name">${name}</span>
        <span class="chev">${ICON.chevronDown}</span>
      </div>
      <div class="channel-list">
        <div class="section-head">
          <button type="button" data-act="toggle-channels" aria-label="Fold channels">${S.channelsCollapsed ? ICON.chevronRight : ICON.chevronDown}</button>
          <span class="label" data-act="toggle-channels">Text channels</span>
          <button type="button" data-act="create-open" title="Create channel" aria-label="Create channel">${ICON.plus}</button>
        </div>
        ${channels}
        ${create}
      </div>
      ${userBar()}
    </aside>`;
}

function userBar() {
  return `
    <div class="user-bar">
      ${avatar(S.user, 32, true)}
      <div class="who">
        <div class="name">${esc(S.user)}</div>
        <div class="state">Online</div>
      </div>
      ${iconBtn('user-settings-open', {}, ICON.gear, 'User settings')}
      ${soonBtn(ICON.headset, 'Voice chat isn\'t available yet')}
      ${soonBtn(ICON.mic, 'Voice chat isn\'t available yet')}
    </div>`;
}

function chatHeader() {
  const members = iconBtn('toggle-members', {}, ICON.members,
    S.membersHidden ? 'Show member list' : 'Hide member list', S.membersHidden ? '' : 'active');
  return `
    <header class="chat-header">
      <span class="hash">#</span>
      <span class="title">${esc(S.server.channel)}</span>
      <span class="spacer"></span>
      <label class="search-box">
        ${ICON.search}
        <input type="text" id="search" data-model="search" data-rerender placeholder="Search ${esc(selectedName() ?? 'Vaxelium')}" value="${esc(S.search)}" autocomplete="off">
      </label>
      <div class="header-icons">
        ${members}
        ${soonBtn(ICON.pin, 'Not available yet')}
        ${soonBtn(ICON.bell, 'Not available yet')}
        ${soonBtn(ICON.hash, 'Not available yet')}
      </div>
    </header>`;
}

function composerHtml() {
  const label = currentPerson() !== null || isDmChannel(S.server.channel)
    ? channelLabel() : `#${S.server.channel}`;
  return `
    <form class="composer" data-form="send" autocomplete="off">
      <div class="composer-row">
        ${soonBtn(ICON.plus, 'Not available yet', 'plus')}
        <div class="composer-box">
          <input type="text" id="composer" data-model="input" placeholder="Message ${esc(label)}" value="${esc(S.input)}">
          ${soonBtn(ICON.gift, 'Not available yet')}
          ${soonBtn(ICON.apps, 'Not available yet')}
          ${soonBtn(ICON.sticker, 'Not available yet')}
          ${soonBtn(ICON.emoji, 'Not available yet')}
        </div>
        ${soonBtn(ICON.apps, 'Not available yet')}
      </div>
      ${S.status.startsWith('Error') ? `<div class="composer-status">${esc(S.status)}</div>` : ''}
    </form>`;
}

function membersHtml() {
  const server = S.server;
  const owner = server.owner;
  const row = (name, online) => `
    <div class="member" data-act="user-menu" data-name="${esc(name)}" data-ctx="user" data-name-ctx="${esc(name)}" data-can-msg="${name !== S.user ? 1 : 0}">
      ${avatar(name, 32, online)}
      <span class="who" style="color:${online ? (name === S.user ? 'var(--text-strong)' : 'var(--text)') : 'var(--text-muted)'}">${esc(name)}</span>
      ${owner === name ? `<img class="crown" src="assets/crown.svg" alt="owner" title="Server owner">` : ''}
    </div>`;
  const online = server.online.filter((n) => n !== owner);
  const offline = server.offline.filter((n) => n !== owner);
  return `
    <aside class="members" data-panel="members" style="width:${S.layout.members}px">
      ${owner ? `<div class="member-head">OWNER — 1</div>${row(owner, server.online.includes(owner))}<div style="height:12px"></div>` : ''}
      <div class="member-head">ONLINE — ${online.length}</div>
      ${online.map((n) => row(n, true)).join('')}
      <div style="height:12px"></div>
      <div class="member-head">OFFLINE — ${offline.length}</div>
      ${offline.map((n) => row(n, false)).join('')}
    </aside>`;
}

// ----- messages -----

/** The message list for the open view, with day dividers, grouped authors and the "NEW" line. */
function messagesHtml() {
  const person = currentPerson();
  const channel = S.server.channel;
  const query = S.search.trim().toLowerCase();
  const entries = currentEntries();
  const newAt = person === null ? S.readMarks[channel] : undefined;
  const welcome = person !== null || isDmChannel(channel)
    ? `<h1>${esc(channelLabel())}</h1><p>This is the start of your direct messages with ${esc(channelLabel().replace(/^@ /, ''))}.</p>`
    : `<h1>Welcome to #${esc(channel)}!</h1><p>This is the start of the #${esc(channel)} channel.</p>`;
  const matches = (e) => e.kind === 'chat' && (e.m.text.toLowerCase().includes(query) || e.m.author.toLowerCase().includes(query));

  let out = '';
  if (query && !entries.some(matches)) out += `<div class="no-match">No messages match your search.</div>`;
  let prevDay = null;
  let prevAuthor = null;
  let prevAt = 0;
  entries.forEach((entry, i) => {
    if (query && !matches(entry)) return;
    if (!query && newAt === i) {
      out += `<div class="new-divider"><span class="pill">NEW</span></div>`;
      prevAuthor = null;
    }
    const at = entryTime(entry);
    const day = dayKey(at);
    if (day !== prevDay) {
      out += `<div class="day-divider">${esc(dayLabel(at))}</div>`;
      prevAuthor = null;
      prevDay = day;
    }
    if (entry.kind === 'system') {
      out += `<div class="msg"><div class="body"><div class="system-text">${esc(entry.text)}</div></div></div>`;
      prevAuthor = null;
    } else {
      const m = entry.m;
      const grouped = prevAuthor === m.author && at - prevAt < GROUP_GAP_SECS;
      out += chatMessage(m, grouped);
      prevAuthor = m.author;
    }
    prevAt = at;
  });
  return `
    <div class="messages" id="messages" data-scroll="${esc(person !== null ? `person-${channel}` : channel)}">
      <div class="welcome">${welcome}</div>
      ${out}
    </div>`;
}

function chatMessage(m, grouped) {
  const editing = S.editing && S.editing.id === m.id;
  const text = editing
    ? `<div class="edit-box"><input type="text" id="edit-box" data-model="editing.text" data-enter="edit-save" data-escape="edit-cancel" value="${esc(S.editing.text)}"><div class="edit-hint">Enter to save · Escape to cancel</div></div>`
    : `<div class="text" title="${esc(timeLabel(m.timestamp))}">${esc(m.text)}${m.edited ? '<span class="edited">(edited)</span>' : ''}</div>`;
  const reactions = m.reactions.length ? `<div class="reactions">${m.reactions.map((r) => {
    const mine = r.users.includes(S.user);
    return `<button type="button" class="reaction ${mine ? 'mine' : ''}" data-act="react" data-id="${m.id}" data-emoji="${esc(r.emoji)}" title="${esc(r.users.join(', '))}">${emojiHtml(r.emoji, 16)} ${r.users.length}</button>`;
  }).join('')}</div>` : '';
  const head = grouped ? '' : `
      <div class="meta">
        <span class="author" style="color:${nameColor(m.author)}" data-act="user-menu" data-name="${esc(m.author)}" data-ctx="user" data-name-ctx="${esc(m.author)}" data-can-msg="${m.author !== S.user ? 1 : 0}">${esc(m.author)}</span>
        <span class="stamp">${esc(stampLabel(m.timestamp))}</span>
      </div>`;
  const avatarHtml = grouped ? '' : `<div class="avatar" style="background:${nameColor(m.author)}">${esc(initial(m.author))}</div>`;
  const toolbar = m.id !== 0 && !editing ? `
      <div class="toolbar" data-toolbar="${m.id}">
        ${QUICK_REACTIONS.map((e) => `<button type="button" class="quick" data-act="react" data-id="${m.id}" data-emoji="${esc(e)}" title="React ${esc(e)}">${emojiHtml(e, 20)}</button>`).join('')}
        <button type="button" class="smile" data-act="picker" data-id="${m.id}" title="Add reaction" aria-label="Add reaction">☺</button>
        ${canDelete(m) ? `<button type="button" data-act="more" data-id="${m.id}" title="More" aria-label="More">⋯</button>` : ''}
      </div>` : '';
  return `
    <div class="msg ${grouped ? 'grouped' : 'head'}" data-mid="${m.id}" data-ctx="message" data-id-ctx="${m.id}">
      ${avatarHtml}
      <div class="body">${head}${text}${reactions}</div>
      ${toolbar}
    </div>`;
}

// ----- direct messages page -----

function homeView() {
  const person = currentPerson();
  const open = person !== null || isDmChannel(S.server.channel);
  const rows = persons().map((p) => {
    const selected = person === p.id;
    return `
      <div class="row ${selected ? 'selected' : ''} ${p.unread && !selected ? 'unread' : ''}" data-act="open-person" data-id="${esc(p.id)}">
        <span class="label-text">@ ${esc(p.name)}</span>
        ${p.unread && !selected ? '<span class="dot"></span>' : ''}
      </div>`;
  }).join('');
  const anyDms = Object.values(S.dmLinks).some((l) => l.dms.length > 0);
  const emptyText = S.prefs.servers.length === 0
    ? 'Add a server to see your direct messages.'
    : Object.keys(S.dmLinks).length === 0
      ? 'Connecting to your servers...'
      : 'No direct messages yet. Right-click a member\'s name and choose Message.';
  const targets = person !== null ? personTargets(person) : [];
  const current = S.main.addr;
  const select = targets.length > 1 ? `
      <select class="dm-select" data-change="dm-server" aria-label="Send new messages to">
        ${targets.map((t, i) => `<option value="${i}" ${t.addr === current ? 'selected' : ''}>Sending to ${esc(t.server)}</option>`).join('')}
      </select>` : '';
  return `
    <aside class="sidebar" data-panel="dms" style="width:${S.layout.dms}px">
      <div class="section-head" style="padding-top:12px"><span class="label" style="padding-left:4px">Direct messages</span></div>
      <div class="channel-list">
        ${anyDms ? rows : `<p class="muted" style="padding:4px 8px">${esc(emptyText)}</p>`}
      </div>
      ${userBar()}
    </aside>
    ${splitter('dms')}
    <section class="center">
      <header class="chat-header">
        <span class="title">${esc(person !== null ? `@ ${personName(person) ?? channelLabel()}` : 'Direct messages')}</span>
        <span class="spacer"></span>
        ${select}
      </header>
      ${open ? messagesHtml() : `<div class="empty-state">Pick a conversation on the left.</div>`}
      ${open ? composerHtml() : ''}
    </section>
    `;
}

// ----- discover, user settings and server settings pages -----

function discoverPage() {
  const d = S.discover;
  const cards = d.found.map((f) => {
    const saved = S.prefs.servers.some((s) => s.addr === f.addr);
    const expanded = d.expanded.has(f.addr);
    const icon = f.icon
      ? `<img class="icon-big" alt="" src="data:image/png;base64,${f.icon}">`
      : `<div class="icon-big" style="background:${nameColor(f.name)}">${esc(initial(f.name))}</div>`;
    return `
      <div class="server-card ${expanded ? 'expanded' : ''}" data-act="discover-toggle" data-addr="${esc(f.addr)}">
        ${icon}
        <div class="info">
          <div class="title">${esc(f.name)}${f.public ? ' <img src="assets/house.svg" width="14" height="14" alt="" title="Known by other servers">' : ''}</div>
          ${f.description ? `<div class="desc">${esc(f.description)}</div>` : ''}
          <div class="meta">${esc(f.addr)} · <span class="green">${f.online} online</span></div>
        </div>
        <div class="side">${saved ? '<span class="added">Added</span>'
          : `<button type="button" class="btn primary small" data-act="discover-add" data-addr="${esc(f.addr)}">Add</button>`}</div>
      </div>`;
  }).join('');
  return `
    <div class="page">
      <nav class="page-nav">
        <div class="label">Discover</div>
        <div class="nav-item">Find servers</div>
      </nav>
      <main class="page-content">
        <div class="page-close">
          <button type="button" data-act="discover-close" aria-label="Close">${ICON.close}</button>
          <span class="esc">ESC</span>
        </div>
        <div class="page-title">Find servers</div>
        <p style="margin:-12px 0 20px">Servers found through the servers you know. Pick one to add it.</p>
        ${d.error ? `<p class="red">${esc(d.error)}</p>` : ''}
        ${!d.error && d.found.length === 0 ? `<p class="muted">Searching...</p>` : ''}
        ${cards}
      </main>
    </div>`;
}

function accountRow(label, valueHtml, actionHtml, desc = '') {
  return `
    <div class="setting-row">
      <div class="what"><div class="name">${esc(label)}</div>${desc ? `<div class="desc">${esc(desc)}</div>` : ''}</div>
      <div class="value">${valueHtml}</div>
      ${actionHtml}
    </div>`;
}

function userSettingsPage() {
  const A = S.account;
  const shownName = A.draft.name ?? S.user;
  const shownUser = A.draft.username ?? A.username;
  const emailValue = A.draft.email !== null
    ? esc(A.draft.email[0])
    : A.email !== null ? esc(A.email)
      : A.emailError ? '<span class="muted" title="' + esc(A.emailError) + '">Unavailable</span>'
        : '<span class="muted">Not on record yet. Set it with Edit.</span>';
  const theme = A.draft.theme ?? S.prefs.theme;
  const themeSelect = `<select class="theme-select" data-change="theme" aria-label="Theme">
      ${THEMES.map(([id, name]) => `<option value="${id}" ${id === theme ? 'selected' : ''}>${esc(name)}</option>`).join('')}
    </select>`;
  return `
    <div class="page">
      <nav class="page-nav">
        <div class="label">User settings</div>
        <div class="nav-item">Account</div>
      </nav>
      <main class="page-content">
        <div class="page-close">
          <button type="button" data-act="user-settings-close" aria-label="Close">${ICON.close}</button>
          <span class="esc">ESC</span>
        </div>
        <div class="page-title">Account</div>
        <div class="profile-head">
          ${avatar(S.user, 72, false)}
          <div><div class="name">${esc(shownName)}</div><div class="state">Signed in on this computer</div></div>
        </div>
        <div class="divider-line"></div>
        <div class="section-title">Account Info</div>
        ${accountRow('Display name', `<span class="strong">${esc(shownName)}</span>`,
          act('account-edit', { field: 'name' }, 'Edit', 'btn small'))}
        ${accountRow('Username', `<span>@${esc(shownUser ?? 'not loaded')}</span>`,
          act('account-edit', { field: 'username' }, 'Edit', 'btn small'))}
        ${accountRow('Email', emailValue, act('account-edit', { field: 'email' }, 'Edit', 'btn small'))}
        <div class="divider-line"></div>
        <div class="section-title">Preferences</div>
        ${accountRow('Theme', themeSelect, '')}
        <div class="divider-line"></div>
        <div class="section-title">Session</div>
        ${accountRow('Log out', '', act('logout', {}, 'Log Out', 'btn danger'), 'Sign out of Vaxelium on this computer.')}
      </main>
    </div>
    ${accountEditDialog()}
    ${accountSaveBar()}`;
}

function accountEditDialog() {
  const A = S.account;
  if (!A.editing) return '';
  const close = `<button type="button" class="x icon-btn" data-act="account-edit-cancel" aria-label="Close">${ICON.close}</button>`;
  let title;
  let fields;
  if (A.editing === 'name') {
    title = 'Edit display name';
    fields = `
      <span class="label">New display name</span>
      <input type="text" id="acc-name" data-model="account.nameInput" value="${esc(A.nameInput)}" placeholder="How your name appears in chat" style="margin-top:6px">`;
  } else if (A.editing === 'username') {
    title = 'Edit username';
    fields = `
      <span class="label">New username</span>
      <input type="text" id="acc-username" data-model="account.usernameInput" data-lower="1" value="${esc(A.usernameInput)}" placeholder="lowercase letters, digits and _" style="margin-top:6px">`;
  } else {
    title = 'Change email';
    fields = `
      <span class="label">New email</span>
      <input type="text" id="acc-email" data-model="account.emailInput" value="${esc(A.emailInput)}" placeholder="you@example.com" style="margin-top:6px">
      <span class="label" style="display:block;margin-top:12px">Current password</span>
      <input type="password" id="acc-email-password" data-model="account.emailPassword" value="${esc(A.emailPassword)}" style="margin-top:6px">`;
  }
  const err = A.formError ? `<div class="toast-error" style="margin-top:10px">${esc(A.formError)}</div>` : '';
  return `
    <div class="backdrop" data-act="account-edit-cancel"></div>
    <form class="dialog" data-form="account-edit" data-field="${A.editing}">
      ${close}
      <h2>${esc(title)}</h2>
      <div style="margin-top:12px">${fields}</div>
      ${err}
      <div class="row-buttons">
        <button type="button" class="btn" data-act="account-edit-cancel">Cancel</button>
        <button type="submit" class="btn primary">Save</button>
      </div>
    </form>`;
}

function accountSaveBar() {
  const A = S.account;
  const dirty = A.draft.name !== null || A.draft.username !== null || A.draft.email !== null || A.draft.theme !== null;
  if (!dirty) return '';
  return `
    <div class="unsaved">
      <div class="what">
        <div class="t">Careful – you have unsaved changes!</div>
        ${A.saveError ? `<div class="e">${esc(A.saveError)}</div>` : ''}
      </div>
      <button type="button" class="link" data-act="account-reset">Reset</button>
      <button type="button" class="btn green" data-act="account-save">Save Changes</button>
    </div>`;
}

function serverSettingsPage() {
  const F = S.serverForm;
  const server = selectedServer();
  const name = server?.name ?? 'Server';
  const dirty = (F.name.trim() !== '' && F.name.trim() !== name) || F.description.trim() !== S.server.description;
  return `
    <div class="page">
      <nav class="page-nav">
        <div class="label">${esc(name.toUpperCase())}</div>
        <div class="nav-item">Server Profile</div>
      </nav>
      <main class="page-content">
        <div class="page-close">
          <button type="button" data-act="server-settings-close" aria-label="Close">${ICON.close}</button>
          <span class="esc">ESC</span>
        </div>
        <div class="page-title">Server Profile</div>
        <p style="margin:-12px 0 24px" class="muted">Customise how your server appears to its members.</p>

        <div class="field">
          <span class="label">Server icon</span>
          <div style="display:flex;gap:12px;margin-top:8px">
            ${S.server.icon
              ? `<div class="icon-preview"><img alt="" src="data:image/png;base64,${S.server.icon}"></div>`
              : `<div class="icon-preview" style="background:${nameColor(name)}">${esc(initial(name))}</div>`}
            <div>
              <button type="button" class="btn small" data-act="icon-pick" style="min-height:32px;padding:0 14px">Upload image</button>
              ${S.server.icon ? `<div style="margin-top:6px"><button type="button" class="link" data-act="icon-remove">Remove icon</button></div>` : ''}
              ${F.iconError ? `<div class="toast-error">${esc(F.iconError)}</div>` : ''}
              <div class="small-text muted" style="margin-top:6px">PNG or JPG. It is shrunk to 128x128.</div>
            </div>
          </div>
        </div>

        <div class="field" style="max-width:560px;margin-top:20px">
          <span class="label">Name</span>
          <input type="text" id="server-name" data-model="serverForm.name" data-rerender data-enter="server-save" value="${esc(F.name)}" placeholder="My server" style="margin-top:6px">
        </div>
        <div class="field" style="max-width:560px;margin-top:20px">
          <span class="label">Description</span>
          <div class="small-text muted" style="margin:4px 0 6px">Tell people what this server is about.</div>
          <textarea id="server-description" data-model="serverForm.description" data-rerender maxlength="1000" placeholder="Welcome to our community!">${esc(F.description)}</textarea>
        </div>
      </main>
    </div>
    ${dirty ? `
    <div class="unsaved">
      <div class="what"><div class="t">Careful – you have unsaved changes!</div></div>
      <button type="button" class="link" data-act="server-reset">Reset</button>
      <button type="button" class="btn green" data-act="server-save">Save Changes</button>
    </div>` : ''}`;
}

// ----- dialogs and popups -----

/** The dialogs drawn over the window: delete confirmation and join a server. */
export function dialogs() {
  let out = '';
  if (S.pendingDelete !== null) {
    out += `
      <div class="backdrop" data-act="delete-cancel"></div>
      <div class="dialog">
        <button type="button" class="x icon-btn" data-act="delete-cancel" aria-label="Close">${ICON.close}</button>
        <h2>Delete message?</h2>
        <p class="lead">This message will be removed for everyone. It can't be undone.</p>
        <div class="row-buttons">
          <button type="button" class="btn" data-act="delete-cancel">Cancel</button>
          <button type="button" class="btn danger" data-act="delete-confirm">Delete</button>
        </div>
      </div>`;
  }
  if (S.addServer.open) out += joinServerDialog();
  return out;
}

function joinServerDialog() {
  const A = S.addServer;
  return `
    <div class="backdrop" data-act="add-server-close"></div>
    <form class="dialog" data-form="add-server">
      <button type="button" class="x icon-btn" data-act="add-server-close" aria-label="Close">${ICON.close}</button>
      <h2>Join a Server</h2>
      <p class="lead" style="margin-bottom:20px">Enter an address below to join an existing server</p>
      <span class="label" style="color:var(--text-strong)">Server address <span style="color:var(--red)">*</span></span>
      <input type="text" id="add-addr" data-model="addServer.addr" value="${esc(A.addr)}" placeholder="chat.example.com" style="margin-top:6px;padding:10px">
      <div class="small-text muted" style="margin:10px 0 6px">Addresses should look like</div>
      <span class="example">chat.example.com</span><span class="example">myserver.example.net</span>
      ${A.error ? `<div class="toast-error" style="margin-top:10px">${esc(A.error)}</div>` : ''}
      <div class="find-card" data-act="discover-from-add" role="button" tabindex="0">
        <div class="what"><div class="t">Don't have an address?</div><div class="d">Find servers on your network and the internet</div></div>
        <span class="muted" style="font-size:20px">›</span>
      </div>
      <div class="row-buttons" style="margin-top:12px">
        <button type="button" class="link" data-act="add-server-close">Back</button>
        <button type="submit" class="btn primary" style="width:132px">Join Server</button>
      </div>
    </form>`;
}

/** The popup for a context menu (right-click, or the ⋯ button). */
export function menuPopup() {
  const m = S.menu;
  if (!m) return '';
  const items = m.items.map((item) => {
    if (item === 'sep') return '<hr>';
    return `<button type="button" class="${item.danger ? 'danger' : ''}" data-act="${item.act}"${data(item.args)}>${item.icon ? item.icon : ''}${esc(item.label)}</button>`;
  }).join('');
  const width = m.width ? `;width:${m.width}px` : '';
  return `<div class="popup menu" id="menu" style="left:${m.x}px;top:${m.y}px${width}" data-menu="1">${items}</div>`;
}

/** The emoji picker for one message. */
export function pickerPopup() {
  const p = S.picker;
  if (!p) return '';
  const tone = S.prefs.skinTone;
  const sample = (i) => (i === 0 ? '👋' : (S.emoji.skin['👋']?.[i - 1] ?? '👋'));
  const tones = TONE_NAMES.map((name, i) => `
      <button type="button" class="${i === tone ? 'on' : ''}" data-act="picker-tone" data-tone="${i}" title="${esc(name)}" aria-label="${esc(name)}">${emojiHtml(sample(i), 20)}</button>`).join('');
  const query = S.pickerQuery.trim().toLowerCase();
  const categories = [];
  for (const [cat] of S.emoji.list) if (!categories.includes(cat)) categories.push(cat);
  let grid = '';
  for (const cat of categories) {
    const items = S.emoji.list.filter(([c, name]) => c === cat && (!query || name.toLowerCase().includes(query)));
    if (!items.length) continue;
    grid += `<div class="picker-cat">${esc(cat)}</div><div class="picker-items">${items.map(([, name, base]) => {
      const variants = S.emoji.skin[base];
      const e = variants && tone > 0 ? variants[tone - 1] : base;
      return `<button type="button" data-act="react" data-id="${p.id}" data-emoji="${esc(e)}" title="${esc(name)}" aria-label="${esc(name)}">${emojiHtml(e, 24)}</button>`;
    }).join('')}</div>`;
  }
  if (!grid) grid = '<p class="muted" style="padding:8px">No emoji match.</p>';
  return `
    <div class="popup picker" id="picker" style="left:${p.x}px;top:${p.y}px" data-picker="1">
      <input type="text" id="picker-search" data-model="pickerQuery" data-rerender placeholder="Search emoji" value="${esc(S.pickerQuery)}" autocomplete="off">
      <div class="tones">${tones}</div>
      <div class="picker-grid">${grid}</div>
    </div>`;
}

/** Items for a channel's menu. */
export function channelMenuItems(name, unread) {
  const items = [];
  if (unread) items.push({ label: 'Mark as read', act: 'mark-read', args: { name } });
  if (!isDmChannel(name)) items.push({ label: 'Copy channel name', act: 'copy', args: { text: `#${name}` } });
  return items;
}

/** Items for a user's name menu. */
export function userMenuItems(name, canMessage) {
  const items = [{ label: 'Mention', act: 'mention', args: { name } }];
  if (canMessage) items.push({ label: 'Message', act: 'message-user', args: { name } });
  items.push({ label: 'Copy name', act: 'copy', args: { text: name } });
  return items;
}

/** Items for a message's menu. */
export function messageMenuItems(m) {
  const items = [];
  if (canEdit(m)) items.push({ label: 'Edit Message', act: 'edit-start', args: { id: m.id }, icon: ICON.pencil });
  if (canEdit(m) && canDelete(m)) items.push('sep');
  items.push({ label: 'Delete Message', act: 'delete-ask', args: { id: m.id }, danger: true, icon: ICON.trash });
  return items;
}

/** Items for the open server's menu. */
export function serverMenuItems() {
  const items = [];
  if (isOwner()) items.push({ label: 'Server settings', act: 'server-settings-open' });
  items.push({ label: 'Create channel', act: 'create-open' });
  items.push('sep');
  items.push({ label: S.membersHidden ? 'Show member list' : 'Hide member list', act: 'toggle-members' });
  items.push({ label: 'Copy server address', act: 'copy', args: { text: S.main.addr ?? '' } });
  items.push('sep');
  items.push({ label: 'Leave server', act: 'remove-server', args: { i: S.selected ?? -1 }, danger: true });
  return items;
}

/** Items for a server icon's right-click menu in the rail. */
export function railMenuItems(i) {
  if (S.selected === i && chatReady()) return serverMenuItems();
  return [{ label: 'Remove server', act: 'remove-server', args: { i }, danger: true }];
}
