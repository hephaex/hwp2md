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
fn detect_heading_level_not_bold_returns_none() {
    // Bold is required for tier-3 (font-size) heading detection.
    // 14pt non-bold is standard article text in Korean government docs.
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let cs = CharShape {
        height: 1600,
        bold: false,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Large not bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
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

// -----------------------------------------------------------------------
// Tier-4: Korean regulation text patterns (제N장/절/조)
// -----------------------------------------------------------------------

#[test]
fn detect_korean_regulation_heading_jang_returns_h1() {
    assert_eq!(detect_korean_regulation_heading("제1장 총칙"), Some(1));
}

#[test]
fn detect_korean_regulation_heading_jeol_returns_h2() {
    assert_eq!(detect_korean_regulation_heading("제2절 운영"), Some(2));
}

#[test]
fn detect_korean_regulation_heading_jo_returns_h2() {
    // 조 maps to H2 (same as 절) to avoid orphan H3 in documents without 절.
    assert_eq!(detect_korean_regulation_heading("제12조"), Some(2));
}

#[test]
fn detect_korean_regulation_heading_no_digits_returns_none() {
    assert_eq!(detect_korean_regulation_heading("제장"), None);
}

#[test]
fn detect_korean_regulation_heading_wrong_suffix_returns_none() {
    assert_eq!(detect_korean_regulation_heading("제3과"), None);
}

#[test]
fn detect_korean_regulation_heading_leading_whitespace_matched() {
    // HWP authors often embed leading spaces in chapter headings as indentation.
    // trim_start() is applied so "   제1장" is correctly detected as H1.
    assert_eq!(detect_korean_regulation_heading("   제1장 총칙"), Some(1));
    assert_eq!(detect_korean_regulation_heading("    제1절 운영"), Some(2));
}

#[test]
fn detect_korean_regulation_heading_amendment_notation_jo_ui_n() {
    // "제N조의M" (amendment sub-articles) should match 조 → H2.
    assert_eq!(detect_korean_regulation_heading("제5조의2 특례"), Some(2));
}

#[test]
fn detect_korean_regulation_heading_jang_jeol_jo_hierarchy() {
    // S55-02: validate that 장→H1, 절→H2, 조→H2 in a document that has all three.
    // 절 and 조 are at the same level to avoid H1→H3 gap in documents without 절.
    assert_eq!(detect_korean_regulation_heading("제1장 총칙"), Some(1));
    assert_eq!(detect_korean_regulation_heading("제1절 일반사항"), Some(2));
    assert_eq!(detect_korean_regulation_heading("제1조 목적"), Some(2));
    // Both must be H2 — compare against a concrete value so None==None would fail.
    let jeol = detect_korean_regulation_heading("제2절 운영");
    let jo = detect_korean_regulation_heading("제3조 적용범위");
    assert_eq!(jeol, Some(2));
    assert_eq!(jo, Some(2));
    assert_eq!(jeol, jo);
}

#[test]
fn detect_korean_regulation_heading_tab_indented_matched() {
    // trim_start() covers \t as well as spaces.
    assert_eq!(detect_korean_regulation_heading("\t제1장 총칙"), Some(1));
    assert_eq!(detect_korean_regulation_heading("\t\t제3조 적용"), Some(2));
}

#[test]
fn detect_korean_regulation_heading_long_article_body_not_promoted() {
    // Real-world moel_02 pattern: article marker + full body in one PARA_HEADER.
    // A paragraph >= 100 chars must NOT be promoted, regardless of prefix.
    let body = "제1조(목적) 이 고시는 「국민 평생 직업능력 개발법」 제12조ㆍ제15조ㆍ제16조ㆍ제17조에서 \
                위임된 사항과 그 시행에 필요한 사항을 규정함으로써 근로자의 직업능력향상을 목적으로 한다.";
    assert!(body.chars().count() >= 100);
    assert_eq!(detect_korean_regulation_heading(body), None);
}

#[test]
fn detect_korean_regulation_heading_tier4_integration() {
    // All tier-1/2/3 signals absent: empty para_shapes, empty styles, no char_shapes.
    let doc_info = DocInfo::default();
    let para = make_para("제5장 시행규칙", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

// -----------------------------------------------------------------------
// S54-03: Tier-1 heading_type works even when bold=false (Sprint 52 carryover)
// -----------------------------------------------------------------------

#[test]
fn detect_heading_level_para_shape_tier1_ignores_bold() {
    // Tier-1 (heading_type) must fire regardless of CharShape.bold.
    let mut doc_info = DocInfo::default();
    let ps = ParaShape {
        heading_type: Some(0),
        ..Default::default()
    };
    doc_info.para_shapes.push(ps);
    // Add a CharShape that is NOT bold and below tier-3 thresholds.
    let cs = CharShape {
        height: 800,
        bold: false,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("제1조 목적", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}
