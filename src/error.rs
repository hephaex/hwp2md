use thiserror::Error;

#[derive(Error, Debug)]
pub enum Hwp2MdError {
    #[error("unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("HWP parse error: {0}")]
    HwpParse(String),

    #[error("HWPX parse error: {0}")]
    HwpxParse(String),

    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("HWPX write error: {0}")]
    HwpxWrite(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
