FROM debian:bookworm-slim AS base
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

FROM rust:1.98.1-slim-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY ./Cargo.toml ./Cargo.lock ./

RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release
COPY ./src ./src
RUN touch src/main.rs && cargo build --release

FROM base AS final
WORKDIR /app
RUN useradd -m -u 1000 appuser
COPY --from=build /usr/src/app/target/release/steal_stickers_bot .
USER appuser
ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/steal_stickers_bot"]