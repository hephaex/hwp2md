use super::*;
use crate::hwp::model::{CharShape, DocInfo, HwpParagraph, ParaShape};

// -----------------------------------------------------------------------
// guess_mime
// -----------------------------------------------------------------------

#[test]
fn guess_mime_png_magic() {
    let data = [0x89, b'P', b'N', b'G', 0x00, 0x00];
    assert_eq!(guess_mime(&data), "image/png");
}

#[test]
fn guess_mime_jpeg_magic() {
    let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00];
    assert_eq!(guess_mime(&data), "image/jpeg");
}

#[test]
fn guess_mime_gif_magic() {
    let data = [b'G', b'I', b'F', b'8', b'9', b'a'];
    assert_eq!(guess_mime(&data), "image/gif");
}

#[test]
fn guess_mime_bmp_magic() {
    let data = [b'B', b'M', 0x00, 0x00];
    assert_eq!(guess_mime(&data), "image/bmp");
}

#[test]
fn guess_mime_webp_magic() {
    let mut data = b"RIFF".to_vec();
    data.extend_from_slice(&[0x00u8; 4]);
    data.extend_from_slice(b"WEBP");
    assert_eq!(guess_mime(&data), "image/webp");
}

#[test]
fn guess_mime_unknown_returns_octet_stream() {
    let data = [0x00, 0x01, 0x02, 0x03, 0x04];
    assert_eq!(guess_mime(&data), "application/octet-stream");
}

#[test]
fn guess_mime_too_short_returns_octet_stream() {
    assert_eq!(guess_mime(&[0x89]), "application/octet-stream");
    assert_eq!(guess_mime(&[]), "application/octet-stream");
}

// -----------------------------------------------------------------------
// mime_to_ext
// -----------------------------------------------------------------------

#[test]
fn mime_to_ext_known_types() {
    assert_eq!(mime_to_ext("image/png"), "png");
    assert_eq!(mime_to_ext("image/jpeg"), "jpg");
    assert_eq!(mime_to_ext("image/gif"), "gif");
    assert_eq!(mime_to_ext("image/bmp"), "bmp");
    assert_eq!(mime_to_ext("image/webp"), "webp");
}

#[test]
fn mime_to_ext_unknown_returns_bin() {
    assert_eq!(mime_to_ext("application/octet-stream"), "bin");
    assert_eq!(mime_to_ext("text/plain"), "bin");
}

// -----------------------------------------------------------------------
// detect_heading_level
// -----------------------------------------------------------------------

fn make_para(text: &str, para_shape_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id,
        style_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

fn make_para_with_cs(text: &str, cs_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: vec![(0, cs_id)],
        para_shape_id: 0,
        style_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

#[test]
fn detect_heading_level_from_para_shape() {
    let mut doc_info = DocInfo::default();
    let ps = ParaShape {
        heading_type: Some(1),
        ..Default::default()
    };
    doc_info.para_shapes.push(ps);

    let para = make_para("A heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(2));
}

#[test]
fn detect_heading_level_para_shape_level_0() {
    let mut doc_info = DocInfo::default();
    let ps = ParaShape {
        heading_type: Some(0),
        ..Default::default()
    };
    doc_info.para_shapes.push(ps);

    let para = make_para("Top heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_para_shape_level_6_clamped() {
    let mut doc_info = DocInfo::default();
    let ps = ParaShape {
        heading_type: Some(6),
        ..Default::default()
    };
    doc_info.para_shapes.push(ps);

    let para = make_para("level6 heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(6));
}

#[test]
fn detect_heading_level_para_shape_level_7_rejected() {
    let mut doc_info = DocInfo::default();
    let ps = ParaShape {
        heading_type: Some(7),
        ..Default::default()
    };
    doc_info.para_shapes.push(ps);

    // No char shapes → falls through to None
    let para = make_para("not a heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}

#[test]
fn detect_heading_level_from_char_shape_bold_large_font() {
    let mut doc_info = DocInfo::default();
    // para shape has no heading_type
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1600,
        bold: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Big bold text", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_bold_medium_font_returns_h2() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1400,
        bold: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Medium bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(2));
}

#[test]
fn detect_heading_level_bold_small_font_returns_h3() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1200,
        bold: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Small bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(3));
}

#[test]
fn detect_heading_level_not_bold_large_font_returns_h1() {
    // Bold is no longer required — large font size alone triggers heading detection.
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1600,
        bold: false,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Large not bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_long_text_skips_heuristic() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1600,
        bold: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    // Text longer than 100 chars should skip the font heuristic.
    let long_text = "A".repeat(101);
    let para = make_para_with_cs(&long_text, 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}

// -----------------------------------------------------------------------
// Style-name based detection (shared parse_heading_style)
// -----------------------------------------------------------------------

#[test]
fn detect_heading_level_outline_english_returns_level() {
    // style_id 1 → "Outline 2" → Some(2)
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default()); // index 0, style_id 0 unused
    doc_info.styles.push("Normal".to_string()); // index 0
    doc_info.styles.push("Outline 2".to_string()); // index 1

    let para = HwpParagraph {
        text: "Section heading".to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: 0,
        style_id: 1,
        controls: Vec::new(),
        raw_para_text: None,
    };
    assert_eq!(detect_heading_level(&para, &doc_info), Some(2));
}

#[test]
fn detect_heading_level_outline_korean_returns_level() {
    // style_id 1 → "개요 3" → Some(3)
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    doc_info.styles.push("바탕글".to_string()); // index 0 (normal body)
    doc_info.styles.push("개요 3".to_string()); // index 1

    let para = HwpParagraph {
        text: "Korean outline".to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: 0,
        style_id: 1,
        controls: Vec::new(),
        raw_para_text: None,
    };
    assert_eq!(detect_heading_level(&para, &doc_info), Some(3));
}

#[test]
fn detect_heading_level_lenient_lowercase_heading() {
    // style_id 1 → "heading 1" (lowercase) → Some(1)
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    doc_info.styles.push("Normal".to_string()); // index 0
    doc_info.styles.push("heading 1".to_string()); // index 1

    let para = HwpParagraph {
        text: "Lowercase heading".to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: 0,
        style_id: 1,
        controls: Vec::new(),
        raw_para_text: None,
    };
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_style_id_out_of_bounds_no_panic() {
    // style_id 99 but styles only has 3 entries → must return None without panic
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    doc_info.styles.push("Normal".to_string()); // index 0
    doc_info.styles.push("Outline 1".to_string()); // index 1
    doc_info.styles.push("Outline 2".to_string()); // index 2

    let para = HwpParagraph {
        text: "Body text".to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: 0,
        style_id: 99,
        controls: Vec::new(),
        raw_para_text: None,
    };
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}
