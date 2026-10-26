use crate::pins::PinMode;
/// A drag latches the action on pointer-down, avoiding mid-drag OCR/Move switching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragAction {
    MovePin,
    SelectText,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DragLatch {
    action: DragAction,
    pressed: bool,
}
impl Default for DragLatch {
    fn default() -> Self {
        Self {
            action: DragAction::None,
            pressed: false,
        }
    }
}
impl DragLatch {
    pub fn down(&mut self, mode: PinMode, shift: bool) -> DragAction {
        self.action = if shift || mode == PinMode::Move {
            DragAction::MovePin
        } else if mode == PinMode::TextSelect {
            DragAction::SelectText
        } else {
            DragAction::None
        };
        self.pressed = true;
        self.action
    }
    pub fn current(&self) -> DragAction {
        if self.pressed {
            self.action
        } else {
            DragAction::None
        }
    }
    pub fn up(&mut self) {
        self.pressed = false;
        self.action = DragAction::None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_mode_is_stable_until_pointer_up() {
        let mut latch = DragLatch::default();
        assert_eq!(
            latch.down(PinMode::TextSelect, false),
            DragAction::SelectText
        );
        assert_eq!(latch.current(), DragAction::SelectText);
        latch.up();
        assert_eq!(latch.down(PinMode::TextSelect, true), DragAction::MovePin);
        latch.up();
        assert_eq!(latch.down(PinMode::Passthrough, false), DragAction::None);
    }
}
