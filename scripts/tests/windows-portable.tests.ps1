#Requires -Version 7.4
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot '../windows-portable.ps1')
$nativeCommand = ${function:Invoke-PortableCommand}
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$fixture = Join-Path $repoRoot ('.tmp/windows-portable-tests-' + [guid]::NewGuid().ToString('N'))
$script:Commit = ([guid]::NewGuid().ToString('N') + '12345678')
$ownedOutput = Join-Path $repoRoot ".tmp/windows-portable-$($script:Commit.Substring(0, 8))"
if (Test-Path $ownedOutput) { throw 'Test output already exists' }
$script:Passed = 0
$script:Packager = @'
$manifest = [ordered]@{
    'pebrel.exe' = Join-Path $targetRoot 'pebrel.exe'
    'pebrel.com' = Join-Path $targetRoot 'pebrel-cli.exe'
    'runtime/pebrel-hook.exe' = Join-Path $targetRoot 'pebrel-hook.exe'
    'runtime/conpty.dll' = Join-Path $targetRoot 'conpty.dll'
    'runtime/OpenConsole.exe' = Join-Path $targetRoot 'OpenConsole.exe'
}
'@

function Reset-Fixture {
    $script:Failure = ''
    $script:Pushes = @()
    $script:Polls = 0
    $script:Sleeps = 0
}

function Assert-True($Condition, [string] $Message) {
    if (-not $Condition) { throw $Message }
}

function Assert-Failure([scriptblock] $Action, [string] $Pattern) {
    $receiptCount = @(Get-ChildItem $ownedOutput -Recurse -Filter verification.json -ErrorAction SilentlyContinue).Count
    $caught = $null
    try { & $Action | Out-Null } catch { $caught = $_ }
    if ($null -eq $caught -or $caught.Exception.Message -notmatch $Pattern) {
        throw "Expected failure '$Pattern', got '$caught'"
    }
    Assert-True (@(Get-ChildItem $ownedOutput -Recurse -Filter verification.json -ErrorAction SilentlyContinue).Count -eq $receiptCount) 'Failure wrote a success receipt'
    $script:Passed++
}

function Invoke-PortableCommand([string] $Command, [string[]] $Arguments) {
    if ($Command -eq 'git') {
        switch ($Arguments[0]) {
            'symbolic-ref' { return 'feature/moni-one' }
            'rev-parse' { return $script:Commit }
            'remote' { return 'git@github.com:owner/example.git' }
            'status' { return ' M ROADMAP.md' }
            'show' { return $script:Packager }
            'push' {
                if ($script:Failure -eq 'push') { throw 'push rejected' }
                $script:Pushes += ,$Arguments
                return
            }
        }
    }
    if ($Command -eq 'gh') {
        if ($Arguments[0] -eq 'run') {
            if ($script:Failure -eq 'log') { return 'unrelated checksum' }
            # Real Actions logs include PowerShell ANSI styling before the digest.
            return "$([char]27)[32;1mSHA256$([char]27)[0m : $([char]27)[0m$script:PackageHash"
        }
        $endpoint = $Arguments[1]
        $value = switch -Regex ($endpoint) {
            '/workflows/windows-portable.yml$' { @{ id = 42; state = 'active' }; break }
            '/git/ref/heads/' { @{ object = @{ sha = $script:Commit } }; break }
            '/workflows/42/runs\?' {
                @{ workflow_runs = @(
                    @{ id = 999; head_sha = 'unrelated'; head_branch = 'feature/moni-one'; event = 'push' },
                    @{ id = 77; head_sha = $script:Commit; head_branch = 'feature/moni-one'; event = 'push' }
                ) }; break
            }
            '/runs/77$' {
                $script:Polls++
                $result = @{ id = 77; head_sha = $script:Commit; head_branch = 'feature/moni-one'; workflow_id = 42
                    html_url = 'https://github.com/owner/example/actions/runs/77'; status = 'completed'; conclusion = 'success' }
                switch ($script:Failure) {
                    'commit' { $result.head_sha = 'wrong' }
                    'branch' { $result.head_branch = 'other' }
                    'workflow' { $result.workflow_id = 43 }
                    'cancelled' { $result.conclusion = 'cancelled' }
                    'build' { $result.conclusion = 'failure' }
                }
                if ($script:Polls -eq 1 -and -not $script:Failure) { $result.status = 'in_progress'; $result.conclusion = $null }
                $result; break
            }
            '/runs/77/artifacts\?' {
                $item = @{ id = 88; name = "Pebrel-windows-x64-$script:Commit"; expired = $false
                    digest = "sha256:$script:ArchiveHash"; size_in_bytes = (Get-Item $script:ArchivePath).Length }
                if ($script:Failure -eq 'expired') { $item.expired = $true }
                if ($script:Failure -eq 'digest') { $item.digest = $null }
                $items = @($item)
                if ($script:Failure -eq 'missing') { $items = @() }
                if ($script:Failure -eq 'duplicate') { $items = @($item, $item) }
                @{ artifacts = $items }; break
            }
            default { throw "Unexpected endpoint: $endpoint" }
        }
        return $value | ConvertTo-Json -Depth 10 -Compress
    }
    if ($Command.EndsWith('pebrel.com')) {
        if ($Arguments[0] -eq '--help') {
            if ($script:Failure -eq 'legacy') { return 'legacy shell' }
            return 'help --gpui'
        }
        if ($script:Failure -eq 'version') { return 'Pebrel 9.8.7 (0000000)' }
        return "Pebrel 9.8.7 ($($script:Commit.Substring(0, 7)))"
    }
    throw "Unexpected command: $Command $Arguments"
}

