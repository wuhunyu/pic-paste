use shot_core::RgbaFrame;
use shot_ocr_protocol::{MAX_RGBA_BYTES, OcrDocument, OcrRequest, OcrResponse, read_response};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn bundled_helper() -> Result<std::path::PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let contents = executable
        .parent()
        .and_then(|macos| macos.parent())
        .ok_or_else(|| String::from("PicPaste must run inside a macOS .app bundle"))?;
    let helper = contents.join("Helpers").join("PicPasteOCR");
    if !helper.is_file() {
        return Err(String::from(
            "OCR 未安装：当前是 Lite 版本。请使用包含离线 OCR 的 Full 版本",
        ));
    }
    // Restrict execution to the helper packaged within our signed application.
    // The future external Add-on flow MUST validate Developer Team ID and
    // version explicitly; this preview does not execute third-party helpers.
    let status = Command::new("/usr/bin/codesign")
        .arg("--verify")
        .arg("--strict")
        .arg(&helper)
        .status()
        .map_err(|e| format!("无法校验本地 OCR helper：{e}"))?;
    if !status.success() {
        return Err(String::from("OCR helper 签名无效，拒绝执行"));
    }
    Ok(helper)
}

/// Explicitly requested, isolated local OCR. Enforces one child process,
/// bounded anonymous pipe framing and a hard recognition timeout.
pub fn recognize(image: RgbaFrame) -> Result<OcrDocument, String> {
    if image.rgba.len() > MAX_RGBA_BYTES {
        return Err(String::from("OCR 图片超过 80MiB 上限，请缩小选区"));
    }
    let helper = bundled_helper()?;
    let mut child = Command::new(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("OCR helper 启动失败：{e}"))?;
    let mut stdin = child.stdin.take().ok_or("OCR helper stdin unavailable")?;
    let mut stdout = child.stdout.take().ok_or("OCR helper stdout unavailable")?;
    let expected_size = (image.width, image.height);
    let request = OcrRequest {
        width: image.width,
        height: image.height,
        rgba: image.rgba,
    };
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = request
            .write_to(&mut stdin)
            .and_then(|()| {
                drop(stdin);
                read_response(&mut stdout)
            })
            .map_err(|error| format!("OCR 通信失败：{error}"));
        let _ = tx.send(result);
    });

    let outcome = rx.recv_timeout(Duration::from_secs(20));
    // Terminate misbehaving or slow helpers. No OCR process survives timeout.
    if outcome.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    match outcome {
        Ok(Ok(OcrResponse::Document(document))) => {
            if (document.pixel_width, document.pixel_height) != expected_size {
                return Err(String::from("OCR 响应图片尺寸不匹配"));
            }
            Ok(document)
        }
        Ok(Ok(OcrResponse::Error(message))) => Err(format!("系统 Vision 识别失败：{message}")),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(String::from("OCR 识别超时（20 秒），已终止 helper")),
    }
}
