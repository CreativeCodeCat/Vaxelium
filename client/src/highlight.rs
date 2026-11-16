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
    Sql,
    Css,
    Markup,
    Yaml,
    Toml,
    Lua,
    Ruby,
    Php,
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
        "sql" | "mysql" | "postgres" | "postgresql" | "sqlite" => Family::Sql,
        "css" | "scss" | "less" => Family::Css,
        "html" | "htm" | "xml" | "svg" | "xhtml" => Family::Markup,
        "yaml" | "yml" => Family::Yaml,
        "toml" | "ini" => Family::Toml,
        "lua" => Family::Lua,
        "ruby" | "rb" => Family::Ruby,
        "php" => Family::Php,
        _ => Family::Plain,
    }
}

#[rustfmt::skip]
fn keywords(family: Family) -> &'static [&'static str] {
    match family {
        Family::Rust => &[
            "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "pub", "use", "mod", "crate",
            "self", "Self", "super", "if", "else", "match", "for", "while", "loop", "return", "break", "continue",
            "in", "as", "where", "ref", "move", "async", "await", "dyn", "true", "false", "None", "Some", "Ok", "Err",
            "unsafe", "type",
        ],
        Family::Script => &[
            "function", "var", "let", "const", "if", "else", "for", "while", "do", "return", "class", "new", "this",
            "import", "export", "from", "default", "async", "await", "try", "catch", "finally", "throw", "true",
            "false", "null", "undefined", "typeof", "instanceof", "interface", "type", "extends", "implements",
            "switch", "case", "break", "continue", "yield",
        ],
        Family::Python => &[
            "def", "class", "import", "from", "as", "if", "elif", "else", "for", "while", "return", "in", "is", "not",
            "and", "or", "None", "True", "False", "lambda", "with", "try", "except", "finally", "raise", "pass",
            "yield", "async", "await", "break", "continue",
        ],
        Family::CLike => &[
            "int", "char", "void", "float", "double", "long", "short", "unsigned", "signed", "bool", "struct", "enum",
            "union", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "default",
            "typedef", "const", "static", "class", "public", "private", "protected", "new", "delete", "true", "false",
            "null", "nil", "func", "package", "import", "var", "type", "try", "catch", "throw", "throws",
        ],
        Family::Shell => &[
            "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac", "function",
            "export", "local", "return", "echo", "cd",
        ],
        Family::Json | Family::Yaml | Family::Toml => &["true", "false", "null", "yes", "no", "on", "off"],
        // SQL keywords are matched without regard to case, so they are listed in lower case.
        Family::Sql => &[
            "select", "from", "where", "insert", "into", "values", "update", "set", "delete", "create", "table",
            "drop", "alter", "add", "index", "view", "join", "inner", "left", "right", "outer", "full", "on", "as",
            "and", "or", "not", "null", "is", "in", "like", "between", "group", "by", "order", "having", "limit",
            "offset", "distinct", "union", "all", "primary", "key", "foreign", "references", "default", "unique",
            "if", "exists", "case", "when", "then", "else", "end", "asc", "desc", "count", "sum", "avg", "min",
            "max", "integer", "int", "text", "varchar", "boolean", "true", "false", "begin", "commit", "rollback",
        ],
        Family::Css => &["important", "inherit", "initial", "none", "auto"],
        Family::Lua => &[
            "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local",
            "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
        ],
        Family::Ruby => &[
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until", "for", "in", "do",
            "return", "yield", "begin", "rescue", "ensure", "raise", "self", "nil", "true", "false", "and", "or",
            "not", "require", "attr_accessor", "puts", "case", "when", "then", "lambda", "proc",
        ],
        Family::Php => &[
            "function", "class", "public", "private", "protected", "static", "return", "if", "else", "elseif",
            "foreach", "for", "while", "as", "new", "echo", "true", "false", "null", "namespace", "use", "try",
            "catch", "throw", "array", "fn", "match", "const", "extends", "implements", "interface", "require",
            "include",
        ],
        Family::Markup | Family::Plain => &[],
    }
}

