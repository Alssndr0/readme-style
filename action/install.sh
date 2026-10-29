#!/usr/bin/env bash
# Installs readme-style for the GitHub Action: a checksum-verified release binary when a version
# is known, otherwise a build from the action's own source.
set -euo pipefail

repository="${RS_REPOSITORY:-Alssndr0/readme-style}"
version="${RS_VERSION:-}"
if [[ -z "$version" && "${RS_ACTION_REF:-}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
  version="$RS_ACTION_REF"
fi
root="${RUNNER_TEMP:-/tmp}/readme-style"
mkdir -p "$root/bin"

download() {
  local target ext
  case "${RUNNER_OS:-$(uname -s)}-${RUNNER_ARCH:-$(uname -m)}" in
    Linux-X64 | Linux-x86_64) target=x86_64-unknown-linux-musl ext=tar.gz ;;
    Linux-ARM64 | Linux-aarch64) target=aarch64-unknown-linux-musl ext=tar.gz ;;
    macOS-X64 | Darwin-x86_64) target=x86_64-apple-darwin ext=tar.gz ;;
    macOS-ARM64 | Darwin-arm64) target=aarch64-apple-darwin ext=tar.gz ;;
    Windows-X64) target=x86_64-pc-windows-msvc ext=zip ;;
    Windows-ARM64) target=aarch64-pc-windows-msvc ext=zip ;;
    *) return 1 ;;
  esac
  local archive="readme-style-$target.$ext"
  local base="https://github.com/$repository/releases/download/$version"
  local tmp
  tmp="$(mktemp -d)"
  curl -fsSL "$base/$archive" -o "$tmp/$archive" || return 1
  curl -fsSL "$base/$archive.sha256" -o "$tmp/$archive.sha256" || return 1
  if command -v sha256sum > /dev/null; then
    (cd "$tmp" && sha256sum -c "$archive.sha256")
  else
    (cd "$tmp" && shasum -a 256 -c "$archive.sha256")
  fi
  if [[ "$ext" == zip ]]; then
    unzip -o -j "$tmp/$archive" readme-style.exe -d "$root/bin" > /dev/null
  else
    tar -xzf "$tmp/$archive" -C "$root/bin" readme-style
  fi
}

if [[ -n "$version" ]] && download; then
  echo "readme-style $version installed from the release"
else
  [[ -n "$version" ]] && echo "::warning::no release binary for $version on this runner; building from source"
  cargo install --locked --quiet --path "${RS_ACTION_PATH:-.}" --root "$root"
fi
echo "$root/bin" >> "${GITHUB_PATH:-/dev/null}"
"$root/bin/readme-style" --version
