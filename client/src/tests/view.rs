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
