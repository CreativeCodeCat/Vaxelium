//! Checks the app's state changes when servers send messages, without a network or a window.

use std::collections::HashMap;
use std::sync::mpsc;

use common::{ChannelRef, ChatMessage, ServerMsg, UserStatus};
use iced::Point;

use crate::app::{App, Message};
use crate::backend::{Backend, Event};
use crate::prefs::{Prefs, ServerPref};

fn app() -> App {
    let (tx, _rx) = mpsc::channel();
    let backend = Backend::new(tx, std::env::temp_dir().join("vaxelium-flows"));
    let mut prefs = Prefs::default();
    prefs.servers = vec![ServerPref { name: "Home".into(), addr: "127.0.0.1:7878".into(), icon: None, id: None, ignored: Vec::new(), protected: Vec::new(), linked: false }];
    let mut app = App::new(backend, prefs, Vec::new(), HashMap::new());
    app.user = Some("alice".into());
    app.selected = Some(0);
    app.main.id = Some(7);
    app.main.addr = Some("127.0.0.1:7878".into());
    app
}

fn net(msg: ServerMsg) -> Message {
    Message::Event(Event::Net { dm: false, addr: "127.0.0.1:7878".into(), id: 7, msg })
}

fn chan(id: i64, name: &str) -> ChannelRef {
    ChannelRef { id, name: name.into() }
}

fn chat(id: i64, channel_id: i64, author: &str, text: &str, ts: u64) -> ChatMessage {
    ChatMessage {
        id,
        channel_id,
        channel: String::new(),
        author: author.into(),
        author_id: format!("id-{author}"),
        text: text.into(),
        timestamp: ts,
        edited: false,
        reactions: Vec::new(),
    }
}

fn welcome(app: &mut App) {
    let _ = app.update(net(ServerMsg::Welcome {
        server_id: "server-1".into(),
        name: "alice".into(),
        user_id: "id-alice".into(),
        server_name: "Home".into(),
        server_description: String::new(),
        server_icon: None,
        server_public: false,
    }));
}

#[test]
fn welcome_then_joined_shows_history_and_presence() {
    let mut app = app();
    welcome(&mut app);
    assert!(app.chat_ready());
    assert_eq!(app.user_id, "id-alice");
    assert_eq!(app.server.server_id, "server-1");

    let _ = app.update(net(ServerMsg::Joined {
        channel: chan(1, "general"),
        history: vec![chat(1, 1, "bob", "hello", 1_700_000_000)],
    }));
    assert_eq!(app.server.channel, "general");
    assert_eq!(app.server.channel_id, 1);
    assert_eq!(app.messages[&1].len(), 1);

    let _ = app.update(net(ServerMsg::Presence {
        users: vec![
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: false, owner: false, roles: vec![] },
        ],
    }));
    assert!(app.is_owner());
    assert_eq!(app.server.offline, vec!["bob".to_string()]);
}

#[test]
fn a_server_that_changes_identity_is_refused() {
    let mut app = app();
    app.prefs.servers[0].id = Some("someone-else".into());
    welcome(&mut app);
    assert!(!app.chat_ready(), "the address now belongs to another server");
    assert!(app.main.error.as_deref().unwrap_or("").contains("different server"));
}

#[test]
fn incoming_message_is_added_to_its_channel_and_marks_unread() {
    let mut app = app();
    welcome(&mut app);
    let _ = app.update(net(ServerMsg::Joined { channel: chan(1, "general"), history: Vec::new() }));
    let _ = app.update(net(ServerMsg::Message { message: chat(2, 1, "bob", "hi", 1_700_000_100) }));
    let _ = app.update(net(ServerMsg::Activity { channel_id: 2 }));
    assert_eq!(app.messages[&1].len(), 1);
    assert!(app.unread.contains(&2));
}

#[test]
fn edit_and_delete_change_the_message_list() {
    let mut app = app();
    let _ = app.update(net(ServerMsg::Joined {
        channel: chan(1, "general"),
        history: vec![chat(3, 1, "alice", "first", 1_700_000_200), chat(4, 1, "alice", "second", 1_700_000_201)],
    }));
    let _ = app.update(net(ServerMsg::MessageEdited { channel_id: 1, id: 3, text: "changed".into() }));
    let _ = app.update(net(ServerMsg::MessageDeleted { channel_id: 1, id: 4 }));
    let list = &app.messages[&1];
    assert_eq!(list.len(), 1);
    match &list[0] {
        crate::model::Entry::Chat(m) => {
            assert_eq!(m.text, "changed");
            assert!(m.edited);
        }
        other => panic!("expected a chat message, got {other:?}"),
    }
}

