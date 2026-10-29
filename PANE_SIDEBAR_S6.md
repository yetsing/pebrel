# S6 Windows Pane 侧栏验收记录

2026-10-29。本记录区分自动检查、隔离实机检查和用户人工验收。
当前进度以 [ROADMAP.md](ROADMAP.md) 为准；用户人工验收尚未完成。

## 本阶段改动

- 修复 Codex 原生 Windows 输入路径丢失非 BMP emoji，提交和草稿共用编码。
- 修复 Kimi Runtime prompt 的粘贴结束及 Enter 提交边界。
- 接入 Codex 0.165.0 默认共享 daemon：按当前 TUI 的真实文件句柄和原生
  提交 UUID 精确关联服务端接受记录；同目录多 Pane 不按目录、时间或文本猜测。
- 原生新会话事件清除 live 绑定；同线程新提交也使旧异步结果失效。
- 原生录制文件采用私有 DACL、稀疏清零和 kernel delete-on-close，保留用户
  显式录制设置。没有新增依赖、持久化设置或独立后台服务。

设计依据见 [输入边界](architecture/notes/nebula_app/input/2026-10-29-agent-submission-boundaries.md)
和 [daemon 归属](architecture/notes/nebula_app/platform/ai_session_identity/windows/2026-10-29-codex-shared-daemon-binding.md)。

## 验收标准与证据

以下检查使用本机 Windows 当前源码。原生窗口夹具启动真实 PTY，并通过真实
GPUI 控件命中路径执行操作；实际 CLI 测试使用独立配置和本地模拟模型。
它们不代替用户在最终 ZIP 中的真实键鼠与日常模型验收。

| 需求标准 | 已执行的检查及结果 |
| --- | --- |
| 1．打开项目、新目录及同目录复用 | GPUI 控件测试覆盖路径选择、取消、无效目录、新组及复用，通过 |
| 2．项目头 `＋` 使用固定目录 | 真实控件测试与全局创建路径分开断言，通过 |
| 3．`cd` 后分屏继承当前目录及原项目 | GPUI 测试通过；隔离产品实测 `cd-target` 分屏启动目录和固定项目均正确 |
| 4．全局新建继承项目，无活动项目用默认目录 | GPUI 控件测试覆盖全局按钮和设置页无活动项目路径，通过；Runtime `new_tab` 属于其独立创建路径，未拿它冒充该按钮验收 |
| 5．`cd` 不换组 | 模型和真实 PowerShell 命令接受协议检查均通过，目录来回切换不改变归属 |
| 6．同 Tab 多 Pane 独立导航与关闭 | GPUI 控件覆盖重排、跨 Tab、退出 zoom、单 Pane 关闭及混合行高滚动；原生窗口点击选择和关闭通过 |
| 7．可靠提交、草稿、占位和长文本 | 四种真实 CLI、多行、中文 emoji、重复接受及空 Enter 通过；Codex 默认 daemon 同目录双客户端、空白 `/new`、重新绑定和忙碌追加通过；原生窗口预览保持最多两行 |
| 8．折叠、置顶、隐藏会话继续运行 | 深浅主题 × 220/300 像素四组原生窗口通过；隐藏 PTY 完成命令，固定目录分支在其他组位于前台时更新，取消置顶恢复稳定顺序 |
| 9．当前项、状态与顺序 | 原生截图检查选中背景和竖线；控件及实机验证预览、状态、Git 刷新及关闭其他 Pane 后顺序稳定 |

控件入口见 [pane_sidebar/tests.rs](nebula_app/src/gpui_shell/workspace/pane_sidebar/tests.rs)、
[pane_groups/tests.rs](nebula_app/src/gpui_shell/workspace/pane_groups/tests.rs)
和 [native_tests.rs](nebula_app/src/gpui_shell/workspace/pane_sidebar/native_tests.rs)。

实际 CLI 版本：Codex 0.165.0、OpenCode 2.1.2、Pi 1.3.0、Kimi 2.3.3。
Codex 官方 daemon version 接口返回 running，CLI、daemon 均为 0.165.0；
测试保留既有 daemon，并在产品重启后连接它，未强制禁用 daemon。
两个客户端的原生线程 ID 不同，依次更新、重复提交及 `/new` 后未串用另一个 Pane。
退出 Codex、Pi、Kimi 后 Agent 身份和预览均清除；结束隔离产品后五个临时
capture 路径均删除。新增原生权限、稀疏清零及句柄测试也检查这些失败边界。

## 保存的原生窗口截图

截图来自本次 S6 的真实窗口，220/300 指逻辑侧栏宽度，设备缩放会改变 PNG 像素宽度。

