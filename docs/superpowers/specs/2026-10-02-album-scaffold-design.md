# Album Scaffold Design

**Date:** 2026-10-02  
**Status:** Approved (approach A)  
**Scope:** Phase 1 — project scaffolding only (no album business features)

## 1. Goal

Build the application framework for a photo-album browser:

1. A **Svelte SPA** frontend that compiles to a static site (hash routing, DaisyUI + Tailwind CSS via local npm packages — no CDN).
2. A **Rust (cargo) backend** that exposes REST APIs under `/api/`, serves the built SPA as static files, and falls back to `index.html` for unmatched paths (including `/`).

Phase 1 delivers both scaffolds wired end-to-end with a minimal health endpoint; album features come later.

## 2. Constraints (from stakeholder)

| Constraint | Value |
|---|---|
| Frontend framework | Svelte (SPA) |
| UI CSS | DaisyUI + Tailwind CSS, **local npm deps only — no CDN** |
| Routing | **Hash** mode (`#/...`) |
| Build output | Static SPA files |
| Backend | `cargo` / Rust, **axum** |
| API prefix | `/api/**` RESTful |
| Static serving | **Embed `ui/dist` into the backend binary** (`rust-embed` or equivalent) |
| Unmatched routes | Serve static file if present; `/` and SPA shell → `index.html` |
| CORS | Enabled for debug (`vite dev` origin) so SPA can call API directly |
| Scaffold API | **Only** `GET /api/health` in phase 1 |
| Layout | `/workspace/ui/` = frontend, `/workspace/album/` = backend (repo root = `/workspace`) |

## 3. Architecture Overview

```
Browser
  │  hash URLs: http://host/#/...
  ▼
album (axum :3000)
  ├── /api/**     → JSON handlers (phase 1: /api/health)
  └── /*          → rust-embed(ui/dist)
                      ├── exact file match → bytes + MIME
                      └── else → index.html (SPA shell)
```

**Dev mode:**

- Frontend: `npm run dev` (Vite, e.g. `:5173`); `server.proxy['/api']` → `http://127.0.0.1:3000`.
- Backend: `cargo run` with CORS allowing `http://localhost:5173` (proxy and direct CORS both work).

**Release build order:** `ui`: `npm run build` → `album`: `cargo build --release` (embed picks up `../ui/dist`).

## 4. Directory Structure

```
/workspace/
  ui/                          # Svelte SPA
    package.json
    vite.config.ts             # base: './'; server.proxy['/api'] → 127.0.0.1:3000
    svelte.config.js
    tsconfig.json
    index.html
    postcss / tailwind config as required by Tailwind 4 + DaisyUI
    src/
      main.ts
      App.svelte
      app.css                  # tailwind + daisyui plugin (build-time CSS)
      lib/
      routes.ts                # hash route table (svelte-spa-router or equivalent)
      pages/
        Home.svelte            # placeholder DaisyUI layout
  album/                       # Rust backend
    Cargo.toml
    src/
      main.rs                  # bind, CORS, router assembly
      api.rs                   # GET /api/health
      static_files.rs          # RustEmbed + fallback to index.html
  docs/
    superpowers/
      specs/2026-10-02-album-scaffold-design.md   # this file
      plans/…
  README.md                    # build/run order, ports, env
```

**File responsibilities (one job each):**

| File | Responsibility |
|---|---|
| `ui/vite.config.ts` | Static base path, dev proxy for `/api` |
| `ui/src/app.css` | Tailwind + DaisyUI import surface (no CDN) |
| `ui/src/routes.ts` | Hash route table only |
| `ui/src/pages/Home.svelte` | Placeholder shell proving DaisyUI + router |
| `album/src/api.rs` | HTTP handlers under `/api` |
| `album/src/static_files.rs` | Embedded assets + `index.html` fallback |
| `album/src/main.rs` | Process entry: config, CORS, route merge, listen |

## 5. Frontend Design

### 5.1 Toolchain

- **Vite + Svelte + TypeScript** (`npm create vite` baseline, then adjust).
- **Svelte** current stable (Svelte 5 syntax acceptable).
- **Tailwind CSS 4** + **DaisyUI** installed via npm; styles compiled by Vite into `dist/assets/*.css`.
- **Forbidden:** any `<script src="https://cdn…">` or `<link href="https://cdn…">` for CSS/JS framework assets.

### 5.2 Styling entry (pattern)

- `src/app.css` uses Tailwind 4 style entry (e.g. `@import "tailwindcss";` and DaisyUI plugin registration per current DaisyUI docs for TW4).
- Imported once from `main.ts`.
- `index.html` contains no external stylesheet URLs.

### 5.3 Hash routing

- Library: **`svelte-spa-router`** (or equivalent hash router if package constraints force a swap — same interface: route table + `#/` navigation).
- Route table in `src/routes.ts`; at minimum:
  - `'/': Home`
- `App.svelte` mounts `<Router {routes}>` and a minimal DaisyUI navbar with link to `#/`.

### 5.4 Placeholder UI

