#!/usr/bin/env bash
#
# Build a deterministic Debian package from a released archive.
#
# The binaries come from the same tarball published on the GitHub release, so
# what apt installs is byte-for-byte what the release ships.

set -euo pipefail

fail() {
    local code=$1
    shift
    printf '::error::%s %s\n' "$code" "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' \
        'usage: build-deb.sh --archive FILE --arch DEB_ARCH --version VERSION' \
        '       --output-dir DIR --source-date-epoch EPOCH'
}

archive=
arch=
version=
output_dir=
source_date_epoch=

while (($#)); do
    case "$1" in
        --archive) archive=${2:-}; shift 2 ;;
        --arch) arch=${2:-}; shift 2 ;;
        --version) version=${2:-}; shift 2 ;;
        --output-dir) output_dir=${2:-}; shift 2 ;;
        --source-date-epoch) source_date_epoch=${2:-}; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; fail UG7300 "unknown argument: $1" ;;
    esac
done

for command in dpkg-deb install tar; do
    command -v "$command" >/dev/null || fail UG7300 "required command is missing: $command"
done

[[ -f "$archive" && ! -L "$archive" ]] || fail UG7300 'archive must be a regular file'
[[ "$arch" == amd64 || "$arch" == arm64 ]] || fail UG7300 'arch must be amd64 or arm64'
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail UG7300 'version must be canonical SemVer'
[[ -n "$output_dir" ]] || fail UG7300 'output-dir is required'
[[ "$source_date_epoch" =~ ^[1-9][0-9]*$ ]] || \
    fail UG7300 'source-date-epoch must be a positive integer'

install -d "$output_dir"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT

tar -xzf "$archive" -C "$stage"
for binary in ugos-cli ugos-mcp; do
    [[ -f "$stage/$binary" ]] || fail UG7301 "archive does not contain $binary"
done

root="$stage/package"
install -d "$root/DEBIAN" "$root/usr/bin"
install -m 0755 "$stage/ugos-cli" "$root/usr/bin/ugos-cli"
install -m 0755 "$stage/ugos-mcp" "$root/usr/bin/ugos-mcp"

# Installed-Size is what apt reports before download; dpkg expects KiB.
installed_size=$(du -ks "$root/usr" | cut -f1)

cat > "$root/DEBIAN/control" <<CONTROL
Package: ugos-cli
Version: ${version}-1
Section: utils
Priority: optional
Architecture: ${arch}
Maintainer: Fabian Schmieder <fabian@schmieder.eu>
Installed-Size: ${installed_size}
Homepage: https://github.com/metaneutrons/ugos-cli
Description: CLI and MCP server for UGREEN NAS (UGOS) management
 Manages UGREEN NAS devices through the API their web UI uses: virtual
 machines, Docker containers, files, downloads, filesystem snapshots,
 system monitoring, logs and users.
 .
 Ships two binaries: ugos-cli for interactive and scripted use, and
 ugos-mcp, an MCP server for AI-assisted management.
CONTROL
chmod 0644 "$root/DEBIAN/control"

output="$output_dir/ugos-cli_${version}_${arch}.deb"
# --root-owner-group keeps the package independent of the build user, and
# SOURCE_DATE_EPOCH makes repeated builds byte-identical.
SOURCE_DATE_EPOCH="$source_date_epoch" \
    dpkg-deb --root-owner-group --build "$root" "$output" >/dev/null

# Prove the package says what it should before anything downstream trusts it.
[[ $(dpkg-deb --field "$output" Version) == "${version}-1" ]] || \
    fail UG7302 'built package carries the wrong version'
[[ $(dpkg-deb --field "$output" Architecture) == "$arch" ]] || \
    fail UG7302 'built package carries the wrong architecture'

printf '%s\n' "$output"
