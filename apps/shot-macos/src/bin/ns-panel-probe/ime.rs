//! Native text-input context for the opt-in NSPanel/egui probe only.
//!
//! Unlike a raw NSEvent.characters() bridge, NSTextInputContext invokes an
//! NSTextInputClient with marked-text updates and final commits. The client
//! queues pure changes; egui is only touched from the outer event loop.
//! The document itself belongs to egui, NOT this client.
use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSTextInputClient, NSTextInputContext, NSView};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSObject, NSObjectProtocol, NSPoint,
    NSRange, NSRangePointer, NSRect, NSSize, NSString,
};
use shot_core::ime::{ImeChange, ImeComposition};

#[derive(Debug)]
struct NativeImeIvars {
    composition: RefCell<ImeComposition>,
    queue: RefCell<Vec<ImeChange>>,
    // Keeps the view used for cursor-coordinate conversion alive. The
    // bridge must be destroyed before the NSPanel and Metal surface.
    host: Retained<NSView>,
    cursor: Cell<Option<egui::Rect>>,
}

define_class!(
    // SAFETY: NSObject supports subclassing. Method signatures match the
    // required NSTextInputClient selectors. All methods run on the main
    // thread with a retained target, and no callback mutates egui or WGPU.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = NativeImeIvars]
    #[derive(Debug)]
    struct NativeImeClient;

    unsafe impl NSObjectProtocol for NativeImeClient {}
    unsafe impl NSTextInputClient for NativeImeClient {}

    impl NativeImeClient {
        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, value: &AnyObject, _replacement: NSRange) {
            if let Some(text) = Self::native_text(value) {
                let change = self.ivars().composition.borrow_mut().commit(text);
                self.ivars().queue.borrow_mut().push(change);
            }
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(&self, value: &AnyObject, selected: NSRange, _replacement: NSRange) {
            if let Some(text) = Self::native_text(value) {
                let change = self.ivars().composition.borrow_mut().preedit(
                    text,
                    selected.location,
                    selected.length,
                );
                self.ivars().queue.borrow_mut().push(change);
            }
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            if let Some(change) = self.ivars().composition.borrow_mut().cancel() {
                self.ivars().queue.borrow_mut().push(change);
            }
        }

        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            self.ivars().composition.borrow().has_marked_text()
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            let state = self.ivars().composition.borrow();
            if state.has_marked_text() {
                NSRange::new(0, state.marked_utf16_len())
            } else {
                NSRange::new(usize::MAX, 0)
            }
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            let state = self.ivars().composition.borrow();
            let (location, length) = state.selected_range_utf16().unwrap_or((0, 0));
            NSRange::new(location, length)
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring(&self, _range: NSRange, actual: NSRangePointer) -> Option<Retained<NSAttributedString>> {
            // We do not own egui's surrounding document. Never hand the
            // system a fake substring that could corrupt a replacement.
            if !actual.is_null() {
                // SAFETY: NSTextInputClient requires an optional, writable
                // NSRangePointer; AppKit guarantees validity when non-null.
                unsafe { *actual = NSRange::new(usize::MAX, 0) };
            }
            None
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes(&self) -> Retained<NSArray<NSAttributedStringKey>> {
            NSArray::new()
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect(&self, range: NSRange, actual: NSRangePointer) -> NSRect {
            if !actual.is_null() {
                // SAFETY: NSTextInputClient guarantees an optional writable
                // pointer. We use the current marked range, if any.
                unsafe { *actual = range };
            }
            let host = &self.ivars().host;
            let bounds = host.bounds();
            let local = if let Some(cursor) = self.ivars().cursor.get() {
                let x = bounds.origin.x + f64::from(cursor.left());
                let y = if host.isFlipped() {
                    bounds.origin.y + f64::from(cursor.top())
                } else {
                    bounds.origin.y + bounds.size.height - f64::from(cursor.bottom())
                };
                NSRect::new(
                    NSPoint::new(x, y),
                    NSSize::new(f64::from(cursor.width().max(1.0)), f64::from(cursor.height().max(14.0))),
                )
            } else {
                NSRect::new(bounds.origin, NSSize::new(1.0, 14.0))
            };
            let window_rect = host.convertRect_toView(local, None);
            if let Some(window) = host.window() {
                window.convertRectToScreen(window_rect)
            } else {
                window_rect
            }
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index(&self, _point: NSPoint) -> usize {
            self.ivars()
                .composition
                .borrow()
                .selected_range_utf16()
                .map_or(0, |(start, _)| start)
        }

        #[unsafe(method(doCommandBySelector:))]
        fn do_command(&self, _selector: Sel) {
            // Non-text keys are forwarded exactly once to egui by the
            // event loop. Do not synthesize duplicate Backspace/Enter here.
        }
    }
);

impl NativeImeClient {
    fn new(mtm: MainThreadMarker, host: &NSView) -> Retained<Self> {
        let obj = Self::alloc(mtm);
        let obj = obj.set_ivars(NativeImeIvars {
            composition: RefCell::new(ImeComposition::default()),
            queue: RefCell::new(Vec::new()),
            host: host.retain(),
            cursor: Cell::new(None),
        });
        // SAFETY: NSObject -init gives an initialized ObjC object; all
        // NativeImeIvars have already been initialized above.
        unsafe { msg_send![super(obj), init] }
    }

    fn native_text(value: &AnyObject) -> Option<String> {
        if let Some(text) = value.downcast_ref::<NSString>() {
            Some(text.to_string())
        } else {
            value
                .downcast_ref::<NSAttributedString>()
                .map(|value| value.string().to_string())
        }
    }

    fn drain(&self) -> Vec<ImeChange> {
        std::mem::take(&mut *self.ivars().queue.borrow_mut())
    }

    fn cancel(&self) {
        if let Some(change) = self.ivars().composition.borrow_mut().cancel() {
            self.ivars().queue.borrow_mut().push(change);
        }
    }
}

/// A self-contained Cocoa input context. Must remain on the AppKit thread.
pub(crate) struct ProbeImeBridge {
    // Field order ensures the input context is released before its retained client.
    context: Retained<NSTextInputContext>,
    client: Retained<NativeImeClient>,
    active: bool,
}

impl ProbeImeBridge {
    pub(crate) fn new(mtm: MainThreadMarker, host: &NSView) -> Self {
        let client = NativeImeClient::new(mtm, host);
        let context = NSTextInputContext::initWithClient(
            NSTextInputContext::alloc(mtm),
            ProtocolObject::from_ref(&*client),
        );
        Self {
            context,
            client,
            active: false,
        }
    }

    pub(crate) fn set_active(&mut self, active: bool) {
        if self.active == active {
            return;
        }
        if active {
            self.context.activate();
        } else {
            self.context.discardMarkedText();
            self.client.cancel();
            self.context.deactivate();
        }
        self.active = active;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn handle_key(&self, event: &NSEvent) -> bool {
        self.active && self.context.handleEvent(event)
    }

    pub(crate) fn set_cursor(&self, rect: Option<egui::Rect>) {
        if self.client.ivars().cursor.replace(rect) != rect {
            self.context.invalidateCharacterCoordinates();
        }
    }

    pub(crate) fn drain(&self) -> Vec<ImeChange> {
        self.client.drain()
    }
}

impl Drop for ProbeImeBridge {
    fn drop(&mut self) {
        self.set_active(false);
    }
}
