//! The full pages: login, the server status page, discovery, and the settings pages.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, svg, text, text_editor, text_input, toggler, Column};
use iced::{border, Background, Color, Element, Length, Padding, Theme};

use super::{divider, link, muted_text, round, server_icon, solid};
use crate::app::{App, EditSection, Field, Message, RoleDraft, RoleTab, ServerSection, UserSection, ADD_ADDR_ID};
use common::{RoleInfo, EVERYONE_ROLE};
use crate::icons;
use crate::theme::Pal;

const CERT_CHANGED: &str = "certificate has changed";

/// A centred card on the window's background.
fn card<'a>(content: Vec<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    container(
        container(Column::with_children(content).spacing(12).width(Length::Fill))
            .padding(Padding::from(24.0))
            .width(Length::Fixed(420.0))
            .style(move |_: &Theme| round(pal.sidebar_bg, 8.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_: &Theme| round(pal.app_bg, 0.0))
    .into()
}

fn field<'a>(label: &str, input: Element<'a, Message>) -> Element<'a, Message> {
    column![text(label.to_uppercase()).size(12).color(Color::from_rgb(0.6, 0.6, 0.6)), input].spacing(6).into()
}

/// The multi-line box uses the same colours as the single-line inputs.
fn editor_style(pal: Pal) -> impl Fn(&Theme, text_editor::Status) -> text_editor::Style {
    move |_: &Theme, _| text_editor::Style {
        background: Background::Color(pal.input_bg),
        border: border::rounded(6.0),
        placeholder: pal.text_muted,
        value: pal.text_strong,
        selection: pal.accent,
    }
}

pub fn input_style(pal: Pal) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_: &Theme, _| text_input::Style {
        background: Background::Color(pal.input_bg),
        border: border::rounded(6.0),
        icon: pal.text_muted,
        placeholder: pal.text_muted,
        value: pal.text_strong,
        selection: pal.accent,
    }
}

pub fn login(app: &App, pal: Pal) -> Element<'_, Message> {
    let login_form = &app.login;
    let mut items: Vec<Element<'_, Message>> = vec![
        text(if login_form.register { "Create an account" } else { "Welcome back!" }).size(26).color(pal.text_strong).into(),
        muted_text("One account works on every Vaxelium server.", 14.0, pal.text_muted),
    ];
    if login_form.register {
        items.push(field(
            "Display name",
            text_input("your name", &login_form.name)
                .on_input(Message::LoginName)
                .padding(10)
                .style(input_style(pal))
                .into(),
        ));
    }
    items.push(field(
        "Email",
        text_input("you@example.com", &login_form.email)
            .on_input(Message::LoginEmail)
            .on_submit(Message::LoginSubmit)
            .padding(10)
            .style(input_style(pal))
            .into(),
    ));
    items.push(field(
        "Password",
        text_input("password", &login_form.password)
            .secure(true)
            .on_input(Message::LoginPassword)
            .on_submit(Message::LoginSubmit)
            .padding(10)
            .style(input_style(pal))
            .into(),
    ));
    items.push(
        row![
            iced::widget::checkbox(app.remember).on_toggle(Message::Remember),
            text("Keep me logged in on this computer").size(14).color(pal.text),
        ]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    );
    items.push(solid(if login_form.register { "Register" } else { "Log in" }, Message::LoginSubmit, pal.accent, pal.on_color, None));
    items.push(
        row![
            muted_text(if login_form.register { "Already have an account?" } else { "Need an account?" }, 14.0, pal.text_muted),
            link(if login_form.register { "Log in" } else { "Register" }, Message::LoginToggle, pal.link),
        ]
        .spacing(6)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    );
    if let Some(error) = &login_form.error {
        items.push(text(error.clone()).size(13).color(pal.red).into());
        if error.contains(CERT_CHANGED) {
            items.push(solid("Trust the new certificate", Message::TrustAuth, pal.accent, pal.on_color, None));
        }
    }
    card(items, pal)
}

/// The page shown when logged in with no server open.
pub fn server_status(app: &App, pal: Pal) -> Element<'_, Message> {
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    if app.prefs.servers.is_empty() {
        items.push(text("Add a server").size(26).color(pal.text_strong).into());
        items.push(muted_text("Enter the address of a Vaxelium server to start chatting.", 14.0, pal.text_muted));
        items.push(solid("Add a server", Message::AddServerOpen, pal.accent, pal.on_color, None));
    } else if app.main.connecting || (app.main.id.is_some() && !app.main.welcomed && app.main.error.is_none()) {
        items.push(text(format!("Joining {}...", app.selected_name().unwrap_or_else(|| "server".into()))).size(26).color(pal.text_strong).into());
        items.push(muted_text("Checking your account with the server.", 14.0, pal.text_muted));
    } else if let Some(error) = &app.main.error {
        items.push(text("Couldn't join the server").size(26).color(pal.text_strong).into());
        items.push(text(error.clone()).size(14).color(pal.red).into());
        if error.contains(CERT_CHANGED) {
            items.push(solid("Trust the new certificate", Message::TrustServer, pal.accent, pal.on_color, None));
        } else if app.selected.is_some() {
            items.push(solid("Try again", Message::RetryServer, pal.accent, pal.on_color, None));
        }
    } else {
        items.push(text("Pick a server").size(26).color(pal.text_strong).into());
        items.push(muted_text("Choose one from the icons on the left, or add a new one with +.", 14.0, pal.text_muted));
    }
    items.push(
        row![
            muted_text(format!("Signed in as {}", app.user.clone().unwrap_or_default()), 13.0, pal.text_muted),
            link("Log out", Message::Logout, pal.link),
        ]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    );
    card(items, pal)
}

/// The page that lists servers found through the servers you know.
pub fn discover(app: &App, pal: Pal) -> Element<'_, Message> {
    let d = &app.discover;
    let mut items: Vec<Element<'_, Message>> = vec![
        page_head("Find servers", pal, Message::DiscoverClose),
        muted_text("Servers found through the servers you know. Pick one to add it.", 14.0, pal.text_muted),
    ];
    if let Some(error) = &d.error {
        items.push(text(error.clone()).size(14).color(pal.red).into());
    } else if d.found.is_empty() {
        items.push(muted_text("Searching...", 14.0, pal.text_muted));
    }
    for f in &d.found {
        let saved = app.prefs.servers.iter().any(|s| s.addr == f.addr);
        let expanded = d.expanded.contains(&f.addr);
        // A house beside the name marks a server that other servers know about.
        let mut title = row![text(f.name.clone()).size(16).color(pal.text_strong)].spacing(8).align_y(iced::alignment::Vertical::Center);
        if f.public {
            title = title.push(svg(svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/house.svg"))).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)));
        }
        let mut info = Column::new().spacing(2).push(title);
        if !f.description.is_empty() {
            info = info.push(super::linked_text(&f.description, 13.0, pal.text, pal));
        }
        let mut details = row![
            text(f.addr.clone()).size(12).color(pal.text_muted),
            text(format!("· {} online", f.online)).size(12).color(pal.green),
        ]
        .spacing(6);
        if f.self_signed {
            details = details.push(text("· Self-signed certificate").size(12).color(pal.red));
        }
        info = info.push(details);
        if expanded {
            info = info.push(muted_text("Click again to fold this card.", 12.0, pal.text_muted));
        }
        let side: Element<'_, Message> = if saved {
            text("Added").size(13).color(pal.text_muted).into()
        } else {
            solid("Add", Message::DiscoverAdd(f.addr.clone()), pal.accent, pal.on_color, None)
        };
        let card_row = row![server_icon(f.icon.as_deref(), &f.name, 48.0, &pal, 8.0), info.width(Length::Fill), side]
            .spacing(14)
            .align_y(iced::alignment::Vertical::Center);
        items.push(
            mouse_area(
                container(card_row)
                    .padding(Padding::from(12.0))
                    .width(Length::Fill)
                    .style(move |_: &Theme| round(pal.sidebar_bg, 8.0)),
            )
            .on_press(Message::DiscoverToggle(f.addr.clone()))
            .into(),
        );
    }
    scrollable(Column::with_children(items).spacing(12).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

/// The heading row of a full page: a section name, the title and a close button.
/// The title at the top of a settings page's content. The close button is in the frame's corner.
fn page_title<'a>(title: &str, pal: Pal) -> Element<'a, Message> {
    text(title.to_string()).size(22).color(pal.text_strong).into()
}

/// A settings page: its navigation on the left, and its content on the right with the close button in
/// the same top-right corner on every settings page.
fn settings_frame<'a>(nav: Element<'a, Message>, body: Element<'a, Message>, close: Message, pal: Pal) -> Element<'a, Message> {
    let corner = row![space().width(Length::Fill), close_button(close, pal)].padding(crate::util::pad2(12.0, 16.0));
    let right = column![corner, body].width(Length::Fill).height(Length::Fill);
    container(row![nav, right].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &Theme| round(pal.app_bg, 0.0))
        .into()
}

fn page_head<'a>(title: &str, pal: Pal, close: Message) -> Element<'a, Message> {
    row![
        text(title.to_string()).size(22).color(pal.text_strong).width(Length::Fill),
        close_button(close, pal),
    ]
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// The X in the corner of a page, with ESC under it, as Discord has it.
fn close_button<'a>(close: Message, pal: Pal) -> Element<'a, Message> {
    button(
        column![icons::icon(icons::CLOSE, 16.0, pal.text_muted), text("ESC").size(11).color(pal.text_muted)]
            .align_x(iced::alignment::Horizontal::Center)
            .spacing(2),
    )
    .padding(4)
    .on_press(close)
    .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() })
    .into()
}

/// The left column of a settings page: its name, and the section that is open.
/// The tabs of user settings: Account and Profile.
fn user_nav<'a>(active: UserSection, pal: Pal) -> Element<'a, Message> {
    let tab = |label: &'static str, section: UserSection| -> Element<'a, Message> {
        let selected = section == active;
        let color = if selected { pal.text_strong } else { pal.text_muted };
        button(text(label).size(15).color(color))
            .width(Length::Fill)
            .padding(crate::util::pad2(6.0, 10.0))
            .on_press(Message::UserSectionPick(section))
            .style(move |_: &Theme, status: button::Status| {
                let bg = if selected {
                    pal.selected_bg
                } else if matches!(status, button::Status::Hovered) {
                    pal.hover_bg
                } else {
                    Color::TRANSPARENT
                };
                button::Style { background: Some(Background::Color(bg)), text_color: color, border: border::rounded(4.0), ..button::Style::default() }
            })
            .into()
    };
    container(column![
        text("USER SETTINGS").size(12).color(pal.text_muted),
        Column::new().spacing(4).push(tab("Account", UserSection::Account)).push(tab("Profile", UserSection::Profile)),
    ]
    .spacing(8))
    .padding(crate::util::pad2(24.0, 12.0))
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
    .into()
}

/// The settings content: a column with a sensible maximum width, so long rows do not stretch.
fn settings_column<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let inner = container(Column::with_children(items).spacing(6).width(Length::Fill))
        .max_width(760.0)
        .width(Length::Fill);
    scrollable(container(inner).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

fn setting_row<'a>(label: &str, desc: Option<&str>, value: Element<'a, Message>, action: Option<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    let mut what = Column::new().push(text(label.to_string()).size(15).color(pal.text));
    if let Some(d) = desc {
        what = what.push(text(d.to_string()).size(12).color(pal.text_muted));
    }
    let mut r = row![what.width(Length::Fill), value].spacing(12).align_y(iced::alignment::Vertical::Center);
    if let Some(a) = action {
        r = r.push(a);
    }
    container(r).padding(crate::util::pad2(8.0, 0.0)).width(Length::Fill).into()
}

/// The profile picture. Saved as soon as it is picked, like a server icon.
fn avatar_editor<'a>(acc: &'a crate::app::Account, name: &str, pal: Pal) -> Element<'a, Message> {
    // Shows the draft while one is pending, so the change can be seen before it is saved.
    let shown = acc.avatar_draft.as_deref().unwrap_or(&acc.avatar);
    let mut buttons = row![solid("Change picture", Message::AvatarPick, pal.accent, pal.on_color, None)].spacing(12).align_y(iced::alignment::Vertical::Center);
    if !shown.is_empty() {
        buttons = buttons.push(link("Remove", Message::AvatarRemove, pal.link));
    }
    let mut column_items: Vec<Element<'a, Message>> = vec![
        text("PROFILE PICTURE").size(12).color(pal.text_muted).into(),
        row![super::avatar(shown, &acc.about_banner, name, 72.0, &pal), buttons].spacing(16).align_y(iced::alignment::Vertical::Center).into(),
    ];
    if let Some(err) = &acc.avatar_error {
        column_items.push(text(err.clone()).size(13).color(pal.red).into());
    }
    Column::with_children(column_items).spacing(8).into()
}

