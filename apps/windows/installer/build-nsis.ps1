<#
.SYNOPSIS
    在 Windows 上打轻书（LightBookInput）NSIS 安装包：两个架构各自构建、签名、用 makensis 编脚本。
.DESCRIPTION
    步骤：
      1) cargo build 出 DLL / Server / 设置程序三件套，x64 与 32 位各一份（32 位 Windows 也需要 Server 与设置程序）；
      2) 按 settings-runtime.txt 从两个 target 目录挑出自包含 Windows App Runtime；
      3) 铺两个暂存目录 target\installer\stage-{x86_64,x86}\，TSF DLL 按版本改名（见 lightbookinput.nsi 头部说明）；
      4) 生成自签名证书并 signtool 签名四个源二进制；
      5) makensis 编两遍：64 位包（/DBUILD_X64，同时带 32 位 DLL）与 32 位包；
      6) 签名两个安装包，成品在 target\installer\。
    随包数据（.qj / .tsv / .qjm）由 lightbookinput.nsi 直接从仓库 data 与 assets 里取，不另建暂存目录；
    打包前确保 data\generated 与 data\models 已就绪（bundle 流程见 CLAUDE.md）。

    版本号：默认取 apps\windows\server\Cargo.toml，-dev 版接 git 短哈希；CI 用 LIGHTBOOKINPUT_VERSION
    覆盖成 1.0.20261006 这种「语义版本 + 日期」形式。
.PARAMETER SkipBuild
    跳过 cargo build（数据或 .nsi 改了、二进制没变时重编安装包用）。
.PARAMETER NoSign
    跳过自签名（本机没 Windows SDK 或只想先看打包流程时用）。
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$NoSign
)

$ErrorActionPreference = 'Stop'

$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$NsiScript = Join-Path $PSScriptRoot 'lightbookinput.nsi'

# 对外分发关 uiAccess（自签证书不被别的机器信任，uiAccess=true 起不来）。
$env:LIGHTBOOKINPUT_UIACCESS = '0'

# 架构表：两个包各自一条。Target 为空即本机默认 target（x64）。
$ArchList = @(
    @{ Name = 'x86_64'; Target = $null;                  IsX64 = $true }
    @{ Name = 'x86';    Target = 'i686-pc-windows-msvc'; IsX64 = $false }
)

function Get-ArchOutDir {
    param([string]$ArchName)
    $arch = $ArchList | Where-Object { $_.Name -eq $ArchName }
    if ($arch.Target) { return "target\$($arch.Target)\release" }
    return 'target\release'
}

# ---------- 1) 构建三件套 ----------
if (-not $SkipBuild) {
    Write-Host '构建 release 产物…' -ForegroundColor Cyan
    Push-Location $Repo
    try {
        foreach ($arch in $ArchList) {
            $targetArg = if ($arch.Target) { @('--target', $arch.Target) } else { @() }
            Write-Host "  $($arch.Name)…" -ForegroundColor DarkCyan
            cargo build --release --locked -p lightbookinput-windows-server -p lightbookinput-windows-tsf -p lightbookinput-windows-settings @targetArg
            if ($LASTEXITCODE -ne 0) { throw "cargo build 失败（$($arch.Name)，退出码 $LASTEXITCODE）" }
        }
    } finally { Pop-Location }
}

# 缺一个产物就早报错。
$produced = @()
foreach ($arch in $ArchList) {
    $outDir = Get-ArchOutDir $arch.Name
    foreach ($name in @('lightbookinput_tsf.dll', 'lightbookinput-server.exe', 'lightbookinput-settings.exe')) {
        $p = Join-Path $Repo "$outDir\$name"
        if (-not (Test-Path -LiteralPath $p)) { throw "缺产物 $p，先跑一次不带 -SkipBuild 的构建" }
        $produced += @{ Path = $p; Arch = $arch.Name }
    }
}

