#![forbid(unsafe_code)]
pub mod hit_test;
pub mod response;
pub use response::{OcrDocument, OcrLine, OcrResponse, OcrWord, read_response};
pub mod request;
pub use request::{MAX_RGBA_BYTES, OcrRequest, PROTOCOL_VERSION};
