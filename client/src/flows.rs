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
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: false, owner: false, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
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
fn the_channel_edit_page_holds_the_name_and_closes_on_done() {
    let mut app = app();
    app.server.channels = vec![chan(2, "dev")];
    let _ = app.update(Message::ChannelEditOpen(2));
    assert_eq!(app.channel_edit, Some(2));
    assert_eq!(app.rename_channel, Some((2, "dev".into(), "dev".into())));
    let _ = app.update(Message::RenameInput("   ".into()));
    let _ = app.update(Message::RenameSubmit);
    assert_eq!(app.channel_edit, Some(2), "saving keeps the page open");
    let _ = app.update(Message::ChannelEditClose);
    assert_eq!(app.channel_edit, Some(2), "closing with an unsaved name is refused");
    let _ = app.update(Message::RenameReset);
    let _ = app.update(Message::ChannelEditClose);
    assert!(app.channel_edit.is_none() && app.rename_channel.is_none(), "Done closes the page once nothing is unsaved");
}

#[test]
fn the_category_edit_page_holds_the_name_and_closes_on_done() {
    let mut app = app();
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    assert_eq!(app.category_edit, Some((5, "Games".into(), "Games".into())));
    let _ = app.update(Message::CategoryEditInput("Play".into()));
    let _ = app.update(Message::CategoryEditSubmit);
    assert_eq!(app.category_edit, Some((5, "Games".into(), "Play".into())), "saving keeps the page open");
    let _ = app.update(Message::CategoryEditClose);
    assert!(app.category_edit.is_some(), "closing with an unsaved name is refused");
    let _ = app.update(Message::CategoryEditReset);
    assert_eq!(app.category_edit, Some((5, "Games".into(), "Games".into())));
    let _ = app.update(Message::CategoryEditClose);
    assert!(app.category_edit.is_none(), "Done closes the page");
}

#[test]
fn category_permissions_arrive_from_the_server_and_open_on_their_section() {
    let mut app = app();
    let _ = app.update(net(ServerMsg::CategoryOverrides {
        overrides: vec![common::CategoryOverride { category_id: 5, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None }],
    }));
    assert_eq!(app.server.category_overrides.len(), 1);
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    assert_eq!(app.edit_section, crate::app::EditSection::Permissions);
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    assert_eq!(app.edit_section, crate::app::EditSection::Overview, "opening the page starts on Overview");
}

#[test]
fn permission_changes_wait_for_the_save_bar() {
    use crate::app::AccessTarget;
    let mut app = app();
    app.server.category_overrides = vec![common::CategoryOverride { category_id: 5, role: 0, view: None, send: None, manage_channels: None, manage_messages: None }];
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    let _ = app.update(Message::AccessDraftSet { target: AccessTarget::Category(5), subject: common::Subject::Role(0), setting: common::Setting { view: Some(false), send: None, ..Default::default() } });
    assert!(app.access_dirty() && app.edit_dirty(), "the change is a draft until saved");
    assert_eq!(app.server_access(AccessTarget::Category(5), &common::Subject::Role(0)), common::Setting { view: None, send: None, ..Default::default() }, "the server is not changed yet");
    assert_eq!(app.access_value(AccessTarget::Category(5), common::Subject::Role(0)), common::Setting { view: Some(false), send: None, ..Default::default() }, "the page shows the draft");
    let _ = app.update(Message::CategoryEditClose);
    assert!(app.category_edit.is_some(), "closing with an unsaved permission is refused");
    let _ = app.update(Message::EditPageReset);
    assert!(!app.edit_dirty() && app.access_draft.is_none(), "reset drops the draft");
    let _ = app.update(Message::CategoryEditClose);
    assert!(app.category_edit.is_none(), "closes once nothing is unsaved");
}

