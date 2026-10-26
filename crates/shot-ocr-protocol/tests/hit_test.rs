use shot_ocr_protocol::hit_test::{hit_line_in_capture, hit_word_in_capture, selected_text};
use shot_ocr_protocol::{OcrDocument, OcrLine, OcrWord};

fn document() -> OcrDocument {
    OcrDocument {
        image_hash: String::from("abc123"),
        pixel_width: 100,
        pixel_height: 100,
        provider: String::from("apple-vision"),
        model_revision: String::from("test"),
        lines: vec![
            OcrLine {
                text: String::from("第一行"),
                confidence: 0.98,
                polygon_px: [[10.0, 10.0], [50.0, 0.0], [60.0, 20.0], [20.0, 30.0]],
                words: vec![OcrWord {
                    text: String::from("第一"),
                    confidence: 0.95,
                    polygon_px: [[25.0, 12.0], [40.0, 12.0], [40.0, 20.0], [25.0, 20.0]],
                }],
            },
            OcrLine {
                text: String::from("Second line"),
                confidence: 0.96,
                polygon_px: [[5.0, 40.0], [90.0, 40.0], [90.0, 60.0], [5.0, 60.0]],
                words: vec![],
            },
        ],
    }
}

#[test]
fn explicit_hit_testing_respects_roi_offset_and_rotated_polygon() {
    let doc = document();
    assert_eq!(hit_line_in_capture(&doc, (100, 200), (135.0, 215.0)), Some(0));
    // Inside the bounding rectangle but outside the rotated polygon.
    assert_eq!(hit_line_in_capture(&doc, (100, 200), (111.0, 227.0)), None);
    assert_eq!(hit_line_in_capture(&doc, (100, 200), (110.0, 250.0)), Some(1));
    assert_eq!(hit_line_in_capture(&doc, (100, 200), (99.0, 215.0)), None);
}

#[test]
fn word_hit_testing_never_fabricates_character_boxes() {
    let doc = document();
    assert_eq!(hit_word_in_capture(&doc, 0, (100, 200), (133.0, 215.0)), Some(0));
    assert_eq!(hit_word_in_capture(&doc, 0, (100, 200), (114.0, 218.0)), None);
    assert_eq!(hit_word_in_capture(&doc, 1, (100, 200), (110.0, 250.0)), None);
    assert_eq!(hit_word_in_capture(&doc, 99, (100, 200), (110.0, 250.0)), None);
}

#[test]
fn selected_text_uses_document_reading_order_and_deduplicates_lines() {
    let doc = document();
    assert_eq!(selected_text(&doc, &[1, 0, 1]), "第一行\nSecond line");
    assert_eq!(selected_text(&doc, &[17]), "");
    assert_eq!(selected_text(&doc, &[]), "");
}

#[test]
fn invalid_coordinates_never_select_a_line() {
    let doc = document();
    assert_eq!(hit_line_in_capture(&doc, (0, 0), (f32::NAN, 15.0)), None);
    assert_eq!(hit_line_in_capture(&doc, (0, 0), (f32::INFINITY, 15.0)), None);
}
