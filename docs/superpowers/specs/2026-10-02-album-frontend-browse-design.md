# Album Frontend Browse Design

**Date:** 2026-10-02  
**Status:** Draft — awaiting human spec review  
**Scope:** SPA browse UI (masonry + lightbox) and the accompanying backend route change (Host-header mounts, no `{host}` path segment).

## 1. Goal

Let a user browse album folders in the Svelte SPA:

1. Open the mount root at `#/album`.
2. Drill into folders; path lives in the hash (`#/album/...`), refresh-safe, no query string.
3. Stream UI: full list fetched once; progressive DOM mount on scroll.
4. Masonry (waterfall) media grid — fixed column widths, heights follow aspect ratio.
5. Click media → lightbox with original file (`/api/fs/file/...`); video plays inline in lightbox.
6. Works on phone and desktop.

## 2. Constraints (agreed)

| Topic | Decision |
|---|---|
| Mount resolution | Backend reads HTTP `Host` (strip port) → `MountTable::get` → miss → `*`; **no `{host}` in FS paths** |
| Old routes | `GET /api/fs/{host}/...` **removed** |
| List API | Unchanged JSON; full children, no server pagination/filter |
| Client streaming | Progressive mount from in-memory array (IntersectionObserver), not server pages |
| Layout | **Masonry/waterfall** (not justified rows): column width fixed, height ∝ aspect |
| Layout lib | Prefer `@masonry-grid/svelte` or equivalent; thin local wrapper OK |
| Lightbox | DaisyUI `modal` (no extra dep unless needed later) |
| Routing | Hash: `'/album'` + `'/album/*'`; path in `params.wild`; no top-level `/*` |
| `kind=dir` | Folder cards above media grid |
| `kind=image\|video` | Masonry tiles |
| `kind=other` | **Hidden in v1** (API still returns them; frontend filters) |
| Dimensions | From `<img>` `onload` naturalWidth/Height; no backend dimension field |
| Thumbnails | **None in v1** — tiles use original `file` URL + `loading="lazy"` (known bandwidth tradeoff) |
| Tech | Svelte 5, DaisyUI 5, Tailwind 4, svelte-spa-router (existing) |

## 3. Backend change (API-A)

| Method | Path | Notes |
|---|---|---|
| GET | `/api/fs/list` | Root of mount for this `Host` |
| GET | `/api/fs/list/{*path}` | Nested |
| GET | `/api/fs/file/{*path}` | Raw file + ETag (unchanged behavior) |

- Host key: `Host` header, strip `:port`; optional trailing dot stripped? **No** — exact match after lowercasing via existing `MountTable::get` (already case-insensitive + `*` fallback).
- No usable Host / no mount / no `*` → `404` `{"error":"not_found"}`.
- Nested `/api` JSON 404 fallback, CORS, health, swagger, static SPA **unchanged**.
- Integration tests: update FS tests for Host-header style; keep confinement/ETag/sort assertions.

## 4. Routing

```ts
// ui/src/routes.ts
export const routes = {
  '/': Home,          // existing scaffold home (optional later redirect to /album)
  '/album': Browse,   // root list
  '/album/*': Browse, // params.wild e.g. "2016/三巨头" (decodeURIComponent as needed)
}
```

- Enter folder: navigate `#/album/` + relative path.
- Breadcrumb links rebuild `#/album/...` prefixes.
- No `?query` for path.

## 5. Components

```text
pages/Browse.svelte
├── Breadcrumb.svelte
├── FolderGrid.svelte      // dirs only
├── MediaMasonry.svelte    // image|video, progressive mount
│   └── MediaTile.svelte
└── Lightbox.svelte        // DaisyUI modal
lib/api.ts                 // listDir(rel), fileUrl(rel, name)
```

### Browse state

- `path` derived from route params; **changing path resets** items, `visibleCount`, lightbox index.
- Fetch `listDir(path)` once per path; split `dirs` / `media` (filter `other`).
- `visibleCount` starts ~30; sentinel near viewport bottom → `visibleCount += 30` (slice only).
- Errors → DaisyUI alert (`not_found` / `internal`).

### MediaTile

- Image: `<img loading="lazy" decoding="async" src={fileUrl}>`; skeleton placeholder until load; `onload` records natural size for masonry.
- Video: tile shows play icon on colored block; open lightbox with `<video controls>`.

### Lightbox

- DaisyUI modal; image `max-h-[90vh] max-w-full`; video `controls`.
- Close: Esc, backdrop, X.
- Prev/Next: buttons + ArrowLeft/ArrowRight when open.
- Lock body scroll while open.

### FolderGrid

- DaisyUI card/button per dir; click → navigate; not part of masonry.

## 6. Responsive

| Viewport | Columns |
|---|---|
| `< 640px` | 2 |
| `640–1024px` | 3 |
| `≥ 1024px` | 4–5 (target tile width ~200–280px desktop, ~45vw mobile) |

Implementation: one mechanism (breakpoint classes or container width ÷ target) — pin in plan; must re-layout on resize.

## 7. Empty / loading / error

| Case | UI |
|---|---|
| Empty dir | info alert「空文件夹」 |
| Loading | skeleton tiles + optional top spinner |
| 404 | error alert + back via breadcrumb |
| 500 | error alert `internal` |
| No mount at all | 404 alert; hint check `--mount` |

## 8. Testing / verification

**Backend:** `cargo test` — Host-based list/file; no `{host}` in path; confinement/ETag regressions; health/CORS/swagger/static.

**Frontend:** `npm run check` + `npm run build` (no CDN); manual E2E (or Playwright if already in workflow):

1. `#/album` lists root (dirs first as API returns; folders in FolderGrid).
2. Drill `#/album/2016` …; refresh restores path.
3. Scroll loads more tiles (visibleCount).
4. Masonry: 2 cols narrow, more cols wide (resize).
5. Click image → lightbox original; arrows; Esc.
6. Click video → player in modal.
7. Unknown path → error alert, not white screen.

## 9. Out of scope (v1)

- Thumbnails, EXIF, search, multi-select, upload, justified rows, virtualizer, auth, service worker.

## 10. Acceptance criteria

1. Phone and desktop: folder drill + masonry + lightbox usable.
2. Hash URL restores same folder after refresh; no query-string path.
3. Backend FS routes work without `{host}` using `Host` + `*`.
4. Progressive mount avoids hanging on huge directories (DOM grows in chunks).
5. `cargo test` and `npm run build` / `npm run check` green; no CDN assets.