#[test]
fn picking_roles_in_add_roles_adds_them_as_unsaved_access() {
    use crate::app::AccessTarget;
    let mut app = app();
    app.server.roles = vec![
        common::RoleInfo { id: 2, name: "Moderator".into(), color: String::new(), permissions: 0, position: 2, mentionable: false, hoisted: false },
        common::RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false },
    ];
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    let _ = app.update(Message::AccessAddOpen(crate::app::AccessTarget::Category(5)));
    let _ = app.update(Message::AccessAddToggle(common::Subject::Role(3), true));
    let _ = app.update(Message::AccessAddDone);
    assert!(app.access_add.is_none(), "Done closes the dialog");
    assert_eq!(app.access_value(AccessTarget::Category(5), common::Subject::Role(3)), common::Setting { view: Some(true), send: None, ..Default::default() }, "the role can view, as an unsaved change");
    assert!(app.access_dirty());
}

#[test]
fn syncing_a_channel_fills_its_draft_with_the_category_settings() {
    use crate::app::AccessTarget;
    let mut app = app();
    app.server.channels = vec![chan(2, "dev")];
    app.server.categories = vec![common::CategoryInfo { id: 5, name: "Games".into(), channels: vec![chan(2, "dev")] }];
    app.server.category_overrides = vec![common::CategoryOverride { category_id: 5, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None }];
    app.server.member_channel_overrides = vec![common::MemberChannelOverride { channel_id: 2, member: "u9".into(), view: Some(true), send: None, manage_channels: None, manage_messages: None }];
    let _ = app.update(Message::ChannelEditOpen(2));
    assert!(!app.channel_synced(2, 5), "the channel differs from its category before syncing");
    let _ = app.update(Message::ChannelSyncWithCategory(2));
    assert_eq!(app.access_value(AccessTarget::Channel(2), common::Subject::Role(0)), common::Setting { view: Some(false), send: None, ..Default::default() });
    assert_eq!(app.access_value(AccessTarget::Channel(2), common::Subject::Member("u9".into())), common::Setting { view: None, send: None, ..Default::default() }, "the channel's own member setting is cleared");
    assert!(app.channel_synced(2, 5) && app.access_dirty(), "synced in the draft; Save applies it");
    let _ = app.update(Message::EditPageSave);
    assert!(app.access_draft.is_none());
}

#[test]
fn a_channel_outside_a_category_has_nothing_to_sync() {
    let mut app = app();
    app.server.channels = vec![chan(3, "loose")];
    let _ = app.update(Message::ChannelEditOpen(3));
    let _ = app.update(Message::ChannelSyncWithCategory(3));
    assert!(app.access_draft.is_none() && app.category_of_channel(3).is_none());
}

#[test]
fn the_channel_add_users_or_roles_flow_works() {
    use crate::app::AccessTarget;
    let mut app = app();
    app.server.channels = vec![chan(2, "dev")];
    app.server.roles = vec![common::RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false }];
    app.server.member_ids = [("alice".to_string(), "u1".to_string())].into_iter().collect();
    let _ = app.update(Message::ChannelEditOpen(2));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    let _ = app.update(Message::AccessAddOpen(AccessTarget::Channel(2)));
    assert!(app.access_add.is_some(), "the dialog opens");
    let _ = app.update(Message::AccessAddToggle(common::Subject::Role(3), true));
    let _ = app.update(Message::AccessAddToggle(common::Subject::Member("u1".into()), true));
    let _ = app.update(Message::AccessAddDone);
    assert!(app.access_add.is_none());
    assert_eq!(app.access_value(AccessTarget::Channel(2), common::Subject::Role(3)), common::Setting { view: Some(true), send: None, ..Default::default() });
    assert_eq!(app.access_value(AccessTarget::Channel(2), common::Subject::Member("u1".into())), common::Setting { view: Some(true), send: None, ..Default::default() });
    assert!(app.edit_dirty(), "the save bar appears");
}

