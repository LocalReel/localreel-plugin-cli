#!/bin/sh
# Install localreel-plugin-cli for Linux or macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.sh | sh
#
# Environment:
#   VERSION       release tag (default: latest)
#   INSTALL_DIR   destination (default: $HOME/.local/bin)
set -eu

version="${VERSION:-latest}"
install_dir="${INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *) echo "unsupported operating system: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) arch="x64" ;;
  arm64 | aarch64) arch="arm64" ;;
  *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

platform="${os}-${arch}"
archive="localreel-plugin-cli-${platform}.tar.gz"
if [ "$version" = "latest" ]; then
  base="https://github.com/LocalReel/localreel-plugin-cli/releases/latest/download"
else
  base="https://github.com/LocalReel/localreel-plugin-cli/releases/download/${version}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "downloading $archive ..."
curl -fsSL "$base/$archive" -o "$tmp/$archive"

if curl -fsSL "$base/$archive.sha256" -o "$tmp/$archive.sha256" 2>/dev/null; then
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$tmp" && sha256sum -c "$archive.sha256")
  else
    (cd "$tmp" && shasum -a 256 -c "$archive.sha256")
  fi
fi

tar -xzf "$tmp/$archive" -C "$tmp"
binary="$(find "$tmp" -type f -name localreel-plugin-cli | head -n 1)"
if [ -z "$binary" ]; then
  echo "archive did not contain localreel-plugin-cli" >&2
  exit 1
fi

mkdir -p "$install_dir"
install -m 0755 "$binary" "$install_dir/localreel-plugin-cli"
echo "installed $install_dir/localreel-plugin-cli"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "add $install_dir to PATH to run it from any directory" ;;
esac
