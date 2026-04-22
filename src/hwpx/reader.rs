use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[path = "context.rs"]
mod context;
pub(crate) use context::{
    apply_charpr_attrs, flush_cell_paragraph, flush_footnote_paragraph, flush_list_item_paragraph,
    flush_paragraph, ParseContext,
};

pub fn read_hwpx(path: &Path) -> Result<ir::Document, anyhow::Error> {
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

fn handle_start_element(local: &str, e: &quick_xml::events::BytesStart, ctx: &mut ParseContext) {
    match local {
        "p" | "hp:p" => {
            ctx.in_paragraph = true;
            ctx.current_text.clear();
            ctx.current_inlines.clear();
            ctx.heading_level = None;

            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "styleIDRef" || key == "hp:styleIDRef" {
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    if let Some(level) = parse_heading_style(&val) {
                        ctx.heading_level = Some(level);
                    }
                }
            }
        }
        "run" | "hp:run" => {
            ctx.in_run = true;
            ctx.current_bold = false;
            ctx.current_italic = false;
            ctx.current_underline = false;
            ctx.current_strike = false;
        }
        "charPr" | "hp:charPr" => apply_charpr_attrs(e, ctx),
        "t" | "hp:t" => {}
        "tbl" | "hp:tbl" => {
            ctx.in_table = true;
            ctx.table_rows.clear();
            ctx.col_count = 0;
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "colCnt" || key == "hp:colCnt" {
                    if let Ok(n) = attr.unescape_value().unwrap_or_default().parse::<usize>() {
                        ctx.col_count = n;
                    }
                }
            }
        }
        "tr" | "hp:tr" => {
            ctx.current_row_cells.clear();
        }
        "tc" | "hp:tc" => {
            ctx.in_cell = true;
            ctx.cell_blocks.clear();
            ctx.cell_inlines.clear();
            ctx.cell_text.clear();
            ctx.current_colspan = 1;
            ctx.current_rowspan = 1;
        }
        "ol" => {
            ctx.in_list = true;
            ctx.list_ordered = true;
            ctx.list_items.clear();
        }
        "ul" => {
            ctx.in_list = true;
            ctx.list_ordered = false;
            ctx.list_items.clear();
        }
        "li" | "hp:li" => {
            ctx.in_list_item = true;
            ctx.list_item_blocks.clear();
            ctx.list_item_inlines.clear();
            ctx.list_item_text.clear();
        }
        "equation" | "hp:equation" | "eqEdit" | "hp:eqEdit" => {
            ctx.in_equation = true;
            ctx.equation_text.clear();
        }
        "fn" | "hp:fn" | "footnote" | "hp:footnote" | "en" | "hp:en" | "endnote" | "hp:endnote" => {
            let mut id = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "id" || key == "hp:id" || key == "noteId" || key == "hp:noteId" {
                    id = attr.unescape_value().unwrap_or_default().to_string();
                    break;
                }
            }
            ctx.in_footnote = true;
            ctx.footnote_id = id;
            ctx.footnote_blocks.clear();
            ctx.footnote_inlines.clear();
            ctx.footnote_text.clear();
        }
        _ => {}
    }
}

