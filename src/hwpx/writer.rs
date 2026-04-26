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
// Image asset collection
// ---------------------------------------------------------------------------

/// Source classification for an image `src` string.
#[derive(Debug)]
enum ImageSource<'a> {
    /// A local file path (not a URL, not a data URI).
    FilePath(&'a str),
    /// A `data:image/<subtype>;base64,<data>` URI.
    DataUri {
        /// The MIME sub-type, e.g. `"png"`.
        subtype: &'a str,
        /// The raw base64 payload (without the header prefix).
        payload: &'a str,
    },
    /// An HTTP or HTTPS URL — not embedded.
    RemoteUrl,
}

/// Classify a raw `src` string into one of the three [`ImageSource`] variants.
fn classify_image_src(src: &str) -> ImageSource<'_> {
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageSource::RemoteUrl;
    }
    if let Some(rest) = src.strip_prefix("data:image/") {
        // Expected format: `data:image/<subtype>;base64,<payload>`
        if let Some((subtype_and_enc, payload)) = rest.split_once(',') {
            if let Some(subtype) = subtype_and_enc.strip_suffix(";base64") {
                return ImageSource::DataUri { subtype, payload };
            }
        }
    }
    ImageSource::FilePath(src)
}

/// Infer the MIME type for a file path based solely on its extension.
///
/// Falls back to `"application/octet-stream"` for unknown extensions.
fn mime_from_extension(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// A resolved image asset ready to be written into the HWPX ZIP.
struct ResolvedAsset {
    /// The bare filename used as the BinData entry (e.g. `"photo.png"`).
    entry_name: String,
    /// Raw bytes of the image.
    data: Vec<u8>,
    /// MIME type (e.g. `"image/png"`).
    ///
    /// Stored for consumers of [`collect_image_assets`] (e.g. tests and
    /// future round-trip extensions) even though the OWPML `<hp:binData>`
    /// element does not carry a MIME attribute in the current schema.
    #[allow(dead_code)]
    mime_type: String,
}

/// Scan every `Block::Image` in `doc`, resolve file-path and data-URI sources,
/// and return:
///
/// 1. An [`ImageAssetMap`] mapping original `src` → bare `entry_name`.
/// 2. A `Vec<ResolvedAsset>` of all assets that must be written to `BinData/`.
///
/// HTTP/HTTPS URLs are skipped silently (they stay as-is in the `binaryItemIDRef`).
/// File paths that cannot be read are logged as warnings and omitted from the map
/// so that the caller can still produce a valid (if incomplete) HWPX.
///
/// Assets already present in `doc.assets` are also included so that documents
/// produced by the HWPX reader (which pre-populates `doc.assets`) continue to
/// roundtrip correctly.
fn collect_image_assets(doc: &ir::Document) -> (ImageAssetMap, Vec<ResolvedAsset>) {
    let mut asset_map: ImageAssetMap = HashMap::new();
    let mut resolved: Vec<ResolvedAsset> = Vec::new();
    let mut counter: u32 = 0;

    // Pre-populate from doc.assets (already-resolved assets from an HWPX reader
    // roundtrip).  Use the asset's bare filename as the entry name.
    for asset in &doc.assets {
        let entry_name = std::path::Path::new(&asset.name)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                counter += 1;
                format!("asset_{counter}")
            });
        // Only register if not already mapped (src key == asset.name for
        // pre-existing assets coming from the reader).
        asset_map
            .entry(asset.name.clone())
            .or_insert_with(|| entry_name.clone());
        // Avoid duplicate entries in resolved.
        if !resolved.iter().any(|r| r.entry_name == entry_name) {
            resolved.push(ResolvedAsset {
                entry_name,
                data: asset.data.clone(),
                mime_type: asset.mime_type.clone(),
            });
        }
    }

    // Walk all blocks to find Image sources that were not already covered.
    for section in &doc.sections {
        collect_images_from_blocks(&section.blocks, &mut asset_map, &mut resolved, &mut counter);
    }

    (asset_map, resolved)
}

/// Recursively walk `blocks` and resolve any `Block::Image` whose `src` has
/// not already been added to `asset_map`.
fn collect_images_from_blocks(
    blocks: &[ir::Block],
    asset_map: &mut ImageAssetMap,
    resolved: &mut Vec<ResolvedAsset>,
    counter: &mut u32,
) {
    for block in blocks {
        match block {
            ir::Block::Image { src, .. } => {
                if asset_map.contains_key(src.as_str()) {
                    continue;
                }
                match classify_image_src(src) {
                    ImageSource::RemoteUrl => {
                        // Leave remote URLs as-is; no embedding.
                    }
                    ImageSource::DataUri { subtype, payload } => match base64_decode(payload) {
                        Ok(bytes) => {
                            *counter += 1;
                            let entry_name = format!("image_{counter}.{subtype}");
                            let mime = format!("image/{subtype}");
                            asset_map.insert(src.clone(), entry_name.clone());
                            resolved.push(ResolvedAsset {
                                entry_name,
                                data: bytes,
                                mime_type: mime,
                            });
                        }
                        Err(e) => {
                            tracing::warn!("Failed to decode data URI for image: {e}");
                        }
                    },
                    ImageSource::FilePath(path) => {
                        match std::fs::read(path) {
                            Ok(bytes) => {
                                let entry_name = std::path::Path::new(path)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| {
                                        *counter += 1;
                                        format!("image_{counter}")
                                    });
                                let mime = mime_from_extension(path).to_owned();
                                asset_map.insert(src.clone(), entry_name.clone());
                                // Avoid duplicate entry_name when multiple srcs
                                // resolve to the same filename.
                                if !resolved.iter().any(|r| r.entry_name == entry_name) {
                                    resolved.push(ResolvedAsset {
                                        entry_name,
                                        data: bytes,
                                        mime_type: mime,
                                    });
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Cannot read image file {path:?}: {e}");
                            }
                        }
                    }
                }
            }
            ir::Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_images_from_blocks(&cell.blocks, asset_map, resolved, counter);
                    }
                }
            }
            ir::Block::BlockQuote { blocks: inner }
            | ir::Block::Footnote { content: inner, .. } => {
                collect_images_from_blocks(inner, asset_map, resolved, counter);
            }
            ir::Block::List { items, .. } => {
                for item in items {
                    collect_images_from_blocks(&item.blocks, asset_map, resolved, counter);
                }
            }
            ir::Block::Heading { .. }
            | ir::Block::Paragraph { .. }
            | ir::Block::CodeBlock { .. }
            | ir::Block::HorizontalRule
            | ir::Block::Math { .. } => {}
        }
    }
}