| 主题与宽度 | 代表画面 |
| --- | --- |
| 深色，220 | [初始分屏、长名称及两行预览](docs/screenshots/pane-sidebar-dark-220.png) |
| 浅色，220 | [置顶后精确选择第二个 Pane](docs/screenshots/pane-sidebar-light-220.png) |
| 深色，300 | [置顶项目、折叠组及固定目录分支更新](docs/screenshots/pane-sidebar-dark-300.png) |
| 浅色，300 | [运行状态、选中背景、树形连接和长文本](docs/screenshots/pane-sidebar-light-300.png) |

## 工程检查

- 相关 Rust/GPUI 回归：368 项通过，3 项忽略。其中原生窗口夹具已按上述四组
  显式执行；SSH 导出和空闲桌面光标对比不在此次验收范围，未执行。
- 设置：103 项通过；国际化合同：24 项通过、1 项忽略。
- 架构治理：52 项通过、3 项跳过；架构门禁通过，1177 个源文件、10 个 crate。
- 本次修改的 Rust 文件格式检查通过。全库 `cargo fmt --all -- --check` 仍因
  未修改文件的既有换行格式失败，没有借本任务批量格式化其他模块。
- 文件预算：2 项通过；产品 `cargo check --tests` 通过，GPUI release 新鲜构建通过。

## 便携包交付

2026-10-29 08:50 核验：`Pebrel-v2.3.1-windows-x64.zip`，18 个文件，
41,219,318 字节。SHA256：
`4E7A130FDA71E48AC54282B058B766A43ACDC372DBD12115D809EAB28825B024`。

使用标准 GPUI release 打包流程，没有跳过构建或放宽新鲜度。ZIP 清单和五个
可执行载荷的 x64 PE 类型通过检查。解压后实际启动，Runtime API、PowerShell
命令执行、默认共享 daemon 的 Codex 原生绑定、中文 emoji 接受及退出清理均通过。
release 不输出隔离 debug QA 观察文件；窗口侧栏的人工键鼠验收仍待用户完成。

版本文本为 `2.3.1 (70ed3959)`，包包含构建时该基线之上的 S6 工作区改动，
版本文本不代表这些改动的提交号。源码按用户指令单独提交，便携包在提交前构建；
没有推送、改版本号或发布 Release。隔离测试产品、本地模拟模型和
测试专用 CODEX_HOME 中的 daemon 已关闭，用户运行环境未用于写入测试。

## 采集边界

- 当前验收只覆盖本机 Windows 的本地 Shell 和上述四种 CLI；其他平台、WSL、SSH
  不在本次验收范围。
- CMD 无可靠接受来源时占位；Kimi TUI 的无序队列按用户已确认的例外占位，
  队列完成后从精确主 Agent 原生记录恢复普通提交预览。
- Codex daemon 绑定需要当前 TUI 的提交 UUID 和服务端接受记录同时存在。
  空白会话、尚未出现可关联提交的恢复会话、用户显式禁用或自定义原生录制，
  以及原生格式/资源上限不能支持的路径继续占位，不使用旧对话或屏幕正文补猜。
- 本地模拟模型用于稳定检验 CLI 的接受链路；没有据此声称日常网络模型、人工
  剪贴板和全部 CLI 版本均已验收。

## 人工验收清单

使用交付 ZIP 解压到独立目录后启动，建议依次检查：

1. “打开项目…”选择两个目录，再次选择第一个目录应复用项目组；取消选择不新建。
2. 项目头 `＋` 在固定项目目录启动。执行 `cd` 到子目录后分屏，新 Pane 使用子目录，
   项目归属不变；全局新建继承当前项目。
3. 同 Tab 创建两个 Pane，分别点击侧栏，应准确聚焦；关闭其中一个仅移除该 Pane。
4. PowerShell 输入命令但不提交，预览不变；提交后更新。输入多行及长文本，预览最多两行。
5. Codex 保持默认 daemon，同一目录开两个客户端，分别提交不同中文及 emoji 消息；
   两个条目独立更新，草稿不更新。执行 `/new` 并确认位置后旧预览清除，新提交恢复。
6. OpenCode、Pi、Kimi 分别检查普通提交、粘贴未提交、多行和重复消息；Kimi 排队允许占位，
   排队结束及后续普通提交应恢复。退出 Agent 后不得留下旧 Agent 消息。
7. 项目折叠时运行一个稍长命令，展开后应已完成；置顶、取消置顶不打乱组内顺序。
8. 切换其他项目，在原项目修改 Git 分支，项目头最终应跟随固定目录刷新。
9. 检查浅深主题、窄侧栏、长项目名、长分支、滚动后点击及当前 Pane 高亮；状态和预览
   刷新不应推动条目跳动。

用户确认这些检查通过前，S6 保持“人工验收待完成”。
