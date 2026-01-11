# syntax=docker/dockerfile:1

FROM rust:1.83-slim AS build

WORKDIR /app

# Copy Rust source
COPY container_src_rust/Cargo.toml ./
COPY container_src_rust/src ./src

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/laserstream-service /laserstream-service
EXPOSE 8080

# Run
CMD ["/laserstream-service"]
