# WSL 后台准备沿用 Pane 的启动器

## Status

Implemented。Windows 验证与未完成范围见 [路线图](../../../../../ROADMAP.md)。

## Context

WSL 复制回归使用不存在的 `missing/wsl.exe`，仅检查发行版、用户、目录和新 Pane 身份。
后台 Hook 准备与来宾 Shell 探测重新使用系统 `wsl.exe`，因此假启动器未隔离这些副作用。

## Evidence

- [复制测试](../../../../../nebula_app/src/gpui_shell/workspace/tab_duplication/layout_tests.rs)使用 `FrozenDistro` 和 `worker`，没有要求操作实际来宾。
- [WSL Hook](../../../../../nebula_app/src/platform/wsl_hooks/windows.rs)与
  [来宾探测](../../../../../nebula_app/src/platform/wsl_guest_shell.rs)都由 Pane 启动路径触发。
- 2026-10-27 本机 Windows 回归的 23 项断言通过后，测试进程退出，仍遗留一个
  `wsl.exe --distribution FrozenDistro --user worker --exec sh -lc "exec python3 -"`。
  其唯一线程保持 Suspended，且持有外层 PowerShell 创建的两条输出管道写句柄。
  托管线程栈确认 PowerShell 阻塞在 NativeCommandProcessor 的输出队列收尾。
- [Windows 管道规则](https://learn.microsoft.com/en-us/windows/win32/ipc/pipe-handle-inheritance)要求所有写端关闭后，读取方才能观察到输出结束。

## Decision

Hook 与来宾探测保留原 Pane 的启动器程序，并将其纳入各自缓存键。
不同启动器路径不能共享“已尝试安装”或来宾探测结果，即使发行版、用户相同。
WSL 参数继续使用现有解析和参数生成规则；来宾探测通过共享 `wsl_exec_command_with` 指定启动器。
显式 `setup-ai --wsl` 保留系统 `wsl.exe` 默认入口。

测试的假启动器现在会按原路径返回 NotFound，不会隐式改用系统 WSL。
回归先断言命令程序路径，再尝试不存在的程序；旧行为在发起真实 WSL 前就会失败。
不通过修改用户设置、关闭全部 Hook、跳过测试或进程名批量终止来维持测试通过。

## Rejected alternatives

- 仅关闭挂起的 PowerShell：丢失现场，并保留绕过假启动器的启动路径。
- 在测试中修改全局 `ai_hooks` 设置：影响其他测试与用户环境，且来宾探测仍可能发生。
- 忽略未提交输出或给整个测试加一个强制成功超时：无法验证真实退出和子进程清理。
- 为此重写通用 Windows 启动器：该层的创建/Job 接管竞态是独立生命周期问题，须单独设计和验证。

## Consequences

显式 WSL 启动器与自动准备保持一致，假的或缺失的启动器按其自身失败处理。
此改动没有增加持久化字段、依赖、后台线程或配置项，也不改变发行版和用户解析。
缓存仍有原有数量与冷却上限。

诊断还发现 [进程适配](../../../../../nebula_app/src/platform/process.rs)先挂起创建子进程，
之后才绑定清理 Job 的窗口。现场与测试主进程在该窗口退出的竞态吻合，但精确中断时序未重现。
本次修复启动器隔离，不宣称已修复这一通用创建竞态。

## Validation

Hook 与来宾探测分别覆盖原启动器路径、缺失启动器的 NotFound、参数与环境继承。
Windows 相关回归 33 项通过，命令实际返回 0；原 WSL 复制测试独立运行 3 轮均正常退出，未新增 WSL 遗留。
精确终止旧测试的遗留 WSL 后，原 PowerShell 自行退出并返回 0，验证了输出管道占用的直接因果。
编译与其他检查结果见路线图。

## Supersedes

None；补充既有 WSL Hook 传输与来宾探测的启动器身份边界。

## Revisit when

需要由一个启动器刻意切换到另一个启动器执行后台准备，或改造 Windows 子进程创建与 Job 接管时。
