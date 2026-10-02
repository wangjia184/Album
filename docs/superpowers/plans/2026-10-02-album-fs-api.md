# Album FS Read API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship read-only list + file endpoints under `/api/fs/{host}/…` with path confinement, and the sync `AlbumFs` type they call via `spawn_blocking`.

**Architecture:** Two tasks: (1) pure sync library `AlbumFs` in `album/src/fs.rs` (canonical root, `list_children` / `open_file`, confinement, sort, kinds) with unit tests on a temp fixture; (2) axum handlers + `MountTable` app state + `spawn_blocking` + weak ETag/file stream, integration-tested on the same fixture pattern. No app cache, no watcher, no DashMap registry.

**Tech Stack:** Rust, axum 0.8, tokio (`spawn_blocking`), serde, `mime_guess` (already present via rust-embed features — use direct dep if needed), `tokio-util` (io feature) for file streaming; existing `MountTable`, CORS, swagger unchanged except `build_app` state.

**Spec:** `docs/superpowers/specs/2026-10-02-album-fs-api-design.md`

## Global Constraints

- No application cache / FS watcher / global registry (spec §2).
- Sort: all dirs by `name`, then all files by `name` (codepoint order).
- List returns **all** direct children with `kind` ∈ `dir|image|video|other` (lowercase JSON); no server filter, no pagination.
- File response body = **raw bytes** (not JSON); ETag only on files: `W/"{mtime_ns:x}-{size:x}"`; `If-None-Match` match → `304`.
- Symlink and `..` escape → `404` `{"error":"not_found"}` (same as unknown host / missing); no 403.
- `AlbumFs` methods **sync**; HTTP handlers **async** and must use `tokio::task::spawn_blocking` for FS work.
- Image exts: `jpg jpeg png gif webp bmp avif`; video: `mp4 mov mkv webm avi m4v` (case-insensitive).
- Stage only intended paths (`album/`, maybe `README.md`); never `.env`; never `git add -A`.
- After each task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full `cargo test` green (prior suites included).

## Review Focus

- **Symlink escape:** fixture with symlink `root/out -> /tmp` must 404 on list and file — pin Task 1 unit test + Task 2 integration test.
- **`..` in URL path:** `GET /api/fs/{h}/list/../secret` (and encoded `%2e%2e`) must 404, not leak — pin Task 2 test.
- **Sort order:** dirs before files, each name-sorted — pin Task 1 unit test asserting exact `Vec` order.
- **Raw file + ETag:** bytes match disk; second request with ETag → 304 — pin Task 2 tests 4–5 (spec §6).
- **build_app signature change:** all prior health/CORS/swagger/static tests must still pass with empty `MountTable` — pin Task 2 Step “full suite”.

---

### Task 1: `AlbumFs` library (TDD)

**Files:**
- Create: `/workspace/album/src/fs.rs`
- Modify: `/workspace/album/src/lib.rs` (`pub mod fs;`)
- Dev-deps: `tempfile` in `album/Cargo.toml` (unit fixture); Unix symlink tests `#[cfg(unix)]`
- Test: unit tests in `fs.rs`

**Interfaces:**
- Consumes: nothing from this plan
- Produces:
  - `pub struct AlbumFs { root: PathBuf }` (root private, canonical absolute)
  - `pub enum ChildKind { Dir, Image, Video, Other }` with `Serialize` as lowercase `dir|image|video|other` (serde rename_all or explicit)
  - `pub struct ListedChild { pub name: String, pub kind: ChildKind }`
  - `pub fn AlbumFs::new(root: impl AsRef<Path>) -> io::Result<Self>`
  - `pub fn list_children(&self, rel: &str) -> io::Result<Vec<ListedChild>>`
  - `pub fn open_file(&self, rel: &str) -> io::Result<std::fs::File>`
  - Internal (not necessarily pub): path join + canonicalize + `starts_with(root)` confinement used by both methods

- [ ] **Step 1: Write failing unit tests in `fs.rs`**

Fixture helper: create `tempfile::TempDir` with layout:

```text
root/
  b_dir/          (empty)
  a_dir/
    nested.txt
  z_file.txt
  a_file.jpg
  m_clip.mp4
  other.bin
  escape_link -> (unix) symlink to tempdir outside root, or to /
```

Tests (exact names):

