# PicPaste 未完成任务与 Issue 对账台账

> **最新对账（2026-10-29）**：`main` 合并 PR #55（`2ba61d8`），47 个已合并 PR；GitHub 实际 **6 个 Open Issue：#6–#11**。#5 已在 2026-10-29 由仓库所有者关闭（见 [#5](https://github.com/wuhunyu/pic-paste/issues/5)），但原生窗口、权限、焦点、Retina/IME、真实性能证据并未完成；相关阻塞条件保留于 #10，不将 #5 的 Closed 推断为实测通过。

> **核对日期：2026-10-29** · 已合并基线扩展至 [PR #52](https://github.com/wuhunyu/pic-paste/pull/52)（#47 椭圆、#48 原生 Undo/Redo；#47 全量 CI #40655168318 success）。实验性原生标注已进入 PNG 保存链路，但 NSPanel/Retina/Spaces/TCC/IME/性能仍无真实设备验收。
>
> 此文档与 [TASKS.md](TASKS.md)、[STATUS.md](STATUS.md) 和 [Issues](https://github.com/wuhunyu/pic-paste/issues?q=is%3Aissue+is%3Aopen) 同步。以下任务状态指 **代码与证据**，不能将单元测试、编译成功等同于操作系统真机功能验收。独立的当前 Issue 正文是每项工作的逐条关闭清单。

## 进度快照

| 口径 | 数量 / 状态 |
| --- | --- |
| 原始 v1.2 任务 | 20 项：**8 项代码基本完成、9 项部分实现、3 项未开始** |
| GitHub Issues（非 PR） | **6 个未关闭：#6–#11**；#5 由所有者关闭，但实际原生 GUI 验收仍由 #10 跟踪 |
| GitHub Pull Requests | **47 个已合并**（#1–#3、#12–#55）；#46 是只读环境预检，#55 是超时恢复，不代表真实设备测试完成 |
| macOS 开发包 | ARM64 Lite/Full ad-hoc 签名 ZIP、SHA256；**不是正式发行件** |
| GitHub Releases | **0** |
| Windows 原生能力 | 未开始；Rust 核心接口存在，不代表 Windows 构建可用 |

## 按 Issue 汇总尚未完成的工作

| 优先级 | Issue / 任务覆盖 | 已有代码证据 | 尚需完成（关闭前必须确认） |
| --- | --- | --- | --- |
| P0 / owner closed, physical QA still open in #10 | [#5 macOS 浮层、多屏、输入](https://github.com/wuhunyu/pic-paste/issues/5)：P0-T2/T3/T4、P1-T1 | #12–#15、#20、#31；[#32](https://github.com/wuhunyu/pic-paste/pull/32) 独立 NSPanel + WGPU Metal 编译试验（非正式 UI） | 完整 NSPanel egui/事件桥接、真实 GUI/Spaces/TCC/焦点/热键和混合 DPI，窗口污染及延迟真机实测（见 [窗口策略](macos-window-strategy.md)） |
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

## 2026-10-28 — PR #46 真机验收工具（不等于验收通过）

[docs/native-macos-acceptance.md](native-macos-acceptance.md) 提供 `ns-panel-probe --preflight` 非侵入环境报告、逐项未执行的真机验收模板及严格 `scripts/native_acceptance.py verify`（记录真实证据和 P95/资源阈值）；CI Runner 的零显示器/无 TCC 权限只算环境观测，**不是 GUI 测试**。真实 Mac 上的混合 DPI/负屏幕原点、NSGlassEffectView/降低透明度、中文 IME、Spaces/TCC、PNG 逐像素确认和资源性能仍须人工操作及真实测量后才能关闭 Issue #5/#10。

## 2026-10-28 — 原生标注增量 #47/#48

已添加实验性的原生椭圆拖拽、`PPANN001` tag 5 兼容回传与 Undo/Redo 本地重做栈，新增系统材质“椭圆/重做”按钮并保持生产 eframe 路径不变。下一阶段仍需线条、模糊、马赛克等原生工具对齐与真实 Mac 交互、可访问性、性能验收；不得把代码 CI 当作 #5/#10 的关闭证据。

## 2026-10-28 — PR #49 原生三工具增量

可选 NSPanel 补齐直线、马赛克、模糊的源像素手势、区域边界与有界协议回传；父编辑器复用原有非破坏性 PNG 合成与撤销重做。双排系统材质工具栏不超过 560pt 宽。原生马赛克/模糊是框选范围提示，**真实像素级预览在确认后原编辑器展示**；这不是生产原生后端完成。仍需真实 Mac 14/26、Retina/Spaces/TCC/IME、PNG 对照与资源预算验收。

## 2026-10-28 — 原生键盘路由增量 #50

可选 NSPanel 已接入 Cmd+Z、Shift+Cmd+Z、Enter、Escape 的原生按键路由，成对拦截 KeyDown/KeyUp，重复按键不会多次执行；文字输入/IME 激活时由 egui/NSTextInputClient 处理，不再强制把 Escape 当作整窗取消。按 `charactersIgnoringModifiers` 支持键盘布局的字符映射。仍待真机 macOS 14/26 非美式布局、中文/日文预编辑、候选框、焦点/Spaces/TCC/实际性能测试；默认 eframe 和 PNG 输出不改，原 20 任务计数不变。

## 2026-10-28 — PR #51 精确 ROI 像素预览

新增原生 NSPanel 仅在已提交、完全互不重叠的马赛克/模糊标注上应用和最终 PNG 相同的区域合成；源 CPU 额外保留 ≤32MiB，缓存的全部效果像素 ≤1MP，缓存按提交/撤销/重做更新。超出预算、任意混合文字/矢量、相交编辑与正在拖拽时仍显示明确的区域框，不宣称最终像素完全可见。默认 eframe 和 PNG-only 不变。接下来仍需混合标注的最终合成预览、真实 Mac 14/26 系统玻璃/Retina/IME/Spaces/TCC、性能和生产 NSPanel 等价迁移。#5/#6/#10 保持开放。

## 2026-10-28 — #5/#10 原生截图试运行的剩余关闭门槛（PR #52）

- [x] PR #52 在 `native-panel-probe` 开发 Feature 下实现 **显式原生优先**截图路由、request ID 和截图/标注 revision 校验、崩溃/协议错误回退、自动取消结束截图，以及菜单恢复稳定 eframe 编辑器的路径；原本的生产默认捕获不变。
- [x] 增加独立命名的 Lite/Full ad-hoc canary 包及 companion 文件安全检查，CI 中验证签名/ZIP；普通包不携带未经验收的原生 companion。
- [ ] PR #52 完整 PR CI [#40677034954](https://github.com/wuhunyu/pic-paste/actions/runs/40677034954) 与主线 CI [#40678378483](https://github.com/wuhunyu/pic-paste/actions/runs/40678378483) 均已 success；**仍不能打勾** macOS GUI、TCC、Retina、IME、CPU/GPU/RSS/P95。
- [ ] 在真实 macOS 14/26 上验证**每个**自动接入分支：成功选区/标注到 PNG、取消不弹回原编辑器、缺少 companion 自动退回、原生卡死后托盘“显示窗口”恢复、重启回滚；收集回放和原始测量数据。
- [ ] 默认稳定原生生产面板仍需通过 #5/#10 全套兼容性/性能门槛才能切换；#9 正式发布与 #11 Windows 无变化。

## 2026-10-29 — 发布证据汇总门禁代码（P5-T3 工具预备）

- 新增 `scripts/release_evidence_gate.py`，复用现有真实设备、OCR 和 P95 统计逻辑，对 macOS 14/26 双报告、原始延迟、≥400 OCR、≥50 长截图与源码 SHA 执行失败即阻断检查。
- 新增 `tests/test_release_evidence_gate.py` 的合成数据拒绝测试，以及 [证据采集指南](release-evidence-runbook.md)，并将私密 `evidence/` 路径加入 gitignore。
- **仅准备验收工具**：真实 macOS 14/26 GUI、CPU/RSS/P95、50 长图、400 OCR、签名/公证都仍缺实测数据；不能关闭 #5–#10，也不能把合成单测当真实测试通过。完整 CI 以本变更 PR 和 SHA 为准。

## 2026-10-29 — P1-T3 原生试运行安全恢复（PR #55，待 CI）

- 新增绑定请求 ID 的 `NativeSelectionWatchdog`：**仅在当前请求仍活跃、超出 300 秒时**触发，旧请求及已取消会话不能对当前截图发出超时回退。
- eframe 主编辑器在开发 Canary 等待期间每秒安排一次轻量检查。超时会先清除会话修订号，再停止独立 NSPanel helper，让可能随后抵达的 pipe EOF / 旧回复自动失效；源 PNG/已保存的内容不被改写。
- 通过“显示窗口”菜单可手动结束正在等待回复的原生框选（包括非自动启动的实验框选）；截图后从稳定编辑器继续，普通生产包行为保持不变。
- 纯 Rust 单元测试覆盖 request ID、时间边界、旧会话、错误 watchdog 参数；macOS 无 GUI 故障注入覆盖子进程没有 stdout 的强制关闭与 pipe reader 退出。
- **此项只是软件安全措施和自动化回归**；macOS 14/26 真机应按 [验收清单](native-macos-acceptance.md) 第九节完成超时、输入法、显示器、权限和焦点测试；#5/#10 不关闭，不提前迁移生产默认 NSPanel。

## 2026-10-29 — P6-T2 开发子任务：Universal 2 双架构开发包（待 PR CI）

- 双架构构建脚本 `scripts/build-macos-universal-dev.sh` 为 Rust 主程序和 Swift OCR 编译 `arm64`、`x86_64` 两份，先验证薄片，再经 `lipo` 合并，并使用 `scripts/universal_macho.py` 严格核对 Fat Mach-O 的两个真实切片，排除缺失、重复、重叠、越界或 CPU 类型错误。
- Lite/Full 仍复用现有 `scripts/package-macos.sh`，新增带安全校验的 `PICPASTE_PACKAGE_ARCH=universal2`；防止把单架构二进制伪装为 Universal 2，并在输入不正确时保留旧的工作包。
- 两个生成包分别为 `PicPaste-macos-universal2-Lite-UNSIGNED-DEV.zip` 和 `Full-UNSIGNED-DEV.zip`，包含真实双架构主程序，Full 还包含双架构 OCR；原有 arm64 / Canary 打包不变。
- 独立按需/发布文件变更触发的 CI 验证双架构编译、应用包、Code Sign（**仅 ad-hoc**）及 SHA256；其通过不等于双架构真机启动/授权/UI 验收。
- #9 仍须完成 Developer ID、Notary、Staple、Gatekeeper、配套 Add-on 的正式签名/公证、真实 Intel/macOS14 及 macOS26 设备验证；**本子任务不创建任何正式 Release**。


## 2026-10-29 — P6-T2 Developer ID/Notary 受控候选包工具（未完成真实签名）

- 独立分支新增 `scripts/release_signing_policy.py`、15 项纯 Python 单测与
  `scripts/release-macos-signed.sh`：对最终源码 SHA、真实 macOS14/26
  采样汇总、手工复核、Intel/Apple Silicon、严重缺陷数量和匹配 Apple
  Team ID 作前置失败即拒绝检查。报告变更会使审批 SHA256 失效。
- 受控 Mac 上重新双架构构建；对 Full 内置 OCR、主程序、Bundle 与独立
  OCR Add-on 使用相同 Developer ID、Hardened Runtime + timestamp
  签名，公证必须 Accepted 后 staple .app、Gatekeeper 预检、最终 ZIP
  校验。生成仅供后续净机验证的 `*-CANDIDATE.zip`，**不创建公开 Release**。
- PR CI 只测试证据政策与 Bash 语法，不调用真实证书/Notary 服务。
  **当前没有**可信 Apple 证书、Mac14/26 真机原始验收数据或人工 QA
  审批输入，因而未实际签名、公证或执行干净 Mac 下载验收。
  #9、#10 继续保持 Open；详见
  [签名候选包操作手册](signed-release-candidate-runbook.md)。
