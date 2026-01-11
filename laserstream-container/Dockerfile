# syntax=docker/dockerfile:1

FROM rust:1.83-slim AS build

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    build-essential \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust source
COPY container_src/Cargo.toml ./
COPY container_src/src ./src

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/laserstream_container /laserstream_container
EXPOSE 8080

CMD ["/laserstream_container"]
