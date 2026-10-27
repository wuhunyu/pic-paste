//! Conservative RGBA displacement detection with bounded animation tolerance.
//!
//! A small dynamic banner or sidebar must not poison the entire row hash.
//! Per-lane fingerprints nominate shifts, and an independent pixel consensus
//! accepts only a unique, high-confidence overlap. Dynamic pixels are ignored
//! for matching only; never modify or synthesize final PNG pixels.
use shot_core::{RgbaFrame, platform::ScrollAxis};
use std::collections::HashMap;

const LANES: usize = 8;
const MIN_STABLE_FRACTION: f64 = 0.75;
const MAX_STABLE_MEAN_ERROR: f64 = 8.0;
const CHANNEL_TOLERANCE_SUM: u32 = 36;
const MIN_AMBIGUITY_FRACTION: f64 = 0.08;

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

fn lane_hash(
    frame: &RgbaFrame,
    axis: ScrollAxis,
    movement: usize,
    from: usize,
    to: usize,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let stride = (to - from).div_ceil(10).max(1);
    for cross in (from..to).step_by(stride) {
        let index = pixel_index(frame, axis, movement, cross);
        for channel in 0..3 {
            hash ^= u64::from(frame.rgba[index + channel]);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

struct MatchQuality {
    stable_fraction: f64,
    stable_error: f64,
}

/// Verify every proposed shift on an independently sampled RGB grid.
/// Up to a quarter of moving pixels may differ (animations, cursor blinks,
/// inline video), but the remaining majority must match in physical pixels.
fn sampled_quality(
    previous: &RgbaFrame,
    next: &RgbaFrame,
    axis: ScrollAxis,
    shift: usize,
    motion_len: usize,
    cross_len: usize,
) -> MatchQuality {
    let overlap = motion_len - shift;
    let motion_step = overlap.div_ceil(256).max(1);
    let cross_step = cross_len.div_ceil(80).max(1);
    let mut inliers = 0u64;
    let mut observations = 0u64;
    let mut stable_error_sum = 0u64;
    for movement in (0..overlap).step_by(motion_step) {
        for cross in (0..cross_len).step_by(cross_step) {
            let a = pixel_index(previous, axis, movement + shift, cross);
            let b = pixel_index(next, axis, movement, cross);
            let mut difference = 0u32;
            for channel in 0..3 {
                difference +=
                    u32::from(previous.rgba[a + channel].abs_diff(next.rgba[b + channel]));
            }
            observations += 1;
            if difference <= CHANNEL_TOLERANCE_SUM {
                inliers += 1;
                stable_error_sum += u64::from(difference);
            }
        }
    }
    MatchQuality {
        stable_fraction: inliers as f64 / observations as f64,
        stable_error: if inliers == 0 {
            f64::INFINITY
        } else {
            stable_error_sum as f64 / (inliers * 3) as f64
        },
    }
}

struct Candidate {
    shift: usize,
    quality: MatchQuality,
    votes: usize,
}

/// Return only a uniquely supported positive displacement in physical pixels.
///
/// Eight independent cross-axis lanes nominate candidates. A dynamic portion
/// occupying at most 25% of the sampled overlap can be ignored for matching,
/// but the complete original captured pixels remain unchanged on disk.
/// Ambiguous / repetitive content and large animated areas are rejected.
pub fn match_scroll_frame(previous: &RgbaFrame, next: &RgbaFrame, axis: ScrollAxis) -> Option<u32> {
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

    let mut votes: HashMap<usize, usize> = HashMap::new();
    for lane in 0..LANES {
        let from = cross_len * lane / LANES;
        let to = cross_len * (lane + 1) / LANES;
        if from >= to {
            continue;
        }
        let mut positions: HashMap<u64, Vec<usize>> = HashMap::new();
        for movement in 0..movement_len {
            let hash = lane_hash(previous, axis, movement, from, to);
            let matching = positions.entry(hash).or_default();
            if matching.len() <= 8 {
                matching.push(movement);
            }
        }
        for next_movement in 0..movement_len {
            let hash = lane_hash(next, axis, next_movement, from, to);
            if let Some(matching) = positions.get(&hash)
                && matching.len() <= 8
            {
                for &previous_movement in matching {
                    if previous_movement > next_movement {
                        let shift = previous_movement - next_movement;
                        if (2..=maximum_shift).contains(&shift) {
                            *votes.entry(shift).or_default() += 1;
                        }
                    }
                }
            }
        }
    }

    let mut proposed: Vec<_> = votes.into_iter().filter(|(_, count)| *count >= 6).collect();
    proposed.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    proposed.truncate(24);

    let mut ranked: Vec<_> = proposed
        .into_iter()
        .filter_map(|(shift, count)| {
            let quality = sampled_quality(previous, next, axis, shift, movement_len, cross_len);
            (quality.stable_fraction >= MIN_STABLE_FRACTION
                && quality.stable_error <= MAX_STABLE_MEAN_ERROR)
                .then_some(Candidate {
                    shift,
                    quality,
                    votes: count,
                })
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.quality
            .stable_fraction
            .total_cmp(&a.quality.stable_fraction)
            .then_with(|| a.quality.stable_error.total_cmp(&b.quality.stable_error))
            .then_with(|| b.votes.cmp(&a.votes))
    });

    let best = ranked.first()?;
    if let Some(runner_up) = ranked.get(1)
        && best.quality.stable_fraction - runner_up.quality.stable_fraction
            < MIN_AMBIGUITY_FRACTION
        && runner_up.quality.stable_error - best.quality.stable_error < 2.0
    {
        return None;
    }
    u32::try_from(best.shift).ok()
}
