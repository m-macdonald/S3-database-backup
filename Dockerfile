# BUIlDER
FROM rust:1.71 as builder
WORKDIR /usr/src/database-backup
COPY . .
RUN cargo install --path

# RUNTIME
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/database-backup /usr/local/bin/database-backup

