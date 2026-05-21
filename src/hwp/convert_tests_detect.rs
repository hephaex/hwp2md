use super::*;
use crate::hwp::heading_style::is_heading_terminator;
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
fn detect_korean_regulation_heading_pyeon_returns_h1() {
    assert_eq!(
        detect_korean_regulation_heading("제1편 총칙"),
        Some(1),
        r#""제1편 총칙" should yield Some(1) — 편 maps to H1 like 장"#
    );
    assert_eq!(
        detect_korean_regulation_heading("제3편 채권"),
        Some(1),
        r#""제3편 채권" should yield Some(1)"#
    );
}

#[test]
fn detect_korean_regulation_heading_pyeon_multi_digit() {
    assert_eq!(
        detect_korean_regulation_heading("제10편 총칙"),
        Some(1),
        r#""제10편 총칙" should yield Some(1) — multi-digit 편"#
    );
}

#[test]
fn detect_korean_regulation_heading_pyeon_leading_whitespace() {
    assert_eq!(
        detect_korean_regulation_heading("  제2편 물권"),
        Some(1),
        r#""  제2편 물권" should yield Some(1) — trim_start() handles leading spaces"#
    );
    assert_eq!(
        detect_korean_regulation_heading("\t제2편 물권"),
        Some(1),
        r#""\t제2편 물권" should yield Some(1) — leading tab stripped"#
    );
}

#[test]
fn detect_korean_regulation_heading_pyeon_particle_rejection() {
    // "제3편은" — the 조사 '은' immediately after 편 is NOT a terminator → None.
    assert_eq!(
        detect_korean_regulation_heading("제3편은 적용한다"),
        None,
        r#""제3편은 적용한다" should yield None — '은' after 편 is a particle, not a terminator"#
    );
    assert_eq!(
        detect_korean_regulation_heading("제1편에서"),
        None,
        r#""제1편에서" should yield None — '에' after 편 is a particle"#
    );
}

#[test]
fn detect_korean_regulation_heading_pyeon_terminator_chars() {
    // Terminator characters immediately after 편 — should all yield Some(1)
    assert_eq!(
        detect_korean_regulation_heading("제2편(총칙)"),
        Some(1),
        r#""제2편(총칙)" should yield Some(1) — open paren is a terminator"#
    );
    assert_eq!(
        detect_korean_regulation_heading("제2편: 총칙"),
        Some(1),
        r#""제2편: 총칙" should yield Some(1) — colon is a terminator"#
    );
}

#[test]
fn detect_korean_regulation_heading_pyeon_jang_both_h1() {
    // 편 and 장 both map to H1 — they do not co-occur as top-level in the same
    // statute, so no level shift is needed. This test documents the intentional
    // "same bucket" policy.
    assert_eq!(
        detect_korean_regulation_heading("제1편 총칙"),
        Some(1),
        r#""제1편 총칙" → Some(1)"#
    );
    assert_eq!(
        detect_korean_regulation_heading("제1장 총칙"),
        Some(1),
        r#""제1장 총칙" → Some(1) — same level as 편"#
    );
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
    // Multi-digit amendment number.
    assert_eq!(detect_korean_regulation_heading("제5조의10 특례"), Some(2));
    // "의" with no following character (EOL) — not a valid amendment form → None.
    assert_eq!(detect_korean_regulation_heading("제5조의"), None);
    // "의" + space + digit (non-canonical form) → None.
    assert_eq!(detect_korean_regulation_heading("제5조의 2"), None);
    // "의" + Korean digit character (not ASCII) → None.
    assert_eq!(detect_korean_regulation_heading("제5조의일"), None);
}

#[test]
fn detect_korean_regulation_heading_inline_reference_particle_returns_none() {
    // Korean grammatical particles directly after 장/절/조 signal an inline
    // reference, not a heading (e.g. "제3장은 적용되지 않는다").
    assert_eq!(detect_korean_regulation_heading("제3장은 적용되지 않는다"), None);
    assert_eq!(detect_korean_regulation_heading("제5조에서 정한 사항"), None);
    assert_eq!(detect_korean_regulation_heading("제2절의 규정"), None); // 의 + non-digit → None
}

