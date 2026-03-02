#!/bin/bash
set -euo pipefail

REPO="OlaProeis/forgeStat"
BINARY="forgeStat"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

info()    { printf '\033[0;34m[INFO]\033[0m %s\n' "$1"; }
success() { printf '\033[0;32m[OK]\033[0m %s\n' "$1"; }
error()   { printf '\033[0;31m[ERROR]\033[0m %s\n' "$1"; exit 1; }

detect_target() {
  case "$(uname -s)" in
    Linux*)  os="unknown-linux-gnu" ;;
    Darwin*) os="apple-darwin" ;;
    *)       error "Unsupported OS: $(uname -s)" ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64)  arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)             error "Unsupported arch: $(uname -m)" ;;
  esac
  echo "${arch}-${os}"
}

main() {
  info "forgeStat Installer"
  local target
  target=$(detect_target)
  info "Platform: $target"

  local version
  version=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
  [ -z "$version" ] && error "Could not determine latest version"
  info "Version: $version"

  local url="https://github.com/$REPO/releases/download/${version}/${BINARY}-${version}-${target}.tar.gz"
  info "Downloading $url"

  local tmp
  tmp=$(mktemp -d)
  trap "rm -rf $tmp" EXIT

  curl -fsSL "$url" -o "$tmp/archive.tar.gz" || error "Download failed — check the release page"
  tar xzf "$tmp/archive.tar.gz" --strip-components=1 -C "$tmp"

  mkdir -p "$INSTALL_DIR"
  cp "$tmp/$BINARY" "$INSTALL_DIR/"
  chmod +x "$INSTALL_DIR/$BINARY"

  success "Installed to $INSTALL_DIR/$BINARY"
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    info "Add to PATH:  export PATH=\"$INSTALL_DIR:\$PATH\""
  else
    success "Run: forgeStat --version"
  fi
}
main "$@"
