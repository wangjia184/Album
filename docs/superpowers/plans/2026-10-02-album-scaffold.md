# Album Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold a Svelte SPA (`ui/`) and an axum backend (`album/`) that serves `/api/health` and the embedded static build with hash-based SPA fallback.

**Architecture:** Two independent projects under `/workspace`. Frontend builds to `ui/dist` with local Tailwind/DaisyUI (no CDN) and hash routing. Backend (axum) mounts JSON API under `/api/`, embeds `ui/dist` via `rust-embed`, and falls back to `index.html` for unmatched paths. Dev uses Vite proxy + CORS for `http://localhost:5173`.

**Tech Stack:** Svelte 5, Vite, TypeScript, Tailwind CSS 4, DaisyUI, svelte-spa-router; Rust, axum, tokio, tower-http (cors), rust-embed, serde/serde_json.

**Spec:** `docs/superpowers/specs/2026-10-02-album-scaffold-design.md`

## Global Constraints

- Layout: frontend root = `/workspace/ui/`, backend root = `/workspace/album/`, repo root = `/workspace`.
- CSS/JS framework assets: **local npm packages only — no CDN** (`cdn.tailwindcss.com`, `cdn.jsdelivr.net`, etc. must not appear in source `index.html` or `ui/dist`).
- Routing: **hash mode** (`#/...`) via `svelte-spa-router`.
- Backend framework: **axum**; API prefix **`/api`**; phase 1 API = **only** `GET /api/health` returning JSON `{"status":"ok"}`.
- Static assets: **embedded** with `rust-embed` from folder `../ui/dist` (relative to `album/`).
- Unmatched non-API paths and `/` → serve embedded `index.html`.
- CORS must allow origin `http://localhost:5173` (methods sufficient for GET/HEAD/OPTIONS JSON API).
- Default listen: port **3000** (env `PORT` may override).
- Vite `base: './'`; dev proxy `'/api'` → `http://127.0.0.1:3000`.
- Git: initialize repo at `/workspace` if absent; commit after each task.

## Review Focus

- **CDN leak in built CSS/HTML:** `ui/dist` and `ui/index.html` contain zero external CDN URLs for Tailwind/DaisyUI — pin with Task 2 grep step.
- **Embed path wrong or empty `ui/dist`:** `cargo build` fails or `/` returns 500 — pin with Task 4 fixture `index.html` + tests for `/` and unknown path.
- **Path traversal in static handler:** request `/../Cargo.toml` or encoded `..` must 404, not leak files — pin with Task 4 test.
- **CORS blocks Vite origin:** preflight/simple GET from `http://localhost:5173` must include `access-control-allow-origin` — pin with Task 3 test asserting CORS headers on `/api/health`.
- **Hash SPA deep-link hard refresh:** `GET /unknown` and `GET /` must return HTML shell (200), not 404 — pin with Task 4 tests.

---

### Task 1: Repo init and root README

**Files:**
- Create: `/workspace/.gitignore`
- Create: `/workspace/README.md`
- (If missing) init git at `/workspace`

**Interfaces:**
- Consumes: nothing
- Produces: documented build order and ports for later tasks; `.gitignore` covers `node_modules/`, `ui/dist/`, `album/target/`

- [ ] **Step 1: Initialize git if needed and create `.gitignore`**

```gitignore
node_modules/
ui/dist/
album/target/
.DS_Store
*.log
```

- [ ] **Step 2: Create root `README.md`**

Content must state: project purpose (album browser scaffold); layout (`ui/`, `album/`); dev: terminal A `cd album && cargo run` (port 3000), terminal B `cd ui && npm run dev` (5173, proxies `/api`); release order: `cd ui && npm run build` then `cd album && cargo build --release`; health URL `http://127.0.0.1:3000/api/health`.

- [ ] **Step 3: Verify files exist**

Run: `test -f /workspace/.gitignore && test -f /workspace/README.md && git -C /workspace rev-parse --is-inside-work-tree`
Expected: `true`

