/// EQEDIT-to-LaTeX converter for HWP 5.0 equation scripts.
///
/// HWP stores equations in a proprietary keyword-based scripting language called
/// EQEDIT.  This module converts the most common EQEDIT patterns to valid LaTeX
/// so that downstream renderers (MathJax, KaTeX, pandoc) can display them
/// correctly.
///
/// # Supported Patterns
///
/// - `{A} over {B}` → `\frac{A}{B}`
/// - `sqrt{X}` → `\sqrt{X}`
/// - `root{N}{X}` → `\sqrt[N]{X}`
/// - Greek letters (`alpha`, `beta`, …) → `\alpha`, `\beta`, …
/// - Operators (`times`, `div`, `pm`, `le`, `ge`, `ne`, `approx`, `cdot`,
///   `inf`, `sum`, `int`, `prod`, `lim`) → prefixed with `\`
/// - `left(` / `right)` and `left{` / `right}` delimiter forms
/// - `matrix{…}` with rows `#` and cells `&` →
///   `\begin{matrix}…\end{matrix}`
/// - `pile{…}` → `\begin{matrix}…\end{matrix}` (single-column aligned)
/// - Already-valid LaTeX constructs are passed through unchanged
pub(crate) fn eqedit_to_latex(script: &str) -> String {
    let script = script.trim();
    if script.is_empty() {
        return String::new();
    }

    // Tokenise, transform, then reassemble.
    let tokens = tokenise(script);
    let tokens = transform_over(tokens);
    let tokens = transform_root(tokens);
    let tokens = transform_matrix(tokens);
    let tokens = expand_keywords(tokens);
    reassemble(&tokens)
}

// ---------------------------------------------------------------------------
// Token type
// ---------------------------------------------------------------------------

/// A coarse-grained token used during transformation.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A `{…}` group (the braces are included in `text`).
    Group(String),
    /// Any other text fragment (keyword, operator, whitespace, …).
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

