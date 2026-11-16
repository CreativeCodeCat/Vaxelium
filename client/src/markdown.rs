//! Chat messages as Markdown. A message is parsed into blocks (paragraphs, headings, code, quotes,
//! lists, tables, rules), and the text inside them into pieces that remember their style and link.
//! The chat view draws the result. Raw HTML is shown as text, never as markup.
//!
//! The syntax is CommonMark, with GFM tables, ~~strikethrough~~ and task lists as well.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::latex;

/// A run of text with one style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Piece {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// Text in a `<kbd>` tag: a key on the keyboard.
    pub key: bool,
    /// Text in a `<mark>` tag: highlighted.
    pub mark: bool,
    /// The address of the link or image this text is in.
    pub link: Option<String>,
    /// Where the text is in the message. Set only when the text is exactly that part of the message,
    /// so mentions can be found in it.
    pub src: Option<Range<usize>>,
}

/// A block of a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Piece>),
    Heading {
        level: u8,
        pieces: Vec<Piece>,
    },
    /// A code block: its language (empty when none was named), and its text as it is.
    Code {
        lang: String,
        text: String,
    },
    Quote(Vec<Block>),
    /// A list. Each item is the blocks inside it.
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
    },
    Rule,
    Table {
        header: Vec<Vec<Piece>>,
        rows: Vec<Vec<Vec<Piece>>>,
    },
    /// A display formula ($$...$$), as its LaTeX source.
    Math(String),
    /// A `<details>` section: its `<summary>` text, whether it starts open, and the blocks inside.
    Details {
        summary: String,
        open: bool,
        blocks: Vec<Block>,
    },
}

/// The style in force for the text being read.
#[derive(Debug, Clone, Default)]
struct Mark {
    bold: bool,
    italic: bool,
    strike: bool,
    key: bool,
    mark: bool,
    link: Option<String>,
}

/// A container that blocks are being collected into.
enum Frame {
    Quote(Vec<Block>),
    Item(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
    },
    Code {
        lang: String,
        text: String,
    },
    Table {
        header: Vec<Vec<Piece>>,
        rows: Vec<Vec<Vec<Piece>>>,
        row: Vec<Vec<Piece>>,
    },
    Details {
        summary: String,
        open: bool,
        blocks: Vec<Block>,
    },
}

/// Parses a message into blocks. Empty text gives no blocks.
pub fn parse(body: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_MATH);

    let body = &join_display_maths(body);
    let mut builder = Builder::default();
    for (event, range) in Parser::new_ext(body, options).into_offset_iter() {
        builder.event(event, range, body);
    }
    builder.flush();
    builder.root
}

/// Display maths written over several lines, between `$$` lines, put on one line. Otherwise a line
/// inside it such as `=` would make the line before it a heading, and a blank line would end it. Each
/// newline becomes one space, so every byte stays where it was. Code blocks are left alone.
fn join_display_maths(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_maths = false;
    let mut in_code = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim();
        if !in_maths && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_code = !in_code;
        }
        if !in_code && trimmed.contains("$$") {
            // A line that opens and closes on its own, such as `$$x$$`, leaves the state as it was.
            let marks = trimmed.matches("$$").count();
            if marks % 2 == 1 {
                in_maths = !in_maths;
            }
        }
        if in_maths {
            out.push_str(&line.replace('\n', " "));
        } else {
            out.push_str(line);
        }
    }
    out
}

#[derive(Default)]
struct Builder {
    root: Vec<Block>,
    stack: Vec<Frame>,
    /// Pieces of the paragraph, heading or table cell being read.
    pieces: Vec<Piece>,
    cur: Mark,
    marks: Vec<Mark>,
    heading: u8,
}

