# syntax=docker/dockerfile:1

FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# The cache mounts keep downloaded crates and compiled dependencies between builds.
# They are not part of the image, so the binary is copied out of target/ here.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/latencylab-api /usr/local/bin/latencylab-api

FROM debian:bookworm-slim
RUN useradd --system --no-create-home --uid 10001 app
COPY --from=build /usr/local/bin/latencylab-api /usr/local/bin/latencylab-api
USER app
ENV PORT=8080
EXPOSE 8080
CMD ["latencylab-api"]
