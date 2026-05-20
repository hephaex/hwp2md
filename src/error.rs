//! Error types for the `hwp2md` crate.

use std::path::PathBuf;
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

    /// An error occurred while generating HWPX output.
    #[error("HWPX write error: {0}")]
    HwpxWrite(String),

    /// A zlib/deflate stream could not be decompressed.
    #[error("decompression error: {0}")]
    Decompress(String),

    /// Decompressed output exceeded the 256 MB safety limit.
    #[error("decompression bomb: output exceeded {0} bytes")]
    DecompressionBomb(u64),

    /// An input file was rejected because it exceeded a hard size limit.
    ///
    /// Carries the file path, observed size, and configured limit so callers
    /// can render a precise message and so the error type cleanly distinguishes
    /// resource-limit violations from genuine format-detection failures
    /// ([`Hwp2MdError::UnsupportedFormat`]).
    ///
    /// `path` is stored as a [`PathBuf`] so callers can re-use it
    /// programmatically (logging, structured error reporting); the
    /// [`std::fmt::Display`] implementation renders the path lossy via
    /// `Path::display`.
    #[error("file too large: {} ({size} bytes > {limit} bytes limit)", path.display())]
    FileTooLarge {
        /// Path of the offending input file.
        path: PathBuf,
        /// Observed size of the file in bytes.
        size: u64,
        /// Configured maximum size in bytes for this code path.
        limit: u64,
    },

    /// The output file already exists and `--force` was not specified.
    ///
    /// Returned by [`convert_auto`](crate::convert::convert_auto) and
    /// [`ConvertOptions::execute`](crate::convert::ConvertOptions) when the
    /// output path is occupied and `force` is `false`.  Callers should
    /// surface the path to users and suggest passing `--force`.
    #[error("output file '{}' already exists; use --force to overwrite", path.display())]
    OutputExists {
        /// Path of the pre-existing output file.
        path: PathBuf,
    },

    /// The HWP file is DRM-protected (encrypted distribution document).
    ///
    /// HWP files with the `has_drm` flag set in the file header cannot be
    /// parsed without a valid decryption key.  This error is distinct from
    /// [`Hwp2MdError::HwpParse`] so that callers can give a precise,
    /// actionable message rather than a generic parse-error string.
    #[error("DRM-protected file: {}", path.display())]
    DrmProtected {
        /// Path of the DRM-protected HWP file.
        path: PathBuf,
    },

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

    /// A style template file could not be loaded or parsed.
    #[error("Style template error: {0}")]
    StyleLoad(String),
}
