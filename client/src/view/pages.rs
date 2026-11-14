//! The full pages: login, the server status page, discovery, and the settings pages.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, text, text_editor, text_input, Column};
use iced::{border, Background, Color, Element, Length, Padding, Theme};

use super::{divider, letter_badge, link, muted_text, round, server_icon, solid};
use crate::app::{App, Field, Message, ADD_ADDR_ID};
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
fn page_head<'a>(title: &str, pal: Pal, close: Message) -> Element<'a, Message> {
    row![
        text(title.to_string()).size(22).color(pal.text_strong).width(Length::Fill),
        button(icons::icon(icons::CLOSE, 16.0, pal.text_muted))
            .padding(4)
            .on_press(close)
            .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() }),
    ]
    .align_y(iced::alignment::Vertical::Center)
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
    let shown_user = acc.draft_username.clone().or_else(|| acc.username.clone()).unwrap_or_else(|| "not loaded".into());
    let email: Element<'_, Message> = match (&acc.draft_email, &acc.email, &acc.email_error) {
        (Some((e, _)), _, _) => text(e.clone()).size(14).color(pal.text).into(),
        (None, Some(e), _) => text(e.clone()).size(14).color(pal.text).into(),
        (None, None, Some(err)) => text(format!("Unavailable ({err})")).size(14).color(pal.text_muted).into(),
        _ => text("Not on record yet. Set it with Edit.").size(14).color(pal.text_muted).into(),
    };
    let theme_name = app.theme_id_name(&acc.draft_theme.clone().unwrap_or_else(|| app.prefs.theme.clone()));
    let theme_button = button(
        row![text(theme_name).size(14).color(pal.text_strong), icons::icon(icons::CHEVRON_DOWN, 14.0, pal.text_muted)]
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

    let mut main: Vec<Element<'_, Message>> = vec![
        page_head("Account", pal, Message::UserSettingsClose),
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
        setting_row("Username", None, text(format!("@{shown_user}")).size(14).color(pal.text).into(), Some(small_button("Edit", Message::AccountEdit(Field::Username), pal)), pal),
        setting_row("Email", None, email, Some(small_button("Edit", Message::AccountEdit(Field::Email), pal)), pal),
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
    let page = column![
        row![settings_nav("User settings", "Account", pal), content].height(Length::Fill),
        if dirty { unsaved_bar(pal, Message::AccountReset, Message::AccountSave, "Careful – you have unsaved changes!") } else { space().height(Length::Fixed(0.0)).into() },
    ]
    .height(Length::Fill);
    let page = container(page).width(Length::Fill).height(Length::Fill).style(move |_: &Theme| round(pal.app_bg, 0.0));
    match acc.editing {
        Some(which) => stack_dialog(page.into(), account_edit_dialog(app, which, pal), pal),
        None => page.into(),
    }
}

/// The bar that appears when there are changes to save.
fn unsaved_bar<'a>(pal: Pal, reset: Message, save: Message, title: &str) -> Element<'a, Message> {
    container(row![
        text(title.to_string()).size(14).color(pal.text_strong).width(Length::Fill),
        link("Reset", reset, pal.link),
        solid("Save Changes", save, pal.green, pal.on_color, None),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center))
    .padding(crate::util::pad2(10.0, 16.0))
    .style(move |_: &Theme| round(pal.sidebar_bg, 0.0))
    .into()
}

/// Puts a dialog over a page, with a dimmed backdrop that is not clickable.
fn stack_dialog<'a>(page: Element<'a, Message>, dialog: Element<'a, Message>, pal: Pal) -> Element<'a, Message> {
    let _ = pal;
    iced::widget::stack![page, dialog].into()
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

/// The server's own settings page: the owner's name, description and picture.
pub fn server_settings(app: &App, pal: Pal) -> Element<'_, Message> {
    let form = &app.server_form;
    let name = app.selected_name().unwrap_or_else(|| "Server".into());
    let dirty = (!form.name.trim().is_empty() && form.name.trim() != name) || form.description.trim() != app.server.description;
    let icon: Element<'_, Message> = server_icon(app.server.icon.as_deref(), &name, 64.0, &pal, 8.0);
    let mut items: Vec<Element<'_, Message>> = vec![
        page_head("Server Profile", pal, Message::ServerSettingsClose),
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
    let content = scrollable(Column::with_children(items).spacing(14).padding(crate::util::pad2(24.0, 40.0)).width(Length::Fill)).height(Length::Fill);
    let bottom: Element<'_, Message> = if dirty {
        unsaved_bar(pal, Message::ServerReset, Message::ServerSave, "Careful – you have unsaved changes!")
    } else {
        space().height(Length::Fixed(0.0)).into()
    };
    container(column![
        row![settings_nav(&name.to_uppercase(), "Server Profile", pal), content].height(Length::Fill),
        bottom,
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_: &Theme| round(pal.app_bg, 0.0))
    .into()
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

