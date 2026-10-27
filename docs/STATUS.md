# PicPaste 开发与验收状态

> **记录时点：2026-10-27。** 此文件以 [`main`](https://github.com/wuhunyu/pic-paste/tree/main) 中已合并的代码为依据；[REMAINING.md](REMAINING.md) 汇总当前七个未关闭 Issue，原始 v1.2 执行计划的 20 个任务逐项记录在 [TASKS.md](TASKS.md)。**代码完成、自动化测试成功、真机验收通过是三个不同的状态。**

## 当前基线

- 已合并：PR [#1](https://github.com/wuhunyu/pic-paste/pull/1)、[#2](https://github.com/wuhunyu/pic-paste/pull/2)、[#3](https://github.com/wuhunyu/pic-paste/pull/3)、[#12](https://github.com/wuhunyu/pic-paste/pull/12)–[#29](https://github.com/wuhunyu/pic-paste/pull/29)。截至本次核验共 **21 个已合并 PR**。
- 最新功能合并：[#29 可信独立 OCR Add-on](https://github.com/wuhunyu/pic-paste/pull/29)，提交 [`3ec7a56`](https://github.com/wuhunyu/pic-paste/commit/3ec7a56092b7ffb43461b5747b16935c3c833ec0)，完整 [PR CI #40456135637](https://github.com/wuhunyu/pic-paste/actions/runs/40456135637) **success**（Linux/macOS、Swift IPC、Lite/Full、独立开发 ZIP 及 ad-hoc 拒绝测试）。#28 OCR/QR 与 #27 长图功能也已合并。**Developer ID 实际安装、公证和真机验收仍未完成。**
- #17 合并提交 `7e492a4bb91f6062c225dfb33f524cc51756e556`；[main CI #40297130672](https://github.com/wuhunyu/pic-paste/actions/runs/40297130672) **success**。
- 历史增量：[#23 手动失败恢复](https://github.com/wuhunyu/pic-paste/pull/23)、[#24 手动固定栏排除](https://github.com/wuhunyu/pic-paste/pull/24)、[#25 长图圆角](https://github.com/wuhunyu/pic-paste/pull/25)、[#26 长图标注/贴图](https://github.com/wuhunyu/pic-paste/pull/26) 均已合并；对应测试记录详见各 PR。合并后的主分支 CI 已在 #40440845772 覆盖这些功能。
- 跟踪 Issues：[#5 原生窗口和多屏](https://github.com/wuhunyu/pic-paste/issues/5)、[#6 标注与贴图](https://github.com/wuhunyu/pic-paste/issues/6)、[#7 长截图](https://github.com/wuhunyu/pic-paste/issues/7)、[#8 OCR/二维码/Add-on](https://github.com/wuhunyu/pic-paste/issues/8)、[#10 真机与性能](https://github.com/wuhunyu/pic-paste/issues/10)、[#9 macOS 正式发布](https://github.com/wuhunyu/pic-paste/issues/9)、[#11 Windows](https://github.com/wuhunyu/pic-paste/issues/11)。[#4](https://github.com/wuhunyu/pic-paste/issues/4) 已关闭。
- **GitHub Issues：7 个未关闭（#5–#11）。原计划 20 项：8 项代码基本完成、9 项部分实现、3 项未开始（按任务项计数，不是工时或验收完成率）。** 详细剩余任务见 [REMAINING.md](REMAINING.md)。
- **GitHub Releases：0。** 目前只有 ARM64 Lite/Full ad-hoc 签名开发 ZIP（#29 另提供独立 Add-on ad-hoc 开发 ZIP），非正式发行件；ad-hoc Add-on 被运行时主动拒绝。

## 功能现状（截至本次基线）

| 工作流 | 已实现 / 已通过 CI 的代码 | 尚缺失或未验收 |
| --- | --- | --- |
| P0 macOS 截图 | 原生 ScreenCaptureKit 选显示器；负坐标/缩放映射测试；独立无边框 eframe/winit 选区视口；窗口候选单击选区；9×9 像素放大镜/HEX；菜单、可配置 Ctrl+Option+1…9 热键与失败回滚 | 不是专用 AppKit NSPanel；真实多屏/Spaces/不同 TCC 权限、热键冲突、启动/延迟没有验收；[#20](https://github.com/wuhunyu/pic-paste/pull/20) 已在代码层按当前 PID 排除本应用浮窗；真实系统完整排除情况待验收 |
| P1 标注与 PNG | 线、箭头、画笔、矩形、椭圆、马赛克、模糊、**中英文离线文字**（[#17](https://github.com/wuhunyu/pic-paste/pull/17)）；Undo/Redo；PNG-only 无覆盖原子保存、透明圆角、限定大小的 PNG 缩略图、⌘S 保存（[#16](https://github.com/wuhunyu/pic-paste/pull/16)） | 文字字体回退与跨版本系统覆盖、Native 剪贴板粘贴/色彩/Alpha 真机验证、工具边界压力测试 |
| P2 多贴图 | 多窗口置顶、缩放、透明度、鼠标穿透和恢复；[#22](https://github.com/wuhunyu/pic-paste/pull/22) 行级、[#28](https://github.com/wuhunyu/pic-paste/pull/28) 词/字符/二维码识别和复制 | GPU 纹理与 5 张贴图帧延迟实测、焦点恢复/拖动及跨 Spaces 真机验收 |
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
