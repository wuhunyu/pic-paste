use shot_core::native_capture_canary::{
    CanaryPhase, CanaryReply, CanaryTransition,
};

#[test]
fn stable_capture_is_default_even_when_a_companion_exists() {
    let mut phase = CanaryPhase::default();
    assert!(!phase.hides_legacy());
    assert_eq!(phase.complete(41, CanaryReply::Cancelled), CanaryTransition::IgnoreStale);
    assert_eq!(CanaryPhase::begin(false, Some(41)), CanaryPhase::default());
    assert_eq!(CanaryPhase::begin(true, None), CanaryPhase::default());
    assert_eq!(CanaryPhase::begin(true, Some(0)), CanaryPhase::default());
}

#[test]
fn successful_opt_in_launch_temporarily_hides_legacy() {
    let phase = CanaryPhase::begin(true, Some(41));
    assert!(phase.hides_legacy());
}

#[test]
fn late_result_cannot_replace_or_dismiss_a_new_capture() {
    let mut phase = CanaryPhase::begin(true, Some(42));
    assert_eq!(phase.complete(41, CanaryReply::Cancelled), CanaryTransition::IgnoreStale);
    assert!(phase.hides_legacy());
    assert_eq!(phase.complete(42, CanaryReply::Applied), CanaryTransition::ShowLegacy);
    assert!(!phase.hides_legacy());
}

#[test]
fn auto_native_cancel_dismisses_capture_instead_of_reopening_editor() {
    let mut phase = CanaryPhase::begin(true, Some(7));
    assert_eq!(phase.complete(7, CanaryReply::Cancelled), CanaryTransition::DismissCapture);
    assert!(!phase.hides_legacy());
}

#[test]
fn failed_or_invalid_native_reply_returns_to_existing_editor() {
    let mut phase = CanaryPhase::begin(true, Some(7));
    assert_eq!(phase.complete(7, CanaryReply::Rejected), CanaryTransition::ShowLegacy);
    assert!(!phase.hides_legacy());
}

#[test]
fn reset_releases_hidden_legacy_viewport_for_menu_recovery() {
    let mut phase = CanaryPhase::begin(true, Some(99));
    phase.reset();
    assert!(!phase.hides_legacy());
    assert_eq!(phase.complete(99, CanaryReply::Cancelled), CanaryTransition::IgnoreStale);
}
