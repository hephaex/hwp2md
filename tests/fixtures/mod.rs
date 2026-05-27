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
    bin_data_entries: Vec<(String, Vec<u8>)>, // (name, data)
    /// Optional raw XML that is embedded inside `<hp:header>` within a
    /// `<hp:headerFooter>` element appended to the section body.
    header_xml: Option<String>,
    /// Optional raw XML that is embedded inside `<hp:footer>` within a
    /// `<hp:headerFooter>` element appended to the section body.
    footer_xml: Option<String>,
}

impl HwpxFixture {
    pub fn new() -> Self {
        Self {
            title: None,
            author: None,
            section_body: String::new(),
            bin_data_entries: Vec::new(),
            header_xml: None,
            footer_xml: None,
        }
    }

    pub fn title(mut self, t: &str) -> Self {
        self.title = Some(t.to_owned());
        self
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn bin_data(mut self, name: &str, data: Vec<u8>) -> Self {
        self.bin_data_entries.push((name.to_owned(), data));
        self
    }

    /// Embed raw XML paragraphs inside a `<hp:header>` element.
    ///
    /// The XML is placed inside a `<hp:headerFooter>` wrapper that is appended
    /// to the section body.  Use this to test lang-hint comment handling in
    /// the header parsing path.
    ///
    /// Calling this more than once replaces the previous header XML.
    #[allow(dead_code)]
    pub fn with_header_xml(mut self, xml: &str) -> Self {
        self.header_xml = Some(xml.to_owned());
        self
    }

    /// Embed raw XML paragraphs inside a `<hp:footer>` element.
    ///
    /// Analogous to [`with_header_xml`] but routes content into
    /// `<hp:footer>` instead.  The `<hp:headerFooter>` wrapper is shared with
    /// any header XML set via `with_header_xml`.
    ///
    /// Calling this more than once replaces the previous footer XML.
    #[allow(dead_code)]
    pub fn with_footer_xml(mut self, xml: &str) -> Self {
        self.footer_xml = Some(xml.to_owned());
        self
    }

    /// Append a lang-hint comment followed by a paragraph to the section body.
    ///
    /// The `<!-- hwp2md:lang:{lang} -->` comment causes the next paragraph to be
    /// read as a `CodeBlock` with the given language.  Pass an empty string for a
    /// no-language code block (`<!-- hwp2md:lang: -->`).
    #[allow(dead_code)]
    pub fn with_lang_hint_paragraph(self, lang: &str, text: &str) -> Self {
        self.section(&lang_hint_para_xml(lang, text))
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
        let section_xml = build_section_xml_with_hf(
            &self.section_body,
            self.header_xml.as_deref(),
            self.footer_xml.as_deref(),
        );
        zip.write_all(section_xml.as_bytes()).unwrap();

        for (name, data) in &self.bin_data_entries {
            zip.start_file(format!("BinData/{name}"), stored).unwrap();
            zip.write_all(data).unwrap();
        }

        let inner = zip.finish().unwrap();
        inner.into_inner()
    }

    /// Write the fixture to a temporary file and return the path together with
    /// the `TempDir` guard (drop the guard to delete the file).
    #[allow(dead_code)]
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

fn build_section_xml_with_hf(
    body: &str,
    header_xml: Option<&str>,
    footer_xml: Option<&str>,
) -> String {
    // Build the optional <hp:headerFooter> element when at least one of header
    // or footer XML is provided.
    let hf_block = match (header_xml, footer_xml) {
        (None, None) => String::new(),
        (h, f) => {
            let header_part = h.map_or(String::new(), |xml| {
                format!("  <hp:header>\n{xml}\n  </hp:header>")
            });
            let footer_part = f.map_or(String::new(), |xml| {
                format!("  <hp:footer>\n{xml}\n  </hp:footer>")
            });
            format!("<hp:headerFooter>\n{header_part}{footer_part}\n</hp:headerFooter>")
        }
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
{body}
{hf_block}
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
// Lang-hint comment helpers
// ---------------------------------------------------------------------------

/// Returns a `<!-- hwp2md:lang:{lang} -->` XML comment string for use in HWPX section XML.
///
/// Pass an empty string for a no-language code block (`<!-- hwp2md:lang: -->`).
pub fn lang_hint_comment(lang: &str) -> String {
    format!("<!-- hwp2md:lang:{lang} -->")
}

/// Returns XML for a lang-hint comment followed by a paragraph, suitable for
/// embedding in an HWPX section body.  Produces a `CodeBlock` in the IR with
/// the given language when parsed by `hwpx::read_hwpx`.
pub fn lang_hint_para_xml(lang: &str, text: &str) -> String {
    format!("{}\n{}", lang_hint_comment(lang), para_xml(text))
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
#[allow(dead_code)]
pub fn styled_run_xml(text: &str) -> String {
    let id = next_para_id();
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:charPr bold="true" italic="true"/><hp:t>{}</hp:t></hp:run></hp:p>"#,
        xml_escape(text)
    )
}
