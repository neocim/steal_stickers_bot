FROM debian:bullseye-slim AS base
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false

FROM rust:1.88.0-slim-bullseye AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl-dev \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false
WORKDIR /usr/src/app
RUN USER=root cargo init
COPY ./Cargo.toml .
RUN cargo build --release
COPY ./src ./src
RUN touch src/main.rs && cargo build --release

FROM base AS final
WORKDIR /app
COPY --from=build /usr/src/app/target/release/steal_stickers_bot .
VOLUME /configs
ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/steal_stickers_bot"]
