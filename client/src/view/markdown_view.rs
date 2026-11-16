//! Draws a chat message's Markdown. Blocks become a column. Text is selectable, and a selection can run
//! across the blocks of a message.

use std::ops::Range;

use iced::border;
use iced::widget::{column, container, row, scrollable, space, text};
use iced::{Color, Element, Length, Theme};

use super::selectable::{range_in_block, selectable, split, Step, Unit};
use super::{divider, link_ranges, round};
use crate::app::{App, Message};
use crate::highlight;
use crate::latex::{self, Node};
use crate::markdown::{self, Block, Piece};
use crate::model::RolePing;
use crate::theme::Pal;

/// A block of selectable text, in the order a message has them: its units (one per grapheme) and size.
struct Text {
    units: Vec<Unit>,
    size: f32,
}

/// The selection a message is drawn with: its two ends, and whether it is being dragged.
#[derive(Clone, Copy)]
pub struct Selection {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub dragging: bool,
}

/// A message's body, drawn as Markdown. `message` is the message's id, which the selection reports.
pub fn message_body<'a>(app: &App, message: i64, body: &str, pal: Pal) -> Element<'a, Message> {
    let blocks = markdown::parse(body);
    let pings = app.role_pings(body);
    let mut texts = Vec::new();
    describe(&blocks, &pings, pal, &mut texts);
    let selection = app
        .text_sel
        .as_ref()
        .filter(|s| s.message == message)
        .map(|s| Selection { from: s.anchor, to: s.focus, dragging: s.dragging });
    let mut view = View { texts, next: 0, message, selection, pal };
    view.blocks(&blocks)
}

/// The text a selection from `from` to `to` covers in a message's body. Blocks are joined by a line
/// break.
pub fn selected_text(body: &str, from: (usize, usize), to: (usize, usize), pal: Pal) -> String {
    let blocks = markdown::parse(body);
    let mut texts = Vec::new();
    describe(&blocks, &[], pal, &mut texts);
    let mut parts: Vec<String> = Vec::new();
    for (index, t) in texts.iter().enumerate() {
        if let Some(range) = range_in_block(from, to, index, t.units.len()) {
            parts.push(t.units[range].iter().map(|u| u.text.as_str()).collect());
        }
    }
    parts.join("\n")
}

/// Every block of text in a message, in order. The drawing takes them in the same order, so a span's
/// position in the selection means the same thing in both.
fn describe(blocks: &[Block], pings: &[RolePing], pal: Pal, out: &mut Vec<Text>) {
    for block in blocks {
        match block {
            Block::Paragraph(pieces) => out.push(Text { units: split(units(pieces, pings, pal)), size: 15.0 }),
            Block::Heading { level, pieces } => {
                let bold: Vec<Piece> = pieces.iter().cloned().map(|p| Piece { bold: true, ..p }).collect();
                out.push(Text { units: split(units(&bold, pings, pal)), size: heading_size(*level) });
            }
            Block::Code { lang, text: code } => {
                let parts: Vec<Unit> = highlight::tokens(lang, code)
                    .into_iter()
                    .map(|(kind, part)| Unit { code: true, ..unit(&part, token_color(kind, pal)) })
                    .collect();
                out.push(Text { units: split(parts), size: 13.0 });
            }
            Block::Quote(inner) => describe(inner, pings, pal, out),
            Block::List { items, .. } => {
                for item in items {
                    describe(item, pings, pal, out);
                }
            }
            Block::Table { header, rows } => {
                for cell in header {
                    let bold: Vec<Piece> = cell.iter().cloned().map(|p| Piece { bold: true, ..p }).collect();
                    out.push(Text { units: split(units(&bold, pings, pal)), size: 14.0 });
                }
                for row in rows {
                    for cell in row {
                        out.push(Text { units: split(units(cell, pings, pal)), size: 14.0 });
                    }
                }
            }
            Block::Rule | Block::Math(_) => {}
        }
    }
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => 22.0,
        2 => 19.0,
        3 => 17.0,
        _ => 15.0,
    }
}

/// Draws the blocks, taking each block of text from the list made by `describe`.
struct View {
    texts: Vec<Text>,
    next: usize,
    message: i64,
    selection: Option<Selection>,
    pal: Pal,
}

impl View {
    /// The next block of text, drawn as selectable text.
    fn text_block<'a>(&mut self) -> Element<'a, Message> {
        let index = self.next;
        self.next += 1;
        let Some(t) = self.texts.get(index) else {
            return space().into();
        };
        let units = t.units.clone();
        let size = t.size;
        let selected = self.selection.and_then(|s| range_in_block(s.from, s.to, index, units.len()));
        let dragging = self.selection.is_some_and(|s| s.dragging);
        let message = self.message;
        selectable(
            units,
            size,
            selection_color(self.pal),
            selected,
            dragging,
            move |step: Step| Message::TextStep { message, block: index, step },
            Message::OpenLink,
        )
    }

    fn blocks<'a>(&mut self, blocks: &[Block]) -> Element<'a, Message> {
        let items: Vec<Element<'a, Message>> = blocks.iter().map(|b| self.block(b)).collect();
        column(items).spacing(6).into()
    }

