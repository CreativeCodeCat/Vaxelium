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
    prefs.servers = vec![ServerPref { name: "Home".into(), addr: "127.0.0.1:7878".into(), icon: None, id: None, ignored: Vec::new(), protected: Vec::new(), linked: false }];
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
    app.server.channels = vec![common::ChannelRef { id: 1, name: "general".into() }, common::ChannelRef { id: 2, name: "random".into() }];
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
    app.server.dms = vec![DirectMessage { channel_id: 5, channel: "dm-1".into(), name: "bob".into(), last_message: 1, other_id: "u2".into() }];
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
        fonts: vec![include_bytes!("../ui/fonts/NotoSans-Subset.ttf").as_slice().into()],
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
        common::ChannelRef { id: 1, name: "general".into() },
        common::ChannelRef { id: 2, name: "random".into() },
        common::ChannelRef { id: 3, name: "dev".into() },
    ];
    app.server.categories = vec![common::CategoryInfo {
        id: 1,
        name: "Voice and text".into(),
        channels: vec![common::ChannelRef { id: 3, name: "dev".into() }, common::ChannelRef { id: 2, name: "random".into() }],
    }];
    save_at(&app, "categories", 1100.0, 700.0);
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
        })],
    );
    save_at(&app, "small-caps", 1100.0, 700.0);
}

#[test]
fn collapsed_category_screen() {
    let mut app = chat_app();
    app.server.channels = vec![common::ChannelRef { id: 1, name: "general".into() }, common::ChannelRef { id: 2, name: "random".into() }];
    app.server.categories = vec![common::CategoryInfo { id: 7, name: "Community".into(), channels: vec![common::ChannelRef { id: 3, name: "dev".into() }] }];
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
        common::RoleInfo { id: 1, name: "Admin".into(), color: "#e91e63".into(), permissions: common::perm::ALL, position: 3, mentionable: false, hoisted: false },
        common::RoleInfo { id: 2, name: "Moderator".into(), color: "#2ecc71".into(), permissions: common::perm::MANAGE_MESSAGES, position: 2, mentionable: false, hoisted: false },
        common::RoleInfo { id: 0, name: "@everyone".into(), color: "#95a5a6".into(), permissions: 0, position: -1_000_000_000, mentionable: false, hoisted: false },
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
    let _ = app.update(Message::AccessDraftSet { target: crate::app::AccessTarget::Category(1), subject: common::Subject::Role(0), setting: common::Setting { view: Some(false), send: None, ..Default::default() } });
    save_at(&app, "category-access-unsaved", 1100.0, 700.0);
}

#[test]
fn access_add_roles_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo { id: 2, name: "Moderator".into(), color: "#2ecc71".into(), permissions: 0, position: 2, mentionable: false, hoisted: false },
        common::RoleInfo { id: 3, name: "Helper".into(), color: "#e91e63".into(), permissions: 0, position: 1, mentionable: false, hoisted: false },
        common::RoleInfo { id: 0, name: "@everyone".into(), color: "#95a5a6".into(), permissions: 0, position: -1_000_000_000, mentionable: false, hoisted: false },
    ];
    app.server.member_ids = [("alice".to_string(), "u1".to_string()), ("bob".to_string(), "u2".to_string())].into_iter().collect();
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
        channels: vec![common::ChannelRef { id: 1, name: "general".into() }],
    }];
    app.server.category_overrides = vec![common::CategoryOverride { category_id: 7, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None }];
    let _ = app.update(Message::ChannelEditOpen(1));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "channel-sync", 1100.0, 700.0);
}

#[test]
fn access_lists_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo { id: 2, name: "Moderator".into(), color: "#2ecc71".into(), permissions: 0, position: 2, mentionable: false, hoisted: false },
        common::RoleInfo { id: 0, name: "@everyone".into(), color: "#95a5a6".into(), permissions: 0, position: -1_000_000_000, mentionable: false, hoisted: false },
    ];
    app.server.member_ids = [("alice".to_string(), "u1".to_string())].into_iter().collect();
    app.server.category_overrides = vec![
        common::CategoryOverride { category_id: 1, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None },
        common::CategoryOverride { category_id: 1, role: 2, view: Some(true), send: Some(true), manage_channels: None, manage_messages: None },
    ];
    app.server.member_category_overrides = vec![common::MemberCategoryOverride { category_id: 1, member: "u1".into(), view: Some(true), send: None, manage_channels: None, manage_messages: None }];
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "access-lists", 1100.0, 700.0);
}

