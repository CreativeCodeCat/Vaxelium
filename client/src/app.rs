//! The app: its state, every message the window can send, and how each one changes the state.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::time::Duration;

use common::{perm, ClientMsg, ServerMsg};
use iced::futures::channel::oneshot;
use iced::widget::operation;
use iced::{event, keyboard, mouse, window, Point, Subscription, Task};

use crate::backend::{Backend, CmdError, Event, Profile, Restored};
use crate::discover::FoundServer;
use crate::model::{is_dm_channel, DmLink, Entry, MainConn, ServerView, VIRTUAL_BASE};
use crate::prefs::{Prefs, ServerPref, SessionPref};
use crate::search::Search;
use crate::theme;

pub const DM_TICK: Duration = Duration::from_secs(5);
pub const MESSAGES_ID: &str = "messages";
pub const SEARCH_ID: &str = "search";
pub const COMPOSER_ID: &str = "composer";
pub const NEW_CHANNEL_ID: &str = "new-channel";
pub const NEW_CATEGORY_ID: &str = "new-category";
pub const RENAME_ID: &str = "rename-channel";
pub const ADD_ADDR_ID: &str = "add-addr";
pub const NEW_DM_ID: &str = "new-dm";
pub const EDIT_ID: &str = "edit-box";
pub const PANEL_MIN: f32 = 250.0;
pub const PANEL_MAX: f32 = 420.0;

/// Which page fills the window besides the chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    None,
    Discover,
    User,
    Server,
}

/// A section of the server settings page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ServerSection {
    #[default]
    Profile,
    Roles,
}

/// A tab of the role editor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RoleTab {
    #[default]
    Display,
    Permissions,
}

/// A role being edited on the roles page. `id` is `None` for a new role.
#[derive(Debug, Clone, Default)]
pub struct RoleDraft {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub permissions: u32,
}

/// Which account field is being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Username,
    Email,
}

