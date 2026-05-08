//! High-level conversion entry points for HWP/HWPX ↔ Markdown.

use std::fs;
use std::path::Path;

/// Maximum permitted size for a Markdown file passed to [`check`].
///
/// Mirrors the 256 MB decompressed-stream limit used for HWP CFB streams so
/// that the `check` function never allocates an unbounded amount of heap memory
/// for a plain-text input.
const MAX_MD_FILE_SIZE: u64 = 256 * 1024 * 1024; // 268_435_456 bytes

use crate::error::Hwp2MdError;
use crate::hwp;
use crate::hwpx;
use crate::ir;
use crate::md;

/// Convert an HWP or HWPX document to Markdown.
///
/// Reads `input` (`.hwp` or `.hwpx`), converts it to Markdown, and writes the
/// result to `output` (or stdout when `None`).  Embedded images are extracted
/// to `assets_dir` when provided.  Set `frontmatter` to `true` to prepend a
/// YAML front-matter block with document metadata.
///
/// # Errors
///
/// Returns [`Hwp2MdError::UnsupportedFormat`] for unknown extensions and
/// propagates I/O or parse errors from the underlying readers.
pub fn to_markdown(
    input: &Path,
    output: Option<&Path>,
    assets_dir: Option<&Path>,
    frontmatter: bool,
) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let doc = match ext.as_str() {
        "hwp" => {
            tracing::info!("Parsing HWP 5.0: {:?}", input);
            hwp::read_hwp(input)?
        }
        "hwpx" => {
            tracing::info!("Parsing HWPX: {:?}", input);
            hwpx::read_hwpx(input)?
        }
        _ => {
            return Err(Hwp2MdError::UnsupportedFormat(format!(
                ".{ext}. Expected .hwp or .hwpx"
            )))
        }
    };

    if let Some(dir) = assets_dir {
        write_assets(&doc, dir)?;
    }

    let markdown = md::write_markdown(&doc, frontmatter);

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, &markdown)?;
            tracing::info!("Written to {:?}", path);
        }
        None => {
            print!("{markdown}");
        }
    }

    Ok(())
}

/// Convert a Markdown file to HWPX format.
///
/// Reads `input` (`.md` or `.markdown`), parses it into the intermediate
/// representation, and writes a conformant HWPX archive to `output`.  When
/// `output` is `None` the output path is derived by replacing the input
/// extension with `.hwpx`.  The optional `style` argument points to a YAML
/// style template that overrides page dimensions, margins, fonts, and heading
/// line spacing in the generated HWPX output.
///
/// # Errors
///
/// Returns [`Hwp2MdError::UnsupportedFormat`] when `input` does not have a
/// Markdown extension, and propagates I/O or write errors.
pub fn to_hwpx(
    input: &Path,
    output: Option<&Path>,
    style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "md" && ext != "markdown" {
        return Err(Hwp2MdError::UnsupportedFormat(format!(
            "Expected .md or .markdown file, got .{ext}"
        )));
    }

    let content = fs::read_to_string(input)?;
    let doc = md::parse_markdown(&content);

    let out_path = output
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("hwpx"));

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    hwpx::write_hwpx(&doc, &out_path, style)?;
    tracing::info!("Written to {:?}", out_path);

    Ok(())
}

/// Print human-readable metadata and statistics for an HWP or HWPX file.
///
/// Writes a summary (format, title, author, section count, block count,
/// estimated character count, asset count) to stdout.
///
/// # Errors
///
/// Returns [`Hwp2MdError::UnsupportedFormat`] for unknown extensions and
/// propagates parse errors from the underlying readers.
pub fn show_info(input: &Path) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "hwp" => {
            let doc = hwp::read_hwp(input)?;
            print_info(&doc, input);
        }
        "hwpx" => {
            let doc = hwpx::read_hwpx(input)?;
            print_info(&doc, input);
        }
        _ => return Err(Hwp2MdError::UnsupportedFormat(format!(".{ext}"))),
    }

    Ok(())
}

