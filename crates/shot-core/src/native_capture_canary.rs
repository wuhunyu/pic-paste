//! Opt-in native capture routing for the separate macOS NSPanel companion.
//! This state never grants a native reply authority over a screenshot: the
//! caller must additionally validate its request ID and capture/edit revision.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanaryPhase {
    #[default]
    Legacy,
    AwaitingNative { request_id: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanaryReply {
    Applied,
    Cancelled,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanaryTransition {
    IgnoreStale,
    ShowLegacy,
    DismissCapture,
}

impl CanaryPhase {
    /// A missing companion or an unchecked default configuration can never
    /// suppress the stable screenshot editor.
    pub fn begin(opted_in: bool, started_id: Option<u64>) -> Self {
        match (opted_in, started_id) {
            (true, Some(request_id)) if request_id != 0 => {
                Self::AwaitingNative { request_id }
            }
            _ => Self::Legacy,
        }
    }

    pub fn hides_legacy(self) -> bool {
        matches!(self, Self::AwaitingNative { .. })
    }

    /// No old process (even an explicit cancellation) may close a newer
    /// capture. A valid cancellation *only* dismisses an auto-started panel;
    /// an invalid or failed reply always exposes the stable editor.
    pub fn complete(&mut self, received_id: u64, reply: CanaryReply) -> CanaryTransition {
        let Self::AwaitingNative { request_id } = *self else {
            return CanaryTransition::IgnoreStale;
        };
        if request_id != received_id {
            return CanaryTransition::IgnoreStale;
        }
        self.reset();
        match reply {
            CanaryReply::Applied | CanaryReply::Rejected => CanaryTransition::ShowLegacy,
            CanaryReply::Cancelled => CanaryTransition::DismissCapture,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::Legacy;
    }
}
