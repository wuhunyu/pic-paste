use shot_core::pin_texture_budget::{
    MAX_PIN_COUNT, MAX_TEXTURE_BYTES, PinBudgetError, PinTextureBudget,
};

const FULL_HD_BYTES: u64 = 1920 * 1080 * 4;

#[test]
fn five_full_hd_pins_fit_and_ninth_distinct_image_is_rejected() {
    let mut budget = PinTextureBudget::default();
    for id in 1..=8 {
        assert_eq!(budget.reserve(id, 1920, 1080, None), Ok(id));
        if id == 5 {
            assert_eq!(budget.resident_bytes(), 5 * FULL_HD_BYTES);
            assert_eq!(budget.active_pins(), 5);
        }
    }
    assert_eq!(budget.resident_bytes(), 8 * FULL_HD_BYTES);
    assert!(8 * FULL_HD_BYTES <= MAX_TEXTURE_BYTES);
    assert_eq!(
        budget.reserve(9, 1920, 1080, None),
        Err(PinBudgetError::TextureBudgetExceeded)
    );
    assert_eq!(budget.active_pins(), 8);
    assert_eq!(budget.resident_bytes(), 8 * FULL_HD_BYTES);
}

#[test]
fn exact_duplicate_reuses_texture_group_until_last_pin_releases() {
    let mut budget = PinTextureBudget::default();
    assert_eq!(budget.reserve(101, 1920, 1080, None), Ok(101));
    assert_eq!(budget.reserve(102, 1920, 1080, Some(101)), Ok(101));
    assert_eq!(budget.resident_bytes(), FULL_HD_BYTES);
    assert_eq!(budget.active_pins(), 2);

    assert!(budget.release(101));
    assert_eq!(budget.resident_bytes(), FULL_HD_BYTES);
    assert_eq!(budget.active_pins(), 1);
    assert_eq!(budget.reserve(103, 1920, 1080, Some(101)), Ok(101));

    assert!(budget.release(102));
    assert_eq!(budget.resident_bytes(), FULL_HD_BYTES);
    assert!(budget.release(103));
    assert_eq!(budget.resident_bytes(), 0);
    assert_eq!(budget.active_pins(), 0);
    assert!(!budget.release(103));
}

#[test]
fn pin_count_is_bounded_even_for_shared_textures() {
    let mut budget = PinTextureBudget::default();
    budget.reserve(1, 1920, 1080, None).unwrap();
    for id in 2..=MAX_PIN_COUNT as u64 {
        assert_eq!(budget.reserve(id, 1920, 1080, Some(1)), Ok(1));
    }
    assert_eq!(budget.active_pins(), MAX_PIN_COUNT);
    assert_eq!(budget.resident_bytes(), FULL_HD_BYTES);
    assert_eq!(
        budget.reserve(200, 1920, 1080, Some(1)),
        Err(PinBudgetError::TooManyPins)
    );
    assert_eq!(budget.active_pins(), MAX_PIN_COUNT);
}

#[test]
fn per_pin_dimensions_and_pixel_limits_reject_unsafe_uploads() {
    for (width, height, error) in [
        (0, 1080, PinBudgetError::InvalidDimensions),
        (1920, 0, PinBudgetError::InvalidDimensions),
        (4097, 1, PinBudgetError::TextureTooLarge),
        (1, 4097, PinBudgetError::TextureTooLarge),
        (2048, 2048, PinBudgetError::TextureTooLarge),
        (60_000, 10, PinBudgetError::TextureTooLarge),
    ] {
        assert_eq!(
            PinTextureBudget::validate_dimensions(width, height),
            Err(error)
        );
    }
    assert_eq!(PinTextureBudget::validate_dimensions(4000, 1000), Ok(16_000_000));
}

#[test]
fn rejected_new_pins_never_mutate_accounting() {
    let mut budget = PinTextureBudget::default();
    budget.reserve(7, 100, 100, None).unwrap();
    let before = (budget.active_pins(), budget.resident_bytes());
    assert_eq!(
        budget.reserve(7, 100, 100, None),
        Err(PinBudgetError::DuplicatePinId)
    );
    assert_eq!(
        budget.reserve(8, 100, 100, Some(404)),
        Err(PinBudgetError::MissingTextureGroup)
    );
    assert_eq!(
        budget.reserve(9, 101, 100, Some(7)),
        Err(PinBudgetError::SharedSizeMismatch)
    );
    assert_eq!(
        budget.reserve(10, 4096, 4096, None),
        Err(PinBudgetError::TextureTooLarge)
    );
    assert_eq!((budget.active_pins(), budget.resident_bytes()), before);
}

#[test]
fn released_budget_can_be_reused_without_eviction() {
    let mut budget = PinTextureBudget::default();
    for id in 1..=8 {
        budget.reserve(id, 1920, 1080, None).unwrap();
    }
    assert!(budget.release(3));
    assert_eq!(budget.active_pins(), 7);
    assert_eq!(budget.resident_bytes(), 7 * FULL_HD_BYTES);
    assert_eq!(budget.reserve(9, 1920, 1080, None), Ok(9));
    assert_eq!(budget.resident_bytes(), 8 * FULL_HD_BYTES);
}
