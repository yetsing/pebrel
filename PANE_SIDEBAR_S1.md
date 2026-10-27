# Pane 会话侧栏 S1 数据来源核验

核验日期：2026-10-27。源码基线：`665f7bc6`。
状态：S1 数据来源核验已完成，2026-10-27 16:04 收口；S2 至 S6 未开始。
需求入口：[PANE_SIDEBAR_REQUIREMENTS.md](PANE_SIDEBAR_REQUIREMENTS.md)。
自用 Agent 范围：Codex、OpenCode、Pi、Kimi；仅 Windows 本机。

## 当前结论与证据边界

Pane 身份、名称、状态、当前目录和创建路径已有来源；项目固定目录与继承归属
尚无模型。预览不能直接读取 `last_committed` 或 `AiHookEvent.message`。

- PowerShell 已上报完整接受的用户命令，优先复用 `pebrel_input`；现有历史
  接收器会拒绝换行，不能直接用历史状态提供多行预览。
- Codex 已有按会话身份校验的原生用户消息投影，可以复用其读取与解析能力；
  当前没有侧栏所需的最近用户消息缓存和刷新接线。
- OpenCode、Pi 的现有桥不发送用户正文；Kimi 的提交正文未进入标准化事件。
  三者均需在 S4 补齐，当前只能显示占位。

本次完成源码核对、Windows PowerShell 原始函数样本和桥接夹具执行，并在用户
提供正在运行的可执行文件后，通过 Runtime 补充实际 PTY 与四种 Agent 的样本。
用户还亲自完成了 Shell 的真实键盘、系统剪贴板和空输入验证；Kimi 已补齐
未提交粘贴、多行、重复、空输入及后续 Runtime 自动提交样本。
运行实例为 `2.3.1 (7311e07)`，不同于上述工作树基线；实测不代替当前源码构建。
S1 交付的来源清单、项目上下文区分、代表性样本与缺口记录均已具备。
正文采集接线和缺口修复属于后续阶段，S1 完成不代表侧栏预览已经实现。

## Pane 与项目上下文

| 数据 | 当前实际来源 | 后续接入边界或缺口 |
| --- | --- | --- |
| Pane 身份 | [workspace.rs](nebula_app/src/gpui_shell/workspace.rs) 的 `TerminalPane.id` 与 `TerminalView.pane_id` 同源；`next_pane_id` 按窗口分区分配 | 使用 Pane ID 关联条目；Tab 下标仅用于当次导航查找，不能充当身份 |
| Tab 与 Pane 所有权 | 同文件的 `WorkspaceTab::Terminal` 持有 `panes`、`tree`、`focused`；[runtime_tabs.rs](nebula_app/src/gpui_shell/workspace/runtime_tabs.rs) 使用活体 Tab ID | 侧栏建立投影，不另建会话或复制 PTY；增删、移窗和恢复须跟随原所有者 |
| 名称与图标 | `TerminalPane.custom_name`、[pane_header.rs](nebula_app/src/gpui_shell/workspace/pane_header.rs) 的 `pane_title`；自动名依次使用程序、SSH 目标、目录名 | 优先保留 Pane 自定义名；[tab_presentation.rs](nebula_app/src/gpui_shell/workspace/tab_presentation.rs) 目前只读取聚焦 Pane，不能将一份 Tab 展示复制给所有 Pane |
| 状态 | [runtime.rs](nebula_app/src/gpui_shell/terminal/view/runtime.rs) 的 `runtime_task_state`、`sidebar_activity`；Agent 事实经 [agent_activity.rs](nebula_app/src/gpui_shell/terminal/view/agent_activity.rs) 接入共享生命周期 | 复用运行、等待输入、attention、完成、故障及命令失败的既有映射；attention 不总等于授权请求，不能另猜状态 |
| BEL 与选中 | [tab_presentation.rs](nebula_app/src/gpui_shell/workspace/tab_presentation.rs) 的 `has_bell`、[sidebar.rs](nebula_app/src/gpui_shell/workspace/sidebar.rs) 的 `resting_activity` 目前按 Tab 裁定 | 当前项必须同时匹配活动 Tab 和 focused Pane；不能把 Tab 活动或 BEL 当作每个 Pane 的等待状态 |
| 请求启动目录 | [startup.rs](nebula_app/src/gpui_shell/terminal/view/startup.rs) 读取 `TerminalLaunch::Local.cwd`、最终 `options.working_directory`；未指定时 PTY 使用默认启动流程 | 创建时捕获可信启动目录；未知时待归组，首个有效目录固定归属；不能日后从可变 `cwd` 还原启动目录 |
| 当前目录 | `TerminalView.cwd`；[cwd_report.rs](nebula_app/src/gpui_shell/terminal/view/cwd_report.rs) 接收 `CwdReport` 和 `NEBULA\|cwd\|...` 标题 | `cd` 可以更新它，但不能更新项目归属；`local_cwd()` 会做 `is_dir`，不要在渲染路径调用 |
| 固定项目目录、Pane 归属 | 当前 `TerminalPane`、`WorkspaceTab` 无对应字段 | S2 保存独立项目身份与固定目录；创建请求显式或继承归属优先，启动目录仅作无归属时的回退 |
| 本地目录校验 | [launch_directory.rs](nebula_app/src/platform/launch_directory.rs) 的 `validate` 检查目录并转绝对路径 | 它不是分组身份实现；S2 需统一完整路径、分隔符、尾斜杠等比较规则并测试，不能用目录名或 Git 根目录代替所选目录 |
| Git 分支 | [vcs_panel.rs](nebula_app/src/gpui_shell/workspace/vcs_panel.rs) 读取 `SidePanel` 的当前 VCS 根和缓存快照 | 查询固定项目目录；不能无条件沿用活动 Pane 或手动浏览目录的分支；无可靠结果隐藏 |

