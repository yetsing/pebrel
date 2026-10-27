# Windows 分支便携包构建

[`Windows portable ZIP`](../.github/workflows/windows-portable.yml)为分支人工验证构建 Windows x64 便携 ZIP。
它复用现有 GPUI 产品构建与打包脚本，保留二进制新鲜度、产品身份、架构和 ZIP 清单检查。
此入口上传 Actions Artifacts，不创建 Release 或标签，不更新正式发布资产。

## 触发与下载

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
