# PicPaste 开发与验收状态

> **记录时点：2026-10-26。** 此文件以 [`main`](https://github.com/wuhunyu/pic-paste/tree/main) 中已合并的代码为依据；原始 v1.2 执行计划的 20 个任务逐项记录在 [TASKS.md](TASKS.md)。**代码完成、自动化测试成功、真机验收通过是三个不同的状态。**

## 当前基线

- 已合并：PR [#1](https://github.com/wuhunyu/pic-paste/pull/1)、[#2](https://github.com/wuhunyu/pic-paste/pull/2)、[#3](https://github.com/wuhunyu/pic-paste/pull/3)、[#12](https://github.com/wuhunyu/pic-paste/pull/12)–[#23](https://github.com/wuhunyu/pic-paste/pull/23)。截至本次核验共 **15 个已合并 PR**。
- 最新功能合并：[#23 手动长截图失败恢复](https://github.com/wuhunyu/pic-paste/pull/23)，合并提交 `32afacba153e51c5a84cccce08860d9ec548d10e`，完整 [CI #40314126081](https://github.com/wuhunyu/pic-paste/actions/runs/40314126081) **success**：Linux core tests/Clippy、macOS workspace tests/Clippy/Release、Swift OCR IPC、Lite/Full 开发包。此前 [#22](https://github.com/wuhunyu/pic-paste/pull/22) 贴图行级 OCR [CI #40309834503](https://github.com/wuhunyu/pic-paste/actions/runs/40309834503) success。
- #17 合并提交 `7e492a4bb91f6062c225dfb33f524cc51756e556`；[main CI #40297130672](https://github.com/wuhunyu/pic-paste/actions/runs/40297130672) **success**。
- #22 合并及文档同步后 [main CI #40311227304](https://github.com/wuhunyu/pic-paste/actions/runs/40311227304) **success**；#23 合并及本次文档同步后的 main CI 仍需核验，不以 PR CI 替代。
- 跟踪 Issues：[#5 原生窗口和多屏](https://github.com/wuhunyu/pic-paste/issues/5)、[#6 标注与贴图](https://github.com/wuhunyu/pic-paste/issues/6)、[#7 长截图](https://github.com/wuhunyu/pic-paste/issues/7)、[#8 OCR/二维码/Add-on](https://github.com/wuhunyu/pic-paste/issues/8)、[#10 真机与性能](https://github.com/wuhunyu/pic-paste/issues/10)、[#9 macOS 正式发布](https://github.com/wuhunyu/pic-paste/issues/9)、[#11 Windows](https://github.com/wuhunyu/pic-paste/issues/11)。[#4](https://github.com/wuhunyu/pic-paste/issues/4) 已关闭。
- **GitHub Releases：0。** 目前只有 ARM64 Lite/Full ad-hoc 签名开发 ZIP，非正式发行件。

## 功能现状（截至本次基线）

| 工作流 | 已实现 / 已通过 CI 的代码 | 尚缺失或未验收 |
| --- | --- | --- |
| P0 macOS 截图 | 原生 ScreenCaptureKit 选显示器；负坐标/缩放映射测试；独立无边框 eframe/winit 选区视口；窗口候选单击选区；9×9 像素放大镜/HEX；菜单、可配置 Ctrl+Option+1…9 热键与失败回滚 | 不是专用 AppKit NSPanel；真实多屏/Spaces/不同 TCC 权限、热键冲突、启动/延迟没有验收；[#20](https://github.com/wuhunyu/pic-paste/pull/20) 已在代码层按当前 PID 排除本应用浮窗；真实系统完整排除情况待验收 |
| P1 标注与 PNG | 线、箭头、画笔、矩形、椭圆、马赛克、模糊、**中英文离线文字**（[#17](https://github.com/wuhunyu/pic-paste/pull/17)）；Undo/Redo；PNG-only 无覆盖原子保存、透明圆角、限定大小的 PNG 缩略图、⌘S 保存（[#16](https://github.com/wuhunyu/pic-paste/pull/16)） | 文字字体回退与跨版本系统覆盖、Native 剪贴板粘贴/色彩/Alpha 真机验证、工具边界压力测试 |
| P2 多贴图 | 多窗口置顶、缩放、透明度、鼠标穿透和一键恢复；[#22](https://github.com/wuhunyu/pic-paste/pull/22) 新增贴图离线 OCR 行级点击与复制、移动/选字/穿透互斥模式 | 词/字符框未提供；GPU 纹理与 5 张贴图帧延迟实测、焦点恢复/拖动及跨 Spaces 真机验收 |
| P3 OCR | Swift Apple Vision 本地 OCR helper（Full）、版本化 SHOC IPC、20s 超时、无 helper 的 Lite 降级、截图与贴图行级 hit test/复制（#22） | 词/字符 bbox、400 张 CER 统计、独立签名 Add-on；**QR 离线识别 R-010 尚未实现** |
| P4 长截图 | 由用户手动纵/横滚动、自动识别可信重叠并拼接；[#23](https://github.com/wuhunyu/pic-paste/pull/23) 连续匹配不可靠时暂停提示，用户手动回滚并在菜单栏继续慢速重试，同一磁盘会话保留可信分段；行流式 PNG、60k/200MP 限制、复核删最后段/保留前 N 段（#21）、实时缩略图（#18） | **自动滚动已从需求移除**；固定页头/动态区域过滤、中间段真正纠错、长图标注/贴图仍缺；50 场景成功率和物理鼠标穿透/无残影/慢速恢复真机验收缺失 |
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
