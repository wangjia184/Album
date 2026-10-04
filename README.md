# Album

Self-hosted photo album browser: a Svelte SPA served by a single axum binary (frontend embedded at build time).

- Browse folders and media under one or more host mounts
- Gallery, film strip, and cover-flow views with on-demand thumbnails
- Site title / footer note configurable via CLI (or env) and exposed over `GET /api/site`
- OpenAPI + Swagger UI included

## Layout

```
album/          axum backend (embeds ui/dist)
ui/             Svelte + Vite + TypeScript frontend
run_dev.sh      build ui, then run backend (:3000) + Vite (:5173)
build_release.sh  ui build + cargo release build
```

## Quick start

Requirements: Rust (cargo), Node.js (npm).

```bash
# Optional: put photos under .www (or any path you mount)
mkdir -p .www

./run_dev.sh
```

`run_dev.sh` always runs `npm run build` first so the embedded SPA is current, then starts:

| Service | URL |
|---|---|
| Backend + embedded SPA | http://127.0.0.1:3000 |
| Vite dev server (proxies `/api` → 3000) | http://127.0.0.1:5173 |

Prefer two terminals? Equivalent manual steps:

```bash
# Terminal A
cd ui && npm run build && cd ../album && cargo run -- --mount "*=../.www"

# Terminal B
cd ui && npm run dev
```

## CLI

```
album [--port <u16>] [--addr <ip>] [--mount <HOST=PATH>]... [--site-name <str>] [--site-note <str>]
```

| Flag | Env | Default | Description |
|---|---|---|---|
| `--port` | — | `3000` | Listen port |
| `--addr` | — | `0.0.0.0` | Bind address |
| `--mount HOST=PATH` | — | — | Host → filesystem root (repeatable). `*` is the fallback host (quote it: `--mount '*=/path'`). Host matching is case-insensitive. |
| `--site-name` | `ALBUM_SITE_NAME` | `""` | Navbar center title (hidden when empty) |
| `--site-note` | `ALBUM_SITE_NOTE` | `""` | Navbar right-side free text (hidden when empty) |

Example with two mounts and branding:

```bash
cargo run -- \
  --mount 'dsm=/volume1/photo' \
  --mount "*=$PWD/.www" \
  --site-name 'Album' \
  --site-note '湘ICP备17022195号'
```

Mount resolution: requests carry no host in the path; the server uses the HTTP `Host` header (port stripped), then falls back to `*`. Paths outside the resolved root → 404 JSON.

## HTTP API

Base: `/api`. Swagger UI: `http://127.0.0.1:3000/swagger-ui/` (spec: `/api-doc/openapi.json`).

| Method & path | Description |
|---|---|
| `GET /api/health` | Liveness → `{"status":"ok"}` |
| `GET /api/site` | Navbar branding → `{"siteName","siteNote"}` |
| `GET /api/fs/list[/{*path}]` | Directory listing → `{path, rootName, children:[{name,kind}]}` (dirs first) |
| `GET /api/fs/file/{*path}` | Raw file bytes (`ETag` / `If-None-Match` → 304) |
| `GET /api/fs/meta/{*path}` | Media metadata (size, dimensions, EXIF, …) |
| `GET /api/fs/thumbs/{*path}?n=` | Up to `n` image URLs for a folder (default 3, max 8) |
| `GET /api/fs/queue?offset&limit&center` | Playback window for cover-flow / strip rotation |

`kind` ∈ `dir` \| `image` \| `video` \| `other`.

## Release build

```bash
./build_release.sh
# → album/target/release/album
```

Order matters: `ui/dist` must exist before `cargo build` / `cargo run` (rust-embed embeds it). Fresh clone:

1. `cd ui && npm run build`
2. `cd album && cargo build --release`

## Tests

```bash
cd ui && npm run build   # tests assert against ui/dist
cd ../album && cargo test
```

Frontend typecheck: `cd ui && npm run check`.

## License

See [LICENSE](LICENSE).