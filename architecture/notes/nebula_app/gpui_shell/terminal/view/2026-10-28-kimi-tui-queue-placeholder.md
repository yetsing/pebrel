# Kimi TUI 队列的预览失效边界

## Status

用户已明确允许 Kimi 队列占位，保留普通提交预览；验证结果见 [路线图](../../../../../../ROADMAP.md)。

## Context

普通用户消息可以按精确会话读取主 Agent wire，但排队消息尚未进入该记录。
仍展示旧消息会把较早回合冒充为最新提交；直接抄终端队列正文则没有可靠来源合同。

## Evidence

- 隔离产品运行本机 Kimi 2.3.3，接入本地模拟模型并保持当前响应未结束。
  两条排队消息已由原生 TUI 显示，但既无 `UserPromptQueued` Hook，也无主 wire 用户记录。
  添加独立的只读 Hook 采样仍只收到当前回合的 SessionStart、UserPromptSubmit 和 TurnStarted。
- 已安装可执行文件内的 `QueuePaneComponent` 使用固定队列行与 `↑ to edit` 页脚。
  这些屏幕结构只能作为失效信号，不能成为正文或完成状态的来源。
- 同一可执行文件包含服务器模式的 SessionEventJournal，带 epoch 和单调序号；
  本机 TUI 实测不生成 `server/events/<session>.jsonl`，因此该来源不适用于本阶段。
- OpenConsole 的便携包运行库参与真实验证。缺失运行库时，系统 ConPTY 会吞掉测试中的
  CSI-u 按键；不能把这条输入失败归因于预览投影。

## Decision

[预览所有者](../../../../../../nebula_app/src/gpui_shell/terminal/view/input_preview.rs)在已绑定
Kimi 的当前可见屏幕观察到队列结构时，只清空正文、取消旧读取并递增 revision。
已有入队 Hook 也沿用同一保守占位。屏幕文字永远不进入提交预览。

缺少页脚不代表完成。只有原生完成状态、可见编辑器和持续两秒无队列结构共同成立，
才允许重新读取该会话主 wire。新预览仍完全来自原生用户记录。
占位及恢复不修改 Agent 生命周期、通知或任务状态，不增加定时器、依赖或持久化字段。

## Rejected alternatives

- 接入服务器日志并宣称本机队列已支持：本机 TUI 没有该日志。
- 按 prompt ULID 排序：同毫秒随机部分不能提供严格顺序，且 TUI 队列尚无对应原生记录。
- 读取工作目录 user-history：没有会话身份，同目录并行会串项。
- 用队列屏幕正文填预览：文本可能截断，也没有用户消息的来源与角色合同。
- 页脚一消失立即恢复：重绘、对话框或开始处理较早排队项都可能暂时隐藏它。

## Consequences

Kimi 队列是用户认可的明确降级路径，不能宣称显示最新排队消息。
屏幕结构变化或极窄视口可能导致无法识别失效信号，需随 Kimi 升级重新核对。
普通提交、队列结束后的恢复继续依赖精确绑定的主 Agent 用户记录。

## Validation

针对真实结构补充回归：入队清空、草稿不提升、缺少页脚不能结束运行状态，
稳定原生完成后仅解除读取限制，旧正文不得自动恢复。真实 CLI 的隔离验收另行记录。

## Supersedes

仅替代 [原预览决策](2026-10-28-submitted-preview-ownership.md)中“Kimi 入队后直到替换身份
才恢复”的一项。其他来源、所有权及角色限制继续有效。

## Revisit when

Kimi TUI 提供带根 Agent 身份与有序接收凭据的提交事件，或队列结构、原生记录格式改变时。
