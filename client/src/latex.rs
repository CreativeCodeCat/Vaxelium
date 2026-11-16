//! A small LaTeX reader for chat maths: superscripts, subscripts, fractions, roots, brackets that grow
//! (`\left`, `\right`), matrices, Greek letters and the common operators. Anything it does not know is
//! shown as its name, so it never fails.
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
    /// A root, with its index (the 3 of a cube root) when it has one.
    Root(Option<Vec<Node>>, Vec<Node>),
    /// `\left( … \right)`: brackets as tall as what they hold. An empty bracket is drawn as nothing.
    Fenced(String, Vec<Node>, String),
    /// A matrix or `cases`: rows of cells, between its brackets.
    Matrix(String, Vec<Vec<Vec<Node>>>, String),
    /// `aligned` and its kin: rows lined up at their `&`, such as the `=` of each line.
    Aligned(Vec<Vec<Vec<Node>>>),
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
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" | "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
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
        "setminus" => "∖",
        "supset" => "⊃",
        "supseteq" => "⊇",
        "bigcup" => "⋃",
        "bigcap" => "⋂",
        "oint" => "∮",
        "iint" => "∬",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        "propto" => "∝",
        "perp" => "⊥",
        "parallel" => "∥",
        "circ" => "∘",
        "ast" => "∗",
        "star" => "⋆",
        "bullet" => "∙",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "leftrightarrow" => "↔",
        "Leftarrow" => "⇐",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "ll" => "≪",
        "gg" => "≫",
        "cong" => "≅",
        "simeq" => "≃",
        "therefore" => "∴",
        "because" => "∵",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "emptyset" => "∅",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "angle" => "∠",
        "degree" => "°",
        "prime" => "′",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "vert" | "mid" => "|",
        "Vert" | "|" => "‖",
        "quad" => "  ",
        "qquad" => "    ",
        "," | ";" | "!" | ":" => " ",
        "{" | "lbrace" => "{",
        "}" | "rbrace" => "}",
        "%" => "%",
        "&" => "&",
        "#" => "#",
        "_" => "_",
        "$" => "$",
        _ => return None,
    })
}

/// What ended a run of nodes.
enum End {
    /// The end of the formula.
    Eof,
    /// A closing brace, or the `]` of a root's index.
    Close,
    /// `\right` and its bracket.
    Right(String),
    /// `&`, the next cell of a matrix.
    Cell,
    /// `\\`, the next row of a matrix.
    Row,
    /// `\end{…}`.
    EndEnv,
}

/// What may end the run being read.
#[derive(Clone, Copy, Default)]
struct Stops {
    brace: bool,
    bracket: bool,
    right: bool,
    matrix: bool,
}

/// Parses a formula into nodes.
pub fn parse(src: &str) -> Vec<Node> {
    let chars: Vec<char> = src.chars().collect();
    let mut at = 0;
    run(&chars, &mut at, Stops::default()).0
}

/// Reads nodes until one of `stops` (which is consumed) or the end of the formula.
fn run(chars: &[char], at: &mut usize, stops: Stops) -> (Vec<Node>, End) {
    let mut nodes = Vec::new();
    while *at < chars.len() {
        let c = chars[*at];
        if c == '}' {
            *at += 1;
            if stops.brace {
                return (nodes, End::Close);
            }
            // A stray closing brace is shown as it is.
            nodes.push(Node::Text("}".into()));
            continue;
        }
        if c == ']' && stops.bracket {
            *at += 1;
            return (nodes, End::Close);
        }
        if stops.matrix && c == '&' {
            *at += 1;
            return (nodes, End::Cell);
        }
        if stops.matrix && is_at(chars, *at, "\\\\") {
            *at += 2;
            return (nodes, End::Row);
        }
        if stops.right && is_command(chars, *at, "right") {
            *at += "\\right".len();
            return (nodes, End::Right(delimiter(chars, at)));
        }
        if stops.matrix && is_command(chars, *at, "end") {
            *at += "\\end".len();
            let _ = braced_name(chars, at);
            return (nodes, End::EndEnv);
        }
        atom(chars, at, &mut nodes);
    }
    (nodes, End::Eof)
}

