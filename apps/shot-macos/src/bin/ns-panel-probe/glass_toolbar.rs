//! Interactive native controls inside the opt-in NSPanel material capsule.
//! Cocoa retains the actual buttons; we separately retain their action target
//! because NSControl.target is weak. Captured pixels and production UI are
//! not changed by these proof-only actions.
use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
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
use shot_core::native_markup::NativeMarkupTool;

const BAR_WIDTH: f64 = 236.0;
const BAR_HEIGHT: f64 = 44.0;
const ANNOTATION_BAR_WIDTH: f64 = 560.0;
const ANNOTATION_BAR_HEIGHT: f64 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarMode {
    Demo,
    Annotation,
}

#[derive(Debug)]
struct GlassTargetIvars {
    pending: Cell<Option<GlassToolbarAction>>,
}

define_class!(
    // SAFETY: NSObject has no special subclassing requirements, and this
    // type has no Drop implementation. All AppKit actions stay on the
    // AppKit main thread. Method signatures match their Objective-C selectors.
    #[unsafe(super(NSObject))]
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

        #[unsafe(method(chooseSelect:))]
        fn choose_select(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ChooseTool(
                NativeMarkupTool::Select,
            )));
        }

        #[unsafe(method(chooseBrush:))]
        fn choose_brush(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ChooseTool(
                NativeMarkupTool::Brush,
            )));
        }

        #[unsafe(method(chooseArrow:))]
        fn choose_arrow(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ChooseTool(
                NativeMarkupTool::Arrow,
            )));
        }

        #[unsafe(method(chooseRectangle:))]
        fn choose_rectangle(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ChooseTool(
                NativeMarkupTool::Rectangle,
            )));
        }

        #[unsafe(method(chooseText:))]
        fn choose_text(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::ChooseTool(
                NativeMarkupTool::Text,
            )));
        }

        #[unsafe(method(undoMarkup:))]
        fn undo_markup(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::UndoMarkup));
        }

        #[unsafe(method(cancelSelection:))]
        fn cancel_selection(&self, _sender: &AnyObject) {
            self.ivars().pending.set(Some(GlassToolbarAction::CancelSelection));
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

fn make_material_view(
    mtm: MainThreadMarker,
    rect: NSRect,
    content: &NSView,
    material: GlassMaterial,
) -> Retained<NSView> {
    match material {
        GlassMaterial::Native => {
            let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), rect);
            glass.setCornerRadius(rect.size.height / 2.0);
            glass.setContentView(Some(content));
            glass.into_super()
        }
        GlassMaterial::Vibrancy => {
            let visual = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), rect);
            visual.setMaterial(NSVisualEffectMaterial::Popover);
            visual.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
            visual.setState(NSVisualEffectState::FollowsWindowActiveState);
            visual.addSubview(content);
            visual.into_super()
        }
        GlassMaterial::Opaque => {
            let solid = NSBox::initWithFrame(NSBox::alloc(mtm), rect);
            solid.setBoxType(NSBoxType::Custom);
            solid.setTitle(ns_string!(""));
            solid.setContentViewMargins(NSSize::new(0.0, 0.0));
            solid.setBorderWidth(0.0);
            solid.setCornerRadius(rect.size.height / 2.0);
            solid.setFillColor(&NSColor::windowBackgroundColor());
            solid.setContentView(Some(content));
            solid.into_super()
        }
    }
}

