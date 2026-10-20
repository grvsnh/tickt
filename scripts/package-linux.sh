#!/usr/bin/env bash
set -euo pipefail

cargo_version="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
version="${1:-$cargo_version}"
target="x86_64-unknown-linux-musl"
package_dir="dist/release/package"
archive="dist/release/tickt-linux-x86_64.tar.gz"
checksum="${archive}.sha256"
source_date_epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
release_notes="release-notes/v$version.txt"

if [[ "$version" != "$cargo_version" ]]; then
  printf 'version %s does not match Cargo.toml version %s\n' "$version" "$cargo_version" >&2
  exit 1
fi
if [[ ! -f "$release_notes" ]]; then
  printf 'release notes are missing: %s\n' "$release_notes" >&2
  exit 1
fi

if ! command -v musl-gcc >/dev/null 2>&1; then
  printf 'musl-gcc is required to create the portable Linux release (install musl-tools)\n' >&2
  exit 1
fi
if ! rustup target list --installed | grep -qx "$target"; then
  printf 'Rust target %s is missing (run: rustup target add %s)\n' "$target" "$target" >&2
  exit 1
fi

CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-musl-gcc}" \
AR_x86_64_unknown_linux_musl="${AR_x86_64_unknown_linux_musl:-ar}" \
  cargo build --release --locked --target "$target"
rm -rf dist/release
mkdir -p "$package_dir"
cp "target/$target/release/tickt" "$package_dir/tickt"
cp LICENSE "$package_dir/LICENSE"
cp README.md "$package_dir/README.md"
cp "$release_notes" "$package_dir/RELEASE_NOTES.txt"
cp -R assets "$package_dir/assets"
chmod 0755 "$package_dir/tickt"
tar -I 'gzip -n' \
  --sort=name \
  --mtime="@$source_date_epoch" \
  --owner=0 \
  --group=0 \
  --numeric-owner \
  -C "$package_dir" \
  -cf "$archive" \
  LICENSE README.md RELEASE_NOTES.txt assets tickt
(cd "$(dirname "$archive")" && sha256sum "$(basename "$archive")" > "$(basename "$checksum")")

printf 'Tickt %s package created:\n  %s\n  %s\n' "$version" "$archive" "$checksum"
