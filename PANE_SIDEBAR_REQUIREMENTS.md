# 按工作目录分组的 Pane 会话侧栏

确认时间：2026-10-27。状态：S1 至 S3 已完成，S4 至 S6 未开始。

## 目标

面向自用，方便同时查看多个项目和 Agent。打开侧栏即可看清每个项目有哪些
Pane、各自在做什么、谁需要处理，点击后准确切到对应 Pane。

## 首版范围

- 项目本身支持多个系统平台；当前需求仅针对本机 Windows，主要用于本地
  命令行（Shell）和运行于其中的 AI Agent。
- macOS、Linux、WSL、SSH 暂不纳入本功能的实现和验收范围。

## 1. 项目分组

- 打开项目时，按所选目录确定项目身份；同目录复用已有项目组。
- 项目内新建会话或分屏时继承项目归属，显式指定的项目归属优先于 Pane 的
  实际启动目录；只有未指定项目归属的新终端，才按启动目录归组。
- 项目头显示文件夹图标、目录名和 Git 分支；没有 Git 信息时隐藏分支。
- 后续执行 `cd` 不改变分组。
- 支持展开、折叠和置顶；置顶项目排在前面，其余保持稳定顺序。

### 创建入口与项目归属

| 操作 | 入口 | 启动位置与归属 |
| --- | --- | --- |
| 在已有项目下新建会话 | 项目标题右侧的 `＋` | 使用该项目的固定目录，新建一个 Tab，包含一个 Pane，加入当前项目组 |
| 在已有会话中分屏 | Pane 的分屏按钮或快捷键 | 在当前 Tab 中新增 Pane，工作目录沿用被分屏 Pane 的当前目录，项目归属继承原 Pane |
| 打开一个项目会话 | 侧栏顶部“打开项目…” | 选择本地文件夹，以该目录建立项目组，并创建首个 Tab 和 Pane；目录已有项目组时，复用该组并新建会话 |

- 全局“新建终端”默认沿用当前 Pane 的项目归属；没有活动项目时使用现有
  默认启动目录，并按该目录归组。
- 项目内新建后，可以按现有方式运行 Shell 或启动 AI Agent。
- 例如，项目目录为 `C:\work\demo`，已有 Pane 执行 `cd C:\work\other` 后，
  项目头 `＋` 仍在 `C:\work\demo` 启动；对该 Pane 分屏则在 `C:\work\other`
  启动，但仍属于 `demo` 项目组。

## 2. 会话条目

- 一个条目对应一个 Pane，包括普通 Shell 和 AI Agent。
- 第一行显示程序图标、会话名称和当前状态，名称优先沿用现有命名。
- 同一个 Tab 内的多个 Pane 独立显示。

## 3. 第二行预览

- AI Agent 显示最近一次已提交的用户消息。
- Shell 显示最近一次已提交的命令。
- 无内容时显示“新会话”。
- 多行内容合并展示，最多显示两行，超出省略。
- 正在编辑、尚未提交的输入不更新预览。

## 4. 点击与状态

- 点击条目，激活所属 Tab 并聚焦对应 Pane。
- 当前 Pane 使用整行背景和左侧竖线标识。
- 活动状态复用项目现有语义，包括运行中、等待输入、等待授权、完成和故障。
- 折叠项目时，其中的会话继续运行。

## 5. 展示效果

- 使用树形缩进和细连接线表达项目与会话的归属。
- 项目名突出，任务预览使用次级文字颜色。
- 项目之间留适量间距，同组会话保持紧凑。
- 状态和预览更新时保持条目位置稳定。

## 验收标准

以下标准仅针对本机 Windows 的本地 Shell 与 AI Agent 场景。

1. “打开项目…”按所选目录创建项目组与首个会话；选择已有项目目录时复用组，
   选择其他目录时建立新组。
2. 点击项目头 `＋`，在该项目固定目录下新建 Tab 和 Pane，并加入该组。
3. 对已执行 `cd` 的 Pane 分屏时，新 Pane 使用其当前工作目录，但保留原项目归属。
4. 全局“新建终端”沿用当前 Pane 的项目归属；没有活动项目时，按默认启动目录归组。
5. Pane 执行 `cd` 后保留原分组。
6. 一个 Tab 中有多个 Pane 时，每个 Pane 都有独立条目，点击后准确聚焦。
7. AI Agent 提交新消息、Shell 提交新命令后，各自更新预览；未提交的输入不更新，
   无内容时显示占位，长内容最多显示两行并省略。
8. 项目支持展开、折叠和置顶，折叠不影响会话运行，置顶项目排在前面。
9. 当前 Pane 高亮明确，状态沿用现有语义，内容更新不改变条目顺序。

## 实现路线表

