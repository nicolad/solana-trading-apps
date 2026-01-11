# ---- build ----
FROM rust:1.75-slim AS builder
WORKDIR /app

# Faster Docker layer caching
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

# ---- run ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/laserstream_container /usr/local/bin/laserstream_container

ENV PORT=8080
EXPOSE 8080

CMD ["/usr/local/bin/laserstream_container"]
