//! Versioned, bounded, stdin-only transport of an immutable screenshot to
//! the opt-in native NSPanel proof. No temporary files or PNG conversion.
//!
//! Logical desktop placement and display-local physical RGBA pixels must
//! never be conflated. The frozen image is read-only in the companion.
use std::io::{Read, Write};

use crate::RgbaFrame;
use crate::display::OverlayPlacement;
use crate::platform::DisplayId;

pub const MAX_SNAPSHOT_BYTES: usize = 128 * 1024 * 1024;
const MAX_IMAGE_EDGE: u32 = 8192;
const MAGIC: &[u8; 8] = b"PPSNAP01";
const HEADER_SIZE: usize = 44;

/// A single full-display captured frame and the corresponding *logical-point*
/// positioning contract. This object cannot be constructed with invalid bytes.
#[derive(Clone, Debug)]
pub struct NativeSnapshot {
    placement: OverlayPlacement,
    image: RgbaFrame,
}

impl NativeSnapshot {
    pub fn validate_dimensions(width: u32, height: u32) -> Result<usize, String> {
        if width == 0 || height == 0 || width > MAX_IMAGE_EDGE || height > MAX_IMAGE_EDGE {
            return Err(String::from("native snapshot dimensions exceed bounds"));
        }
        let length = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or("native snapshot pixel length overflow")?;
        let length = usize::try_from(length).map_err(|error| error.to_string())?;
        if length > MAX_SNAPSHOT_BYTES {
            return Err(String::from(
                "native snapshot exceeds the 128 MiB RGBA limit",
            ));
        }
        Ok(length)
    }

    pub fn new(placement: OverlayPlacement, image: RgbaFrame) -> Result<Self, String> {
        let length = Self::validate_dimensions(image.width, image.height)?;
        if image.rgba.len() != length {
            return Err(String::from("native snapshot RGBA byte count mismatch"));
        }
        Ok(Self { placement, image })
    }

    pub fn placement(&self) -> OverlayPlacement {
        self.placement
    }

    pub fn image(&self) -> &RgbaFrame {
        &self.image
    }

    pub fn into_image(self) -> RgbaFrame {
        self.image
    }

    pub fn write_to<W: Write>(&self, sink: &mut W) -> Result<(), String> {
        let length = Self::validate_dimensions(self.image.width, self.image.height)?;
        if self.image.rgba.len() != length {
            return Err(String::from("native snapshot mutated RGBA byte count"));
        }
        let (x, y) = self.placement.origin_pt();
        let (width_pt, height_pt) = self.placement.size_pt();
        let mut header = [0u8; HEADER_SIZE];
        header[..8].copy_from_slice(MAGIC);
        header[8..16].copy_from_slice(&self.placement.display_id().0.to_le_bytes());
        header[16..20].copy_from_slice(&self.image.width.to_le_bytes());
        header[20..24].copy_from_slice(&self.image.height.to_le_bytes());
        header[24..28].copy_from_slice(&x.to_le_bytes());
        header[28..32].copy_from_slice(&y.to_le_bytes());
        header[32..36].copy_from_slice(&width_pt.to_le_bytes());
        header[36..40].copy_from_slice(&height_pt.to_le_bytes());
        header[40..44].copy_from_slice(
            &u32::try_from(length)
                .map_err(|error| error.to_string())?
                .to_le_bytes(),
        );
        sink.write_all(&header).map_err(|error| error.to_string())?;
        sink.write_all(&self.image.rgba)
            .map_err(|error| error.to_string())
    }

    pub fn read_from<R: Read>(source: &mut R) -> Result<Self, String> {
        let mut header = [0u8; HEADER_SIZE];
        source
            .read_exact(&mut header)
            .map_err(|error| error.to_string())?;
        if &header[..8] != MAGIC {
            return Err(String::from("invalid native snapshot magic/version"));
        }
        // The header is a fixed-size, validated array. These offsets cannot
        // be influenced by an untrusted length or external filesystem path.
        let u32_at = |start: usize| {
            u32::from_le_bytes(
                header[start..start + 4]
                    .try_into()
                    .expect("constant-width header field"),
            )
        };
        let f32_at = |start: usize| f32::from_bits(u32_at(start));
        let display_id = DisplayId(u64::from_le_bytes(
            header[8..16].try_into().expect("constant-width display id"),
        ));
        let width = u32_at(16);
        let height = u32_at(20);
        let expected = Self::validate_dimensions(width, height)?;
        if u32_at(40) as usize != expected {
            return Err(String::from("native snapshot declared byte count mismatch"));
        }
        let placement = OverlayPlacement::new(
            display_id,
            (f64::from(f32_at(24)), f64::from(f32_at(28))),
            (f64::from(f32_at(32)), f64::from(f32_at(36))),
        )
        .map_err(|error| format!("invalid native snapshot placement: {error:?}"))?;
        let mut rgba = vec![0u8; expected];
        source
            .read_exact(&mut rgba)
            .map_err(|error| error.to_string())?;
        // The companion receives one frame only. After the sender closes its
        // pipe, extra bytes are an error, not a second untrusted frame.
        let mut extra = [0u8; 1];
        if source.read(&mut extra).map_err(|error| error.to_string())? != 0 {
            return Err(String::from("trailing data after native snapshot frame"));
        }
        let image = RgbaFrame::new(width, height, rgba)
            .map_err(|error| format!("invalid native snapshot RGBA: {error:?}"))?;
        Self::new(placement, image)
    }
}