制定时间：2026-10-27。按 S1 → S6 顺序推进，每阶段验收通过后再进入下一阶段。
各阶段均以本机 Windows 的本地 Shell 与 AI Agent 为对象。S1 已完成来源与缺口
清单、夹具及实机代表样本核验；补齐 Kimi 的多行、未提交、重复、空输入和
Runtime 提交，以及用户真实 Shell 键盘、剪贴板样本。S2 分组模型及生命周期接线
已完成 Windows 源码编译与针对性回归；S3 已接入侧栏与创建导航，控件回归及原生窗口代表检查通过，
结果见 [路线图](ROADMAP.md)。S4 至 S6 未开始，消息行暂用“新会话”占位。
自用 Agent 核验范围为 Codex、OpenCode、Pi、Kimi，来源、缺口和验证边界见
[S1 核验记录](PANE_SIDEBAR_S1.md)。

| 阶段 | 依赖 | 实现范围 | 阶段交付与验收 | 状态 |
| --- | --- | --- | --- | --- |
| S1：确认数据来源 | 无 | 核对 Windows 本地 Pane 身份、名称、活动状态、启动目录及创建入口的项目上下文；重点验证自用 AI Agent 的用户消息与 Shell 命令来源 | 列出实际来源和缺口，区分项目固定目录、Pane 当前目录及继承归属；用代表性样本区分已提交、仅编辑、多行换行及粘贴未提交；不能把一次 Enter 或任意 hook 的 `message` 直接视为用户消息 | 已完成：来源、缺口及代表样本已核验，后续采集限制见 [S1 记录](PANE_SIDEBAR_S1.md) |
| S2：建立 Pane 分组模型 | S1 | 基于现有 Pane ID 建立条目投影，保存项目固定目录及 Pane 归属；创建时优先使用指定或继承的项目，仅在未指定时按启动目录归组；形成稳定项目顺序 | 同目录打开项目时复用组；项目内新建和分屏继承归属；`cd` 不换组；新增、关闭 Pane 后条目正确，不因 Tab 下标或焦点变化串项 | 已完成：固定/待归属、稳定顺序及创建、复制、冷恢复、关闭和移窗接线；Windows 编译与回归通过，项目头和打开项目的界面入口留在 S3 |
| S3：接入侧栏与创建导航 | S2 | 将终端侧栏由 Tab 平铺改为项目头和 Pane 条目；接入项目头 `＋`、“打开项目…”及全局新建终端的归属规则；支持当前 Pane 高亮，适配项目头和双行条目的滚动与命中范围 | 各创建入口按约定启动并归组；同一个 Tab 的多个 Pane 均可独立点击；跨 Tab 点击准确激活并聚焦；滚动后命中正确；设置和文档等非终端 Tab 仍有可用入口 | 已完成：项目头、Pane 双行条目、选择与单 Pane 关闭、项目/全局创建入口、文档入口及混合行高滚动已接入；30 项相关回归与原生窗口代表检查通过，完整便携包人工验收留在 S6 |
| S4：接入最近提交预览 | S1、S3 | 按 S1 核实的来源更新 AI 用户消息和 Shell 命令；实现占位、多行合并、两行省略及异步结果归属检查 | 新提交更新对应 Pane，未提交不更新；切换 Agent、替换对话及返回 Shell 后不串用上一种身份的预览；采集不到可靠内容时显示占位，并记录未覆盖路径 | 待开始 |
| S5：补齐项目交互与视觉 | S3、S4 | 实现分组折叠、项目置顶、分支显示、树形连接线和间距；沿用现有主题及国际化方式 | 折叠不影响运行；置顶项靠前且组内顺序稳定；分支对应分组的固定目录；窄侧栏和长文本下仍能辨认选中项、状态和预览 | 待开始 |
| S6：回归验证与交付 | S2 至 S5 | 补齐 Windows 本地 Shell 与 AI Agent 的关键行为回归测试，在本机 Windows 执行项目检查并核对实际窗口交互与视觉 | 本文全部验收标准通过；保存实际检查结果和必要截图，明确 Windows 本机场景仍未验证的采集路径；同步更新路线图后标记完成 | 待开始 |

### 现有代码入口

以下是本次查阅确认的入口，具体改动以 S1 核实结果为准。

