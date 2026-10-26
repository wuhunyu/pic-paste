pub const MAX_LONG_EDGE_PX: u64 = 60_000;
pub const MAX_TOTAL_PIXELS: u64 = 200_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeError { Empty, TooLong, TooManyPixels, Overflow }

pub fn validate_canvas(width: u64, height: u64) -> Result<(), SizeError> {
    if width == 0 || height == 0 { return Err(SizeError::Empty); }
    if width.max(height) > MAX_LONG_EDGE_PX { return Err(SizeError::TooLong); }
    let pixels = width.checked_mul(height).ok_or(SizeError::Overflow)?;
    if pixels > MAX_TOTAL_PIXELS { return Err(SizeError::TooManyPixels); }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundaries_are_inclusive() {
        assert_eq!(validate_canvas(3_000, 60_000), Ok(()));
        assert_eq!(validate_canvas(60_001, 1), Err(SizeError::TooLong));
        assert_eq!(validate_canvas(4_000, 60_000), Err(SizeError::TooManyPixels));
        assert_eq!(validate_canvas(0, 40), Err(SizeError::Empty));
    }
}
