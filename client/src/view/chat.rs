//! The chat views: the server rail, the channel and direct message lists, the message list, the
//! composer, the member list and the message search.

use common::ChatMessage;
use iced::widget::{button, column, container, mouse_area, rich_text, row, scrollable, span, space, svg, text, text_input, tooltip, Column};
use iced::{border, Background, Color, Element, Length, Theme};

use super::{bar, divider, ghost, letter_badge, muted_text, panel, round, server_icon};
use crate::app::{App, DmChoice, Message, COMPOSER_ID, EDIT_ID, MESSAGES_ID, NEW_CHANNEL_ID, NEW_DM_ID, SEARCH_ID};
use crate::icons;
use crate::model::{is_dm_channel, Entry, GROUP_GAP_SECS};
use crate::search::Search;
use crate::theme::Pal;
use crate::util::{day_label, emoji_path, stamp_label};

/// Emoji offered on every message's toolbar.
pub const QUICK_REACTIONS: [&str; 3] = ["👍", "❤️", "😂"];
/// The height of the top row's share above the message list, for placing the search filter menu.
const FILTERS_TOP: f32 = super::TITLEBAR_HEIGHT + 48.0 + 8.0;
const SPLITTER: f32 = 6.0;

/// An emoji as a picture, or its text when no artwork exists.
pub fn emoji<'a>(e: &str, size: f32) -> Element<'a, Message> {
    let path = emoji_path(e);
    if path.exists() {
        svg(svg::Handle::from_path(path)).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
    } else {
        text(e.to_string()).size(size * 0.8).into()
    }
}

/// A draggable edge between two panels.
pub fn splitter<'a>(key: &'static str, right: bool) -> Element<'a, Message> {
    mouse_area(container(space()).width(Length::Fixed(SPLITTER)).height(Length::Fill))
        .on_press(Message::DragStart(key, right))
        .interaction(super::drag_cursor())
        .into()
}

// ----- server rail -----

pub fn rail(app: &App, pal: Pal) -> Element<'_, Message> {
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    items.push(rail_button(
        svg(svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/dm.svg")))
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .into(),
        Message::HomeOpen,
        app.home,
        pal,
        None,
    ));
    items.push(bar(32.0, 2.0, pal.hover_bg));
    for (i, server) in app.prefs.servers.iter().enumerate() {
        let active = app.selected == Some(i) && !app.home;
        let radius = if active { 16.0 } else { 24.0 };
        let picture = server_icon(server.icon.as_deref(), &server.name, 48.0, &pal, radius);
        let button = rail_button(picture, Message::SelectServer(i), active, pal, Some(Message::OpenMenu(crate::app::MenuKind::Rail(i))));
        items.push(name_bubble(button, &server.name, server.linked, pal));
    }
    items.push(rail_button(
        text("+").size(24).color(pal.green).into(),
        Message::AddServerOpen,
        false,
        pal,
        None,
    ));
    items.push(rail_button(
        icons::icon(icons::EXPLORE, 22.0, pal.text),
        Message::DiscoverOpen,
        false,
        pal,
        None,
    ));
    // The column fills the rail, so its buttons are centred in it rather than kept at its left edge.
    container(Column::with_children(items).spacing(8).align_x(iced::alignment::Horizontal::Center).width(Length::Fill))
        .width(Length::Fixed(72.0))
        .height(Length::Fill)
        .padding(crate::util::pad2(12.0, 0.0))
        .style(move |_: &Theme| round(pal.rail_bg, 0.0))
        .into()
}

/// Shows a server's name in a bubble to the right of its rail button while the pointer is over it.
/// A house before the name marks a server that other servers know about.
fn name_bubble<'a>(button: Element<'a, Message>, name: &str, linked: bool, pal: Pal) -> Element<'a, Message> {
    let mut parts = row![].spacing(8).align_y(iced::alignment::Vertical::Center);
    if linked {
        parts = parts.push(svg(svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/house.svg"))).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)));
    }
    parts = parts.push(text(name.to_string()).size(14).color(pal.text_strong));
    let content: Element<'a, Message> = parts.into();
    let bubble = container(content)
        .padding(crate::util::pad2(6.0, 10.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(6.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    tooltip(button, bubble, tooltip::Position::Right).gap(10).into()
}

fn rail_button<'a>(content: Element<'a, Message>, msg: Message, active: bool, pal: Pal, right: Option<Message>) -> Element<'a, Message> {
    let radius = if active { 16.0 } else { 24.0 };
    let b = button(container(content).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).align_x(iced::alignment::Horizontal::Center).align_y(iced::alignment::Vertical::Center))
        .padding(0)
        .on_press(msg)
        .style(move |_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => pal.accent,
                _ if active => pal.accent,
                _ => pal.input_bg,
            };
            button::Style { background: Some(Background::Color(bg)), border: border::rounded(radius), text_color: pal.text_strong, ..button::Style::default() }
        });
    match right {
        Some(menu) => mouse_area(b).on_right_press(menu).into(),
        None => b.into(),
    }
}

// ----- channels -----

/// The chat view: channel list, header, messages, composer and members.
pub fn channel(app: &App, pal: Pal) -> Element<'_, Message> {
    let search = Search::parse(&app.search);
    let mut body_row = row![center(app, pal, &search)];
    if !app.members_hidden {
        body_row = body_row.push(splitter("members", true)).push(members(app, pal));
    }
    // The top row always has the same parts, so the search box keeps its place (and its focus and
    // cursor) when the member list is hidden. Hiding only changes the widths.
    // The search slot keeps the member column's width even when the list is hidden, so the header
    // stays the same size. Only its background changes.
    let members_hidden = app.members_hidden;
    let search_width = app.panel_width("members");
    let gap = SPLITTER;
    let top = row![
        header(app, pal),
        space().width(Length::Fixed(gap)).height(Length::Fixed(48.0)),
        container(search_box(app, pal, Length::Fill))
            .width(Length::Fixed(search_width))
            .height(Length::Fixed(48.0))
            .padding(crate::util::pad2(0.0, 8.0))
            .align_y(iced::alignment::Vertical::Center)
            // With the member list hidden, the slot takes the chat area's colour instead.
            .style(move |_: &Theme| round(if members_hidden { pal.app_bg } else { pal.sidebar_bg }, 0.0)),
    ];
    let top = top.height(Length::Fixed(48.0));
    let chat_main = column![top, body_row.height(Length::Fill)].width(Length::Fill).height(Length::Fill);
    row![sidebar(app, pal), splitter("channels", false), chat_main].height(Length::Fill).into()
}