/// What starts a comment that runs to the end of the line.
fn line_comments(family: Family) -> &'static [&'static str] {
    match family {
        Family::Rust | Family::Script | Family::CLike => &["//"],
        Family::Php => &["//", "#"],
        Family::Python | Family::Shell | Family::Yaml | Family::Toml | Family::Ruby => &["#"],
        Family::Sql | Family::Lua => &["--"],
        Family::Json | Family::Css | Family::Markup | Family::Plain => &[],
    }
}

/// What opens and closes a comment that can span lines.
fn block_comment(family: Family) -> Option<(&'static str, &'static str)> {
    match family {
        Family::Rust | Family::Script | Family::CLike | Family::Css | Family::Sql | Family::Php => Some(("/*", "*/")),
        Family::Lua => Some(("--[[", "]]")),
        _ => None,
    }
}

/// Whether a name can have dashes in it, as CSS properties and YAML keys can.
fn dashed_names(family: Family) -> bool {
    matches!(family, Family::Css | Family::Yaml | Family::Toml)
}

/// Splits code into coloured parts. The parts, joined, are the code exactly.
pub fn tokens(lang: &str, code: &str) -> Vec<(Kind, String)> {
    let family = family(lang);
    match family {
        Family::Plain => vec![(Kind::Plain, code.to_string())],
        Family::Markup => markup(code),
        _ => code_tokens(family, code),
    }
}

fn code_tokens(family: Family, code: &str) -> Vec<(Kind, String)> {
    let words = keywords(family);
    let chars: Vec<char> = code.chars().collect();
    let mut out: Vec<(Kind, String)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        // Block comments, up to their end (or the end of the code). Checked first, as Lua's `--[[` also
        // starts like a line comment.
        if let Some((open, close)) = block_comment(family) {
            if starts_with(&chars, i, open) {
                let end = find_after(&chars, i + open.chars().count(), close).unwrap_or(chars.len());
                emit(&mut out, Kind::Comment, &chars[i..end]);
                i = end;
                continue;
            }
        }
        // Line comments run to the end of the line. A `#` in Rust is an attribute, so Rust has none.
        if line_comments(family).iter().any(|p| starts_with(&chars, i, p)) {
            let end = line_end(&chars, i);
            emit(&mut out, Kind::Comment, &chars[i..end]);
            i = end;
            continue;
        }
        // Strings end at their closing quote, or at the end of the line.
        let quote = c == '"' || c == '\'' || (c == '`' && matches!(family, Family::Script | Family::Sql));
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
        // CSS colours such as #fff, and at-rules such as @media.
        if family == Family::Css && c == '#' {
            let end = word_end(&chars, i + 1, true);
            let hex = &chars[i + 1..end];
            if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.iter().all(|h| h.is_ascii_hexdigit()) {
                emit(&mut out, Kind::Number, &chars[i..end]);
                i = end;
                continue;
            }
        }
        if family == Family::Css && c == '@' {
            let end = word_end(&chars, i + 1, true);
            emit(&mut out, Kind::Keyword, &chars[i..end]);
            i = end;
            continue;
        }
        // TOML section headers such as [server].
        if family == Family::Toml && c == '[' && first_on_line(&chars, i) {
            let end = line_end(&chars, i);
            emit(&mut out, Kind::Keyword, &chars[i..end]);
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
        // Names are keywords when the language says so, and keys in data files are coloured like them.
        if c.is_alphabetic() || c == '_' {
            let end = word_end(&chars, i, dashed_names(family));
            let word: String = chars[i..end].iter().collect();
            let keyword = if family == Family::Sql {
                words.contains(&word.to_ascii_lowercase().as_str())
            } else {
                words.contains(&word.as_str())
            };
            let kind = if keyword || is_key(family, &chars, i, end) {
                Kind::Keyword
            } else {
                Kind::Plain
            };
            emit(&mut out, kind, &chars[i..end]);
            i = end;
            continue;
        }
        emit(&mut out, Kind::Plain, &[c]);
        i += 1;
    }
    out
}

