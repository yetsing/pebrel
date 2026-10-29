# Codex shared daemon 的 Pane 归属

## Status

Implemented；S6 的产品及便携包验收单独记录，不以本记录代替验收。

## Context

Codex 0.165.0 默认共享 daemon 脱离 Pane Shell 的进程树。daemon 持有原生
rollout，环境变量和 Hook 又可能沿用首次启动客户端的值。单纯寻找 Shell 后代
持有的 rollout 无法覆盖该路径；把 daemon 纳入任意 Pane 则可能串会话。
用户明确要求支持共享 daemon，不能以缺少可靠身份的占位例外结束 S6。

## Evidence

- 官方 `rust-v0.165.0` 源码提交
  [360319d8](https://github.com/openai/codex/tree/360319d8dee62d87e2d59fc0c21805d79519cfed)
  的 `tui/src/session_log.rs` 提供每个 TUI 进程的原生录制接口；
  `UserTurn.client_user_message_id` 是提交意图，不代表服务端已经接受。
- 本机默认 daemon 实测：原生 `thread_history_1.sqlite` 的 `userMessage.clientId`
  与该 UUID 一致；`state_5.sqlite` 提供相应线程的 rollout 路径。
- 从当前 Codex 环境启动的真实 TUI 会继承 VS Code originator，原生主会话
  实测为 `source=vscode, thread_source=user`。因此共享头校验接受 cli/vscode，
  仍拒绝 subagent/guardian 等来源；source 不能代替句柄和接受 UUID 的归属证明。
- 原生 capability 标记可以在退出后保留，不能作为活跃 Pane 的所有权证据。
- 官方接口使用普通文件的 create/truncate。Rust Windows 实现及本机 native
  probe 均确认命名管道不能承担这个接口，打开返回错误 87。
- 代码与失败路径见 [capture](../../../../../../nebula_app/src/platform/ai_session_identity/windows/tui_capture.rs)
  和 [原生测试](../../../../../../nebula_app/src/platform/ai_session_identity/windows/tui_capture_tests.rs)。

## Decision

在 Windows 本地 PTY 创建时准备独立临时文件，并只在没有显式用户录制设置时
注入原生录制环境变量。文件原子创建为 owner/SYSTEM 可访问的保护 DACL，
不继承句柄，使用 kernel delete-on-close；不新增持久化设置或独立服务。

复用现有有界探测 helper，仅检查已验证 Shell 后代中 Codex 实际持有的文件
句柄。主进程要求 capture 的 File ID 与唯一的 PID/创建时间匹配，再从同一
进程持有的 SQLite 句柄确定唯一的原生数据目录。不能改用应用自身的 CODEX_HOME。

消费录制时仅保留合法提交 UUID 与原生导航重置事件；正文不作为预览来源。
按 UUID 精确关联服务端原生接受记录，再复用 rollout 文件名和主会话头校验。
同一 UUID 对应多个线程、缺少记录、子 Agent 或身份失配均清除 live 绑定。
保存的恢复目标不因此抹除；预览必须重新获得当前 live 身份。

每次最多读取 256 KiB，每行至多 128 KiB，积压超过 4 MiB 时丢弃并清零。
已消费范围通过稀疏清零释放正文，不移动或截断生产者的文件游标。SQLite 查询
至多考察最近 4096 项，每项 JSON 长度至多 128K 字符，busy timeout 为 50 ms，执行期限
为 1 秒。所有磁盘工作仍在原后台任务中执行，UI 只接收结果。

同线程的新原生提交 UUID 也推进预览 revision，使早先异步读取失效；重复
轮询同一个 UUID 保持幂等。Pane、进程和会话切换继续受现有 epoch 约束。

## Rejected alternatives

- 放宽 Hook 的进程祖先校验，或使用 daemon 的环境变量：不能证明当前客户端。
- 使用最近文件、文本相似度、时间邻近或持久 capability 标记：都可能串 Pane。
- 直接展示 TUI 的提交意图：服务端可能拒绝，队列顺序也不能据此证明。
- 代理 daemon、修改全局 Codex 配置或强制关闭 daemon：扩大运行和配置边界。
- 以命名管道代替原生录制文件：与已核验的文件打开合同不兼容。
- 始终优先旧 rollout 探测：daemon 偶尔仍是某个 Shell 后代，可能误绑其他客户端。

## Consequences

没有新增依赖、后台线程、公共 API 或持久化字段；启用已有 rusqlite hooks
以限制查询期限。原生录制短暂写入私有临时文件，消费后清零；应用异常退出也
由内核在最后一个句柄关闭时删除。临时盘不支持稀疏操作时不启用该来源。

显式录制设置归用户所有，Pebrel 不覆盖或清理该文件。原生格式升级、超限或
不能证明归属时继续保守占位。空白会话及未出现可关联提交的会话不猜测线程。

## Validation

针对性测试覆盖精确接受 ID、主会话头、复制 ID 歧义、分块/超长记录、导航
重置、真实 sparse zero/delete-on-close、错误 File ID、多个所有者和原生
helper 进程树；helper 测试同时检查写入方游标不变及关闭后清除绑定。
GPUI 回归覆盖原生 Agent 出现时隐藏 Shell 启动命令，以及同线程新提交令旧读取失效。
最终实机、主题和便携包结果记录在 S6 验收报告中。

## Supersedes

None。扩展 [探测进程生命周期](2026-10-23-reusable-probe-process.md)，保留其隔离和期限合同。

## Revisit when

Codex 提供稳定、带客户端身份的接受/会话事件接口，或其原生 TUI、数据库格式
改变。届时替换窄适配器，不以推测匹配维持兼容。
