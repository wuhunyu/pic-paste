//! Verify a separately installed local OCR helper before any execution.
//! A standalone Add-on requires the same *non-empty* Apple Developer Team ID
//! as the host app. An ad-hoc development app cannot trust standalone code.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CAPABILITY: &[u8] = b"PICPASTE-OCR-ADDON/1 SHOC/1\n";

fn team_identifier(report: &str) -> Option<&str> {
    let team = report
        .lines()
        .find_map(|line| line.strip_prefix("TeamIdentifier="))?;
    // Apple issues a ten-character team identifier. An ad-hoc signature
    // prints "not set"; never treat two absent IDs as a matching identity.
    (team.len() == 10
        && team
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()))
    .then_some(team)
}

fn matching_team_ids(app_info: &str, helper_info: &str) -> bool {
    matches!(
        (team_identifier(app_info), team_identifier(helper_info)),
        (Some(app), Some(helper)) if app == helper
    )
}

fn compatible(reply: &[u8]) -> bool {
    reply == CAPABILITY
}

fn verify_signature(path: &Path, deep: bool) -> Result<(), String> {
    let mut command = Command::new("/usr/bin/codesign");
    command.arg("--verify").arg("--strict");
    if deep {
        command.arg("--deep");
    }
    let status = command
        .arg(path)
        .status()
        .map_err(|error| format!("无法验证 OCR Add-on 的签名：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(String::from("OCR Add-on 签名无效或内容已被更改"))
    }
}

fn signature_details(path: &Path) -> Result<String, String> {
    let output = Command::new("/usr/bin/codesign")
        .arg("--display")
        .arg("--verbose=4")
        .arg(path)
        .output()
        .map_err(|error| format!("无法读取 Apple 签名身份：{error}"))?;
    if !output.status.success() {
        return Err(String::from("无法读取 OCR Add-on 的签名身份"));
    }
    // codesign writes signing details to stderr, not stdout.
    let mut details = String::from_utf8_lossy(&output.stderr).into_owned();
    details.push_str(&String::from_utf8_lossy(&output.stdout));
    Ok(details)
}

fn check_capability(helper: &Path) -> Result<(), String> {
    let mut child = Command::new(helper)
        .arg("--addon-info")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("无法查询 OCR Add-on 版本：{error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("OCR Add-on 无法提供版本信息"))?;
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(128).read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    let bytes = match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(error)) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("OCR Add-on 版本信息读取失败：{error}"));
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(String::from("OCR Add-on 版本查询超时，已终止进程"));
        }
    };
    let status = child
        .wait()
        .map_err(|error| format!("OCR Add-on 版本查询进程异常：{error}"))?;
    if !status.success() || !compatible(&bytes) {
        return Err(String::from(
            "OCR Add-on 不兼容：需要 Add-on API 1 与 SHOC 1",
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("无法读取 OCR Add-on 目录：{error}"))?;
    if metadata.file_type().is_symlink() {
        return Err(String::from("OCR Add-on 路径包含不可信的符号链接"));
    }
    Ok(())
}

fn add_on_location(home: &Path) -> Result<PathBuf, String> {
    if !home.is_absolute() {
        return Err(String::from("OCR Add-on HOME 路径必须为绝对路径"));
    }
    let locations = [
        home.to_path_buf(),
        home.join("Library"),
        home.join("Library/Application Support"),
        home.join("Library/Application Support/PicPaste"),
        home.join("Library/Application Support/PicPaste/AddOns"),
        home.join("Library/Application Support/PicPaste/AddOns/OCR"),
    ];
    for directory in locations {
        reject_symlink(&directory)?;
        if !directory.is_dir() {
            return Err(String::from("OCR Add-on 安装目录不存在或类型不正确"));
        }
    }
    Ok(home.join("Library/Application Support/PicPaste/AddOns/OCR/PicPasteOCR"))
}

/// Resolve only the fixed per-user installation. No environment overrides or
/// PATH search are ever used to find or verify executable OCR code.
pub(crate) fn external_helper(app_executable: &Path) -> Result<PathBuf, String> {
    let contents = app_executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| String::from("PicPaste 必须从 macOS .app 中运行"))?;
    let app = contents
        .parent()
        .filter(|path| path.extension() == Some(OsStr::new("app")))
        .ok_or_else(|| String::from("PicPaste .app 路径无效"))?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| String::from("无法确定用户主目录"))?;
    let helper = add_on_location(&home)?;
    reject_symlink(&helper)?;
    if !helper.is_file() {
        return Err(String::from("OCR Add-on 不是可执行文件"));
    }

    // Validate both sides before running even the informational handshake.
    verify_signature(app, true)?;
    verify_signature(&helper, false)?;
    let app_info = signature_details(app)?;
    let helper_info = signature_details(&helper)?;
    if !matching_team_ids(&app_info, &helper_info) {
        return Err(String::from(
            "OCR Add-on 与 PicPaste 的 Apple Team ID 不一致，或使用了不可信的 ad-hoc 签名",
        ));
    }
    check_capability(&helper)?;
    Ok(helper)
}

#[cfg(test)]
mod tests {
    use super::{CAPABILITY, compatible, matching_team_ids, team_identifier};

    #[test]
    fn rejects_adhoc_and_missing_team_even_when_both_are_adhoc() {
        assert_eq!(team_identifier("TeamIdentifier=not set\n"), None);
        assert_eq!(team_identifier("Identifier=PicPaste\n"), None);
        assert!(!matching_team_ids(
            "TeamIdentifier=not set\n",
            "TeamIdentifier=not set\n",
        ));
    }

    #[test]
    fn only_the_same_nonempty_developer_team_is_accepted() {
        let app = "Identifier=com.picpaste.app\nTeamIdentifier=A1B2C3D4E5\n";
        let valid = "TeamIdentifier=A1B2C3D4E5\n";
        let foreign = "TeamIdentifier=F6G7H8J9K0\n";
        assert!(matching_team_ids(app, valid));
        assert!(!matching_team_ids(app, foreign));
        assert_eq!(team_identifier("TeamIdentifier=A1B2C3D4E5\n"), Some("A1B2C3D4E5"));
        assert_eq!(team_identifier("TeamIdentifier=A1B2-C3D4\n"), None);
    }

    #[test]
    fn capability_rejects_unknown_api_shoc_versions_and_extra_output() {
        assert!(compatible(CAPABILITY));
        assert!(!compatible(b"PICPASTE-OCR-ADDON/2 SHOC/1\n"));
        assert!(!compatible(b"PICPASTE-OCR-ADDON/1 SHOC/2\n"));
        assert!(!compatible(b"PICPASTE-OCR-ADDON/1 SHOC/1"));
        assert!(!compatible(b"PICPASTE-OCR-ADDON/1 SHOC/1\nmalicious"));
    }
}