#[test]
fn escape_closes_the_search_menu_first() {
    let mut app = app();
    let _ = app.update(Message::SearchOpen);
    assert!(app.search_open);
    let _ = app.update(Message::Escape);
    assert!(!app.search_open);
}

#[test]
fn search_filters_match_author_mention_channel_and_dates() {
    use crate::search::Search;
    // 2023-11-14 12:00 UTC is 14 November 2023 in every time zone from UTC-12 to UTC+11.
    let mut m = chat(1, 1, "Bob", "hello @alice see https://example.com", 1_699_963_200);
    m.channel = "general".into();
    assert!(Search::parse("from:bob").matches(&m, "general"));
    assert!(!Search::parse("from:carol").matches(&m, "general"));
    assert!(Search::parse("mentions:alice").matches(&m, "general"));
    assert!(!Search::parse("mentions:ali").matches(&m, "general"), "a partial name is not a mention");
    assert!(Search::parse("in:general").matches(&m, "general"));
    assert!(!Search::parse("in:random").matches(&m, "general"));
    assert!(Search::parse("has:link").matches(&m, "general"));
    assert!(!Search::parse("has:file").matches(&m, "general"), "messages carry no files");
    assert!(Search::parse("during:2023-11-14").matches(&m, "general"));
    assert!(!Search::parse("before:2023-11-14").matches(&m, "general"));
    assert!(Search::parse("after:2023-11-01").matches(&m, "general"));
    assert!(!Search::parse("during:2023-13-40").matches(&m, "general"), "an invalid date matches nothing");
    assert!(Search::parse("hello from:bob").matches(&m, "general"));
}

#[test]
fn dragging_a_splitter_changes_the_panel_width_live() {
    let mut app = app();
    let _ = app.update(Message::CursorMoved(Point::new(900.0, 300.0)));
    let before = app.panel_width("members");
    let _ = app.update(Message::DragStart("members", true));
    // The member list grows to the left: moving 60 px left makes it 60 px wider.
    let _ = app.update(Message::CursorMoved(Point::new(840.0, 300.0)));
    assert_eq!(app.panel_width("members"), before + 60.0);
    let _ = app.update(Message::MouseReleased);
    assert_eq!(app.prefs.layout.members, before + 60.0);
}

#[test]
fn view_build_time_with_a_busy_channel() {
    let mut app = app();
    app.user_id = "id-alice".into();
    app.server.channel = "general".into();
    app.server.channel_id = 1;
    app.main.welcomed = true;
    let now = crate::util::now();
    let history: Vec<ChatMessage> = (0..300)
        .map(|i| chat(i + 1, 1, if i % 3 == 0 { "alice" } else { "bob" }, &format!("message number {i} with some words to wrap across the line"), now - 3000 + i as u64))
        .collect();
    app.messages.insert(1, history.into_iter().map(crate::model::Entry::Chat).collect());
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _ = crate::view::view(&app);
    }
    eprintln!("view build: {:?} per frame", start.elapsed() / 10);
}

#[test]
fn picking_a_suggestion_replaces_the_word_being_typed() {
    let mut app = app();
    app.search = "hello in:ge".into();
    let _ = app.update(Message::SearchPick("in:general".into()));
    assert_eq!(app.search, "hello in:general ");
    app.search = "in:".into();
    let _ = app.update(Message::SearchPick("in:random".into()));
    assert_eq!(app.search, "in:random ");
}

#[test]
fn clear_empties_the_search_and_closes_its_menu() {
    let mut app = app();
    let _ = app.update(Message::SearchOpen);
    let _ = app.update(Message::SearchChanged("hello".into()));
    let _ = app.update(Message::SearchClear);
    assert!(app.search.is_empty());
    assert!(!app.search_open);
}

#[test]
fn categories_from_the_server_are_shown() {
    let mut app = app();
    let _ = app.update(net(ServerMsg::Channels { channels: vec![chan(1, "general"), chan(2, "random")] }));
    let _ = app.update(net(ServerMsg::Categories {
        categories: vec![common::CategoryInfo { id: 10, name: "Text".into(), channels: vec![chan(1, "general")] }],
        uncategorized: vec![chan(2, "random")],
    }));
    assert_eq!(app.server.categories[0].name, "Text");
    assert_eq!(app.server.uncategorized[0].name, "random");
}

