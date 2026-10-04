/**
 * Cover worker — the ONLY place the coverflow touches the network.
 *
 * Owns: queue API (index → path), image fetch/decode, canvas draw.
 * Parent-dir badges are derived on the main thread from path strings.
 */
import { fetchQueue, fileUrl } from './api'
import type { MainToWorker, WorkerToMain } from './coverProtocol'
import { LIMIT, MID, pathEntries, wrapIndex } from './coverShared'

const ctx = self as unknown as {
  postMessage(msg: unknown, transfer?: Transferable[]): void
  onmessage: ((ev: MessageEvent<MainToWorker>) => void) | null
}

let displayBase = 0
let centerQueueIdx = 0
let total = 0
let paths: string[] = []
let gen = 0
let disposed = false
/** Monotonic rebase commits — ignore out-of-order completions. */
let seqApplied = 0
let seqInFlight = 0
let bufW = 1
let bufH = 1

const canvases = new Map<number, OffscreenCanvas>()
/** In-flight loads keyed by `${key}:${path}` — duplicate paths in a short
 *  queue window must each get their own draw (small albums wrap). */
const loading = new Set<string>()
/** Path already painted on this key (skip reload). */
const drawnPath = new Map<number, string>()
/** Decoded bitmaps by path — resize redraws without refetch/redecode. */
const bitmaps = new Map<string, ImageBitmap>()
const BITMAP_CACHE_MAX = 24

function post(msg: WorkerToMain): void {
  if (disposed) return
  ctx.postMessage(msg)
}

function publishPaths(seq: number, base: number): void {
  post({
    type: 'paths',
    gen,
    seq,
    displayBase: base,
    entries: pathEntries(base, paths),
  })
}

function drawCover(canvas: OffscreenCanvas, bitmap: ImageBitmap): void {
  const w = canvas.width
  const h = canvas.height
  if (w <= 0 || h <= 0) return
  const scale = Math.max(w / bitmap.width, h / bitmap.height)
  const dw = bitmap.width * scale
  const dh = bitmap.height * scale
  const g = canvas.getContext('2d')
  if (g === null) return
  g.fillStyle = '#ffffff'
  g.fillRect(0, 0, w, h)
  g.drawImage(bitmap, (w - dw) / 2, (h - dh) / 2, dw, dh)
}

function cacheBitmap(path: string, bitmap: ImageBitmap): ImageBitmap {
  const prev = bitmaps.get(path)
  if (prev !== undefined && prev !== bitmap) prev.close()
  bitmaps.set(path, bitmap)
  while (bitmaps.size > BITMAP_CACHE_MAX) {
    const oldest = bitmaps.keys().next()
    if (oldest.done === true) break
    const p = oldest.value
    if (p === path) break
    bitmaps.get(p)?.close()
    bitmaps.delete(p)
  }
  return bitmap
}

async function getBitmap(path: string): Promise<ImageBitmap> {
  const hit = bitmaps.get(path)
  if (hit !== undefined) return hit
  const res = await fetch(fileUrl('', path))
  if (!res.ok) throw new Error(String(res.status))
  const blob = await res.blob()
  return cacheBitmap(path, await createImageBitmap(blob))
}

async function loadAndDraw(key: number, path: string, g: number): Promise<void> {
  if (disposed || g !== gen) return
  const token = `${key}:${path}`
  if (loading.has(token)) return
  if (drawnPath.get(key) === path) return
  loading.add(token)
  try {
    const bitmap = await getBitmap(path)
    if (disposed || g !== gen) return
    const c = canvases.get(key)
    if (c === undefined) return
    drawCover(c, bitmap)
    drawnPath.set(key, path)
  } catch (err: unknown) {
    post({
      level: 'warn',
      type: 'log',
      message: `cover draw failed: ${err instanceof Error ? err.message : String(err)}`,
    })
  } finally {
    loading.delete(token)
  }
}

function warmImages(g: number): void {
  for (const { key, path } of pathEntries(displayBase, paths)) {
    if (path !== null) void loadAndDraw(key, path, g)
  }
}

function setAllCanvasSize(w: number, h: number): void {
  bufW = w
  bufH = h
  for (const canvas of canvases.values()) {
    canvas.width = w
    canvas.height = h
  }
}

async function applyWindow(
  queueIdx: number,
  base: number,
  seq: number,
  g: number,
): Promise<void> {
  try {
    const resp = await fetchQueue(wrapIndex(queueIdx, total), LIMIT)
    if (disposed || g !== gen) return
    if (seq < seqInFlight || seq <= seqApplied) return
    if (resp.images.length === 0) return
    paths = resp.images.map((i) => i.path)
    total = resp.total
    centerQueueIdx = resp.images[MID].index
    displayBase = base
    seqApplied = seq
    publishPaths(seq, base)
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
      const resp = await fetchQueue(offset0, LIMIT)
      if (disposed || g !== gen) return
      if (resp.images.length > 0) {
        paths = resp.images.map((i) => i.path)
        total = resp.total
        centerQueueIdx = resp.images[MID].index
        displayBase = 0
        seqApplied = 0
        seqInFlight = 0
        post({ type: 'status', gen: g, status: 'ready' })
        publishPaths(0, 0)
        warmImages(g)
        return
      }
      if (resp.done) {
        post({ type: 'status', gen: g, status: 'empty' })
        return
      }
      attempts += 1
      if (attempts > 30) {
        post({ type: 'status', gen: g, status: 'error', error: '扫描超时' })
        return
      }
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 1000)
      })
    } catch (err: unknown) {
      post({
        type: 'status',
        gen: g,
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
      disposed = false
      for (const b of bitmaps.values()) b.close()
      bitmaps.clear()
      paths = []
      total = 0
      seqApplied = 0
      seqInFlight = 0
      canvases.clear()
      drawnPath.clear()
      void bootstrap(msg.offset0, gen)
      break
    }
    case 'rebase': {
      if (msg.gen !== gen) return
      if (msg.seq <= seqApplied) return
      seqInFlight = Math.max(seqInFlight, msg.seq)
      void applyWindow(
        wrapIndex(centerQueueIdx + msg.queueDelta, total),
        msg.displayBase,
        msg.seq,
        msg.gen,
      )
      break
    }
    case 'attach': {
      if (msg.gen !== gen) return
      // Adopt the transferred backing store if main already sized it;
      // otherwise force-sync every canvas to the shared buffer size.
      if (msg.canvas.width > 1 && msg.canvas.height > 1) {
        setAllCanvasSize(msg.canvas.width, msg.canvas.height)
      } else {
        msg.canvas.width = bufW
        msg.canvas.height = bufH
      }
      canvases.set(msg.key, msg.canvas)
      const j = msg.key - displayBase + MID
      const path = j >= 0 && j < paths.length ? paths[j] : null
      if (path !== null) void loadAndDraw(msg.key, path, msg.gen)
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
      setAllCanvasSize(msg.width, msg.height)
      drawnPath.clear()
      warmImages(msg.gen)
      break
    }
    case 'dispose': {
      disposed = true
      gen += 1
      for (const b of bitmaps.values()) b.close()
      bitmaps.clear()
      canvases.clear()
      drawnPath.clear()
      loading.clear()
      break
    }
  }
}
