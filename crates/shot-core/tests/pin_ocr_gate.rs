use shot_core::pins::{PinMode, PinOcrGate};

#[test]
fn an_ocr_job_cannot_be_started_twice_on_the_same_pin() {
    let mut gate = PinOcrGate::default();
    assert!(gate.begin());
    assert!(!gate.begin());
    assert!(gate.pending());
    assert!(gate.finish(PinMode::TextSelect));
    assert!(!gate.pending());
    assert!(!gate.finish(PinMode::TextSelect));
}

#[test]
fn leaving_and_reentering_text_mode_does_not_resurrect_old_ocr_result() {
    let mut gate = PinOcrGate::default();
    assert!(gate.begin());
    gate.cancel_selection();
    assert!(!gate.finish(PinMode::TextSelect));
    assert!(!gate.pending());
    assert!(gate.begin());
    assert!(gate.finish(PinMode::TextSelect));
}

#[test]
fn passthrough_and_move_never_accept_a_pending_selection_reply() {
    for mode in [PinMode::Move, PinMode::Passthrough] {
        let mut gate = PinOcrGate::default();
        assert!(gate.begin());
        assert!(!gate.finish(mode));
        assert!(!gate.pending());
    }
}
