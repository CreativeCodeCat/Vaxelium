//! Checks the app's state changes when servers send messages, without a network or a window.

use std::collections::HashMap;
use std::sync::mpsc;

use common::{ChannelRef, ChatMessage, ServerMsg, UserStatus};
use iced::Point;

use crate::app::{App, Message, MenuItem};
use crate::backend::{Backend, Event};
use crate::prefs::{Prefs, ServerPref};

fn app() -> App {
    let (tx, _rx) = mpsc::channel();
    let backend = Backend::new(tx, std::env::temp_dir().join("vaxelium-flows"));
    let mut prefs = Prefs::default();
    prefs.servers = vec![ServerPref { name: "Home".into(), addr: "127.0.0.1:7878".into(), icon: None, id: None }];
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
            UserStatus { name: "alice".into(), id: "id-alice".into(), online: true, owner: true },
            UserStatus { name: "bob".into(), id: "id-bob".into(), online: false, owner: false },
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