fn handle_end_element(local: &str, ctx: &mut ParseContext, section: &mut ir::Section) {
    match local {
        "p" | "hp:p" => {
            if ctx.in_footnote {
                flush_footnote_paragraph(ctx);
            } else if ctx.in_cell {
                flush_cell_paragraph(ctx);
            } else if ctx.in_list_item {
                flush_list_item_paragraph(ctx);
            } else {
                flush_paragraph(ctx, section);
            }
            ctx.in_paragraph = false;
        }
        "run" | "hp:run" => {
            ctx.in_run = false;
        }
        "t" | "hp:t" => {
            if !ctx.current_text.is_empty() {
                let text = std::mem::take(&mut ctx.current_text);
                let inline = ir::Inline {
                    text,
                    bold: ctx.current_bold,
                    italic: ctx.current_italic,
                    underline: ctx.current_underline,
                    strikethrough: ctx.current_strike,
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        "tbl" | "hp:tbl" => {
            let col_count = ctx.col_count.max(
                ctx.table_rows
                    .iter()
                    .map(|r| r.cells.len())
                    .max()
                    .unwrap_or(0),
            );
            if !ctx.table_rows.is_empty() {
                let rows = std::mem::take(&mut ctx.table_rows);
                section.blocks.push(ir::Block::Table { rows, col_count });
            }
            ctx.in_table = false;
        }
        "tr" | "hp:tr" => {
            let cells = std::mem::take(&mut ctx.current_row_cells);
            ctx.table_rows.push(ir::TableRow {
                cells,
                is_header: ctx.table_rows.is_empty(),
            });
        }
        "tc" | "hp:tc" => {
            flush_cell_paragraph(ctx);
            let blocks = std::mem::take(&mut ctx.cell_blocks);
            ctx.current_row_cells.push(ir::TableCell {
                blocks,
                colspan: ctx.current_colspan,
                rowspan: ctx.current_rowspan,
            });
            ctx.in_cell = false;
        }
        "li" | "hp:li" => {
            flush_list_item_paragraph(ctx);
            let blocks = std::mem::take(&mut ctx.list_item_blocks);
            ctx.list_items.push(ir::ListItem {
                blocks,
                children: Vec::new(),
            });
            ctx.in_list_item = false;
        }
        "ol" | "ul" => {
            if !ctx.list_items.is_empty() {
                let items = std::mem::take(&mut ctx.list_items);
                section.blocks.push(ir::Block::List {
                    ordered: ctx.list_ordered,
                    start: 1,
                    items,
                });
            }
            ctx.in_list = false;
        }
        "equation" | "hp:equation" | "eqEdit" | "hp:eqEdit" => {
            if !ctx.equation_text.is_empty() {
                let tex = std::mem::take(&mut ctx.equation_text);
                section.blocks.push(ir::Block::Math { display: true, tex });
            }
            ctx.in_equation = false;
        }
        "fn" | "hp:fn" | "footnote" | "hp:footnote" | "en" | "hp:en" | "endnote" | "hp:endnote" => {
            flush_footnote_paragraph(ctx);
            if !ctx.footnote_blocks.is_empty() {
                let id = std::mem::take(&mut ctx.footnote_id);
                let content = std::mem::take(&mut ctx.footnote_blocks);
                section.blocks.push(ir::Block::Footnote { id, content });
            } else {
                ctx.footnote_id.clear();
            }
            ctx.in_footnote = false;
        }
        _ => {}
    }
}

fn handle_text(text: &str, ctx: &mut ParseContext) {
    if ctx.in_equation {
        ctx.equation_text.push_str(text);
        return;
    }
    if ctx.in_run {
        ctx.active_text_buf().push_str(text);
    }
}

fn handle_empty_element(
    local: &str,
    e: &quick_xml::events::BytesStart,
    ctx: &mut ParseContext,
    section: &mut ir::Section,
) {
    match local {
        "img" | "hp:img" | "picture" | "hp:picture" => {
            let mut src = String::new();
            let mut alt = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "src" | "href" | "hp:href" | "binaryItemIDRef" | "hp:binaryItemIDRef" => {
                        src = val;
                    }
                    "alt" => alt = val,
                    _ => {}
                }
            }
            if !src.is_empty() {
                let img = ir::Block::Image { src, alt };
                if let Some(block) = ctx.push_block_scoped(img) {
                    section.blocks.push(block);
                }
            }
        }
        "lineBreak" | "hp:lineBreak" => {
            ctx.active_text_buf().push('\n');
        }
        // <hp:cellAddr colAddr="0" rowAddr="0" colSpan="2" rowSpan="1"/>
        // Appears as a self-closing child inside <hp:tc>.
        "cellAddr" | "hp:cellAddr" => {
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default();
                match key {
                    "colSpan" | "hp:colSpan" => {
                        if let Ok(n) = val.parse::<u32>() {
                            if n >= 1 {
                                ctx.current_colspan = n;
                            }
                        }
                    }
                    "rowSpan" | "hp:rowSpan" => {
                        if let Ok(n) = val.parse::<u32>() {
                            if n >= 1 {
                                ctx.current_rowspan = n;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // <hp:charPr bold="true" italic="true" .../>  (self-closing variant)
        // Delegates to apply_charpr_attrs -- same logic as the Start element path.
        "charPr" | "hp:charPr" => apply_charpr_attrs(e, ctx),
        // Footnote / endnote reference inline: a self-closing marker that records
        // which footnote the current text position cites.
        //
        // Accepted forms:
        //   <hp:noteRef noteId="1"/>
        //   <hp:ctrl id="fn" idRef="1"/>
        //   <hp:ctrl id="en" idRef="1"/>
        "noteRef" | "hp:noteRef" => {
            let mut note_id = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "noteId" || key == "hp:noteId" || key == "id" || key == "hp:id" {
                    note_id = attr.unescape_value().unwrap_or_default().to_string();
                    break;
                }
            }
            if !note_id.is_empty() {
                let inline = ir::Inline {
                    footnote_ref: Some(note_id),
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        // <hp:ctrl id="fn" idRef="1"/> -- HWP-binary-style ctrl inline.
        "ctrl" | "hp:ctrl" => {
            let mut ctrl_kind = String::new();
            let mut id_ref = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "id" | "hp:id" => ctrl_kind = val,
                    "idRef" | "hp:idRef" => id_ref = val,
                    _ => {}
                }
            }
            if (ctrl_kind == "fn" || ctrl_kind == "en") && !id_ref.is_empty() {
                let inline = ir::Inline {
                    footnote_ref: Some(id_ref),
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_heading_style(style_ref: &str) -> Option<u8> {
    let lower = style_ref.to_lowercase();
    if lower.contains("heading") || lower.contains("제목") || lower.contains("개요") {
        // Extract the trailing number so "Heading12" -> 12, "제목 10" -> 10
        let num_str: String = style_ref
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = num_str.parse::<u8>() {
            if (1..=6).contains(&n) {
                return Some(n);
            }
        }
        return Some(1);
    }
    None
}

fn read_zip_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &str,
) -> Result<String, Hwp2MdError> {
    let mut file = archive
        .by_name(path)
        .map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP entry '{path}': {e}")))?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn read_bin_asset(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &str,
) -> Result<ir::Asset, Hwp2MdError> {
    let mut file = archive
        .by_name(path)
        .map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP asset '{path}': {e}")))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

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
