use crate::hwp::eqedit::eqedit_to_latex;
use crate::hwp::model::*;
use crate::ir;

const SAFE_URL_SCHEMES: &[&str] = &["http://", "https://", "mailto:", "ftp://", "ftps://"];

fn is_safe_url_scheme(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    SAFE_URL_SCHEMES.iter().any(|s| lower.starts_with(s))
}

/// HWP font height is in 1/100 point units (HWP internal unit).
/// e.g. 1600 = 16pt, 1400 = 14pt, 1200 = 12pt.
const HEADING1_MIN_HEIGHT: u32 = 1600; // 16pt
const HEADING2_MIN_HEIGHT: u32 = 1400; // 14pt
const HEADING3_MIN_HEIGHT: u32 = 1200; // 12pt

pub(crate) fn hwp_to_ir(hwp: &HwpDocument) -> ir::Document {
    let mut doc = ir::Document::new();

    doc.metadata.title = hwp.summary_title.clone();
    doc.metadata.author = hwp.summary_author.clone();
    doc.metadata.subject = hwp.summary_subject.clone();
    doc.metadata.keywords = hwp.summary_keywords.clone();

    let mut footnote_counter: u32 = 0;
    let mut endnote_counter: u32 = 0;

    for section in &hwp.sections {
        let mut ir_section = ir::Section { blocks: Vec::new() };

        for para in &section.paragraphs {
            let blocks =
                paragraph_to_blocks_counted(para, &hwp.doc_info, &mut footnote_counter, &mut endnote_counter);
            ir_section.blocks.extend(blocks);
        }

        doc.sections.push(ir_section);
    }

    for (id, data) in &hwp.bin_data {
        let mime = guess_mime(data);
        let ext = mime_to_ext(&mime);
        doc.assets.push(ir::Asset {
            name: format!("image_{id}.{ext}"),
            data: data.clone(),
            mime_type: mime,
        });
    }

    doc
}

/// Counter-aware variant used by `hwp_to_ir` to assign unique sequential IDs
/// to footnotes and endnotes across the whole document.
fn paragraph_to_blocks_counted(
    para: &HwpParagraph,
    doc_info: &DocInfo,
    footnote_counter: &mut u32,
    endnote_counter: &mut u32,
) -> Vec<ir::Block> {
    let mut blocks: Vec<ir::Block> = Vec::new();

    for ctrl in &para.controls {
        if let Some(block) =
            control_to_block_counted(ctrl, doc_info, footnote_counter, endnote_counter)
        {
            blocks.push(block);
        }
    }

    let text = para.text.trim();
    if !text.is_empty() {
        let heading_level = detect_heading_level(para, doc_info);
        let inlines = build_inlines(para, doc_info);
        if !inlines.is_empty() {
            let ps_id = para.para_shape_id as usize;
            if ps_id < doc_info.para_shapes.len() {
                if let Some(nid) = doc_info.para_shapes[ps_id].numbering_id {
                    tracing::debug!(
                        numbering_id = nid,
                        "paragraph may be a list item; full list conversion not yet implemented"
                    );
                }
            }

            if let Some(level) = heading_level {
                blocks.push(ir::Block::Heading { level, inlines });
            } else {
                blocks.push(ir::Block::Paragraph { inlines });
            }
        }
    }

    blocks
}

/// Counter-aware variant of `control_to_block` that assigns sequential IDs
/// to footnotes (`footnote-1`, `footnote-2`, …) and endnotes (`endnote-1`, …).
fn control_to_block_counted(
    ctrl: &HwpControl,
    doc_info: &DocInfo,
    footnote_counter: &mut u32,
    endnote_counter: &mut u32,
) -> Option<ir::Block> {
    if let HwpControl::FootnoteEndnote {
        is_endnote,
        paragraphs,
    } = ctrl
    {
        let content: Vec<ir::Block> = paragraphs
            .iter()
            .flat_map(|p| {
                paragraph_to_blocks_counted(p, doc_info, footnote_counter, endnote_counter)
            })
            .collect();
        let id = if *is_endnote {
            *endnote_counter += 1;
            format!("endnote-{endnote_counter}")
        } else {
            *footnote_counter += 1;
            format!("footnote-{footnote_counter}")
        };
        return Some(ir::Block::Footnote { id, content });
    }
    control_to_block(ctrl, doc_info)
}

