//! A small LaTeX reader for chat maths: superscripts, subscripts, fractions, roots, Greek letters and
//! the common operators. Anything it does not know is shown as its name, so it never fails.
//!
//! Inline maths is turned into Unicode text. Display maths is laid out as a tree (see the chat view).

/// One part of a formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// One symbol, or a word such as `sin` or a name from `\text{}`.
    Text(String),
    /// A braced group, which is laid out as its parts in a row.
    Group(Vec<Node>),
    Sup(Box<Node>),
    Sub(Box<Node>),
    Frac(Vec<Node>, Vec<Node>),
    Root(Vec<Node>),
}

/// The Unicode for a command such as `\alpha`, when there is one.
fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" | "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "phi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "pm" => "±",
        "mp" => "∓",
        "times" => "×",
        "cdot" => "⋅",
        "div" => "÷",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "sim" => "∼",
        "infty" => "∞",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "Rightarrow" | "implies" => "⇒",
        "Leftrightarrow" | "iff" => "⇔",
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "partial" => "∂",
        "nabla" => "∇",
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "cup" => "∪",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "emptyset" => "∅",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "angle" => "∠",
        "degree" => "°",
        "prime" => "′",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "quad" => "  ",
        "qquad" => "    ",
        "," | ";" | "!" | ":" => " ",
        "{" => "{",
        "}" => "}",
        "%" => "%",
        "&" => "&",
        "#" => "#",
        "_" => "_",
        "$" => "$",
        _ => return None,
    })
}

/// Parses a formula into nodes.
pub fn parse(src: &str) -> Vec<Node> {
    let chars: Vec<char> = src.chars().collect();
    let mut at = 0;
    let mut nodes = Vec::new();
    while at < chars.len() {
        if chars[at] == '}' {
            // A stray closing brace is shown as it is.
            nodes.push(Node::Text("}".into()));
            at += 1;
            continue;
        }
        parse_atom(&chars, &mut at, &mut nodes);
    }
    nodes
}

/// Reads one piece of the formula into `out`: a symbol, a command, a group, or a sup/subscript that
/// attaches to the piece before it.
fn parse_atom(chars: &[char], at: &mut usize, out: &mut Vec<Node>) {
    let c = chars[*at];
    match c {
        ' ' | '\t' | '\n' => {
            // One space stays between words, so inline maths reads as the source did.
            *at += 1;
            if !matches!(out.last(), Some(Node::Text(t)) if t == " ") {
                out.push(Node::Text(" ".into()));
            }
        }
        '{' => {
            *at += 1;
            let inner = parse_until_brace(chars, at);
            out.push(Node::Group(inner));
        }
        '^' | '_' => {
            *at += 1;
            let arg = argument(chars, at);
            let attached = out.pop().unwrap_or(Node::Text(String::new()));
            // A script with nothing before it applies to an empty base.
            let node = if c == '^' {
                Node::Sup(Box::new(arg))
            } else {
                Node::Sub(Box::new(arg))
            };
            out.push(attached);
            out.push(node);
        }
        '\\' => {
            *at += 1;
            command(chars, at, out);
        }
        _ => {
            *at += 1;
            out.push(Node::Text(c.to_string()));
        }
    }
}

/// The nodes up to the matching closing brace, which is consumed.
fn parse_until_brace(chars: &[char], at: &mut usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    while *at < chars.len() {
        if chars[*at] == '}' {
            *at += 1;
            return nodes;
        }
        parse_atom(chars, at, &mut nodes);
    }
    nodes
}

/// The argument of a script or a command: a braced group, or one atom.
fn argument(chars: &[char], at: &mut usize) -> Node {
    while *at < chars.len() && chars[*at] == ' ' {
        *at += 1;
    }
    if *at >= chars.len() {
        return Node::Group(Vec::new());
    }
    if chars[*at] == '{' {
        *at += 1;
        return Node::Group(parse_until_brace(chars, at));
    }
    let mut one = Vec::new();
    parse_atom(chars, at, &mut one);
    one.pop().unwrap_or(Node::Group(Vec::new()))
}

