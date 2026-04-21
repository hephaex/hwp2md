/// EQEDIT-to-LaTeX converter for HWP 5.0 equation scripts.
///
/// HWP stores equations in a proprietary keyword-based scripting language
/// called EQEDIT.  This module converts the most common EQEDIT patterns to
/// valid LaTeX so that downstream renderers (MathJax, KaTeX, pandoc) can
/// display them correctly.
///
/// # Supported Patterns
///
/// - `{A} over {B}` → `\frac{A}{B}`
/// - `sqrt{X}` → `\sqrt{X}`
/// - `root {N} {X}` → `\sqrt[N]{X}`
/// - Greek letters (`alpha`, `beta`, …) → `\alpha`, `\beta`, …
/// - Operators (`times`, `div`, `pm`, `le`, `ge`, `ne`, `approx`, `cdot`,
///   `inf`, `sum`, `int`, `prod`, `lim`) → prefixed with `\`
/// - `left(` / `right)` and `left{` / `right}` delimiter forms
/// - `matrix{…}` with rows `#` and cells `&` →
///   `\begin{matrix}…\end{matrix}`
/// - `pile{…}` → `\begin{matrix}…\end{matrix}` (single-column aligned)
/// - Already-valid LaTeX constructs are passed through unchanged
const MAX_RECURSION_DEPTH: usize = 32;

pub(crate) fn eqedit_to_latex(script: &str) -> String {
    convert_with_depth(script, 0)
}

fn convert_with_depth(script: &str, depth: usize) -> String {
    let script = script.trim();
    if script.is_empty() {
        return String::new();
    }
    if depth >= MAX_RECURSION_DEPTH {
        return script.to_string();
    }

    let tokens = tokenise(script);
    let tokens = transform_over(tokens, depth);
    let tokens = transform_root(tokens, depth);
    let tokens = transform_matrix(tokens, depth);
    let tokens = expand_keywords(tokens, depth);
    let tokens = expand_left_right(tokens, depth);
    reassemble(&tokens)
}

// ---------------------------------------------------------------------------
// Token type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A `{…}` group (braces included).
    Group(String),
    /// Any other text fragment.
    Word(String),
}

impl Token {
    fn as_str(&self) -> &str {
        match self {
            Token::Group(s) | Token::Word(s) => s,
        }
    }
}

// ---------------------------------------------------------------------------
// Step 1 – Tokenise
// ---------------------------------------------------------------------------

/// Split `script` into flat `Token`s.
///
/// `{…}` groups are consumed with balanced-brace tracking.  Single-character
/// operator characters (`_`, `^`, `(`, `)`, …) are emitted as individual
/// `Word` tokens so that `sum_{i=1}^{n}` splits at `_` and `^`, allowing the
/// keyword `sum` to be matched.
fn tokenise(script: &str) -> Vec<Token> {
    let chars: Vec<char> = script.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            let (group, end) = consume_brace_group(&chars, i);
            tokens.push(Token::Group(group));
            i = end;
        } else if c.is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token::Word(" ".to_string()));
        } else if is_operator_char(c) {
            tokens.push(Token::Word(c.to_string()));
            i += 1;
        } else {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_whitespace() || ch == '{' || ch == '}' || is_operator_char(ch) {
                    break;
                }
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if !word.is_empty() {
                tokens.push(Token::Word(word));
            }
        }
    }

    tokens
}

fn is_operator_char(c: char) -> bool {
    matches!(
        c,
        '_' | '^'
            | '('
            | ')'
            | '['
            | ']'
            | '+'
            | '-'
            | '='
            | '/'
            | '|'
            | '&'
            | '#'
            | '!'
            | ','
            | '.'
    )
}

fn consume_brace_group(chars: &[char], start: usize) -> (String, usize) {
    debug_assert_eq!(chars[start], '{');
    let mut depth = 0usize;
    let mut i = start;
    let mut s = String::new();

    while i < chars.len() {
        let c = chars[i];
        s.push(c);
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                i += 1;
                return (s, i);
            }
        }
        i += 1;
    }
    (s, i)
}

