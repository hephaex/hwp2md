use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
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
    in_list_item: bool,
    list_item_blocks: Vec<ir::Block>,
    list_item_inlines: Vec<ir::Inline>,
    list_item_text: String,
    equation_text: String,
    in_equation: bool,
    // footnote / endnote accumulation
    in_footnote: bool,
    footnote_id: String,
    footnote_blocks: Vec<ir::Block>,
    footnote_inlines: Vec<ir::Inline>,
    footnote_text: String,
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
            in_list_item: false,
            list_item_blocks: Vec::new(),
            list_item_inlines: Vec::new(),
            list_item_text: String::new(),
            equation_text: String::new(),
            in_equation: false,
            in_footnote: false,
            footnote_id: String::new(),
            footnote_blocks: Vec::new(),
            footnote_inlines: Vec::new(),
            footnote_text: String::new(),
        }
    }
}

/// Parse `bold`, `italic`, `underline`, and `strikeout` attributes from a
/// `<charPr>` or `<hp:charPr>` element and write them onto `ctx`.
///
/// Called from both `handle_start_element` (non-self-closing variant) and
/// `handle_empty_element` (self-closing variant) so the two paths are
/// guaranteed to behave identically.
fn apply_charpr_attrs(e: &quick_xml::events::BytesStart, ctx: &mut ParseContext) {
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let val = attr.unescape_value().unwrap_or_default();
        match key {
            "bold" | "hp:bold" => ctx.current_bold = val.as_ref() == "true" || val.as_ref() == "1",
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
                if ctx.in_footnote {
                    ctx.footnote_inlines.push(inline);
                } else if ctx.in_cell {
                    ctx.cell_inlines.push(inline);
                } else if ctx.in_list_item {
                    ctx.list_item_inlines.push(inline);
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
        if ctx.in_footnote {
            ctx.footnote_text.push_str(text);
        } else if ctx.in_cell {
            ctx.cell_text.push_str(text);
        } else if ctx.in_list_item {
            ctx.list_item_text.push_str(text);
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
                let img = ir::Block::Image { src, alt };
                if ctx.in_footnote {
                    ctx.footnote_blocks.push(img);
                } else if ctx.in_list_item {
                    ctx.list_item_blocks.push(img);
                } else if ctx.in_cell {
                    ctx.cell_blocks.push(img);
                } else {
                    section.blocks.push(img);
                }
            }
        }
        "lineBreak" | "hp:lineBreak" => {
            if ctx.in_footnote {
                ctx.footnote_text.push('\n');
            } else if ctx.in_list_item {
                ctx.list_item_text.push('\n');
            } else if ctx.in_cell {
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
                if ctx.in_footnote {
                    ctx.footnote_inlines.push(inline);
                } else if ctx.in_list_item {
                    ctx.list_item_inlines.push(inline);
                } else if ctx.in_cell {
                    ctx.cell_inlines.push(inline);
                } else {
                    ctx.current_inlines.push(inline);
                }
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
                if ctx.in_footnote {
                    ctx.footnote_inlines.push(inline);
                } else if ctx.in_list_item {
                    ctx.list_item_inlines.push(inline);
                } else if ctx.in_cell {
                    ctx.cell_inlines.push(inline);
                } else {
                    ctx.current_inlines.push(inline);
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

fn flush_list_item_paragraph(ctx: &mut ParseContext) {
    if !ctx.list_item_text.is_empty() {
        let text = std::mem::take(&mut ctx.list_item_text);
        ctx.list_item_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            ..ir::Inline::default()
        });
    }

    if !ctx.list_item_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.list_item_inlines);
        ctx.list_item_blocks.push(ir::Block::Paragraph { inlines });
    }
}

/// Flush any pending run text and inline list from within a footnote/endnote
/// paragraph into `footnote_blocks`.  Mirrors the logic of `flush_cell_paragraph`.
fn flush_footnote_paragraph(ctx: &mut ParseContext) {
    if !ctx.footnote_text.is_empty() {
        let text = std::mem::take(&mut ctx.footnote_text);
        ctx.footnote_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            ..ir::Inline::default()
        });
    }

    if !ctx.footnote_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.footnote_inlines);
        ctx.footnote_blocks.push(ir::Block::Paragraph { inlines });
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
        // "제목1" -> 1
        assert_eq!(parse_heading_style("제목1"), Some(1));
    }

    #[test]
    fn parse_heading_style_korean_outline_3() {
        // "개요3" -> 3
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
        // "Heading" without a trailing digit -> defaults to level 1.
        assert_eq!(parse_heading_style("Heading"), Some(1));
    }

    #[test]
    fn parse_heading_style_case_insensitive() {
        assert_eq!(parse_heading_style("HEADING2"), Some(2));
    }

    // -----------------------------------------------------------------------
    // parse_section_xml -- helper for asserting on the returned Section
    // -----------------------------------------------------------------------

    /// Unwrap the section and panic with a descriptive message on error.
    fn section(xml: &str) -> ir::Section {
        parse_section_xml(xml).expect("parse_section_xml must not fail")
    }

    // -----------------------------------------------------------------------
    // parse_section_xml -- empty / minimal documents
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
    // parse_section_xml -- simple paragraph
    // -----------------------------------------------------------------------

    #[test]
    fn simple_paragraph_text() {
        // Compact XML -- no whitespace text nodes between tags (matches real HWPX).
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
    // parse_section_xml -- heading via styleIDRef
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
    // parse_section_xml -- bold / italic via Start-element charPr
    // -----------------------------------------------------------------------

    #[test]
    fn bold_text_via_charpr_start_element() {
        // Start-element charPr (non-self-closing) -- handled by handle_start_element.
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
        // Self-closing charPr -- handled by handle_empty_element.
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
    // parse_section_xml -- lineBreak (empty element)
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
    // parse_section_xml -- image (empty element)
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
    // parse_section_xml -- equation
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
    // parse_section_xml -- table
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
        // colCnt="3" but only 2 cells per row -- col_count must be max(3, 2) = 3.
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
    // parse_section_xml -- list
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
    // guess_mime_from_name -- all extensions
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

    // -----------------------------------------------------------------------
    // BinData reference resolution -- resolve_bin_refs + build_bin_map
    // -----------------------------------------------------------------------

    /// Helper: build a section containing a single top-level Image block.
    fn make_image_section(src: &str) -> ir::Section {
        ir::Section {
            blocks: vec![ir::Block::Image {
                src: src.to_string(),
                alt: String::new(),
            }],
        }
    }

    #[test]
    fn resolve_bin_refs_replaces_image_src() {
        // An Image whose src matches a BinData stem must be updated to the
        // full ZIP path, including the extension.
        let bin_files = vec!["BinData/BIN0001.png".to_string()];
        let bin_map = build_bin_map(&bin_files);

        let mut section = make_image_section("BIN0001");
        resolve_bin_refs(&mut section, &bin_map);

        match &section.blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(
                    src, "BinData/BIN0001.png",
                    "src must be resolved to full path"
                );
            }
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn resolve_bin_refs_no_match_leaves_src_unchanged() {
        // An Image with a src that has no entry in the bin_map must not be
        // modified -- e.g. when src is already a full filename or an URL.
        let bin_files = vec!["BinData/BIN0001.png".to_string()];
        let bin_map = build_bin_map(&bin_files);

        let mut section = make_image_section("img.png");
        resolve_bin_refs(&mut section, &bin_map);

        match &section.blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(src, "img.png", "unmatched src must remain unchanged");
            }
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn resolve_bin_refs_inside_table_cell() {
        // resolve_block_bin_refs must recurse into Table -> rows -> cells -> blocks.
        let bin_files = vec!["BinData/BIN0002.jpg".to_string()];
        let bin_map = build_bin_map(&bin_files);

        let cell_image = ir::Block::Image {
            src: "BIN0002".to_string(),
            alt: String::new(),
        };
        let cell = ir::TableCell {
            blocks: vec![cell_image],
            colspan: 1,
            rowspan: 1,
        };
        let row = ir::TableRow {
            cells: vec![cell],
            is_header: false,
        };
        let mut section = ir::Section {
            blocks: vec![ir::Block::Table {
                rows: vec![row],
                col_count: 1,
            }],
        };

        resolve_bin_refs(&mut section, &bin_map);

        match &section.blocks[0] {
            ir::Block::Table { rows, .. } => match &rows[0].cells[0].blocks[0] {
                ir::Block::Image { src, .. } => {
                    assert_eq!(
                        src, "BinData/BIN0002.jpg",
                        "image inside table cell must be resolved"
                    );
                }
                other => panic!("expected Image inside cell, got {other:?}"),
            },
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn bin_map_from_bin_files() {
        // build_bin_map must produce a map with stem keys and full-path values.
        // It must handle both prefixes (BinData/ and Contents/BinData/).
        let bin_files = vec![
            "BinData/BIN0001.png".to_string(),
            "BinData/BIN0002.jpg".to_string(),
            "Contents/BinData/BIN0003.emf".to_string(),
        ];
        let map = build_bin_map(&bin_files);

        assert_eq!(
            map.get("BIN0001").map(String::as_str),
            Some("BinData/BIN0001.png")
        );
        assert_eq!(
            map.get("BIN0002").map(String::as_str),
            Some("BinData/BIN0002.jpg")
        );
        assert_eq!(
            map.get("BIN0003").map(String::as_str),
            Some("Contents/BinData/BIN0003.emf")
        );
        assert_eq!(map.len(), 3, "map must contain exactly 3 entries");
    }
    // -----------------------------------------------------------------------
    // parse_section_xml -- footnote / endnote parsing
    // -----------------------------------------------------------------------

    fn first_footnote(s: &ir::Section) -> (&str, &[ir::Block]) {
        match &s.blocks[0] {
            ir::Block::Footnote { id, content } => (id.as_str(), content.as_slice()),
            other => panic!("expected Block::Footnote, got {other:?}"),
        }
    }

    #[test]
    fn footnote_produces_footnote_block() {
        let xml = r#"<root><hp:fn id="1"><hp:p><hp:run><hp:t>note text</hp:t></hp:run></hp:p></hp:fn></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1, "one footnote block expected");
        let (id, content) = first_footnote(&s);
        assert_eq!(id, "1");
        assert_eq!(
            content.len(),
            1,
            "footnote must have exactly one inner block"
        );
        match &content[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines[0].text, "note text");
            }
            other => panic!("expected Paragraph inside footnote, got {other:?}"),
        }
    }

    #[test]
    fn endnote_produces_footnote_block() {
        let xml = r#"<root><hp:en id="2"><hp:p><hp:run><hp:t>end note</hp:t></hp:run></hp:p></hp:en></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        let (id, content) = first_footnote(&s);
        assert_eq!(id, "2");
        match &content[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines[0].text, "end note");
            }
            other => panic!("expected Paragraph inside endnote block, got {other:?}"),
        }
    }

    #[test]
    fn footnote_alt_tag_name() {
        let xml = r#"<root><hp:footnote id="3"><hp:p><hp:run><hp:t>alt tag</hp:t></hp:run></hp:p></hp:footnote></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        let (id, content) = first_footnote(&s);
        assert_eq!(id, "3");
        match &content[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines[0].text, "alt tag");
            }
            other => panic!("expected Paragraph inside footnote (alt tag), got {other:?}"),
        }
    }

    #[test]
    fn note_ref_produces_footnote_ref_inline() {
        // <hp:noteRef noteId="1"/> produces an Inline with footnote_ref set and empty text.
        let xml = r#"<root><hp:p><hp:noteRef noteId="1"/></hp:p></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1, "one paragraph block expected");
        match &s.blocks[0] {
            ir::Block::Paragraph { inlines } => {
                assert_eq!(inlines.len(), 1, "one inline expected");
                assert_eq!(
                    inlines[0].footnote_ref.as_deref(),
                    Some("1"),
                    "inline must carry footnote_ref=\"1\""
                );
                assert!(
                    inlines[0].text.is_empty(),
                    "footnote_ref inline must have empty text"
                );
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn empty_footnote_ignored() {
        let xml = r#"<root><hp:fn id="1"></hp:fn></root>"#;
        let s = section(xml);
        assert!(
            s.blocks.is_empty(),
            "empty footnote must not produce a Block::Footnote"
        );
    }

    // -----------------------------------------------------------------------
    // Cross-cutting: context x element combinations
    // -----------------------------------------------------------------------

    #[test]
    fn image_inside_footnote_goes_to_footnote_blocks() {
        let xml = r#"<root><hp:fn id="1"><hp:img src="fig.png" alt="fn-img"/></hp:fn></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::Footnote { content, .. } => {
                assert!(
                    content
                        .iter()
                        .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "fig.png")),
                    "footnote must contain the image block"
                );
            }
            other => panic!("expected Footnote, got {other:?}"),
        }
    }

    #[test]
    fn image_inside_list_item_goes_to_list_item_blocks() {
        let xml = r#"<root><ul><li><hp:img src="pic.png" alt="li-img"/></li></ul></root>"#;
        let s = section(xml);
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            ir::Block::List { items, .. } => {
                assert_eq!(items.len(), 1);
                assert!(
                    items[0]
                        .blocks
                        .iter()
                        .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "pic.png")),
                    "list item must contain the image block"
                );
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn linebreak_inside_list_item_appends_newline() {
        let xml = r#"<root><ul><li><hp:p><hp:run><hp:t>before</hp:t><hp:lineBreak/></hp:run></hp:p></li></ul></root>"#;
        let s = section(xml);
        match &s.blocks[0] {
            ir::Block::List { items, .. } => {
                let text: String = items[0]
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        ir::Block::Paragraph { inlines } => {
                            Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    text.contains('\n'),
                    "lineBreak in list item must produce newline; got: {text:?}"
                );
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn resolve_bin_refs_inside_footnote() {
        let bin_map: HashMap<String, String> =
            [("BIN0002".to_string(), "BinData/BIN0002.jpg".to_string())]
                .into_iter()
                .collect();
        let mut section = ir::Section {
            blocks: vec![ir::Block::Footnote {
                id: "1".to_string(),
                content: vec![ir::Block::Image {
                    src: "BIN0002".to_string(),
                    alt: String::new(),
                }],
            }],
        };
        resolve_bin_refs(&mut section, &bin_map);
        match &section.blocks[0] {
            ir::Block::Footnote { content, .. } => match &content[0] {
                ir::Block::Image { src, .. } => {
                    assert_eq!(src, "BinData/BIN0002.jpg");
                }
                other => panic!("expected Image, got {other:?}"),
            },
            other => panic!("expected Footnote, got {other:?}"),
        }
    }

    #[test]
    fn resolve_bin_refs_inside_list() {
        let bin_map: HashMap<String, String> =
            [("BIN0003".to_string(), "BinData/BIN0003.png".to_string())]
                .into_iter()
                .collect();
        let mut section = ir::Section {
            blocks: vec![ir::Block::List {
                ordered: false,
                start: 1,
                items: vec![ir::ListItem {
                    blocks: vec![ir::Block::Image {
                        src: "BIN0003".to_string(),
                        alt: String::new(),
                    }],
                    children: Vec::new(),
                }],
            }],
        };
        resolve_bin_refs(&mut section, &bin_map);
        match &section.blocks[0] {
            ir::Block::List { items, .. } => match &items[0].blocks[0] {
                ir::Block::Image { src, .. } => {
                    assert_eq!(src, "BinData/BIN0003.png");
                }
                other => panic!("expected Image, got {other:?}"),
            },
            other => panic!("expected List, got {other:?}"),
        }
    }
}