/// The members of the server in the main area, in place of a channel. The sidebar stays as it is.
pub fn members_page(app: &App, pal: Pal) -> Element<'_, Message> {
    let title = container(
        row![icons::icon(icons::MEMBERS, 16.0, pal.text_muted), text("Members").size(16).color(pal.text_strong)]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(48.0))
    .padding(crate::util::pad2(0.0, 16.0))
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_: &Theme| round(pal.app_bg, 0.0));
    let chat_main = column![title, super::pages::members_section(app, pal)].width(Length::Fill).height(Length::Fill);
    row![sidebar(app, pal), splitter("channels", false), chat_main].height(Length::Fill).into()
}

fn sidebar(app: &App, pal: Pal) -> Element<'_, Message> {
    let mut list: Vec<Element<'_, Message>> = Vec::new();
    // A heading above every channel, in the same style as the category headings.
    list.push(container(text("CHANNELS").size(12).color(pal.text_muted)).padding(crate::util::pad2(0.0, 4.0)).into());
    // While a channel is dragged, the zone above the channels takes it out of its category.
    let dragging_channel = matches!(app.dnd.as_ref().map(|d| (d.moved, d.item)), Some((true, crate::app::DragItem::Channel(_))));
    let zone_over = app.dnd.as_ref().and_then(|d| d.over) == Some(crate::app::DropTarget::Uncategorized);
    if dragging_channel {
        let zone = container(text("Drop here to take out of its category").size(12).color(pal.text_muted))
            .padding(crate::util::pad2(4.0, 6.0))
            .width(Length::Fill)
            .height(Length::Fixed(28.0))
            .style(move |_: &Theme| round(if zone_over { pal.selected_bg } else { Color::TRANSPARENT }, 4.0));
        list.push(
            mouse_area(zone)
                .on_enter(Message::DndOver(Some(crate::app::DropTarget::Uncategorized)))
                .on_exit(Message::DndOver(None))
                .into(),
        );
    }
    // Channels not in any category come first, with no heading.
    for c in app.server.uncategorized.iter().filter(|c| !c.name.starts_with("dm-")) {
        list.push(channel_row(app, pal, c));
    }
    // Each category has a heading that folds its channels. A folded category still shows the open channel.
    for category in &app.server.categories {
        let collapsed = app.collapsed_categories.contains(&category.id);
        let chevron = if collapsed { icons::CHEVRON_RIGHT } else { icons::CHEVRON_DOWN };
        let heading = row![
            button(icons::icon(chevron, 14.0, pal.text_muted))
                .padding(2)
                .on_press(Message::ToggleCategory(category.id))
                .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() }),
            text(category.name.to_uppercase()).size(12).color(pal.text_muted),
        ]
        .spacing(6)
        .align_y(iced::alignment::Vertical::Center);
        // A category can be dragged to reorder, and a channel dropped on it joins it.
        let hovered = app.dnd.as_ref().and_then(|d| d.over) == Some(crate::app::DropTarget::Category(category.id));
        let target = crate::app::DropTarget::Category(category.id);
        list.push(
            mouse_area(
                container(heading)
                    .padding(crate::util::pad(12.0, 4.0, 2.0, 0.0))
                    .width(Length::Fill)
                    .style(move |_: &Theme| round(if hovered { pal.selected_bg } else { Color::TRANSPARENT }, 4.0)),
            )
            .on_press(Message::DndPress(crate::app::DragItem::Category(category.id)))
            .on_enter(Message::DndOver(Some(target)))
            .on_exit(Message::DndOver(None))
            .on_right_press(Message::OpenMenu(crate::app::MenuKind::Category { id: category.id, name: category.name.clone() }))
            .into(),
        );
        for c in &category.channels {
            if collapsed && c.name != app.server.channel {
                continue;
            }
            list.push(channel_row(app, pal, c));
        }
    }
    // While a category is dragged, this zone after the last one moves it to the end.
    let dragging_category = matches!(app.dnd.as_ref().map(|d| (d.moved, d.item)), Some((true, crate::app::DragItem::Category(_))));
    if dragging_category {
        let over_end = app.dnd.as_ref().and_then(|d| d.over) == Some(crate::app::DropTarget::End);
        let zone = container(text("Drop here to move to the end").size(12).color(pal.text_muted))
            .padding(crate::util::pad2(4.0, 6.0))
            .width(Length::Fill)
            .height(Length::Fixed(28.0))
            .style(move |_: &Theme| round(if over_end { pal.selected_bg } else { Color::TRANSPARENT }, 4.0));
        list.push(
            mouse_area(zone)
                .on_enter(Message::DndOver(Some(crate::app::DropTarget::End)))
                .on_exit(Message::DndOver(None))
                .into(),
        );
    }
    if app.category_open {
        list.push(
            container(
                text_input("Create category", &app.new_category)
                    .id(crate::app::NEW_CATEGORY_ID)
                    .on_input(Message::NewCategory)
                    .on_submit(Message::CreateCategory)
                    .padding(7)
                    .size(14),
            )
            .padding(crate::util::pad(12.0, 0.0, 0.0, 0.0))
            .into(),
        );
    }
    if app.create_open {
        list.push(
            container(
                text_input("Create channel", &app.new_channel)
                    .id(NEW_CHANNEL_ID)
                    .on_input(Message::NewChannel)
                    .on_submit(Message::CreateChannel)
                    .padding(7)
                    .size(14),
            )
            .padding(crate::util::pad(12.0, 0.0, 0.0, 0.0))
            .into(),
        );
    }
    // The house beside the name marks a server that other servers know about.
    let linked = app.selected.and_then(|i| app.prefs.servers.get(i)).is_some_and(|s| s.linked);
    // The house sits before the name; the drop-down stays at the right edge.
    let mut header_row = row![].spacing(8).align_y(iced::alignment::Vertical::Center);
    if linked {
        header_row = header_row.push(svg(svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/house.svg"))).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)));
    }
    // The drop-down always shows; a long name is cut short with an ellipsis to make room for it.
    // Room is what the sidebar has, less the padding, the drop-down, the house when shown, and the gaps.
    let reserved = 28.0 + 14.0 + 8.0 + if linked { 14.0 + 8.0 } else { 0.0 } + 8.0;
    let room = (app.panel_width("channels") - reserved).max(0.0);
    let full = app.selected_name().unwrap_or_else(|| "Vaxelium".into());
    let shown = fit_name(&full, room, 16.0);
    let header_row = header_row
        .push(container(text(shown).size(16).color(pal.text_strong).wrapping(iced::widget::text::Wrapping::None)).width(Length::Fill).clip(true))
        .push(icons::icon(icons::CHEVRON_DOWN, 14.0, pal.text));
    let header_btn = mouse_area(
        container(header_row)
        .width(Length::Fill)
        .padding(crate::util::pad2(0.0, 14.0))
        .height(Length::Fixed(48.0))
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| round(pal.sidebar_bg, 0.0)),
    )
    .on_press(Message::OpenMenu(crate::app::MenuKind::Server));
    // Members sits above the channels and opens the member list in place of the chat.
    let members_selected = app.members_view;
    let members_color = if members_selected { pal.text_strong } else { pal.text_muted };
    let members_nav = container(
        button(
            row![icons::icon(icons::MEMBERS, 16.0, members_color), text("Members").size(15).color(members_color)]
                .spacing(10)
                .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding(crate::util::pad2(6.0, 10.0))
        .on_press(Message::MembersView(true))
        .style(move |_: &Theme, status: button::Status| {
            let bg = if members_selected {
                pal.selected_bg
            } else if matches!(status, button::Status::Hovered) {
                pal.hover_bg
            } else {
                Color::TRANSPARENT
            };
            button::Style { background: Some(Background::Color(bg)), text_color: members_color, border: border::rounded(4.0), ..button::Style::default() }
        }),
    )
    .padding(crate::util::pad2(8.0, 8.0));
    panel(
        column![
            header_btn,
            members_nav,
            container(Column::with_children(list).spacing(1)).padding(crate::util::pad2(12.0, 8.0)).height(Length::Fill),
            user_bar(app, pal),
        ]
        .height(Length::Fill),
        Length::Fixed(app.panel_width("channels")),
        pal.sidebar_bg,
    )
}

fn channel_row<'a>(app: &App, pal: Pal, channel: &common::ChannelRef) -> Element<'a, Message> {
    let name = channel.name.as_str();
    let selected = channel.id == app.server.channel_id;
    let unread = app.unread.contains(&channel.id);
    let pinged = app.pinged.contains(&channel.id);
    let color = if selected || unread { pal.text_strong } else { pal.text_muted };
    let bg = if selected { pal.selected_bg } else { Color::TRANSPARENT };
    let mut inner = row![text(format!("# {name}")).size(15).color(color).width(Length::Fill)]
        .spacing(6)
        .align_y(iced::alignment::Vertical::Center);
    // A ping for one of your roles is red, so it stands out from ordinary unread messages.
    if unread && !selected {
        let dot = if pinged { pal.red } else { pal.text_strong };
        inner = inner.push(container(space()).width(Length::Fixed(8.0)).height(Length::Fixed(8.0)).style(move |_: &Theme| round(dot, 4.0)));
    }
    // While another channel is dragged over this one, it is highlighted: the drop goes before it.
    let target = crate::app::DropTarget::Channel { id: channel.id, category: category_of(app, channel.id) };
    let dragged_channel = matches!(app.dnd.as_ref().map(|d| (d.moved, d.item)), Some((true, crate::app::DragItem::Channel(_))));
    let hovered = dragged_channel && app.dnd.as_ref().and_then(|d| d.over) == Some(target);
    let bg = if hovered { pal.selected_bg } else { bg };
    let body = container(inner)
        .padding(crate::util::pad2(0.0, 10.0))
        .height(Length::Fixed(32.0))
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill)
        .style(move |_: &Theme| round(bg, 4.0));
    let mut area = mouse_area(body).on_press(Message::DndPress(crate::app::DragItem::Channel(channel.id)));
    if dragged_channel {
        area = area.on_enter(Message::DndOver(Some(target))).on_exit(Message::DndOver(None));
    }
    area.on_right_press(Message::OpenMenu(crate::app::MenuKind::Channel { id: channel.id, name: name.to_string(), unread }))
        .into()
}

