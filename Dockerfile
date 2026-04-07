FROM rust:1.75-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/types/Cargo.toml crates/types/Cargo.toml
COPY crates/cache/Cargo.toml crates/cache/Cargo.toml
COPY crates/db/Cargo.toml crates/db/Cargo.toml
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/api/Cargo.toml crates/api/Cargo.toml

RUN mkdir -p crates/types/src && echo "" > crates/types/src/lib.rs
RUN mkdir -p crates/cache/src && echo "" > crates/cache/src/lib.rs
RUN mkdir -p crates/db/src && echo "" > crates/db/src/lib.rs
RUN mkdir -p crates/core/src && echo "" > crates/core/src/lib.rs
RUN mkdir -p crates/api/src && echo "fn main() {}" > crates/api/src/main.rs

RUN cargo build --release 2>/dev/null || true

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r npcapi && useradd -r -g npcapi npcapi

COPY --from=builder /app/target/release/npc-api /usr/local/bin/npc-api

COPY .env.example /app/.env.example

USER npcapi

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/api/v1/health || exit 1

CMD ["npc-api"]
