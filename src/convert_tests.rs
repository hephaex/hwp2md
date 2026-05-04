use super::*;
use crate::ir::{Asset, Document};
use std::path::PathBuf;

// -----------------------------------------------------------------------
// write_assets — no-op when doc has no assets
// -----------------------------------------------------------------------

#[test]
fn write_assets_empty_doc_does_nothing() {
    let doc = Document::new();
    let dir = tempfile::tempdir().unwrap();
    // Must succeed and must NOT create any files.
    write_assets(&doc, dir.path()).unwrap();
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "Expected no files extracted for empty doc"
    );
}

#[test]
fn write_assets_creates_dir_and_extracts_files() {
    let mut doc = Document::new();
    doc.assets.push(Asset {
        name: "image.png".into(),
        data: vec![0x89, 0x50, 0x4e, 0x47],
        mime_type: "image/png".into(),
    });
    doc.assets.push(Asset {
        name: "style.css".into(),
        data: b"body{}".to_vec(),
        mime_type: "text/css".into(),
    });

    let dir = tempfile::tempdir().unwrap();
    let assets_dir = dir.path().join("assets");
    // Directory does not exist yet — write_assets must create it.
    assert!(!assets_dir.exists());
    write_assets(&doc, &assets_dir).unwrap();

    assert!(assets_dir.join("image.png").exists());
    assert!(assets_dir.join("style.css").exists());
    assert_eq!(
        std::fs::read(assets_dir.join("style.css")).unwrap(),
        b"body{}"
    );
}

#[test]
fn write_assets_path_traversal_name_is_sanitised() {
    // An asset with a path-traversal name should only use the file-name
    // component, not the directory path prefix.
    let mut doc = Document::new();
    doc.assets.push(Asset {
        name: "../../etc/evil.txt".into(),
        data: b"evil".to_vec(),
        mime_type: "text/plain".into(),
    });

    let dir = tempfile::tempdir().unwrap();
    write_assets(&doc, dir.path()).unwrap();

    // File must land inside the assets dir, not above it.
    assert!(dir.path().join("evil.txt").exists());
    // The parent directory must NOT have been escaped.
    assert!(!dir.path().join("../../etc/evil.txt").exists());
}

// -----------------------------------------------------------------------
// to_hwpx — unsupported extension rejected
// -----------------------------------------------------------------------

#[test]
fn to_hwpx_rejects_non_markdown_extension() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.txt");
    std::fs::write(&input, "# Hello\n").unwrap();
    let result = to_hwpx(&input, None, None);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Expected .md or .markdown"),
        "unexpected error message: {msg}"
    );
}

// -----------------------------------------------------------------------
// to_markdown — unsupported extension rejected
// -----------------------------------------------------------------------

#[test]
fn to_markdown_rejects_unknown_extension() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.docx");
    std::fs::write(&input, b"placeholder").unwrap();
    let result = to_markdown(&input, None, None, false);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Unsupported format"),
        "unexpected error message: {msg}"
    );
}

// -----------------------------------------------------------------------
// to_markdown — markdown output to file
// -----------------------------------------------------------------------

#[test]
fn to_markdown_md_input_to_output_file_via_hwpx_roundtrip() {
    // We cannot round-trip a full HWP binary here, but we can verify
    // that to_hwpx writes a file and to_markdown on a .hwpx file is
    // rejected for an unknown extension (coverage smoke test).
    let dir = tempfile::tempdir().unwrap();
    let md_in = dir.path().join("input.md");
    std::fs::write(&md_in, "# Title\n\nBody text.\n").unwrap();
    let hwpx_out = dir.path().join("output.hwpx");

    // to_hwpx must succeed for a valid .md file.
    to_hwpx(&md_in, Some(&hwpx_out), None).unwrap();
    assert!(hwpx_out.exists(), "hwpx output file not created");

    // to_markdown on the resulting hwpx must succeed.
    let md_out = dir.path().join("result.md");
    to_markdown(&hwpx_out, Some(&md_out), None, false).unwrap();
    assert!(md_out.exists(), "markdown output file not created");

    let content = std::fs::read_to_string(&md_out).unwrap();
    assert!(
        content.contains("Title"),
        "heading lost in hwpx roundtrip; got: {content:?}"
    );
}

// -----------------------------------------------------------------------
// show_info — unsupported extension rejected
// -----------------------------------------------------------------------

#[test]
fn show_info_rejects_unknown_extension() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.pdf");
    std::fs::write(&input, b"fake-pdf").unwrap();
    let result = show_info(&input);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Unsupported format"),
        "unexpected error message: {msg}"
    );
}

// -----------------------------------------------------------------------
// check — valid .md file
// -----------------------------------------------------------------------

#[test]
fn check_valid_md_returns_ok() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    std::fs::write(&input, "# Hello\n\nParagraph.\n").unwrap();
    assert!(check(&input).is_ok());
}

