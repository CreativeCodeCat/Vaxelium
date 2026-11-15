//! Popups drawn over the window: the context menus, the emoji picker, the dialogs, and the edges
//! that resize the window.

use iced::widget::{button, checkbox, column, container, mouse_area, row, scrollable, space, text, text_input, tooltip, Column};
use iced::{border, window, Background, Color, Element, Length, Padding, Theme};

use super::chat::emoji;
use super::pages::{input_style, modal};
use super::{link, solid};
use crate::app::{App, Menu, MenuItem, Message, Picker};
use crate::icons;
use crate::theme::Pal;

const TONE_NAMES: [&str; 6] = [
    "Default",
    "Light skin tone",
    "Medium-light skin tone",
    "Medium skin tone",
    "Medium-dark skin tone",
    "Dark skin tone",
];

/// A full-window layer that closes when pressed, and holds one popup at a point.
fn anchored<'a>(popup: Element<'a, Message>, at: iced::Point, close: Message) -> Element<'a, Message> {
    // The backdrop reports a pointer cursor, so the stack hides the cursor from the layers below. That
    // keeps messages underneath from being hovered while the popup is up.
    let backdrop = mouse_area(container(space()).width(Length::Fill).height(Length::Fill))
        .on_press(close)
        .interaction(iced::mouse::Interaction::Idle);
    let placed = container(popup)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::default().top(at.y).left(at.x));
    iced::widget::stack![backdrop, placed].into()
}

/// Roughly how tall a menu is drawn: rows are about 30 pixels, headings and dividers less. Used to
/// keep the menu on screen before it is drawn.
fn menu_height(menu: &Menu) -> f32 {
    let rows: f32 = menu
        .items
        .iter()
        .map(|item| match item {
            MenuItem::Divider => 9.0,
            MenuItem::Heading(_) => 28.0,
            MenuItem::Action { .. } | MenuItem::Roles { .. } => 30.0,
        })
        .sum();
    rows + 10.0
}

/// Moves a point back so a popup of `size` stays inside the window, with a margin at each edge.
fn keep_on_screen(at: iced::Point, size: iced::Size, window: iced::Size) -> iced::Point {
    const EDGE: f32 = 8.0;
    let x = at.x.min(window.width - size.width - EDGE).max(EDGE);
    let y = at.y.min(window.height - size.height - EDGE).max(EDGE);
    iced::Point::new(x, y)
}

/// Like `anchored`, but the popup's bottom edge sits 8 pixels above `at.y`, so it grows upward.
fn anchored_above<'a>(popup: Element<'a, Message>, at: iced::Point, window_height: f32, close: Message) -> Element<'a, Message> {
    let backdrop = mouse_area(container(space()).width(Length::Fill).height(Length::Fill))
        .on_press(close)
        .interaction(iced::mouse::Interaction::Idle);
    let placed = container(popup)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding::default().bottom(window_height - at.y + 8.0).left(at.x));
    iced::widget::stack![backdrop, placed].into()
}

