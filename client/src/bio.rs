//! Formatting for bios on the profile card: **bold**, *italic*, bullet lists ("- " or "* "), numbered
//! lists ("1. " or "1) ") and web links. Links are found by `view::link_ranges`.

/// The style of one character of a bio.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// Turns the lists in a bio into plain text. Bullet lines become "•", and numbered items are numbered
/// in order from the first number, so "3." then "7." shows as "3." then "4.".
pub fn lists(body: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    // The number the next numbered item should show, when the line before was a numbered item.
    let mut next_number: Option<u32> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            out.push(format!("• {rest}"));
            next_number = None;
        } else if let Some((number, rest)) = numbered_item(trimmed) {
            let shown = next_number.unwrap_or(number);
            out.push(format!("{shown}. {rest}"));
            next_number = Some(shown + 1);
        } else {
            out.push(line.to_string());
            next_number = None;
        }
    }
    out.join("\n")
}

/// A line that starts with a number, then "." or ")", then a space: its number and the rest.
fn numbered_item(line: &str) -> Option<(u32, &str)> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 4 {
        return None;
    }
    let (number, rest) = line.split_at(digits);
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some((number.parse().ok()?, rest.trim_start()))
}

/// The bio's text with the bold and italic marks taken out, and the style of each character (in the
/// same order as the characters of the text). A mark with no partner is shown as it is.
pub fn styled(body: &str) -> (String, Vec<Style>) {
    let mut text = String::new();
    let mut styles = Vec::new();
    for (i, line) in body.split('\n').enumerate() {
        if i > 0 {
            text.push('\n');
            styles.push(Style::default());
        }
        styled_line(line, &mut text, &mut styles);
    }
    (text, styles)
}

fn styled_line(line: &str, text: &mut String, styles: &mut Vec<Style>) {
    let chars: Vec<char> = line.chars().collect();
    let (mut bold, mut italic, mut underline) = (false, false, false);
    let mut i = 0;
    while i < chars.len() {
        // _text_ is underlined. An underscore only opens or closes at a word edge, so names such as
        // snake_case_name keep their underscores.
        if chars[i] == '_' {
            if underline && underline_closes(&chars, i) {
                underline = false;
                i += 1;
                continue;
            }
            if !underline && underline_opens(&chars, i) {
                underline = true;
                i += 1;
                continue;
            }
        }
        // ***text*** is bold and italic together.
        if starts_with(&chars, i, "***") && ((bold && italic) || closes(&chars, i + 3, "***")) {
            bold = !bold;
            italic = !italic;
            i += 3;
            continue;
        }
        if starts_with(&chars, i, "**") && (bold || closes(&chars, i + 2, "**")) {
            bold = !bold;
            i += 2;
            continue;
        }
        if chars[i] == '*' && (italic || closes_single_star(&chars, i + 1)) {
            italic = !italic;
            i += 1;
            continue;
        }
        text.push(chars[i]);
        styles.push(Style {
            bold,
            italic,
            underline,
        });
        i += 1;
    }
}

/// An underscore that can close an underline: it follows a character that is not a space, and no
/// letter or digit comes after it.
fn underline_closes(chars: &[char], at: usize) -> bool {
    at > 0 && !chars[at - 1].is_whitespace() && chars.get(at + 1).is_none_or(|c| !c.is_alphanumeric())
}

/// An underscore that can open an underline: no letter or digit comes before it, a non-space character
/// follows, and an underline closes somewhere later on the line.
fn underline_opens(chars: &[char], at: usize) -> bool {
    let word_edge = at == 0 || !chars[at - 1].is_alphanumeric();
    word_edge
        && chars.get(at + 1).is_some_and(|c| !c.is_whitespace())
        && (at + 1..chars.len()).any(|j| chars[j] == '_' && underline_closes(chars, j))
}

fn starts_with(chars: &[char], at: usize, pattern: &str) -> bool {
    pattern.chars().enumerate().all(|(k, c)| chars.get(at + k) == Some(&c))
}

/// True when `pattern` appears at or after `from`.
fn closes(chars: &[char], from: usize, pattern: &str) -> bool {
    (from..chars.len()).any(|j| starts_with(chars, j, pattern))
}

/// True when a single `*` (not part of `**`) appears at or after `from`.
fn closes_single_star(chars: &[char], from: usize) -> bool {
    (from..chars.len())
        .any(|j| chars[j] == '*' && !(j + 1 < chars.len() && chars[j + 1] == '*') && !(j > 0 && chars[j - 1] == '*'))
}

#[cfg(test)]
#[path = "tests/bio.rs"]
mod tests;