/// Auto-detect the conversion direction from the input/output file extensions
/// and dispatch to [`to_markdown`] or [`to_hwpx`].
///
/// Supported extension pairs (case-insensitive):
///
/// | Input ext            | Output ext           | Action       |
/// | -------------------- | -------------------- | ------------ |
/// | `.hwp`, `.hwpx`      | `.md`, `.markdown`   | [`to_markdown`] |
/// | `.md`, `.markdown`   | `.hwpx`              | [`to_hwpx`]  |
///
/// `.hwp` is a **read-only** format here — Markdown can never be written to
/// a `.hwp` output, only `.hwpx`.  The legacy binary HWP writer is out of
/// scope for this crate.
///
/// Any other combination — including same-format pairs, `.hwp` as the
/// output extension, or unknown extensions — returns
/// [`Hwp2MdError::UnsupportedFormat`] with a message describing the
/// offending pair.  The function never queries the network and never
/// inspects file contents to determine the direction; only the file
/// extensions are consulted.
///
/// When `force` is `false` and `output` already exists the function
/// returns [`Hwp2MdError::OutputExists`] instead of silently overwriting.
/// Pass `true` to permit overwriting.
pub fn convert_auto(input: &Path, output: &Path, force: bool) -> Result<(), Hwp2MdError> {
    if !force && output.exists() {
        return Err(Hwp2MdError::OutputExists {
            path: output.to_path_buf(),
        });
    }

    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let in_kind = classify_format(&in_ext);
    let out_kind = classify_format(&out_ext);

    match (in_kind, out_kind) {
        (FormatKind::Hwp | FormatKind::Hwpx, FormatKind::Markdown) => {
            to_markdown(input, Some(output), None, false)
        }
        (FormatKind::Markdown, FormatKind::Hwpx) => to_hwpx(input, Some(output), None),
        _ => Err(Hwp2MdError::UnsupportedFormat(format!(
            "cannot infer conversion direction from .{in_ext} -> .{out_ext}; \
             expected .hwp/.hwpx -> .md/.markdown or .md/.markdown -> .hwpx"
        ))),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FormatKind {
    Hwp,
    Hwpx,
    Markdown,
    Unknown,
}

fn classify_format(ext: &str) -> FormatKind {
    match ext {
        "hwp" => FormatKind::Hwp,
        "hwpx" => FormatKind::Hwpx,
        "md" | "markdown" => FormatKind::Markdown,
        _ => FormatKind::Unknown,
    }
}

/// Validate a file by parsing it into the IR without producing any output.
///
/// Detects the format from the file extension, reads the file, and attempts
/// to parse it.  Returns `Ok(())` if parsing succeeds, or an [`Hwp2MdError`]
/// with details if the file cannot be read or is structurally invalid.
///
/// Supported extensions: `.hwp`, `.hwpx`, `.md`, `.markdown`.
pub fn check(input: &Path) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "hwp" => {
            tracing::info!("Checking HWP 5.0: {:?}", input);
            hwp::read_hwp(input)?;
        }
        "hwpx" => {
            tracing::info!("Checking HWPX: {:?}", input);
            hwpx::read_hwpx(input)?;
        }
        "md" | "markdown" => {
            tracing::info!("Checking Markdown: {:?}", input);
            let file_size = fs::metadata(input)?.len();
            if file_size > MAX_MD_FILE_SIZE {
                return Err(Hwp2MdError::FileTooLarge {
                    path: input.to_path_buf(),
                    size: file_size,
                    limit: MAX_MD_FILE_SIZE,
                });
            }
            let content = fs::read_to_string(input)?;
            let _doc = md::parse_markdown(&content);
        }
        _ => {
            return Err(Hwp2MdError::UnsupportedFormat(format!(
                ".{ext}. Expected .hwp, .hwpx, .md, or .markdown"
            )));
        }
    }

    Ok(())
}

fn print_info(doc: &ir::Document, path: &Path) {
    println!("File: {}", path.display());
    println!(
        "Format: {}",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
    );

    if let Some(ref title) = doc.metadata.title {
        println!("Title: {title}");
    }
    if let Some(ref author) = doc.metadata.author {
        println!("Author: {author}");
    }

    println!("Sections: {}", doc.sections.len());

    let block_count: usize = doc.sections.iter().map(|s| s.blocks.len()).sum();
    println!("Blocks: {block_count}");

    let char_count: usize = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .map(count_chars)
        .sum();
    println!("Characters: ~{char_count}");
    println!("Assets: {}", doc.assets.len());
}

fn count_chars(block: &ir::Block) -> usize {
    match block {
        ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
            inlines.iter().map(|i| i.text.chars().count()).sum()
        }
        ir::Block::CodeBlock { code, .. } => code.chars().count(),
        ir::Block::BlockQuote { blocks } => blocks.iter().map(count_chars).sum(),
        ir::Block::List { items, .. } => {
            items.iter().flat_map(|i| &i.blocks).map(count_chars).sum()
        }
        ir::Block::Table { rows, .. } => rows
            .iter()
            .flat_map(|r| &r.cells)
            .flat_map(|c| &c.blocks)
            .map(count_chars)
            .sum(),
        ir::Block::Math { tex, .. } => tex.chars().count(),
        ir::Block::Footnote { content, .. } => content.iter().map(count_chars).sum(),
        ir::Block::Image { .. } | ir::Block::HorizontalRule | ir::Block::PageBreak => 0,
    }
}

fn write_assets(doc: &ir::Document, dir: &Path) -> Result<(), Hwp2MdError> {
    if doc.assets.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(dir)?;

    for asset in &doc.assets {
        let safe_name = std::path::Path::new(&asset.name)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("asset"));
        let path = dir.join(safe_name);
        fs::write(&path, &asset.data)?;
        tracing::info!("Extracted: {:?}", path);
    }

    Ok(())
}

// ── ConvertOptions builder ────────────────────────────────────────────────────