/// The context menu, when one is open.
pub fn menu_layer(app: &App, pal: Pal) -> Option<Element<'_, Message>> {
    let menu: &Menu = app.menu.as_ref()?;
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    for item in &menu.items {
        match item {
            MenuItem::Divider => items.push(super::divider(pal.selected_bg)),
            MenuItem::Heading(label) => items.push(
                container(text(label.to_uppercase()).size(11).color(pal.text_muted))
                    .padding(crate::util::pad(6.0, 10.0, 2.0, 10.0))
                    .into(),
            ),
            MenuItem::Roles { .. } => {
                // The row only opens the flyout beside the menu (see below).
                let chevron = icons::CHEVRON_RIGHT;
                items.push(
                    mouse_area(button(
                        row![
                            text("Roles").size(14).color(pal.text).width(Length::Fill),
                            icons::icon(chevron, 12.0, pal.text_muted),
                        ]
                        .align_y(iced::alignment::Vertical::Center),
                    )
                    .padding(crate::util::pad2(6.0, 10.0))
                    .width(Length::Fill)
                    .on_press(Message::RolesMenuOpen)
                    .style(move |_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => Some(Background::Color(pal.accent)),
                            _ => None,
                        };
                        let text_color = match status {
                            button::Status::Hovered => pal.text_strong,
                            _ => pal.text,
                        };
                        button::Style { background: bg, text_color, border: border::rounded(4.0), ..button::Style::default() }
                    })
                    )
                    .on_enter(Message::RolesMenuOpen)
                    .into(),
                );
            }
            MenuItem::Action { label, icon, danger, msg } => {
                let color = if *danger { pal.red } else { pal.text };
                let mut r = row![].spacing(10).align_y(iced::alignment::Vertical::Center);
                if let Some(icon) = icon {
                    r = r.push(icons::icon(icon, 14.0, color));
                }
                r = r.push(text(label.clone()).size(14).color(color));
                items.push(
                    mouse_area(button(r)
                        .padding(crate::util::pad2(6.0, 10.0))
                        .width(Length::Fill)
                        .on_press(msg.clone())
                        .style(move |_: &Theme, status: button::Status| {
                            let bg = match status {
                                button::Status::Hovered => Some(Background::Color(if *danger { pal.red } else { pal.accent })),
                                _ => None,
                            };
                            let text_color = match status {
                                button::Status::Hovered if *danger => pal.on_color,
                                button::Status::Hovered => pal.text_strong,
                                _ => color,
                            };
                            button::Style { background: bg, text_color, border: border::rounded(4.0), ..button::Style::default() }
                        })
                    )
                    .on_enter(Message::RolesMenuClose)
                    .into(),
                );
            }
        }
    }
    let width = menu.width.unwrap_or(200.0);
    let height = menu_height(menu);
    let body = container(Column::with_children(items).padding(4))
        .width(Length::Fixed(width))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    // The Roles flyout sits beside the menu, in the same overlay, so the menu's backdrop cannot take its clicks.
    let member = menu.items.iter().find_map(|item| match item {
        MenuItem::Roles { member } => Some(member.as_str()),
        _ => None,
    });
    // Opens to the left of the menu, unless the menu is too close to the left edge to fit it.
    const FLYOUT_WIDTH: f32 = 200.0;
    let flyout_open = app.roles_menu_open && member.is_some();
    if menu.centered {
        // Member menus sit over the member list, centred in it. The Roles flyout goes just to its left.
        let at = keep_on_screen(menu.at, iced::Size::new(0.0, height), app.window_size);
        let panel = app.panel_width("members");
        let flyout: Element<'_, Message> = match (app.roles_menu_open, member) {
            (true, Some(member)) => roles_flyout(app, member, pal),
            _ => space().width(Length::Fixed(0.0)).into(),
        };
        let centred = container(body).width(Length::Fixed(panel)).align_x(iced::alignment::Horizontal::Center);
        let content: Element<'_, Message> = row![space().width(Length::Fill), flyout, centred].into();
        return Some(anchored(content, iced::Point::new(0.0, at.y), Message::CloseMenu));
    }
    // The menu and its flyout must fit on screen together, so the point is moved back if they would not.
    let total_width = width + if flyout_open { FLYOUT_WIDTH + 6.0 } else { 0.0 };
    let at = keep_on_screen(menu.at, iced::Size::new(total_width, height), app.window_size);
    let left_fits = at.x >= FLYOUT_WIDTH + 6.0;
    let (content, anchor): (Element<'_, Message>, iced::Point) = match (app.roles_menu_open, member) {
        // The row is anchored at the flyout's left edge, so the menu itself stays where it was opened.
        (true, Some(member)) if left_fits => (
            row![roles_flyout(app, member, pal), body].spacing(6).into(),
            iced::Point::new(at.x - FLYOUT_WIDTH - 6.0, at.y),
        ),
        (true, Some(member)) => (row![body, roles_flyout(app, member, pal)].spacing(6).into(), at),
        _ => (body.into(), at),
    };
    Some(anchored(content, anchor, Message::CloseMenu))
}

/// The checklist of roles for one member, shown beside their menu. Each checkbox reads the
/// member's roles when it is drawn, so a change from anyone shows straight away.
fn roles_flyout<'a>(app: &'a App, member: &str, pal: Pal) -> Element<'a, Message> {
    let held = app.server.member_roles.get(member).cloned().unwrap_or_default();
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for role in app.server.roles.iter().filter(|r| r.id != common::EVERYONE_ROLE && app.outranks_role(r.id)) {
        let on = held.contains(&role.id);
        let who = member.to_string();
        let role_id = role.id;
        let role_color = crate::util::hex_color(&role.color).unwrap_or(pal.text);
        rows.push(
            container(
                checkbox(on)
                    .label(role.name.clone())
                    .style(move |theme: &Theme, status| {
                        // The label takes the role's colour; the box keeps the standard look.
                        let mut style = iced::widget::checkbox::primary(theme, status);
                        style.text_color = Some(role_color);
                        style
                    })
                    .on_toggle(move |v| Message::SetMemberRole(who.clone(), role_id, v)),
            )
            .padding(crate::util::pad2(6.0, 10.0))
            .into(),
        );
    }
    container(scrollable(Column::with_children(rows).spacing(2).padding(4)).height(Length::Shrink))
        .max_height(320.0)
        .width(Length::Fixed(200.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        })
        .into()
}

/// A small × that removes the role or badge it sits on when pressed. Hovering shows what it does.
fn remove_button<'a>(msg: Message, label: &'static str, pal: Pal) -> Element<'a, Message> {
    let x = button(icons::icon(icons::CLOSE, 10.0, pal.text_muted))
        .padding(0)
        .on_press(msg)
        .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() });
    let bubble = container(text(label).size(12).color(pal.text_strong))
        .padding(crate::util::pad2(4.0, 8.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(6.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    tooltip(x, bubble, tooltip::Position::Top).gap(6).into()
}

/// One row of a pick list: a role or badge, its colour, and the message a click sends.
struct PickRow {
    name: String,
    color: Color,
    msg: Message,
}

/// A list to pick from, shown on a card: a search box, then a row per item. Clicking a row gives that item.
fn pick_list<'a>(placeholder: &str, search: &str, rows: Vec<PickRow>, pal: Pal) -> Element<'a, Message> {
    let query = search.trim().to_lowercase();
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for row_data in rows.into_iter().filter(|r| query.is_empty() || r.name.to_lowercase().contains(&query)) {
        let color = row_data.color;
        let dot = container(space()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: border::rounded(5.0),
            ..container::Style::default()
        });
        items.push(
            button(
                row![dot, text(row_data.name).size(13).color(pal.text).width(Length::Fill)]
                    .spacing(10)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .padding(crate::util::pad2(6.0, 10.0))
            .width(Length::Fill)
            .on_press(row_data.msg)
            .style(move |_: &Theme, status: button::Status| button::Style {
                background: match status {
                    button::Status::Hovered => Some(Background::Color(pal.hover_bg)),
                    _ => None,
                },
                text_color: pal.text,
                border: border::rounded(4.0),
                ..button::Style::default()
            })
            .into(),
        );
    }
    if items.is_empty() {
        items.push(text("Nothing to show.").size(12).color(pal.text_muted).into());
    }
    let search_box = text_input(placeholder, search)
        .on_input(Message::CardListSearch)
        .padding(8)
        .size(13)
        .style(input_style(pal));
    let list = scrollable(Column::with_children(items).spacing(2).padding(4)).height(Length::Shrink);
    container(Column::new().push(search_box).push(container(list).max_height(240.0)).spacing(6).padding(6))
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        })
        .into()
}

