FROM rust:1.95-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY migrations ./migrations
COPY contracts ./contracts
COPY specs/010-managed-integrations/fixtures ./specs/010-managed-integrations/fixtures
RUN cargo build --release -p kusi-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates ffmpeg fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/kusi-server /usr/local/bin/kusi-server
COPY migrations ./migrations
COPY contracts ./contracts
EXPOSE 8080
ENTRYPOINT ["kusi-server"]
