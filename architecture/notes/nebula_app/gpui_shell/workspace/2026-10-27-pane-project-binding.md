# Pane 项目归属的一次性绑定

## Status

Implemented；S2 的验证结果见 [路线图](../../../../../ROADMAP.md)。侧栏展示与创建导航在 S3 接入。

## Context

[需求](../../../../../PANE_SIDEBAR_REQUIREMENTS.md)要求以 Pane 为条目，项目固定目录与 Shell 当前目录分开。
分屏沿用当前目录启动，归属却必须继承项目；Tab 拖动、停靠和移窗不能改变这一归属。
某些 Shell 启动时没有目录，必须等待首个有效 cwd。等待期间仍可能分屏、关闭原 Pane 或移窗。

## Evidence

- [S1 核验](../../../../../PANE_SIDEBAR_S1.md)确认现有 Pane ID、当前 cwd 和创建入口；当前目录不能充当固定项目身份。
- [创建适配](../../../../../nebula_app/src/gpui_shell/workspace/terminal_creation.rs)在视图建立后读取实际启动 cwd 与机器环境。
- [复制路径](../../../../../nebula_app/src/gpui_shell/workspace/tab_duplication.rs)按布局树叶序保存元数据；Pane 存储顺序可能与布局不同。
- Windows [CompareStringOrdinal 文档](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal)定义系统大写表与 ordinal 大小写比较；Unicode 字符串转小写不能替代目录比较策略。

## Decision

- 活体 `TerminalPane` 拥有可选的 `PaneProject`。Windows 本地 Shell 才参与分组，SSH、WSL 和串口不参与。
- [模型](../../../../../nebula_app/src/gpui_shell/workspace/pane_groups/model.rs)用 `Rc<OnceCell<ProjectDirectory>>` 表达一次性归属。
  显式或继承项目优先；无项目时使用启动目录；仍未知时保留待归组。首次有效目录固定后，后续 cwd 不再修改它。
- 分屏、全局新建、内存复制与 AI fork 共享归属绑定。待归组的后代也共享绑定，任一后代首次取得有效目录即可固定该继承链。
  绑定仅持有目录数据，不保留 GPUI 实体、订阅、PTY 或窗口。
- 每窗口的 `PaneGroups` 仅投影活体 Pane ID 与绑定。保留幸存组和条目的首次出现顺序，新项追加，空组移除。
  只在完整结构操作结束时同步；移 Tab 的中间 remove 不同步，避免将原有条目当成新项。
- cwd 事件更新对应实体的待归属；实体已移走或关闭时不再写回。跨窗口的待绑定解析通过现有工作区 watchdog 同步，最多等待其下一轮，不新增任务。
- [目录身份](../../../../../nebula_app/src/platform/project_directory.rs)使用本地绝对路径的词法规范化：分隔符、`.`、`..` 和常见 verbatim 前缀统一，Windows 使用 ordinal 忽略大小写比较。
  原始 verbatim 启动形式保留；不访问磁盘，不解析符号链接、junction 或短文件名。
- 冷恢复继续按恢复后初始目录归组，不修改会话 schema。内存复制按布局叶序传递各 Pane 项目，活体移窗直接携带原绑定。
- 终端创建函数按职责迁到 `terminal_creation.rs`，避免继续增加超预算的工作区根文件。

## Rejected alternatives

- 直接按当前 cwd 分组：执行 `cd` 会移动条目，违反固定项目语义。
- 用 Tab 下标存归属：拖动、混合项目停靠及跨窗会串项；Tab 不是 Pane 身份。
- 每 Pane 独立复制未知目录：未知时分屏后，两者可能各自固定到不同项目，丢失继承语义。
- 绑定持有父 Pane 或窗口：父 Pane 关闭或移窗后会留下悬空关联，或延长无关实体生命周期。
- 每次渲染重新分组或 canonicalize：造成顺序漂移，或在渲染中增加磁盘与网络目录访问。
- 立即持久化项目目录：当前需求只要求窗口内存归属，冷恢复规则已有明确约定，不扩大 schema 范围。

## Consequences

投影不拥有会话；关闭和移窗仍由现有工作区生命周期管理。pending 继承链可在原 Pane 关闭后解析。
多个路径别名指向同一目录时可能分组不同；Windows 启用区分大小写的目录仍使用首版忽略大小写策略。
上述身份策略针对本机常规 Windows 项目路径，不能当作文件系统等价或访问权限判断。
侧栏暂未呈现分组，后续 S3 直接读取该投影；折叠和置顶也不进入会话持久化。

## Validation

- [纯模型测试](../../../../../nebula_app/src/gpui_shell/workspace/pane_groups/model/tests.rs)覆盖固定归属、待归属继承、稳定顺序、关闭与跨窗投影。
- [工作区测试](../../../../../nebula_app/src/gpui_shell/workspace/pane_groups/tests.rs)覆盖真实创建、分屏、全局新建、关闭事件、cwd 订阅和复制/冷恢复。
- [跨窗测试](../../../../../nebula_app/src/gpui_shell/workspace/windowing/transfer_tests.rs)覆盖移动后的源清理、目标停靠与关闭。
- [Pane 头部测试](../../../../../nebula_app/src/gpui_shell/workspace/pane_header/native_tests.rs)覆盖拆为新 Tab 后的稳定归属顺序。
- 编译、测试、架构门禁与格式检查的实际执行结果记录在路线图，不能用已安装旧版本的 Runtime 代替当前源码验证。

## Supersedes

None。

## Revisit when

需要跨重启固定项目、支持远端项目、支持区分大小写的 Windows 目录，或要合并 symlink/junction/短文件名别名时重新设计身份与持久化合同。
