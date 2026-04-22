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
    // colspan/rowspan for the cell currently being parsed
    current_colspan: u32,
    current_rowspan: u32,
    list_ordered: bool,
    in_list: bool,
    list_items: Vec<ir::ListItem>,
    equation_text: String,
    in_equation: bool,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            in_paragraph: false,
            in_run: false,
            in_table: false,
            in_cell: false,
            current_text: String::new(),
            current_inlines: Vec::new(),
            current_bold: false,
            current_italic: false,
            current_underline: false,
            current_strike: false,
            heading_level: None,
            table_rows: Vec::new(),
            current_row_cells: Vec::new(),
            cell_blocks: Vec::new(),
            cell_inlines: Vec::new(),
            cell_text: String::new(),
            col_count: 0,
            // A cell with no span attributes spans exactly 1 column and 1 row.
            current_colspan: 1,
            current_rowspan: 1,
            list_ordered: false,
            in_list: false,
            list_items: Vec::new(),
            equation_text: String::new(),
            in_equation: false,
        }
    }
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
                colspan: ctx.current_colspan,
                rowspan: ctx.current_rowspan,
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
        // Mirrors the same attribute logic in handle_start_element for <hp:charPr>.
        "charPr" | "hp:charPr" => {
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default();
                match key {
                    "bold" | "hp:bold" => {
                        ctx.current_bold = val.as_ref() == "true" || val.as_ref() == "1"
                    }
                    "italic" | "hp:italic" => {
                        ctx.current_italic = val.as_ref() == "true" || val.as_ref() == "1"
                    }
                    "underline" | "hp:underline" => {
                        ctx.current_underline =
                            !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
                    }
                    "strikeout" | "hp:strikeout" => {
                        ctx.current_strike =
                            !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
                    }
                    _ => {}
                }
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

    // -----------------------------------------------------------------------
    // Existing parse_heading_style tests
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // parse_section_xml — helper for asserting on the returned Section
    // -----------------------------------------------------------------------

    /// Unwrap the section and panic with a descriptive message on error.
    fn section(xml: &str) -> ir::Section {
        parse_section_xml(xml).expect("parse_section_xml must not fail")
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — empty / minimal documents
    // -----------------------------------------------------------------------

    #[test]
    fn empty_document_produces_no_blocks() {
        let s = section("<root/>");
        assert!(s.blocks.is_empty(), "expected no blocks for empty XML");
    }

    #[test]
    fn empty_paragraph_produces_no_blocks() {
        // A paragraph element with no run content must be silently dropped.
        let xml = r#"<root><hp:p></hp:p></root>"#;
        let s = section(xml);
        assert!(
            s.blocks.is_empty(),
            "empty paragraph must not produce a block"
        );
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — simple paragraph
    // -----------------------------------------------------------------------

    #[test]
    fn simple_paragraph_text() {
        // Compact XML — no whitespace text nodes between tags (matches real HWPX).
        let xml = r#"<root><hp:p><hp:run><hp:t>Hello World</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines.len(), 1);
                assert_eq!(inlines[0].text, "Hello World");
                assert!(!inlines[0].bold);
                assert!(!inlines[0].italic);
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn paragraph_without_hp_prefix() {
        // Bare element names (no namespace prefix) must also parse correctly.
        let xml = r#"<root><p><run><t>bare prefix</t></run></p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines[0].text, "bare prefix");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn multiple_runs_in_one_paragraph_produce_multiple_inlines() {
        let xml = r#"<root><hp:p><hp:run><hp:t>first</hp:t></hp:run><hp:run><hp:t>second</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines.len(), 2);
                assert_eq!(inlines[0].text, "first");
                assert_eq!(inlines[1].text, "second");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — heading via styleIDRef
    // -----------------------------------------------------------------------

    #[test]
    fn heading_level2_via_style_id_ref() {
        let xml = r#"<root><hp:p styleIDRef="Heading2"><hp:run><hp:t>Chapter title</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Heading { level, inlines } => {
                assert_eq!(*level, 2);
                assert_eq!(inlines[0].text, "Chapter title");
            }
            other => panic!("expected Heading, got {other:?}"),
        }
    }

    #[test]
    fn heading_level3_korean_style() {
        let xml =
            r#"<root><hp:p styleIDRef="제목3"><hp:run><hp:t>소제목</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Heading { level, inlines } => {
                assert_eq!(*level, 3);
                assert_eq!(inlines[0].text, "소제목");
            }
            other => panic!("expected Heading, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — bold / italic via Start-element charPr
    // -----------------------------------------------------------------------

    #[test]
    fn bold_text_via_charpr_start_element() {
        // Start-element charPr (non-self-closing) — handled by handle_start_element.
        let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="false"></hp:charPr><hp:t>bold text</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(inlines[0].bold, "inline must be bold");
                assert!(!inlines[0].italic, "inline must not be italic");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn italic_text_via_charpr_empty_element() {
        // Self-closing charPr — handled by handle_empty_element.
        let xml = r#"<root><hp:p><hp:run><hp:charPr bold="false" italic="true"/><hp:t>italic text</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(!inlines[0].bold, "inline must not be bold");
                assert!(inlines[0].italic, "inline must be italic");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn bold_and_italic_via_empty_charpr() {
        let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="true"/><hp:t>strong em</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(inlines[0].bold);
                assert!(inlines[0].italic);
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn charpr_numeric_1_means_bold() {
        // The parser accepts "1" as well as "true" for boolean attributes.
        let xml = r#"<root><hp:p><hp:run><hp:charPr bold="1"/><hp:t>numeric bold</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(inlines[0].bold, "bold=\"1\" must be treated as true");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn charpr_resets_between_runs() {
        // Second run has no charPr, so bold must revert to false.
        // Compact XML avoids spurious whitespace-only text nodes.
        let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true"/><hp:t>bold</hp:t></hp:run><hp:run><hp:t>plain</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines.len(), 2);
                assert!(inlines[0].bold, "first inline must be bold");
                // The run end event resets bold to false.
                assert!(!inlines[1].bold, "second inline must not be bold");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — lineBreak (empty element)
    // -----------------------------------------------------------------------

    #[test]
    fn line_break_appends_newline_to_inline_text() {
        // lineBreak is an empty element that appends \n to current_text.
        // It lives outside a <t> so flush_paragraph picks it up at paragraph end.
        let xml =
            r#"<root><hp:p><hp:run><hp:t>line one</hp:t><hp:lineBreak/></hp:run></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                let full: String = inlines.iter().map(|i| i.text.as_str()).collect();
                assert!(
                    full.contains('\n'),
                    "paragraph inlines must contain a newline from lineBreak; got: {full:?}"
                );
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — image (empty element)
    // -----------------------------------------------------------------------

    #[test]
    fn image_element_produces_image_block() {
        let xml = r#"<root>
            <hp:img src="image1.png" alt="photo"/>
        </root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Image { src, alt } => {
                assert_eq!(src, "image1.png");
                assert_eq!(alt, "photo");
            }
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn image_with_empty_src_is_ignored() {
        // An img element with no src attribute must not produce a block.
        let xml = r#"<root><hp:img alt="no src"/></root>"#;
        let s = section(xml);
        assert!(s.blocks.is_empty(), "img without src must be dropped");
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — equation
    // -----------------------------------------------------------------------

    #[test]
    fn equation_element_produces_math_block() {
        let xml = r#"<root><hp:equation>x^2 + y^2</hp:equation></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Math { display, tex } => {
                assert!(*display, "equation must be display mode");
                assert_eq!(tex, "x^2 + y^2");
            }
            other => panic!("expected Math, got {other:?}"),
        }
    }

    #[test]
    fn empty_equation_produces_no_block() {
        // An equation element with no text content must be silently dropped.
        let xml = r#"<root><hp:equation></hp:equation></root>"#;
        let s = section(xml);
        assert!(
            s.blocks.is_empty(),
            "empty equation must not produce a block"
        );
    }

    #[test]
    fn eqedit_alias_also_produces_math_block() {
        // The parser accepts both <hp:equation> and <hp:eqEdit>.
        let xml = r#"<root><hp:eqEdit>a + b = c</hp:eqEdit></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Math { tex, .. } => assert_eq!(tex, "a + b = c"),
            other => panic!("expected Math, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — table
    // -----------------------------------------------------------------------

    #[test]
    fn simple_table_two_rows_two_cols() {
        let xml = concat!(
            r#"<root><hp:tbl colCnt="2">"#,
            r#"<hp:tr><hp:tc><hp:p><hp:run><hp:t>A1</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>A2</hp:t></hp:run></hp:p></hp:tc></hp:tr>"#,
            r#"<hp:tr><hp:tc><hp:p><hp:run><hp:t>B1</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>B2</hp:t></hp:run></hp:p></hp:tc></hp:tr>"#,
            r#"</hp:tbl></root>"#,
        );
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Table { rows, col_count } => {
                assert_eq!(*col_count, 2);
                assert_eq!(rows.len(), 2);
                // First row is always the header row.
                assert!(rows[0].is_header, "first row must be is_header=true");
                assert!(!rows[1].is_header, "second row must be is_header=false");
                // Verify cell text content.
                let text_of = |row: usize, col: usize| -> String {
                    match &rows[row].cells[col].blocks[0] {
                        ir::Block::Paragraph { inlines } => inlines[0].text.clone(),
                        other => panic!("unexpected block {other:?}"),
                    }
                };
                assert_eq!(text_of(0, 0), "A1");
                assert_eq!(text_of(0, 1), "A2");
                assert_eq!(text_of(1, 0), "B1");
                assert_eq!(text_of(1, 1), "B2");
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn table_col_count_from_colcnt_attribute() {
        // colCnt="3" but only 2 cells per row — col_count must be max(3, 2) = 3.
        let xml = concat!(
            r#"<root><hp:tbl colCnt="3"><hp:tr>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>X</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>Y</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { col_count, .. } => {
                assert_eq!(*col_count, 3);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn table_cell_colspan_from_celladdr() {
        // cellAddr is a self-closing child of tc that sets colspan/rowspan.
        let xml = concat!(
            r#"<root><hp:tbl colCnt="3"><hp:tr>"#,
            r#"<hp:tc><hp:cellAddr colSpan="2" rowSpan="1"/><hp:p><hp:run><hp:t>merged</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>single</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                let cells = &rows[0].cells;
                assert_eq!(cells[0].colspan, 2, "first cell must have colspan=2");
                assert_eq!(cells[0].rowspan, 1, "first cell must have rowspan=1");
                assert_eq!(
                    cells[1].colspan, 1,
                    "second cell must have default colspan=1"
                );
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn table_cell_rowspan_from_celladdr() {
        let xml = concat!(
            r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
            r#"<hp:tc><hp:cellAddr colSpan="1" rowSpan="3"/><hp:p><hp:run><hp:t>tall</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                assert_eq!(rows[0].cells[0].rowspan, 3);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn table_default_colspan_rowspan_without_celladdr() {
        // When no cellAddr is present the defaults must be colspan=1, rowspan=1.
        let xml = concat!(
            r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>cell</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                assert_eq!(rows[0].cells[0].colspan, 1);
                assert_eq!(rows[0].cells[0].rowspan, 1);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn nested_paragraph_inside_table_cell() {
        // Text inside a table cell must end up in cell blocks, not section blocks.
        let xml = concat!(
            r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
            r#"<hp:tc><hp:p><hp:run><hp:t>cell content</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                let cell = &rows[0].cells[0];
                assert_eq!(cell.blocks.len(), 1);
                match &cell.blocks[0] {
                    ir::Block::Paragraph { inlines } => {
                        assert_eq!(inlines[0].text, "cell content");
                    }
                    other => panic!("expected Paragraph inside cell, got {other:?}"),
                }
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn image_inside_table_cell_goes_to_cell_blocks() {
        let xml = concat!(
            r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
            r#"<hp:tc><hp:img src="fig.png" alt="figure"/></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                let cell = &rows[0].cells[0];
                assert_eq!(cell.blocks.len(), 1);
                match &cell.blocks[0] {
                    ir::Block::Image { src, alt } => {
                        assert_eq!(src, "fig.png");
                        assert_eq!(alt, "figure");
                    }
                    other => panic!("expected Image inside cell, got {other:?}"),
                }
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // parse_section_xml — list
    // -----------------------------------------------------------------------

    #[test]
    fn ordered_list_without_li_produces_no_block() {
        // The current parser recognises <ol>/<ul> open/close but has no <li>
        // handler, so items will be empty.  The block is only pushed when
        // list_items is non-empty, so an empty ol must produce no block.
        // This test documents the current behaviour explicitly.
        let xml = r#"<root><ol></ol></root>"#;
        let s = section(xml);
        assert!(
            s.blocks.is_empty(),
            "empty <ol> without <li> children must produce no block (current behaviour)"
        );
    }

    #[test]
    fn underline_via_empty_charpr() {
        let xml = r#"<root><hp:p><hp:run><hp:charPr underline="solid"/><hp:t>underlined</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(inlines[0].underline, "inline must be underlined");
                assert!(!inlines[0].bold);
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn strikeout_via_empty_charpr() {
        let xml = r#"<root><hp:p><hp:run><hp:charPr strikeout="true"/><hp:t>struck</hp:t></hp:run></hp:p></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert!(inlines[0].strikethrough, "inline must be strikethrough");
                assert!(!inlines[0].bold);
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn colspan_zero_defaults_to_one() {
        let xml = concat!(
            r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
            r#"<hp:tc><hp:cellAddr colSpan="0" rowSpan="0"/><hp:p><hp:run><hp:t>x</hp:t></hp:run></hp:p></hp:tc>"#,
            r#"</hp:tr></hp:tbl></root>"#,
        );
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::Table { rows, .. } => {
                assert_eq!(rows[0].cells[0].colspan, 1, "colSpan=0 must default to 1");
                assert_eq!(rows[0].cells[0].rowspan, 1, "rowSpan=0 must default to 1");
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // guess_mime_from_name — all extensions
    // -----------------------------------------------------------------------

    #[test]
    fn guess_mime_png() {
        assert_eq!(guess_mime_from_name("image.png"), "image/png");
    }

    #[test]
    fn guess_mime_jpg() {
        assert_eq!(guess_mime_from_name("photo.jpg"), "image/jpeg");
    }

    #[test]
    fn guess_mime_jpeg() {
        assert_eq!(guess_mime_from_name("photo.jpeg"), "image/jpeg");
    }

    #[test]
    fn guess_mime_gif() {
        assert_eq!(guess_mime_from_name("anim.gif"), "image/gif");
    }

    #[test]
    fn guess_mime_bmp() {
        assert_eq!(guess_mime_from_name("bitmap.bmp"), "image/bmp");
    }

    #[test]
    fn guess_mime_svg() {
        assert_eq!(guess_mime_from_name("vector.svg"), "image/svg+xml");
    }

    #[test]
    fn guess_mime_wmf() {
        assert_eq!(guess_mime_from_name("metafile.wmf"), "image/x-wmf");
    }

    #[test]
    fn guess_mime_emf() {
        assert_eq!(guess_mime_from_name("enhanced.emf"), "image/x-emf");
    }

    #[test]
    fn guess_mime_unknown_extension_falls_back_to_octet_stream() {
        assert_eq!(
            guess_mime_from_name("archive.xyz"),
            "application/octet-stream"
        );
    }

    #[test]
    fn guess_mime_no_extension_falls_back_to_octet_stream() {
        assert_eq!(
            guess_mime_from_name("nodotfile"),
            "application/octet-stream"
        );
    }

    #[test]
    fn guess_mime_case_insensitive_uppercase_png() {
        assert_eq!(guess_mime_from_name("PHOTO.PNG"), "image/png");
    }

    #[test]
    fn guess_mime_case_insensitive_mixed_jpg() {
        assert_eq!(guess_mime_from_name("Photo.Jpg"), "image/jpeg");
    }

    #[test]
    fn guess_mime_case_insensitive_svg() {
        assert_eq!(guess_mime_from_name("LOGO.SVG"), "image/svg+xml");
    }
}
