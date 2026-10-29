//! Opt-in native capture routing for the separate macOS NSPanel companion.
//! This state never grants a native reply authority over a screenshot: the
//! caller must additionally validate its request ID and capture/edit revision.

use std::time::{Duration, Instant};

/// Bound the opt-in native selector if it stops replying. This is deliberately
/// generous for manual annotations; it is NOT a deadline for normal capture.
pub const NATIVE_SELECTION_TIMEOUT: Duration = Duration::from_secs(300);

/// A deadline belongs to one immutable request, never to the last running
/// subprocess or the current overlay. Old helper results cannot time out a
/// new capture, even if the child exits after its request was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeSelectionWatchdog {
    request_id: u64,
    started_at: Instant,
    timeout: Duration,
}

impl NativeSelectionWatchdog {
    pub fn new(request_id: u64, started_at: Instant, timeout: Duration) -> Result<Self, &'static str> {
        if request_id == 0 || timeout.is_zero() {
            return Err("native selection watchdog requires a valid request and timeout");
        }
        Ok(Self {
            request_id,
            started_at,
            timeout,
        })
    }

    /// A missing session, a newer request, or an earlier clock reading cannot
    /// authorize the watchdog to dismiss the trusted screenshot editor.
    pub fn expired_for(self, active_id: Option<u64>, now: Instant) -> bool {
        active_id == Some(self.request_id)
            && now
                .checked_duration_since(self.started_at)
                .is_some_and(|elapsed| elapsed >= self.timeout)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanaryPhase {
    #[default]
    Legacy,
    AwaitingNative {
        request_id: u64,
    },
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
            (true, Some(request_id)) if request_id != 0 => Self::AwaitingNative { request_id },
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

#[cfg(test)]
mod selection_watchdog_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn timeout_only_applies_to_the_current_request_at_its_deadline() {
        let start = Instant::now();
        let watchdog = NativeSelectionWatchdog::new(7, start, Duration::from_millis(50)).unwrap();
        assert!(!watchdog.expired_for(Some(7), start));
        assert!(!watchdog.expired_for(Some(7), start + Duration::from_millis(49)));
        assert!(watchdog.expired_for(Some(7), start + Duration::from_millis(50)));
        assert!(!watchdog.expired_for(Some(8), start + Duration::from_secs(1)));
        assert!(!watchdog.expired_for(None, start + Duration::from_secs(1)));
    }

    #[test]
    fn a_new_session_must_not_inherit_the_previous_sessions_deadline() {
        let start = Instant::now();
        let old = NativeSelectionWatchdog::new(11, start, Duration::from_millis(100)).unwrap();
        let newer = NativeSelectionWatchdog::new(
            12,
            start + Duration::from_millis(90),
            Duration::from_millis(100),
        )
        .unwrap();
        assert!(old.expired_for(Some(11), start + Duration::from_millis(100)));
        assert!(!newer.expired_for(Some(12), start + Duration::from_millis(100)));
        assert!(!old.expired_for(Some(12), start + Duration::from_secs(2)));
        assert!(newer.expired_for(Some(12), start + Duration::from_millis(190)));
    }

    #[test]
    fn invalid_watchdogs_cannot_block_the_stable_editor() {
        let start = Instant::now();
        assert!(NativeSelectionWatchdog::new(0, start, Duration::from_secs(1)).is_err());
        assert!(NativeSelectionWatchdog::new(1, start, Duration::ZERO).is_err());
        assert!(NativeSelectionWatchdog::new(1, start, Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn clock_rollback_does_not_falsely_expire_the_watchdog() {
        let start = Instant::now();
        let watchdog = NativeSelectionWatchdog::new(
            9,
            start + Duration::from_millis(100),
            Duration::from_millis(20),
        )
        .unwrap();
        assert!(!watchdog.expired_for(Some(9), start));
    }
}
