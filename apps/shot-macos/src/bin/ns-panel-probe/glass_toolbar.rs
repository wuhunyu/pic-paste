//! Optional AppKit floating capsule above the WGPU-backed NSPanel.
//!
//! This is a native material proof, NOT the production screenshot toolbar.
//! The label lives *inside* NSGlassEffectView.contentView (WWDC25 guidance).
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBox, NSBoxType, NSColor, NSEvent, NSGlassEffectView, NSTextField, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSProcessInfo, NSRect, NSSize, ns_string};
use shot_core::glass_toolbar::{FloatingBarFrame, GlassMaterial, select_glass_material};

const BAR_WIDTH: f64 = 236.0;
const BAR_HEIGHT: f64 = 44.0;

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
    pub(crate) material: GlassMaterial,
}

impl ProbeGlassToolbar {
    pub(crate) fn new(mtm: MainThreadMarker, host: &NSView) -> Self {
        let material = current_material();
        let local_rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(BAR_WIDTH, BAR_HEIGHT),
        );
        let content = NSView::initWithFrame(NSView::alloc(mtm), local_rect);
        let text = match material {
            GlassMaterial::Native => ns_string!("PicPaste · Liquid Glass"),
            GlassMaterial::Vibrancy => ns_string!("PicPaste · Vibrancy"),
            GlassMaterial::Opaque => ns_string!("PicPaste · 高对比"),
        };
        let label = NSTextField::labelWithString(text, mtm);
        label.setFrame(NSRect::new(
            NSPoint::new(14.0, 12.0),
            NSSize::new(BAR_WIDTH - 28.0, 20.0),
        ));
        label.setTextColor(Some(&NSColor::labelColor()));
        content.addSubview(&label);

        // Only the single status capsule is glass. A second glass element
        // would need an NSGlassEffectContainerView to combine sampling.
        let view: Retained<NSView> = match material {
            GlassMaterial::Native => {
                let glass = NSGlassEffectView::initWithFrame(
                    NSGlassEffectView::alloc(mtm),
                    local_rect,
                );
                glass.setCornerRadius(22.0);
                glass.setContentView(Some(&content));
                glass.into_super()
            }
            GlassMaterial::Vibrancy => {
                let visual = NSVisualEffectView::initWithFrame(
                    NSVisualEffectView::alloc(mtm),
                    local_rect,
                );
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
        let toolbar = Self { view, material };
        toolbar.relayout(host.bounds());
        toolbar
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
    /// Mouse-up is always forwarded, including when a drag ends over it.
    pub(crate) fn contains_pointer(&self, event: &NSEvent, host: &NSView) -> bool {
        if self.view.isHidden() {
            return false;
        }
        let bounds = host.bounds();
        let Some(frame) =
            FloatingBarFrame::for_content_size(bounds.size.width, bounds.size.height)
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
