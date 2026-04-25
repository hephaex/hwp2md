# hwp2md

[![Crates.io](https://img.shields.io/crates/v/hwp2md.svg)](https://crates.io/crates/hwp2md)
[![CI](https://github.com/hephaex/hwp2md/actions/workflows/ci.yml/badge.svg)](https://github.com/hephaex/hwp2md/actions/workflows/ci.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)

**hwp2md** is a bidirectional converter between Korean 한글(Hangul) document formats — HWP 5.0 (binary OLE2) and HWPX (XML/ZIP) — and CommonMark-compatible Markdown. It ships as both a command-line tool and a Rust library, making it straightforward to integrate document conversion into build pipelines, static-site generators, or document-management workflows that need to exchange content with the Korean public-sector ecosystem.

## Features

- HWP 5.0 binary format (OLE2/CFB container) to Markdown
- HWPX (ZIP + XML) to Markdown
- Markdown to HWPX (write path; binary HWP output is not yet supported)
- Headings levels 1-6, paragraphs, bold, italic, underline, strikethrough, inline code
- Superscript and subscript inline styles
- Hyperlinks (reader + writer; `fieldBegin`/`fieldEnd` control pattern)
- Ruby annotations (reader + writer; rendered as `<ruby>base<rt>annotation</rt></ruby>`)
- Footnote references (writer; `hp:noteRef` emission)
- Inline code mapped to monospace font (Courier New) in HWPX output
- Metadata preservation (title, author) round-trips through HWPX `hp:docInfo`
- Ordered and unordered lists with nested items
- Tables (GFM pipe syntax; colspan/rowspan fall back to HTML)
- Fenced code blocks with language annotation
- Block quotes
- Images with alt text; optional extraction to an assets directory
- Footnotes (`[^id]` syntax)
- Math expressions — HWP EqEdit equations converted to LaTeX (`$...$` / `$$...$$`)
- Document metadata (title, author, date) optionally emitted as YAML front matter
- `info` subcommand for quick document inspection without conversion
- YAML style templates for Markdown-to-HWPX output (interface defined; implementation in progress)
- Structured intermediate representation (IR) exposed as a public library API
- Release builds with LTO and symbol stripping for minimal binary size

## Installation

### From crates.io

```bash
cargo install hwp2md
```

### From source

```bash
git clone https://github.com/hephaex/hwp2md.git
cd hwp2md
cargo build --release
# Binary at: target/release/hwp2md
```

Minimum supported Rust version: **1.75**.

## CLI Usage

### Convert HWP or HWPX to Markdown

```bash
# Write Markdown to stdout
hwp2md to-md report.hwp

# Write to a file
hwp2md to-md report.hwp -o report.md

# Extract embedded images alongside the Markdown
hwp2md to-md report.hwpx -o report.md --assets-dir ./images

# Include document metadata as YAML front matter
hwp2md to-md report.hwp -o report.md --frontmatter
```

### Convert Markdown to HWPX

```bash
# Output file defaults to input name with .hwpx extension
hwp2md to-hwpx draft.md

# Specify output path
hwp2md to-hwpx draft.md -o final.hwpx

# Apply a YAML style template (interface available; full implementation pending)
hwp2md to-hwpx draft.md -o final.hwpx --style corporate.yaml
```

### Inspect a document without converting

```bash
hwp2md info report.hwp
# File: report.hwp
# Format: hwp
# Title: Annual Report 2025
# Author: Jane Doe
# Sections: 4
# Blocks: 87
# Characters: ~12430
# Assets: 6
```

### Logging verbosity

The `--log-level` flag accepts any `tracing` filter string (default: `info`):

```bash
hwp2md --log-level debug to-md report.hwp
hwp2md --log-level warn  to-md report.hwp -o report.md
```

## Library Usage

Add to `Cargo.toml`:

```toml
[dependencies]
hwp2md = "0.3"
```

### Convert a file

```rust
use hwp2md::convert;

fn main() -> anyhow::Result<()> {
    // HWP or HWPX to Markdown (written to stdout when output is None)
    convert::to_markdown(
        "report.hwpx".as_ref(),
        Some("report.md".as_ref()),
        Some("assets/".as_ref()),
        true, // emit YAML front matter
    )?;

    // Markdown to HWPX
    convert::to_hwpx(
        "draft.md".as_ref(),
        Some("draft.hwpx".as_ref()),
        None, // style template
    )?;

    Ok(())
}
```

### Work with the intermediate representation

```rust
use hwp2md::{hwp, hwpx, md, ir};

// Parse a document into the IR
let doc: ir::Document = hwpx::read_hwpx("report.hwpx".as_ref())?;

// Inspect metadata
if let Some(title) = &doc.metadata.title {
    println!("Title: {title}");
}

// Iterate blocks in the first section
for block in &doc.sections[0].blocks {
    if let ir::Block::Heading { level, inlines } = block {
        let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
        println!("H{level}: {text}");
    } else if let ir::Block::Table { rows, col_count } = block {
        println!("Table {col_count} cols x {} rows", rows.len());
    }
}

// Render back to Markdown
let markdown = md::write_markdown(&doc, false);
println!("{markdown}");
```

### Parse Markdown into the IR

```rust
use hwp2md::md;

let source = std::fs::read_to_string("document.md")?;
let doc = md::parse_markdown(&source);
println!("{} sections", doc.sections.len());
```

## Format Support Matrix

| Feature | HWP 5.0 -> MD | HWPX -> MD | MD -> HWPX |
|---------|:---:|:---:|:---:|
| Headings (H1-H6) | yes | yes | yes |
| Paragraphs | yes | yes | yes |
| Bold / Italic | yes | yes | yes |
| Underline | yes | yes | yes |
| Strikethrough | yes | yes | yes |
| Inline code | yes | yes | yes |
| Superscript / Subscript | yes | yes | yes |
| Hyperlinks | yes | yes | yes |
| Ruby annotations | yes | yes | yes |
| Ordered lists | yes | yes | yes |
| Unordered lists | yes | yes | yes |
| Nested lists | yes | yes | yes |
| Tables | yes | yes | yes |
| Images (extract) | yes | yes | yes |
| Fenced code blocks | yes | yes | yes |
| Block quotes | yes | yes | yes |
| Footnotes | yes | yes | yes |
| Math (LaTeX) | yes | yes | yes |
| YAML front matter | yes | yes | n/a |
| Multi-column layout | flattened | flattened | n/a |
| Headers / footers | skipped | skipped | planned |
| DRM-protected HWP | no | no | n/a |
| MD -> HWP binary | n/a | n/a | no |

## Architecture

```
HWP 5.0 (.hwp)  ──── hwp::read_hwp()   ──┐
                                           ├──> ir::Document ──> md::write_markdown() ──> Markdown
HWPX (.hwpx)    ──── hwpx::read_hwpx() ──┘
                                           ┌── ir::Document <── md::parse_markdown() <── Markdown
                                           └──> hwpx::write_hwpx() ──> HWPX (.hwpx)
```

The conversion pipeline is decoupled through a format-neutral intermediate representation (`ir::Document`). Every reader produces an `ir::Document`; every writer consumes one. This keeps format-specific code isolated and makes it straightforward to add new input or output formats in the future.

### Key types in `ir`

| Type | Description |
|------|-------------|
| `Document` | Root: metadata + sections + extracted assets |
| `Metadata` | Title, author, creation/modification date, subject, keywords |
| `Section` | Ordered sequence of `Block` values |
| `Block` | `Heading`, `Paragraph`, `Table`, `CodeBlock`, `BlockQuote`, `List`, `Image`, `HorizontalRule`, `Footnote`, `Math` |
| `Inline` | Leaf text with style flags (bold, italic, underline, strikethrough, code, superscript, subscript, link, footnote reference, ruby annotation) |
| `Asset` | Embedded binary (image or other media) with MIME type |

### Crate layout

```
src/
  main.rs          CLI entry point (clap)
  lib.rs           Public re-exports
  convert.rs       High-level convert::to_markdown / to_hwpx / show_info
  ir.rs            Intermediate representation types
  error.rs         Hwp2MdError enum (thiserror)
  hwp/             HWP 5.0 reader (CFB container, record parser, EqEdit)
  hwpx/            HWPX reader + writer (ZIP + quick-xml)
  md/              Markdown parser (comrak) + writer
tests/             Integration tests
```

## Known Limitations

- DRM-protected (배포용) HWP files are not supported.
- Multi-column (다단) layouts are flattened to a single column.
- Tables with non-trivial `colspan`/`rowspan` fall back to raw HTML in the Markdown output.
- Headers and footers are currently skipped.
- Writing back to the binary HWP 5.0 format (MD -> HWP) is not supported; only HWPX output is available.
- The `--style` YAML template option for `to-hwpx` is accepted by the CLI but not yet applied.

## Contributing

Bug reports and pull requests are welcome at <https://github.com/hephaex/hwp2md>.

Before submitting a patch:

1. Run `cargo fmt` and `cargo clippy -- -D warnings`.
2. Ensure `cargo test --all-targets` passes.
3. Add or update tests for any changed behaviour.

## License

Copyright (c) 2026 Mario Cho \<hephaex@gmail.com\>

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 3 only.

See [LICENSE](LICENSE) for the full text.