#[test]
fn a_rename_keeps_the_channel_open_and_its_messages() {
    let mut app = app();
    let _ = app.update(net(ServerMsg::Channels { channels: vec![chan(1, "general"), chan(2, "dev")] }));
    let _ = app.update(net(ServerMsg::Joined {
        channel: chan(1, "general"),
        history: vec![chat(1, 1, "bob", "hello", 1_700_000_000)],
    }));
    let _ = app.update(net(ServerMsg::ChannelRenamed { id: 1, name: "lobby".into() }));
    assert_eq!(app.server.channel, "lobby");
    assert_eq!(app.server.channel_id, 1, "the id does not change");
    assert_eq!(app.server.channels[0].name, "lobby");
    assert_eq!(app.messages[&1].len(), 1);
}

#[test]
fn rename_dialog_opens_and_saves_only_a_real_change() {
    let mut app = app();
    let _ = app.update(Message::RenameStart(2, "dev".into()));
    assert_eq!(app.rename_channel, Some((2, "dev".into(), "dev".into())));
    let _ = app.update(Message::RenameInput("   ".into()));
    let _ = app.update(Message::RenameSubmit);
    assert!(app.rename_channel.is_none(), "the dialog closes on save even with an empty name");
}

#[test]
fn a_press_that_does_not_move_is_a_click_and_a_drag_clears_on_release() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    let _ = app.update(net(ServerMsg::Channels { channels: vec![chan(1, "general"), chan(2, "random")] }));
    let _ = app.update(net(ServerMsg::Categories {
        categories: vec![common::CategoryInfo { id: 10, name: "Text".into(), channels: Vec::new() }],
        uncategorized: Vec::new(),
    }));
    // A press and release in place is a click: the drag ends.
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 100.0)));
    let _ = app.update(Message::DndPress(DragItem::Channel(2)));
    let _ = app.update(Message::MouseReleased);
    assert!(app.dnd.is_none());
    // A press, a move, and a release over a category: the drag ends too, and it was a real move.
    let _ = app.update(Message::DndPress(DragItem::Channel(2)));
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 160.0)));
    assert!(app.dnd.as_ref().is_some_and(|d| d.moved));
    let _ = app.update(Message::DndOver(Some(DropTarget::Category(10))));
    let _ = app.update(Message::MouseReleased);
    assert!(app.dnd.is_none());
}

#[test]
fn deleting_a_channel_asks_first_and_removes_it_when_the_server_says_so() {
    let mut app = app();
    let _ = app.update(net(ServerMsg::Channels { channels: vec![chan(1, "general"), chan(2, "random")] }));
    let _ = app.update(net(ServerMsg::Joined { channel: chan(2, "random"), history: Vec::new() }));
    let _ = app.update(Message::DeleteChannelAsk(2, "random".into()));
    assert_eq!(app.pending_channel_delete, Some((2, "random".into())));
    let _ = app.update(Message::DeleteChannelCancel);
    assert!(app.pending_channel_delete.is_none());

    let _ = app.update(net(ServerMsg::ChannelDeleted { id: 2 }));
    assert_eq!(app.server.channels.len(), 1);
    assert!(!app.messages.contains_key(&2));
}

#[test]
fn a_category_dropped_on_the_end_zone_moves_to_the_end() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    let _ = app.update(net(ServerMsg::Categories {
        categories: vec![
            common::CategoryInfo { id: 1, name: "One".into(), channels: Vec::new() },
            common::CategoryInfo { id: 2, name: "Two".into(), channels: Vec::new() },
        ],
        uncategorized: Vec::new(),
    }));
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 100.0)));
    let _ = app.update(Message::DndPress(DragItem::Category(1)));
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 200.0)));
    let _ = app.update(Message::DndOver(Some(DropTarget::End)));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), Some(DropTarget::End));
    let _ = app.update(Message::MouseReleased);
    assert!(app.dnd.is_none());
}

