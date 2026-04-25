use super::*;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn make_para(text: &str, para_shape_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

// -----------------------------------------------------------------------
// build_inlines
// -----------------------------------------------------------------------

#[test]
fn build_inlines_empty_text_returns_empty() {
    let doc_info = DocInfo::default();
    let para = make_para("", 0);
    let inlines = build_inlines(&para, &doc_info);
    assert!(inlines.is_empty());
}

#[test]
fn build_inlines_no_char_shapes_returns_plain_inline() {
    let doc_info = DocInfo::default();
    let para = make_para("Hello world", 0);
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].text, "Hello world");
    assert!(!inlines[0].bold);
}

#[test]
fn build_inlines_with_bold_char_shape() {
    let mut doc_info = DocInfo::default();
    let cs = CharShape {
        bold: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Bold text".to_string(),
        char_shape_ids: vec![(0, 0)], // position 0, shape 0
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(inlines[0].bold);
    assert_eq!(inlines[0].text.trim_end_matches('\r'), "Bold text");
}

#[test]
fn build_inlines_with_italic_char_shape() {
    let mut doc_info = DocInfo::default();
    let cs = CharShape {
        italic: true,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Italic text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(inlines[0].italic);
}

#[test]
fn build_inlines_unknown_cs_id_falls_back_to_plain() {
    let doc_info = DocInfo::default(); // no char shapes

    let para = HwpParagraph {
        text: "Plain fallback".to_string(),
        char_shape_ids: vec![(0, 99)], // cs_id 99 doesn't exist
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(!inlines[0].bold);
}

#[test]
fn build_inlines_position_past_end_stops() {
    let doc_info = DocInfo::default();
    let para = HwpParagraph {
        text: "Hi".to_string(),
        char_shape_ids: vec![(100, 0)], // position 100 > text length 2
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    // No inline emitted because start >= chars.len()
    assert!(inlines.is_empty());
}

#[test]
fn build_inlines_multiple_segments() {
    let mut doc_info = DocInfo::default();
    let cs0 = CharShape {
        bold: true,
        ..Default::default()
    };
    let cs1 = CharShape::default(); // not bold
    doc_info.char_shapes.push(cs0);
    doc_info.char_shapes.push(cs1);

    let para = HwpParagraph {
        text: "BoldNormal".to_string(),
        char_shape_ids: vec![(0, 0), (4, 1)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 2);
    assert!(inlines[0].bold);
    assert!(!inlines[1].bold);
}

// -----------------------------------------------------------------------
// build_inlines — color propagation
// -----------------------------------------------------------------------

#[test]
fn build_inlines_non_black_color_sets_css_hex() {
    let mut doc_info = DocInfo::default();
    // Store red: HWP BGR byte order, u32 = (0xFF << 16) | 0x00 | 0x00 = 0x00FF0000
    // red stored in BGR: bit[23:16]=red=0xFF
    let cs = CharShape {
        color: 0x00FF_0000,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Red text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].color.as_deref(), Some("#FF0000"));
}

#[test]
fn build_inlines_black_color_is_none() {
    let mut doc_info = DocInfo::default();
    let cs = CharShape {
        color: 0x0000_0000,
        ..Default::default()
    }; // black
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Black text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert!(inlines[0].color.is_none(), "black must not set color");
}

#[test]
fn build_inlines_bgr_green_color_maps_correctly() {
    let mut doc_info = DocInfo::default();
    // Pure green in BGR: u32 = (0x00 << 16) | (0xFF << 8) | 0x00 = 0x0000_FF00
    let cs = CharShape {
        color: 0x0000_FF00,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Green".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines[0].color.as_deref(), Some("#00FF00"));
}

// -----------------------------------------------------------------------
// build_inlines — face_id → font_name resolution
// -----------------------------------------------------------------------

#[test]
fn build_inlines_face_id_resolves_font_name() {
    let mut doc_info = DocInfo {
        face_names: vec!["Arial".to_string(), "Batang".to_string()],
        ..Default::default()
    };
    let cs = CharShape {
        face_id: 1,
        ..Default::default()
    }; // index into face_names → "Batang"
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Korean".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].font_name.as_deref(), Some("Batang"));
}

#[test]
fn build_inlines_face_id_out_of_bounds_font_name_is_none() {
    let mut doc_info = DocInfo::default();
    // face_names is empty, so any face_id is out of bounds.
    let cs = CharShape {
        face_id: 5,
        ..Default::default()
    };
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(inlines[0].font_name.is_none());
}
