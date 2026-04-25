use super::*;

// -----------------------------------------------------------------------
// escape_yaml
// -----------------------------------------------------------------------

#[test]
fn escape_yaml_backslash() {
    assert_eq!(escape_yaml("a\\b"), "a\\\\b");
}

#[test]
fn escape_yaml_double_quote() {
    assert_eq!(escape_yaml("say \"hi\""), "say \\\"hi\\\"");
}

#[test]
fn escape_yaml_no_special_chars() {
    assert_eq!(escape_yaml("hello world"), "hello world");
}

#[test]
fn escape_yaml_newline() {
    assert_eq!(escape_yaml("line1\nline2"), "line1\\nline2");
}

#[test]
fn escape_yaml_carriage_return() {
    assert_eq!(escape_yaml("a\rb"), "a\\rb");
}

#[test]
fn escape_yaml_tab() {
    assert_eq!(escape_yaml("a\tb"), "a\\tb");
}
