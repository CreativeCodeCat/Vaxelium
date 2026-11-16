//! Renders the main screens headless and saves a picture of each one under `target/screens`, so the
//! layout can be checked without a display. Run with `cargo test -p client screens`.

use std::collections::HashMap;
use std::path::PathBuf;

use common::{ChatMessage, DirectMessage};
use iced::Size;

use crate::app::{App, MenuKind, Message, Page};
use crate::backend::Backend;
use crate::model::Entry;
use crate::prefs::{Prefs, ServerPref};

fn shots_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/screens")
}

fn app() -> App {
    let (tx, _rx) = crate::backend::event_channel();
    let dir = std::env::temp_dir().join("vaxelium-screens");
    let backend = Backend::new(tx, dir);
    let mut prefs = Prefs::default();
    prefs.servers = vec![ServerPref {
        name: "Home".into(),
        addr: "127.0.0.1:7878".into(),
        icon: None,
        id: None,
        ignored: Vec::new(),
        protected: Vec::new(),
        linked: false,
    }];
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
    app.server.channels = vec![
        common::ChannelRef {
            id: 1,
            name: "general".into(),
        },
        common::ChannelRef {
            id: 2,
            name: "random".into(),
        },
    ];
    app.server.channel = "general".into();
    app.server.channel_id = 1;
    app.server.user_id = "u1".into();
    app.server.owner = Some("alice".into());
    app.server.owner_id = Some("u1".into());
    app.server.online = vec!["alice".into()];
    app.server.offline = vec!["bob".into()];
    let msg = |id: i64, author: &str, author_id: &str, text: &str, ts: u64| {
        Entry::Chat(ChatMessage {
            id,
            channel_id: 1,
            channel: "general".into(),
            author: author.into(),
            author_id: author_id.into(),
            text: text.into(),
            timestamp: ts,
            edited: id == 2,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })
    };
    let now = crate::util::now();
    let entries = vec![
        msg(1, "bob", "u2", "hello there", now - 3600),
        msg(2, "alice", "u1", "hi bob, the new client is up", now - 3500),
        msg(3, "alice", "u1", "see https://example.com for details", now - 3400),
    ];
    app.messages.insert(1, entries);
    app.dm_links.insert(
        "127.0.0.1:7878".into(),
        crate::model::DmLink {
            id: 9,
            server_name: "Home".into(),
            dms: vec![DirectMessage {
                channel_id: 5,
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
    // Only this picture's own files go, not those of a longer name that starts the same way.
    for own in [
        format!("{name}.png"),
        format!("{name}-wgpu.png"),
        format!("{name}-tiny-skia.png"),
    ] {
        let _ = std::fs::remove_file(shots_dir().join(own));
    }
    std::fs::create_dir_all(shots_dir()).unwrap();
    let theme = crate::theme::iced_theme(&app.prefs.theme);
    let snap = ui.snapshot(&theme).expect("screenshot");
    // The first call writes the picture and reports it as different; that is expected here.
    let _ = snap.matches_image(&path).expect("save screenshot");
}

/// A mouse drag: press at the first point, move through the rest, release at the last. Each step is
/// sent to a freshly built view and its messages are applied to the app, as the running app does.
fn drag(app: &mut App, points: &[(f32, f32)]) {
    use iced::mouse::{Button, Event as Mouse};
    let (&first, rest) = points.split_first().expect("a drag has a start");
    let mut steps: Vec<(iced::Point, iced::Event)> = vec![(
        iced::Point::new(first.0, first.1),
        iced::Event::Mouse(Mouse::ButtonPressed(Button::Left)),
    )];
    for &(x, y) in rest {
        steps.push((
            iced::Point::new(x, y),
            iced::Event::Mouse(Mouse::CursorMoved {
                position: iced::Point::new(x, y),
            }),
        ));
    }
    let (last, _) = *steps.last().expect("at least the press");
    steps.push((last, iced::Event::Mouse(Mouse::ButtonReleased(Button::Left))));
    for (at, event) in steps {
        let messages = {
            let mut ui = simulate(app);
            ui.point_at((at.x, at.y));
            ui.simulate([event]);
            ui.into_messages()
        };
        for message in messages {
            let _ = app.update(message);
        }
    }
}

fn simulate(app: &App) -> iced_test::Simulator<'_, Message> {
    iced_test::Simulator::with_size(
        iced::Settings::default(),
        Size::new(1100.0, 700.0),
        crate::view::view(app),
    )
}

#[test]
fn login_screen() {
    let app = app();
    let mut ui = simulate(&app);
    save(&mut ui, &app, "login");
}

#[test]
fn login_email_screen() {
    let mut app = app();
    app.login.email_open = true;
    let mut ui = simulate(&app);
    save(&mut ui, &app, "login-email");
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
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
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
    app.server.dms = vec![DirectMessage {
        channel_id: 5,
        channel: "dm-1".into(),
        name: "bob".into(),
        last_message: 1,
        other_id: "u2".into(),
    }];
    let mut ui = simulate(&app);
    save(&mut ui, &app, "dms");
}

#[test]
fn discover_screen() {
    let mut app = chat_app();
    app.page = Page::Discover;
    app.discover.found = vec![crate::discover::FoundServer {
        server_id: "server-2".into(),
        addr: "127.0.0.1:7879".into(),
        name: "Second".into(),
        description: "A second server".into(),
        icon: None,
        online: 2,
        public: true,
        self_signed: true,
    }];
    let mut ui = simulate(&app);
    save(&mut ui, &app, "discover");
}

fn save_at(app: &App, name: &str, w: f32, h: f32) {
    let settings = iced::Settings {
        fonts: vec![include_bytes!("../../ui/fonts/NotoSans-Subset.ttf").as_slice().into()],
        default_font: iced::Font::with_name("Noto Sans"),
        ..iced::Settings::default()
    };
    let mut ui = iced_test::Simulator::with_size(settings, Size::new(w, h), crate::view::view(app));
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

#[test]
fn server_profile_multiline_screen() {
    let mut app = chat_app();
    app.server.description = "First line of the description.\nSecond line, which is longer and keeps going.".into();
    let _ = app.update(Message::ServerSettingsOpen);
    save_at(&app, "server-profile", 1100.0, 700.0);
}

#[test]
fn categories_screen() {
    let mut app = chat_app();
    app.server.channels = vec![
        common::ChannelRef {
            id: 1,
            name: "general".into(),
        },
        common::ChannelRef {
            id: 2,
            name: "random".into(),
        },
        common::ChannelRef {
            id: 3,
            name: "dev".into(),
        },
    ];
    app.server.categories = vec![common::CategoryInfo {
        id: 1,
        name: "Voice and text".into(),
        channels: vec![
            common::ChannelRef {
                id: 3,
                name: "dev".into(),
            },
            common::ChannelRef {
                id: 2,
                name: "random".into(),
            },
        ],
    }];
    save_at(&app, "categories", 1100.0, 700.0);
}

#[test]
fn markdown_message_screen() {
    let mut app = chat_app();
    let now = crate::util::now();
    let body = "## Release notes\n**Bold**, *italic*, ~~gone~~ and `inline code`.\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n> Quoted advice, see https://example.com\n\n1. first\n2. second\n   - nested\n\n| name | role |\n|---|---|\n| alice | owner |\n| bob | member |\n\n---\n[a link](https://example.com/docs) and <b>raw html</b>";
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 11,
            channel_id: 1,
            channel: "general".into(),
            author: "alice".into(),
            author_id: "u1".into(),
            text: body.into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "markdown-message", 1100.0, 900.0);
}

#[test]
fn composer_multiline_screen() {
    let mut app = chat_app();
    app.set_input("first line of a longer message\nsecond line\nthird line".into());
    save_at(&app, "composer-multiline", 1100.0, 700.0);
}

#[test]
fn math_message_screen() {
    let mut app = chat_app();
    let now = crate::util::now();
    let body = "Inline maths: $E = mc^2$, and $x_1 + x_{10} \\leq \\infty$.\n\n$$\n\\frac{a+b}{c} + \\sqrt{x^2 + 1} = \\sum_{i=1}^{n} i\n$$\n\nBlock maths: $$a^2 + b^2 = c^2$$";
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 12,
            channel_id: 1,
            channel: "general".into(),
            author: "alice".into(),
            author_id: "u1".into(),
            text: body.into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "math-message", 1100.0, 800.0);
}

#[test]
fn math_layout_screen() {
    let mut app = chat_app();
    let now = crate::util::now();
    let body = "Inline: $\\sqrt[3]{8} = 2$, $x^{q}$, $\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$\n\n$$\n\\left( \\frac{a}{b} \\right) + \\sqrt[3]{x+1} + \\begin{bmatrix} 1 & 0 \\\\ 0 & 1 \\end{bmatrix}\n$$\n\n$$\nf(x) = \\begin{cases} x & x \\geq 0 \\\\ -x & x < 0 \\end{cases}\n$$";
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 16,
            channel_id: 1,
            channel: "general".into(),
            author: "alice".into(),
            author_id: "u1".into(),
            text: body.into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "math-layout", 1100.0, 800.0);
}

#[test]
fn dragging_across_message_text_selects_it() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 13,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "Select some **bold** words here".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    // The pointer moves as a real drag does: press here, then move and release there. The messages the
    // widgets send are then applied to the app, which draws the selection.
    drag(&mut app, &[(440.0, 240.0), (560.0, 240.0)]);
    save_at(&app, "text-selection", 1100.0, 700.0);
}