/// The category a channel is in, if any.
fn category_of(app: &App, channel: i64) -> Option<i64> {
    app.server.categories.iter().find(|c| c.channels.iter().any(|ch| ch.id == channel)).map(|c| c.id)
}

/// The height of the user bar at the bottom of the sidebar.
pub const USER_BAR_HEIGHT: f32 = 56.0;

fn user_bar(app: &App, pal: Pal) -> Element<'_, Message> {
    let user = app.user.clone().unwrap_or_default();
    let own_status = app.effective_status();
    // The room is the sidebar less the picture, the icons, the settings button and the gaps.
    let room = (app.panel_width("channels") - 150.0).max(0.0);
    let own_text = app.profiles.get(&app.user_id).map(|look| look.status.clone()).unwrap_or_default();
    let status_text_line: Element<'_, Message> = if own_text.is_empty() {
        space().height(Length::Fixed(0.0)).into()
    } else {
        text(fit_name(&own_text, room, 11.0)).size(11).color(pal.text).wrapping(iced::widget::text::Wrapping::None).into()
    };
    let who = column![
        // Your name opens your own profile card, where your status bubble is edited.
        button(text(user.clone()).size(14).color(pal.text_strong))
            .padding(0)
            .on_press(Message::CardOpenAbove { id: app.user_id.clone(), name: user.clone() })
            .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() }),
        // Your own status. Click it to change it.
        button(text(status_label(own_status)).size(11).color(pal.text_muted))
            .padding(0)
            .on_press(Message::OpenMenu(crate::app::MenuKind::Status))
            .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() }),
        // Your status text, if you have set one, on one line.
        status_text_line,
    ]
    .width(Length::Fill);
    // Shows the same picture and banner as your entry in the member list, which are known as soon as
    // the server is connected. The account's own copy is only loaded when the settings are opened.
    let (picture, banner) = match app.profiles.get(&app.user_id) {
        Some(look) => (look.avatar.as_str(), look.banner.as_str()),
        None => (app.account.avatar.as_str(), app.account.banner.as_str()),
    };
    let content = row![
        own_badge(picture, banner, &user, own_status, pal),
        who,
        icons::icon(icons::HEADSET, 16.0, pal.text_muted),
        icons::icon(icons::MIC, 16.0, pal.text_muted),
        button(icons::icon(icons::GEAR, 16.0, pal.text))
            .padding(4)
            .on_press(Message::UserSettingsOpen)
            .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() }),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center);
    container(content)
        .padding(crate::util::pad2(4.0, 8.0))
        .height(Length::Fixed(USER_BAR_HEIGHT))
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| round(pal.user_bar_bg, 8.0))
        .into()
}

// ----- header and search -----

fn header(app: &App, pal: Pal) -> Element<'_, Message> {
    let mut items = row![
        text("#").size(22).color(pal.text_muted),
        text(app.server.channel.clone()).size(16).color(pal.text_strong),
        space().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center);
    let members_color = if app.members_hidden { pal.text_muted } else { pal.text_strong };
    items = items.push(
        row![
            disabled_icon(icons::HASH, pal),
            disabled_icon(icons::BELL, pal),
            disabled_icon(icons::PIN, pal),
            ghost(icons::icon(icons::MEMBERS, 16.0, members_color), Some(Message::ToggleMembers), pal.hover_bg, 4.0),
        ]
        .spacing(6),
    );
    container(items)
        .width(Length::Fill)
        .height(Length::Fixed(48.0))
        .padding(crate::util::pad2(0.0, 16.0))
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| round(pal.app_bg, 0.0))
        .into()
}

/// An icon for a feature that is not built yet: dimmed, and does nothing.
fn disabled_icon<'a>(icon: &'static str, pal: Pal) -> Element<'a, Message> {
    container(icons::icon(icon, 16.0, pal.text_muted)).padding(crate::util::pad2(2.0, 4.0)).into()
}

