/**
 * Cover worker — the ONLY place the coverflow touches the network.
 *
 * Owns: queue API (index → path), image fetch/decode, canvas draw, parent-dir
 * strings. Main thread stays on pure index motion + DOM badges.
 */
import { fetchQueue, fileUrl, type QueueItem } from './api'
import type { MainToWorker, WorkerToMain } from './coverProtocol'

const ctx = self as unknown as {
  postMessage(msg: unknown, transfer?: Transferable[]): void
  onmessage: ((ev: MessageEvent<MainToWorker>) => void) | null
}

let limit = 9
let mid = 4
let displayBase = 0
let centerQueueIdx = 0
let total = 0
let items: QueueItem[] = []
let gen = 0
let disposed = false

/** Transferred canvases keyed by display index. */
const canvases = new Map<number, OffscreenCanvas>()
/** In-flight image loads by path (dedupe). */
const loading = new Set<string>()
/** Path already drawn per key (skip reload). */
const drawnPath = new Map<number, string>()

function post(msg: WorkerToMain, transfer?: Transferable[]): void {
  if (disposed) return
  ctx.postMessage(msg, transfer ?? [])
}

function wrapQueue(idx: number): number {
  if (total <= 0) return Math.max(0, idx)
  return ((idx % total) + total) % total
}

function parentDir(path: string): string {
  const cut = path.lastIndexOf('/')
  return cut < 0 ? '' : path.slice(0, cut)
}

function keysFor(base: number): number[] {
  const lo = base - mid
  const hi = base + (limit - 1 - mid)
  const out: number[] = []
  for (let k = lo; k <= hi; k++) out.push(k)
  return out
}

function pathEntries(base: number): { key: number; path: string | null }[] {
  return keysFor(base).map((key, j) => ({
    key,
    path: j < items.length ? items[j].path : null,
  }))
}

function drawCover(canvas: OffscreenCanvas, bitmap: ImageBitmap): void {
  const w = canvas.width
  const h = canvas.height
  if (w <= 0 || h <= 0) {
    bitmap.close()
    return
  }
  const scale = Math.max(w / bitmap.width, h / bitmap.height)
  const dw = bitmap.width * scale
  const dh = bitmap.height * scale
  const dx = (w - dw) / 2
  const dy = (h - dh) / 2
  const g = canvas.getContext('2d')
  if (g === null) {
    bitmap.close()
    return
  }
  g.fillStyle = '#ffffff'
  g.fillRect(0, 0, w, h)
  g.drawImage(bitmap, dx, dy, dw, dh)
  bitmap.close()
}

async function loadAndDraw(key: number, path: string, g: number): Promise<void> {
  if (disposed || g !== gen) return
  const canvas = canvases.get(key)
  if (canvas === undefined) return
  if (loading.has(path)) return
  loading.add(path)
  try {
    const res = await fetch(fileUrl('', path))
    if (!res.ok) throw new Error(String(res.status))
    const blob = await res.blob()
    const bitmap = await createImageBitmap(blob)
    if (disposed || g !== gen) {
      bitmap.close()
      return
    }
    const c = canvases.get(key)
    if (c === undefined) {
      bitmap.close()
      return
    }
    drawCover(c, bitmap)
    drawnPath.set(key, path)
  } catch (err: unknown) {
    post({
      level: 'warn',
      type: 'log',
      message: `cover draw failed: ${err instanceof Error ? err.message : String(err)}`,
    })
  } finally {
    loading.delete(path)
  }
}

function warmImages(g: number): void {
  for (const { key, path } of pathEntries(displayBase)) {
    if (path === null) continue
    if (drawnPath.get(key) === path) continue
    void loadAndDraw(key, path, g)
  }
}

async function applyWindow(queueIdx: number, base: number, g: number): Promise<void> {
  try {
    const resp = await fetchQueue(wrapQueue(queueIdx), limit)
    if (disposed || g !== gen) return
    if (resp.images.length === 0) return
    items = resp.images
    total = resp.total
    centerQueueIdx = items[mid].index
    displayBase = base
    const entries = pathEntries(base)
    const pathsByKey: Record<number, string> = {}
    for (const e of entries) {
      if (e.path !== null) pathsByKey[e.key] = e.path
    }
    post({ type: 'paths', gen: g, displayBase: base, entries, pathsByKey })
    warmImages(g)
  } catch (err: unknown) {
    post({
      level: 'warn',
      type: 'log',
      message: `cover queue failed: ${err instanceof Error ? err.message : String(err)}`,
    })
  }
}

async function bootstrap(offset0: number, g: number): Promise<void> {
  let attempts = 0
  for (;;) {
    if (disposed || g !== gen) return
    try {
      const resp = await fetchQueue(offset0, limit)
      if (disposed || g !== gen) return
      if (resp.images.length > 0) {
        items = resp.images
        total = resp.total
        centerQueueIdx = items[mid].index
        displayBase = 0
        const entries = pathEntries(0)
        const pathsByKey: Record<number, string> = {}
        for (const e of entries) {
          if (e.path !== null) pathsByKey[e.key] = e.path
        }
        post({ type: 'status', status: 'ready' })
        post({ type: 'paths', gen: g, displayBase: 0, entries, pathsByKey })
        warmImages(g)
        return
      }
      if (resp.done) {
        post({ type: 'status', status: 'empty' })
        return
      }
      attempts += 1
      if (attempts > 30) {
        post({ type: 'status', status: 'error', error: '扫描超时' })
        return
      }
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 1000)
      })
    } catch (err: unknown) {
      post({
        type: 'status',
        status: 'error',
        error: err instanceof Error ? err.message : String(err),
      })
      return
    }
  }
}

ctx.onmessage = (ev: MessageEvent<MainToWorker>): void => {
  const msg = ev.data
  switch (msg.type) {
    case 'init': {
      gen = msg.gen
      limit = msg.limit
      mid = msg.mid
      disposed = false
      items = []
      total = 0
      canvases.clear()
      drawnPath.clear()
      void bootstrap(msg.offset0, gen)
      break
    }
    case 'rebase': {
      if (msg.gen !== gen) return
      // Do NOT mutate centerQueueIdx here — applyWindow commits on success.
      void applyWindow(
        wrapQueue(centerQueueIdx + msg.queueDelta),
        msg.displayBase,
        msg.gen,
      )
      break
    }
    case 'attach': {
      if (msg.gen !== gen) return
      canvases.set(msg.key, msg.canvas)
      const path = items.length
        ? (pathEntries(displayBase).find((e) => e.key === msg.key)?.path ?? null)
        : null
      if (path !== null && drawnPath.get(msg.key) !== path) {
        void loadAndDraw(msg.key, path, msg.gen)
      }
      break
    }
    case 'detach': {
      if (msg.gen !== gen) return
      canvases.delete(msg.key)
      drawnPath.delete(msg.key)
      break
    }
    case 'resize': {
      if (msg.gen !== gen) return
      for (const canvas of canvases.values()) {
        canvas.width = msg.width
        canvas.height = msg.height
      }
      drawnPath.clear()
      warmImages(msg.gen)
      break
    }
    case 'dispose': {
      disposed = true
      gen += 1
      canvases.clear()
      drawnPath.clear()
      loading.clear()
      break
    }
  }
}