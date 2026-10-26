use shot_core::overlay::{OverlayPhase, OverlayState};

#[test]
fn screenshot_overlay_only_enters_fullscreen_after_a_successful_capture() {
    let mut overlay = OverlayState::default();
    assert_eq!(overlay.phase(), OverlayPhase::Idle);
    assert!(!overlay.is_fullscreen());
    assert!(overlay.begin_capture());
    assert_eq!(overlay.phase(), OverlayPhase::Capturing);
    assert!(!overlay.is_fullscreen());
    assert!(!overlay.begin_capture());
    assert!(overlay.captured(true));
    assert_eq!(overlay.phase(), OverlayPhase::Selecting);
    assert!(overlay.is_fullscreen());
    assert!(!overlay.begin_capture());
}

#[test]
fn screenshot_overlay_restores_normal_window_on_cancel_and_confirm() {
    let mut overlay = OverlayState::default();
    assert!(overlay.begin_capture());
    assert!(overlay.captured(true));
    assert!(overlay.dismiss());
    assert_eq!(overlay.phase(), OverlayPhase::Idle);
    assert!(!overlay.is_fullscreen());
    assert!(!overlay.dismiss());
    assert!(overlay.begin_capture());
    assert!(overlay.captured(true));
    assert!(overlay.accept());
    assert_eq!(overlay.phase(), OverlayPhase::Idle);
}

#[test]
fn failed_capture_returns_to_idle_without_opening_a_fullscreen_overlay() {
    let mut overlay = OverlayState::default();
    assert!(overlay.begin_capture());
    assert!(!overlay.captured(false));
    assert_eq!(overlay.phase(), OverlayPhase::Idle);
    assert!(!overlay.is_fullscreen());
    assert!(!overlay.captured(true));
    assert!(overlay.begin_capture());
}