/// The bio and profile banner colour, shown on the profile card. Always an editor; the save bar
/// appears under it when something has changed.
fn about_editor<'a>(app: &'a App, pal: Pal) -> Element<'a, Message> {
    let acc = &app.account;
    let mut rows: Vec<Element<'a, Message>> = vec![
        text("ABOUT ME").size(12).color(pal.text_muted).into(),
        iced::widget::text_editor(&app.bio_editor)
            .placeholder("A short bio (190 characters at most)")
            .on_action(Message::BioEdit)
            .height(Length::Fixed(96.0))
            .padding(10)
            .style(editor_style(pal))
            .into(),
    ];
    // The editor cannot show links, so a preview appears under it while the bio has any.
    let draft = app.bio_editor.text();
    if !super::link_ranges(&draft).is_empty() {
        rows.push(text("PREVIEW").size(12).color(pal.text_muted).into());
        rows.push(super::linked_text(draft.trim_end(), 13.0, pal.text, pal));
    }
    rows.push(text("BANNER COLOUR").size(12).color(pal.text_muted).into());
    let picked = acc.about_banner.to_lowercase();
    let mut swatches = row![].spacing(8).align_y(iced::alignment::Vertical::Center);
    for hex in ROLE_COLORS.iter().take(10) {
        let color = crate::util::hex_color(hex).unwrap_or(Color::BLACK);
        let on = picked == *hex;
        swatches = swatches.push(
            button(space().width(Length::Fixed(24.0)).height(Length::Fixed(24.0)))
                .padding(0)
                .on_press(Message::AboutBanner(hex.to_string()))
                .style(move |_: &Theme, _: button::Status| {
                    let mut ring = border::rounded(12.0);
                    ring.color = if on { pal.text_strong } else { Color::TRANSPARENT };
                    ring.width = if on { 2.0 } else { 0.0 };
                    button::Style { background: Some(Background::Color(color)), border: ring, ..button::Style::default() }
                }),
        );
    }
    swatches = swatches.push(link(if picked.is_empty() { "Default ✓" } else { "Default" }, Message::AboutBanner(String::new()), pal.link));
    rows.push(swatches.into());
    if let Some(err) = &acc.about_error {
        rows.push(text(err.clone()).size(13).color(pal.red).into());
    }
    container(Column::with_children(rows).spacing(8)).padding(crate::util::pad2(8.0, 0.0)).width(Length::Fill).into()
}

fn small_button<'a>(label: &str, msg: Message, pal: Pal) -> Element<'a, Message> {
    button(text(label.to_string()).size(13).color(pal.text_strong))
        .padding(crate::util::pad2(5.0, 12.0))
        .on_press(msg)
        .style(move |_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => pal.hover_bg,
                _ => pal.input_bg,
            };
            button::Style { background: Some(Background::Color(bg)), text_color: pal.text_strong, border: border::rounded(4.0), ..button::Style::default() }
        })
        .into()
}

/// The error under the server icon buttons, or nothing.
fn icon_error<'a>(error: Option<String>, pal: Pal) -> Element<'a, Message> {
    match error {
        Some(e) => text(e).size(13).color(pal.red).into(),
        None => space().height(Length::Fixed(0.0)).into(),
    }
}

/// The account page: name, username, email, the theme, and log out.
pub fn user_settings(app: &App, pal: Pal) -> Element<'_, Message> {
    let acc = &app.account;
    let user = app.user.clone().unwrap_or_default();
    let shown_name = acc.draft_name.clone().unwrap_or_else(|| user.clone());
    // When the auth service cannot be reached, the account details are unknown. Show that in the
    // value, put the reason under the label, and drop the Edit buttons that could not save anyway.
    let unavailable = acc.username.is_none() && acc.draft_username.is_none() && acc.email_error.is_some();
    let reason = acc.email_error.clone();
    let shown_user = acc.draft_username.clone().or_else(|| acc.username.clone()).unwrap_or_else(|| "Unavailable".into());
    let email: Element<'_, Message> = match (&acc.draft_email, &acc.email) {
        (Some((e, _)), _) => text(e.clone()).size(14).color(pal.text).into(),
        (None, Some(e)) => text(e.clone()).size(14).color(pal.text).into(),
        _ if unavailable => text("Unavailable").size(14).color(pal.text_muted).into(),
        _ => text("Not on record yet. Set it with Edit.").size(14).color(pal.text_muted).into(),
    };
    let theme_name = app.theme_id_name(&acc.draft_theme.clone().unwrap_or_else(|| app.prefs.theme.clone()));
    let theme_button = button(
        row![
            text(theme_name).size(14).color(pal.text_strong),
            space().width(Length::Fill),
            icons::icon(icons::CHEVRON_DOWN, 14.0, pal.text_muted),
        ]
            .spacing(12)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(crate::util::pad2(7.0, 10.0))
    .width(Length::Fixed(190.0))
    .on_press(Message::OpenMenu(crate::app::MenuKind::Theme))
    .style(move |_: &Theme, _| button::Style {
        background: Some(Background::Color(pal.input_bg)),
        text_color: pal.text_strong,
        border: border::rounded(6.0).color(pal.hover_bg).width(1.0),
        ..button::Style::default()
    });

    // Verified shows as a label. Otherwise a Verify button starts the code emailed to the address, but
    // only when this server can send email.
    let email_status: Element<'_, Message> = if acc.verified {
        text("Verified").size(13).color(pal.green).into()
    } else if acc.can_verify {
        small_button("Verify", Message::VerifyOpen, pal)
    } else {
        space().width(Length::Fixed(0.0)).into()
    };
    let mut main: Vec<Element<'_, Message>> = vec![
        page_title("Account", pal),
        row![
            super::avatar(&acc.avatar, &acc.banner, &user, 72.0, &pal),
            column![
                text(shown_name.clone()).size(20).color(pal.text_strong),
                text("Signed in on this computer").size(13).color(pal.green),
            ],
        ]
        .spacing(16)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
        divider(pal.selected_bg),
        text("ACCOUNT INFO").size(12).color(pal.text_muted).into(),
        setting_row("Display name", None, text(shown_name).size(14).color(pal.text_strong).into(), Some(small_button("Edit", Message::AccountEdit(Field::Name), pal)), pal),
        if unavailable {
            setting_row("Username", reason.as_deref(), text(shown_user).size(14).color(pal.text_muted).into(), None, pal)
        } else {
            setting_row("Username", None, text(format!("@{shown_user}")).size(14).color(pal.text).into(), Some(small_button("Edit", Message::AccountEdit(Field::Username), pal)), pal)
        },
        if unavailable {
            setting_row("Email", reason.as_deref(), email, None, pal)
        } else {
            setting_row(
                "Email",
                None,
                email,
                Some(
                    row![
                        email_status,
                        small_button("Edit", Message::AccountEdit(Field::Email), pal),
                    ]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Center)
                    .into(),
                ),
                pal,
            )
        },
        divider(pal.selected_bg),
        text("PREFERENCES").size(12).color(pal.text_muted).into(),
        setting_row("Theme", None, theme_button.into(), None, pal),
        divider(pal.selected_bg),
        text("SESSION").size(12).color(pal.text_muted).into(),
        setting_row("Log out", Some("Sign out of Vaxelium on this computer."), space().width(Length::Fixed(0.0)).into(), Some(solid("Log Out", Message::Logout, pal.red, pal.on_color, None)), pal),
    ];
    if let Some(error) = &acc.save_error {
        main.push(text(error.clone()).size(13).color(pal.red).into());
    }
    let (content, dirty) = match app.user_section {
        UserSection::Account => {
            let dirty = acc.draft_name.is_some() || acc.draft_username.is_some() || acc.draft_email.is_some() || acc.draft_theme.is_some();
            (settings_column(main), dirty)
        }
        UserSection::Profile => {
            // The bar appears only when the bio or banner differ from what is saved.
            let dirty = acc.about_bio != acc.bio || acc.about_banner != acc.banner || acc.avatar_draft.is_some();
            let name = app.user.clone().unwrap_or_default();
            (settings_column(vec![page_title("Profile", pal), avatar_editor(acc, &name, pal), about_editor(app, pal)]), dirty)
        }
    };
    let page = settings_frame(user_nav(app.user_section, pal), content, Message::UserSettingsClose, pal);
    let mut layers: Vec<Element<'_, Message>> = vec![page];
    if dirty {
        let (reset, save) = match app.user_section {
            UserSection::Account => (Message::AccountReset, Message::AccountSave),
            UserSection::Profile => (Message::AboutReset, Message::AboutSave),
        };
        layers.push(unsaved_bar(app, pal, reset, save, "Careful – you have unsaved changes!"));
    }
    if let Some(which) = acc.editing {
        layers.push(account_edit_dialog(app, which, pal));
    }
    iced::widget::stack(layers).into()
}