/// The message search field. The filter menu is drawn over the window by `search_filters_layer`.
pub fn search_box(app: &App, pal: Pal, width: Length) -> Element<'_, Message> {
    let placeholder = format!("Search {}", app.selected_name().unwrap_or_else(|| "Vaxelium".into()));
    // The X shows only while there is text to clear.
    let clear: Element<'_, Message> = if app.search.is_empty() {
        space().width(Length::Fixed(14.0)).into()
    } else {
        button(icons::icon(icons::CLOSE, 12.0, pal.text_muted))
            .padding(0)
            .on_press(Message::SearchClear)
            .style(|_: &Theme, _| button::Style { background: None, ..button::Style::default() })
            .into()
    };
    let field = row![
        icons::icon(icons::SEARCH, 14.0, pal.text_muted),
        text_input(&placeholder, &app.search)
            .id(SEARCH_ID)
            .on_input(Message::SearchChanged)
            .padding(0)
            .size(13)
            .style(move |_: &Theme, _| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: border::rounded(0.0),
                icon: pal.text_muted,
                placeholder: pal.text_muted,
                value: pal.text_strong,
                selection: pal.accent,
            })
            .width(Length::Fill),
        clear,
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Center);
    mouse_area(
        container(field)
            .width(width)
            .padding(crate::util::pad2(5.0, 8.0))
            .style(move |_: &Theme| round(pal.rail_bg, 4.0)),
    )
    .on_press(Message::SearchOpen)
    .into()
}

/// One row of the search filter menu: an icon, the filter's name, and what it takes.
fn menu_item<'a>(pal: Pal, prefix: &'static str, label: &str, hint: &str, icon: Element<'a, Message>) -> Element<'a, Message> {
    button(
        row![
            container(icon).width(Length::Fixed(20.0)),
            column![
                text(label.to_string()).size(14).color(pal.text_strong),
                text(hint.to_string()).size(12).color(pal.text_muted),
            ],
        ]
        .spacing(12)
        .align_y(iced::alignment::Vertical::Center),
    )
    .padding(crate::util::pad2(8.0, 16.0))
    .width(Length::Fill)
    .on_press(Message::SearchFilter(prefix))
    .style(move |_: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(pal.hover_bg)),
            _ => None,
        };
        button::Style { background: bg, text_color: pal.text, border: border::rounded(0.0), ..button::Style::default() }
    })
    .into()
}

/// The search filter menu, shown over the window below the search field while it is in use.
pub fn search_filters_layer(app: &App, pal: Pal) -> Option<Element<'_, Message>> {
    if !app.search_open || !app.chat_ready() {
        return None;
    }
    if let Some(items) = suggestions_menu(app, pal) {
        return Some(filters_panel(items, pal));
    }
    let hash = text("#").size(18).color(pal.text_muted).into();
    let mut items: Vec<Element<'_, Message>> = vec![
        container(text("Filters").size(12).color(pal.text_muted)).padding(crate::util::pad(4.0, 16.0, 8.0, 16.0)).into(),
        menu_item(pal, "from:", "From a specific user", "from: user", icons::icon(icons::USER, 16.0, pal.text_muted)),
        menu_item(pal, "in:", "Sent in a specific channel", "in: channel", hash),
        menu_item(pal, "has:", "Includes a specific type of data", "has: link, embed or file", icons::icon(icons::ATTACH, 16.0, pal.text_muted)),
        menu_item(pal, "mentions:", "Mentions a specific user", "mentions: user", icons::icon(icons::AT, 16.0, pal.text_muted)),
        button(
            row![
                container(icons::icon(icons::SLIDERS, 16.0, pal.text_muted)).width(Length::Fixed(20.0)),
                column![
                    text("More filters").size(14).color(pal.text_strong),
                    text("dates").size(12).color(pal.text_muted),
                ]
                .width(Length::Fill),
                icons::icon(if app.search_more { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT }, 14.0, pal.text_muted),
            ]
            .spacing(12)
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(crate::util::pad2(8.0, 16.0))
        .width(Length::Fill)
        .on_press(Message::SearchMore)
        .style(move |_: &Theme, _| button::Style { background: None, text_color: pal.text, ..button::Style::default() })
        .into(),
    ];
    if app.search_more {
        items.push(
            container(column![
                menu_item(pal, "before:", "Before a date", "before: YYYY-MM-DD", icons::icon(icons::SEARCH, 14.0, pal.text_muted)),
                menu_item(pal, "after:", "On or after a date", "after: YYYY-MM-DD", icons::icon(icons::SEARCH, 14.0, pal.text_muted)),
                menu_item(pal, "during:", "During a date", "during: YYYY-MM-DD", icons::icon(icons::SEARCH, 14.0, pal.text_muted)),
            ])
            .padding(crate::util::pad(0.0, 0.0, 0.0, 16.0))
            .into(),
        );
    }
    Some(filters_panel(items, pal))
}

/// The suggestions for the word being typed: channels after `in:`, members after `from:` or
/// `mentions:`. None when the word is not one of those.
fn suggestions_menu<'a>(app: &App, pal: Pal) -> Option<Vec<Element<'a, Message>>> {
    let word = app.search.split_whitespace().last().filter(|_| !app.search.ends_with(char::is_whitespace))?;
    let (filter, query) = word.split_once(':')?;
    let query = query.to_lowercase();
    let filter = filter.to_lowercase();
    let (heading, names): (&str, Vec<String>) = match filter.as_str() {
        "in" => ("In Channel", app.server.channels.iter().map(|c| c.name.clone()).collect()),
        "from" | "mentions" => (
            "Members",
            app.server.online.iter().chain(app.server.offline.iter()).cloned().collect(),
        ),
        _ => return None,
    };
    let matches: Vec<String> = names.into_iter().filter(|n| n.to_lowercase().contains(&query)).collect();
    let mut items: Vec<Element<'a, Message>> = vec![
        container(text(heading.to_uppercase()).size(11).color(pal.text_muted))
            .padding(crate::util::pad(4.0, 16.0, 8.0, 16.0))
            .into(),
    ];
    if matches.is_empty() {
        items.push(container(text("No matches").size(13).color(pal.text_muted)).padding(crate::util::pad2(6.0, 16.0)).into());
    }
    for name in matches {
        let icon: Element<'a, Message> = if filter == "in" {
            text("#").size(18).color(pal.text_muted).into()
        } else {
            letter_badge(&name, 20.0, &pal)
        };
        let value = format!("{filter}:{name}");
        let label = text(name).size(14).color(pal.text_strong);
        items.push(
            button(row![container(icon).width(Length::Fixed(20.0)), label].spacing(12).align_y(iced::alignment::Vertical::Center))
                .padding(crate::util::pad2(8.0, 16.0))
                .width(Length::Fill)
                .on_press(Message::SearchPick(value))
                .style(move |_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Some(Background::Color(pal.hover_bg)),
                        _ => None,
                    };
                    button::Style { background: bg, text_color: pal.text, border: border::rounded(0.0), ..button::Style::default() }
                })
                .into(),
        );
    }
    Some(items)
}