```rust
#[test] fn new_rejects_missing_root()
#[test] fn list_root_sorts_dirs_then_files() 
// expect names order: ["a_dir","b_dir","a_file.jpg","m_clip.mp4","other.bin","z_file.txt"]
// kinds: Dir, Dir, Image, Video, Other, Other

#[test] fn list_nested_relative_path()
// list_children("a_dir") → only nested.txt as Other (or name check)

#[test] fn list_rejects_dotdot()
// list_children("../x") is Err (or NotFound-equivalent Err)

#[test] fn list_rejects_absolute()
// list_children("/etc") is Err

#[cfg(unix)]
#[test] fn list_symlink_escape_is_err()
// list_children("escape_link") or path under it → Err

#[test] fn open_file_returns_file_with_contents()
// open_file("z_file.txt") read_to_string == fixture contents

#[test] fn open_file_on_directory_is_err()
#[cfg(unix)]
#[test] fn open_symlink_escape_is_err()
```

For confinement errors, use `io::ErrorKind::NotFound` (or `PermissionDenied` — **pick NotFound** so HTTP maps uniformly to 404).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/album && cargo test --lib fs`
Expected: FAIL (module missing)

- [ ] **Step 3: Implement `fs.rs` + `pub mod fs;`**

- `new`: canonicalize root; error if not a directory.
- `rel` parse: reject absolute, `..`, `.` components, interior `\0`; empty → root.
- Join → `canonicalize` → `starts_with(&self.root)` else `NotFound`.
- `list_children`: `read_dir` on confined dir; build `ListedChild` (name from file_name; kind via `file_type().is_dir()` or extension table); sort dirs then files by name.
- `open_file`: confined path must be a file; `std::fs::File::open`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib fs`
Expected: PASS (all unit tests)

- [ ] **Step 5: fmt / clippy / full test**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: green

- [ ] **Step 6: Commit**

```bash
git -C /workspace add album/src/fs.rs album/src/lib.rs album/Cargo.toml album/Cargo.lock
git -C /workspace commit -m "feat(album): AlbumFs list/open with path confinement"
```

---

### Task 2: HTTP `/api/fs` + state + ETag (TDD)

**Files:**
- Create: `/workspace/album/src/fs_api.rs` (handlers + routes; keeps `api.rs` health-focused)
- Modify: `/workspace/album/src/api.rs` (expose nest or merge fs routes; **or** mount from `lib.rs` — prefer **merge in `lib.rs`**: `api::router()` unchanged except optional `utoipa` paths; `fs_api::router()` merged inside `/api`)
- Modify: `/workspace/album/src/lib.rs` (`pub mod fs_api`; `build_app(state)`; merge routes under `/api` without breaking nested health fallback)
- Modify: `/workspace/album/src/main.rs` (`AppState`, pass mounts into `build_app`)
- Modify: existing tests using `build_app()` → `build_app(AppState::empty())` or equivalent
- Modify: `/workspace/run_dev.sh` — pass `--mount` so `*` points at repo `.www` for local testing
- Modify: `/workspace/album/Cargo.toml` — `tokio = { features += ["io-util"] }` if needed; `tokio-util` with `io`; ensure `mime_guess` available for Content-Type
- Create: `/workspace/album/tests/fs_api.rs`
- Fixture: `tempfile` in integration tests (same layout as Task 1)

**Interfaces:**
- Consumes:
  - `album::fs::{AlbumFs, ListedChild, ChildKind}`
  - `album::mount::MountTable` (`get(&self, hostname: &str) -> Option<&PathBuf>`); add `#[derive(Clone)]` on `MountTable` if missing; add `Default` or `empty()` for tests
- Produces:
  - `pub struct AppState { pub mounts: MountTable }` (`Clone`) — location: `lib.rs` or `fs_api.rs`
  - `pub fn build_app(state: AppState) -> axum::Router` (replaces `build_app()`; all prior call sites updated)
  - Routes: `GET /api/fs/{host}/list`, `GET /api/fs/{host}/list/{*path}`, `GET /api/fs/{host}/file/{*path}`
  - List JSON: `{ "path": string, "children": [{ "name": string, "kind": "dir"|"image"|"video"|"other" }] }`
  - Errors: 404 `{"error":"not_found"}`; 500 `{"error":"internal"}`
  - File: raw body, `Content-Type` from extension (`mime_guess` or `application/octet-stream`), `ETag`, conditional 304

**Routing note (implement carefully):** Health uses `Router::new().nest("/api", api_routes)` with **inner** fallback. FS routes must join that nest **without** being swallowed and without breaking `/api/nope` JSON 404. Recommended shape:

```rust
// api.rs
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(fs_api::routes())   // paths relative: /fs/{host}/list ...
        .fallback(api_not_found)
}
// nest /api unchanged in lib.rs — fs_api::routes() has NO fallback
```

