use super::*;

#[test]
fn inline_formulas_become_unicode() {
    assert_eq!(inline_text("E = mc^2"), "E = mc²");
    assert_eq!(inline_text("x_1 + x_{10}"), "x₁ + x₁₀");
    assert_eq!(inline_text("\\alpha + \\beta \\leq \\infty"), "α + β ≤ ∞");
    assert_eq!(inline_text("\\frac{a}{b}"), "(a)/(b)");
    assert_eq!(inline_text("\\sqrt{x}"), "√(x)");
}

#[test]
fn a_script_without_a_unicode_form_is_bracketed() {
    assert_eq!(inline_text("x^{q}"), "x^q");
    assert_eq!(inline_text("x^{qq}"), "x^(qq)");
}

#[test]
fn unknown_commands_show_their_name() {
    assert_eq!(inline_text("\\sin x"), "sin x");
    assert_eq!(inline_text("\\text{if } x"), "if  x");
}

#[test]
fn a_fraction_has_two_parts() {
    let nodes = parse("\\frac{1}{x+1}");
    assert!(matches!(&nodes[0], Node::Frac(top, bottom) if top.len() == 1 && bottom.len() == 3));
}

#[test]
fn braces_group_a_script() {
    let nodes = parse("a^{bc}");
    assert_eq!(nodes.len(), 2);
    assert!(matches!(&nodes[1], Node::Sup(arg) if matches!(&**arg, Node::Group(inner) if inner.len() == 2)));
}

#[test]
fn an_unclosed_group_does_not_panic() {
    let _ = parse("\\frac{1");
    let _ = parse("x^");
    let _ = parse("}");
}

#[test]
fn root_index() {
    assert_eq!(inline_text("\\sqrt[3]{x}"), "∛(x)");
    assert_eq!(inline_text("\\sqrt[n]{x}"), "ⁿ√(x)");
    assert!(matches!(&parse("\\sqrt[3]{x}")[0], Node::Root(Some(i), b) if i.len() == 1 && b.len() == 1));
}

#[test]
fn left_right_brackets() {
    assert_eq!(inline_text("\\left( \\frac{a}{b} \\right)"), "( (a)/(b) )");
    assert_eq!(inline_text("\\left\\{ x \\right."), "{ x ");
    assert!(matches!(&parse("\\left[x\\right]")[0], Node::Fenced(o, _, c) if o == "[" && c == "]"));
    // \rightarrow inside is an arrow, not the end of the brackets.
    assert_eq!(inline_text("\\left(a \\rightarrow b\\right)"), "(a → b)");
}

#[test]
fn matrices() {
    let nodes = parse("\\begin{pmatrix} a & b \\\\ c & d \\\\ \\end{pmatrix}");
    match &nodes[0] {
        Node::Matrix(open, rows, close) => {
            assert_eq!((open.as_str(), close.as_str()), ("(", ")"));
            assert_eq!(rows.len(), 2, "the trailing \\\\ adds no row");
            assert!(rows.iter().all(|r| r.len() == 2));
        }
        other => panic!("not a matrix: {other:?}"),
    }
    assert_eq!(
        inline_text("\\begin{bmatrix}1 & 2 \\\\ 3 & 4\\end{bmatrix}"),
        "[1 2; 3 4]"
    );
    assert_eq!(inline_text("\\begin{matrix}1 & 2\\end{matrix}"), "[1 2]");
    assert!(matches!(&parse("\\begin{cases} 1 \\end{cases}")[0], Node::Matrix(o, _, c) if o == "{" && c.is_empty()));
}

#[test]
fn more_unicode_scripts() {
    assert_eq!(inline_text("x^{ab}"), "xᵃᵇ");
    assert_eq!(inline_text("a_{ij}"), "aᵢⱼ");
    assert_eq!(inline_text("x_{q}"), "x_q");
}

#[test]
fn unfinished_new_forms_do_not_panic() {
    for src in [
        "\\left(",
        "\\sqrt[3",
        "\\begin{pmatrix} a &",
        "\\begin",
        "\\right)",
        "\\left",
    ] {
        let _ = inline_text(src);
    }
}

#[test]
fn letters_accents_and_sets() {
    assert_eq!(inline_text("x \\in \\mathbb{R}"), "x ∈ ℝ");
    assert_eq!(inline_text("A \\setminus B"), "A ∖ B");
    assert_eq!(inline_text("\\hat{H}"), "H\u{302}");
    assert_eq!(inline_text("\\mathbf{r}"), "r");
}

#[test]
fn chemistry_formulas() {
    assert_eq!(inline_text("\\ce{CH4 + 2O2 -> CO2 + 2H2O}"), "CH₄ + 2O₂ → CO₂ + 2H₂O");
    assert_eq!(inline_text("\\ce{SO4^{2-}}"), "SO₄²⁻");
}

#[test]
fn aligned_rows() {
    let nodes = parse("\\begin{aligned} a &= b \\\\ &= c \\end{aligned}");
    assert!(matches!(&nodes[0], Node::Aligned(rows) if rows.len() == 2 && rows[1].len() == 2));
    assert_eq!(
        inline_text("\\begin{aligned} a &= b \\\\ &= c \\end{aligned}"),
        "a= b; = c"
    );
}
