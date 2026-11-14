//! The views. Each function turns part of the app's state into widgets. Nothing here changes the
//! state: buttons and fields send messages, which `app.rs` and `update.rs` handle.

mod chat;
mod pages;
mod popups;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::widget::{button, column, container, mouse_area, row, space, stack, text};
use iced::{border, mouse, Background, Color, Element, Length, Theme};

use crate::app::{App, Message, Page};
use crate::icons;
use crate::theme::Pal;
use crate::util::{initial, name_color};


/// The height of the titlebar, in pixels.
pub const TITLEBAR_HEIGHT: f32 = 32.0;

/// A fixed-size coloured square with a letter, used for avatars and icons without a picture.
pub fn letter_badge<'a>(name: &str, size: f32, pal: &Pal) -> Element<'a, Message> {
    let bg = name_color(pal, name);
    container(text(initial(name)).size(size * 0.45).color(Color::WHITE))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| round(bg, size / 2.0))
        .into()
}

/// A plain container style: background and corner radius.
pub fn round(bg: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg)),
        border: border::rounded(radius),
        ..container::Style::default()
    }
}

/// A transparent button that shows `hover` behind it when the pointer is over it.
pub fn ghost<'a>(content: impl Into<Element<'a, Message>>, msg: Option<Message>, hover: Color, radius: f32) -> Element<'a, Message> {
    let mut b = button(content).padding(crate::util::pad2(2.0, 4.0)).style(move |_: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(hover)),
            _ => None,
        };
        button::Style { background: bg, border: border::rounded(radius), ..button::Style::default() }
    });
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}

/// A filled button in a colour, with text in another.
pub fn solid<'a>(label: &str, msg: Message, bg: Color, fg: Color, width: Option<f32>) -> Element<'a, Message> {
    let mut b = button(container(text(label.to_string()).size(14).color(fg)).align_x(iced::alignment::Horizontal::Center))
        .padding(crate::util::pad2(8.0, 14.0))
        .on_press(msg)
        .style(move |_: &Theme, status: button::Status| {
            let shade = match status {
                button::Status::Hovered => Color { a: 0.85, ..bg },
                _ => bg,
            };
            button::Style { background: Some(Background::Color(shade)), text_color: fg, border: border::rounded(6.0), ..button::Style::default() }
        });
    if let Some(w) = width {
        b = b.width(Length::Fixed(w));
    }
    b.into()
}

/// A text link: coloured text that is a button.
pub fn link<'a>(label: &str, msg: Message, color: Color) -> Element<'a, Message> {
    button(text(label.to_string()).size(14).color(color))
        .padding(crate::util::pad2(2.0, 4.0))
        .style(|_: &Theme, _| button::Style { background: None, text_color: Color::WHITE, ..button::Style::default() })
        .on_press(msg)
        .into()
}

/// A thin filled rectangle.
pub fn bar<'a>(width: f32, height: f32, color: Color) -> Element<'a, Message> {
    container(space())
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(move |_: &Theme| round(color, 0.0))
        .into()
}

/// A horizontal divider line.
pub fn divider<'a>(color: Color) -> Element<'a, Message> {
    container(space()).width(Length::Fill).height(Length::Fixed(1.0)).style(move |_: &Theme| round(color, 0.0)).into()
}

/// A panel: a column with a background.
pub fn panel<'a>(content: impl Into<Element<'a, Message>>, width: Length, bg: Color) -> Element<'a, Message> {
    container(content).width(width).height(Length::Fill).style(move |_: &Theme| round(bg, 0.0)).into()
}

/// A text label in the app's muted colour.
pub fn muted_text<'a>(s: impl Into<String>, size: f32, color: Color) -> Element<'a, Message> {
    text(s.into()).size(size).color(color).into()
}

/// The whole window: the titlebar, the auth banner, the page, and the popups over it.
pub fn view(app: &App) -> Element<'_, Message> {
    let pal = app.pal();
    let body = body(app, pal);
    let frame = column![titlebar(app, pal), auth_banner(app, pal), body].height(Length::Fill);
    let mut layers: Vec<Element<'_, Message>> = vec![frame.into()];
    if let Some(menu) = popups::menu_layer(app, pal) {
        layers.push(menu);
    }
    if let Some(picker) = popups::picker_layer(app, pal) {
        layers.push(picker);
    }
    if let Some(filters) = chat::search_filters_layer(app, pal) {
        layers.push(filters);
    }
    if let Some(dialog) = popups::dialog_layer(app, pal) {
        layers.push(dialog);
    }
    layers.push(popups::resize_edges());
    stack(layers).into()
}

