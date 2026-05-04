# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-05-04

### Added
- **Phase B-1**: YAML-based style template (`--style`) — `StyleTemplate` with
  page dimensions/margins, font overrides (body + code), and heading line
  spacing; `RefTables` carries resolved `code_font` and optional template;
  `CharPrKey` parameterized with `code_font`; `writer_section` applies style
  page layout; `writer_header` reads heading `line_spacing` from template;
  `serde_yml` dependency added; 4 integration tests + 5 unit tests.
- **Phase B-2**: GitHub-style task list support — `ListItem.checked: Option<bool>`
  field in IR; comrak `tasklist` extension enabled in MD parser; `NodeValue::TaskItem`
  mapped to checked state; MD writer emits `[x]`/`[ ]` prefixes; HWPX writer
  renders ☑ (U+2611) / ☐ (U+2610) checkbox characters; 15 new tests
  (6 parser + 5 writer + 4 HWPX).
- **Phase C-1**: `--check` subcommand — validates `.hwp`, `.hwpx`, `.md`, and
  `.markdown` files by parsing into IR without producing output; `check()` function
  in `convert.rs` dispatches on file extension; 14 new tests (8 unit + 6 CLI).
- **Phase D-1**: Criterion benchmarks — 5 conversion benchmarks (MD→IR, IR→MD,
  IR→HWPX, HWPX→IR, full roundtrip) with representative Korean document input;
  HTML reports via `criterion 0.5`.
- **Phase B-3**: Page break round-trip — new `Block::PageBreak` IR variant
  emitted by HWPX reader for `<hp:ctrl id="newPage|pageBreak|cnpb"/>`,
  serialised as `<!-- pagebreak -->` HTML comment in Markdown (invisible when
  rendered), recognised by the Markdown parser, and written back to HWPX as
  `<hp:p><hp:run><hp:ctrl id="newPage"/></hp:run></hp:p>`. Survives
  MD → HWPX → MD and HWPX → IR → MD round-trips.
- **Phase C-2**: `convert` subcommand — extension-driven format
  auto-detection. `hwp2md convert <input> <output>` infers direction from
  file extensions (`.hwp`/`.hwpx` ↔ `.md`/`.markdown`) and dispatches to
  the appropriate reader/writer; ambiguous or same-format pairs are rejected
  with a descriptive error.
- **Sprint 4 / Phase B-4**: Header/footer reader — `ir::Section` gains
  `header: Option<Vec<Block>>` and `footer: Option<Vec<Block>>`; HWPX
  `<hp:headerFooter>` → `<hp:header>` / `<hp:footer>` parsed into IR; MD
  output wraps content in `<!-- header -->` / `<!-- footer -->` markers;
  full round-trip writer support; 1 062 tests.
- **Sprint 4 / Phase M3**: `convert` subcommand `--force` flag — prevents
  accidental overwrite of existing output files; explicit opt-in required.
- **Sprint 4 / Phase C-3**: `ConvertOptions<'a>` builder — `assets_dir`,
  `frontmatter`, `style`, `force` consolidated into a single fluent API;
  re-exported from `lib.rs`.
- **Sprint 5 / Phase C-4**: Structured error variants — `OutputExists { path }`
  (overwrite guard) and `DrmProtected { path }` (encrypted HWP) replace
  prior `UnsupportedFormat` reuse; `convert_auto` and `hwp::reader` migrated.
- **Sprint 5 / Phase D-2**: Cross-platform CI — GitHub Actions matrix for
  ubuntu / windows / macos at MSRV 1.75; lint step isolated to ubuntu.
- **Sprint 5 / Phase D-3**: Coverage reporting — `cargo-tarpaulin` + Codecov
  upload; `codecov` badge added to README.
- **Sprint 6 / S6-01**: Markdown header/footer round-trip — `parse_markdown()`
  region state machine detects `<!-- header/footer -->` markers in the comrak
  AST and routes blocks into the correct `Section` bucket.
- **Sprint 6 / S6-02**: DRM integration test — synthetic CFB fixture with the
  `has_drm` bit set, constructed with `cfb::CompoundFile::create()`, exercising
  the full `DrmProtected` error path.
