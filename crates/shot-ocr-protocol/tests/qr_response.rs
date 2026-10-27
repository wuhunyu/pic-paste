use shot_ocr_protocol::{OcrResponse, read_response};
use std::io;

fn framed(payload: &str) -> Vec<u8> {
    let mut output = (payload.len() as u32).to_le_bytes().to_vec();
    output.extend_from_slice(payload.as_bytes());
    output
}

#[test]
fn decodes_explicit_offline_qr_payload_with_real_pixel_geometry() {
    let json = r#"{"protocol_version":1,"qr_document":{"image_hash":"abc","pixel_width":640,"pixel_height":480,"provider":"apple-vision-qr","codes":[{"payload":"https://example.com/","symbology":"qr","polygon_px":[[10,20],[90,20],[90,100],[10,100]]}]}}"#;
    let doc = match read_response(framed(json).as_slice()).unwrap() {
        OcrResponse::QrDocument(document) => document,
        _ => panic!("expected QR document"),
    };
    assert_eq!(doc.codes.len(), 1);
    assert_eq!(doc.codes[0].payload, "https://example.com/");
    assert_eq!(doc.codes[0].polygon_px[1], [90.0, 20.0]);
}

#[test]
fn empty_scan_is_valid_and_does_not_invent_payloads() {
    let json = r#"{"protocol_version":1,"qr_document":{"image_hash":"a","pixel_width":256,"pixel_height":256,"provider":"apple-vision-qr","codes":[]}}"#;
    let response = read_response(framed(json).as_slice()).unwrap();
    let OcrResponse::QrDocument(doc) = response else {
        panic!("expected empty QR scan");
    };
    assert!(doc.codes.is_empty());
}

#[test]
fn rejects_malformed_qr_frames_before_ui_or_clipboard() {
    let prefix = r#"{"protocol_version":1,"qr_document":{"image_hash":"abc","pixel_width":100,"pixel_height":100,"provider":"apple-vision-qr","codes":["#;
    let suffix = r#"]}}"#;
    for code in [
        r#"{"payload":"","symbology":"qr","polygon_px":[[1,1],[5,1],[5,5],[1,5]]}"#,
        r#"{"payload":"abc","symbology":"qr","polygon_px":[[1,1],[999,1],[5,5],[1,5]]}"#,
        r#"{"payload":"abc","symbology":"unknown","polygon_px":[[1,1],[5,1],[5,5],[1,5]]}"#,
        r#"{"symbology":"qr","polygon_px":[[1,1],[5,1],[5,5],[1,5]]}"#,
    ] {
        let frame = framed(&format!("{prefix}{code}{suffix}"));
        assert_eq!(
            read_response(frame.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn rejects_qr_and_text_in_same_envelope() {
    let json = r#"{"protocol_version":1,"qr_document":{"image_hash":"abc","pixel_width":64,"pixel_height":64,"provider":"apple-vision-qr","codes":[]},"document":{"image_hash":"abc","pixel_width":64,"pixel_height":64,"provider":"apple-vision","model_revision":"test","lines":[]}}"#;
    assert!(read_response(framed(json).as_slice()).is_err());
}
