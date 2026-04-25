/// In-memory HWPX fixture builder.
///
/// HWPX is a ZIP container whose required entries are:
///   - `mimetype`              (stored, not deflated)
///   - `META-INF/container.xml`
///   - `Contents/content.hpf`
///   - `Contents/section0.xml`
///
/// Optional but recognised:
///   - `Contents/header.xml`  (title / author metadata)
///
/// # Usage
///
/// ```rust,no_run
/// let hwpx_bytes = HwpxFixture::new()
///     .title("My Doc")
///     .section(r#"<hp:p><hp:run><hp:t>Hello</hp:t></hp:run></hp:p>"#)
///     .build();
/// ```
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Builds a minimal, well-formed HWPX file in memory.
pub struct HwpxFixture {
    title: Option<String>,
    author: Option<String>,
    /// Raw XML snippets (paragraph / table / etc.) that go *inside* `<hs:sec …>`.
    section_body: String,
}

impl HwpxFixture {
    pub fn new() -> Self {
        Self {
            title: None,
            author: None,
            section_body: String::new(),
        }
    }

    pub fn title(mut self, t: &str) -> Self {
        self.title = Some(t.to_owned());
        self
    }

    pub fn author(mut self, a: &str) -> Self {
        self.author = Some(a.to_owned());
        self
    }

    /// Append raw XML that will be embedded inside `<hs:sec>`.
    /// Pass one or more paragraph / table XML strings.
    pub fn section(mut self, xml: &str) -> Self {
        self.section_body.push_str(xml);
        self
    }

    /// Produce the in-memory bytes of a valid HWPX ZIP.
    pub fn build(self) -> Vec<u8> {
        let buf = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buf);

        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        // mimetype — must be stored, not compressed
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/hwp+zip").unwrap();

        // META-INF/container.xml
        zip.start_file("META-INF/container.xml", deflated).unwrap();
        zip.write_all(CONTAINER_XML.as_bytes()).unwrap();

        // Contents/header.xml (optional metadata)
        zip.start_file("Contents/header.xml", deflated).unwrap();
        zip.write_all(
            build_header_xml(
                self.title.as_deref().unwrap_or(""),
                self.author.as_deref().unwrap_or(""),
            )
            .as_bytes(),
        )
        .unwrap();

        // Contents/content.hpf
        zip.start_file("Contents/content.hpf", deflated).unwrap();
        zip.write_all(CONTENT_HPF.as_bytes()).unwrap();

        // Contents/section0.xml
        zip.start_file("Contents/section0.xml", deflated).unwrap();
        zip.write_all(build_section_xml(&self.section_body).as_bytes())
            .unwrap();

        let inner = zip.finish().unwrap();
        inner.into_inner()
    }

    /// Write the fixture to a temporary file and return the path together with
    /// the `TempDir` guard (drop the guard to delete the file).
    pub fn write_to_tempfile(self) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixture.hwpx");
        let bytes = self.build();
        std::fs::write(&path, &bytes).expect("write fixture");
        (dir, path)
    }
}

// ---------------------------------------------------------------------------
// Static XML templates
// ---------------------------------------------------------------------------

const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0">
  <rootfiles>
    <rootfile full-path="Contents/content.hpf" media-type="application/hwp+xml"/>
  </rootfiles>
</container>"#;

const CONTENT_HPF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<hp:HWPMLPackage xmlns:hp="http://www.hancom.co.kr/hwpml/2011/packageInfo">
  <hp:compatibledocument version="1.1"/>
  <hp:contents>
    <hp:item href="section0.xml" type="Section"/>
  </hp:contents>
</hp:HWPMLPackage>"#;

fn build_header_xml(title: &str, author: &str) -> String {
    let title_escaped = xml_escape(title);
    let author_escaped = xml_escape(author);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<head xmlns="http://www.hancom.co.kr/hwpml/2011/head">
  <title>{title_escaped}</title>
  <creator>{author_escaped}</creator>
</head>"#
    )
}

fn build_section_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
{body}
</hs:sec>"#
    )
}

/// Minimal XML character escaping for attribute/text values inserted into
/// fixture XML.  Only the five predefined XML entities are handled; this is
/// sufficient for test fixture data that contains no raw `<` / `>` in content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// Pre-built XML snippets for common document patterns
// ---------------------------------------------------------------------------

/// Paragraph ID counter for fixture snippets.
static FIXTURE_PARA_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn next_para_id() -> u32 {
    FIXTURE_PARA_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A single paragraph with plain text.
pub fn para_xml(text: &str) -> String {
    let id = next_para_id();
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p>"#,
        xml_escape(text)
    )
}

/// A heading paragraph at the given level (1–6).
pub fn heading_xml(level: u8, text: &str) -> String {
    debug_assert!((1..=6).contains(&level), "heading level must be 1–6");
    let id = next_para_id();
    format!(
        r#"<hp:p id="{id}" hp:styleIDRef="{level}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p>"#,
        xml_escape(text)
    )
}

/// A 2×2 table with four plain-text cells, wrapped in a paragraph container.
pub fn table_2x2_xml(r0c0: &str, r0c1: &str, r1c0: &str, r1c1: &str) -> String {
    let wrapper_id = next_para_id();
    let c0 = next_para_id();
    let c1 = next_para_id();
    let c2 = next_para_id();
    let c3 = next_para_id();
    format!(
        r#"<hp:p id="{wrapper_id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:tbl rowCnt="2" colCnt="2">
  <hp:tr>
    <hp:tc><hp:p id="{c0}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p></hp:tc>
    <hp:tc><hp:p id="{c1}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p></hp:tc>
  </hp:tr>
  <hp:tr>
    <hp:tc><hp:p id="{c2}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p></hp:tc>
    <hp:tc><hp:p id="{c3}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run></hp:p></hp:tc>
  </hp:tr>
</hp:tbl></hp:run></hp:p>"#,
        xml_escape(r0c0),
        xml_escape(r0c1),
        xml_escape(r1c0),
        xml_escape(r1c1)
    )
}

/// A bold + italic inline run (includes inline charPr for reader compatibility).
pub fn styled_run_xml(text: &str) -> String {
    let id = next_para_id();
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:charPr bold="true" italic="true"/><hp:t>{}</hp:t></hp:run></hp:p>"#,
        xml_escape(text)
    )
}
