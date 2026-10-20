FROM rust:1.88.0-slim-bookworm

RUN apt-get update \
    && apt-get install --no-install-recommends -y build-essential ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml clippy.toml ./
RUN cargo fetch --locked
COPY src ./src
COPY examples ./examples
COPY docs ./docs
COPY README.md LICENSE CHANGELOG.md CONTRIBUTING.md ./
RUN cargo build --release --locked

ENTRYPOINT ["/app/target/release/tickt"]
