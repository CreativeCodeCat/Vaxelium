//! Popups drawn over the window: the context menus, the emoji picker, the dialogs, and the edges
//! that resize the window.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, text, text_input, Column};
use iced::{border, window, Background, Color, Element, Length, Padding, Theme};

use super::chat::emoji;
use super::pages::modal;
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
    let backdrop = mouse_area(container(space()).width(Length::Fill).height(Length::Fill)).on_press(close);
    let placed = container(popup)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::default().top(at.y).left(at.x));
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
            MenuItem::Action { label, icon, danger, msg } => {
                let color = if *danger { pal.red } else { pal.text };
                let mut r = row![].spacing(10).align_y(iced::alignment::Vertical::Center);
                if let Some(icon) = icon {
                    r = r.push(icons::icon(icon, 14.0, color));
                }
                r = r.push(text(label.clone()).size(14).color(color));
                items.push(
                    button(r)
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
                        .into(),
                );
            }
        }
    }
    let width = menu.width.unwrap_or(200.0);
    let body = container(Column::with_children(items).padding(4))
        .width(Length::Fixed(width))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    Some(anchored(body.into(), menu.at, Message::CloseMenu))
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
    if let Some((old, new)) = &app.rename_channel {
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
