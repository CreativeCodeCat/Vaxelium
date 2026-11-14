//! The app's data: the open server and its channels, the messages, the direct message links, and the
//! helpers that answer questions about them.

use std::collections::{HashMap, HashSet};

use common::{perm, ChatMessage, DirectMessage, ServerMsg};

use crate::app::App;
use crate::prefs::ServerPref;

/// The gap, in seconds, after which the same author starts a new group of messages.
pub const GROUP_GAP_SECS: u64 = 7 * 60;

/// One entry in a channel's list: a chat message or a notice from the server.
#[derive(Debug, Clone)]
pub enum Entry {
    Chat(ChatMessage),
    System { text: String, at: u64 },
}

impl Entry {
    pub fn time(&self) -> u64 {
        match self {
            Entry::Chat(m) => m.timestamp,
            Entry::System { at, .. } => *at,
        }
    }
}

/// The open server: its details, channels, direct messages and members.
#[derive(Debug, Clone, Default)]
pub struct ServerView {
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub public: bool,
    pub channels: Vec<common::ChannelRef>,
    /// The server's categories, in order, with the channels in each.
    pub categories: Vec<common::CategoryInfo>,
    /// The channels in no category, in order.
    pub uncategorized: Vec<common::ChannelRef>,
    /// The open channel's name, and its permanent id.
    pub channel: String,
    pub channel_id: i64,
    /// The server's own id, as it sent it.
    pub server_id: String,
    pub dms: Vec<DirectMessage>,
    pub online: Vec<String>,
    pub offline: Vec<String>,
    /// Each member's account id, by display name, as the server's member list gave them.
    pub member_ids: std::collections::HashMap<String, String>,
    pub owner: Option<String>,
    pub owner_id: Option<String>,
    pub user_id: String,
    /// The server's roles, highest first.
    pub roles: Vec<common::RoleInfo>,
    /// Each member's role ids, by account id.
    pub member_roles: HashMap<String, Vec<i64>>,
}

/// A background connection to another server, for its direct messages.
#[derive(Debug, Clone, Default)]
pub struct DmLink {
    pub id: u64,
    pub server_name: String,
    pub dms: Vec<DirectMessage>,
    /// Conversations with unread messages, by channel id.
    pub unread: HashSet<i64>,
    pub history: HashMap<String, Vec<ChatMessage>>,
}

/// The connection to the open server.
#[derive(Debug, Clone, Default)]
pub struct MainConn {
    pub id: Option<u64>,
    pub addr: Option<String>,
    pub seq: u64,
    pub connecting: bool,
    /// Messages that arrive before the connect call returns the connection's id.
    pub buffer: Vec<(u64, ServerMsg)>,
    pub error: Option<String>,
    pub welcomed: bool,
    /// The first channel has been chosen (or a direct message was asked for instead).
    pub joined_initial: bool,
}

/// One conversation with a person, on one server.
#[derive(Debug, Clone)]
pub struct Target {
    pub addr: String,
    pub channel: String,
    pub server: String,
    pub last: u64,
}

/// One row of the direct messages list: a person across every server.
#[derive(Debug, Clone)]
pub struct PersonRow {
    pub id: String,
    pub name: String,
    pub last: u64,
    pub unread: bool,
}

pub fn is_dm_channel(channel: &str) -> bool {
    channel.starts_with("dm-")
}

impl App {
    /// True when logged in to the auth service.
    pub fn is_logged_in(&self) -> bool {
        self.user.is_some()
    }

    /// True when joined the open server and ready to show its channels.
    pub fn chat_ready(&self) -> bool {
        self.main.id.is_some() && self.main.welcomed
    }

    pub fn selected_server(&self) -> Option<&ServerPref> {
        self.selected.and_then(|i| self.prefs.servers.get(i))
    }

    pub fn selected_name(&self) -> Option<String> {
        self.selected_server().map(|s| s.name.clone())
    }

    pub fn is_owner(&self) -> bool {
        self.server.owner_id.as_deref().is_some_and(|id| id == self.user_id)
    }

    /// The permissions this account has on the open server. The owner has all of them; anyone else
    /// has the ones their roles grant together.
    pub fn my_permissions(&self) -> u32 {
        if self.is_owner() {
            return perm::ALL;
        }
        // @everyone is held by everyone, so its permissions apply first.
        let held = self.server.member_roles.get(&self.user_id);
        let everyone = self.server.roles.iter().find(|r| r.id == common::EVERYONE_ROLE).map_or(0, |r| r.permissions);
        self.server
            .roles
            .iter()
            .filter(|r| r.id != common::EVERYONE_ROLE && held.is_some_and(|h| h.contains(&r.id)))
            .fold(everyone, |bits, r| bits | r.permissions)
    }