/// Blends `a` towards `b` by `t` (0 is `a`, 1 is `b`).
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a }
}

/// The bar that appears when there are changes to save. It floats over the bottom of the page,
/// centred, at most 640 pixels wide, as it did in the old client.
fn unsaved_bar<'a>(app: &App, pal: Pal, reset: Message, save: Message, title: &str) -> Element<'a, Message> {
    // Red while a close was refused, so the user sees they have not saved.
    let flash = app.unsaved_flash_on();
    // A hint, not an alarm: the background warms slightly and the edge takes the red.
    let bg = if flash { mix(pal.rail_bg, pal.red, 0.22) } else { pal.rail_bg };
    let edge = if flash { pal.red } else { bg };
    let card = container(row![
        text(title.to_string()).size(14).color(pal.text_strong).width(Length::Fill),
        link("Reset", reset, pal.link),
        solid("Save Changes", save, pal.green, pal.on_color, None),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center))
    .padding(crate::util::pad2(12.0, 16.0))
    .max_width(640.0)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: border::rounded(8.0).color(edge).width(1.0),
        ..container::Style::default()
    });
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(crate::util::pad(0.0, 0.0, 24.0, 0.0))
        .into()
}


fn account_edit_dialog(app: &App, which: Field, pal: Pal) -> Element<'_, Message> {
    let acc = &app.account;
    let (title, body): (&str, Element<'_, Message>) = match which {
        Field::Name => (
            "Edit display name",
            text_input("How your name appears in chat", &acc.name_input)
                .on_input(Message::AccountNameInput)
                .on_submit(Message::AccountEditSubmit)
                .padding(10)
                .style(input_style(pal))
                .into(),
        ),
        Field::Username => (
            "Edit username",
            text_input("lowercase letters, digits and _", &acc.username_input)
                .on_input(Message::AccountUsernameInput)
                .on_submit(Message::AccountEditSubmit)
                .padding(10)
                .style(input_style(pal))
                .into(),
        ),
        Field::Email => (
            "Change email",
            column![
                field("New email", text_input("you@example.com", &acc.email_input).on_input(Message::AccountEmailInput).padding(10).style(input_style(pal)).into()),
                field("Current password", text_input("", &acc.email_password).secure(true).on_input(Message::AccountEmailPassword).on_submit(Message::AccountEditSubmit).padding(10).style(input_style(pal)).into()),
            ]
            .spacing(12)
            .into(),
        ),
    };
    let mut items: Vec<Element<'_, Message>> = vec![text(title).size(20).color(pal.text_strong).into(), body];
    if let Some(err) = &acc.form_error {
        items.push(text(err.clone()).size(13).color(pal.red).into());
    }
    items.push(
        row![
            space().width(Length::Fill),
            link("Cancel", Message::AccountEditCancel, pal.text_muted),
            solid("Save", Message::AccountEditSubmit, pal.accent, pal.on_color, Some(96.0)),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    );
    modal(items, pal, Message::AccountEditCancel)
}

/// A dialog box over a dimmed backdrop, centred. The backdrop sends `close` when pressed.
pub fn modal<'a>(items: Vec<Element<'a, Message>>, pal: Pal, close: Message) -> Element<'a, Message> {
    let card = container(Column::with_children(items).spacing(12).width(Length::Fill))
        .padding(Padding::from(20.0))
        .width(Length::Fixed(440.0))
        .style(move |_: &Theme| round(pal.sidebar_bg, 8.0));
    let backdrop = mouse_area(
        container(space()).width(Length::Fill).height(Length::Fill).style(|_: &Theme| round(Color { a: 0.5, ..Color::BLACK }, 0.0)),
    )
    .on_press(close);
    let centred = container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    iced::widget::stack![backdrop, centred].into()
}

/// The server settings page. The owner sees the Server Profile section and the Roles section; anyone
/// who may manage roles sees Roles only.
pub fn server_settings(app: &App, pal: Pal) -> Element<'_, Message> {
    let name = app.selected_name().unwrap_or_else(|| "Server".into());
    let section = app.shown_server_section();
    let (content, dirty): (Element<'_, Message>, bool) = match section {
        ServerSection::Profile => {
            let (items, dirty) = profile_items(app, pal);
            let content = scrollable(Column::with_children(items).spacing(14).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill)).height(Length::Fill);
            (content.into(), dirty)
        }
        ServerSection::Roles => roles_section(app, pal),
        ServerSection::Badges => badges_section(app, pal),
    };
    // The bar saves whichever section is open.
    let (reset, save) = if section == ServerSection::Roles {
        (Message::RoleReset, Message::RoleSave)
    } else {
        (Message::ServerReset, Message::ServerSave)
    };
    let manage = app.my_permissions() & common::perm::MANAGE_ROLES != 0;
    let may_edit_profile = app.is_owner() || app.can(common::perm::MANAGE_SERVER);
    let page = settings_frame(server_nav(&name, may_edit_profile, manage, section, pal), content, Message::ServerSettingsClose, pal);
    if dirty {
        iced::widget::stack![page, unsaved_bar(app, pal, reset, save, "Careful – you have unsaved changes!")].into()
    } else {
        page.into()
    }
}

/// The navigation down the left of the server settings page.
fn server_nav<'a>(name: &str, owner: bool, manage: bool, active: ServerSection, pal: Pal) -> Element<'a, Message> {
    let mut items = Column::new().spacing(4);
    if owner {
        items = items.push(nav_item("Server Profile", ServerSection::Profile, active, pal));
    }
    items = items.push(nav_item("Roles", ServerSection::Roles, active, pal));
    if manage {
        items = items.push(nav_item("Badges", ServerSection::Badges, active, pal));
    }
    container(column![text(name.to_uppercase()).size(12).color(pal.text_muted), items].spacing(8))
        .padding(crate::util::pad2(24.0, 12.0))
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
        .into()
}

fn nav_item<'a>(label: &str, section: ServerSection, active: ServerSection, pal: Pal) -> Element<'a, Message> {
    nav_button(label, Message::ServerSectionPick(section), section == active, pal)
}

/// One item of a settings page's navigation.
fn nav_button<'a>(label: &str, msg: Message, selected: bool, pal: Pal) -> Element<'a, Message> {
    nav_button_tinted(label, msg, selected, None, pal)
}

/// A navigation item whose text can take a colour, such as a role's.
fn nav_button_tinted<'a>(label: &str, msg: Message, selected: bool, tint: Option<Color>, pal: Pal) -> Element<'a, Message> {
    let color = tint.unwrap_or(if selected { pal.text_strong } else { pal.text_muted });
    button(text(label.to_string()).size(15).color(color))
        .width(Length::Fill)
        .padding(crate::util::pad2(6.0, 10.0))
        .on_press(msg)
        .style(move |_: &Theme, status: button::Status| {
            let bg = if selected {
                pal.selected_bg
            } else if matches!(status, button::Status::Hovered) {
                pal.hover_bg
            } else {
                Color::TRANSPARENT
            };
            button::Style { background: Some(Background::Color(bg)), text_color: color, border: border::rounded(4.0), ..button::Style::default() }
        })
        .into()
}

