# Tickt

**Make time count.**

Tickt is a fast, local-first Pomodoro dashboard for Linux terminals. It is keyboard-driven,
works offline, stores history in SQLite, and has no account, daemon, telemetry, or web UI.

## Install a release

The release is one statically linked x86_64 Linux binary. It is tested on Ubuntu, Debian, Arch,
Fedora, openSUSE, Rocky Linux, and Alpine. Download the latest archive from:

<https://github.com/grvsnh/tickt/releases>

```sh
release_url="https://github.com/grvsnh/tickt/releases/latest/download"
curl -fL -o /tmp/tickt-linux-x86_64.tar.gz \
  "$release_url/tickt-linux-x86_64.tar.gz"
curl -fL -o /tmp/tickt-linux-x86_64.tar.gz.sha256 \
  "$release_url/tickt-linux-x86_64.tar.gz.sha256"
(cd /tmp && sha256sum -c tickt-linux-x86_64.tar.gz.sha256)
mkdir -p "$HOME/.local/bin"
tar -xzf /tmp/tickt-linux-x86_64.tar.gz -C "$HOME/.local/bin"
tickt --version
tickt
```

## Build from source

Rust 1.88.0, a C compiler, and a Linux terminal are required. SQLite is bundled. For the
portable static release build, install the musl linker and target too.

Debian/Ubuntu:

```sh
sudo apt install build-essential musl-tools
rustup target add x86_64-unknown-linux-musl
```

Arch:

```sh
sudo pacman -S --needed base-devel musl rustup
rustup target add x86_64-unknown-linux-musl
```

Fedora:

```sh
sudo dnf install gcc gcc-c++ make musl-gcc
rustup target add x86_64-unknown-linux-musl
```

Build and run the application:

```sh
git clone https://github.com/grvsnh/tickt.git
cd tickt
cargo run --locked
```

Create the exact release archive locally:

```sh
./scripts/package-linux.sh 1.0.0
sha256sum -c dist/release/tickt-linux-x86_64.tar.gz.sha256
```

## Controls

| Key         | Action                           |
| ----------- | -------------------------------- |
| `Space`     | Start, pause, or resume          |
| `n`         | Start the suggested next session |
| `b` / `l`   | Start a short / long break       |
| `r` / `s`   | Confirm before reset / skip      |
| `h`         | History                          |
| `t`         | Statistics                       |
| `,`         | Settings                         |
| `?`         | Help                             |
| `Esc` / `d` | Dashboard                        |
| `q`         | Confirm and quit                 |

## Configuration and data

Tickt follows the Linux XDG directories:

```text
~/.config/tickt/config.toml
~/.local/share/tickt/tickt.db
~/.local/state/tickt/tickt.log
```

Use custom absolute XDG roots with `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_STATE_HOME`.

```sh
tickt config path
```

The complete configuration example is in `examples/config.toml`. Defaults are 25-minute focus,
5-minute short break, 15-minute long break, and a 100-minute daily goal. Only fully completed
focus sessions count toward statistics; skipped, reset, interrupted, partial, and break sessions
do not count. See [docs/configuration.md](docs/configuration.md) for ranges and every field.

An active timer is saved locally. A paused timer stays paused after restart. A timer that expired
while Tickt was closed is recorded as interrupted rather than fabricated as completed work.

## Release automation

The GitHub workflow at `.github/workflows/release.yml` creates the release automatically when
`main` is pushed:

1. It reads the version from `Cargo.toml`.
2. It creates and pushes the matching `v<version>` tag.
3. It builds the static Linux binary.
4. It verifies the archive across representative Linux distributions.
5. It publishes the GitHub release and checksum.

For a new release, update the version in `Cargo.toml`, run the checks, and push `main`:

```sh
git push origin main
```

The current release version is `1.0.0`. Do not push another `main` commit without changing the
version after a release tag already exists; the workflow will safely finish as a no-op instead of
overwriting a release.

## Testing

```sh
make fmt
make check
make test
make lint
make release
```

The test suite covers timer transitions and recovery, configuration validation, statistics,
streaks, SQLite migrations, ordering, and persistence. CI builds once and smoke-tests the same
static archive on Ubuntu, Debian, Arch, Fedora, openSUSE, Rocky Linux, and Alpine.

## Architecture

- `src/timer.rs` — monotonic timer state machine.
- `src/statistics.rs` — deterministic goals, streaks, and daily totals.
- `src/storage/` — SQLite schema, migration, and transactions.
- `src/config.rs` — Linux XDG paths and TOML validation.
- `src/app.rs` — event loop and application state.
- `src/ui/` — Ratatui terminal views.
- `scripts/package-linux.sh` — reproducible static release packaging.

See [docs/performance.md](docs/performance.md) for the measured statistics optimization and
future profiling targets.

Tickt is intentionally Linux-terminal-only for v1.0. There is no native desktop app or web app.

## OmniRush idea

Tickt is independent today. A future OmniRush feature could provide an inbuilt terminal focus
mode that combines the active OmniRush task, a Pomodoro timer, and local summaries. This is only
an idea for OmniRush developers; Tickt makes no OmniRush calls and requires no OmniRush install.