- [ ] **Step 4: Commit**

```bash
git -C /workspace add .gitignore README.md docs/
git -C /workspace commit -m "chore: init repo with scaffold README and specs"
```

---

### Task 2: Frontend scaffold (Svelte + Vite + Tailwind/DaisyUI + hash router)

**Files:**
- Create: `/workspace/ui/` (via Vite scaffold, then adjust)
- Create/Modify: `ui/package.json`, `ui/vite.config.ts`, `ui/index.html`, `ui/src/main.ts`, `ui/src/app.css`, `ui/src/App.svelte`, `ui/src/routes.ts`, `ui/src/pages/Home.svelte`, `ui/svelte.config.js`, `ui/tsconfig.json` (and Tailwind setup files as required by TW4)

**Interfaces:**
- Consumes: Task 1 layout
- Produces:
  - `npm run build` → `ui/dist/index.html` + hashed assets under `ui/dist/assets/`
  - Hash routes exported from `ui/src/routes.ts` as route table for `svelte-spa-router` (`export const routes = { '/': Home }`)
  - Dev server proxy for `/api` (consumed implicitly by browser / Task 5)
  - Placeholder Home page using DaisyUI classes

- [ ] **Step 1: Scaffold Vite + Svelte + TS project in `ui/`**

Run (from `/workspace`): `npm create vite@latest ui -- --template svelte-ts` (non-interactive equivalent; do not overwrite if interactive prompt needed — use `--overwrite` only on empty dir). Then `cd ui && npm install`.

Expected: `ui/package.json` exists with `vite`, `svelte`, `typescript`.

- [ ] **Step 2: Add UI deps and hash router**

Run in `ui/`: `npm install` tailwindcss @tailwindcss/vite daisyui svelte-spa-router  
(If DaisyUI current major documents different peer setup for TW4, follow package README for Vite plugin wiring — still **local npm only**.)

Expected: deps listed in `package.json` dependencies/devDependencies; no CDN instructions left in HTML.

- [ ] **Step 3: Configure Vite `base` and `/api` proxy**

In `ui/vite.config.ts` set `base: './'` and:

```ts
server: {
  proxy: {
    '/api': 'http://127.0.0.1:3000',
  },
},
```

- [ ] **Step 4: Wire Tailwind 4 + DaisyUI as local CSS**

`ui/src/app.css`: Tailwind 4 entry (`@import "tailwindcss";` + DaisyUI plugin per current docs). Import `./app.css` from `src/main.ts`. Ensure `ui/index.html` has **no** external `<script>`/`<link>` to CDN hosts.

- [ ] **Step 5: Add hash routes and Home placeholder**

- `src/routes.ts`: `import Home from './pages/Home.svelte'; export const routes = { '/': Home };`
- `src/App.svelte`: DaisyUI navbar + `<Router {routes} />` from `svelte-spa-router`.
- `src/pages/Home.svelte`: minimal DaisyUI `hero`/`card` with text `Album scaffold`.

- [ ] **Step 6: Build and fail if CDN references exist**

Run: `cd /workspace/ui && npm run build && ! grep -rE 'cdn\.tailwindcss\.com|cdn\.jsdelivr\.net|unpkg\.com' dist index.html src`
Expected: build success; grep finds no matches (exit of `!` grep is success).

- [ ] **Step 7: Smoke-check built HTML**

Run: `grep -q 'Album scaffold\|root\|app' /workspace/ui/dist/index.html || test -s /workspace/ui/dist/index.html`
Expected: `dist/index.html` non-empty; assets directory present: `test -d /workspace/ui/dist/assets`

- [ ] **Step 8: Commit**

```bash
git -C /workspace add ui
git -C /workspace commit -m "feat(ui): scaffold Svelte SPA with Tailwind/DaisyUI and hash routes"
```

---

### Task 3: Backend health API + CORS (TDD)

