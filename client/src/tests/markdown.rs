use super::*;

fn text_of(pieces: &[Piece]) -> String {
    pieces.iter().map(|p| p.text.as_str()).collect()
}

#[test]
fn plain_text_is_one_paragraph_with_its_source_range() {
    let blocks = parse("hello there");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
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
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
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
    assert_eq!(
        blocks,
        vec![Block::Code {
            lang: "rust".into(),
            text: "fn main() {}\n  x".into()
        }]
    );
}

#[test]
fn quotes_and_rules_are_blocks() {
    let blocks = parse("> quoted\n\n---");
    assert!(
        matches!(&blocks[0], Block::Quote(inner) if matches!(&inner[0], Block::Paragraph(p) if text_of(p) == "quoted"))
    );
    assert_eq!(blocks[1], Block::Rule);
}

#[test]
fn lists_keep_their_order_and_nest() {
    let blocks = parse("3. three\n4. four\n   - inner\n\n- bullet");
    let Block::List {
        ordered: true,
        start: 3,
        items,
    } = &blocks[0]
    else {
        panic!("{blocks:?}")
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[1][1], Block::List { ordered: false, .. }));
    assert!(matches!(&blocks[1], Block::List { ordered: false, .. }));
}

#[test]
fn tables_have_a_header_and_rows() {
    let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |");
    let Block::Table { header, rows } = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(text_of(&header[1]), "b");
    assert_eq!(text_of(&rows[0][0]), "1");
}

#[test]
fn links_and_images_carry_their_address() {
    let blocks = parse("[site](https://example.com) and ![pic](https://example.com/a.png)");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(pieces[0].link.as_deref(), Some("https://example.com"));
    assert_eq!(pieces[0].text, "site");
    let image = pieces.iter().find(|p| p.text == "pic").unwrap();
    assert_eq!(image.link.as_deref(), Some("https://example.com/a.png"));
}

#[test]
fn raw_html_is_text() {
    let blocks = parse("<b>not bold</b>");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert!(!pieces.iter().any(|p| p.bold));
    assert!(text_of(pieces).contains("<b>"));
}

#[test]
fn inline_maths_is_text_and_display_maths_is_a_block() {
    let blocks = parse("Energy $E = mc^2$ holds.\n\n$$\\frac{1}{2}$$");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(text_of(pieces), "Energy E = mc² holds.");
    assert_eq!(blocks[1], Block::Math("\\frac{1}{2}".into()));
}

#[test]
fn kbd_and_mark_style_the_text_inside_them() {
    let blocks = parse("<kbd>Ctrl</kbd> + <kbd>C</kbd>\n<mark>Highlighted text</mark>");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert!(pieces.iter().find(|p| p.text == "Ctrl").unwrap().key);
    assert!(
        !pieces.iter().find(|p| p.text == " + ").unwrap().key,
        "text outside the tags is plain"
    );
    assert!(pieces.iter().find(|p| p.text == "Highlighted text").unwrap().mark);
}

#[test]
fn a_details_section_holds_its_summary_and_blocks() {
    let blocks = parse("<details>\n<summary>Click to expand</summary>\n\nThis is hidden.\n\n</details>");
    let Block::Details {
        summary,
        open,
        blocks: inner,
    } = &blocks[0]
    else {
        panic!("{blocks:?}")
    };
    assert_eq!(summary, "Click to expand");
    assert!(!open, "closed unless it says open");
    assert!(matches!(&inner[0], Block::Paragraph(p) if text_of(p) == "This is hidden."));
}

#[test]
fn an_open_details_section_starts_open() {
    let blocks = parse("<details open>\n<summary>Shown</summary>\n\nText\n\n</details>");
    assert!(matches!(&blocks[0], Block::Details { open: true, .. }));
}

#[test]
fn single_line_breaks_stay_line_breaks() {
    let blocks = parse("one\ntwo");
    let Block::Paragraph(pieces) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(text_of(pieces), "one\ntwo");
}
