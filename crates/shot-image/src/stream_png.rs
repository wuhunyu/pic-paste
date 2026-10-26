use std::io::{self, Write};

/// Encode an RGBA image by producing one scanline at a time.
pub fn encode_png_rows<W, F>(sink: W, width: u32, height: u32, mut fill_row: F) -> io::Result<()>
where
    W: Write,
    F: FnMut(u32, &mut [u8]) -> io::Result<()>,
{
    if width == 0 || height == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty image"));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "row overflow"))?;
    let mut encoder = png::Encoder::new(sink, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    let mut stream = writer.stream_writer().map_err(io::Error::other)?;
    let mut row = vec![0; row_bytes];
    for y in 0..height {
        fill_row(y, &mut row)?;
        stream.write_all(&row)?;
    }
    stream.finish().map_err(io::Error::other)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_png_without_full_rgba_image() {
        let mut output = Vec::new();
        encode_png_rows(&mut output, 4, 10, |y, row| {
            for pixel in row.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[y as u8, 80, 120, 255]);
            }
            Ok(())
        }).unwrap();
        assert_eq!(&output[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn cancellation_propagates() {
        let result = encode_png_rows(Vec::new(), 4, 10, |y, _| {
            if y == 3 { return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled")); }
            Ok(())
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
    }
}
