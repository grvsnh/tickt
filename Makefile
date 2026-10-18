.PHONY: build check test fmt lint quality fetch release-metadata run release release-check

build:
	cargo build --locked

check:
	cargo check --locked

test:
	cargo test --locked --offline

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets -- -D warnings

quality: fmt check test lint

release-metadata:
	./scripts/check-release-metadata.sh

fetch:
	cargo fetch --locked

run:
	cargo run --locked

release:
	./scripts/package-linux.sh

release-check: release
	(cd dist/release && sha256sum -c tickt-linux-x86_64.tar.gz.sha256)
	tmpdir=$$(mktemp -d); trap 'rmdir "$$tmpdir" 2>/dev/null || true' EXIT; tar -xzf dist/release/tickt-linux-x86_64.tar.gz -C "$$tmpdir"; "$$tmpdir/tickt" --version