- **Sprint 6 / S6-03**: `<hp:headerFooter type="both|even|odd">` attribute —
  `header_footer_type: Option<String>` added to `ir::Section`; reader parses
  and writer emits the attribute.
- **Sprint 7 / S7-01**: Unclosed marker fallback — if `parse_markdown()` reaches
  EOF while inside a header or footer region, all accumulated blocks are drained
  back to body with a `tracing::warn!`; 4 new tests.
- **Sprint 7 / S7-02**: `HeaderFooterType` enum (`Both` / `Even` / `Odd` /
  `Other(String)`) replacing bare `Option<String>`; `#[serde(rename_all =
  "camelCase")]`; `as_str()` method; HWPX reader, writer, and tests migrated.
- **Sprint 7 / S7-03**: `<ruby>base<rt>annotation</rt></ruby>` HTML parsing
  in the Markdown parser — comrak HTML inline nodes are scanned for ruby
  structure and converted to `Inline::ruby` fields; nested-ruby guard prevents
  infinite recursion; 1 091 tests total (949 unit + 23 CLI + 46 integration +
  30 roundtrip).
- **Sprint 8 / S8-02**: `HeaderFooterType` `From<String>` normalizer — serde
  round-trip asymmetry fixed; `From<&str>` impl normalizes known values,
  `From<String>` delegates to it; HWPX handler simplified to `.into()`.
- **Sprint 9 / S9-01**: `From<&str>` edge case tests — empty string, whitespace,
  capitalized, and str-ref known-value coverage.
- **Sprint 9 / S9-03**: `batch` CLI subcommand — directory-level HWP/HWPX → MD
  conversion; `--force` overwrite flag; input validation for missing or
  non-directory paths; 8 integration tests.
- **Sprint 10 / S10-01**: `batch` hidden-file and symlink guards — dotfiles
  (`.DS_Store`, `.env`, etc.) and symlinks are silently skipped during
  directory walk; 2 new CLI tests.

### Changed
- **Sprint 10 / S10-02**: `batch` output now reports separate counters:
  "N converted, M skipped, F failed" — previously-existing files that are
  skipped without `--force` are counted as "skipped" rather than "errors".
- **Sprint 10 / S10-03**: `hwpx/writer.rs` split (822 → 390 + 453 lines);
  image-asset collection, base64, and static XML generators extracted to
  `writer_content.rs`.
- **Sprint 10 / S10-04**: Orphaned `reader_tests.rs` (1 296 lines) deleted;
  4 page-break tests recovered to `reader_tests_structure.rs`.
- **Sprint 10 / S10-05**: `hwpx_roundtrip.rs` split (1 288 → 275 + 536 +
  630 lines) into three integration test files.
- **Sprint 8 / S8-01**: `parser_tests.rs` split (1 622 → 521 + 390 + 730 lines)
  into core, inline, and marker test modules; `pub(super)` shared helpers.
- **Sprint 9 / S9-02**: `convert.rs` inline tests extracted to `convert_tests.rs`
  (1 316 → 508 + 808 lines); 54 tests preserved.
- **Sprint 9 / S9-04**: `writer_tests.rs` split (1 395 → 718 + 705 lines) into
  core and paragraph test modules; 97 tests preserved.
- **Phase A-1**: `ParseContext` god object (37 flat fields) refactored into 5
  sub-structs: `FormattingState`, `TableState`, `ListState`, `FootnoteState`,
  `PageLayoutState`; `flush_inlines_to_blocks` simplified from 11 parameters
  to 4 (buffers + `&FormattingState`); `make_inline()` helper extracted.
- **Phase A-2**: Page layout parsing deduplication — `pageSize`, `margin`, and
  `pagePr` attribute parsing extracted into `PageLayoutState` methods
  (`parse_page_size`, `parse_margin`, `parse_page_pr`), eliminating 50-line
  duplication between `handle_start_element` and `handle_empty_element`.
- `build_list` merged identical `depth == 0` and `items.is_empty()` branches
  (clippy `if_same_then_else` fix).
- `push_block_scoped` / `flush_active_paragraph_scope` gained header/footer
  scope routing so that blocks accumulated in those regions land in the correct
  `Section` field rather than body.