/// The emoji picker for one message, when it is open.
pub fn picker_layer(app: &App, pal: Pal) -> Option<Element<'_, Message>> {
    let p: &Picker = app.picker.as_ref()?;
    let tone = app.prefs.skin_tone;
    let mut tones = row![].spacing(2);
    for i in 0..TONE_NAMES.len() {
        let sample = if i == 0 {
            "👋".to_string()
        } else {
            app.skin.get("👋").map(|v| v[i - 1].clone()).unwrap_or_else(|| "👋".into())
        };
        let selected = i == tone;
        tones = tones.push(
            button(emoji(&sample, 20.0))
                .padding(2)
                .on_press(Message::PickerTone(i))
                .style(move |_: &Theme, _| button::Style {
                    background: None,
                    border: border::rounded(6.0).color(if selected { pal.accent } else { Color::TRANSPARENT }).width(2.0),
                    ..button::Style::default()
                }),
        );
    }
    let query = app.picker_query.trim().to_lowercase();
    let mut grid: Vec<Element<'_, Message>> = Vec::new();
    let mut categories: Vec<&str> = Vec::new();
    for (cat, _, _) in &app.emoji {
        if !categories.contains(&cat.as_str()) {
            categories.push(cat.as_str());
        }
    }
    for cat in categories {
        let items: Vec<(&String, &String)> = app
            .emoji
            .iter()
            .filter(|(c, name, _)| c == cat && (query.is_empty() || name.to_lowercase().contains(&query)))
            .map(|(_, name, base)| (name, base))
            .collect();
        if items.is_empty() {
            continue;
        }
        grid.push(text(cat.to_uppercase()).size(11).color(pal.text_muted).into());
        let mut line = row![].spacing(2);
        let mut count = 0;
        for (_, base) in items {
            let variant = app.skin.get(base.as_str());
            let e = match variant {
                Some(v) if tone > 0 => v[tone - 1].clone(),
                _ => base.clone(),
            };
            line = line.push(
                button(emoji(&e, 24.0))
                    .padding(4)
                    .on_press(Message::React(p.message, e.clone()))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.25))),
                            _ => None,
                        };
                        button::Style { background: bg, border: border::rounded(4.0), ..button::Style::default() }
                    }),
            );
            count += 1;
            if count % 8 == 0 {
                grid.push(line.into());
                line = row![].spacing(2);
            }
        }
        grid.push(line.into());
    }
    if grid.is_empty() {
        grid.push(text("No emoji match.").size(13).color(pal.text_muted).into());
    }
    let panel = container(
        column![
            text_input("Search emoji", &app.picker_query)
                .on_input(Message::PickerQuery)
                .padding(8)
                .size(14),
            tones,
            scrollable(Column::with_children(grid).spacing(4)).height(Length::Fixed(320.0)),
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Length::Fixed(340.0))
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(pal.sidebar_bg)),
        border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
        ..container::Style::default()
    });
    Some(anchored(panel.into(), p.at, Message::PickerClose))
}