#[test]
fn detect_korean_regulation_heading_terminator_chars_still_match() {
    // Headings followed by word-terminating punctuation must still be detected.
    assert_eq!(detect_korean_regulation_heading("제1장(총칙)"), Some(1));
    assert_eq!(detect_korean_regulation_heading("제1장. 총칙"), Some(1));
    assert_eq!(detect_korean_regulation_heading("제3조:"), Some(2));
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
fn detect_korean_regulation_heading_tab_between_marker_and_title() {
    // Tab AFTER the suffix boundary — caught by is_heading_terminator('\t') via is_whitespace().
    // Isolates the tab-as-terminator path. tab_indented_matched exercises trim_start() on the
    // leading tab and then hits a space terminator; here we exercise the tab terminator alone.
    assert_eq!(
        detect_korean_regulation_heading("제3조\t참조"),
        Some(2),
        "tab (U+0009) between marker and title should yield Some(2)"
    );
    assert_eq!(
        detect_korean_regulation_heading("제1장\t총칙"),
        Some(1),
        "tab (U+0009) after 장 should yield Some(1)"
    );
}

#[test]
fn detect_korean_regulation_heading_ideographic_space_treated_as_heading() {
    // U+3000 (IDEOGRAPHIC SPACE) is caught by is_whitespace() per Unicode spec.
    // HWP/HWPX documents often separate 조/장 from the title with U+3000 instead of U+0020.
    // Using escape form (\u{3000}) to make the codepoint unambiguous in source.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{3000}참조"),
        Some(2),
        "ideographic space (U+3000) after 조 should yield Some(2)"
    );
    assert_eq!(
        detect_korean_regulation_heading("제1장\u{3000}총칙"),
        Some(1),
        "ideographic space (U+3000) after 장 should yield Some(1)"
    );
    // U+00A0 (NBSP) — also caught by is_whitespace(). Common in copy-pasted statute text.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{00A0}참조"),
        Some(2),
        "NBSP (U+00A0) after 조 should yield Some(2)"
    );
    // U+202F (NARROW NO-BREAK SPACE) — is_whitespace() = true. Used in formatted Korean
    // numbers and some HWP authoring tools emit it as a no-break separator.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{202F}참조"),
        Some(2),
        "narrow NBSP (U+202F) after 조 should yield Some(2)"
    );
    // U+205F (MEDIUM MATHEMATICAL SPACE) — is_whitespace() = true per Unicode White_Space.
    // Rare in statute text but covered by the same is_whitespace() branch.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{205F}참조"),
        Some(2),
        "medium math space (U+205F) after 조 should yield Some(2)"
    );
    // U+200B (ZERO WIDTH SPACE) is NOT Unicode White_Space → not a heading terminator.
    // Regression pin: if a future change makes ZWSP a terminator, this test trips.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{200B}참조"),
        None,
        "ZWSP (U+200B) after 조 is not is_whitespace() — should yield None"
    );
    // U+FEFF (BOM/ZWNBSP) is NOT Unicode White_Space → not a heading terminator.
    // Regression pin: Sprint 68 negative-pin partner to U+205F (positive pin above);
    // together with U+200B these complete the White_Space coverage table.
    assert_eq!(
        detect_korean_regulation_heading("제3조\u{FEFF}참조"),
        None,
        "BOM/ZWNBSP (U+FEFF) after 조 is not is_whitespace() — should yield None"
    );
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
// is_heading_terminator policy tests (see git log for sprint history)
// -----------------------------------------------------------------------

