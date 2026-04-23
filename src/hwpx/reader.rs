use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[path = "context.rs"]
mod context;
pub(crate) use context::{flush_paragraph, ParseContext};

#[path = "handlers.rs"]
mod handlers;
#[cfg(test)]
pub(crate) use handlers::parse_heading_style;
use handlers::{handle_empty_element, handle_end_element, handle_start_element, handle_text};

pub fn read_hwpx(path: &Path) -> Result<ir::Document, Hwp2MdError> {
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP open: {e}")))?;

    let mut doc = ir::Document::new();

    if let Ok(metadata) = read_metadata(&mut archive) {
        doc.metadata = metadata;
    }

    // Build the BinData ID -> full ZIP path map before parsing sections so that
    // binaryItemIDRef references can be resolved immediately.
    let bin_files = find_bin_files(&mut archive);
    let bin_map = build_bin_map(&bin_files);

    let section_files = find_section_files(&mut archive);

    for section_path in &section_files {
        match read_section_xml(&mut archive, section_path) {
            Ok(mut section) => {
                resolve_bin_refs(&mut section, &bin_map);
                doc.sections.push(section);
            }
            Err(e) => {
                tracing::warn!("Failed to read {section_path}: {e}");
            }
        }
    }

    for bin_path in &bin_files {
        if let Ok(asset) = read_bin_asset(&mut archive, bin_path) {
            doc.assets.push(asset);
        }
    }

    Ok(doc)
}

/// Build a map from bare BinData stem (e.g. `"BIN0001"`) to the full ZIP path
/// (e.g. `"BinData/BIN0001.png"`).
///
/// When a section XML references `binaryItemIDRef="BIN0001"`, the parser stores
/// `"BIN0001"` as the image src.  This map is used by [`resolve_bin_refs`] to
/// replace that bare ID with the real path so downstream consumers can locate
/// the asset.
fn build_bin_map(bin_files: &[String]) -> HashMap<String, String> {
    bin_files
        .iter()
        .filter_map(|path| {
            let filename = path.rsplit('/').next()?;
            let stem = filename
                .rsplit_once('.')
                .map(|(s, _)| s)
                .unwrap_or(filename);
            Some((stem.to_string(), path.clone()))
        })
        .collect()
}

/// Walk all blocks in `section` and replace any `Image { src }` whose `src`
/// equals a key in `bin_map` with the corresponding full path.
fn resolve_bin_refs(section: &mut ir::Section, bin_map: &HashMap<String, String>) {
    for block in &mut section.blocks {
        resolve_block_bin_refs(block, bin_map);
    }
}

fn resolve_block_bin_refs(block: &mut ir::Block, bin_map: &HashMap<String, String>) {
    match block {
        ir::Block::Image { src, .. } => {
            if let Some(full_path) = bin_map.get(src.as_str()) {
                *src = full_path.clone();
            }
        }
        ir::Block::Table { rows, .. } => {
            for row in rows {
                for cell in &mut row.cells {
                    for b in &mut cell.blocks {
                        resolve_block_bin_refs(b, bin_map);
                    }
                }
            }
        }
        ir::Block::Footnote { content, .. } => {
            for b in content {
                resolve_block_bin_refs(b, bin_map);
            }
        }
        ir::Block::List { items, .. } => {
            for item in items {
                for b in &mut item.blocks {
                    resolve_block_bin_refs(b, bin_map);
                }
            }
        }
        ir::Block::BlockQuote { blocks } => {
            for b in blocks {
                resolve_block_bin_refs(b, bin_map);
            }
        }
        ir::Block::Heading { .. }
        | ir::Block::Paragraph { .. }
        | ir::Block::CodeBlock { .. }
        | ir::Block::HorizontalRule
        | ir::Block::Math { .. } => {}
    }
}