/// The Server Profile section: the name, description and picture. Also says whether it has unsaved changes.
/// True when the server profile form has changes not yet saved.
pub(crate) fn server_profile_dirty(app: &App) -> bool {
    let form = &app.server_form;
    let name = app.selected_name().unwrap_or_else(|| "Server".into());
    (!form.name.trim().is_empty() && form.name.trim() != name) || form.description.trim() != app.server.description
}

fn profile_items(app: &App, pal: Pal) -> (Vec<Element<'_, Message>>, bool) {
    let form = &app.server_form;
    let name = app.selected_name().unwrap_or_else(|| "Server".into());
    let dirty = server_profile_dirty(app);
    let icon: Element<'_, Message> = server_icon(app.server.icon.as_deref(), &name, 64.0, &pal, 8.0);
    let mut items: Vec<Element<'_, Message>> = vec![
        page_title("Server Profile", pal),
        muted_text("Customise how your server appears to its members.", 14.0, pal.text_muted),
        text("SERVER ICON").size(12).color(pal.text_muted).into(),
        row![
            icon,
            column![
                row![
                    small_button("Upload image", Message::IconPick, pal),
                    if app.server.icon.is_some() { link("Remove icon", Message::IconRemove, pal.link) } else { space().width(Length::Fixed(0.0)).into() },
                ]
                .spacing(12),
                icon_error(form.icon_error.clone(), pal),
                muted_text("PNG or JPG. It is shrunk to 128x128.", 12.0, pal.text_muted),
            ]
            .spacing(6),
        ]
        .spacing(16)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
        field("Name", text_input("My server", &form.name).id("server-name").on_input(Message::ServerName).on_submit(Message::ServerSave).padding(10).style(input_style(pal)).into()),
        field(
            "Description",
            iced::widget::text_editor(&app.description_editor)
                .placeholder("Welcome to our community!")
                .on_action(Message::ServerDescriptionEdit)
                .height(Length::Fixed(140.0))
                .padding(10)
                .style(editor_style(pal))
                .into(),
        ),
    ];
    if dirty {
        items.push(divider(pal.selected_bg));
    }
    (items, dirty)
}

/// The colours offered for a role, as in Discord's palette.
const ROLE_COLORS: [&str; 20] = [
    "#1abc9c", "#2ecc71", "#3498db", "#9b59b6", "#e91e63", "#f1c40f", "#e67e22", "#e74c3c", "#95a5a6", "#607d8b",
    "#11806a", "#1f8b4c", "#206694", "#71368a", "#ad1457", "#c27c0e", "#a84300", "#992d22", "#979c9f", "#546e7a",
];

/// The permissions a role can grant: the label, what it lets a member do, and its bit.
const PERMISSIONS: [(&str, &str, u32); 8] = [
    ("Administrator", "Has every permission, even ones switched off. Turning it on asks first.", common::perm::ADMINISTRATOR),
    ("Manage roles", "Allows members to create and edit roles below their own, and give them to members below them.", common::perm::MANAGE_ROLES),
    ("Manage server", "Allows members to change the server's name, description and picture.", common::perm::MANAGE_SERVER),
    ("Manage channels", "Allows members to create, rename, move and delete channels and categories.", common::perm::MANAGE_CHANNELS),
    ("Manage messages", "Allows members to delete other members' messages.", common::perm::MANAGE_MESSAGES),
    ("Bypass ignore", "Members with this cannot be ignored or blocked by anyone.", common::perm::BYPASS_IGNORE),
    ("View channels", "Lets members see the channels. A channel can hide them from this role.", common::perm::VIEW_CHANNEL),
    ("Message channels", "Lets members send messages. A channel can stop this role from sending.", common::perm::SEND_MESSAGES),
];

/// The Roles section, laid out like Discord's: the roles on the left, and the chosen role's settings on the right.
/// True when the editor's draft differs from the role as the server has it, so there is something to save.
pub(crate) fn role_changed(app: &App, draft: &RoleDraft) -> bool {
    if !draft.id.map_or(true, |id| app.outranks_role(id)) {
        return false;
    }
    match draft.id.and_then(|id| app.server.roles.iter().find(|r| r.id == id)) {
        Some(role) => {
            role.name != draft.name.trim()
                || role.color != draft.color.trim()
                || role.permissions != draft.permissions
                || role.mentionable != draft.mentionable
                || role.hoisted != draft.hoisted
        }
        // A new role is made with its own Create button, so it has no bar.
        None => false,
    }
}

/// The members section: every member with their status and roles, read-only.
pub(crate) fn members_section(app: &App, pal: Pal) -> Element<'_, Message> {
    let members: Vec<(&String, bool)> = app.server.online.iter().map(|n| (n, true)).chain(app.server.offline.iter().map(|n| (n, false))).collect();
    let count = members.len();
    let head = |label: &str, width: Length| text(label.to_string()).size(12).color(pal.text_muted).width(width);
    let mut table: Vec<Element<'_, Message>> = vec![
        row![
            head("MEMBER", Length::FillPortion(MEMBER_PORTION)),
            head("STATUS", Length::Fixed(STATUS_W)),
            head("CREATED", Length::Fixed(DATE_W)),
            head("JOINED", Length::Fixed(DATE_W)),
            head("ROLES", Length::FillPortion(ROLES_PORTION)),
        ]
        .spacing(COLUMN_GAP)
        .padding(crate::util::pad2(8.0, 14.0))
        .into(),
        divider(pal.hover_bg),
    ];
    for (name, online) in members {
        table.push(member_row(app, name, online, pal));
    }
    let card = container(Column::with_children(table).width(Length::Fill)).style(move |_: &Theme| round(pal.sidebar_bg, 8.0));
    let footer = text(format!("Showing {count} {}", if count == 1 { "member" } else { "members" })).size(13).color(pal.text_muted);
    scrollable(
        column![card, footer]
            .spacing(14)
            .padding(crate::util::pad2(24.0, 40.0))
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}

// The members table columns. The name and the roles share the width that is left; the status and
// the two dates have fixed widths, so their columns line up from row to row.
const MEMBER_PORTION: u16 = 3;
const ROLES_PORTION: u16 = 4;
const STATUS_W: f32 = 96.0;
const DATE_W: f32 = 160.0;
const COLUMN_GAP: f32 = 12.0;

/// One row of the members table: avatar and name, status, the two dates, then up to three of their roles.
fn member_row<'a>(app: &App, name: &String, online: bool, pal: Pal) -> Element<'a, Message> {
    let id = app.server.member_ids.get(name).cloned().unwrap_or_default();
    let (picture, banner) = app.member_id_look(name);
    let color = app.member_color(&id).unwrap_or(pal.text);
    let held: &[i64] = app.server.member_roles.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
    let roles: Vec<&RoleInfo> = app.server.roles.iter().filter(|r| r.id != EVERYONE_ROLE && held.contains(&r.id)).collect();
    let who = row![super::avatar(picture, banner, name, 28.0, &pal), text(name.clone()).size(14).color(color)]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::FillPortion(MEMBER_PORTION));
    let status = text(if online { "Online" } else { "Offline" })
        .size(13)
        .color(if online { pal.green } else { pal.text_muted })
        .width(Length::Fixed(STATUS_W));
    let created_at = app.profiles.get(&id).map(|p| p.created).unwrap_or(0);
    let created = text(crate::util::joined_label(created_at)).size(13).color(pal.text_muted).width(Length::Fixed(DATE_W));
    let joined_here = app.server.member_joined.get(&id).copied().unwrap_or(0);
    let joined = text(crate::util::joined_label(joined_here)).size(13).color(pal.text_muted).width(Length::Fixed(DATE_W));
    let mut chips = row![].spacing(6).align_y(iced::alignment::Vertical::Center).width(Length::FillPortion(ROLES_PORTION));
    for role in roles.iter().take(3) {
        let swatch = crate::util::hex_color(&role.color).unwrap_or(pal.text_muted);
        let dot = container(space().width(Length::Fixed(8.0)).height(Length::Fixed(8.0))).style(move |_: &Theme| round(swatch, 4.0));
        let chip = row![dot, text(role.name.clone()).size(12).color(pal.text)]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center);
        chips = chips.push(container(chip).padding(crate::util::pad2(2.0, 8.0)).style(move |_: &Theme| round(pal.hover_bg, 10.0)));
    }
    if roles.len() > 3 {
        chips = chips.push(text(format!("+{}", roles.len() - 3)).size(12).color(pal.text_muted));
    }
    row![who, status, created, joined, chips]
        .spacing(COLUMN_GAP)
        .padding(crate::util::pad2(8.0, 14.0))
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}