#[test]
fn click_away_clears_selection() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 15,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "Select some **bold** words here".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    drag(&mut app, &[(440.0, 240.0), (560.0, 240.0)]);
    // The window's release event comes through a subscription, which the simulator does not run.
    let _ = app.update(Message::MouseReleased);
    assert!(app.text_sel.is_some(), "the drag should select text");

    // A press on other text in the same message starts a new selection rather than clearing it.
    drag(&mut app, &[(460.0, 240.0)]);
    let _ = app.update(Message::MouseReleased);
    assert!(app.text_sel.is_some(), "a press on the text starts a new selection");

    drag(&mut app, &[(700.0, 450.0)]);
    assert!(
        app.text_sel.is_none(),
        "a click away from the text should clear the selection"
    );
}

#[test]
fn clicking_a_link_in_a_message_still_opens_it() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 14,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "see https://example.com for details".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    let mut ui = simulate(&app);
    ui.point_at((520.0, 240.0));
    ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
    ]);
    let opened = ui
        .into_messages()
        .into_iter()
        .any(|m| matches!(m, Message::OpenLink(ref url) if url == "https://example.com"));
    assert!(opened, "a click on the link should open it");
}

#[test]
fn dragging_over_several_lines_selects_them() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 15,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "first line of text\nsecond line\nthird line".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    drag(&mut app, &[(440.0, 240.0), (700.0, 275.0)]);
    save_at(&app, "multiline-selection", 1100.0, 700.0);
}