/// The panel that holds the filter menu or the suggestions, under the search box.
fn filters_panel<'a>(items: Vec<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    let menu = container(Column::with_children(items).padding(crate::util::pad2(8.0, 0.0)))
        .width(Length::Fixed(340.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        });
    container(menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .padding(crate::util::pad(FILTERS_TOP, 12.0, 0.0, 0.0))
        .into()
}

// ----- center: messages and composer -----

fn center<'a>(app: &'a App, pal: Pal, search: &Search) -> Element<'a, Message> {
    let entries = app.current_entries();
    column![
        messages(app, pal, entries, search),
        composer(app, pal),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// The message list: day dividers, grouped authors, the NEW line, and the search results.
fn messages<'a>(app: &'a App, pal: Pal, entries: Vec<Entry>, search: &Search) -> Element<'a, Message> {
    let person = app.current_person();
    let channel = app.server.channel.clone();
    let active = !search.is_empty();
    let new_at = if person.is_none() { app.read_marks.get(&app.server.channel_id).copied() } else { None };
    let now = crate::util::now();
    let mut out: Vec<Element<'a, Message>> = Vec::new();

    let welcome = if person.is_some() || is_dm_channel(&channel) {
        let label = app.channel_label();
        column![
            text(label.clone()).size(28).color(pal.text_strong),
            muted_text(format!("This is the start of your direct messages with {}.", label.trim_start_matches("@ ")), 14.0, pal.text_muted),
        ]
    } else {
        column![
            text(format!("Welcome to #{channel}!")).size(28).color(pal.text_strong),
            muted_text(format!("This is the start of the #{channel} channel."), 14.0, pal.text_muted),
        ]
    };
    out.push(container(welcome).padding(crate::util::pad(0.0, 16.0, 16.0, 16.0)).into());

    let matches = |e: &Entry| matches!(e, Entry::Chat(m) if search.matches(m, &channel));
    if active && !entries.iter().any(matches) {
        out.push(container(muted_text("No messages match your search.", 14.0, pal.text_muted)).padding(crate::util::pad2(8.0, 16.0)).into());
    }
    let mut prev_day: Option<i64> = None;
    let mut prev_author: Option<String> = None;
    let mut prev_at: u64 = 0;
    for (i, entry) in entries.into_iter().enumerate() {
        if active && !entry_visible(&channel, &entry, search) {
            continue;
        }
        if !active && new_at == Some(i) {
            out.push(new_divider(pal));
            prev_author = None;
        }
        let at = entry.time();
        let day = crate::util::local_day(at);
        if prev_day != Some(day) {
            out.push(day_divider(&day_label(at, now), pal));
            prev_author = None;
            prev_day = Some(day);
        }
        match entry {
            Entry::System { text: t, .. } => {
                out.push(container(muted_text(t.clone(), 13.0, pal.text_muted)).padding(crate::util::pad2(4.0, 16.0)).into());
                prev_author = None;
            }
            Entry::Chat(m) => {
                let grouped = prev_author.as_deref() == Some(m.author.as_str()) && at.saturating_sub(prev_at) < GROUP_GAP_SECS;
                prev_author = Some(m.author.clone());
                out.push(chat_message(app, pal, m, grouped, now));
            }
        }
        prev_at = at;
    }
    scrollable(Column::with_children(out).padding(crate::util::pad(16.0, 0.0, 12.0, 0.0)).width(Length::Fill))
        .id(MESSAGES_ID)
        .anchor_bottom()
        .height(Length::Fill)
        .into()
}

fn day_divider<'a>(label: &str, pal: Pal) -> Element<'a, Message> {
    container(row![
        divider(pal.selected_bg),
        text(label.to_string()).size(12).color(pal.text_muted),
        divider(pal.selected_bg),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center))
    .padding(crate::util::pad(12.0, 16.0, 8.0, 16.0))
    .into()
}

fn new_divider<'a>(pal: Pal) -> Element<'a, Message> {
    container(row![
        divider(pal.red),
        container(text("NEW").size(10).color(pal.on_color))
            .padding(crate::util::pad2(1.0, 6.0))
            .style(move |_: &Theme| round(pal.red, 4.0)),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center))
    .padding(crate::util::pad2(8.0, 16.0))
    .into()
}

/// Whether a timeline entry matches the search. System notices are hidden while searching.
fn entry_visible(channel: &str, e: &Entry, search: &Search) -> bool {
    match e {
        Entry::Chat(m) => search.matches(m, channel),
        Entry::System { .. } => false,
    }
}

/// One chat message. Hovering shows its toolbar; right-clicking opens its menu.
/// The name, cut short with an ellipsis when it would be wider than `room` pixels at `size`.
/// Letters are estimated at two thirds of the font size, a little wide on purpose so the name never
/// reaches the drop-down.
fn fit_name(name: &str, room: f32, size: f32) -> String {
    let per_char = size * 0.62;
    let max_chars = (room / per_char).floor() as usize;
    if name.chars().count() <= max_chars {
        return name.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut cut: String = name.chars().take(keep).collect();
    cut.push('…');
    cut
}

/// A message's text. `@role` pings for roles anyone may ping are shown in the role's colour, in bold.
fn message_text<'a>(app: &App, body: &str, pal: Pal) -> Element<'a, Message> {
    let pings = app.role_pings(body);
    let links = super::link_ranges(body);
    if pings.is_empty() && links.is_empty() {
        return text(body.to_string()).size(15).color(pal.text).width(Length::Shrink).into();
    }
    let bold = iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT };
    // Pings and links in the order they appear. A link that overlaps a ping is shown as plain text.
    let mut pieces: Vec<(std::ops::Range<usize>, Option<Option<Color>>)> = pings
        .iter()
        .map(|p| (p.range.clone(), Some(p.color)))
        .chain(links.into_iter().map(|r| (r, None)))
        .collect();
    pieces.sort_by_key(|(r, _)| r.start);
    let mut spans: Vec<iced::widget::text::Span<'static, String>> = Vec::new();
    let mut at = 0;
    for (range, ping) in pieces {
        if range.start < at {
            continue;
        }
        if range.start > at {
            spans.push(span(body[at..range.start].to_string()).color(pal.text));
        }
        let part = body[range.clone()].to_string();
        spans.push(match ping {
            Some(color) => span(part).color(color.unwrap_or(pal.text_strong)).font(bold),
            None => span(part.clone()).color(pal.link).underline(true).link(part),
        });
        at = range.end;
    }
    if at < body.len() {
        spans.push(span(body[at..].to_string()).color(pal.text));
    }
    rich_text(spans).size(15).width(Length::Shrink).on_link_click(Message::OpenLink).into()
}