#[test]
fn manage_channel_given_on_one_channel_only_shows_its_menu_there() {
    let mut app = app();
    app.user_id = "me".into();
    app.server.channels = vec![chan(2, "dev"), chan(4, "ops")];
    app.server.roles = vec![common::RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false }];
    app.server.member_roles = [("me".to_string(), vec![3])].into_iter().collect();
    app.server.overrides = vec![common::ChannelOverride { channel_id: 2, role: 3, view: None, send: None, manage_channels: Some(true), manage_messages: None }];
    assert!(app.channel_permission(2, common::perm::MANAGE_CHANNELS));
    assert!(!app.channel_permission(4, common::perm::MANAGE_CHANNELS));
    let labels = |app: &App| -> Vec<String> {
        app.menu
            .as_ref()
            .map(|m| m.items.iter().filter_map(|i| match i { crate::app::MenuItem::Action { label, .. } => Some(label.clone()), _ => None }).collect())
            .unwrap_or_default()
    };
    let _ = app.update(Message::OpenMenu(crate::app::MenuKind::Channel { id: 2, name: "dev".into(), unread: false }));
    assert!(labels(&app).contains(&"Edit channel".to_string()), "the channel they manage has Edit channel");
    let _ = app.update(Message::OpenMenu(crate::app::MenuKind::Channel { id: 4, name: "ops".into(), unread: false }));
    assert!(!labels(&app).contains(&"Edit channel".to_string()), "other channels do not");
}

#[test]
fn saving_clears_the_permission_draft() {
    use crate::app::AccessTarget;
    let mut app = app();
    let _ = app.update(Message::ChannelEditOpen(2));
    let _ = app.update(Message::AccessDraftSet { target: AccessTarget::Channel(2), subject: common::Subject::Role(0), setting: common::Setting { view: Some(true), send: None, ..Default::default() } });
    let _ = app.update(Message::EditPageSave);
    assert!(app.access_draft.is_none() && !app.edit_dirty());
}

#[test]
fn a_category_that_is_deleted_closes_its_edit_page() {
    let mut app = app();
    let _ = app.update(Message::CategoryEditOpen(5, "Games".into()));
    let _ = app.update(net(ServerMsg::Categories { categories: vec![], uncategorized: vec![] }));
    assert!(app.category_edit.is_none());
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
    let _ = app.update(Message::DeleteCategoryAsk(10, "Text".into()));
    assert_eq!(app.pending_category_delete, Some((10, "Text".into())));
    let _ = app.update(Message::DeleteCategoryCancel);
    assert!(app.pending_category_delete.is_none(), "cancelling keeps the category");

    let _ = app.update(net(ServerMsg::ChannelDeleted { id: 2 }));
    assert_eq!(app.server.channels.len(), 1);
    assert!(!app.messages.contains_key(&2));
}

#[test]
fn a_category_dropped_on_the_end_zone_moves_to_the_end() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    // The owner may move channels and categories.
    app.user_id = "u1".into();
    app.server.owner_id = Some("u1".into());
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
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![5], joined: 0, status: Default::default(), badges: vec![] },
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
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![7], joined: 0, status: Default::default(), badges: vec![] },
            UserStatus { name: "carol".into(), id: "id-carol".into(), online: true, owner: false, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
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
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true, roles: vec![], joined: 0, status: Default::default(), badges: vec![] },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: true, owner: false, roles: vec![7], joined: 0, status: Default::default(), badges: vec![] },
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
fn a_too_long_message_is_kept_in_the_box_and_not_sent() {
    let mut app = app();
    welcome(&mut app);
    app.set_input("x".repeat(crate::app::MAX_MESSAGE_CHARS + 1));
    let _ = app.update(Message::Send);
    assert_eq!(app.input.chars().count(), crate::app::MAX_MESSAGE_CHARS + 1, "the text stays in the box");
    assert!(app.status.starts_with("Error:"));
}

#[test]
fn text_past_the_limit_is_cut_off_as_it_is_pasted() {
    use iced::widget::text_editor::{Action, Edit};
    use std::sync::Arc;
    let mut app = app();
    welcome(&mut app);
    let pasted = "y".repeat(crate::app::MAX_MESSAGE_CHARS + 50);
    let _ = app.update(Message::ComposerAction(Action::Edit(Edit::Paste(Arc::new(pasted)))));
    assert_eq!(app.input.chars().count(), crate::app::MAX_MESSAGE_CHARS, "the extra characters are gone");
    assert_eq!(app.composer.text().chars().count(), crate::app::MAX_MESSAGE_CHARS, "and gone from the editor");
}

