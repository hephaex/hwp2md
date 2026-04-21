use anyhow::{bail, Result};
use std::path::Path;

pub fn to_markdown(
    input: &Path,
    output: Option<&Path>,
    assets_dir: Option<&Path>,
    _frontmatter: bool,
) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "hwp" | "hwpx" => {
            tracing::info!("Detected format: {}", ext.to_uppercase());
        }
        _ => bail!("Unsupported format: .{ext}. Expected .hwp or .hwpx"),
    }

    let _ = (output, assets_dir);
    tracing::info!("HWP/HWPX → Markdown conversion not yet implemented");
    Ok(())
}

pub fn to_hwpx(input: &Path, output: Option<&Path>, style: Option<&Path>) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "md" && ext != "markdown" {
        bail!("Expected .md or .markdown file, got .{ext}");
    }

    let _ = (output, style);
    tracing::info!("Markdown → HWPX conversion not yet implemented");
    Ok(())
}

pub fn show_info(input: &Path) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "hwp" | "hwpx" => {
            tracing::info!("Document info for: {:?}", input);
        }
        _ => bail!("Unsupported format: .{ext}"),
    }

    tracing::info!("Document info not yet implemented");
    Ok(())
}