impl Builder {
    fn event(&mut self, event: Event<'_>, range: Range<usize>, body: &str) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                if let Some(Frame::Code { text: code, .. }) = self.stack.last_mut() {
                    code.push_str(&text);
                    return;
                }
                let src = (body.get(range.clone()) == Some(&*text)).then_some(range);
                self.push_piece(text.to_string(), src);
            }
            Event::Code(code) => {
                let mut piece = self.piece(code.to_string(), None);
                piece.code = true;
                self.pieces.push(piece);
            }
            Event::Html(html) => self.html_block(&html),
            Event::InlineHtml(html) => self.inline_html(&html),
            Event::SoftBreak | Event::HardBreak => self.push_piece("\n".into(), None),
            Event::Rule => {
                self.flush();
                self.sink().push(Block::Rule);
            }
            Event::TaskListMarker(done) => self.push_piece(if done { "☑ " } else { "☐ " }.into(), None),
            Event::FootnoteReference(label) => self.push_piece(format!("[{label}]"), None),
            Event::InlineMath(src) => {
                let text = latex::inline_text(&src);
                self.push_piece(text, None);
            }
            Event::DisplayMath(src) => {
                self.flush();
                self.sink().push(Block::Math(src.to_string()));
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.flush(),
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = heading_number(level);
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.stack.push(Frame::Quote(Vec::new()));
            }
            Tag::List(start) => {
                self.flush();
                self.stack.push(Frame::List {
                    ordered: start.is_some(),
                    start: start.unwrap_or(1),
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                self.flush();
                self.stack.push(Frame::Item(Vec::new()));
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.stack.push(Frame::Code {
                    lang,
                    text: String::new(),
                });
            }
            Tag::Table(_) => {
                self.flush();
                self.stack.push(Frame::Table {
                    header: Vec::new(),
                    rows: Vec::new(),
                    row: Vec::new(),
                });
            }
            Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
            Tag::Emphasis => self.enter(|m| m.italic = true),
            Tag::Strong => self.enter(|m| m.bold = true),
            Tag::Strikethrough => self.enter(|m| m.strike = true),
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                let url = dest_url.to_string();
                self.enter(|m| m.link = Some(url));
            }
            // Other containers hold text that is shown in place.
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                let pieces = std::mem::take(&mut self.pieces);
                let level = self.heading;
                self.sink().push(Block::Heading { level, pieces });
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                if let Some(Frame::Quote(blocks)) = self.stack.pop() {
                    self.sink().push(Block::Quote(blocks));
                }
            }
            TagEnd::List(_) => {
                self.flush();
                if let Some(Frame::List { ordered, start, items }) = self.stack.pop() {
                    self.sink().push(Block::List { ordered, start, items });
                }
            }
            TagEnd::Item => {
                self.flush();
                if let Some(Frame::Item(blocks)) = self.stack.pop() {
                    if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                        items.push(blocks);
                    }
                }
            }
            TagEnd::CodeBlock => {
                if let Some(Frame::Code { lang, mut text }) = self.stack.pop() {
                    // A code block's text ends with a line break that is not part of the code.
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    self.sink().push(Block::Code { lang, text });
                }
            }
            TagEnd::Table => {
                if let Some(Frame::Table { header, rows, .. }) = self.stack.pop() {
                    self.sink().push(Block::Table { header, rows });
                }
            }
            TagEnd::TableHead => {
                let cells = self.take_row();
                if let Some(Frame::Table { header, .. }) = self.stack.last_mut() {
                    *header = cells;
                }
            }
            TagEnd::TableRow => {
                let cells = self.take_row();
                if let Some(Frame::Table { rows, .. }) = self.stack.last_mut() {
                    rows.push(cells);
                }
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.pieces);
                if let Some(Frame::Table { row, .. }) = self.stack.last_mut() {
                    row.push(cell);
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link | TagEnd::Image => {
                self.cur = self.marks.pop().unwrap_or_default();
            }
            _ => {}
        }
    }

    /// Starts a styled run: the current style is kept, then `change` applies to the new one.
    fn enter(&mut self, change: impl FnOnce(&mut Mark)) {
        self.marks.push(self.cur.clone());
        change(&mut self.cur);
    }

    fn take_row(&mut self) -> Vec<Vec<Piece>> {
        match self.stack.last_mut() {
            Some(Frame::Table { row, .. }) => std::mem::take(row),
            _ => Vec::new(),
        }
    }

    fn piece(&self, text: String, src: Option<Range<usize>>) -> Piece {
        Piece {
            text,
            bold: self.cur.bold,
            italic: self.cur.italic,
            strike: self.cur.strike,
            code: false,
            key: self.cur.key,
            mark: self.cur.mark,
            link: self.cur.link.clone(),
            src,
        }
    }

    fn push_piece(&mut self, text: String, src: Option<Range<usize>>) {
        if text.is_empty() {
            return;
        }
        let piece = self.piece(text, src);
        self.pieces.push(piece);
    }

    /// The blocks being collected now: the innermost quote or list item, or the message itself.
    fn sink(&mut self) -> &mut Vec<Block> {
        match self.stack.last_mut() {
            Some(Frame::Quote(blocks)) | Some(Frame::Item(blocks)) => blocks,
            Some(Frame::Details { blocks, .. }) => blocks,
            _ => &mut self.root,
        }
    }

    /// Inline HTML. `<kbd>` and `<mark>` style the text inside them. Other tags are shown as text.
    fn inline_html(&mut self, html: &str) {
        match html.trim().to_ascii_lowercase().as_str() {
            "<kbd>" => self.enter(|m| m.key = true),
            "<mark>" => self.enter(|m| m.mark = true),
            "</kbd>" if self.cur.key => self.cur = self.marks.pop().unwrap_or_default(),
            "</mark>" if self.cur.mark => self.cur = self.marks.pop().unwrap_or_default(),
            _ => self.push_piece(html.to_string(), None),
        }
    }

    /// An HTML block. `<details>` starts a collapsible section, which the `</details>` closes, and its
    /// `<summary>` names it. Other HTML blocks are shown as text.
    fn html_block(&mut self, html: &str) {
        if html.contains("<details") {
            self.flush();
            let tag_end = html.find('>').map_or(html.len(), |i| i + 1);
            let open = html[..tag_end].to_ascii_lowercase().contains("open");
            let summary = between(html, "<summary>", "</summary>")
                .unwrap_or_default()
                .trim()
                .to_string();
            self.stack.push(Frame::Details {
                summary,
                open,
                blocks: Vec::new(),
            });
            return;
        }
        if let Some(Frame::Details { summary, .. }) = self.stack.last_mut() {
            // The summary can come in a later piece of HTML than the opening tag.
            if summary.is_empty() {
                if let Some(found) = between(html, "<summary>", "</summary>") {
                    *summary = found.trim().to_string();
                    return;
                }
            }
            if html.contains("</details>") {
                self.flush();
                if let Some(Frame::Details { summary, open, blocks }) = self.stack.pop() {
                    self.sink().push(Block::Details { summary, open, blocks });
                }
                return;
            }
        }
        self.push_piece(html.to_string(), None);
    }

    /// Turns the pieces read so far into a paragraph, if there are any.
    fn flush(&mut self) {
        if self.pieces.is_empty() {
            return;
        }
        let pieces = std::mem::take(&mut self.pieces);
        self.sink().push(Block::Paragraph(pieces));
    }
}

/// The text between the first `start` and the `end` after it.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(&text[from..to])
}

fn heading_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
#[path = "tests/markdown.rs"]
mod tests;
