use crate::hwp::eqedit::eqedit_to_latex;
use crate::hwp::model::*;
use crate::ir;
use crate::url_util::is_safe_url_scheme;

/// HWP font height is in 1/100 point units (HWP internal unit).
/// e.g. 1600 = 16pt, 1400 = 14pt, 1200 = 12pt.
const HEADING1_MIN_HEIGHT: u32 = 1600; // 16pt
const HEADING2_MIN_HEIGHT: u32 = 1400; // 14pt
const HEADING3_MIN_HEIGHT: u32 = 1200; // 12pt

// ── List detection ────────────────────────────────────────────────────────────

/// The list kind detected for a single HWP binary paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// Unordered (bullet) list item.
    Unordered,
    /// Ordered (numbered) list item.
    Ordered,
}

/// Detect whether a paragraph is a list item and return its [`ListKind`].
///
/// # Detection strategy (two-tier)
///
/// **Tier 1 — binary numbering_id** (preferred):
/// When the paragraph's `ParaShape` record carries a non-zero `numbering_id`
/// the paragraph is formally defined as a list item by the HWP document model.
/// We inspect the paragraph text to heuristically decide whether it is ordered
/// or unordered, because the numbering _style_ is stored in a separate
/// `HWPTAG_NUMBERING` record that we do not currently parse.
///
/// **Tier 2 — text heuristics** (pragmatic fallback):
/// When no `numbering_id` is available we scan the leading characters of the
/// trimmed paragraph text for common bullet and numbering patterns.
///
/// Returns `None` when the paragraph is not a list item.
pub(crate) fn detect_list_kind(para: &HwpParagraph, doc_info: &DocInfo) -> Option<ListKind> {
    let text = para.text.trim();
    if text.is_empty() {
        return None;
    }

    // Tier 1: use the binary numbering_id field when present.
    let ps_id = para.para_shape_id as usize;
    let has_numbering_id =
        ps_id < doc_info.para_shapes.len() && doc_info.para_shapes[ps_id].numbering_id.is_some();

    if has_numbering_id {
        // The paragraph is formally a list item.  Determine order by inspecting
        // the text prefix (the numbering _style_ record is not yet parsed).
        return Some(if is_ordered_prefix(text) {
            ListKind::Ordered
        } else {
            ListKind::Unordered
        });
    }

    // Tier 2: heuristic text-pattern detection.
    detect_list_kind_from_text(text)
}

/// Return `true` when `text` starts with a common ordered-list prefix such as
/// `"1. "`, `"2) "`, `"a. "`, `"i. "`, etc.
///
/// We deliberately avoid matching year-like patterns such as `"2026년…"` by
/// requiring that the numeric prefix is short (≤ 3 digits) **and** immediately
/// followed by `.` or `)` and then whitespace (or end-of-string).
fn is_ordered_prefix(text: &str) -> bool {
    let s = text.trim_start_matches(' ');

    let first = match s.chars().next() {
        Some(c) => c,
        None => return false,
    };

    if first.is_ascii_digit() {
        // Count consecutive digits (must be ≤ 3 to reject "2026년…").
        let digit_end = s
            .char_indices()
            .take_while(|(_, c)| c.is_ascii_digit())
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        let digit_count = digit_end; // ASCII digits: byte length == char count
        if digit_count == 0 || digit_count > 3 {
            return false;
        }
        let rest = &s[digit_end..];
        let sep = rest.chars().next();
        if matches!(sep, Some('.') | Some(')')) {
            let after_sep = &rest[1..]; // separator is ASCII, 1 byte
            return matches!(after_sep.chars().next(), Some(' ') | Some('\t') | None);
        }
        return false;
    }

    if first.is_ascii_alphabetic() {
        let rest = &s[1..]; // letter is ASCII, 1 byte
        let sep = rest.chars().next();
        if matches!(sep, Some('.') | Some(')')) {
            let after_sep = &rest[1..];
            return matches!(after_sep.chars().next(), Some(' ') | Some('\t') | None);
        }
    }

    false
}

/// Detect list kind purely from the leading characters of `text`.
///
/// # Recognised unordered bullet characters
/// `●`, `■`, `▶`, `▷`, `◆`, `◇`, `•`, `·`, `-`, `*`
///
/// # Recognised ordered patterns
/// `"1. "`, `"2. "` (up to 3 digits), `"a. "`, `"i. "` etc.
fn detect_list_kind_from_text(text: &str) -> Option<ListKind> {
    // Ordered detection — check first (a "1." prefix beats any bullet check).
    if is_ordered_prefix(text) {
        return Some(ListKind::Ordered);
    }

    // Unordered bullet characters — require a space/tab/EOL after the bullet.
    let first_char = text.chars().next()?;
    let is_bullet = matches!(
        first_char,
        '●' | '■' | '▶' | '▷' | '◆' | '◇' | '•' | '·' | '-' | '*'
    );
    if is_bullet {
        let mut chars = text.chars();
        chars.next(); // consume bullet
        let next = chars.next();
        if matches!(next, Some(' ') | Some('\t') | None) {
            return Some(ListKind::Unordered);
        }
    }

    None
}

// ── Staged-block grouping ─────────────────────────────────────────────────────

/// An intermediate block produced when translating a section's flat paragraph
/// sequence into proper `Block::List` structures.
///
/// This mirrors the `StagedBlock` type in the HWPX reader (`hwpx::context`)
/// but is private to the HWP binary converter.
#[derive(Debug)]
enum StagedBlock {
    Plain(ir::Block),
    ListPara { ordered: bool, block: ir::Block },
}

