use eframe::egui;
use std::sync::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Capture,
    Paste,
    Recover,
    Show,
    Quit,
}

pub struct MenuBar {
    _tray: TrayIcon,
}

fn pin_icon() -> Vec<u8> {
    let mut pixels = vec![0; 24 * 24 * 4];
    for y in 0usize..24 {
        for x in 0usize..24 {
            // Compact, high-contrast mint/cobalt pin icon for dark/light menu bars.
            let pin_head = (8..=16).contains(&x)
                && (4..=12).contains(&y)
                && ((x as i32 - 12).pow(2) + (y as i32 - 8).pow(2) <= 24);
            let stem = (11..=13).contains(&x) && (11..=20).contains(&y);
            let outline = (5..=19).contains(&x)
                && (2..=21).contains(&y)
                && (x == 5 || x == 19 || y == 2 || y == 21);
            let index = (y * 24 + x) * 4;
            if pin_head || stem {
                pixels[index..index + 4].copy_from_slice(&[30, 192, 223, 255]);
            } else if outline {
                pixels[index..index + 4].copy_from_slice(&[37, 91, 218, 230]);
            }
        }
    }
    pixels
}

impl MenuBar {
    pub fn install(ctx: egui::Context, queue: Sender<MenuAction>) -> Result<Self, String> {
        let menu = Menu::new();
        let capture = MenuItem::new("截图 · Ctrl+Option+1", true, None);
        let pin = MenuItem::new("贴图 · Ctrl+Option+2", true, None);
        let recover = MenuItem::new("恢复鼠标交互 · Ctrl+Option+3", true, None);
        let show = MenuItem::new("显示 PicPaste", true, None);
        let quit = MenuItem::new("退出 PicPaste", true, None);
        for item in [&capture, &pin, &recover, &show, &quit] {
            menu.append(item).map_err(|e| e.to_string())?;
        }
        let icon = Icon::from_rgba(pin_icon(), 24, 24).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("PicPaste 截图与贴图")
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())?;

        let capture_id = capture.id().clone();
        let pin_id = pin.id().clone();
        let recover_id = recover.id().clone();
        let show_id = show.id().clone();
        let quit_id = quit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == capture_id {
                Some(MenuAction::Capture)
            } else if event.id == pin_id {
                Some(MenuAction::Paste)
            } else if event.id == recover_id {
                Some(MenuAction::Recover)
            } else if event.id == show_id {
                Some(MenuAction::Show)
            } else if event.id == quit_id {
                Some(MenuAction::Quit)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = queue.send(action);
                ctx.request_repaint();
            }
        }));
        Ok(Self { _tray: tray })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_pin_has_rgba_pixels() {
        let icon = pin_icon();
        assert_eq!(icon.len(), 24 * 24 * 4);
        assert_eq!(icon[3], 0);
        assert!(icon.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 255));
    }
}