#[test]
fn dragging_across_markdown_blocks_selects_them() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 16,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "First paragraph here\n\n- item one\n- item two\n\nLast paragraph".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    drag(&mut app, &[(440.0, 240.0), (520.0, 312.0)]);
    save_at(&app, "markdown-selection", 1100.0, 700.0);
}

#[test]
fn html_elements_message_screen() {
    let mut app = chat_app();
    let now = crate::util::now();
    let body = "9. HTML Elements\n<details>\n<summary>Click to expand</summary>\n\nThis is hidden expandable content.\n</details>\n\n<kbd>Ctrl</kbd> + <kbd>C</kbd>\n<mark>Highlighted text</mark>";
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 17,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: body.into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "html-elements", 1100.0, 700.0);
}

#[test]
fn clicking_a_details_summary_opens_it() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 18,
            channel_id: 1,
            channel: "general".into(),
            author: "bob".into(),
            author_id: "u2".into(),
            text: "<details>\n<summary>Click to expand</summary>\n\nThis is hidden expandable content.\n</details>"
                .into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    // A click on the summary, which sits at the top of the message. A button needs the press and the
    // release in the same view, so both are sent in one step.
    let messages = {
        let mut ui = simulate(&app);
        ui.point_at((470.0, 240.0));
        ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        ]);
        ui.into_messages()
    };
    for message in messages {
        let _ = app.update(message);
    }
    assert!(app.details_flipped.contains(&(18, 0)), "the click opened the section");
    save_at(&app, "details-open", 1100.0, 700.0);
}

#[test]
fn small_caps_screen() {
    let mut app = chat_app();
    let now = crate::util::now();
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 9,
            channel_id: 1,
            channel: "general".into(),
            author: "alice".into(),
            author_id: "u1".into(),
            text: "`| ====== ᴍᴀɪɴ ====== |`".into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "small-caps", 1100.0, 700.0);
}

#[test]
fn collapsed_category_screen() {
    let mut app = chat_app();
    app.server.channels = vec![
        common::ChannelRef {
            id: 1,
            name: "general".into(),
        },
        common::ChannelRef {
            id: 2,
            name: "random".into(),
        },
    ];
    app.server.categories = vec![common::CategoryInfo {
        id: 7,
        name: "Community".into(),
        channels: vec![common::ChannelRef {
            id: 3,
            name: "dev".into(),
        }],
    }];
    app.collapsed_categories.insert(7);
    save_at(&app, "collapsed", 1100.0, 700.0);
}

#[test]
fn channel_edit_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::ChannelEditOpen(1));
    save_at(&app, "channel-edit", 1100.0, 700.0);
}

#[test]
fn channel_permissions_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo {
            id: 1,
            name: "Admin".into(),
            color: "#e91e63".into(),
            permissions: common::perm::ALL,
            position: 3,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 2,
            name: "Moderator".into(),
            color: "#2ecc71".into(),
            permissions: common::perm::MANAGE_MESSAGES,
            position: 2,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 0,
            name: "@everyone".into(),
            color: "#95a5a6".into(),
            permissions: 0,
            position: -1_000_000_000,
            mentionable: false,
            hoisted: false,
        },
    ];
    let _ = app.update(Message::ChannelEditOpen(1));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "channel-permissions", 1100.0, 700.0);
}

#[test]
fn category_unsaved_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::CategoryEditInput("Changed".into()));
    save_at(&app, "category-unsaved", 1100.0, 700.0);
}

#[test]
fn channel_unsaved_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::ChannelEditOpen(1));
    let _ = app.update(Message::RenameInput("changed".into()));
    save_at(&app, "channel-unsaved", 1100.0, 700.0);
}

#[test]
fn category_access_unsaved_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    let _ = app.update(Message::AccessDraftSet {
        target: crate::app::AccessTarget::Category(1),
        subject: common::Subject::Role(0),
        setting: common::Setting {
            view: Some(false),
            send: None,
            ..Default::default()
        },
    });
    save_at(&app, "category-access-unsaved", 1100.0, 700.0);
}

#[test]
fn access_add_roles_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo {
            id: 2,
            name: "Moderator".into(),
            color: "#2ecc71".into(),
            permissions: 0,
            position: 2,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 3,
            name: "Helper".into(),
            color: "#e91e63".into(),
            permissions: 0,
            position: 1,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 0,
            name: "@everyone".into(),
            color: "#95a5a6".into(),
            permissions: 0,
            position: -1_000_000_000,
            mentionable: false,
            hoisted: false,
        },
    ];
    app.server.member_ids = [
        ("alice".to_string(), "u1".to_string()),
        ("bob".to_string(), "u2".to_string()),
    ]
    .into_iter()
    .collect();
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    let _ = app.update(Message::AccessAddOpen(crate::app::AccessTarget::Category(1)));
    let _ = app.update(Message::AccessAddToggle(common::Subject::Role(3), true));
    save_at(&app, "category-add-roles", 1100.0, 700.0);
}

#[test]
fn channel_sync_screen() {
    let mut app = chat_app();
    app.server.categories = vec![common::CategoryInfo {
        id: 7,
        name: "Community".into(),
        channels: vec![common::ChannelRef {
            id: 1,
            name: "general".into(),
        }],
    }];
    app.server.category_overrides = vec![common::CategoryOverride {
        category_id: 7,
        role: 0,
        view: Some(false),
        send: None,
        manage_channels: None,
        manage_messages: None,
    }];
    let _ = app.update(Message::ChannelEditOpen(1));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "channel-sync", 1100.0, 700.0);
}

