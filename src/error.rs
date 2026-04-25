//! Error types for the `hwp2md` crate.

use thiserror::Error;

/// All errors that can be returned by this crate.
#[derive(Error, Debug)]
pub enum Hwp2MdError {
    /// The file extension is not `.hwp`, `.hwpx`, `.md`, or `.markdown`.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// A structural or data error was encountered while reading a HWP 5.0 file.
    #[error("HWP parse error: {0}")]
    HwpParse(String),

    /// A structural or data error was encountered while reading an HWPX file.
    #[error("HWPX parse error: {0}")]
    HwpxParse(String),

    /// The Markdown source could not be parsed.
    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    /// An error occurred while generating HWPX output.
    #[error("HWPX write error: {0}")]
    HwpxWrite(String),

    /// A zlib/deflate stream could not be decompressed.
    #[error("decompression error: {0}")]
    Decompress(String),

    /// Decompressed output exceeded the 256 MB safety limit.
    #[error("decompression bomb: output exceeded {0} bytes")]
    DecompressionBomb(u64),

    /// A binary record in the HWP stream has an invalid or inconsistent header.
    #[error("invalid record: {0}")]
    InvalidRecord(String),

    /// A text encoding conversion failed (e.g. UTF-16LE decode error).
    #[error("encoding error: {0}")]
    Encoding(String),

    /// An underlying I/O operation failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// An error was returned by the `quick-xml` writer.
    #[error("XML write error: {0}")]
    XmlWrite(#[from] quick_xml::Error),

    /// An error was returned by the `zip` archive library.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
}
