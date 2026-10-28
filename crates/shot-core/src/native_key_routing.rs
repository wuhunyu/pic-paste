//! Native NSPanel shortcut routing independent of AppKit, Metal, and eframe.
//! Press and release are consumed together so egui, IME, and Cocoa do not
//! receive a stray key-up event after the panel handles a shortcut.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutModifiers {
    pub command: bool,
    pub shift: bool,
    pub control: bool,
    pub option: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeKeyDown<'a> {
    pub key_code: u16,
    /// The layout-dependent character with Command/Option removed by AppKit.
    pub characters_ignoring_modifiers: Option<&'a str>,
    pub modifiers: ShortcutModifiers,
    pub repeat: bool,
    pub text_input_active: bool,
    pub selection_mode: bool,
    pub panel_focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeShortcutAction {
    Undo,
    Redo,
    Confirm,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeKeyDecision {
    Pass,
    Consumed,
    Action(NativeShortcutAction),
}

#[derive(Debug, Default)]
pub struct NativeShortcutRouter {
    consumed_keycode: Option<u16>,
}

impl NativeShortcutRouter {
    /// Never steal editing or preedit keys from an active NSTextInputClient.
    /// Command+Z follows the active keyboard layout rather than keycode 6.
    pub fn key_down(&mut self, event: NativeKeyDown<'_>) -> NativeKeyDecision {
        if let Some(held) = self.consumed_keycode {
            return if held == event.key_code {
                NativeKeyDecision::Consumed
            } else {
                NativeKeyDecision::Pass
            };
        }
        if event.repeat || !event.panel_focused || event.text_input_active {
            return NativeKeyDecision::Pass;
        }

        let modifiers = event.modifiers;
        let plain = !modifiers.command && !modifiers.control && !modifiers.option;
        let action = if plain && event.key_code == 53 {
            Some(NativeShortcutAction::Cancel)
        } else if plain && matches!(event.key_code, 36 | 76) && event.selection_mode {
            Some(NativeShortcutAction::Confirm)
        } else if event.selection_mode
            && modifiers.command
            && !modifiers.control
            && !modifiers.option
            && event
                .characters_ignoring_modifiers
                .is_some_and(|key| key.eq_ignore_ascii_case("z"))
        {
            Some(if modifiers.shift {
                NativeShortcutAction::Redo
            } else {
                NativeShortcutAction::Undo
            })
        } else {
            None
        };

        if let Some(action) = action {
            self.consumed_keycode = Some(event.key_code);
            NativeKeyDecision::Action(action)
        } else {
            NativeKeyDecision::Pass
        }
    }

    /// Consume only the release of a key-down we already handled.
    pub fn key_up(&mut self, key_code: u16) -> NativeKeyDecision {
        if self.consumed_keycode == Some(key_code) {
            self.consumed_keycode = None;
            NativeKeyDecision::Consumed
        } else {
            NativeKeyDecision::Pass
        }
    }

    /// Focus loss may drop a physical key-up event; do not retain stale state.
    pub fn reset(&mut self) {
        self.consumed_keycode = None;
    }
}
