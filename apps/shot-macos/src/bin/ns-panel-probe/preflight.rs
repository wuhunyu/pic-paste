//! Read-only native Mac environment preflight; does not show an NSPanel,
//! create a Metal surface, capture a screenshot, or request TCC permission.
use objc2::MainThreadMarker;
use objc2::runtime::AnyClass;
use objc2_app_kit::{NSScreen, NSWorkspace};
use objc2_foundation::NSProcessInfo;
use shot_core::native_acceptance::{NativePreflightReport, NativeScreenDiagnostic};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    // Reads existing Screen Recording authorization without triggering the
    // system permission prompt. Unlike CGRequestScreenCaptureAccess.
    #[link_name = "CGPreflightScreenCaptureAccess"]
    fn cg_preflight_screen_capture_access() -> bool;
}

pub(crate) fn run() -> Result<(), String> {
    let mtm =
        MainThreadMarker::new().ok_or("macOS preflight must run on the AppKit main thread")?;
    let version = NSProcessInfo::processInfo().operatingSystemVersion();
    let os = (
        u32::try_from(version.majorVersion).map_err(|_| "invalid macOS major version")?,
        u32::try_from(version.minorVersion).map_err(|_| "invalid macOS minor version")?,
        u32::try_from(version.patchVersion).map_err(|_| "invalid macOS patch version")?,
    );

    // NSScreen.frame() is global *AppKit* bottom-left logical points, not
    // Quartz/ScreenCaptureKit top-left source pixels. Retain raw values:
    // multiplying global desktop origins by backingScaleFactor is incorrect.
    let mut screens = Vec::new();
    for screen in NSScreen::screens(mtm).iter() {
        let frame = screen.frame();
        screens.push(NativeScreenDiagnostic::new(
            (frame.origin.x, frame.origin.y),
            (frame.size.width, frame.size.height),
            screen.backingScaleFactor(),
        )?);
    }
    let reduce_transparency =
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency();
    let native_class_available = AnyClass::get(c"NSGlassEffectView").is_some();
    // SAFETY: CGPreflightScreenCaptureAccess is a side-effect-free public
    // CoreGraphics permission status query available on macOS 10.15+.
    // It does not request authorization or return any capture pixel data.
    let tcc_screen_recording_authorized = unsafe { cg_preflight_screen_capture_access() };

    let report = NativePreflightReport::new(
        os,
        reduce_transparency,
        native_class_available,
        tcc_screen_recording_authorized,
        screens,
    )?;
    println!("{}", report.to_json_line());
    Ok(())
}
