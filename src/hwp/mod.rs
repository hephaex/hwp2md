//! HWP 5.0 reader (CFB/OLE2 binary format).
#![allow(missing_docs)]

mod control;
mod convert;
pub(crate) mod crypto;
pub(crate) mod eqedit;
mod lenient;
mod model;
mod reader;
mod record;
mod summary;

pub use model::*;
pub use reader::read_hwp;
