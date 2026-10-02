# Album FS Read API Design

**Date:** 2026-10-02  
**Status:** Draft — awaiting human spec review  
**Scope:** Read-only filesystem access for the album SPA — `AlbumFs` + HTTP API under `/api/fs`. No application-level cache, no FS watcher, no registry.

## 1. Goal

Expose each mounted hostname’s local folder tree to the SPA over HTTP:

1. List direct children of a directory (name + kind only; no server filter, no pagination).
2. Stream a single file so the browser can load it directly (`<img>` / `<video>` src).

Security: all paths are confined to the mount root (`..` and symlink escapes → 404).

## 2. Constraints (agreed)

| Topic | Decision |
|---|---|
| Consumer | HTTP API → SPA; app is read-only on album trees |
| App-level cache | **None** — rely on OS page/dentry cache; each list/open hits the FS |
| Registry (`DashMap`) | **None** — stateless per request after `MountTable` lookup |
| FS watcher / background threads | **None** |
| Server filtering | **None** — return all direct children with `kind` |
| Pagination | **Frontend only** |
| Sort order | **Directories first** (each group by `name`), then **files** (by `name`) |
| File response body | **Raw bytes**, not JSON |
| Symlink policy | Resolved real path must stay under root; else 404 |
| ETag | On **file** responses only: weak `W/"{mtime_ns:x}-{size:x}"`; honor `If-None-Match` → 304 |
| Host lookup | `MountTable` (existing): hostname → root `PathBuf` |
| Path keying | Relative path from mount root; empty/`/` = root |

## 3. Types (`album/src/fs.rs` or equivalent)

```rust
/// Immutable, canonical absolute root for one mount.
pub struct AlbumFs {
    root: PathBuf,
}

pub enum ChildKind { Dir, Image, Video, Other }

pub struct ListedChild {
    pub name: String,   // basename
    pub kind: ChildKind,
}

impl AlbumFs {
    /// Canonicalize `root` at construction; fail if not a directory.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self>;

    /// Direct children only. `rel` is a relative path; empty = root.
    /// Sort: dirs by name, then files by name.
    pub fn list_children(&self, rel: &str) -> io::Result<Vec<ListedChild>>;

    /// Open file for streaming after path checks. Caller maps to HTTP response.
    pub fn open_file(&self, rel: &str) -> io::Result<std::fs::File>;
}
```

- `kind`: directory from `file_type()`; else extension (case-insensitive):  
  - image: `jpg jpeg png gif webp bmp avif`  
  - video: `mp4 mov mkv webm avi m4v`  
  - else: `Other`  
- No width/height in v1 (no image-header reads on list).  
- `AlbumFs` is constructed per request (cheap) or once per handler setup; **no global map**.

### Path confinement (shared by list and open)

1. Reject empty segments, `.` / `..` components, absolute `rel`, and interior NUL.  
2. Join `root` + `rel`.  
3. `canonicalize` the target (and for list, the parent directory).  
4. Require `canonical.starts_with(canonical_root)`; else treat as not found (404).  
5. Non-existent path → 404 (not 500).

`list_children` on a non-directory → 404. `open_file` on a directory → 404.

## 4. HTTP API

Nested under existing `/api` (JSON 404 fallback for unknown `/api/*` unchanged).

| Method | Path | Success |
|---|---|---|
| GET | `/api/fs/{host}/list` | `200` JSON list of root children |
| GET | `/api/fs/{host}/list/{*path}` | `200` JSON list of `path` children |
| GET | `/api/fs/{host}/file/{*path}` | `200` raw file bytes + `Content-Type` |

- `{host}`: must resolve via `MountTable`; unknown host → 404.  
- `{*path}`: URL-decoded relative path; missing wildcard = root for `list`.

### List response (`application/json`)

```json
{
  "path": "2016",
  "children": [
    { "name": "三巨头", "kind": "dir" },
    { "name": "烤全羊", "kind": "dir" },
    { "name": "1.jpg", "kind": "image" },
    { "name": "readme.txt", "kind": "other" }
  ]
}
```

`path` echoes the normalized relative path (`""` for root). Order: all `dir` by name, then all non-dir by name (byte/codepoint order is fine; no locale collation).

### File response

- Body: file contents (streamed; not read entirely into memory).  
- `Content-Type`: `mime_guess` from extension; default `application/octet-stream`.  
- Header `ETag: W/"{mtime_ns:x}-{size:x}"` (mtime and size from `stat` after confinement check).  
- `If-None-Match` matching current ETag → `304` (no body).  
- Prefer implementing with `tower_http::services::ServeFile` **after** confinement, or hand-rolled stream + ETag if ServeFile’s path handling fights the checks — implementer choice; behavior above is binding.  
- Range/`Content-Length`: as provided by the chosen implementation (ServeFile gives Range for free).

### Errors (JSON, consistent with existing API)

| Condition | Status | Body |
|---|---|---|
| Unknown host; escape (`..`, symlink, absolute); missing path; not a dir / not a file | 404 | `{"error":"not_found"}` |
| Permission / unexpected IO | 500 | `{"error":"internal"}` |

No 403 (avoid probing).

## 5. Wiring

```text
MountTable (AppState) 
  → host param 
  → MountTable::get(host) 
  → AlbumFs::new(root)          // sync, cheap
  → spawn_blocking { list_children | open_file }
  → Json | file response
```

- **Runtime:** `#[tokio::main]` (default `multi_thread`; may write `flavor = "multi_thread"` explicitly).
- **Blocking FS:** `AlbumFs` methods stay **synchronous** (`std::fs`). Handlers are `async` and must run list/open (and any `canonicalize`/`stat` that can hit disk) inside `tokio::task::spawn_blocking` so they do not stall worker threads. Return values are plain data / `File` to send on the async side.
- `MountTable` collected from CLI `Args.mount` in `main`, inserted into axum state.  
- Routes registered inside `api::router()` (or nested module) so static SPA fallback and swagger paths are unaffected.  
- OpenAPI: add `utoipa` path docs for list (and file as binary response) when convenient; not a blocker for behavior tests.

## 6. Testing (TDD)

| # | Assertion |
|---|---|
| 1 | `GET /api/fs/{known}/list` on fixture tree → 200; dirs before files; each group name-sorted; `kind` correct |
| 2 | Nested `list/{*path}` works; `path` echo normalized |
| 3 | `list` with `../` or symlink pointing outside root → 404 |
| 4 | `file` → 200, bytes match on-disk file; `Content-Type` sensible |
| 5 | `file` with `If-None-Match: <etag>` → 304 |
| 6 | Unknown `{host}` → 404 JSON |
| 7 | Regression: existing health / CORS / swagger / static tests still pass |

Fixture: small temp dir tree (or a committed `album/tests/fixtures/fs/`); do not depend on `.www` or NAS.

## 7. Out of scope (v1)

- Thumbnail generation, image dimension extraction  
- Server pagination / sorting options / query filters  
- Write/upload APIs  
- Application cache, FS watcher, `DashMap` registry  
- AuthN/Z beyond current CORS scaffold  
- Video streaming optimizations beyond basic file send / Range via ServeFile  

## 8. Acceptance criteria

1. Browser can set `<img src="http://…/api/fs/{host}/file/…">` and see the image (raw body, correct content type).  
2. List JSON matches sort rules and `kind` values.  
3. Escaping attempts and unknown hosts return 404 JSON.  
4. Conditional GET returns 304 when ETag matches.  
5. `cargo test` green including prior suites.  