/// The roles section. Also says whether the open role has changes that are not saved.
/// Badges: the ones this server has, a delete button for each, and a form for a new one.
fn badges_section(app: &App, pal: Pal) -> (Element<'_, Message>, bool) {
    let mut items: Vec<Element<'_, Message>> = vec![
        text("BADGES").size(12).color(pal.text_muted).into(),
        text("Badges show on members' profile cards. Give them to members from a member's card.").size(13).color(pal.text_muted).into(),
    ];
    for badge in &app.server.badges {
        let members = app.server.member_badges.values().filter(|ids| ids.contains(&badge.id)).count();
        let color = crate::util::hex_color(&badge.color).unwrap_or(pal.text_strong);
        let dot = container(space()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: border::rounded(5.0),
            ..container::Style::default()
        });
        items.push(
            row![
                dot,
                text(badge.name.clone()).size(14).color(pal.text),
                text(pluralise(members, "member")).size(12).color(pal.text_muted).width(Length::Fill),
                small_button("Delete", Message::BadgeDelete(badge.id), pal),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        );
    }
    items.push(text("NEW BADGE").size(12).color(pal.text_muted).into());
    items.push(
        text_input("Badge name", &app.badge_name)
            .on_input(Message::BadgeName)
            .on_submit(Message::BadgeCreate)
            .padding(10)
            .style(input_style(pal))
            .into(),
    );
    let mut swatches = row![].spacing(8).align_y(iced::alignment::Vertical::Center);
    for hex in ROLE_COLORS.iter().take(10) {
        let color = crate::util::hex_color(hex).unwrap_or(Color::BLACK);
        let on = app.badge_color.to_lowercase() == hex.to_lowercase();
        swatches = swatches.push(
            button(space().width(Length::Fixed(24.0)).height(Length::Fixed(24.0)))
                .padding(0)
                .on_press(Message::BadgeColor(hex.to_string()))
                .style(move |_: &Theme, _: button::Status| {
                    let mut ring = border::rounded(12.0);
                    ring.color = if on { pal.text_strong } else { Color::TRANSPARENT };
                    ring.width = if on { 2.0 } else { 0.0 };
                    button::Style { background: Some(Background::Color(color)), border: ring, ..button::Style::default() }
                }),
        );
    }
    items.push(swatches.into());
    items.push(small_button("Create badge", Message::BadgeCreate, pal));
    let content = scrollable(Column::with_children(items).spacing(14).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill)).height(Length::Fill);
    (content.into(), false)
}

fn roles_section(app: &App, pal: Pal) -> (Element<'_, Message>, bool) {
    let dirty = app.role_form.as_ref().is_some_and(|draft| role_changed(app, draft));
    let selected = app.role_form.as_ref().map(|d| d.id);
    let mut list: Vec<Element<'_, Message>> = vec![
        row![
            text(format!("ROLES — {}", app.server.roles.len())).size(12).color(pal.text_muted).width(Length::Fill),
            small_button("New role", Message::RoleEdit(None), pal),
        ]
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    ];
    for role in &app.server.roles {
        list.push(role_row(role, members_with(app, role.id), selected == Some(Some(role.id)), pal));
    }
    let roles_list = container(scrollable(Column::with_children(list).spacing(4).padding(crate::util::pad2(16.0, 10.0)).width(Length::Fill)).height(Length::Fill))
        .width(Length::Fixed(240.0))
        .height(Length::Fill)
        .style(move |_: &Theme| round(pal.app_bg, 0.0));
    let editor: Element<'_, Message> = match app.role_form.as_ref() {
        Some(draft) => role_editor(app, draft, pal),
        None => container(
            column![
                text("Pick a role to edit it").size(18).color(pal.text_strong),
                muted_text("Roles give members permissions and a colour in the member list. Choose one on the left, or make a new one.", 14.0, pal.text_muted),
                solid("Create role", Message::RoleEdit(None), pal.accent, pal.on_color, None),
            ]
            .spacing(12)
            .max_width(420.0)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(crate::util::pad2(96.0, 24.0))
        .align_x(iced::alignment::Horizontal::Center)
        .into(),
    };
    let body = scrollable(container(editor).padding(crate::util::pad(24.0, 32.0, 96.0, 32.0)).max_width(640.0).width(Length::Fill)).height(Length::Fill).width(Length::Fill);
    // A thin vertical line between the list of roles and the editor.
    let rule = container(space().width(Length::Fixed(1.0)).height(Length::Fill)).height(Length::Fill).style(move |_: &Theme| round(pal.hover_bg, 0.0));
    (row![roles_list, rule, body].height(Length::Fill).into(), dirty)
}

/// How many members hold a role.
fn members_with(app: &App, role: i64) -> usize {
    app.server.member_roles.values().filter(|held| held.contains(&role)).count()
}

/// One role in the list: a colour, its name, how many hold it, and a click to edit it.
fn role_row<'a>(role: &RoleInfo, members: usize, selected: bool, pal: Pal) -> Element<'a, Message> {
    let color = crate::util::hex_color(&role.color).unwrap_or(pal.text_muted);
    let everyone = role.id == EVERYONE_ROLE;
    let subtitle = if everyone {
        "Every member has this".to_string()
    } else {
        format!("{members} {}", if members == 1 { "member" } else { "members" })
    };
    let marker = container(space().width(Length::Fixed(3.0)).height(Length::Fixed(30.0)))
        .style(move |_: &Theme| round(if selected { pal.accent } else { Color::TRANSPARENT }, 2.0));
    let dot = container(space().width(Length::Fixed(12.0)).height(Length::Fixed(12.0))).style(move |_: &Theme| round(color, 6.0));
    let label = column![
        text(role.name.clone()).size(15).color(if selected { pal.text_strong } else { pal.text }),
        text(subtitle).size(12).color(pal.text_muted),
    ]
    .spacing(2);
    button(
        row![marker, dot, label]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(crate::util::pad2(6.0, 8.0))
    .on_press(Message::RoleEdit(Some(role.id)))
    .style(move |_: &Theme, status: button::Status| {
        let bg = if selected {
            pal.selected_bg
        } else if matches!(status, button::Status::Hovered) {
            pal.hover_bg
        } else {
            Color::TRANSPARENT
        };
        button::Style { background: Some(Background::Color(bg)), text_color: pal.text, border: border::rounded(6.0), ..button::Style::default() }
    })
    .into()
}

/// A rounded card of the editor's content.
fn editor_card<'a>(items: Vec<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    container(Column::with_children(items).spacing(14).width(Length::Fill))
        .padding(crate::util::pad2(18.0, 20.0))
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.sidebar_bg, 8.0))
        .into()
}