#[test]
fn access_lists_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo {
            id: 2,
            name: "Moderator".into(),
            color: "#2ecc71".into(),
            permissions: 0,
            position: 2,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 0,
            name: "@everyone".into(),
            color: "#95a5a6".into(),
            permissions: 0,
            position: -1_000_000_000,
            mentionable: false,
            hoisted: false,
        },
    ];
    app.server.member_ids = [("alice".to_string(), "u1".to_string())].into_iter().collect();
    app.server.category_overrides = vec![
        common::CategoryOverride {
            category_id: 1,
            role: 0,
            view: Some(false),
            send: None,
            manage_channels: None,
            manage_messages: None,
        },
        common::CategoryOverride {
            category_id: 1,
            role: 2,
            view: Some(true),
            send: Some(true),
            manage_channels: None,
            manage_messages: None,
        },
    ];
    app.server.member_category_overrides = vec![common::MemberCategoryOverride {
        category_id: 1,
        member: "u1".into(),
        view: Some(true),
        send: None,
        manage_channels: None,
        manage_messages: None,
    }];
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "access-lists", 1100.0, 700.0);
}

#[test]
fn category_private_locked_screen() {
    let mut app = chat_app();
    app.server.category_overrides = vec![common::CategoryOverride {
        category_id: 1,
        role: 0,
        view: Some(false),
        send: None,
        manage_channels: None,
        manage_messages: None,
    }];
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "category-private-locked", 1100.0, 700.0);
}

#[test]
fn category_permissions_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo {
            id: 2,
            name: "Moderator".into(),
            color: "#2ecc71".into(),
            permissions: common::perm::MANAGE_MESSAGES,
            position: 2,
            mentionable: false,
            hoisted: false,
        },
        common::RoleInfo {
            id: 0,
            name: "@everyone".into(),
            color: "#95a5a6".into(),
            permissions: 0,
            position: -1_000_000_000,
            mentionable: false,
            hoisted: false,
        },
    ];
    app.server.category_overrides = vec![
        common::CategoryOverride {
            category_id: 1,
            role: 0,
            view: Some(false),
            send: None,
            manage_channels: None,
            manage_messages: None,
        },
        common::CategoryOverride {
            category_id: 1,
            role: 2,
            view: Some(true),
            send: Some(true),
            manage_channels: None,
            manage_messages: None,
        },
    ];
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "category-permissions", 1100.0, 700.0);
}

#[test]
fn category_edit_screen() {
    let mut app = chat_app();
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    save_at(&app, "category-edit", 1100.0, 700.0);
}

#[test]
fn unsaved_bar_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.account.draft_name = Some("New name".into());
    save_at(&app, "unsaved", 1100.0, 700.0);
}

#[test]
fn dragging_category_screen() {
    use crate::app::{Dnd, DragItem, DropTarget};
    let mut app = chat_app();
    app.server.channels = vec![common::ChannelRef {
        id: 1,
        name: "general".into(),
    }];
    app.server.categories = vec![
        common::CategoryInfo {
            id: 1,
            name: "Community".into(),
            channels: Vec::new(),
        },
        common::CategoryInfo {
            id: 2,
            name: "Games".into(),
            channels: Vec::new(),
        },
    ];
    app.dnd = Some(Dnd {
        item: DragItem::Category(1),
        start: iced::Point::ORIGIN,
        moved: true,
        over: Some(DropTarget::End),
    });
    save_at(&app, "dragging-category", 1100.0, 700.0);
}

fn with_roles(app: &mut App) {
    use common::{perm, RoleInfo};
    app.server.roles = vec![
        RoleInfo {
            id: 1,
            name: "Admin".into(),
            color: "#e91e63".into(),
            permissions: perm::ALL,
            position: 3,
            mentionable: false,
            hoisted: false,
        },
        RoleInfo {
            id: 2,
            name: "Moderator".into(),
            color: "#2ecc71".into(),
            permissions: perm::MANAGE_MESSAGES,
            position: 2,
            mentionable: false,
            hoisted: false,
        },
        RoleInfo {
            id: 3,
            name: "Helper".into(),
            color: String::new(),
            permissions: 0,
            position: 1,
            mentionable: false,
            hoisted: false,
        },
        RoleInfo {
            id: 0,
            name: "@everyone".into(),
            color: "#95a5a6".into(),
            permissions: 0,
            position: -1_000_000_000,
            mentionable: false,
            hoisted: false,
        },
    ];
    app.server.online = vec!["alice".into(), "bob".into(), "carol".into(), "erin".into()];
    app.server.offline = vec!["dan".into()];
    app.server.member_ids = [
        ("alice", "u1"),
        ("bob", "u2"),
        ("carol", "u3"),
        ("erin", "u5"),
        ("dan", "u4"),
    ]
    .into_iter()
    .map(|(n, i)| (n.to_string(), i.to_string()))
    .collect();
    app.server.member_roles = [("u2".to_string(), vec![2]), ("u5".to_string(), vec![1, 3])]
        .into_iter()
        .collect();
}

#[test]
fn members_grouped_by_role_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    save_at(&app, "members-roles", 1100.0, 700.0);
}

#[test]
fn roles_page_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::Roles));
    let _ = app.update(Message::RoleEdit(Some(2)));
    save_at(&app, "roles-page", 1100.0, 900.0);
}

