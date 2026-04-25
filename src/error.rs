use thiserror::Error;

#[derive(Error, Debug)]
pub enum Hwp2MdError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("HWP parse error: {0}")]
    HwpParse(String),

    #[error("HWPX parse error: {0}")]
    HwpxParse(String),

    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("HWPX write error: {0}")]
    HwpxWrite(String),

    #[error("decompression error: {0}")]
    Decompress(String),

    #[error("decompression bomb: output exceeded {0} bytes")]
    DecompressionBomb(u64),

    #[error("invalid record: {0}")]
    InvalidRecord(String),

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML write error: {0}")]
    XmlWrite(#[from] quick_xml::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
}