/// The editor for the chosen role: a header with its colour, then its name, colour and position on the
/// Display tab, or its permissions on the Permissions tab, and the save and cancel buttons.
fn role_editor<'a>(app: &'a App, draft: &'a RoleDraft, pal: Pal) -> Element<'a, Message> {
    let everyone = draft.id == Some(EVERYONE_ROLE);
    // A new role is always allowed here (the server checks the permission). An existing role must sit below the viewer's highest.
    let editable = draft.id.map_or(true, |id| app.outranks_role(id));
    let swatch = crate::util::hex_color(&draft.color).unwrap_or(pal.text_muted);
    let granted = PERMISSIONS.iter().filter(|(_, _, bit)| draft.permissions & bit == *bit).count();
    let (title, detail) = match draft.id {
        Some(id) => (
            draft.name.clone(),
            format!("{} · {granted} of {} permissions", pluralise(members_with(app, id), "member"), PERMISSIONS.len()),
        ),
        None => ("New role".to_string(), "Give it a name, a colour and some permissions".to_string()),
    };
    let header = container(
        row![
            container(space().width(Length::Fixed(44.0)).height(Length::Fixed(44.0))).style(move |_: &Theme| round(swatch, 10.0)),
            column![text(title).size(20).color(pal.text_strong), text(detail).size(13).color(pal.text_muted)].spacing(4),
        ]
        .spacing(16)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(crate::util::pad2(16.0, 20.0))
    .width(Length::Fill)
    .style(move |_: &Theme| round(pal.sidebar_bg, 8.0));

    let mut items: Vec<Element<'a, Message>> = vec![header.into()];
    if !editable {
        items.push(muted_text("Your highest role is not above this one, so you cannot change it.", 13.0, pal.text_muted));
    }
    items.push(
        row![
            tab_button("Display", RoleTab::Display, app.role_tab, pal),
            tab_button("Permissions", RoleTab::Permissions, app.role_tab, pal),
        ]
        .spacing(18)
        .into(),
    );
    match app.role_tab {
        RoleTab::Display => items.push(editor_card(display_tab(app, draft, everyone, editable, pal), pal)),
        RoleTab::Permissions => items.push(editor_card(permissions_tab(app, draft, editable, pal), pal)),
    }

    // Saving and resetting an existing role are in the bar at the bottom of the page, so the footer
    // only holds creating a new role and deleting an existing one.
    let mut buttons = row![].spacing(16).align_y(iced::alignment::Vertical::Center);
    match draft.id {
        None => {
            if editable {
                buttons = buttons.push(solid("Create role", Message::RoleSave, pal.accent, pal.on_color, None));
            }
            buttons = buttons.push(link("Cancel", Message::RoleEditCancel, pal.link));
        }
        Some(id) => {
            if editable && !everyone {
                buttons = buttons.push(link("Delete role", Message::RoleDeleteAsk(id, draft.name.clone()), pal.red));
            }
        }
    }
    items.push(divider(pal.selected_bg));
    items.push(buttons.into());
    Column::with_children(items).spacing(16).width(Length::Fill).into()
}

fn pluralise(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// A tab title with an underline when it is the open one.
fn tab_button<'a>(label: &str, tab: RoleTab, active: RoleTab, pal: Pal) -> Element<'a, Message> {
    let selected = tab == active;
    let color = if selected { pal.text_strong } else { pal.text_muted };
    let underline = container(space().width(Length::Fill).height(Length::Fixed(2.0)))
        .style(move |_: &Theme| round(if selected { pal.accent } else { Color::TRANSPARENT }, 0.0));
    let title = button(text(label.to_string()).size(15).color(color))
        .padding(crate::util::pad2(4.0, 2.0))
        .on_press(Message::RoleTab(tab))
        .style(|_: &Theme, _: button::Status| button::Style { background: None, ..button::Style::default() });
    column![title, underline].spacing(4).width(Length::Shrink).into()
}

fn display_tab<'a>(app: &'a App, draft: &'a RoleDraft, everyone: bool, editable: bool, pal: Pal) -> Vec<Element<'a, Message>> {
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    if everyone {
        items.push(muted_text("Every member holds @everyone. Its colour and permissions apply to everyone.", 13.0, pal.text_muted));
    }
    let mut name = text_input("Moderator", &draft.name).padding(10).style(input_style(pal));
    if editable && !everyone {
        name = name.on_input(Message::RoleDraftName);
    }
    items.push(field("Role name", name.into()));

    items.push(text("ROLE COLOUR").size(12).color(pal.text_muted).into());
    let picked = draft.color.to_lowercase();
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for chunk in ROLE_COLORS.chunks(10) {
        let mut swatches = row![].spacing(8);
        for hex in chunk {
            let color = crate::util::hex_color(hex).unwrap_or(Color::BLACK);
            let on = picked == *hex;
            let mut swatch = button(space().width(Length::Fixed(26.0)).height(Length::Fixed(26.0)))
                .padding(0)
                .style(move |_: &Theme, _: button::Status| {
                    let mut ring = border::rounded(13.0);
                    ring.color = if on { pal.text_strong } else { Color::TRANSPARENT };
                    ring.width = if on { 2.0 } else { 0.0 };
                    button::Style { background: Some(Background::Color(color)), border: ring, ..button::Style::default() }
                });
            if editable {
                swatch = swatch.on_press(Message::RoleColorPick(hex.to_string()));
            }
            swatches = swatches.push(swatch);
        }
        rows.push(swatches.into());
    }
    items.push(Column::with_children(rows).spacing(8).into());
    if editable {
        items.push(field(
            "Custom colour",
            text_input("#rrggbb, or empty for none", &draft.color).on_input(Message::RoleDraftColor).padding(10).style(input_style(pal)).into(),
        ));
    }
    let preview = crate::util::hex_color(&draft.color).unwrap_or(pal.text_strong);
    let shown = if draft.name.trim().is_empty() { "Role name".to_string() } else { draft.name.trim().to_string() };
    items.push(
        row![text("Preview").size(13).color(pal.text_muted), text(shown).size(15).color(preview)]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
    );

    // How the role behaves for members: pinged by name, and grouped in the member list.
    if !everyone {
        items.push(divider(pal.selected_bg));
        items.push(flag_row(
            "Display role members separately",
            "Members with this role are listed in their own group in the member list.",
            draft.hoisted,
            (editable).then_some(Message::RoleDraftHoisted as fn(bool) -> Message),
            pal,
        ));
        items.push(divider(pal.selected_bg));
        items.push(flag_row(
            "Allow anyone to @mention this role",
            "Anyone can type @name to ping everyone with this role.",
            draft.mentionable,
            (editable).then_some(Message::RoleDraftMentionable as fn(bool) -> Message),
            pal,
        ));
    }

    // Moving is among the real roles. @everyone stays at the bottom.
    if let Some(id) = draft.id {
        if editable && !everyone {
            let real: Vec<i64> = app.server.roles.iter().map(|r| r.id).filter(|&r| r != EVERYONE_ROLE).collect();
            if let Some(index) = real.iter().position(|&r| r == id) {
                let mut moves = row![].spacing(12);
                if index > 0 {
                    moves = moves.push(link("Move up", Message::RoleMove(id, Some(real[index - 1])), pal.link));
                }
                if index + 1 < real.len() {
                    moves = moves.push(link("Move down", Message::RoleMove(id, real.get(index + 2).copied()), pal.link));
                }
                items.push(moves.into());
            }
        }
    }
    items
}

/// A labelled switch for a role option. `on_toggle` is `None` where the role cannot be changed.
fn flag_row<'a>(label: &'a str, hint: &'a str, on: bool, on_toggle: Option<fn(bool) -> Message>, pal: Pal) -> Element<'a, Message> {
    let mut switch = toggler(on);
    if let Some(msg) = on_toggle {
        switch = switch.on_toggle(msg);
    }
    row![
        column![text(label).size(15).color(pal.text_strong), muted_text(hint, 12.0, pal.text_muted)].spacing(4).width(Length::Fill),
        switch,
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// The channel permissions that saving `draft` would take away from the last role holding them. A
/// non-@everyone role that gives one up while no other role has it leaves nobody with it, which can
/// lock people out. Empty when the save is safe.
pub(crate) fn lockout_risk(app: &App, draft: &RoleDraft) -> Vec<&'static str> {
    let Some(id) = draft.id else { return Vec::new() };
    if id == common::EVERYONE_ROLE {
        return Vec::new();
    }
    let Some(current) = app.server.roles.iter().find(|r| r.id == id) else { return Vec::new() };
    let mut lost = Vec::new();
    for (bit, label) in [(common::perm::VIEW_CHANNEL, "View channels"), (common::perm::SEND_MESSAGES, "Message channels")] {
        let removed = current.permissions & bit != 0 && draft.permissions & bit == 0;
        let others_hold = app.server.roles.iter().any(|r| r.id != common::EVERYONE_ROLE && r.id != id && r.permissions & (bit | common::perm::ADMINISTRATOR) != 0);
        if removed && !others_hold {
            lost.push(label);
        }
    }
    lost
}

/// True when saving `draft` turns Administrator on for a role that did not have it, or creates a
/// role with it. The save asks again then, as the switch did.
pub(crate) fn grants_administrator(app: &App, draft: &RoleDraft) -> bool {
    if draft.permissions & common::perm::ADMINISTRATOR == 0 {
        return false;
    }
    match draft.id {
        None => true,
        Some(id) => !app.server.roles.iter().any(|r| r.id == id && r.permissions & common::perm::ADMINISTRATOR != 0),
    }
}

/// True when a permission must stay on for @everyone: it is one of the two channel permissions, the
/// role being edited is @everyone, and no other role has it. Mirrors the server's rule.
pub(crate) fn everyone_keeps(app: &App, role: Option<i64>, bit: u32) -> bool {
    let kept = common::perm::VIEW_CHANNEL | common::perm::SEND_MESSAGES;
    role == Some(common::EVERYONE_ROLE)
        && kept & bit != 0
        && !app.server.roles.iter().any(|r| r.id != common::EVERYONE_ROLE && r.permissions & (bit | common::perm::ADMINISTRATOR) != 0)
}

fn permissions_tab<'a>(app: &'a App, draft: &'a RoleDraft, editable: bool, pal: Pal) -> Vec<Element<'a, Message>> {
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for (index, (label, hint, bit)) in PERMISSIONS.into_iter().enumerate() {
        if index > 0 {
            items.push(divider(pal.selected_bg));
        }
        let on = draft.permissions & bit == bit;
        let mut toggle = toggler(on);
        // Only someone with the permission can grant it, and only where the role is editable.
        let everyone_admin = draft.id == Some(common::EVERYONE_ROLE) && bit == common::perm::ADMINISTRATOR;
        let locked = everyone_keeps(app, draft.id, bit) || everyone_admin;
        if editable && app.can(bit) && !locked {
            toggle = toggle.on_toggle(move |v| Message::RoleDraftPerm(bit, v));
        }
        let hint = if everyone_admin {
            "@everyone cannot be an administrator.".to_string()
        } else if locked {
            format!("{hint} Stays on until another role has it.")
        } else {
            hint.to_string()
        };
        items.push(
            row![
                column![text(label).size(15).color(pal.text_strong), muted_text(hint, 12.0, pal.text_muted)].spacing(4).width(Length::Fill),
                toggle,
            ]
            .spacing(12)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        );
    }
    items
}

/// The join-a-server dialog, opened from the + button.
pub fn join_server(app: &App, pal: Pal) -> Element<'_, Message> {
    let add = &app.add_server;
    let mut items: Vec<Element<'_, Message>> = vec![
        text("Join a Server").size(20).color(pal.text_strong).into(),
        muted_text("Enter an address below to join an existing server", 13.0, pal.text_muted),
        field(
            "Server address",
            text_input("chat.example.com", &add.addr)
                .id(ADD_ADDR_ID)
                .on_input(Message::AddServerAddr)
                .on_submit(Message::AddServerSubmit)
                .padding(10)
                .style(input_style(pal))
                .into(),
        ),
        muted_text("Addresses should look like chat.example.com or myserver.example.net", 12.0, pal.text_muted),
    ];
    if let Some(err) = &add.error {
        items.push(text(err.clone()).size(13).color(pal.red).into());
    }
    items.push(
        button(row![
            column![
                text("Don't have an address?").size(14).color(pal.text_strong),
                text("Find servers on your network and the internet").size(12).color(pal.text_muted),
            ]
            .width(Length::Fill),
            icons::icon(icons::CHEVRON_RIGHT, 14.0, pal.text_muted),
        ])
        .padding(Padding::from(10.0))
        .width(Length::Fill)
        .on_press(Message::FromAddDiscover)
        .style(move |_: &Theme, _| button::Style { background: Some(Background::Color(pal.input_bg)), text_color: pal.text, border: border::rounded(6.0), ..button::Style::default() })
        .into(),
    );
    items.push(
        row![
            space().width(Length::Fill),
            link("Back", Message::AddServerClose, pal.text_muted),
            solid("Join Server", Message::AddServerSubmit, pal.accent, pal.on_color, Some(132.0)),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center)
        .into(),
    );
    modal(items, pal, Message::AddServerClose)
}


/// The edit page of a channel, opened from its menu: its name, and what each role may do there.
pub fn channel_edit(app: &App, channel: i64, pal: Pal) -> Element<'_, Message> {
    let name = app.channel_name_of(channel).unwrap_or_default();
    let draft = app.rename_channel.as_ref().map_or("", |(_, _, new)| new.as_str());
    let dirty = app.edit_dirty();
    let section = app.edit_section;
    let content = match section {
        EditSection::Overview => settings_column(vec![
            name_field(draft, crate::app::RENAME_ID, Message::RenameInput, Message::RenameSubmit, "Channel Name", "new-channel-name", pal),
            muted_text("Messages and the channel's place in its category move with it.", 13.0, pal.text_muted),
        ]),
        EditSection::Permissions => settings_column(vec![access_page(app, crate::app::AccessTarget::Channel(channel), pal)]),
    };
    let nav = page_nav(
        &format!("#{name}"),
        vec![
            ("Overview", Message::EditSectionPick(EditSection::Overview), section == EditSection::Overview),
            ("Permissions", Message::EditSectionPick(EditSection::Permissions), section == EditSection::Permissions),
        ],
        pal,
    );
    let page = settings_frame(nav, content, Message::ChannelEditClose, pal);
    if dirty {
        iced::widget::stack![page, unsaved_bar(app, pal, Message::EditPageReset, Message::EditPageSave, "Careful – you have unsaved changes!")].into()
    } else {
        page
    }
}

