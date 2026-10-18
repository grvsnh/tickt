.PHONY: build check test fmt lint run release

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

run:
	cargo run --locked

release:
	./scripts/package-linux.sh
