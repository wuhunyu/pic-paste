use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use shot_core::shortcuts::{HotkeyBindings, ShortcutAction};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Capture,
    PastePin,
    RecoverPins,
    FinishScroll,
}

pub struct HotkeyController {
    manager: GlobalHotKeyManager,
    registered: [HotKey; 4],
    ids: Arc<Mutex<[u32; 4]>>,
    bindings: HotkeyBindings,
}

fn code_for_digit(digit: u8) -> Code {
    match digit {
        1 => Code::Digit1,
        2 => Code::Digit2,
        3 => Code::Digit3,
        4 => Code::Digit4,
        5 => Code::Digit5,
        6 => Code::Digit6,
        7 => Code::Digit7,
        8 => Code::Digit8,
        9 => Code::Digit9,
        _ => unreachable!("shortcuts are validated before conversion"),
    }
}

fn keys_for(bindings: HotkeyBindings) -> [HotKey; 4] {
    ShortcutAction::ALL.map(|action| {
        HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::ALT),
            code_for_digit(bindings.digit(action)),
        )
    })
}

fn register_checked(manager: &GlobalHotKeyManager, keys: &[HotKey; 4]) -> Result<(), String> {
    let mut installed = Vec::new();
    for key in keys.iter().copied() {
        if let Err(error) = manager.register(key) {
            for previous in installed {
                let _ = manager.unregister(previous);
            }
            return Err(format!("操作系统拒绝注册组合键：{error}"));
        }
        installed.push(key);
    }
    Ok(())
}

fn settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME 环境变量不存在")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("PicPaste")
        .join("hotkeys.conf"))
}

impl HotkeyController {
    /// The macOS global-hotkey manager must be created and used on the UI thread.
    pub fn install(
        ctx: egui::Context,
        queue: Sender<HotkeyAction>,
        bindings: HotkeyBindings,
    ) -> Result<Self, String> {
        let manager =
            GlobalHotKeyManager::new().map_err(|error| format!("无法启动全局快捷键：{error}"))?;
        let keys = keys_for(bindings);
        register_checked(&manager, &keys)?;
        let ids = Arc::new(Mutex::new(keys.map(|key| key.id())));
        let dispatch_ids = Arc::clone(&ids);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            let position = dispatch_ids
                .lock()
                .ok()
                .and_then(|ids| ids.iter().position(|id| *id == event.id));
            let action = match position {
                Some(0) => Some(HotkeyAction::Capture),
                Some(1) => Some(HotkeyAction::PastePin),
                Some(2) => Some(HotkeyAction::RecoverPins),
                Some(3) => Some(HotkeyAction::FinishScroll),
                _ => None,
            };
            if let Some(action) = action {
                let _ = queue.send(action);
                ctx.request_repaint();
            }
        }));
        Ok(Self {
            manager,
            registered: keys,
            ids,
            bindings,
        })
    }

    /// OS shortcut changes are transactional: failed registrations restore
    /// the previous working set before any in-memory mapping is updated.
    pub fn rebind(&mut self, next: HotkeyBindings) -> Result<(), String> {
        if self.bindings == next {
            return Ok(());
        }
        self.manager
            .unregister_all(&self.registered)
            .map_err(|error| format!("无法释放原快捷键：{error}"))?;
        let keys = keys_for(next);
        if let Err(error) = register_checked(&self.manager, &keys) {
            if let Err(rollback) = register_checked(&self.manager, &self.registered) {
                return Err(format!(
                    "新快捷键注册失败（{error}），原快捷键恢复也失败（{rollback}），请重启程序"
                ));
            }
            return Err(format!("{error}；原快捷键已恢复"));
        }
        if let Ok(mut ids) = self.ids.lock() {
            *ids = keys.map(|key| key.id());
        } else {
            let _ = self.manager.unregister_all(&keys);
            let _ = register_checked(&self.manager, &self.registered);
            return Err(String::from("快捷键事件路由已失效；原快捷键恢复操作已尝试"));
        }
        self.registered = keys;
        self.bindings = next;
        Ok(())
    }

    pub fn load_settings() -> Result<HotkeyBindings, String> {
        let path = settings_path()?;
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(HotkeyBindings::default());
            }
            Err(error) => return Err(format!("无法读取快捷键配置：{error}")),
        };
        HotkeyBindings::parse(&contents).map_err(|error| {
            format!(
                "快捷键配置无效（{error:?}），请在设置中重新保存：{}",
                path.display()
            )
        })
    }

    pub fn save_settings(bindings: HotkeyBindings) -> Result<(), String> {
        let path = settings_path()?;
        let parent = path.parent().ok_or("配置路径不合法")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let temp = path.with_file_name(format!(".hotkeys-{}-{now}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| -> io::Result<()> {
            let mut file = options.open(&temp)?;
            file.write_all(bindings.serialize().as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &path)?;
            Ok(())
        })();
        let _ = fs::remove_file(&temp);
        result.map_err(|error| format!("无法保存快捷键设置：{error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrations_follow_configured_digits_without_conflicting_ids() {
        let config = HotkeyBindings::new([9, 8, 7, 6]).unwrap();
        let keys = keys_for(config);
        let ids = keys.map(|key| key.id());
        assert!(
            ids.iter()
                .all(|id| ids.iter().filter(|other| *other == id).count() == 1)
        );
        assert_ne!(keys[0].id(), keys_for(HotkeyBindings::default())[0].id());
    }
}
