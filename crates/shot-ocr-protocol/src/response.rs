use serde::Deserialize;
use std::io::{self, Read};

pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub confidence: f32,
    pub polygon_px: [[f32; 2]; 4],
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
    pub polygon_px: [[f32; 2]; 4],
    pub words: Vec<OcrWord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OcrDocument {
    pub image_hash: String,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub provider: String,
    pub model_revision: String,
    pub lines: Vec<OcrLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OcrResponse {
    Document(OcrDocument),
    Error(String),
}

#[derive(Deserialize)]
struct Envelope {
    protocol_version: u32,
    document: Option<OcrDocument>,
    error: Option<String>,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn valid_polygon(points: &[[f32; 2]; 4], width: u32, height: u32) -> bool {
    points.iter().all(|[x, y]| {
        x.is_finite()
            && y.is_finite()
            && (0.0..=width as f32).contains(x)
            && (0.0..=height as f32).contains(y)
    })
}

fn valid_confidence(c: f32) -> bool {
    c.is_finite() && (0.0..=1.0).contains(&c)
}

fn valid_document(doc: &OcrDocument) -> bool {
    if doc.pixel_width == 0
        || doc.pixel_height == 0
        || doc.image_hash.is_empty()
        || doc.provider.is_empty()
        || doc.model_revision.is_empty()
        || doc.lines.len() > 20_000
    {
        return false;
    }
    doc.lines.iter().all(|line| {
        !line.text.is_empty()
            && line.text.len() <= 32_768
            && valid_confidence(line.confidence)
            && valid_polygon(&line.polygon_px, doc.pixel_width, doc.pixel_height)
            && line.words.len() <= 4096
            && line.words.iter().all(|word| {
                !word.text.is_empty()
                    && valid_confidence(word.confidence)
                    && valid_polygon(&word.polygon_px, doc.pixel_width, doc.pixel_height)
            })
    })
}

/// Helper writes one u32 little-endian byte-length and UTF-8 JSON payload.
/// Always bound the size *before* allocation and refuse any malformed result.
pub fn read_response<R: Read>(mut reader: R) -> io::Result<OcrResponse> {
    let mut header = [0u8; 4];
    reader.read_exact(&mut header)?;
    let len = u32::from_le_bytes(header) as usize;
    if len == 0 || len > MAX_RESPONSE_BYTES {
        return Err(invalid("invalid OCR result size"));
    }
    let mut payload = vec![0; len];
    reader.read_exact(&mut payload)?;
    let reply: Envelope = serde_json::from_slice(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if reply.protocol_version != crate::PROTOCOL_VERSION {
        return Err(invalid("OCR protocol version mismatch"));
    }
    match (reply.document, reply.error) {
        (Some(document), None) if valid_document(&document) => Ok(OcrResponse::Document(document)),
        (None, Some(message)) if !message.is_empty() => Ok(OcrResponse::Error(message)),
        _ => Err(invalid("invalid OCR response contents")),
    }
}
