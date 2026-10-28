# PicPaste 开发与验收状态

> **核对日期：2026-10-28。** 此文件以 [`main`](https://github.com/wuhunyu/pic-paste/tree/main) 中已合并的代码为依据；[REMAINING.md](REMAINING.md) 汇总当前七个未关闭 Issue，原始 v1.2 执行计划的 20 个任务逐项记录在 [TASKS.md](TASKS.md)。**代码完成、自动化测试成功、真机验收通过是三个不同的状态。**

## 当前基线

- 已合并基线更新至 PR #49（#1–#3、#12–#49，共 **41 个 PR**）。#46 只读真机预检工具、#47 实验原生椭圆标注与 #48 原生撤销/重做都属于代码里程碑，不能替代实际 Mac GUI 验收。
- 最新原生实验功能：[#47 椭圆标注](https://github.com/wuhunyu/pic-paste/pull/47)已通过[全量 CI #40655168318](https://github.com/wuhunyu/pic-paste/actions/runs/40655168318)，[#48 原生撤销/重做](https://github.com/wuhunyu/pic-paste/pull/48)新增独立的本地重做历史及工具栏操作。默认生产截图/贴图仍用 eframe；真实 GUI/Retina/Spaces/TCC/IME/性能必须另行验收。
- #17 合并提交 `7e492a4bb91f6062c225dfb33f524cc51756e556`；[main CI #40297130672](https://github.com/wuhunyu/pic-paste/actions/runs/40297130672) **success**。
- 历史增量：[#23 手动失败恢复](https://github.com/wuhunyu/pic-paste/pull/23)、[#24 手动固定栏排除](https://github.com/wuhunyu/pic-paste/pull/24)、[#25 长图圆角](https://github.com/wuhunyu/pic-paste/pull/25)、[#26 长图标注/贴图](https://github.com/wuhunyu/pic-paste/pull/26) 均已合并；对应测试记录详见各 PR。合并后的主分支 CI 已在 #40440845772 覆盖这些功能。
- 跟踪 Issues：[#5 原生窗口和多屏](https://github.com/wuhunyu/pic-paste/issues/5)、[#6 标注与贴图](https://github.com/wuhunyu/pic-paste/issues/6)、[#7 长截图](https://github.com/wuhunyu/pic-paste/issues/7)、[#8 OCR/二维码/Add-on](https://github.com/wuhunyu/pic-paste/issues/8)、[#10 真机与性能](https://github.com/wuhunyu/pic-paste/issues/10)、[#9 macOS 正式发布](https://github.com/wuhunyu/pic-paste/issues/9)、[#11 Windows](https://github.com/wuhunyu/pic-paste/issues/11)。[#4](https://github.com/wuhunyu/pic-paste/issues/4) 已关闭。
- **GitHub Issues：7 个未关闭（#5–#11）。原计划 20 项：8 项代码基本完成、9 项部分实现、3 项未开始（按任务项计数，不是工时或验收完成率）。** 详细剩余任务见 [REMAINING.md](REMAINING.md)。
- **GitHub Releases：0。** 目前只有 ARM64 Lite/Full ad-hoc 签名开发 ZIP（#29 另提供独立 Add-on ad-hoc 开发 ZIP），非正式发行件；ad-hoc Add-on 被运行时主动拒绝。

## 功能现状（截至本次基线）

| 工作流 | 已实现 / 已通过 CI 的代码 | 尚缺失或未验收 |
| --- | --- | --- |
| P0 macOS 截图 | ScreenCaptureKit 选显示器；#31 局部物理像素/裁剪及初始焦点策略；[#32](https://github.com/wuhunyu/pic-paste/pull/32) 独立真 NSPanel + WGPU Metal surface 验证程序（显式编译，不替换主应用） | 正式 UI 仍为 eframe/winit NSWindow；NSPanel 真实渲染/关闭/输入桥接未实机验证，多屏 DPI/Spaces/TCC/延迟仍开放（见 [窗口策略](macos-window-strategy.md)） |
| P1 标注与 PNG | 线、箭头、画笔、矩形、椭圆、马赛克、模糊、**中英文离线文字**（[#17](https://github.com/wuhunyu/pic-paste/pull/17)）；Undo/Redo；PNG-only 无覆盖原子保存、透明圆角、限定大小的 PNG 缩略图、⌘S 保存（[#16](https://github.com/wuhunyu/pic-paste/pull/16)） | 文字字体回退与跨版本系统覆盖、Native 剪贴板粘贴/色彩/Alpha 真机验证、工具边界压力测试 |
| P2 多贴图 | 多窗口置顶、缩放、透明度、鼠标穿透和恢复；#22 行级、#28 词/字符/QR；[#30](https://github.com/wuhunyu/pic-paste/pull/30) 增加独立 RGBA 预算、逐像素相同的 TextureHandle 复用和关闭释放 | 5 张 1080p 帧 P95 ≤16.7ms、RSS/CPU、焦点/拖动/鼠标穿透、跨 Spaces 真机验收 |
| P3 OCR | 本地 Vision、SHOC IPC、20s 超时、Lite 降级；#28 原生词/字符 bbox、显式离线 QR；Add-on 固定路径、Developer ID/Team ID/API 验证与安装/卸载代码已通过 #29 CI 并合并 | ≥400 张 CER 实测、旋转多码/权限/焦点、Developer ID 真签名安装、公证与真机验收 |
| P4 长截图 | 由用户手动纵/横滚动、自动识别可信重叠并拼接；[#23](https://github.com/wuhunyu/pic-paste/pull/23) 连续匹配不可靠时暂停提示，用户手动回滚并在菜单栏继续慢速重试，同一磁盘会话保留可信分段；行流式 PNG、60k/200MP 限制、复核删最后段/保留前 N 段（#21）、实时缩略图（#18）、#25 透明圆角、#26 磁盘补丁标注/撤销重做、复核选区复制与贴图，以及 #27 动态区域稳定像素匹配和从指定中间段重拍后续；[#24](https://github.com/wuhunyu/pic-paste/pull/24) 已实现手动排除上/下固定栏或左/右侧栏（裁剪内容不进入 PNG） | **自动滚动已从需求移除**；自动识别固定栏和自动擦除动画像素仍未实现；#27 可容忍部分动画干扰并安全重拍中间段之后所有片段，但不保留未经验证的后续旧片段；50 场景真机验收仍缺；长图贴图/复制仅支持经过 4MP/4096px 限制的原像素选区 |
| P5 测试/发布 | macOS CI fmt/test/clippy/release、Swift OCR IPC，Lite/Full 开发包、SHA256；基础 `scripts/bench_report.py` 测试 | P50/P95、RSS/CPU、OLED/Retina/Spaces、TCC、CER、50 场景报告；x86_64、正式 Bundle ID/图标、Developer ID 签名/公证/Gatekeeper、发行和回滚 |
| P6 Windows | Rust core/platform 接口可复用 | **无 Windows 原生捕获/贴图/OCR/安装包**；按计划在 macOS 验收后开始 |

## 必须保留的验收标准

1. **性能**：30 次冷启动 P95 ≤1.5s；100 次截图热键响应 P95 ≤200ms；5×1080p 贴图帧 P95 ≤16.7ms；空闲 CPU ≤0.5%、RSS ≤80MiB。没有真实测量不能勾选。
2. **OCR**：至少 400 张中英文混合图；中文 CER ≤2%、混合 CER ≤5%、文字命中率 ≥95%；需提供原始样本与汇总。
3. **长截图**：至少 50 个真实场景，可信拼接成功率 ≥95%；必须覆盖超快跳跃、反向回滚、重复/动态内容、固定页头、重复暂停/慢速重试、结束保留可信部分和取消清理；滚动样本与失败样本可重测；30k/60k 边界测试。用户全程手动滚动，不开发自动滚动。
4. **平台**：macOS 14 起的实际设备；多显示器负原点/不同缩放、Spaces、权限、热键占用、粘贴到其他 App；签名公证通过后方可正式发布。
5. **安全与格式**：文件输出 PNG-only；圆角保持 Alpha；不以在线 OCR、Electron、WebView 代替原生离线方案；Lite 缺失 OCR helper 不能阻断截图/贴图。

## 进度维护规则

- PR 合并后，在同一次变更中更新 [README.md](../README.md)、此 STATUS、[TASKS.md](TASKS.md) 和相应 GitHub Issue；避免“代码已合并但文档仍写未实现”。
- `[x] 代码/CI` 不表示原计划 `[x] 验收`；真机用例及性能报告需要独立证据和可复测环境。
- 有依赖于苹果开发者账号、证书或物理显示器的事项，标明阻塞因素，不假定 Actions 的 macOS runner 已完成交互测试。
- 新的开发任务按照 P0–P6 的依赖推进，不能因为当前 PR 为 Draft 或 CI 仍运行而声称已经完成该功能。

## 2026-10-27 — PR #25 rounded long PNG

- [x] [PR #25](https://github.com/wuhunyu/pic-paste/pull/25) 已合并：长截图复核与 PNG 导出使用已有 0–64 物理像素透明圆角设置；仅逐行更改 Alpha，仍为磁盘分段、流式 PNG、原子防覆盖导出。无圆角时保留原有行为。
- [x] 新增 PNG 解码像素比较回归、完整 Linux core + macOS CI [#40432104187](https://github.com/wuhunyu/pic-paste/actions/runs/40432104187) **success**。
- [ ] 真实 macOS 上验证长图透明角、不同缩放与其他应用显示效果；长图标注、贴图与安全中间段重拍已分别由 #26/#27 实现，但 50 场景和真机验收仍未完成。

## 2026-10-27 — PR #26 长图编辑、复制与贴图

- [x] [PR #26](https://github.com/wuhunyu/pic-paste/pull/26) 已合并：长图复核增加线、箭头、画笔、矩形、椭圆、马赛克、模糊、文字标注与撤销/重做；每项标注以 ≤400 万像素磁盘补丁持久化，原始可信块不修改。完整 PNG 以源分辨率逐行合成和导出（带透明圆角）。
- [x] 可拖选或输入 X/Y/宽/高复制或创建贴图，输出无压缩、无静默降采样；上限 400 万像素/单边 4096 像素。超限显式拒绝，不声称支持整张 60000px 的原生贴图或剪贴板。
- [x] 纵横长截图、重叠标注、撤销/重做、PNG 原子防覆盖、原像素选区与错误边界自动回归。完整 PR [CI #40435034912](https://github.com/wuhunyu/pic-paste/actions/runs/40435034912) **success**（Linux/macOS 单元测试、Clippy、Release、Swift Vision IPC、Lite/Full 开发包）。
- [ ] 真机交互、原生应用剪贴板兼容、贴图焦点、50 场景长截图和真实性能预算仍待完成。动态内容容错匹配及从中间段重拍已由 #27 实现；原位保留后续旧段的拼接属于可选增强，未实现。P4-T3 保持「部分实现」，不得把 CI 视作设备验收。

## 2026-10-27 — PR #27 动态内容容错匹配与中间段重拍

- [x] [PR #27](https://github.com/wuhunyu/pic-paste/pull/27) 已合并：位移提议按跨轴 8 区域采样，对通过正向位移和歧义判定的候选检查 ≥75% 稳定像素共识；小范围动画不再使整行指纹失效。大面积变化、重复花纹、无实际滚动仍拒绝；不会修改最终 PNG 的动态区域像素。
- [x] 复核可指定从第 2 段至最后一段的任一段“重新手动采集”，只保留前面可信片段，并删除目标段及后续内容；用磁盘原始片段重建最后一个完整 ROI，沿原显示器、原物理坐标与原方向继续采集。未确认后缀绝不直接拼接。取消或采集错误会保留可复核前缀，预览按实际最新磁盘数据重建。
- [x] 新增纵横向动态干扰、动画独自变化、重复内容拒绝、260+ 分段视口提取、重拍剪尾/续采、异常预览恢复、实时预览纹理复用自动回归。完整 PR [CI #40440066887](https://github.com/wuhunyu/pic-paste/actions/runs/40440066887) 通过 Linux/macOS 工作区测试、严格 Clippy、Release、离线 OCR Helper 与 Lite/Full 打包。
- [ ] 自动动态像素清理、错误中间段原位替换并保留后续旧片段、真实 50 场景质量统计、macOS 14/多屏/TCC/资源验收仍未完成。P4-T3 继续记为**部分实现**。

## 2026-10-27 — P2 GPU 贴图预算增量（PR #30）

- [x] 纯 Rust 资源账本已通过 Linux 核心测试：12 Pin、64 MiB 唯一 RGBA8 估算、4MP/4096px 单图、8 张 HD 上限、同图引用计数/关闭回收、非法/重复 ID 与失败不改账。
- [x] macOS 共享 `egui::TextureHandle`、隐藏根窗口关闭通知及完整 [PR CI #40458668058](https://github.com/wuhunyu/pic-paste/actions/runs/40458668058) success；仍不能当作真机性能验收。
- [ ] 五张 1080p 悬浮贴图帧 P95≤16.7ms、空闲 CPU/RSS、系统缩放/Retina/Spaces/剪贴板一致性需真实 macOS 采样，不能由核心单测替代。

## 2026-10-27 — #5 显示器像素坐标与窗口焦点（PR #31）

- [x] 测试先行：Linux RED 缺少 `CapturedFrame::for_display` / `region_origin_px`；macOS RED 缺少 `pin_builder`，均为预期接口缺失而非误报。
- [x] 生产路径：完整屏幕 (0,0) 的局部物理像素 + 校验区域源矩形；截图浮层请求焦点、Pin 初始不激活、纯透明阴影关闭、穿透按模式配置。已通过 [PR CI #40461875462](https://github.com/wuhunyu/pic-paste/actions/runs/40461875462)，不等于实机焦点保证。
- [ ] 真机：双显示器负原点/不同 Retina DPI、插拔、全屏 Spaces、TCC 授予/撤销、系统热键/焦点/拖动和 10 帧自窗残影；30 次冷启动及 100 次热键 P95。专用 NSPanel 尚未提供，详见 [决策](macos-window-strategy.md)。

## 2026-10-27 — #5 独立 NSPanel / Metal 试验（PR #32）

- [x] 增加 `native-panel-probe` 可选 Feature 和真实 AppKit NSPanel/WGPU Metal 单帧渲染尝试；`--check` 不触发 AppKit/Metal/TCC；Linux Retina backing 几何单测已通过。
- [ ] 独立 macOS Feature 编译、CLI、严格 Clippy、标准 Lite/Full/Add-on CI 验证以 PR #32 的最新完整 run 为准。
- [ ] 在真实 macOS 14+ 桌面上确认 NSPanel 的 Metal 画面、关闭、1×/2×、Spaces、焦点；**没有运行过 GUI 模式即不能标为完成**。
- [ ] egui-wgpu UI、native 事件/IME/关闭/窗口尺寸生命周期、回退开关和生产截屏/贴图面板迁移均未实现。

## 2026-10-27 — Native panel lifecycle incremental (#34)

- [x] Core close/suspend/Retina size-change transition tests: RED CI [#40483025505](https://github.com/wuhunyu/pic-paste/actions/runs/40483025505) failed with expected unresolved imports; follow-up GREEN core tests and strict Clippy passed.
- [ ] PR [#34](https://github.com/wuhunyu/pic-paste/pull/34) adds actual AppKit X-close event pump, native view-size/backing-scale reconfiguration, minimize/restore redraw, and explicit WGPU surface-before-view drop order. The Mac compile/full CI and interactive desktop acceptance must both be checked independently.
- [ ] UI/egui mouse-keyboard/IME event forwarding, production overlay migration and true macOS 14+ Retina/Spaces/TCC tests remain open. Do not close Issue #5 for a successful probe compile.

## 2026-10-27 — PR #35: experimental egui in NSPanel

- [x] TDD RED: Linux #40486798074 expected missing `PanelPointerPoint`; macOS #40487437579 expected missing egui/ProbeControls. These are **intentional failures before implementation**.
- [x] Feature-gated egui-wgpu rendering and native mouse/keyboard event forwarding: [PR #35](https://github.com/wuhunyu/pic-paste/pull/35) merged after complete [CI #40491725274](https://github.com/wuhunyu/pic-paste/actions/runs/40491725274) success; no interactive Mac acceptance or production migration.
- [ ] Native NSTextInputClient marked-text, CJK candidate window and full input/IME/focus/Spaces/device performance still outstanding.

## 2026-10-27 — macOS 26 original Liquid Glass design / PR #36

- [x] Native glass preference documented and tracked in [Issue #5](https://github.com/wuhunyu/pic-paste/issues/5); see the decision in [macos-window-strategy.md](macos-window-strategy.md).
- [x] [PR #35](https://github.com/wuhunyu/pic-paste/pull/35) was merged after complete [CI #40491725274](https://github.com/wuhunyu/pic-paste/actions/runs/40491725274) success. Its new egui-wgpu/mouse/ASCII input code is **only** in the opt-in native panel probe; no physical GUI validation.
- [x] [PR #36](https://github.com/wuhunyu/pic-paste/pull/36) adds portable material/geometry tests, a single native `NSGlassEffectView` capsule, legacy `NSVisualEffectView`, and accessibility solid `NSBox`, without changing screenshot pixels or default app/packaging code.
- [ ] **Do not equate code commit with acceptance**: the final PR full CI and interactive macOS 26/14+, accessibility, Retina/Spaces/focus/performance evidence must be recorded separately. This badge is a noninteractive proof, not the finished toolbar or CJK IME.

## 2026-10-28 — PR #37 interactive glass demo

- [x] Theme/next-action label TDD RED [CI #40606391694](https://github.com/wuhunyu/pic-paste/actions/runs/40606391694) (missing `PreviewTheme`); pure theme contract GREEN with Linux core tests/Clippy.
- [ ] Add native NSButton target/actions for preview theme and safe close, retain target lifetime and preserve fallback; latest complete PR CI and real Mac GUI still pending. Do **not** claim production glass toolbar completion.

## 2026-10-28 — native input and staged NSPanel access

- [x] [PR #38](https://github.com/wuhunyu/pic-paste/pull/38) native NSTextInputClient marked-text/IME into the independent NSPanel probe, CI #40613304167 success, merged as `1156961`.
- [x] [PR #39](https://github.com/wuhunyu/pic-paste/pull/39) prevents duplicate AppKit-consumed key events in the probe, CI #40615197102 success, merged as `2760542`.
- [ ] [PR #40](https://github.com/wuhunyu/pic-paste/pull/40) adds a manual native-probe launch entry from the regular settings window **only when compiled with the optional `native-panel-probe` feature**. The separate helper binary must already exist next to the app executable, and the settings page reports failure without changing screenshot routing. Headless CI and actual Mac GUI acceptance are distinct.
- [ ] **Not completed:** a production NSPanel capture/editor replacement; normal capture, selection and pins remain eframe/winit. No claim of actual Liquid Glass appearance, IME, Retina, Spaces, Accessibility, TCC or latency verification without device evidence.


## 2026-10-28 — PR #41：真实截图冻结画面进入 NSPanel（开发 Feature）

- [x] 跨进程原始 RGBA 的 `PPSNAP01` 有界协议（128 MiB、8192 px 边长、明确区分桌面逻辑点和屏幕局部物理像素）；输入非法或截断数据在 AppKit 初始化前拒绝。
- [x] `native-panel-probe` 功能构建可用 `ns-panel-probe --snapshot` 通过匿名 stdin 管道只读显示真实截图；截图编辑器中需显式点击实验入口，主程序异步写入，关闭/下次截图前回收子进程。既有 eframe 选区/标注/PNG/贴图路径不变。
- [ ] 必须记录 PR #41 完整 Linux/macOS CI success、真实 macOS 14+/26 GUI 显示/色彩/Retina/Spaces/权限/性能验收；这是只读预览，不具备 NSPanel 原生选区和编辑器等价功能。Issue #5/#10 不关闭。


## 2026-10-28 — PR #42: NSPanel 原生拖拽选区 / 安全回传（可选实验）

- [x] `native_selection` 纯 Rust：CG 左上角桌面逻辑坐标→AppKit 左下角窗口定位（不缩放全局原点）；NSPanel 局部点→目标显示器物理像素选区（反向拖动、1×/2×、裁剪、零面积拒绝）。
- [x] 严格 33 字节 `PPSEL001` 回传协议：固定版本、状态、显示器 ID 和局部物理像素矩形，拒绝乱码、截断、追加字节、越界与跨显示器结果。
- [x] 仅 `native-panel-probe` Feature 增加 `ns-panel-probe --select` 及编辑器“NSPanel 原生框选（实验）”：原生窗口在所选屏幕上作为独立 AppKit/Metal 浮层，拖动后异步 stdout 回传并由 eframe 编辑器二次校验版本及边界；取消/进程异常均保留编辑器。
- [ ] 真实 macOS 14+/26、Retina 1x/2x、负原点多屏、Spaces/fullscreen、TCC、焦点、GPU 和 CPU/RSS 验收；完整原生标注工具栏/保存/贴图尚未迁移，默认路径不变。


## 2026-10-28 — PR #43：NSPanel 八方向选区调整与确认/取消

- [x] 新增 `native_selection_edit` 纯 Rust 物理像素状态机：创建、内部整体移动、八方向手柄、反向与越界缩放、最低 1px、无效坐标拒绝、单次拖拽撤销；Linux 测试验证 Retina 2× 与负坐标显示器的局部像素操作。
- [x] `ns-panel-probe --select` 在保持真实 RGBA 冻结帧的前提下绘制八个白色手柄、黄色选区轮廓、大小信息及独立的 egui 确认/取消悬浮操作区；鼠标松开只更新本地选区，Enter/确认才通过既有 `PPSEL001` 回传。Escape/取消与关闭返回取消。
- [ ] macOS 14+/26 真机鼠标、Retina、多屏、Spaces/TCC、焦点以及 GPU/CPU/RSS 验收；系统 Liquid Glass 原生标注工具栏和完整原生编辑器仍未迁移，Issue #5/#10 保持开放。


## 2026-10-28 — PR #44：原生 Liquid Glass 标注工具栏（本地预览里程碑）

- [x] 仅开发 Feature 的 NSPanel 中使用系统 AppKit 原生按钮和保持引用的 Cocoa action target，工具选择包含选区、画笔、箭头、矩形、文字、撤销、取消。macOS 26 采用 `NSGlassEffectView`，macOS 14–25 回退 `NSVisualEffectView`，降低透明度优先使用不透明系统背景；不模拟玻璃、不改变截图纹理。
- [x] `shot-core::native_markup` 实现限定规模的物理像素标注命令（≤128 条、≤2048 画笔点、≤256 UTF-8 文字字节）、草稿预览与撤销。NSPanel egui/WGPU 渲染覆盖层接入画笔、箭头、矩形和文字输入框。源 RGBA/PNG-only 保持不变。
- [ ] **标注暂不跨进程回传，也不保存到 PNG**；已有 `PPSEL001` 仅返回选区。下一阶段须另行设计受限的标注 IPC 与主程序 AnnotationDocument 持久化。macOS 真机、Liquid Glass 实物、IME、Retina、Spaces、TCC、性能验收仍未完成。


## 2026-10-28 — PR #45：原生标注进入原有 PNG 保存链路

- [x] 实验 NSPanel 确认后回传有界 `PPANN001`（源图尺寸、显示器、选区和画笔/箭头/矩形/UTF-8 文字），无标注及取消仍发送兼容的 33 字节 `PPSEL001`。
- [x] 主程序对截图与编辑版本、源图物理像素、标注数据进行完整校验；在克隆历史中导入并预合成成功后才原子更新 `AnnotationDocument` 和图片预览。沿用原撤销/重做及 PNG 保存/剪贴板/贴图代码，不改变原始 RGBA。
- [ ] 真实 Mac 14–26、Retina 多屏/负原点、Spaces、权限、Liquid Glass、中文输入法和性能验收仍未完成；默认生产 UI 不变，Issue #5/#10 保持开放。

## 2026-10-28 — PR #46：只读 Mac 环境预检与真实设备验收证据

- [x] 已开发 `ns-panel-probe --preflight`：只采集 macOS 版本、NSScreen AppKit 逻辑点/缩放、系统玻璃策略、降低透明度与**不弹窗**的 Screen Recording 当前权限状态；不捕获画面、不创建原生窗口或 GPU。
- [x] [真机验收规程](native-macos-acceptance.md)及 `scripts/native_acceptance.py` 默认生成全部 `not_run`，要求逐项实际证据、30 次冷启动/100 次热键样本及 P95/CPU/RSS/贴图帧指标；Linux/macOS CI 仅验证工具自身，不把预检当作交互通过。
- [ ] 仍无真实 Mac 14/26 的系统玻璃、中文输入法、TCC/Spaces/混合 Retina/实际性能数据；Issue #5/#10 保持开放，默认生产浮层不切换。

## 2026-10-28 — PR #47/#48：原生椭圆与本地 Undo/Redo

- [x] #47 在可选 NSPanel 追加原生椭圆工具，沿源物理像素绘制并通过 `PPANN001` 新类型 tag 5 进入既有 `AnnotationDocument`、撤销/重做与 PNG 合成；保持 `PPSEL001` 及旧 tag 1–4 兼容。PR 全量 CI #40655168318 success。
- [x] #48 为独立原生面板标注增加真正的 Redo：先撤销后重做恢复已提交标注；新成功标注分支清理重做栈；无效拖拽保留重做历史；重做和撤销都清除未提交草稿。系统材质工具栏新增“重做”按钮；协议仍只传输当前可见的已提交标注，且原始截图像素始终不可变。
- [ ] macOS 14+/26 真机界面、Retina 多屏、中文 IME、焦点/Spaces/TCC、渲染结果和性能指标仍未验证；#5/#10 不关闭，正式生产后端不切换。

## 2026-10-28 — PR #49 实验原生直线、马赛克与模糊

- [x] 在 `native-panel-probe` 内新增源局部物理像素直线、马赛克和模糊，受限 `PPANN001` tags 6–8；旧 `PPSEL001` 和已用 tags 1–5 兼容。
- [x] 单次效果区域 ≤400 万物理像素；无效、越界、超限和损坏协议拒绝，未提交草稿不跨进程，编辑器的 PNG 与撤销/重做复用 `shot-image` 原生绘制、固定 12px 马赛克块和 6px 模糊半径。源 RGBA 不改变。
- [x] macOS 26 系统 Glass、14–25 Vibrancy 与降低透明度回退继续适用；工具栏改两排六列，宽度 560pt。**马赛克、模糊 NSPanel 内仅框选范围，不是最终像素预览**，需确认后看原编辑器合成效果。
- [ ] CI 编译验证和真实 macOS GUI 是两种不同证据；混合 Retina/Spaces/TCC/中文 IME/色彩/性能与生产 NSPanel 迁移仍未验收，#5/#6/#10 不关闭。
