# PicPaste 未完成任务与 Issue 对账台账

> **核对日期：2026-10-27** · 主线基线 [`main@0657fd5`](https://github.com/wuhunyu/pic-paste/commit/0657fd52dcb3f1250a2a5db8aae2ff63993ecd5e)，[PR #30](https://github.com/wuhunyu/pic-paste/pull/30) 已合并 · 完整 [PR CI #40458668058](https://github.com/wuhunyu/pic-paste/actions/runs/40458668058) 成功。独立 Add-on 的真实签名和多贴图真实性能仍需设备验收。
>
> 此文档与 [TASKS.md](TASKS.md)、[STATUS.md](STATUS.md) 和 [Issues](https://github.com/wuhunyu/pic-paste/issues?q=is%3Aissue+is%3Aopen) 同步。以下任务状态指 **代码与证据**，不能将单元测试、编译成功等同于操作系统真机功能验收。独立的当前 Issue 正文是每项工作的逐条关闭清单。

## 进度快照

| 口径 | 数量 / 状态 |
| --- | --- |
| 原始 v1.2 任务 | 20 项：**8 项代码基本完成、9 项部分实现、3 项未开始** |
| GitHub Issues（非 PR） | **7 个未关闭：#5–#11**；最初 workspace/CI 对应的 #4 已关闭 |
| GitHub Pull Requests | **22 个已合并**（#1–#3、#12–#30），当前无待合并功能 PR |
| macOS 开发包 | ARM64 Lite/Full ad-hoc 签名 ZIP、SHA256；**不是正式发行件** |
| GitHub Releases | **0** |
| Windows 原生能力 | 未开始；Rust 核心接口存在，不代表 Windows 构建可用 |

## 按 Issue 汇总尚未完成的工作

| 优先级 | Issue / 任务覆盖 | 已有代码证据 | 尚需完成（关闭前必须确认） |
| --- | --- | --- | --- |
| P0 | [#5 macOS 浮层、多屏、输入](https://github.com/wuhunyu/pic-paste/issues/5)：P0-T2/T3/T4、P1-T1 | #12–#15、#20 与 [#31](https://github.com/wuhunyu/pic-paste/pull/31)：屏幕选择、eframe 浮层、PID 排除、局部物理像素归一化、校验裁剪及焦点策略 | [AppKit NSPanel 评估](macos-window-strategy.md)（仍未实现）、macOS 14+ 负原点混合 DPI/Spaces/TCC/热键/焦点真机验收、本应用窗口污染及启动延迟 |
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
- [ ] [#5](https://github.com/wuhunyu/pic-paste/issues/5)：[窗口策略](macos-window-strategy.md)已明确选择先保留 eframe/winit 的临时实现以继续真机验证；**不等同于原定 NSPanel 要求被撤销**。真正 NSPanel 的 Metal/事件桥接与真机决定仍是未完成项；#31 已开始局部物理像素/焦点加固。
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
