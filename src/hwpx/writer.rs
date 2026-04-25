use crate::error::Hwp2MdError;
use crate::ir;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[path = "writer_header.rs"]
mod header;

#[path = "writer_section.rs"]
mod section;

// Re-export submodule functions for test access.
#[cfg(test)]
use section::generate_section_xml;

// ---------------------------------------------------------------------------
// Font / character-property tables
// ---------------------------------------------------------------------------

const DEFAULT_FONT: &str = "\u{BC14}\u{D0D5}"; // 바탕
pub(crate) const CODE_FONT: &str = "Courier New";
pub(crate) const LANG_SLOTS: [&str; 7] = [
    "HANGUL", "LATIN", "HANJA", "JAPANESE", "OTHER", "SYMBOL", "USER",
];

/// A unique character-formatting combination extracted from the document IR.
///
/// `id=0` is always the plain/default entry (all fields false, no color, no
/// custom font).  Additional entries are assigned IDs starting at 1.
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

    fn from_inline(inline: &ir::Inline) -> Self {
        // When the inline is marked as code, force monospace font regardless
        // of any other font_name the inline may carry.
        let font_name = if inline.code {
            Some(CODE_FONT.to_owned())
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

    fn code_block() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            code: true,
            superscript: false,
            subscript: false,
            color: None,
            font_name: Some(CODE_FONT.to_owned()),
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
    /// The single default borderFill entry ID (always 1).
    pub(crate) border_fill_id: u32,
}

impl RefTables {
    fn build(doc: &ir::Document) -> Self {
        let mut char_pr_ids: HashMap<CharPrKey, u32> = HashMap::new();
        let mut font_names: Vec<String> = Vec::new();
        let mut font_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        // id=0 is always the plain entry.
        char_pr_ids.insert(CharPrKey::plain(), 0);

        // Collect default font as id=0 font.
        font_set.insert(DEFAULT_FONT.to_owned());
        font_names.push(DEFAULT_FONT.to_owned());

        let mut next_id: u32 = 1;

        for section in &doc.sections {
            collect_from_blocks(
                &section.blocks,
                &mut char_pr_ids,
                &mut next_id,
                &mut font_names,
                &mut font_set,
            );
        }

        // Register heading charPr entries (one per level 1-6).
        for level in 1..=6u8 {
            let key = CharPrKey::heading(level);
            if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(key) {
                e.insert(next_id);
                next_id += 1;
            }
        }

        // Always register the code-block monospace entry.
        let code_key = CharPrKey::code_block();
        if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(code_key) {
            e.insert(next_id);
            next_id += 1;
        }
        if font_set.insert(CODE_FONT.to_owned()) {
            font_names.push(CODE_FONT.to_owned());
        }
        let _ = next_id;

        Self {
            char_pr_ids,
            font_names,
            border_fill_id: 1,
        }
    }

    fn code_block_char_pr_id(&self) -> u32 {
        self.char_pr_id(&CharPrKey::code_block())
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
) {
    for block in blocks {
        match block {
            ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
                collect_from_inlines(inlines, char_pr_ids, next_id, font_names, font_set);
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
                        );
                    }
                }
            }
            ir::Block::BlockQuote { blocks }
            | ir::Block::Footnote {
                content: blocks, ..
            } => {
                collect_from_blocks(blocks, char_pr_ids, next_id, font_names, font_set);
            }
            ir::Block::List { items, .. } => {
                for item in items {
                    collect_from_blocks(&item.blocks, char_pr_ids, next_id, font_names, font_set);
                }
            }
            ir::Block::CodeBlock { .. }
            | ir::Block::Image { .. }
            | ir::Block::HorizontalRule
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
) {
    for inline in inlines {
        let key = CharPrKey::from_inline(inline);

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

pub fn write_hwpx(
    doc: &ir::Document,
    output: &Path,
    _style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let tables = RefTables::build(doc);

    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(b"application/hwp+zip")?;

    zip.start_file("META-INF/container.xml", options)?;
    zip.write_all(generate_container_xml().as_bytes())?;

    zip.start_file("version.xml", options)?;
    zip.write_all(generate_version_xml().as_bytes())?;

    zip.start_file("Contents/header.xml", options)?;
    zip.write_all(header::generate_header_xml(doc, &tables)?.as_bytes())?;

    zip.start_file("Contents/content.hpf", options)?;
    zip.write_all(generate_content_hpf(doc).as_bytes())?;

    for (i, sec) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(section::generate_section_xml(sec, i, &tables)?.as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section { blocks: Vec::new() };
        zip.write_all(section::generate_section_xml(&empty_section, 0, &tables)?.as_bytes())?;
    }

    for (i, asset) in doc.assets.iter().enumerate() {
        let safe_name = std::path::Path::new(&asset.name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("asset_{i}"));
        let path = format!("BinData/{safe_name}");
        zip.start_file(&path, options)?;
        zip.write_all(&asset.data)?;
    }

    zip.finish()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Static file generators
// ---------------------------------------------------------------------------

fn generate_container_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0">
  <rootfiles>
    <rootfile full-path="Contents/content.hpf" media-type="application/hwp+xml"/>
  </rootfiles>
</container>"#
        .to_string()
}

fn generate_version_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<hh:HWPCompatibleDocument xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" version="1.1">
  <hh:DocInfo>
    <hh:HWPVersion Major="5" Minor="1" Micro="0" Build="0"/>
  </hh:DocInfo>
</hh:HWPCompatibleDocument>"#
        .to_string()
}

// ---------------------------------------------------------------------------
// content.hpf
// ---------------------------------------------------------------------------

fn generate_content_hpf(doc: &ir::Document) -> String {
    let section_count = doc.sections.len().max(1);
    let mut items = String::new();
    for i in 0..section_count {
        items.push_str(&format!(
            "    <hp:item href=\"section{i}.xml\" type=\"Section\"/>\n"
        ));
    }

    // Build optional <hp:docInfo> with title/author metadata.
    let has_title = doc.metadata.title.as_ref().is_some_and(|t| !t.is_empty());
    let has_author = doc.metadata.author.as_ref().is_some_and(|a| !a.is_empty());
    let doc_info = if has_title || has_author {
        let mut info = String::from("  <hp:docInfo>\n");
        if let Some(title) = doc.metadata.title.as_deref() {
            if !title.is_empty() {
                info.push_str(&format!(
                    "    <hp:title>{}</hp:title>\n",
                    xml_escape_content(title)
                ));
            }
        }
        if let Some(author) = doc.metadata.author.as_deref() {
            if !author.is_empty() {
                info.push_str(&format!(
                    "    <hp:author>{}</hp:author>\n",
                    xml_escape_content(author)
                ));
            }
        }
        info.push_str("  </hp:docInfo>\n");
        info
    } else {
        String::new()
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hp:HWPMLPackage xmlns:hp="http://www.hancom.co.kr/hwpml/2011/packageInfo">
  <hp:compatibledocument version="1.1"/>
{doc_info}  <hp:contents>
{items}  </hp:contents>
</hp:HWPMLPackage>"#
    )
}

/// Minimal XML content escaping for text nodes.
fn xml_escape_content(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