// ---------------------------------------------------------------------------
// Step 2 – Transform `over`
// ---------------------------------------------------------------------------

fn transform_over(tokens: Vec<Token>, depth: usize) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Some(m) = try_match_over(&tokens, i, depth) {
            out.push(Token::Word(m.latex));
            i = m.next_idx;
        } else {
            out.push(tokens[i].clone());
            i += 1;
        }
    }
    out
}

struct OverMatch {
    latex: String,
    next_idx: usize,
}

fn try_match_over(tokens: &[Token], i: usize, depth: usize) -> Option<OverMatch> {
    let num_group = match tokens.get(i) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };

    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }
    if !matches!(tokens.get(j), Some(Token::Word(w)) if w == "over") {
        return None;
    }
    j += 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    let den_group = match tokens.get(j) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };
    j += 1;

    let num_latex = convert_with_depth(strip_braces(&num_group), depth + 1);
    let den_latex = convert_with_depth(strip_braces(&den_group), depth + 1);
    let latex = format!("\\frac{{{num_latex}}}{{{den_latex}}}");
    Some(OverMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 2b – Transform `root`
// ---------------------------------------------------------------------------

fn transform_root(tokens: Vec<Token>, depth: usize) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Some(m) = try_match_root(&tokens, i, depth) {
            out.push(Token::Word(m.latex));
            i = m.next_idx;
        } else {
            out.push(tokens[i].clone());
            i += 1;
        }
    }
    out
}

struct RootMatch {
    latex: String,
    next_idx: usize,
}

fn try_match_root(tokens: &[Token], i: usize, depth: usize) -> Option<RootMatch> {
    if !matches!(tokens.get(i), Some(Token::Word(w)) if w == "root") {
        return None;
    }
    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }
    let n_group = match tokens.get(j) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };
    j += 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }
    let x_group = match tokens.get(j) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };
    j += 1;

    let n_latex = convert_with_depth(strip_braces(&n_group), depth + 1);
    let x_latex = convert_with_depth(strip_braces(&x_group), depth + 1);
    let latex = format!("\\sqrt[{n_latex}]{{{x_latex}}}");
    Some(RootMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 2c – Transform `matrix` / `pile`
// ---------------------------------------------------------------------------

fn transform_matrix(tokens: Vec<Token>, depth: usize) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Some(m) = try_match_matrix(&tokens, i, depth) {
            out.push(Token::Word(m.latex));
            i = m.next_idx;
        } else {
            out.push(tokens[i].clone());
            i += 1;
        }
    }
    out
}

struct MatrixMatch {
    latex: String,
    next_idx: usize,
}

