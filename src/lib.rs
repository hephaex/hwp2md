//! Bidirectional converter between HWP/HWPX documents and Markdown.
//!
//! Supports HWP 5.0 (binary OLE2-based) and HWPX (ZIP/XML-based) as input or
//! output, with a language-agnostic intermediate representation ([`ir`]) in
//! between.
//!
//! # Quick start
//!
//! ```no_run
//! use std::path::Path;
//!
//! // HWP/HWPX → Markdown
//! hwp2md::convert::to_markdown(
//!     Path::new("document.hwpx"),
//!     Some(Path::new("document.md")),
//!     None,   // assets_dir
//!     false,  // frontmatter
//! ).expect("conversion failed");
//!
//! // Markdown → HWPX
//! hwp2md::convert::to_hwpx(
//!     Path::new("document.md"),
//!     Some(Path::new("document.hwpx")),
//!     None,   // style template
//! ).expect("conversion failed");
//! ```
//!
//! # Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`convert`] | Top-level conversion entry points |
//! | [`ir`] | Intermediate representation (Document, Block, Inline, …) |
//! | [`error`] | [`Hwp2MdError`] error type |
//! | [`hwp`] | HWP 5.0 reader (CFB/OLE2) |
//! | [`hwpx`] | HWPX reader and writer (ZIP/XML) |
//! | [`md`] | Markdown reader and writer |
//!
//! See the [README](https://github.com/CasterLink/hwp2md#readme) for detailed
//! usage, CLI reference, and format support matrix.
#![warn(missing_docs)]

pub mod convert;
pub mod error;
pub mod hwp;
pub mod hwpx;
pub mod ir;
pub mod md;
pub(crate) mod url_util;

pub use error::Hwp2MdError;
