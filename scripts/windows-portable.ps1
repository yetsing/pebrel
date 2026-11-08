#Requires -Version 7.4
<#
.SYNOPSIS
Push the current commit, wait for its Windows portable build, download and extract it.
.EXAMPLE
pwsh -File scripts/windows-portable.ps1
.EXAMPLE
pwsh -File scripts/windows-portable.ps1 -RunId 42441741184
#>
[CmdletBinding()]
param(
    [ValidateRange(1, [long]::MaxValue)][long] $RunId,
    [ValidateRange(1, 240)][int] $TimeoutMinutes = 130
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-PortableCommand([string] $Command, [string[]] $Arguments) {
    $output = & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command 执行失败，退出码为 $LASTEXITCODE。" }
    return $output
}

function Get-PortableJson([string] $Endpoint) {
    (Invoke-PortableCommand gh @('api', $Endpoint) | Out-String) | ConvertFrom-Json
}

function Assert-PortableRun($Run, [string] $Commit, [string] $Branch, [long] $WorkflowId) {
    if ($Run.head_sha -ne $Commit -or $Run.head_branch -ne $Branch -or $Run.workflow_id -ne $WorkflowId) {
        throw '运行的提交、分支或工作流与当前目标不一致，停止下载。'
    }
}

function Wait-PortableRun([string] $Repository, [string] $Commit, [string] $Branch,
    [long] $WorkflowId, [long] $SelectedRunId, [datetime] $Deadline) {
    $discoveryDeadline = [datetime]::UtcNow.AddMinutes(3)
    $encodedBranch = [uri]::EscapeDataString($Branch)
    while ([datetime]::UtcNow -lt $Deadline) {
        if ($SelectedRunId) {
            $run = Get-PortableJson "repos/$Repository/actions/runs/$SelectedRunId"
            Assert-PortableRun $run $Commit $Branch $WorkflowId
            Write-Host "构建：$($run.html_url) [$($run.status)/$($run.conclusion)]"
            if ($run.status -eq 'completed') {
                if ($run.conclusion -ne 'success') { throw "构建未成功：$($run.conclusion)。$($run.html_url)" }
                return $run
            }
        } else {
            $runs = Get-PortableJson "repos/$Repository/actions/workflows/$WorkflowId/runs?branch=$encodedBranch&head_sha=$Commit&event=push&per_page=50"
            $matches = @($runs.workflow_runs | Where-Object {
                $_.head_sha -eq $Commit -and $_.head_branch -eq $Branch -and $_.event -eq 'push'
            } | Sort-Object id -Descending)
            if ($matches.Count) {
                $SelectedRunId = $matches[0].id
                continue
            }
            if ([datetime]::UtcNow -ge $discoveryDeadline) {
                throw '3 分钟内未找到该提交的 push 构建，请检查工作流触发分支及 Actions 状态。'
            }
            Write-Host '等待 GitHub 创建当前提交的 Windows 构建……'
        }
        Start-Sleep -Seconds 15
    }
    throw '等待构建超时；远端运行未被取消，可稍后使用 -RunId 继续。'
}

function Save-PortableArtifact([string] $Repository, [long] $ArtifactId, [string] $Destination) {
    # PowerShell 7.4+ preserves native stdout bytes through redirection.
    $gh = (Get-Command gh -CommandType Application -ErrorAction Stop).Source
    & $gh api "repos/$Repository/actions/artifacts/$ArtifactId/zip" > $Destination
    if ($LASTEXITCODE -ne 0) { throw "下载 artifact 失败，退出码为 $LASTEXITCODE。" }
}

function Expand-PortableArchive([string] $Archive, [string] $Destination, [string[]] $ExpectedFiles) {
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $prefix = [IO.Path]::GetFullPath($Destination).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
        $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($entry in $zip.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            $target = [IO.Path]::GetFullPath((Join-Path $Destination $name))
            if ($name.Contains(':') -or -not $target.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw "ZIP 路径超出解压目录：$name"
            }
            if (-not $name.EndsWith('/')) {
                if (-not $names.Add($name)) { throw "ZIP 包含重复文件：$name" }
            }
        }
        $difference = @(Compare-Object @($ExpectedFiles | Sort-Object) @($names | Sort-Object))
        if ($difference.Count -or $names.Count -ne $ExpectedFiles.Count) { throw 'ZIP 文件清单与打包合同不一致。' }
    } finally {
        $zip.Dispose()
    }
    # The destination is new; never replace files from an earlier invocation.
    [IO.Compression.ZipFile]::ExtractToDirectory($Archive, $Destination)
}

