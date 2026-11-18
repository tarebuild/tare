#!/bin/sh
set -eu

base_url='__TARE_RELEASES_BASE_URL__'
fail() { printf 'tare install: %s\n' "$*" >&2; exit 1; }

case "$base_url" in
  __TARE_RELEASES_BASE_URL__) fail 'the release URL is not configured.' ;;
esac
base_url=${base_url%/}
command -v curl >/dev/null 2>&1 || fail 'curl is required.'
command -v tar >/dev/null 2>&1 || fail 'tar is required.'

system=$(uname -s)
machine=$(uname -m)
case "$machine" in
  x86_64|amd64) architecture=x86_64 ;;
  aarch64|arm64) architecture=aarch64 ;;
  *) fail "unsupported architecture: $machine." ;;
esac
binary=tare
case "$system" in
  Linux)
    if ldd --version 2>&1 | grep -qi musl; then
      fail 'musl-based Linux (such as Alpine) is not served; the prebuilt binaries are for macOS, glibc Linux and Windows.'
    fi
    target=$architecture-unknown-linux-gnu
    ;;
  Darwin) target=$architecture-apple-darwin ;;
  MINGW*|MSYS*|CYGWIN*)
    [ "$architecture" = x86_64 ] || fail "unsupported Windows architecture: $machine."
    target=x86_64-pc-windows-msvc
    binary=tare.exe
    ;;
  *) fail "unsupported operating system: $system." ;;
esac

work=$(mktemp -d "${TMPDIR:-/tmp}/tare-install.XXXXXX") || fail 'could not create a temporary directory.'
trap 'rm -rf "$work"' 0
trap 'exit 1' HUP INT TERM

version=${TARE_VERSION:-}
if [ -z "$version" ]; then
  curl --fail --silent --show-error --location "$base_url/latest" -o "$work/latest" || fail 'could not read the latest release version.'
  version=$(cat "$work/latest")
fi
case "$version" in
  ''|*[!A-Za-z0-9._+-]*) fail 'the release version is invalid.' ;;
esac

archive=tare-$version-$target.tar.gz
curl --fail --silent --show-error --location "$base_url/$archive" -o "$work/$archive" || fail "archive unavailable: $archive."
curl --fail --silent --show-error --location "$base_url/$archive.sha256" -o "$work/$archive.sha256" || fail "checksum unavailable: $archive.sha256."
expected=''
read -r expected checksum_name < "$work/$archive.sha256" || fail 'the release checksum is invalid.'
case "$expected" in
  *[!0-9a-f]*|'') fail 'the release checksum is invalid.' ;;
esac
[ "${#expected}" -eq 64 ] || fail 'the release checksum is invalid.'
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$work/$archive")
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$work/$archive")
else
  fail 'sha256sum or shasum is required.'
fi
actual=${actual%% *}
[ "$actual" = "$expected" ] || fail "checksum mismatch: $archive."

members=$(tar -tzf "$work/$archive") || fail "could not read archive: $archive."
[ "$members" = "$binary" ] || fail "archive must contain only $binary."
tar -xzf "$work/$archive" -C "$work" || fail "could not extract archive: $archive."
install_dir=${TARE_INSTALL_DIR:-"$HOME/.tare/bin"}
mkdir -p "$install_dir" || fail "could not create install directory: $install_dir."
cp "$work/$binary" "$install_dir/$binary" || fail "could not install $binary."
chmod +x "$install_dir/$binary" || fail "could not make $binary executable."
printf 'Installed %s to %s\n' "$version" "$install_dir"
case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *)
    escaped_dir=$(printf '%s' "$install_dir" | sed "s/'/'\\\\''/g")
    printf "Add Tare to PATH: export PATH='%s':\"\$PATH\"\n" "$escaped_dir"
    ;;
esac
"$install_dir/$binary" --version
