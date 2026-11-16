use super::*;

fn kinds(lang: &str, code: &str) -> Vec<(Kind, String)> {
    tokens(lang, code)
        .into_iter()
        .filter(|(k, _)| *k != Kind::Plain)
        .collect()
}

#[test]
fn the_parts_join_back_to_the_code() {
    let code = "fn main() {\n    let x = \"hi\"; // note\n}\n";
    let joined: String = tokens("rust", code).into_iter().map(|(_, t)| t).collect();
    assert_eq!(joined, code);
}

#[test]
fn rust_keywords_strings_comments_and_numbers_are_coloured() {
    let found = kinds("rust", "let x = 42; // answer\nlet s = \"hi\";");
    assert!(found.contains(&(Kind::Keyword, "let".into())));
    assert!(found.contains(&(Kind::Number, "42".into())));
    assert!(found.contains(&(Kind::Comment, "// answer".into())));
    assert!(found.contains(&(Kind::Str, "\"hi\"".into())));
}

#[test]
fn python_and_shell_use_hash_comments() {
    assert!(kinds("python", "def f(): # note").contains(&(Kind::Comment, "# note".into())));
    assert!(kinds("bash", "echo hi # note").contains(&(Kind::Comment, "# note".into())));
}

#[test]
fn rust_attributes_are_not_comments() {
    assert!(kinds("rust", "#[derive(Debug)]")
        .iter()
        .all(|(k, _)| *k != Kind::Comment));
}

#[test]
fn a_name_with_digits_is_not_a_number() {
    assert!(kinds("rust", "x1 + 2").contains(&(Kind::Number, "2".into())));
    assert!(!kinds("rust", "x1 + 2")
        .iter()
        .any(|(k, t)| *k == Kind::Number && t == "1"));
}

#[test]
fn unknown_languages_are_plain() {
    assert!(kinds("cobol", "MOVE 1 TO X").is_empty());
    assert!(kinds("", "let x = 1").is_empty());
}

#[test]
fn an_unclosed_string_stops_at_the_line_end() {
    let found = kinds("rust", "let s = \"open\nnext");
    assert!(found.contains(&(Kind::Str, "\"open".into())));
    assert!(!found.iter().any(|(k, t)| *k == Kind::Str && t.contains("next")));
}
