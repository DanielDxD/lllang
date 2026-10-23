# Install a prebuilt lang.exe from GitHub Releases into %LOCALAPPDATA%\lang\toolchain
# Usage:
#   irm https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.ps1 | iex
#   .\install.ps1 -Version 0.2.0 -Prefix "$env:LOCALAPPDATA\lang\toolchain"
param(
    [string]$Version = "",
    [string]$Prefix = "",
    [switch]$NoModifyPath,
    [switch]$SkipDeps,
    [string]$Repo = "lllang-lang/llang"
)

$ErrorActionPreference = "Stop"

if (-not $Prefix) {
    $Prefix = Join-Path $env:LOCALAPPDATA "lang\toolchain"
}

$Triple = "x86_64-pc-windows-msvc"

function Install-HostDeps {
    if ($SkipDeps) { return }
    $clang = Get-Command clang -ErrorAction SilentlyContinue
    if (-not $clang) {
        Write-Host "Installing LLVM/Clang via winget…"
        winget install --id LLVM.LLVM -e --accept-package-agreements --accept-source-agreements
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("Path", "User")
    }
}

function Get-LatestTag {
    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $rel.tag_name
}

if (-not $Version) {
    Write-Host "Resolving latest release…"
    $Tag = Get-LatestTag
    $Version = $Tag.TrimStart("v")
} else {
    $Tag = "v$($Version.TrimStart('v'))"
    $Version = $Tag.TrimStart("v")
}

$Asset = "lang-$Version-$Triple.zip"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
$SumsUrl = "https://github.com/$Repo/releases/download/$Tag/SHA256SUMS"

Write-Host "Installing LLLang $Version ($Triple) → $Prefix"

$Tmp = Join-Path $env:TEMP "lllang-install-$(Get-Random)"
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
try {
    $ZipPath = Join-Path $Tmp $Asset
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath

    try {
        $SumsPath = Join-Path $Tmp "SHA256SUMS"
        Invoke-WebRequest -Uri $SumsUrl -OutFile $SumsPath
        $line = Get-Content $SumsPath | Where-Object { $_ -match [regex]::Escape($Asset) } | Select-Object -First 1
        if ($line) {
            $expected = ($line -split '\s+')[0].ToLower()
            $actual = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLower()
            if ($expected -ne $actual) {
                throw "SHA256 mismatch for $Asset (expected $expected, got $actual)"
            }
        }
    } catch {
        Write-Warning "SHA256 verify skipped or failed: $_"
    }

    if (Test-Path $Prefix) {
        # Keep store/cache elsewhere; replace toolchain tree.
        Remove-Item -Recurse -Force (Join-Path $Prefix "bin"), (Join-Path $Prefix "lib"), (Join-Path $Prefix "runtime"), (Join-Path $Prefix "docs") -ErrorAction SilentlyContinue
    }
    New-Item -ItemType Directory -Force -Path $Prefix | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $Prefix -Force

    Install-HostDeps

    $BinDir = Join-Path $Prefix "bin"
    if (-not $NoModifyPath) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$BinDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$BinDir;$userPath", "User")
            Write-Host "Added $BinDir to user PATH"
        }
        $env:Path = "$BinDir;$env:Path"
    }

    Write-Host ""
    & (Join-Path $BinDir "lang.exe") --version
    Write-Host "Installed. Open a new terminal if PATH was just updated."
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
