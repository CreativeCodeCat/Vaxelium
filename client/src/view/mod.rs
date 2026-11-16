//! The views. Each function turns part of the app's state into widgets. Nothing here changes the
//! state: buttons and fields send messages, which `app.rs` and `update.rs` handle.

mod chat;
pub mod markdown_view;
pub mod selectable;
pub use chat::USER_BAR_HEIGHT;
pub(crate) mod pages;
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

/// A profile picture. With none, the person's banner colour (or their name colour when they have no
/// banner) with their initial on it.
pub fn avatar<'a>(image: &str, banner: &str, name: &str, size: f32, pal: &Pal) -> Element<'a, Message> {
    // An empty string decodes to an empty image, so only a real picture is drawn.
    if let Some(handle) = Some(image).filter(|i| !i.is_empty()).and_then(decode_icon) {
        return iced::widget::image(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .border_radius(size / 2.0)
            .into();
    }
    let bg = crate::util::hex_color(banner).unwrap_or_else(|| name_color(pal, name));
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
/// Where each web link in `body` is: an `http://` or `https://` address, up to the next space.
/// Punctuation that ends a sentence is left out of the link.
pub fn link_ranges(body: &str) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    for (start, _) in body.match_indices("http") {
        let rest = &body[start..];
        if !(rest.starts_with("http://") || rest.starts_with("https://")) {
            continue;
        }
        // Only at the start of the text or after a space or an opening bracket, so a link inside another one is not found twice.
        let after_opener = body[..start].ends_with(|c: char| c.is_whitespace() || matches!(c, '(' | '[' | '<' | '"' | '\''));
        if start > 0 && !after_opener {
            continue;
        }
        let mut end = start + rest.find(char::is_whitespace).unwrap_or(rest.len());
        while end > start && matches!(body.as_bytes()[end - 1], b'.' | b',' | b')' | b'!' | b'?' | b';' | b':' | b'"' | b'\'') {
            end -= 1;
        }
        if end - start > "https://".len() {
            found.push(start..end);
        }
    }
    found
}

/// Text in which web links are underlined and open when clicked.
pub fn linked_text<'a>(body: &str, size: f32, color: Color, pal: Pal) -> Element<'a, Message> {
    use iced::widget::text::Span;
    let links = link_ranges(body);
    if links.is_empty() {
        return text(body.to_string()).size(size).color(color).into();
    }
    let mut spans: Vec<Span<'static, String>> = Vec::new();
    let mut at = 0;
    for range in links {
        if range.start > at {
            spans.push(iced::widget::span(body[at..range.start].to_string()).color(color));
        }
        let url = body[range.clone()].to_string();
        spans.push(iced::widget::span(url.clone()).color(pal.link).underline(true).link(url));
        at = range.end;
    }
    if at < body.len() {
        spans.push(iced::widget::span(body[at..].to_string()).color(color));
    }
    iced::widget::rich_text(spans).size(size).on_link_click(Message::OpenLink).into()
}

/// A bio: bold, italic, lists and links, as `bio::styled` and `bio::lists` format it.
pub fn bio_text<'a>(body: &str, size: f32, color: Color, pal: Pal) -> Element<'a, Message> {
    use crate::bio::Style;
    use iced::font::{Style as FontStyle, Weight};
    use iced::widget::text::Span;

    let (plain, styles) = crate::bio::styled(&crate::bio::lists(body));
    let links = link_ranges(&plain);
    // What decides how a character looks: its style, and whether it is inside a link.
    let key_at = |k: usize, pos: usize| {
        let style = styles.get(k).copied().unwrap_or_default();
        (style, links.iter().any(|r| r.contains(&pos)))
    };
    let make = |piece: &str, (style, is_link): (Style, bool)| -> Span<'static, String> {
        let mut font = iced::Font::DEFAULT;
        if style.bold {
            font.weight = Weight::Bold;
        }
        if style.italic {
            font.style = FontStyle::Italic;
        }
        let span = iced::widget::span(piece.to_string()).font(font);
        if is_link {
            span.color(pal.link).underline(true).link(piece.to_string())
        } else {
            let span = span.color(color);
            if style.underline {
                span.underline(true)
            } else {
                span
            }
        }
    };

    // Runs of characters that look the same become one span each.
    let mut spans: Vec<Span<'static, String>> = Vec::new();
    let mut run: Option<((Style, bool), usize)> = None;
    for (k, (pos, _)) in plain.char_indices().enumerate() {
        let key = key_at(k, pos);
        match run {
            Some((current, start)) if current == key => run = Some((current, start)),
            Some((current, start)) => {
                spans.push(make(&plain[start..pos], current));
                run = Some((key, pos));
            }
            None => run = Some((key, pos)),
        }
    }
    if let Some((current, start)) = run {
        spans.push(make(&plain[start..], current));
    }
    iced::widget::rich_text(spans).size(size).on_link_click(Message::OpenLink).into()
}

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
    if let Some(card) = popups::card_layer(app, pal) {
        layers.push(card);
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
    if app.chat_ready() {
        if let Some(channel) = app.channel_edit {
            return pages::channel_edit(app, channel, pal);
        }
        if app.category_edit.is_some() {
            return pages::category_edit(app, pal);
        }
    }
    match app.page {
        Page::Discover => pages::discover(app, pal),
        Page::User => pages::user_settings(app, pal),
        Page::Server if app.chat_ready() => pages::server_settings(app, pal),
        _ => {
            let main: Element<'_, Message> = if app.home && app.chat_ready() {
                chat::home(app, pal)
            } else if app.chat_ready() {
                if app.members_view { chat::members_page(app, pal) } else { chat::channel(app, pal) }
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
pub(super) fn decode_icon(b64: &str) -> Option<iced::widget::image::Handle> {
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

#[cfg(test)]
mod link_tests {
    use super::link_ranges;

    #[test]
    fn finds_web_links_and_leaves_out_trailing_punctuation() {
        let body = "see https://example.com/a, or (http://x.org/b). not a link: ftp://no ahttps://no";
        let found: Vec<&str> = link_ranges(body).into_iter().map(|r| &body[r]).collect();
        assert_eq!(found, vec!["https://example.com/a", "http://x.org/b"]);
    }

    #[test]
    fn a_link_inside_another_is_not_found_twice() {
        let body = "https://a.example/http://b.example";
        assert_eq!(link_ranges(body).len(), 1);
    }
}
