# Windows 分支便携包构建

[`Windows portable ZIP`](../.github/workflows/windows-portable.yml)为分支人工验证构建 Windows x64 便携 ZIP。
它复用现有 GPUI 产品构建与打包脚本，保留二进制新鲜度、产品身份、架构和 ZIP 清单检查。
此入口上传 Actions Artifacts，不创建 Release 或标签，不更新正式发布资产。

## 一键推送、构建、下载与解压

Windows 下使用 PowerShell 7.4 或更新版本，安装 Git 和 GitHub CLI，并先通过 `gh auth login`
登录有权推送和读取 Actions 产物的账号。脚本从 `origin` 的 GitHub SSH 或 HTTPS 推送地址识别仓库。

在项目根目录执行：

```powershell
pwsh -File scripts/windows-portable.ps1
```

[`windows-portable.ps1`](../scripts/windows-portable.ps1)执行以下步骤：

1. 固定当前分支的完整 `HEAD`，普通推送至 `origin` 的同名分支。存在未提交改动时提示，
   不自动提交；本地改动不会进入构建。当前自动触发分支为 `feature/moni-one`。
2. 只等待当前仓库、工作流、分支及提交匹配的 push 运行。重复执行时复用该提交已有的
   最新 push 运行；3 分钟内没有找到运行则报错，不会自动触发或重跑其他构建。
3. 构建成功后，下载以完整 SHA 命名的唯一 artifact，核对 GitHub 提供的大小及 SHA256，
   再核对便携 ZIP 的 SHA256 与构建日志。
4. 在新目录解压，核对该提交的打包文件清单、x64 二进制架构、程序版本与提交号以及
   GPUI 帮助入口，最后输出 ZIP 路径、程序路径及 SHA256。

输出位于 `.tmp/windows-portable-<提交前八位>/run-<运行编号>-<时间>-<随机后缀>/`：

- `download/`：便携 ZIP。
- `extracted/`：解压后的完整程序，可自行运行 `pebrel.exe`。
- `github-artifact.zip`、`run.json`、`artifact.json`、`build.log`、`verification.json`：
  下载原件、来源及验证回执。仅全部检查成功后才生成 `verification.json`。

每次执行使用独立目录，不覆盖旧包，也不自动启动或替换正在运行的 Pebrel。
失败时保留已经下载的文件供排查。构建失败、取消、摘要不一致或产物过期均以非零退出码结束。

已有运行编号时，可跳过推送，继续等待、下载和解压；运行仍必须匹配当前分支和 `HEAD`：

```powershell
pwsh -File scripts/windows-portable.ps1 -RunId 42441741184
```

默认等待上限为 130 分钟，休眠时间计入上限；可用 `-TimeoutMinutes 180` 修改。
超时或退出脚本不会取消远端构建，可随后使用对应的 `-RunId` 继续。

## 手动触发与下载

1. 将代码和工作流提交并推送到 `feature/moni-one`。每次推送自动构建该提交，
   同分支的新运行会取消尚未完成的旧运行。
2. 在仓库 **Actions → Windows portable ZIP** 中打开对应运行，核对分支与提交 SHA。
3. 构建成功后，在 **Artifacts** 下载 `Pebrel-windows-x64-<完整提交 SHA>`。
   下载的 artifact 归档中包含 `Pebrel-v<当前 Cargo 版本>-windows-x64.zip`，保留 14 天。
4. 再解压其中的便携 ZIP 到单独目录，运行该目录的 `pebrel.exe`，进行人工验证。
   文件名中的版本来自 Cargo，具体源码以运行和 artifact 名称中的提交 SHA 为准。

手动运行使用 **Run workflow**，在 **Branch** 中选择 `feature/moni-one`。
该入口也可以构建其他已包含此工作流的分支。
GitHub 要求手动触发的工作流先存在于默认分支，才会提供手动入口；
默认分支尚未包含本文件时，先使用上面的分支推送触发。
见 [GitHub 手动运行工作流文档](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)。

仅下载和解压不会自动替换正在运行的 Pebrel。人工验收需自行启动对应包并核对实际运行版本。
该工作流只负责构建和打包检查，不代替 **Full native tests**、终端行为回归或人工验收。
