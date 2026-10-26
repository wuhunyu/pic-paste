use shot_core::platform::ScrollAxis;
use crate::limits::{validate_canvas, SizeError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Configuring, Capturing, Paused, Mismatch, Reviewing, Cancelled }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input { Manual, Auto }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError { InvalidTransition, Size(SizeError) }

#[derive(Clone, Debug)]
pub struct Session {
    pub phase: Phase,
    pub axis: ScrollAxis,
    pub input: Input,
    pub canvas_width: u64,
    pub canvas_height: u64,
    pub accepted_frames: u32,
}

impl Session {
    pub fn new(width: u64, height: u64, axis: ScrollAxis) -> Result<Self, SessionError> {
        validate_canvas(width, height).map_err(SessionError::Size)?;
        Ok(Self {
            phase: Phase::Configuring,
            axis,
            input: Input::Manual,
            canvas_width: width,
            canvas_height: height,
            accepted_frames: 0,
        })
    }

    pub fn start(&mut self, input: Input) -> Result<(), SessionError> {
        if self.phase != Phase::Configuring { return Err(SessionError::InvalidTransition); }
        self.input = input;
        self.phase = Phase::Capturing;
        Ok(())
    }

    pub fn accept_strip(&mut self, pixels: u64) -> Result<(), SessionError> {
        if self.phase != Phase::Capturing || pixels == 0 {
            return Err(SessionError::InvalidTransition);
        }
        let (w, h) = match self.axis {
            ScrollAxis::Vertical => (self.canvas_width, self.canvas_height.checked_add(pixels)),
            ScrollAxis::Horizontal => {
                let width = self.canvas_width.checked_add(pixels);
                (width.unwrap_or(u64::MAX), Some(self.canvas_height))
            }
        };
        let h = h.ok_or(SessionError::Size(SizeError::Overflow))?;
        validate_canvas(w, h).map_err(SessionError::Size)?;
        self.canvas_width = w;
        self.canvas_height = h;
        self.accepted_frames += 1;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), SessionError> {
        if self.phase != Phase::Capturing { return Err(SessionError::InvalidTransition); }
        self.phase = Phase::Paused;
        Ok(())
    }

    pub fn mismatch(&mut self) -> Result<(), SessionError> {
        if self.phase != Phase::Capturing { return Err(SessionError::InvalidTransition); }
        self.phase = Phase::Mismatch;
        Ok(())
    }

    pub fn resume_manually(&mut self) -> Result<(), SessionError> {
        if !matches!(self.phase, Phase::Paused | Phase::Mismatch) {
            return Err(SessionError::InvalidTransition);
        }
        self.input = Input::Manual;
        self.phase = Phase::Capturing;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), SessionError> {
        if !matches!(self.phase, Phase::Capturing | Phase::Paused | Phase::Mismatch) {
            return Err(SessionError::InvalidTransition);
        }
        self.phase = Phase::Reviewing;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.phase = Phase::Cancelled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_fallback_after_auto_mismatch() {
        let mut s = Session::new(100, 100, ScrollAxis::Vertical).unwrap();
        s.start(Input::Auto).unwrap();
        s.accept_strip(30).unwrap();
        s.mismatch().unwrap();
        assert!(s.accept_strip(1).is_err());
        s.resume_manually().unwrap();
        assert_eq!(s.input, Input::Manual);
        s.accept_strip(20).unwrap();
        assert_eq!((s.canvas_width, s.canvas_height), (100, 150));
        s.finish().unwrap();
        assert_eq!(s.phase, Phase::Reviewing);
    }

    #[test]
    fn limit_failure_does_not_mutate_canvas() {
        let mut s = Session::new(100, 59_999, ScrollAxis::Vertical).unwrap();
        s.start(Input::Manual).unwrap();
        assert_eq!(s.accept_strip(2), Err(SessionError::Size(SizeError::TooLong)));
        assert_eq!(s.canvas_height, 59_999);
    }
}