#[derive(Debug, Clone, Default)]
pub struct LoginForm {
    pub register: bool,
    pub name: String,
    pub email: String,
    pub password: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AddServer {
    pub open: bool,
    pub addr: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Discover {
    pub found: Vec<FoundServer>,
    pub error: Option<String>,
    pub expanded: HashSet<String>,
    pub searching: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Account {
    pub email: Option<String>,
    pub email_error: Option<String>,
    pub username: Option<String>,
    pub editing: Option<Field>,
    pub name_input: String,
    pub username_input: String,
    pub email_input: String,
    pub email_password: String,
    pub form_error: Option<String>,
    /// Changes not saved yet.
    pub draft_name: Option<String>,
    pub draft_username: Option<String>,
    pub draft_email: Option<(String, String)>,
    pub draft_theme: Option<String>,
    pub save_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerForm {
    pub name: String,
    pub description: String,
    pub icon_error: Option<String>,
}

/// A popup menu at a point on the window.
#[derive(Debug, Clone)]
pub struct Menu {
    pub at: Point,
    pub items: Vec<MenuItem>,
    pub width: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum MenuItem {
    Action { label: String, icon: Option<&'static str>, danger: bool, msg: Message },
    Heading(String),
    Divider,
}

/// The emoji picker for one message.
#[derive(Debug, Clone)]
pub struct Picker {
    pub message: i64,
    pub at: Point,
}

/// What is being dragged: a channel or a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragItem {
    Channel(i64),
    Category(i64),
}

/// Where a dragged item would be dropped: onto a category, or into the list outside any category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropTarget {
    Category(i64),
    Uncategorized,
    /// After the last category.
    End,
    /// Before this channel, in the category it is in (`None` for the uncategorized list).
    Channel { id: i64, category: Option<i64> },
}

/// A press on a channel or category that may turn into a drag. If it does not move, it is a click.
#[derive(Debug, Clone)]
pub struct Dnd {
    pub item: DragItem,
    pub start: Point,
    pub moved: bool,
    pub over: Option<DropTarget>,
}

/// A panel edge being dragged.
#[derive(Debug, Clone)]
pub struct Drag {
    pub key: &'static str,
    pub right: bool,
    pub start_x: f32,
    pub start_width: f32,
    pub width: f32,
}

/// What a menu is opened for. Its items are built when it opens.
#[derive(Debug, Clone)]
pub enum MenuKind {
    Channel { id: i64, name: String, unread: bool },
    User { name: String, can_message: bool },
    Rail(usize),
    Message(i64),
    Server,
    Theme,
    Category { id: i64, name: String },
}

/// Everything the app shows and remembers.
pub struct App {
    pub backend: Backend,
    pub prefs: Prefs,
    /// Emoji for reactions: (category, name, base emoji).
    pub emoji: Vec<(String, String, String)>,
    /// Skin tone variants of emoji that have them, by base emoji.
    pub skin: HashMap<String, [String; 5]>,
    pub window: Option<window::Id>,
    pub cursor: Point,

    pub user: Option<String>,
    pub user_id: String,
    pub auth_down: bool,
    pub auth_banner_closed: bool,
    pub notice: Option<String>,
    pub remember: bool,
    pub login: LoginForm,

    pub selected: Option<usize>,
    pub home: bool,
    pub page: Page,
    pub add_server: AddServer,
    pub discover: Discover,
    pub account: Account,
    pub server_form: ServerForm,
    /// The role being edited on the roles page, if one is open.
    pub role_form: Option<RoleDraft>,
    /// The section of the server settings page that is showing.
    pub server_section: ServerSection,
    /// The tab open in the role editor.
    pub role_tab: RoleTab,
    /// The last refusal from the server about roles, shown on the roles page.
    pub role_error: Option<String>,
    /// The editor state for the description box. `server_form.description` follows its text.
    pub description_editor: iced::widget::text_editor::Content,

    pub main: MainConn,
    pub server: ServerView,
    /// Messages by channel id, and the channels with unread messages, by id.
    pub messages: HashMap<i64, Vec<Entry>>,
    pub unread: HashSet<i64>,
    pub read_marks: HashMap<i64, usize>,
    pub search: String,
    pub search_more: bool,
    pub editing: Option<(i64, String)>,
    pub pending_delete: Option<i64>,
    pub input: String,
    pub new_channel: String,
    pub create_open: bool,
    /// The category name box in the channel list is showing.
    pub category_open: bool,
    /// The rename dialog: the channel's current name, and the name being typed.
    pub rename_channel: Option<(i64, String, String)>,
    /// A channel and category press in progress, and the drop target under the pointer.
    pub dnd: Option<Dnd>,
    /// The channel waiting for its delete confirmation.
    pub pending_channel_delete: Option<(i64, String)>,
    /// The role waiting for confirmation before it is deleted.
    pub pending_role_delete: Option<(i64, String)>,
    pub new_category: String,
    pub members_hidden: bool,
    /// Categories whose channels are folded away, by category id.
    pub collapsed_categories: HashSet<i64>,
    pub menu: Option<Menu>,
    pub picker: Option<Picker>,
    pub picker_query: String,
    /// A direct message to open once the server it is on has been joined.
    pub pending_dm: Option<(String, String)>,
    pub dm_new_name: String,
    pub dm_links: HashMap<String, DmLink>,
    pub status: String,
    pub drag: Option<Drag>,
    /// The search filter menu is showing.
    pub search_open: bool,
    /// The message under the pointer, for its toolbar.
    pub hover: Option<i64>,
}

/// Every message the window and the background work send to the app.
#[derive(Debug, Clone)]
pub enum Message {
    Event(Event),
    Tick,
    WindowOpened(window::Id),
    CursorMoved(Point),
    MouseReleased,
    Escape,
    WinMinimize,
    WinMaxToggle,
    WinClose,
    WinDrag,
    Resize(window::Direction),

    LoginName(String),
    LoginEmail(String),
    LoginPassword(String),
    LoginToggle,
    LoginSubmit,
    LoginDone(Result<String, CmdError>),
    Remember(bool),
    TrustAuth,
    Restored(Restored),
    AuthBannerClose,
    Logout,
    LoggedOut,

    SelectServer(usize),
    RemoveServer(usize),
    Connected(u64, Result<u64, CmdError>),
    RetryServer,
    TrustServer,
    AddServerOpen,
    AddServerClose,
    AddServerAddr(String),
    AddServerSubmit,
    FromAddDiscover,
    DiscoverOpen,
    DiscoverClose,
    DiscoverToggle(String),
    DiscoverAdd(String),

    HomeOpen,
    OpenPerson(String),
    NewDmName(String),
    NewDmSubmit,
    DmServerPick(DmChoice),

    DndPress(DragItem),
    DndOver(Option<DropTarget>),
    DeleteChannelAsk(i64, String),
    DeleteChannelConfirm,
    DeleteChannelCancel,
    CreateOpen,
    CategoryOpen,
    NewCategory(String),
    CreateCategory,
    DeleteCategory(i64),
    RenameStart(i64, String),
    RenameInput(String),
    RenameSubmit,
    RenameCancel,
    NewChannel(String),
    CreateChannel,
    ToggleCategory(i64),
    ToggleMembers,
    MarkRead(i64),
    CopyText(String),
    SearchChanged(String),
    SearchOpen,
    Hover(Option<i64>),
    SearchFilter(&'static str),
    /// Picks a suggestion: the word being typed is replaced by this filter.
    SearchPick(String),
    /// Clears the search text.
    SearchClear,
    SearchMore,
    Mention(String),
    MessageUser(String),

    InputChanged(String),
    Send,
    React(i64, String),
    PickerOpen(i64),
    PickerClose,
    PickerQuery(String),
    PickerTone(usize),
    EditStart(i64),
    EditChanged(String),
    EditSave,
    DeleteAsk(i64),
    DeleteConfirm,
    DeleteCancel,

    OpenMenu(MenuKind),
    CloseMenu,

    UserSettingsOpen,
    UserSettingsClose,
    ProfileLoaded(Result<Profile, CmdError>),
    AccountEdit(Field),
    AccountEditCancel,
    AccountNameInput(String),
    AccountUsernameInput(String),
    AccountEmailInput(String),
    AccountEmailPassword(String),
    AccountEditSubmit,
    AccountReset,
    AccountSave,
    AccountSaved(AccountSaved),
    ThemePick(String),

    ServerSettingsOpen,
    ServerSettingsClose,
    /// Switch the server settings page to another section.
    ServerSectionPick(ServerSection),
    /// Open the editor for a role, or for a new one with `None`.
    RoleEdit(Option<i64>),
    RoleEditCancel,
    RoleDraftName(String),
    /// Pick a colour from the palette.
    RoleColorPick(String),
    /// Switch the role editor to another tab.
    RoleTab(RoleTab),
    RoleDraftColor(String),
    RoleDraftPerm(u32, bool),
    RoleSave,
    /// Put the role's saved values back into the editor.
    RoleReset,
    /// Ask before deleting a role, with its name for the dialog.
    RoleDeleteAsk(i64, String),
    RoleDeleteConfirm,
    RoleDeleteCancel,
    /// Move a role so it sits just above another one, or at the bottom with `None`.
    RoleMove(i64, Option<i64>),
    /// Give a member a role (`true`) or take it away (`false`): the member's id, the role, and on/off.
    SetMemberRole(String, i64, bool),
    ServerName(String),
    /// An edit in the multi-line description box.
    ServerDescriptionEdit(iced::widget::text_editor::Action),
    ServerSave,
    ServerReset,
    IconPick,
    IconPicked(Result<Option<String>, CmdError>),
    IconRemove,

    DragStart(&'static str, bool),
}

/// The result of saving the account page: what was stored, and the first error if a step failed.
#[derive(Debug, Clone)]
pub struct AccountSaved {
    pub profile: Option<Profile>,
    pub email: Option<String>,
    pub name_saved: bool,
    pub username_saved: bool,
    pub email_saved: bool,
    pub error: Option<String>,
}

/// Runs blocking work on a worker thread and returns a future for its result, so the window stays
/// responsive while the network is used.
pub fn offload<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> impl Future<Output = T> {
    let (tx, rx) = oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    async move { rx.await.expect("background work ended without a result") }
}

/// Sends a message to the open server. Errors are ignored: a dropped connection is reported by its
/// own event.
fn send_main(app: &App, msg: ClientMsg) {
    let _ = app.backend.send_main(msg);
}

fn send_dm(app: &App, addr: &str, msg: ClientMsg) {
    let _ = app.backend.send_dm(addr, msg);
}

impl App {
    pub fn new(backend: Backend, prefs: Prefs, emoji: Vec<(String, String, String)>, skin: HashMap<String, [String; 5]>) -> Self {
        Self {
            backend,
            prefs,
            emoji,
            skin,
            window: None,
            cursor: Point::ORIGIN,
            user: None,
            user_id: String::new(),
            auth_down: false,
            auth_banner_closed: false,
            notice: None,
            remember: true,
            login: LoginForm::default(),
            selected: None,
            home: false,
            page: Page::None,
            add_server: AddServer::default(),
            discover: Discover::default(),
            account: Account::default(),
            server_form: ServerForm::default(),
            role_form: None,
            server_section: ServerSection::default(),
            role_tab: RoleTab::default(),
            role_error: None,
            description_editor: iced::widget::text_editor::Content::new(),
            main: MainConn::default(),
            server: ServerView::default(),
            messages: HashMap::new(),
            unread: HashSet::new(),
            read_marks: HashMap::new(),
            search: String::new(),
            search_more: false,
            editing: None,
            pending_delete: None,
            input: String::new(),
            new_channel: String::new(),
            create_open: false,
            category_open: false,
            rename_channel: None,
            dnd: None,
            pending_channel_delete: None,
            pending_role_delete: None,
            new_category: String::new(),
            members_hidden: false,
            collapsed_categories: HashSet::new(),
            menu: None,
            picker: None,
            picker_query: String::new(),
            pending_dm: None,
            dm_new_name: String::new(),
            dm_links: HashMap::new(),
            status: String::new(),
            drag: None,
            search_open: false,
            hover: None,
        }
    }

    pub fn pal(&self) -> theme::Pal {
        theme::palette(&self.prefs.theme)
    }

    pub fn save_prefs(&self) {
        self.prefs.save(self.backend.config_dir());
    }

    /// The settings saved with the open server, the remembered login and the panel widths.
    pub(crate) fn remember_session(&mut self) {
        self.prefs.session = match (&self.user, self.remember) {
            (Some(name), true) => Some(SessionPref {
                name: name.clone(),
                server: self.selected_server().map(|s| s.addr.clone()),
            }),
            _ => None,
        };
    }

    /// Clears the view of the open server, keeping the list of servers.
    pub(crate) fn reset_server_view(&mut self, addr: Option<String>) {
        let server = addr.as_ref().and_then(|a| self.prefs.servers.iter().find(|s| s.addr == *a)).cloned();
        let channel = addr
            .as_ref()
            .and_then(|a| self.prefs.last_channels.get(a).cloned())
            .unwrap_or_else(|| "general".into());
        self.server = ServerView {
            name: server.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
            icon: server.and_then(|s| s.icon),
            channel,
            user_id: self.user_id.clone(),
            ..ServerView::default()
        };
        self.messages.clear();
        self.unread.clear();
        self.read_marks.clear();
        self.search.clear();
        self.search_more = false;
        self.editing = None;
        self.pending_delete = None;
        self.input.clear();
        self.status.clear();
    }

    pub(crate) fn fresh_main(&self) -> MainConn {
        MainConn { seq: self.main.seq, ..MainConn::default() }
    }

    /// Switches to a channel by its id, unless it is already open.
    pub(crate) fn join_id(&self, id: i64) {
        if id != self.server.channel_id {
            send_main(self, ClientMsg::JoinChannel { id });
        }
    }

    pub(crate) fn join(&self, channel: &str) {
        let name = channel.trim().trim_start_matches('#');
        if name.is_empty() || name == self.server.channel {
            return;
        }
        send_main(self, ClientMsg::Join { channel: name.to_string() });
    }

    /// Opens the channel named by an in: filter, when it is a channel of the open server.
    pub(crate) fn search_channel(&self) -> Option<common::ChannelRef> {
        if !self.chat_ready() || self.current_person().is_some() || is_dm_channel(&self.server.channel) {
            return None;
        }
        Search::parse(&self.search)
            .channels
            .iter()
            .find_map(|name| self.server.channels.iter().find(|c| c.name.to_lowercase() == *name).cloned())
    }

    /// Connects to the server that is selected, in the background.
    pub(crate) fn connect_selected(&mut self) -> Task<Message> {
        let Some(addr) = self.selected_server().map(|s| s.addr.clone()) else {
            return Task::none();
        };
        if !self.is_logged_in() {
            return Task::none();
        }
        let seq = self.main.seq + 1;
        self.reset_server_view(Some(addr.clone()));
        self.main = MainConn { seq, addr: Some(addr.clone()), connecting: true, ..MainConn::default() };
        let backend = self.backend.clone();
        let addr2 = addr.clone();
        Task::perform(offload(move || backend.connect_main(&addr2)), move |r| Message::Connected(seq, r))
    }

    pub(crate) fn select_server(&mut self, index: usize) -> Task<Message> {
        self.home = false;
        self.page = Page::None;
        if index >= self.prefs.servers.len() {
            return Task::none();
        }
        if self.selected == Some(index) {
            // Still connected here, but a direct message may be open: go back to this server's channel.
            if self.main.id.is_some() && is_dm_channel(&self.server.channel) {
                // Go back to the last channel used here, if it still exists, else the first one.
                // Joining by name would create a channel that was deleted, so join by id.
                let last = self.main.addr.as_ref().and_then(|a| self.prefs.last_channels.get(a).cloned());
                let target = last
                    .and_then(|name| self.server.channels.iter().find(|c| c.name == name).map(|c| c.id))
                    .or_else(|| self.server.channels.first().map(|c| c.id));
                if let Some(id) = target {
                    self.join_id(id);
                }
            }
            return Task::none();
        }
        self.selected = Some(index);
        self.remember_session();
        self.save_prefs();
        self.connect_selected()
    }

    pub(crate) fn disconnect_main(&mut self) {
        self.backend.disconnect_main();
        self.main = self.fresh_main();
        self.main.seq += 1;
        self.reset_server_view(None);
    }

    pub(crate) fn remove_server(&mut self, index: usize) {
        let Some(srv) = self.prefs.servers.get(index).cloned() else {
            return;
        };
        if self.selected == Some(index) {
            self.disconnect_main();
            self.selected = None;
        } else if let Some(sel) = self.selected.filter(|s| *s > index) {
            self.selected = Some(sel - 1);
        }
        self.backend.dm_disconnect(&srv.addr);
        crate::trust::forget(&format!("server:{}", srv.addr));
        self.dm_links.remove(&srv.addr);
        self.prefs.servers.remove(index);
        self.save_prefs();
        self.dm_tick();
    }

    pub(crate) fn dm_tick(&self) {
        if !self.is_logged_in() {
            return;
        }
        self.backend.dm_connect_all(self.prefs.servers.iter().map(|s| s.addr.clone()).collect());
    }

    pub(crate) fn handle_main(&mut self, msg: ServerMsg) -> Task<Message> {
        match msg {
            ServerMsg::Welcome { server_id, name, user_id, server_name, server_description, server_icon, server_public } => {
                // The address must still be the same server. A different one here is refused.
                let known = self.selected_server().and_then(|s| s.id.clone());
                if known.as_deref().is_some_and(|k| !server_id.is_empty() && k != server_id) {
                    self.disconnect_main();
                    self.main.error = Some("This address now belongs to a different server. Remove it and add it again.".into());
                    return Task::none();
                }
                if let Some(server) = self.selected.and_then(|i| self.prefs.servers.get_mut(i)) {
                    if server_id != "" {
                        server.id = Some(server_id.clone());
                    }
                }
                self.server.server_id = server_id;
                self.main.welcomed = true;
                self.main.error = None;
                self.user_id = user_id.clone();
                self.user = Some(name.clone());
                self.server.user_id = user_id;
                self.server.public = server_public;
                self.server.description = server_description;
                self.server.icon = server_icon.clone();
                self.set_server_profile(server_name);
                // The rail draws the icon from the saved server list, so keep that copy in step too.
                if let Some(server) = self.selected.and_then(|i| self.prefs.servers.get_mut(i)) {
                    server.icon = server_icon;
                }
                self.save_prefs();
                self.status = format!("Connected as {name}");
                send_main(self, ClientMsg::ListChannels);
                // A direct message asked for before connecting is opened instead of the first channel.
                if let Some((addr, channel)) = self.pending_dm.take() {
                    if self.main.addr.as_deref() == Some(addr.as_str()) {
                        self.main.joined_initial = true;
                        send_main(self, ClientMsg::Join { channel });
                    }
                }
            }
            ServerMsg::Joined { channel, history } => {
                if is_dm_channel(&channel.name) {
                    self.list_direct_messages_everywhere();
                }
                let previous = self.server.channel_id;
                if channel.id != previous {
                    if let Some(list) = self.messages.get(&previous) {
                        self.read_marks.insert(previous, list.len());
                    }
                }
                self.unread.remove(&channel.id);
                self.messages.insert(channel.id, history.into_iter().map(Entry::Chat).collect());
                self.server.channel = channel.name.clone();
                self.server.channel_id = channel.id;
                if !is_dm_channel(&channel.name) {
                    if let Some(addr) = self.main.addr.clone() {
                        self.prefs.last_channels.insert(addr, channel.name);
                        self.save_prefs();
                    }
                }
                send_main(self, ClientMsg::ListChannels);
                return Task::batch([
                    operation::snap_to_end(MESSAGES_ID),
                    iced::widget::operation::focus(COMPOSER_ID),
                ]);
            }
            ServerMsg::Message { message } => {
                self.messages.entry(message.channel_id).or_default().push(Entry::Chat(message));
            }
            ServerMsg::Reactions { channel_id, id, reactions } => {
                self.set_reactions(channel_id, id, reactions);
            }
            ServerMsg::MessageDeleted { channel_id, id } => {
                if let Some(list) = self.messages.get_mut(&channel_id) {
                    list.retain(|e| !matches!(e, Entry::Chat(m) if m.id == id));
                }
                if self.editing.as_ref().is_some_and(|(e, _)| *e == id) {
                    self.editing = None;
                }
            }
            ServerMsg::MessageEdited { channel_id, id, text } => {
                if let Some(list) = self.messages.get_mut(&channel_id) {
                    for e in list.iter_mut() {
                        if let Entry::Chat(m) = e {
                            if m.id == id {
                                m.text = text.clone();
                                m.edited = true;
                            }
                        }
                    }
                }
            }
            ServerMsg::Activity { channel_id } => {
                // A channel not in the list is a direct message: refresh the list so it appears.
                if self.channel_name_of(channel_id).is_none() {
                    send_main(self, ClientMsg::ListDirectMessages);
                }
                if channel_id != self.server.channel_id {
                    self.unread.insert(channel_id);
                }
            }
            ServerMsg::ServerUpdated { name, description } => {
                self.server.description = description;
                self.set_server_profile(name);
            }
            ServerMsg::ServerIcon { png_base64 } => {
                self.server.icon = png_base64.clone();
                if let Some(server) = self.selected.and_then(|i| self.prefs.servers.get_mut(i)) {
                    server.icon = png_base64;
                }
                self.save_prefs();
            }
            ServerMsg::Channels { channels } => {
                self.server.channels = channels;
                // Open a channel that exists: the last one used here, else general, else the first.
                // Nothing is created: a channel is only made when it is asked for by name.
                if !self.main.joined_initial && self.chat_ready() {
                    self.main.joined_initial = true;
                    let last = self.main.addr.as_ref().and_then(|a| self.prefs.last_channels.get(a).cloned());
                    let pick = last
                        .filter(|l| self.server.channels.iter().any(|c| &c.name == l))
                        .or_else(|| self.server.channels.iter().find(|c| c.name == "general").map(|c| c.name.clone()))
                        .or_else(|| self.server.channels.first().map(|c| c.name.clone()));
                    if let Some(name) = pick {
                        send_main(self, ClientMsg::Join { channel: name });
                    }
                }
            }
            ServerMsg::Categories { categories, uncategorized } => {
                self.server.categories = categories;
                self.server.uncategorized = uncategorized;
            }
            ServerMsg::ChannelDeleted { id } => {
                let was_open = self.server.channel_id == id;
                self.server.channels.retain(|c| c.id != id);
                self.server.uncategorized.retain(|c| c.id != id);
                for cat in self.server.categories.iter_mut() {
                    cat.channels.retain(|c| c.id != id);
                }
                self.messages.remove(&id);
                self.unread.remove(&id);
                self.read_marks.remove(&id);
                if was_open {
                    // The channel is gone. Move to another channel that exists, if there is one.
                    self.server.channel_id = 0;
                    if let Some(next) = self.server.channels.first().map(|c| c.name.clone()) {
                        send_main(self, ClientMsg::Join { channel: next });
                    }
                }
            }
            ServerMsg::ChannelRenamed { id, name } => self.channel_renamed(id, &name),
            ServerMsg::DirectMessages { list } => self.server.dms = list,
            ServerMsg::Presence { users } => {
                self.server.member_ids = users.iter().map(|u| (u.name.clone(), u.id.clone())).collect();
                let owner = users.iter().find(|u| u.owner).cloned();
                self.server.online = users.iter().filter(|u| u.online).map(|u| u.name.clone()).collect();
                self.server.offline = users.iter().filter(|u| !u.online).map(|u| u.name.clone()).collect();
                self.server.member_roles = users.iter().map(|u| (u.id.clone(), u.roles.clone())).collect();
                self.server.owner = owner.as_ref().map(|o| o.name.clone());
                self.server.owner_id = owner.map(|o| o.id);
            }
            ServerMsg::Roles { roles } => self.server.roles = roles,
            ServerMsg::System { channel_id, text } => {
                self.messages
                    .entry(channel_id)
                    .or_default()
                    .push(Entry::System { text, at: crate::util::now() });
            }
            ServerMsg::Error { text } => {
                if self.page == Page::Server {
                    self.role_error = Some(text.clone());
                }
                if self.main.welcomed {
                    self.status = format!("Error: {text}");
                } else {
                    self.main.error = Some(text);
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn set_reactions(&mut self, channel_id: i64, id: i64, reactions: Vec<common::Reaction>) {
        if let Some(list) = self.messages.get_mut(&channel_id) {
            for e in list.iter_mut() {
                if let Entry::Chat(m) = e {
                    if m.id == id {
                        m.reactions = reactions.clone();
                    }
                }
            }
        }
    }

    /// Ends a press: a press that did not move opens the channel; a drag drops it on its target.
    pub(crate) fn finish_dnd(&mut self, dnd: Dnd) {
        if !dnd.moved {
            if let DragItem::Channel(id) = dnd.item {
                self.join_id(id);
            }
            return;
        }
        match (dnd.item, dnd.over) {
            (DragItem::Channel(id), Some(DropTarget::Category(category))) => {
                let _ = self.backend.send_main(ClientMsg::MoveChannel { channel: id, category: Some(category) });
            }
            (DragItem::Channel(id), Some(DropTarget::Uncategorized)) => {
                let _ = self.backend.send_main(ClientMsg::MoveChannel { channel: id, category: None });
            }
            (DragItem::Category(id), Some(DropTarget::Category(before))) if id != before => {
                let _ = self.backend.send_main(ClientMsg::MoveCategory { category: id, before: Some(before) });
            }
            (DragItem::Channel(id), Some(DropTarget::Channel { id: before, category })) if id != before => {
                let _ = self.backend.send_main(ClientMsg::PlaceChannel { channel: id, category, before: Some(before) });
            }
            (DragItem::Category(id), Some(DropTarget::End)) => {
                let _ = self.backend.send_main(ClientMsg::MoveCategory { category: id, before: None });
            }
            _ => {}
        }
    }

    /// Follows a rename. Channels keep their id, so only the name shown changes.
    pub(crate) fn channel_renamed(&mut self, id: i64, name: &str) {
        let old = self.channel_name_of(id);
        for c in self.server.channels.iter_mut().chain(self.server.uncategorized.iter_mut()).filter(|c| c.id == id) {
            c.name = name.to_string();
        }
        for cat in self.server.categories.iter_mut() {
            for c in cat.channels.iter_mut().filter(|c| c.id == id) {
                c.name = name.to_string();
            }
        }
        if self.server.channel_id == id {
            self.server.channel = name.to_string();
        }
        if let (Some(old), Some(addr)) = (old, self.main.addr.clone()) {
            if let Some(last) = self.prefs.last_channels.get_mut(&addr) {
                if *last == old {
                    *last = name.to_string();
                    self.save_prefs();
                }
            }
        }
    }

    pub(crate) fn set_server_profile(&mut self, name: String) {
        self.server.name = name.clone();
        if let Some(server) = self.selected.and_then(|i| self.prefs.servers.get_mut(i)) {
            server.name = name;
            self.save_prefs();
        }
    }

    pub(crate) fn handle_dm(&mut self, addr: &str, msg: ServerMsg) {
        let Some(link) = self.dm_links.get_mut(addr) else {
            return;
        };
        match msg {
            ServerMsg::Welcome { server_name, .. } => link.server_name = server_name,
            ServerMsg::DirectMessages { list } => link.dms = list,
            ServerMsg::DmHistory { channel, messages } => {
                link.history.insert(channel, messages);
            }
            ServerMsg::Activity { channel_id } => {
                link.unread.insert(channel_id);
                let addr = addr.to_string();
                send_dm(self, &addr, ClientMsg::ListDirectMessages);
            }
            _ => {}
        }
    }

    pub(crate) fn list_direct_messages_everywhere(&self) {
        for addr in self.dm_links.keys() {
            send_dm(self, addr, ClientMsg::ListDirectMessages);
        }
    }

    pub(crate) fn fetch_person(&self, other_id: &str) {
        for target in self.person_targets(other_id) {
            send_dm(self, &target.addr, ClientMsg::FetchDm { channel: target.channel });
        }
    }

    /// Sends a change to one message. Direct message ids go to their own server.
    pub(crate) fn message_action(&self, id: i64, msg: ClientMsg) {
        if id < VIRTUAL_BASE {
            send_main(self, with_id(msg, id));
            return;
        }
        let Some((target, real)) = self.virtual_target(id) else {
            return;
        };
        send_dm(self, &target.addr, with_id(msg, real));
        send_dm(self, &target.addr, ClientMsg::FetchDm { channel: target.channel });
    }

    pub(crate) fn open_person(&mut self, other_id: &str) {
        let targets = self.person_targets(other_id);
        let Some(newest) = targets.into_iter().max_by_key(|t| t.last) else {
            return;
        };
        self.home = true;
        let _ = self.open_dm_on(&newest.addr, &newest.channel);
    }

    pub(crate) fn open_dm_on(&mut self, addr: &str, channel: &str) -> Task<Message> {
        let dm = self.dm_links.get(addr).and_then(|l| l.dms.iter().find(|d| d.channel == channel)).cloned();
        if let Some(link) = self.dm_links.get_mut(addr) {
            if let Some(dm) = &dm {
                link.unread.remove(&dm.channel_id);
            }
        }
        let person = dm.map(|d| d.other_id);
        if let Some(person) = person {
            self.fetch_person(&person);
        }
        if self.main.id.is_some() && self.main.addr.as_deref() == Some(addr) {
            self.join(channel);
            return Task::none();
        }
        let Some(index) = self.prefs.servers.iter().position(|s| s.addr == addr) else {
            return Task::none();
        };
        self.pending_dm = Some((addr.to_string(), channel.to_string()));
        self.home = false;
        self.selected = Some(index);
        self.remember_session();
        self.save_prefs();
        self.connect_selected()
    }

    pub(crate) fn add_server_submit(&mut self) -> Task<Message> {
        let mut addr = self.add_server.addr.trim().to_string();
        if addr.is_empty() {
            addr = DEFAULT_ADDR.to_string();
        } else if !addr.contains(':') {
            addr = format!("{addr}:7878");
        }
        if self.prefs.servers.iter().any(|s| s.addr == addr) {
            self.add_server.error = Some("That server is already in your list.".into());
            return Task::none();
        }
        self.prefs.servers.push(ServerPref { name: addr.clone(), addr, icon: None, id: None });
        self.add_server = AddServer::default();
        self.save_prefs();
        let index = self.prefs.servers.len() - 1;
        let task = self.select_server(index);
        self.dm_tick();
        task
    }

    pub(crate) fn discover_add(&mut self, addr: &str) {
        let Some(found) = self.discover.found.iter().find(|f| f.addr == addr).cloned() else {
            return;
        };
        if self.prefs.servers.iter().any(|s| s.addr == addr) {
            return;
        }
        self.prefs.servers.push(ServerPref { name: found.name, addr: found.addr, icon: found.icon, id: Some(found.server_id) });
        self.save_prefs();
        self.dm_tick();
    }

    pub(crate) fn open_discover(&mut self) {
        self.page = Page::Discover;
        let via = self.selected_server().map(|s| s.addr.clone());
        self.discover = Discover {
            found: Vec::new(),
            error: None,
            expanded: HashSet::new(),
            searching: via.is_some(),
        };
        match via {
            Some(addr) => crate::discover::start(addr, self.backend.events_sender()),
            None => {
                self.discover.error = Some("Add a server first. Searching starts from a server you know.".into());
            }
        }
    }

    pub(crate) fn login_submit(&mut self) -> Task<Message> {
        let backend = self.backend.clone();
        let email = self.login.email.clone();
        let password = self.login.password.clone();
        let name = self.login.register.then(|| self.login.name.clone());
        let remember = self.remember;
        Task::perform(offload(move || backend.login(email, password, name, remember)), Message::LoginDone)
    }

    pub(crate) fn save_prefs_with_session(&mut self) {
        self.remember_session();
        self.save_prefs();
    }

    pub(crate) fn logout(&mut self) -> Task<Message> {
        self.dm_links.clear();
        self.main = self.fresh_main();
        self.reset_server_view(None);
        self.selected = None;
        self.home = false;
        self.page = Page::None;
        self.login.password.clear();
        self.account = Account::default();
        self.user = None;
        self.user_id.clear();
        self.remember_session();
        self.save_prefs();
        let backend = self.backend.clone();
        Task::perform(offload(move || backend.logout()), |_| Message::LoggedOut)
    }

    pub(crate) fn open_menu(&mut self, kind: MenuKind) {
        let at = self.cursor;
        let (_key, items, width) = match kind {
            MenuKind::Channel { id, name, unread } => {
                let mut items = Vec::new();
                if unread {
                    items.push(item("Mark as read", None, false, Message::MarkRead(id)));
                }
                if !is_dm_channel(&name) {
                    if self.can(perm::MANAGE_CHANNELS) {
                        items.push(item("Rename channel", None, false, Message::RenameStart(id, name.clone())));
                        items.push(item("Delete channel", Some(crate::icons::TRASH), true, Message::DeleteChannelAsk(id, name.clone())));
                    }
                    items.push(item("Copy channel name", None, false, Message::CopyText(format!("#{name}"))));
                }
                // The ID is always last, below a divider.
                items.push(MenuItem::Divider);
                items.push(item("Copy ID", None, false, Message::CopyText(id.to_string())));
                (format!("channel-{name}"), items, None)
            }
            MenuKind::Category { id, name } => {
                let mut items = Vec::new();
                if self.can(perm::MANAGE_CHANNELS) {
                    items.push(item("Delete category", Some(crate::icons::TRASH), true, Message::DeleteCategory(id)));
                    items.push(MenuItem::Divider);
                }
                items.push(item("Copy ID", None, false, Message::CopyText(id.to_string())));
                (format!("category-{name}"), items, None)
            }
            MenuKind::User { name, can_message } => {
                let mut items = vec![item("Mention", None, false, Message::Mention(name.clone()))];
                if can_message {
                    items.push(item("Message", None, false, Message::MessageUser(name.clone())));
                }
                items.push(item("Copy name", None, false, Message::CopyText(name.clone())));
                // The ID is always last, below a divider. Known from the member list, or the messages.
                let id = self.server.member_ids.get(&name).cloned().or_else(|| {
                    self.current_entries().into_iter().find_map(|e| match e {
                        Entry::Chat(m) if m.author == name => Some(m.author_id),
                        _ => None,
                    })
                });
                if let Some(id) = id {
                    // Roles this member can be given or have taken away by the viewer.
                    let member_is_owner = self.server.owner_id.as_deref() == Some(id.as_str());
                    // Only the owner may change the owner's roles.
                    let member_can_change = if member_is_owner {
                        self.is_owner()
                    } else {
                        self.top_role(&id).map_or(true, |top| self.outranks_role(top.id))
                    };
                    if self.can(perm::MANAGE_ROLES) && member_can_change {
                        let held = self.server.member_roles.get(&id).cloned().unwrap_or_default();
                        // @everyone is never given out; every member has it already.
                        let giveable: Vec<&common::RoleInfo> = self
                            .server
                            .roles
                            .iter()
                            .filter(|r| r.id != common::EVERYONE_ROLE && self.outranks_role(r.id))
                            .collect();
                        if !giveable.is_empty() {
                            items.push(MenuItem::Divider);
                            items.push(MenuItem::Heading("Roles".into()));
                            for role in giveable {
                                let has = held.contains(&role.id);
                                let label = if has { format!("Remove {}", role.name) } else { format!("Add {}", role.name) };
                                items.push(item(&label, None, false, Message::SetMemberRole(id.clone(), role.id, !has)));
                            }
                        }
                    }
                    items.push(MenuItem::Divider);
                    items.push(item("Copy ID", None, false, Message::CopyText(id)));
                }
                (format!("user-{name}"), items, None)
            }
            MenuKind::Rail(i) => {
                if self.selected == Some(i) && self.chat_ready() {
                    (String::from("rail"), self.server_menu_items(), None)
                } else {
                    (
                        format!("rail-{i}"),
                        vec![item("Remove server", None, true, Message::RemoveServer(i))],
                        None,
                    )
                }
            }
            MenuKind::Message(id) => {
                let mut items = Vec::new();
                if let Some(m) = self.find_message(id) {
                    if self.can_edit(&m) {
                        items.push(item("Edit Message", Some(crate::icons::PENCIL), false, Message::EditStart(id)));
                    }
                    if self.can_edit(&m) && self.can_delete(&m) {
                        items.push(MenuItem::Divider);
                    }
                    if self.can_delete(&m) {
                        items.push(item("Delete Message", Some(crate::icons::TRASH), true, Message::DeleteAsk(id)));
                    }
                }
                (format!("message-{id}"), items, None)
            }
            MenuKind::Server => (String::from("server"), self.server_menu_items(), Some(200.0)),
            MenuKind::Theme => (String::from("theme"), self.theme_menu_items(), Some(180.0)),
        };
        self.menu = Some(Menu { at, items, width });
        self.picker = None;
    }

    pub(crate) fn server_menu_items(&self) -> Vec<MenuItem> {
        let mut items = Vec::new();
        if self.is_owner() || self.can(perm::MANAGE_ROLES) {
            items.push(item("Server settings", None, false, Message::ServerSettingsOpen));
        }
        if self.can(perm::MANAGE_CHANNELS) {
            items.push(item("Create channel", None, false, Message::CreateOpen));
            items.push(item("Create category", None, false, Message::CategoryOpen));
        }
        items.push(MenuItem::Divider);
        let label = if self.members_hidden { "Show member list" } else { "Hide member list" };
        items.push(item(label, None, false, Message::ToggleMembers));
        items.push(item("Copy server address", None, false, Message::CopyText(self.main.addr.clone().unwrap_or_default())));
        items.push(MenuItem::Divider);
        items.push(item("Leave server", None, true, Message::RemoveServer(self.selected.unwrap_or(usize::MAX))));
        items
    }

    pub(crate) fn theme_menu_items(&self) -> Vec<MenuItem> {
        let entry = |(id, name, _): &(&str, &str, &str)| item(name, None, false, Message::ThemePick(id.to_string()));
        let mut items = vec![MenuItem::Heading("Dark".into())];
        items.extend(theme::THEMES.iter().filter(|t| t.2 == "dark").map(|t| entry(t)));
        items.push(MenuItem::Divider);
        items.push(MenuItem::Heading("Light".into()));
        items.extend(theme::THEMES.iter().filter(|t| t.2 == "light").map(|t| entry(t)));
        items
    }

    pub fn theme_id_name(&self, id: &str) -> String {
        theme::THEMES.iter().find(|t| t.0 == id).map(|t| t.1.to_string()).unwrap_or_else(|| id.to_string())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // The search filter menu stays open only while the search is being used.
        if !matches!(
            message,
            Message::SearchChanged(_)
                | Message::SearchOpen
                | Message::SearchFilter(_)
                | Message::SearchPick(_)
                | Message::SearchMore
                | Message::Event(_)
                | Message::Tick
                | Message::CursorMoved(_)
                | Message::MouseReleased
                | Message::Hover(_)
        ) {
            self.search_open = false;
        }
        match message {
            Message::Event(event) => self.on_event(event),
            Message::Tick => {
                self.dm_tick();
                Task::none()
            }
            Message::WindowOpened(id) => {
                self.window = Some(id);
                Task::none()
            }
            Message::CursorMoved(p) => {
                self.cursor = p;
                if let Some(dnd) = self.dnd.as_mut() {
                    if (p.x - dnd.start.x).abs() > 4.0 || (p.y - dnd.start.y).abs() > 4.0 {
                        dnd.moved = true;
                    }
                }
                if let Some(drag) = self.drag.as_mut() {
                    let dx = p.x - drag.start_x;
                    let delta = if drag.right { -dx } else { dx };
                    drag.width = (drag.start_width + delta).clamp(PANEL_MIN, PANEL_MAX);
                }
                Task::none()
            }
            Message::MouseReleased => {
                if let Some(drag) = self.drag.take() {
                    self.set_layout(drag.key, drag.width);
                    self.save_prefs();
                }
                if let Some(dnd) = self.dnd.take() {
                    self.finish_dnd(dnd);
                }
                Task::none()
            }
            Message::DragStart(key, right) => {
                let start_width = self.layout_width(key);
                self.drag = Some(Drag { key, right, start_x: self.cursor.x, start_width, width: start_width });
                Task::none()
            }
            Message::Escape => self.on_escape(),
            Message::WinMinimize => self.win(|id| window::minimize(id, true)),
            Message::WinMaxToggle => self.win(window::toggle_maximize),
            Message::WinClose => self.win(window::close),
            Message::WinDrag => self.win(window::drag),
            Message::Resize(direction) => self.win(|id| window::drag_resize(id, direction)),
            _ => self.update_rest(message),
        }
    }

    /// Runs a window action on the app's window, once the window is known.
    pub(crate) fn win(&self, action: impl Fn(window::Id) -> Task<Message>) -> Task<Message> {
        match self.window {
            Some(id) => action(id),
            None => Task::none(),
        }
    }

    /// The width a side panel is drawn at: the width being dragged, or the saved width.
    pub fn panel_width(&self, key: &str) -> f32 {
        match &self.drag {
            Some(drag) if drag.key == key => drag.width,
            _ => self.layout_width(key),
        }
    }

    pub(crate) fn layout_width(&self, key: &str) -> f32 {
        match key {
            "channels" => self.prefs.layout.channels,
            "dms" => self.prefs.layout.dms,
            _ => self.prefs.layout.members,
        }
    }

    pub(crate) fn set_layout(&mut self, key: &str, width: f32) {
        match key {
            "channels" => self.prefs.layout.channels = width,
            "dms" => self.prefs.layout.dms = width,
            _ => self.prefs.layout.members = width,
        }
    }

    pub(crate) fn on_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Net { dm: false, addr, id, msg } => {
                if self.main.id.is_none() && self.main.connecting {
                    // The connection's id is not known until the connect call returns; keep the messages.
                    self.main.buffer.push((id, msg));
                    return Task::none();
                }
                if self.main.id == Some(id) && self.main.addr.as_deref() == Some(addr.as_str()) {
                    return self.handle_main(msg);
                }
                Task::none()
            }
            Event::Net { dm: true, addr, id, msg } => {
                let link = self.dm_links.entry(addr.clone()).or_insert_with(|| DmLink { id, ..DmLink::default() });
                if link.id == id {
                    self.handle_dm(&addr, msg);
                }
                Task::none()
            }
            Event::Closed { dm: false, id, reason, .. } => {
                if self.main.id != Some(id) {
                    return Task::none();
                }
                let name = self.selected_name().unwrap_or_else(|| "the server".into());
                let was_welcomed = self.main.welcomed;
                let friendly = friendly_disconnect(&reason);
                self.main = self.fresh_main();
                let addr = self.selected_server().map(|s| s.addr.clone());
                self.reset_server_view(addr);
                if was_welcomed {
                    self.main.error = Some(format!("Lost the connection to {name}. {friendly}"));
                } else if self.main.error.is_none() {
                    self.main.error = Some(format!("Couldn't join {name}. {friendly}"));
                }
                Task::none()
            }
            Event::Closed { dm: true, addr, id, .. } => {
                if self.dm_links.get(&addr).is_some_and(|l| l.id == id) {
                    self.dm_links.remove(&addr);
                }
                Task::none()
            }
            Event::DmLink { addr, id, up } => {
                if up {
                    if self.dm_links.get(&addr).map(|l| l.id) != Some(id) {
                        self.dm_links.insert(addr, DmLink { id, ..DmLink::default() });
                    }
                } else if self.dm_links.get(&addr).is_some_and(|l| l.id == id) {
                    self.dm_links.remove(&addr);
                }
                Task::none()
            }
            Event::Discovered(found) => {
                if !self.discover.found.iter().any(|f| f.addr == found.addr) {
                    self.discover.found.push(found);
                }
                self.discover.searching = false;
                Task::none()
            }
        }
    }
}

/// One choice in the direct messages server picker: a conversation with the person on one server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmChoice {
    pub index: usize,
    pub label: String,
}

impl std::fmt::Display for DmChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// A menu item with its icon and action.
pub fn item(label: &str, icon: Option<&'static str>, danger: bool, msg: Message) -> MenuItem {
    MenuItem::Action { label: label.to_string(), icon, danger, msg }
}

/// The default address for a server typed without one.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7878";

/// A message for the open server's message action with the id filled in.
fn with_id(msg: ClientMsg, id: i64) -> ClientMsg {
    match msg {
        ClientMsg::Edit { text, .. } => ClientMsg::Edit { id, text },
        ClientMsg::Delete { .. } => ClientMsg::Delete { id },
        ClientMsg::React { emoji, .. } => ClientMsg::React { id, emoji },
        other => other,
    }
}

/// The message shown for a connection that ended, from the reason it ended with.
pub fn friendly_disconnect(reason: &str) -> String {
    let r = reason.to_lowercase();
    if r.contains("refused") {
        return "The server isn't running or can't be reached. Check the address and try again.".into();
    }
    if r.contains("timed out") || r.contains("timeout") || r.contains("did not answer") {
        return "The server stopped responding. Check your connection and try again.".into();
    }
    "Something interrupted the connection. Try again.".into()
}

impl App {
    /// The keys and the window events the app listens to.
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            Subscription::run(crate::events_stream).map(Message::Event),
            window::open_events().map(Message::WindowOpened),
            event::listen_with(|event, _status, _window| match event {
                iced::Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::CursorMoved(position)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(Message::MouseReleased),
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                }) => Some(Message::Escape),
                _ => None,
            }),
        ];
        if self.is_logged_in() {
            subs.push(iced::time::every(DM_TICK).map(|_| Message::Tick));
        }
        Subscription::batch(subs)
    }
}