fn read_metadata(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Result<ir::Metadata, Hwp2MdError> {
    let mut meta = ir::Metadata::default();

    let xml = read_zip_entry(archive, "Contents/header.xml")
        .or_else(|_| read_zip_entry(archive, "header.xml"))?;

    let mut reader = Reader::from_str(&xml);
    let mut buf = Vec::new();
    let mut in_title = false;
    let mut in_author = false;
    let mut in_subject = false;
    let mut in_description = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match name {
                    "title" => in_title = true,
                    "creator" | "author" => in_author = true,
                    "subject" => in_subject = true,
                    "description" => in_description = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_title {
                    meta.title = Some(text);
                    in_title = false;
                } else if in_author {
                    meta.author = Some(text);
                    in_author = false;
                } else if in_subject {
                    meta.subject = Some(text);
                    in_subject = false;
                } else if in_description {
                    meta.description = Some(text);
                    in_description = false;
                }
            }
            Ok(Event::End(_)) => {
                in_title = false;
                in_author = false;
                in_subject = false;
                in_description = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(meta)
}

fn find_section_files(archive: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let mut sections = Vec::new();

    if let Ok(manifest) = read_zip_entry(archive, "Contents/content.hpf")
        .or_else(|_| read_zip_entry(archive, "content.hpf"))
    {
        let mut reader = Reader::from_str(&manifest);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e) | Event::Start(e)) => {
                    let local_name = e.local_name();
                    let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                    if name == "item" || name == "hp:item" {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            if key == "href" || key == "hp:href" {
                                let val = attr.unescape_value().unwrap_or_default().to_string();
                                if val.contains("section") || val.contains("Section") {
                                    let full_path =
                                        if val.starts_with("Contents/") || val.starts_with('/') {
                                            val.trim_start_matches('/').to_string()
                                        } else {
                                            format!("Contents/{val}")
                                        };
                                    sections.push(full_path);
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    if sections.is_empty() {
        for i in 0..100 {
            let path = format!("Contents/section{i}.xml");
            if archive.by_name(&path).is_ok() {
                sections.push(path);
            } else {
                let path = format!("Contents/Section{i}.xml");
                if archive.by_name(&path).is_ok() {
                    sections.push(path);
                } else if i > 0 {
                    break;
                }
            }
        }
    }

    sections
}

fn find_bin_files(archive: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let mut bins = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            if name.starts_with("BinData/") || name.starts_with("Contents/BinData/") {
                bins.push(name);
            }
        }
    }
    bins
}

fn read_section_xml(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &str,
) -> Result<ir::Section, Hwp2MdError> {
    let xml = read_zip_entry(archive, path)?;
    parse_section_xml(&xml)
}

pub(crate) fn parse_section_xml(xml: &str) -> Result<ir::Section, Hwp2MdError> {
    let mut section = ir::Section { blocks: Vec::new() };
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut context = ParseContext::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                handle_start_element(local, e, &mut context);
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                handle_end_element(local, &mut context, &mut section);
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                handle_text(&text, &mut context);
            }
            Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                handle_empty_element(local, e, &mut context, &mut section);
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parse error: {e}");
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    flush_paragraph(&mut context, &mut section);

    Ok(section)
}

/// Maximum size for a single ZIP entry read from untrusted HWPX input (256 MB).
const MAX_ZIP_ENTRY: u64 = 256 * 1024 * 1024;

fn read_zip_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &str,
) -> Result<String, Hwp2MdError> {
    let file = archive
        .by_name(path)
        .map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP entry '{path}': {e}")))?;
    let mut bytes = Vec::new();
    file.take(MAX_ZIP_ENTRY).read_to_end(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP entry '{path}' not valid UTF-8: {e}")))
}

fn read_bin_asset(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &str,
) -> Result<ir::Asset, Hwp2MdError> {
    let file = archive
        .by_name(path)
        .map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP asset '{path}': {e}")))?;
    let mut data = Vec::new();
    file.take(MAX_ZIP_ENTRY).read_to_end(&mut data)?;

    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    let mime = guess_mime_from_name(&name);

    Ok(ir::Asset {
        name,
        data,
        mime_type: mime,
    })
}

fn guess_mime_from_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".bmp") {
        "image/bmp".to_string()
    } else if lower.ends_with(".svg") {
        "image/svg+xml".to_string()
    } else if lower.ends_with(".wmf") {
        "image/x-wmf".to_string()
    } else if lower.ends_with(".emf") {
        "image/x-emf".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