    fn block<'a>(&mut self, block: &Block) -> Element<'a, Message> {
        let pal = self.pal;
        match block {
            Block::Paragraph(_) | Block::Heading { .. } => self.text_block(),
            Block::Code { .. } => {
                // Long lines scroll sideways instead of wrapping, so the code keeps its layout.
                let code_text = self.text_block();
                container(scrollable(code_text).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default())))
                    .padding(8)
                    .style(move |_: &Theme| round(panel_background(pal), 6.0))
                    .into()
            }
            // A quote is a panel. A bar beside it would need to stretch to the quote's height, and that
            // collapses the message's layout.
            Block::Quote(inner) => container(self.blocks(inner))
                .padding([6, 10])
                .style(move |_: &Theme| round(panel_background(pal), 6.0))
                .into(),
            Block::List { ordered, start, items } => {
                let rows: Vec<Element<'a, Message>> = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let marker = if *ordered { format!("{}.", start + i as u64) } else { "•".to_string() };
                        row![text(marker).size(15).color(pal.text).width(Length::Fixed(28.0)), self.blocks(item)]
                            .spacing(4)
                            .into()
                    })
                    .collect();
                column(rows).spacing(4).into()
            }
            Block::Rule => divider(pal.selected_bg),
            Block::Math(src) => math_block(src, pal),
            Block::Table { header, rows } => {
                let mut lines: Vec<Element<'a, Message>> = Vec::new();
                let head: Vec<Element<'a, Message>> = header.iter().map(|_| self.table_cell()).collect();
                lines.push(table_row(head, pal));
                for r in rows {
                    let cells: Vec<Element<'a, Message>> = r.iter().map(|_| self.table_cell()).collect();
                    lines.push(table_row(cells, pal));
                }
                column(lines).into()
            }
        }
    }

    fn table_cell<'a>(&mut self) -> Element<'a, Message> {
        self.text_block()
    }
}

/// One table row. Every cell has a 1px border, so neighbouring cells share their lines.
fn table_row<'a>(cells: Vec<Element<'a, Message>>, pal: Pal) -> Element<'a, Message> {
    let items: Vec<Element<'a, Message>> = cells
        .into_iter()
        .map(|cell| {
            container(cell)
                .padding([4, 8])
                .width(Length::FillPortion(1))
                .style(move |_: &Theme| container::Style {
                    border: border::rounded(0.0).color(pal.selected_bg).width(1.0),
                    ..container::Style::default()
                })
                .into()
        })
        .collect();
    row(items).into()
}

/// A display formula. It scrolls sideways when it is wider than the message.
fn math_block<'a>(src: &str, pal: Pal) -> Element<'a, Message> {
    let nodes = latex::parse(src);
    container(scrollable(math_view(&nodes, 18.0, pal)).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default())))
        .padding(6)
        .into()
}

/// A formula laid out in two dimensions: superscripts raised, subscripts lowered, fractions stacked
/// with a bar, and roots with a bar over their body.
fn math_view<'a>(nodes: &[Node], size: f32, pal: Pal) -> Element<'a, Message> {
    let items: Vec<Element<'a, Message>> = nodes.iter().map(|n| math_node(n, size, pal)).collect();
    row(items).align_y(iced::alignment::Vertical::Top).into()
}

fn math_node<'a>(node: &Node, size: f32, pal: Pal) -> Element<'a, Message> {
    match node {
        Node::Text(t) => text(t.clone()).size(size).color(pal.text).into(),
        Node::Group(inner) => math_view(inner, size, pal),
        // Top alignment in the row raises a superscript; a spacer lowers a subscript.
        Node::Sup(arg) => math_view(std::slice::from_ref(&**arg), size * 0.7, pal),
        Node::Sub(arg) => column![
            space().height(Length::Fixed(size * 0.45)),
            math_view(std::slice::from_ref(&**arg), size * 0.7, pal),
        ]
        .into(),
        Node::Frac(top, bottom) => {
            let width = est_width(top, size * 0.9).max(est_width(bottom, size * 0.9)) + 4.0;
            column![
                math_view(top, size * 0.9, pal),
                container(space())
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(1.0))
                    .style(move |_: &Theme| round(pal.text, 0.0)),
                math_view(bottom, size * 0.9, pal),
            ]
            .align_x(iced::alignment::Horizontal::Center)
            .into()
        }
        Node::Root(body) => {
            let width = est_width(body, size) + 4.0;
            row![
                text("√").size(size * 1.1).color(pal.text),
                column![
                    container(space())
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(1.0))
                        .style(move |_: &Theme| round(pal.text, 0.0)),
                    math_view(body, size, pal),
                ],
            ]
            .align_y(iced::alignment::Vertical::Top)
            .into()
        }
    }
}

