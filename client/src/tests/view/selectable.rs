use super::*;

fn unit(text: &str) -> Unit {
    Unit {
        text: text.into(),
        color: Color::WHITE,
        bold: false,
        italic: false,
        code: false,
        underline: false,
        strike: false,
        background: None,
        link: None,
    }
}

#[test]
fn units_split_into_grapheme_clusters() {
    let pieces = split(vec![unit("ab"), unit("e\u{301}x")]);
    let texts: Vec<&str> = pieces.iter().map(|u| u.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["a", "b", "e\u{301}", "x"],
        "an accent stays with its letter"
    );
}

#[test]
fn touching_rectangles_on_one_line_merge() {
    let r = |x: f32| Rectangle::new(Point::new(x, 0.0), Size::new(10.0, 14.0));
    let merged = merged(vec![
        r(0.0),
        r(10.0),
        r(20.0),
        Rectangle::new(Point::new(0.0, 30.0), Size::new(5.0, 5.0)),
    ]);
    assert_eq!(
        merged.len(),
        2,
        "three touching boxes on one line become one, the other line stays apart"
    );
    assert_eq!(merged[0].width, 30.0);
}

#[test]
fn a_selection_covers_the_blocks_between_its_ends() {
    // From block 1, span 2, to block 3, span 1.
    let (from, to) = ((1, 2), (3, 1));
    assert_eq!(range_in_block(from, to, 0, 5), None, "before the selection");
    assert_eq!(
        range_in_block(from, to, 1, 5),
        Some(2..5),
        "the first block, from its span on"
    );
    assert_eq!(
        range_in_block(from, to, 2, 5),
        Some(0..5),
        "a block in the middle is all selected"
    );
    assert_eq!(
        range_in_block(from, to, 3, 5),
        Some(0..2),
        "the last block, up to its span"
    );
    assert_eq!(range_in_block(from, to, 4, 5), None, "after the selection");
}

#[test]
fn a_selection_can_run_backwards() {
    assert_eq!(range_in_block((2, 0), (1, 3), 1, 5), Some(3..5));
}