function Invoke-WindowsPortable([long] $SelectedRunId = 0, [int] $WaitMinutes = 130) {
    $repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
    $previousEncoding = [Console]::OutputEncoding
    [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
    $OutputEncoding = [Console]::OutputEncoding
    Push-Location $repoRoot
    try {
        $null = Get-Command git, gh -ErrorAction Stop
        $branch = (Invoke-PortableCommand git @('symbolic-ref', '--quiet', '--short', 'HEAD') | Out-String).Trim()
        $commit = (Invoke-PortableCommand git @('rev-parse', 'HEAD') | Out-String).Trim()
        if ($commit -notmatch '^[0-9a-f]{40}$' -or -not $branch) { throw '需要有效的分支和完整 HEAD 提交。' }
        $remote = (Invoke-PortableCommand git @('remote', 'get-url', '--push', 'origin') | Out-String).Trim()
        if ($remote -notmatch '^(?:git@github\.com:|ssh://git@github\.com/|https://github\.com/)(?<repository>[\w.-]+/[\w.-]+?)(?:\.git)?$') {
            throw 'origin 的推送地址必须为 github.com 的 SSH 或 HTTPS 仓库地址。'
        }
        $repository = $Matches.repository
        $workflow = Get-PortableJson "repos/$repository/actions/workflows/windows-portable.yml"
        if (-not $SelectedRunId -and $workflow.state -ne 'active') { throw 'Windows 便携包工作流未启用，停止推送。' }
        $dirty = @(Invoke-PortableCommand git @('status', '--porcelain'))
        if ($dirty.Count) { Write-Warning '工作区有未提交改动；本次仅处理已提交的 HEAD，不自动提交。' }
        Write-Host "目标：$repository，$branch，$commit"
        if (-not $SelectedRunId) {
            # Pin the SHA even if another terminal moves the local branch during the push.
            Invoke-PortableCommand git @('push', 'origin', "${commit}:refs/heads/$branch") | Out-Host
            $encodedBranch = [uri]::EscapeDataString($branch)
            $remoteRef = Get-PortableJson "repos/$repository/git/ref/heads/$encodedBranch"
            if ($remoteRef.object.sha -ne $commit) { throw '推送后远端分支与目标提交不一致，停止等待。' }
        }
        $run = Wait-PortableRun $repository $commit $branch $workflow.id $SelectedRunId ([datetime]::UtcNow.AddMinutes($WaitMinutes))
        $artifacts = Get-PortableJson "repos/$repository/actions/runs/$($run.id)/artifacts?per_page=100"
        $matching = @($artifacts.artifacts | Where-Object { $_.name -eq "Pebrel-windows-x64-$commit" })
        if ($matching.Count -ne 1) { throw '未找到唯一的当前提交 Windows artifact。' }
        $artifact = $matching[0]
        if ($artifact.expired) { throw '构建产物已过期，需要重新构建。' }
        if ($artifact.digest -notmatch '^sha256:[0-9a-fA-F]{64}$') { throw 'GitHub 未提供有效的 artifact SHA256，停止下载。' }

        # Derive the manifest from the same committed packager used by Actions.
        $packager = Invoke-PortableCommand git @('show', "${commit}:scripts/package-release.ps1") | Out-String
        $manifest = [regex]::Match($packager, '(?ms)^\$manifest = \[ordered\]@\{(?<body>.*?)^\}')
        $expectedFiles = @([regex]::Matches($manifest.Groups['body'].Value, "(?m)^\s*'([^']+)'\s*=") | ForEach-Object { $_.Groups[1].Value })
        if (-not $manifest.Success -or -not $expectedFiles.Count) { throw '无法读取已提交的打包清单。' }
        $directoryName = "run-$($run.id)-$([datetime]::UtcNow.ToString('yyyyMMdd-HHmmss'))-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
        $output = Join-Path $repoRoot ".tmp/windows-portable-$($commit.Substring(0, 8))/$directoryName"
        $null = New-Item -ItemType Directory -Path $output
        Write-Host "下载及解压目录：$output"
        $run | ConvertTo-Json -Depth 20 | Set-Content (Join-Path $output 'run.json') -Encoding utf8
        $artifact | ConvertTo-Json -Depth 20 | Set-Content (Join-Path $output 'artifact.json') -Encoding utf8
        $archivePath = Join-Path $output 'github-artifact.zip'
        Save-PortableArtifact $repository $artifact.id $archivePath
        $archiveHash = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ("sha256:$archiveHash" -ne $artifact.digest) { throw '下载的 artifact SHA256 与 GitHub 不一致。' }
        if ((Get-Item $archivePath).Length -ne $artifact.size_in_bytes) { throw '下载的 artifact 大小与 GitHub 不一致。' }
        $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
        try {
            $files = @($archive.Entries | Where-Object { -not $_.FullName.EndsWith('/') })
            if ($files.Count -ne 1 -or $files[0].FullName -notmatch '^Pebrel-v(?<version>[0-9A-Za-z][0-9A-Za-z.-]*)-windows-x64\.zip$') {
                throw 'artifact 必须且只能包含一个 Windows x64 便携 ZIP。'
            }
            $packageName = $files[0].FullName
            $packageVersion = $Matches.version
        } finally { $archive.Dispose() }
        $download = Join-Path $output 'download'
        Expand-PortableArchive $archivePath $download @($packageName)
        $packagePath = Join-Path $download $packageName
        $packageHash = (Get-FileHash $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
        $buildLog = Invoke-PortableCommand gh @('run', 'view', [string]$run.id, '--repo', $repository, '--log') | Out-String
        $buildLog | Set-Content (Join-Path $output 'build.log') -Encoding utf8
        $plainLog = $buildLog -replace '\x1b\[[0-9;]*m', ''
        if ($plainLog -notmatch "(?im)\bSHA256\s*:\s*$packageHash\b") { throw '便携 ZIP 的 SHA256 与构建日志不一致。' }
        $extracted = Join-Path $output 'extracted'
        Expand-PortableArchive $packagePath $extracted $expectedFiles
        . (Join-Path $PSScriptRoot 'windows-package-architecture.ps1')
        Assert-WindowsPackageArchitecture -Root $extracted -RuntimeDirectory (Join-Path $extracted 'runtime') -Architecture x64 -CliExecutable pebrel.com
        $cli = Join-Path $extracted 'pebrel.com'
        $version = (Invoke-PortableCommand $cli @('--version') | Out-String).Trim()
        if ($version -notmatch "^Pebrel $([regex]::Escape($packageVersion)) \((?<revision>[0-9a-f]{7,40})\)$" -or -not $commit.StartsWith($Matches.revision)) {
            throw "程序版本与目标提交不一致：$version"
        }
        $helpText = Invoke-PortableCommand $cli @('--help') | Out-String
        if ($helpText -notmatch '--gpui') { throw '下载的程序缺少 GPUI 产品入口。' }
        [ordered]@{
            repository = $repository; branch = $branch; commit = $commit; run_url = $run.html_url
            artifact_id = $artifact.id; artifact_sha256 = $archiveHash; package_sha256 = $packageHash
            package = "download/$packageName"; executable = 'extracted/pebrel.exe'
            version = $version; architecture = 'x64'; manifest_files = $expectedFiles
            verified_at = [datetime]::UtcNow.ToString('o')
        } | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $output 'verification.json') -Encoding utf8
        Write-Host "完成：$version"
        [pscustomobject]@{
            RunUrl = $run.html_url; Zip = $packagePath
            Executable = Join-Path $extracted 'pebrel.exe'; SHA256 = $packageHash
        }
    } finally {
        Pop-Location
        [Console]::OutputEncoding = $previousEncoding
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    $result = Invoke-WindowsPortable -SelectedRunId $RunId -WaitMinutes $TimeoutMinutes
    Write-Host "ZIP：$($result.Zip)"
    Write-Host "程序：$($result.Executable)"
    Write-Host "SHA256：$($result.SHA256)"
}
