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
| `native_markup_png` | 画笔/直线/箭头/矩形/椭圆/马赛克/模糊/中文文字→确认→主编辑器撤销/重做→输出 PNG，检查裁剪尺寸、像素/alpha 与取消时零修改 |
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

## 五、PR #49 原生直线/区域效果补充验收（不能伪造 CI 证据）

在真实 Mac 双屏混合 1×/2× 上，用原生面板分别绘制反向直线、400 万物理像素区域马赛克、带 Alpha 边缘的模糊，以及越界和超限操作。原生模糊/马赛克阶段应只出现*作用区域*标框；确认后检查现有编辑器中的准确像素效果、PNG 逐像素差异和撤销重做。重点记录 4MP 临界值、非法区域不提交、不丢弃重做历史、源 RGBA/透明角保留、不同缩放和取消不修改。此项实际操作未完成前不得将 `native_markup_png` 填为 `passed`。

## 六、PR #50 原生键盘与输入法补充验收

在真实 macOS 14 和 26（如可用），用 US/其他键盘布局测试 `⌘Z` 与 `⇧⌘Z` 在多次已提交标注后的撤销/重做，确认按住或重复键不产生额外操作；按一次/松开后再次操作正常。非选区演示中的 Escape 应退出；选区中 Enter 只能确认有效区域；Esc 取消后原图不变。重点在文字输入框内用简体中文拼音、日文输入法建立正在编辑的候选文本，确认 Escape 只取消或交给输入组件处理、Enter 只提交输入而不关闭截图窗口；同样验证 Ctrl/Option+Cmd+Z 不修改标注历史。记录实际机器、操作系统、键盘布局、输入法版本、视频及失败样本。自动 CI 的状态机单测和 no-GUI probe 不能代替这些实测。

## 七、PR #51 原生像素预览专项真机验收

在 macOS 14/26（分别记录系统版本）使用 3840×2160 及 5120×2880、1×/2× 混合双屏，完成单个 12px 马赛克块、半径 6px 模糊、不同奇数原点的 ROI、部分透明 PNG 区域处理。仅在**已提交、互不重叠、纯马赛克/模糊**且总效果≤1MP、源 RGBA≤32MiB 时要求 NSPanel 原生图像层与父编辑器 PNG 同一 ROI 逐像素匹配；拖动草稿为框选，不计算效果。对相交/混合矢量或文字/超 1MP/超 32MiB 的图像，应只显示区域框，确认后父编辑器仍可按单次 4MP 效果完成输出；测试撤销、重做后缓存失效、是否出现残留纹理、边界色差、GPU 与 CPU/RSS 峰值。请记录实际机器、提交 SHA、视频/截图及测量原始数据，CI 的字节测试不代表 GPU 显示或性能已通过。

## 八、PR #52 原生自动截图试运行（仅开发版，未完成真机验收）

构建一个**单独标记**、可运行的开发包（不会改变常规 Lite/Full ZIP）：

```bash
cargo build --release -p shot-macos --features native-panel-probe --bins --locked
PICPASTE_VARIANT=Lite PICPASTE_NATIVE_CANARY=1 bash scripts/package-macos.sh
PICPASTE_VARIANT=Full PICPASTE_NATIVE_CANARY=1 bash scripts/package-macos.sh
```

两个包的文件名包含 `NATIVE-CANARY-UNSIGNED-DEV`，应用 `Contents/MacOS` 内有同目录已 ad-hoc 签名的 `ns-panel-probe`。**非正式 Developer ID 签名或公证发行件。** 普通构建不附带 companion、也没有试运行开关。

在真实 macOS 14/26 上执行并记录截屏、录屏、机器/系统/构建 SHA 与错误日志：

1. 在试运行包设置页打开“下一次截图优先使用原生 NSPanel”；每次重启默认关闭，不会暗中改变所有用户的行为。
2. 使用快捷键和按钮分别截图；一次有效 Capture 后应进入独立 NSPanel 而不是同时弹出两个置顶编辑器；显示器全局**逻辑点**定位、选区和标注仍按原屏局部**物理像素**核验。
3. 点击原生“确认”，核对旧编辑器中的 PNG 合成、Undo/Redo、保存/复制/贴图与源图 Alpha；再次截图、改编辑历史后，旧子进程回复不能覆盖新内容。
4. 原生 Esc/取消必须**结束本次自动截图**并恢复设置/托盘操作，不得无故出现另一个编辑器；手动实验入口取消时仍只保留已有截图。
5. 用移除/不可执行的 sibling companion 测试自动回退稳定 eframe，截图不得丢失。只对临时开发包操作，不要修改正在使用的应用。
6. 在原生面板卡住、输入法抢焦点或缺少回包时，选择菜单栏“显示窗口”应停止 companion 并恢复原来的稳定截图编辑器。重复测试 10 次，确认不出现捕获自窗口的残影。
7. 记录混合 1×/2×、负原点、Spaces、全屏、TCC 拒绝/授权/撤销、中文与日文 IME，以及启动/P95/内存/帧指标。CI 的头less smoke、preflight `not_run` 和签名验证不能替代实际 GUI 验收。

**回退方式：** 在设置页关闭试运行选项，或重新启动应用；旧 eframe 截图路径继续工作。此里程碑不是“完整原生截图 UI 已发布”，Issue #5/#10 在采集真实证据前继续开放。