/// The colour of a message's author name: their highest role's colour, or the strong text colour
/// (white on the dark themes) when that role has no colour or they have no role.
fn author_color(app: &App, m: &ChatMessage, pal: &Pal) -> Color {
    app.member_color(&m.author_id).unwrap_or(pal.text_strong)
}

fn chat_message<'a>(app: &'a App, pal: Pal, m: ChatMessage, grouped: bool, now: u64) -> Element<'a, Message> {
    let m_id = m.id;
    let editing = app.editing.as_ref().is_some_and(|(id, _)| *id == m.id);
    let hovered = app.hover == Some(m_id);

    let body_text: Element<'a, Message> = if editing {
        let value = app.editing.as_ref().map(|(_, t)| t.clone()).unwrap_or_default();
        column![
            text_input("", &value)
                .id(EDIT_ID)
                .on_input(Message::EditChanged)
                .on_submit(Message::EditSave)
                .padding(8)
                .size(14),
            muted_text("Enter to save · Escape to cancel", 12.0, pal.text_muted),
        ]
        .spacing(4)
        .into()
    } else {
        let mut line = row![message_text(app, &m.text, pal)].spacing(6);
        if m.edited {
            line = line.push(text("(edited)").size(11).color(pal.text_muted));
        }
        line.into()
    };

    let mut content = Column::new().spacing(2).width(Length::Fill);
    if !grouped {
        content = content.push(
            row![
                // Names are white unless the author has a role with a colour. Read from the current roles, so
                // a change to them shows on every message at once.
                mouse_area(text(m.author.clone()).size(15).color(author_color(app, &m, &pal)))
                    .on_press(Message::CardOpen { id: m.author_id.clone(), name: m.author.clone() })
                    .on_right_press(Message::OpenMenu(crate::app::MenuKind::User {
                        name: m.author.clone(),
                        can_message: m.author != app.user.clone().unwrap_or_default(),
                    })),
                text(stamp_label(m.timestamp, now)).size(12).color(pal.text_muted),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        );
    }
    content = content.push(body_text);
    if !m.reactions.is_empty() {
        let mut reactions = row![].spacing(4);
        for r in &m.reactions {
            let mine = r.users.iter().any(|u| Some(u) == app.user.as_ref());
            let label = row![emoji(&r.emoji, 16.0), text(r.users.len().to_string()).size(12).color(pal.text)]
                .spacing(4)
                .align_y(iced::alignment::Vertical::Center);
            let border_color = if mine { pal.accent } else { pal.selected_bg };
            reactions = reactions.push(
                button(label)
                    .padding(crate::util::pad2(2.0, 6.0))
                    .on_press(Message::React(m.id, r.emoji.clone()))
                    .style(move |_: &Theme, _| button::Style {
                        background: Some(Background::Color(pal.input_bg)),
                        text_color: pal.text,
                        border: border::rounded(6.0).color(border_color).width(1.0),
                        ..button::Style::default()
                    }),
            );
        }
        content = content.push(reactions);
    }

    let avatar: Element<'a, Message> = if grouped {
        space().width(Length::Fixed(40.0)).into()
    } else {
        let (picture, banner) = app.profile_look(&m.author_id);
        super::avatar(picture, banner, &m.author, 40.0, &pal)
    };
    let mut line = row![avatar, content].spacing(14).width(Length::Fill);
    if hovered && !editing && m.id != 0 {
        line = line.push(toolbar(app, pal, m.id, app.can_delete(&m)));
    }
    let padded = container(line)
        .padding(crate::util::pad2(if grouped { 1.0 } else { 6.0 }, 16.0))
        .width(Length::Fill)
        .style(move |_: &Theme| round(if hovered { pal.hover_bg } else { Color::TRANSPARENT }, 0.0));
    let area = mouse_area(padded)
        .on_enter(Message::Hover(Some(m.id)))
        .on_exit(Message::Hover(None));
    if m.id != 0 && (app.can_edit(&m) || app.can_delete(&m)) {
        area.on_right_press(Message::OpenMenu(crate::app::MenuKind::Message(m.id))).into()
    } else {
        area.into()
    }
}

/// The toolbar that floats over a hovered message: quick reactions, the picker and more.
fn toolbar<'a>(app: &'a App, pal: Pal, id: i64, can_delete: bool) -> Element<'a, Message> {
    let _ = app;
    let mut items = row![].spacing(2).align_y(iced::alignment::Vertical::Center);
    for e in QUICK_REACTIONS {
        items = items.push(ghost(emoji(e, 20.0), Some(Message::React(id, e.to_string())), pal.selected_bg, 4.0));
    }
    items = items.push(ghost(icons::icon(icons::SMILE, 16.0, pal.text), Some(Message::PickerOpen(id)), pal.selected_bg, 4.0));
    if can_delete {
        items = items.push(ghost(icons::icon(icons::GEAR, 16.0, pal.text), Some(Message::OpenMenu(crate::app::MenuKind::Message(id))), pal.selected_bg, 4.0));
    }
    container(items)
        .padding(crate::util::pad2(2.0, 4.0))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(pal.sidebar_bg)),
            border: border::rounded(6.0).color(pal.hover_bg).width(1.0),
            ..container::Style::default()
        })
        .into()
}

/// The menu of roles and members that an `@` in the message box can ping. None when no `@` word is
/// being typed, or nothing matches it.
fn mention_menu<'a>(app: &App, pal: Pal) -> Option<Element<'a, Message>> {
    let query = app.mention_query()?.to_lowercase();
    let roles: Vec<&common::RoleInfo> = app
        .server
        .roles
        .iter()
        .filter(|r| r.mentionable && r.id != common::EVERYONE_ROLE && r.name.to_lowercase().contains(&query))
        .collect();
    let members: Vec<&String> = app
        .server
        .online
        .iter()
        .chain(app.server.offline.iter())
        .filter(|n| n.to_lowercase().contains(&query))
        .take(8)
        .collect();
    if roles.is_empty() && members.is_empty() {
        return None;
    }
    let heading = |t: &str| container(text(t.to_uppercase()).size(11).color(pal.text_muted)).padding(crate::util::pad(4.0, 12.0, 6.0, 12.0));
    let entry = |name: String, icon: Element<'a, Message>, color: Color, hint: &'static str| -> Element<'a, Message> {
        let label = row![
            text(format!("@{name}")).size(14).color(color).width(Length::Fill),
            text(hint).size(12).color(pal.text_muted),
        ]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center);
        button(row![container(icon).width(Length::Fixed(24.0)), label].spacing(10).align_y(iced::alignment::Vertical::Center))
            .padding(crate::util::pad2(6.0, 12.0))
            .width(Length::Fill)
            .on_press(Message::MentionPick(name))
            .style(move |_: &Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Some(Background::Color(pal.hover_bg)),
                    _ => None,
                };
                button::Style { background: bg, text_color: pal.text, border: border::rounded(0.0), ..button::Style::default() }
            })
            .into()
    };
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    if !roles.is_empty() {
        items.push(heading("Roles").into());
        for role in roles {
            let color = crate::util::hex_color(&role.color).unwrap_or(pal.text_strong);
            let dot = container(space()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(move |_: &Theme| round(color, 5.0));
            items.push(entry(role.name.clone(), dot.into(), color, "role"));
        }
    }
    if !members.is_empty() {
        items.push(heading("Members").into());
        for name in members {
            items.push(entry(name.clone(), letter_badge(name, 20.0, &pal), pal.text, "member"));
        }
    }
    Some(
        container(Column::with_children(items).padding(crate::util::pad2(4.0, 0.0)))
            .width(Length::Fixed(300.0))
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(pal.sidebar_bg)),
                border: border::rounded(8.0).color(pal.hover_bg).width(1.0),
                ..container::Style::default()
            })
            .into(),
    )
}

