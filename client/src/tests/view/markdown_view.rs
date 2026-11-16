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