## 创建与导航入口

| 入口 | 当前行为 | S2、S3 所需接线 |
| --- | --- | --- |
| 全局新建 | `add_terminal` 优先配置启动目录，否则经 [tab_duplication.rs](nebula_app/src/gpui_shell/workspace/tab_duplication.rs) 的 `new_tab_from_focused` 使用聚焦 Pane 的目录 | 创建目录与项目归属分别传递；活动 Pane 有项目时继承归属，即使启动设置指向其他目录 |
| 指定目录新建 | `add_terminal_at` → `shell_launch::configured_at` → `add_terminal_with` → `new_pane` | 可复用创建能力；新增显式项目上下文，并确保固定项目目录覆盖 profile 自带目录 |
| 分屏 | `split_focused` → `copy_launch(CopyKind::Split, PaneOrigin::of(...))` → `new_pane` | 本地 Shell 已读取源 Pane 当前目录；另外传递原 Pane 项目，不能按新 Pane 的启动目录重新分组 |
| 项目头 `＋` | 无项目头入口 | 指定固定项目目录和项目身份，创建一个新 Tab 与单 Pane |
| “打开项目…” | 无此项目会话入口 | 选择本地目录后复用或新建项目，再调用明确目录的创建路径；取消不创建组或会话 |
| 恢复、复制与移窗 | 已有恢复、Tab 复制与移动生命周期，Pane ID 与 Tab 下标分离 | 恢复按恢复后的初始目录分组；逐 Pane 对账，不能遗漏这些间接创建路径；分组不引入新的持久化格式 |
| 点击 Pane | `focus_pane(tab_ix, pane_id)` 只修改指定 Tab 内焦点；跨 Tab 不自动激活 | 先按 Pane ID 查当下所属 Tab，激活它，再聚焦 Pane；单独调用 `focus_pane` 不足以跨 Tab 导航 |

## Shell 已提交命令

### PowerShell 的可靠来源

[windows/mod.rs](nebula_terminal/src/tty/windows/mod.rs) 的
`PSConsoleHostReadLine` 包装器在原生 ReadLine 返回非空完整命令后依次发送：

1. `pebrel_input`：Base64 编码的 `shell_owner + LF + 原始用户命令`。
2. `pebrel_command`：同一 owner 加上用于范围识别的命令，可能展开首词别名。
3. `OSC 133;C`：命令开始边界。

`pebrel_input` 保留多行、粘贴、历史召回和原生补全后的用户文本。
`pebrel_command` 是执行范围元数据，不应代替原始命令预览。
普通编辑、未完成的多行输入和粘贴本身不意味着 ReadLine 已返回。
空行与全空白行不产生上述提交信号。

