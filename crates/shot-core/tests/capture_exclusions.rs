use shot_core::window_candidates::is_self_capture_window;

#[test]
fn only_our_own_process_windows_are_excluded() {
    assert!(is_self_capture_window(Some(4287), 4287));
    assert!(!is_self_capture_window(Some(4288), 4287));
    assert!(!is_self_capture_window(None, 4287));
}

#[test]
fn absent_or_invalid_process_ids_cannot_hide_unrelated_windows() {
    assert!(!is_self_capture_window(None, -1));
    assert!(!is_self_capture_window(Some(0), 0));
    assert!(!is_self_capture_window(Some(-1), -1));
    assert!(!is_self_capture_window(Some(1), 0));
}

#[test]
fn separate_pins_and_preview_windows_with_same_pid_are_both_excluded() {
    let ids = [
        (10_u64, Some(4287)),
        (11, Some(4287)),
        (12, Some(7777)),
        (13, None),
    ];
    let excluded: Vec<u64> = ids
        .into_iter()
        .filter(|(_, pid)| is_self_capture_window(*pid, 4287))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(excluded, vec![10, 11]);
}