#[test]
fn chat_names_by_role_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    save_at(&app, "chat-names", 1100.0, 700.0);
}

#[test]
fn roles_empty_state_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::Roles));
    save_at(&app, "roles-empty", 1100.0, 700.0);
}

#[test]
fn roles_unsaved_bar_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::Roles));
    let _ = app.update(Message::RoleEdit(Some(2)));
    let _ = app.update(Message::RoleDraftName("Mod Team".into()));
    save_at(&app, "roles-unsaved", 1100.0, 700.0);
}

#[test]
fn member_roles_checklist_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::OpenMenu(MenuKind::User {
        name: "bob".into(),
        can_message: true,
    }));
    let _ = app.update(Message::RolesMenuOpen);
    save_at(&app, "member-roles-menu", 1100.0, 700.0);
}

#[test]
fn member_roles_flyout_left_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.cursor = iced::Point::new(900.0, 300.0);
    let _ = app.update(Message::OpenMenu(MenuKind::User {
        name: "bob".into(),
        can_message: true,
    }));
    let _ = app.update(Message::RolesMenuOpen);
    save_at(&app, "member-roles-left", 1100.0, 700.0);
}

#[test]
fn verify_email_dialog_screen() {
    let mut app = chat_app();
    app.account.email = Some("alice@example.com".into());
    app.account.verify_open = true;
    app.account.verify_error = Some("that code is wrong".into());
    save_at(&app, "verify-email", 1100.0, 700.0);
}

#[test]
fn account_auth_unavailable_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.account.username = None;
    app.account.email = None;
    app.account.email_error = Some("The account service is offline right now. Try again in a little while.".into());
    save_at(&app, "account-auth-unavailable", 1100.0, 700.0);
}

#[test]
fn role_display_toggles_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.server.roles[1].hoisted = true;
    app.server.roles[1].mentionable = true;
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::Roles));
    let _ = app.update(Message::RoleEdit(Some(2)));
    save_at(&app, "role-display-toggles", 1100.0, 900.0);
}

#[test]
fn at_mention_menu_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.server.roles[1].mentionable = true;
    app.set_input("hey @".into());
    save_at(&app, "at-mention-menu", 1100.0, 700.0);
}

#[test]
fn linked_server_header_screen() {
    let mut app = chat_app();
    app.prefs.servers[0].linked = true;
    save_at(&app, "linked-server-header", 1100.0, 500.0);
}

#[test]
fn long_server_name_header_screen() {
    let mut app = chat_app();
    app.prefs.servers[0].linked = true;
    app.prefs.servers[0].name = "HeCodes2Much Support Community Hub".into();
    app.server.name = app.prefs.servers[0].name.clone();
    save_at(&app, "long-server-name-header", 1100.0, 500.0);
}

#[test]
fn member_profile_card_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.server.member_ids.insert("bob".into(), "id-bob".into());
    app.server.member_roles.insert("id-bob".into(), vec![1, 2]);
    app.server
        .member_status
        .insert("id-bob".into(), common::PresenceStatus::Online);
    app.server.badges = vec![
        common::Badge {
            id: 1,
            name: "Early Adopter".into(),
            color: "#f1c40f".into(),
        },
        common::Badge {
            id: 2,
            name: "Helper".into(),
            color: "#2ecc71".into(),
        },
    ];
    app.server.member_badges.insert("id-bob".into(), vec![1]);
    app.card = Some(crate::app::CardView {
        id: "id-bob".into(),
        name: "bob".into(),
        at: iced::Point::new(700.0, 180.0),
        loading: false,
        card: Some(crate::backend::Card {
            name: "bob".into(),
            username: "bob".into(),
            bio: "I Use Arch BTW. Maintainer of the Discord bot.".into(),
            banner: "#5865f2".into(),
            avatar: String::new(),
            status: "CodeWorks Creative Hub".into(),
            created: 0,
            mutual: vec!["10.0.0.1:7878".into(), "10.0.0.2:7878".into()],
        }),
        error: None,
        above: false,
    });
    save_at(&app, "member-profile-card", 1100.0, 700.0);
}

#[test]
fn bio_formatting_card_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.server.member_ids.insert("bob".into(), "id-bob".into());
    app.server.member_roles.insert("id-bob".into(), vec![1, 2]);
    app.server
        .member_status
        .insert("id-bob".into(), common::PresenceStatus::Online);
    app.server.badges = vec![
        common::Badge {
            id: 1,
            name: "Early Adopter".into(),
            color: "#f1c40f".into(),
        },
        common::Badge {
            id: 2,
            name: "Helper".into(),
            color: "#2ecc71".into(),
        },
    ];
    app.server.member_badges.insert("id-bob".into(), vec![1]);
    app.card = Some(crate::app::CardView {
        id: "id-bob".into(),
        name: "bob".into(),
        at: iced::Point::new(700.0, 180.0),
        loading: false,
        card: Some(crate::backend::Card {
            name: "bob".into(),
            username: "bob".into(),
            bio: "**Maintainer** of the *Discord* bot, _really_.\n- Arch user\n- See https://example.com\n1. first\n2. second".into(),
            banner: "#5865f2".into(),
            avatar: String::new(),
            status: "CodeWorks Creative Hub".into(),
            created: 0,
            mutual: vec!["10.0.0.1:7878".into(), "10.0.0.2:7878".into()],
        }),
        error: None,
        above: false,
    });
    save_at(&app, "bio-formatting-card", 1100.0, 700.0);
}