/// The dialogs: delete a message, and joining a server.
pub fn dialog_layer(app: &App, pal: Pal) -> Option<Element<'_, Message>> {
    if app.admin_grant_pending {
        let items: Vec<Element<'_, Message>> = vec![
            text("Give Administrator?").size(20).color(pal.text_strong).into(),
            text("Administrators have every permission, including ones switched off in this role or anywhere else. Only give it to people you fully trust.")
                .size(14)
                .color(pal.text)
                .into(),
            row![
                space().width(Length::Fill),
                link("Cancel", Message::AdminGrantCancel, pal.text_muted),
                solid("Turn on", Message::AdminGrantConfirm, pal.red, pal.on_color, None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        ];
        return Some(modal(items, pal, Message::AdminGrantCancel));
    }
    if app.role_save_warning {
        if let Some(draft) = &app.role_form {
            let lost = crate::view::pages::lockout_risk(app, draft);
            let mut paragraphs: Vec<Element<'_, Message>> = vec![text("Save this role?").size(20).color(pal.text_strong).into()];
            if crate::view::pages::grants_administrator(app, draft) {
                paragraphs.push(
                    text("This gives Administrator, which has every permission, including ones switched off. Only give it to people you fully trust.")
                        .size(14)
                        .color(pal.text)
                        .into(),
                );
            }
            if !lost.is_empty() {
                let which = if lost.len() == 1 { lost[0] } else { "them" };
                paragraphs.push(
                    text(format!(
                        "This takes {} away from {}. No other role has {which}. At least one role must have {which} or you can be locked out.",
                        lost.join(" and "),
                        draft.name.trim(),
                    ))
                    .size(14)
                    .color(pal.text)
                    .into(),
                );
            }
            let items: Vec<Element<'_, Message>> = paragraphs.into_iter().chain(std::iter::once(
                row![
                    space().width(Length::Fill),
                    link("Cancel", Message::RoleSaveCancel, pal.text_muted),
                    solid("Save anyway", Message::RoleSaveConfirm, pal.red, pal.on_color, None),
                ]
                .spacing(10)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
            )).collect();
            return Some(modal(items, pal, Message::RoleSaveCancel));
        }
    }
    if let Some(channel) = app.channel_access {
        return Some(channel_access_dialog(app, channel, pal));
    }
    if app.account.verify_open {
        let acc = &app.account;
        let address = app.account.email.clone().unwrap_or_else(|| "your address".into());
        let mut items: Vec<Element<'_, Message>> = vec![
            text("Confirm your email").size(20).color(pal.text_strong).into(),
            text(format!("Enter the 6-digit code we emailed to {address}. It expires in 15 minutes."))
                .size(14)
                .color(pal.text)
                .into(),
            text_input("123456", &acc.verify_code)
                .on_input(Message::VerifyCodeInput)
                .on_submit(Message::VerifySubmit)
                .padding(10)
                .into(),
        ];
        if let Some(error) = &acc.verify_error {
            items.push(text(error.clone()).size(13).color(pal.red).into());
        }
        items.push(
            row![
                link("Send a new code", Message::VerifyOpen, pal.link),
                space().width(Length::Fill),
                link("Cancel", Message::VerifyCancel, pal.text_muted),
                solid("Confirm", Message::VerifySubmit, pal.accent, pal.on_color, None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        );
        return Some(modal(items, pal, Message::VerifyCancel));
    }
    if app.pending_delete.is_some() {
        let items: Vec<Element<'_, Message>> = vec![
            text("Delete message?").size(20).color(pal.text_strong).into(),
            text("This message will be removed for everyone. It can't be undone.").size(14).color(pal.text).into(),
            row![
                space().width(Length::Fill),
                link("Cancel", Message::DeleteCancel, pal.text_muted),
                solid("Delete", Message::DeleteConfirm, pal.red, pal.on_color, None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        ];
        return Some(modal(items, pal, Message::DeleteCancel));
    }
    if app.add_server.open {
        return Some(super::pages::join_server(app, pal));
    }
    if let Some((_, name)) = &app.pending_role_delete {
        let items: Vec<Element<'_, Message>> = vec![
            text(format!("Delete role {name}?")).size(20).color(pal.text_strong).into(),
            text("Members with this role will lose it. Its permissions go with it. It can't be undone.")
                .size(14)
                .color(pal.text)
                .into(),
            row![
                space().width(Length::Fill),
                link("Cancel", Message::RoleDeleteCancel, pal.text_muted),
                solid("Delete", Message::RoleDeleteConfirm, pal.red, pal.on_color, None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        ];
        return Some(modal(items, pal, Message::RoleDeleteCancel));
    }
    if let Some((_, name)) = &app.pending_channel_delete {
        let items: Vec<Element<'_, Message>> = vec![
            text(format!("Delete #{name}?")).size(20).color(pal.text_strong).into(),
            text("The channel and every message in it will be removed for everyone. It can't be undone.")
                .size(14)
                .color(pal.text)
                .into(),
            row![
                space().width(Length::Fill),
                link("Cancel", Message::DeleteChannelCancel, pal.text_muted),
                solid("Delete", Message::DeleteChannelConfirm, pal.red, pal.on_color, None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        ];
        return Some(modal(items, pal, Message::DeleteChannelCancel));
    }
    if let Some((_, old, new)) = &app.rename_channel {
        let items: Vec<Element<'_, Message>> = vec![
            text(format!("Rename #{old}")).size(20).color(pal.text_strong).into(),
            text_input("new-channel-name", new)
                .id(crate::app::RENAME_ID)
                .on_input(Message::RenameInput)
                .on_submit(Message::RenameSubmit)
                .padding(10)
                .size(14)
                .into(),
            text("Messages and the channel's place in its category move with it.").size(12).color(pal.text_muted).into(),
            row![
                space().width(Length::Fill),
                link("Cancel", Message::RenameCancel, pal.text_muted),
                solid("Save", Message::RenameSubmit, pal.accent, pal.on_color, Some(96.0)),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        ];
        return Some(modal(items, pal, Message::RenameCancel));
    }
    None
}

/// Thin strips along the window's edges and corners, which resize the window when dragged.
pub fn resize_edges<'a>() -> Element<'a, Message> {
    const EDGE: f32 = 4.0;
    const CORNER: f32 = 10.0;
    let strip = |w: Length, h: Length, dir: window::Direction| -> Element<'a, Message> {
        mouse_area(container(space()).width(w).height(h))
            .on_press(Message::Resize(dir))
            .interaction(iced::mouse::Interaction::ResizingDiagonallyDown)
            .into()
    };
    let left = container(strip(Length::Fixed(EDGE), Length::Fill, window::Direction::West)).align_left(Length::Fill);
    let right = container(strip(Length::Fixed(EDGE), Length::Fill, window::Direction::East)).align_right(Length::Fill);
    let top = container(strip(Length::Fill, Length::Fixed(EDGE), window::Direction::North)).align_top(Length::Fill);
    let bottom = container(strip(Length::Fill, Length::Fixed(EDGE), window::Direction::South)).align_bottom(Length::Fill);
    let corner = |dir: window::Direction, h: iced::alignment::Horizontal, v: iced::alignment::Vertical| -> Element<'a, Message> {
        container(strip(Length::Fixed(CORNER), Length::Fixed(CORNER), dir))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(h)
            .align_y(v)
            .into()
    };
    iced::widget::stack![
        left,
        right,
        top,
        bottom,
        corner(window::Direction::NorthWest, iced::alignment::Horizontal::Left, iced::alignment::Vertical::Top),
        corner(window::Direction::NorthEast, iced::alignment::Horizontal::Right, iced::alignment::Vertical::Top),
        corner(window::Direction::SouthWest, iced::alignment::Horizontal::Left, iced::alignment::Vertical::Bottom),
        corner(window::Direction::SouthEast, iced::alignment::Horizontal::Right, iced::alignment::Vertical::Bottom),
    ]
    .into()
}

/// Lines that start with "- " or "* " become bullet points in a bio.
fn with_bullets(body: &str) -> String {
    body.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            match trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                Some(rest) => format!("• {rest}"),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The profile card, when one is open: a banner, the name, the bio and the member's roles here.
pub fn card_layer(app: &App, pal: Pal) -> Option<Element<'_, Message>> {
    let view = app.card.as_ref()?;
    let card = view.card.as_ref();
    let name = card.map_or(view.name.clone(), |c| c.name.clone());
    let banner = card.and_then(|c| crate::util::hex_color(&c.banner)).unwrap_or(pal.accent);
    // Every card has the same tall banner and large picture, so your card and other members' match.
    let own = view.id == app.user_id;
    let banner_height = 96.0;
    // The banner follows the card's rounded top corners.
    let banner_block = container(space())
        .width(Length::Fill)
        .height(Length::Fixed(banner_height))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(banner)),
            border: border::Border {
                radius: border::Radius { top_left: 8.0, top_right: 8.0, bottom_right: 0.0, bottom_left: 0.0 },
                ..border::Border::default()
            },
            ..container::Style::default()
        });
    let picture = card.map_or(String::new(), |c| c.avatar.clone());
    let banner_hex = card.map_or(String::new(), |c| c.banner.clone());
    // The status bubble. On your own card it can be edited, with the pencil and the trash, and it sits
    // under the name. On other members' cards it sits beside the avatar.
    let status = card.map(|c| c.status.clone()).unwrap_or_default();
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    // The role or badge list opened from a +. It floats where the + was clicked, over the card.
    let mut pick_popup: Option<(Element<'_, Message>, Message)> = None;
    let name_size = 20.0;
    // The picture overlaps the bottom of the banner, in a ring of the card's colour, as on Discord. The
    // base layer is as tall as the whole header, since an overlay is clipped to the base's height.
    let picture_el = super::avatar(&picture, &banner_hex, &name, 88.0, &pal);
    // A dot shows the member's presence at the picture's bottom right, as in the member list. Your own
    // card shows your status; an offline member has no dot.
    let presence = if own {
        Some(app.effective_status())
    } else {
        app.server.member_ids.get(&name).and_then(|id| app.server.member_status.get(id)).copied()
    };
    let picture_with_dot: Element<'_, Message> = match presence.and_then(|p| super::chat::status_color(p, &pal)) {
        Some(color) => {
            let dot = container(space())
                .width(Length::Fixed(18.0))
                .height(Length::Fixed(18.0))
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(color)),
                    border: border::rounded(9.0).color(pal.sidebar_bg).width(3.0),
                    ..container::Style::default()
                });
            iced::widget::stack![picture_el, container(dot).padding(crate::util::pad(70.0, 0.0, 0.0, 70.0))].into()
        }
        None => picture_el.into(),
    };
    let ring = container(picture_with_dot).padding(4).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(pal.sidebar_bg)),
        border: border::rounded(48.0),
        ..container::Style::default()
    });
    // Other members' status bubble sits beside the picture, at its bottom edge.
    let mut overlay_row = row![ring].align_y(iced::alignment::Vertical::Bottom);
    if !own && !status.is_empty() {
        overlay_row = overlay_row.push(container(status_bubble(&status, pal)).padding(crate::util::pad(0.0, 0.0, 8.0, 10.0)));
    }
    let base = column![banner_block, space().height(Length::Fixed(48.0))].width(Length::Fill);
    let head = iced::widget::stack![base, container(overlay_row).padding(crate::util::pad(48.0, 0.0, 0.0, 14.0))];
    items.push(container(head).width(Length::Fill).height(Length::Fixed(144.0)).into());
    items.push(container(text(name.clone()).size(name_size).color(pal.text_strong)).padding(crate::util::pad(12.0, 14.0, 0.0, 14.0)).into());
    if let Some(c) = card {
        items.push(container(text(format!("@{}", c.username)).size(13).color(pal.text_muted)).padding(crate::util::pad(0.0, 14.0, 8.0, 14.0)).into());
        // The servers you and this member both are in, by name.
        // Shared servers only make sense on someone else's card: on your own, every server is shared.
        if !own && !c.mutual.is_empty() {
            let names: Vec<String> = c
                .mutual
                .iter()
                .map(|addr| app.prefs.servers.iter().find(|s| s.addr == *addr).map_or(addr.clone(), |s| s.name.clone()))
                .collect();
            let label = format!("{} Mutual Server{}", names.len(), if names.len() == 1 { "" } else { "s" });
            items.push(
                container(
                    column![
                        text(label).size(12).color(pal.text_muted),
                        text(names.join(" · ")).size(12).color(pal.text),
                    ]
                    .spacing(2),
                )
                .padding(crate::util::pad(0.0, 14.0, 10.0, 14.0))
                .into(),
            );
        }
    }
    if own {
        items.push(status_bubble_editor(app, &status, pal));
    }
    items.push(super::divider(pal.selected_bg));
    items.push(container(text("ABOUT ME").size(11).color(pal.text_muted)).padding(crate::util::pad(12.0, 14.0, 4.0, 14.0)).into());
    let bio: Element<'_, Message> = match (card, &view.error, view.loading) {
        (Some(c), _, _) if c.bio.is_empty() => text("No bio yet.").size(13).color(pal.text_muted).into(),
        (Some(c), _, _) => super::linked_text(&with_bullets(&c.bio), 13.0, pal.text, pal),
        (None, _, true) => text("Loading…").size(13).color(pal.text_muted).into(),
        (None, Some(err), _) => text(format!("Profile unavailable: {err}")).size(13).color(pal.text_muted).into(),
        (None, None, false) => text("Profile unavailable.").size(13).color(pal.text_muted).into(),
    };
    items.push(container(bio).padding(crate::util::pad(0.0, 14.0, 8.0, 14.0)).into());

    // The roles this member holds on this server, highest first, in their colours. Always shown, so a
    // member with none says so. Admins get a + to give roles, which opens the role checklist here.
    let member_id = if own { Some(app.user_id.clone()) } else { app.server.member_ids.get(&name).cloned() };
    let held = member_id.as_ref().and_then(|id| app.server.member_roles.get(id));
    let held_roles: Vec<i64> = held.cloned().unwrap_or_default();
    let roles: Vec<&common::RoleInfo> = app
        .server
        .roles
        .iter()
        .filter(|r| r.id != common::EVERYONE_ROLE && held.is_some_and(|h| h.contains(&r.id)))
        .collect();
    let can_assign = app.my_permissions() & common::perm::MANAGE_ROLES != 0;
    // The + sits at the end of the roles, on the same line, when the viewer may give roles.
    let plus: Option<Element<'_, Message>> = (can_assign && member_id.is_some()).then(|| {
        button(
            container(text("+").size(15).color(pal.text))
                .width(Length::Fixed(22.0))
                .height(Length::Fixed(22.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(pal.input_bg)),
                    border: border::rounded(11.0).color(pal.text_muted).width(1.0),
                    ..container::Style::default()
                }),
        )
        .padding(0)
        .on_press(Message::CardRolesToggle)
        .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() })
        .into()
    });
    items.push(container(text("ROLES").size(11).color(pal.text_muted)).padding(crate::util::pad(12.0, 14.0, 4.0, 14.0)).into());
    let mut role_chips: Vec<Element<'_, Message>> = roles
        .into_iter()
        .map(|r| {
            let color = crate::util::hex_color(&r.color).unwrap_or(pal.text_strong);
            let dot = container(space()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(move |_: &Theme| container::Style {
                background: Some(Background::Color(color)),
                border: border::rounded(5.0),
                ..container::Style::default()
            });
            let mut content = row![dot, text(r.name.clone()).size(12).color(pal.text)].spacing(6).align_y(iced::alignment::Vertical::Center);
            if let (true, Some(who)) = (can_assign, member_id.clone()) {
                content = content.push(remove_button(Message::SetMemberRole(who, r.id, false), "Remove role", pal));
            }
            container(content)
                .padding(crate::util::pad2(3.0, 8.0))
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(pal.input_bg)),
                    border: border::rounded(10.0),
                    ..container::Style::default()
                })
                .into()
        })
        .collect();
    if role_chips.is_empty() {
        role_chips.push(text("No roles yet.").size(12).color(pal.text_muted).into());
    }
    role_chips.extend(plus);
    items.push(container(row(role_chips).spacing(6).align_y(iced::alignment::Vertical::Center).wrap()).padding(crate::util::pad(0.0, 14.0, 10.0, 14.0)).into());
    if can_assign && app.card_roles_open {
        if let Some(id) = member_id.as_deref() {
            let rows: Vec<PickRow> = app
                .server
                .roles
                .iter()
                .filter(|r| r.id != common::EVERYONE_ROLE && app.outranks_role(r.id) && !held_roles.contains(&r.id))
                .map(|r| PickRow {
                    name: r.name.clone(),
                    color: crate::util::hex_color(&r.color).unwrap_or(pal.text),
                    msg: Message::SetMemberRole(id.to_string(), r.id, true),
                })
                .collect();
            pick_popup = Some((container(pick_list("Role", &app.card_list_search, rows, pal)).width(Length::Fixed(260.0)).into(), Message::CardRolesToggle));
        }
    }

    // The badges this member has, then a + for admins, which opens a checklist of every badge here.
    if let Some(id) = member_id.as_deref() {
        let has: Vec<i64> = app.server.member_badges.get(id).cloned().unwrap_or_default();
        let owned: Vec<&common::Badge> = app.server.badges.iter().filter(|b| has.contains(&b.id)).collect();
        if can_assign || !owned.is_empty() {
            let plus: Option<Element<'_, Message>> = can_assign.then(|| {
                button(
                    container(text("+").size(15).color(pal.text))
                        .width(Length::Fixed(22.0))
                        .height(Length::Fixed(22.0))
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(pal.input_bg)),
                            border: border::rounded(11.0).color(pal.text_muted).width(1.0),
                            ..container::Style::default()
                        }),
                )
                .padding(0)
                .on_press(Message::CardBadgesToggle)
                .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() })
                .into()
            });
            items.push(container(text("BADGES").size(11).color(pal.text_muted)).padding(crate::util::pad(12.0, 14.0, 4.0, 14.0)).into());
            let mut badge_chips: Vec<Element<'_, Message>> = owned
                .into_iter()
                .map(|b| {
                    let color = crate::util::hex_color(&b.color).unwrap_or(pal.text_strong);
                    let dot = container(space()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(color)),
                        border: border::rounded(5.0),
                        ..container::Style::default()
                    });
                    let mut content = row![dot, text(b.name.clone()).size(12).color(pal.text)].spacing(6).align_y(iced::alignment::Vertical::Center);
                    if can_assign {
                        content = content.push(remove_button(Message::SetMemberBadge { user: id.to_string(), badge: b.id, on: false }, "Remove badge", pal));
                    }
                    container(content)
                        .padding(crate::util::pad2(3.0, 8.0))
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(pal.input_bg)),
                            border: border::rounded(10.0),
                            ..container::Style::default()
                        })
                        .into()
                })
                .collect();
            if badge_chips.is_empty() {
                badge_chips.push(text("No badges yet.").size(12).color(pal.text_muted).into());
            }
            badge_chips.extend(plus);
            items.push(container(row(badge_chips).spacing(6).align_y(iced::alignment::Vertical::Center).wrap()).padding(crate::util::pad(0.0, 14.0, 10.0, 14.0)).into());
            if can_assign && app.card_badges_open {
                let rows: Vec<PickRow> = app
                    .server
                    .badges
                    .iter()
                    .filter(|b| !has.contains(&b.id))
                    .map(|b| PickRow {
                        name: b.name.clone(),
                        color: crate::util::hex_color(&b.color).unwrap_or(pal.text),
                        msg: Message::SetMemberBadge { user: id.to_string(), badge: b.id, on: true },
                    })
                    .collect();
                pick_popup = Some((container(pick_list("Badge", &app.card_list_search, rows, pal)).width(Length::Fixed(260.0)).into(), Message::CardBadgesToggle));
            }
        }
    }

    if own {
        items.push(super::divider(pal.selected_bg));
        items.push(
            container(solid("Edit Profile", Message::UserSettingsOpen, pal.accent, pal.on_color, None))
                .padding(crate::util::pad(8.0, 14.0, 14.0, 14.0))
                .into(),
        );
    } else {
        // A message box, as on Discord: pressing it opens a conversation with this member.
        items.push(super::divider(pal.selected_bg));
        items.push(
            container(
                text_input(&format!("Message @{name}"), &app.card_message)
                    .on_input(Message::CardMessageInput)
                    .on_submit(Message::CardMessageSend)
                    .padding(10)
                    .size(13)
                    .style(input_style(pal)),
            )
            .padding(crate::util::pad(10.0, 14.0, 14.0, 14.0))
            .into(),
        );
    }

    // Beside the click, to its left, kept on screen. The card is never taller than the window allows: it
    // moves up if there is little room below, and then scrolls if it is still too tall.
    const MIN_CARD_HEIGHT: f32 = 320.0;
    let window_h = app.window_size.height;
    let x = (view.at.x - 316.0).max(8.0);
    let y = (view.at.y - 40.0).max(8.0).min((window_h - MIN_CARD_HEIGHT - 8.0).max(8.0));
    let max_card_height = if view.above { (view.at.y - 16.0).max(MIN_CARD_HEIGHT) } else { (window_h - y - 16.0).max(MIN_CARD_HEIGHT) };
    let body = container(scrollable(Column::with_children(items)).height(Length::Shrink))
        .width(Length::Fixed(300.0))
        .max_height(max_card_height)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    let card_layer: Element<'_, Message> = if view.above {
        // Opened from the user bar: the card's bottom edge sits just above the bar, whatever its height.
        anchored_above(body.into(), iced::Point::new(x, view.at.y), app.window_size.height, Message::CardClose)
    } else {
        anchored(body.into(), iced::Point::new(x, y), Message::CardClose)
    };
    match pick_popup {
        Some((list, close)) => {
            // Where the + was clicked, kept on screen. Clicking outside the list closes it, not the card.
            const LIST_SIZE: iced::Size = iced::Size::new(260.0, 320.0);
            let at = keep_on_screen(app.card_pick_at, LIST_SIZE, app.window_size);
            Some(iced::widget::stack![card_layer, anchored(list, at, close)].into())
        }
        None => Some(card_layer),
    }
}