#[test]
fn is_heading_terminator_canonical_allowed_set() {
    // Whitespace variants — is_whitespace() covers all Unicode White_Space chars
    assert!(is_heading_terminator(' '), "space");
    assert!(is_heading_terminator('\t'), "tab");
    assert!(is_heading_terminator('\u{3000}'), "U+3000 ideographic space");
    assert!(is_heading_terminator('\u{00A0}'), "U+00A0 non-breaking space");
    assert!(is_heading_terminator('\u{202F}'), "U+202F narrow non-breaking space");
    assert!(is_heading_terminator('\u{205F}'), "U+205F medium mathematical space");
    // ASCII parens and brackets (both directions)
    assert!(is_heading_terminator('('), "open paren");
    assert!(is_heading_terminator(')'), "close paren");
    assert!(is_heading_terminator('['), "open bracket");
    assert!(is_heading_terminator(']'), "close bracket");
    // CJK quotes — all open/close pairs (symmetric — review enforces symmetry)
    assert!(is_heading_terminator('「'), "CJK open guillemet");
    assert!(is_heading_terminator('」'), "CJK close guillemet");
    assert!(is_heading_terminator('『'), "CJK double open guillemet");
    assert!(is_heading_terminator('』'), "CJK double close guillemet");
    // ASCII angle brackets (< >)
    assert!(is_heading_terminator('<'), "ASCII angle open");
    assert!(is_heading_terminator('>'), "ASCII angle close");
    // CJK title brackets — canonical Korean title delimiters
    assert!(is_heading_terminator('《'), "CJK double open angle");
    assert!(is_heading_terminator('》'), "CJK double close angle");
    assert!(is_heading_terminator('〈'), "CJK single open angle");
    assert!(is_heading_terminator('〉'), "CJK single close angle");
    // Fullwidth parens (common in OCR'd Korean docs)
    assert!(is_heading_terminator('（'), "fullwidth open");
    assert!(is_heading_terminator('）'), "fullwidth close");
    // Punctuation — ASCII (including separators that appear after 조/절/장)
    assert!(is_heading_terminator('.'), "period");
    assert!(is_heading_terminator(':'), "colon");
    assert!(is_heading_terminator(','), "comma");
    assert!(is_heading_terminator('-'), "hyphen");
    assert!(is_heading_terminator('~'), "tilde");
    assert!(is_heading_terminator('·'), "middle dot");
    assert!(is_heading_terminator('ㆍ'), "Korean middle dot");
    assert!(is_heading_terminator('…'), "ellipsis");
    // Punctuation — fullwidth variants
    assert!(is_heading_terminator('：'), "fullwidth colon");
    assert!(is_heading_terminator('．'), "fullwidth period");
    assert!(is_heading_terminator('；'), "fullwidth semicolon");
    assert!(is_heading_terminator('，'), "fullwidth comma");
}

#[test]
fn is_heading_terminator_blocked_set() {
    // Korean grammatical particles — must NOT terminate
    assert!(!is_heading_terminator('은'), "subject particle 은");
    assert!(!is_heading_terminator('에'), "location particle 에");
    assert!(!is_heading_terminator('이'), "subject particle 이");
    assert!(!is_heading_terminator('가'), "subject particle 가");
    assert!(!is_heading_terminator('의'), "possessive particle 의");
    // ASCII letters and digits — must NOT terminate
    assert!(!is_heading_terminator('a'), "ASCII letter");
    assert!(!is_heading_terminator('1'), "ASCII digit");
    // Non-Korean punctuation not in the allowed set — must NOT terminate
    assert!(!is_heading_terminator('%'), "percent");
    assert!(!is_heading_terminator('&'), "ampersand");
    assert!(!is_heading_terminator('/'), "slash");
    assert!(!is_heading_terminator('"'), "double quote");
    // ASCII semicolon ';' is NOT in the allowlist — only the fullwidth variant '；' is.
    // This guards against accidentally promoting ASCII ';' to terminator status.
    assert!(!is_heading_terminator(';'), "ASCII semicolon (fullwidth ； is allowed, ASCII ';' is not)");
    // Zero-width chars that look whitespace-adjacent but are NOT Unicode White_Space.
    assert!(!is_heading_terminator('\u{200B}'), "U+200B ZWSP is not is_whitespace() — must NOT terminate");
    assert!(!is_heading_terminator('\u{FEFF}'), "U+FEFF BOM/ZWNBSP is not is_whitespace() — must NOT terminate");
}

