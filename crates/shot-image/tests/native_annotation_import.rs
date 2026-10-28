use std::sync::Arc;

use shot_core::RgbaFrame;
use shot_core::native_markup::NativeMark;
use shot_image::composite::composite;
use shot_image::history::{Annotation, AnnotationDocument};
use shot_image::native_import::import_native_marks;

fn document() -> AnnotationDocument {
    AnnotationDocument::new(Arc::new(
        RgbaFrame::new(64, 48, vec![255; 64 * 48 * 4]).unwrap(),
    ))
}

fn sample_marks() -> Vec<NativeMark> {
    vec![
        NativeMark::Brush {
            points: vec![(2, 3), (4, 5), (6, 7)],
        },
        NativeMark::Arrow {
            from: (8, 9),
            to: (20, 11),
        },
        NativeMark::Rectangle {
            top_left: (5, 7),
            bottom_right: (30, 25),
        },
        NativeMark::Text {
            at: (12, 17),
            content: String::from("Hello 世界"),
        },
    ]
}

#[test]
fn imported_native_marks_are_undoable_redoable_and_preserve_original_pixels() {
    let mut doc = document();
    let original = doc.source.rgba.clone();
    assert_eq!(import_native_marks(&mut doc, &sample_marks()).unwrap(), 4);
    assert_eq!(doc.visible().len(), 4);
    assert!(
        matches!(&doc.visible()[0], Annotation::Brush { points, width, rgba } if points.len()==3 && *width==3.0 && *rgba==[39,178,231,255])
    );
    assert!(
        matches!(&doc.visible()[3], Annotation::Text { content, size, .. } if content=="Hello 世界" && *size==26.0)
    );
    assert_eq!(doc.source.rgba, original);

    assert!(doc.undo());
    assert_eq!(doc.visible().len(), 3);
    assert!(doc.redo());
    assert_eq!(doc.visible().len(), 4);
    assert_eq!(doc.source.rgba, original);
}

#[test]
fn real_composite_and_png_come_from_imported_annotation_history() {
    let mut doc = document();
    let mut before_png = Vec::new();
    shot_image::encode_png(&doc.source, &mut before_png).unwrap();

    let marks = vec![
        NativeMark::Rectangle {
            top_left: (5, 5),
            bottom_right: (25, 24),
        },
        NativeMark::Arrow {
            from: (10, 12),
            to: (30, 20),
        },
    ];
    import_native_marks(&mut doc, &marks).unwrap();
    let rendered = composite(&doc.source, doc.visible()).unwrap();
    assert_eq!(rendered.width, 64);
    assert_eq!(rendered.height, 48);
    assert_ne!(rendered.rgba, doc.source.rgba);

    let mut after_png = Vec::new();
    shot_image::encode_png(&rendered, &mut after_png).unwrap();
    assert!(after_png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_ne!(before_png, after_png);
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(
        composite(&doc.source, doc.visible()).unwrap().rgba,
        doc.source.rgba
    );
}

#[test]
fn corrupted_second_mark_never_partially_changes_existing_history() {
    let mut doc = document();
    doc.apply(Annotation::Line {
        from: (2.0, 2.0),
        to: (3.0, 3.0),
        rgba: [100, 0, 0, 255],
        width: 2.0,
    });
    let original = doc.visible().to_vec();
    let mut marks = sample_marks();
    marks[1] = NativeMark::Arrow {
        from: (63, 47),
        to: (64, 47),
    };
    assert!(import_native_marks(&mut doc, &marks).is_err());
    assert_eq!(doc.visible(), original.as_slice());
    let mut invalid_text = sample_marks();
    invalid_text[3] = NativeMark::Text {
        at: (1, 1),
        content: String::from(" "),
    };
    assert!(import_native_marks(&mut doc, &invalid_text).is_err());
    assert_eq!(doc.visible(), original.as_slice());
}

#[test]
fn empty_annotation_reply_is_noop_and_cannot_erase_existing_history() {
    let mut doc = document();
    assert_eq!(import_native_marks(&mut doc, &[]).unwrap(), 0);
    assert!(doc.visible().is_empty());
}
