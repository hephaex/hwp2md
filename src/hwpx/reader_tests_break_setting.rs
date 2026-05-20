use super::parse_break_setting;
use crate::ir::{Block, BreakSetting, Document, Inline, Metadata, Section};

// ── Unit tests: parse_break_setting from raw header XML ────────────────────

/// `BreakSetting::default()` must have all four flags false.
#[test]
fn break_setting_defaults_all_false() {
    let bs = BreakSetting::default();
    assert!(!bs.widow_orphan, "widow_orphan must default to false");
    assert!(!bs.keep_with_next, "keep_with_next must default to false");
    assert!(!bs.keep_lines, "keep_lines must default to false");
    assert!(!bs.page_break_before, "page_break_before must default to false");
}

/// Parsing an empty string (no header.xml) must yield all-false defaults.
#[test]
fn break_setting_empty_xml_yields_defaults() {
    let bs = parse_break_setting("");
    assert_eq!(bs, BreakSetting::default(), "empty XML must yield default BreakSetting");
}

/// A header.xml with no paraPr must yield all-false defaults.
#[test]
fn break_setting_no_para_pr_yields_defaults() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="0"/>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert_eq!(bs, BreakSetting::default(), "header with no paraPr must yield defaults");
}

/// `widowOrphan="true"` in the id=0 paraPr must be parsed as `true`.
#[test]
fn break_setting_widow_orphan_true_parsed() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="1">
      <hh:paraPr id="0">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="true" keepWithNext="false"
                         keepLines="false" pageBreakBefore="false" lineWrap="BREAK"/>
      </hh:paraPr>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert!(bs.widow_orphan, "widowOrphan=\"true\" must parse to true");
    assert!(!bs.keep_with_next, "keepWithNext must remain false");
    assert!(!bs.keep_lines, "keepLines must remain false");
    assert!(!bs.page_break_before, "pageBreakBefore must remain false");
}

/// `keepWithNext="true"` in the id=0 paraPr must be parsed as `true`.
#[test]
fn break_setting_keep_with_next_true_parsed() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="1">
      <hh:paraPr id="0">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="false" keepWithNext="true"
                         keepLines="false" pageBreakBefore="false" lineWrap="BREAK"/>
      </hh:paraPr>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert!(!bs.widow_orphan, "widowOrphan must remain false");
    assert!(bs.keep_with_next, "keepWithNext=\"true\" must parse to true");
    assert!(!bs.keep_lines, "keepLines must remain false");
    assert!(!bs.page_break_before, "pageBreakBefore must remain false");
}

/// All four flags set to `"true"` must all parse as `true`.
#[test]
fn break_setting_all_true_parsed() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="1">
      <hh:paraPr id="0">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="true" keepWithNext="true"
                         keepLines="true" pageBreakBefore="true" lineWrap="BREAK"/>
      </hh:paraPr>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert!(bs.widow_orphan, "widowOrphan must be true");
    assert!(bs.keep_with_next, "keepWithNext must be true");
    assert!(bs.keep_lines, "keepLines must be true");
    assert!(bs.page_break_before, "pageBreakBefore must be true");
}

/// All four flags explicitly `"false"` must parse as false (same as defaults).
#[test]
fn break_setting_all_false_parsed() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="1">
      <hh:paraPr id="0">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="false" keepWithNext="false"
                         keepLines="false" pageBreakBefore="false" lineWrap="BREAK"/>
      </hh:paraPr>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert_eq!(bs, BreakSetting::default(), "all-false paraPr must equal BreakSetting::default()");
}

/// Only id=0 is read; a breakSetting in id=1 must not affect the result.
#[test]
fn break_setting_only_id0_is_read() {
    let xml = r#"<?xml version="1.0"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:paraProperties itemCnt="2">
      <hh:paraPr id="0">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="false" keepWithNext="false"
                         keepLines="false" pageBreakBefore="false" lineWrap="BREAK"/>
      </hh:paraPr>
      <hh:paraPr id="1">
        <hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="KEEP_WORD"
                         widowOrphan="true" keepWithNext="true"
                         keepLines="true" pageBreakBefore="true" lineWrap="BREAK"/>
      </hh:paraPr>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
    let bs = parse_break_setting(xml);
    assert_eq!(
        bs,
        BreakSetting::default(),
        "breakSetting from id=1 must NOT bleed into result; only id=0 is used"
    );
}

// ── Roundtrip tests: write an HWPX file, read it back, verify IR ───────────

/// Helper: write a document with the given `BreakSetting` to a temp HWPX, then
/// read it back and return the first section's `BreakSetting`.
fn roundtrip_break_setting(bs: BreakSetting) -> BreakSetting {
    use crate::hwpx::{read_hwpx, write_hwpx};

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::plain("roundtrip text")],
            }],
            break_setting: bs,
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx must not fail");

    let recovered = read_hwpx(tmp.path()).expect("read_hwpx must not fail");
    recovered
        .sections
        .into_iter()
        .next()
        .expect("must have at least one section")
        .break_setting
}

/// A default section (all false) survives a full HWPX write → read roundtrip.
#[test]
fn break_setting_default_section_has_false_values() {
    let recovered = roundtrip_break_setting(BreakSetting::default());
    assert_eq!(
        recovered,
        BreakSetting::default(),
        "default (all-false) BreakSetting must survive HWPX roundtrip"
    );
}

/// `widow_orphan=true` survives a full HWPX write → read roundtrip.
#[test]
fn break_setting_widow_orphan_true_roundtrip() {
    let bs = BreakSetting {
        widow_orphan: true,
        ..Default::default()
    };
    let recovered = roundtrip_break_setting(bs.clone());
    assert!(
        recovered.widow_orphan,
        "widow_orphan=true must survive HWPX roundtrip; got {recovered:?}"
    );
}

/// `keep_with_next=true` survives a full HWPX write → read roundtrip.
#[test]
fn break_setting_keep_with_next_true_roundtrip() {
    let bs = BreakSetting {
        keep_with_next: true,
        ..Default::default()
    };
    let recovered = roundtrip_break_setting(bs.clone());
    assert!(
        recovered.keep_with_next,
        "keep_with_next=true must survive HWPX roundtrip; got {recovered:?}"
    );
}

/// All-false values survive a full HWPX write → read roundtrip (regression guard).
#[test]
fn break_setting_all_false_roundtrip() {
    let recovered = roundtrip_break_setting(BreakSetting::default());
    assert_eq!(
        recovered,
        BreakSetting::default(),
        "all-false BreakSetting must survive HWPX roundtrip unchanged"
    );
}

/// All-true values survive a full HWPX write → read roundtrip.
#[test]
fn break_setting_all_true_roundtrip() {
    let bs = BreakSetting {
        widow_orphan: true,
        keep_with_next: true,
        keep_lines: true,
        page_break_before: true,
    };
    let recovered = roundtrip_break_setting(bs.clone());
    assert_eq!(
        recovered, bs,
        "all-true BreakSetting must survive HWPX roundtrip unchanged"
    );
}