#[test]
fn check_valid_markdown_extension_returns_ok() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.markdown");
    std::fs::write(&input, "Some *content*.\n").unwrap();
    assert!(check(&input).is_ok());
}

// -----------------------------------------------------------------------
// check — valid HWPX (roundtrip through to_hwpx)
// -----------------------------------------------------------------------

#[test]
fn check_valid_hwpx_returns_ok() {
    let dir = tempfile::tempdir().unwrap();
    let md_in = dir.path().join("source.md");
    std::fs::write(&md_in, "# Check test\n\nBody.\n").unwrap();
    let hwpx_out = dir.path().join("output.hwpx");
    to_hwpx(&md_in, Some(&hwpx_out), None).unwrap();

    assert!(check(&hwpx_out).is_ok());
}

// -----------------------------------------------------------------------
// check — unsupported extension
// -----------------------------------------------------------------------

#[test]
fn check_unsupported_extension_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.docx");
    std::fs::write(&input, b"placeholder").unwrap();
    let result = check(&input);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Unsupported format"),
        "unexpected error: {msg}"
    );
}

#[test]
fn check_no_extension_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("noext");
    std::fs::write(&input, b"data").unwrap();
    let result = check(&input);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Unsupported format"),
        "unexpected error: {msg}"
    );
}

// -----------------------------------------------------------------------
// check — I/O error (file does not exist)
// -----------------------------------------------------------------------

#[test]
fn check_nonexistent_md_file_returns_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("missing.md");
    let result = check(&input);
    assert!(result.is_err());
}

#[test]
fn check_nonexistent_hwpx_file_returns_error() {
    let input = std::path::Path::new("/nonexistent/path/doc.hwpx");
    let result = check(input);
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// check — invalid HWPX (truncated / corrupt ZIP)
// -----------------------------------------------------------------------

#[test]
fn check_invalid_hwpx_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("corrupt.hwpx");
    std::fs::write(&input, b"not a zip file").unwrap();
    let result = check(&input);
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// MAX_MD_FILE_SIZE constant value
// -----------------------------------------------------------------------

#[test]
fn max_md_file_size_constant_is_256_mib() {
    assert_eq!(MAX_MD_FILE_SIZE, 256 * 1024 * 1024);
    assert_eq!(MAX_MD_FILE_SIZE, 268_435_456);
}

// -----------------------------------------------------------------------
// check — file-size guard for Markdown
// -----------------------------------------------------------------------

#[test]
fn check_md_within_size_limit_returns_ok() {
    // A small but valid Markdown file must pass the size guard and parse
    // without error.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("small.md");
    std::fs::write(&input, "# Small\n\nThis file is well under 256 MB.\n").unwrap();
    assert!(check(&input).is_ok());
}

#[test]
fn check_md_exceeds_size_limit_returns_file_too_large_variant() {
    let tmp = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
    tmp.as_file().set_len(MAX_MD_FILE_SIZE + 1).unwrap();
    let expected_path = tmp.path().to_path_buf();

    let err = check(tmp.path()).expect_err("oversized md must fail check()");
    match err {
        Hwp2MdError::FileTooLarge { size, limit, path } => {
            assert_eq!(limit, MAX_MD_FILE_SIZE);
            assert_eq!(size, MAX_MD_FILE_SIZE + 1);
            assert_eq!(path, expected_path, "path payload must match input");
            assert_eq!(
                path.extension().and_then(|e| e.to_str()),
                Some("md"),
                "expected .md extension in error path, got {path:?}"
            );
        }
        other => panic!("expected FileTooLarge variant, got {other:?}"),
    }
}

#[test]
fn file_too_large_display_includes_path_size_and_limit() {
    let err = Hwp2MdError::FileTooLarge {
        path: std::path::PathBuf::from("/tmp/big.md"),
        size: 300_000_000,
        limit: 268_435_456,
    };
    let msg = format!("{err}");
    assert!(msg.contains("/tmp/big.md"), "missing path: {msg}");
    assert!(msg.contains("300000000"), "missing size: {msg}");
    assert!(msg.contains("268435456"), "missing limit: {msg}");
}

// -----------------------------------------------------------------------
// convert_auto — extension-based format detection (Sprint 3, C-2)
// -----------------------------------------------------------------------

#[test]
fn classify_format_recognises_known_extensions() {
    assert_eq!(classify_format("hwp"), FormatKind::Hwp);
    assert_eq!(classify_format("hwpx"), FormatKind::Hwpx);
    assert_eq!(classify_format("md"), FormatKind::Markdown);
    assert_eq!(classify_format("markdown"), FormatKind::Markdown);
    assert_eq!(classify_format("txt"), FormatKind::Unknown);
    assert_eq!(classify_format(""), FormatKind::Unknown);
}

#[test]
fn convert_auto_md_to_hwpx_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    let output = dir.path().join("out.hwpx");
    std::fs::write(&input, "# Hello\n\nBody.\n").unwrap();
    convert_auto(&input, &output, true).unwrap();
    assert!(output.exists(), "convert_auto must create the output file");
    // The result must be a valid HWPX, accepted by check().
    check(&output).unwrap();
}

