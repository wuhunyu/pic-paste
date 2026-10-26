use shot_scroll::recovery::{FrameObservation, RecoveryDecision, RecoveryTracker};

#[test]
fn fast_scroll_pauses_after_three_untrusted_frames_without_inventing_progress() {
    let mut tracker = RecoveryTracker::default();
    assert_eq!(
        tracker.observe(FrameObservation::Untrusted),
        RecoveryDecision::Continue
    );
    assert_eq!(
        tracker.observe(FrameObservation::Untrusted),
        RecoveryDecision::Continue
    );
    assert_eq!(tracker.consecutive_failures(), 2);
    assert_eq!(
        tracker.observe(FrameObservation::Untrusted),
        RecoveryDecision::Pause
    );
    assert!(tracker.paused());
    assert!(tracker.needs_partial_review());
    assert_eq!(
        tracker.observe(FrameObservation::Matched),
        RecoveryDecision::IgnoreWhilePaused
    );
    assert!(tracker.paused());
}

#[test]
fn manual_backtrack_and_slow_retry_require_verified_overlap_before_clearing_warning() {
    let mut tracker = RecoveryTracker::default();
    for _ in 0..3 {
        tracker.observe(FrameObservation::Untrusted);
    }
    assert!(tracker.resume());
    assert!(!tracker.paused());
    assert!(!tracker.resume());
    assert!(tracker.needs_partial_review());
    assert_eq!(
        tracker.observe(FrameObservation::Unchanged),
        RecoveryDecision::Continue
    );
    assert!(tracker.needs_partial_review());
    assert_eq!(
        tracker.observe(FrameObservation::Matched),
        RecoveryDecision::Continue
    );
    assert!(!tracker.needs_partial_review());
    assert_eq!(tracker.consecutive_failures(), 0);
}

#[test]
fn repeated_fast_scroll_after_retry_pauses_again_and_does_not_lose_recovery_state() {
    let mut tracker = RecoveryTracker::default();
    for _ in 0..3 {
        tracker.observe(FrameObservation::Untrusted);
    }
    assert!(tracker.resume());
    for i in 0..3 {
        let decision = tracker.observe(FrameObservation::Untrusted);
        assert_eq!(
            decision,
            if i == 2 {
                RecoveryDecision::Pause
            } else {
                RecoveryDecision::Continue
            }
        );
    }
    assert!(tracker.paused());
    assert!(tracker.needs_partial_review());
}

#[test]
fn unchanged_frame_does_not_count_as_bad_or_reset_existing_failures() {
    let mut tracker = RecoveryTracker::default();
    tracker.observe(FrameObservation::Untrusted);
    tracker.observe(FrameObservation::Untrusted);
    assert_eq!(
        tracker.observe(FrameObservation::Unchanged),
        RecoveryDecision::Continue
    );
    assert_eq!(tracker.consecutive_failures(), 2);
    assert_eq!(
        tracker.observe(FrameObservation::Untrusted),
        RecoveryDecision::Pause
    );
}

#[test]
fn verified_good_scroll_resets_stale_mismatch_counter() {
    let mut tracker = RecoveryTracker::default();
    tracker.observe(FrameObservation::Untrusted);
    tracker.observe(FrameObservation::Untrusted);
    assert_eq!(
        tracker.observe(FrameObservation::Matched),
        RecoveryDecision::Continue
    );
    assert!(!tracker.paused());
    assert!(!tracker.needs_partial_review());
    assert_eq!(tracker.consecutive_failures(), 0);
    assert_eq!(
        tracker.observe(FrameObservation::Untrusted),
        RecoveryDecision::Continue
    );
}