#[test]
fn account_about_editor_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.account.about_bio = "I Use Arch BTW".into();
    app.account.about_banner = "#2ecc71".into();
    save_at(&app, "account-about", 1100.0, 800.0);
}

#[test]
fn profile_tab_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Profile;
    app.account.bio = "I Use Arch BTW".into();
    app.account.banner = "#2ecc71".into();
    app.account.about_bio = app.account.bio.clone();
    app.account.about_banner = app.account.banner.clone();
    save_at(&app, "profile-tab", 1100.0, 700.0);
}

#[test]
fn linked_accounts_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Linked;
    app.account.discord = Some("Sky".into());
    app.account.can_unlink_discord = true;
    save_at(&app, "linked-accounts", 1100.0, 700.0);
}

#[test]
fn account_password_and_delete_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Account;
    app.account.has_password = false;
    app.account.delete_open = true;
    save_at(&app, "account-delete-dialog", 1100.0, 800.0);
}

#[test]
fn linked_accounts_connect_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Linked;
    app.account.discord = None;
    save_at(&app, "linked-accounts-connect", 1100.0, 700.0);
}

#[test]
fn profile_unsaved_bar_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Profile;
    app.account.bio = "I Use Arch BTW".into();
    app.account.about_bio = "I Use Arch BTW, btw".into();
    save_at(&app, "profile-unsaved", 1100.0, 700.0);
}

#[test]
fn unsaved_bar_flash_screen() {
    let mut app = chat_app();
    app.page = Page::User;
    app.user_section = crate::app::UserSection::Profile;
    app.account.about_bio = "unsaved bio".into();
    app.unsaved_flash = Some(std::time::Instant::now());
    save_at(&app, "unsaved-bar-flash", 1100.0, 700.0);
}

/// How many message hovers a pointer sweep over the chat produces. Used to check that an open
/// profile card does not stop messages from being hovered.
#[cfg(test)]
fn hovers_during_sweep(app: &App) -> usize {
    use iced::{mouse, Event, Point};
    let mut ui = simulate(app);
    for y in (60..660).step_by(12) {
        for x in (300..700).step_by(12) {
            let p = Point::new(x as f32, y as f32);
            ui.point_at(p);
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: p })]);
        }
    }
    ui.into_messages()
        .filter(|m| matches!(m, Message::Hover(Some(_))))
        .count()
}

#[test]
fn profile_card_blocks_hovers_underneath() {
    let mut app = chat_app();
    let closed = hovers_during_sweep(&app);
    app.card = Some(crate::app::CardView {
        id: "u2".into(),
        name: "bob".into(),
        at: iced::Point::new(500.0, 300.0),
        loading: false,
        card: None,
        error: None,
        above: false,
    });
    let open = hovers_during_sweep(&app);
    assert!(closed > 0, "the sweep should hover messages at all");
    assert_eq!(open, 0, "the card should block hovers underneath it");
}

#[test]
fn clicking_own_status_opens_the_status_menu() {
    use iced::{mouse, Event, Point};
    let app = chat_app();
    let mut ui = simulate(&app);
    // Sweep the bottom of the channel list, where the user bar and its status label sit.
    for y in (560..700).step_by(6) {
        for x in (60..320).step_by(6) {
            let p = Point::new(x as f32, y as f32);
            ui.point_at(p);
            ui.simulate([
                Event::Mouse(mouse::Event::CursorMoved { position: p }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
        }
    }
    let messages: Vec<Message> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::OpenMenu(crate::app::MenuKind::Status))),
        "clicking the status should open its menu"
    );
}

