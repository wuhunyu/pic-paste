use shot_ocr_protocol::{OcrResponse, read_response};
use std::io;

fn framed(body: &[u8]) -> Vec<u8> {
    let mut output = u32::try_from(body.len()).unwrap().to_le_bytes().to_vec();
    output.extend_from_slice(body);
    output
}

#[test]
fn correctly_decodes_bounded_vision_text_lines_with_pixel_positions() {
    let content = br#"{"protocol_version":1,"document":{"image_hash":"a8","pixel_width":640,"pixel_height":480,"provider":"apple-vision","model_revision":"macos-vision","lines":[{"text":"Hello World","confidence":0.97,"polygon_px":[[10.0,20.0],[180.0,20.0],[180.0,44.0],[10.0,44.0]],"words":[]}]}}"#;
    let response = read_response(framed(content).as_slice()).unwrap();
    let OcrResponse::Document(doc) = response else { panic!("expected OCR document") };
    assert_eq!(doc.pixel_width, 640);
    assert_eq!(doc.lines[0].text, "Hello World");
    assert_eq!(doc.lines[0].polygon_px[0], [10.0, 20.0]);
}

#[test]
fn propagates_helper_errors_as_values_not_parsing_failures() {
    let content = br#"{"protocol_version":1,"error":"Vision permission denied"}"#;
    assert_eq!(
        read_response(framed(content).as_slice()).unwrap(),
        OcrResponse::Error("Vision permission denied".to_string())
    );
}

#[test]
fn rejects_oversized_or_truncated_response_before_unbounded_allocations() {
    let mut frame = (u32::MAX).to_le_bytes().to_vec();
    let err = read_response(frame.as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    frame = framed(b"{\"protocol_version\":1");
    assert!(read_response(frame.as_slice()).is_err());
}

#[test]
fn rejects_unknown_protocol_versions_and_non_finite_coordinates() {
    let bad_version = br#"{"protocol_version":9,"error":"no"}"#;
    assert!(read_response(framed(bad_version).as_slice()).is_err());
    let bad_coordinates = br#"{"protocol_version":1,"document":{"image_hash":"x","pixel_width":10,"pixel_height":10,"provider":"apple-vision","model_revision":"a","lines":[{"text":"x","confidence":0.5,"polygon_px":[[999.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]],"words":[]}]}}"#;
    assert!(read_response(framed(bad_coordinates).as_slice()).is_err());
}
