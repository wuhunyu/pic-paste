use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::sync::mpsc::Sender;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Capture,
    PastePin,
    RecoverPins,
}

pub struct HotkeyController {
    _manager: GlobalHotKeyManager,
}

impl HotkeyController {
    /// Install hotkeys on the UI/main thread, before entering the eframe event loop.
    pub fn install(ctx: egui::Context, queue: Sender<HotkeyAction>) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| format!("Cannot start global hotkeys: {error}"))?;
        let capture = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit1);
        let pin = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit2);
        manager
            .register(capture)
            .map_err(|error| format!("Screenshot hotkey unavailable: {error}"))?;
        manager
            .register(pin)
            .map_err(|error| format!("Pin hotkey unavailable: {error}"))?;
        let recover = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit3);
        manager
            .register(recover)
            .map_err(|error| format!("Recovery hotkey unavailable: {error}"))?;
        let capture_id = capture.id();
        let pin_id = pin.id();
        let recover_id = recover.id();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            let action = if event.id == capture_id {
                Some(HotkeyAction::Capture)
            } else if event.id == pin_id {
                Some(HotkeyAction::PastePin)
            } else if event.id == recover_id {
                Some(HotkeyAction::RecoverPins)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = queue.send(action);
                ctx.request_repaint();
            }
        }));
        Ok(Self { _manager: manager })
    }
}