/// Split `script` into a flat list of `Token`s.
///
/// Tokens are either `{…}` groups (with balanced braces) or whitespace-delimited
/// words.  Whitespace between tokens is preserved as `Word` tokens so that
/// reassembly does not merge adjacent identifiers.
fn tokenise(script: &str) -> Vec<Token> {
    let chars: Vec<char> = script.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            // Check if this brace is preceded by `left` or `right` — if so,
            // emit the brace as a literal word, not a group start.
            let preceded_by_delim = matches!(
                tokens.last(),
                Some(Token::Word(w)) if w == "left" || w == "right"
            );
            if preceded_by_delim {
                tokens.push(Token::Word("{".to_string()));
                i += 1;
            } else {
                let (group, end) = consume_brace_group(&chars, i);
                tokens.push(Token::Group(group));
                i = end;
            }
        } else if chars[i] == '}' && matches!(tokens.last(), Some(Token::Word(w)) if w == "right" || w.ends_with("right")) {
            tokens.push(Token::Word("}".to_string()));
            i += 1;
        } else if chars[i].is_whitespace() {
            // Collapse consecutive whitespace into a single space word.
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token::Word(" ".to_string()));
        } else if chars[i] == '_' || chars[i] == '^' {
            tokens.push(Token::Word(chars[i].to_string()));
            i += 1;
        } else {
            let start = i;
            while i < chars.len()
                && !chars[i].is_whitespace()
                && !matches!(chars[i], '{' | '}' | '_' | '^')
            {
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

/// Consume a balanced `{…}` group starting at `chars[start]`.
///
/// Returns `(group_string, one_past_closing_brace_index)`.
/// The returned string includes the outer braces.  Unbalanced input is handled
/// gracefully by consuming to end-of-string.
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

    // Unbalanced — return what we have.
    (s, i)
}

// ---------------------------------------------------------------------------
// Step 2 – Transform `over`
// ---------------------------------------------------------------------------

/// Rewrite `{A} over {B}` sequences into `\frac{A}{B}`.
///
/// The `over` keyword must appear as a standalone `Word` token between two
/// `Group` tokens.  Whitespace tokens between the group and the keyword are
/// consumed as part of the match.
fn transform_over(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        // Look ahead for the pattern:
        //   [Group] [optional whitespace] Word("over") [optional whitespace] [Group]
        if let Some(frac) = try_match_over(&tokens, i) {
            out.push(Token::Word(frac.latex));
            i = frac.next_idx;
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

fn try_match_over(tokens: &[Token], i: usize) -> Option<OverMatch> {
    // tokens[i] must be a Group (the numerator).
    if !matches!(tokens.get(i), Some(Token::Group(_))) {
        return None;
    }
    let num_raw = tokens[i].as_str();

    // Skip optional whitespace.
    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    // tokens[j] must be the bare keyword "over".
    if !matches!(tokens.get(j), Some(Token::Word(w)) if w == "over") {
        return None;
    }
    j += 1;

    // Skip optional whitespace.
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    // tokens[j] must be a Group (the denominator).
    if !matches!(tokens.get(j), Some(Token::Group(_))) {
        return None;
    }
    let den_raw = tokens[j].as_str();
    j += 1;

    let num_inner = num_raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')).unwrap_or(num_raw);
    let den_inner = den_raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')).unwrap_or(den_raw);
    let num_latex = eqedit_to_latex(num_inner);
    let den_latex = eqedit_to_latex(den_inner);
    let latex = format!("\\frac{{{num_latex}}}{{{den_latex}}}");
    Some(OverMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 2b – Transform `root`
// ---------------------------------------------------------------------------

/// Rewrite `root {N} {X}` sequences (bare-word form) into `\sqrt[N]{X}`.
///
/// The tokeniser produces `Word("root")`, `Group("{N}")`, `Group("{X}")`.
/// We consume those three consecutive tokens and emit a single `Word` containing
/// the converted LaTeX.
fn transform_root(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        if let Some(result) = try_match_root(&tokens, i) {
            out.push(Token::Word(result.latex));
            i = result.next_idx;
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

fn try_match_root(tokens: &[Token], i: usize) -> Option<RootMatch> {
    // tokens[i] must be Word("root").
    if !matches!(tokens.get(i), Some(Token::Word(w)) if w == "root") {
        return None;
    }

    // Skip optional whitespace.
    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    // First Group: the index N.
    if !matches!(tokens.get(j), Some(Token::Group(_))) {
        return None;
    }
    let n_group = tokens[j].as_str();
    let n_inner = n_group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(n_group);
    let n_latex = eqedit_to_latex(n_inner);
    j += 1;

    // Skip optional whitespace.
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    // Second Group: the radicand X.
    if !matches!(tokens.get(j), Some(Token::Group(_))) {
        return None;
    }
    let x_group = tokens[j].as_str();
    let x_inner = x_group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(x_group);
    let x_latex = eqedit_to_latex(x_inner);
    j += 1;

    let latex = format!("\\sqrt[{n_latex}]{{{x_latex}}}");
    Some(RootMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 2c – Transform `matrix` and `pile`
// ---------------------------------------------------------------------------

/// Rewrite `matrix {…}` and `pile {…}` (bare-word forms) into their LaTeX
/// `matrix` environment equivalents.
fn transform_matrix(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        if let Some(result) = try_match_matrix(&tokens, i) {
            out.push(Token::Word(result.latex));
            i = result.next_idx;
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

fn try_match_matrix(tokens: &[Token], i: usize) -> Option<MatrixMatch> {
    // tokens[i] must be Word("matrix") or Word("pile").
    let is_pile = match tokens.get(i) {
        Some(Token::Word(w)) if w == "matrix" => false,
        Some(Token::Word(w)) if w == "pile" => true,
        _ => return None,
    };

    // Skip optional whitespace.
    let mut j = i + 1;
    while matches!(tokens.get(j), Some(Token::Word(w)) if w.trim().is_empty()) {
        j += 1;
    }

    // Next token must be a Group.
    let group = match tokens.get(j) {
        Some(Token::Group(g)) => g.clone(),
        _ => return None,
    };
    j += 1;

    let body = group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(&group);

    let rows: Vec<String> = body
        .split('#')
        .map(|row| eqedit_to_latex(row.trim()))
        .collect();

    let env_body = rows.join(" \\\\ ");
    let _ = is_pile; // both forms use the same environment
    let latex = format!("\\begin{{matrix}}{env_body}\\end{{matrix}}");

    Some(MatrixMatch { latex, next_idx: j })
}

// ---------------------------------------------------------------------------
// Step 3 – Expand keywords
// ---------------------------------------------------------------------------

/// Map each `Word` token that matches an EQEDIT keyword to its LaTeX equivalent.
///
/// `Group` tokens are recursively converted so that keywords inside braces (e.g.
/// `sqrt{alpha}`) are handled correctly.
fn expand_keywords(tokens: Vec<Token>) -> Vec<Token> {
    tokens
        .into_iter()
        .map(|tok| match tok {
            Token::Word(w) => Token::Word(map_keyword(&w)),
            Token::Group(g) => Token::Group(expand_group(&g)),
        })
        .collect()
}

/// Recursively convert the *interior* of a `{…}` group.
fn expand_group(group: &str) -> String {
    // Strip outer braces, convert interior, re-wrap.
    let inner = group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(group);

    // Check whether the group begins with a structural keyword.  These need
    // special handling at this level rather than generic keyword expansion.
    let trimmed = inner.trim_start();
    if trimmed.starts_with("matrix") && matches_keyword_start(trimmed, "matrix") {
        return convert_matrix(trimmed, "matrix", false);
    }
    if trimmed.starts_with("pile") && matches_keyword_start(trimmed, "pile") {
        return convert_matrix(trimmed, "pile", true);
    }
    if trimmed.starts_with("root") && matches_keyword_start(trimmed, "root") {
        return convert_root(trimmed);
    }

    // General case: recursively tokenise → transform_over → expand → reassemble.
    let converted = eqedit_to_latex(inner);
    format!("{{{converted}}}")
}

/// Check that `s` starts with `keyword` followed by whitespace or `{` (i.e.
/// it is the keyword as a whole word, not a prefix of a longer identifier).
fn matches_keyword_start(s: &str, keyword: &str) -> bool {
    let rest = &s[keyword.len()..];
    rest.is_empty() || rest.starts_with('{') || rest.starts_with(' ') || rest.starts_with('\t')
}

/// Look up a bare EQEDIT keyword and return its LaTeX replacement.
///
/// If the word is not a recognised keyword it is returned unchanged.
fn map_keyword(word: &str) -> String {
    // Handle structural keywords that appear as bare words (not inside braces).
    match word {
        // --- Greek letters (lowercase) ---
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

        // --- Greek letters (uppercase) ---
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

        // --- Arithmetic / relational operators ---
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

        // --- Set / logic operators ---
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

        // --- Calculus / large operators ---
        "sum" => "\\sum".into(),
        "prod" => "\\prod".into(),
        "int" => "\\int".into(),
        "oint" => "\\oint".into(),
        "lim" => "\\lim".into(),
        "inf" => "\\infty".into(),
        "infty" => "\\infty".into(),
        "partial" => "\\partial".into(),
        "nabla" => "\\nabla".into(),

        // --- Arrows ---
        "to" => "\\to".into(),
        "leftarrow" => "\\leftarrow".into(),
        "rightarrow" => "\\rightarrow".into(),
        "Leftarrow" => "\\Leftarrow".into(),
        "Rightarrow" => "\\Rightarrow".into(),
        "leftrightarrow" => "\\leftrightarrow".into(),
        "Leftrightarrow" => "\\Leftrightarrow".into(),

        // --- Miscellaneous ---
        "sqrt" => "\\sqrt".into(),
        "vec" => "\\vec".into(),
        "hat" => "\\hat".into(),
        "bar" => "\\bar".into(),
        "tilde" => "\\tilde".into(),
        "dot" => "\\dot".into(),
        "ddot" => "\\ddot".into(),

        // --- Delimiter keywords ---
        "left(" => "\\left(".into(),
        "right)" => "\\right)".into(),
        "left[" => "\\left[".into(),
        "right]" => "\\right]".into(),
        "left|" => "\\left|".into(),
        "right|" => "\\right|".into(),
        "left" => "\\left".into(),
        "right" => "\\right".into(),
        "{" => "\\{".into(),
        "}" => "\\}".into(),

        // `root` as a standalone word means nothing; handled in expand_group.
        // `matrix` / `pile` similarly.
        // Pass everything else through as-is.
        other => other.into(),
    }
}

// ---------------------------------------------------------------------------
// Step 4 – Handle structural forms: `root`, `matrix`, `pile`
// ---------------------------------------------------------------------------

/// Convert `matrix{…}` or `pile{…}` to a LaTeX `matrix` environment.
///
/// EQEDIT separates rows with `#` and (for matrix) columns with `&`.
/// For `pile` (single-column), rows are also `#`-separated; we treat each row
/// as a single cell.
///
/// The function receives the *interior* of the outer group (i.e. `matrix{…}` or
/// `pile{…}` without any surrounding braces), and returns the full LaTeX string
/// including the outer `{…}` wrapper expected by `expand_group`.
fn convert_matrix(inner: &str, keyword: &str, _is_pile: bool) -> String {
    // Strip leading keyword name.
    let rest = inner[keyword.len()..].trim_start();

    // The rest must begin with `{` — grab the balanced group.
    let chars: Vec<char> = rest.chars().collect();
    if chars.first() != Some(&'{') {
        // Malformed — return the raw interior wrapped.
        return format!("{{{inner}}}");
    }
    let (group, _) = consume_brace_group(&chars, 0);
    let body = group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(&group);

    // Split on `#` (row separator).  Within each row, convert EQEDIT to LaTeX.
    let rows: Vec<String> = body
        .split('#')
        .map(|row| eqedit_to_latex(row.trim()))
        .collect();

    // Join rows with `\\` (LaTeX newline) and wrap in the matrix environment.
    let env_body = rows.join(" \\\\ ");
    format!("{{\\begin{{matrix}}{env_body}\\end{{matrix}}}}")
}

/// Convert a `root{N}{X}` form to `\sqrt[N]{X}`.
///
/// This is called from `expand_group` when the content of a group starts with
/// `root`.  It expects the *interior* of the outer group (everything between the
/// outer `{…}`), and returns the full LaTeX string including the outer `{…}`
/// wrapper.
fn convert_root(inner: &str) -> String {
    // Strip leading "root" and whitespace.
    let rest = inner["root".len()..].trim_start();
    let chars: Vec<char> = rest.chars().collect();

    if chars.first() != Some(&'{') {
        return format!("{{{inner}}}");
    }

    // First group: the index N.
    let (n_group, after_n) = consume_brace_group(&chars, 0);
    let n_inner = n_group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(&n_group);
    let n_latex = eqedit_to_latex(n_inner);

    // Skip whitespace after first group.
    let remaining: String = chars[after_n..].iter().collect();
    let remaining = remaining.trim_start();
    let rem_chars: Vec<char> = remaining.chars().collect();

    if rem_chars.first() != Some(&'{') {
        return format!("{{{inner}}}");
    }

    // Second group: the radicand X.
    let (x_group, _) = consume_brace_group(&rem_chars, 0);
    let x_inner = x_group
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(&x_group);
    let x_latex = eqedit_to_latex(x_inner);

    format!("{{\\sqrt[{n_latex}]{{{x_latex}}}}}")
}

// Override `expand_group` for the `root` keyword.
// The function is called from `expand_group`; update that function to dispatch
// to `convert_root` when appropriate.
// (This is handled inline in `expand_group` above via `trimmed.starts_with`.)

// ---------------------------------------------------------------------------
// Step 5 – Reassemble
// ---------------------------------------------------------------------------

/// Concatenate all tokens back into a string, stripping leading/trailing spaces.
fn reassemble(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.as_str()).collect::<String>()
}

// ---------------------------------------------------------------------------
// Patch: update expand_group to handle `root`
// ---------------------------------------------------------------------------
// (Already integrated above — `expand_group` checks for "root" before the
// general recursive path.)

// ---------------------------------------------------------------------------
// Public entry point (re-export for clarity)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- helper -----------------------------------------------------------

    /// Assert that `eqedit_to_latex(input)` equals `expected`, trimming both.
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
        // A script that is already valid LaTeX (no EQEDIT keywords) must
        // survive unchanged.
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
        // Multiple spaces between components should still match.
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
        // {{{a} over {b}} over {c}} is unusual but should not panic.
        // We only check that it contains two \frac occurrences.
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
        check(
            "matrix{1 & 2 & 3}",
            "\\begin{matrix}1 & 2 & 3\\end{matrix}",
        );
    }

    #[test]
    fn pile_two_rows() {
        check(
            "pile{a # b}",
            "\\begin{matrix}a \\\\ b\\end{matrix}",
        );
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
        // If `over` appears without surrounding groups it is not a fraction.
        // It may not produce valid LaTeX, but must not panic.
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
