//! Select only the moving content of a manually scrolled capture region.
//! Both the matcher and the on-disk tile store receive the same physical ROI;
//! fixed page chrome is never copied into the long PNG.

use shot_core::{RectPx, platform::ScrollAxis};

/// User-supplied fixed band sizes in physical pixels, measured inward from
/// the start/end of the scrolling direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedBands {
    pub leading: u32,
    pub trailing: u32,
}

impl FixedBands {
    /// For vertical scroll: exclude top/bottom. For horizontal: exclude
    /// left/right. Refuse invalid geometry before launching any capture job.
    pub fn content_roi(self, original: RectPx, axis: ScrollAxis) -> Result<RectPx, &'static str> {
        if original.width < 24 || original.height < 24 {
            return Err("scroll ROI must be at least 24x24 pixels");
        }
        let extent = match axis {
            ScrollAxis::Vertical => original.height,
            ScrollAxis::Horizontal => original.width,
        };
        let remaining = extent
            .checked_sub(self.leading)
            .and_then(|n| n.checked_sub(self.trailing))
            .filter(|n| *n >= 24)
            .ok_or("fixed bands leave less than 24 pixels of scroll content")?;
        let offset = i32::try_from(self.leading).map_err(|_| "fixed band offset overflow")?;
        let mut cropped = original;
        match axis {
            ScrollAxis::Vertical => {
                cropped.y = cropped
                    .y
                    .checked_add(offset)
                    .ok_or("scroll Y origin overflow")?;
                cropped.height = remaining;
            }
            ScrollAxis::Horizontal => {
                cropped.x = cropped
                    .x
                    .checked_add(offset)
                    .ok_or("scroll X origin overflow")?;
                cropped.width = remaining;
            }
        }
        Ok(cropped)
    }
}