#[test]
fn roles_grant_permissions_to_their_holders() {
    let mut app = app();
    welcome(&mut app);
    let mods = common::RoleInfo {
        id: 5,
        name: "Mods".into(),
        color: "#ff0000".into(),
        permissions: common::perm::MANAGE_MESSAGES,
        position: 1,
        mentionable: false,
        hoisted: false,
    };
    let _ = app.update(net(ServerMsg::Roles { roles: vec![mods] }));
    let _ = app.update(net(ServerMsg::Presence {
        users: vec![
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![5] },
        ],
    }));

    // Alice is the owner, so she has everything and may manage every role.
    assert!(app.can(common::perm::MANAGE_ROLES));
    assert!(app.outranks_role(5));

    // Bob holds Mods: he may delete messages, but not channels, and cannot manage the role he holds.
    app.user_id = "id-bob".into();
    assert!(app.can(common::perm::MANAGE_MESSAGES));
    assert!(!app.can(common::perm::MANAGE_CHANNELS));
    assert!(!app.outranks_role(5));
    let top = |name: &str| app.server.member_ids.get(name).and_then(|id| app.top_role(id)).map(|r| r.name.clone());
    assert_eq!(top("bob").as_deref(), Some("Mods"));
    assert!(top("alice").is_none());
}

#[test]
fn deleting_a_role_asks_first() {
    let mut app = app();
    welcome(&mut app);
    let _ = app.update(Message::RoleDeleteAsk(5, "Mods".into()));
    assert_eq!(app.pending_role_delete, Some((5, "Mods".into())));
    let _ = app.update(Message::RoleDeleteCancel);
    assert!(app.pending_role_delete.is_none());
    let _ = app.update(Message::RoleDeleteAsk(5, "Mods".into()));
    let _ = app.update(Message::RoleDeleteConfirm);
    assert!(app.pending_role_delete.is_none());
}

#[test]
fn ignore_hides_on_this_server_and_block_hides_everywhere() {
    let mut app = app();
    welcome(&mut app);
    let _ = app.update(net(ServerMsg::Joined {
        channel: chan(1, "general"),
        history: vec![chat(1, 1, "bob", "hi", 1), chat(2, 1, "carol", "hello", 2)],
    }));
    assert_eq!(app.current_entries().len(), 2);

    // Ignoring bob hides his messages here, and only here.
    let _ = app.update(Message::IgnoreToggle("id-bob".into()));
    assert!(app.is_ignored_here("id-bob"));
    assert_eq!(app.current_entries().len(), 1);
    assert_eq!(app.prefs.servers[0].ignored, vec!["id-bob".to_string()]);

    // Blocking carol hides her on every server. Unignoring bob shows him again.
    let _ = app.update(Message::BlockToggle("id-carol".into()));
    assert!(app.is_blocked("id-carol"));
    assert_eq!(app.current_entries().len(), 0);
    let _ = app.update(Message::IgnoreToggle("id-bob".into()));
    assert!(!app.is_ignored_here("id-bob"));
    assert_eq!(app.current_entries().len(), 1);

    // Blocking again unblocks.
    let _ = app.update(Message::BlockToggle("id-carol".into()));
    assert!(!app.is_blocked("id-carol"));
}

#[test]
fn the_owner_and_bypass_holders_cannot_be_ignored_or_blocked() {
    let mut app = app();
    welcome(&mut app);
    let _ = app.update(net(ServerMsg::Roles {
        roles: vec![common::RoleInfo {
            id: 7,
            name: "Staff".into(),
            color: String::new(),
            permissions: common::perm::BYPASS_IGNORE,
            position: 1,
            mentionable: false,
            hoisted: false,
        }],
    }));
    let _ = app.update(net(ServerMsg::Presence {
        users: vec![
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![7] },
            UserStatus { name: "carol".into(), id: "id-carol".into(), online: true, owner: false, roles: vec![] },
        ],
    }));
    // The owner and bob (who has Bypass ignore) cannot be ignored or blocked.
    for id in ["id-alice", "id-bob"] {
        let _ = app.update(Message::IgnoreToggle(id.into()));
        let _ = app.update(Message::BlockToggle(id.into()));
        assert!(!app.is_ignored_here(id));
        assert!(!app.is_blocked(id));
        assert!(app.prefs.blocked.is_empty());
        assert!(app.prefs.servers[0].ignored.is_empty());
    }
    // Carol can be.
    let _ = app.update(Message::BlockToggle("id-carol".into()));
    assert!(app.is_blocked("id-carol"));
}