#[test]
fn deleting_back_under_the_limit_in_the_composer_clears_the_warning() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = app();
    welcome(&mut app);
    app.set_input("x".repeat(crate::app::MAX_MESSAGE_CHARS + 1));
    let _ = app.update(Message::Send);
    assert!(app.status.starts_with("Error:"));
    let _ = app.update(Message::ComposerAction(Action::Edit(Edit::Backspace)));
    assert_eq!(app.input.chars().count(), crate::app::MAX_MESSAGE_CHARS, "one character removed");
    assert!(app.status.is_empty(), "status after backspace: {:?}", app.status);
}

#[test]
fn the_too_long_error_clears_once_the_message_is_short_enough() {
    let mut app = app();
    welcome(&mut app);
    app.status = format!("Error: message longer than {} characters", crate::app::MAX_MESSAGE_CHARS);
    app.set_input("x".repeat(crate::app::MAX_MESSAGE_CHARS + 1));
    assert!(app.status.starts_with("Error:"), "still too long, so the error stays");
    app.set_input("x".repeat(10));
    assert!(app.status.is_empty(), "shortened, so the error goes");
}

#[test]
fn a_message_can_span_several_lines() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = app();
    welcome(&mut app);
    let _ = app.update(Message::ComposerAction(Action::Edit(Edit::Insert('a'))));
    let _ = app.update(Message::ComposerAction(Action::Edit(Edit::Enter)));
    let _ = app.update(Message::ComposerAction(Action::Edit(Edit::Insert('b'))));
    assert_eq!(app.input, "a\nb", "Enter starts a new line in the composer");
    assert_eq!(app.composer.line_count(), 2);
    // Setting the text again (after a send, or a picked mention) keeps every line.
    app.set_input("one\ntwo".into());
    assert_eq!(app.composer.line_count(), 2);
}

#[test]
fn an_edited_message_keeps_its_line_breaks() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = app();
    welcome(&mut app);
    app.editing = Some((7, "a".into()));
    app.edit_box = crate::app::editor_at_end("a");
    let _ = app.update(Message::EditAction(Action::Edit(Edit::Enter)));
    let _ = app.update(Message::EditAction(Action::Edit(Edit::Insert('b'))));
    assert_eq!(app.editing.as_ref().map(|(_, t)| t.as_str()), Some("a\nb"));
}

#[test]
fn picking_from_the_at_menu_replaces_the_word_being_typed() {
    let mut app = app();
    welcome(&mut app);
    app.set_input("hello there @Mo".into());
    assert_eq!(app.mention_query().as_deref(), Some("Mo"));
    let _ = app.update(Message::MentionPick("Mods".into()));
    assert_eq!(app.input, "hello there @Mods ");
    assert_eq!(app.mention_query(), None, "after a space the menu closes");
    app.set_input("email me@example".into());
    assert_eq!(app.mention_query(), None, "an @ inside a word is not a mention");
}

#[test]
fn a_member_card_loads_for_the_member_clicked() {
    use crate::app::CardView;
    use crate::backend::{Card, CmdError};
    let mut app = app();
    welcome(&mut app);
    app.card = Some(CardView { id: "id-bob".into(), name: "bob".into(), at: iced::Point::new(300.0, 200.0), loading: true, card: None, error: None, above: false });
    let _ = app.update(Message::CardLoaded("id-other".into(), Ok(Card { name: "x".into(), username: "x".into(), bio: String::new(), banner: String::new(), avatar: String::new(), status: String::new(), created: 0, mutual: vec![] })));
    assert!(app.card.as_ref().is_some_and(|c| c.loading), "a reply for another member is ignored");
    let _ = app.update(Message::CardLoaded("id-bob".into(), Ok(Card { name: "bob".into(), username: "bob".into(), bio: "hi".into(), banner: String::new(), avatar: String::new(), status: String::new(), created: 0, mutual: vec![] })));
    assert!(app.card.as_ref().is_some_and(|c| !c.loading && c.card.is_some()));
    let _ = app.update(Message::CardLoaded("id-bob".into(), Err(CmdError::new("nope"))));
    let _ = app.update(Message::CardClose);
    assert!(app.card.is_none());
}

