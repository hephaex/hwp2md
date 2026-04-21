use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read;
use std::path::Path;

pub fn read_hwpx(path: &Path) -> Result<ir::Document, anyhow::Error> {
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Hwp2MdError::HwpxParse(format!("ZIP open: {e}")))?;

    let mut doc = ir::Document::new();

    if let Ok(metadata) = read_metadata(&mut archive) {
        doc.metadata = metadata;
    }

    let section_files = find_section_files(&mut archive);

    for section_path in &section_files {
        match read_section_xml(&mut archive, section_path) {
            Ok(section) => doc.sections.push(section),
            Err(e) => {
                tracing::warn!("Failed to read {section_path}: {e}");
            }
        }
    }

    let bin_files = find_bin_files(&mut archive);
    for bin_path in &bin_files {
        if let Ok(asset) = read_bin_asset(&mut archive, bin_path) {
            doc.assets.push(asset);
        }
    }

    Ok(doc)
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

fn parse_section_xml(xml: &str) -> Result<ir::Section, Hwp2MdError> {
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

#[derive(Default)]
struct ParseContext {
    in_paragraph: bool,
    in_run: bool,
    in_table: bool,
    in_cell: bool,
    current_text: String,
    current_inlines: Vec<ir::Inline>,
    current_bold: bool,
    current_italic: bool,
    current_underline: bool,
    current_strike: bool,
    heading_level: Option<u8>,
    table_rows: Vec<ir::TableRow>,
    current_row_cells: Vec<ir::TableCell>,
    cell_blocks: Vec<ir::Block>,
    cell_inlines: Vec<ir::Inline>,
    cell_text: String,
    col_count: usize,
    list_ordered: bool,
    in_list: bool,
    list_items: Vec<ir::ListItem>,
    equation_text: String,
    in_equation: bool,
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
        "charPr" | "hp:charPr" => {
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "bold" | "hp:bold" => ctx.current_bold = val == "true" || val == "1",
                    "italic" | "hp:italic" => ctx.current_italic = val == "true" || val == "1",
                    "underline" | "hp:underline" => {
                        ctx.current_underline = !val.is_empty() && val != "none" && val != "0"
                    }
                    "strikeout" | "hp:strikeout" => {
                        ctx.current_strike = !val.is_empty() && val != "none" && val != "0"
                    }
                    _ => {}
                }
            }
        }
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
        "equation" | "hp:equation" | "eqEdit" | "hp:eqEdit" => {
            ctx.in_equation = true;
            ctx.equation_text.clear();
        }
        _ => {}
    }
}

fn handle_end_element(local: &str, ctx: &mut ParseContext, section: &mut ir::Section) {
    match local {
        "p" | "hp:p" => {
            if ctx.in_cell {
                flush_cell_paragraph(ctx);
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
                if ctx.in_cell {
                    ctx.cell_inlines.push(inline);
                } else {
                    ctx.current_inlines.push(inline);
                }
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
                colspan: 1,
                rowspan: 1,
            });
            ctx.in_cell = false;
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
        _ => {}
    }
}

fn handle_text(text: &str, ctx: &mut ParseContext) {
    if ctx.in_equation {
        ctx.equation_text.push_str(text);
        return;
    }
    if ctx.in_run {
        if ctx.in_cell {
            ctx.cell_text.push_str(text);
        } else {
            ctx.current_text.push_str(text);
        }
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
                if ctx.in_cell {
                    ctx.cell_blocks.push(ir::Block::Image { src, alt });
                } else {
                    section.blocks.push(ir::Block::Image { src, alt });
                }
            }
        }
        "lineBreak" | "hp:lineBreak" => {
            if ctx.in_cell {
                ctx.cell_text.push('\n');
            } else {
                ctx.current_text.push('\n');
            }
        }
        _ => {}
    }
}

fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    if !ctx.current_text.is_empty() {
        let text = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            ..ir::Inline::default()
        });
    }

    if ctx.current_inlines.is_empty() {
        return;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);
    let block = if let Some(level) = ctx.heading_level {
        ir::Block::Heading { level, inlines }
    } else {
        ir::Block::Paragraph { inlines }
    };
    section.blocks.push(block);
}

fn flush_cell_paragraph(ctx: &mut ParseContext) {
    if !ctx.cell_text.is_empty() {
        let text = std::mem::take(&mut ctx.cell_text);
        ctx.cell_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            ..ir::Inline::default()
        });
    }

    if !ctx.cell_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.cell_inlines);
        ctx.cell_blocks.push(ir::Block::Paragraph { inlines });
    }
}

pub(crate) fn parse_heading_style(style_ref: &str) -> Option<u8> {
    let lower = style_ref.to_lowercase();
    if lower.contains("heading") || lower.contains("제목") || lower.contains("개요") {
        // Extract the trailing number so "Heading12" → 12, "제목 10" → 10
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
mod tests {
    use super::*;

    #[test]
    fn parse_heading_style_heading1() {
        assert_eq!(parse_heading_style("Heading1"), Some(1));
    }

    #[test]
    fn parse_heading_style_heading6() {
        assert_eq!(parse_heading_style("Heading6"), Some(6));
    }

    #[test]
    fn parse_heading_style_korean_title() {
        // "제목1" → 1
        assert_eq!(parse_heading_style("제목1"), Some(1));
    }

    #[test]
    fn parse_heading_style_korean_outline_3() {
        // "개요3" → 3
        assert_eq!(parse_heading_style("개요3"), Some(3));
    }

    #[test]
    fn parse_heading_style_normal_is_none() {
        assert_eq!(parse_heading_style("Normal"), None);
    }

    #[test]
    fn parse_heading_style_body_text_is_none() {
        assert_eq!(parse_heading_style("BodyText"), None);
    }

    #[test]
    fn parse_heading_style_heading_no_digit_defaults_to_1() {
        // "Heading" without a trailing digit → defaults to level 1.
        assert_eq!(parse_heading_style("Heading"), Some(1));
    }

    #[test]
    fn parse_heading_style_case_insensitive() {
        assert_eq!(parse_heading_style("HEADING2"), Some(2));
    }
}