fn try_match_matrix(tokens: &[Token], i: usize, depth: usize) -> Option<MatrixMatch> {
    match tokens.get(i) {
        Some(Token::Word(w)) if w == "matrix" || w == "pile" => {}
        _ => return None,
    }
    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }
    let group = match tokens.get(j) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };
    j += 1;

    let body = strip_braces(&group);
    let rows: Vec<String> = body
        .split('#')
        .map(|row| convert_with_depth(row.trim(), depth + 1))
        .collect();
    let env_body = rows.join(" \\\\ ");
    let latex = format!("\\begin{{matrix}}{env_body}\\end{{matrix}}");
    Some(MatrixMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 3 – Expand keywords
// ---------------------------------------------------------------------------

fn expand_keywords(tokens: Vec<Token>, depth: usize) -> Vec<Token> {
    tokens
        .into_iter()
        .map(|tok| match tok {
            Token::Word(w) => Token::Word(map_keyword(&w)),
            Token::Group(g) => Token::Group(expand_group(&g, depth)),
        })
        .collect()
}

fn expand_group(group: &str, depth: usize) -> String {
    let inner = strip_braces(group);
    let converted = convert_with_depth(inner, depth + 1);
    format!("{{{converted}}}")
}

fn map_keyword(word: &str) -> String {
    match word {
        // Greek (lowercase)
        "alpha" => "\\alpha".into(),
        "beta" => "\\beta".into(),
        "gamma" => "\\gamma".into(),
        "delta" => "\\delta".into(),
        "epsilon" => "\\epsilon".into(),
        "varepsilon" => "\\varepsilon".into(),
        "zeta" => "\\zeta".into(),
        "eta" => "\\eta".into(),
        "theta" => "\\theta".into(),
        "vartheta" => "\\vartheta".into(),
        "iota" => "\\iota".into(),
        "kappa" => "\\kappa".into(),
        "lambda" => "\\lambda".into(),
        "mu" => "\\mu".into(),
        "nu" => "\\nu".into(),
        "xi" => "\\xi".into(),
        "pi" => "\\pi".into(),
        "varpi" => "\\varpi".into(),
        "rho" => "\\rho".into(),
        "varrho" => "\\varrho".into(),
        "sigma" => "\\sigma".into(),
        "varsigma" => "\\varsigma".into(),
        "tau" => "\\tau".into(),
        "upsilon" => "\\upsilon".into(),
        "phi" => "\\phi".into(),
        "varphi" => "\\varphi".into(),
        "chi" => "\\chi".into(),
        "psi" => "\\psi".into(),
        "omega" => "\\omega".into(),
        // Greek (uppercase)
        "Alpha" => "\\Alpha".into(),
        "Beta" => "\\Beta".into(),
        "Gamma" => "\\Gamma".into(),
        "Delta" => "\\Delta".into(),
        "Epsilon" => "\\Epsilon".into(),
        "Zeta" => "\\Zeta".into(),
        "Eta" => "\\Eta".into(),
        "Theta" => "\\Theta".into(),
        "Iota" => "\\Iota".into(),
        "Kappa" => "\\Kappa".into(),
        "Lambda" => "\\Lambda".into(),
        "Mu" => "\\Mu".into(),
        "Nu" => "\\Nu".into(),
        "Xi" => "\\Xi".into(),
        "Pi" => "\\Pi".into(),
        "Rho" => "\\Rho".into(),
        "Sigma" => "\\Sigma".into(),
        "Tau" => "\\Tau".into(),
        "Upsilon" => "\\Upsilon".into(),
        "Phi" => "\\Phi".into(),
        "Chi" => "\\Chi".into(),
        "Psi" => "\\Psi".into(),
        "Omega" => "\\Omega".into(),
        // Arithmetic / relational
        "times" => "\\times".into(),
        "div" => "\\div".into(),
        "pm" => "\\pm".into(),
        "mp" => "\\mp".into(),
        "le" | "leq" => "\\le".into(),
        "ge" | "geq" => "\\ge".into(),
        "ne" | "neq" => "\\ne".into(),
        "approx" => "\\approx".into(),
        "equiv" => "\\equiv".into(),
        "sim" => "\\sim".into(),
        "cdot" => "\\cdot".into(),
        "ldots" => "\\ldots".into(),
        "cdots" => "\\cdots".into(),
        "vdots" => "\\vdots".into(),
        "ddots" => "\\ddots".into(),
        // Set / logic
        "in" => "\\in".into(),
        "notin" => "\\notin".into(),
        "subset" => "\\subset".into(),
        "supset" => "\\supset".into(),
        "subseteq" => "\\subseteq".into(),
        "supseteq" => "\\supseteq".into(),
        "cup" => "\\cup".into(),
        "cap" => "\\cap".into(),
        "forall" => "\\forall".into(),
        "exists" => "\\exists".into(),
        // Calculus / large operators
        "sum" => "\\sum".into(),
        "prod" => "\\prod".into(),
        "int" => "\\int".into(),
        "oint" => "\\oint".into(),
        "lim" => "\\lim".into(),
        "inf" => "\\infty".into(),
        "infty" => "\\infty".into(),
        "partial" => "\\partial".into(),
        "nabla" => "\\nabla".into(),
        // Arrows
        "to" => "\\to".into(),
        "leftarrow" => "\\leftarrow".into(),
        "rightarrow" => "\\rightarrow".into(),
        "Leftarrow" => "\\Leftarrow".into(),
        "Rightarrow" => "\\Rightarrow".into(),
        "leftrightarrow" => "\\leftrightarrow".into(),
        "Leftrightarrow" => "\\Leftrightarrow".into(),
        // Miscellaneous
        "sqrt" => "\\sqrt".into(),
        "vec" => "\\vec".into(),
        "hat" => "\\hat".into(),
        "bar" => "\\bar".into(),
        "tilde" => "\\tilde".into(),
        "dot" => "\\dot".into(),
        "ddot" => "\\ddot".into(),
        other => other.into(),
    }
}

// ---------------------------------------------------------------------------
// Step 4 – Expand `left` / `right` delimiter sequences
// ---------------------------------------------------------------------------

/// Collapse `Word("left")` + delimiter and `Word("right")` + delimiter pairs
/// into `\left…` / `\right…` LaTeX forms.
///
/// After tokenisation:
/// - `left(` → `Word("left")` + `Word("(")`  → `\left(`
/// - `left[` → `Word("left")` + `Word("[")`  → `\left[`
/// - `left{ … }` → `Word("left")` + `Group("{…}")` → `\left\{…\right\}`
fn expand_left_right(tokens: Vec<Token>, depth: usize) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(w) if w == "left" => {
                let mut j = i + 1;
                while matches!(tokens.get(j), Some(Token::Word(ww)) if ww == " ") {
                    j += 1;
                }
                match tokens.get(j) {
                    Some(Token::Word(d)) if d == "(" => {
                        out.push(Token::Word("\\left(".into()));
                        i = j + 1;
                    }
                    Some(Token::Word(d)) if d == "[" => {
                        out.push(Token::Word("\\left[".into()));
                        i = j + 1;
                    }
                    Some(Token::Word(d)) if d == "|" => {
                        out.push(Token::Word("\\left|".into()));
                        i = j + 1;
                    }
                    Some(Token::Group(g)) => {
                        // In EQEDIT, `left{ content right}` forms a braced-delimiter
                        // pair.  The tokeniser consumed `{ content right}` as a group.
                        // Strip any trailing `right` keyword from the interior.
                        let interior = strip_braces(g);
                        let (content, _had_right) = strip_trailing_right(interior);
                        let converted = convert_with_depth(content.trim(), depth + 1);
                        out.push(Token::Word(format!("\\left\\{{ {converted} \\right\\}}")));
                        i = j + 1;
                    }
                    _ => {
                        out.push(tokens[i].clone());
                        i += 1;
                    }
                }
            }
            Token::Word(w) if w == "right" => {
                let mut j = i + 1;
                while matches!(tokens.get(j), Some(Token::Word(ww)) if ww == " ") {
                    j += 1;
                }
                match tokens.get(j) {
                    Some(Token::Word(d)) if d == ")" => {
                        out.push(Token::Word("\\right)".into()));
                        i = j + 1;
                    }
                    Some(Token::Word(d)) if d == "]" => {
                        out.push(Token::Word("\\right]".into()));
                        i = j + 1;
                    }
                    Some(Token::Word(d)) if d == "|" => {
                        out.push(Token::Word("\\right|".into()));
                        i = j + 1;
                    }
                    _ => {
                        out.push(tokens[i].clone());
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn strip_braces(s: &str) -> &str {
    s.strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(s)
}

/// Strip a trailing ` right` keyword (with optional leading whitespace) from
/// `s`, returning `(content, true)` if found, or `(s, false)` if not.
///
/// Used when processing the interior of a `left{…}` EQEDIT group where the
/// tokeniser consumed `right}` as part of the balanced brace group.
fn strip_trailing_right(s: &str) -> (&str, bool) {
    let trimmed = s.trim_end();
    if let Some(without) = trimmed.strip_suffix("right") {
        (without.trim_end(), true)
    } else {
        (s, false)
    }
}

fn reassemble(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.as_str()).collect::<String>()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert!(!result.is_empty());
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
}
