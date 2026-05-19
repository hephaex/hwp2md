use super::sanitize_asset_name;
use super::next_available_name;
use std::collections::HashMap;

// -----------------------------------------------------------------------
// sanitize_asset_name — path-traversal stripping
// -----------------------------------------------------------------------

#[test]
fn sanitize_strips_path_traversal() {
    assert_eq!(sanitize_asset_name("../../etc/passwd"), "passwd");
}

#[test]
fn sanitize_strips_single_directory_prefix() {
    assert_eq!(sanitize_asset_name("subdir/image.png"), "image.png");
}

#[test]
fn sanitize_replaces_backslashes_on_unix() {
    // On Unix, backslash is not a path separator, so the whole string becomes
    // the basename. The function must still replace the backslashes with '_'.
    assert_eq!(
        sanitize_asset_name(r"C:\Windows\evil.exe"),
        "C:_Windows_evil.exe"
    );
}

// -----------------------------------------------------------------------
// sanitize_asset_name — NUL byte replacement
// -----------------------------------------------------------------------

#[test]
fn sanitize_replaces_nul() {
    assert_eq!(sanitize_asset_name("bad\0name.png"), "bad_name.png");
}

// -----------------------------------------------------------------------
// sanitize_asset_name — Windows reserved names
// -----------------------------------------------------------------------

#[test]
fn sanitize_windows_reserved_stem_uppercase() {
    assert_eq!(sanitize_asset_name("CON.png"), "_CON.png");
}

#[test]
fn sanitize_windows_reserved_stem_lowercase() {
    assert_eq!(sanitize_asset_name("con.txt"), "_con.txt");
}

#[test]
fn sanitize_windows_reserved_nul_no_ext() {
    assert_eq!(sanitize_asset_name("NUL"), "_NUL");
}

#[test]
fn sanitize_windows_reserved_com1() {
    assert_eq!(sanitize_asset_name("COM1.log"), "_COM1.log");
}

#[test]
fn sanitize_windows_reserved_lpt9() {
    assert_eq!(sanitize_asset_name("LPT9"), "_LPT9");
}

#[test]
fn sanitize_non_reserved_name_unchanged() {
    assert_eq!(sanitize_asset_name("logo.png"), "logo.png");
}

// -----------------------------------------------------------------------
// sanitize_asset_name — empty / dot-only fallback
// -----------------------------------------------------------------------

#[test]
fn sanitize_empty_falls_back() {
    assert_eq!(sanitize_asset_name(""), "asset");
}

#[test]
fn sanitize_single_dot_falls_back() {
    assert_eq!(sanitize_asset_name("."), "asset");
}

#[test]
fn sanitize_double_dot_falls_back() {
    assert_eq!(sanitize_asset_name(".."), "asset");
}

// -----------------------------------------------------------------------
// next_available_name — collision counter
// -----------------------------------------------------------------------

#[test]
fn next_available_name_first_use_unchanged() {
    let mut seen = HashMap::new();
    assert_eq!(next_available_name("logo.png", &mut seen), "logo.png");
}

#[test]
fn next_available_name_second_use_gets_suffix_2() {
    let mut seen = HashMap::new();
    next_available_name("logo.png", &mut seen);
    assert_eq!(next_available_name("logo.png", &mut seen), "logo (2).png");
}

#[test]
fn next_available_name_third_use_gets_suffix_3() {
    let mut seen = HashMap::new();
    next_available_name("logo.png", &mut seen);
    next_available_name("logo.png", &mut seen);
    assert_eq!(next_available_name("logo.png", &mut seen), "logo (3).png");
}

#[test]
fn next_available_name_no_extension_collision() {
    let mut seen = HashMap::new();
    next_available_name("readme", &mut seen);
    assert_eq!(next_available_name("readme", &mut seen), "readme (2)");
}

#[test]
fn next_available_name_independent_names_dont_collide() {
    let mut seen = HashMap::new();
    assert_eq!(next_available_name("a.png", &mut seen), "a.png");
    assert_eq!(next_available_name("b.png", &mut seen), "b.png");
    assert_eq!(next_available_name("a.png", &mut seen), "a (2).png");
}
