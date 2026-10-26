//! A bounded newest-frame mailbox for low-memory long-screenshot previews.
//! The worker replaces the previous thumbnail rather than building a queue.
use shot_core::RgbaFrame;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct LatestPreview {
    slot: Arc<Mutex<Option<RgbaFrame>>>,
}

impl LatestPreview {
    pub fn publish(&self, frame: RgbaFrame) {
        if let Ok(mut latest) = self.slot.lock() {
            *latest = Some(frame);
        }
    }

    pub fn take(&self) -> Option<RgbaFrame> {
        self.slot.lock().ok().and_then(|mut latest| latest.take())
    }
}