function Save-PortableArtifact([string] $Repository, [long] $ArtifactId, [string] $Destination) {
    Assert-True ($Repository -eq 'owner/example' -and $ArtifactId -eq 88) 'Wrong download target'
    if ($script:Failure -eq 'corrupt') { [IO.File]::WriteAllText($Destination, 'corrupt'); return }
    Copy-Item -LiteralPath $script:ArchivePath -Destination $Destination
}

function Start-Sleep { param([int] $Seconds) $script:Sleeps++ }

function New-TestArchive([string] $Path, [string[]] $Names) {
    $zip = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $Names) {
            $entry = $zip.CreateEntry($name)
            $writer = [IO.StreamWriter]::new($entry.Open())
            try { $writer.Write('fixture') } finally { $writer.Dispose() }
        }
    } finally { $zip.Dispose() }
}

try {
    $payload = Join-Path $fixture 'payload'
    $null = New-Item -ItemType Directory -Path (Join-Path $payload 'runtime')
    $bytes = [byte[]]::new(96)
    [BitConverter]::GetBytes([uint16]0x5A4D).CopyTo($bytes, 0)
    [BitConverter]::GetBytes([uint32]64).CopyTo($bytes, 60)
    [BitConverter]::GetBytes([uint32]0x4550).CopyTo($bytes, 64)
    [BitConverter]::GetBytes([uint16]0x8664).CopyTo($bytes, 68)
    foreach ($name in @('pebrel.exe', 'pebrel.com', 'runtime/pebrel-hook.exe', 'runtime/conpty.dll', 'runtime/OpenConsole.exe')) {
        [IO.File]::WriteAllBytes((Join-Path $payload $name), $bytes)
    }
    $outer = Join-Path $fixture 'outer'
    $null = New-Item -ItemType Directory -Path $outer
    $package = Join-Path $outer 'Pebrel-v9.8.7-windows-x64.zip'
    [IO.Compression.ZipFile]::CreateFromDirectory($payload, $package)
    $script:PackageHash = (Get-FileHash $package).Hash.ToLowerInvariant()
    $script:ArchivePath = Join-Path $fixture 'artifact.zip'
    [IO.Compression.ZipFile]::CreateFromDirectory($outer, $script:ArchivePath)
    $script:ArchiveHash = (Get-FileHash $script:ArchivePath).Hash.ToLowerInvariant()

    Reset-Fixture
    $first = Invoke-WindowsPortable
    Assert-True ($script:Pushes.Count -eq 1) 'Default mode must push once'
    Assert-True (($script:Pushes[0] -join ' ') -eq "push origin ${script:Commit}:refs/heads/feature/moni-one") 'Push must pin the SHA without force'
    Assert-True ($script:Polls -eq 2 -and $script:Sleeps -eq 1) 'Must wait for the matching build'
    Assert-True (Test-Path $first.Zip) 'ZIP missing'
    Assert-True (Test-Path $first.Executable) 'Extraction missing'
    $script:Passed++

    [IO.File]::WriteAllText($first.Executable, 'preserve previous extraction')
    Reset-Fixture
    $second = Invoke-WindowsPortable -SelectedRunId 77
    Assert-True ($script:Pushes.Count -eq 0) 'RunId mode must not push'
    Assert-True ($first.Zip -ne $second.Zip) 'Repeated runs must use a new directory'
    Assert-True ([IO.File]::ReadAllText($first.Executable) -eq 'preserve previous extraction') 'Previous files changed'
    $script:Passed++

    foreach ($case in @(
        @('push', 'push rejected'), @('commit', '不一致'), @('branch', '不一致'), @('workflow', '不一致'),
        @('cancelled', 'cancelled'), @('build', 'failure'), @('expired', '过期'),
        @('missing', '唯一'), @('duplicate', '唯一'), @('digest', '有效的 artifact SHA256'),
        @('corrupt', 'SHA256 与 GitHub 不一致'), @('log', 'SHA256 与构建日志不一致'),
        @('version', '程序版本'), @('legacy', 'GPUI')
    )) {
        Reset-Fixture
        $script:Failure = $case[0]
        Assert-Failure { Invoke-WindowsPortable } $case[1]
    }
    Reset-Fixture
    Assert-Failure { Wait-PortableRun 'owner/example' $script:Commit 'feature/moni-one' 42 77 ([datetime]::UtcNow.AddSeconds(-1)) } '超时'
    Assert-Failure { & $nativeCommand (Get-Command pwsh).Source @('-NoProfile', '-Command', 'exit 7') } '退出码为 7'

    $validZip = Join-Path $fixture 'valid.zip'
    New-TestArchive $validZip @('folder with space/readme.txt')
    $validOutput = Join-Path $fixture 'valid-output'
    Expand-PortableArchive $validZip $validOutput @('folder with space/readme.txt')
    Assert-True (Test-Path (Join-Path $validOutput 'folder with space/readme.txt')) 'Valid nested ZIP rejected'
    $script:Passed++
    foreach ($case in @(
        @('traversal', @('../escaped.txt'), @('../escaped.txt'), 'ZIP 路径'),
        @('duplicate-path', @('a.txt', 'A.txt'), @('a.txt'), '重复文件'),
        @('unexpected', @('extra.txt'), @('required.txt'), '文件清单')
    )) {
        $path = Join-Path $fixture "$($case[0]).zip"
        New-TestArchive $path $case[1]
        Assert-Failure { Expand-PortableArchive $path (Join-Path $fixture "$($case[0])-output") $case[2] } $case[3]
    }
    Assert-True (-not (Test-Path (Join-Path $fixture 'escaped.txt'))) 'ZIP escaped the destination'
    Write-Output "windows-portable.tests.ps1: PASS ($script:Passed cases)"
} finally {
    foreach ($path in @($fixture, $ownedOutput)) {
        $resolved = [IO.Path]::GetFullPath($path)
        $prefix = [IO.Path]::GetFullPath((Join-Path $repoRoot '.tmp')).TrimEnd('\') + '\windows-portable-'
        if (-not $resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test cleanup path' }
        if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
    }
}