/// A builder for configuring and executing a single HWP/HWPX ↔ Markdown
/// conversion.
///
/// `ConvertOptions` provides a fluent API that is easier to use than the
/// individual [`to_markdown`] / [`to_hwpx`] functions when several optional
/// parameters are needed.  The conversion direction is inferred automatically
/// from the `input` and `output` file extensions, identical to
/// [`convert_auto`].
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use hwp2md::convert::ConvertOptions;
///
/// // HWPX → Markdown with frontmatter and image extraction
/// ConvertOptions::new(Path::new("doc.hwpx"), Path::new("doc.md"))
///     .frontmatter(true)
///     .assets_dir(Path::new("images"))
///     .execute()
///     .expect("conversion failed");
///
/// // Markdown → HWPX, overwrite if output already exists
/// ConvertOptions::new(Path::new("doc.md"), Path::new("doc.hwpx"))
///     .force(true)
///     .execute()
///     .expect("conversion failed");
/// ```
#[derive(Debug)]
pub struct ConvertOptions<'a> {
    input: &'a Path,
    output: &'a Path,
    assets_dir: Option<&'a Path>,
    frontmatter: bool,
    style: Option<&'a Path>,
    force: bool,
}

impl<'a> ConvertOptions<'a> {
    /// Create a new builder for the given `input` → `output` conversion.
    ///
    /// The conversion direction is inferred from the file extensions:
    ///
    /// | `input` extension     | `output` extension    | Action           |
    /// | --------------------- | --------------------- | ---------------- |
    /// | `.hwp`, `.hwpx`       | `.md`, `.markdown`    | → Markdown       |
    /// | `.md`, `.markdown`    | `.hwpx`               | → HWPX           |
    ///
    /// All optional settings default to their "off" value; call the builder
    /// methods to customise them before calling [`execute`](Self::execute).
    #[must_use]
    pub fn new(input: &'a Path, output: &'a Path) -> Self {
        Self {
            input,
            output,
            assets_dir: None,
            frontmatter: false,
            style: None,
            force: false,
        }
    }

    /// Set the directory into which embedded images are extracted.
    ///
    /// Only used when converting HWP/HWPX → Markdown.  Ignored for the
    /// reverse direction.
    #[must_use]
    pub fn assets_dir(mut self, dir: &'a Path) -> Self {
        self.assets_dir = Some(dir);
        self
    }

    /// Prepend a YAML front-matter block with document metadata.
    ///
    /// Only used when converting HWP/HWPX → Markdown.  Defaults to `false`.
    #[must_use]
    pub fn frontmatter(mut self, enabled: bool) -> Self {
        self.frontmatter = enabled;
        self
    }

    /// Use `path` as the YAML style template for the generated HWPX.
    ///
    /// Only used when converting Markdown → HWPX.  Ignored for the reverse
    /// direction.
    #[must_use]
    pub fn style(mut self, path: &'a Path) -> Self {
        self.style = Some(path);
        self
    }

    /// Allow overwriting an existing output file.
    ///
    /// When `false` (the default) [`execute`](Self::execute) returns
    /// [`Hwp2MdError::OutputExists`] if the output path already exists.
    /// Set to `true` to permit overwriting.
    #[must_use]
    pub fn force(mut self, enabled: bool) -> Self {
        self.force = enabled;
        self
    }

    /// Execute the conversion described by this builder.
    ///
    /// # Errors
    ///
    /// - [`Hwp2MdError::UnsupportedFormat`] — unknown extension pair.
    /// - [`Hwp2MdError::OutputExists`] — output exists and `force` is `false`.
    /// - [`Hwp2MdError::Io`] — file read/write failure.
    /// - [`Hwp2MdError::HwpParse`] / [`Hwp2MdError::HwpxParse`] — parse error
    ///   in the input document.
    /// - [`Hwp2MdError::HwpxWrite`] — error while generating the HWPX output.
    pub fn execute(self) -> Result<(), Hwp2MdError> {
        if !self.force && self.output.exists() {
            return Err(Hwp2MdError::OutputExists {
                path: self.output.to_path_buf(),
            });
        }

        let in_ext = self
            .input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let out_ext = self
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match (classify_format(&in_ext), classify_format(&out_ext)) {
            (FormatKind::Hwp | FormatKind::Hwpx, FormatKind::Markdown) => to_markdown(
                self.input,
                Some(self.output),
                self.assets_dir,
                self.frontmatter,
            ),
            (FormatKind::Markdown, FormatKind::Hwpx) => {
                to_hwpx(self.input, Some(self.output), self.style)
            }
            _ => Err(Hwp2MdError::UnsupportedFormat(format!(
                "cannot infer conversion direction from .{in_ext} -> .{out_ext}; \
                 expected .hwp/.hwpx -> .md/.markdown or .md/.markdown -> .hwpx"
            ))),
        }
    }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "convert_tests_count.rs"]
mod tests_count;

#[cfg(test)]
#[path = "convert_tests_builder.rs"]
mod tests_builder;
