//! The messages for logging in, the pages and the chat actions. The core of the app is in `app.rs`.

use common::ClientMsg;
use iced::widget::operation;
use iced::Task;

use crate::app::{offload, App, AccountSaved, Field, Message, Page, Picker, ADD_ADDR_ID, COMPOSER_ID, EDIT_ID, NEW_CHANNEL_ID};

impl App {
    /// Handles every message that is not an event, a window action or a drag.
    pub fn update_rest(&mut self, message: Message) -> Task<Message> {
        match message {
            // ----- logging in and out -----
            Message::LoginName(v) => self.login.name = v,
            Message::LoginEmail(v) => self.login.email = v,
            Message::LoginPassword(v) => self.login.password = v,
            Message::LoginToggle => {
                self.login.register = !self.login.register;
                self.login.error = None;
            }
            Message::LoginSubmit => return self.login_submit(),
            Message::LoginDone(result) => match result {
                Ok(name) => {
                    self.user = Some(name);
                    self.auth_down = false;
                    self.login.password.clear();
                    self.login.name.clear();
                    self.login.error = None;
                    self.remember_session_now();
                    self.dm_tick();
                }
                Err(e) => {
                    self.login.error = Some(e.message);
                    self.auth_down = e.auth_down;
                }
            },
            Message::Remember(on) => {
                self.remember = on;
                self.remember_session_now();
            }
            Message::TrustAuth => {
                crate::trust::forget(&format!("auth:{}", crate::auth_client::auth_addr()));
                self.login.error = None;
            }
            Message::Restored(r) => {
                if let Some(user) = r.user {
                    self.user = Some(user);
                    self.auth_down = r.auth_down;
                    self.notice = r.notice;
                    self.remember = true;
                    self.dm_tick();
                    let saved = self.prefs.session.clone();
                    if let Some(index) = saved.and_then(|s| self.prefs.servers.iter().position(|p| Some(&p.addr) == s.server.as_ref())) {
                        return self.select_server(index);
                    }
                } else {
                    self.prefs.session = None;
                    self.login.error = r.error;
                    self.save_prefs();
                }
            }
            Message::AuthBannerClose => self.auth_banner_closed = true,
            Message::Logout => return self.logout(),
            Message::LoggedOut => {}

            // ----- servers -----
            Message::SelectServer(i) => return self.select_server(i),
            Message::RemoveServer(i) => {
                self.menu = None;
                self.remove_server(i);
            }
            Message::Connected(seq, result) => {
                if self.main.seq != seq {
                    return Task::none();
                }
                self.main.connecting = false;
                match result {
                    Ok(id) => {
                        self.main.id = Some(id);
                        let buffered: Vec<_> = self.main.buffer.drain(..).collect();
                        let mut tasks = Vec::new();
                        for (bid, msg) in buffered {
                            if bid == id {
                                tasks.push(self.handle_main(msg));
                            }
                        }
                        return Task::batch(tasks);
                    }
                    Err(e) => {
                        let name = self.selected_name().unwrap_or_default();
                        self.main.error = Some(format!("Could not connect to {name}: {}", e.message));
                    }
                }
            }
            Message::RetryServer => return self.connect_selected(),
            Message::TrustServer => {
                if let Some(addr) = self.main.addr.clone() {
                    crate::trust::forget(&format!("server:{addr}"));
                }
                return self.connect_selected();
            }
            Message::AddServerOpen => {
                self.add_server = crate::app::AddServer { open: true, addr: String::new(), error: None };
                return operation::focus(ADD_ADDR_ID);
            }
            Message::AddServerClose => self.add_server.open = false,
            Message::AddServerAddr(v) => self.add_server.addr = v,
            Message::AddServerSubmit => return self.add_server_submit(),
            Message::FromAddDiscover => {
                self.add_server.open = false;
                self.open_discover();
            }
            Message::DiscoverOpen => self.open_discover(),
            Message::DiscoverClose => self.page = Page::None,
            Message::DiscoverToggle(addr) => {
                if !self.discover.expanded.remove(&addr) {
                    self.discover.expanded.insert(addr);
                }
            }
            Message::DiscoverAdd(addr) => self.discover_add(&addr),

            // ----- direct messages -----
            Message::HomeOpen => {
                self.home = true;
                self.page = Page::None;
                self.list_direct_messages_everywhere();
            }
            Message::OpenPerson(id) => self.open_person(&id),
            Message::NewDmName(v) => self.dm_new_name = v,
            Message::NewDmSubmit => {
                let name = self.dm_new_name.trim().to_string();
                self.dm_new_name.clear();
                if name.is_empty() {
                    return Task::none();
                }
                if !self.chat_ready() {
                    self.status = "Error: connect to a server first, then message someone by name.".into();
                    return Task::none();
                }
                self.home = true;
                let _ = self.backend.send_main(ClientMsg::OpenDm { name });
            }
            Message::DmServerPick(choice) => {
                let targets = self.current_person().map(|p| self.person_targets(&p)).unwrap_or_default();
                if let Some(t) = targets.get(choice.index).cloned() {
                    let _ = self.open_dm_on(&t.addr, &t.channel);
                }
            }

            // ----- channels -----
            Message::DndPress(item) => {
                self.dnd = Some(crate::app::Dnd { item, start: self.cursor, moved: false, over: None });
            }
            Message::DndOver(target) => {
                if let Some(dnd) = self.dnd.as_mut() {
                    dnd.over = target;
                }
            }
            Message::DeleteChannelAsk(id, name) => {
                self.menu = None;
                self.pending_channel_delete = Some((id, name));
            }
            Message::DeleteChannelConfirm => {
                if let Some((id, _)) = self.pending_channel_delete.take() {
                    let _ = self.backend.send_main(ClientMsg::DeleteChannel { channel: id });
                }
            }
            Message::DeleteChannelCancel => self.pending_channel_delete = None,
            Message::CreateOpen => {
                self.create_open = true;
                self.menu = None;
                return operation::focus(NEW_CHANNEL_ID);
            }
            Message::NewChannel(v) => self.new_channel = v,
            Message::CategoryOpen => {
                self.category_open = true;
                self.menu = None;
                return operation::focus(crate::app::NEW_CATEGORY_ID);
            }
            Message::NewCategory(v) => self.new_category = v,
            Message::CreateCategory => {
                let name = std::mem::take(&mut self.new_category).trim().to_string();
                self.category_open = false;
                if !name.is_empty() {
                    let _ = self.backend.send_main(ClientMsg::CreateCategory { name });
                }
            }
            Message::DeleteCategory(category) => {
                self.menu = None;
                let _ = self.backend.send_main(ClientMsg::DeleteCategory { category });
            }
            Message::RenameStart(id, name) => {
                self.menu = None;
                self.rename_channel = Some((id, name.clone(), name));
                return operation::focus(crate::app::RENAME_ID);
            }
            Message::RenameInput(v) => {
                if let Some((_, _, text)) = self.rename_channel.as_mut() {
                    *text = v;
                }
            }
            Message::RenameSubmit => {
                if let Some((id, old, new)) = self.rename_channel.take() {
                    let new = new.trim().to_string();
                    if !new.is_empty() && new != old {
                        let _ = self.backend.send_main(ClientMsg::RenameChannel { channel: id, new_name: new });
                    }
                }
            }
            Message::RenameCancel => self.rename_channel = None,
            Message::CreateChannel => {
                let name = std::mem::take(&mut self.new_channel);
                self.create_open = false;
                self.join(&name);
            }
            Message::ToggleCategory(id) => {
                if !self.collapsed_categories.remove(&id) {
                    self.collapsed_categories.insert(id);
                }
            }
            Message::ToggleMembers => {
                self.members_hidden = !self.members_hidden;
                self.menu = None;
            }
            Message::MarkRead(id) => {
                self.unread.remove(&id);
                self.read_marks.remove(&id);
                self.menu = None;
            }
            Message::CopyText(text) => {
                self.menu = None;
                return iced::clipboard::write(text);
            }
            Message::SearchChanged(v) => {
                self.search = v;
                self.search_open = true;
                return self.search_channel_task();
            }
            Message::SearchFilter(prefix) => {
                self.search = if self.search.trim().is_empty() {
                    prefix.to_string()
                } else {
                    format!("{} {prefix}", self.search.trim_end())
                };
                return operation::focus(crate::app::SEARCH_ID);
            }
            Message::SearchOpen => self.search_open = true,
            Message::Hover(id) => self.hover = id,
            Message::SearchClear => {
                self.search.clear();
                self.search_open = false;
                return operation::focus(crate::app::SEARCH_ID);
            }
            Message::SearchPick(filter) => {
                // Replace the word being typed with the chosen filter.
                let keep = self.search.trim_end();
                let start = keep.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
                self.search = format!("{}{filter} ", &keep[..start]);
                // A channel picked with in: opens, as it does when typed.
                if let Some(channel) = self.search_channel() {
                    self.join_id(channel.id);
                }
                return operation::focus(crate::app::SEARCH_ID);
            }
            Message::SearchMore => {
                self.search_more = !self.search_more;
                return operation::focus(crate::app::SEARCH_ID);
            }
            Message::Mention(name) => {
                self.input.push_str(&format!("@{name} "));
                self.menu = None;
                return operation::focus(COMPOSER_ID);
            }
            Message::MessageUser(name) => {
                self.menu = None;
                self.home = true;
                let _ = self.backend.send_main(ClientMsg::OpenDm { name });
            }

            // ----- composer and messages -----
            Message::InputChanged(v) => self.input = v,
            Message::Send => {
                let text = self.input.trim().to_string();
                self.input.clear();
                if text.is_empty() {
                    return Task::none();
                }
                if let Some(channel) = text.strip_prefix("/join ") {
                    self.join(channel);
                } else {
                    let _ = self.backend.send_main(ClientMsg::Say { text });
                }
            }
            Message::React(id, emoji) => {
                self.menu = None;
                self.picker = None;
                self.message_action(id, ClientMsg::React { id, emoji: emoji.clone() });
            }
            Message::PickerOpen(id) => {
                if self.picker.as_ref().is_some_and(|p| p.message == id) {
                    self.picker = None;
                    return Task::none();
                }
                self.menu = None;
                let at = iced::Point::new((self.cursor.x - 280.0).max(4.0), self.cursor.y + 4.0);
                self.picker = Some(Picker { message: id, at });
                self.picker_query.clear();
            }
            Message::PickerClose => self.picker = None,
            Message::PickerQuery(v) => self.picker_query = v,
            Message::PickerTone(tone) => {
                self.prefs.skin_tone = tone;
                self.save_prefs();
            }
            Message::EditStart(id) => {
                self.menu = None;
                if let Some(m) = self.find_message(id) {
                    self.editing = Some((m.id, m.text.clone()));
                    return operation::focus(EDIT_ID);
                }
            }
            Message::EditChanged(v) => {
                if let Some((_, text)) = self.editing.as_mut() {
                    *text = v;
                }
            }
            Message::EditSave => {
                if let Some((id, text)) = self.editing.take() {
                    self.message_action(id, ClientMsg::Edit { id, text });
                }
            }
            Message::DeleteAsk(id) => {
                self.pending_delete = Some(id);
                self.menu = None;
            }
            Message::DeleteConfirm => {
                if let Some(id) = self.pending_delete.take() {
                    self.message_action(id, ClientMsg::Delete { id });
                }
            }
            Message::DeleteCancel => self.pending_delete = None,

            Message::OpenMenu(kind) => self.open_menu(kind),
            Message::CloseMenu => self.menu = None,

            // ----- settings pages -----
            Message::UserSettingsOpen => {
                self.page = Page::User;
                self.account.editing = None;
                return self.load_profile();
            }
            Message::UserSettingsClose => self.page = Page::None,
            Message::ProfileLoaded(result) => match result {
                Ok(p) => {
                    self.user = Some(p.name);
                    self.account.username = Some(p.username);
                    self.account.email = p.email;
                    self.account.email_error = None;
                }
                Err(e) => self.account.email_error = Some(e.message),
            },
            Message::AccountEdit(field) => {
                let acc = &mut self.account;
                acc.editing = Some(field);
                acc.form_error = None;
                let id = match field {
                    Field::Name => {
                        acc.name_input = acc.draft_name.clone().or_else(|| self.user.clone()).unwrap_or_default();
                        "acc-name"
                    }
                    Field::Username => {
                        acc.username_input = acc.draft_username.clone().or_else(|| acc.username.clone()).unwrap_or_default();
                        "acc-username"
                    }
                    Field::Email => {
                        acc.email_input = acc.draft_email.as_ref().map(|d| d.0.clone()).unwrap_or_default();
                        acc.email_password.clear();
                        "acc-email"
                    }
                };
                return operation::focus(id);
            }
            Message::AccountEditCancel => {
                self.account.editing = None;
                self.account.form_error = None;
            }
            Message::AccountNameInput(v) => self.account.name_input = v,
            Message::AccountUsernameInput(v) => self.account.username_input = v.to_lowercase(),
            Message::AccountEmailInput(v) => self.account.email_input = v,
            Message::AccountEmailPassword(v) => self.account.email_password = v,
            Message::AccountEditSubmit => {
                let acc = &mut self.account;
                match acc.editing {
                    Some(Field::Name) => acc.draft_name = Some(acc.name_input.trim().to_string()),
                    Some(Field::Username) => acc.draft_username = Some(acc.username_input.trim().to_lowercase()),
                    Some(Field::Email) => {
                        acc.draft_email = Some((acc.email_input.trim().to_string(), acc.email_password.clone()));
                    }
                    None => {}
                }
                acc.editing = None;
                acc.form_error = None;
            }
            Message::AccountReset => {
                let acc = &mut self.account;
                acc.draft_name = None;
                acc.draft_username = None;
                acc.draft_email = None;
                acc.draft_theme = None;
                acc.save_error = None;
            }
            Message::AccountSave => return self.save_account(),
            Message::AccountSaved(saved) => self.apply_account_saved(saved),
            Message::ThemePick(id) => {
                self.menu = None;
                self.account.draft_theme = if id == self.prefs.theme { None } else { Some(id) };
            }

            Message::ServerSettingsOpen => {
                self.menu = None;
                self.reset_server_form();
                self.page = Page::Server;
            }
            Message::ServerSettingsClose => self.page = Page::None,
            Message::ServerName(v) => self.server_form.name = v,
            Message::ServerDescriptionEdit(action) => {
                self.description_editor.perform(action);
                self.server_form.description = self.description_editor.text();
            }
            Message::ServerSave => {
                let name = self.server_form.name.trim().to_string();
                if !name.is_empty() {
                    let description = self.server_form.description.trim().to_string();
                    let _ = self.backend.send_main(ClientMsg::UpdateServer { name, description });
                }
            }
            Message::ServerReset => self.reset_server_form(),
            Message::IconPick => {
                self.server_form.icon_error = None;
                return Task::perform(offload(crate::icons::pick_server_icon), Message::IconPicked);
            }
            Message::IconPicked(result) => match result {
                Ok(Some(png)) => {
                    let _ = self.backend.send_main(ClientMsg::SetServerIcon { png_base64: Some(png) });
                }
                Ok(None) => {}
                Err(e) => self.server_form.icon_error = Some(e.message),
            },
            Message::IconRemove => {
                self.server_form.icon_error = None;
                let _ = self.backend.send_main(ClientMsg::SetServerIcon { png_base64: None });
            }
            _ => {}
        }
        Task::none()
    }

