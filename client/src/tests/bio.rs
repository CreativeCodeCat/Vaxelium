use super::*;

fn plain(body: &str) -> String {
    styled(body).0
}

#[test]
fn bullets_and_numbers_are_formatted() {
    assert_eq!(lists("Hi\n- one\n  * two\nlast"), "Hi\n• one\n• two\nlast");
    assert_eq!(lists("1. a\n2) b\n\n5. c"), "1. a\n2. b\n\n5. c");
    // Numbers count on from the first item.
    assert_eq!(lists("3. x\n9. y"), "3. x\n4. y");
    // A number with no space after the dot is not a list item.
    assert_eq!(lists("2.5 kg"), "2.5 kg");
}

#[test]
fn bold_and_italic_marks_are_removed_and_styled() {
    let (text, styles) = styled("a **b** *c* ***d***");
    assert_eq!(text, "a b c d");
    let bold: Vec<bool> = styles.iter().map(|s| s.bold).collect();
    let italic: Vec<bool> = styles.iter().map(|s| s.italic).collect();
    // "a b c d": b is bold, c is italic, d is both.
    assert_eq!(bold, vec![false, false, true, false, false, false, true]);
    assert_eq!(italic, vec![false, false, false, false, true, false, true]);
}

#[test]
fn unpaired_marks_are_shown_as_they_are() {
    assert_eq!(plain("5 * 3 = 15"), "5 * 3 = 15");
    assert_eq!(plain("**open"), "**open");
}

#[test]
fn underscores_around_text_underline_it() {
    let (text, styles) = styled("a _b_ c");
    assert_eq!(text, "a b c");
    let underline: Vec<bool> = styles.iter().map(|s| s.underline).collect();
    assert_eq!(underline, vec![false, false, true, false, false]);
}

#[test]
fn underline_works_with_bold_and_italic() {
    let (text, styles) = styled("_**x**_");
    assert_eq!(text, "x");
    assert!(styles[0].underline && styles[0].bold && !styles[0].italic);
}

#[test]
fn underscores_inside_words_and_stray_ones_are_plain() {
    assert_eq!(plain("my_snake_case_name"), "my_snake_case_name");
    assert!(styled("my_snake_case_name").1.iter().all(|s| !s.underline));
    assert_eq!(plain("_open only"), "_open only");
    assert_eq!(plain("5 _ 3"), "5 _ 3");
    assert_eq!(plain("trailing_ x"), "trailing_ x");
}

#[test]
fn styles_stay_on_one_line() {
    assert_eq!(plain("**a\nb**"), "**a\nb**");
}