**Files:**
- Create: `/workspace/album/Cargo.toml`
- Create: `/workspace/album/src/main.rs`
- Create: `/workspace/album/src/api.rs`
- Create: `/workspace/album/tests/health.rs` (integration) **or** unit tests in `api.rs` using `oneshot` — prefer `album/tests/health.rs`

**Interfaces:**
- Consumes: nothing from UI yet (static handler is Task 4)
- Produces:
  - `api::router() -> axum::Router` — routes under `/api` including `GET /api/health` and JSON 404 fallback for other `/api/*`
  - `main` listens on `0.0.0.0:{PORT or 3000}` with CORS layer allowing `http://localhost:5173`
  - Health response: `200`, `Content-Type: application/json`, body exactly `{"status":"ok"}` (JSON object with string field `status`)

- [ ] **Step 1: Write failing integration test `album/tests/health.rs`**

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt; // requires tower with util feature for tests

// app under test: crate must expose a way to build the API router.
// If only a binary exists, put `build_app()` in main.rs as `pub fn` behind `mod` visibility
// OR convert to lib+bin: src/lib.rs exports build_app(), src/main.rs calls it.
// Prefer: src/lib.rs with `pub fn build_app() -> Router` used by both main and tests.
```

Assertions:
1. `GET /api/health` → status `200`, body parses as JSON with `status == "ok"`.
2. `GET /api/nope` → status `404`, JSON body has `"error"` field (value `not_found`).
3. Same request with `Origin: http://localhost:5173` includes response header `access-control-allow-origin` equal to `http://localhost:5173` (or `*` if layer allows that — **spec pins allow of 5173 origin**).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /workspace/album && cargo test`
Expected: FAIL (no crate / no `build_app` yet)

- [ ] **Step 3: Create `Cargo.toml` and `src/lib.rs` + `src/api.rs`**

Dependencies: `axum`, `tokio` (macros, rt-multi-thread, net), `tower-http` (cors), `serde`, `serde_json`, `serde` derive; dev-deps: `tower` (util), `http-body-util` as needed.