/// Whether `text` starts at `at`.
fn is_at(chars: &[char], at: usize, text: &str) -> bool {
    text.chars().enumerate().all(|(i, t)| chars.get(at + i) == Some(&t))
}

/// Whether the command `\name` (and not a longer one, such as `\rightarrow` for `right`) starts at `at`.
fn is_command(chars: &[char], at: usize, name: &str) -> bool {
    let after = at + 1 + name.chars().count();
    chars.get(at) == Some(&'\\')
        && is_at(chars, at + 1, name)
        && !chars.get(after).is_some_and(|c| c.is_ascii_alphabetic())
}

/// Reads one piece of the formula into `out`: a symbol, a command, a group, or a sup/subscript that
/// attaches to the piece before it.
fn atom(chars: &[char], at: &mut usize, out: &mut Vec<Node>) {
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
            out.push(Node::Group(
                run(
                    chars,
                    at,
                    Stops {
                        brace: true,
                        ..Stops::default()
                    },
                )
                .0,
            ));
        }
        '^' | '_' => {
            *at += 1;
            let arg = argument(chars, at);
            // A script with nothing before it applies to an empty base.
            if out.is_empty() {
                out.push(Node::Text(String::new()));
            }
            out.push(if c == '^' {
                Node::Sup(Box::new(arg))
            } else {
                Node::Sub(Box::new(arg))
            });
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

/// The argument of a script or a command: a braced group, or one atom.
fn argument(chars: &[char], at: &mut usize) -> Node {
    skip_spaces(chars, at);
    if *at >= chars.len() {
        return Node::Group(Vec::new());
    }
    if chars[*at] == '{' {
        *at += 1;
        return Node::Group(
            run(
                chars,
                at,
                Stops {
                    brace: true,
                    ..Stops::default()
                },
            )
            .0,
        );
    }
    let mut one = Vec::new();
    atom(chars, at, &mut one);
    one.pop().unwrap_or(Node::Group(Vec::new()))
}

fn skip_spaces(chars: &[char], at: &mut usize) {
    while *at < chars.len() && chars[*at] == ' ' {
        *at += 1;
    }
}

/// The name in `{…}`, such as the `pmatrix` of `\begin{pmatrix}`. Empty when there are no braces.
fn braced_name(chars: &[char], at: &mut usize) -> String {
    skip_spaces(chars, at);
    if chars.get(*at) != Some(&'{') {
        return String::new();
    }
    let start = *at + 1;
    let end = (start..chars.len()).find(|&j| chars[j] == '}').unwrap_or(chars.len());
    *at = (end + 1).min(chars.len());
    chars[start..end].iter().collect()
}

/// The raw text in `{…}` at `at`, up to the brace that matches the first. Empty when there are no braces.
fn balanced(chars: &[char], at: &mut usize) -> String {
    if chars.get(*at) != Some(&'{') {
        return String::new();
    }
    let start = *at + 1;
    let mut depth = 0;
    let mut end = chars.len();
    for (j, &c) in chars.iter().enumerate().skip(*at) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = j;
                    break;
                }
            }
            _ => {}
        }
    }
    *at = (end + 1).min(chars.len());
    chars[start..end].iter().collect()
}

/// The bracket after `\left` or `\right`: a character, or a command such as `\{` or `\langle`. A `.`
/// means no bracket.
fn delimiter(chars: &[char], at: &mut usize) -> String {
    skip_spaces(chars, at);
    let Some(&c) = chars.get(*at) else { return String::new() };
    *at += 1;
    if c == '.' {
        return String::new();
    }
    if c != '\\' {
        return c.to_string();
    }
    let name = command_name(chars, at);
    symbol(&name).map(str::to_string).unwrap_or(name)
}

