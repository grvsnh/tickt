# Release platform testing

Tickt v1.0 is a Linux-first release. The release workflow builds one optimized, statically linked
`x86_64-unknown-linux-musl` binary and checks the extracted archive across representative Linux
distributions. This covers distro families without compiling the same Rust application repeatedly.

The release artifact is intended for x86_64 Linux distributions, including Ubuntu, Debian, Arch
Linux, Fedora, openSUSE, Rocky Linux, and Alpine. It does not provide a native Windows `.exe`.

## Linux terminals

Test in GNOME Terminal, Konsole, Alacritty, Foot, or another xterm-compatible terminal:

```sh
./tickt --version
./tickt --help
./tickt
```

Start, pause, resume, skip, reset, resize the window, open history/statistics/help, quit, and
relaunch. Confirm that the shell is not left in raw mode.

## Linux distribution compatibility

CI builds the release binary once on Ubuntu 22.04, extracts the archive, then runs that same
static binary in Ubuntu 22.04/24.04, Debian Bullseye/Bookworm, Arch Linux, Fedora, openSUSE
Leap, Rocky Linux, and Alpine containers. This verifies that the release is not dependent on
Ubuntu-only runtime files or glibc shared libraries.

When Docker is available locally:

```sh
./scripts/package-linux.sh 1.0.0
mkdir -p /tmp/tickt-release-check
tar -xzf dist/release/tickt-linux-x86_64.tar.gz -C /tmp/tickt-release-check
for image in ubuntu:22.04 debian:bookworm-slim archlinux:latest fedora:latest alpine:3.20; do
  docker run --rm -v /tmp/tickt-release-check:/release:ro "$image" /release/tickt --version
done
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