- `Home.svelte`: DaisyUI navbar + one card/hero stating the app is scaffolded; enough to prove DaisyUI classes compile and render.
- No album list/detail/lightbox in phase 1.

### 5.5 Vite config

- `base: './'` so assets work when served from any path by axum.
- `server.proxy`: `{ '/api': 'http://127.0.0.1:3000' }`.
- Output dir: `dist/` (default).

## 6. Backend Design (axum)

### 6.1 Dependencies (`album/Cargo.toml`)

Expected crates (versions: current stable on crates.io at implementation time):

- `axum`
- `tokio` (features: `full` or at least `macros`, `rt-multi-thread`, `net`)
- `tower-http` (features: `cors`)
- `rust-embed` (or `include_dir` — **choose `rust-embed`** for MIME + dev rebuild ergonomics)
- `serde`, `serde_json`
- `anyhow` or `thiserror` (light use OK)

### 6.2 Configuration

| Setting | Default | Override |
|---|---|---|
| Listen address | `0.0.0.0:3000` | env `ADDR` or `PORT` (pick **`ADDR` full form optional; minimum `PORT=3000`**) |
| CORS allowed origin (dev) | `http://localhost:5173` | env optional later; scaffold = constant or env with that default |
| Embed folder | `../ui/dist` relative to `album/` | fixed in `#[folder = …]` |

### 6.3 Routes

| Method | Path | Behavior |
|---|---|---|
| `GET` | `/api/health` | `200` `application/json` body `{"status":"ok"}` |
| `GET` | `/api/*` (unmatched) | `404` JSON `{"error":"not_found"}` (consistent JSON errors) |
| any | `/*` | Static embed pipeline (below) |

### 6.4 Static pipeline (`static_files.rs`)

1. Normalize request path (strip leading `/`; reject `..` segments → 404).
2. If embed has file at path → return bytes + content-type (rust-embed MIME).
3. Else → serve embed `index.html` (SPA shell). Covers `/` and unknown paths.
4. If `index.html` missing from embed → `500` with clear log message: run `ui` build first.

**Note:** With hash routing, client-side routes never hit the server as path segments; fallback mainly serves `/` and any hard refresh on `/`.

### 6.5 CORS

- `tower_http::cors::CorsLayer` allowing:
  - origin: `http://localhost:5173` (and optionally `http://127.0.0.1:5173`)
  - methods: `GET`, `HEAD`, `OPTIONS` (enough for phase 1; include `POST` etc. if cheap)
  - headers: common content-type / accept as needed for JSON APIs
- Applied to the whole router (or at least `/api` — whole router is simpler for scaffold).

### 6.6 Assembly (`main.rs`)

```text
Router::new()
  .merge(api_routes)           // /api/health, /api fallback
  .fallback(static_handler)    // embed + index.html
  .layer(cors)
→ axum::serve(TcpListener::bind(addr), app)
```

## 7. Error Handling

- API handlers return JSON errors with appropriate status (`404`, `500`).
- Static: missing embed root → `500` + log; path escape → `404`.
- No panic on missing optional assets beyond the clear `index.html` failure above.

## 8. Testing / Verification

| Layer | Check |
|---|---|
| Backend unit/integration | `cargo test`: `GET /api/health` → 200 + `{"status":"ok"}` (use `axum::Router` + `tower::ServiceExt::oneshot` or spawn test) |
| Backend static | test or manual: `/` returns HTML containing SPA root; unknown path returns HTML (index fallback) — requires `ui/dist` present in CI/test (document or fixture) |
| Frontend build | `npm run build` succeeds; `dist/` has `index.html` + hashed CSS/JS; **grep dist for `cdn.jsdelivr` / `cdn.tailwindcss.com` → no matches** |
| Dev联调 | `npm run dev` + `cargo run`; browser or `curl` via proxy reaches `/api/health` |
| E2E manual | `cargo run` after ui build; `curl localhost:3000/api/health`; `curl localhost:3000/` → index.html |

## 9. Out of Scope (phase 1)

- Album/photo domain APIs, data model, storage, upload, thumbnails.
- Auth, HTTPS, production CORS lockdown details (beyond a sane default).
- Docker/deploy pipelines (optional README notes only).
- CI.

## 10. Acceptance Criteria

1. `cd ui && npm install && npm run build` produces `ui/dist/index.html` and assets; no CDN references in `dist`.
2. `cd album && cargo run` starts on port 3000 (after ui build so embed succeeds).
3. `curl -s localhost:3000/api/health` → `{"status":"ok"}`.
4. `curl -sI localhost:3000/` or GET returns `index.html` content type HTML.
5. Vite dev server proxies or CORS-allows calls to `/api/health`.
6. Hash URL `http://localhost:3000/#/` loads the scaffold home page with DaisyUI styles from local bundle.

## 11. Open Points (non-blocking)

- Exact Tailwind 4 + DaisyUI npm package versions: pin at implementation to mutually compatible releases.
- `PORT` vs `ADDR` env name: implementer may use `PORT` only if simpler; default remains 3000.
- Whether tests embed a tiny fixture `index.html` vs requiring prior `ui build`: implementer choice; document in README.