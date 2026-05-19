use crate::error::Hwp2MdError;
use crate::ir;
use crate::style::StyleTemplate;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[path = "writer_header.rs"]
mod header;

#[path = "writer_section.rs"]
mod section;

#[path = "writer_content.rs"]
mod content;

// Re-export submodule functions for test access.
#[cfg(test)]
use content::{
    base64_decode, collect_image_assets, generate_content_hpf, mime_from_extension,
    xml_escape_content,
};
#[cfg(test)]
use section::generate_section_xml;

// ---------------------------------------------------------------------------
// Image asset map
// ---------------------------------------------------------------------------

/// Maps an image's original `src` string to the bare filename used as the
/// `BinData/` entry inside the HWPX ZIP (e.g. `"photo.png"`).
///
/// The map is built once from the complete document IR before writing begins,
/// so that both `write_hwpx` (ZIP entry creation) and `generate_section_xml`
/// (binaryItemIDRef emission) can reference the same resolved names.
pub(crate) type ImageAssetMap = HashMap<String, String>;

// ---------------------------------------------------------------------------
// Font / character-property tables
// ---------------------------------------------------------------------------

const DEFAULT_FONT: &str = "\u{BC14}\u{D0D5}"; // 바탕
const DEFAULT_CODE_FONT: &str = "Courier New";
pub(crate) const LANG_SLOTS: [&str; 7] = [
    "HANGUL", "LATIN", "HANJA", "JAPANESE", "OTHER", "SYMBOL", "USER",
];

/// A unique character-formatting combination extracted from the document IR.
///
/// `id=0` is always the plain/default entry (all fields false, no color, no
/// custom font).  Additional entries are assigned IDs starting at 1.
// Bool fields represent character property flags from the HWP spec (bold, italic, etc.).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CharPrKey {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) code: bool,
    pub(crate) superscript: bool,
    pub(crate) subscript: bool,
    pub(crate) color: Option<String>,
    pub(crate) font_name: Option<String>,
    pub(crate) height: u32,
}

const HEADING_HEIGHTS: [u32; 7] = [1000, 2400, 2000, 1600, 1400, 1200, 1000];

impl CharPrKey {
    fn plain() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            code: false,
            superscript: false,
            subscript: false,
            color: None,
            font_name: None,
            height: 1000,
        }
    }

    fn from_inline(inline: &ir::Inline, code_font: &str) -> Self {
        let font_name = if inline.code {
            Some(code_font.to_owned())
        } else {
            inline.font_name.clone()
        };

        Self {
            bold: inline.bold,
            italic: inline.italic,
            underline: inline.underline,
            strikethrough: inline.strikethrough,
            code: inline.code,
            superscript: inline.superscript,
            subscript: inline.subscript,
            color: inline.color.clone(),
            font_name,
            height: 1000,
        }
    }

    fn code_block(code_font: &str) -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            code: true,
            superscript: false,
            subscript: false,
            color: None,
            font_name: Some(code_font.to_owned()),
            height: 1000,
        }
    }

    fn heading(level: u8) -> Self {
        let idx = (level as usize).clamp(1, 6);
        Self {
            bold: true,
            italic: false,
            underline: false,
            strikethrough: false,
            code: false,
            superscript: false,
            subscript: false,
            color: None,
            font_name: None,
            height: HEADING_HEIGHTS[idx],
        }
    }
}

/// Collected reference tables built from a complete document scan.
pub(crate) struct RefTables {
    /// Maps `CharPrKey` -> sequential ID (0 = default/plain).
    pub(crate) char_pr_ids: HashMap<CharPrKey, u32>,
    /// Unique font names in document order (first seen wins for ordering).
    pub(crate) font_names: Vec<String>,
    /// The default no-border fill entry ID (always 1, used by charPr).
    pub(crate) border_fill_id: u32,
    /// The solid-border fill entry ID (always 2, referenced by table cells).
    pub(crate) table_border_fill_id: u32,
    /// Resolved code font name (from style template or default).
    pub(crate) code_font: String,
    /// User-supplied style template (for page layout, heading spacing, etc.).
    pub(crate) style: Option<StyleTemplate>,
}

impl RefTables {
    fn build(doc: &ir::Document, style: Option<StyleTemplate>) -> Self {
        let mut char_pr_ids: HashMap<CharPrKey, u32> = HashMap::new();
        let mut font_names: Vec<String> = Vec::new();
        let mut font_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        // id=0 is always the plain entry.
        char_pr_ids.insert(CharPrKey::plain(), 0);

        // Resolve default font from style template or built-in default.
        let default_font = style
            .as_ref()
            .and_then(|s| s.font.default.as_deref())
            .unwrap_or(DEFAULT_FONT);
        font_set.insert(default_font.to_owned());
        font_names.push(default_font.to_owned());

        let code_font = style
            .as_ref()
            .and_then(|s| s.font.code.as_deref())
            .unwrap_or(DEFAULT_CODE_FONT)
            .to_owned();

        let mut next_id: u32 = 1;

        for section in &doc.sections {
            collect_from_blocks(
                &section.blocks,
                &mut char_pr_ids,
                &mut next_id,
                &mut font_names,
                &mut font_set,
                &code_font,
            );
        }

        for level in 1..=6u8 {
            let key = CharPrKey::heading(level);
            if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(key) {
                e.insert(next_id);
                next_id += 1;
            }
        }

        let code_key = CharPrKey::code_block(&code_font);
        if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(code_key) {
            e.insert(next_id);
            next_id += 1;
        }
        if font_set.insert(code_font.clone()) {
            font_names.push(code_font.clone());
        }
        let _ = next_id;

        Self {
            char_pr_ids,
            font_names,
            border_fill_id: 1,
            table_border_fill_id: 2,
            code_font,
            style,
        }
    }

