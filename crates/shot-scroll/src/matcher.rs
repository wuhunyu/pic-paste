#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrayFrame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl GrayFrame {
    pub fn new(width: usize, height: usize, data: Vec<u8>) -> Option<Self> {
        if width == 0 || height == 0 || width.checked_mul(height)? != data.len() {
            return None;
        }
        Some(Self { width, height, data })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMatch {
    pub displacement_px: usize,
    pub mean_absolute_error: f64,
    pub ambiguity_gap: f64,
}

/// A correctness-oriented matcher for vertically scrolling image regions.
///
/// Deliberately reject ambiguous patterns rather than producing corrupted long screenshots.
/// Real-time large-frame matching should use a downsampled coarse stage.
pub fn match_vertical(
    previous: &GrayFrame,
    next: &GrayFrame,
    min_overlap_fraction: f64,
    maximum_error: f64,
    minimum_ambiguity_gap: f64,
) -> Option<ScrollMatch> {
    if previous.width != next.width || previous.height != next.height
        || !(0.0..1.0).contains(&min_overlap_fraction)
        || !maximum_error.is_finite() || maximum_error < 0.0
        || !minimum_ambiguity_gap.is_finite() || minimum_ambiguity_gap < 0.0
    {
        return None;
    }
    let w = previous.width;
    let h = previous.height;
    let min_overlap = ((h as f64 * min_overlap_fraction).ceil() as usize).max(1);
    let limit = h.checked_sub(min_overlap)?;
    if limit < 2 { return None; }
    let mut candidates = Vec::new();
    for shift in 2..=limit {
        let overlap = h - shift;
        let mut sum: u64 = 0;
        for y in 0..overlap {
            let start_previous = (y + shift) * w;
            let start_next = y * w;
            for x in 0..w {
                sum += u64::from(previous.data[start_previous + x]
                    .abs_diff(next.data[start_next + x]));
            }
        }
        candidates.push((shift, sum as f64 / (overlap * w) as f64));
    }
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let &(displacement_px, best_error) = candidates.first()?;
    let ambiguity_gap = candidates.get(1).map_or(f64::INFINITY, |x| x.1 - best_error);
    if best_error > maximum_error || ambiguity_gap < minimum_ambiguity_gap { return None; }
    Some(ScrollMatch {
        displacement_px, mean_absolute_error: best_error, ambiguity_gap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(offset: usize) -> GrayFrame {
        let width = 13;
        let height = 32;
        let data = (0..height).flat_map(|y| {
            (0..width).map(move |x| {
                let v = y + offset;
                ((v * v * 19 + v * x * 7 + x * 37) % 251) as u8
            })
        }).collect();
        GrayFrame::new(width, height, data).unwrap()
    }
    #[test]
    fn finds_known_displacement() {
        assert_eq!(match_vertical(&sample(0), &sample(5), 0.4, 1.0, 1.0)
            .unwrap().displacement_px, 5);
    }
    #[test]
    fn rejects_uniform_and_misaligned_frames() {
        let flat = GrayFrame::new(8, 32, vec![128; 256]).unwrap();
        assert!(match_vertical(&flat, &flat, 0.4, 1.0, 1.0).is_none());
        assert!(match_vertical(&sample(0), &flat, 0.4, 1.0, 1.0).is_none());
    }
    #[test]
    fn rejects_invalid_inputs() {
        assert!(GrayFrame::new(2, 2, vec![0; 3]).is_none());
        assert!(match_vertical(&sample(0), &sample(1), 1.0, 1.0, 1.0).is_none());
        assert!(match_vertical(&sample(0), &sample(1), 0.4, f64::NAN, 1.0).is_none());
    }
}