#[test]
fn choosing_a_status_in_the_menu_sends_it() {
    use iced::{mouse, Event, Point};
    let mut app = chat_app();
    app.cursor = Point::new(150.0, 560.0);
    let _ = app.update(Message::OpenMenu(crate::app::MenuKind::Status));
    let mut ui = simulate(&app);
    let mut picked = Vec::new();
    for y in (540..680).step_by(4) {
        for x in (100..340).step_by(4) {
            let p = Point::new(x as f32, y as f32);
            ui.point_at(p);
            ui.simulate([
                Event::Mouse(mouse::Event::CursorMoved { position: p }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
        }
    }
    for m in ui.into_messages() {
        if matches!(m, Message::SetStatus(_)) {
            picked.push(m);
        }
    }
    assert!(!picked.is_empty(), "clicking a status in the menu should choose it");
}

#[test]
fn a_manage_server_role_alone_shows_server_settings() {
    let mut app = chat_app();
    // alice is not the owner. Her only role is one with Manage server and nothing else.
    app.server.owner_id = Some("someone-else".into());
    app.server.roles.push(common::RoleInfo {
        id: 9,
        name: "Editor".into(),
        color: String::new(),
        permissions: common::perm::MANAGE_SERVER,
        position: 1,
        mentionable: false,
        hoisted: false,
    });
    app.server.member_roles.insert("u1".into(), vec![9]);
    let labels: Vec<String> = app
        .server_menu_items()
        .into_iter()
        .filter_map(|item| match item {
            crate::app::MenuItem::Action { label, .. } => Some(label),
            _ => None,
        })
        .collect();
    assert!(labels.iter().any(|l| l == "Server settings"), "items: {labels:?}");
}

#[test]
fn latex_sample_screen() {
    let mut app = chat_app();
    let body = include_str!("latex_sample.md");
    app.messages.insert(
        1,
        vec![Entry::Chat(ChatMessage {
            id: 17,
            channel_id: 1,
            channel: "general".into(),
            author: "alice".into(),
            author_id: "u1".into(),
            text: body.into(),
            timestamp: crate::util::now(),
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })],
    );
    save_at(&app, "latex-sample", 1100.0, 1500.0);
}

fn two_messages(app: &mut App) {
    let now = crate::util::now();
    let chat = |id: i64, author: &str, text: &str| {
        Entry::Chat(ChatMessage {
            id,
            channel_id: 1,
            channel: "general".into(),
            author: author.into(),
            author_id: format!("id-{author}"),
            text: text.into(),
            timestamp: now,
            edited: false,
            reactions: Vec::new(),
            reply: None,
            pinned: false,
        })
    };
    app.messages.insert(
        1,
        vec![
            chat(20, "bob", "First message line\n\nIts second paragraph"),
            chat(21, "carol", "Second message text"),
        ],
    );
}

#[test]
fn selection_across_messages() {
    let mut app = chat_app();
    two_messages(&mut app);
    drag(&mut app, &[(420.0, 240.0), (470.0, 318.0)]);
    let sel = app.text_sel.clone().expect("a selection");
    assert_eq!(sel.anchor.0, 20);
    assert_eq!(sel.focus.0, 21);
    assert_eq!(sel.messages(), vec![20, 21]);
    // The first message is covered from the press to its end, the second from its start.
    let (from, to) = sel.within(20).expect("first message selected");
    assert_eq!(from.0, 0);
    assert_eq!(to, (usize::MAX, usize::MAX));
    assert_eq!(sel.within(21).expect("second message selected").0, (0, 0));
    save_at(&app, "selection-across-messages", 1100.0, 700.0);
}

#[test]
fn drag_into_gap_selects() {
    let mut app = chat_app();
    two_messages(&mut app);
    // Into the gap between the two paragraphs: the first is selected to its end.
    drag(&mut app, &[(420.0, 240.0), (420.0, 253.0)]);
    let sel = app.text_sel.clone().expect("a selection");
    assert_eq!((sel.focus.0, sel.focus.1), (20, 0));
    assert_eq!(sel.focus.2, "First message line".len() - 1);

    // Onto the next message's name line: the selection reaches the end of the last paragraph above.
    drag(&mut app, &[(420.0, 240.0), (420.0, 296.0)]);
    let sel = app.text_sel.clone().expect("a selection");
    assert_eq!((sel.focus.0, sel.focus.1), (20, 1));
}

#[test]
fn reply_screen() {
    let mut app = chat_app();
    two_messages(&mut app);
    // carol's message answers bob's first one.
    if let Some(Entry::Chat(m)) = app.messages.get_mut(&1).and_then(|l| l.get_mut(1)) {
        m.reply = Some(common::ReplyRef {
            id: 20,
            author: "bob".into(),
            author_id: "id-bob".into(),
            text: common::reply_preview("First message line\n\nIts second paragraph"),
        });
    }
    let _ = app.update(Message::ReplyStart(21));
    assert_eq!(app.replying_to, Some(21));
    save_at(&app, "reply", 1100.0, 700.0);

    // Escape stops the reply.
    let _ = app.update(Message::Escape);
    assert_eq!(app.replying_to, None);

    // A jump lights up the original; one that is not loaded says so instead.
    let _ = app.update(Message::JumpTo(20));
    assert_eq!(app.jump_flash, Some(20));
    let _ = app.update(Message::JumpFlashEnd);
    assert_eq!(app.jump_flash, None);
    let _ = app.update(Message::JumpTo(999));
    assert_eq!(app.jump_flash, None);
    assert!(app.status.contains("too far back"));
}

#[test]
fn reply_follows_original() {
    let mut app = chat_app();
    two_messages(&mut app);
    if let Some(Entry::Chat(m)) = app.messages.get_mut(&1).and_then(|l| l.get_mut(1)) {
        m.reply = Some(common::ReplyRef {
            id: 20,
            author: "bob".into(),
            author_id: "id-bob".into(),
            text: "old".into(),
        });
    }
    let reply = |app: &App| match app.messages[&1]
        .iter()
        .find(|e| matches!(e, Entry::Chat(m) if m.id == 21))
    {
        Some(Entry::Chat(m)) => m.reply.clone().unwrap(),
        _ => panic!("no reply"),
    };
    // What the edit and delete handlers do to the replies of a message.
    app.update_replies(1, 20, Some("new text"));
    assert_eq!(reply(&app).text, "new text");
    app.update_replies(1, 20, None);
    assert!(
        reply(&app).author_id.is_empty(),
        "a deleted original is no longer shown"
    );
    save_at(&app, "reply-deleted", 1100.0, 700.0);
}

#[test]
fn pins_screen() {
    let mut app = chat_app();
    two_messages(&mut app);
    let mut pinned = Vec::new();
    for e in app.messages.get_mut(&1).unwrap().iter_mut() {
        if let Entry::Chat(m) = e {
            if m.id == 20 {
                m.pinned = true;
                pinned.push(m.clone());
            }
        }
    }
    app.pins_open = true;
    app.pins = Some((1, pinned));
    save_at(&app, "pins", 1100.0, 700.0);

    // A click inside the dropdown keeps it open; one outside closes it.
    drag(&mut app, &[(700.0, 100.0)]);
    assert!(app.pins_open, "a click inside keeps it open");
    drag(&mut app, &[(300.0, 500.0)]);
    assert!(!app.pins_open, "a click outside closes it");
}

#[test]
fn pins_empty_screen() {
    let mut app = chat_app();
    two_messages(&mut app);
    app.pins_open = true;
    app.pins = Some((1, Vec::new()));
    save_at(&app, "pins-empty", 1100.0, 700.0);
    let _ = app.update(Message::Escape);
    assert!(!app.pins_open, "Escape closes it");
}

#[test]
fn automod_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::AutoMod));
    // The rules arrive from the server and fill the page.
    app.automod = Some(common::AutoMod {
        blocked_words: vec!["spoiler".into()],
        block_links: true,
        spam_filter: false,
        names: common::NameRule::default(),
        alert_channel: None,
    });
    app.automod_reset();
    assert!(!app.automod_dirty());

    // A change shows the unsaved bar; resetting puts the saved rules back.
    let _ = app.update(Message::AutoModSpam(true));
    assert!(app.automod_dirty());
    save_at(&app, "automod", 1100.0, 700.0);
    let _ = app.update(Message::AutoModReset);
    assert!(!app.automod_dirty() && !app.automod_spam);
    assert_eq!(app.automod_draft().blocked_words, vec!["spoiler".to_string()]);
}