/// Minimal base64 decoder (standard alphabet, ignores whitespace).
///
/// No external crate is required — the `base64` crate is not in the dependency
/// tree.  The implementation handles the standard RFC 4648 alphabet and padding.
fn base64_decode(input: &str) -> Result<Vec<u8>, Hwp2MdError> {
    const TABLE: [i8; 256] = {
        let mut t = [-1i8; 256];
        let mut i = 0usize;
        // A-Z
        while i < 26 {
            t[b'A' as usize + i] = i as i8;
            i += 1;
        }
        // a-z
        i = 0;
        while i < 26 {
            t[b'a' as usize + i] = (i + 26) as i8;
            i += 1;
        }
        // 0-9
        i = 0;
        while i < 10 {
            t[b'0' as usize + i] = (i + 52) as i8;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let input = input.trim();
    // Strip trailing '=' padding for length calculation.
    let pad = input.bytes().rev().take(2).filter(|&b| b == b'=').count();
    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();

    if clean.len() % 4 != 0 && (clean.len() + pad) % 4 != 0 {
        // Allow length not divisible by 4 if padding was stripped.
    }

    let mut out = Vec::with_capacity(clean.len() * 3 / 4 + 2);
    let mut i = 0;
    while i + 3 < clean.len() {
        let [a, b, c, d] = [
            TABLE[clean[i] as usize],
            TABLE[clean[i + 1] as usize],
            TABLE[clean[i + 2] as usize],
            TABLE[clean[i + 3] as usize],
        ];
        if a < 0 || b < 0 || c < 0 || d < 0 {
            return Err(Hwp2MdError::HwpxWrite(
                "invalid base64 character in data URI".into(),
            ));
        }
        let n = (a as u32) << 18 | (b as u32) << 12 | (c as u32) << 6 | (d as u32);
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
        i += 4;
    }
    // Handle remaining 2- or 3-byte groups (with stripped padding).
    let rem = clean.len() - i;
    if rem == 2 {
        let [a, b] = [TABLE[clean[i] as usize], TABLE[clean[i + 1] as usize]];
        if a < 0 || b < 0 {
            return Err(Hwp2MdError::HwpxWrite(
                "invalid base64 character in data URI".into(),
            ));
        }
        let n = (a as u32) << 18 | (b as u32) << 12;
        out.push((n >> 16) as u8);
    } else if rem == 3 {
        let [a, b, c] = [
            TABLE[clean[i] as usize],
            TABLE[clean[i + 1] as usize],
            TABLE[clean[i + 2] as usize],
        ];
        if a < 0 || b < 0 || c < 0 {
            return Err(Hwp2MdError::HwpxWrite(
                "invalid base64 character in data URI".into(),
            ));
        }
        let n = (a as u32) << 18 | (b as u32) << 12 | (c as u32) << 6;
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
    }

    Ok(out)
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
    let (asset_map, resolved_assets) = collect_image_assets(doc);

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
    zip.write_all(generate_content_hpf(doc, &resolved_assets).as_bytes())?;

    for (i, sec) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(section::generate_section_xml(sec, i, &tables, &asset_map)?.as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section { blocks: Vec::new() };
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

fn generate_content_hpf(doc: &ir::Document, resolved_assets: &[ResolvedAsset]) -> String {
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

    // Emit <hp:binData> manifest entries for each embedded asset.
    //
    // The OWPML packageInfo schema expects:
    //   <hp:binData itemId="<basename_no_ext>" file="BinData/<filename>"
    //               type="EMBED" compress="true"/>
    //
    // The `itemId` is the bare stem (without extension) so that readers can
    // locate the entry when they see a `binaryItemIDRef` in section XML.
    let mut bin_data_entries = String::new();
    for asset in resolved_assets {
        let item_id = std::path::Path::new(&asset.entry_name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| asset.entry_name.clone());
        bin_data_entries.push_str(&format!(
            "    <hp:binData itemId=\"{}\" file=\"BinData/{}\" type=\"EMBED\" compress=\"true\"/>\n",
            xml_escape_content(&item_id),
            xml_escape_content(&asset.entry_name),
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hp:HWPMLPackage xmlns:hp="http://www.hancom.co.kr/hwpml/2011/packageInfo">
  <hp:compatibledocument version="1.1"/>
{doc_info}  <hp:contents>
{items}  </hp:contents>
{bin_data_entries}</hp:HWPMLPackage>"#
    )
}

/// Minimal XML escaping for text nodes.
fn xml_escape_content(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
