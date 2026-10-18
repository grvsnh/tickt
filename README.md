# Tickt

**Make time count.**

Tickt is a local-first Pomodoro focus dashboard for developers who already live in a terminal.
It provides a reliable keyboard-driven timer, persistent session history, and honest daily
statistics without an account, a daemon, cloud sync, or network access during normal use.

## Why Tickt?

- **A timer you can trust:** elapsed time comes from a monotonic clock, so redraw delays do not
  make sessions drift.
- **Useful, not decorative:** completed focus time, daily goals, streaks, and recent activity
  come from a local SQLite database.
- **Recoverable by design:** active sessions are persisted. A restart offers recovery, and a
  timer that expired while Tickt was closed is conservatively recorded as interrupted.
- **Terminal-native:** Ratatui and Crossterm provide a responsive interface with no mouse or
  patched font required.
- **Local and private:** data stays under XDG directories. There is no telemetry or required
  service.
- **Graceful integrations:** desktop notifications use `notify-send` when available; the timer
  continues normally when notification services or sound support are missing.

## What it looks like

```text
 ┌ TICKT   Make time count                         Sat 18 Oct  17:40 ┐
 │ FOCUS  ● RUNNING                                                    │
 │                           24:37                                    │
 │                    ████████████░░░░░░░░░░  3%                     │
 │                 Space pause · s skip · r reset                     │
 ├─────────────────────────────────┬──────────────────────────────────┤
 │ RECENT ACTIVITY                 │ TODAY'S FOCUS                    │
 │ focus       25m  completed  17:05│  2 completed sessions            │
 │ short break  5m  completed  16:40│  50 / 100 min (50%)             │
 │                                 │  2 day streak                    │
 └─────────────────────────────────┴──────────────────────────────────┘
```

This is a representative layout, not a screenshot and not seeded application data.

## Installation

Build from source with the pinned Rust toolchain:

```sh
git clone https://github.com/grvsnh/tickt.git
cd tickt
cargo build --release --locked
./target/release/tickt --help
```

The `rust-toolchain.toml` file pins the compiler used by Cargo. If you already have a checkout,
run the build commands from its root.

The source build needs a stable Rust toolchain (Rust 1.88.0), a C compiler, and `pkg-config`.
SQLite is compiled through `rusqlite`'s bundled feature, so a system SQLite development package
is not required. The included `Dockerfile` installs the build prerequisites and produces the
release binary in a Linux container.

To build and run the container image with persistent local data:

```sh
docker build -t tickt:local .
docker run --rm -it \
  -e XDG_CONFIG_HOME=/root/.config \
  -e XDG_DATA_HOME=/root/.local/share \
  -v "$HOME/.config/tickt:/root/.config/tickt" \
  -v "$HOME/.local/share/tickt:/root/.local/share/tickt" \
  tickt:local
```

The container command is interactive and does not provide desktop notifications from the host;
the terminal timer and SQLite history still work. Docker is an optional reproducible build path,
not a required runtime dependency.

For a local checkout, `cargo run --locked` launches Tickt immediately with default settings.

### Release binaries

Tagged GitHub releases provide archives for Linux x86_64, macOS Intel, macOS Apple Silicon, and
Windows x86_64. Download the archive for your platform from
<https://github.com/grvsnh/tickt/releases>, extract it, and run `tickt --help`.

On Linux or macOS, you can put the binary on your PATH:

```sh
tar -xzf tickt-linux-x86_64.tar.gz # use the matching macOS archive on macOS
mkdir -p "$HOME/.local/bin"
install -m 0755 tickt "$HOME/.local/bin/tickt"
tickt --version
```

On Windows, extract `tickt-windows-x86_64.zip`, then add the extracted directory to your user
`PATH`. The release page includes a SHA-256 file for every archive.

## CLI

```text
tickt                 Launch the interactive dashboard
tickt --help          Show command help
tickt --version       Show the version
tickt config path     Print the resolved TOML path
tickt config init     Create a sample config safely
tickt stats           Print today's summary
tickt history         Print recent sessions
```

Noninteractive commands do not enter raw mode or the alternate screen and are safe to pipe.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `Space` | Start, pause, or resume the current session |
| `n` | Start the suggested next session |
| `b` / `l` | Start a short / long break while idle |
| `r` / `s` | Confirm before reset / skip |
| `h` | History |
| `t` | Statistics |
| `,` | Settings |
| `?` | Help |
| `Esc` / `d` | Return to the dashboard |
| `q` | Confirm and quit |

See [docs/keybindings.md](docs/keybindings.md) for modal and recovery behavior.

## Configuration and data

On Linux, Tickt follows the XDG Base Directory Specification:

| Purpose | Default path |
| --- | --- |
| Configuration | `~/.config/tickt/config.toml` |
| SQLite history and recovery state | `~/.local/share/tickt/tickt.db` |
| Reserved log state path | `~/.local/state/tickt/tickt.log` |

`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_STATE_HOME` override these roots when they are
absolute paths. On macOS and Windows, Tickt uses the platform-native application data directory
provided by the `directories` crate. Print the active configuration path with:

```sh
tickt config path
```

`config init` never overwrites an existing file. A complete, valid example is in
[examples/config.toml](examples/config.toml). The main options are:

```toml
[timer]
focus_minutes = 25
short_break_minutes = 5
long_break_minutes = 15
long_break_interval = 4

[productivity]
daily_goal_minutes = 100

[ui]
theme = "dark" # dark, light, default
unicode = true # set false for ASCII-friendly progress bars
```

`theme = "default"` uses terminal-default colors, while `dark` and `light` use Tickt's curated
palettes. Set `unicode = false` when a terminal or font is not comfortable with block glyphs.