/// Retained AppKit subview. Its host NSView (which owns the Metal drawable)
/// remains unchanged and must outlive the WGPU Surface during shutdown.
pub(crate) struct ProbeGlassToolbar {
    view: Retained<NSView>,
    theme_button: Option<Retained<NSButton>>,
    markup_buttons: Vec<(NativeMarkupTool, Retained<NSButton>)>,
    mode: ToolbarMode,
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
        // SAFETY: GlassActionTarget implements togglePreviewTheme: and remains retained.
        let theme_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                ns_string!("浅色预览"),
                Some(&*target),
                Some(sel!(togglePreviewTheme:)),
                mtm,
            )
        };
        theme_button.setFrame(NSRect::new(
            NSPoint::new(8.0, 6.0),
            NSSize::new(120.0, 32.0),
        ));
        // Avoid a nested bezel/glass surface; AppKit still handles button
        // tracking, keyboard actions, focus and accessibility labels.
        theme_button.setBordered(false);
        content.addSubview(&theme_button);

        // SAFETY: GlassActionTarget implements closeProbe: and remains retained.
        let close_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                ns_string!("关闭"),
                Some(&*target),
                Some(sel!(closeProbe:)),
                mtm,
            )
        };
        close_button.setFrame(NSRect::new(
            NSPoint::new(134.0, 6.0),
            NSSize::new(94.0, 32.0),
        ));
        close_button.setBordered(false);
        content.addSubview(&close_button);

        // Native system material wraps one retained content view; no nested
        // simulated glass and no modifications to captured RGBA pixels.
        let view = make_material_view(mtm, local_rect, &content, material);
        host.addSubview(&view);
        let toolbar = Self {
            view,
            theme_button: Some(theme_button),
            markup_buttons: Vec::new(),
            mode: ToolbarMode::Demo,
            _target: target,
            material,
        };
        toolbar.relayout(host.bounds());
        toolbar
    }

    /// Opt-in selection overlay: real AppKit buttons inside the same
    /// NSGlassEffectView/NSVisualEffectView/opaque material policy.
    pub(crate) fn new_selection(mtm: MainThreadMarker, host: &NSView) -> Self {
        let material = current_material();
        let rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(ANNOTATION_BAR_WIDTH, ANNOTATION_BAR_HEIGHT),
        );
        let content = NSView::initWithFrame(NSView::alloc(mtm), rect);
        let target = GlassActionTarget::new(mtm);
        let buttons = [
            (
                ns_string!("选区"),
                sel!(chooseSelect:),
                Some(NativeMarkupTool::Select),
            ),
            (
                ns_string!("画笔"),
                sel!(chooseBrush:),
                Some(NativeMarkupTool::Brush),
            ),
            (
                ns_string!("箭头"),
                sel!(chooseArrow:),
                Some(NativeMarkupTool::Arrow),
            ),
            (
                ns_string!("矩形"),
                sel!(chooseRectangle:),
                Some(NativeMarkupTool::Rectangle),
            ),
            (
                ns_string!("文字"),
                sel!(chooseText:),
                Some(NativeMarkupTool::Text),
            ),
            (ns_string!("撤销"), sel!(undoMarkup:), None),
            (ns_string!("取消"), sel!(cancelSelection:), None),
        ];
        let mut markup_buttons = Vec::new();
        for (index, (title, selector, tool)) in buttons.into_iter().enumerate() {
            // SAFETY: every selector above is a concrete method registered on
            // GlassActionTarget; the retained target outlives all buttons.
            let button = unsafe {
                NSButton::buttonWithTitle_target_action(title, Some(&*target), Some(selector), mtm)
            };
            button.setFrame(NSRect::new(
                NSPoint::new(8.0 + index as f64 * 78.0, 6.0),
                NSSize::new(76.0, 36.0),
            ));
            button.setBordered(false);
            content.addSubview(&button);
            if let Some(tool) = tool {
                markup_buttons.push((tool, button));
            }
        }
        let view = make_material_view(mtm, rect, &content, material);
        host.addSubview(&view);
        let toolbar = Self {
            view,
            theme_button: None,
            markup_buttons,
            mode: ToolbarMode::Annotation,
            _target: target,
            material,
        };
        toolbar.set_markup_tool(NativeMarkupTool::Select);
        toolbar.relayout(host.bounds());
        toolbar
    }

    pub(crate) fn set_markup_tool(&self, selected: NativeMarkupTool) {
        for (tool, button) in &self.markup_buttons {
            let title = match (*tool, *tool == selected) {
                (NativeMarkupTool::Select, true) => ns_string!("●选区"),
                (NativeMarkupTool::Select, false) => ns_string!("选区"),
                (NativeMarkupTool::Brush, true) => ns_string!("●画笔"),
                (NativeMarkupTool::Brush, false) => ns_string!("画笔"),
                (NativeMarkupTool::Arrow, true) => ns_string!("●箭头"),
                (NativeMarkupTool::Arrow, false) => ns_string!("箭头"),
                (NativeMarkupTool::Rectangle, true) => ns_string!("●矩形"),
                (NativeMarkupTool::Rectangle, false) => ns_string!("矩形"),
                (NativeMarkupTool::Text, true) => ns_string!("●文字"),
                (NativeMarkupTool::Text, false) => ns_string!("文字"),
            };
            button.setTitle(title);
        }
    }

    pub(crate) fn take_action(&self) -> Option<GlassToolbarAction> {
        self._target.take_action()
    }

    pub(crate) fn set_preview_theme(&self, theme: PreviewTheme) {
        if let Some(button) = self.theme_button.as_ref() {
            match theme {
                PreviewTheme::Dark => button.setTitle(ns_string!("浅色预览")),
                PreviewTheme::Light => button.setTitle(ns_string!("深色预览")),
            }
        }
    }

    pub(crate) fn relayout(&self, bounds: NSRect) {
        match self.frame_for_bounds(bounds) {
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

    fn frame_for_bounds(&self, bounds: NSRect) -> Option<FloatingBarFrame> {
        match self.mode {
            ToolbarMode::Demo => {
                FloatingBarFrame::for_content_size(bounds.size.width, bounds.size.height)
            }
            ToolbarMode::Annotation => {
                FloatingBarFrame::for_annotation_size(bounds.size.width, bounds.size.height)
            }
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
        let Some(frame) = self.frame_for_bounds(bounds) else {
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