    /// The colour of a member's name: the highest role they have that has a colour, or else
    /// @everyone's colour. `None` when there is no colour to show.
    pub fn member_color(&self, member_id: &str) -> Option<iced::Color> {
        let held = self.server.member_roles.get(member_id);
        let own = self
            .server
            .roles
            .iter()
            .find(|r| r.id != common::EVERYONE_ROLE && !r.color.is_empty() && held.is_some_and(|h| h.contains(&r.id)));
        let role = own.or_else(|| self.server.roles.iter().find(|r| r.id == common::EVERYONE_ROLE))?;
        crate::util::hex_color(&role.color)
    }

    /// True when this account has the permission (see `common::perm`).
    pub fn can(&self, permission: u32) -> bool {
        self.my_permissions() & permission == permission
    }

    /// The index, in the role order, of this account's highest role.
    fn my_top_role(&self) -> Option<usize> {
        let held = self.server.member_roles.get(&self.user_id)?;
        self.server.roles.iter().position(|r| held.contains(&r.id))
    }

    /// True when this account sits above the role, so it may edit, move or give it. Matches the server.
    pub fn outranks_role(&self, role: i64) -> bool {
        if self.is_owner() {
            return true;
        }
        let Some(index) = self.server.roles.iter().position(|r| r.id == role) else {
            return false;
        };
        self.my_top_role().is_some_and(|top| index > top)
    }

    /// The highest role a member has, by account id.
    pub fn top_role(&self, member_id: &str) -> Option<&common::RoleInfo> {
        let held = self.server.member_roles.get(member_id)?;
        self.server.roles.iter().find(|r| held.contains(&r.id))
    }

    /// The highest role a member has, by display name.
    pub fn top_role_by_name(&self, name: &str) -> Option<&common::RoleInfo> {
        let id = self.server.member_ids.get(name)?;
        self.top_role(id)
    }

    /// The name of a direct message's other member, from this server's list or another server's.
    pub(crate) fn dm_name(&self, channel: &str) -> Option<String> {
        if let Some(own) = self.server.dms.iter().find(|d| d.channel == channel) {
            return Some(own.name.clone());
        }
        self.dm_links
            .values()
            .find_map(|link| link.dms.iter().find(|d| d.channel == channel).map(|d| d.name.clone()))
    }

    /// How the channel is named: "#name", or "@ name" for a direct message.
    pub fn channel_label(&self) -> String {
        self.channel_label_for(&self.server.channel)
    }

    /// The name of a channel by its id, from the server's list.
    pub fn channel_name_of(&self, id: i64) -> Option<String> {
        self.server.channels.iter().find(|c| c.id == id).map(|c| c.name.clone())
    }

    pub fn channel_label_for(&self, channel: &str) -> String {
        if is_dm_channel(channel) {
            format!("@ {}", self.dm_name(channel).unwrap_or_else(|| "Direct message".into()))
        } else {
            format!("#{channel}")
        }
    }

    /// The person whose conversation is open on the direct messages page, by hidden account id.
    pub fn current_person(&self) -> Option<String> {
        if self.main.id.is_none() || !is_dm_channel(&self.server.channel) {
            return None;
        }
        let channel = &self.server.channel;
        if let Some(own) = self.server.dms.iter().find(|d| d.channel == *channel) {
            return Some(own.other_id.clone());
        }
        let link = self.main.addr.as_ref().and_then(|a| self.dm_links.get(a))?;
        link.dms.iter().find(|d| d.channel == *channel).map(|d| d.other_id.clone())
    }

    pub(crate) fn server_label(&self, addr: &str, server_name: &str) -> String {
        if !server_name.is_empty() {
            return server_name.to_string();
        }
        self.prefs.servers.iter().find(|s| s.addr == addr).map(|s| s.name.clone()).unwrap_or_else(|| addr.to_string())
    }

    /// Every conversation with this person, on every server, in a stable order.
    pub fn person_targets(&self, other_id: &str) -> Vec<Target> {
        let mut out = Vec::new();
        for (addr, link) in &self.dm_links {
            for dm in link.dms.iter().filter(|d| d.other_id == other_id) {
                out.push(Target {
                    addr: addr.clone(),
                    channel: dm.channel.clone(),
                    server: self.server_label(addr, &link.server_name),
                    last: dm.last_message,
                });
            }
        }
        out.sort_by(|a, b| (&a.addr, &a.channel).cmp(&(&b.addr, &b.channel)));
        out
    }

