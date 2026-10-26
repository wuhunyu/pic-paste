//! Fast, conservative RGBA scroll displacement detection.
//!
//! Row/column fingerprints propose only physically plausible offsets; sparse
//! RGB pixel comparisons then reject animated, repeated or ambiguous scenes.
//! There is no speculative append when an offset is uncertain.

use shot_core::{RgbaFrame, platform::ScrollAxis};
use std::collections::HashMap;

fn valid(frame: &RgbaFrame) -> bool {
    let expected = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .and_then(|value| value.checked_mul(4));
    frame.width > 0
        && frame.height > 0
        && expected.and_then(|n| usize::try_from(n).ok()) == Some(frame.rgba.len())
}

fn dimensions(frame: &RgbaFrame, axis: ScrollAxis) -> (usize, usize) {
    match axis {
        ScrollAxis::Vertical => (frame.height as usize, frame.width as usize),
        ScrollAxis::Horizontal => (frame.width as usize, frame.height as usize),
    }
}

fn pixel_index(frame: &RgbaFrame, axis: ScrollAxis, movement: usize, cross: usize) -> usize {
    let (x, y) = match axis {
        ScrollAxis::Vertical => (cross, movement),
        ScrollAxis::Horizontal => (movement, cross),
    };
    (y * frame.width as usize + x) * 4
}

fn line_hash(frame: &RgbaFrame, axis: ScrollAxis, movement: usize, cross_len: usize) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let stride = cross_len.div_ceil(80).max(1);
    for cross in (0..cross_len).step_by(stride) {
        let index = pixel_index(frame, axis, movement, cross);
        for channel in 0..3 {
            hash ^= u64::from(frame.rgba[index + channel]);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn sampled_error(
    previous: &RgbaFrame,
    next: &RgbaFrame,
    axis: ScrollAxis,
    shift: usize,
    motion_len: usize,
    cross_len: usize,
) -> f64 {
    let overlap = motion_len - shift;
    let motion_step = overlap.div_ceil(256).max(1);
    let cross_step = cross_len.div_ceil(80).max(1);
    let mut total = 0u64;
    let mut count = 0u64;
    for movement in (0..overlap).step_by(motion_step) {
        for cross in (0..cross_len).step_by(cross_step) {
            let a = pixel_index(previous, axis, movement + shift, cross);
            let b = pixel_index(next, axis, movement, cross);
            for channel in 0..3 {
                total += u64::from(previous.rgba[a + channel].abs_diff(next.rgba[b + channel]));
                count += 1;
            }
        }
    }
    total as f64 / count as f64
}

/// Return a trustworthy *positive* displacement in native physical pixels.
///
/// Reverse scrolling and uncertain scenes intentionally return None. The caller
/// must pause or retry rather than silently append broken image segments.
pub fn match_scroll_frame(
    previous: &RgbaFrame,
    next: &RgbaFrame,
    axis: ScrollAxis,
) -> Option<u32> {
    if !valid(previous)
        || !valid(next)
        || previous.width != next.width
        || previous.height != next.height
        || previous.rgba == next.rgba
    {
        return None;
    }
    let (movement_len, cross_len) = dimensions(previous, axis);
    if movement_len < 16 || cross_len < 8 {
        return None;
    }
    let minimum_overlap = movement_len.saturating_mul(35).div_ceil(100);
    let maximum_shift = movement_len.saturating_sub(minimum_overlap.max(1));
    if maximum_shift < 2 {
        return None;
    }

    let mut line_positions: HashMap<u64, Vec<usize>> = HashMap::new();
    for i in 0..movement_len {
        let hash = line_hash(previous, axis, i, cross_len);
        let positions = line_positions.entry(hash).or_default();
        if positions.len() <= 8 {
            positions.push(i);
        }
    }
    let mut votes: HashMap<usize, usize> = HashMap::new();
    for j in 0..movement_len {
        let hash = line_hash(next, axis, j, cross_len);
        if let Some(positions) = line_positions.get(&hash)
            && positions.len() <= 8
        {
            for &i in positions {
                if i > j {
                    let shift = i - j;
                    if (2..=maximum_shift).contains(&shift) {
                        *votes.entry(shift).or_default() += 1;
                    }
                }
            }
        }
    }
    let mut candidates: Vec<_> = votes
        .into_iter()
        .filter(|(_, votes)| *votes >= 3)
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    let mut ranked: Vec<_> = candidates
        .into_iter()
        .take(12)
        .map(|(shift, count)| {
            (shift, sampled_error(previous, next, axis, shift, movement_len, cross_len), count)
        })
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.cmp(&a.2)));
    let &(best_shift, best_error, _) = ranked.first()?;
    let next_error = ranked.get(1).map_or(f64::INFINITY, |v| v.1);
    if best_error > 8.0 || next_error - best_error < 2.0 {
        return None;
    }
    u32::try_from(best_shift).ok()
}
