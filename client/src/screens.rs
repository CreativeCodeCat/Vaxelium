//! Renders the main screens headless and saves a picture of each one under `target/screens`, so the
//! layout can be checked without a display. Run with `cargo test -p client screens`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use common::{ChatMessage, DirectMessage};
use iced::Size;

use crate::app::{App, Message, MenuKind, Page};
use crate::backend::Backend;
use crate::model::Entry;
use crate::prefs::{Prefs, ServerPref};

fn shots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/screens")
}

fn app() -> App {
    let (tx, _rx) = mpsc::channel();
    let dir = std::env::temp_dir().join("vaxelium-screens");
    let backend = Backend::new(tx, dir);
    let mut prefs = Prefs::default();
    prefs.servers = vec![ServerPref { name: "Home".into(), addr: "127.0.0.1:7878".into(), icon: None }];
    App::new(backend, prefs, Vec::new(), HashMap::new())
}

fn chat_app() -> App {
    let mut app = app();
    app.user = Some("alice".into());
    app.user_id = "u1".into();
    app.selected = Some(0);
    app.main.id = Some(1);
    app.main.addr = Some("127.0.0.1:7878".into());
    app.main.welcomed = true;
    app.server.name = "Home".into();
    app.server.channels = vec!["general".into(), "random".into()];
    app.server.channel = "general".into();
    app.server.user_id = "u1".into();
    app.server.owner = Some("alice".into());
    app.server.owner_id = Some("u1".into());
    app.server.online = vec!["alice".into()];
    app.server.offline = vec!["bob".into()];
    let msg = |id: i64, author: &str, author_id: &str, text: &str, ts: u64| {
        Entry::Chat(ChatMessage {
            id,
            channel: "general".into(),
            author: author.into(),
            author_id: author_id.into(),
            text: text.into(),
            timestamp: ts,
            edited: id == 2,
            reactions: Vec::new(),
        })
    };
    let now = crate::util::now();
    let entries = vec![
        msg(1, "bob", "u2", "hello there", now - 3600),
        msg(2, "alice", "u1", "hi bob, the new client is up", now - 3500),
        msg(3, "alice", "u1", "see https://example.com for details", now - 3400),
    ];
    app.messages.insert("general".into(), entries);
    app.dm_links.insert(
        "127.0.0.1:7878".into(),
        crate::model::DmLink {
            id: 9,
            server_name: "Home".into(),
            dms: vec![DirectMessage {
                channel: "dm-1".into(),
                name: "bob".into(),
                last_message: now,
                other_id: "u2".into(),
            }],
            ..Default::default()
        },
    );
    app
}

fn save(ui: &mut iced_test::Simulator<'_, Message>, app: &App, name: &str) {
    let path = shots_dir().join(format!("{name}.png"));
    for old in std::fs::read_dir(shots_dir()).into_iter().flatten().flatten() {
        if old.file_name().to_string_lossy().starts_with(name) {
            let _ = std::fs::remove_file(old.path());
        }
    }
    std::fs::create_dir_all(shots_dir()).unwrap();
    let theme = crate::theme::iced_theme(&app.prefs.theme);
    let snap = ui.snapshot(&theme).expect("screenshot");
    // The first call writes the picture and reports it as different; that is expected here.
    let _ = snap.matches_image(&path).expect("save screenshot");

}

fn simulate(app: &App) -> iced_test::Simulator<'_, Message> {
    iced_test::Simulator::with_size(iced::Settings::default(), Size::new(1100.0, 700.0), crate::view::view(app))
}

#[test]
fn login_screen() {
    let app = app();
    let mut ui = simulate(&app);
    save(&mut ui, &app, "login");
}

#[test]
fn chat_screen() {
    let app = chat_app();
    let mut ui = simulate(&app);
    save(&mut ui, &app, "chat");
}

#[test]
fn search_filters_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::SearchOpen);
    let _ = app.update(Message::SearchChanged("from:bob".into()));
    let mut ui = simulate(&app);
    save(&mut ui, &app, "search");
}

#[test]
fn server_icon_screen() {
    use base64::Engine;
    let mut img = image::RgbaImage::new(128, 128);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgba([(x * 2) as u8, 120, (y * 2) as u8, 255]);
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let b64 = base64::engine::general_purpose::STANDARD.encode(png.into_inner());
    let mut app = chat_app();
    app.prefs.servers[0].icon = Some(b64.clone());
    app.server.icon = Some(b64);
    let mut ui = simulate(&app);
    save(&mut ui, &app, "server-icon");
}

#[test]
fn settings_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    let mut ui = simulate(&app);
    save(&mut ui, &app, "settings");
}

#[test]
fn theme_menu_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    let _ = app.update(Message::OpenMenu(MenuKind::Theme));
    let mut ui = simulate(&app);
    save(&mut ui, &app, "theme-menu");
}

#[test]
fn direct_messages_screen() {
    let mut app = chat_app();
    app.home = true;
    app.server.channel = "dm-1".into();
    app.server.dms = vec![DirectMessage { channel: "dm-1".into(), name: "bob".into(), last_message: 1, other_id: "u2".into() }];
    let mut ui = simulate(&app);
    save(&mut ui, &app, "dms");
}

#[test]
fn discover_screen() {
    let mut app = chat_app();
    app.page = Page::Discover;
    app.discover.found = vec![crate::discover::FoundServer {
        addr: "127.0.0.1:7879".into(),
        name: "Second".into(),
        description: "A second server".into(),
        icon: None,
        online: 2,
        public: true,
    }];
    let mut ui = simulate(&app);
    save(&mut ui, &app, "discover");
}

fn save_at(app: &App, name: &str, w: f32, h: f32) {
    let mut ui = iced_test::Simulator::with_size(iced::Settings::default(), Size::new(w, h), crate::view::view(app));
    save(&mut ui, app, name);
}

#[test]
fn chat_sizes_and_members() {
    let mut app = chat_app();
    save_at(&app, "chat-min", 820.0, 480.0);
    app.server.online = (0..25).map(|i| format!("member{i}")).collect();
    app.server.offline = (0..25).map(|i| format!("away{i}")).collect();
    save_at(&app, "chat-many", 1100.0, 700.0);
}

#[test]
fn chat_members_hidden() {
    let mut app = chat_app();
    app.members_hidden = true;
    save_at(&app, "chat-hidden", 1100.0, 700.0);
}

#[test]
fn search_channel_suggestions_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::SearchOpen);
    let _ = app.update(Message::SearchChanged("in:".into()));
    save_at(&app, "search-in", 1100.0, 700.0);
}

#[test]
fn search_with_text_screen() {
    let mut app = chat_app();
    app.members_hidden = false;
    let _ = app.update(Message::SearchChanged("hello".into()));
    save_at(&app, "search-text", 1100.0, 700.0);
}
