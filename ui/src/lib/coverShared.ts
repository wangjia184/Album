/**
 * Coverflow shared pure helpers — used by BOTH the main thread (motion) and
 * the cover worker (path windows). No DOM, no network: safe in either realm.
 */

/** Spare slots each side beyond the visible ±3 range. */
export const SPARE = 4
/** Fetch window length (center=true → MID at the center slot). */
export const LIMIT = 2 * SPARE + 1
/** Center offset inside a center=true window. */
export const MID = SPARE
/** Frame padding: white board is `p-3` and canvas is `inset-3` → −24px total. */
export const FRAME_PAD = 24

/** Expand [lo,hi] to cover both endpoints ± SPARE (before a jump). */
export function expandKeyWindow(
  lo: number,
  hi: number,
  from: number,
  to: number,
): { lo: number; hi: number } {
  const nlo = Math.min(from, to) - SPARE
  const nhi = Math.max(from, to) + SPARE
  return { lo: Math.min(lo, nlo), hi: Math.max(hi, nhi) }
}

/** Rest window for center display key `base`. */
export function settleKeyWindow(base: number): {
  base: number
  lo: number
  hi: number
} {
  return { base, lo: base - SPARE, hi: base + SPARE }
}

/** Display keys for a queue window centered on `base` (length LIMIT). */
export function keysFor(base: number): number[] {
  const out: number[] = []
  for (let i = 0; i < LIMIT; i++) out.push(base - MID + i)
  return out
}

/** Map display keys to paths from a center=true window of length ≤ LIMIT. */
export function pathEntries(
  base: number,
  paths: readonly string[],
): { key: number; path: string | null }[] {
  return keysFor(base).map((key, j) => ({
    key,
    path: j < paths.length ? paths[j] : null,
  }))
}

/** Wrap a signed queue index into [0, total). total≤0 → clamp at 0. */
export function wrapIndex(idx: number, total: number): number {
  if (total <= 0) return Math.max(0, idx)
  return ((idx % total) + total) % total
}

/** CSS box side for the cover canvas inside a slot of side `S`. */
export function canvasCssPx(S: number): number {
  return Math.max(1, Math.round(S - FRAME_PAD))
}

/** Backing-store side (CSS × devicePixelRatio). */
export function canvasBufferPx(S: number, dpr = 1): number {
  return Math.max(1, Math.round(canvasCssPx(S) * Math.max(1, dpr)))
}