/// The permissions of one channel: what each role may do there, with the server-wide setting as the default.
fn channel_access_dialog(app: &App, channel: i64, pal: Pal) -> Element<'_, Message> {
    let name = app.server.channels.iter().find(|c| c.id == channel).map(|c| c.name.clone()).unwrap_or_default();
    let mut items: Vec<Element<'_, Message>> = vec![
        text(format!("Permissions for #{name}")).size(20).color(pal.text_strong).into(),
        text("Inherit keeps the role's server-wide setting. Deny wins over Allow when a member has several roles.")
            .size(13)
            .color(pal.text_muted)
            .into(),
    ];
    for role in &app.server.roles {
        let current = app.server.overrides.iter().find(|o| o.channel_id == channel && o.role == role.id);
        let (view, send) = current.map_or((None, None), |o| (o.view, o.send));
        let role_id = role.id;
        items.push(
            column![
                text(role.name.clone()).size(14).color(pal.text),
                access_choice("View", view, move |v| Message::ChannelAccessSet { channel, role: role_id, view: v, send }, pal),
                access_choice("Send", send, move |s| Message::ChannelAccessSet { channel, role: role_id, view, send: s }, pal),
            ]
            .spacing(6)
            .into(),
        );
    }
    items.push(
        row![space().width(Length::Fill), link("Done", Message::ChannelAccessClose, pal.text_muted)]
            .align_y(iced::alignment::Vertical::Center)
            .into(),
    );
    modal(items, pal, Message::ChannelAccessClose)
}