#[test]
fn category_private_locked_screen() {
    let mut app = chat_app();
    app.server.category_overrides = vec![common::CategoryOverride { category_id: 1, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None }];
    let _ = app.update(Message::CategoryEditOpen(1, "Text".into()));
    let _ = app.update(Message::EditSectionPick(crate::app::EditSection::Permissions));
    save_at(&app, "category-private-locked", 1100.0, 700.0);
}

#[test]
fn category_permissions_screen() {
    let mut app = chat_app();
    app.server.roles = vec![
        common::RoleInfo { id: 2, name: "Moderator".into(), color: "#2ecc71".into(), permissions: common::perm::MANAGE_MESSAGES, position: 2, mentionable: false, hoisted: false },
        common::RoleInfo { id: 0, name: "@everyone".into(), color: "#95a5a6".into(), permissions: 0, position: -1_000_000_000, mentionable: false, hoisted: false },
    ];
    app.server.category_overrides = vec![
        common::CategoryOverride { category_id: 1, role: 0, view: Some(false), send: None, manage_channels: None, manage_messages: None },
        common::CategoryOverride { category_id: 1, role: 2, view: Some(true), send: Some(true), manage_channels: None, manage_messages: None },
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
    app.server.channels = vec![common::ChannelRef { id: 1, name: "general".into() }];
    app.server.categories = vec![
        common::CategoryInfo { id: 1, name: "Community".into(), channels: Vec::new() },
        common::CategoryInfo { id: 2, name: "Games".into(), channels: Vec::new() },
    ];
    app.dnd = Some(Dnd { item: DragItem::Category(1), start: iced::Point::ORIGIN, moved: true, over: Some(DropTarget::End) });
    save_at(&app, "dragging-category", 1100.0, 700.0);
}

fn with_roles(app: &mut App) {
    use common::{perm, RoleInfo};
    app.server.roles = vec![
        RoleInfo { id: 1, name: "Admin".into(), color: "#e91e63".into(), permissions: perm::ALL, position: 3, mentionable: false, hoisted: false },
        RoleInfo { id: 2, name: "Moderator".into(), color: "#2ecc71".into(), permissions: perm::MANAGE_MESSAGES, position: 2, mentionable: false, hoisted: false },
        RoleInfo { id: 3, name: "Helper".into(), color: String::new(), permissions: 0, position: 1, mentionable: false, hoisted: false },
        RoleInfo { id: 0, name: "@everyone".into(), color: "#95a5a6".into(), permissions: 0, position: -1_000_000_000, mentionable: false, hoisted: false },
    ];
    app.server.online = vec!["alice".into(), "bob".into(), "carol".into(), "erin".into()];
    app.server.offline = vec!["dan".into()];
    app.server.member_ids = [("alice", "u1"), ("bob", "u2"), ("carol", "u3"), ("erin", "u5"), ("dan", "u4")]
        .into_iter()
        .map(|(n, i)| (n.to_string(), i.to_string()))
        .collect();
    app.server.member_roles = [("u2".to_string(), vec![2]), ("u5".to_string(), vec![1, 3])].into_iter().collect();
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
    let _ = app.update(Message::OpenMenu(MenuKind::User { name: "bob".into(), can_message: true }));
    let _ = app.update(Message::RolesMenuOpen);
    save_at(&app, "member-roles-menu", 1100.0, 700.0);
}

#[test]
fn member_roles_flyout_left_screen() {
    let mut app = chat_app();
    with_roles(&mut app);
    app.cursor = iced::Point::new(900.0, 300.0);
    let _ = app.update(Message::OpenMenu(MenuKind::User { name: "bob".into(), can_message: true }));
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
    app.input = "hey @".into();
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
    app.server.member_status.insert("id-bob".into(), common::PresenceStatus::Online);
    app.server.badges = vec![
        common::Badge { id: 1, name: "Early Adopter".into(), color: "#f1c40f".into() },
        common::Badge { id: 2, name: "Helper".into(), color: "#2ecc71".into() },
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
    ui.into_messages().filter(|m| matches!(m, Message::Hover(Some(_)))).count()
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
    assert!(messages.iter().any(|m| matches!(m, Message::OpenMenu(crate::app::MenuKind::Status))), "clicking the status should open its menu");
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