/// The name of a command after its backslash: letters, or one other character such as `,` or `{`.
fn command_name(chars: &[char], at: &mut usize) -> String {
    let start = *at;
    while *at < chars.len() && chars[*at].is_ascii_alphabetic() {
        *at += 1;
    }
    if *at == start && *at < chars.len() {
        *at += 1;
    }
    chars[start..*at].iter().collect()
}

/// A command after its backslash.
fn command(chars: &[char], at: &mut usize, out: &mut Vec<Node>) {
    let name = command_name(chars, at);
    match name.as_str() {
        "frac" | "dfrac" | "tfrac" => {
            let top = argument(chars, at);
            let bottom = argument(chars, at);
            out.push(Node::Frac(group_of(top), group_of(bottom)));
        }
        "sqrt" => {
            let index = if chars.get(*at) == Some(&'[') {
                *at += 1;
                Some(
                    run(
                        chars,
                        at,
                        Stops {
                            bracket: true,
                            ..Stops::default()
                        },
                    )
                    .0,
                )
            } else {
                None
            };
            let body = argument(chars, at);
            out.push(Node::Root(index, group_of(body)));
        }
        "left" => {
            let open = delimiter(chars, at);
            let (body, end) = run(
                chars,
                at,
                Stops {
                    right: true,
                    ..Stops::default()
                },
            );
            let close = match end {
                End::Right(close) => close,
                _ => String::new(),
            };
            out.push(Node::Fenced(open, body, close));
        }
        "begin" => out.push(environment(chars, at)),
        "mathbb" => {
            let body = plain(&argument(chars, at));
            out.push(Node::Text(body.chars().map(double_struck).collect()));
        }
        "hat" | "widehat" | "bar" | "overline" | "vec" | "dot" | "ddot" | "tilde" | "widetilde" => {
            // An accent is a combining mark after each letter it covers.
            let mark = match name.as_str() {
                "hat" | "widehat" => '\u{302}',
                "bar" | "overline" => '\u{305}',
                "vec" => '\u{20D7}',
                "dot" => '\u{307}',
                "ddot" => '\u{308}',
                _ => '\u{303}',
            };
            let body = plain(&argument(chars, at));
            out.push(Node::Text(body.chars().flat_map(|c| [c, mark]).collect()));
        }
        "ce" => {
            skip_spaces(chars, at);
            let raw = balanced(chars, at);
            out.push(Node::Text(chemistry(&raw)));
        }
        "text" | "mathrm" | "textrm" | "mathbf" | "mathit" | "mathsf" | "mathtt" | "mathcal" | "mathfrak"
        | "boldsymbol" | "operatorname" | "mbox" => {
            let body = argument(chars, at);
            out.push(Node::Group(group_of(body)));
        }
        // Sizing and spacing commands have no meaning in a chat message, and a `\right` without its
        // `\left` has nothing to close.
        "right" | "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" | "displaystyle"
        | "textstyle" => {}
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

/// `\begin{name} … \end{name}`, read as rows of cells. Matrices take the brackets their name gives.
fn environment(chars: &[char], at: &mut usize) -> Node {
    let name = braced_name(chars, at);
    if name == "array" {
        // The column layout, such as {cc}, is not needed.
        let _ = braced_name(chars, at);
    }
    let aligned = matches!(
        name.trim_end_matches('*'),
        "aligned" | "align" | "alignat" | "split" | "eqnarray" | "gathered" | "gather"
    );
    let (open, close) = match name.as_str() {
        "pmatrix" => ("(", ")"),
        "bmatrix" => ("[", "]"),
        "Bmatrix" => ("{", "}"),
        "cases" => ("{", ""),
        "vmatrix" => ("|", "|"),
        "Vmatrix" => ("‖", "‖"),
        _ => ("", ""),
    };
    let mut rows: Vec<Vec<Vec<Node>>> = Vec::new();
    let mut cells: Vec<Vec<Node>> = Vec::new();
    loop {
        let (cell, end) = run(
            chars,
            at,
            Stops {
                matrix: true,
                ..Stops::default()
            },
        );
        cells.push(trim(cell));
        match end {
            End::Cell => {}
            End::Row => rows.push(std::mem::take(&mut cells)),
            _ => {
                rows.push(cells);
                break;
            }
        }
    }
    // A `\\` after the last row leaves an empty row behind.
    if rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(Vec::is_empty)) {
        rows.pop();
    }
    if aligned {
        Node::Aligned(rows)
    } else {
        Node::Matrix(open.into(), rows, close.into())
    }
}

/// The double-struck letter for `\mathbb`, such as ℝ for R.
fn double_struck(c: char) -> char {
    match c {
        'C' => 'ℂ',
        'H' => 'ℍ',
        'N' => 'ℕ',
        'P' => 'ℙ',
        'Q' => 'ℚ',
        'R' => 'ℝ',
        'Z' => 'ℤ',
        // The other double-struck letters are missing from most fonts, so they stay plain.
        _ => c,
    }
}

/// A chemical formula from `\ce{}`, such as `2H2O -> ...`: counts after an element are lowered,
/// charges after `^` are raised, and arrows are drawn as arrows.
fn chemistry(raw: &str) -> String {
    let raw = raw.replace("<=>", "⇌").replace("->", "→").replace("<-", "←");
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let after_element = i > 0 && (chars[i - 1].is_alphabetic() || chars[i - 1] == ')' || chars[i - 1] == ']');
        if c.is_ascii_digit() && after_element {
            while i < chars.len() && chars[i].is_ascii_digit() {
                out.push_str(&subscript(&chars[i].to_string()).unwrap_or_default());
                i += 1;
            }
            continue;
        }
        if c == '^' {
            i += 1;
            let charge: String = if chars.get(i) == Some(&'{') {
                let end = (i..chars.len()).find(|&j| chars[j] == '}').unwrap_or(chars.len());
                let inner = chars[i + 1..end].iter().collect();
                i = (end + 1).min(chars.len());
                inner
            } else {
                let end = (i..chars.len())
                    .find(|&j| !matches!(chars[j], '0'..='9' | '+' | '-'))
                    .unwrap_or(chars.len());
                let inner = chars[i..end].iter().collect();
                i = end;
                inner
            };
            out.push_str(&superscript(&charge).unwrap_or(format!("^{charge}")));
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// A cell without the spaces around it.
fn trim(mut nodes: Vec<Node>) -> Vec<Node> {
    let space = |n: &Node| matches!(n, Node::Text(t) if t == " ");
    while nodes.last().is_some_and(space) {
        nodes.pop();
    }
    let lead = nodes.iter().take_while(|n| space(n)).count();
    nodes.drain(..lead);
    nodes
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
            Node::Sup(arg) => write_script(&plain(arg), '^', superscript, out),
            Node::Sub(arg) => write_script(&plain(arg), '_', subscript, out),
            Node::Frac(top, bottom) => {
                out.push('(');
                write_nodes(top, out);
                out.push_str(")/(");
                write_nodes(bottom, out);
                out.push(')');
            }
            Node::Root(index, body) => {
                let index = index.as_deref().map(plain_nodes).unwrap_or_default();
                match index.as_str() {
                    "" | "2" => out.push('√'),
                    "3" => out.push('∛'),
                    "4" => out.push('∜'),
                    other => {
                        write_script(other, '^', superscript, out);
                        out.push('√');
                    }
                }
                out.push('(');
                write_nodes(body, out);
                out.push(')');
            }
            Node::Fenced(open, body, close) => {
                out.push_str(open);
                write_nodes(body, out);
                out.push_str(close);
            }
            Node::Aligned(rows) => {
                let rows: Vec<String> = rows
                    .iter()
                    .map(|cells| cells.iter().map(|c| plain_nodes(c)).collect())
                    .collect();
                out.push_str(&rows.join("; "));
            }
            Node::Matrix(open, rows, close) => {
                // Rows are separated by `;` and cells by spaces, as in [a b; c d].
                let rows: Vec<String> = rows
                    .iter()
                    .map(|cells| cells.iter().map(|c| plain_nodes(c)).collect::<Vec<_>>().join(" "))
                    .collect();
                let (open, close) = if open.is_empty() {
                    ("[", "]")
                } else {
                    (open.as_str(), close.as_str())
                };
                out.push_str(open);
                out.push_str(&rows.join("; "));
                out.push_str(close);
            }
        }
    }
}

/// A script in Unicode when every character has a raised or lowered form. Otherwise it is written with
/// `^` or `_`, and bracketed when it is longer than one character.
fn write_script(inner: &str, mark: char, convert: fn(&str) -> Option<String>, out: &mut String) {
    if let Some(s) = convert(inner) {
        out.push_str(&s);
        return;
    }
    out.push(mark);
    if inner.chars().count() == 1 {
        out.push_str(inner);
    } else {
        out.push('(');
        out.push_str(inner);
        out.push(')');
    }
}

fn plain(node: &Node) -> String {
    plain_nodes(std::slice::from_ref(node))
}

fn plain_nodes(nodes: &[Node]) -> String {
    let mut out = String::new();
    write_nodes(nodes, &mut out);
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
                '-' | '−' => '⁻',
                '=' => '⁼',
                '(' => '⁽',
                ')' => '⁾',
                'a' => 'ᵃ',
                'b' => 'ᵇ',
                'c' => 'ᶜ',
                'd' => 'ᵈ',
                'e' => 'ᵉ',
                'f' => 'ᶠ',
                'g' => 'ᵍ',
                'h' => 'ʰ',
                'i' => 'ⁱ',
                'j' => 'ʲ',
                'k' => 'ᵏ',
                'l' => 'ˡ',
                'm' => 'ᵐ',
                'n' => 'ⁿ',
                'o' => 'ᵒ',
                'p' => 'ᵖ',
                'r' => 'ʳ',
                's' => 'ˢ',
                't' => 'ᵗ',
                'u' => 'ᵘ',
                'v' => 'ᵛ',
                'w' => 'ʷ',
                'x' => 'ˣ',
                'y' => 'ʸ',
                'z' => 'ᶻ',
                'A' => 'ᴬ',
                'B' => 'ᴮ',
                'D' => 'ᴰ',
                'E' => 'ᴱ',
                'G' => 'ᴳ',
                'H' => 'ᴴ',
                'I' => 'ᴵ',
                'J' => 'ᴶ',
                'K' => 'ᴷ',
                'L' => 'ᴸ',
                'M' => 'ᴹ',
                'N' => 'ᴺ',
                'O' => 'ᴼ',
                'P' => 'ᴾ',
                'R' => 'ᴿ',
                'T' => 'ᵀ',
                'U' => 'ᵁ',
                'V' => 'ⱽ',
                'W' => 'ᵂ',
                'α' => 'ᵅ',
                'β' => 'ᵝ',
                'γ' => 'ᵞ',
                'δ' => 'ᵟ',
                'ε' => 'ᵋ',
                'θ' => 'ᶿ',
                'φ' => 'ᵠ',
                'χ' => 'ᵡ',
                '′' => '′',
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
                '-' | '−' => '₋',
                '=' => '₌',
                '(' => '₍',
                ')' => '₎',
                'a' => 'ₐ',
                'e' => 'ₑ',
                'h' => 'ₕ',
                'i' => 'ᵢ',
                'j' => 'ⱼ',
                'k' => 'ₖ',
                'l' => 'ₗ',
                'm' => 'ₘ',
                'n' => 'ₙ',
                'o' => 'ₒ',
                'p' => 'ₚ',
                'r' => 'ᵣ',
                's' => 'ₛ',
                't' => 'ₜ',
                'u' => 'ᵤ',
                'v' => 'ᵥ',
                'x' => 'ₓ',
                'β' => 'ᵦ',
                'γ' => 'ᵧ',
                'ρ' => 'ᵨ',
                'φ' => 'ᵩ',
                'χ' => 'ᵪ',
                _ => return None,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/latex.rs"]
mod tests;