/// A command after its backslash.
fn command(chars: &[char], at: &mut usize, out: &mut Vec<Node>) {
    let start = *at;
    while *at < chars.len() && chars[*at].is_ascii_alphabetic() {
        *at += 1;
    }
    if *at == start && *at < chars.len() {
        // A single symbol command such as \, or \{.
        *at += 1;
    }
    let name: String = chars[start..*at].iter().collect();
    match name.as_str() {
        "frac" | "dfrac" | "tfrac" => {
            let top = argument(chars, at);
            let bottom = argument(chars, at);
            out.push(Node::Frac(group_of(top), group_of(bottom)));
        }
        "sqrt" => {
            // An optional index in brackets is skipped; only the square root is drawn.
            if *at < chars.len() && chars[*at] == '[' {
                while *at < chars.len() && chars[*at] != ']' {
                    *at += 1;
                }
                *at += 1;
            }
            let body = argument(chars, at);
            out.push(Node::Root(group_of(body)));
        }
        "text" | "mathrm" | "textrm" | "mathbf" | "mathit" | "operatorname" | "mbox" => {
            let body = argument(chars, at);
            out.push(Node::Group(group_of(body)));
        }
        // Sizing and spacing commands have no meaning in a chat message.
        "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "displaystyle" | "textstyle" => {}
        other => {
            if let Some(sym) = symbol(other) {
                out.push(Node::Text(sym.to_string()));
            } else if other.is_empty() {
                out.push(Node::Text("\\".into()));
            } else {
                // Functions such as \sin and \log are written as they are.
                out.push(Node::Text(other.to_string()));
            }
        }
    }
}

fn group_of(node: Node) -> Vec<Node> {
    match node {
        Node::Group(nodes) => nodes,
        other => vec![other],
    }
}

/// The formula as Unicode text, for inline maths.
pub fn inline_text(src: &str) -> String {
    let mut out = String::new();
    write_nodes(&parse(src), &mut out);
    out
}

fn write_nodes(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Group(inner) => write_nodes(inner, out),
            Node::Sup(arg) => {
                let inner = plain(arg);
                match superscript(&inner) {
                    Some(s) => out.push_str(&s),
                    None => {
                        out.push('^');
                        out.push('(');
                        out.push_str(&inner);
                        out.push(')');
                    }
                }
            }
            Node::Sub(arg) => {
                let inner = plain(arg);
                match subscript(&inner) {
                    Some(s) => out.push_str(&s),
                    None => {
                        out.push('_');
                        out.push('(');
                        out.push_str(&inner);
                        out.push(')');
                    }
                }
            }
            Node::Frac(top, bottom) => {
                out.push('(');
                write_nodes(top, out);
                out.push_str(")/(");
                write_nodes(bottom, out);
                out.push(')');
            }
            Node::Root(body) => {
                out.push('√');
                out.push('(');
                write_nodes(body, out);
                out.push(')');
            }
        }
    }
}

fn plain(node: &Node) -> String {
    let mut out = String::new();
    write_nodes(std::slice::from_ref(node), &mut out);
    out
}

/// Unicode superscripts for `text`, or None when a character has none.
fn superscript(text: &str) -> Option<String> {
    text.chars()
        .map(|c| {
            Some(match c {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                '+' => '⁺',
                '-' => '⁻',
                '=' => '⁼',
                '(' => '⁽',
                ')' => '⁾',
                'n' => 'ⁿ',
                'i' => 'ⁱ',
                _ => return None,
            })
        })
        .collect()
}

/// Unicode subscripts for `text`, or None when a character has none.
fn subscript(text: &str) -> Option<String> {
    text.chars()
        .map(|c| {
            Some(match c {
                '0' => '₀',
                '1' => '₁',
                '2' => '₂',
                '3' => '₃',
                '4' => '₄',
                '5' => '₅',
                '6' => '₆',
                '7' => '₇',
                '8' => '₈',
                '9' => '₉',
                '+' => '₊',
                '-' => '₋',
                '=' => '₌',
                '(' => '₍',
                ')' => '₎',
                'a' => 'ₐ',
                'e' => 'ₑ',
                'o' => 'ₒ',
                'x' => 'ₓ',
                'h' => 'ₕ',
                'k' => 'ₖ',
                'l' => 'ₗ',
                'm' => 'ₘ',
                'n' => 'ₙ',
                'p' => 'ₚ',
                's' => 'ₛ',
                't' => 'ₜ',
                _ => return None,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/latex.rs"]
mod tests;
