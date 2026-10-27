# PicPaste 未完成任务与 Issue 对账台账

> **核对日期：2026-10-27** · 本轮代码基线 [`main@36e7aa2`](https://github.com/wuhunyu/pic-paste/commit/36e7aa28571da2502e168f1dd50549c84ed2cad4)（[PR #27](https://github.com/wuhunyu/pic-paste/pull/27) 已合并）· 功能 [PR CI #40440066887](https://github.com/wuhunyu/pic-paste/actions/runs/40440066887) 成功 · 该代码基线 [main CI #40440845772](https://github.com/wuhunyu/pic-paste/actions/runs/40440845772) 成功。
>
> 此文档与 [TASKS.md](TASKS.md)、[STATUS.md](STATUS.md) 和 [Issues](https://github.com/wuhunyu/pic-paste/issues?q=is%3Aissue+is%3Aopen) 同步。以下任务状态指 **代码与证据**，不能将单元测试、编译成功等同于操作系统真机功能验收。独立的当前 Issue 正文是每项工作的逐条关闭清单。

## 进度快照

| 口径 | 数量 / 状态 |
| --- | --- |
| 原始 v1.2 任务 | 20 项：**8 项代码基本完成、9 项部分实现、3 项未开始** |
| GitHub Issues（非 PR） | **7 个未关闭：#5–#11**；最初 workspace/CI 对应的 #4 已关闭 |
| GitHub Pull Requests | **19 个已合并**（#1–#3、#12–#27），当前无待合并功能 PR |
| macOS 开发包 | ARM64 Lite/Full ad-hoc 签名 ZIP、SHA256；**不是正式发行件** |
| GitHub Releases | **0** |
| Windows 原生能力 | 未开始；Rust 核心接口存在，不代表 Windows 构建可用 |

## 按 Issue 汇总尚未完成的工作

| 优先级 | Issue / 任务覆盖 | 已有代码证据 | 尚需完成（关闭前必须确认） |
| --- | --- | --- | --- |
| P0 | [#5 macOS 浮层、多屏、输入](https://github.com/wuhunyu/pic-paste/issues/5)：P0-T2/T3/T4、P1-T1 | #12–#15、#20：选显示器、独立 eframe 无边框窗、窗口候选、热键、PID 排除 | AppKit NSPanel 方案决策；macOS 14+ 多屏/负原点/Retina/Spaces/TCC/热键冲突实测；本应用窗口污染及启动延迟验证 |
| P1 | [#6 标注、PNG、剪贴板、贴图](https://github.com/wuhunyu/pic-paste/issues/6)：P1-T2/T3、P2-T1/T2 | #16、#17、#22、#26：PNG-only/Alpha、CJK 标注、多 Pin、行 OCR、长图选区贴图 | 多贴图纹理与资源预算、贴图焦点/拖动/穿透恢复、字体覆盖、粘贴到常见应用的 Alpha/色彩一致性 |
| P1 | [#7 手动长截图](https://github.com/wuhunyu/pic-paste/issues/7)：P4-T1/T2/T3/T4 | #18、#20、#21、#23–#27：实时预览、拼接、手动慢速恢复、固定边距、PNG/标注/圆角、安全中间重拍 | ≥50 个真实网页/文档、可信拼接率 ≥95%、30k/60k/200MP 边界、动态场景与异常/内存/临时文件实测 |
| P1 | [#8 OCR / QR / Add-on](https://github.com/wuhunyu/pic-paste/issues/8)：P3-T1/T2/T3、R-010 | Swift Vision Helper、SHOC IPC、显式行级 OCR 与 Lite 降级 | **词/字符 bbox 与选择、离线二维码 R-010、独立签名 Add-on**、取消/错误生命周期、400 样本 CER 与真机验收 |
| P2 | [#10 真机测试与性能](https://github.com/wuhunyu/pic-paste/issues/10)：P5-T1 | 自动 CI 和外部 CSV 统计脚本存在；无真实设备采集报告 | 30 次冷启动、100 次热键、5 图贴图、CPU/RSS、50 长图、400 OCR、macOS 14+ 多屏/权限/异常矩阵和原始数据 |
| P2 | [#9 正式发布](https://github.com/wuhunyu/pic-paste/issues/9)：P5-T2 | ARM64 Lite/Full 开发 ZIP + SHA256 | 正式 Bundle ID/图标、x86_64、Developer ID、Notarization/Gatekeeper、正式 Release/回滚；依赖真实测试和苹果证书 |
| P3 | [#11 Windows](https://github.com/wuhunyu/pic-paste/issues/11)：P6-T1/T2 | 可复用 Rust 核心 | Windows 捕获/贴图/输入/OCR Provider、安装/代码签名/CI/性能，**macOS 验收后推进** |

## 不依赖真机仍需开发的功能

- [ ] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：截图与贴图按词/字符选择 OCR，提供精确 bbox 与缓存、选择/复制的交互和测试。
- [ ] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：R-010 本地二维码解析（显式触发、离线、旋转/多码/失败输入回归）。
- [ ] [#8](https://github.com/wuhunyu/pic-paste/issues/8)：独立 OCR Add-on 的安装、签名 Team ID、版本兼容和安全降级。
- [ ] [#5](https://github.com/wuhunyu/pic-paste/issues/5)：决定 AppKit NSPanel 专用窗口是否必须实现（目前只有 eframe/winit 独立无边框窗口），再落实缺口。
- [ ] [#6](https://github.com/wuhunyu/pic-paste/issues/6)：多贴图 GPU 纹理预算及资源上限的实际约束与回归。
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

**#8 剩余 OCR/二维码功能 → #5 原生浮层策略及 #6 贴图资源预算 → #10/#7 真实设备与长图验收 → #9 正式发行 → #11 Windows。** [Issue #10](https://github.com/wuhunyu/pic-paste/issues/10) 的测试仪表与数据集整理可与 #8/#5/#6 并行实施，但不能把模拟结果当成设备实测。