    fn code_block_char_pr_id(&self) -> u32 {
        self.char_pr_id(&CharPrKey::code_block(&self.code_font))
    }

    fn heading_char_pr_id(&self, level: u8) -> u32 {
        self.char_pr_id(&CharPrKey::heading(level))
    }

    fn char_pr_id(&self, key: &CharPrKey) -> u32 {
        *self.char_pr_ids.get(key).unwrap_or(&0)
    }
}

fn collect_from_blocks(
    blocks: &[ir::Block],
    char_pr_ids: &mut HashMap<CharPrKey, u32>,
    next_id: &mut u32,
    font_names: &mut Vec<String>,
    font_set: &mut std::collections::HashSet<String>,
    code_font: &str,
) {
    for block in blocks {
        match block {
            ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
                collect_from_inlines(
                    inlines,
                    char_pr_ids,
                    next_id,
                    font_names,
                    font_set,
                    code_font,
                );
            }
            ir::Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_from_blocks(
                            &cell.blocks,
                            char_pr_ids,
                            next_id,
                            font_names,
                            font_set,
                            code_font,
                        );
                    }
                }
            }
            ir::Block::BlockQuote { blocks }
            | ir::Block::Footnote {
                content: blocks, ..
            } => {
                collect_from_blocks(
                    blocks,
                    char_pr_ids,
                    next_id,
                    font_names,
                    font_set,
                    code_font,
                );
            }
            ir::Block::List { items, .. } => {
                for item in items {
                    collect_from_blocks(
                        &item.blocks,
                        char_pr_ids,
                        next_id,
                        font_names,
                        font_set,
                        code_font,
                    );
                }
            }
            ir::Block::CodeBlock { .. }
            | ir::Block::Image { .. }
            | ir::Block::HorizontalRule
            | ir::Block::PageBreak
            | ir::Block::Math { .. } => {}
        }
    }
}

fn collect_from_inlines(
    inlines: &[ir::Inline],
    char_pr_ids: &mut HashMap<CharPrKey, u32>,
    next_id: &mut u32,
    font_names: &mut Vec<String>,
    font_set: &mut std::collections::HashSet<String>,
    code_font: &str,
) {
    for inline in inlines {
        let key = CharPrKey::from_inline(inline, code_font);

        // Register the font from the resolved key (which overrides
        // font_name to CODE_FONT for inline code), not from the raw
        // IR inline.
        if let Some(font) = &key.font_name {
            if font_set.insert(font.clone()) {
                font_names.push(font.clone());
            }
        }

        if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(key) {
            e.insert(*next_id);
            *next_id += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Write an IR document to an HWPX (OWPML ZIP) file.
///
/// # Errors
///
/// Returns an error if the output file cannot be created, ZIP writing
/// fails, or the optional style template is invalid.
pub fn write_hwpx(
    doc: &ir::Document,
    output: &Path,
    style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let template = style.map(StyleTemplate::from_file).transpose()?;
    let tables = RefTables::build(doc, template);
    let (asset_map, resolved_assets) = content::collect_image_assets(doc);

    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(b"application/hwp+zip")?;

    zip.start_file("META-INF/container.xml", options)?;
    zip.write_all(content::generate_container_xml().as_bytes())?;

    zip.start_file("version.xml", options)?;
    zip.write_all(content::generate_version_xml().as_bytes())?;

    zip.start_file("Contents/header.xml", options)?;
    zip.write_all(header::generate_header_xml(doc, &tables)?.as_bytes())?;

    zip.start_file("Contents/content.hpf", options)?;
    zip.write_all(content::generate_content_hpf(doc, &resolved_assets).as_bytes())?;

    for (i, sec) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(section::generate_section_xml(sec, i, &tables, &asset_map)?.as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section {
            blocks: Vec::new(),
            page_layout: None,
            ..Default::default()
        };
        zip.write_all(
            section::generate_section_xml(&empty_section, 0, &tables, &asset_map)?.as_bytes(),
        )?;
    }

    // Write all resolved image assets into BinData/.
    for asset in &resolved_assets {
        let path = format!("BinData/{}", asset.entry_name);
        zip.start_file(&path, options)?;
        zip.write_all(&asset.data)?;
    }

    zip.finish()?;
    Ok(())
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
