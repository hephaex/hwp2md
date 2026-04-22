use super::*;

fn check(input: &str, expected: &str) {
    let got = eqedit_to_latex(input);
    assert_eq!(
        got.trim(),
        expected.trim(),
        "input: {input:?}\n  got: {got:?}\n  exp: {expected:?}"
    );
}

// --- empty / passthrough ----------------------------------------------

#[test]
fn empty_input_returns_empty() {
    check("", "");
}

#[test]
fn whitespace_only_returns_empty() {
    check("   ", "");
}

#[test]
fn plain_text_passthrough() {
    check("x", "x");
}

#[test]
fn already_valid_latex_passthrough() {
    check("\\frac{1}{2}", "\\frac{1}{2}");
}

#[test]
fn superscript_passthrough() {
    check("x^{2}", "x^{2}");
}

#[test]
fn subscript_passthrough() {
    check("x_{i}", "x_{i}");
}

// --- over (fraction) --------------------------------------------------

#[test]
fn simple_fraction() {
    check("{a} over {b}", "\\frac{a}{b}");
}

#[test]
fn fraction_numeric() {
    check("{1} over {2}", "\\frac{1}{2}");
}

#[test]
fn fraction_with_extra_whitespace() {
    check("{p}  over  {q}", "\\frac{p}{q}");
}

#[test]
fn fraction_nested_groups() {
    check("{x+1} over {y-1}", "\\frac{x+1}{y-1}");
}

// --- sqrt / root ------------------------------------------------------

#[test]
fn sqrt_simple() {
    check("sqrt{x}", "\\sqrt{x}");
}

#[test]
fn sqrt_compound() {
    check("sqrt{x+1}", "\\sqrt{x+1}");
}

#[test]
fn root_nth() {
    check("root{n}{x}", "\\sqrt[n]{x}");
}

#[test]
fn root_numeric_index() {
    check("root{3}{8}", "\\sqrt[3]{8}");
}

// --- Greek letters ----------------------------------------------------

#[test]
fn greek_alpha_beta() {
    check("alpha + beta", "\\alpha + \\beta");
}

#[test]
fn greek_pi() {
    check("pi", "\\pi");
}

#[test]
fn greek_omega_uppercase() {
    check("Omega", "\\Omega");
}

#[test]
fn greek_in_expression() {
    check("2 pi r", "2 \\pi r");
}

// --- operators --------------------------------------------------------

#[test]
fn operator_times() {
    check("a times b", "a \\times b");
}

#[test]
fn operator_div() {
    check("a div b", "a \\div b");
}

#[test]
fn operator_pm() {
    check("x pm y", "x \\pm y");
}

#[test]
fn operator_le_ge() {
    check("a le b", "a \\le b");
    check("a ge b", "a \\ge b");
}

#[test]
fn operator_ne() {
    check("a ne b", "a \\ne b");
}

#[test]
fn operator_approx() {
    check("x approx y", "x \\approx y");
}

#[test]
fn operator_cdot() {
    check("a cdot b", "a \\cdot b");
}

// --- calculus / large operators ---------------------------------------

#[test]
fn sum_with_limits() {
    check("sum_{i=1}^{n}", "\\sum_{i=1}^{n}");
}

#[test]
fn integral_with_limits() {
    check("int_{a}^{b}", "\\int_{a}^{b}");
}

#[test]
fn product_operator() {
    check("prod_{k=1}^{n} k", "\\prod_{k=1}^{n} k");
}

#[test]
fn limit_operator() {
    check("lim_{x to 0}", "\\lim_{x \\to 0}");
}

#[test]
fn infinity() {
    check("inf", "\\infty");
}

// --- combined expressions ---------------------------------------------

#[test]
fn fraction_with_sqrt_denominator() {
    check("{1} over {sqrt{2}}", "\\frac{1}{\\sqrt{2}}");
}

#[test]
fn greek_in_fraction() {
    check("{alpha} over {beta}", "\\frac{\\alpha}{\\beta}");
}

#[test]
fn sum_fraction_combined() {
    check(
        "sum_{i=1}^{n} {x_{i}} over {n}",
        "\\sum_{i=1}^{n} \\frac{x_{i}}{n}",
    );
}

#[test]
fn nested_fractions() {
    let result = eqedit_to_latex("{{a} over {b}} over {c}");
    assert!(
        result.contains("\\frac"),
        "expected \\frac in result, got: {result}"
    );
}

// --- matrix -----------------------------------------------------------

#[test]
fn matrix_two_by_two() {
    check(
        "matrix{a & b # c & d}",
        "\\begin{matrix}a & b \\\\ c & d\\end{matrix}",
    );
}

#[test]
fn matrix_single_row() {
    check("matrix{1 & 2 & 3}", "\\begin{matrix}1 & 2 & 3\\end{matrix}");
}

#[test]
fn pile_two_rows() {
    check("pile{a # b}", "\\begin{matrix}a \\\\ b\\end{matrix}");
}

// --- delimiter forms --------------------------------------------------

#[test]
fn left_right_parens() {
    check("left( x right)", "\\left( x \\right)");
}

#[test]
fn left_right_braces() {
    check("left{ x right}", "\\left\\{ x \\right\\}");
}

// --- edge cases -------------------------------------------------------

#[test]
fn over_without_groups_is_passed_through() {
    let result = eqedit_to_latex("a over b");
    assert!(
        result.contains("over") || result.contains("a"),
        "bare 'a over b' (no group braces) must pass through as-is; got: {result:?}"
    );
    assert!(
        !result.contains("\\frac"),
        "bare 'a over b' must NOT produce \\frac (no brace groups); got: {result:?}"
    );
}

#[test]
fn deeply_nested_sqrt_and_fraction() {
    check(
        "{1} over {sqrt{alpha^{2} + beta^{2}}}",
        "\\frac{1}{\\sqrt{\\alpha^{2} + \\beta^{2}}}",
    );
}

#[test]
fn partial_derivative_expression() {
    check(
        "{partial f} over {partial x}",
        "\\frac{\\partial f}{\\partial x}",
    );
}

#[test]
fn deep_nesting_does_not_panic() {
    let open: String = "{a over ".repeat(50);
    let close: String = "}".repeat(50);
    let input = format!("{open}x{close}");
    let result = eqedit_to_latex(&input);
    assert!(!result.is_empty());
}

#[test]
fn unmatched_closing_brace_no_underflow() {
    let result = eqedit_to_latex("a}b}c");
    assert!(!result.is_empty());
}
