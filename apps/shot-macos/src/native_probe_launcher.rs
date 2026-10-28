//! Explicit, opt-in launcher for the separate AppKit NSPanel experiment.
//!
//! The production eframe event loop must never host this companion's native
//! NSApplication event loop or own its Metal surface. This process boundary
//! leaves capture, pins, and PNG output on the established backend.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use shot_core::native_snapshot::NativeSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeLaunchResult {
    Started,
    AlreadyRunning,
}

/// Never resolve from PATH, the working directory or user input.
/// In dev builds run `cargo build --features native-panel-probe --bins`
/// so the two package binaries reside in the same target directory.
pub(crate) fn sibling_probe_path(executable: &Path) -> Result<PathBuf, String> {
    if !executable.is_absolute() || executable.file_name().is_none() {
        return Err(String::from("主程序路径必须是绝对的可执行文件路径"));
    }
    let parent = executable.parent().ok_or("无法定位主程序所在目录")?;
    Ok(parent.join("ns-panel-probe"))
}

/// Reject missing, symlinked, directory or nonexecutable files, instead of
/// silently falling back to an arbitrary helper outside the app directory.
pub(crate) fn validate_probe_path(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("未找到原生实验程序 {}：{error}", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "原生实验程序不是普通文件（不允许符号链接）：{}",
            path.display()
        ));
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(format!("原生实验程序没有执行权限：{}", path.display()));
    }
    Ok(())
}

/// Retain the child so multiple button clicks cannot launch competing
/// AppKit panels. On parent shutdown, close the experimental process.
#[derive(Default)]
pub(crate) struct NativeProbeLauncher {
    child: Option<Child>,
}

impl NativeProbeLauncher {
    pub(crate) fn launch(&mut self) -> Result<ProbeLaunchResult, String> {
        let executable =
            std::env::current_exe().map_err(|error| format!("无法定位 PicPaste：{error}"))?;
        self.launch_from(&executable)
    }

    pub(crate) fn launch_from(&mut self, executable: &Path) -> Result<ProbeLaunchResult, String> {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(None) => return Ok(ProbeLaunchResult::AlreadyRunning),
                Ok(Some(_)) => {
                    self.child = None;
                }
                Err(error) => return Err(format!("无法检查原生实验窗口进程：{error}")),
            }
        }

        let path = sibling_probe_path(executable)?;
        validate_probe_path(&path)?;
        let child = Command::new(&path)
            .arg("--show")
            .spawn()
            .map_err(|error| format!("无法启动独立原生实验窗口：{error}"))?;
        self.child = Some(child);
        // Spawn does not guarantee the Cocoa window actually appeared;
        // physical macOS GUI acceptance is a separate test gate.
        Ok(ProbeLaunchResult::Started)
    }

    /// Stream already-captured immutable RGBA bytes through an anonymous
    /// pipe. Nothing is written into a temporary file, shell or command line.
    /// A worker owns the write side so a multi-megabyte frame cannot block
    /// the egui event loop while the NSPanel creates its Metal surface.
    pub(crate) fn launch_snapshot(
        &mut self,
        snapshot: NativeSnapshot,
    ) -> Result<ProbeLaunchResult, String> {
        let executable =
            std::env::current_exe().map_err(|error| format!("无法定位 PicPaste：{error}"))?;
        self.launch_snapshot_from(&executable, snapshot)
    }

    pub(crate) fn launch_snapshot_from(
        &mut self,
        executable: &Path,
        snapshot: NativeSnapshot,
    ) -> Result<ProbeLaunchResult, String> {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(None) => return Ok(ProbeLaunchResult::AlreadyRunning),
                Ok(Some(_)) => self.child = None,
                Err(error) => return Err(format!("无法检查原生实验窗口进程：{error}")),
            }
        }
        let path = sibling_probe_path(executable)?;
        validate_probe_path(&path)?;
        let mut child = Command::new(&path)
            .arg("--snapshot")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|error| format!("无法启动原生截图预览：{error}"))?;
        let Some(mut pipe) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(String::from("无法建立原生截图预览的私有输入管道"));
        };
        thread::spawn(move || {
            if let Err(error) = snapshot.write_to(&mut pipe) {
                eprintln!("PicPaste native snapshot IPC failed: {error}");
            }
        });
        self.child = Some(child);
        // Process creation is not a claim of an actual on-screen frame;
        // native GUI and Retina validation remain device-only.
        Ok(ProbeLaunchResult::Started)
    }

    /// Stop the separate AppKit process before taking a screenshot: unlike our
    /// own windows, the companion has another PID and is not filtered by the
    /// ScreenCaptureKit self-window exclusion. Idempotent after it exits.
    pub(crate) fn stop_for_capture(&mut self) -> Result<(), String> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if let Err(error) = child.kill()
                    && child
                        .try_wait()
                        .map_err(|check| format!("无法检查实验窗口退出状态：{check}"))?
                        .is_none()
                {
                    return Err(format!("无法关闭原生实验窗口：{error}"));
                }
                child
                    .wait()
                    .map_err(|error| format!("无法等待原生实验窗口关闭：{error}"))?;
            }
            Err(error) => return Err(format!("无法检查原生实验窗口进程：{error}")),
        }
        self.child = None;
        Ok(())
    }
}

impl Drop for NativeProbeLauncher {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take()
            && matches!(child.try_wait(), Ok(None))
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