- `api.rs`: `pub fn router() -> Router` with `GET /api/health` handler returning `Json(json!({"status":"ok"}))` or typed struct `{ status: &'static str }`, plus nested fallback for `/api/*` → 404 JSON `{"error":"not_found"}`.
- `lib.rs`: `pub fn build_app() -> Router` = `api::router()` + `CorsLayer` for origins `http://localhost:5173` and `http://127.0.0.1:5173`, methods `GET,HEAD,OPTIONS`, appropriate headers. (Task 4 replaces/extends fallback.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /workspace/album && cargo test`
Expected: PASS (all three assertions)

- [ ] **Step 5: Implement `src/main.rs` listen loop**

Read `PORT` env (default `3000`), bind `0.0.0.0:port`, `axum::serve` with `build_app()`.

- [ ] **Step 6: Manual verify (optional if CI-less)**

Run: `cd /workspace/album && cargo run &` then `curl -s http://127.0.0.1:3000/api/health`  
Expected: `{"status":"ok"}` (static 500/404 acceptable until Task 4)

- [ ] **Step 7: Commit**

```bash
git -C /workspace add album
git -C /workspace commit -m "feat(album): axum /api/health with CORS for Vite origin"
```

---

### Task 4: Embedded static files + index.html fallback (TDD)

**Files:**
- Create: `/workspace/album/src/static_files.rs`
- Modify: `/workspace/album/src/lib.rs` (wire fallback + module)
- Modify: `/workspace/album/Cargo.toml` (`rust-embed`)
- Create: `/workspace/album/tests/static_files.rs`
- Ensure embed source: `/workspace/ui/dist` exists at compile time (Task 2 artifact, or fixture Step 1)

**Interfaces:**
- Consumes: `build_app()` from Task 3 (same crate)
- Produces:
  - `static_files::static_handler(axum::extract::Path<String>) -> Response` (or `axum::response::Response`)
  - Behavior: path normalize; reject `..` → 404; hit embed → bytes+MIME; miss → embed `index.html` as `text/html`; missing `index.html` in embed → 500
  - `build_app()` fallback = `static_handler` for all non-`/api` routes

- [ ] **Step 1: Ensure embed fixture exists**

If `/workspace/ui/dist/index.html` missing: create minimal placeholder `ui/dist/index.html` containing `<div id="app">Album scaffold</div>` (real build from Task 2 preferred when present).  
`rust-embed` folder attribute: `#[folder = "../ui/dist"]` (path relative to `album/src` or crate root per rust-embed rules — use path that compiles from `album/`).

- [ ] **Step 2: Write failing tests `album/tests/static_files.rs`**

Using `build_app()` + `oneshot`:
1. `GET /` → `200`, `content-type` contains `text/html`, body contains `Album scaffold` or non-empty HTML.
2. `GET /does-not-exist` → `200`, `text/html` (SPA shell fallback).
3. `GET /../Cargo.toml` (or path that normalizes to parent) → `404`.
4. If a known asset exists under `ui/dist` (e.g. after real build, a file under `assets/`): optional assert 200; skip if only placeholder index (use conditional or only fixture-only asserts if no real dist).

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /workspace/album && cargo test --test static_files`
Expected: FAIL (no fallback yet / 404 from missing route)

- [ ] **Step 4: Implement `static_files.rs` and wire into `build_app()`**

- `RustEmbed` struct with `#[folder = "<correct relative path to ui/dist>"]`.
- Handler: percent-decode optional; reject `..` components; `Assets::get(path)`; else `Assets::get("index.html")`.
- Set content-type via rust-embed metadata or `mime_guess`; missing index → `500` + `eprintln!` hint to run `ui` build.
- `lib.rs`: `Router::new().merge(api::router()).fallback(static_files::static_handler)` (keep API 404 under `/api` from Task 3 — merge order such that `/api/*` not swallowed by static; e.g. nest api at `/api` with its own fallback, static as outer fallback only for non-API — **implementer: API routes registered first; static fallback must not override `/api/health`.**)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /workspace/album && cargo test`
Expected: PASS including Task 3 health tests and Task 4 static tests

- [ ] **Step 6: Manual E2E**

Run: `cd /workspace/ui && npm run build && cd ../album && cargo run`  
Then: `curl -s http://127.0.0.1:3000/api/health` and `curl -s http://127.0.0.1:3000/`  
Expected: health JSON; `/` returns HTML.

- [ ] **Step 7: Commit**

```bash
git -C /workspace add album ui
git -C /workspace commit -m "feat(album): embed ui/dist and SPA index.html fallback"
```

---

### Task 5: End-to-end acceptance

**Files:**
- Modify: `/workspace/README.md` only if commands drifted

**Interfaces:**
- Consumes: Tasks 2–4
- Produces: verified acceptance criteria from spec §10

- [ ] **Step 1: Full build chain**

```bash
cd /workspace/ui && npm run build
cd /workspace/album && cargo test && cargo build --release
```

Expected: all succeed; no CDN grep failures (re-run Task 2 Step 6 grep).

- [ ] **Step 2: Serve and curl acceptance**

Start `cargo run` (debug or release).  
`curl -s http://127.0.0.1:3000/api/health` → `{"status":"ok"}`  
`curl -s -o /dev/null -w '%{http_code} %{content_type}' http://127.0.0.1:3000/` → `200` + `text/html`  
`curl -s -H 'Origin: http://localhost:5173' -D- -o /dev/null http://127.0.0.1:3000/api/health` → contains `access-control-allow-origin`

- [ ] **Step 3: Dev proxy check (manual or scripted)**

With backend running: `cd ui && npm run dev`, then `curl -s http://127.0.0.1:5173/api/health`  
Expected: `{"status":"ok"}` via Vite proxy.

- [ ] **Step 4: Final commit if README updated**

```bash
git -C /workspace add -A
git -C /workspace commit -m "chore: verify scaffold acceptance end-to-end"
```

(Only if changes exist; empty commit not required.)