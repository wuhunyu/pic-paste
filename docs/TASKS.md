# 原始 v1.2 执行计划：20 项任务同步台账

更新于 **2026-10-27**，对应 [STATUS.md](STATUS.md)、[REMAINING.md](REMAINING.md) 与 [GitHub Issues](https://github.com/wuhunyu/pic-paste/issues)。**20 项按任务数：8 项代码基本完成、9 项部分实现、3 项未开始**。`代码基本完成` 指核心实现与自动化测试存在，**不表示功能已通过真实 macOS/Windows 验收**；`部分实现` 指原任务至少有一项实质性缺口；`未开始` 指该目标尚无功能代码。

| 序号 | 原计划任务 | 当前状态 | 可核验代码 / 剩余缺口 | 跟踪 |
| --- | --- | --- | --- | --- |
| P0-T1 | Rust Cargo workspace 与 CI | **代码基本完成** | `Cargo.lock`、macOS/Linux fmt/test/Clippy/Release、Swift IPC、Lite/Full 构建；缺真机基准 | [#5](https://github.com/wuhunyu/pic-paste/issues/5)、[#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P0-T2 | AppKit 菜单栏、全局快捷键 | **部分实现** | 菜单栏、四功能快捷键、Ctrl+Option+1–9 自定义及失败回滚（PR #14）；真机冲突/恢复/权限未验收 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P0-T3 | ScreenCaptureKit、多屏与选区浮层 | **部分实现** | 选显示器、八手柄、窗口候选、放大镜；#31 局部像素/逻辑点隔离；[#32](https://github.com/wuhunyu/pic-paste/pull/32) 提供独立 AppKit NSPanel/WGPU Metal 单帧验证程序（不替换正式 UI） | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P0-T4 | 原生贴图、鼠标穿透与性能 | **部分实现** | 多贴图/穿透/恢复与 #30 纹理预算；#31 独立 Pin 初始不激活/无阴影、仅穿透模式透传鼠标；真实跨 Spaces、焦点/拖动与性能未验收 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P1-T1 | 选区、窗口识别、取色/放大镜 | **代码基本完成** | 窗口点击选择 #15、9×9 放大镜/HEX；真实重叠窗口和负坐标验证未完成 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P1-T2 | 标注、撤销重做 | **代码基本完成** | 线/箭头/画笔/几何/模糊/马赛克、文字 #17、Undo/Redo；CJK 系统字体实际覆盖需验证 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P1-T3 | PNG-only、透明圆角、剪贴板 | **代码基本完成** | PNG-only 原子保存、⌘S、圆角预览 #16；原生应用剪贴板/色彩/Alpha 未验收 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P2-T1 | 多贴图缩放、透明度与资源 | **部分实现** | Pin 视口/缩放/透明度；[#30](https://github.com/wuhunyu/pic-paste/pull/30) 新增硬性纹理容量、像素/Pin 数限制、精确同图复用、关闭回收及单测；五张 HD 真实 P95/CPU/RSS 未验收 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P2-T2 | 贴图模式状态机/异常恢复 | **部分实现** | 穿透恢复和 #22 贴图行级选字/复制及移动/选字/穿透互斥代码已实现；仍需真机拖动/焦点/重启恢复验收 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P3-T1 | OCR helper、SHOC、生命周期 | **代码基本完成** | 本机 Swift Vision helper、IPC、超时/退出/签名校验；独立 Add-on 安全加载/安装/打包代码 #29 已合并并通过 CI，需真签名实测 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P3-T2 | OCR 行/词选择与 400 图测试 | **部分实现** | 截图与贴图行级选择 (#22) 及词/字符 bbox、离线二维码 (#28)；仍无 400 图 CER 和真实设备证据 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P3-T3 | Lite / Full / Add-on 交付 | **部分实现** | Lite/Full 开发包已存在；PR #29 已增加单独签名 Add-on ZIP、安装/卸载、Team ID/API 校验；正式证书、签名公证/真实安装未完成 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P4-T1 | 手动纵向/横向滚动及 ROI | **代码基本完成** | 手动区域采样与纵横方向；[#24](https://github.com/wuhunyu/pic-paste/pull/24) 增加可配置固定边距后再取帧（完整 CI 通过，已合并）；真实网页/文档和鼠标穿透未验收 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T2 | 重叠拼接、磁盘块与实时预览 | **代码基本完成** | 匹配拒绝/TileStore/实时小窗 #18；[#20](https://github.com/wuhunyu/pic-paste/pull/20) 已按 PID 排除自身窗口；复杂页面与真实无残影效果仍待实测 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T3 | 手动滚动失败恢复、拼接纠错、编辑 | **部分实现** | [#23](https://github.com/wuhunyu/pic-paste/pull/23) 失败暂停、人工回滚慢速重试、复用可信锚点；[#21](https://github.com/wuhunyu/pic-paste/pull/21) 可裁剪尾部；**[#24](https://github.com/wuhunyu/pic-paste/pull/24) 提供手动固定页眉/页脚/侧栏裁剪（完整 CI 通过，已合并），[#25](https://github.com/wuhunyu/pic-paste/pull/25) 支持 PNG 圆角；[#26](https://github.com/wuhunyu/pic-paste/pull/26) 已实现完整长图的磁盘补丁标注/撤销重做/流式导出及有界原像素复制/贴图；[#27](https://github.com/wuhunyu/pic-paste/pull/27) 新增小范围动态像素容错匹配、从指定中间错误段裁尾并重新手动采集，尚无 50 场景真机验证；自动擦除动画像素、保留后续旧段的安全中间原位替换属于未实现的可选增强（不影响当前安全重拍路径）**。自动滚动为用户明确取消的需求，不计为缺项 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T4 | 行流式 PNG/30k–60k 限制 | **代码基本完成** | 行流式 PNG 和 60k/200MP 上限；极端尺寸/内存压力报告缺失 | [#7](https://github.com/wuhunyu/pic-paste/issues/7)、[#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P5-T1 | 稳定性、质量、性能验收 | **部分实现** | CI 和统计脚本存在；真实设备 P95/CPU/RSS、400 OCR/50 长截图报告尚无 | [#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P5-T2 | Developer ID、公证、正式发布/回滚 | **未开始正式发布** | ad-hoc 开发 ZIP 有；正式 ID、双架构、Developer ID/notarization/Gatekeeper、回滚未完成；Release 0 | [#9](https://github.com/wuhunyu/pic-paste/issues/9) |
| P6-T1 | Windows 原生功能 | **未开始** | 无 Windows 专属屏幕捕获、窗口、输入或安装包；Rust core 可复用 | [#11](https://github.com/wuhunyu/pic-paste/issues/11) |
| P6-T2 | Windows 离线 OCR/打包 | **未开始** | 无 Windows OCR Provider、安装与分发 | [#11](https://github.com/wuhunyu/pic-paste/issues/11) |

## 单独补录的原始需求

- **R-010：本地手动触发二维码识别**。原任务 20 项的拆分没有列出来，已纳入 [Issue #8](https://github.com/wuhunyu/pic-paste/issues/8)；**PR #28 代码及基础 CI 已实现，旋转/多码真机回归尚未完成**。

## 优先级 / 执行顺序（2026-10-27 对账后）

详细可执行剩余清单见 [REMAINING.md](REMAINING.md)，并以各 Issue 最新验收清单为准。

1. **先补不依赖真机的真实功能缺口**：[#8](https://github.com/wuhunyu/pic-paste/issues/8) #28 已完成词/字符级 OCR、R-010 本地二维码；独立 Add-on 的接口/版本兼容与签名验收仍需完整 CI/正式证书；维持纯离线与 Lite/Full 降级。
2. **macOS 原生窗口与交互收口**：[#5](https://github.com/wuhunyu/pic-paste/issues/5) 专用 AppKit NSPanel 方案决策、混合 DPI/Spaces/TCC/热键；[#6](https://github.com/wuhunyu/pic-paste/issues/6) 多贴图 GPU 资源预算、拖动/穿透、PNG Alpha/剪贴板验证。
3. **完成设备级自动采样和验收证据**：[#10](https://github.com/wuhunyu/pic-paste/issues/10) 30 次冷启动、100 次热键、5 图贴图、400 OCR、[#7](https://github.com/wuhunyu/pic-paste/issues/7) 的至少 50 个真实长截图场景 ≥95%、30k/60k 压力及失败恢复。此前 #18–#27 已实现主要长图代码，但 **CI 不能替代真机验收**。
4. **完成 macOS 正式发行**：[#9](https://github.com/wuhunyu/pic-paste/issues/9) 正式 Bundle ID、双架构、Developer ID、公证、Gatekeeper、发行和回滚，需苹果证书及 #10 的核心验收。
5. **最后推进 Windows**：[#11](https://github.com/wuhunyu/pic-paste/issues/11) 原生截图贴图/OCR/打包、性能/系统兼容；Rust core 复用不等于已有 Windows 功能。

**确定不做：** 自动滚动。**PNG-only**、Alpha 透明圆角、离线能力为固定需求；#27 动态像素只在位移匹配中忽略，最终 PNG 保留原始像素。可选的“保留旧后缀中间原位替换”必须具备两侧可信重叠证据，不能作为无证据拼接的默认行为。

更新规则：**代码/PR 是否已合并，以 GitHub 当前状态为准；完成代码 ≠ 满足验收。** 所有阶段关闭时必须附 CI run、设备矩阵、可复测指标和最后的发行证据。

## 2026-10-27 — P4-T3 incremental delivery

- [x] [PR #25](https://github.com/wuhunyu/pic-paste/pull/25) 长截图复核可按现有圆角半径预览并导出带透明圆角的 PNG，逐行处理避免整图 RGBA 内存分配。Linux 和 macOS 完整 CI [#40432104187](https://github.com/wuhunyu/pic-paste/actions/runs/40432104187) success。
- [x] #26 已完成完整长图标注 PNG 与有界选区复制/贴图；#27 已完成小比例动态像素容错匹配与从指定中间段安全重拍。
- [ ] 真机交互、50 场景、资源与质量性能验收仍未完成；P4-T3 继续保持「部分实现」。

## 2026-10-27 — P4-T3 复核编辑闭环（PR #26）

- [x] [PR #26](https://github.com/wuhunyu/pic-paste/pull/26)：磁盘补丁保存各项源像素标注，支持撤销/重做；逐行合成完整 PNG；原像素选区复制/贴图（最多 4MP/4096 px 单边）；保留 PNG-only 与圆角 Alpha。
- [x] 代码层回归：横纵拼接、叠加标注、马赛克/模糊、撤销/重做、原子无覆盖 PNG 和有界原生像素裁剪；Linux + macOS 完整 [CI #40435034912](https://github.com/wuhunyu/pic-paste/actions/runs/40435034912) success。
- [ ] 尚无 50 场景真实滚动准确率、macOS 14 多屏/权限与剪贴板真机验收；#27 已补充部分动态容错及安全中间重拍。注意 #26 不允许完整 60k 边长长图作为一张贴图常驻 GPU。

## 2026-10-27 — P4-T3 动态匹配与可信前缀重拍（PR #27）

- [x] [PR #27](https://github.com/wuhunyu/pic-paste/pull/27)：8 区域位移提议、≥75% 稳定像素匹配共识、重复纹理歧义拒绝；动态内容只影响匹配采样，不清理、不伪造 PNG 原始像素。
- [x] 中间纠错由用户选择首次出错段，从磁盘保留前缀并放弃全部后缀，以最后完整物理 ROI 为重拍锚点；错误或取消保留可信数据。对大数量片段锚点单次只打开一个相关文件句柄。
- [x] 完整 PR [CI #40440066887](https://github.com/wuhunyu/pic-paste/actions/runs/40440066887) success：Linux/macOS 工作区测试、Clippy、Release、OCR Helper 与 Lite/Full 包装；包含纵横动画案例、重复场景拒绝、续采和复核状态测试。
- [ ] 仍需真实 macOS 上 ≥50 场景滚动验收，以及 macOS 14 多屏、动态网页准确率和资源压力统计。可选增强：支持两侧证据的中间原位替换并保留后缀。用户明确不需要自动滚动。


## 2026-10-27 — OCR Add-on 信任边界增量

- [x] [PR #28](https://github.com/wuhunyu/pic-paste/pull/28) 已合并 Vision 原生词/字符定位选择、显式离线二维码，主线 [CI #40453830133](https://github.com/wuhunyu/pic-paste/actions/runs/40453830133) 成功。
- [ ] PR #29 已合并固定路径与双端非空 Team ID 检查、严格代码签名、`--addon-info` API 1/SHOC 1 握手、阶段化安装/回滚/卸载和独立 ZIP；已通过全量 PR CI #40456135637；仍须 Developer ID 真证书及 macOS 14+ 设备验证，才可从“部分实现”转为验收通过。
- [ ] 保留 P3-T2 的 400 张 CER 与实际文字/QR 选取交互数据、P5 和 P6 设备/Windows 依赖项。ad-hoc 开发包**不是**可独立安装的可信 Add-on。

## 2026-10-27 — P2-T1 纹理缓存与资源上限（PR #30）

- [x] `shot-core::pin_texture_budget`：12 贴图、64MiB 唯一 RGBA8 纹理估算、4MP/4096px 单图限制、共享引用计数、拒绝不修改账本、最后引用关闭回收，Linux 核心测试/Clippy 通过。
- [x] macOS 逐像素相同的 `TextureHandle` 复用、关闭时根窗口唤醒/资源回收，已通过 [PR CI #40458668058](https://github.com/wuhunyu/pic-paste/actions/runs/40458668058) Linux/macOS 测试、Clippy、Release 与打包；真机性能仍单独验收。
- [ ] 核心实测仍需 5 张 1080p 贴图帧 P95≤16.7ms、RSS/CPU、焦点/穿透、Retina/Spaces、多应用剪贴板 Alpha。

## 2026-10-27 — P0-T3/P0-T4 局部像素和焦点加固（PR #31）

- [x] [PR #31](https://github.com/wuhunyu/pic-paste/pull/31) 增加 `CapturedFrame` 局部像素/校验裁剪、截图请求激活和 Pin 初始不激活/无阴影/仅模式穿透。
- [x] 代码通过完整 [PR CI #40461875462](https://github.com/wuhunyu/pic-paste/actions/runs/40461875462)：Linux/macOS 全工作区测试、严格 Clippy、Release、Swift IPC 与 Lite/Full/Add-on 打包；**macOS 设备验证仍未完成**，不能将 `with_active` 当作实机焦点保证。
- [ ] [窗口策略](macos-window-strategy.md)明确 NSPanel 仍需独立 AppKit 主线程窗口、Metal surface 和事件桥接，不得声称已替代。

## 2026-10-27 — P0-T3 AppKit NSPanel + Metal 试验（PR #32）

- [x] 独立 `ns-panel-probe` Feature 开发：真实 AppKit NSPanel/NSView 与 WGPU Metal-only surface，基于点/Retina 比例的 8192px 物理画布容量单测，命令 `--check`（纯验证）和 `--show`（真实 Mac 交互）。
- [ ] CI 确认 opt-in Feature 的 macOS 单测/编译、严格 Clippy、`--check`、Linux 核心/默认 Release 与 Lite/Full/Add-on 打包全绿。
- [ ] 真实 macOS 桌面上的显示/生命周期/混合 DPI 与 Spaces 验证，以及 egui 绘制、键鼠/IME/关闭事件桥接、原生截图窗口切换仍未完成。

## 2026-10-27 — Native panel lifecycle incremental (#34)

- [x] Core close/suspend/Retina size-change transition tests: RED CI [#40483025505](https://github.com/wuhunyu/pic-paste/actions/runs/40483025505) failed with expected unresolved imports; follow-up GREEN core tests and strict Clippy passed.
- [ ] PR [#34](https://github.com/wuhunyu/pic-paste/pull/34) adds actual AppKit X-close event pump, native view-size/backing-scale reconfiguration, minimize/restore redraw, and explicit WGPU surface-before-view drop order. The Mac compile/full CI and interactive desktop acceptance must both be checked independently.
- [ ] UI/egui mouse-keyboard/IME event forwarding, production overlay migration and true macOS 14+ Retina/Spaces/TCC tests remain open. Do not close Issue #5 for a successful probe compile.
