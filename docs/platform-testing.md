# Release platform testing

The release workflow builds and runs the noninteractive `--version` smoke command on every
release target:

- Linux x86_64 (`x86_64-unknown-linux-gnu`)
- macOS Intel (`x86_64-apple-darwin`)
- macOS Apple Silicon (`aarch64-apple-darwin`)
- Windows x86_64 (`x86_64-pc-windows-msvc`)

The smoke command verifies that the binary starts on the target OS without requiring a real TTY.
Before calling an artifact production-ready, perform the interactive checklist below on at least
one terminal for each platform.

## Linux

Test in GNOME Terminal, Konsole, or another xterm-compatible terminal:

```sh
./tickt --version
./tickt --help
./tickt
```

Start, pause, resume, skip, reset, resize the window, open history/statistics/help, quit, and
relaunch. Confirm that the shell is not left in raw mode.

The release workflow also runs the Linux binary in current Arch Linux and Fedora containers. This
checks that the Ubuntu-runner-built glibc binary is not tied to Ubuntu-specific runtime files. If
Docker is available locally, run the same smoke test with:

```sh
cargo build --release --locked
docker run --rm -v "$PWD:/work:ro" -w /work archlinux:latest /work/target/release/tickt --version
docker run --rm -v "$PWD:/work:ro" -w /work fedora:latest /work/target/release/tickt --version
```

## macOS

Test in macOS Terminal and iTerm2:

```sh
./tickt --version
./tickt
```

Use the same timer, resize, restart, and cleanup checks. Confirm that configuration and data are
created under the platform-native application support directory.

## Windows

Test in Windows Terminal and PowerShell:

```powershell
.\tickt.exe --version
.\tickt.exe
```

Use the same keyboard and resize checks. Confirm that the application exits cleanly and that
history persists between launches.

The current development environment verified the Linux binary and a pseudo-terminal launch;
macOS and Windows interactive behavior remains a release-runner/manual verification item.
