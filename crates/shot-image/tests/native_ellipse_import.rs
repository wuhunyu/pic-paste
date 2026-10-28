use std::sync::Arc;

use shot_core::RgbaFrame;
use shot_core::native_markup::NativeMark;
use shot_image::composite::composite;
use shot_image::history::{Annotation, AnnotationDocument};
use shot_image::native_import::import_native_marks;

#[test]
fn imported_native_ellipse_renders_to_png_and_remains_undoable() {
    let source = Arc::new(RgbaFrame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
    let mut doc = AnnotationDocument::new(Arc::clone(&source));
    let marks = [NativeMark::Ellipse {
        top_left: (10, 12),
        bottom_right: (50, 52),
    }];
    assert_eq!(import_native_marks(&mut doc, &marks).unwrap(), 1);
    assert_eq!(
        doc.visible(),
        &[Annotation::Ellipse {
            center: (30.0, 32.0),
            radius: (20.0, 20.0),
            rgba: [39, 178, 231, 255],
            width: 3.0,
        }]
    );
    let painted = composite(&source, doc.visible()).unwrap();
    assert_ne!(painted.rgba, source.rgba);
    let mut png = Vec::new();
    shot_image::encode_png(&painted, &mut png).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(doc.undo());
    assert_eq!(composite(&source, doc.visible()).unwrap().rgba, source.rgba);
    assert!(doc.redo());
    assert_eq!(doc.visible().len(), 1);
    assert_eq!(source.rgba, vec![255; 100 * 80 * 4]);
}

#[test]
fn invalid_native_ellipse_never_partially_mutates_existing_history() {
    let source = Arc::new(RgbaFrame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
    let mut doc = AnnotationDocument::new(source);
    let marks = [
        NativeMark::Ellipse {
            top_left: (10, 12),
            bottom_right: (50, 52),
        },
        NativeMark::Ellipse {
            top_left: (40, 20),
            bottom_right: (100, 40),
        },
    ];
    assert!(import_native_marks(&mut doc, &marks).is_err());
    assert!(doc.visible().is_empty());
}