/// A rough width of a formula, used where a bar has to be drawn to a length.
fn est_width(nodes: &[Node], size: f32) -> f32 {
    nodes
        .iter()
        .map(|n| match n {
            Node::Text(t) => t.chars().count() as f32 * size * 0.55,
            Node::Group(inner) => est_width(inner, size),
            Node::Sup(arg) | Node::Sub(arg) => est_width(std::slice::from_ref(&**arg), size * 0.7),
            Node::Frac(top, bottom) => est_width(top, size * 0.9).max(est_width(bottom, size * 0.9)),
            Node::Root(body) => est_width(body, size) + size * 0.6,
        })
        .sum()
}

/// The highlight colour of selected text.
fn selection_color(pal: Pal) -> Color {
    Color { a: 0.35, ..pal.accent }
}

/// The text of a run of pieces, as units to draw and select.
fn units(pieces: &[Piece], pings: &[RolePing], pal: Pal) -> Vec<Unit> {
    let mut out = Vec::new();
    for piece in pieces {
        push_units(&mut out, piece, pings, pal);
    }
    out
}

/// A plain unit of text in a colour.
fn unit(text: &str, color: Color) -> Unit {
    Unit {
        text: text.to_string(),
        color,
        bold: false,
        italic: false,
        code: false,
        underline: false,
        strike: false,
        background: None,
        link: None,
    }
}

/// What a part of a piece is: plain text, a mention of a role, or a bare web link.
enum Kind {
    Plain,
    Ping(Option<Color>),
    Link(String),
}

fn push_units(out: &mut Vec<Unit>, piece: &Piece, pings: &[RolePing], pal: Pal) {
    if piece.text.is_empty() {
        return;
    }
    let mut parts: Vec<(Range<usize>, Kind)> = Vec::new();
    // Mentions are found in the message's own text, so only pieces that are exactly that text have them.
    if let Some(src) = &piece.src {
        for ping in pings {
            if ping.range.start >= src.start && ping.range.end <= src.end {
                let inside = ping.range.start - src.start..ping.range.end - src.start;
                parts.push((inside, Kind::Ping(ping.color)));
            }
        }
    }
    // Web links written out in plain text become links. Text that is already a link is left alone.
    if piece.link.is_none() {
        for range in link_ranges(&piece.text) {
            if !parts.iter().any(|(r, _)| overlaps(r, &range)) {
                let url = piece.text[range.clone()].to_string();
                parts.push((range, Kind::Link(url)));
            }
        }
    }
    parts.sort_by_key(|(r, _)| r.start);

    let mut at = 0;
    for (range, kind) in parts {
        if range.start < at {
            continue;
        }
        if range.start > at {
            out.push(make(&piece.text[at..range.start], piece, Kind::Plain, pal));
        }
        out.push(make(&piece.text[range.clone()], piece, kind, pal));
        at = range.end;
    }
    if at < piece.text.len() {
        out.push(make(&piece.text[at..], piece, Kind::Plain, pal));
    }
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

fn is_web(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn make(part: &str, piece: &Piece, kind: Kind, pal: Pal) -> Unit {
    let mut u = Unit {
        bold: piece.bold,
        italic: piece.italic,
        code: piece.code,
        strike: piece.strike,
        background: piece.code.then(|| panel_background(pal)),
        ..unit(part, pal.text)
    };
    match kind {
        Kind::Ping(color) => {
            // Mentions are bold in the mention's colour, as they were before Markdown.
            u.bold = true;
            u.color = color.unwrap_or(pal.text_strong);
        }
        Kind::Link(url) => {
            u.color = pal.link;
            u.underline = true;
            u.link = Some(url);
        }
        Kind::Plain => match &piece.link {
            Some(url) if is_web(url) => {
                u.color = pal.link;
                u.underline = true;
                u.link = Some(url.clone());
            }
            // A link to somewhere that is not a web page is shown, but cannot be opened.
            Some(_) => {
                u.color = pal.link;
                u.underline = true;
            }
            None => u.color = if piece.code { pal.text_strong } else { pal.text },
        },
    }
    u
}

/// The colour of a part of code: keywords in the accent colour, strings in green, comments muted, and
/// numbers in amber.
fn token_color(kind: highlight::Kind, pal: Pal) -> Color {
    use highlight::Kind;
    match kind {
        Kind::Plain => pal.text,
        Kind::Keyword => pal.accent,
        Kind::Str => pal.green,
        Kind::Comment => pal.text_muted,
        Kind::Number => Color::from_rgb8(0xE5, 0xA5, 0x3B),
    }
}

/// The background of code and quotes, inline and in blocks. It is darker than the chat, so it shows.
fn panel_background(pal: Pal) -> Color {
    pal.rail_bg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_across_blocks_copies_them_with_line_breaks() {
        // Blocks in order: "First", the list items "one" and "two", then "Last".
        let body = "First\n\n- one\n- two\n\nLast";
        let pal = crate::theme::palette("dark");
        let copied = selected_text(body, (0, 2), (3, 1), pal);
        assert_eq!(copied, "rst\none\ntwo\nLa");
    }

    #[test]
    fn a_selection_inside_one_block_copies_just_that_part() {
        let pal = crate::theme::palette("dark");
        assert_eq!(selected_text("Hello **bold** world", (0, 6), (0, 9), pal), "bold");
    }
}
