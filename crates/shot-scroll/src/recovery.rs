//! Pure recovery policy for manual long screenshots.
//! Untrusted frames are never approved for insertion into the TileStore.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameObservation {
    Matched,
    Unchanged,
    Untrusted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDecision {
    Continue,
    Pause,
    IgnoreWhilePaused,
}

#[derive(Default)]
pub struct RecoveryTracker {
    failures: u8,
    paused: bool,
    awaiting_verified_overlap: bool,
}

impl RecoveryTracker {
    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn consecutive_failures(&self) -> u8 {
        self.failures
    }

    pub fn needs_partial_review(&self) -> bool {
        self.paused || self.awaiting_verified_overlap
    }

    /// A manual retry must start from the old trusted anchor. Only a future
    /// verified match clears the still-incomplete capture warning.
    pub fn resume(&mut self) -> bool {
        if !self.paused {
            return false;
        }
        self.paused = false;
        self.failures = 0;
        self.awaiting_verified_overlap = true;
        true
    }

    pub fn observe(&mut self, observation: FrameObservation) -> RecoveryDecision {
        if self.paused {
            return RecoveryDecision::IgnoreWhilePaused;
        }
        match observation {
            FrameObservation::Matched => {
                self.failures = 0;
                self.awaiting_verified_overlap = false;
                RecoveryDecision::Continue
            }
            FrameObservation::Unchanged => RecoveryDecision::Continue,
            FrameObservation::Untrusted => {
                self.failures = self.failures.saturating_add(1);
                if self.failures >= 3 {
                    self.paused = true;
                    RecoveryDecision::Pause
                } else {
                    RecoveryDecision::Continue
                }
            }
        }
    }
}
