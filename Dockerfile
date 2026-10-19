FROM debian:bookworm-slim AS base
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false

FROM rust:1.98.1-slim-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl-dev \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false

WORKDIR /usr/src/app
COPY ./Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src target/release/deps/steal_stickers_bot*
COPY ./src ./src
RUN cargo build --release

FROM base AS final
WORKDIR /app

RUN useradd -m -u 1000 appuser
COPY --from=build /usr/src/app/target/release/steal_stickers_bot .

USER appuser
ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/steal_stickers_bot"]