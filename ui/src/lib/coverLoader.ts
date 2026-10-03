import { fileUrl } from './api'

/**
 * Cover image loader — the ONLY component that turns paths into bytes.
 *
 * Decoupled from motion on purpose: the coverflow animates purely from
 * indices and never awaits this. Missing images simply leave the white
 * placeholder board visible until bytes settle.
 *
 * Future seam: swap `warm` for a Web Worker (queue API + fetch/decode off
 * the main thread) without touching the view — same interface.
 */
export interface CoverLoader {
  /** Prefetch image bytes for these paths. Idempotent; holds refs until settle. */
  warm(paths: readonly string[]): void
  /** Drop settled/in-flight state for paths outside the live window. */
  prune(keep: ReadonlySet<string>): void
  /** Abort in-flight loads and forget everything (component teardown). */
  dispose(): void
}

export function createCoverLoader(): CoverLoader {
  const inFlight = new Set<HTMLImageElement>()
  const settled = new Set<string>()

  return {
    warm(paths) {
      for (const path of paths) {
        if (settled.has(path)) continue
        const im = new Image()
        inFlight.add(im)
        const settle = (): void => {
          inFlight.delete(im)
          settled.add(path)
        }
        im.onload = settle
        im.onerror = settle
        im.src = fileUrl('', path)
      }
    },
    prune(keep) {
      for (const path of [...settled]) {
        if (!keep.has(path)) settled.delete(path)
      }
    },
    dispose() {
      for (const im of inFlight) {
        im.onload = null
        im.onerror = null
        im.src = ''
      }
      inFlight.clear()
      settled.clear()
    },
  }
}