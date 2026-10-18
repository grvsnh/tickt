# Tickt

**Make time count.**

Tickt is a local-first, keyboard-driven Pomodoro focus dashboard for the terminal.

The project is under active development. The complete application will provide a reliable
timer, persistent local history, daily focus statistics, and a polished Ratatui interface.

## Development checks

The pinned toolchain is Rust 1.88.0. After installing Rust, run:

```sh
cargo build --locked
cargo test --locked --offline
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```