#[test]
fn bypass_protects_only_on_the_server_that_grants_it() {
    let mut app = app();
    app.prefs.servers.push(ServerPref {
        name: "Other".into(),
        addr: "other.example:7878".into(),
        icon: None,
        id: None,
        ignored: Vec::new(),
        protected: Vec::new(),
        linked: false,
    });
    welcome(&mut app);
    let addr = app.prefs.servers[0].addr.clone();
    let _ = app.update(net(ServerMsg::Roles {
        roles: vec![common::RoleInfo {
            id: 7,
            name: "Staff".into(),
            color: String::new(),
            permissions: common::perm::BYPASS_IGNORE,
            position: 1,
            mentionable: false,
            hoisted: false,
        }],
    }));
    let _ = app.update(net(ServerMsg::Presence {
        users: vec![
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![7] },
        ],
    }));
    // The open server remembers bob as protected; the other server does not.
    assert!(app.prefs.servers[0].protected.contains(&"id-bob".to_string()));
    // A block made on this device (the block button refuses bob here, so set it directly).
    app.prefs.blocked.push("id-bob".into());
    // Bob has Bypass ignore on this server, so the block does not apply here; the other server still hides him.
    assert!(!app.blocked_on(&addr, "id-bob"));
    assert!(app.blocked_on("other.example:7878", "id-bob"));
}

#[test]
fn offline_banner_hides_when_the_auth_service_returns() {
    let mut app = app();
    app.auth_down = true;
    app.auth_banner_closed = true;
    let _ = app.update(Message::AuthProbed(false));
    assert!(app.auth_down, "still offline: the banner stays");
    let _ = app.update(Message::AuthProbed(true));
    assert!(!app.auth_down, "back online: the banner hides");
    assert!(!app.auth_banner_closed, "a later outage shows the banner again");
}

#[test]
fn losing_the_auth_service_shows_the_banner() {
    use crate::backend::CmdError;
    let mut lost = app();
    assert!(!lost.auth_down);
    let _ = lost.update(Message::VerifyDone(Err(CmdError { message: "unreachable".into(), auth_down: true })));
    assert!(lost.auth_down, "an unreachable auth service shows the banner");

    let mut wrong = app();
    let _ = wrong.update(Message::VerifyDone(Err(CmdError::new("Wrong code."))));
    assert!(!wrong.auth_down, "an ordinary error does not show the banner");
}

#[test]
fn a_ping_marks_the_channel_only_for_members_of_that_role() {
    let mut app = app();
    welcome(&mut app);
    app.server.roles = vec![common::RoleInfo {
        id: 5,
        name: "Mods".into(),
        color: "#ff0000".into(),
        permissions: 0,
        position: 1,
        mentionable: true,
        hoisted: true,
    }];
    app.server.member_roles.insert("id-alice".into(), vec![5]);
    let _ = app.update(net(ServerMsg::Message { message: chat(2, 2, "bob", "hey @Mods look", 1_700_000_100) }));
    assert!(app.pinged.contains(&2), "a ping for a role alice holds is marked");

    let _ = app.update(net(ServerMsg::Message { message: chat(3, 3, "bob", "hey Mods, no at sign", 1_700_000_101) }));
    assert!(!app.pinged.contains(&3), "a name without @ is not a ping");

    app.server.member_roles.insert("id-alice".into(), vec![]);
    let _ = app.update(net(ServerMsg::Message { message: chat(4, 4, "bob", "hey @Mods", 1_700_000_102) }));
    assert!(!app.pinged.contains(&4), "a role alice does not hold is not marked");

    app.server.roles[0].mentionable = false;
    app.server.member_roles.insert("id-alice".into(), vec![5]);
    let _ = app.update(net(ServerMsg::Message { message: chat(5, 5, "bob", "hey @Mods", 1_700_000_103) }));
    assert!(!app.pinged.contains(&5), "a role that cannot be pinged is not marked");
}

#[test]
fn picking_from_the_at_menu_replaces_the_word_being_typed() {
    let mut app = app();
    welcome(&mut app);
    app.input = "hello there @Mo".into();
    assert_eq!(app.mention_query().as_deref(), Some("Mo"));
    let _ = app.update(Message::MentionPick("Mods".into()));
    assert_eq!(app.input, "hello there @Mods ");
    assert_eq!(app.mention_query(), None, "after a space the menu closes");
    app.input = "email me@example".into();
    assert_eq!(app.mention_query(), None, "an @ inside a word is not a mention");
}
