use shot_ocr_protocol::hit_test::{
    TextPickMode, TextSelection, hit_text_selection, selected_selection_text, selection_polygon,
};
use shot_ocr_protocol::{OcrDocument, OcrLine, OcrWord};

fn document() -> OcrDocument {
    OcrDocument {
        image_hash: String::from("test-image"),
        pixel_width: 120,
        pixel_height: 80,
        provider: String::from("apple-vision"),
        model_revision: String::from("test"),
        lines: vec![OcrLine {
            text: String::from("你好 world"),
            confidence: 0.98,
            polygon_px: [[10.0, 10.0], [100.0, 10.0], [100.0, 32.0], [10.0, 32.0]],
            words: vec![
                OcrWord {
                    text: String::from("你好"),
                    confidence: 0.98,
                    polygon_px: [[10.0, 10.0], [48.0, 10.0], [48.0, 32.0], [10.0, 32.0]],
                },
                OcrWord {
                    text: String::from("world"),
                    confidence: 0.98,
                    polygon_px: [[56.0, 10.0], [100.0, 10.0], [100.0, 32.0], [56.0, 32.0]],
                },
            ],
        }],
    }
}

#[test]
fn selects_exact_provider_word_in_source_pixel_coordinates() {
    let doc = document();
    let selected = hit_text_selection(&doc, (100, 200), (160.0, 214.0), TextPickMode::Unit);
    assert_eq!(selected, Some(TextSelection::Unit { line: 0, unit: 1 }));
    assert_eq!(
        selected_selection_text(&doc, selected.unwrap()),
        Some("world")
    );
    assert_eq!(
        selection_polygon(&doc, selected.unwrap()).unwrap()[0],
        [56.0, 10.0]
    );
}

#[test]
fn line_mode_returns_whole_line_instead_of_subdividing() {
    let doc = document();
    let selected = hit_text_selection(&doc, (100, 200), (160.0, 214.0), TextPickMode::Line);
    assert_eq!(selected, Some(TextSelection::Line(0)));
    assert_eq!(
        selected_selection_text(&doc, selected.unwrap()),
        Some("你好 world")
    );
}

#[test]
fn unit_mode_never_fabricates_word_or_character_bounds() {
    let mut doc = document();
    doc.lines[0].words.clear();
    assert_eq!(
        hit_text_selection(&doc, (0, 0), (30.0, 15.0), TextPickMode::Unit),
        None
    );
    assert_eq!(
        hit_text_selection(&doc, (0, 0), (30.0, 15.0), TextPickMode::Line),
        Some(TextSelection::Line(0))
    );
    assert_eq!(
        selected_selection_text(&doc, TextSelection::Unit { line: 0, unit: 1 }),
        None
    );
    assert_eq!(selection_polygon(&doc, TextSelection::Line(10)), None);
}

#[test]
fn utf8_character_box_is_selected_without_splitting_a_line_rect() {
    let mut doc = document();
    doc.lines[0].words = vec![
        OcrWord {
            text: String::from("中"),
            confidence: 0.97,
            polygon_px: [[20.0, 10.0], [35.0, 10.0], [35.0, 32.0], [20.0, 32.0]],
        },
        OcrWord {
            text: String::from("文"),
            confidence: 0.97,
            polygon_px: [[35.0, 10.0], [50.0, 10.0], [50.0, 32.0], [35.0, 32.0]],
        },
    ];
    let selected = hit_text_selection(&doc, (0, 0), (40.0, 20.0), TextPickMode::Unit);
    assert_eq!(selected, Some(TextSelection::Unit { line: 0, unit: 1 }));
    assert_eq!(selected_selection_text(&doc, selected.unwrap()), Some("文"));
    assert_eq!(
        hit_text_selection(&doc, (0, 0), (f32::NAN, 20.0), TextPickMode::Unit),
        None
    );
}
