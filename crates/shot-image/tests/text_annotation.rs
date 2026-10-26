use shot_core::RgbaFrame;
use shot_image::composite::composite;
use shot_image::history::Annotation;

#[test]
fn text_annotation_draws_and_preserves_immutable_source() {
    let source = RgbaFrame::new(192, 72, vec![0; 192 * 72 * 4]).unwrap();
    let annotated = composite(
        &source,
        &[Annotation::Text {
            at: (8.0, 8.0),
            content: String::from("Hello"),
            size: 26.0,
            rgba: [20, 180, 240, 255],
        }],
    )
    .unwrap();
    assert!(annotated.rgba.chunks_exact(4).any(|px| px[3] > 0));
    assert!(source.rgba.iter().all(|pixel| *pixel == 0));
}

#[test]
fn rejects_bad_text_size_and_unbounded_content() {
    let source = RgbaFrame::new(128, 48, vec![0; 128 * 48 * 4]).unwrap();
    for (size, text) in [
        (f32::NAN, "test".to_owned()),
        (0.0, "test".to_owned()),
        (10000.0, "test".to_owned()),
        (12.0, "字".repeat(10_001)),
    ] {
        assert!(
            composite(
                &source,
                &[Annotation::Text {
                    at: (0.0, 0.0),
                    content: text,
                    size,
                    rgba: [255; 4],
                }]
            )
            .is_err()
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_system_cjk_font_renders_real_chinese_characters() {
    let source = RgbaFrame::new(192, 72, vec![0; 192 * 72 * 4]).unwrap();
    let annotated = composite(
        &source,
        &[Annotation::Text {
            at: (8.0, 8.0),
            content: String::from("中文标注"),
            size: 26.0,
            rgba: [255, 255, 255, 255],
        }],
    )
    .unwrap();
    assert!(annotated.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
}
