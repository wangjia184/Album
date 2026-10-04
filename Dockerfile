# Build ui → embed into album release binary → slim runtime image.
# Context: repo root. Build: docker build -t album .

FROM node:24-bookworm-slim AS ui
WORKDIR /src/ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci
COPY ui/ ./
RUN npm run build

FROM rust:1.98-bookworm AS rust
WORKDIR /src
COPY --from=ui /src/ui/dist /src/ui/dist
COPY album/Cargo.toml album/Cargo.lock ./
# Warm dependency cache separately from sources.
RUN mkdir -p album/src album/tests \
  && echo 'fn main() {}' > album/src/main.rs \
  && echo '' > album/src/lib.rs \
  && cargo build --release --manifest-path album/Cargo.toml || true
COPY album/ ./
RUN touch album/src/main.rs album/src/lib.rs && cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=rust /src/target/release/album /usr/local/bin/album
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/album"]
CMD ["--addr", "0.0.0.0", "--port", "3000"]