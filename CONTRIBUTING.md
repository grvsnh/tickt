# Contributing to Tickt

Thanks for helping make time count.

## Development setup

Tickt is a Rust 1.88.0 binary crate. The repository pins the toolchain in
`rust-toolchain.toml`. Build from the project root:

```sh
cargo check --locked
cargo test --locked --offline
```

The first dependency download may require network access. Normal application use does not.

## Quality gates

Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --offline
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Architecture boundaries

- `timer.rs` owns monotonic timer calculations and valid state transitions.
- `statistics.rs` is a pure calculation layer over stored session records.
- `storage/` owns SQLite schema, migrations, transactions, and conversion at the database boundary.
- `config.rs` owns TOML parsing, XDG paths, defaults, and validation.
- `app.rs` coordinates input, timer transitions, persistence, and views.
- `ui/` renders application state and must not make database changes.

Keep domain behavior independent of Ratatui and Crossterm where possible. Add deterministic
tests for timer, statistics, configuration, or storage behavior that changes.

## Database migrations

The SQLite schema uses `PRAGMA user_version`. Add a numbered migration branch in
`storage/mod.rs`, keep migrations transactional, and never replace a user's database when a
migration fails. Add a temporary-database test for every schema change.

## Error handling

Use contextual `anyhow::Result` errors at application boundaries and explicit domain errors
for invalid timer transitions. Do not use `unwrap()` or `expect()` in ordinary application
paths. Optional desktop integrations must degrade without stopping the timer.

## Pull requests

Keep changes focused, explain user-visible behavior, include tests, and update the README or
keybinding documentation when behavior changes. Do not add network services, telemetry, or
unnecessary dependencies to the local-first application.