/// One setting of a channel exception: Inherit, Allow or Deny.
fn access_choice<'a>(label: &str, current: Option<bool>, on_pick: impl Fn(Option<bool>) -> Message, pal: Pal) -> Element<'a, Message> {
    let choice = |text_: &str, value: Option<bool>| -> Element<'a, Message> {
        let selected = current == value;
        button(text(text_.to_string()).size(12).color(if selected { pal.text_strong } else { pal.text_muted }))
            .padding(crate::util::pad2(4.0, 10.0))
            .on_press(on_pick(value))
            .style(move |_: &Theme, status: button::Status| {
                let bg = if selected {
                    pal.selected_bg
                } else if matches!(status, button::Status::Hovered) {
                    pal.hover_bg
                } else {
                    Color::TRANSPARENT
                };
                button::Style { background: Some(Background::Color(bg)), text_color: pal.text, border: border::rounded(4.0), ..button::Style::default() }
            })
            .into()
    };
    row![
        text(label.to_string()).size(13).color(pal.text_muted).width(Length::Fixed(44.0)),
        choice("Inherit", None),
        choice("Allow", Some(true)),
        choice("Deny", Some(false)),
    ]
    .spacing(4)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// A rounded bubble with a status text in it.
fn status_bubble<'a>(status: &str, pal: Pal) -> Element<'a, Message> {
    container(text(status.to_string()).size(12).color(pal.text))
        .padding(crate::util::pad2(4.0, 8.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.input_bg)),
            border: border::rounded(10.0),
            ..container::Style::default()
        })
        .into()
}

