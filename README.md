# Album Browser Scaffold

Project purpose: a scaffold for an album browser app — a Svelte SPA frontend talking to an axum backend.

## Layout

- `ui/` — frontend (Svelte SPA)
- `album/` — backend (axum)
- Repo root: `/workspace`

## Development

Run in two terminals:

- **Terminal A (backend):** `cd album && cargo run` — listens on port **3000**
- **Terminal B (frontend):** `cd ui && npm run dev` — serves on port **5173**, proxies `/api` to the backend

## Release build order

1. `cd ui && npm run build`
2. `cd album && cargo build --release`

## Prerequisite: build the frontend first

`cd ui && npm run build` must run before `cargo test` / `cargo run` in `album/` — the tests and the embedded server assert against `ui/dist` (which is gitignored, so a fresh clone has no `ui/dist` until the ui build runs).

## Health check

`http://127.0.0.1:3000/api/health`
