# 原始 v1.2 执行计划：20 项任务同步台账

更新于 **2026-10-26**，对应 [STATUS.md](STATUS.md) 与 [GitHub Issues](https://github.com/wuhunyu/pic-paste/issues)。`代码基本完成` 指核心实现与自动化测试存在，**不表示功能已通过真实 macOS/Windows 验收**；`部分实现` 指原任务至少有一项实质性缺口；`未开始` 指该目标尚无功能代码。

| 序号 | 原计划任务 | 当前状态 | 可核验代码 / 剩余缺口 | 跟踪 |
| --- | --- | --- | --- | --- |
| P0-T1 | Rust Cargo workspace 与 CI | **代码基本完成** | `Cargo.lock`、macOS/Linux fmt/test/Clippy/Release、Swift IPC、Lite/Full 构建；缺真机基准 | [#5](https://github.com/wuhunyu/pic-paste/issues/5)、[#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P0-T2 | AppKit 菜单栏、全局快捷键 | **部分实现** | 菜单栏、四功能快捷键、Ctrl+Option+1–9 自定义及失败回滚（PR #14）；真机冲突/恢复/权限未验收 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P0-T3 | ScreenCaptureKit、多屏与选区浮层 | **部分实现** | 选显示器、窗口候选、八个手柄、放大镜；独立 eframe 原生窗（#12/#13/#15）**不是 AppKit NSPanel**，需混合 DPI/Spaces 设备验证 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P0-T4 | 原生贴图、鼠标穿透与性能 | **部分实现** | 多贴图/穿透/恢复有 UI；真实跨 Spaces、交互和性能不达验收证据标准 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P1-T1 | 选区、窗口识别、取色/放大镜 | **代码基本完成** | 窗口点击选择 #15、9×9 放大镜/HEX；真实重叠窗口和负坐标验证未完成 | [#5](https://github.com/wuhunyu/pic-paste/issues/5) |
| P1-T2 | 标注、撤销重做 | **代码基本完成** | 线/箭头/画笔/几何/模糊/马赛克、文字 #17、Undo/Redo；CJK 系统字体实际覆盖需验证 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P1-T3 | PNG-only、透明圆角、剪贴板 | **代码基本完成** | PNG-only 原子保存、⌘S、圆角预览 #16；原生应用剪贴板/色彩/Alpha 未验收 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P2-T1 | 多贴图缩放、透明度与资源 | **部分实现** | Pin 视口/缩放/透明度；纹理预算、五图基准缺失 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P2-T2 | 贴图模式状态机/异常恢复 | **部分实现** | 穿透恢复和 #22 贴图行级选字/复制及移动/选字/穿透互斥代码已实现；仍需真机拖动/焦点/重启恢复验收 | [#6](https://github.com/wuhunyu/pic-paste/issues/6) |
| P3-T1 | OCR helper、SHOC、生命周期 | **代码基本完成** | 本机 Swift Vision helper、IPC、超时/退出/签名校验；外置 Add-on 安全还未提供 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P3-T2 | OCR 行/词选择与 400 图测试 | **部分实现** | 截图与贴图行级选择/复制 (#22)；仍无词/字符 bbox 和 400 图 CER 证据 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P3-T3 | Lite / Full / Add-on 交付 | **部分实现** | Lite/Full 开发包自动打包；独立签名 Add-on 安装/版本兼容未实现 | [#8](https://github.com/wuhunyu/pic-paste/issues/8) |
| P4-T1 | 手动纵向/横向滚动及 ROI | **代码基本完成** | 手动区域采样与纵横方向；真实网页/文档和鼠标穿透未验收 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T2 | 重叠拼接、磁盘块与实时预览 | **代码基本完成** | 匹配拒绝/TileStore/实时小窗 #18；[#20](https://github.com/wuhunyu/pic-paste/pull/20) 已按 PID 排除自身窗口；复杂页面与真实无残影效果仍待实测 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T3 | 手动滚动失败恢复、拼接纠错、编辑 | **部分实现** | [#23](https://github.com/wuhunyu/pic-paste/pull/23) 失败暂停、人工回滚慢速重试、复用可信锚点；[#21](https://github.com/wuhunyu/pic-paste/pull/21) 可裁剪尾部；**仍缺固定页头遮罩、真正中间段纠错、长图二次标注与 50 场景实测**。自动滚动为用户明确取消的需求，不计为缺项 | [#7](https://github.com/wuhunyu/pic-paste/issues/7) |
| P4-T4 | 行流式 PNG/30k–60k 限制 | **代码基本完成** | 行流式 PNG 和 60k/200MP 上限；极端尺寸/内存压力报告缺失 | [#7](https://github.com/wuhunyu/pic-paste/issues/7)、[#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P5-T1 | 稳定性、质量、性能验收 | **部分实现** | CI 和统计脚本存在；真实设备 P95/CPU/RSS、400 OCR/50 长截图报告尚无 | [#10](https://github.com/wuhunyu/pic-paste/issues/10) |
| P5-T2 | Developer ID、公证、正式发布/回滚 | **未开始正式发布** | ad-hoc 开发 ZIP 有；正式 ID、双架构、Developer ID/notarization/Gatekeeper、回滚未完成；Release 0 | [#9](https://github.com/wuhunyu/pic-paste/issues/9) |
| P6-T1 | Windows 原生功能 | **未开始** | 无 Windows 专属屏幕捕获、窗口、输入或安装包；Rust core 可复用 | [#11](https://github.com/wuhunyu/pic-paste/issues/11) |
| P6-T2 | Windows 离线 OCR/打包 | **未开始** | 无 Windows OCR Provider、安装与分发 | [#11](https://github.com/wuhunyu/pic-paste/issues/11) |

## 单独补录的原始需求

- **R-010：本地手动触发二维码识别**。原任务 20 项的拆分没有列出来，已纳入 [Issue #8](https://github.com/wuhunyu/pic-paste/issues/8)；**未实现**。

## 优先级 / 执行顺序

1. 验证 PR #18 合并后的 `main` CI；修复原生截图视口与长截图小窗潜在互相捕获/遮挡问题；把多屏与系统交互尽可能自动化（[#5](https://github.com/wuhunyu/pic-paste/issues/5)、[#7](https://github.com/wuhunyu/pic-paste/issues/7)）。
2. 补充贴图手势/选择/穿透模式、缓存预算与原生剪贴板验收（[#6](https://github.com/wuhunyu/pic-paste/issues/6)）。
3. 完善**手动**长截图的失败暂停/慢速重试、固定区域过滤、分段纠错和长图闭环（[#7](https://github.com/wuhunyu/pic-paste/issues/7)）；**不实现自动滚动**。
4. 贴图 OCR 词/字符、QR、本地 Add-on 和统计测试（[#8](https://github.com/wuhunyu/pic-paste/issues/8)）。
5. 真实 macOS 14+ 测试矩阵和质量/性能指标（[#10](https://github.com/wuhunyu/pic-paste/issues/10)）；通过后构建正式签名公证包（[#9](https://github.com/wuhunyu/pic-paste/issues/9)）。
6. 最后接入 Windows 截图贴图/OCR（[#11](https://github.com/wuhunyu/pic-paste/issues/11)）。

更新规则：**代码/PR 是否已合并，以 GitHub 当前状态为准；完成代码 ≠ 满足验收。** 所有阶段关闭时必须附 CI run、设备矩阵、可复测指标和最后的发行证据。
