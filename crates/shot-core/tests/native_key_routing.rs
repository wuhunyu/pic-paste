use shot_core::native_key_routing::{
    NativeKeyDecision, NativeKeyDown, NativeShortcutAction, NativeShortcutRouter, ShortcutModifiers,
};

fn command_z() -> NativeKeyDown<'static> {
    NativeKeyDown {
        key_code: 6,
        characters_ignoring_modifiers: Some("z"),
        modifiers: ShortcutModifiers {
            command: true,
            ..ShortcutModifiers::default()
        },
        repeat: false,
        text_input_active: false,
        selection_mode: true,
        panel_focused: true,
    }
}

#[test]
fn command_z_and_shift_command_z_route_once_per_press() {
    let mut router = NativeShortcutRouter::default();
    assert_eq!(
        router.key_down(command_z()),
        NativeKeyDecision::Action(NativeShortcutAction::Undo)
    );
    assert_eq!(
        router.key_down(NativeKeyDown {
            repeat: true,
            ..command_z()
        }),
        NativeKeyDecision::Consumed
    );
    assert_eq!(router.key_up(6), NativeKeyDecision::Consumed);
    assert_eq!(router.key_up(6), NativeKeyDecision::Pass);

    assert_eq!(
        router.key_down(NativeKeyDown {
            key_code: 99,
            characters_ignoring_modifiers: Some("Z"),
            modifiers: ShortcutModifiers {
                command: true,
                shift: true,
                ..ShortcutModifiers::default()
            },
            ..command_z()
        }),
        NativeKeyDecision::Action(NativeShortcutAction::Redo)
    );
    assert_eq!(router.key_up(99), NativeKeyDecision::Consumed);
}

#[test]
fn text_editor_and_composition_keep_all_command_keys_and_escape() {
    let mut router = NativeShortcutRouter::default();
    for event in [
        command_z(),
        NativeKeyDown {
            modifiers: ShortcutModifiers {
                command: true,
                shift: true,
                ..ShortcutModifiers::default()
            },
            ..command_z()
        },
        NativeKeyDown {
            key_code: 53,
            characters_ignoring_modifiers: None,
            modifiers: ShortcutModifiers::default(),
            ..command_z()
        },
        NativeKeyDown {
            key_code: 36,
            characters_ignoring_modifiers: None,
            modifiers: ShortcutModifiers::default(),
            ..command_z()
        },
    ] {
        assert_eq!(
            router.key_down(NativeKeyDown {
                text_input_active: true,
                ..event
            }),
            NativeKeyDecision::Pass
        );
        assert_eq!(router.key_up(event.key_code), NativeKeyDecision::Pass);
    }
}

#[test]
fn command_equivalents_are_character_based_and_reject_other_modifiers() {
    let mut router = NativeShortcutRouter::default();
    for key in [
        NativeKeyDown {
            characters_ignoring_modifiers: Some("ж"),
            ..command_z()
        },
        NativeKeyDown {
            characters_ignoring_modifiers: Some("z"),
            modifiers: ShortcutModifiers {
                command: true,
                option: true,
                ..ShortcutModifiers::default()
            },
            ..command_z()
        },
        NativeKeyDown {
            modifiers: ShortcutModifiers {
                command: true,
                control: true,
                ..ShortcutModifiers::default()
            },
            ..command_z()
        },
        NativeKeyDown {
            modifiers: ShortcutModifiers::default(),
            ..command_z()
        },
        NativeKeyDown {
            panel_focused: false,
            ..command_z()
        },
        NativeKeyDown {
            selection_mode: false,
            ..command_z()
        },
    ] {
        assert_eq!(router.key_down(key), NativeKeyDecision::Pass);
    }
    assert_eq!(router.key_up(6), NativeKeyDecision::Pass);
}

#[test]
fn enter_and_escape_respect_focus_text_input_and_selection_state() {
    let mut router = NativeShortcutRouter::default();
    let escape = NativeKeyDown {
        key_code: 53,
        characters_ignoring_modifiers: None,
        modifiers: ShortcutModifiers::default(),
        ..command_z()
    };
    assert_eq!(
        router.key_down(escape),
        NativeKeyDecision::Action(NativeShortcutAction::Cancel)
    );
    assert_eq!(router.key_up(53), NativeKeyDecision::Consumed);
    assert_eq!(
        router.key_down(NativeKeyDown {
            selection_mode: false,
            ..escape
        }),
        NativeKeyDecision::Action(NativeShortcutAction::Cancel)
    );
    assert_eq!(router.key_up(53), NativeKeyDecision::Consumed);

    let enter = NativeKeyDown {
        key_code: 36,
        characters_ignoring_modifiers: None,
        modifiers: ShortcutModifiers::default(),
        ..command_z()
    };
    assert_eq!(
        router.key_down(enter),
        NativeKeyDecision::Action(NativeShortcutAction::Confirm)
    );
    assert_eq!(router.key_up(36), NativeKeyDecision::Consumed);
    assert_eq!(
        router.key_down(NativeKeyDown {
            selection_mode: false,
            ..enter
        }),
        NativeKeyDecision::Pass
    );
    assert_eq!(
        router.key_down(NativeKeyDown {
            modifiers: ShortcutModifiers {
                control: true,
                ..ShortcutModifiers::default()
            },
            ..enter
        }),
        NativeKeyDecision::Pass
    );
}

#[test]
fn consumed_release_ignores_modifier_changes_and_focus_reset() {
    let mut router = NativeShortcutRouter::default();
    assert_eq!(
        router.key_down(command_z()),
        NativeKeyDecision::Action(NativeShortcutAction::Undo)
    );
    assert_eq!(router.key_up(101), NativeKeyDecision::Pass);
    assert_eq!(router.key_up(6), NativeKeyDecision::Consumed);
    assert_eq!(
        router.key_down(command_z()),
        NativeKeyDecision::Action(NativeShortcutAction::Undo)
    );
    router.reset();
    assert_eq!(router.key_up(6), NativeKeyDecision::Pass);
    assert_eq!(
        router.key_down(NativeKeyDown {
            repeat: true,
            ..command_z()
        }),
        NativeKeyDecision::Pass
    );
}
