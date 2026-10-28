# Pane 最近提交预览的来源与所有权

## Status

实现已接线；当前验证结论与覆盖边界见 [路线图](../../../../../../ROADMAP.md)。
Kimi 队列失效及恢复边界由 [后续决策](2026-10-28-kimi-tui-queue-placeholder.md)替代。

## Context

[S1 核验](../../../../../../PANE_SIDEBAR_S1.md)确认：按下 Enter、Runtime 接受输入、
生命周期点火与原生用户消息是不同边界。侧栏不能读取 `last_committed` 或通用 hook
`message` 来替代最近提交，否则会显示草稿、助手回答或另一个会话的内容。

## Evidence

- PowerShell 的 `pebrel_input` 携带所属 Shell token 与完整接受文本，原有历史合同会拒绝多行；
  预览可独立消费接受协议，不能为此放宽历史写入。
- [Codex 投影](../../../../../../nebula_app/src/assistant_answer/conversation.rs)已核验原生文件身份，
  只提升真实用户事件。单条超长工具输出可以占满尾页，预览需要有界继续向前读取。
- 本机 Pi 1.3.0 SDK 提供用户角色 `message_start` 与当前分支 `getBranch()`；
  `agent_start` 也会用于重试，不能提供用户正文。原生 JSONL 是分支树，物理尾部不代表所选分支。
- 本机 OpenCode 2.1.2 的嵌入实现声明 `session.inbox.enqueued`，用户条目为
  `item.type=user`、`item.payload.text`，执行开始事件不含用户正文。
  V1 仍使用用户身份消息与文本片段；两条接口不能相互猜测字段。
- 本机 Kimi 2.3.1 的嵌入实现将 hook 字段转为 snake_case，提交正文可以是文本片段数组。
  hook 不携带来源 Agent ID，独立发送也没有跨进程序号。
  原生主 Agent wire 才能核对 `agentId=main`、`origin.kind=user`。
- [Kimi hook 文档](https://moonshotai.github.io/kimi-code/zh/customization/hooks)区分入队和用户回合；
  [Pi 类型](https://raw.githubusercontent.com/earendil-works/pi/v1.3.0/packages/coding-agent/src/core/extensions/types.ts)
  与安装的同版实现共同用于核对回调。OpenCode 的本机 V2 字段没有套用旧版网页推断。

## Decision

- [预览状态](../../../../../../nebula_app/src/gpui_shell/terminal/view/input_preview.rs)归终端实体所有，
  跟随 Pane 移窗；侧栏只读取已准备好的文本。状态不写入会话持久化，不增加独立定时器或依赖。
- 所有权包含 Agent 类型、原生 session ID 和会话 epoch。退出回 Shell、替换身份或关闭实体时，
  旧结果不能发布；同身份读取还要核对预览 revision，避免晚到结果覆盖新提交。
- Shell 仅消费当前 token 的接受文本；CMD 没有同等正文协议时保持占位。
  键盘、剪贴板和 Runtime 输入只有后续得到接受证据才更新；独立 `pane.exec` 不更新。
- Pi 桥发送用户角色消息，在 session switch/fork/tree 时投影 SDK 当前分支，空分支清除旧文案。
  桥级序号与进程身份沿用已有门禁。重复正文的新消息仍是不同提交。
- OpenCode V1 缓存有界文本片段，仅在已确认用户消息身份后发布；V2 消费持久化用户入队记录，
  排除已知子会话和 synthetic 条目。正文与通知字段独立，保留 Pane ID 和内核进程树验证。
- Codex 原生读取复用身份及完整行解析，最多检查 8 页；同轮多页的文件 stamp 必须一致。
  没有完整消息时保留当前身份已确认的预览，不从 cwd、修改时间或屏幕猜测。
- Kimi 使用已绑定的精确 session ID 查询有界索引，核对 `state.json.id`，仅读取该会话
  `agents/main/wire.jsonl` 的真实用户回合。没有可靠活体绑定、索引或受支持根目录时保持占位。
- Kimi 入队后，没有证据将乱序 hook 对齐到最新提交。当前身份的预览因此清空并保持占位，
  直到切换原生会话或重建身份；不能让较早用户回合覆盖较晚排队消息。
  这是明确的未覆盖路径，后续需补有序入队来源；不能据此宣称 Kimi 队列预览完成。
- 后台读取由预览状态持有 Task，身份变化时取消；任务只持 WeakEntity，完成后复查所有权。
  复用既有看门狗并限制读取间隔；渲染不访问原生文件，不同步运行 Agent CLI。
- 展示合并空白、按 UTF-8 边界限制正文，并使用实际两行排版省略；条目高度为 74 logical px，
  预览区域为 32 logical px。没有可靠文本时沿用国际化“新会话”占位。

## Rejected alternatives

- 观察 `last_committed` 或任意 Enter：会把编辑、粘贴和 TUI 确认当作提交。
- 用通知正文或最后助手回答：角色与提交语义不成立。
- 按 cwd 或最近文件猜原生会话：同目录并行会话会串项。
- 读取 Pi 文件最后一条用户消息：可能属于未选中的分支。
- 直接使用 Kimi hook 的正文：无法排除子 Agent，排队场景还可能乱序覆盖。
- 为侧栏再次实现 Codex 用户事件解析或放宽共享历史合同：会产生两个事实源。

## Consequences

可靠性优先于填满预览。缺少安装桥、原生会话绑定、CMD 接受正文及 Kimi 队列顺序的路径会占位，
不会回退到其他会话或键盘镜像。S4 的完成状态仍须根据阶段验收决定；S3 用户人工验收不代表
新的采集路径已完成人工验证。未修改用户全局 Agent 配置。

## Validation

直接运行实际桥接源与 Rust 产品模块，覆盖用户角色、重复消息、分支切换、身份替换、无效 token、
多行与空输入、超长尾页、部分 JSONL 行、Kimi 主 Agent 过滤及队列占位。
GPUI 控件回归核对两行预览边界、滚动后导航和单 Pane 关闭；原生截图与人工验收单独记录。

## Supersedes

None。

## Revisit when

Kimi 提供可绑定主 Agent 的有序入队流、其他 Shell 提供完整接受正文、Agent SDK 改变事件合同，
或项目侧栏扩展到其他平台时，重新核对来源与生命周期。
