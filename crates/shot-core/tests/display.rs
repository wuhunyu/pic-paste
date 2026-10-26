use shot_core::display::{DisplayMapping, DisplaySelection};
use shot_core::platform::{DisplayId, PlatformError};

#[test]
fn chosen_monitor_survives_unstable_discovery_order() {
    let mut selection = DisplaySelection::default();
    let monitors = [DisplayId(85), DisplayId(22), DisplayId(40)];
    assert_eq!(selection.resolve(&monitors), Ok(DisplayId(85)));
    assert_eq!(selection.advance(&monitors), Ok(DisplayId(22)));
    assert_eq!(
        selection.resolve(&[DisplayId(40), DisplayId(22), DisplayId(85)]),
        Ok(DisplayId(22))
    );
}

#[test]
fn unplugging_monitor_uses_available_display_without_stale_id() {
    let mut selection = DisplaySelection::default();
    assert_eq!(
        selection.advance(&[DisplayId(5), DisplayId(8)]),
        Ok(DisplayId(8))
    );
    assert_eq!(selection.resolve(&[DisplayId(5)]), Ok(DisplayId(5)));
    assert_eq!(selection.current(), Some(DisplayId(5)));
}

#[test]
fn empty_display_list_is_an_error_and_does_not_reset_preference() {
    let mut selection = DisplaySelection::default();
    assert_eq!(selection.resolve(&[DisplayId(12)]), Ok(DisplayId(12)));
    assert_eq!(selection.advance(&[]), Err(PlatformError::Unavailable));
    assert_eq!(selection.current(), Some(DisplayId(12)));
}

#[test]
fn cyclic_selection_wraps_across_monitors() {
    let mut selection = DisplaySelection::default();
    let monitors = [DisplayId(1), DisplayId(7)];
    assert_eq!(selection.resolve(&monitors), Ok(DisplayId(1)));
    assert_eq!(selection.advance(&monitors), Ok(DisplayId(7)));
    assert_eq!(selection.advance(&monitors), Ok(DisplayId(1)));
}

#[test]
fn mixed_dpi_negative_origin_maps_to_display_local_pixels() {
    let mapping = DisplayMapping::new((-1440.0, -100.0), (1440.0, 900.0), (2880, 1800)).unwrap();
    assert_eq!(mapping.pixel_at((-1439.5, -99.5)), Some((1, 1)));
    assert_eq!(mapping.pixel_at((-720.0, 350.0)), Some((1440, 900)));
    assert_eq!(mapping.pixel_at((0.0, 0.0)), None);
    assert_eq!(mapping.pixel_at((-1440.1, 0.0)), None);
}

#[test]
fn geometry_rejects_non_finite_sizes_and_zero_dimensions() {
    assert!(DisplayMapping::new((f64::NAN, 0.0), (1440.0, 900.0), (2880, 1800)).is_err());
    assert!(DisplayMapping::new((0.0, 0.0), (0.0, 900.0), (2880, 1800)).is_err());
    assert!(DisplayMapping::new((0.0, 0.0), (1440.0, 900.0), (0, 1800)).is_err());
}
