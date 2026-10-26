use crate::RgbaFrame;
use crate::platform::PinId;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMode {
    Move,
    TextSelect,
    Passthrough,
}

/// Keep at most one active OCR request per pin. Leaving TextSelect permanently
/// invalidates that request even if the user re-enters selection before it ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PinOcrGate {
    pending: bool,
    cancelled: bool,
}

impl PinOcrGate {
    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn begin(&mut self) -> bool {
        if self.pending {
            return false;
        }
        self.pending = true;
        self.cancelled = false;
        true
    }

    pub fn cancel_selection(&mut self) {
        if self.pending {
            self.cancelled = true;
        }
    }

    pub fn finish(&mut self, mode: PinMode) -> bool {
        if !self.pending {
            return false;
        }
        self.pending = false;
        let accepted = !self.cancelled && mode == PinMode::TextSelect;
        self.cancelled = false;
        accepted
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
}
impl PinGeometry {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height, self.opacity]
            .iter()
            .all(|value| value.is_finite())
            && self.width > 0.0
            && self.height > 0.0
            && (0.0..=1.0).contains(&self.opacity)
    }
}
pub struct Pin {
    pub id: PinId,
    pub source: Arc<RgbaFrame>,
    pub geometry: PinGeometry,
    pub mode: PinMode,
}
#[derive(Default)]
pub struct Pins {
    items: BTreeMap<u64, Pin>,
    next_id: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinError {
    InvalidGeometry,
    UnknownPin,
    ExhaustedIds,
}
impl Pins {
    pub fn insert(
        &mut self,
        source: Arc<RgbaFrame>,
        geometry: PinGeometry,
    ) -> Result<PinId, PinError> {
        if !geometry.valid() {
            return Err(PinError::InvalidGeometry);
        }
        let next = self.next_id.checked_add(1).ok_or(PinError::ExhaustedIds)?;
        self.next_id = next;
        let id = PinId(next);
        self.items.insert(
            next,
            Pin {
                id,
                source,
                geometry,
                mode: PinMode::Move,
            },
        );
        Ok(id)
    }
    pub fn get(&self, id: PinId) -> Option<&Pin> {
        self.items.get(&id.0)
    }
    pub fn set_mode(&mut self, id: PinId, mode: PinMode) -> Result<(), PinError> {
        let pin = self.items.get_mut(&id.0).ok_or(PinError::UnknownPin)?;
        pin.mode = mode;
        Ok(())
    }
    pub fn move_to(&mut self, id: PinId, x: f32, y: f32) -> Result<(), PinError> {
        let pin = self.items.get_mut(&id.0).ok_or(PinError::UnknownPin)?;
        let mut next = pin.geometry;
        next.x = x;
        next.y = y;
        if !next.valid() {
            return Err(PinError::InvalidGeometry);
        }
        pin.geometry = next;
        Ok(())
    }
    pub fn recover_passthrough(&mut self) -> usize {
        let mut count = 0;
        for pin in self.items.values_mut() {
            if pin.mode == PinMode::Passthrough {
                pin.mode = PinMode::Move;
                count += 1;
            }
        }
        count
    }
    pub fn remove(&mut self, id: PinId) -> Option<Pin> {
        self.items.remove(&id.0)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn geometry() -> PinGeometry {
        PinGeometry {
            x: -200.0,
            y: 40.0,
            width: 300.0,
            height: 200.0,
            opacity: 0.85,
        }
    }
    #[test]
    fn multiple_pins_share_source_and_recover_passthrough() {
        let source = Arc::new(RgbaFrame::new(2, 2, vec![255; 16]).unwrap());
        let mut pins = Pins::default();
        let a = pins.insert(source.clone(), geometry()).unwrap();
        let b = pins.insert(source.clone(), geometry()).unwrap();
        assert!(Arc::ptr_eq(
            &pins.get(a).unwrap().source,
            &pins.get(b).unwrap().source
        ));
        pins.set_mode(a, PinMode::Passthrough).unwrap();
        pins.set_mode(b, PinMode::Passthrough).unwrap();
        assert_eq!(pins.recover_passthrough(), 2);
        assert_eq!(pins.get(a).unwrap().mode, PinMode::Move);
        assert_eq!(pins.get(b).unwrap().mode, PinMode::Move);
        assert_eq!(pins.remove(a).unwrap().id, a);
        assert_eq!(pins.len(), 1);
    }
    #[test]
    fn invalid_move_cannot_corrupt_geometry() {
        let source = Arc::new(RgbaFrame::new(1, 1, vec![0; 4]).unwrap());
        let mut pins = Pins::default();
        let id = pins.insert(source, geometry()).unwrap();
        assert_eq!(
            pins.move_to(id, f32::NAN, 0.0),
            Err(PinError::InvalidGeometry)
        );
        assert_eq!(pins.get(id).unwrap().geometry.x, -200.0);
    }
}
