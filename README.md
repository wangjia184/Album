# Album Browser Scaffold

Project purpose: a scaffold for an album browser app — a Svelte SPA frontend talking to an axum backend.

## Layout

- `ui/` — frontend (Svelte SPA)
- `album/` — backend (axum)
- Repo root: `/workspace`

## Development

Run in two terminals:

- **Terminal A (backend):** `cd album && cargo run` — listens on `0.0.0.0:3000` (override with `--port` / `--addr`)
- **Terminal B (frontend):** `cd ui && npm run dev` — serves on port **5173**, proxies `/api` to the backend

Backend CLI (clap): `album [--port <u16>] [--addr <ip>]` — defaults `0.0.0.0:3000`. Logs (tracing) go to stdout at `info` level. The old `PORT` env var is no longer read.

Swagger UI: `http://127.0.0.1:3000/swagger-ui/` (OpenAPI JSON at `/api-doc/openapi.json`).

## Release build order

1. `cd ui && npm run build`
2. `cd album && cargo build --release`

## Prerequisite: build the frontend first

`cd ui && npm run build` must run before `cargo test` / `cargo run` in `album/` — the tests and the embedded server assert against `ui/dist` (which is gitignored, so a fresh clone has no `ui/dist` until the ui build runs).

## Health check

`http://127.0.0.1:3000/api/health`