- [ ] **Step 1: Write failing integration tests `album/tests/fs_api.rs`**

Setup: temp fixture tree (as Task 1); `AppState { mounts: MountTable from [("t", fixture_root)] }`; `build_app(state)`; `oneshot` requests.

```rust
#[tokio::test] async fn list_root_ok_and_sorted()
// GET /api/fs/t/list → 200; parse JSON; dirs first; kinds lowercase

#[tokio::test] async fn list_nested_path_echoes_normalized_path()
// GET /api/fs/t/list/a_dir → 200; body["path"]=="a_dir"; children contain nested file

#[tokio::test] async fn list_dotdot_returns_404_json()
// GET /api/fs/t/list/../etc → 404; json error=="not_found"

#[tokio::test] async fn list_unknown_host_404()
// GET /api/fs/nope/list → 404

#[tokio::test] async fn file_returns_raw_bytes_and_content_type()
// GET /api/fs/t/file/z_file.txt → 200; body == fixture bytes; content-type contains text/plain or octet-stream as implemented

#[tokio::test] async fn file_etag_then_304()
// GET file → capture ETag header; GET again with If-None-Match → 304 empty body

#[cfg(unix)]
#[tokio::test] async fn file_symlink_escape_404()

#[tokio::test] async fn health_still_works_with_fs_routes() // regression glue
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/album && cargo test --test fs_api`
Expected: FAIL (routes / build_app signature)

- [ ] **Step 3: Implement state + routes + handlers**

- `MountTable`: `Clone` + `Default` (empty entries) if needed.
- `AppState { mounts: MountTable }`, `Clone`.
- `build_app(state: AppState) -> Router`: same CORS + swagger + static fallback as today; `api::router().with_state(state)` or merge pattern that preserves nested `/api` fallback.
- `fs_api::routes() -> Router` (no fallback):
  - `list` handlers: extract `host`, optional `Path(path)`; `state.mounts.get(&host)` → None → 404; `AlbumFs::new(root)` + `list_children` inside **one** `spawn_blocking`; map `Err` NotFound → 404, other → 500.
  - `file` handler: same resolve in `spawn_blocking`, also `std::fs::metadata` for mtime+size → ETag string `format!("W/\"{:x}-{:x}\"", mtime_ns, size)`; if request `If-None-Match` equals → 304; else open file, convert to `tokio::fs::File::from_std`, stream with `tokio_util::io::ReaderStream` → `axum::body::Body::from_stream`; set `Content-Type` via `mime_guess::from_path().first_or_octet_stream()`.
- `main.rs`: `let mounts: MountTable = …; axum::serve(listener, album::build_app(album::AppState { mounts }))`.

- [ ] **Step 4: Update prior `build_app()` call sites**

Search `build_app()` in `album/`; pass empty/default `AppState` (health/CORS/swagger/static tests do not need mounts).

- [ ] **Step 4b: Update `run_dev.sh` backend launch**

Change the album cargo invocation so `*` mounts to `<repo>/.www` (create the dir if missing is **not** required — warn if absent):

```bash
# inside run_dev.sh, album launch line becomes roughly:
(cd "$ROOT/album" && cargo run -- --mount "*=$ROOT/.www") &
```

Verify: with dev running, `curl -s http://127.0.0.1:3000/api/fs/*/list` returns JSON listing `.www` children (shell-quote the `*` in curl: `curl 'http://127.0.0.1:3000/api/fs/*/list'`).

- [ ] **Step 5: Run full suite + manual smoke**

Run: `cargo test && cargo fmt --check && cargo clippy --all-targets -- -D warnings`
Optional: run binary with `--mount t=/workspace/.www` and curl list/file.

- [ ] **Step 6: Commit**

```bash
git -C /workspace add album README.md
git -C /workspace commit -m "feat(album): /api/fs list and file endpoints with confinement and ETag"
```

(README only if you document the new routes — one short bullet under health/API is enough.)

---

## Self-review notes (writer)

- Spec coverage: types/confinement/sort → T1; HTTP/ETag/errors/state/spawn_blocking → T2; testing rows 1–7 split across T1/T2; out-of-scope respected (no cache/watcher).
- Types match: `AlbumFs`, `ListedChild`, `MountTable::get`, `build_app(state)` consistent across tasks.
- Review Focus each has a named test in T1 or T2.
- Proportion: no handler bodies transcribed; signatures + JSON shapes + ETag format pinned.