#[test]
fn automod_refusal_keeps_text() {
    let mut app = chat_app();
    two_messages(&mut app);
    app.server.channel_id = 1;
    app.main.welcomed = true;
    app.set_input("buy now https://spam.example".into());
    let _ = app.update(Message::Send);
    assert!(app.input.is_empty(), "sent");
    let refuse = |app: &mut App, text: &str| {
        let _ = app.update(Message::Event(crate::backend::Event::Net {
            dm: false,
            addr: app.main.addr.clone().unwrap_or_default(),
            id: app.main.id.unwrap_or_default(),
            msg: common::ServerMsg::Error { text: text.into() },
        }));
    };
    refuse(&mut app, "AutoMod: links are not allowed here");
    // The refused message shows in the chat, for this member only, instead of the red status line.
    let notice = app.messages[&1].iter().find_map(|e| match e {
        Entry::Notice { id, text, reason, .. } => Some((*id, text.clone(), reason.clone())),
        _ => None,
    });
    let (id, text, reason) = notice.expect("a notice in the chat");
    assert_eq!(text, "buy now https://spam.example");
    assert_eq!(reason, "links are not allowed here");
    assert!(!app.status.contains("links"), "no red status line");

    // A refusal with nothing sent (a blocked name) shows just the reason.
    refuse(&mut app, "AutoMod: your name is not allowed on this server.");
    save_at(&app, "automod-notice", 1100.0, 700.0);

    // Dismissing removes only that notice.
    let _ = app.update(Message::DismissNotice(id));
    let left = app.messages[&1]
        .iter()
        .filter(|e| matches!(e, Entry::Notice { .. }))
        .count();
    assert_eq!(left, 1);
}

#[test]
fn automod_names_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    let _ = app.update(Message::ServerSettingsOpen);
    let _ = app.update(Message::ServerSectionPick(crate::app::ServerSection::AutoMod));
    let channel = app.server.channels.first().map(|c| c.id);
    app.automod = Some(common::AutoMod {
        names: common::NameRule {
            enabled: true,
            words: vec!["admin*".into(), "*mod*".into()],
            block_interactions: true,
            send_alert: true,
            ..common::NameRule::default()
        },
        alert_channel: channel,
        ..common::AutoMod::default()
    });
    app.automod_reset();
    assert!(!app.automod_dirty(), "the page matches what is saved");
    assert_eq!(
        app.automod_draft().names.words,
        vec!["admin*".to_string(), "*mod*".to_string()]
    );

    // Allowing a role and turning the alert off are unsaved changes.
    let _ = app.update(Message::NamesRole(2, true));
    assert!(app.automod_dirty());
    assert_eq!(app.automod_draft().names.allowed_roles, vec![2]);
    let _ = app.update(Message::NamesRole(2, false));
    assert!(!app.automod_dirty());
    let _ = app.update(Message::NamesAlert(false));
    assert!(app.automod_dirty());
    let _ = app.update(Message::AutoModChannel(None));
    assert_eq!(app.automod_draft().alert_channel, None);
    let _ = app.update(Message::AutoModReset);
    assert!(app.names_alert);
    assert_eq!(app.automod_channel, channel);

    // The rule starts closed, as a summary; Edit opens it.
    assert!(!app.names_editing);
    save_at(&app, "automod-names-closed", 1100.0, 1400.0);
    let _ = app.update(Message::NamesEdit(true));
    let _ = app.update(Message::NamesRole(2, true));
    save_at(&app, "automod-names", 1100.0, 1400.0);
}

#[test]
fn automod_alert_screen() {
    let mut app = chat_app();
    two_messages(&mut app);
    app.messages.get_mut(&1).unwrap().push(Entry::Chat(ChatMessage {
        id: 30,
        channel_id: 1,
        channel: "general".into(),
        author: "AutoMod".into(),
        author_id: common::AUTOMOD_ID.into(),
        text: "Flagged the name of **Dummy** (matched `dumm*`).".into(),
        timestamp: crate::util::now(),
        edited: false,
        reactions: Vec::new(),
        reply: None,
        pinned: false,
    }));
    // Hovering it shows no toolbar, as no member wrote it.
    app.hover = Some(30);
    save_at(&app, "automod-alert", 1100.0, 700.0);
}
