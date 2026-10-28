//! Interactive native controls inside the opt-in NSPanel material capsule.
//! Cocoa retains the actual buttons; we separately retain their action target
//! because NSControl.target is weak. Captured pixels and production UI are
//! not changed by these proof-only actions.
use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSColor, NSEvent, NSGlassEffectView, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWorkspace,
};
use objc2_foundation::{
    NSObject, NSObjectProtocol, NSPoint, NSProcessInfo, NSRect, NSSize, ns_string,
};
use shot_core::glass_toolbar::{
    FloatingBarFrame, GlassMaterial, GlassToolbarAction, PreviewTheme, select_glass_material,
};

const BAR_WIDTH: f64 = 236.0;
const BAR_HEIGHT: f64 = 44.0;

#[derive(Debug)]
struct GlassTargetIvars {
    pending: Cell<Option<GlassToolbarAction>>,
}

struct GlassActionTargetIvars {
    pending: Cell<Option<GlassToolbarAction>>,
}

define_class!(
    // SAFETY: NSObject has no special subclassing requirements, and this
    // type has no Drop implementation. All AppKit actions stay on the
    // AppKit main thread. Method signatures match their Objective-C selectors.
    #[unsafe(super(NSObject))]
    #[ivars = GlassActionTargetIvars]
    #[thread_kind = MainThreadOnly]
    #[ivars = GlassTargetIvars]
    #[derive(Debug)]
    struct GlassActionTarget;

    unsafe impl NSObjectProtocol for GlassActionTarget {}

    impl GlassActionTarget {
        #[unsafe(method(togglePreviewTheme:))]
        fn toggle_theme(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ToggleTheme));
        }

        #[unsafe(method(closeProbe:))]
        fn close_probe(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::CloseProbe));
        }
    }
);

impl GlassActionTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let obj = Self::alloc(mtm);
        let obj = obj.set_ivars(GlassTargetIvars {
            pending: Cell::new(None),
        });
        // SAFETY: NSObject initialization creates a valid instance of the
        // registered main-thread-only subclass.
        unsafe { msg_send![super(obj), init] }
    }

    fn take_action(&self) -> Option<GlassToolbarAction> {
        self.ivars().pending.take()
    }
}

fn current_material() -> GlassMaterial {
    let os = NSProcessInfo::processInfo().operatingSystemVersion();
    let mac_major = u32::try_from(os.majorVersion).unwrap_or_default();
    let native_class_exists = AnyClass::get(c"NSGlassEffectView").is_some();
    let reduce_transparency =
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency();
    select_glass_material(mac_major, native_class_exists, reduce_transparency)
}

/// Retained AppKit subview. Its host NSView (which owns the Metal drawable)
/// remains unchanged and must outlive the WGPU Surface during shutdown.
pub(crate) struct ProbeGlassToolbar {
    view: Retained<NSView>,
    theme_button: Retained<NSButton>,
    // Must outlive the NSButtons: NSControl stores its target weakly.
    _target: Retained<GlassActionTarget>,
    pub(crate) material: GlassMaterial,
}

impl ProbeGlassToolbar {
    pub(crate) fn new(mtm: MainThreadMarker, host: &NSView) -> Self {
        let material = current_material();
        let local_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(BAR_WIDTH, BAR_HEIGHT));
        let content = NSView::initWithFrame(NSView::alloc(mtm), local_rect);
        let target = GlassActionTarget::new(mtm);
        let theme_button = NSButton::buttonWithTitle_target_action(
            ns_string!("浅色预览"),
            Some(&*target),
            Some(sel!(togglePreviewTheme:)),
            mtm,
        );
        theme_button.setFrame(NSRect::new(
            NSPoint::new(8.0, 6.0),
            NSSize::new(120.0, 32.0),
        ));
        // Avoid a nested bezel/glass surface; AppKit still handles button
        // tracking, keyboard actions, focus and accessibility labels.
        theme_button.setBordered(false);
        content.addSubview(&theme_button);

        let close_button = NSButton::buttonWithTitle_target_action(
            ns_string!("关闭"),
            Some(&*target),
            Some(sel!(closeProbe:)),
            mtm,
        );
        close_button.setFrame(NSRect::new(
            NSPoint::new(134.0, 6.0),
            NSSize::new(94.0, 32.0),
        ));
        close_button.setBordered(false);
        content.addSubview(&close_button);

        // One parent glass material. Child buttons are native controls
        // rather than additional nested glass effects.
        let view: Retained<NSView> = match material {
            GlassMaterial::Native => {
                let glass =
                    NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), local_rect);
                glass.setCornerRadius(22.0);
                glass.setContentView(Some(&content));
                glass.into_super()
            }
            GlassMaterial::Vibrancy => {
                let visual =
                    NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), local_rect);
                visual.setMaterial(NSVisualEffectMaterial::Popover);
                visual.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
                visual.setState(NSVisualEffectState::FollowsWindowActiveState);
                visual.addSubview(&content);
                visual.into_super()
            }
            GlassMaterial::Opaque => {
                let solid = NSBox::initWithFrame(NSBox::alloc(mtm), local_rect);
                solid.setBoxType(NSBoxType::Custom);
                solid.setTitle(ns_string!(""));
                solid.setContentViewMargins(NSSize::new(0.0, 0.0));
                solid.setBorderWidth(0.0);
                solid.setCornerRadius(22.0);
                solid.setFillColor(&NSColor::windowBackgroundColor());
                solid.setContentView(Some(&content));
                solid.into_super()
            }
        };
        host.addSubview(&view);
        let toolbar = Self {
            view,
            theme_button,
            _target: target,
            material,
        };
        toolbar.relayout(host.bounds());
        toolbar
    }

    pub(crate) fn take_action(&self) -> Option<GlassToolbarAction> {
        self._target.take_action()
    }

    pub(crate) fn set_preview_theme(&self, theme: PreviewTheme) {
        match theme {
            PreviewTheme::Dark => self.theme_button.setTitle(ns_string!("浅色预览")),
            PreviewTheme::Light => self.theme_button.setTitle(ns_string!("深色预览")),
        }
    }

    pub(crate) fn relayout(&self, bounds: NSRect) {
        match FloatingBarFrame::for_content_size(bounds.size.width, bounds.size.height) {
            Some(frame) => {
                self.view.setFrame(NSRect::new(
                    NSPoint::new(bounds.origin.x + frame.x, bounds.origin.y + frame.y),
                    NSSize::new(frame.width, frame.height),
                ));
                self.view.setHidden(false);
            }
            None => self.view.setHidden(true),
        }
    }

    /// Prevent new egui pointer presses underneath the native capsule.
    /// Mouse-up remains forwarded so a drag initiated on the canvas cannot
    /// get stuck when it ends over the native toolbar.
    pub(crate) fn contains_pointer(&self, event: &NSEvent, host: &NSView) -> bool {
        if self.view.isHidden() {
            return false;
        }
        let bounds = host.bounds();
        let Some(frame) = FloatingBarFrame::for_content_size(bounds.size.width, bounds.size.height)
        else {
            return false;
        };
        let point = host.convertPoint_fromView(event.locationInWindow(), None);
        let local_x = point.x - bounds.origin.x;
        let local_y = point.y - bounds.origin.y;
        let bottom_y = if host.isFlipped() {
            bounds.size.height - local_y
        } else {
            local_y
        };
        frame.contains(local_x, bottom_y)
    }

    pub(crate) fn detach(&self) {
        self.view.removeFromSuperview();
    }
}
