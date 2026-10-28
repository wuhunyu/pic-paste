use shot_core::ime::{ImeKeyRoute, ImeKeyRouting};

#[test]
fn handled_native_key_does_not_mutate_egui_twice() {
    let mut route = ImeKeyRouting::default();
    assert_eq!(route.key_down(42, true, true), ImeKeyRoute::NativeOnly);
    assert_eq!(route.key_up(42), ImeKeyRoute::SuppressRelease);
}

#[test]
fn unhandled_native_key_reaches_egui_with_balanced_release() {
    let mut route = ImeKeyRouting::default();
    assert_eq!(route.key_down(42, true, false), ImeKeyRoute::Egui);
    assert_eq!(route.key_up(42), ImeKeyRoute::Egui);
}

#[test]
fn modified_shortcuts_bypass_native_context() {
    let mut route = ImeKeyRouting::default();
    assert_eq!(route.key_down(8, false, false), ImeKeyRoute::Egui);
    assert_eq!(route.key_up(8), ImeKeyRoute::Egui);
}

#[test]
fn repeat_and_focus_loss_do_not_leave_stale_suppressed_releases() {
    let mut route = ImeKeyRouting::default();
    assert_eq!(route.key_down(42, true, true), ImeKeyRoute::NativeOnly);
    assert_eq!(route.key_down(42, true, true), ImeKeyRoute::NativeOnly);
    route.reset();
    assert_eq!(route.key_up(42), ImeKeyRoute::Egui);
}

#[test]
fn second_key_does_not_inherit_other_key_suppression() {
    let mut route = ImeKeyRouting::default();
    assert_eq!(route.key_down(11, true, true), ImeKeyRoute::NativeOnly);
    assert_eq!(route.key_down(12, true, false), ImeKeyRoute::Egui);
    assert_eq!(route.key_up(12), ImeKeyRoute::Egui);
    assert_eq!(route.key_up(11), ImeKeyRoute::SuppressRelease);
}
