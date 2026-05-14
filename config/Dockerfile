# syntax=docker/dockerfile:1.7

FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies — dummy main.rs so the dep layer compiles independently
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,id=gotify-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=gotify-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=gotify-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked -p gotify-mcp && rm -rf src

# Build real binary
COPY src/ src/
RUN --mount=type=cache,id=gotify-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=gotify-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=gotify-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked -p gotify-mcp && cp target/release/gotify /usr/local/bin/gotify

# Runtime — minimal Debian image with ca-certificates, curl (healthcheck), su-exec (entrypoint)
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ca-certificates curl gosu && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/gotify /usr/local/bin/gotify
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

RUN groupadd --gid 1000 gotify && \
    useradd --uid 1000 --gid gotify --no-create-home --shell /sbin/nologin gotify && \
    mkdir -p /data && \
    chown gotify:gotify /data

ENV RUST_LOG=info

EXPOSE 40020/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40020/health || exit 1

LABEL io.modelcontextprotocol.server.name="tv.tootie/gotify-mcp"

ENTRYPOINT ["/entrypoint.sh"]
CMD ["serve", "mcp"]
