#!/usr/bin/env bash
# Pack `lang` binary + runtime archive + docs into dist/lang-<ver>-<triple>.tar.gz with SHA256SUMS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
VERSION="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')"
OUT_DIR="$ROOT/dist"
STAGE="$OUT_DIR/stage"
NAME="lang-${VERSION}-${TRIPLE}"

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/lib/$TRIPLE" "$STAGE/runtime" "$STAGE/docs"

BIN="$ROOT/target/release/lang"
if [[ ! -x "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build -p lang --release" >&2
  exit 1
fi

cp "$BIN" "$STAGE/bin/lang"
chmod +x "$STAGE/bin/lang"

RT_ARCHIVE="$(
  find "$ROOT/target/release/build" -name 'liblllang_rt.a' 2>/dev/null | head -1
)"
if [[ -z "$RT_ARCHIVE" || ! -f "$RT_ARCHIVE" ]]; then
  echo "error: liblllang_rt.a not found under target/release/build" >&2
  exit 1
fi
cp "$RT_ARCHIVE" "$STAGE/lib/$TRIPLE/liblllang_rt.a"
# Legacy flat path for older installers / LANG_RUNTIME_LIB fallbacks.
cp "$RT_ARCHIVE" "$STAGE/lib/liblllang_rt.a"

cp -R "$ROOT/runtime/." "$STAGE/runtime/"
# Unix archives do not need Windows sqlite binaries.
rm -rf "$STAGE/runtime/windows/bin" "$STAGE/runtime/windows/lib" "$STAGE/runtime/windows/include" 2>/dev/null || true

cat > "$STAGE/README.txt" << EOF
LLLang ${VERSION} (${TRIPLE})

Contents:
  bin/lang                          CLI (includes LSP via \`lang lsp\`)
  lib/<triple>/liblllang_rt.a       Prebuilt C runtime for this host triple
  lib/liblllang_rt.a                Same archive (legacy flat layout)
  runtime/                          ARC runtime sources + headers
  docs/                             Getting-started install notes

Multi-target: \`lang build --target linux/amd64\` (etc.) finalizes portable \`.bll\`
bitcode for the chosen triple. Cross-link needs a runtime under
lib/<triple>/ (or \$LANG_HOME/toolchain/lib/<triple>/).

Install (manual):
  1. Extract to ~/.lang/toolchain (or any prefix)
  2. export PATH="\$PWD/bin:\$PATH"
  3. lang --version

One-liner:
  curl -fsSL https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.sh | bash

Host deps for \`lang build\` / \`lang run\`: clang (or Xcode CLT on macOS) and libsqlite3.
LLVM 22 is only required when building the compiler from source.

See docs/getting-started/installation.md
EOF
cp "$ROOT/docs/getting-started/installation.md" "$STAGE/docs/" 2>/dev/null || true

mkdir -p "$OUT_DIR"
ARCHIVE="$OUT_DIR/${NAME}.tar.gz"
tar -C "$STAGE" -czf "$ARCHIVE" .

SUM_FILE="$OUT_DIR/${NAME}.sha256"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && sha256sum "$(basename "$ARCHIVE")" > "$(basename "$SUM_FILE")")
else
  (cd "$OUT_DIR" && shasum -a 256 "$(basename "$ARCHIVE")" > "$(basename "$SUM_FILE")")
fi
# Maintain a combined SHA256SUMS for local convenience (last pack wins per filename).
touch "$OUT_DIR/SHA256SUMS"
grep -v " $(basename "$ARCHIVE")\$" "$OUT_DIR/SHA256SUMS" > "$OUT_DIR/SHA256SUMS.tmp" || true
cat "$SUM_FILE" >> "$OUT_DIR/SHA256SUMS.tmp"
mv "$OUT_DIR/SHA256SUMS.tmp" "$OUT_DIR/SHA256SUMS"

echo "Wrote $ARCHIVE"
echo "Wrote $OUT_DIR/SHA256SUMS"
cat "$OUT_DIR/SHA256SUMS"