pub(crate) fn paragraph_to_blocks(para: &HwpParagraph, doc_info: &DocInfo) -> Vec<ir::Block> {
    let mut blocks: Vec<ir::Block> = Vec::new();

    // Emit IR blocks for each embedded control first.  Controls are independent
    // of the paragraph text (a paragraph may contain *only* a table, for example).
    for ctrl in &para.controls {
        if let Some(block) = control_to_block(ctrl, doc_info) {
            blocks.push(block);
        }
    }

    // Emit the text content of the paragraph, if any.
    let text = para.text.trim();
    if !text.is_empty() {
        let heading_level = detect_heading_level(para, doc_info);
        let inlines = build_inlines(para, doc_info);
        if !inlines.is_empty() {
            // Log list-item hint when the paragraph has a numbering_id.
            // Full list-item conversion is left for a future implementation pass.
            let ps_id = para.para_shape_id as usize;
            if ps_id < doc_info.para_shapes.len() {
                if let Some(nid) = doc_info.para_shapes[ps_id].numbering_id {
                    tracing::debug!(
                        numbering_id = nid,
                        "paragraph may be a list item; full list conversion not yet implemented"
                    );
                }
            }

            if let Some(level) = heading_level {
                blocks.push(ir::Block::Heading { level, inlines });
            } else {
                blocks.push(ir::Block::Paragraph { inlines });
            }
        }
    }

    blocks
}

/// Convert a single `HwpControl` to an `ir::Block`.  Returns `None` for
/// controls that have no direct IR representation (e.g. page-break hints).
pub(crate) fn control_to_block(ctrl: &HwpControl, doc_info: &DocInfo) -> Option<ir::Block> {
    match ctrl {
        HwpControl::Table {
            row_count,
            col_count,
            cells,
        } => {
            // Group cells by row index, then sort each row by col index.
            let n_rows = *row_count as usize;
            let n_cols = *col_count as usize;
            let effective_cols = if n_cols > 0 {
                n_cols
            } else {
                cells.iter().map(|c| c.col as usize + 1).max().unwrap_or(1)
            };

            let mut rows: Vec<Vec<&HwpTableCell>> = vec![Vec::new(); n_rows.max(1)];
            for cell in cells {
                let row_idx = cell.row as usize;
                if row_idx < rows.len() {
                    rows[row_idx].push(cell);
                } else if row_idx < 10_000 {
                    rows.resize(row_idx + 1, Vec::new());
                    rows[row_idx].push(cell);
                }
            }

            let ir_rows: Vec<ir::TableRow> = rows
                .into_iter()
                .enumerate()
                .map(|(row_idx, row_cells)| {
                    // Capture is_header from the first cell before sorting consumes the vec.
                    // Fall back to row_idx == 0 for empty rows (no cells parsed).
                    let row_is_header = row_cells
                        .first()
                        .map(|c| c.is_header)
                        .unwrap_or(row_idx == 0);
                    let mut sorted = row_cells;
                    sorted.sort_by_key(|c| c.col);
                    let ir_cells: Vec<ir::TableCell> = sorted
                        .into_iter()
                        .map(|cell| ir::TableCell {
                            blocks: cell
                                .paragraphs
                                .iter()
                                .flat_map(|p| paragraph_to_blocks(p, doc_info))
                                .collect(),
                            colspan: cell.col_span as u32,
                            rowspan: cell.row_span as u32,
                        })
                        .collect();
                    ir::TableRow {
                        cells: ir_cells,
                        is_header: row_is_header,
                    }
                })
                .collect();

            Some(ir::Block::Table {
                rows: ir_rows,
                col_count: effective_cols,
            })
        }
        HwpControl::Image { bin_data_id, .. } => {
            let src = format!("image_{bin_data_id}.bin");
            Some(ir::Block::Image {
                src,
                alt: String::new(),
            })
        }
        HwpControl::Equation { script } => {
            let tex = eqedit_to_latex(script);
            Some(ir::Block::Math {
                display: false,
                tex,
            })
        }
        HwpControl::FootnoteEndnote {
            is_endnote,
            paragraphs,
        } => {
            let content: Vec<ir::Block> = paragraphs
                .iter()
                .flat_map(|p| paragraph_to_blocks(p, doc_info))
                .collect();
            let id = if *is_endnote {
                "endnote".to_string()
            } else {
                "footnote".to_string()
            };
            Some(ir::Block::Footnote { id, content })
        }
        HwpControl::Hyperlink { ref url } => {
            if url.is_empty() || !is_safe_url_scheme(url) {
                None
            } else {
                Some(ir::Block::Paragraph {
                    inlines: vec![ir::Inline {
                        text: url.clone(),
                        link: Some(url.clone()),
                        ..Default::default()
                    }],
                })
            }
        }
        HwpControl::PageBreak | HwpControl::ColumnBreak => None,
    }
}

