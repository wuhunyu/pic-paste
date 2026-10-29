# PicPaste macOS 内测安装说明

这是 GitHub Actions 构建的 **未公证测试版本**，不是正式发布版。

- **Full**：截图、标注、PNG 保存、贴图、内置离线 OCR（推荐先测）。
- **Lite**：不包含 OCR 的精简版本。
- ARM64 适用于 Apple Silicon（M1/M2/M3/M4 等）；测试包带有 ad-hoc 代码签名，**没有 Developer ID 正式签名，也没有 Apple 公证**。
- 此 DMG 不包含实验性 NSPanel Canary，截图使用默认稳定版 eframe 实现。

## 安装

1. 打开 GitHub Actions 成功运行页面，下载 `picpaste-macos-test-dmg-ARM64` 产物，解压 GitHub 外层 ZIP，得到 Full 和 Lite 两个 `.dmg`。
2. Apple Silicon Mac 推荐双击 `PicPaste-macos-arm64-Full-UNNOTARIZED-TEST.dmg`，将 **PicPaste.app 拖到 Applications（应用程序）**。Lite 与 Full 的 Bundle ID 相同，**不要同时安装两份**。
3. 在 Finder → 应用程序打开 PicPaste。如果系统提示无法验证开发者，进入 **系统设置 → 隐私与安全性**，针对刚刚被阻止的 PicPaste 点击“仍要打开”（Open Anyway），然后按系统提示确认。部分系统也可按住 Control 点击 App → “打开”。仅在确认它确实来自你自己的 GitHub Actions 构建后操作。
4. 到 **系统设置 → 隐私与安全性 → 屏幕与系统音频录制**（部分系统称“屏幕录制”），允许 PicPaste；根据提示完全退出并重新打开。
5. 测试：截图 `Ctrl+Option+1`、贴图 `Ctrl+Option+2`、恢复贴图交互 `Ctrl+Option+3`、选区与标注、圆角 PNG Alpha 和剪贴板。Full 再测试 OCR。

**请勿**用终端批量删除 `com.apple.quarantine`，不要关闭 Gatekeeper。若设备受单位 MDM 策略限制而不能手工批准，应等待正式签名/公证版。

## 记录测试反馈

保存 macOS 版本、芯片类型、CI run URL、DMG 文件名、出错操作及日志，并提交到 [PicPaste Issues](https://github.com/wuhunyu/pic-paste/issues)。无须把私人截图和 OCR 内容放到公开 Issue。

这个 DMG 仅供自愿授权测试，不代表 macOS 14/26 真机、性能、OCR 400 样本或长图 50 场景验收通过。没有创建 `v1.0.0` 正式 Release。