#[test]
fn bio_is_limited_to_190_characters() {
    let mut app = app();
    let _ = app.update(Message::UserSectionPick(crate::app::UserSection::Profile));
    let _ = app.update(Message::BioEdit(iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(std::sync::Arc::new("a".repeat(300))))));
    assert_eq!(app.account.about_bio.chars().count(), 190);
}

#[test]
fn a_saved_picture_or_its_removal_is_kept() {
    use crate::backend::CmdError;
    let mut app = app();
    let _ = app.update(Message::AvatarDone(Ok("aGVsbG8=".into())));
    assert_eq!(app.account.avatar, "aGVsbG8=");
    let _ = app.update(Message::AvatarDone(Err(CmdError::new("too large"))));
    assert_eq!(app.account.avatar, "aGVsbG8=", "a failed save keeps the saved picture");
    assert_eq!(app.account.avatar_error.as_deref(), Some("too large"));
    let _ = app.update(Message::AvatarDone(Ok(String::new())));
    assert!(app.account.avatar.is_empty());
}

#[test]
fn fetched_profile_looks_are_kept_for_members_and_messages() {
    use crate::backend::Card;
    let mut app = app();
    welcome(&mut app);
    app.server.member_ids.insert("bob".into(), "id-bob".into());
    app.profile_pending.insert("id-bob".into());
    let card = Card { name: "bob".into(), username: "bob".into(), bio: String::new(), banner: "#2ecc71".into(), avatar: String::new(), status: String::new(), created: 0, mutual: vec![] };
    let _ = app.update(Message::ProfilesLoaded(vec![("id-bob".into(), Some(card))]));
    assert!(!app.profile_pending.contains("id-bob"));
    assert_eq!(app.member_id_look("bob"), ("", "#2ecc71"));
    assert_eq!(app.profile_look("id-bob"), ("", "#2ecc71"));
    assert_eq!(app.profile_look("id-nobody"), ("", ""), "someone not fetched has no picture");
}

#[test]
fn a_picked_picture_is_a_draft_until_saved_or_reset() {
    let mut app = app();
    let _ = app.update(Message::AvatarPicked(Ok(Some("aGVsbG8=".into()))));
    assert_eq!(app.account.avatar_draft.as_deref(), Some("aGVsbG8="));
    assert!(app.account.avatar.is_empty(), "nothing is saved yet");
    let _ = app.update(Message::AboutReset);
    assert!(app.account.avatar_draft.is_none(), "Reset drops the draft");
    let _ = app.update(Message::AvatarRemove);
    assert_eq!(app.account.avatar_draft.as_deref(), Some(""), "a removal is a draft too");
}

#[test]
fn closing_settings_with_unsaved_changes_is_refused_and_flashes() {
    let mut app = app();
    app.page = crate::app::Page::User;
    app.user_section = crate::app::UserSection::Profile;
    app.account.about_bio = "changed".into();
    let _ = app.update(Message::UserSettingsClose);
    assert!(matches!(app.page, crate::app::Page::User), "the page stays open");
    assert!(app.unsaved_flash_on(), "the save bar flashes");
    let _ = app.update(Message::AboutReset);
    let _ = app.update(Message::UserSettingsClose);
    assert!(matches!(app.page, crate::app::Page::None), "after Reset the page closes");
}

#[test]
fn escape_over_unsaved_settings_flashes_instead_of_closing() {
    let mut app = app();
    app.page = crate::app::Page::User;
    app.user_section = crate::app::UserSection::Account;
    app.account.draft_name = Some("New name".into());
    let _ = app.on_escape();
    assert!(matches!(app.page, crate::app::Page::User));
    assert!(app.unsaved_flash_on());
}

