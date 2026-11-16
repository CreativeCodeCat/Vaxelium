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
    assert_eq!(inline_text("x^{q}"), "x^(q)");
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
