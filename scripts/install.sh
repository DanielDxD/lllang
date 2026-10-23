#!/usr/bin/env bash
# Install a prebuilt `lang` binary from GitHub Releases into ~/.lang/toolchain.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.sh | bash
#   curl … | bash -s -- --version 0.2.0 --prefix "$HOME/.lang/toolchain"
set -euo pipefail

REPO="${LANG_INSTALL_REPO:-lllang-lang/llang}"
PREFIX="${LANG_INSTALL_PREFIX:-$HOME/.lang/toolchain}"
VERSION=""
MODIFY_PATH=1
SKIP_DEPS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --no-modify-path) MODIFY_PATH=0; shift ;;
    --skip-deps) SKIP_DEPS=1; shift ;;
    -h|--help)
      cat <<EOF
Install LLLang (lang) from GitHub Releases.

Options:
  --version VER     Install tag vVER / asset for VER (default: latest)
  --prefix DIR      Install root (default: \$HOME/.lang/toolchain)
  --no-modify-path  Do not append PATH export to shell rc
  --skip-deps       Do not attempt to install clang / sqlite / Xcode CLT
EOF
      exit 0
      ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

detect_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) echo "unsupported macOS arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64|amd64) echo "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
        *) echo "unsupported Linux arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    *)
      echo "unsupported OS: $os (use install.ps1 on Windows)" >&2
      exit 1
      ;;
  esac
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command '$1' not found" >&2
    exit 1
  }
}

install_host_deps() {
  [[ "$SKIP_DEPS" -eq 1 ]] && return 0
  local os
  os="$(uname -s)"
  if [[ "$os" == "Darwin" ]]; then
    if ! xcode-select -p >/dev/null 2>&1; then
      echo "Installing Xcode Command Line Tools (clang)…"
      xcode-select --install || true
      echo "Complete the CLT installer GUI, then re-run this script if needed."
    fi
    if ! command -v sqlite3 >/dev/null 2>&1; then
      if command -v brew >/dev/null 2>&1; then
        brew install sqlite || true
      fi
    fi
  elif [[ "$os" == "Linux" ]]; then
    if ! command -v clang >/dev/null 2>&1 || ! ldconfig -p 2>/dev/null | grep -q libsqlite3; then
      if command -v apt-get >/dev/null 2>&1; then
        echo "Installing clang and libsqlite3 via apt (may prompt for sudo)…"
        sudo apt-get update -y
        sudo apt-get install -y clang libsqlite3-dev
      elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y clang sqlite-devel
      elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -Sy --noconfirm clang sqlite
      else
        echo "warning: install clang and libsqlite3 development packages manually" >&2
      fi
    fi
  fi
}

need_cmd curl
need_cmd tar
need_cmd uname

TRIPLE="$(detect_triple)"
API="https://api.github.com/repos/${REPO}/releases"

if [[ -z "$VERSION" ]]; then
  echo "Resolving latest release…"
  TAG="$(curl -fsSL "$API/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  if [[ -z "$TAG" ]]; then
    echo "error: could not resolve latest release for $REPO" >&2
    exit 1
  fi
  VERSION="${TAG#v}"
else
  TAG="v${VERSION#v}"
  VERSION="${TAG#v}"
fi

ASSET="lang-${VERSION}-${TRIPLE}.tar.gz"
echo "Installing LLLang ${VERSION} (${TRIPLE}) → ${PREFIX}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
SUMS_URL="https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS"

curl -fsSL "$DOWNLOAD_URL" -o "$TMP/$ASSET"
if curl -fsSL "$SUMS_URL" -o "$TMP/SHA256SUMS" 2>/dev/null; then
  EXPECTED="$(awk -v f="$ASSET" '$2 == f {print $1; exit}' "$TMP/SHA256SUMS")"
  if [[ -n "$EXPECTED" ]]; then
    if command -v sha256sum >/dev/null 2>&1; then
      ACTUAL="$(sha256sum "$TMP/$ASSET" | awk '{print $1}')"
    else
      ACTUAL="$(shasum -a 256 "$TMP/$ASSET" | awk '{print $1}')"
    fi
    if [[ "$EXPECTED" != "$ACTUAL" ]]; then
      echo "error: SHA256 mismatch for $ASSET" >&2
      echo "  expected: $EXPECTED" >&2
      echo "  actual:   $ACTUAL" >&2
      exit 1
    fi
    echo "SHA256 OK"
  else
    echo "warning: $ASSET not listed in SHA256SUMS; skipping hash verify"
  fi
else
  echo "warning: SHA256SUMS not found; skipping hash verify"
fi

mkdir -p "$PREFIX"
tar -xzf "$TMP/$ASSET" -C "$PREFIX"
chmod +x "$PREFIX/bin/lang" 2>/dev/null || true

install_host_deps

if [[ "$MODIFY_PATH" -eq 1 ]]; then
  PATH_LINE="export PATH=\"${PREFIX}/bin:\$PATH\""
  for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
    if [[ -f "$rc" ]] || [[ "$rc" == "$HOME/.profile" ]]; then
      if ! grep -qF "$PREFIX/bin" "$rc" 2>/dev/null; then
        echo "" >> "$rc"
        echo "# LLLang" >> "$rc"
        echo "$PATH_LINE" >> "$rc"
        echo "Added PATH to $rc"
        break
      fi
    fi
  done
fi

export PATH="$PREFIX/bin:$PATH"
echo ""
"$PREFIX/bin/lang" --version || true
echo "Installed. Open a new shell or: export PATH=\"$PREFIX/bin:\$PATH\""