### Fixed
- `serde_yaml` (deprecated) replaced with `serde_yml 0.0.12`.
- `PageLayout` now derives `Copy` (all fields are `Option<u32>` or `bool`).
- `StyleTemplate::validate()` rejects zero `width`, `height`, and `line_spacing`.
- `check()` now rejects Markdown files >256 MB (`MAX_MD_FILE_SIZE`) before reading.
- **Sprint 3 / M1**: dedicated `Hwp2MdError::FileTooLarge { path, size, limit }`
  variant replaces the misleading `UnsupportedFormat` reuse for the Markdown
  size guard, distinguishing resource-limit violations from extension errors.
- **Sprint 5 review**: `read_hwp()` lenient CFB fallback no longer silently
  swallows `DrmProtected` errors — the error is re-surfaced after partial
  section collection.
- **Sprint 7 review**: multi-region drain correctly flushes both header and
  footer block buffers on unclosed-marker fallback; nested `<ruby>` guard
  prevents infinite recursion in the HTML ruby parser.

## [0.4.0] - 2026-04-26

### Added
- **Phase A-1**: Image embedding pipeline — file path and data URI images
  are read, base64-decoded or loaded from disk, and embedded as `BinData/`
  entries in the HWPX ZIP; `binaryItemIDRef` references are emitted on
  `<hp:pic>` elements; HTTP/HTTPS URLs are preserved as external references.
- **Phase A-2**: List writer with bullet/numbering — `<hh:numbering>`
  definitions (BULLET id=1, DIGIT id=2) emitted in header.xml;
  `numPrIDRef` set on list-item paragraphs; nested list support via
  `write_list_items()` with depth-based `paraPrIDRef` (id=2 depth-0,
  id=3 depth-1+); 23 new list tests.
- **Phase A-3**: BlockQuote visual indentation — `paraPr id=1` with left
  margin 800 HWP units (~20mm); `quote_depth` parameter threaded through
  `write_block()`; 6 new blockquote tests.
- **Phase A-4**: Footnote OWPML structure — `<hp:fn noteId>` wrapping
  around footnote content blocks; 7 new footnote tests.
- **Phase B-1**: HWPX nested list reader — `StagedBlock` enum for
  list-paragraph grouping; `group_list_paragraphs` collapses flat
  sequences into nested `Block::List`; `paraPrIDRef`/`numPrIDRef`
  parsing from `<hp:p>`; 13 new reader list tests.
- **Phase B-3**: HWPX lenient XML error recovery — malformed section
  XML parsing continues with partial results; missing section files
  skipped with warning; missing attributes use defaults.
- **Phase C-3**: Code block language preservation — language hint stored
  as `<!-- hwp2md:lang:X -->` XML comment in HWPX; reader parses it back;
  MD→HWPX→MD roundtrip preserves language info; `-->` injection sanitized.
- **Phase B-2**: HWP binary list recognition — two-tier detection:
  `numbering_id` from ParaShape records, then text-prefix heuristics
  (●■▶•-* bullets, 1./2)/a. ordered); year-like prefix rejection;
  `StagedBlock` grouping; 32 new HWP list tests.
- **Phase B-4**: Page layout parsing — `PageLayout` IR struct (width,
  height, landscape, margins); HWPX reader parses `<hp:secPr>` →
  `<hp:pagePr>` → `<hp:pageSize>`/`<hp:margin>`; writer emits `<hp:secPr>`
  with A4 portrait defaults; 11 new page layout tests.
- **Phase C-1**: Heading paraPr (id=4) with 180% line spacing; `ParaPrConfig`
  struct replacing bare `(id, left_margin)` parameters; 14 new paraPr tests.
- **Phase D-1**: Comprehensive roundtrip integration tests — 37 tests covering
  all block types (paragraph, H1-H6, ordered/unordered/nested list, table,
  code block, horizontal rule, blockquote, image, footnote, inline formatting,
  combined document).

### Changed
- `paraPr` table expanded from 2 to 5 entries (normal, blockquote,
  list-depth-0, list-depth-1+, heading).
- `writer_tests_roundtrip.rs` split: golden tests → `writer_tests_golden.rs`,
  code language tests → `writer_tests_code_lang.rs`.
- `ir::Section` now carries `page_layout: Option<PageLayout>`.
- Headings use `paraPrIDRef="4"` (180% line spacing) outside blockquotes.

