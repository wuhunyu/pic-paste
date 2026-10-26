use crate::RectPx;

/// Identifies the exact screenshot revision and selection that an asynchronous
/// OCR helper was asked to recognize. Late helper replies must not be shown on
/// an image or selection that has since changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OcrTicket {
    capture_revision: u64,
    edit_revision: u64,
    region: RectPx,
}

impl OcrTicket {
    pub fn new(capture_revision: u64, edit_revision: u64, region: RectPx) -> Self {
        Self {
            capture_revision,
            edit_revision,
            region,
        }
    }

    pub fn matches(
        self,
        capture_revision: u64,
        edit_revision: u64,
        region: RectPx,
    ) -> bool {
        self.capture_revision == capture_revision
            && self.edit_revision == edit_revision
            && self.region == region
    }
}
