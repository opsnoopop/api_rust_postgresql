# ---- Builder ----
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.* ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release || true

COPY . .
RUN cargo build --release

# ---- Runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libgcc-s1 libstdc++6 libc-bin && rm -rf /var/lib/apt/lists/*
RUN useradd -m appuser
WORKDIR /app
COPY --from=builder /app/target/release/rust-axum-mysql-api /usr/local/bin/app
EXPOSE 3000
USER appuser
CMD ["/usr/local/bin/app"]