#[test]
fn convert_auto_hwpx_to_md_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let md_in = dir.path().join("source.md");
    std::fs::write(&md_in, "# Title\n\nContent.\n").unwrap();
    let hwpx = dir.path().join("intermediate.hwpx");
    to_hwpx(&md_in, Some(&hwpx), None).unwrap();

    let md_out = dir.path().join("converted.md");
    convert_auto(&hwpx, &md_out, true).unwrap();
    let content = std::fs::read_to_string(&md_out).unwrap();
    assert!(
        content.contains("Title"),
        "heading lost in convert_auto: {content:?}"
    );
}

#[test]
fn convert_auto_markdown_extension_alias_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.markdown");
    let output = dir.path().join("out.hwpx");
    std::fs::write(&input, "# A\n").unwrap();
    convert_auto(&input, &output, true).unwrap();
    assert!(output.exists());
}

#[test]
fn convert_auto_md_to_md_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.md");
    let output = dir.path().join("b.md");
    std::fs::write(&input, "# Hello\n").unwrap();
    let result = convert_auto(&input, &output, true);
    assert!(result.is_err(), "same-format conversion must be rejected");
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("cannot infer conversion direction"),
        "unexpected error: {msg}"
    );
}

#[test]
fn convert_auto_hwp_to_hwpx_is_rejected() {
    // .hwp → .hwpx is currently unsupported (writer only emits HWPX from
    // Markdown).  Auto-detect must surface this as an extension-pair
    // error rather than attempting a partial conversion.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.hwp");
    let output = dir.path().join("dest.hwpx");
    std::fs::write(&input, b"placeholder").unwrap();
    let result = convert_auto(&input, &output, true);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("cannot infer conversion direction"),
        "unexpected error: {msg}"
    );
}

#[test]
fn convert_auto_unknown_extension_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.docx");
    let output = dir.path().join("out.md");
    std::fs::write(&input, b"x").unwrap();
    let result = convert_auto(&input, &output, true);
    assert!(result.is_err());
}

#[test]
fn convert_auto_extension_check_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("DOC.MD");
    let output = dir.path().join("OUT.HWPX");
    std::fs::write(&input, "# Upper\n").unwrap();
    convert_auto(&input, &output, true).unwrap();
    assert!(output.exists());
}

// -----------------------------------------------------------------------
// convert_auto — --force / overwrite behaviour (Sprint 4, M-3)
// -----------------------------------------------------------------------

#[test]
fn convert_auto_refuses_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&input, "# Hello").unwrap();
    std::fs::write(&output, "existing").unwrap();

    let result = convert_auto(&input, &output, false);
    match result.unwrap_err() {
        Hwp2MdError::OutputExists { path } => {
            assert_eq!(path, output, "OutputExists must carry the output path");
        }
        other => panic!("expected OutputExists, got: {other}"),
    }
}

#[test]
fn convert_auto_overwrites_with_force() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&input, "# Hello").unwrap();
    std::fs::write(&output, "existing").unwrap();

    convert_auto(&input, &output, true).unwrap();
    // The output should now be a valid HWPX, not "existing".
    assert!(output.exists());
    let content = std::fs::read(&output).unwrap();
    assert_ne!(
        content, b"existing",
        "file must be overwritten with force=true"
    );
}

#[test]
fn convert_auto_no_force_needed_when_output_missing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&input, "# Hello").unwrap();
    // output doesn't exist — force=false must still succeed

    convert_auto(&input, &output, false).unwrap();
    assert!(output.exists());
}

// -----------------------------------------------------------------------
// C-4 — Structured error payload Display format tests
// -----------------------------------------------------------------------

#[test]
fn output_exists_display_includes_path() {
    let err = Hwp2MdError::OutputExists {
        path: PathBuf::from("test.hwpx"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("test.hwpx"),
        "display must include path: {msg}"
    );
    assert!(
        msg.contains("--force"),
        "display must mention --force: {msg}"
    );
}

#[test]
fn drm_protected_display_includes_path() {
    let err = Hwp2MdError::DrmProtected {
        path: PathBuf::from("secret.hwp"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("secret.hwp"),
        "display must include path: {msg}"
    );
    assert!(msg.contains("DRM"), "display must mention DRM: {msg}");
}

#[test]
fn output_exists_carries_correct_path() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("doc.md");
    let output = dir.path().join("doc.hwpx");
    std::fs::write(&input, "# Hi").unwrap();
    std::fs::write(&output, b"existing").unwrap();

    let err = convert_auto(&input, &output, false).unwrap_err();
    match err {
        Hwp2MdError::OutputExists { path } => {
            assert_eq!(path, output);
        }
        other => panic!("expected OutputExists, got: {other}"),
    }
}
