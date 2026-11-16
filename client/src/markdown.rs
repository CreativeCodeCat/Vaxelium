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
    Heading { level: u8, pieces: Vec<Piece> },
    /// A code block: its language (empty when none was named), and its text as it is.
    Code { lang: String, text: String },
    Quote(Vec<Block>),
    /// A list. Each item is the blocks inside it.
    List { ordered: bool, start: u64, items: Vec<Vec<Block>> },
    Rule,
    Table { header: Vec<Vec<Piece>>, rows: Vec<Vec<Vec<Piece>>> },
    /// A display formula ($$...$$), as its LaTeX source.
    Math(String),
}

/// The style in force for the text being read.
#[derive(Debug, Clone, Default)]
struct Mark {
    bold: bool,
    italic: bool,
    strike: bool,
    link: Option<String>,
}

/// A container that blocks are being collected into.
enum Frame {
    Quote(Vec<Block>),
    Item(Vec<Block>),
    List { ordered: bool, start: u64, items: Vec<Vec<Block>> },
    Code { lang: String, text: String },
    Table { header: Vec<Vec<Piece>>, rows: Vec<Vec<Vec<Piece>>>, row: Vec<Vec<Piece>> },
}

/// Parses a message into blocks. Empty text gives no blocks.
pub fn parse(body: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_MATH);

    let mut builder = Builder::default();
    for (event, range) in Parser::new_ext(body, options).into_offset_iter() {
        builder.event(event, range, body);
    }
    builder.flush();
    builder.root
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
            Event::Html(html) | Event::InlineHtml(html) => self.push_piece(html.to_string(), None),
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
                self.stack.push(Frame::List { ordered: start.is_some(), start: start.unwrap_or(1), items: Vec::new() });
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
                self.stack.push(Frame::Code { lang, text: String::new() });
            }
            Tag::Table(_) => {
                self.flush();
                self.stack.push(Frame::Table { header: Vec::new(), rows: Vec::new(), row: Vec::new() });
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
            _ => &mut self.root,
        }
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
mod tests {
    use super::*;

    fn text_of(pieces: &[Piece]) -> String {
        pieces.iter().map(|p| p.text.as_str()).collect()
    }

    #[test]
    fn plain_text_is_one_paragraph_with_its_source_range() {
        let blocks = parse("hello there");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(text_of(pieces), "hello there");
        assert_eq!(pieces[0].src, Some(0..11));
    }

    #[test]
    fn empty_text_has_no_blocks() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn emphasis_strikethrough_and_code_are_styled() {
        let blocks = parse("**b** *i* ~~s~~ `c`");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        let bold = pieces.iter().find(|p| p.text == "b").unwrap();
        assert!(bold.bold && !bold.italic);
        assert!(pieces.iter().find(|p| p.text == "i").unwrap().italic);
        assert!(pieces.iter().find(|p| p.text == "s").unwrap().strike);
        assert!(pieces.iter().find(|p| p.text == "c").unwrap().code);
    }

    #[test]
    fn headings_have_their_level() {
        let blocks = parse("# Title\n\n### Small");
        assert!(matches!(&blocks[0], Block::Heading { level: 1, pieces } if text_of(pieces) == "Title"));
        assert!(matches!(&blocks[1], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn fenced_code_keeps_its_text_as_it_is() {
        let blocks = parse("```rust\nfn main() {}\n  x\n```");
        assert_eq!(blocks, vec![Block::Code { lang: "rust".into(), text: "fn main() {}\n  x".into() }]);
    }

    #[test]
    fn quotes_and_rules_are_blocks() {
        let blocks = parse("> quoted\n\n---");
        assert!(matches!(&blocks[0], Block::Quote(inner) if matches!(&inner[0], Block::Paragraph(p) if text_of(p) == "quoted")));
        assert_eq!(blocks[1], Block::Rule);
    }

    #[test]
    fn lists_keep_their_order_and_nest() {
        let blocks = parse("3. three\n4. four\n   - inner\n\n- bullet");
        let Block::List { ordered: true, start: 3, items } = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[1][1], Block::List { ordered: false, .. }));
        assert!(matches!(&blocks[1], Block::List { ordered: false, .. }));
    }

    #[test]
    fn tables_have_a_header_and_rows() {
        let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |");
        let Block::Table { header, rows } = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(text_of(&header[1]), "b");
        assert_eq!(text_of(&rows[0][0]), "1");
    }

    #[test]
    fn links_and_images_carry_their_address() {
        let blocks = parse("[site](https://example.com) and ![pic](https://example.com/a.png)");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(pieces[0].link.as_deref(), Some("https://example.com"));
        assert_eq!(pieces[0].text, "site");
        let image = pieces.iter().find(|p| p.text == "pic").unwrap();
        assert_eq!(image.link.as_deref(), Some("https://example.com/a.png"));
    }

    #[test]
    fn raw_html_is_text() {
        let blocks = parse("<b>not bold</b>");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        assert!(!pieces.iter().any(|p| p.bold));
        assert!(text_of(pieces).contains("<b>"));
    }

    #[test]
    fn inline_maths_is_text_and_display_maths_is_a_block() {
        let blocks = parse("Energy $E = mc^2$ holds.\n\n$$\\frac{1}{2}$$");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(text_of(pieces), "Energy E = mc² holds.");
        assert_eq!(blocks[1], Block::Math("\\frac{1}{2}".into()));
    }

    #[test]
    fn single_line_breaks_stay_line_breaks() {
        let blocks = parse("one\ntwo");
        let Block::Paragraph(pieces) = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(text_of(pieces), "one\ntwo");
    }
}