/// Your own status bubble: shows the status with a pencil to edit it and a trash to clear it. While
/// editing it is a text box with Save and Cancel.
fn status_bubble_editor<'a>(app: &App, status: &str, pal: Pal) -> Element<'a, Message> {
    if let Some(draft) = &app.card_status_draft {
        let mut col = Column::new().spacing(6);
        col = col.push(
            text_input("What's your status?", draft)
                .on_input(Message::CardStatusInput)
                .on_submit(Message::CardStatusSave)
                .padding(8)
                .size(13),
        );
        if let Some(error) = &app.card_status_error {
            col = col.push(text(error.clone()).size(12).color(pal.red));
        }
        col = col.push(
            row![
                space().width(Length::Fill),
                link("Cancel", Message::CardStatusCancel, pal.text_muted),
                solid("Save", Message::CardStatusSave, pal.accent, pal.on_color, Some(72.0)),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
        );
        return container(col).padding(crate::util::pad(4.0, 14.0, 8.0, 14.0)).into();
    }
    let edit = button(icons::icon(icons::PENCIL, 13.0, pal.text_muted))
        .padding(3)
        .on_press(Message::CardStatusStart)
        .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() });
    let clear = button(icons::icon(icons::TRASH, 13.0, pal.text_muted))
        .padding(3)
        .on_press(Message::CardStatusClear)
        .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() });
    let shown: Element<'a, Message> = if status.is_empty() {
        text("Set a status").size(12).color(pal.text_muted).into()
    } else {
        status_bubble(status, pal)
    };
    container(row![shown, space().width(Length::Fill), edit, clear].spacing(6).align_y(iced::alignment::Vertical::Center))
        .padding(crate::util::pad(4.0, 14.0, 8.0, 14.0))
        .into()
}

#[cfg(test)]
mod placement_tests {
    use super::keep_on_screen;
    use iced::{Point, Size};

    #[test]
    fn menus_near_the_bottom_or_right_move_back_onto_the_window() {
        let window = Size::new(1000.0, 700.0);
        // Opened near the bottom right: moved so the whole menu is visible.
        let at = keep_on_screen(Point::new(990.0, 690.0), Size::new(200.0, 300.0), window);
        assert_eq!(at, Point::new(792.0, 392.0));
        // Room to spare: stays where it was opened.
        assert_eq!(keep_on_screen(Point::new(100.0, 50.0), Size::new(200.0, 300.0), window), Point::new(100.0, 50.0));
        // A menu taller than the window stays at the top margin rather than going off the top.
        assert_eq!(keep_on_screen(Point::new(10.0, 10.0), Size::new(200.0, 900.0), window).y, 8.0);
    }
}

#[cfg(test)]
mod bio_tests {
    use super::with_bullets;

    #[test]
    fn dash_and_star_lines_become_bullets_and_others_stay() {
        assert_eq!(with_bullets("Hi\n- one\n  * two\nlast"), "Hi\n• one\n• two\nlast");
    }
}