pub(crate) fn detect_heading_level(para: &HwpParagraph, doc_info: &DocInfo) -> Option<u8> {
    let ps_id = para.para_shape_id as usize;
    if ps_id < doc_info.para_shapes.len() {
        if let Some(level) = doc_info.para_shapes[ps_id].heading_type {
            if level < 7 {
                return Some((level + 1).min(6));
            }
        }
    }

    let text = para.text.trim();
    if text.chars().count() < 100 {
        if let Some(first_cs) = para.char_shape_ids.first() {
            let cs_id = first_cs.1 as usize;
            if cs_id < doc_info.char_shapes.len() {
                let cs = &doc_info.char_shapes[cs_id];
                if cs.height >= HEADING1_MIN_HEIGHT && cs.bold {
                    return Some(1);
                }
                if cs.height >= HEADING2_MIN_HEIGHT && cs.bold {
                    return Some(2);
                }
                if cs.height >= HEADING3_MIN_HEIGHT && cs.bold {
                    return Some(3);
                }
            }
        }
    }

    None
}

pub(crate) fn build_inlines(para: &HwpParagraph, doc_info: &DocInfo) -> Vec<ir::Inline> {
    let text = &para.text;
    if text.is_empty() {
        return Vec::new();
    }

    if para.char_shape_ids.is_empty() {
        return vec![ir::Inline::plain(text.clone())];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut inlines = Vec::new();
    let char_refs = &para.char_shape_ids;

    for (idx, &(pos, cs_id)) in char_refs.iter().enumerate() {
        let start = pos as usize;
        let end = if idx + 1 < char_refs.len() {
            char_refs[idx + 1].0 as usize
        } else {
            chars.len()
        };

        if start >= chars.len() {
            break;
        }
        let end = end.min(chars.len());
        let segment: String = chars[start..end].iter().collect();
        let segment = segment.trim_end_matches('\r').to_string();

        if segment.is_empty() {
            continue;
        }

        let cs_idx = cs_id as usize;
        let inline = if cs_idx < doc_info.char_shapes.len() {
            let cs = &doc_info.char_shapes[cs_idx];

            // HWP color is stored as u32 in BGR byte order:
            //   bits[ 7: 0] = blue, bits[15: 8] = green, bits[23:16] = red.
            // Only emit a color when the value is not black (0x000000) to avoid
            // wrapping every default run in a redundant <span>.
            let color = if cs.color & 0x00FF_FFFF != 0 {
                let b = (cs.color & 0xFF) as u8;
                let g = ((cs.color >> 8) & 0xFF) as u8;
                let r = ((cs.color >> 16) & 0xFF) as u8;
                Some(format!("#{r:02X}{g:02X}{b:02X}"))
            } else {
                None
            };

            // Resolve font name via face_id lookup in the DocInfo face_names table.
            let font_name = doc_info.face_names.get(cs.face_id as usize).cloned();

            ir::Inline {
                text: segment,
                bold: cs.bold,
                italic: cs.italic,
                underline: cs.underline,
                strikethrough: cs.strikethrough,
                superscript: cs.superscript,
                subscript: cs.subscript,
                color,
                font_name,
                ..ir::Inline::default()
            }
        } else {
            ir::Inline::plain(segment)
        };

        inlines.push(inline);
    }

    inlines
}

fn guess_mime(data: &[u8]) -> String {
    if data.len() < 4 {
        return "application/octet-stream".to_string();
    }
    match &data[..4] {
        [0x89, b'P', b'N', b'G'] => "image/png".to_string(),
        [0xFF, 0xD8, 0xFF, _] => "image/jpeg".to_string(),
        [b'G', b'I', b'F', b'8'] => "image/gif".to_string(),
        [b'B', b'M', _, _] => "image/bmp".to_string(),
        _ => {
            if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
                "image/webp".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        }
    }
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        _ => "bin",
    }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
