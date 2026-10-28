#![forbid(unsafe_code)]
pub mod async_ticket;
pub mod display;
pub mod glass_toolbar;
pub mod ime;
pub mod input;
pub mod native_panel;
pub mod native_selection;
pub mod native_selection_edit;
pub mod native_snapshot;
pub mod overlay;
pub mod pin_texture_budget;
pub mod pin_view;
pub mod pins;
pub mod platform;
pub mod selection;
pub mod shortcuts;
pub mod window_candidates;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl RectPx {
    pub fn contains(self, x: i32, y: i32) -> bool {
        let dx = i64::from(x) - i64::from(self.x);
        let dy = i64::from(y) - i64::from(self.y);
        dx >= 0 && dy >= 0 && dx < i64::from(self.width) && dy < i64::from(self.height)
    }
    pub fn validated(self) -> Result<Self, CoreError> {
        if self.width == 0 || self.height == 0 {
            Err(CoreError::EmptyRegion)
        } else {
            Ok(self)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    EmptyRegion,
    PixelSizeMismatch,
    Overflow,
    OutOfBounds,
    InvalidTransition,
}
#[derive(Clone, Debug)]
pub struct RgbaFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl RgbaFrame {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, CoreError> {
        if width == 0 || height == 0 {
            return Err(CoreError::EmptyRegion);
        }
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4))
            .ok_or(CoreError::Overflow)?;
        if usize::try_from(expected).map_err(|_| CoreError::Overflow)? != rgba.len() {
            return Err(CoreError::PixelSizeMismatch);
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollPhase {
    Configuring,
    Manual,
    Auto,
    Paused,
    Mismatch,
    Review,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Selecting,
    Annotating,
    PinMoving,
    PinTextSelecting,
    PinPassthrough,
    Scrolling(ScrollPhase),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    BeginCapture,
    SelectionReady,
    EnterScroll,
    StartManual,
    StartAuto,
    Pause,
    Mismatch,
    Resume,
    FinishScroll,
    Cancel,
    EnterTextMode,
    LeaveTextMode,
    EnablePassthrough,
    DisablePassthrough,
}
pub struct Session {
    pub mode: Mode,
}
impl Default for Session {
    fn default() -> Self {
        Self { mode: Mode::Idle }
    }
}
impl Session {
    pub fn apply(&mut self, command: Command) -> Result<Mode, CoreError> {
        use Command::*;
        use Mode::*;
        let next = match (self.mode, command) {
            (_, Cancel) => Idle,
            (Idle, BeginCapture) => Selecting,
            (Selecting, SelectionReady) => Annotating,
            (Annotating, EnterScroll) => Scrolling(ScrollPhase::Configuring),
            (Scrolling(ScrollPhase::Configuring), StartManual) => Scrolling(ScrollPhase::Manual),
            (Scrolling(ScrollPhase::Configuring), StartAuto) => Scrolling(ScrollPhase::Auto),
            (Scrolling(ScrollPhase::Manual | ScrollPhase::Auto), Pause) => {
                Scrolling(ScrollPhase::Paused)
            }
            (Scrolling(ScrollPhase::Manual | ScrollPhase::Auto), Mismatch) => {
                Scrolling(ScrollPhase::Mismatch)
            }
            (Scrolling(ScrollPhase::Paused | ScrollPhase::Mismatch), Resume) => {
                Scrolling(ScrollPhase::Manual)
            }
            (
                Scrolling(
                    ScrollPhase::Manual
                    | ScrollPhase::Auto
                    | ScrollPhase::Paused
                    | ScrollPhase::Mismatch,
                ),
                FinishScroll,
            ) => Scrolling(ScrollPhase::Review),
            (PinMoving, EnterTextMode) => PinTextSelecting,
            (PinTextSelecting, LeaveTextMode) => PinMoving,
            (PinMoving | PinTextSelecting, EnablePassthrough) => PinPassthrough,
            (PinPassthrough, DisablePassthrough) => PinMoving,
            _ => return Err(CoreError::InvalidTransition),
        };
        self.mode = next;
        Ok(next)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_negative_monitor_origin() {
        let r = RectPx {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert!(r.contains(-100, 100));
        assert!(!r.contains(0, 100));
        assert!(!r.contains(-1921, 100));
    }
    #[test]
    fn rejects_bad_rgba_buffers() {
        assert!(RgbaFrame::new(2, 2, vec![0; 15]).is_err());
        assert!(RgbaFrame::new(2, 2, vec![0; 16]).is_ok());
    }
    #[test]
    fn rejects_invalid_transition_without_mutation() {
        let mut s = Session::default();
        assert_eq!(
            s.apply(Command::StartAuto),
            Err(CoreError::InvalidTransition)
        );
        assert_eq!(s.mode, Mode::Idle);
    }
    #[test]
    fn mismatch_resumes_in_manual_mode() {
        let mut s = Session::default();
        for cmd in [
            Command::BeginCapture,
            Command::SelectionReady,
            Command::EnterScroll,
            Command::StartAuto,
            Command::Mismatch,
        ] {
            s.apply(cmd).unwrap();
        }
        assert_eq!(
            s.apply(Command::Resume).unwrap(),
            Mode::Scrolling(ScrollPhase::Manual)
        );
    }
}
