use std::sync::Arc;

use shot_core::RgbaFrame;
use shot_core::native_markup::NativeMark;
use shot_image::composite::composite;
use shot_image::history::{Annotation, AnnotationDocument};
use shot_image::native_import::import_native_marks;

fn image() -> Arc<RgbaFrame> {
    let mut rgba = Vec::new();
    for y in 0..60u32 {
        for x in 0..80u32 {
            let value = if (x + y) % 2 == 0 { 0 } else { 255 };
            rgba.extend_from_slice(&[value, 255 - value, (x * 3) as u8, 255]);
        }
    }
    Arc::new(RgbaFrame::new(80, 60, rgba).unwrap())
}

#[test]
fn line_pixelate_and_blur_import_to_existing_non_destructive_png_history() {
    let source = image();
    let before = source.rgba.clone();
    let mut doc = AnnotationDocument::new(Arc::clone(&source));
    let marks = [
        NativeMark::Line {
            from: (2, 3),
            to: (25, 20),
        },
        NativeMark::Pixelate {
            left: 30,
            top: 5,
            width: 20,
            height: 20,
        },
        NativeMark::Blur {
            left: 5,
            top: 25,
            width: 20,
            height: 20,
        },
    ];
    assert_eq!(import_native_marks(&mut doc, &marks).unwrap(), 3);
    assert!(matches!(&doc.visible()[0], Annotation::Line {from, to, width, ..} if *from == (2.0,3.0) && *to == (25.0,20.0) && *width == 3.0));
    assert_eq!(
        doc.visible()[1],
        Annotation::Pixelate {
            left: 30,
            top: 5,
            width: 20,
            height: 20,
            block_size: 12,
        }
    );
    assert_eq!(
        doc.visible()[2],
        Annotation::Blur {
            left: 5,
            top: 25,
            width: 20,
            height: 20,
            radius: 6,
        }
    );
    let result = composite(&source, doc.visible()).unwrap();
    assert_ne!(result.rgba, before);
    let mut png = Vec::new();
    shot_image::encode_png(&result, &mut png).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(source.rgba, before);
    for _ in 0..3 {
        assert!(doc.undo());
    }
    assert_eq!(composite(&source, doc.visible()).unwrap().rgba, before);
    for _ in 0..3 {
        assert!(doc.redo());
    }
    assert_eq!(doc.visible().len(), 3);
}

#[test]
fn invalid_trailing_effect_keeps_prior_annotation_history_unchanged() {
    let mut doc = AnnotationDocument::new(image());
    let commands = [
        NativeMark::Line {
            from: (2, 3),
            to: (25, 20),
        },
        NativeMark::Blur {
            left: 75,
            top: 50,
            width: 40,
            height: 40,
        },
    ];
    assert!(import_native_marks(&mut doc, &commands).is_err());
    assert!(doc.visible().is_empty());
}
