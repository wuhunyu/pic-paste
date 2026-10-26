#![forbid(unsafe_code)]
pub mod response;
pub use response::{read_response, OcrDocument, OcrLine, OcrResponse, OcrWord};
pub mod request;
pub use request::{MAX_RGBA_BYTES, OcrRequest, PROTOCOL_VERSION};
