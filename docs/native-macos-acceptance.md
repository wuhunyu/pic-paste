# macOS 原生 NSPanel 真机验收与证据采集

> 此文档用于记录**真实 Mac 上的实际操作和测量证据**。GitHub Actions 的 macOS Runner 仅证明编译、无 GUI 测试与只读环境探测，不提供真实鼠标、输入法、Spaces、TCC 或 Liquid Glass 外观验收。

## 一、构建与只读预检

在一台装有 macOS 14 或更高版本、Xcode 命令行工具和 Rust 的真实 Mac 上：

```bash
cargo build --locked -p shot-macos --features native-panel-probe --bins
target/debug/ns-panel-probe --check
target/debug/ns-panel-probe --preflight > native-preflight.json
python3 scripts/native_acceptance.py prepare \
  --preflight native-preflight.json \
  --output native-acceptance.json
```

`--preflight` 不创建窗口、不初始化 WGPU、不截取屏幕，也**不弹出** Screen Recording 权限请求。它只读取 NSProcessInfo 系统版本、AppKit `NSScreen.frame` 的**全局逻辑点**范围和逐屏 `backingScaleFactor`、NSGlassEffectView 类可用性、NSWorkspace 的“降低透明度”偏好及 `CGPreflightScreenCaptureAccess` 当前权限状态。

预检报告的 `screens[].origin_x_pt/origin_y_pt` 使用 AppKit **左下角原点**的逻辑点坐标，**不是** ScreenCaptureKit 的左上角坐标，也不是截图 PNG 的源物理像素；不要把屏幕全局原点乘以 Retina 缩放因子。报表只包含坐标尺寸、系统版本和布尔状态，不采集屏幕内容、用户名、序列号或应用路径。若 Runner 无窗口桌面，允许 `screens=[]`，但这不代表真机验收通过。预检中的 `interactive_gui_observed` 恒为 `false`。

## 二、真正执行交互测试

运行主程序并启动当前显式的实验入口：

```bash
cargo run --locked -p shot-macos --features native-panel-probe --bin shot-macos
```

打开截图浮层后选择 **NSPanel 原生框选（实验）**，逐项执行以下真实测试。在生成的 `native-acceptance.json` 中把每一项的 `status` 从 `not_run` 修改为 `passed` 或 `failed`，并在 `evidence` 填写实测系统版本、复现步骤、PNG 样本尺寸/结果、录屏、终端日志或测量 CSV 的路径/摘要。**不要提前把所有状态批量填成 passed。**

| ID | 真机验收步骤 |
| --- | --- |
| `window_lifecycle` | 开启/关闭、最小化/恢复、尺寸变化、焦点恢复；反复截屏检查不残留旧浮层 |
| `material_accessibility` | macOS 26 系统 Liquid Glass、macOS 14–25 振动材质；明暗模式及降低透明度（切换后重新启动实验窗） |
| `retina_multiscreen` | 左侧或上方负原点外屏、1×/2×混合缩放，八个选区手柄和源像素尺寸、监视器插拔 |
| `native_markup_png` | 画笔/箭头/矩形/中文文字→确认→主编辑器撤销/重做→输出 PNG，检查裁剪尺寸、像素/alpha 与取消时零修改 |
| `chinese_ime` | 中文预编辑、候选窗口位置、提交文字、Enter/Escape、聚焦与失焦恢复 |
| `spaces_fullscreen` | Spaces、全屏应用、Stage Manager、切回前台以及再次截图不混入实验窗口 |
| `tcc_recovery` | Screen Recording 未授权/允许/撤销与重启，检查应用提示、功能恢复；只通过系统设置手动操作 |
| `clipboard_pins` | PNG 剪贴板粘贴、悬浮贴图透明度与鼠标穿透恢复、窗口释放 |
| `hotkey_behavior` | 快捷键反复触发、系统快捷键冲突和回滚、切换不同显示器 |
| `performance_budgets` | 在当前机器**实测** 30 次冷启动、100 次快捷键、空闲资源及 5 张 1080p 贴图帧耗时 |

## 三、填写与验证量化指标

`native-acceptance.json` 的 `performance` 默认所有计数为 0、毫秒/资源占用为 `null`。将**实际测量**填写为以下字段；可用现有 `scripts/bench_report.py` 对 `scenario,latency_ms` CSV 计算插值 P95，不能把 `ns-panel-probe --check` 或 `--preflight` 启动时间充当截图热键的延迟：

| 指标 | 样本与上限 |
| --- | --- |
| `cold_launch_samples` / `cold_launch_p95_ms` | 至少 30 次；P95 ≤1500 ms |
| `hotkey_samples` / `hotkey_p95_ms` | 至少 100 次；P95 ≤200 ms |
| `idle_cpu_percent` | ≤0.5% |
| `idle_rss_mib` | ≤80 MiB |
| `five_pin_frame_samples` / `five_pin_frame_p95_ms` | 至少 100 帧，在 5 张 1080p 贴图状态；P95 ≤16.7 ms |

```bash
python3 scripts/bench_report.py latency_samples.csv --output latency_summary.json
python3 scripts/native_acceptance.py verify --report native-acceptance.json
```

`verify` 会拒绝无显示器的 Runner 记录、缺失或重复用例、未执行状态、空证据、非有限数字、样本不足和超标指标。通过仅表示**这一台机器**的报告符合完整填写规则，不表示自动执行了任何图形交互，也不代表 macOS 14 与 26 跨机型发行条件都满足。每个系统版本和显示器配置需要独立报告；软件发布、公证、50 个长截图场景和 400 张 OCR 样本属于其他 Issue 的独立验收门槛。

## 四、CI 能与不能证明什么

GitHub macOS runner 执行 `ns-panel-probe --preflight`，并上传只读 JSON 和全部 `not_run` 的模板作为 artifact；Linux 运行纯 geometry/material 测试，macOS 执行原生无 GUI 测试和完整 Release/打包。即使这些全部成功，Issue #5/#10 的真实 Mac GUI、TCC、Retina、IME 和性能打勾**仍须用户在真机上完成**。当前默认截图、PNG、OCR、贴图与长截图继续走原有 eframe/winit 后端，实验 NSPanel 不自动取代生产实现。
