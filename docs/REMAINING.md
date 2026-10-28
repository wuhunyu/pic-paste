# PicPaste 未完成任务与 Issue 对账台账

> **核对日期：2026-10-27** · 最新功能基线 [`main@73122e0`](https://github.com/wuhunyu/pic-paste/commit/73122e009c4bef66b39c4ef0f0f60def4680c76e)，[PR #31](https://github.com/wuhunyu/pic-paste/pull/31) 已合并 · 完整 [PR CI #40461875462](https://github.com/wuhunyu/pic-paste/actions/runs/40461875462) 成功。原生 NSPanel、多屏焦点/Spaces/TCC、OCR Add-on 真签名及性能仍需设备验收。
>
> 此文档与 [TASKS.md](TASKS.md)、[STATUS.md](STATUS.md) 和 [Issues](https://github.com/wuhunyu/pic-paste/issues?q=is%3Aissue+is%3Aopen) 同步。以下任务状态指 **代码与证据**，不能将单元测试、编译成功等同于操作系统真机功能验收。独立的当前 Issue 正文是每项工作的逐条关闭清单。

## 进度快照

| 口径 | 数量 / 状态 |
| --- | --- |
| 原始 v1.2 任务 | 20 项：**8 项代码基本完成、9 项部分实现、3 项未开始** |
| GitHub Issues（非 PR） | **7 个未关闭：#5–#11**；最初 workspace/CI 对应的 #4 已关闭 |
| GitHub Pull Requests | **23 个已合并**（#1–#3、#12–#31），当前无待合并功能 PR |
| macOS 开发包 | ARM64 Lite/Full ad-hoc 签名 ZIP、SHA256；**不是正式发行件** |
| GitHub Releases | **0** |
| Windows 原生能力 | 未开始；Rust 核心接口存在，不代表 Windows 构建可用 |

## 按 Issue 汇总尚未完成的工作

| 优先级 | Issue / 任务覆盖 | 已有代码证据 | 尚需完成（关闭前必须确认） |
| --- | --- | --- | --- |
| P0 | [#5 macOS 浮层、多屏、输入](https://github.com/wuhunyu/pic-paste/issues/5)：P0-T2/T3/T4、P1-T1 | #12–#15、#20、#31；[#32](https://github.com/wuhunyu/pic-paste/pull/32) 独立 NSPanel + WGPU Metal 编译试验（非正式 UI） | 完整 NSPanel egui/事件桥接、真实 GUI/Spaces/TCC/焦点/热键和混合 DPI，窗口污染及延迟真机实测（见 [窗口策略](macos-window-strategy.md)） |
| P1 | [#6 标注、PNG、剪贴板、贴图](https://github.com/wuhunyu/pic-paste/issues/6)：P1-T2/T3、P2-T1/T2 | #16、#17、#22、#26 与 [#30](https://github.com/wuhunyu/pic-paste/pull/30)：PNG/Alpha、CJK、Pin OCR、长图选区及 64MiB GPU 纹理预算/复用/关闭回收代码 | 5-pin 帧 P95、RSS/CPU、焦点/拖动/穿透恢复、字体覆盖、macOS 真实粘贴 Alpha/色彩一致性 |
| P1 | [#7 手动长截图](https://github.com/wuhunyu/pic-paste/issues/7)：P4-T1/T2/T3/T4 | #18、#20、#21、#23–#27：实时预览、拼接、手动慢速恢复、固定边距、PNG/标注/圆角、安全中间重拍 | ≥50 个真实网页/文档、可信拼接率 ≥95%、30k/60k/200MP 边界、动态场景与异常/内存/临时文件实测 |
| P1 | [#8 OCR / QR / Add-on](https://github.com/wuhunyu/pic-paste/issues/8)：P3-T1/T2/T3、R-010 | Swift Vision Helper、SHOC IPC、#28 原生词/字符 bbox 与离线 QR、Lite 降级；独立 Add-on 安全安装/运行时代码已由 #29 合并且 CI 通过 | 开发者证书与匹配 Team 的真实安装、签名公证/跨版本、取消/错误生命周期、400 样本 CER 与真机验收 |
| P2 | [#10 真机测试与性能](https://github.com/wuhunyu/pic-paste/issues/10)：P5-T1 | 自动 CI 和外部 CSV 统计脚本存在；无真实设备采集报告 | 30 次冷启动、100 次热键、5 图贴图、CPU/RSS、50 长图、400 OCR、macOS 14+ 多屏/权限/异常矩阵和原始数据 |
| P2 | [#9 正式发布](https://github.com/wuhunyu/pic-paste/issues/9)：P5-T2 | ARM64 Lite/Full 开发 ZIP + SHA256 | 正式 Bundle ID/图标、x86_64、Developer ID、Notarization/Gatekeeper、正式 Release/回滚；依赖真实测试和苹果证书 |
| P3 | [#11 Windows](https://github.com/wuhunyu/pic-paste/issues/11)：P6-T1/T2 | 可复用 Rust 核心 | Windows 捕获/贴图/输入/OCR Provider、安装/代码签名/CI/性能，**macOS 验收后推进** |

## 不依赖真机仍需开发的功能

- [x] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：PR #28 已提供 Vision 原生词/字符 bbox、截图/贴图选择与复制；几何缓存与真实焦点交互待设备验证。
- [x] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：PR #28 已提供 R-010 显式离线 QR 检测和复制、空码/正向本地样本回归；旋转/多码真机样本待补。
- [ ] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：独立 OCR Add-on 的固定目录、Developer ID/Team ID、版本与安装卸载代码由 #29 合并且 CI 通过；**仍需真实 Developer ID 安装/公证/恢复与设备测试**才能关闭验收。
- [ ] [#5](https://github.com/wuhunyu/pic-paste/issues/5)：[窗口策略](macos-window-strategy.md)记录 #32 独立真实 NSPanel + WGPU Metal smoke probe；**没有替换 eframe 生产窗口**，真正 NSPanel 的 egui/输入/IME/窗口关闭桥接与真实 Mac 验收仍未完成。
- [x] [#6](https://github.com/wuhunyu/pic-paste/issues/6)：PR #30 实装 12 个 Pin、64MiB 唯一纹理估算、4MP/4096px 上限，精确 RGBA 相同共享 TextureHandle，末个 Pin 关闭释放；真实 GPU/RSS/P95 仍需设备验收。
- [ ] [#10](https://github.com/wuhunyu/pic-paste/issues/10)：补充可复用的性能/长图/OCR 数据采集与报表自动化；已有 `scripts/bench_report.py` **只计算输入 CSV 汇总**，不是性能数据采集器。

## 必须使用真机/证书完成的验收与发行

- [ ] [#5 / #10](https://github.com/wuhunyu/pic-paste/issues/10)：macOS 14+ 混合 DPI、Spaces、Screen Recording TCC、窗口焦点、热键与重启。
- [ ] [#6 / #10](https://github.com/wuhunyu/pic-paste/issues/10)：30 冷启动 P95 ≤1.5s，100 次热键 P95 ≤200ms，空闲 CPU ≤0.5% / RSS ≤80MiB，5 张 1080p Pin 帧 P95 ≤16.7ms。
- [ ] [#7 / #10](https://github.com/wuhunyu/pic-paste/issues/7)：50 个真实滚动场景、可信成功率 ≥95%；30k/60k 边界及失败保留/取消清理。
- [ ] [#8 / #10](https://github.com/wuhunyu/pic-paste/issues/8)：400 中英混合 OCR 图、中文 CER ≤2%、混合 CER ≤5%、命中率 ≥95%，含原样本、设备、失败样例。
- [ ] [#9](https://github.com/wuhunyu/pic-paste/issues/9)：正式标识、双架构、Developer ID、公证、Gatekeeper、签名发行、回滚和 GitHub Release。
- [ ] [#11](https://github.com/wuhunyu/pic-paste/issues/11)：macOS 验收后进行 Windows 原生能力完整实现和测试。

## 需求边界与非本阶段承诺

1. **自动滚动不做。** 手动滚动 + 自动取帧拼接是最终需求；禁止将其当作未完成。
2. **仅 PNG。** 不做 JPG/WebP 格式选择；圆角透明 Alpha 不应丢失。
3. **长图贴图/复制允许原分辨率有界选区**（≤400 万像素、单边≤4096 px）；不能声称支持 60,000 px 整图作为单张 GPU Pin。
4. [PR #27](https://github.com/wuhunyu/pic-paste/pull/27) 的动态过滤**只参与拼接匹配**，不自动擦除导出 PNG 上的动画元素。中间错误段重拍会放弃该段之后的旧内容；若将来要在原位置替换后保留旧后缀，必须建立双侧可信匹配协议，该能力**目前未实现，属于可选增强**。
5. 以上“代码基本完成”仅代表核心实现和自动化测试；每个 Issue 的真实验收项在拿到可复测证据之前一律保持未勾选/未关闭。

## 当前推荐推进顺序

**#5 原生浮层策略及 #6 贴图焦点/性能真机验收 → #10/#7 真实设备与长图验收（含 #8 OCR 400 样本和 Add-on 可信安装）→ #9 正式发行 → #11 Windows。** [Issue #10](https://github.com/wuhunyu/pic-paste/issues/10) 的测试仪表与数据集整理可与 #8/#5/#6 并行实施，但不能把模拟结果当成设备实测。

## 2026-10-27 — Native panel lifecycle incremental (#34)

- [x] Core close/suspend/Retina size-change transition tests: RED CI [#40483025505](https://github.com/wuhunyu/pic-paste/actions/runs/40483025505) failed with expected unresolved imports; follow-up GREEN core tests and strict Clippy passed.
- [ ] PR [#34](https://github.com/wuhunyu/pic-paste/pull/34) adds actual AppKit X-close event pump, native view-size/backing-scale reconfiguration, minimize/restore redraw, and explicit WGPU surface-before-view drop order. The Mac compile/full CI and interactive desktop acceptance must both be checked independently.
- [ ] UI/egui mouse-keyboard/IME event forwarding, production overlay migration and true macOS 14+ Retina/Spaces/TCC tests remain open. Do not close Issue #5 for a successful probe compile.

## 2026-10-27 — NSPanel egui input proof (#35)

- [x] [PR #35](https://github.com/wuhunyu/pic-paste/pull/35) merged after [CI #40491725274](https://github.com/wuhunyu/pic-paste/actions/runs/40491725274): opt-in egui and mouse/keyboard probe; no macOS 14+ GUI acceptance.
- [ ] Native NSTextInputClient CJK IME/marked-text bridge, full keyboard/accessibility behavior and replacing production eframe viewports are still outstanding; Issue #5 stays open.

## 2026-10-27 — macOS 26 Liquid Glass material proof (#36)

- [x] Design decision and isolated [PR #36](https://github.com/wuhunyu/pic-paste/pull/36): one native `NSGlassEffectView.contentView` label floating above the existing Metal probe; macOS 14–25 vibrancy, Reduce Transparency solid-color fallback, no screenshot pixel changes.
- [x] TDD policy/geometry RED [CI #40494581713](https://github.com/wuhunyu/pic-paste/actions/runs/40494581713) (missing module) and GREEN Linux core/strict Clippy.
- [ ] Record the latest **complete** Linux/macOS CI run (including native probe, default workspace, Release and Lite/Full/Add-on) before marking PR code as verified or merging.
- [ ] Real macOS **26** visual/interaction tests: glass material and label content; light/dark; Reduce Transparency; Retina 1x/2x; resize/minimize/restore/close; keyboard/mouse/Spaces/focus; check CPU and memory. macOS 14–25 fallback also needs a real host. Live accessibility preference change is not watched (reopen probe).
- [ ] Create actual functional screenshot and pin toolbars, with native actions and accessibility and an explicit rollback switch; do not confuse this material status badge with completed production UI.

## 2026-10-28 — Interactive native glass preview controls (#37)

- [x] [PR #37](https://github.com/wuhunyu/pic-paste/pull/37) adds two native AppKit `NSButton` controls to the existing opt-in glass/vibrancy/opaque capsule: theme toggle of the **demo only** and safe window close. Merged to main after [CI #40609033351](https://github.com/wuhunyu/pic-paste/actions/runs/40609033351) succeeded.
- [ ] Real macOS 26/14+ visual and mouse/keyboard/VoiceOver/Retina/Spaces/Reduce Transparency acceptance, actual screenshot toolbar actions, clipboard/save/annotate and production rollout switch remain **open**.

## 2026-10-28 — #38/#39 native IME and #40 explicit companion entry

- [x] [#38](https://github.com/wuhunyu/pic-paste/pull/38): opt-in NSPanel NSTextInputClient preedit/candidate coordinates, UTF-16 range tracking and text commit; merged after [full CI #40613304167](https://github.com/wuhunyu/pic-paste/actions/runs/40613304167) success. GUI input not physically checked.
- [x] [#39](https://github.com/wuhunyu/pic-paste/pull/39): native-consumed key-down/key-up suppression, other keys and Command/Control shortcuts preserved, focus-loss reset; merged after [full CI #40615197102](https://github.com/wuhunyu/pic-paste/actions/runs/40615197102) success.
- [ ] [#40](https://github.com/wuhunyu/pic-paste/pull/40): optional main-app button launching a same-directory, out-of-process NSPanel experiment. This is a **developer-only manual test entry**, not an actual native screenshot/editor backend. Test-only feature, single child, no PATH fallback, errors leave normal capture untouched. Latest CI and physical Mac GUI remain independent gates.
- [ ] Dedicated production screenshot/pin NSPanel with complete capture/editor/clipboard/annotation parity, real macOS 14+/26 GUI/IME/Retina/Spaces/VoiceOver/TCC tests and rollback acceptance. **Issue #5 stays open.**


## 2026-10-28 — PR #41 有界真实截图到 NSPanel 的桥接

- [x] 单次版式化帧协议 `PPSNAP01`：有界 RGBA 原图、显示器逻辑点元数据、匿名 stdin、不落盘；Linux 纯逻辑测试与 macOS 原生无 GUI 集成测试。
- [x] 仅开发 Feature 启用真实截图冻结画面的只读 AppKit NSPanel/Metal 预览；默认正式选区和标注仍为 eframe，PNG-only 输出不变。
- [ ] PR #41 全量 CI、真机 macOS 14+/26 显示和多个屏幕/缩放/Spaces、内存峰值验收；真实原生选区拖拽、选区结果返回主进程、标注和回退仍是后续工作，不能在 PR #41 中宣称完成。


## 2026-10-28 — Native selection handoff milestone (PR #42)

- [x] Experimental NSPanel captures a **single drag rectangle** over immutable full-target-display pixels, transports exactly one validated `PPSEL001` reply, checks display ID, source bounds and capture revision before updating the legacy eframe editor.
- [x] Core tests reject malformed response or cancelled selection; macOS headless tests exercise CLI, native paint and sibling stdin/stdout process fixtures; the production path remains eframe.
- [ ] Device-level macOS NSPanel presentation, borderless keyboard focus/Escape, Retina/negative-origin monitor matching, Spaces and screen unplug behavior; no test-automation result substitutes for these.
- [ ] Native selection handles/resize/move, annotations, hotkeys, OCR interaction and full editor feature parity remain separate later gates; Issues #5/#10 stay open.


## 2026-10-28 — PR #43 原生可调整选区

已实现仅开发 Feature 下的 NSPanel 八方向手柄缩放、内部移动、物理像素尺寸反馈，鼠标拖动结束不会提交选区，只有 Enter/确认才回传，Esc/取消一律保留原截图。沿用 PPSEL001 和旧 eframe 编辑器；源 RGBA 和 PNG 保存链路不修改。剩余任务：真实 Mac 图形交互/负坐标与 Retina/Spaces/权限验收、完整原生标注与 Liquid Glass 工具栏、性能测试和正式可回退后端迁移，保持 Issue #5/#10 开放。


## 2026-10-28 — PR #44 原生标注工具栏本地预览

NSPanel 代码已加入真正 AppKit 系统材质工具栏，画笔/箭头/矩形/文字与撤销只操作受限的本地标注图层；已设定一条不可误解的交付边界：**本地标注不会保存到 PNG，也不会通过当前 PPSEL001 同步到 eframe**。后续必须以新的版本化、长度受限标注 IPC 验证传输，父进程在截图 revision/显示器/源尺寸核验通过后再把标注转成 shot-image AnnotationDocument 命令。macOS 26 真实 Liquid Glass、14–25 视觉回退、降低透明度/焦点/中文输入、Retina 多屏、Spaces 与 CPU/GPU/RSS 仍待真机验证。


## 2026-10-28 — PR #45 原生标注回传已接入（仍为实验）

此前 PR #44 标注只在独立 NSPanel 中预览。本阶段的版本化有界 `PPANN001` 将已提交标注转为原编辑器 `AnnotationDocument`，可撤销/重做并通过现有 PNG 合成/导出。取消、无效协议、超限/越界或过期截图均不修改历史。仍需 macOS 14–26 真机多屏 Retina、Spaces/TCC/焦点、降低透明度、中文 IME、GPU/CPU/RSS 与真实保存对比验收；不将自动化测试当作 GUI 完成认证。
