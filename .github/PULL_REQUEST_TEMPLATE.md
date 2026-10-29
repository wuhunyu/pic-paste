## 变更范围与需求映射
- 关联 Issue：#（填写实际编号）
- 用户可见行为、受影响平台与文件：
- 与现有 PNG-only、离线、macOS 14+、Canary 默认关闭的兼容性：

## 测试证据（不得把 CI 当作真机验证）
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace --locked`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] Linux `core` 与 macOS `rust` Job 对应当前 PR HEAD，均 success
- Actions run URL：
- [ ] 涉及 Swift Vision/原生 IPC 时，运行对应 helper/协议回归
- [ ] 涉及 macOS UI/TCC/IME/多屏/贴图/性能时，记录真机系统版本、设备、提交 SHA 与证据链接；若未执行，请在下方标明 `NOT RUN`，不可勾选

## 边界、错误和回退
- 失败/取消/异常权限与旧进程回复的处理：
- 可复现的回退操作（稳定 eframe、恢复旧版本或 revert）：
- 是否更改打包、签名、公证、用户数据：
- [ ] 未以开发版 ad-hoc 包冒充可正式分发的 Developer ID 版本
- [ ] 未将构建/单元测试推断为真实 Mac GUI 验收
- [ ] 更新对应 README / STATUS / TASKS / REMAINING / Issue（如适用）

## 验收说明
- 已满足的可客观重测条件：
- 未完成/被阻塞条件（具体设备、证书、样本、风险）：
- 审核重点及回归命令：
