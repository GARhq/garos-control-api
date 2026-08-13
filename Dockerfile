FROM rust:1.82-slim AS builder
WORKDIR /build

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() { println!(\"placeholder\"); }" > src/main.rs && \
    echo "" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl3 ca-certificates tini curl iproute2 iputils-ping && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 -d /var/lib/garos -s /usr/sbin/nologin garos && \
    mkdir -p /var/lib/garos /etc/garos && \
    chown -R garos:garos /var/lib/garos /etc/garos

COPY --from=builder /build/target/release/garos-backend /usr/local/bin/garos-backend

USER garos
WORKDIR /var/lib/garos

ENV RUST_LOG=info,garos_backend=info \
    GAROS__SERVER__BIND_ADDR=0.0.0.0 \
    GAROS__SERVER__PORT=8080

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/garos-backend"]
CMD ["serve"]
