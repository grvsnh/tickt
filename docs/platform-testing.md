# Linux release testing

Tickt v1.0 ships one static x86_64 Linux archive. The release workflow builds it once and runs
the extracted binary in representative containers:

- Ubuntu 22.04 and 24.04
- Debian Bullseye and Bookworm
- Arch Linux
- Fedora
- openSUSE Leap
- Rocky Linux
- Alpine

This is distro compatibility testing, not a separate package for each distribution. The binary
is Linux terminal software and the release target is `x86_64-unknown-linux-musl`.

## Local release test

Install Rust, `musl-gcc`, and the musl target for the host distribution, then run:

```sh
./scripts/package-linux.sh
(cd dist/release && sha256sum -c tickt-linux-x86_64.tar.gz.sha256)
mkdir -p /tmp/tickt-release-check
tar -xzf dist/release/tickt-linux-x86_64.tar.gz -C /tmp/tickt-release-check
/tmp/tickt-release-check/tickt --version
```

With Docker, test the same extracted binary in several distro families:

```sh
for image in ubuntu:22.04 debian:bookworm-slim archlinux:latest fedora:latest alpine:3.20; do
  docker run --rm \
    -v /tmp/tickt-release-check:/release:ro \
    "$image" /release/tickt --version
done
```

## Terminal checklist

In a real Linux terminal such as GNOME Terminal, Konsole, Alacritty, Foot, or xterm:

1. Run `tickt --help` and `tickt --version`.
2. Start, pause, resume, reset, and skip a session.
3. Resize the terminal while the timer is running.
4. Open history, statistics, settings, and help.
5. Quit and confirm raw mode and the cursor are restored.
6. Relaunch and verify history and active-session recovery.
