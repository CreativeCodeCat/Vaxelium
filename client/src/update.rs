//! The messages for logging in, the pages and the chat actions. The core of the app is in `app.rs`.

use common::ClientMsg;
use iced::widget::operation;
use iced::window;
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
            Message::AuthProbe => return Task::perform(offload(crate::auth_client::reachable), Message::AuthProbed),
            Message::AuthProbed(up) => {
                if up {
                    // The service is back: hide the banner, and let a later outage show it again.
                    self.auth_down = false;
                    self.auth_banner_closed = false;
                }
            }
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
                // Only members who may manage channels get drop targets, so nothing lights up for them.
                let allowed = self.can(common::perm::MANAGE_CHANNELS);
                if let Some(dnd) = self.dnd.as_mut() {
                    dnd.over = if allowed { target } else { None };
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
            Message::ChannelEditOpen(id) => {
                self.menu = None;
                self.access_draft = None;
                self.access_subject = common::Subject::Role(common::EVERYONE_ROLE);
                let name = self.channel_name_of(id).unwrap_or_default();
                self.rename_channel = Some((id, name.clone(), name));
                self.channel_edit = Some(id);
                self.edit_section = crate::app::EditSection::Overview;
            }
            Message::ChannelEditClose => return self.close_channel_edit(),
            Message::EditSectionPick(section) => self.edit_section = section,
            Message::AccessDraftSet { target, subject, view, send } => {
                if self.access_draft.as_ref().map_or(true, |d| d.target != target) {
                    self.access_draft = Some(crate::app::AccessDraft { target, subjects: Default::default() });
                }
                if let Some(draft) = self.access_draft.as_mut() {
                    draft.subjects.insert(subject, (view, send));
                }
            }
            Message::AccessAddOpen(target) => self.access_add = Some(crate::app::AccessAdd { target, query: String::new(), picked: Default::default() }),
            Message::AccessAddClose => self.access_add = None,
            Message::AccessAddQuery(query) => {
                if let Some(add) = self.access_add.as_mut() {
                    add.query = query;
                }
            }
            Message::AccessAddToggle(subject, on) => {
                if let Some(add) = self.access_add.as_mut() {
                    if on {
                        add.picked.insert(subject);
                    } else {
                        add.picked.remove(&subject);
                    }
                }
            }
            Message::AccessAddDone => {
                // Added to whichever channel or category the dialog was opened for.
                if let Some(add) = self.access_add.take() {
                    let target = add.target;
                    for subject in add.picked {
                        let (_, send) = self.access_value(target, subject.clone());
                        let _ = self.update(Message::AccessDraftSet { target, subject, view: Some(true), send });
                    }
                }
            }
            Message::ChannelSyncWithCategory(channel) => {
                if let Some((category, _)) = self.category_of_channel(channel) {
                    let target = crate::app::AccessTarget::Channel(channel);
                    let subjects = self
                        .subjects_of(channel, category)
                        .into_iter()
                        .map(|subject| {
                            let value = self.server_access(crate::app::AccessTarget::Category(category), &subject);
                            (subject, value)
                        })
                        .collect();
                    self.access_draft = Some(crate::app::AccessDraft { target, subjects });
                }
            }
            Message::EditPageSave => {
                if self.channel_edit.is_some() {
                    let _ = self.update(Message::RenameSubmit);
                }
                if self.category_edit.is_some() {
                    let _ = self.update(Message::CategoryEditSubmit);
                }
                self.save_access();
            }
            Message::EditPageReset => {
                if self.channel_edit.is_some() {
                    let _ = self.update(Message::RenameReset);
                }
                if self.category_edit.is_some() {
                    let _ = self.update(Message::CategoryEditReset);
                }
                self.access_draft = None;
            }
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
            Message::DeleteCategoryAsk(id, name) => {
                self.menu = None;
                self.pending_category_delete = Some((id, name));
            }
            Message::DeleteCategoryConfirm => {
                if let Some((category, _)) = self.pending_category_delete.take() {
                    let _ = self.backend.send_main(ClientMsg::DeleteCategory { category });
                }
            }
            Message::DeleteCategoryCancel => self.pending_category_delete = None,
            Message::RenameInput(v) => {
                if let Some((_, _, text)) = self.rename_channel.as_mut() {
                    *text = v;
                }
            }
            Message::RenameReset => {
                if let Some((_, old, new)) = self.rename_channel.as_mut() {
                    *new = old.clone();
                }
            }
            Message::RenameSubmit => {
                if let Some((id, old, new)) = self.rename_channel.clone() {
                    let new = new.trim().to_string();
                    if !new.is_empty() && new != old {
                        let _ = self.backend.send_main(ClientMsg::RenameChannel { channel: id, new_name: new });
                    }
                }
            }
            Message::CategoryEditOpen(id, name) => {
                self.menu = None;
                self.category_edit = Some((id, name.clone(), name));
                self.edit_section = crate::app::EditSection::Overview;
                self.access_subject = common::Subject::Role(common::EVERYONE_ROLE);
                self.access_draft = None;
                return operation::focus(crate::app::CATEGORY_EDIT_ID);
            }
            Message::CategoryEditInput(v) => {
                if let Some((_, _, text)) = self.category_edit.as_mut() {
                    *text = v;
                }
            }
            Message::CategoryEditSubmit => {
                if let Some((id, old, new)) = self.category_edit.clone() {
                    let new = new.trim().to_string();
                    if !new.is_empty() && new != old {
                        let _ = self.backend.send_main(ClientMsg::RenameCategory { category: id, new_name: new });
                    }
                }
            }
            Message::CategoryEditReset => {
                if let Some((_, old, new)) = self.category_edit.as_mut() {
                    *new = old.clone();
                }
            }
            Message::CategoryEditClose => return self.close_category_edit(),
            Message::AccessSubjectPick(subject) => self.access_subject = subject,
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
            Message::MembersView(show) => {
                self.members_view = show;
                self.menu = None;
            }
            Message::MarkRead(id) => {
                self.unread.remove(&id);
                self.pinged.remove(&id);
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
                self.card = None;
                let blocked = self.server.member_ids.get(&name).is_some_and(|id| self.is_blocked(id));
                if blocked {
                    self.status = format!("You have blocked {name}. Unblock them from the member list to message them.");
                    return Task::none();
                }
                self.home = true;
                // An existing conversation opens straight away, so it loads without a click. Its messages
                // are then sent as soon as it has loaded.
                let existing = self.server.member_ids.get(&name).cloned().filter(|id| !self.person_targets(id).is_empty());
                match existing {
                    Some(id) => self.open_person(&id),
                    None => {
                        let _ = self.backend.send_main(ClientMsg::OpenDm { name });
                    }
                }
            }
            Message::IgnoreToggle(id) => {
                self.menu = None;
                if self.is_protected(&id) && !self.selected_server().is_some_and(|s| s.ignored.contains(&id)) {
                    self.status = "This member cannot be ignored.".into();
                    return Task::none();
                }
                if let Some(server) = self.selected.and_then(|i| self.prefs.servers.get_mut(i)) {
                    if let Some(pos) = server.ignored.iter().position(|i| *i == id) {
                        server.ignored.remove(pos);
                    } else {
                        server.ignored.push(id);
                    }
                }
                self.save_prefs();
            }
            Message::BlockToggle(id) => {
                self.menu = None;
                if self.is_protected(&id) && !self.prefs.blocked.contains(&id) {
                    self.status = "This member cannot be blocked.".into();
                    return Task::none();
                }
                if let Some(pos) = self.prefs.blocked.iter().position(|b| *b == id) {
                    self.prefs.blocked.remove(pos);
                } else {
                    self.prefs.blocked.push(id);
                }
                self.save_prefs();
            }

            // ----- composer and messages -----
            Message::InputChanged(v) => {
                let typing = !v.trim().is_empty() && self.server.channel_id != 0;
                self.input = v;
                let due = self.last_typing_sent.is_none_or(|at| at.elapsed() >= crate::app::TYPING_SEND_EVERY);
                if typing && due {
                    self.last_typing_sent = Some(std::time::Instant::now());
                    let _ = self.backend.send_main(ClientMsg::Typing);
                }
            }
            Message::SetStatus(status) => {
                self.menu = None;
                // Online goes back to following input. Any other choice is kept by hand.
                self.account_status = status;
                self.auto_status = status == common::PresenceStatus::Online;
                self.auto_idle = false;
                self.backend.set_status_everywhere(self.effective_status());
                let _ = self.backend.send_main(ClientMsg::SetStatus { status });
            }
            Message::MentionPick(name) => {
                // Replace the @ word being typed with the picked name, and leave a space after it.
                let start = self.input.char_indices().rev().find(|(_, c)| c.is_whitespace()).map_or(0, |(i, c)| i + c.len_utf8());
                self.input.truncate(start);
                self.input.push('@');
                self.input.push_str(&name);
                self.input.push(' ');
                return operation::focus(COMPOSER_ID);
            }
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
                self.card = None;
                self.page = Page::User;
                self.account.editing = None;
                return self.load_profile();
            }
            Message::UserSettingsClose => return self.close_settings(),
            Message::ProfileLoaded(result) => match result {
                Ok(p) => {
                    // Keep the picture and banner the member list and messages use in step with the account.
                    let created = self.profiles.get(&self.user_id).map_or(0, |look| look.created);
                    let status = self.profiles.get(&self.user_id).map(|look| look.status.clone()).unwrap_or_default();
                    self.profiles.insert(self.user_id.clone(), crate::app::ProfileLook { avatar: p.avatar.clone(), banner: p.banner.clone(), created, status });
                    self.user = Some(p.name);
                    self.account.username = Some(p.username);
                    self.account.email = p.email;
                    self.account.verified = p.verified;
                    self.account.can_verify = p.can_verify;
                    self.account.bio = p.bio;
                    self.account.banner = p.banner;
                    self.account.avatar = p.avatar;
                    self.account.email_error = None;
                    self.auth_down = false;
                    self.auth_banner_closed = false;
                }
                Err(e) => {
                    self.account.email_error = Some(e.message);
                    if e.auth_down {
                        self.auth_down = true;
                    }
                }
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
            Message::VerifyOpen => {
                self.account.verify_busy = true;
                self.account.verify_error = None;
                let backend = self.backend.clone();
                return Task::perform(offload(move || backend.request_email_code()), Message::CodeRequested);
            }
            Message::CodeRequested(result) => {
                self.account.verify_busy = false;
                self.account.verify_open = true;
                self.account.verify_code.clear();
                if let Err(e) = &result {
                    if e.auth_down {
                        self.auth_down = true;
                    }
                }
                self.account.verify_error = result.err().map(|e| e.message);
            }
            Message::VerifyCodeInput(code) => self.account.verify_code = code,
            Message::VerifySubmit => {
                if self.account.verify_busy {
                    return Task::none();
                }
                self.account.verify_busy = true;
                self.account.verify_error = None;
                let code = self.account.verify_code.trim().to_string();
                let backend = self.backend.clone();
                return Task::perform(offload(move || backend.verify_email(code)), Message::VerifyDone);
            }
            Message::VerifyDone(result) => {
                self.account.verify_busy = false;
                match result {
                    Ok(()) => {
                        self.account.verified = true;
                        self.account.verify_open = false;
                        self.account.verify_code.clear();
                        self.account.verify_error = None;
                    }
                    Err(e) => {
                        self.account.verify_error = Some(e.message);
                        if e.auth_down {
                            self.auth_down = true;
                        }
                    }
                }
            }
            Message::CardStatusStart => {
                let current = self.card.as_ref().and_then(|v| v.card.as_ref()).map(|c| c.status.clone()).unwrap_or_default();
                self.card_status_draft = Some(current);
                self.card_status_error = None;
            }
            Message::CardStatusInput(text) => {
                self.card_status_draft = Some(text.chars().take(common::MAX_STATUS_TEXT_LEN).collect());
            }
            Message::CardStatusCancel => {
                self.card_status_draft = None;
                self.card_status_error = None;
            }
            Message::CardStatusSave => {
                let text = self.card_status_draft.clone().unwrap_or_default().trim().to_string();
                self.card_status_error = None;
                let backend = self.backend.clone();
                return Task::perform(offload(move || backend.set_status_text(text)), Message::CardStatusDone);
            }
            Message::CardStatusClear => {
                self.card_status_draft = Some(String::new());
                self.card_status_error = None;
                let backend = self.backend.clone();
                return Task::perform(offload(move || backend.set_status_text(String::new())), Message::CardStatusDone);
            }
            Message::CardStatusDone(result) => match result {
                Ok(text) => {
                    self.backend.set_profile_status_everywhere(&text);
                    self.card_status_draft = None;
                    if let Some(look) = self.profiles.get_mut(&self.user_id) {
                        look.status = text.clone();
                    }
                    if let Some(card) = self.card.as_mut().and_then(|v| v.card.as_mut()) {
                        card.status = text;
                    }
                }
                Err(e) => self.card_status_error = Some(e.message),
            },
            Message::CardOpen { id, name } => return self.open_card(id, name, self.cursor, false),
            Message::CardMessageInput(text) => self.card_message = text,
            Message::CardRolesToggle => {
                // Opens at the + that was clicked. Only one of the roles and badges lists is open at a time.
                self.card_pick_at = self.cursor;
                self.card_badges_open = false;
                self.card_roles_open = !self.card_roles_open;
            }
            Message::CardBadgesToggle => {
                self.card_pick_at = self.cursor;
                self.card_roles_open = false;
                self.card_badges_open = !self.card_badges_open;
            }
            Message::CardListSearch(text) => self.card_list_search = text,
            Message::BadgeName(name) => self.badge_name = name,
            Message::BadgeColor(color) => self.badge_color = color,
            Message::BadgeCreate => {
                let name = self.badge_name.trim().to_string();
                if !name.is_empty() {
                    let color = self.badge_color.clone();
                    let _ = self.backend.send_main(ClientMsg::CreateBadge { name, color });
                    self.badge_name.clear();
                }
            }
            Message::BadgeDelete(badge) => {
                let _ = self.backend.send_main(ClientMsg::DeleteBadge { badge });
            }
            Message::SetMemberBadge { user, badge, on } => {
                let _ = self.backend.send_main(ClientMsg::SetMemberBadge { user, badge, on });
            }
            Message::CardMessageSend => {
                let text = std::mem::take(&mut self.card_message).trim().to_string();
                let name = self.card.as_ref().map(|v| v.name.clone()).unwrap_or_default();
                if text.is_empty() || name.is_empty() {
                    return Task::none();
                }
                self.pending_card_send = Some(text);
                return Task::done(Message::MessageUser(name));
            }
            Message::CardOpenAbove { id, name } => {
                // The top edge of the user bar, at the cursor's column, so the card sits just above it.
                let at = iced::Point::new(self.cursor.x, self.window_size.height - crate::view::USER_BAR_HEIGHT);
                return self.open_card(id, name, at, true);
            }
            Message::CardLoaded(id, result) => {
                if let Ok(card) = &result {
                    self.profiles.insert(id.clone(), crate::app::ProfileLook { avatar: card.avatar.clone(), banner: card.banner.clone(), created: card.created, status: card.status.clone() });
                }
                if let Some(view) = self.card.as_mut().filter(|v| v.id == id) {
                    view.loading = false;
                    match result {
                        Ok(card) => view.card = Some(card),
                        Err(e) => view.error = Some(e.message),
                    }
                }
            }
            Message::CardClose => {
                self.card = None;
                self.card_status_draft = None;
                self.card_status_error = None;
            }
            Message::OpenLink(url) => {
                if url.starts_with("https://") || url.starts_with("http://") {
                    crate::util::open_url(&url);
                }
            }
            Message::BioEdit(action) => {
                self.bio_editor.perform(action);
                // The bio is at most 190 characters, counting line breaks.
                let mut text = self.bio_editor.text();
                if text.chars().count() > crate::app::BIO_MAX {
                    text = text.chars().take(crate::app::BIO_MAX).collect();
                    self.bio_editor = iced::widget::text_editor::Content::with_text(&text);
                }
                self.account.about_bio = text;
            }
            Message::AboutBanner(hex) => self.account.about_banner = hex,
            Message::ProfilesLoaded(found) => {
                for (id, card) in found {
                    self.profile_pending.remove(&id);
                    if let Some(card) = card {
                        self.profiles.insert(id, crate::app::ProfileLook { avatar: card.avatar, banner: card.banner, created: card.created, status: card.status });
                    }
                }
            }
            Message::AvatarPick => {
                self.account.avatar_error = None;
                return Task::perform(offload(crate::icons::pick_avatar), Message::AvatarPicked);
            }
            // A picked or removed picture is a draft until Save; Reset drops it.
            Message::AvatarPicked(result) => match result {
                Ok(Some(png)) => {
                    self.account.avatar_draft = Some(png);
                    self.account.avatar_error = None;
                }
                Ok(None) => {}
                Err(e) => self.account.avatar_error = Some(e.message),
            },
            Message::AvatarRemove => {
                self.account.avatar_draft = Some(String::new());
                self.account.avatar_error = None;
            }
            Message::AvatarDone(result) => match result {
                Ok(avatar) => {
                    let created = self.profiles.get(&self.user_id).map(|p| p.created).unwrap_or(0);
                    let status = self.profiles.get(&self.user_id).map(|look| look.status.clone()).unwrap_or_default();
                    self.profiles.insert(self.user_id.clone(), crate::app::ProfileLook { avatar: avatar.clone(), banner: self.account.banner.clone(), created, status });
                    self.account.avatar = avatar;
                    self.account.avatar_draft = None;
                    self.account.avatar_error = None;
                }
                Err(e) => self.account.avatar_error = Some(e.message),
            },
            Message::AboutReset => {
                self.account.about_bio = self.account.bio.clone();
                self.reset_bio_editor();
                self.account.about_banner = self.account.banner.clone();
                self.account.avatar_draft = None;
                self.account.about_error = None;
                self.account.avatar_error = None;
            }
            Message::AboutSave => {
                if self.account.about_busy {
                    return Task::none();
                }
                self.account.about_busy = true;
                self.account.about_error = None;
                let (bio, banner) = (self.account.about_bio.trim().to_string(), self.account.about_banner.clone());
                let backend = self.backend.clone();
                let about = Task::perform(offload(move || backend.set_about(bio, banner)), Message::AboutDone);
                // A picture change is saved with the rest.
                return match self.account.avatar_draft.clone() {
                    Some(draft) => Task::batch([about, self.save_avatar((!draft.is_empty()).then_some(draft))]),
                    None => about,
                };
            }
            Message::AboutDone(result) => {
                self.account.about_busy = false;
                match result {
                    Ok((bio, banner)) => {
                        if let Some(look) = self.profiles.get_mut(&self.user_id) {
                            look.banner = banner.clone();
                        }
                        self.account.bio = bio;
                        self.account.banner = banner;
                    }
                    Err(e) => self.account.about_error = Some(e.message),
                }
            }
            Message::VerifyCancel => {
                self.account.verify_open = false;
                self.account.verify_code.clear();
                self.account.verify_error = None;
            }
            Message::ThemePick(id) => {
                self.menu = None;
                self.account.draft_theme = if id == self.prefs.theme { None } else { Some(id) };
            }

            Message::ServerSettingsOpen => {
                self.menu = None;
                self.reset_server_form();
                self.role_form = None;
                self.role_error = None;
                // The owner starts on the profile; everyone else who can open the page starts on roles.
                self.server_section = if self.can(common::perm::MANAGE_SERVER) { crate::app::ServerSection::Profile } else { crate::app::ServerSection::Roles };
                self.page = Page::Server;
            }
            Message::ServerSettingsClose => return self.close_settings(),
            Message::TrayToggle => {
                // Hiding closes the window and keeps the app running in the tray; showing opens it
                // again with the same state.
                return match self.window {
                    Some(_) => self.hide_window(),
                    None => {
                        let (_, open) = window::open(crate::main_window_settings());
                        Task::batch([open.map(Message::WindowOpened), Task::done(Message::TrayVisible(true))])
                    }
                };
            }
            Message::WindowResized(size) => self.window_size = size,
            Message::TrayVisible(shown) => crate::tray::set_shown(shown),
            Message::TrayQuit => return iced::exit(),
            Message::FlashTick => {
                if !self.unsaved_flash_on() {
                    self.unsaved_flash = None;
                }
            }
            Message::UserSectionPick(section) => {
                self.user_section = section;
                // The Profile tab is always an editor, so it starts from what is saved.
                if section == crate::app::UserSection::Profile {
                    self.account.about_bio = self.account.bio.clone();
                    self.reset_bio_editor();
                    self.account.about_banner = self.account.banner.clone();
                    self.account.about_error = None;
                }
            }
            Message::ServerSectionPick(section) => {
                self.role_form = None;
                self.role_error = None;
                self.server_section = section;
            }
            Message::RoleTab(tab) => self.role_tab = tab,
            Message::RoleColorPick(hex) => {
                if let Some(draft) = self.role_form.as_mut() {
                    draft.color = hex;
                }
            }
            Message::RoleEdit(id) => {
                self.role_error = None;
                self.role_tab = crate::app::RoleTab::Display;
                self.role_form = Some(match id.and_then(|id| self.server.roles.iter().find(|r| r.id == id)) {
                    Some(role) => crate::app::RoleDraft {
                        id: Some(role.id),
                        name: role.name.clone(),
                        color: role.color.clone(),
                        permissions: role.permissions,
                        mentionable: role.mentionable,
                        hoisted: role.hoisted,
                    },
                    None => crate::app::RoleDraft::default(),
                });
            }
            Message::RoleEditCancel => self.role_form = None,
            Message::RoleDraftName(v) => {
                if let Some(draft) = self.role_form.as_mut() {
                    draft.name = v;
                }
            }
            Message::RoleDraftColor(v) => {
                if let Some(draft) = self.role_form.as_mut() {
                    draft.color = v;
                }
            }
            Message::RoleDraftMentionable(on) => {
                if let Some(draft) = self.role_form.as_mut() {
                    draft.mentionable = on;
                }
            }
            Message::RoleDraftHoisted(on) => {
                if let Some(draft) = self.role_form.as_mut() {
                    draft.hoisted = on;
                }
            }
            Message::RoleDraftPerm(bit, on) => {
                // Turning Administrator on asks first, through the dialog.
                if bit == common::perm::ADMINISTRATOR && on {
                    self.admin_grant_pending = true;
                    return Task::none();
                }
                let locked = self.role_form.as_ref().is_some_and(|d| crate::view::pages::everyone_keeps(self, d.id, bit));
                if let Some(draft) = self.role_form.as_mut().filter(|_| !locked) {
                    if on {
                        draft.permissions |= bit;
                    } else {
                        draft.permissions &= !bit;
                    }
                }
            }
            Message::RoleReset => {
                self.role_error = None;
                let saved = self
                    .role_form
                    .as_ref()
                    .and_then(|d| d.id)
                    .and_then(|id| self.server.roles.iter().find(|r| r.id == id).cloned());
                if let Some(draft) = self.role_form.as_mut() {
                    *draft = match saved {
                        Some(role) => crate::app::RoleDraft {
                            id: Some(role.id),
                            name: role.name,
                            color: role.color,
                            permissions: role.permissions,
                            mentionable: role.mentionable,
                            hoisted: role.hoisted,
                        },
                        None => crate::app::RoleDraft::default(),
                    };
                }
            }
            Message::RoleSave => {
                if let Some(draft) = self.role_form.clone() {
                    let risky = !crate::view::pages::lockout_risk(self, &draft).is_empty() || crate::view::pages::grants_administrator(self, &draft);
                    if risky {
                        self.role_save_warning = true;
                    } else {
                        self.send_role_save(draft);
                    }
                }
            }
            Message::RoleSaveConfirm => {
                self.role_save_warning = false;
                if let Some(draft) = self.role_form.clone() {
                    self.send_role_save(draft);
                }
            }
            Message::RoleSaveCancel => self.role_save_warning = false,
            Message::AdminGrantConfirm => {
                self.admin_grant_pending = false;
                if let Some(draft) = self.role_form.as_mut() {
                    draft.permissions |= common::perm::ADMINISTRATOR;
                }
            }
            Message::AdminGrantCancel => self.admin_grant_pending = false,
            Message::RoleDeleteAsk(role, name) => {
                self.pending_role_delete = Some((role, name));
            }
            Message::RoleDeleteCancel => self.pending_role_delete = None,
            Message::RoleDeleteConfirm => {
                if let Some((role, _)) = self.pending_role_delete.take() {
                    self.role_form = None;
                    self.role_error = None;
                    let _ = self.backend.send_main(ClientMsg::DeleteRole { role });
                }
            }
            Message::RoleMove(role, before) => {
                self.role_error = None;
                let _ = self.backend.send_main(ClientMsg::MoveRole { role, before });
            }
            Message::RolesMenuOpen => self.roles_menu_open = true,
            Message::RolesMenuClose => self.roles_menu_open = false,
            // The menu stays open, so several roles can be changed in a row.
            Message::SetMemberRole(user, role, on) => {
                let _ = self.backend.send_main(ClientMsg::SetMemberRole { user, role, on });
            }
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
        if self.access_add.is_some() {
            self.access_add = None;
        } else if self.search_open {
            self.search_open = false;
        } else if self.card.is_some() {
            self.card = None;
        } else if self.menu.is_some() || self.picker.is_some() {
            self.menu = None;
            self.picker = None;
        } else if self.editing.is_some() {
            self.editing = None;
        } else if self.pending_delete.is_some() {
            self.pending_delete = None;
        } else if self.pending_role_delete.is_some() {
            self.pending_role_delete = None;
        } else if self.pending_channel_delete.is_some() {
            self.pending_channel_delete = None;
        } else if self.pending_category_delete.is_some() {
            self.pending_category_delete = None;
        } else if self.channel_edit.is_some() {
            return self.close_channel_edit();
        } else if self.category_edit.is_some() {
            return self.close_category_edit();
        } else if self.category_open {
            self.category_open = false;
        } else if self.add_server.open {
            self.add_server.open = false;
        } else if self.account.editing.is_some() {
            self.account.editing = None;
        } else if matches!(self.page, Page::User | Page::Server) {
            return self.close_settings();
        } else if matches!(self.page, Page::Discover) {
            self.page = Page::None;
        }
        Task::none()
    }

    /// Changes between Online and Idle with input, while the status follows input.
    pub(crate) fn apply_auto_status(&mut self) {
        if !self.is_logged_in() {
            return;
        }
        let Some(idle) = crate::app::auto_idle_change(self.auto_status, self.last_active.elapsed(), self.auto_idle) else {
            return;
        };
        self.auto_idle = idle;
        // Every server the app is connected to, not only the open one.
        self.backend.set_status_everywhere(self.effective_status());
    }

    /// Starts the bio editor from the bio being edited.
    /// Opens a profile card at a point and asks for the card's details.
    fn open_card(&mut self, id: String, name: String, at: iced::Point, above: bool) -> Task<Message> {
        self.card_message.clear();
        self.card_roles_open = false;
        self.card_badges_open = false;
        self.card_list_search.clear();
        self.card = Some(crate::app::CardView { id: id.clone(), name, at, loading: true, card: None, error: None, above });
        let backend = self.backend.clone();
        Task::perform(offload(move || {
            let result = backend.profile_card(&id);
            (id, result)
        }), |(id, result)| Message::CardLoaded(id, result))
    }

    fn reset_bio_editor(&mut self) {
        self.bio_editor = iced::widget::text_editor::Content::with_text(&self.account.about_bio);
    }

    /// Sends a role save. The editor stays open with what was saved, so the page does not change.
    fn send_role_save(&mut self, draft: crate::app::RoleDraft) {
        self.role_error = None;
        let name = draft.name.trim().to_string();
        let color = draft.color.trim().to_string();
        if draft.id.is_none() {
            self.role_created_name = Some(name.clone());
        }
        let msg = match draft.id {
            Some(role) => ClientMsg::UpdateRole {
                role,
                name,
                color,
                permissions: draft.permissions,
                mentionable: draft.mentionable,
                hoisted: draft.hoisted,
            },
            None => ClientMsg::CreateRole {
                name,
                color,
                permissions: draft.permissions,
                mentionable: draft.mentionable,
                hoisted: draft.hoisted,
            },
        };
        let _ = self.backend.send_main(msg);
    }

    /// Closes a settings page. Refused while it has unsaved changes: the save bar flashes red instead.
    /// Sends the open page's permission changes that differ from the server's, and clears the draft.
    fn save_access(&mut self) {
        use crate::app::AccessTarget;
        use common::Subject;
        let Some(draft) = self.access_draft.take() else { return };
        for (subject, (view, send)) in draft.subjects.clone() {
            if (view, send) == self.server_access(draft.target, &subject) {
                continue;
            }
            let msg = match (draft.target, subject) {
                (AccessTarget::Channel(channel), Subject::Role(role)) => ClientMsg::SetChannelOverride { channel, role, view, send },
                (AccessTarget::Channel(channel), Subject::Member(member)) => ClientMsg::SetChannelMemberOverride { channel, member, view, send },
                (AccessTarget::Category(category), Subject::Role(role)) => ClientMsg::SetCategoryOverride { category, role, view, send },
                (AccessTarget::Category(category), Subject::Member(member)) => ClientMsg::SetCategoryMemberOverride { category, member, view, send },
            };
            let _ = self.backend.send_main(msg);
        }
    }

    /// Closes the channel edit page. Refused, with the save bar flashing red, while it has unsaved changes.
    fn close_channel_edit(&mut self) -> Task<Message> {
        if self.edit_dirty() {
            self.unsaved_flash = Some(std::time::Instant::now());
            return Task::none();
        }
        self.unsaved_flash = None;
        self.channel_edit = None;
        self.rename_channel = None;
        self.access_draft = None;
        Task::none()
    }

    /// Closes the category edit page. Refused, with the save bar flashing red, while it has unsaved changes.
    fn close_category_edit(&mut self) -> Task<Message> {
        if self.edit_dirty() {
            self.unsaved_flash = Some(std::time::Instant::now());
            return Task::none();
        }
        self.unsaved_flash = None;
        self.category_edit = None;
        self.access_draft = None;
        Task::none()
    }

    fn close_settings(&mut self) -> Task<Message> {
        if self.settings_unsaved() {
            self.unsaved_flash = Some(std::time::Instant::now());
            return Task::none();
        }
        self.unsaved_flash = None;
        self.role_form = None;
        self.role_error = None;
        self.page = Page::None;
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
                let mut out = AccountSaved { profile: None, email: None, name_saved: false, username_saved: false, email_saved: false, error: None, auth_down: false };
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
                            out.auth_down = e.auth_down;
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
                            out.auth_down = e.auth_down;
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
            // A new address has to be confirmed again.
            self.account.verified = false;
        }
        self.account.save_error = saved.error;
        if saved.auth_down {
            self.auth_down = true;
        }
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

impl App {
    /// Saves the profile picture (or removes it) and reports the result as `AvatarDone`.
    fn save_avatar(&mut self, png: Option<String>) -> Task<Message> {
        self.account.avatar_error = None;
        let backend = self.backend.clone();
        Task::perform(offload(move || backend.set_avatar(png)), Message::AvatarDone)
    }
}

impl App {
    /// Closes the main window and keeps the app running in the tray.
    pub(crate) fn hide_window(&mut self) -> Task<Message> {
        match self.window.take() {
            Some(id) => Task::batch([window::close(id), Task::done(Message::TrayVisible(false))]),
            None => Task::none(),
        }
    }
}
