use shot_core::window_candidates::{
    WindowBounds, candidate_at, candidates_for_capture,
};
use shot_core::RectPx;

fn window(id: u64, rect: (f64, f64, f64, f64)) -> WindowBounds {
    WindowBounds {
        id,
        origin_pt: (rect.0, rect.1),
        size_pt: (rect.2, rect.3),
        layer: 0,
        on_screen: true,
    }
}

#[test]
fn maps_secondary_retina_monitor_into_local_source_pixels() {
    let list = candidates_for_capture(
        (-1440.0, 0.0),
        (1440.0, 900.0),
        (2880, 1800),
        &[window(7, (-1200.0, 100.0, 600.0, 300.0))],
    );
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].rect,
        RectPx { x: 480, y: 200, width: 1200, height: 600 },
    );
    assert_eq!(candidate_at(&list, (481, 201)).map(|w| w.id), Some(7));
    assert_eq!(candidate_at(&list, (480, 800)).map(|w| w.id), None);
}

#[test]
fn clips_partial_offscreen_windows_to_capture_and_rejects_outside() {
    let list = candidates_for_capture(
        (0.0, 0.0),
        (1000.0, 800.0),
        (1000, 800),
        &[window(1, (-100.0, 50.0, 300.0, 400.0)),
          window(2, (950.0, 300.0, 200.0, 300.0)),
          window(3, (1100.0, 30.0, 500.0, 400.0))],
    );
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].rect, RectPx { x: 0, y: 50, width: 200, height: 400 });
    assert_eq!(list[1].rect, RectPx { x: 950, y: 300, width: 50, height: 300 });
}

#[test]
fn ignores_desktop_layers_invisible_tiny_invalid_and_duplicate_windows() {
    let mut desktop = window(1, (0.0, 0.0, 800.0, 600.0));
    desktop.layer = -1;
    let mut hidden = window(2, (0.0, 0.0, 500.0, 500.0));
    hidden.on_screen = false;
    let list = candidates_for_capture(
        (0.0, 0.0),
        (800.0, 600.0),
        (800, 600),
        &[desktop, hidden, window(3, (0.0, 0.0, 10.0, 200.0)),
          window(4, (f64::NAN, 0.0, 100.0, 100.0)),
          window(5, (30.0, 40.0, 300.0, 200.0)),
          window(5, (100.0, 40.0, 300.0, 200.0))],
    );
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, 5);
}

#[test]
fn prefers_front_to_back_discovery_order_for_overlap() {
    let list = candidates_for_capture(
        (0.0, 0.0),
        (1000.0, 800.0),
        (1000, 800),
        &[window(12, (100.0, 100.0, 300.0, 300.0)),
          window(13, (0.0, 0.0, 700.0, 700.0))],
    );
    assert_eq!(candidate_at(&list, (200, 200)).map(|w| w.id), Some(12));
    assert_eq!(candidate_at(&list, (500, 500)).map(|w| w.id), Some(13));
}

#[test]
fn invalid_display_geometry_returns_no_candidates() {
    assert!(candidates_for_capture(
        (f64::NAN, 0.0), (100.0, 100.0), (200, 200),
        &[window(1, (0.0, 0.0, 100.0, 100.0))]
    ).is_empty());
}
