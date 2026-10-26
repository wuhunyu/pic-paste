use std::io::{self, Read, Write};

const MAGIC: &[u8; 4] = b"SHOC";
pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_RGBA_BYTES: usize = 80 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcrRequest {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn checked_size(width: u32, height: u32) -> io::Result<usize> {
    let byte_count = (width as usize).checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "dimension overflow"))?;
    if width == 0 || height == 0 || byte_count > MAX_RGBA_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid image dimensions"));
    }
    Ok(byte_count)
}
impl OcrRequest {
    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        if checked_size(self.width, self.height)? != self.rgba.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pixel count mismatch"));
        }
        writer.write_all(MAGIC)?;
        writer.write_all(&PROTOCOL_VERSION.to_le_bytes())?;
        writer.write_all(&self.width.to_le_bytes())?;
        writer.write_all(&self.height.to_le_bytes())?;
        writer.write_all(&self.rgba)?;
        writer.flush()
    }
    pub fn read_from<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut header = [0; 16];
        reader.read_exact(&mut header)?;
        if &header[..4] != MAGIC
            || u32::from_le_bytes(header[4..8].try_into().unwrap()) != PROTOCOL_VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported OCR protocol"));
        }
        let width = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let height = u32::from_le_bytes(header[12..16].try_into().unwrap());
        let mut rgba = vec![0; checked_size(width, height)?];
        reader.read_exact(&mut rgba)?;
        Ok(Self { width, height, rgba })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_rgba() {
        let original = OcrRequest { width: 2, height: 2, rgba: vec![9; 16] };
        let mut buf = vec![];
        original.write_to(&mut buf).unwrap();
        assert_eq!(OcrRequest::read_from(buf.as_slice()).unwrap(), original);
    }
    #[test]
    fn blocks_oversize_before_allocation() {
        let mut header = b"SHOC".to_vec();
        header.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(OcrRequest::read_from(header.as_slice()).is_err());
    }
    #[test]
    fn blocks_version_and_truncation() {
        assert!(OcrRequest::read_from(&b"NOPE"[..]).is_err());
        let request = OcrRequest { width: 1, height: 1, rgba: vec![0; 4] };
        let mut bytes = vec![];
        request.write_to(&mut bytes).unwrap();
        bytes[4] = 7;
        assert!(OcrRequest::read_from(bytes.as_slice()).is_err());
        bytes[4] = 1;
        bytes.pop();
        assert!(OcrRequest::read_from(bytes.as_slice()).is_err());
    }
}
