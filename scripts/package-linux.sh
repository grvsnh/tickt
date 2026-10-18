#!/usr/bin/env bash
set -euo pipefail

cargo_version="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
version="${1:-$cargo_version}"
package_dir="dist/release/package"
archive="dist/release/tickt-linux-x86_64.tar.gz"
checksum="${archive}.sha256"

if [[ "$version" != "$cargo_version" ]]; then
  printf 'version %s does not match Cargo.toml version %s\n' "$version" "$cargo_version" >&2
  exit 1
fi

cargo build --release --locked
rm -rf dist/release
mkdir -p "$package_dir"
cp target/release/tickt "$package_dir/tickt"
chmod 0755 "$package_dir/tickt"
tar -C "$package_dir" -czf "$archive" tickt
(cd "$(dirname "$archive")" && sha256sum "$(basename "$archive")" > "$(basename "$checksum")")

printf 'Tickt %s package created:\n  %s\n  %s\n' "$version" "$archive" "$checksum"
