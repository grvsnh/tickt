.PHONY: build check test fmt lint run

build:
	cargo build --locked

check:
	cargo check --locked

test:
	cargo test --locked --offline

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets --all-features -- -D warnings

run:
	cargo run --locked
