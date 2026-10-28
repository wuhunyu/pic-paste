use shot_core::RgbaFrame;
use shot_core::native_markup::NativeMark;
use shot_image::composite::composite;
use shot_image::history::Annotation;
use shot_image::native_effect_preview::{
    MAX_NATIVE_PREVIEW_PIXELS, NativeEffectPatch, can_preview_native_effects,
    preview_source_supported, render_native_effect_patch,
};

fn source() -> RgbaFrame {
    let mut rgba = Vec::new();
    for y in 0..72u32 {
        for x in 0..96u32 {
            rgba.extend_from_slice(&[
                (x * 3 + y) as u8,
                (y * 4 + x) as u8,
                ((x * y) % 255) as u8,
                if (x + y) % 5 == 0 { 0 } else { 230 },
            ]);
        }
    }
    RgbaFrame::new(96, 72, rgba).unwrap()
}

fn crop(frame: &RgbaFrame, left: u32, top: u32, width: u32, height: u32) -> Vec<u8> {
    let mut result = Vec::new();
    for y in top..top + height {
        let start = (y as usize * frame.width as usize + left as usize) * 4;
        result.extend_from_slice(&frame.rgba[start..start + width as usize * 4]);
    }
    result
}

fn expect_patch_matches_parent(mark: NativeMark, annotation: Annotation) {
    let original = source();
    let immutable = original.rgba.clone();
    let composed = composite(&original, &[annotation]).unwrap();
    let NativeEffectPatch { left, top, image } =
        render_native_effect_patch(&original, &mark).unwrap();
    assert_eq!(
        image.rgba,
        crop(&composed, left, top, image.width, image.height),
        "ROI preview must match byte-for-byte the parent PNG compositor"
    );
    assert_ne!(image.rgba, crop(&original, left, top, image.width, image.height));
    assert_eq!(original.rgba, immutable, "source RGBA must be immutable");
}

#[test]
fn mosaic_patch_matches_full_png_with_non_aligned_source_local_origin() {
    expect_patch_matches_parent(
        NativeMark::Pixelate {
            left: 7,
            top: 9,
            width: 35,
            height: 29,
        },
        Annotation::Pixelate {
            left: 7,
            top: 9,
            width: 35,
            height: 29,
            block_size: 12,
        },
    );
}

#[test]
fn blur_patch_matches_full_png_with_alpha_and_clipped_edge_samples() {
    expect_patch_matches_parent(
        NativeMark::Blur {
            left: 83,
            top: 51,
            width: 13,
            height: 21,
        },
        Annotation::Blur {
            left: 83,
            top: 51,
            width: 13,
            height: 21,
            radius: 6,
        },
    );
}

#[test]
fn disjoint_effects_preview_exactly_but_overlapping_or_mixed_annotations_fall_back() {
    let a = NativeMark::Pixelate {
        left: 2,
        top: 2,
        width: 22,
        height: 18,
    };
    let b = NativeMark::Blur {
        left: 24,
        top: 2,
        width: 26,
        height: 19,
    };
    assert!(can_preview_native_effects(&[a.clone(), b.clone()], (96, 72)));
    assert!(can_preview_native_effects(&[b.clone(), a.clone()], (96, 72)));

    let overlapping = NativeMark::Blur {
        left: 23,
        top: 3,
        width: 12,
        height: 12,
    };
    assert!(!can_preview_native_effects(
        &[a.clone(), overlapping],
        (96, 72)
    ));
    assert!(!can_preview_native_effects(
        &[
            a.clone(),
            NativeMark::Line {
                from: (0, 0),
                to: (50, 50),
            }
        ],
        (96, 72)
    ));
    assert!(!can_preview_native_effects(&[], (96, 72)));
    assert!(!can_preview_native_effects(
        &[NativeMark::Pixelate {
            left: 95,
            top: 9,
            width: 3,
            height: 12,
        }],
        (96, 72)
    ));
}

#[test]
fn preview_memory_budget_distinguishes_four_k_five_k_and_many_effects() {
    assert!(preview_source_supported(3840, 2160));
    assert!(!preview_source_supported(5120, 2880));
    assert!(!preview_source_supported(0, 200));
    assert_eq!(MAX_NATIVE_PREVIEW_PIXELS, 1_000_000);

    // Both are valid for the parent editor, but exceed native preview budget.
    assert!(!can_preview_native_effects(
        &[NativeMark::Pixelate {
            left: 1,
            top: 1,
            width: 1001,
            height: 1000,
        }],
        (2000, 2000)
    ));
    assert!(!can_preview_native_effects(
        &[
            NativeMark::Pixelate {
                left: 0,
                top: 0,
                width: 800,
                height: 800,
            },
            NativeMark::Blur {
                left: 1000,
                top: 1000,
                width: 650,
                height: 650,
            },
        ],
        (2000, 2000)
    ));
}

#[test]
fn invalid_or_unsupported_patch_fails_before_any_image_allocation() {
    let img = source();
    assert!(render_native_effect_patch(
        &img,
        &NativeMark::Blur {
            left: u32::MAX,
            top: 0,
            width: 20,
            height: 20,
        }
    )
    .is_err());
    assert!(render_native_effect_patch(
        &img,
        &NativeMark::Pixelate {
            left: 4,
            top: 4,
            width: 0,
            height: 16,
        }
    )
    .is_err());
    assert!(render_native_effect_patch(
        &img,
        &NativeMark::Line {
            from: (0, 0),
            to: (10, 10),
        }
    )
    .is_err());
}