### Fixed
- Image filename collision: counter suffix dedup (`photo_2.png`) instead
  of silent drop; `unique_entry_name` bounded to 10,000 iterations.
- XML comment injection: code language `-->` sanitized via `--` collapse.
- `flush_paragraph` marked `#[cfg(test)]` instead of `#[allow(dead_code)]`.

## [0.3.1] - 2026-04-26

### Added
- **Phase 16**: Golden file test (`golden_comprehensive_document_structure`):
  validates internal ZIP XML structure (section0.xml, header.xml, content.hpf,
  mimetype) of generated HWPX archives; OWPML schema validation re-verified
  with polaris DVC after inline charPr changes.
- **Phase 15**: `faceNameIDRef` attribute emitted on section-level inline
  `<hp:charPr>` when the inline carries a `font_name`, completing the font
  name write-to-read roundtrip; 4 new font name roundtrip tests.
- **Phase 14**: Section-level inline `<hp:charPr>` emission in
  `write_inline_charpr()`: bold, italic, underline, strikeout, superscript,
  subscript, and color attributes are now written inside `<hp:run>` elements
  for OWPML conformance; 16 new bold/italic/underline/strike/color roundtrip
  tests; 3 new `xml_escape_content` tests (apostrophe, all special chars,
  passthrough).
- **Phase 13**: Font name reader: `parse_face_names()` from header.xml
  `<hh:fontface>` entries; `faceNameIDRef` / `hangulIDRef` resolution in
  `apply_charpr_attrs`; `with_font_name()` builder method on `Inline`; 9 new
  font resolution tests; README updated with Phase 9-12 features (hyperlinks,
  ruby, footnote_ref, inline code, metadata).
- **Phase 12**: `writer_tests.rs` split into 8 topic-based test modules
  (`writer_tests_charpr`, `writer_tests_section`, `writer_tests_metadata`,
  `writer_tests_hyperlink`, `writer_tests_ruby`, `writer_tests_footnote`,
  `writer_tests_roundtrip`); ruby + hyperlink combination test added.
- `xml_escape_content` now covers the complete set of XML 1.0 predefined
  entities (`&`, `<`, `>`, `"`, `'`).

### Changed
- `..Default::default()` struct update syntax eliminated across the entire
  codebase (`md/parser.rs`, `hwp/convert.rs`) — all fields are now set
  explicitly.

### Fixed
- `trim_start_matches('#')` replaced with `strip_prefix('#')` in color
  attribute emission to prevent stripping multiple `#` characters.
- Font name propagation in flush paths (`flush_paragraph`,
  `flush_cell_paragraph`, `flush_list_item_paragraph`,
  `flush_footnote_paragraph`) now chains `.with_font_name()`.
- Replaced `.unwrap()` with `if let` pattern in `write_inlines` to prevent
  potential panic on link URL access.
- Empty text inline run guard in `writer_section.rs`: zero-length text runs are
  skipped, preventing emission of empty `<hp:t/>` elements.
- Dead code removed: `InlineStyle.code` field (superseded by the `CharPrKey`
  path introduced in Phase 8).
- Broken intra-doc link in `md/mod.rs` corrected.

## [0.3.0] - 2026-04-26

### Added
- **Phase 11**: Crate-level `//!` documentation on `lib.rs`; `///` doc comments
  on all public types in `ir.rs`, `error.rs`, and all public functions in
  `convert.rs`; `#![warn(missing_docs)]` lint enabled; `xml_escape_content`
  extended with `"` → `&quot;` escaping for defense-in-depth.
- **Phase 10**: Ruby annotation writer (`hp:ruby` / `baseText` / `rubyText`);
  `footnote_ref` writer emitting `hp:noteRef`; `Inline` builder pattern with
  `with_formatting` / `with_link` / `with_ruby` constructors; ruby formatting
  propagation fix ensuring annotation runs inherit the base run's charPr.
- **Phase 9**: Superscript/subscript writer using the `supscript` charPr
  attribute; hyperlink reader and writer using `fieldBegin` / `fieldEnd`
  controls; writer module split into three focused submodules; reader run-start
  reset fix preventing stale formatting from leaking across paragraphs.
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