[event_loop.rs](nebula_terminal/src/event_loop.rs) 将 `UserVar` 与命令边界按序传给
[view.rs](nebula_app/src/gpui_shell/terminal/view.rs)。现有
[completion_context.rs](nebula_app/src/completion_context.rs) 通过已知 shell owner
接收 `pebrel_input` 并去除历史重复记录，但拒绝含控制字符或超过 4096 字节的正文，
因此多行已提交命令会被它丢弃。S4 应复用已接受输入协议，单独维护有界预览投影，
保留 owner 校验；不为预览放宽历史记录合同。

### 较弱路径与回退

- [completion.rs](nebula_app/src/gpui_shell/terminal/view/completion.rs) 的
  `commit_line` 从可识别提示符的网格取输入；Agent/前台程序内的 Enter 会走另一分支。
  [suggest.rs](nebula_app/src/gpui_shell/terminal/suggest.rs) 在网格取值失败时仍可能
  将按键镜像写进 `last_committed`，用途是身份推断，不能证明完整命令已被接受。
- [keymap.rs](nebula_app/src/gpui_shell/terminal/keymap.rs) 保留修饰 Enter 的语义。
  Shift+Enter、续行 Enter 和 TUI 确认键均不能单凭按键认定用户提交了消息。
- `runtime_run`、`runtime_prompt(submit=true)` 在回显屏障完成前即可更新
  `last_committed`；`runtime_paste_inner` 没有同样的正文更新路径。
  所以既不能观察该字段变化判提交，也不能仅挂钩某一个 Runtime 方法。
- `submit=false` 的 Runtime 输入、bracketed paste、Windows 原生剪贴板粘贴
  均要等待后续接受证据；`pane.exec` 是独立非 TTY 执行，不是 Shell 命令行提交。
- [cmd_prompt.rs](nebula_terminal/src/tty/windows/cmd_prompt.rs) 只有提示符边界，
  没有 PowerShell 同等的 accepted-input 正文协议。CMD 的网格提交是较弱路径，
  多行、原生编辑及丢失集成场景未实测时保持占位，不从滚屏或共享历史猜测。
- [reading.rs](nebula_terminal/src/term/reading.rs) 的 `completed_commands` 等到
  `133;D` 才产生记录，不能作为“刚提交就更新”的唯一来源。

## 自用 Agent 的用户消息

| Agent | 已核实来源 | 当前缺口与 S4 方向 |
| --- | --- | --- |
| Codex | [conversation.rs](nebula_app/src/assistant_answer/conversation.rs) 的 `codex` 读取 `event_msg/user_message.message`，或 `event_msg/item_completed` 中 `UserMessage.content`；原始 `response_item` 的 `role=user` 可能含环境和 AGENTS 上下文，现有实现不将其提升为用户消息 | 复用身份校验、完整 JSONL 行和有界读取；只选实际用户消息，不能选助手最后一条、原始上下文或屏幕内容；缺原生路径时占位 |
| OpenCode | [opencode.js](nebula_app/res/hooks/opencode.js) 的 V1 `message.updated` 使用 `info.role=user` 与消息 ID 去重；V2 将 `session.execution.started` 转成同类生命周期信号 | 两条路径都只发 `kind=prompt`，没有正文，V2 执行开始也不能证明新用户消息。须按实际版本接入用户消息及文本内容事件或原生查询，绑定根会话和消息 ID；不得套用 V1 文档猜 V2 字段 |
| Pi | [pi.ts](nebula_app/res/hooks/pi.ts) 从 `agent_start` 发 `kind=prompt`；已提供 session ID、可用时的 session file 和 bridge 序号 | 这个事件不带正文，也可能随重试或继续执行触发。本机 Pi 1.3.0 SDK 提供 `input`、`before_agent_start`、`message_start/end`；应按真实用户角色和会话确认采集，不能把一次 agent_start 当作新用户消息 |
| Kimi | [protocol.rs](nebula_app/src/ai_hook/protocol.rs) 识别 `UserPromptSubmit`，但标准化为 `PromptSubmit, None`；[kimi.rs](nebula_app/src/ai_hook/local/kimi.rs) 安装了该事件 | 当前丢弃提交正文，没有已接入的对话投影。官方文档确认该提交事件；本机 Kimi 2.3.1 的正文形状、队列与会话替换须实样核对，再提取有界用户文本，不复用通知或错误正文 |