/// The message box at the bottom. Its placeholder names the channel or the person.
/// Your picture with your status as a dot on its bottom-right corner. Invisible shows as grey, so you
/// can still see that you are hidden.
fn own_badge<'a>(picture: &str, banner: &str, name: &str, status: common::PresenceStatus, pal: Pal) -> Element<'a, Message> {
    let dot = status_color(status, &pal).unwrap_or(pal.text_muted);
    let ring = container(space().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(dot)),
        border: border::rounded(5.0).color(pal.user_bar_bg).width(2.0),
        ..container::Style::default()
    });
    let corner = container(ring).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom);
    let picture_el = super::avatar(picture, banner, name, 32.0, &pal);
    container(iced::widget::stack![picture_el, corner]).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)).into()
}

/// The label for a status, as shown on the user bar and in the status menu.
fn status_label(status: common::PresenceStatus) -> &'static str {
    use common::PresenceStatus::*;
    match status {
        Online => "Online",
        Idle => "Idle",
        DoNotDisturb => "Do not disturb",
        Invisible => "Invisible",
    }
}

/// The dot that shows a status beside a name. Invisible members have none.
fn status_color(status: common::PresenceStatus, pal: &Pal) -> Option<Color> {
    use common::PresenceStatus::*;
    match status {
        Online => Some(pal.green),
        Idle => Some(Color::from_rgb8(0xf0, 0xb4, 0x32)),
        DoNotDisturb => Some(pal.red),
        Invisible => None,
    }
}

/// "alice is typing…" for the people who have typed in this channel recently. None when nobody has.
fn typing_line(app: &App) -> Option<String> {
    let now = std::time::Instant::now();
    let names: Vec<&str> = app
        .server
        .typing
        .get(&app.server.channel_id)?
        .iter()
        .filter(|(_, _, at)| now.duration_since(*at) < crate::app::TYPING_FOR)
        .map(|(_, name, _)| name.as_str())
        .collect();
    match names.as_slice() {
        [] => None,
        [one] => Some(format!("{one} is typing…")),
        [a, b] => Some(format!("{a} and {b} are typing…")),
        _ => Some("Several people are typing…".into()),
    }
}

fn composer(app: &App, pal: Pal) -> Element<'_, Message> {
    if !app.may_send_here() {
        return container(text("You can't send messages in this channel").size(14).color(pal.text_muted))
            .padding(crate::util::pad(0.0, 16.0, 20.0, 16.0))
            .width(Length::Fill)
            .into();
    }
    let label = if app.current_person().is_some() || is_dm_channel(&app.server.channel) {
        app.channel_label()
    } else {
        format!("#{}", app.server.channel)
    };
    let placeholder = format!("Message {label}");
    let field = row![
        text_input(&placeholder, &app.input)
            .id(COMPOSER_ID)
            .on_input(Message::InputChanged)
            .on_submit(Message::Send)
            .padding(10)
            .size(15)
            .style(move |_: &Theme, _| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: border::rounded(0.0),
                icon: pal.text_muted,
                placeholder: pal.text_muted,
                value: pal.text,
                selection: pal.accent,
            })
            .width(Length::Fill),
        icons::icon(icons::GIFT, 18.0, pal.text_muted),
        icons::icon(icons::APPS, 18.0, pal.text_muted),
        icons::icon(icons::STICKER, 18.0, pal.text_muted),
        icons::icon(icons::EMOJI, 18.0, pal.text_muted),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center);
    let box_ = container(field)
        .padding(crate::util::pad2(0.0, 12.0))
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.input_bg, 8.0));
    let mut col = Column::new();
    // The @ menu opens above the message box while an @ word is being typed.
    if let Some(menu) = mention_menu(app, pal) {
        col = col.push(container(menu).padding(crate::util::pad(0.0, 16.0, 6.0, 16.0)));
    }
    let mut col = col
        .push(row![icons::icon(icons::PLUS, 18.0, pal.text_muted), box_].spacing(12).align_y(iced::alignment::Vertical::Center))
        .padding(crate::util::pad(0.0, 16.0, 20.0, 16.0));
    // Who is typing in this channel, while their notices are fresh.
    if let Some(line) = typing_line(app) {
        col = col.push(container(text(line).size(12).color(pal.text_muted)).padding(crate::util::pad(4.0, 0.0, 0.0, 16.0)));
    }
    if app.status.starts_with("Error") {
        col = col.push(text(app.status.clone()).size(12).color(pal.red));
    }
    col.into()
}


// ----- members -----

