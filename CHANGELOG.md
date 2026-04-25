# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-26

### Added
- **Phase 3**: charPr / paraPr / fontface reference tables in `header.xml` with
  IDRef linking between section paragraph runs and the header table entries.
- **Phase 4**: Style table (`hh:styles`) with Normal + Heading1-6, numeric
  `styleIDRef` and `charPrIDRef` values replacing string-form references for
  OWPML schema compliance.
- **Phase 5**: Sequential paragraph IDs (`id` attribute on `<hp:p>`), table
  block wrapping in `<hp:p>/<hp:run>/<hp:tbl>` hierarchy, heading-specific
  charPr entries with level-differentiated font heights.
- **Phase 6**: OWPML schema validation pass (`enable_schema=true`) with polaris
  DVC; fixed `breakSetting` attribute set, `align` horizontal/vertical attrs,
  `margin` child elements, `heading` / `border` / `autoSpacing` / `lineSpacing`
  required children in `paraPr`.
- **Phase 7**: `hh:borderFills` table with default entry (id=1), `slash` /
  `backSlash` / border / diagonal children; `borderFillIDRef` on every `charPr`
  entry; polaris_dvc rev pinning; schema validation expansion covering all
  writer-emitted elements.
- **Phase 8**: Inline code (`code: true`) mapped to distinct charPr entry with
  Courier New monospace font; metadata preservation (`hp:docInfo` with title and
  author in `content.hpf`); HWPX structural roundtrip tests; dead code audit;
  version bump to 0.3.0.

### Changed
- `CharPrKey` struct gains a `code` field; `from_inline()` now forces
  `Courier New` font for inline code spans, producing a distinct charPr ID.
- `generate_content_hpf()` emits `<hp:docInfo>` when the document carries
  title or author metadata.
- README library usage example updated to `hwp2md = "0.3"`.

### Fixed
- Font registration for inline code spans: the monospace font is now registered
  from the resolved `CharPrKey` (which overrides to `Courier New` for code)
  rather than from the raw IR inline's `font_name` field, ensuring the font
  table entry always exists when referenced by a charPr.

## [0.2.0] - 2026-04-23

### Added
- Ruby text control parsing (`ruby` ctrl_id): base text and phonetic annotation
  are extracted and emitted as `<ruby>` HTML in Markdown output.
- Lenient CFB fallback: corrupted or partially-written HWP files are now
  partially recovered instead of returning a hard error. Successfully read
  sections are returned with a warning.
- Distributed-document decryption: HWP files with the `distributed` flag set
  are now decrypted with AES-128 ECB before parsing.
- EQEDIT → LaTeX converter: `HWPTAG_EQEDIT` blocks are converted to fenced
  `$$` display-math blocks in Markdown output.
- Image embedding: `gso ` (GShapeObject) controls are resolved to their
  `BinData` streams and written as `![](data:image/...)` inline Markdown.
- Footnote and endnote parsing: `fn  ` and `en  ` controls are extracted and
  rendered as Markdown footnote references with collected definitions.
- Hyperlink extraction: `hyln` controls produce `[text](url)` Markdown links
  with URL sanitisation (scheme allow-list: `http`, `https`, `mailto`).
- Superscript and subscript character shapes are now mapped to `<sup>` / `<sub>`.
- Heading type detection from `HWPTAG_PARA_SHAPE` outlineLv attribute.
- HWPX colspan and rowspan extraction for table cells.
- Table parser now enforces a row-count cap (4 096) to prevent allocation DoS.
- Decompression-bomb guard: deflate output is capped at 256 MB; exceeding this
  limit returns `Hwp2MdError::DecompressionBomb`.
- CFB stream reads are capped at 256 MB (`MAX_CFB_STREAM`).
- GitHub Actions CI workflow (`cargo test`, `cargo clippy -- -D warnings`).
- 614 unit and integration tests (82%+ line coverage).

### Fixed
- 6 CRITICAL security issues resolved in the initial rewrite phase, including
  unbounded allocation on untrusted record sizes and integer overflow in
  dimension calculations.
- Infinite-loop in EQEDIT tokeniser on malformed input (recursion depth guard).
- Offset clamp in `read_utf16le_str` to prevent out-of-bounds reads.
- Heading level clamped to 1–6 (H7+ demoted to H6).
- URL deduplication: identical hyperlinks within the same paragraph are
  collapsed to a single link.
- `parse_records` extended-size field handling (0xFFF sentinel + follow-up u32).
- Zlib fallback path for deflate-wrapped CFB streams.
- Dead `CTRL_EQUATION`, `CTRL_HEADER`, `CTRL_FOOTER` constants removed.

### Changed
- Replaced all HWP-specific third-party crate dependencies with a
  self-contained parser based on the `cfb` (OLE2) and `zip` (HWPX) crates.
- `control.rs` split into category modules (`table`, `image`, `hyperlink`,
  `ruby`, `dispatcher`, `common`) for maintainability.
- `reader.rs` split into focused submodules.
- `tracing::debug!` demoted to `tracing::trace!` for high-frequency paths
  (table dims, image shapes, unhandled ctrl_id, list-item hints).
- `MAX_CFB_STREAM` deduplicated: defined once as `pub(crate)` in `reader.rs`
  and imported in `summary.rs`.
- Error types converted to `thiserror`-derived enum (`Hwp2MdError`) with
  structured variants instead of stringly-typed errors.
- Markdown inline escaping hardened: characters that start a Markdown
  construct at line start are escaped.

## [0.1.0] - 2026-04-02

### Added
- Initial project scaffolding with HWP 5.0 (CFB-based) and HWPX (ZIP/XML)
  parsing skeletons.
- Basic paragraph text extraction.
- CLI binary (`hwp2md`) with `--input`, `--output`, and `--style` flags.
- Bidirectional conversion: `HWP → Markdown` and `HWPX → Markdown`.
- Markdown → IR → Markdown roundtrip support.
