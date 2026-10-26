use std::sync::Arc;
use shot_core::RgbaFrame;

/// Non-destructive edit commands; rendering must always use the immutable source.
#[derive(Clone, Debug, PartialEq)]
pub enum Annotation {
    Line { from: (f32, f32), to: (f32, f32), rgba: [u8; 4], width: f32 },
    Arrow { from: (f32, f32), to: (f32, f32), rgba: [u8; 4], width: f32 },
    Brush { points: Vec<(f32, f32)>, rgba: [u8; 4], width: f32 },
    Rectangle { top_left: (f32, f32), bottom_right: (f32, f32), rgba: [u8; 4], width: f32 },
    Ellipse { center: (f32, f32), radius: (f32, f32), rgba: [u8; 4], width: f32 },
    Text { at: (f32, f32), content: String, size: f32, rgba: [u8; 4] },
    Pixelate { left: u32, top: u32, width: u32, height: u32, block_size: u32 },
    Blur { left: u32, top: u32, width: u32, height: u32, radius: u32 },
}
pub struct AnnotationDocument {
    pub source: Arc<RgbaFrame>,
    history: Vec<Annotation>,
    cursor: usize,
}
impl AnnotationDocument {
    pub fn new(source: Arc<RgbaFrame>) -> Self {
        Self { source, history: Vec::new(), cursor: 0 }
    }
    pub fn apply(&mut self, command: Annotation) {
        self.history.truncate(self.cursor);
        self.history.push(command);
        self.cursor += 1;
    }
    pub fn undo(&mut self) -> bool {
        if self.cursor == 0 { return false; }
        self.cursor -= 1;
        true
    }
    pub fn redo(&mut self) -> bool {
        if self.cursor == self.history.len() { return false; }
        self.cursor += 1;
        true
    }
    pub fn visible(&self) -> &[Annotation] {
        &self.history[..self.cursor]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line() -> Annotation {
        Annotation::Line { from: (0.0, 0.0), to: (2.0, 3.0), rgba: [255; 4], width: 1.0 }
    }
    #[test]
    fn undo_redo_and_branching_preserve_source() {
        let pixels = Arc::new(RgbaFrame::new(2, 2, vec![123; 16]).unwrap());
        let mut doc = AnnotationDocument::new(pixels.clone());
        for _ in 0..100 { doc.apply(line()); }
        for _ in 0..100 { assert!(doc.undo()); }
        assert!(!doc.undo());
        for _ in 0..100 { assert!(doc.redo()); }
        assert_eq!(doc.visible().len(), 100);
        assert!(doc.undo());
        doc.apply(Annotation::Text {
            at: (1.0, 1.0), content: String::from("test"), size: 12.0, rgba: [0; 4],
        });
        assert!(!doc.redo());
        assert_eq!(doc.visible().len(), 100);
        assert_eq!(pixels.rgba, vec![123; 16]);
    }
}