fn members(app: &App, pal: Pal) -> Element<'_, Message> {
    let server = &app.server;
    let owner = server.owner.clone();
    let row_for = |name: &String, online: bool| -> Element<'_, Message> {
        let color = if online { if *name == app.user.clone().unwrap_or_default() { pal.text_strong } else { pal.text } } else { pal.text_muted };
        // A member's highest role colours their name.
        let color = app.server.member_ids.get(name).and_then(|id| app.member_color(id)).unwrap_or(color);
        let (picture, banner) = app.member_id_look(name);
        let status = app.server.member_ids.get(name).and_then(|id| app.server.member_status.get(id)).copied().unwrap_or_default();
        // The status shows as a dot on the bottom-right of the picture, ringed in the sidebar colour.
        let picture_el = super::avatar(picture, banner, name, 32.0, &pal);
        let badge: Element<'_, Message> = match status_color(status, &pal).filter(|_| online) {
            Some(dot) => {
                let ring = container(space().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(dot)),
                    border: border::rounded(5.0).color(pal.sidebar_bg).width(2.0),
                    ..container::Style::default()
                });
                let corner = container(ring).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom);
                container(iced::widget::stack![picture_el, corner]).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)).into()
            }
            None => picture_el,
        };
        // The status text, when they have set one, under the name.
        let status_text = app.server.member_ids.get(name).and_then(|id| app.profiles.get(id)).map(|look| look.status.clone()).unwrap_or_default();
        let mut names = Column::new().push(text(name.clone()).size(15).color(color));
        if !status_text.is_empty() {
            // One line, cut short with an ellipsis when the member list is too narrow for it.
            let room = (app.panel_width("members") - 80.0).max(0.0);
            names = names.push(text(fit_name(&status_text, room, 12.0)).size(12).color(pal.text_muted).wrapping(iced::widget::text::Wrapping::None));
        }
        let mut r = row![badge, names.width(Length::Fill)]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        if owner.as_deref() == Some(name.as_str()) {
            r = r.push(svg(svg::Handle::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/crown.svg"))).width(Length::Fixed(12.0)).height(Length::Fixed(9.0)));
        }
        // A left click opens the member's profile card; a right click opens the menu.
        let id = app.server.member_ids.get(name).cloned().unwrap_or_default();
        mouse_area(container(r).padding(crate::util::pad2(4.0, 8.0)).width(Length::Fill).style(move |_: &Theme| round(Color::TRANSPARENT, 4.0)))
            .on_press(Message::CardOpen { id, name: name.clone() })
            .on_right_press(Message::OpenMenu(crate::app::MenuKind::User {
                name: name.clone(),
                can_message: *name != app.user.clone().unwrap_or_default(),
            }))
            .into()
    };
    let online: Vec<&String> = server.online.iter().collect();
    let offline: Vec<&String> = server.offline.iter().collect();
    let head = |t: String| container(text(t).size(11).color(pal.text_muted)).padding(crate::util::pad(0.0, 12.0, 4.0, 12.0));
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    // Roles set to display separately get their own group, under their highest such role, highest
    // role first. Everyone else online is listed under ONLINE.
    let mut plain: Vec<&String> = Vec::new();
    let mut groups: Vec<(&common::RoleInfo, Vec<&String>)> =
        server.roles.iter().filter(|r| r.hoisted && r.id != common::EVERYONE_ROLE).map(|r| (r, Vec::new())).collect();
    for n in &online {
        match app.top_hoisted_role_by_name(n) {
            Some(top) => {
                if let Some((_, members)) = groups.iter_mut().find(|(r, _)| r.id == top.id) {
                    members.push(n);
                }
            }
            None => plain.push(n),
        }
    }
    for (role, members) in groups.iter().filter(|(_, m)| !m.is_empty()) {
        items.push(head(format!("{} — {}", role.name.to_uppercase(), members.len())).into());
        for n in members {
            items.push(row_for(n, true));
        }
        items.push(space().height(Length::Fixed(12.0)).into());
    }
    items.push(head(format!("ONLINE — {}", plain.len())).into());
    for n in plain {
        items.push(row_for(n, true));
    }
    items.push(space().height(Length::Fixed(12.0)).into());
    items.push(head(format!("OFFLINE — {}", offline.len())).into());
    for n in offline {
        items.push(row_for(n, false));
    }
    panel(
        scrollable(Column::with_children(items).padding(crate::util::pad2(12.0, 8.0)).width(Length::Fill)).height(Length::Fill),
        Length::Fixed(app.panel_width("members")),
        pal.sidebar_bg,
    )
}

// ----- direct messages page -----

/// The direct messages page: the list of people on the left, and the conversation.
pub fn home(app: &App, pal: Pal) -> Element<'_, Message> {
    let person = app.current_person();
    let open = person.is_some() || is_dm_channel(&app.server.channel);
    let persons = app.persons();
    let any = !persons.is_empty();
    let empty_text = if app.prefs.servers.is_empty() {
        "Add a server to see your direct messages."
    } else if app.dm_links.is_empty() {
        "Connecting to your servers..."
    } else {
        "No direct messages yet. Right-click a member's name and choose Message."
    };
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for p in &persons {
        let selected = person.as_deref() == Some(p.id.as_str());
        let color = if selected || p.unread { pal.text_strong } else { pal.text_muted };
        let bg = if selected { pal.selected_bg } else { Color::TRANSPARENT };
        rows.push(
            mouse_area(
                container(text(format!("@ {}", p.name)).size(15).color(color))
                    .padding(crate::util::pad2(0.0, 10.0))
                    .height(Length::Fixed(32.0))
                    .align_y(iced::alignment::Vertical::Center)
                    .width(Length::Fill)
                    .style(move |_: &Theme| round(bg, 4.0)),
            )
            .on_press(Message::OpenPerson(p.id.clone()))
            .into(),
        );
    }
    let list: Element<'_, Message> = if any {
        Column::with_children(rows).spacing(1).into()
    } else {
        muted_text(empty_text, 14.0, pal.text_muted)
    };
    let new_dm = text_input("Message someone by name", &app.dm_new_name)
        .id(NEW_DM_ID)
        .on_input(Message::NewDmName)
        .on_submit(Message::NewDmSubmit)
        .padding(7)
        .size(14);
    let side = panel(
        column![
            container(text("Direct messages").size(12).color(pal.text_muted)).padding(crate::util::pad2(12.0, 8.0)),
            container(new_dm).padding(crate::util::pad(0.0, 8.0, 8.0, 8.0)),
            container(list).padding(crate::util::pad2(4.0, 8.0)).height(Length::Fill),
            user_bar(app, pal),
        ]
        .height(Length::Fill),
        Length::Fixed(app.panel_width("dms")),
        pal.sidebar_bg,
    );
    let title = match &person {
        Some(id) => format!("@ {}", app.person_name(id).unwrap_or_else(|| app.channel_label())),
        None => "Direct messages".to_string(),
    };
    // When the person is on several servers, the picker chooses which one a new message goes to.
    let targets = person.as_deref().map(|p| app.person_targets(p)).unwrap_or_default();
    let mut header_row = row![text(title).size(16).color(pal.text_strong), space().width(Length::Fill)]
        .align_y(iced::alignment::Vertical::Center);
    if targets.len() > 1 {
        let options: Vec<DmChoice> = targets
            .iter()
            .enumerate()
            .map(|(index, t)| DmChoice { index, label: format!("Sending to {}", t.server) })
            .collect();
        let selected = targets
            .iter()
            .position(|t| Some(t.addr.as_str()) == app.main.addr.as_deref())
            .and_then(|i| options.get(i).cloned());
        header_row = header_row.push(iced::widget::pick_list(options, selected, Message::DmServerPick));
    }
    let header = container(header_row)
        .padding(crate::util::pad2(0.0, 16.0))
        .height(Length::Fixed(48.0))
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| round(pal.app_bg, 0.0));
    let center_col: Element<'_, Message> = if open {
        column![header, messages(app, pal, app.current_entries(), &Search::default()), composer(app, pal)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        column![header, container(muted_text("Pick a conversation on the left.", 15.0, pal.text_muted)).center(Length::Fill)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };
    row![side, splitter("dms", false), center_col].height(Length::Fill).into()
}
