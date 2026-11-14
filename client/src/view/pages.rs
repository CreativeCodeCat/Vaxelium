//! The full pages: login, the server status page, discovery, and the settings pages.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, text, text_editor, text_input, toggler, Column};
use iced::{border, Background, Color, Element, Length, Padding, Theme};

use super::{divider, letter_badge, link, muted_text, round, server_icon, solid};
use crate::app::{App, Field, Message, RoleDraft, RoleTab, ServerSection, ADD_ADDR_ID};
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

fn input_style(pal: Pal) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
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
        let mut info = Column::new().spacing(2).push(text(f.name.clone()).size(16).color(pal.text_strong));
        if !f.description.is_empty() {
            info = info.push(text(f.description.clone()).size(13).color(pal.text));
        }
        info = info.push(
            row![
                text(f.addr.clone()).size(12).color(pal.text_muted),
                text(format!("· {} online", f.online)).size(12).color(pal.green),
            ]
            .spacing(6),
        );
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
fn settings_nav<'a>(name: &str, section: &str, pal: Pal) -> Element<'a, Message> {
    container(column![
        text(name.to_uppercase()).size(12).color(pal.text_muted),
        container(text(section.to_string()).size(15).color(pal.text_strong))
            .padding(crate::util::pad2(6.0, 10.0))
            .width(Length::Fill)
            .style(move |_: &Theme| round(pal.selected_bg, 4.0)),
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
            letter_badge(&user, 72.0, &pal),
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
    let content = settings_column(main);
    let dirty = acc.draft_name.is_some() || acc.draft_username.is_some() || acc.draft_email.is_some() || acc.draft_theme.is_some();
    let page = settings_frame(settings_nav("User settings", "Account", pal), content, Message::UserSettingsClose, pal);
    let mut layers: Vec<Element<'_, Message>> = vec![page];
    if dirty {
        layers.push(unsaved_bar(pal, Message::AccountReset, Message::AccountSave, "Careful – you have unsaved changes!"));
    }
    if let Some(which) = acc.editing {
        layers.push(account_edit_dialog(app, which, pal));
    }
    iced::widget::stack(layers).into()
}

/// The bar that appears when there are changes to save. It floats over the bottom of the page,
/// centred, at most 640 pixels wide, as it did in the old client.
fn unsaved_bar<'a>(pal: Pal, reset: Message, save: Message, title: &str) -> Element<'a, Message> {
    let card = container(row![
        text(title.to_string()).size(14).color(pal.text_strong).width(Length::Fill),
        link("Reset", reset, pal.link),
        solid("Save Changes", save, pal.green, pal.on_color, None),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center))
    .padding(crate::util::pad2(12.0, 16.0))
    .max_width(640.0)
    .style(move |_: &Theme| round(pal.rail_bg, 8.0));
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
    let section = if app.is_owner() { app.server_section } else { ServerSection::Roles };
    let (content, dirty): (Element<'_, Message>, bool) = match section {
        ServerSection::Profile => {
            let (items, dirty) = profile_items(app, pal);
            let content = scrollable(Column::with_children(items).spacing(14).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill)).height(Length::Fill);
            (content.into(), dirty)
        }
        ServerSection::Roles => roles_section(app, pal),
    };
    // The bar saves whichever section is open.
    let (reset, save) = if section == ServerSection::Roles {
        (Message::RoleReset, Message::RoleSave)
    } else {
        (Message::ServerReset, Message::ServerSave)
    };
    let page = settings_frame(server_nav(&name, app.is_owner(), section, pal), content, Message::ServerSettingsClose, pal);
    if dirty {
        iced::widget::stack![page, unsaved_bar(pal, reset, save, "Careful – you have unsaved changes!")].into()
    } else {
        page.into()
    }
}

/// The navigation down the left of the server settings page.
fn server_nav<'a>(name: &str, owner: bool, active: ServerSection, pal: Pal) -> Element<'a, Message> {
    let mut items = Column::new().spacing(4);
    if owner {
        items = items.push(nav_item("Server Profile", ServerSection::Profile, active, pal));
    }
    items = items.push(nav_item("Roles", ServerSection::Roles, active, pal));
    container(column![text(name.to_uppercase()).size(12).color(pal.text_muted), items].spacing(8))
        .padding(crate::util::pad2(24.0, 12.0))
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
        .into()
}

fn nav_item<'a>(label: &str, section: ServerSection, active: ServerSection, pal: Pal) -> Element<'a, Message> {
    let selected = section == active;
    let color = if selected { pal.text_strong } else { pal.text_muted };
    button(text(label.to_string()).size(15).color(color))
        .width(Length::Fill)
        .padding(crate::util::pad2(6.0, 10.0))
        .on_press(Message::ServerSectionPick(section))
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
fn profile_items(app: &App, pal: Pal) -> (Vec<Element<'_, Message>>, bool) {
    let form = &app.server_form;
    let name = app.selected_name().unwrap_or_else(|| "Server".into());
    let dirty = (!form.name.trim().is_empty() && form.name.trim() != name) || form.description.trim() != app.server.description;
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
const PERMISSIONS: [(&str, &str, u32); 4] = [
    ("Manage channels", "Allows members to create, rename, move and delete channels and categories.", common::perm::MANAGE_CHANNELS),
    ("Manage roles", "Allows members to create and edit roles below their own, and give them to members below them.", common::perm::MANAGE_ROLES),
    ("Manage messages", "Allows members to delete other members' messages.", common::perm::MANAGE_MESSAGES),
    ("Bypass ignore", "Members with this cannot be ignored or blocked by anyone.", common::perm::BYPASS_IGNORE),
];

/// The Roles section, laid out like Discord's: the roles on the left, and the chosen role's settings on the right.
/// True when the editor's draft differs from the role as the server has it, so there is something to save.
fn role_changed(app: &App, draft: &RoleDraft) -> bool {
    if !draft.id.map_or(true, |id| app.outranks_role(id)) {
        return false;
    }
    match draft.id.and_then(|id| app.server.roles.iter().find(|r| r.id == id)) {
        Some(role) => role.name != draft.name.trim() || role.color != draft.color.trim() || role.permissions != draft.permissions,
        // A new role is made with its own Create button, so it has no bar.
        None => false,
    }
}

/// The roles section. Also says whether the open role has changes that are not saved.
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
    (row![roles_list, body].height(Length::Fill).into(), dirty)
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

fn permissions_tab<'a>(app: &'a App, draft: &'a RoleDraft, editable: bool, pal: Pal) -> Vec<Element<'a, Message>> {
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for (index, (label, hint, bit)) in PERMISSIONS.into_iter().enumerate() {
        if index > 0 {
            items.push(divider(pal.selected_bg));
        }
        let on = draft.permissions & bit == bit;
        let mut toggle = toggler(on);
        // Only someone with the permission can grant it, and only where the role is editable.
        if editable && app.can(bit) {
            toggle = toggle.on_toggle(move |v| Message::RoleDraftPerm(bit, v));
        }
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

