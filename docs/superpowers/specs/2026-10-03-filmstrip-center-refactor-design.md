# FilmStrip Center-Refactor Design — Idle-Driven Player with Manual Selection

**Date:** 2026-10-03  
**Status:** Draft — awaiting human spec review  
**Scope:** Rework of the 流光 home film strip: idle-driven rotation with global activity reset, manual thumbnail selection that does not move the bar, both-sides-filled initial window, and the queue API change (`center` windowing + per-image `index`) that enables them. Stage/frame/transitions untouched.

## 1. Goal

1. Auto-rotation only fires after **5s of user inactivity** (global `mousemove` and any `click` reset the countdown; moving the mouse continuously pauses rotation indefinitely).
2. Clicking a strip thumbnail switches the center photo to it **without moving the bar** (ring jumps to the clicked slot); the bar re-anchors (current horizontally centered) only on the subsequent **automatic** switch.
3. At initialization the strip is **full on both sides** of the current photo (fixes today's empty left side).
4. Each queue item carries its **absolute `index`** in the shuffled array so the client can drive cursors after a manual jump.

## 2. Constraints (agreed)

| Topic | Decision |
|---|---|
| API direction | **`center` query param** (approach 1), not `dir`, not client-side `length` math |
| Response shape | `{"images": [{"index": u64, "path": String}], "done": bool}` — **`nextOffset` removed** (breaking; update all tests + client) |
| `center=false` (default) | Existing forward semantics, now with `index` per item |
| `center=true` | `offset` = the current photo; server windows around it; client never sends negatives, never mods |
| Idle duration | `IDLE_MS = 5000` (same cadence as today's tick) |
| Activity scope | **A:** `document`-level `mousemove` (passive) + `click` |
| Manual click | Zero network; bar window unchanged; only `current`/`ringIndex` change |
| Auto switch | One `center=true` fetch from `current.index + 1`; window re-anchored so current sits at `mid` |
| `mid` formula | `mid = floor((limit - 1) / 2)` — **identical** server-side and client-side (even capacities sit half a slot left of geometric center; accepted) |
| Thumb geometry / capacity | Unchanged: `SLOT_W=112`, `GAP=8`, `thumbCapacity(width)` via `bind:clientWidth` |
| Stage / frame / badge / transitions | Untouched; `oncurrent(path: string)` signature unchanged |
| New deps | None |
| Verification | Backend TDD (`cargo fmt/clippy/test`); frontend `svelte-check` + build + Playwright (user-approved) |

## 3. API contract

`GET /api/fs/queue?offset=&limit=&center=true|false`

**Query**

| Param | Type | Default | Rules |
|---|---|---|---|
| `offset` | `u64` | `0` | Negative/non-numeric → 400. With `center=true` it names the current photo (server normalizes `% len`). |
| `limit` | `usize` | `12` | Clamped `1..=64`. |
| `center` | `bool` | `false` | serde bool (`true`/`false`); invalid → 400. |

**Response** `200 application/json`

```json
{ "images": [ { "index": 41, "path": "2016/三巨头/IMG_1.jpg" } ], "done": true }
```

- `len == 0` → `images: []`, `done` as known (never `% 0`).
- `center=false`: `start = offset % len`; `images[i] = { index: (start+i) % len, path: arr[(start+i) % len] }`.
- `center=true`: `mid = (limit - 1) / 2`; `start = floor_mod(offset % len − mid, len)`; same per-item formulas — item at slot `mid` has `index == offset % len`.
- `QueueWindow` becomes `{ items: Vec<QueueItem { index: u64, path: String }>, done: bool }`; `next_offset` field deleted.
- Symlink/image policy, `done` semantics, Host→mount resolution: unchanged (spec `2026-10-03-image-queue-design.md` still governs scanning/shuffle).

## 4. FilmStrip state model (rewrite)

**State**

```ts
window: QueueItem[]        // length === capacity once ready; never has holes
current: QueueItem | null
ringIndex: number          // slot of current in window (mid normally; clicked slot after manual select)
status: 'loading' | 'ready' | 'empty' | 'error'
error, width ($state, bind:clientWidth), capacity/mid ($derived), inFlight
idleTimer: ReturnType<typeof setTimeout> | undefined
```

`windowSlots` is deleted from `lib/pipeline.ts` (sole consumer gone). `thumbCapacity`, `SLOT_W`, `GAP`, `splitMediaPath` stay.

**Flows**

1. **Bootstrap** — gated on `width > 0` (unchanged). `offset0 = floor(random * 1_000_000)` captured once; `fetchQueue(offset0, capacity, center=true)`:
   - non-empty → `window = images`, `current = window[mid]`, `ringIndex = mid`, `status='ready'`, `oncurrent(current.path)`, prefetch, `resetIdle()`.
   - empty && `!done` → retry every 1s, same `offset0`, max 60 → then `error`.
   - empty && `done` → `empty` (「暂无照片」), no timer.
   - throw → `error` (「照片列表加载失败：…」), no auto-retry.
2. **Auto advance** — fires only from the idle timer:
   - Guard: not ready / `inFlight`. `fetchQueue(current.index + 1, capacity, center=true)` → replace `window`, `current = window[mid]`, `ringIndex = mid`, emit, prefetch, `resetIdle()`.
   - On throw: keep state, `console.warn`, `resetIdle()` (retry next idle period).
3. **Manual select(i)** — click slot `i`:
   - Guard: `status==='ready'`, `window[i]` exists, not `inFlight`. **No fetch.**
   - `current = window[i]`, `ringIndex = i`, emit, prefetch right side (`window.slice(i+1)` up to 16), `resetIdle()`.
   - Clicking the already-current slot: reset timer only.
4. **Idle reset** — `resetIdle()` = `clearTimeout` + `setTimeout(autoAdvance, 5000)`. Listeners registered in `onMount`, removed in cleanup:
   - `document.addEventListener('mousemove', resetIdle, { passive: true })`
   - `document.addEventListener('click', resetIdle, { passive: true })` (strip select also resets; double call harmless)
   - Also called after bootstrap-ready, after every auto advance (success or failure), and on manual select.
   - Continuous mouse activity ⇒ rotation never fires; 5s of stillness ⇒ one advance.
5. **Resize** — when `status==='ready'` and `capacity` changes: refetch `fetchQueue(current.index, capacity, center=true)` **debounced 300ms** (window-drag resizes fire often) so the new width is filled exactly; `ringIndex = mid`.
6. **Prefetch** — `prefetch(items: QueueItem[])`: warm `fileUrl(item.path)` for the first 16 items of the given list; called with the current window's post-current slice after bootstrap, after auto advance, after manual select.

**Render**

- Root keeps `bind:clientWidth`, `data-testid="filmstrip"`, bar classes (unchanged geometry).
- Each slot is a `<button type="button">` (a11y; no nested-interactive issues) with `<img src={fileUrl(item.path)}>`, `cursor-pointer hover:ring-1 hover:ring-primary/60`; selected slot: `ring-2 ring-primary` keyed by `ringIndex === i` (not hardcoded mid).
- `loading` / `empty` / `error` states: copy unchanged.
- Strip remains one of the two ownership poles: Stage only receives `oncurrent(path)`.

## 5. `api.ts`

```ts
export interface QueueItem { index: number; path: string }
export interface QueueResponse { images: QueueItem[]; done: boolean }
export async function fetchQueue(offset: number, limit: number, center = true): Promise<QueueResponse>
// GET /api/fs/queue?offset=&limit=&center={center}
// validate: images items must be objects with numeric index + string path (drop invalid)
```

Client call sites always use `center=true` (default).

## 6. Edge cases

| Case | Behavior |
|---|---|
| Init, both sides | Single `center=true` call; server wrap fills left of `offset − mid` |
| `offset < mid` (client never sends it, but server handles) | `floor_mod` → wrap; no underflow |
| Manual click, then idle | Next advance windows around `clicked.index + 1` (queue continues from the click, not from the old cursor) |
| Click during `inFlight` | Ignored (fetches are local-fast; avoids races) |
| Continuous mousemove | Timer perpetually reset — no auto rotation until 5s still |
| Route away (lightbox) | Cleanup removes listeners, clears timer |
| Pool smaller than capacity | Server wrap repeats items (unchanged; pre-existing deferred minor) |
| Even capacity | `mid = floor((n-1)/2)` both sides; current half a slot left of geometric center (accepted) |
| Resize while idle/after click | Refetch centered on `current.index`; `ringIndex = mid` |
| Tick fetch failure | Keep window/current/ring; idle timer re-armed (retry in 5s) |
| Empty / error / retry caps | Identical to current behavior |

## 7. Testing strategy

**Backend (TDD red → green; fmt/clippy/test green):**

- Update all existing `queue_api` / `queue` tests to the `{index, path}` shape and drop `nextOffset` assertions.
- New: `center=true` places `offset % len` at slot `mid` (odd and even limits); `offset < mid` wraps without panic; per-item `index` sequence equals `(start..start+limit) % len`; `center=false` preserves forward semantics; empty queue with `center=true`; limit clamp; `offset=-1` → 400; `center=maybe` → 400.

**Frontend (svelte-check + build + Playwright):**

- Init: all slots filled (no empty placeholders), ring at mid, current src matches center slot.
- Manual click on slot `mid+3`: stage switches to that path, **window slot srcs byte-identical (bar unmoved)**, ring moves to `mid+3`.
- Idle: mouse moves every 1.5s for >6s → stage src unchanged; stop moving → src changes within 5±1.5s.
- Two consecutive auto advances: ring back at `mid`, leftmost slot src changed (window re-anchored).
- Resize 1280→800: slot count = `thumbCapacity(newWidth)`, all filled.
- Regression: prefetch resource entries exist for upcoming items; stage photo click still opens lightbox; parent badge still correct; `cargo test` full suite green.

## 8. Out of scope

- Animated sliding of the strip (window swap stays instant, as today).
- Thumbnail drag, scrubbing, volume of any kind; persisting cursor across reloads.
- `dir` / bidirectional fetch parameter (superseded by `center=true`).
- Stage, frame, badge, fancy/fade transitions — no changes.
- Fetch timeouts (pre-existing deferred minor).

## 9. Supersedes

- No prior spec covers the film strip interaction; this spec extends `2026-10-03-image-queue-design.md` §4/§5 (FilmStrip behavior + API response) — where they conflict on `nextOffset`/response shape, this spec wins.