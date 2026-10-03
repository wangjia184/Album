# Image Queue Design — Server-Side Shuffled Playback Cursor

**Date:** 2026-10-03  
**Status:** Draft — awaiting human spec review  
**Scope:** Server-side progressive image index per mount + a cursor-based queue API powering the 流光 home random player. Replaces the previously planned client-side tree walk (that plan is superseded).

## 1. Goal

The home page (`#/`, 流光) is a random photo player: center stage shows one photo, advances every 10s with a fade transition, clicking it opens the album lightbox on that exact photo; a bottom film strip shows the sliding past/current/future thumbnail window and owns all scheduling/prefetch.

To feed that player without hammering the server, the **server** scans each mount once (progressively, in the background), flattens all image paths, shuffles them **exactly once**, and serves a windowed cursor API. The client keeps a single integer cursor.

## 2. Constraints (agreed)

| Topic | Decision |
|---|---|
| Randomness model | Server shuffles **once** per mount scan (no wrap reshuffle, no periodic reshuffle) |
| Start-position variety | Client sends a **random initial offset** on first fetch; server normalizes `offset % length` |
| Cursor advance | `nextOffset = (start + 1) % len` — one step per 10s tick (window slides by one) |
| Progressive serving | Requests never wait for the scan; empty-while-scanning is a normal response |
| Client tree walk | **None** — delete planned `collectPhotos` / `nextOrder` / `photo-index` |
| New endpoint | `GET /api/fs/queue?offset=&limit=` (this spec's only API addition) |
| Image kinds | `IMAGE_EXTS` only (`jpg jpeg png gif webp bmp avif`); videos excluded |
| Symlinks | Not followed during scan (same policy as `fs.rs`) |
| Unreadable dirs during scan | Skipped (best-effort; scan continues) |
| Shuffle algorithm | Full Fisher–Yates over the whole array (equivalent to head/tail mixing) |
| RNG | Existing dep `fastrand` (backend); `Math.random` (client initial offset) |
| Component split | Unchanged from approved design: FilmStrip = data/schedule/prefetch; Stage = display only; Home = wiring |
| Verification | Backend: TDD (`cargo fmt/clippy/test` green). Frontend: `svelte-check` + build + Playwright (no frontend test runner — user-approved) |
| New npm/cargo deps | None (`fastrand` already present) |

## 3. Server architecture

### 3.1 State wiring

- `AppState` gains `queues: Arc<HashMap<PathBuf, Arc<ImageQueue>>>` keyed by **mount root** (`album/src/lib.rs`).
- `main` builds it once: for each entry in `MountTable`, `ImageQueue::start(root)` → two detached `std::thread`s per mount. Duplicate roots share one queue (dedupe by root path).
- `AppState::empty()` (tests: health/CORS/swagger/static) keeps an empty map — queue routes then 404.
- Request resolution: existing `resolve_root(state, headers)` → look up `queues` by that root; miss → 404 `{"error":"not_found"}`.

### 3.2 `ImageQueue` (new module `album/src/queue.rs`)

```rust
pub struct ImageQueue {
    images: Mutex<Vec<String>>,   // rel paths, '/'-separated, e.g. "2016/三巨头/IMG_1.jpg"
    done: AtomicBool,              // scanner+final-shuffle finished
}
```

**Thread A — scanner**

- Iterative walk of the mount root: `std::fs::read_dir` stack; directories entered only when `file_type().is_dir()` (symlinks to dirs are **not** followed); files kept when extension ∈ `IMAGE_EXTS` (case-insensitive).
- Paths stored relative to root, `/`-joined (unix separators).
- Unreadable directory → skip + `tracing::debug`, continue.
- Each found path sent over `std::sync::mpsc` (unbounded channel); sender dropped when the walk ends.

**Thread B — collector**

```
loop {
    match rx.recv_timeout(RECV_TIMEOUT) {          // RECV_TIMEOUT = 2s
        Ok(path) => { push(path); drain try_recv; dirty = true; }
        Err(Timeout) if dirty && len > 1 => { fisher_yates(&mut vec, fastrand); dirty = false; }
        Err(Disconnected) => { if dirty && len > 1 { shuffle; } done.store(true); break; }
        Err(Timeout) => { /* not dirty: keep waiting */ }
    }
}
```

- During an active scan, appends only → existing positions stay stable (client cursors do not drift).
- The first shuffle normally happens at `Disconnected` (scan end) for local scans; a mid-scan quiet period > 2s can also trigger one. **After `done`, the order never changes.**
- Scan order may be served unshuffled during the first moments of a progressive fill — accepted (local scans finish in well under one 10s tick).
- `Mutex` hold times: handler copies its window out under lock; shuffle is O(n) µs–ms at photo-library scale — acceptable.

### 3.3 Test seams

- `ImageQueue::start_with_timeout(root, Duration)` — injectable recv timeout for deterministic tests.
- `fn wait_until_ready(&self)` — spin-sleep on `done` (tests only).
- `#[cfg(test)] fn from_paths(Vec<String>)` — pre-seeded queue for handler tests without threads.

## 4. API contract

`GET /api/fs/queue?offset=&limit=` — Host → mount resolution identical to existing `/api/fs/*` handlers.

**Query**

| Param | Type | Default | Rules |
|---|---|---|---|
| `offset` | `u64` | `0` | Client's first request sends `floor(Math.random() * 1_000_000)`; every response returns a normalized value. Negative/non-numeric → 400 (axum `Query` rejection). |
| `limit` | `usize` | `12` | Clamped to `1..=64`. |

**Response** `200 application/json`

```json
{ "images": ["2016/三巨头/IMG_1.jpg", "many/img_18.png"], "nextOffset": 3, "done": true }
```

- `len == 0` (scan not started yielding yet, or genuinely no images): `images: []`, `nextOffset: 0`, `done` as known — **never `% 0`**.
- Otherwise `start = offset % len`, then:
  - `images[i] = arr[(start + i) % len]` for `i in 0..limit` — **wraps inside the window** so the strip never gets a short row at the array tail;
  - `nextOffset = (start + 1) % len`.
- `done` — collector thread finished (scan complete + final shuffle applied). Lets the client distinguish "still indexing" from "empty library".
- `images` entries are mount-root-relative paths, same string shape the thumbs API already returns — the existing client `fileUrl` consumes them directly.

## 5. Frontend

### 5.1 `lib/api.ts`

```ts
export interface QueueResponse { images: string[]; nextOffset: number; done: boolean }
export async function fetchQueue(offset: number, limit: number): Promise<QueueResponse>
// GET /api/fs/queue?offset=&limit=
```

### 5.2 `lib/pipeline.ts` (shrinks)

Keep: `windowSlots(history, upcoming, capacity)`, `thumbCapacity(width)`, `SLOT_W = 112`, `GAP = 8`.
Add: `splitMediaPath(path: string): { dir: string; file: string }` — split on last `/`; no `/` → `dir = ''`.
Delete: `PhotoRef`, `ListFn`, `collectPhotos`, `shuffled`, `nextOrder` (server owns the pool now).

### 5.3 `FilmStrip.svelte` (still the sole scheduler)

State: `history: string[]` (shown, last = current), `upcoming: string[]`, `nextOffset: number | null`, `status: 'loading' | 'ready' | 'empty' | 'error'`, `error: string`, `width` via `bind:clientWidth`; `capacity = $derived(thumbCapacity(width))`, `slots = windowSlots(history, upcoming, capacity)`, current index = `mid = floor((capacity-1)/2)`.

Flow:

1. **Mount:** gated on `width > 0` (`bind:clientWidth` settles after first layout — never fetch with capacity from width 0). Then `offset0 = Math.floor(Math.random() * 1_000_000)`; `fetchQueue(offset0, capacity)`.
   - Network failure → `status = 'error'` (`照片列表加载失败：…`, no auto-retry; refresh recovers).
   - `images: [] && !done` → retry every **1s** with the **same `offset0`** (keeps per-visit start randomness), bounded at **60** tries then `error`.
   - `images: [] && done` → `status = 'empty'` (「暂无照片」), no timer.
   - Non-empty → `history = [images[0]]`, `upcoming = images.slice(1)`, `nextOffset = resp.nextOffset`, `status = 'ready'`, emit `oncurrent(images[0])`, prefetch, start `setInterval(advance, 10_000)`.
2. **advance (each tick):** `fetchQueue(nextOffset, capacity)` → on failure keep current photo, log/skip this tick (do not tear down); on success `history.push(images[0])`, `upcoming = images.slice(1)`, `nextOffset = resp.nextOffset`, emit `oncurrent(images[0])`, prefetch `upcoming.slice(0, PREFETCH_AHEAD)` via `new Image()` with `fileUrl('', path)`.
3. **Resize:** `bind:clientWidth` updates `capacity`; window re-renders immediately (may pad `null` on the right until the next tick refetches with the new limit).
4. **Unmount:** clear interval + retry timer (`cancelled` flag guards in-flight fetches).

Render/status copy, thumb geometry (`h-16 w-28`, `gap-2`, bar `h-20`, current slot `ring-2 ring-primary`), non-interactive thumbs, testid `filmstrip` — unchanged from the superseded plan.

### 5.4 `Stage.svelte` — unchanged in behavior

Only the prop type changes: `current: string | null` (relative path); `fileUrl('', current)` for the src. Preload-aware sequential fade (`FADE_MS = 400`, seq guard), click → `onopen(current)`, failed → 「图片加载失败」, null → empty box, testid `stage-img`, TODO comment for the follow-up same-aspect fancy transition.

### 5.5 `Home.svelte` — wiring

```ts
let current = $state<string | null>(null)
function open(path: string): void {
  const { dir, file } = splitMediaPath(path)
  push(mediaHref(dir, file))   // → #/album/<dir>?m=<file> opens the lightbox there
}
```

Layout: `flex h-[calc(100dvh-6rem)] flex-col gap-3` (adjust only if the browser layout check fails).

## 6. Edge cases

| Case | Behavior |
|---|---|
| Request before any image scanned | `images: []`, `done: false` → client 1s backoff |
| Mount with zero images | `images: []`, `done: true` → 「暂无照片」 |
| `% 0` | Impossible — guarded by `len == 0` branch |
| Window crosses array tail | In-response wrap (`i % len`) keeps the row full |
| Client offset ≥ len (stale after growth) | Normalized by `% len` on every request |
| Every page visit starts at the same photo | Client random initial offset (§4) |
| Scan yields unshuffled prefix briefly | Accepted (§3.2) |
| Shuffle moves items under a live cursor | Accepted; single-user cosmetic risk only |
| Videos / non-images in tree | Excluded at scan (`IMAGE_EXTS`) |
| Symlink loops / escapes | Not followed (`file_type().is_dir()` only) |
| Wide viewport, `capacity > 64` | Server clamp; right side may pad — acceptable |
| Tick fetch fails mid-session | Keep showing current photo; retry naturally next tick |
| Multiple mounts | One queue pair per mount root; Host header selects |

## 7. Testing strategy

**Backend (TDD, red → green; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full `cargo test`):**

- `queue` unit tests (temp fixture dirs):
  - scanner collects exactly the image set (skips non-images, skips symlinked dirs, nested depths);
  - collector: after `wait_until_ready`, `images` is a permutation of the scanned set (set equality + same length) and `done == true`;
  - timeout+dirty shuffle fires (inject short timeout; feed a batch, sleep past timeout while sender still open, assert order changed but set equal);
  - not-dirty timeout does not panic/no-op;
  - `from_paths` seeds serve deterministic handler-facing state.
- `fs_api` integration tests (pre-seeded queue in `AppState`):
  - basic window + `nextOffset == (offset % len + 1) % len`;
  - `offset` beyond length wraps;
  - window wraps at tail (last element followed by `arr[0]`);
  - `len == 0` → empty + `nextOffset 0` + `done`;
  - `limit` clamp (0→1, 999→64) and default 12;
  - negative `offset` → 400;
  - unknown host → 404; `AppState::empty()` → 404.

**Frontend (existing convention):**

- `cd ui && npm run check && npm run build` after every task.
- Playwright against `http://127.0.0.1:3000` (no-cache headers + `?v=` cache-bust):
  - main flow: stage img loads, slot count = `thumbCapacity(stripWidth)`, current ringed at `mid`, strip bottom ≈ viewport bottom − 16px;
  - 10s tick advances stage src, ring stays centered;
  - click → hash contains `#/album/…?m=…`, lightbox `div[aria-modal="true"]` opens;
  - resize to 800×900 → slot count matches new strip width;
  - empty: `route.fulfill` queue response `{images:[],done:true}` → 「暂无照片」, no stage img, no `pageerror`;
  - error: `route.abort` on `/api/fs/queue` → error alert, no `pageerror`;
  - slow file: delay `/api/fs/file/**` 1500ms → at tick+300…900ms stage opacity still `1` (fade waits for preload);
  - restore smoke: real data loads again.

## 8. Out of scope

- Same-aspect fancy stage transition (explicit follow-up; Stage carries a TODO comment only).
- Re-shuffle on wrap / periodic reshuffle / persistent client cursor (localStorage).
- Path-scoped queues (`/api/fs/queue/{*path}`) — whole-mount only.
- Strip thumbnail click-to-jump, manual next/prev controls.
- Server-side image resizing/thumbnail renditions.
- FS watcher (libraries changed after server start are invisible until restart).

## 9. Superseded artifacts

- Plan `docs/superpowers/plans/2026-10-03-liuguang-home-random-player.md` (client-side `collectPhotos` walk) is **void**; a new plan will be written from this spec via writing-plans after spec approval.