#[test]
fn detect_korean_regulation_heading_range_expression_treated_as_heading() {
    // "제3조-제5조": dash is in the terminator allowlist so the check at
    // the first char after '조' succeeds and yields Some(2). Tier-4 fires
    // only when tier-1/2/3 format signals are absent, so a bare range line
    // is treated as a heading-like fragment. Inline-particle references
    // ("제3조에서…", "제3조는…") are caught by the particle rejection path.
    assert_eq!(
        detect_korean_regulation_heading("제3조-제5조"),
        Some(2),
        "'제3조-제5조' range expression should yield article heading level Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_double_angle_open_treated_as_heading() {
    // "제5장《한국》": 《 (U+300A) opens the CJK double-angle pair — heading terminator.
    // Symmetric with detect_korean_regulation_heading_cjk_double_angle_close_treated_as_heading.
    assert_eq!(
        detect_korean_regulation_heading("제5장《한국》"),
        Some(1),
        "'제5장《한국》' CJK double-angle open after 장 should yield chapter heading level Some(1)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_guillemet_open_treated_as_heading() {
    // "제3조『인용』": 『 is in the terminator allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조『인용』"),
        Some(2),
        "'제3조『인용』' CJK double-guillemet open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_guillemet_close_treated_as_heading() {
    // Closer symmetry: 』 is also in the allowlist. A paragraph starting with
    // "제3조』" (e.g. after an orphaned close-guillemet) still terminates.
    assert_eq!(
        detect_korean_regulation_heading("제3조』참조"),
        Some(2),
        "'제3조』참조' CJK double-guillemet close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_single_guillemet_open_treated_as_heading() {
    // "제3조「참조」": 「 is in the terminator allowlist → Some(2).
    // Symmetric with the 『』 pair added in Sprint 61.
    assert_eq!(
        detect_korean_regulation_heading("제3조「참조」"),
        Some(2),
        "'제3조「참조」' CJK single-guillemet open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_single_guillemet_close_treated_as_heading() {
    // Closer symmetry: 」 is also in the allowlist.
    assert_eq!(
        detect_korean_regulation_heading("제3조」참조"),
        Some(2),
        "'제3조」참조' CJK single-guillemet close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_single_angle_open_treated_as_heading() {
    // "제3조〈참조〉": 〈 (U+3008) is in the terminator allowlist → Some(2).
    // Symmetric with 〉 close test below.
    assert_eq!(
        detect_korean_regulation_heading("제3조〈참조〉"),
        Some(2),
        "'제3조〈참조〉' CJK single-angle open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_single_angle_close_treated_as_heading() {
    // Closer symmetry: 〉 (U+3009) is also in the allowlist.
    assert_eq!(
        detect_korean_regulation_heading("제3조〉참조"),
        Some(2),
        "'제3조〉참조' CJK single-angle close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_cjk_double_angle_close_treated_as_heading() {
    // "제3조》참조": 》 (U+300B) close-side of the 《》 pair is in the allowlist.
    // The opener 《 behavioral test: detect_korean_regulation_heading_cjk_double_angle_open_treated_as_heading.
    assert_eq!(
        detect_korean_regulation_heading("제3조》참조"),
        Some(2),
        "'제3조》참조' CJK double-angle close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_angle_open_treated_as_heading() {
    // "제3조<참조>": ASCII '<' is in the terminator allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조<참조>"),
        Some(2),
        "'제3조<참조>' ASCII angle-open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_angle_close_treated_as_heading() {
    // Closer symmetry: ASCII '>' is also in the allowlist.
    assert_eq!(
        detect_korean_regulation_heading("제3조>참조"),
        Some(2),
        "'제3조>참조' ASCII angle-close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_bracket_open_treated_as_heading() {
    // "제3조[참조]": '[' is in the terminator allowlist → Some(2). Closes bracket matrix.
    assert_eq!(
        detect_korean_regulation_heading("제3조[참조]"),
        Some(2),
        "'제3조[참조]' ASCII bracket-open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_bracket_close_treated_as_heading() {
    // Closer symmetry: ']' is also in the allowlist. Orphaned close-bracket at boundary.
    assert_eq!(
        detect_korean_regulation_heading("제3조]참조"),
        Some(2),
        "'제3조]참조' ASCII bracket-close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_paren_open_treated_as_heading() {
    // "제3조（참조）": '（' (U+FF08) fullwidth open paren is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조（참조）"),
        Some(2),
        "'제3조（참조）' fullwidth paren-open after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_paren_close_treated_as_heading() {
    // Closer symmetry: '）' (U+FF09) is also in the allowlist. Orphaned close.
    assert_eq!(
        detect_korean_regulation_heading("제3조）참조"),
        Some(2),
        "'제3조）참조' fullwidth paren-close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_tilde_treated_as_heading() {
    // "제3조~제5조": '~' is in the terminator allowlist (range notation) → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조~제5조"),
        Some(2),
        "'제3조~제5조' tilde range after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_middle_dot_treated_as_heading() {
    // "제3조·제5조": '·' (U+00B7 middle dot) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조·제5조"),
        Some(2),
        "'제3조·제5조' middle-dot separator after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_korean_middle_dot_treated_as_heading() {
    // "제3조ㆍ제5조": 'ㆍ' (U+318D Korean middle dot) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조ㆍ제5조"),
        Some(2),
        "'제3조ㆍ제5조' Korean middle-dot separator after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ellipsis_treated_as_heading() {
    // "제3조…": '…' (U+2026) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조…"),
        Some(2),
        "'제3조…' ellipsis after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_colon_treated_as_heading() {
    // "제3조：참조": '：' (U+FF1A fullwidth colon) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조：참조"),
        Some(2),
        "'제3조：참조' fullwidth colon after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_period_treated_as_heading() {
    // "제3조．": '．' (U+FF0E fullwidth period) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조．"),
        Some(2),
        "'제3조．' fullwidth period after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_semicolon_treated_as_heading() {
    // "제3조；참조": '；' (U+FF1B fullwidth semicolon) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조；참조"),
        Some(2),
        "'제3조；참조' fullwidth semicolon after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_fullwidth_comma_treated_as_heading() {
    // "제3조，제5조": '，' (U+FF0C fullwidth comma) is in the allowlist → Some(2).
    assert_eq!(
        detect_korean_regulation_heading("제3조，제5조"),
        Some(2),
        "'제3조，제5조' fullwidth comma after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_comma_treated_as_heading() {
    // "제3조,제5조": ',' (U+002C) ASCII comma is in the allowlist → Some(2).
    // Symmetric with detect_korean_regulation_heading_fullwidth_comma_treated_as_heading.
    assert_eq!(
        detect_korean_regulation_heading("제3조,제5조"),
        Some(2),
        "'제3조,제5조' ASCII comma after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_ascii_paren_close_treated_as_heading() {
    // "제3조)참조": ')' is in the allowlist. Orphaned close-paren at the suffix boundary.
    // Complements the open-paren coverage in terminator_chars_still_match ("제1장(총칙)").
    assert_eq!(
        detect_korean_regulation_heading("제3조)참조"),
        Some(2),
        "'제3조)참조' ASCII paren-close after 조 should yield Some(2)"
    );
}

#[test]
fn detect_korean_regulation_heading_dash_then_particle_still_classifies_as_heading() {
    // Known limitation: the function checks only the first char after 장/절/조.
    // "제3조-제5조는 적용 제외" starts with dash (a terminator), so the trailing
    // "는" particle is never reached and the function returns Some(2) — even
    // though this text is a cross-reference clause, not a heading.
    // Tier-4 fires only when tier-1/2/3 signals are absent; in real HWP
    // documents, such paragraphs usually carry non-zero style_id that satisfies
    // tier-2 instead. Pinned here as a regression anchor, not a design intent.
    assert_eq!(
        detect_korean_regulation_heading("제3조-제5조는 적용 제외"),
        Some(2),
        "'제3조-제5조는 적용 제외' dash terminates before particle — known tier-4 limitation"
    );
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