| 能力 | 入口与复用方向 |
| --- | --- |
| Pane 所有权、创建、分屏、聚焦 | [workspace.rs](nebula_app/src/gpui_shell/workspace.rs) 中的 `TerminalPane`、`WorkspaceTab` 与 `focus_pane`；创建路径已迁到 [terminal_creation.rs](nebula_app/src/gpui_shell/workspace/terminal_creation.rs)，保留现有会话和分屏所有权 |
| 固定项目归属与分组 | [pane_groups/model.rs](nebula_app/src/gpui_shell/workspace/pane_groups/model.rs) 维护一次性项目绑定与稳定的窗口内 Pane ID 投影；[pane_groups.rs](nebula_app/src/gpui_shell/workspace/pane_groups.rs) 适配工作区生命周期；设计边界见 [归属记录](architecture/notes/nebula_app/gpui_shell/workspace/2026-10-27-pane-project-binding.md) |
| 当前侧栏与滚动 | [pane_sidebar.rs](nebula_app/src/gpui_shell/workspace/pane_sidebar.rs)及其子模块读取 S2 投影，Windows 使用 `ScrollHandle` 处理混合行高；[sidebar.rs](nebula_app/src/gpui_shell/workspace/sidebar.rs)、[tab_scroll.rs](nebula_app/src/gpui_shell/workspace/tab_scroll.rs)保留其他平台及顶部 Tab 的现有路径 |
| 名称、图标和状态 | Pane 条目复用 [pane_header.rs](nebula_app/src/gpui_shell/workspace/pane_header.rs)及每个终端的 [tab_identity.rs](nebula_app/src/gpui_shell/terminal/view/tab_identity.rs)、[runtime.rs](nebula_app/src/gpui_shell/terminal/view/runtime.rs)；文档及旧 Tab 路径继续使用 [tab_presentation.rs](nebula_app/src/gpui_shell/workspace/tab_presentation.rs) |
| Shell 命令与提交路径 | [view.rs](nebula_app/src/gpui_shell/terminal/view.rs) 的命令事件、[runtime.rs](nebula_app/src/gpui_shell/terminal/view/runtime.rs) 的提交路径及 [keymap.rs](nebula_app/src/gpui_shell/terminal/keymap.rs) 的按键语义；S1 同时核对手动输入和 Runtime 输入 |
| AI 消息来源候选 | [conversation.rs](nebula_app/src/assistant_answer/conversation.rs) 已有原生对话消息投影；[event.rs](nebula_app/src/ai_hook/event.rs) 已有提交事件，但 `message` 也可能是通知或助手消息，不能直接复用为预览 |
| Git 上下文 | [vcs_panel.rs](nebula_app/src/gpui_shell/workspace/vcs_panel.rs) 及已有 Git 数据获取能力；查询分组固定的 Windows 本地目录，仅在目录一致时复用分支结果 |

### 首版实现约定

- 聚焦自用，不新增第三方依赖或独立后台服务；新增展示和分组逻辑放在职责明确的
  侧栏模块中，避免继续堆入 `workspace.rs`。
- 复用项目现有跨平台结构；Windows 特有的采集逻辑通过已有平台边界接入。
- 项目身份使用 Windows 本地完整目录，显示目录名；Pane 的所属项目与其当前
  工作目录分别维护。打开项目以所选目录确定身份，项目内新建和分屏继承归属。
- 未指定项目归属且启动目录暂不可知时，保留待归组状态，得到首个有效目录后
  固定归属。
- 新组和新 Pane 默认按出现顺序追加，状态、预览及 Git 刷新不触发重新排序。
- 折叠和置顶先按当前窗口内存状态实现，跨重启记忆不作为首版验收项；恢复
  Pane 时沿用现有恢复流程，以恢复后的初始目录建立分组。
- 预览采集与 Git 查询复用现有能力，在后台有界执行；关闭 Pane 或更换会话后
  丢弃旧结果。渲染函数只读取已准备的数据。
- 不可靠的数据源保持占位；若自用 Agent 的用户消息仍无法采集，须记录缺口，
  不能据此将 S4 或整项功能标记为完成。

### 验证安排

- 以下验证均在本机 Windows 执行，其他系统平台不作为本需求的验收条件。
- S1、S4：在 Windows 本地 Shell 与 AI Agent 中核对手动提交、Runtime 提交、多行换行、
  粘贴未提交、空内容和连续提交；模拟旧异步结果晚到，确保不覆盖新消息或其他 Pane。
- S2、S3：验证打开项目的新建与复用、项目头 `＋`、全局新建终端的归属规则，
  以及 `cd` 后分屏的目录继承与项目归属；验证同 Tab 多 Pane、关闭后清理，
  分组和双行条目混合滚动后的真实点击位置；检查已有 Tab 关闭与拖拽操作未被误绑定。
- S5、S6：在实际窗口核对折叠、置顶、当前项高亮、长消息、窄侧栏、浅深主题，
  以及鼠标和键盘入口；编译与状态单测不代替真实交互和视觉验收。
- 实现后执行相关 Rust 行为测试及以下基础检查；影响设置或国际化时，再执行
  对应模块检查，具体入口遵循 [贡献指南](CONTRIBUTING.md#local-checks)。

```powershell
python scripts/check_architecture.py
cargo fmt --all -- --check
cargo test -p nebula --test file_line_budget --locked
cargo check -p nebula --bin pebrel --features gpui-shell --tests --locked
```

需要真实 GPUI 控件回归时，使用 `gpui-test-support` feature 运行相关测试。
交付记录区分已执行的测试、人工验收和未验证范围；每阶段完成后更新本表状态，
最终验收通过后再更新 README 中的现状说明。
