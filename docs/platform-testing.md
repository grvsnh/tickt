# Release platform testing

Tickt v1.0 is a Linux-first release. The release workflow builds one optimized
`x86_64-unknown-linux-gnu` binary and checks the extracted archive in current Arch Linux and
Fedora containers. This keeps release time short while covering the main Linux distribution
families without compiling the same Rust application repeatedly.

The release artifact is intended for current x86_64 glibc distributions, including Ubuntu,
Debian, Arch Linux, Fedora, and similar systems. It is not an Alpine/musl artifact and does not
provide a native Windows `.exe`.

## Linux terminals

Test in GNOME Terminal, Konsole, Alacritty, Foot, or another xterm-compatible terminal:

```sh
./tickt --version
./tickt --help
./tickt
```

Start, pause, resume, skip, reset, resize the window, open history/statistics/help, quit, and
relaunch. Confirm that the shell is not left in raw mode.

## Arch Linux and Fedora compatibility

CI builds the release binary once on Ubuntu 22.04, extracts the archive, then runs that same
binary in both Arch Linux and Fedora containers. This verifies that the release is not dependent
on Ubuntu-only runtime files.

When Docker is available locally:

```sh
cargo build --release --locked
docker run --rm -v "$PWD:/work:ro" -w /work archlinux:latest /work/target/release/tickt --version
docker run --rm -v "$PWD:/work:ro" -w /work fedora:latest /work/target/release/tickt --version
```

## Windows via WSL2

Native Windows builds are not part of v1.0. Test the Linux artifact inside WSL2 using Windows
Terminal:

```powershell
wsl --install -d Ubuntu
```

Inside WSL, extract `tickt-linux-x86_64.tar.gz`, run `tickt --version`, and perform the same
timer, resize, restart, and terminal-cleanup checks. WSL uses Linux/XDG paths and may not provide
desktop notifications; those integrations are optional.

The current development environment verified the Linux binary and a pseudo-terminal launch.
Arch/Fedora container checks and WSL checks are configured for CI/manual execution, but were not
run locally because Docker and WSL are unavailable in the current environment.