/// The edit page of a category, opened from its menu: its name and permissions, and deleting it.
/// Deleting it keeps its channels, which become uncategorized.
pub fn category_edit(app: &App, pal: Pal) -> Element<'_, Message> {
    let (id, old, draft) = app.category_edit.as_ref().map_or((0, "", ""), |(id, old, new)| (*id, old.as_str(), new.as_str()));
    let dirty = app.edit_dirty();
    let section = app.edit_section;
    let delete = button(
        row![icons::icon(icons::TRASH, 16.0, pal.red), text("Delete Category").size(15).color(pal.red)]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(crate::util::pad2(6.0, 10.0))
    .on_press(Message::DeleteCategoryAsk(id, old.to_string()))
    .style(move |_: &Theme, status: button::Status| {
        let bg = if matches!(status, button::Status::Hovered) { pal.hover_bg } else { Color::TRANSPARENT };
        button::Style { background: Some(Background::Color(bg)), text_color: pal.red, border: border::rounded(4.0), ..button::Style::default() }
    });
    let nav = category_nav(
        old,
        vec![
            ("Overview", Message::EditSectionPick(EditSection::Overview), section == EditSection::Overview),
            ("Permissions", Message::EditSectionPick(EditSection::Permissions), section == EditSection::Permissions),
        ],
        delete,
        pal,
    );
    let content = match section {
        EditSection::Overview => settings_column(vec![
            text("Overview").size(20).color(pal.text_strong).into(),
            name_field(draft, crate::app::CATEGORY_EDIT_ID, Message::CategoryEditInput, Message::CategoryEditSubmit, "Category Name", "new-category-name", pal),
        ]),
        EditSection::Permissions => settings_column(vec![access_page(app, crate::app::AccessTarget::Category(id), pal)]),
    };
    let page = settings_frame(nav, content, Message::CategoryEditClose, pal);
    if dirty {
        iced::widget::stack![page, unsaved_bar(app, pal, Message::EditPageReset, Message::EditPageSave, "Careful – you have unsaved changes!")].into()
    } else {
        page
    }
}

/// The navigation of the category page: the folder and its name, its sections, then Delete at the bottom.
fn category_nav<'a>(name: &str, items: Vec<(&str, Message, bool)>, delete: impl Into<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    let header = row![
        icons::icon(icons::FOLDER, 14.0, pal.text_muted),
        text(name.to_uppercase()).size(12).color(pal.text_muted),
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Center);
    let mut list = Column::new().spacing(4);
    for (label, msg, selected) in items {
        list = list.push(nav_button(label, msg, selected, pal));
    }
    container(
        column![
            header,
            list,
            space().height(Length::Fill),
            super::divider(pal.selected_bg),
            delete.into(),
        ]
        .spacing(8),
    )
    .padding(crate::util::pad2(24.0, 12.0))
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
    .into()
}

/// The left navigation of an edit page: its title, then its items.
fn page_nav<'a>(title: &str, items: Vec<(&str, Message, bool)>, pal: Pal) -> Element<'a, Message> {
    let mut list = Column::new().spacing(4);
    for (label, msg, selected) in items {
        list = list.push(nav_button(label, msg, selected, pal));
    }
    container(column![text(title.to_uppercase()).size(12).color(pal.text_muted), list].spacing(8))
        .padding(crate::util::pad2(24.0, 12.0))
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
        .into()
}

