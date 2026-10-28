# Pane 侧栏导航与混合行高滚动

## Status

Implemented；S3 实际验证结果见 [路线图](../../../../../ROADMAP.md)。

## Context

[需求](../../../../../PANE_SIDEBAR_REQUIREMENTS.md)以 Pane 为导航条目，但会话所有权仍属于 Tab 的分屏树。
同一个 Tab 可以包含多个项目的 Pane，项目顺序也不随 Tab 重排改变。
项目头、双行 Pane 和文档条目高度不同，旧侧栏的 Tab 下标与固定行高不能作为命中依据。

## Evidence

- [分组模型](../../../../../nebula_app/src/gpui_shell/workspace/pane_groups/model.rs)只维护活体 Pane ID 与固定项目目录。
- [旧滚动实现](../../../../../nebula_app/src/gpui_shell/workspace/tab_scroll.rs)仍服务旧侧栏与顶部 Tab；其整行窗口不能直接用于混合行高。
- [GPUI 控件测试](../../../../../nebula_app/src/gpui_shell/workspace/pane_sidebar/tests.rs)复现了首个项目头 margin 使最后一行被裁切的问题。
- Windows 原生测试使用真实 GPUI 窗口和 PTY；确定性 TestApp 的调度器不接受真实 PTY 线程唤醒，因此两类测试分开运行。

## Decision

- [侧栏投影](../../../../../nebula_app/src/gpui_shell/workspace/pane_sidebar.rs)直接读取 S2 模型，不持有第二套项目归属或 Tab 状态。
  待归属、未纳入本地项目的终端及文档保留可达入口。Windows 使用新侧栏，其他平台保留原侧栏。
- [导航](../../../../../nebula_app/src/gpui_shell/workspace/pane_navigation.rs)每次通过 Pane ID 查找当前所属 Tab，
  离开设置/阅读器后聚焦目标 Pane；目标被 zoom 隐藏时退出 zoom。文档通过稳定 Tab ID 定位。
- 单 Pane 关闭沿用繁忙进程确认；确认后再次按 ID 定位，避免等待期间 Tab 重排造成误关。
  整 Tab 的上下文菜单继续保留原语义，整 Tab 拖动只绑定独立手柄，不绑定整行 Pane。
- 手柄复用现有 Tab 拖动状态机、Alt 撕窗、窗外释放捕获、停靠和跨窗传输。
  投影侧栏的目标按 `ScrollHandle` 实际子项边界及滚动偏移命中，以 Pane ID 保存；
  释放时重新定位源和目标 Tab，不用像素步距换算 Tab 槽位。
- 混合行直接交给 GPUI `ScrollHandle` 与现有组件滚动条；选中项定位使用实际平铺投影序号。
  项目头不放首项外边距，避免内容高度与可见边界出现差额。
- “打开项目”任务由侧栏状态持有，选择期间禁用重复提交；取消不创建会话。
  目录有效性检查放在后台，返回后在仍存活的工作区中按所选目录创建，
  不重新读取当时焦点项目。任务随工作区销毁取消。
- 项目头新建显式传递固定目录与项目归属；全局新建继承当前项目和已有目录规则。
  设置页面不继承隐藏终端的项目。默认启动器为 WSL/SSH/串口时禁用本地项目入口并解释原因。
- 控件夹具用 App 内的测试启动快照隔离本地 Shell 和默认分屏，不使用进程级环境变量替换其他测试的启动器。
  该入口仅在测试 feature 下编译；原生夹具不设置它，继续运行真实 PTY。

## Rejected alternatives

- 把项目/Panes 填入旧 Tab 行下标：重排、关闭或跨窗后会指向其他对象。
- 用固定像素乘条目序号：无法同时正确处理项目头、双行会话和文档，也会让 scrollbar 与点击区域分离。
- 拖动整行时默认移动整个 Tab：Pane 行会暗示只移动一个 Pane，必须明确整 Tab 边界。
- 只给手柄挂原生 `on_drag`：不能替代现有状态机的 Alt 撕窗与窗外释放行为，扩展回归会失败。
- 目录选择完成后继承当前焦点：用户等待期间切换 Tab 会导致选中的项目被错误归组。
- 在确定性 GPUI 测试中直接启动默认 Shell：后台 PTY 线程使测试调度不可复现；不能把这种失败解释为 UI 通过。

## Consequences

侧栏只改变本机 Windows 的侧栏呈现，不改变终端树、项目身份策略或会话持久化格式。
分支、折叠、置顶与树形连接线仍由 S5 实现；消息行在 S4 接入可靠采集前显示“新会话”占位。
Tab 重排不会改变项目条目的稳定顺序，因此拖动手柄影响 Tab 所有权/顺序，不用视觉排序伪装 Pane 移动。

## Validation

- 控件测试覆盖独立 Pane 选择、zoom、重排、单 Pane 关闭、固定目录新建、全局继承、
  目录选择取消/无效/复用、混合行高滚动、窄侧栏、文档及不支持启动器的 disabled 状态。
- [原生夹具](../../../../../nebula_app/src/gpui_shell/workspace/pane_sidebar/native_tests.rs)提供隔离窗口，
  在实际控件边界分发 GPUI 输入事件，供检查真实渲染、选择与关闭；截图与具体执行结果记录在路线图。
- 原生夹具的事件分发不是用户键盘鼠标验收；完整手动交互与 Agent 预览仍需 S6。

## Supersedes

None。

## Revisit when

要支持跨平台项目侧栏、对 Pane 进行项目内拖动排序，或项目规模需要虚拟化时，重新评估投影与输入合同。