#[test]
fn a_channel_manager_can_drag_only_the_channels_they_manage() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    app.user_id = "me".into();
    app.server.channels = vec![chan(2, "dev"), chan(4, "ops")];
    app.server.roles = vec![common::RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false }];
    app.server.member_roles = [("me".to_string(), vec![3])].into_iter().collect();
    app.server.overrides = vec![common::ChannelOverride { channel_id: 2, role: 3, view: None, send: None, manage_channels: Some(true), manage_messages: None }];
    let target = Some(DropTarget::Uncategorized);
    let _ = app.update(Message::DndPress(DragItem::Channel(2)));
    let _ = app.update(Message::DndOver(target));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), target, "the channel they manage can be dropped");
    let _ = app.update(Message::DndPress(DragItem::Channel(4)));
    let _ = app.update(Message::DndOver(target));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), None, "a channel they do not manage cannot");
}

#[test]
fn leaving_a_category_needs_manage_channel_in_that_category() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    app.user_id = "me".into();
    app.server.channels = vec![chan(2, "dev"), chan(9, "ops")];
    app.server.categories = vec![common::CategoryInfo { id: 5, name: "Games".into(), channels: vec![chan(2, "dev"), chan(9, "ops")] }];
    app.server.roles = vec![common::RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false }];
    app.server.member_roles = [("me".to_string(), vec![3])].into_iter().collect();
    app.server.overrides = vec![common::ChannelOverride { channel_id: 2, role: 3, view: None, send: None, manage_channels: Some(true), manage_messages: None }];
    let _ = app.update(Message::DndPress(DragItem::Channel(2)));
    // Moving within the category needs only the channel.
    let inside = Some(DropTarget::Channel { id: 9, category: Some(5) });
    let _ = app.update(Message::DndOver(inside));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), inside, "reordering inside the category is allowed");
    // Moving it out does not, without Manage channel in the category.
    let _ = app.update(Message::DndOver(Some(DropTarget::Uncategorized)));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), None, "leaving the category is blocked");
    // With Manage channel in the category, it can leave.
    app.server.category_overrides = vec![common::CategoryOverride { category_id: 5, role: 3, view: None, send: None, manage_channels: Some(true), manage_messages: None }];
    let _ = app.update(Message::DndOver(Some(DropTarget::Uncategorized)));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), Some(DropTarget::Uncategorized), "leaving is allowed with it");
    // Joining another category needs Manage channel in that one too.
    app.server.categories.push(common::CategoryInfo { id: 6, name: "Other".into(), channels: Vec::new() });
    let into_other = Some(DropTarget::Category(6));
    let _ = app.update(Message::DndOver(into_other));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), None, "joining a category without the permission is blocked");
    app.server.category_overrides.push(common::CategoryOverride { category_id: 6, role: 3, view: None, send: None, manage_channels: Some(true), manage_messages: None });
    let _ = app.update(Message::DndOver(into_other));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), into_other, "joining is allowed with it");
}

#[test]
fn a_member_without_manage_channels_gets_no_drop_targets_and_is_told_why() {
    use crate::app::{DragItem, DropTarget};
    let mut app = app();
    app.user_id = "u2".into();
    app.server.owner_id = Some("u1".into());
    let _ = app.update(net(ServerMsg::Categories {
        categories: vec![common::CategoryInfo { id: 1, name: "One".into(), channels: Vec::new() }],
        uncategorized: Vec::new(),
    }));
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 100.0)));
    let _ = app.update(Message::DndPress(DragItem::Category(1)));
    let _ = app.update(Message::CursorMoved(Point::new(100.0, 200.0)));
    let _ = app.update(Message::DndOver(Some(DropTarget::End)));
    assert_eq!(app.dnd.as_ref().and_then(|d| d.over), None, "no drop target without the permission");
    let _ = app.update(Message::MouseReleased);
    assert!(app.status.contains("Manage channels"), "status: {}", app.status);
}
