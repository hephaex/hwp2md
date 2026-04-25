//! Markdown parser and writer built on top of the `pulldown_cmark` ecosystem.
#![allow(missing_docs)]

mod parser;
mod writer;

pub use parser::parse_markdown;
pub use writer::write_markdown;