    /// Fills the server settings form from the open server, including the description editor.
    fn reset_server_form(&mut self) {
        self.server_form = crate::app::ServerForm {
            name: self.selected_name().unwrap_or_default(),
            description: self.server.description.clone(),
            icon_error: None,
        };
        self.description_editor = iced::widget::text_editor::Content::with_text(&self.server.description);
    }

    /// Keeps the session of a remembered login up to date on disk.
    pub(crate) fn remember_session_now(&mut self) {
        self.save_prefs_with_session();
    }

    /// Escape closes the topmost thing that is open.
    pub fn on_escape(&mut self) -> Task<Message> {
        if self.search_open {
            self.search_open = false;
        } else if self.menu.is_some() || self.picker.is_some() {
            self.menu = None;
            self.picker = None;
        } else if self.editing.is_some() {
            self.editing = None;
        } else if self.pending_delete.is_some() {
            self.pending_delete = None;
        } else if self.pending_channel_delete.is_some() {
            self.pending_channel_delete = None;
        } else if self.rename_channel.is_some() {
            self.rename_channel = None;
        } else if self.category_open {
            self.category_open = false;
        } else if self.add_server.open {
            self.add_server.open = false;
        } else if self.account.editing.is_some() {
            self.account.editing = None;
        } else if matches!(self.page, Page::Discover | Page::User | Page::Server) {
            self.page = Page::None;
        }
        Task::none()
    }