/// Collapse a flat sequence of [`StagedBlock`]s into a proper `Vec<ir::Block>`
/// where consecutive `ListPara` entries of the same list type are grouped into
/// `Block::List` values.
///
/// # Grouping rules
///
/// - Consecutive `ListPara` items of the **same type** (both ordered or both
///   unordered) are folded into a single `Block::List`.
/// - A type transition (ordered → unordered or vice versa) starts a new list.
/// - Any `Plain` block flushes the pending list and is emitted directly.
fn group_list_paragraphs(staged: Vec<StagedBlock>) -> Vec<ir::Block> {
    let mut out: Vec<ir::Block> = Vec::with_capacity(staged.len());

    // Pending run: (ordered, accumulated items).
    let mut pending: Option<(bool, Vec<ir::ListItem>)> = None;

    let flush = |pending: &mut Option<(bool, Vec<ir::ListItem>)>, out: &mut Vec<ir::Block>| {
        if let Some((ordered, items)) = pending.take() {
            out.push(ir::Block::List {
                ordered,
                start: 1,
                items,
            });
        }
    };

    for sb in staged {
        match sb {
            StagedBlock::Plain(block) => {
                flush(&mut pending, &mut out);
                out.push(block);
            }
            StagedBlock::ListPara { ordered, block } => {
                let item = ir::ListItem {
                    blocks: vec![block],
                    children: vec![],
                };
                match pending {
                    Some((ref p_ordered, ref mut items)) if *p_ordered == ordered => {
                        items.push(item);
                    }
                    _ => {
                        // Different type or no pending list — flush old, start new.
                        flush(&mut pending, &mut out);
                        pending = Some((ordered, vec![item]));
                    }
                }
            }
        }
    }
    flush(&mut pending, &mut out);

    out
}

// ── Top-level IR conversion ───────────────────────────────────────────────────

pub(crate) fn hwp_to_ir(hwp: &HwpDocument) -> ir::Document {
    let mut doc = ir::Document::new();

    doc.metadata.title = hwp.summary_title.clone();
    doc.metadata.author = hwp.summary_author.clone();
    doc.metadata.subject = hwp.summary_subject.clone();
    doc.metadata.keywords = hwp.summary_keywords.clone();

    let mut footnote_counter: u32 = 0;
    let mut endnote_counter: u32 = 0;

    for section in &hwp.sections {
        // Stage each paragraph (or control block) into a StagedBlock, then
        // collapse consecutive list-item paragraphs into Block::List values.
        let mut staged: Vec<StagedBlock> = Vec::new();

        for para in &section.paragraphs {
            paragraph_to_staged_counted(
                para,
                &hwp.doc_info,
                &mut footnote_counter,
                &mut endnote_counter,
                &mut staged,
            );
        }

        let blocks = group_list_paragraphs(staged);
        doc.sections.push(ir::Section {
            blocks,
            page_layout: None,
        });
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

/// Push staged blocks for a paragraph, using sequential footnote/endnote IDs.
///
/// Control blocks (tables, images, footnotes, …) are always emitted as
/// `StagedBlock::Plain`.  The paragraph text is classified by
/// [`detect_list_kind`] and wrapped in either `StagedBlock::ListPara` or
/// `StagedBlock::Plain`.  Headings are always plain (never list items).
fn paragraph_to_staged_counted(
    para: &HwpParagraph,
    doc_info: &DocInfo,
    footnote_counter: &mut u32,
    endnote_counter: &mut u32,
    out: &mut Vec<StagedBlock>,
) {
    for ctrl in &para.controls {
        if let Some(block) =
            control_to_block_counted(ctrl, doc_info, footnote_counter, endnote_counter)
        {
            out.push(StagedBlock::Plain(block));
        }
    }

    let text = para.text.trim();
    if text.is_empty() {
        return;
    }

    let heading_level = detect_heading_level(para, doc_info);
    let inlines = build_inlines(para, doc_info);
    if inlines.is_empty() {
        return;
    }

    // Headings are never list items.
    if let Some(level) = heading_level {
        out.push(StagedBlock::Plain(ir::Block::Heading { level, inlines }));
        return;
    }

    match detect_list_kind(para, doc_info) {
        Some(ListKind::Ordered) => {
            out.push(StagedBlock::ListPara {
                ordered: true,
                block: ir::Block::Paragraph { inlines },
            });
        }
        Some(ListKind::Unordered) => {
            out.push(StagedBlock::ListPara {
                ordered: false,
                block: ir::Block::Paragraph { inlines },
            });
        }
        None => {
            out.push(StagedBlock::Plain(ir::Block::Paragraph { inlines }));
        }
    }
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
            .flat_map(|p| paragraph_to_blocks(p, doc_info))
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
                    inlines: vec![ir::Inline::plain(url.clone()).with_link(Some(url.clone()))],
                })
            }
        }
        HwpControl::Ruby {
            base_text,
            ruby_text,
        } => {
            if base_text.is_empty() && ruby_text.is_empty() {
                return None;
            }
            let ruby = if ruby_text.is_empty() {
                None
            } else {
                Some(ruby_text.clone())
            };
            Some(ir::Block::Paragraph {
                inlines: vec![ir::Inline::plain(base_text.clone()).with_ruby(ruby)],
            })
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

            let mut inline = ir::Inline::with_formatting(
                segment,
                cs.bold,
                cs.italic,
                cs.underline,
                cs.strikethrough,
                cs.superscript,
                cs.subscript,
                color,
            );
            inline.font_name = font_name;
            inline
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
#[path = "convert_tests_control.rs"]
mod tests_control;

#[cfg(test)]
#[path = "convert_tests_detect.rs"]
mod tests_detect;

#[cfg(test)]
#[path = "convert_tests_ir.rs"]
mod tests_ir;

#[cfg(test)]
#[path = "convert_tests_build.rs"]
mod tests_build;

#[cfg(test)]
#[path = "convert_tests_list.rs"]
mod tests_list;