四种 Agent 的 `PromptSubmit` 当前均不暴露用户正文。
[event.rs](nebula_app/src/ai_hook/event.rs) 的 `message` 是通用人类可读文本：
可能是 Codex 最后助手回答、授权问题、通知、错误或桥接完成消息。
该字段存在、状态变忙、收到 Enter，都不足以证明最近用户消息发生变化。

Codex 的现有读取会验证首条 `session_meta.payload.id`，只解析完整换行的 JSONL
记录，单页读取 1 MiB、消息数量 160、单消息 48 KiB、返回正文合计 128 KiB。
如果最新用户消息被大量工具输出挤出尾页，不能误取上一会话或转而猜测；S4 需要
有界继续查找或保留当前身份已经确认的最近提交，无法确认则占位。

Pi 的 `input` 发生在处理前，后续扩展可以 transform 或 handled；
`before_agent_start` 还区分 user/custom。仅挂其中一个回调不自动满足所有提交路径。
本机 SDK 的 `message_start` 会先转发给扩展，`message_end` 后写入原生会话。
文件是带 `parentId` 的分支树，不能把 JSONL 物理末尾当成当前分支最新用户消息。

外部一手资料核对于 2026-10-27：

- [Pi 1.3.0 扩展类型](https://raw.githubusercontent.com/earendil-works/pi/v1.3.0/packages/coding-agent/src/core/extensions/types.ts)：确认输入、消息角色及 user/custom 回调的类型合同；本机安装的同版类型与实现亦已读取。
- [OpenCode 插件文档](https://dev.opencode.ai/docs/plugins/)：确认旧式消息与文本片段事件可用；该页的插件接口不能直接证明本机 2.1.2 的 V2 正文合同。
- [Kimi hook 文档](https://moonshotai.github.io/kimi-code/zh/customization/hooks)：确认提交、排队与回合来源事件的区别；未据此认定 2.3.1 实际正文是字符串。对应版本正文载荷仍待实样核验。

## 样本与已执行结果

本节输入为固定夹具，不读取用户的会话原文、不修改全局 Agent 配置，不调用模型。
临时探针位于 `.tmp/pane-sidebar-s1/`，只保留本次复核所需脚本；正式实现不依赖它。

| 样本 | 执行对象 | 实际结果 |
| --- | --- | --- |
| 空内容、全空白 | 从产品源码提取原样 `PSConsoleHostReadLine`，注入原生 ReadLine 返回值 | 各 0 个 input report、0 个命令开始信号 |
| 单行命令 | 同上 | 1 个 input report、1 个开始信号；用户原文不变 |
| 完整多行 `if`、中文与 emoji | 同上 | 1 次提交，换行与完整原文保留；不是逐行产生提交 |
| 连续提交相同命令两次 | 同上 | 两次分别产生提交，不能按正文去重 |
| `gci .` 别名 | 同上 | `pebrel_input` 保留 `gci .` 原文 |
| 仅文本片段更新的反例 | 执行原样 OpenCode V1 桥，注入 `message.part.updated` 夹具 | 不产生 prompt；这只是片段夹具，不是已验证真实编辑器草稿事件 |
| 用户消息、同 ID 重复事件、相同内容新 ID | 执行原样 OpenCode V1 桥 | 分别新增 1、0、1 个 prompt；全部未发送用户正文 |
| Pi `agent_start` | 执行原样 Pi 桥，模拟 1.3.0 SDK | 产生 prompt，但无用户正文；未注册 input、before_agent_start 或 message 回调 |
| OpenCode V1/V2、Pi 重试及生命周期回归 | 下列现有 Node 测试 | 40 项通过，0 失败、0 跳过；不包含四种真实 CLI 的输入采集验收 |

```powershell
node --test scripts/tests/opencode-notification-bridge.test.mjs scripts/tests/pi-notification-bridge.test.mjs scripts/tests/test_pi_bridge.mjs
```

Rust 原生对话测试尝试执行：

```powershell
cargo test --offline -p nebula --bin pebrel --locked assistant_answer::conversation::tests -- --nocapture
```

未进入编译：本机缓存缺少 `Kuddev/zed` 的 GPUI revision
`1e716d16ec54b361499931b4278903cdcaae8949`；离线 checkout 失败。
此前在线尝试进入该仓库更新后已中止，没有 Rust 测试通过结论。
Codex 的既有完整行、上下文过滤、多行正文、重复消息与 epoch 测试仅做源码核对。

2026-10-27 15:09 基础检查：Python 3.12.13 执行
`scripts/check_architecture.py`，1156 个源文件、10 个 crate 检查通过。
`cargo fmt --all -- --check` 失败，现有 Rust 文件出现 `Incorrect newline style`；
抽查 `git ls-files --eol` 确认 index 为 LF、工作树为 CRLF，本次没有改动 Rust 文件。
未为消除检查失败而重写现有源码换行。完整产品构建及文件行数 Rust 测试未执行。

## 运行中实例补充核验

2026-10-27 15:48 记录。用户指定运行实例后，执行其 `env --pretty`，确认
Runtime 可达。Windows GUI 子系统的 CLI 输出通过 PTY 取得；没有扫描运行端口、
控制用户原有 Agent 或调用 UI 自动化。以下“按键”均指 Runtime 编码的终端按键，
不是人工物理键盘验收。

测试新建 Pane 3 至 8；原有 Pane 1、2 保留。测试目录为
`.tmp/pane-sidebar-s1/project-a/` 与 `project-b/`。PowerShell 仅在 Pane 3
包装既有 ReadLine 函数，记录完整接受的返回值，不修改全局 Shell 配置。
原样安装函数确认包含 `pebrel_input`、`pebrel_command` 和 `133;C` 协议。

| 对象与操作 | 原生记录及实际结果 |
| --- | --- |
| PowerShell 输入但不提交 | `prompt --no-submit` 后 ReadLine 日志仍只有安装元数据；Pane 维持 idle |
| PowerShell Shift+Enter 换行 | 未产生接受记录；补第二行后发普通 Enter，才产生一条保留 LF 的完整命令 |
| PowerShell 空 Enter | ReadLine 返回空串，记录 `submitted=false`；不能当作新命令 |
| Runtime 连续两次相同命令 | 各有一条接受记录和独立 run ID，均返回真实退出码 0 |
| Runtime 失败命令 | `cmd /c exit 7` 返回 `state=failed`、`exit_code=7` |
| PowerShell 多行 paste、不提交 | 返回 `unsafe_input_mode`，没有新增接受记录；属于明确拒绝，不能记为粘贴成功 |
| `cd` 后分屏 | Pane 3 从 project-a 改到 project-b；新 Pane 4 使用 project-b，同 Tab 的 Pane 3、4 保持独立 ID。项目固定归属尚无模型，本次没有验证归属继承 |
| Codex 0.164.0 | 原生 rollout 中 `event_msg/item_completed/UserMessage` 累计 3 条；不提交的两行粘贴未增加条数，Enter 后新增并保留 LF；重复同一多行文本产生两条独立 ID；空 Enter 后仍为 3 条 |
| Pi 1.3.0 | 原生 JSONL 的 `type=message`、`message.role=user` 累计 3 条；未提交粘贴、提交后的 LF、重复消息独立记录和空 Enter 均与 Codex 样本一致 |
| OpenCode 2.1.2 | 项目限定的 `session list --format json` 取得测试会话，`session export <id>` 中 `messages[type=user].text` 累计 2 条，均含 LF；第二次未提交粘贴时仍为 1 条，Enter 后为 2 条，空 Enter 后仍为 2 条 |
| Kimi 2.3.1 | 用户授权后信任测试目录。首条测试消息提交并完成；原生 `agents/main/wire.jsonl` 出现 `turn.prompt`，`agentId=main`、`origin.kind=user`、`input=[{type:text,text:...}]`；`state.json.lastPrompt` 与之相同 |

实际数据来源的新增结论：

- **API 接受输入不等于编辑器已提交。** OpenCode 初始 orchestration 返回
  `submitted=true`，但文本仍在编辑器；后来的未提交粘贴与它拼接，直到显式
  Enter 才成为一条用户消息。Kimi 同样在 `agent-prompt` 返回成功后仍显示草稿，
  原生会话尚未建立；显式 Enter 后才有用户消息与会话记录。不得把上述 API
  回执直接接成侧栏“最近提交”。本次没有定位或修改这两个提交边界问题。
- Codex、OpenCode 测试 Pane 的 Runtime `session_id=null`、`hook_seen=false`；
  原生记录虽存在，当前产品不能据此按 Pane 可靠路由。现场读取只限定本次唯一
  测试目录及已核对的会话，**不能把这种人工定位办法用于产品按 cwd 猜会话**。
- Pi 的 Runtime session ID 与原生首条 session header 一致，且 `hook_seen=true`；
  但既有 Pi 桥仍不发送正文，读取用户消息还需要新的投影接线。
- OpenCode 2.1.2 的原生导出已核实，用户消息使用 `type=user` 与 `text`；
  导出中的 assistant、idle 不属于用户消息。导出能力可作为后续候选，仍需
  可靠 Pane 会话绑定及有界后台查询，不能在渲染时调用 CLI。
- Kimi 的 `turn.prompt.input` 数组是本次原生 wire 事实，**不是已捕获的 hook
  `prompt` 载荷合同**。session index 提供 session ID、session dir 与 work dir；
  目录含多个会话时仍须可靠绑定当前 Pane，不能挑最近文件。`lastPrompt` 是
  无消息 ID 的便捷快照，不能单独证明重复发送次数。
- Kimi 管理的 `agent-31088-4/generation=1` 返回 `agent_exited`，但原 Pane
  仍显示相同 Kimi 会话及已完成回答，`pane.exited=false`。因此停止该托管
  身份的后续发送，没有把它换成别的 Agent。当时未继续多行等输入；后续通过
  新建独立 PTY 完成下节采样。不能把托管失效归因于 CLI 进程退出。

夹具与实机证据分开保留：`.tmp/pane-sidebar-s1/live-shell-accepted.jsonl`
记录测试 Shell 返回值，`live-native-users.json` 只投影本次四个测试会话的用户
消息，不保存模型推理或全量会话导出。测试结束后退出并关闭 Pane 3 至 8，
再次 snapshot 确认只剩原有 Pane 1、2，原 Tab ID、顺序与活动 Pane 恢复。
已获授权的 Kimi 目录信任及 Agent 自身生成的测试会话记录保留。

时间校准：本机 Git Bash 的 `TZ=Asia/Shanghai date` 实际返回 UTC；经 UTC
时钟、Windows `China Standard Time` 和 `TZ=CST-8 date` 交叉核对，本文与
ROADMAP 本次新增的 07:09 记录已更正为北京时间 15:09。

## 收口补测与 S1 验收

2026-10-27 16:04 记录。新建 Pane 9 运行 Kimi CLI，按本次新 Pane 的明确
身份采样；没有恢复或复用已失效的托管 Agent generation。Pane 10 是
“S1 手动输入验证”Tab，只对该 Shell 安装接受文本探针。

| 补测 | 实际证据与结果 |
| --- | --- |
| 用户真实键盘，仅编辑 | 用户键入 `Write-Output S1_KEYBOARD` 后暂停；Runtime 读取到编辑行，ReadLine 日志没有接受记录 |
| 用户真实 Enter 提交 | 用户按 Enter 后，日志新增一条完全相同的接受命令，终端实际输出 `S1_KEYBOARD` |
| 用户系统剪贴板，两行但不提交 | 用户用平时的粘贴快捷键粘贴 `Write-Output S1_CLIP_A`、`Write-Output S1_CLIP_B`；编辑器显示两行，接受记录仍只有前一条键盘命令 |
| 用户提交剪贴板内容 | Enter 后只增加一条带 LF 的完整接受命令，终端输出两行标记；证明实际粘贴与提交是不同边界 |
| 清空与空 Enter | 首次空 Enter 未取得接受记录，未记为通过；用户随后按 Ctrl+C 清空，再空 Enter，取得两个 `submitted=false` 的空返回，未增加命令 |
| Kimi 首次两行 paste、不提交 | 新 CLI 仍显示未建立会话，原生 session index 没有新增该会话；文本只在编辑器中 |
| Kimi 显式 Enter | 新增本轮 session；原生 `turn.prompt` 的 `input[0].text` 保留两行 LF |
| Kimi 重复同一多行消息 | 第二次 `paste --no-submit` 后用户记录仍为 1；Enter 后为 2，正文完全相同，`promptId` 不同，`turnId` 分别为 0、1 |
| Kimi 空 Enter | 未增加用户消息；后续第三条指定 Runtime 消息产生 `turnId=2`，没有夹入空用户消息 |
| Kimi Runtime 自动提交 | `pane.paste` 默认提交产生第三条真实 `turn.prompt`，含 `S1_KIMI_RUNTIME` 及 LF；未补发 Enter，3 个 prompt 最终均有 `prompt.completed` |

Kimi 本机全局配置经 TOML 只读解析，`hooks` 为空，未修改配置来制造 hook
样本。这是当前路径的确定缺口，原生 wire 已提供用户角色、正文、prompt ID 和
turn ID 的代表性样本；未安装的 hook 不能被当作已有的可靠采集来源。
带队列的提交路径和 hook 原始 `prompt` 形状仍未覆盖，归入 S4 接线验收。

托管身份失效的机制已从当前源码核对：
[runtime_api.rs](nebula_app/src/runtime_api.rs) 的 `project_managed_agents`
在一个已观察的 Agent 对应 `pane.agent=None` 时就永久关闭其托管身份，理由为
`agent_exited`；这条分支不要求 PTY 已退出，也不会因下一次再次识别出同种
程序而自动恢复 generation。上轮现场与该机制相容，但引发那次身份短暂缺失的
具体原因未定位。本轮不修复 Runtime，也不将这项待修问题混入 Pane 分组实现。

本轮实际证据保存在 `.tmp/pane-sidebar-s1/round2/evidence.json`：Shell 有
2 条非空接受命令与 2 条空返回；新 Kimi 会话有 3 条用户消息、3 个不同
prompt ID 和 3 条完成记录。探针只摘取本次用户样本与必要元数据，不保存模型推理。
退出 Kimi 后关闭 Pane 9、10，snapshot 再次确认仅保留原 Pane 1、2，
原 Tab ID、顺序和活动 Pane 均保持。全局 Agent 配置没有修改。

S1 验收逐项核对：

| S1 交付要求 | 结论 |
| --- | --- |
| Pane 身份、名称、活动状态及目录来源 | 已列出当前代码权威来源、现场投影和限制 |
| 固定项目目录、当前目录、继承归属区分 | 已列出创建入口、优先级和缺失模型；`cd` 后分屏实测目录来源 |
| 自用四种 Agent 用户消息与 Shell 命令来源 | 已核对原生用户记录、Shell 接受文本及可靠性缺口 |
| 已提交、仅编辑、多行换行、粘贴未提交、空内容和连续提交的代表样本 | 已取得 Runtime、原生记录及用户真实键盘/剪贴板样本；人工样本范围为本地 PowerShell |
| 不把 Enter 或通用 hook message 当成用户消息 | 已取得反例并明确来源、接受和提交三个边界 |

S1 已完成数据来源核验。下面是实施阶段必须处理的已知缺口，不是已经通过的
产品预览验收；S2 至 S6 仍未开始。

## 后续阶段约束与未覆盖路径

- S4 处理 Codex、OpenCode 的活体会话绑定缺失，接入 OpenCode、Pi、Kimi 的
  用户正文；Kimi 无 hook 的当前路径、hook 载荷及排队消息必须明确覆盖或占位，
  未解决时不得将 S4 标为完成。不能按 cwd 或文件修改时间猜同目录的活体会话。
- S4、S6 复测 API 接受后文本仍留在编辑器的路径、托管身份失效，以及四种
  Agent 的真实键盘交互；本次 Shell 人工样本不代表四种 Agent 的物理键盘均已验收。
  原生 PowerShell 的 Runtime paste 拒绝与用户真实剪贴板成功是两条独立路径。
- S4 的刷新任务绑定 Pane ID、Agent kind、session ID、会话 epoch，以及本次
  预览请求序号；同会话的旧请求也不能晚到覆盖新提交。共享
  [runtime_conversation.rs](nebula_app/src/gpui_shell/workspace/runtime_conversation.rs)
  已有后台读取、WeakEntity 和完成后重查身份，可复用其边界，但侧栏缓存仍待实现。
- 切换 Agent、替换会话、退出回 Shell、关闭 Pane 时使旧请求失效；不得用
  Shell 的启动 Agent 命令填充 AI 预览，也不得把上一 Agent 消息带回 Shell。
- S2 可依据本次来源与上下文清单建立分组模型；本轮没有开始 S2，也没有修改产品代码。
