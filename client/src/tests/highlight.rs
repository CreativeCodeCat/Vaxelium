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

#[test]
fn sql_keywords_ignore_case() {
    let found = kinds("sql", "SELECT name from users -- all\nWHERE id = 1;");
    assert!(found.contains(&(Kind::Keyword, "SELECT".into())));
    assert!(found.contains(&(Kind::Keyword, "from".into())));
    assert!(found.contains(&(Kind::Comment, "-- all".into())));
    assert!(found.contains(&(Kind::Number, "1".into())));
}

#[test]
fn html_tags_and_attributes() {
    let found = kinds("html", "<a href=\"/x\">it's \"text\"</a><!-- note -->");
    assert!(found.contains(&(Kind::Keyword, "a".into())));
    assert!(found.contains(&(Kind::Str, "\"/x\"".into())));
    assert!(found.contains(&(Kind::Comment, "<!-- note -->".into())));
    assert!(
        !found.iter().any(|(_, t)| t.contains("text")),
        "quotes in text stay plain"
    );
}

#[test]
fn css_properties_and_colours() {
    let found = kinds(
        "css",
        "a:hover {\n  background-color: #fff;\n  margin: 4px;\n}\n@media print {}",
    );
    assert!(found.contains(&(Kind::Keyword, "background-color".into())));
    assert!(found.contains(&(Kind::Number, "#fff".into())));
    assert!(found.contains(&(Kind::Number, "4px".into())));
    assert!(found.contains(&(Kind::Keyword, "@media".into())));
    assert!(
        !found.contains(&(Kind::Keyword, "a".into())),
        "a selector is not a property"
    );
}

#[test]
fn yaml_and_toml_keys() {
    let yaml = kinds("yaml", "name: app\n- port: 80 # web\n");
    assert!(yaml.contains(&(Kind::Keyword, "name".into())));
    assert!(yaml.contains(&(Kind::Keyword, "port".into())));
    assert!(yaml.contains(&(Kind::Comment, "# web".into())));
    let toml = kinds("toml", "[server]\nport = 80\n");
    assert!(toml.contains(&(Kind::Keyword, "[server]".into())));
    assert!(toml.contains(&(Kind::Keyword, "port".into())));
}

#[test]
fn lua_block_comments() {
    let found = kinds("lua", "--[[ a\nb ]] local x = 1 -- end");
    assert!(found.contains(&(Kind::Comment, "--[[ a\nb ]]".into())));
    assert!(found.contains(&(Kind::Keyword, "local".into())));
    assert!(found.contains(&(Kind::Comment, "-- end".into())));
}

#[test]
fn new_languages_rejoin_exactly() {
    for (lang, code) in [
        ("html", "<p class='x'>a < b</p>\n<br/>"),
        ("css", "p { color: red }"),
        ("ruby", "def f # c\n  'x'\nend"),
        ("php", "<?php echo $x; # c"),
        ("sql", "select 'unclosed"),
    ] {
        let joined: String = tokens(lang, code).into_iter().map(|(_, t)| t).collect();
        assert_eq!(joined, code, "{lang}");
    }
}
