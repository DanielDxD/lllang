# Pack lang.exe + runtime + Windows sqlite into dist/lang-<ver>-<triple>.zip
# Prerequisites: cargo build -p lang --release; scripts/fetch-sqlite-windows.ps1
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$Triple = (rustc -vV | Select-String "^host: ").ToString().Replace("host: ", "").Trim()
$Version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
$OutDir = Join-Path $Root "dist"
$Stage = Join-Path $OutDir "stage"
$Name = "lang-$Version-$Triple"

if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "lib") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "lib\$Triple") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "runtime") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "docs") | Out-Null

$Bin = Join-Path $Root "target\release\lang.exe"
if (-not (Test-Path $Bin)) {
    throw "missing $Bin — run: cargo build -p lang --release"
}
Copy-Item $Bin (Join-Path $Stage "bin\lang.exe")

# Ensure sqlite artifacts exist.
$Fetch = Join-Path $Root "scripts\fetch-sqlite-windows.ps1"
& $Fetch

$RtLib = Get-ChildItem -Path (Join-Path $Root "target\release\build") -Filter "lllang_rt.lib" -Recurse -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $RtLib) {
    throw "lllang_rt.lib not found under target/release/build"
}
Copy-Item $RtLib.FullName (Join-Path $Stage "lib\$Triple\lllang_rt.lib")
Copy-Item $RtLib.FullName (Join-Path $Stage "lib\lllang_rt.lib")

Copy-Item -Recurse (Join-Path $Root "runtime\*") (Join-Path $Stage "runtime") -Force
# Also put sqlite3.dll next to lang.exe for convenience.
$Dll = Join-Path $Root "runtime\windows\bin\sqlite3.dll"
if (Test-Path $Dll) {
    Copy-Item $Dll (Join-Path $Stage "bin\sqlite3.dll") -Force
}

@"
LLLang $Version ($Triple)

Contents:
  bin/lang.exe           CLI (LSP via ``lang lsp``)
  bin/sqlite3.dll        SQLite runtime (Windows)
  lib/<triple>/lllang_rt.lib  Prebuilt C runtime for this host
  lib/lllang_rt.lib            Legacy flat layout
  runtime/               Sources + windows/ sqlite headers & import lib
  docs/                  Install notes

Install:
  irm https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.ps1 | iex

Host deps: Clang (winget install LLVM.LLVM). LLVM 22 is only needed to build the compiler from source.
"@ | Set-Content -Path (Join-Path $Stage "README.txt") -Encoding UTF8

$InstallDoc = Join-Path $Root "docs\getting-started\installation.md"
if (Test-Path $InstallDoc) {
    Copy-Item $InstallDoc (Join-Path $Stage "docs\installation.md")
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Zip = Join-Path $OutDir "$Name.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip -Force

$hash = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLower()
$line = "$hash  $Name.zip"
$shaFile = Join-Path $OutDir "$Name.sha256"
Set-Content -Path $shaFile -Value $line -Encoding ASCII
$sums = Join-Path $OutDir "SHA256SUMS"
if (Test-Path $sums) {
    $existing = Get-Content $sums | Where-Object { $_ -notmatch [regex]::Escape("$Name.zip") }
    @($existing + $line) | Set-Content $sums -Encoding ASCII
} else {
    Set-Content -Path $sums -Value $line -Encoding ASCII
}

Write-Host "Wrote $Zip"
Write-Host "Wrote $shaFile"
Write-Host "Wrote $sums"
Get-Content $sums
