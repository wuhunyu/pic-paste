//! Explicit, opt-in launcher for the separate AppKit NSPanel experiment.
//!
//! The production eframe event loop must never host this companion's native
//! NSApplication event loop or own its Metal surface. This process boundary
//! leaves capture, pins, and PNG output on the established backend.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

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

    pub(crate) fn launch_from(
        &mut self,
        executable: &Path,
    ) -> Result<ProbeLaunchResult, String> {
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
}

impl Drop for NativeProbeLauncher {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
