# BUIlDER
FROM nixos/nix:latest as builder
WORKDIR /usr/src/database-backup
COPY . .
RUN nix \
    --extra-experimental-features "nix-command flakes" \
    --option filter-syscalls false \
    build

# RUNTIME
FROM alpine:3.14
RUN apk add --update --no-cache postgresql-client
#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/database-backup /usr/local/bin/database-backup

ENTRYPOINT ["database-backup"]