    fn load_profile(&self) -> Task<Message> {
        let backend = self.backend.clone();
        Task::perform(offload(move || backend.profile()), Message::ProfileLoaded)
    }

    /// Saves the account drafts one by one: name, username, email, then theme. The theme is kept on
    /// this computer, so it is applied straight away.
    fn save_account(&mut self) -> Task<Message> {
        let name = self.account.draft_name.clone();
        let username = self.account.draft_username.clone();
        let email = self.account.draft_email.clone();
        let current_name = self.user.clone().unwrap_or_default();
        let current_username = self.account.username.clone().unwrap_or_default();
        if let Some(theme) = self.account.draft_theme.clone() {
            self.prefs.theme = theme;
            self.account.draft_theme = None;
            self.save_prefs();
        }
        self.account.save_error = None;
        let backend = self.backend.clone();
        Task::perform(
            offload(move || {
                let mut out = AccountSaved { profile: None, email: None, name_saved: false, username_saved: false, email_saved: false, error: None };
                let name_change = name.filter(|n| *n != current_name);
                let username_change = username.filter(|u| *u != current_username);
                if name_change.is_some() || username_change.is_some() {
                    let result = backend.update_profile(name_change.clone(), username_change.clone());
                    match result {
                        Ok(p) => {
                            out.profile = Some(p);
                            out.name_saved = true;
                            out.username_saved = true;
                        }
                        Err(e) => {
                            out.error = Some(e.message);
                            return out;
                        }
                    }
                } else {
                    out.name_saved = true;
                    out.username_saved = true;
                }
                if let Some((new_email, password)) = email {
                    match backend.change_email(password, new_email) {
                        Ok(addr) => {
                            out.email = Some(addr);
                            out.email_saved = true;
                        }
                        Err(e) => {
                            out.error = Some(e.message);
                            return out;
                        }
                    }
                } else {
                    out.email_saved = true;
                }
                out
            }),
            Message::AccountSaved,
        )
    }

    fn apply_account_saved(&mut self, saved: AccountSaved) {
        if let Some(p) = saved.profile {
            self.user = Some(p.name);
            self.account.username = Some(p.username);
            self.save_prefs_with_session();
        }
        if saved.name_saved {
            self.account.draft_name = None;
        }
        if saved.username_saved {
            self.account.draft_username = None;
        }
        if saved.email_saved {
            self.account.draft_email = None;
            self.account.email_password.clear();
        }
        if let Some(addr) = saved.email {
            self.account.email = Some(addr);
        }
        self.account.save_error = saved.error;
    }

    /// Starts a search for a channel named in an in: filter, when the search asks for one.
    fn search_channel_task(&mut self) -> Task<Message> {
        match self.search_channel() {
            Some(channel) => {
                self.join_id(channel.id);
                Task::none()
            }
            None => Task::none(),
        }
    }
}

