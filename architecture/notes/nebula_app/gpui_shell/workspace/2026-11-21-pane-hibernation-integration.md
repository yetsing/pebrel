# Pane 项目归属与标签休眠集成

## Status

Implemented on `feature/moni-one` while integrating upstream v2.6.1.

## Context

上游休眠会将单 Pane 终端替换为没有活体 Pane 的 `SleepingTab`，唤醒时重新创建终端。
本分支的 Windows 侧栏按活体 Pane 分组，项目归属保存在 `TerminalPane` 中。
休眠标签仍属于终端类型，但没有可供侧栏渲染和导航的活体 Pane。

## Evidence

- 初次产品编译在 `hibernation.rs` 的唤醒构造处报告缺少 `project` 字段。
- 仅补齐构造字段仍不足：侧栏原先排除所有终端类型，休眠标签会丢失导航入口。
- 最初把真实 CMD 唤醒放入 `#[gpui::test]`，单独运行可以通过，但完整测试组稳定触发
  PTY reader 跨线程唤醒确定性调度器的断言，随后进程异常退出。
- 改为原生事件循环后，原生回归全部行为断言通过，普通功能组 367 项通过。

## Decision

- `SleepingTab` 在内存中保留原 Pane 的 `PaneProject`；恢复原项目，不用当前 cwd 重算归属。
- 替换休眠和唤醒状态后刷新活体 Pane 分组。休眠期间使用独立 Tab 行及暂停图标，
  复用真实 Tab 激活路径完成唤醒，不保留已关闭的 PTY 或伪造活体 Pane。
- 唤醒失败保留休眠标签；成功后恢复项目、名称及名称历史，并显露新 Pane 的侧栏选择。
- 真实 PTY 回归使用 `gpui_platform::application()` 的原生事件循环，在独立进程、
  独立配置及项目目录中显式执行。

## Rejected alternatives

- 仅把新字段设为 `None`：会丢失固定项目与当前 cwd 不同的归属。
- 把休眠标签作为活体 Pane 留在分组：关闭的 Pane ID 没有可交互的终端实体。
- 在确定性 GPUI 测试中延长等待：无法改变真实 PTY 的线程归属，不能保证退出时序。
- 删除唤醒断言或只忽略测试：不能验证本次新增路径。

## Consequences

没有修改持久化 schema、跨窗口协议或终端核心。分组仍只投影活体 Pane；最后一个 Pane
休眠时分组会消失，唤醒后按现有分组规则重新建立，窗口内折叠和置顶状态不作持久化保证。
普通测试组会忽略需要桌面的原生回归，同步手册要求另行运行并核对回执。

## Validation

[原生回归](../../../../../nebula_app/src/gpui_shell/workspace/hibernation/tests.rs)覆盖启动失败、
实际侧栏点击、真实 CMD PTY、冻结项目与不同 cwd、名称恢复以及关闭清理。
执行入口和隔离要求见[同步手册](../../../../../docs/upstream-release-validation.md)。
本轮四组原生侧栏、休眠原生回归、功能测试和终端回归通过；不代表真实 AI CLI 接入验收。

## Supersedes

None.

## Revisit when

休眠 Pane 需要继续显示在项目组内，或要求跨休眠保留窗口内项目折叠、置顶状态时，
先定义非活体条目的身份与生命周期，再扩展侧栏投影合同。
