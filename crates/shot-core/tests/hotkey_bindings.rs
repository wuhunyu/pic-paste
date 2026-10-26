use shot_core::shortcuts::{HotkeyBindings, ShortcutAction, ShortcutError};

#[test]
fn defaults_match_existing_global_shortcuts() {
    let b = HotkeyBindings::default();
    assert_eq!(b.digit(ShortcutAction::Capture), 1);
    assert_eq!(b.digit(ShortcutAction::PastePin), 2);
    assert_eq!(b.digit(ShortcutAction::RecoverPins), 3);
    assert_eq!(b.digit(ShortcutAction::FinishScroll), 4);
}

#[test]
fn custom_bindings_are_unique_and_reject_reserved_or_invalid_digits() {
    assert_eq!(
        HotkeyBindings::new([1, 2, 2, 4]),
        Err(ShortcutError::DuplicateDigit)
    );
    assert_eq!(
        HotkeyBindings::new([1, 2, 3, 0]),
        Err(ShortcutError::InvalidDigit)
    );
    assert_eq!(
        HotkeyBindings::new([1, 2, 3, 10]),
        Err(ShortcutError::InvalidDigit)
    );
    assert!(HotkeyBindings::new([9, 8, 7, 6]).is_ok());
}

#[test]
fn changes_are_transactional_and_preserve_original_on_error() {
    let original = HotkeyBindings::default();
    assert_eq!(
        original.with_digit(ShortcutAction::Capture, 2),
        Err(ShortcutError::DuplicateDigit)
    );
    assert_eq!(original.digit(ShortcutAction::Capture), 1);
    let changed = original.with_digit(ShortcutAction::Capture, 9).unwrap();
    assert_eq!(changed.digit(ShortcutAction::Capture), 9);
    assert_eq!(original.digit(ShortcutAction::Capture), 1);
}

#[test]
fn config_roundtrip_is_strict_and_versioned() {
    let original = HotkeyBindings::new([9, 8, 7, 6]).unwrap();
    let text = original.serialize();
    assert_eq!(HotkeyBindings::parse(&text), Ok(original));
    assert!(text.starts_with("version=1\n"));
    assert!(text.contains("capture=9\n"));
}

#[test]
fn damaged_or_duplicate_settings_are_not_accepted() {
    for invalid in [
        "",
        "version=2\ncapture=1\npaste=2\nrecover=3\nfinish=4\n",
        "version=1\ncapture=1\npaste=2\nrecover=3\n",
        "version=1\ncapture=1\npaste=1\nrecover=3\nfinish=4\n",
        "version=1\ncapture=1\npaste=2\nrecover=3\nfinish=4\nunknown=9\n",
        "version=1\ncapture=1\npaste=2\nrecover=3\nfinish=4\nfinish=5\n",
        "version=1\ncapture=01\npaste=2\nrecover=3\nfinish=4\n",
    ] {
        assert!(HotkeyBindings::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn shortcut_labels_follow_custom_digits() {
    let b = HotkeyBindings::new([9, 8, 7, 6]).unwrap();
    assert_eq!(b.label(ShortcutAction::Capture), "Ctrl+Option+9");
    assert_eq!(b.label(ShortcutAction::FinishScroll), "Ctrl+Option+6");
}