fn body(app: &App, pal: Pal) -> Element<'_, Message> {
    if !app.is_logged_in() {
        return pages::login(app, pal);
    }
    match app.page {
        Page::Discover => pages::discover(app, pal),
        Page::User => pages::user_settings(app, pal),
        Page::Server if app.chat_ready() => pages::server_settings(app, pal),
        _ => {
            let main: Element<'_, Message> = if app.home && app.chat_ready() {
                chat::home(app, pal)
            } else if app.chat_ready() {
                chat::channel(app, pal)
            } else {
                pages::server_status(app, pal)
            };
            row![chat::rail(app, pal), main].height(Length::Fill).into()
        }
    }
}

fn titlebar(app: &App, pal: Pal) -> Element<'_, Message> {
    let title = if !app.is_logged_in() {
        "Vaxelium".to_string()
    } else if !app.chat_ready() {
        app.selected_name().unwrap_or_else(|| "Vaxelium".into())
    } else if app.home {
        app.channel_label()
    } else {
        format!("{}  ·  {}", app.channel_label(), app.selected_name().unwrap_or_default())
    };
    let drag = mouse_area(
        container(text(title).size(13).color(pal.text))
            .width(Length::Fill)
            .height(Length::Fixed(TITLEBAR_HEIGHT))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::WinDrag);
    let controls = row![
        titlebar_button(icons::MINIMIZE, Message::WinMinimize, pal.text, pal.hover_bg),
        titlebar_button(icons::MAXIMIZE, Message::WinMaxToggle, pal.text, pal.hover_bg),
        titlebar_button(icons::CLOSE, Message::WinClose, pal.text, pal.red),
    ];
    container(row![drag, controls])
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.rail_bg, 0.0))
        .into()
}

fn titlebar_button<'a>(icon: &'static str, msg: Message, fg: Color, hover: Color) -> Element<'a, Message> {
    button(icons::icon(icon, 14.0, fg))
        .padding(crate::util::pad2(6.0, 14.0))
        .on_press(msg)
        .style(move |_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => Some(Background::Color(hover)),
                _ => None,
            };
            button::Style { background: bg, text_color: fg, border: border::rounded(0.0), ..button::Style::default() }
        })
        .into()
}

fn auth_banner(app: &App, pal: Pal) -> Element<'_, Message> {
    if !app.auth_down || app.auth_banner_closed {
        return space().height(Length::Fixed(0.0)).into();
    }
    let text_row = row![
        text("The auth service cannot be reached. You can't log in or create accounts until it is back. Servers still accept your saved login for up to 24 hours.")
            .size(13)
            .color(pal.on_color)
            .width(Length::Fill),
        ghost(icons::icon(icons::CLOSE, 12.0, pal.on_color), Some(Message::AuthBannerClose), Color::TRANSPARENT, 4.0),
    ]
    .spacing(8)
    .padding(crate::util::pad2(6.0, 12.0));
    container(text_row)
        .width(Length::Fill)
        .style(move |_: &Theme| round(pal.red, 0.0))
        .into()
}

/// Shows a server's picture, or a coloured square with its first letter.
/// `radius` rounds the picture's corners, so it matches the shape it is shown in.
pub fn server_icon<'a>(icon: Option<&str>, name: &str, size: f32, pal: &Pal, radius: f32) -> Element<'a, Message> {
    match icon.and_then(decode_icon) {
        Some(handle) => iced::widget::image(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .border_radius(radius)
            .into(),
        None => {
            let bg = name_color(pal, name);
            container(text(initial(name)).size(size * 0.45).color(Color::WHITE))
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_: &Theme| round(bg, size / 2.0))
                .into()
        }
    }
}

/// Decoded server pictures, by their base64 text. An image handle gets a new identity each time it
/// is made, so the same handle must be reused every frame, or the picture is reloaded every frame.
static ICON_CACHE: OnceLock<Mutex<HashMap<String, iced::widget::image::Handle>>> = OnceLock::new();

/// Turns a base64 PNG from the server into an image handle, reusing the handle made before.
fn decode_icon(b64: &str) -> Option<iced::widget::image::Handle> {
    use base64::Engine;
    let cache = ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(handle) = cache.get(b64) {
        return Some(handle.clone());
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let handle = iced::widget::image::Handle::from_bytes(bytes);
    cache.insert(b64.to_string(), handle.clone());
    Some(handle)
}

/// The cursor for something that can be dragged.
pub fn drag_cursor() -> mouse::Interaction {
    mouse::Interaction::ResizingHorizontally
}
