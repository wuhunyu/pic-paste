use shot_core::RgbaFrame;
use shot_ocr_protocol::{
    MAX_RGBA_BYTES, OcrDocument, OcrRequest, OcrResponse, QrDocument, read_response,
};
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
    match std::fs::symlink_metadata(&helper) {
        Ok(metadata) => {
            // An invalid Full helper is never replaced with untrusted external
            // code: fail closed until the app installation is repaired.
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(String::from("Full OCR helper 路径无效或为符号链接，拒绝执行"));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Only Lite (no embedded helper) can use the independent Add-on.
            return crate::ocr_addon::external_helper(&executable);
        }
        Err(error) => {
            return Err(format!("无法读取 Full OCR helper：{error}"));
        }
    }
    let status = Command::new("/usr/bin/codesign")
        .arg("--verify")
        .arg("--strict")
        .arg(&helper)
        .status()
        .map_err(|e| format!("无法校验本地 OCR helper：{e}"))?;
    if !status.success() {
        return Err(String::from("Full OCR helper 签名无效，拒绝执行"));
    }
    Ok(helper)
}

/// A signed, one-shot local helper executes only after a user request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    AccurateWords,
    FastCharacters,
    Qr,
}

fn call_helper(image: RgbaFrame, mode: Mode) -> Result<OcrResponse, String> {
    if image.rgba.len() > MAX_RGBA_BYTES {
        return Err(String::from("OCR 图片超过 80MiB 上限，请缩小选区"));
    }
    let helper = bundled_helper()?;
    let mut command = Command::new(helper);
    match mode {
        Mode::AccurateWords => {}
        Mode::FastCharacters => {
            command.arg("--characters");
        }
        Mode::Qr => {
            command.arg("--qr");
        }
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Vision helper 启动失败：{e}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Vision helper stdin unavailable")?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or("Vision helper stdout unavailable")?;
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
            .map_err(|error| format!("Vision 通信失败：{error}"));
        let _ = tx.send(result);
    });
    let outcome = rx.recv_timeout(Duration::from_secs(20));
    if outcome.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let response = match outcome {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => return Err(error),
        Err(_) => return Err(String::from("Vision 识别超时（20 秒），已终止 helper")),
    };
    let dimensions = match &response {
        OcrResponse::Document(doc) => (doc.pixel_width, doc.pixel_height),
        OcrResponse::QrDocument(doc) => (doc.pixel_width, doc.pixel_height),
        OcrResponse::Error(message) => {
            return Err(format!("本地 Vision 识别失败：{message}"));
        }
    };
    if dimensions != expected_size {
        return Err(String::from("Vision 响应图片尺寸不匹配"));
    }
    Ok(response)
}

fn recognize_with_mode(image: RgbaFrame, mode: Mode) -> Result<OcrDocument, String> {
    match call_helper(image, mode)? {
        OcrResponse::Document(doc) => Ok(doc),
        _ => Err(String::from("Vision helper 返回了错误的文字识别响应")),
    }
}

/// Explicit Accurate OCR: genuine provider word positions, no fake character boxes.
pub fn recognize(image: RgbaFrame) -> Result<OcrDocument, String> {
    recognize_with_mode(image, Mode::AccurateWords)
}

/// Explicit Fast OCR: native character positions may differ in recognition quality.
pub fn recognize_characters(image: RgbaFrame) -> Result<OcrDocument, String> {
    recognize_with_mode(image, Mode::FastCharacters)
}

/// Explicit QR-only Vision request, never open decoded URLs automatically.
pub fn recognize_qr(image: RgbaFrame) -> Result<QrDocument, String> {
    match call_helper(image, Mode::Qr)? {
        OcrResponse::QrDocument(doc) => Ok(doc),
        _ => Err(String::from("Vision helper 返回了错误的二维码响应")),
    }
}