/// Whether the name at `start..end` is a key: a YAML key before `:`, a TOML key before `=`, or a CSS
/// property before `:`. Each must come first on its line (after a YAML list dash).
fn is_key(family: Family, chars: &[char], start: usize, end: usize) -> bool {
    let sep = match family {
        Family::Yaml | Family::Css => ':',
        Family::Toml => '=',
        _ => return false,
    };
    if !first_on_line(chars, start) {
        return false;
    }
    let mut j = end;
    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
        j += 1;
    }
    if j >= chars.len() || chars[j] != sep {
        return false;
    }
    // A CSS selector such as `a:hover {` is not a property.
    family != Family::Css || !chars[end..line_end(chars, end)].contains(&'{')
}

/// HTML and XML: tag names are keywords, attribute values are strings, and `<!-- -->` are comments.
/// Text between tags stays plain, even when it has quotes in it.
fn markup(code: &str) -> Vec<(Kind, String)> {
    let chars: Vec<char> = code.chars().collect();
    let mut out: Vec<(Kind, String)> = Vec::new();
    let mut i = 0;
    let mut in_tag = false;
    while i < chars.len() {
        let c = chars[i];
        if !in_tag {
            if starts_with(&chars, i, "<!--") {
                let end = find_after(&chars, i + 4, "-->").unwrap_or(chars.len());
                emit(&mut out, Kind::Comment, &chars[i..end]);
                i = end;
                continue;
            }
            if c == '<' {
                // The `<`, an optional `/`, `!` or `?`, then the tag name.
                let mut j = i + 1;
                if j < chars.len() && matches!(chars[j], '/' | '!' | '?') {
                    j += 1;
                }
                let end = word_end(&chars, j, true);
                if end > j {
                    emit(&mut out, Kind::Plain, &chars[i..j]);
                    emit(&mut out, Kind::Keyword, &chars[j..end]);
                    in_tag = true;
                    i = end;
                    continue;
                }
            }
            emit(&mut out, Kind::Plain, &[c]);
            i += 1;
            continue;
        }
        if c == '>' {
            in_tag = false;
        }
        if c == '"' || c == '\'' {
            let end = (i + 1..chars.len())
                .find(|&j| chars[j] == c)
                .map_or(chars.len(), |j| j + 1);
            emit(&mut out, Kind::Str, &chars[i..end]);
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

/// Whether `pattern` starts at `at`.
fn starts_with(chars: &[char], at: usize, pattern: &str) -> bool {
    let mut j = at;
    for p in pattern.chars() {
        if j >= chars.len() || chars[j] != p {
            return false;
        }
        j += 1;
    }
    true
}

/// The index just past a name that starts at `from`. Dashes count as part of it when `dashed` is set.
fn word_end(chars: &[char], from: usize, dashed: bool) -> usize {
    let mut end = from;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_' || (dashed && chars[end] == '-')) {
        end += 1;
    }
    end
}

/// Whether only spaces (and a YAML list dash) come before `at` on its line.
fn first_on_line(chars: &[char], at: usize) -> bool {
    let mut j = at;
    while j > 0 && chars[j - 1] != '\n' {
        j -= 1;
        if !matches!(chars[j], ' ' | '\t' | '-') {
            return false;
        }
    }
    true
}

/// The index just past the end of the line that `from` is on (the newline itself is not included).
fn line_end(chars: &[char], from: usize) -> usize {
    (from..chars.len()).find(|&j| chars[j] == '\n').unwrap_or(chars.len())
}

/// The index just past the first `pattern` at or after `from`.
fn find_after(chars: &[char], from: usize, pattern: &str) -> Option<usize> {
    let len = pattern.chars().count();
    (from..chars.len())
        .find(|&j| starts_with(chars, j, pattern))
        .map(|j| j + len)
}

#[cfg(test)]
#[path = "tests/highlight.rs"]
mod tests;