    /// The direct messages list: one row per person across every server, most recent first.
    pub fn persons(&self) -> Vec<PersonRow> {
        let mut rows: HashMap<String, PersonRow> = HashMap::new();
        for link in self.dm_links.values() {
            for dm in &link.dms {
                let row = rows.entry(dm.other_id.clone()).or_insert_with(|| PersonRow {
                    id: dm.other_id.clone(),
                    name: dm.name.clone(),
                    last: 0,
                    unread: false,
                });
                if dm.last_message >= row.last {
                    row.last = dm.last_message;
                    row.name = dm.name.clone();
                }
                if link.unread.contains(&dm.channel_id) {
                    row.unread = true;
                }
            }
        }
        // Blocked people are not listed.
        let mut rows: Vec<PersonRow> = rows.into_values().filter(|r| !self.is_blocked(&r.id)).collect();
        rows.sort_by(|a, b| b.last.cmp(&a.last).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    pub fn person_name(&self, other_id: &str) -> Option<String> {
        self.persons().into_iter().find(|p| p.id == other_id).map(|p| p.name)
    }

    /// The messages with one person from every server, merged by time. Ids are shifted per server
    /// (see `VIRTUAL_BASE`), so the server a message belongs to can be found again.
    pub fn dm_timeline(&self) -> Vec<Entry> {
        let Some(person) = self.current_person() else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        for (index, target) in self.person_targets(&person).iter().enumerate() {
            let Some(history) = self.dm_links.get(&target.addr).and_then(|l| l.history.get(&target.channel)) else {
                continue;
            };
            for message in history {
                let mut m = message.clone();
                if m.id != 0 {
                    m.id += (index as i64 + 1) * VIRTUAL_BASE;
                }
                entries.push(Entry::Chat(m));
            }
        }
        entries.sort_by_key(|e| e.time());
        entries
    }

    /// The entries shown for the open view: channel messages, or the direct message timeline.
    pub fn current_entries(&self) -> Vec<Entry> {
        if self.current_person().is_some() {
            return self
                .dm_timeline()
                .into_iter()
                .filter(|e| !matches!(e, Entry::Chat(m) if self.is_blocked(&m.author_id)))
                .collect();
        }
        self.messages
            .get(&self.server.channel_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|e| match e {
                Entry::Chat(m) => !self.is_blocked(&m.author_id) && !self.is_ignored_here(&m.author_id),
                Entry::System { .. } => true,
            })
            .collect()
    }

    /// The permissions a member has on the open server, by account id: everyone's, plus their roles'.
    /// The owner has every permission.
    pub fn permissions_of(&self, member_id: &str) -> u32 {
        if self.server.owner_id.as_deref() == Some(member_id) {
            return perm::ALL;
        }
        let held = self.server.member_roles.get(member_id);
        let everyone = self.server.roles.iter().find(|r| r.id == common::EVERYONE_ROLE).map_or(0, |r| r.permissions);
        self.server
            .roles
            .iter()
            .filter(|r| r.id != common::EVERYONE_ROLE && held.is_some_and(|h| h.contains(&r.id)))
            .fold(everyone, |bits, r| bits | r.permissions)
    }

    /// True when nobody may ignore or block this account: the owner, and anyone with Bypass ignore.
    pub fn is_protected(&self, account_id: &str) -> bool {
        self.permissions_of(account_id) & perm::BYPASS_IGNORE != 0
    }

    /// True when this account is blocked on every server. A protected account is never blocked.
    pub fn is_blocked(&self, account_id: &str) -> bool {
        !self.is_protected(account_id) && self.prefs.blocked.iter().any(|id| id == account_id)
    }

    /// True when this account is ignored on the open server. A protected account is never ignored.
    pub fn is_ignored_here(&self, account_id: &str) -> bool {
        !self.is_protected(account_id) && self.selected_server().is_some_and(|s| s.ignored.iter().any(|id| id == account_id))
    }

    /// Which server a direct message id belongs to, and its real id there.
    pub fn virtual_target(&self, id: i64) -> Option<(Target, i64)> {
        if id < VIRTUAL_BASE {
            return None;
        }
        let person = self.current_person()?;
        let index = (id / VIRTUAL_BASE) as usize - 1;
        let target = self.person_targets(&person).into_iter().nth(index)?;
        Some((target, id % VIRTUAL_BASE))
    }

    pub fn can_edit(&self, m: &ChatMessage) -> bool {
        m.author_id == self.user_id
    }

    pub fn can_delete(&self, m: &ChatMessage) -> bool {
        self.can_edit(m) || self.can(perm::MANAGE_MESSAGES)
    }

    /// The message with this id in the open view, if there is one.
    pub fn find_message(&self, id: i64) -> Option<ChatMessage> {
        self.current_entries().into_iter().find_map(|e| match e {
            Entry::Chat(m) if m.id == id => Some(m),
            _ => None,
        })
    }
}

/// Added to the ids of direct messages from the second server on, so each message id is unique in
/// the merged timeline. Real ids stay far below this.
pub const VIRTUAL_BASE: i64 = 1_000_000_000_000;