Durations are validated on startup. Focus and long-break durations may be 1–240 minutes;
short breaks may be 1–120 minutes; goals may be 1–1440 minutes. Unknown TOML fields are
rejected with a parse error instead of being silently ignored.

## Timer and statistics rules

The default cycle is 25 minutes of focus, a five-minute short break, and a 15-minute long break
after every four completed focus sessions. A focus session counts only when it reaches its full
planned duration. Skipped, reset, partial, interrupted, and break sessions never count as focus
work.

Dates use the local calendar day for display and grouping. The current streak is a consecutive
run of days that met the daily goal; before today's goal is reached, the streak is measured
backward from yesterday. The database stores UTC epoch timestamps so local presentation remains
unambiguous across restarts and daylight-saving changes.

When Tickt starts, a saved paused timer stays paused. A saved running timer is reconciled against
its UTC start timestamp. If its deadline passed while the process was closed, Tickt does not
pretend that it observed the work: the recovery prompt records it as interrupted. Monotonic
continuity cannot be preserved across a process restart, which is why this conservative policy
exists.

## Notifications and sound

Notifications are best effort. When enabled, Tickt safely starts `notify-send` with fixed
arguments after a session completes; it never invokes a shell and never blocks the event loop.
Install a desktop notification provider and `notify-send` if you want desktop alerts. If the
command or service is absent, the timer continues and Tickt displays a small status message.

The optional sound setting emits the terminal bell. It is disabled by default and does not
require an audio player or a particular sound server.

## Testing and development

The repository includes a pinned lockfile, Makefile targets, and an offline test command after
dependencies have been fetched once:

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --offline
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Equivalent shortcuts are available as `make fmt`, `make check`, `make test`, and `make lint`.
The deterministic unit tests cover timer transitions and recovery, configuration validation,
statistics/streak rules, SQLite migrations, session ordering, and active-state persistence.

## Fast demo

Use the one-minute development configuration from [docs/demo.md](docs/demo.md), then launch
with temporary XDG directories. Start, pause, resume, complete a session, inspect history and
statistics, quit, and relaunch. The normal default configuration remains 25 minutes; Tickt
never inserts fake completions or sample history.

## Release process

Releases can be created automatically from the GitHub Actions UI by running the `Release`
workflow and entering the next semantic version, currently `0.1.1`. The workflow creates and
pushes the `v0.1.1` tag itself. The traditional command-line path also works:

```sh
git tag v0.1.1
git push origin v0.1.1
```

The workflow in `.github/workflows/release.yml` verifies the version against `Cargo.toml`, builds
native release binaries on Linux, macOS Intel, macOS Apple Silicon, and Windows x86_64, creates
archives and SHA-256 files, and publishes them to the GitHub release. No npm account or token is
required. The first release has not been published from this checkout yet.

## OmniRush integration idea

Tickt is currently a standalone terminal focus tool. A future OmniRush feature could provide an
inbuilt CLI-based focus mode inside OmniRush itself: a user could launch a focused terminal
workspace, see the active task and timer together, and receive local session summaries without
opening another application. This is intentionally only a product idea for the OmniRush
developers, not an implemented integration, plugin, or dependency. Tickt currently makes no
OmniRush calls and does not require an OmniRush installation.

## Project architecture

The domain is intentionally small and synchronous:

- `src/timer.rs` — monotonic timer state machine and restart recovery.
- `src/statistics.rs` — pure daily aggregation, goals, streaks, and seven-day summaries.
- `src/storage/` — SQLite schema, migration, parameterized queries, and transactions.
- `src/config.rs` — typed TOML config, XDG paths, defaults, and validation.
- `src/app.rs` — event loop, keyboard actions, persistence coordination, and recovery prompts.
- `src/ui/` — Ratatui layout and all views.
- `src/terminal.rs` — raw mode, alternate screen, cursor cleanup, and error-safe lifecycle.

See [docs/architecture.md](docs/architecture.md) for the state and persistence boundaries.
See [docs/platform-testing.md](docs/platform-testing.md) for release-platform smoke tests and
the manual terminal checklist.

## Troubleshooting

- **Invalid TOML or value:** run `tickt config path`, open the reported file, and check the
  field-specific validation message. Copy the known-good example if necessary.
- **Notification unavailable:** install `notify-send` and a desktop notification service, or
  disable `[notifications].enabled`; this does not affect timer accuracy.
- **Terminal looks stuck after an interruption:** Tickt restores raw mode on normal errors and
  panic unwinding. If a terminal emulator still needs resetting, run `stty sane`.
- **Database cannot be opened:** verify the parent XDG data directory is writable. Tickt never
  replaces a database when opening or migrating it fails.
- **Small terminal:** resize to at least 64×18 for the complete dashboard. Smaller sizes show a
  readable compact timer and quit hint.

## Current limitations

- Release artifacts target Linux x86_64, macOS Intel, macOS Apple Silicon, and Windows x86_64.
- There is no published release artifact from this checkout yet, and no package-manager formula,
  sync service, or remote API.
- Interactive behavior is implemented with Crossterm, but automated CI smoke-tests the
  noninteractive CLI on each release platform rather than requiring a real terminal session.
- Perfect monotonic continuity is impossible across a process restart; recovery uses wall-clock
  timestamps and intentionally avoids fabricating completed work.
- Visual verification requires a real terminal. Automated tests cover the domain and persistence
  but do not replace manual resize and terminal-cleanup checks.

## Contributing and license

See [CONTRIBUTING.md](CONTRIBUTING.md) for development boundaries and migration conventions.
Tickt is released under the [MIT License](LICENSE).
