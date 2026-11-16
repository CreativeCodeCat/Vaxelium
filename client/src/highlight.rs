//! Colours code in a code block by what each part is: keywords, strings, comments and numbers. It knows
//! the common language families. Any other language is shown as plain text.

/// What a part of the code is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Keyword,
    Str,
    Comment,
    Number,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Rust,
    Script,
    Python,
    CLike,
    Shell,
    Json,
    Plain,
}

fn family(lang: &str) -> Family {
    match lang.trim().to_ascii_lowercase().as_str() {
        "rust" | "rs" => Family::Rust,
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => Family::Script,
        "python" | "py" => Family::Python,
        "c" | "cpp" | "c++" | "h" | "java" | "go" | "golang" | "csharp" | "cs" | "kotlin" | "swift" => Family::CLike,
        "sh" | "bash" | "shell" | "zsh" => Family::Shell,
        "json" => Family::Json,
        _ => Family::Plain,
    }
}

fn keywords(family: Family) -> &'static [&'static str] {
    match family {
        Family::Rust => &[
            "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "pub", "use", "mod", "crate", "self",
            "Self", "super", "if", "else", "match", "for", "while", "loop", "return", "break", "continue", "in", "as",
            "where", "ref", "move", "async", "await", "dyn", "true", "false", "None", "Some", "Ok", "Err", "unsafe", "type",
        ],
        Family::Script => &[
            "function", "var", "let", "const", "if", "else", "for", "while", "do", "return", "class", "new", "this",
            "import", "export", "from", "default", "async", "await", "try", "catch", "finally", "throw", "true", "false",
            "null", "undefined", "typeof", "instanceof", "interface", "type", "extends", "implements", "switch", "case",
            "break", "continue", "yield",
        ],
        Family::Python => &[
            "def", "class", "import", "from", "as", "if", "elif", "else", "for", "while", "return", "in", "is", "not",
            "and", "or", "None", "True", "False", "lambda", "with", "try", "except", "finally", "raise", "pass", "yield",
            "async", "await", "break", "continue",
        ],
        Family::CLike => &[
            "int", "char", "void", "float", "double", "long", "short", "unsigned", "signed", "bool", "struct", "enum",
            "union", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "default",
            "typedef", "const", "static", "class", "public", "private", "protected", "new", "delete", "true", "false",
            "null", "nil", "func", "package", "import", "var", "type", "try", "catch", "throw", "throws",
        ],
        Family::Shell => &[
            "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac", "function", "export",
            "local", "return", "echo", "cd",
        ],
        Family::Json => &["true", "false", "null"],
        Family::Plain => &[],
    }
}

/// Splits code into coloured parts. The parts, joined, are the code exactly.
pub fn tokens(lang: &str, code: &str) -> Vec<(Kind, String)> {
    let family = family(lang);
    if family == Family::Plain {
        return vec![(Kind::Plain, code.to_string())];
    }
    let words = keywords(family);
    let chars: Vec<char> = code.chars().collect();
    let mut out: Vec<(Kind, String)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let rest = &chars[i..];

        // Line comments run to the end of the line.
        let line_comment = match family {
            Family::Python | Family::Shell => c == '#',
            Family::Json => false,
            _ => rest.starts_with(&['/', '/']),
        };
        if line_comment {
            let end = line_end(&chars, i);
            emit(&mut out, Kind::Comment, &chars[i..end]);
            i = end;
            continue;
        }
        // Block comments in the C-like families, up to their end (or the end of the code).
        if matches!(family, Family::Rust | Family::Script | Family::CLike) && rest.starts_with(&['/', '*']) {
            let end = find_after(&chars, i + 2, &['*', '/']).unwrap_or(chars.len());
            emit(&mut out, Kind::Comment, &chars[i..end]);
            i = end;
            continue;
        }
        // Strings end at their closing quote, or at the end of the line.
        let quote = c == '"' || c == '\'' || (c == '`' && family == Family::Script);
        if quote {
            let mut end = i + 1;
            while end < chars.len() && chars[end] != '\n' {
                if chars[end] == '\\' {
                    end += 2;
                    continue;
                }
                if chars[end] == c {
                    end += 1;
                    break;
                }
                end += 1;
            }
            let end = end.min(chars.len());
            emit(&mut out, Kind::Str, &chars[i..end]);
            i = end;
            continue;
        }
        // A number starts a token only when it does not continue a name, such as x1.
        let after_name = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        if c.is_ascii_digit() && !after_name {
            let mut end = i;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '.' || chars[end] == '_') {
                end += 1;
            }
            emit(&mut out, Kind::Number, &chars[i..end]);
            i = end;
            continue;
        }
        // Names are keywords when the language says so.
        if c.is_alphabetic() || c == '_' {
            let mut end = i;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let word: String = chars[i..end].iter().collect();
            let kind = if words.contains(&word.as_str()) { Kind::Keyword } else { Kind::Plain };
            emit(&mut out, kind, &chars[i..end]);
            i = end;
            continue;
        }
        emit(&mut out, Kind::Plain, &[c]);
        i += 1;
    }
    out
}

/// Adds text to the output, joining it to the last part when both are plain.
fn emit(out: &mut Vec<(Kind, String)>, kind: Kind, chars: &[char]) {
    let text: String = chars.iter().collect();
    if let Some((last_kind, last)) = out.last_mut() {
        if *last_kind == kind && kind == Kind::Plain {
            last.push_str(&text);
            return;
        }
    }
    out.push((kind, text));
}

/// The index just past the end of the line that `from` is on (the newline itself is not included).
fn line_end(chars: &[char], from: usize) -> usize {
    (from..chars.len()).find(|&j| chars[j] == '\n').unwrap_or(chars.len())
}

/// The index just past the first `pattern` at or after `from`.
fn find_after(chars: &[char], from: usize, pattern: &[char; 2]) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&j| chars[j] == pattern[0] && chars[j + 1] == pattern[1]).map(|j| j + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lang: &str, code: &str) -> Vec<(Kind, String)> {
        tokens(lang, code).into_iter().filter(|(k, _)| *k != Kind::Plain).collect()
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
        assert!(kinds("rust", "#[derive(Debug)]").iter().all(|(k, _)| *k != Kind::Comment));
    }

    #[test]
    fn a_name_with_digits_is_not_a_number() {
        assert!(kinds("rust", "x1 + 2").contains(&(Kind::Number, "2".into())));
        assert!(!kinds("rust", "x1 + 2").iter().any(|(k, t)| *k == Kind::Number && t == "1"));
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
}
