/// Window/selection state is independent of the raw capture worker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayPhase {
    #[default]
    Idle,
    Capturing,
    Selecting,
}

#[derive(Clone, Debug, Default)]
pub struct OverlayState {
    phase: OverlayPhase,
}

impl OverlayState {
    pub fn phase(&self) -> OverlayPhase {
        self.phase
    }

    pub fn is_fullscreen(&self) -> bool {
        self.phase == OverlayPhase::Selecting
    }

    pub fn begin_capture(&mut self) -> bool {
        if self.phase != OverlayPhase::Idle {
            return false;
        }
        self.phase = OverlayPhase::Capturing;
        true
    }

    /// A delayed worker result may only open an overlay while capturing.
    pub fn captured(&mut self, success: bool) -> bool {
        if self.phase != OverlayPhase::Capturing {
            return false;
        }
        self.phase = if success {
            OverlayPhase::Selecting
        } else {
            OverlayPhase::Idle
        };
        success
    }

    pub fn dismiss(&mut self) -> bool {
        if self.phase == OverlayPhase::Idle {
            return false;
        }
        self.phase = OverlayPhase::Idle;
        true
    }

    pub fn accept(&mut self) -> bool {
        if self.phase != OverlayPhase::Selecting {
            return false;
        }
        self.phase = OverlayPhase::Idle;
        true
    }
}