# ---------- 2) 版本号 ----------
$Version = $env:LIGHTBOOKINPUT_VERSION
if (-not $Version) {
    $cargoToml = Get-Content (Join-Path $Repo 'apps\windows\server\Cargo.toml')
    $verLine = $cargoToml | Where-Object { $_ -match '^\s*version\s*=\s*"(.+)"' } | Select-Object -First 1
    if (-not ($verLine -match '"(.+)"')) { throw '在 apps\windows\server\Cargo.toml 里没找到 version' }
    $Version = $Matches[1]
    # 开发版接 git 短哈希（1.0.0-dev-1a2b3c4，脏加 +），有 bug 能定位到哪次改动。
    if ($Version.EndsWith('-dev')) {
        Push-Location $Repo
        try {
            $rev = (git rev-parse --short HEAD 2>$null)
            if ($LASTEXITCODE -eq 0 -and $rev) {
                if (git status --porcelain 2>$null) { $rev = "$rev+" }
                $Version = "$Version-$rev"
            }
        } finally { Pop-Location }
    }
}
# VIProductVersion 只认 a.b.c.d 四点数字：1.0.20261006 -> 1.0.2026.1006。
$versionCore = ($Version -split '-')[0]
$parts = $versionCore -split '\.'
if ($parts.Count -ge 4) {
    $VersionNumeric = ($parts[0..3]) -join '.'
} elseif ($parts.Count -eq 3 -and $parts[2].Length -ge 6) {
    # 日期部分 yyyymmdd 拆成 yyyy.mmdd，凑够四段又不丢信息。
    $VersionNumeric = "$($parts[0]).$($parts[1]).$($parts[2].Substring(0, 4)).$($parts[2].Substring(4))"
} else {
    $VersionNumeric = "$versionCore.0"
}
Write-Host "版本 $Version（VIProductVersion $VersionNumeric）" -ForegroundColor Cyan

# ---------- 3) 自包含 Windows App Runtime ----------
# settings.exe 不依赖机器上装的框架包（见 apps\windows\settings\build.rs），cargo 构建时
# windows-reactor-setup 已按清单把运行时铺到各 target 的 release\，这里挑进暂存目录。
$runtimeList = Join-Path $PSScriptRoot 'settings-runtime.txt'
$wanted = Get-Content $runtimeList -Encoding UTF8 | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object { $_.Trim() }
foreach ($arch in $ArchList) {
    $srcDir = Join-Path $Repo (Get-ArchOutDir $arch.Name)
    $runtimeStage = Join-Path $Repo "target\installer\settings-runtime-$($arch.Name)"
    if (Test-Path -LiteralPath $runtimeStage) { Remove-Item -LiteralPath $runtimeStage -Recurse -Force }
    New-Item -ItemType Directory -Path $runtimeStage -Force | Out-Null
    $missing = @()
    foreach ($name in $wanted) {
        $src = Join-Path $srcDir $name
        if (Test-Path -LiteralPath $src) {
            Copy-Item -LiteralPath $src -Destination (Join-Path $runtimeStage $name) -Recurse -Force
        } else {
            $missing += $name
        }
    }
    # 缺文件说明自包含运行时没铺成功（NuGet 下载或 MSIX 解压失败），早报错，别打出个跑不起来的包。
    if ($missing.Count -gt 0) { throw "$($arch.Name) 自包含 Windows App Runtime 缺 $($missing.Count) 项：$($missing -join ', ')" }
    Write-Host "  $($arch.Name) 运行时 $($wanted.Count) 项 -> target\installer\settings-runtime-$($arch.Name)" -ForegroundColor DarkCyan
}

# ---------- 4) 铺暂存目录 ----------
# DLL 起名策略与原 Inno 一致：64 位包 <版本>.dll + <版本>-x86.dll，32 位包只有 <版本>.dll。
foreach ($arch in $ArchList) {
    $stage = Join-Path $Repo "target\installer\stage-$($arch.Name)"
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null

    $outDir = Join-Path $Repo (Get-ArchOutDir $arch.Name)
    Copy-Item -LiteralPath (Join-Path $outDir 'lightbookinput-settings.exe') -Destination $stage -Force
    Copy-Item -LiteralPath (Join-Path $outDir 'lightbookinput-server.exe') -Destination $stage -Force
    Copy-Item -LiteralPath (Join-Path $Repo 'apps\windows\tsf\resources\lightbookinput.ico') -Destination $stage -Force
    Copy-Item -LiteralPath (Join-Path $Repo "target\installer\settings-runtime-$($arch.Name)") -Destination (Join-Path $stage 'settings-runtime') -Recurse -Force

    if ($arch.IsX64) {
        Copy-Item -LiteralPath (Join-Path $outDir 'lightbookinput_tsf.dll') -Destination (Join-Path $stage "lightbookinput_tsf-$Version.dll") -Force
        $x86Out = Join-Path $Repo 'target\i686-pc-windows-msvc\release'
        Copy-Item -LiteralPath (Join-Path $x86Out 'lightbookinput_tsf.dll') -Destination (Join-Path $stage "lightbookinput_tsf-$Version-x86.dll") -Force
    } else {
        Copy-Item -LiteralPath (Join-Path $outDir 'lightbookinput_tsf.dll') -Destination (Join-Path $stage "lightbookinput_tsf-$Version.dll") -Force
    }
    Write-Host "  stage-$($arch.Name) 就绪" -ForegroundColor DarkCyan
}

