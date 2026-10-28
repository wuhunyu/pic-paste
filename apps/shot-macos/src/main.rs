#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod hotkey;
#[cfg(target_os = "macos")]
mod menu;
#[cfg(all(target_os = "macos", feature = "native-panel-probe"))]
mod native_probe_launcher;
#[cfg(target_os = "macos")]
mod ocr;
#[cfg(target_os = "macos")]
mod ocr_addon;
#[cfg(target_os = "macos")]
mod overlay;
#[cfg(target_os = "macos")]
mod pin;
#[cfg(target_os = "macos")]
mod scroll;

#[cfg(target_os = "macos")]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("PicPaste")
            .with_inner_size([760.0, 660.0])
            .with_min_inner_size([480.0, 380.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "PicPaste",
        options,
        Box::new(|creation_context| Ok(Box::new(app::PicPasteApp::new(creation_context)))),
    )
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("PicPaste native UI currently requires macOS 14 or later");
}

#[cfg(all(test, target_os = "macos", feature = "native-panel-probe"))]
mod native_probe_launcher_tests {
    use super::native_probe_launcher::{
        NativeProbeLauncher, ProbeLaunchResult, sibling_probe_path, validate_probe_path,
    };
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("picpaste-probe-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn companion_resolves_only_next_to_an_absolute_app_executable() {
        let app = Path::new("/Applications/PicPaste.app/Contents/MacOS/PicPaste");
        assert_eq!(
            sibling_probe_path(app).unwrap(),
            Path::new("/Applications/PicPaste.app/Contents/MacOS/ns-panel-probe")
        );
        assert!(sibling_probe_path(Path::new("target/debug/shot-macos")).is_err());
        assert!(sibling_probe_path(Path::new("/")).is_err());
    }

    #[test]
    fn missing_nonexecutable_and_symlinked_companions_are_rejected() {
        let dir = unique_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ns-panel-probe");
        assert!(validate_probe_path(&path).is_err());
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(validate_probe_path(&path).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(validate_probe_path(&path).is_ok());
        fs::remove_file(&path).unwrap();
        let target = dir.join("other-executable");
        fs::write(&target, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&target, &path).unwrap();
        assert!(validate_probe_path(&path).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn capture_interlock_stops_live_probe_and_allows_relaunch() {
        let dir = unique_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ns-panel-probe");
        fs::write(&path, "#!/bin/sh\nexec /bin/sleep 15\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let app = dir.join("PicPaste");
        let mut launcher = NativeProbeLauncher::default();
        assert_eq!(
            launcher.launch_from(&app).unwrap(),
            ProbeLaunchResult::Started
        );
        launcher.stop_for_capture().unwrap();
        launcher.stop_for_capture().unwrap();
        assert_eq!(
            launcher.launch_from(&app).unwrap(),
            ProbeLaunchResult::Started
        );
        drop(launcher);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn repeated_launch_reuses_the_same_live_companion() {
        let dir = unique_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ns-panel-probe");
        fs::write(&path, "#!/bin/sh\nexec /bin/sleep 15\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let app = dir.join("PicPaste");
        let mut launcher = NativeProbeLauncher::default();
        assert_eq!(
            launcher.launch_from(&app).unwrap(),
            ProbeLaunchResult::Started
        );
        assert_eq!(
            launcher.launch_from(&app).unwrap(),
            ProbeLaunchResult::AlreadyRunning
        );
        drop(launcher);
        fs::remove_dir_all(&dir).unwrap();
    }
}
