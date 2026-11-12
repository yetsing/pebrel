# 上游正式发布同步与自动验证

适用于 Windows 上的 `feature/moni-one` 及后续保留 Pane 侧栏的分支。
目标是同步上游已发布的稳定 Release，并验证本分支功能；发布标签可能来自独立发布分支，
不能用当前 `main`、版本号文件或提交日期代替它。预发布版本需另行明确指定。

固定流程为：GitHub **Sync fork** 同步 fork 的 `main` → 本地快进更新 `main` →
核对正式 Release 标签与完整 SHA → 将该 Release SHA 合入当前功能分支并验证。
远端和本地 `main` 跟踪上游完整进度，功能分支的合并边界始终是选定 Release。

代码块共享变量，应在仓库根目录、同一个独立 PowerShell 7 进程中依次执行。
使用已安装的 Git、GitHub CLI、
Node.js、uv/Python 3.12 和项目固定 Rust 工具链。Python 命令禁止自动下载运行时。
目录职责与基础检查以 [贡献指南](../CONTRIBUTING.md#local-checks) 和
[工程约束](project-constraints.md) 为准。本文不创建 CI 或服务端保护。

## 1．同步 fork、更新本地 main 并固定发布目标

先读根 `AGENTS.md`、`ROADMAP.md` 和涉及目录的规则。Sync fork 会更新远端 fork，
快进更新会移动本地 `main`，Release 合并会修改功能分支工作区；这些操作以及提交、推送
分别按用户实际授权执行并记录，不能把本节整体称为只读操作。
工作区不干净或存在未完成合并时，先识别并保留已有改动，不能自行 stash、清空或重置，
也不能直接切换 `main` 或开始另一轮合并。

用户明确要求清理并重来时，先备份工作树文件、状态清单、索引及合并元数据，再撤销旧合并。
备份路径清单应合并 `git diff --name-only HEAD`、`git ls-files -u` 中的冲突路径及需要保留的
未跟踪文件。不能只依赖前一个命令：冲突处理后内容与 HEAD 相同的文件可能不在其输出中。
手册、进度记录和手工修复也须保存；核对备份后，才按授权逐项恢复对应路径。
不使用全仓 `clean` 或硬重置代替状态识别，重试时使用新的验证目录。

### 1.1．记录功能分支并检查前置状态

```powershell
$ErrorActionPreference = 'Stop'
if (git status --porcelain) { throw 'Worktree is not clean' }
$before = git rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Not a Git checkout' }
$branch = git branch --show-current
if ($LASTEXITCODE -ne 0 -or !$branch -or $branch -eq 'main') {
    throw 'Start from the feature branch, not main or a detached HEAD'
}
git rev-parse -q --verify MERGE_HEAD > $null
if ($LASTEXITCODE -eq 0) { throw 'A merge is already pending' }
$runDir = Join-Path (Get-Location) ('.tmp/upstream-sync-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $runDir | Out-Null
$upstream = 'Kuddev/pebrel'
$fork = 'yetsing/pebrel' # 使用其他 fork 时显式修改，并核对 origin 指向。
$upstreamUrl = "https://github.com/$upstream.git"
git remote -v
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect origin' }
```

### 1.2．通过 GitHub Sync fork 更新远端 main

核对 `origin` 指向自己的 fork。在 GitHub 打开 fork 仓库，选择 `main`，执行
**Sync fork → Update branch**，将上游 `Kuddev/pebrel` 的 `main` 同步到 fork 的 `main`。
若提示存在冲突或不能直接同步，停止并检查 fork 独有提交；不能用强制同步丢弃它们。
Sync fork 成功只表示远端 fork 已更新，不表示本地 `main` 或当前功能分支已经更新。

也可在获得相同远端写入授权后调用 GitHub 的非强制同步接口：

```powershell
gh api --method POST "repos/$fork/merge-upstream" -f branch=main
if ($LASTEXITCODE -ne 0) { throw 'GitHub Sync fork failed' }
```

返回 `merge_type: none` 且说明分支未落后，表示无需移动远端分支；仍应执行后续 SHA 核对。

### 1.3．快进更新本地 main，再返回原功能分支

以下代码核对 fork 与上游的当前 `main` SHA，获取 `origin/main` 并只允许快进更新。
若同步期间上游又有新提交，应重新 Sync fork 和获取，不使用旧快照声称已同步最新 `main`。

```powershell
$upstreamMain = gh api "repos/$upstream/commits/main" --jq .sha
if ($LASTEXITCODE -ne 0) { throw 'Upstream main lookup failed' }
$forkMain = gh api "repos/$fork/commits/main" --jq .sha
if ($LASTEXITCODE -ne 0 -or $forkMain -ne $upstreamMain) {
    throw 'Sync fork main on GitHub before updating local main'
}
git fetch origin 'refs/heads/main:refs/remotes/origin/main'
if ($LASTEXITCODE -ne 0) { throw 'Origin main fetch failed' }
if ((git rev-parse origin/main) -ne $forkMain) { throw 'Fetched origin/main differs from fork main' }
$mainBefore = git rev-parse main
if ($LASTEXITCODE -ne 0) { throw 'Local main is missing; inspect before proceeding' }
git merge-base --is-ancestor main origin/main
if ($LASTEXITCODE -ne 0) { throw 'Local main is missing, ahead or diverged; inspect before proceeding' }
git switch main
if ($LASTEXITCODE -ne 0) { throw 'Cannot switch to main' }
try {
    git merge --ff-only origin/main
    if ($LASTEXITCODE -ne 0) { throw 'Local main fast-forward failed' }
    if ((git rev-parse HEAD) -ne $forkMain) { throw 'Local main was not updated to fork main' }
} finally {
    git switch $branch
    if ($LASTEXITCODE -ne 0) { throw 'Cannot return to the original feature branch' }
}
if ((git rev-parse HEAD) -ne $before) { throw 'Feature HEAD changed while updating main' }
```

本地 `main` 缺失、领先或分叉时，先检查原因，不自动重置或改写历史。
返回功能分支后，其 `HEAD` 应保持不变；不能在这里执行 `git merge main` 或 `git merge origin/main`。

### 1.4．从上游正式 Release 固定合并提交

发布标签以真正的上游仓库为准；Sync fork 不保证 fork 拥有完整的 Release 元数据或标签。
标签即使位于独立发布分支，也使用下列核对后的 SHA，不改用 `main` 头提交。

```powershell
$releaseJson = gh api "repos/$upstream/releases/latest"
if ($LASTEXITCODE -ne 0) { throw 'Release lookup failed' }
$release = $releaseJson | ConvertFrom-Json
if ($release.draft -or $release.prerelease -or !$release.published_at) {
    throw 'Expected a published stable release'
}
$tag = $release.tag_name
if (!$tag) { throw 'Release has no tag' }
git fetch $upstreamUrl "refs/tags/$tag"
if ($LASTEXITCODE -ne 0) { throw 'Release tag fetch failed' }
$targetCommit = git rev-parse 'FETCH_HEAD^{commit}'
if ($LASTEXITCODE -ne 0) { throw 'Release tag is not a commit' }
$apiCommit = gh api "repos/$upstream/commits/$([Uri]::EscapeDataString($tag))" --jq .sha
if ($LASTEXITCODE -ne 0 -or $apiCommit -ne $targetCommit) {
    throw 'Fetched tag and GitHub commit disagree'
}
$releaseJson | Set-Content (Join-Path $runDir 'release.json') -Encoding utf8
@{ branch = $branch; before = $before; fork = $fork; main_before = $mainBefore; main = $forkMain
   upstream_main = $upstreamMain; tag = $tag
   target = $targetCommit; release_url = $release.html_url } |
    ConvertTo-Json | Set-Content (Join-Path $runDir 'baseline.json') -Encoding utf8
git log --oneline "$before..$targetCommit"
git diff --stat "$before...$targetCommit"
git merge-tree --write-tree --name-only $before $targetCommit > (Join-Path $runDir 'merge-preview.log')
if ($LASTEXITCODE -ne 0) { throw 'Review merge preview conflicts before merging' }
```

不要把 `FETCH_HEAD` 留作后续合并目标：下一次 fetch 会改变它，后续一律用已保存的
`$targetCommit`。`merge-tree` 不修改工作区，但无文本冲突不代表编译或功能兼容。
若网络不可用，不把旧缓存称为最新发布；记录阻塞点。若已包含该标签，跳过重复合并。

## 2．执行已授权的合并

```powershell
git merge-base --is-ancestor $targetCommit HEAD
$ancestryCode = $LASTEXITCODE
if ($ancestryCode -eq 0) {
    Write-Output 'Release already included; continue with validation only'
} elseif ($ancestryCode -eq 1) {
    git merge --no-commit --no-ff $targetCommit
    if ($LASTEXITCODE -ne 0) { throw 'Resolve merge conflicts before validation' }
    $mergeHead = git rev-parse --verify MERGE_HEAD
    if ($LASTEXITCODE -ne 0 -or $mergeHead -ne $targetCommit) { throw 'Wrong merge target' }
    if ((git rev-parse HEAD) -ne $before) { throw 'HEAD moved before commit authorization' }
    $unmerged = @(git diff --name-only --diff-filter=U)
    if ($LASTEXITCODE -ne 0 -or $unmerged.Count -gt 0) { throw 'Unresolved merge entries' }
} else {
    throw 'Cannot inspect release ancestry'
}
```

新执行合并时，`HEAD` 应仍为 `$before`，`MERGE_HEAD` 应等于 `$targetCommit`。未解决冲突必须为空。
保留两侧功能合同，不能整文件选一侧来绕过冲突。不要顺带合并 `main` 或挑入未发布修复。
需要额外提交时，先说明其范围并取得对应授权。

工作树已无冲突标记与 Git 索引已确认解决是两个状态。使用 `--no-commit` 时，可先基于
处理后的工作树验证；若当前授权不包含暂存和提交，明确记录剩余索引冲突，不宣称合并已提交。
获准提交后精确暂存解决后的文件，确认 `git ls-files -u` 为空、暂存差异检查通过，再提交。

## 3．自动检查入口

以下 helper 记录日志、立即处理非零退出码，并拒绝零用例的 Rust 测试结果。
先设置隔离配置；在单独 PowerShell 进程运行，结束后环境变量不会留在用户终端。

```powershell
$env:PEBREL_CONFIG_DIR = Join-Path $runDir 'unit-config'
function Invoke-Check {
    param([string]$Name, [scriptblock]$Command, [switch]$RustTests)
    $log = Join-Path $runDir "$Name.log"
    & $Command *> $log
    $code = $LASTEXITCODE
    if ($code -ne 0) { throw "$Name failed ($code); see $log" }
    if ($RustTests -and !(Select-String -Path $log -Pattern 'test result: ok\. [1-9][0-9]* passed;')) {
        throw "$Name ran no passing Rust tests; inspect filters and features"
    }
}
Invoke-Check architecture {
    uv run --no-project --no-python-downloads --python 3.12 scripts/check_architecture.py --base $targetCommit
}
Invoke-Check governance {
    uv run --no-project --no-python-downloads --python 3.12 -m unittest scripts.tests.test_architecture_budgets scripts.tests.test_architecture_dependencies scripts.tests.test_architecture_governance
}
Invoke-Check product {
    cargo check -p nebula --bin pebrel --features gpui-shell --tests --locked
}
Invoke-Check feature-tests {
    cargo test -p nebula --bin pebrel --features gpui-shell,gpui-test-support --locked -- pane_groups pane_sidebar input_preview ai_hook:: assistant_answer:: input::terminal_input platform::ai_session_identity platform::project_directory platform::private_file workspace::windowing::transfer_tests workspace::tab_drag workspace::tab_duplication workspace::settings_navigation::ui_tests workspace::terminal_activity::tests --test-threads=1
} -RustTests
Invoke-Check bridges {
    node --test scripts/tests/opencode-notification-bridge.test.mjs scripts/tests/pi-notification-bridge.test.mjs
}
Invoke-Check settings { cargo test -p nebula-settings --locked } -RustTests
Invoke-Check i18n { cargo test --manifest-path tools/i18n-contract/Cargo.toml --locked } -RustTests
Invoke-Check line-budget { cargo test -p nebula --features gpui-shell --test file_line_budget --locked } -RustTests
Invoke-Check whitespace { git diff --check }
```

`gpui-test-support` 必须启用，否则原生控件测试会被条件编译移除。
检查日志中的实际测试名称、数量、忽略项与失败项，不能只看进程返回 0。
过滤器失效后，helper 会拒绝全部零用例；部分过滤器失效仍需对照实际名称核对。
至少确认以下三项存在并通过，不固定总数，因为上游可能增加或重命名测试：

- `complete_hooks_own_every_state_even_when_screen_disagrees`。
- `density_changes_real_tab_bounds_and_sidebar_drag_pitch`。
- `sidebar_spinner_follows_real_collapse_and_window_focus`。

上述测试覆盖本分支关键路径，不代替上游全平台矩阵。根据实际 diff 增补受影响的测试。
构建和测试顺序执行，共享同一 Cargo 缓存；不要为普通依赖编译等待重复启动构建。
不同 feature 组合可能分别编译 GPUI；日志暂时不更新不能证明构建卡死。对话中断后先读取
已有任务的日志和完成回执，确认任务是否仍运行，不直接启动第二组 Cargo 检查。
只有源码、feature、配置及目标未变且有完整成功结果时才复用已完成检查；部分日志或旧的
`ready.json` 不算完成证据。后续改动仅影响单元测试时，重跑相关测试及工程检查即可，
不重复执行与改动无关且已经通过的原生窗口检查。

格式检查执行 `cargo fmt --all -- --check` 并保留结果。已有 CRLF 导致失败时，
核对其是否在本次改动之外，不批量格式化来清障；另对修改的 Rust 文件使用
`rustfmt --check --edition 2024 --config newline_style=Auto <files>` 检查。
若默认检查只报告换行问题，可进一步执行并保留以下全仓结果：

```powershell
Invoke-Check format-auto { cargo fmt --all -- --check --config newline_style=Auto }
```

它仍检查全仓格式，只按文件已有换行风格比较，不修改文件或仓库配置。报告分别说明
默认命令、显式 Auto 全仓命令、修改文件检查的结果；不能用局部通过冒充全仓通过。

## 4．获取本次编译的原生测试程序

不能复用记忆中的 `target/debug/deps/pebrel-<hash>.exe`。通过 Cargo JSON 产物信息获取：

```powershell
Invoke-Check native-build {
    cargo test -p nebula --bin pebrel --features gpui-shell,gpui-test-support --locked --no-run --message-format=json
}
$artifacts = Get-Content (Join-Path $runDir 'native-build.log') | ForEach-Object {
    try { $_ | ConvertFrom-Json -ErrorAction Stop } catch { }
}
$executables = @($artifacts | Where-Object {
    $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'pebrel' -and
    $_.profile.test -and $_.executable
} | Select-Object -ExpandProperty executable -Unique)
if ($executables.Count -ne 1 -or !(Test-Path -LiteralPath $executables[0])) {
    throw 'Expected exactly one current Pebrel test executable'
}
$testExe = $executables[0]
```

JSON 日志可能夹有编译器消息；这里只忽略非 JSON 行，构建错误已由退出码拦截。
不要通过文件时间排序选择旧程序，也不要把构建缓存当作当前源码已验证的证据。

## 5．隔离原生窗口验证

需要 Windows 桌面；窗口全部来自测试夹具，不操作用户已有窗口。权威入口是
[原生侧栏夹具](../nebula_app/src/gpui_shell/workspace/pane_sidebar/native_tests.rs)。
每次创建新目录，不能复用已有 `ready.json`。以下片段自动执行 Nord/Paper × 220/300
logical px 四组真实控件交互，使用真实 CMD PTY 和独立 Git 项目。

```powershell
function Wait-NativeJson {
    param([string]$Path, [scriptblock]$Predicate, [int]$Seconds = 15)
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if ($fixture.HasExited) { throw "Fixture exited early: $($fixture.ExitCode)" }
        $value = $null
        try { $value = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json -ErrorAction Stop } catch { }
        if ($null -ne $value -and (& $Predicate $value)) { return $value }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Native fixture timeout: $Path"
}
foreach ($theme in @('dark', 'light')) {
    foreach ($width in @(220, 300)) {
        $qa = Join-Path $runDir "$theme-$width"
        New-Item -ItemType Directory -Path $qa | Out-Null
        $env:PEBREL_CONFIG_DIR = Join-Path $qa 'config'
        $env:PEBREL_SIDEBAR_QA_DIR = $qa
        $env:PEBREL_SIDEBAR_QA_THEME = $theme
        $env:PEBREL_SIDEBAR_QA_NARROW = if ($width -eq 220) { '1' } else { '0' }
        $env:PEBREL_SIDEBAR_QA_PROJECTS = '1'
        $env:PEBREL_SIDEBAR_QA_PREVIEW = '1'
        $fixture = Start-Process -FilePath $testExe -ArgumentList @(
            '--ignored', '--exact', 'gpui_shell::workspace::pane_sidebar::native_tests::native_project_sidebar', '--nocapture'
        ) -WindowStyle Hidden -PassThru -RedirectStandardOutput "$qa-stdout.log" -RedirectStandardError "$qa-stderr.log"
        try {
            $ready = Wait-NativeJson (Join-Path $qa 'ready.json') { param($s) $s.hwnd } 30
            $statePath = Join-Path $qa 'state.json'
            $state = Wait-NativeJson $statePath { param($s) $s.branches[0].branch -and $s.branches[1].branch }
            Set-Content (Join-Path $qa 'collapse-project') 'click'
            $state = Wait-NativeJson $statePath { param($s) $s.stage -eq 1 -and $s.running_verified }
            if ($state.panes -contains $ready.panes[0] -or $state.panes -contains $ready.panes[1]) {
                throw 'Collapsed panes are still visible'
            }
            git -C $ready.projects[0] symbolic-ref HEAD refs/heads/feature/updated-fixed-project
            if ($LASTEXITCODE -ne 0) { throw 'Fixture branch update failed' }
            $state = Wait-NativeJson $statePath { param($s) $s.branches[0].branch -eq 'feature/updated-fixed-project' }
            foreach ($step in @(@('pin-project', 2), @('expand-project', 3), @('select-pane', 4), @('close-pane', 5), @('unpin-project', 6))) {
                Set-Content (Join-Path $qa $step[0]) 'click'
                $expected = [int]$step[1]
                $state = Wait-NativeJson $statePath { param($s) $s.stage -ge $expected }
            }
            if ($state.tabs -ne 2 -or $state.panes.Count -ne 2 -or $state.panes -contains $ready.panes[1]) {
                throw 'Wrong pane closed'
            }
            $state | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $qa 'verified.json') -Encoding utf8
        } finally {
            Set-Content (Join-Path $qa 'capture-complete') 'finish own fixture'
            if (!$fixture.WaitForExit(15000)) {
                Stop-Process -Id $fixture.Id
                throw 'Own fixture did not exit normally'
            }
            if ($fixture.ExitCode -ne 0) { throw "Native fixture failed: $($fixture.ExitCode)" }
        }
    }
}
```

夹具内部验证置顶顺序、恢复顺序、精确 Pane 选择及单 Pane 关闭。外部驱动补充检查
折叠后 PTY 继续执行和固定目录分支刷新。所有 marker 文件只触发夹具的真实控件事件。
这里的预览由测试协议注入，不能作为四种 CLI 真实采集链路的证明。

上述片段不自动截图。需要视觉核对时，在各 marker 推进前，按 `ready.json` 的 `hwnd`
只截取夹具窗口，并检查深浅主题、长项目名、同行分支、图钉、两行预览及选中竖线。
状态 JSON 写入可能早于新帧呈现，先等待渲染稳定再截图；窗口打开不等于视觉验收。
增加截图停顿时注意夹具自身约 60 秒的交互循环，不能无限等待人工操作。
无桌面或权限被拒绝时，标记原生窗口检查未执行，不反复重试或换用用户窗口。

## 6．按实际差异处理兼容问题

| 症状 | 核对与处理 |
| --- | --- |
| 无文本冲突，编译出现重复方法 | 核对双方是否把同一方法迁到不同模块；保留单一实现及双方新增行为 |
| 调用参数与接口不匹配 | 对比发布标签中的定义和本分支调用；同步测试夹具初始化，不能只修普通产品编译 |
| `sidebar-tab-0` 等元素不存在 | Windows 改用实际 Pane/Document ID；其他平台保留自己的 Tab 合同，不增加生产兼容选择器 |
| 拖动或点击测试不稳定 | 对照实际控件、行高、拖柄与当前几何；固定测试布局和动画，验证点击前后选择、释放后的重排 |
| OpenCode 无结果的停止事件得到 `Idle` | 对照[状态合同](../architecture/notes/nebula_app/ai_hook/2026-11-02-explicit-status-authority.md)；无结果与成功分开测试，不能改实现强行显示成功 |
| 缺少 ConPTY 侧载文件 | 区分编译警告与真实 PTY 验证；产品包必须按[构建指南](windows-portable-build.md)保留侧载运行库 |
| PowerShell 提示符路径断言失败 | 区分协议中的绝对 cwd 与提示符中的 `~` 显示；测试显式覆盖主目录映射，不依赖运行机器的用户目录 |
| OpenCode 活动回合中的回复不再触发恢复 | 检查是否先报告对应等待；覆盖无等待、有效等待及重复回复，不能把“回合活动”代替“已报告等待” |
| Pane 行高测试与原生标题栏断言冲突 | 保留 Windows Pane ID、固定行高和拖柄重排断言，同时合入上游原生控件可见时的标题栏中心与边界容差 |

若差异涉及终端核心，可额外执行 `cargo test -p nebula_terminal --locked`。
其中真实虚拟环境用例支持通过 `PEBREL_QA_VENV_ACTIVATE` 提供已有激活脚本，避免依赖
Windows 的 `python` 命令是否为有效运行时。该用例要求标准 CPython `venv` 的 `-Prompt`
接口，不能用 `uv venv` 生成的另一种激活脚本替代；可在本次隔离目录准备：

```powershell
uv run --no-project --no-python-downloads --python 3.12 -m venv --without-pip (Join-Path $runDir 'cpython-venv')
if ($LASTEXITCODE -ne 0) { throw 'CPython venv fixture creation failed' }
$env:PEBREL_QA_VENV_ACTIVATE = Join-Path $runDir 'cpython-venv/Scripts/Activate.ps1'
Invoke-Check terminal { cargo test -p nebula_terminal --locked } -RustTests
```

修复前稳定复现；同一方向连续两轮未解决时，回查源码、可运行测试与正常夹具。
不删断言、跳过测试、放宽预算或用旧程序取得绿色结果。

## 7．交付与提交前核对

- 记录发布页面、标签、目标 SHA、原分支 SHA、合并后 SHA（如已提交）、实际修改及必要取舍。
- 分别记录 GitHub Sync fork 结果、本地 `main` 更新前后 SHA、返回的功能分支及其未变的 HEAD。
  本地 `main` 更新完成不代表已将其全部变更合入功能分支。
- 核对合并提交的两个父提交；发布目标应是其中之一。可另获取同一上游 `main`，
  使用第 1 节保存的 `$upstreamMain` 列出 `$targetCommit..$upstreamMain`，逐项检查发布边界外的提交没有被意外引入；
  对本分支原本已包含的提交单独说明，不能重写历史来隐藏它们。
- 记录每组命令、退出码、实际测试数、失败与忽略项、隔离配置及原生窗口结果。
  有失败或未执行项时，不能写“全部正常”；旧断言失败也须记录和修复。
- 若改动涉及实际 CLI 接入或提交边界，按 [S6 验收合同](../PANE_SIDEBAR_S6.md)
  重测真实 CLI、草稿、多行、重复消息、空 Enter、新会话、返回 Shell 和同目录双 Pane。
  没有重测就明确列为未覆盖，不复用旧 `.tmp/` 环境或历史人工结果代替本轮验收。
- 更新 `ROADMAP.md` 的当前状态和验证记录；README 只同步已改变的当前事实。
  日志和截图放在本次 `$runDir`，文档内项目路径使用相对路径，不提交临时产物或私有数据。
- 用户要求提交时，精确暂存代码和受影响文档，使用中文提交信息；验证工作区及提交内容。
  没有推送、发布或部署授权时，到本地交付为止。

报告建议包含“目标发布、合并范围、自动检查、原生交互、真实 CLI、遗留问题、提交/推送状态”。

## 8．v2.5.3 同步实例（2026-11-12）

以下是合并提交 `57092f0a13f09354386d5e1106d43e85354876f2` 的已核验结果，作为流程实例；
版本、SHA 和测试数量不能直接当作下次同步的目标或验收结果。

| 对象 | 本次结果 |
| --- | --- |
| 正式发布 | [v2.5.3](https://github.com/Kuddev/pebrel/releases/tag/v2.5.3)，目标 `07eb3a7a9e5d23ef4cb104a51436eb3aa3f330d4` |
| GitHub Sync fork | 确认父仓库为 `Kuddev/pebrel`，同步接口返回 `merge_type: none`，fork 与上游 main 的 SHA 一致 |
| 本地 main | 从 `f3201403` 快进到 `8f69d3327000976bcc56442ffaedfcd8a1e24159` |
| 功能分支原提交 | `8fdcc5f19512624f28eff41a406676214ae89a28`，更新 main 后保持不变 |
| 发布边界 | main 相对发布目标额外的 43 个提交原先也不在功能分支中，本次未纳入 |
| 合并父提交 | 第一父提交为原功能分支 HEAD，第二父提交为正式 Release SHA；提交后工作区干净，未推送 |

初次操作留下未完成合并，按用户要求保存备份并撤销后，重新执行 Sync fork、本地 main
快进及 Release 合并。10 个文件存在文本冲突，按块审阅而非整文件选边：保留 Pane 归属
规则及真实选择器、布局和重排断言，合入上游窗口恢复指引与原生几何检查；保留生命周期
结果分类覆盖，采用上游 OpenCode 已报告等待的回复准入和完成去重规则。

最终产品 check、架构检查通过。治理测试执行 62 项，其中 3 项跳过；功能回归 319 项通过、
3 项忽略；桥接 31 项、设置 107 项、国际化 24 项（另 1 项忽略）、行数 2 项及补全 35 项通过。
终端单元测试 393 项通过、2 项忽略，全部终端集成及文档测试通过。四组原生窗口均完成
全部交互阶段、通过测试并正常退出。默认格式检查因换行风格失败，70 个改动文件检查及
显式 Auto 全仓格式检查通过，未批量改动换行。

补充终端检查时，提示符测试曾把主目录显示误当成绝对路径，修正为显式覆盖两种显示。
真实 venv 测试先因本机 `python` 入口不可用失败，随后发现 uv 夹具与所需 `-Prompt`
接口不匹配；改用标准 CPython 3.12 venv 激活脚本后全套终端测试通过。失败日志与最终
成功回执均保留在 `.tmp/upstream-resync-20261112/`，汇总为其中的 `verification.json`；
这些临时文件仅是本机证据，手册执行不依赖它们。

本轮未重测四种真实 CLI 的消息接受链路，未构建或人工验收新便携包，也未执行上游
全平台矩阵。自动测试与隔离窗口通过不能替代这些验收。
