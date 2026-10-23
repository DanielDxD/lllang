# Download official SQLite prebuilt DLL (+ generate import lib) into runtime/windows/.
# Usage: pwsh ./scripts/fetch-sqlite-windows.ps1
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out = Join-Path $Root "runtime\windows"
$Tmp = Join-Path $env:TEMP "lllang-sqlite-fetch"
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Out "include") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Out "lib") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Out "bin") | Out-Null

$Year = "2024"
$DllZip = "sqlite-dll-win-x64-3460100.zip"
$SrcZip = "sqlite-amalgamation-3460100.zip"
$Base = "https://www.sqlite.org/$Year"

function Get-Zip([string]$name) {
    $dest = Join-Path $Tmp $name
    if (-not (Test-Path $dest)) {
        Write-Host "Downloading $name ..."
        Invoke-WebRequest -Uri "$Base/$name" -OutFile $dest
    }
    return $dest
}

$dllPath = Get-Zip $DllZip
$srcPath = Get-Zip $SrcZip

$DllExtract = Join-Path $Tmp "dll"
$SrcExtract = Join-Path $Tmp "src"
Remove-Item -Recurse -Force $DllExtract, $SrcExtract -ErrorAction SilentlyContinue
Expand-Archive -Path $dllPath -DestinationPath $DllExtract -Force
Expand-Archive -Path $srcPath -DestinationPath $SrcExtract -Force

$dllFile = Get-ChildItem -Path $DllExtract -Filter "sqlite3.dll" -Recurse | Select-Object -First 1
if (-not $dllFile) { throw "sqlite3.dll not found in $DllZip" }
Copy-Item $dllFile.FullName (Join-Path $Out "bin\sqlite3.dll") -Force

$header = Get-ChildItem -Path $SrcExtract -Filter "sqlite3.h" -Recurse | Select-Object -First 1
if (-not $header) { throw "sqlite3.h not found" }
Copy-Item $header.FullName (Join-Path $Out "include\sqlite3.h") -Force

$libOut = Join-Path $Out "lib\sqlite3.lib"
$defFile = Get-ChildItem -Path $DllExtract -Filter "sqlite3.def" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
$libCandidate = Get-ChildItem -Path $DllExtract -Filter "sqlite3.lib" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1

if ($libCandidate) {
    Copy-Item $libCandidate.FullName $libOut -Force
} elseif ($defFile) {
    $made = $false
    if (Get-Command lib.exe -ErrorAction SilentlyContinue) {
        & lib.exe "/def:$($defFile.FullName)" "/out:$libOut" /machine:x64
        $made = (Test-Path $libOut)
    }
    if (-not $made -and (Get-Command dlltool.exe -ErrorAction SilentlyContinue)) {
        & dlltool.exe -d $defFile.FullName -l $libOut -D sqlite3.dll
        $made = (Test-Path $libOut)
    }
    if (-not $made -and (Get-Command llvm-dlltool -ErrorAction SilentlyContinue)) {
        & llvm-dlltool -d $defFile.FullName -l $libOut -D sqlite3.dll
        $made = (Test-Path $libOut)
    }
    if (-not $made) {
        throw "Could not generate sqlite3.lib from .def (need lib.exe, dlltool, or llvm-dlltool)"
    }
} else {
    # Last resort: static amalgamation object as the "import" lib (no DLL needed at runtime then).
    Write-Host "No .def/.lib in DLL zip; compiling amalgamation statically..."
    $cFile = Get-ChildItem -Path $SrcExtract -Filter "sqlite3.c" -Recurse | Select-Object -First 1
    if (-not $cFile) { throw "sqlite3.c not found" }
    $obj = Join-Path $Tmp "sqlite3.obj"
    & clang -c $cFile.FullName -o $obj -O2 -DSQLITE_OMIT_LOAD_EXTENSION
    if ($LASTEXITCODE -ne 0) { throw "clang failed compiling sqlite3.c" }
    if (Get-Command llvm-ar -ErrorAction SilentlyContinue) {
        & llvm-ar rcs $libOut $obj
    } elseif (Get-Command llvm-lib -ErrorAction SilentlyContinue) {
        & llvm-lib "/out:$libOut" $obj
    } else {
        throw "need llvm-ar or llvm-lib to archive sqlite3.obj"
    }
}

Write-Host "SQLite Windows artifacts ready under $Out"
Get-ChildItem -Recurse $Out | ForEach-Object { $_.FullName }
