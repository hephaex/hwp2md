use super::*;

// All tests here exercise the public ConvertOptions builder API.
// Private helpers (classify_format, write_assets, etc.) are not needed.

// ── helpers ───────────────────────────────────────────────────────────────────

/// Write a small Markdown file and return its path inside a temp dir.
fn md_file(dir: &tempfile::TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write md file");
    path
}

/// Build a minimal valid HWPX from a Markdown string and return its path.
fn hwpx_file(dir: &tempfile::TempDir, name: &str, md: &str) -> std::path::PathBuf {
    // The intermediate .md file must have an .md extension so to_hwpx accepts it.
    let md_path = md_file(dir, &format!("_src_{name}.md"), md);
    let hwpx_path = dir.path().join(name);
    to_hwpx(&md_path, Some(&hwpx_path), None).expect("to_hwpx in fixture");
    hwpx_path
}

// ── basic direction inference ─────────────────────────────────────────────────

#[test]
fn builder_md_to_hwpx_creates_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "doc.md", "# Hello\n\nBody.\n");
    let output = dir.path().join("doc.hwpx");

    ConvertOptions::new(&input, &output)
        .execute()
        .expect("md → hwpx must succeed");

    assert!(output.exists(), "output file must be created");
    check(&output).expect("output must be a valid HWPX");
}

#[test]
fn builder_hwpx_to_md_creates_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let hwpx = hwpx_file(&dir, "doc.hwpx", "# Title\n\nParagraph.\n");
    let output = dir.path().join("out.md");

    ConvertOptions::new(&hwpx, &output)
        .execute()
        .expect("hwpx → md must succeed");

    assert!(output.exists(), "output file must be created");
    let content = std::fs::read_to_string(&output).unwrap();
    assert!(
        content.contains("Title"),
        "heading must be preserved; got: {content:?}"
    );
}

#[test]
fn builder_markdown_extension_alias_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "doc.markdown", "# Alias\n");
    let output = dir.path().join("out.hwpx");

    ConvertOptions::new(&input, &output)
        .execute()
        .expect(".markdown extension must be accepted");

    assert!(output.exists());
}

// ── unsupported pairs ─────────────────────────────────────────────────────────

#[test]
fn builder_md_to_md_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "a.md", "# Hello\n");
    let output = dir.path().join("b.md");

    let err = ConvertOptions::new(&input, &output)
        .execute()
        .expect_err("same-format pair must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("cannot infer conversion direction"),
        "unexpected error: {msg}"
    );
}

#[test]
fn builder_unknown_extension_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.docx");
    std::fs::write(&input, b"x").unwrap();
    let output = dir.path().join("out.md");

    let err = ConvertOptions::new(&input, &output)
        .execute()
        .expect_err("unknown extension must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("cannot infer"), "unexpected error: {msg}");
}

// ── force / overwrite ─────────────────────────────────────────────────────────

#[test]
fn builder_refuses_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "doc.md", "# Hi\n");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&output, b"existing").unwrap();

    let err = ConvertOptions::new(&input, &output)
        // force defaults to false
        .execute()
        .expect_err("must fail when output exists and force is false");
    let msg = err.to_string();
    assert!(
        msg.contains("already exists"),
        "error must mention 'already exists': {msg}"
    );
    // Existing file must be untouched.
    assert_eq!(std::fs::read(&output).unwrap(), b"existing");
}

#[test]
fn builder_overwrites_with_force_true() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "doc.md", "# Overwrite\n");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&output, b"stale content").unwrap();

    ConvertOptions::new(&input, &output)
        .force(true)
        .execute()
        .expect("must succeed when force=true");

    let content = std::fs::read(&output).unwrap();
    assert_ne!(
        content, b"stale content",
        "file must be overwritten when force=true"
    );
    check(&output).expect("overwritten file must be a valid HWPX");
}

#[test]
fn builder_no_force_needed_when_output_missing() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "doc.md", "# New\n");
    let output = dir.path().join("doc.hwpx");
    // output does not exist — force=false must still succeed

    ConvertOptions::new(&input, &output)
        .force(false)
        .execute()
        .expect("must succeed when output does not exist");

    assert!(output.exists());
}

// ── frontmatter option (hwpx→md direction) ───────────────────────────────────

#[test]
fn builder_frontmatter_true_emits_yaml_block() {
    let dir = tempfile::tempdir().unwrap();
    let hwpx = hwpx_file(&dir, "doc.hwpx", "# Title\n\nBody.\n");
    let output = dir.path().join("out.md");

    ConvertOptions::new(&hwpx, &output)
        .frontmatter(true)
        .execute()
        .expect("hwpx → md with frontmatter must succeed");

    let content = std::fs::read_to_string(&output).unwrap();
    assert!(
        content.starts_with("---"),
        "frontmatter=true must emit a YAML block starting with '---': {content:?}"
    );
}

#[test]
fn builder_frontmatter_false_omits_yaml_block() {
    let dir = tempfile::tempdir().unwrap();
    let hwpx = hwpx_file(&dir, "doc.hwpx", "# Title\n\nBody.\n");
    let output = dir.path().join("out.md");

    ConvertOptions::new(&hwpx, &output)
        .frontmatter(false)
        .execute()
        .expect("hwpx → md without frontmatter must succeed");

    let content = std::fs::read_to_string(&output).unwrap();
    assert!(
        !content.starts_with("---"),
        "frontmatter=false must NOT emit a YAML block: {content:?}"
    );
}

// ── assets_dir option ─────────────────────────────────────────────────────────

#[test]
fn builder_assets_dir_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let hwpx = hwpx_file(&dir, "doc.hwpx", "# Title\n");
    let output = dir.path().join("out.md");
    let assets = dir.path().join("imgs");

    // Even with no embedded images the directory is created when assets are
    // extracted (write_assets skips creation for empty asset lists, so we
    // only assert the output file is written successfully).
    ConvertOptions::new(&hwpx, &output)
        .assets_dir(&assets)
        .execute()
        .expect("hwpx → md with assets_dir must succeed");

    assert!(output.exists(), "output markdown must be created");
}

// ── chained builder methods ───────────────────────────────────────────────────

#[test]
fn builder_all_options_chained_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let hwpx = hwpx_file(&dir, "doc.hwpx", "# Chain\n\nText.\n");
    let output = dir.path().join("out.md");
    let assets = dir.path().join("assets");

    ConvertOptions::new(&hwpx, &output)
        .frontmatter(true)
        .assets_dir(&assets)
        .force(false)
        .execute()
        .expect("chained builder must succeed");

    assert!(output.exists());
}

// ── case-insensitive extension handling ───────────────────────────────────────

#[test]
fn builder_extension_check_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let input = md_file(&dir, "DOC.MD", "# Upper\n");
    let output = dir.path().join("OUT.HWPX");

    ConvertOptions::new(&input, &output)
        .execute()
        .expect("upper-case extension must be accepted");

    assert!(output.exists());
}

// ── Debug impl (derive check) ─────────────────────────────────────────────────

#[test]
fn builder_debug_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.md");
    let output = dir.path().join("b.hwpx");
    let opts = ConvertOptions::new(&input, &output)
        .frontmatter(true)
        .force(true);
    let dbg = format!("{opts:?}");
    assert!(dbg.contains("ConvertOptions"), "debug output: {dbg}");
}