/// A name box with its Save button.
fn name_field<'a>(
    draft: &'a str,
    id: &'static str,
    on_input: fn(String) -> Message,
    submit: Message,
    label: &'static str,
    placeholder: &'static str,
    pal: Pal,
) -> Element<'a, Message> {
    // Saved and reset from the bar at the bottom, as in server settings. Enter also saves.
    column![
        text(label).size(13).color(pal.text_muted),
        text_input(placeholder, draft)
            .id(id)
            .on_input(on_input)
            .on_submit(submit)
            .padding(10)
            .size(14)
            .width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

/// The access page of a channel or a category, as Discord lays it out: a Private switch, the roles and
/// users that can access it, and the advanced permissions of one of them at a time. A channel also
/// has a sync row that pulls its category's settings in. Category settings apply to every channel in
/// the category, and a channel moved in takes them on.
fn access_page<'a>(app: &'a App, target: crate::app::AccessTarget, pal: Pal) -> Element<'a, Message> {
    use crate::app::AccessTarget;
    use common::{Setting, Subject};
    let is_category = matches!(target, AccessTarget::Category(_));
    let noun = if is_category { "category" } else { "channel" };
    let setting = |subject: &Subject| app.access_value(target, subject.clone());
    let everyone = Subject::Role(EVERYONE_ROLE);
    let everyone_setting = setting(&everyone);
    let private = everyone_setting.view == Some(false);

    let private_note = if is_category {
        "Only the roles and users listed below can view this category. Its channels match this setting."
    } else {
        "Only the roles and users listed below can view this channel."
    };
    let private_card = container(
        column![
            row![
                icons::icon(icons::LOCK, 16.0, pal.text_strong),
                text(format!("Private {}", if is_category { "Category" } else { "Channel" })).size(15).color(pal.text_strong).width(Length::Fill),
                toggler(private).on_toggle(move |on| Message::AccessDraftSet {
                    target,
                    subject: Subject::Role(EVERYONE_ROLE),
                    setting: Setting { view: if on { Some(false) } else { None }, ..everyone_setting },
                }),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
            muted_text(private_note, 12.0, pal.text_muted),
        ]
        .spacing(8),
    )
    .padding(crate::util::pad2(14.0, 16.0))
    .width(Length::Fill)
    .style(move |_: &Theme| round(pal.sidebar_bg, 8.0));

    // Roles and members with any setting here are listed; the rest can be added.
    let members = member_entries(app);
    let mut listed: Vec<(Subject, String)> = Vec::new();
    for role in app.server.roles.iter().filter(|r| r.id != EVERYONE_ROLE) {
        let subject = Subject::Role(role.id);
        if !setting(&subject).is_empty() {
            listed.push((subject, role.name.clone()));
        }
    }
    for (id, name) in &members {
        let subject = Subject::Member(id.clone());
        if !setting(&subject).is_empty() {
            listed.push((subject, name.clone()));
        }
    }
    let mut access = Column::new().spacing(14);
    if listed.is_empty() {
        let note = if private { format!("No roles or users can view this {noun} yet.") } else { format!("Everyone can view this {noun}.") };
        access = access.push(muted_text(note, 13.0, pal.text_muted));
    }
    let mut role_rows = Column::new().spacing(10);
    let mut member_rows = Column::new().spacing(10);
    let (mut any_role, mut any_member) = (false, false);
    for (subject, name) in &listed {
        let remove = Message::AccessDraftSet { target, subject: subject.clone(), setting: Setting::default() };
        let description = match subject {
            Subject::Role(_) => "Server Role".to_string(),
            Subject::Member(id) => member_standing(app, id),
        };
        match subject {
            Subject::Role(_) => {
                let colour = role_colour(app, subject).unwrap_or(pal.text_muted);
                let dot = container(space()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(move |_: &Theme| round(colour, 6.0));
                role_rows = role_rows.push(access_row(dot.into(), name, colour, description, remove, pal));
                any_role = true;
            }
            Subject::Member(_) => {
                let initial = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                let avatar = container(text(initial).size(11).color(pal.text))
                    .width(Length::Fixed(22.0))
                    .height(Length::Fixed(22.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(move |_: &Theme| round(pal.hover_bg, 11.0));
                member_rows = member_rows.push(access_row(avatar.into(), name, pal.text, description, remove, pal));
                any_member = true;
            }
        }
    }
    if any_member {
        access = access.push(subheading("Members", pal)).push(member_rows);
    }
    if any_role {
        access = access.push(subheading("Roles", pal)).push(role_rows);
    }
    let who_head = row![
        text(format!("Who can access this {noun}?")).size(15).color(pal.text_strong).width(Length::Fill),
        solid("Add users or roles", Message::AccessAddOpen(target), pal.accent, pal.on_color, None),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center);
    let who_card = container(column![who_head, divider(pal.selected_bg), access].spacing(12))
        .padding(crate::util::pad2(14.0, 16.0))
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.rail_bg, 8.0));

    // Listed users first, then the roles in the order of the Roles page, and @everyone always last.
    let selected = app.access_subject.clone();
    let mut subject_list = Column::new().spacing(4);
    let role_item = |role: &RoleInfo| {
        let subject = Subject::Role(role.id);
        nav_button_tinted(
            &role.name,
            Message::AccessSubjectPick(subject.clone()),
            selected == subject,
            crate::util::hex_color(&role.color),
            pal,
        )
    };
    for (subject, name) in listed.iter().filter(|(s, _)| matches!(s, Subject::Member(_))) {
        subject_list = subject_list.push(nav_button(name, Message::AccessSubjectPick(subject.clone()), selected == *subject, pal));
    }
    for role in app.server.roles.iter().filter(|r| r.id != EVERYONE_ROLE) {
        subject_list = subject_list.push(role_item(role));
    }
    if let Some(everyone_role) = app.server.roles.iter().find(|r| r.id == EVERYONE_ROLE) {
        subject_list = subject_list.push(role_item(everyone_role));
    }

    let current = setting(&selected);
    // While Private is on, @everyone's View and Send are set by the switch above, so they cannot be changed here.
    let everyone_locked = private && selected == everyone;
    let view_hint = if everyone_locked {
        "Turned on or off by the Private switch."
    } else if is_category {
        "Allows members to view the channels in this category by default."
    } else {
        "Allows members to view this channel by default."
    };
    let send_hint = if everyone_locked {
        "Locked while the Private switch is on."
    } else if is_category {
        "Allows members to send messages in the channels in this category by default."
    } else {
        "Allows members to send messages in this channel by default."
    };
    let manage_hint = if is_category {
        "Lets members rename, delete and change the permissions of this category, and manage the channels in it."
    } else {
        "Lets members rename, move, delete and change the permissions of this channel."
    };
    let messages_hint = if is_category {
        "Lets members delete other members' messages in the channels in this category."
    } else {
        "Lets members delete other members' messages in this channel."
    };
    let settings = column![
        text(format!("General {} Permissions", if is_category { "Category" } else { "Channel" })).size(15).color(pal.text_strong),
        perm_row(
            "View Channels",
            view_hint,
            current.view,
            {
                let subject = selected.clone();
                move |v| Message::AccessDraftSet { target, subject: subject.clone(), setting: Setting { view: v, ..current } }
            },
            everyone_locked,
            pal,
        ),
        divider(pal.selected_bg),
        perm_row(
            "Send Messages",
            send_hint,
            current.send,
            {
                let subject = selected.clone();
                move |v| Message::AccessDraftSet { target, subject: subject.clone(), setting: Setting { send: v, ..current } }
            },
            everyone_locked,
            pal,
        ),
        divider(pal.selected_bg),
        perm_row(
            "Manage Channel",
            manage_hint,
            current.manage_channels,
            {
                let subject = selected.clone();
                move |v| Message::AccessDraftSet { target, subject: subject.clone(), setting: Setting { manage_channels: v, ..current } }
            },
            false,
            pal,
        ),
        divider(pal.selected_bg),
        perm_row(
            "Manage Messages",
            messages_hint,
            current.manage_messages,
            {
                let subject = selected.clone();
                move |v| Message::AccessDraftSet { target, subject: subject.clone(), setting: Setting { manage_messages: v, ..current } }
            },
            false,
            pal,
        ),
    ]
    .spacing(4)
    .width(Length::Fill);
    let advanced = row![container(subject_list).width(Length::Fixed(200.0)), settings].spacing(24);

    let mut page = Column::new().spacing(18);
    if let AccessTarget::Channel(channel) = target {
        page = page.push(sync_row(app, channel, pal));
    }
    page.push(private_card)
        .push(who_card)
        .push(text("Advanced permissions").size(18).color(pal.text_strong))
        .push(advanced)
        .into()
}

/// The row that pulls a channel's category settings in. They go into the channel's unsaved changes,
/// so Save applies them.
fn sync_row<'a>(app: &App, channel: i64, pal: Pal) -> Element<'a, Message> {
    let body: Element<'a, Message> = match app.category_of_channel(channel) {
        None => muted_text("This channel is not in a category, so there is nothing to sync with.", 13.0, pal.text_muted),
        Some((category, name)) => {
            let synced = app.channel_synced(channel, category);
            let status = if synced { format!("Synced with the category \"{name}\"") } else { format!("Not synced with the category \"{name}\"") };
            row![
                text(status).size(13).color(if synced { pal.green } else { pal.text_muted }).width(Length::Fill),
                solid("Sync with category", Message::ChannelSyncWithCategory(channel), pal.accent, pal.on_color, None),
            ]
            .spacing(12)
            .align_y(iced::alignment::Vertical::Center)
            .into()
        }
    };
    container(body)
        .padding(crate::util::pad2(14.0, 16.0))
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.rail_bg, 8.0))
        .into()
}

/// Every member of the server as (account id, name), by name.
pub fn member_entries(app: &App) -> Vec<(String, String)> {
    let mut members: Vec<(String, String)> = app.server.member_ids.iter().map(|(name, id)| (id.clone(), name.clone())).collect();
    members.sort_by_key(|(_, name)| name.to_lowercase());
    members
}

/// What a member is on the server, for the access lists: the owner, else their highest role, else Member.
fn member_standing(app: &App, member: &str) -> String {
    if app.server.owner_id.as_deref() == Some(member) {
        return "Server Owner".to_string();
    }
    let held = app.server.member_roles.get(member);
    app.server
        .roles
        .iter()
        .find(|r| r.id != EVERYONE_ROLE && held.is_some_and(|h| h.contains(&r.id)))
        .map_or_else(|| "Member".to_string(), |r| r.name.clone())
}

/// A small heading over one of the access lists.
fn subheading<'a>(label: &str, pal: Pal) -> Element<'a, Message> {
    text(label.to_uppercase()).size(12).color(pal.text_muted).into()
}

/// One row of an access list: its icon, its name, what it allows, and a remove button.
fn access_row<'a>(leading: Element<'a, Message>, name: &str, colour: Color, description: String, remove: Message, pal: Pal) -> Element<'a, Message> {
    row![
        leading,
        text(name.to_string()).size(14).color(colour).width(Length::Fill),
        muted_text(description, 12.0, pal.text_muted),
        button(icons::icon(icons::CLOSE, 14.0, pal.text_muted))
            .padding(crate::util::pad2(3.0, 4.0))
            .on_press(remove)
            .style(move |_: &Theme, status: button::Status| button::Style {
                background: Some(Background::Color(if matches!(status, button::Status::Hovered) { pal.hover_bg } else { Color::TRANSPARENT })),
                border: border::rounded(4.0),
                ..button::Style::default()
            }),
    ]
    .spacing(10)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// The colour a role or member shows in: a role's own colour, or none for a member.
pub fn role_colour(app: &App, subject: &common::Subject) -> Option<Color> {
    match subject {
        common::Subject::Role(id) => app.server.roles.iter().find(|r| r.id == *id).and_then(|r| crate::util::hex_color(&r.color)),
        common::Subject::Member(_) => None,
    }
}

/// One permission of the advanced list: its name and what it does, with its ✕ / slash / ✓ control.
fn perm_row<'a>(
    title: &str,
    hint: &str,
    value: Option<bool>,
    on_pick: impl Fn(Option<bool>) -> Message,
    locked: bool,
    pal: Pal,
) -> Element<'a, Message> {
    row![
        column![
            text(title.to_string()).size(14).color(if locked { pal.text_muted } else { pal.text }),
            text(hint.to_string()).size(12).color(pal.text_muted),
        ]
        .spacing(4)
        .width(Length::Fill),
        tristate(value, on_pick, locked, pal),
    ]
    .spacing(16)
    .padding(crate::util::pad2(12.0, 0.0))
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// Deny (✕), inherit (slash) or allow (✓) as three icons in one control. The chosen one is lit.
/// A locked control shows its setting but cannot be changed.
fn tristate<'a>(value: Option<bool>, on_pick: impl Fn(Option<bool>) -> Message, locked: bool, pal: Pal) -> Element<'a, Message> {
    let segment = |icon: &'static str, colour: Color, choice: Option<bool>| -> Element<'a, Message> {
        let selected = value == choice;
        let mut control = button(icons::icon(icon, 14.0, if selected { colour } else { pal.text_muted }))
            .padding(crate::util::pad2(5.0, 9.0));
        if !locked {
            control = control.on_press(on_pick(choice));
        }
        control
            .style(move |_: &Theme, status: button::Status| {
                let bg = if selected {
                    pal.selected_bg
                } else if matches!(status, button::Status::Hovered) {
                    pal.hover_bg
                } else {
                    Color::TRANSPARENT
                };
                button::Style { background: Some(Background::Color(bg)), border: border::rounded(4.0), ..button::Style::default() }
            })
            .into()
    };
    container(row![
        segment(icons::CLOSE, pal.red, Some(false)),
        segment(icons::SLASH, pal.text_strong, None),
        segment(icons::CHECK, pal.green, Some(true)),
    ]
    .spacing(2))
    .padding(2)
    .style(move |_: &Theme| round(pal.app_bg, 6.0))
    .into()
}