# ---------- 5) 自签证书 ----------
function Find-SignTool {
    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $kit = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1
    if ($kit) { return $kit.FullName }
    foreach ($sdk in @("${env:ProgramFiles(x86)}\Windows Kits\10\bin", "${env:ProgramFiles}\Windows Kits\10\bin")) {
        if (-not (Test-Path -LiteralPath $sdk)) { continue }
        $found = Get-ChildItem -Path $sdk -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending | Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

$doSign = -not $NoSign
$signTool = if ($doSign) { Find-SignTool } else { $null }
$certPath = Join-Path $Repo 'target\installer\lightbookinput-cert.pfx'
$certPwd = 'lightbookinput'
$CertSubject = 'CN=LightBookInput (Mutantcat Working Group)'
if ($doSign -and -not $signTool) { throw '找不到 signtool.exe（Windows SDK）；不想签名就加 -NoSign' }

if ($doSign) {
    if (-not (Test-Path -LiteralPath $certPath)) {
        Write-Host '生成自签名证书…' -ForegroundColor Cyan
        $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $CertSubject -KeyUsage DigitalSignature -KeySpec Signature -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddYears(5)
        $certBytes = $cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx, $certPwd)
        [System.IO.File]::WriteAllBytes($certPath, $certBytes)
    }
    Write-Host "签名 $($produced.Count) 个源二进制（signtool $signTool）…" -ForegroundColor Cyan
    foreach ($item in $produced) {
        & $signTool sign /f $certPath /p $certPwd /fd sha256 /d "轻书 LightBookInput" $item.Path
        if ($LASTEXITCODE -ne 0) { throw "signtool 签名失败：$($item.Path)（退出码 $LASTEXITCODE）" }
    }
} else {
    Write-Host '跳过签名（-NoSign）。' -ForegroundColor Yellow
}

# ---------- 6) 找 makensis ----------
$makensis = $env:MAKENSIS
if (-not $makensis) {
    $candidates = @(
        "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
        "${env:ProgramFiles}\NSIS\makensis.exe"
    )
    $makensis = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $makensis) { $makensis = (Get-Command makensis.exe -ErrorAction SilentlyContinue).Source }
if (-not $makensis) { throw '找不到 makensis.exe：装 NSIS 或用 MAKENSIS 指定' }
Write-Host "用 $makensis" -ForegroundColor Cyan

# ---------- 7) 编两个架构的安装包 ----------
$outputs = @()
foreach ($arch in $ArchList) {
    $defines = @(
        "/DAppVersion=$Version"
        "/DAppVersionNumeric=$VersionNumeric"
        "/DRepo=$Repo"
        "/DAppArch=$($arch.Name)"
    )
    if ($arch.IsX64) { $defines += '/DBUILD_X64' }
    Write-Host "编译 $($arch.Name) 安装包…" -ForegroundColor Cyan
    & $makensis @defines $NsiScript
    if ($LASTEXITCODE -ne 0) { throw "makensis 失败（$($arch.Name)，退出码 $LASTEXITCODE）" }
    $outputs += Join-Path $Repo "target\installer\lightbookinput-$Version-windows-$($arch.Name)-setup.exe"
}

# ---------- 8) 签名安装包 ----------
if ($doSign) {
    foreach ($out in $outputs) {
        if (-not (Test-Path -LiteralPath $out)) { throw "makensis 说成了但没找到 $out" }
        & $signTool sign /f $certPath /p $certPwd /fd sha256 /d "轻书 LightBookInput" $out
        if ($LASTEXITCODE -ne 0) { throw "signtool 签名安装包失败：$out（退出码 $LASTEXITCODE）" }
    }
    Write-Host '安装包已签名（自签证书，只证明来源一致；Windows 仍会标未知发布者）。' -ForegroundColor Cyan
}

foreach ($out in $outputs) {
    $size = [math]::Round((Get-Item -LiteralPath $out).Length / 1MB, 1)
    Write-Host "完成：$out（$size MB）" -ForegroundColor Green
}
