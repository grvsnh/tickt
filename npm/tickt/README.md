# tickt (npm installer)

This package installs the prebuilt Linux x86_64 Tickt binary published by the
[Tickt GitHub releases](https://github.com/grvsnh/tickt/releases).

```sh
npm install --global tickt
tickt
```

The npm package is a distribution wrapper, not a JavaScript reimplementation. Its postinstall
step downloads the matching release asset, verifies its SHA-256 checksum, and installs the
executable beside the wrapper. Normal Tickt use remains local and does not need Node or a
network connection after installation.

Supported platform: Linux x86_64. Source builds and the release artifacts remain available for
other environments.

For local package testing against a built binary:

```sh
TICKT_BINARY_PATH=/absolute/path/to/tickt npm install --global ./npm/tickt
```

The package's release version and binary assets are kept in sync by the tagged GitHub release
workflow.
