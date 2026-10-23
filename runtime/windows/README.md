# Windows SQLite for LLLang

Native Windows builds link against a prebuilt SQLite DLL + import library.

## Layout

```
runtime/windows/
  include/sqlite3.h
  lib/sqlite3.lib
  bin/sqlite3.dll
```

These files are **not** committed (see `.gitignore`). Populate them with:

```powershell
pwsh ./scripts/fetch-sqlite-windows.ps1
```

CI release / `pack-release.ps1` runs the same fetch before packaging so the zip ships `runtime/windows/` for end users.

## End-user installs

`install.ps1` copies this tree under `%LOCALAPPDATA%\lang\toolchain\runtime\windows` and places `sqlite3.dll` next to built executables when `lang build` / `lang run` links.
