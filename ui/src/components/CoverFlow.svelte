<script lang="ts">
  import { onMount } from 'svelte'
  import { applyPreset, slotTransform } from '../lib/coverflow'
  import type { MainToWorker, WorkerToMain } from '../lib/coverProtocol'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref, toSegments } from '../lib/path'

  // ── Motion layer (pure index) ─────────────────────────────────────────
  // Main thread knows ONLY a random start, integer display keys, and `p`.
  // No fetch, no image bytes. Slots are white boards + canvas until the
  // worker paints. Badges are ordinary DOM, filled from worker messages.
  //
  // ── Worker layer ──────────────────────────────────────────────────────
  // cover.worker.ts owns queue API, image decode, canvas draw, parent dir.
  applyPreset('tightSeam')

  const SPARE = 4
  const LIMIT = 2 * SPARE + 1
  const MID = SPARE
  const MOVE_MS = 950
  const PAUSE_MS = 4050

  type Status = 'loading' | 'ready' | 'empty' | 'error'

  let p = $state(0)
  let status = $state<Status>('loading')
  let error = $state('')
  let keyLo = $state(-SPARE)
  let keyHi = $state(SPARE)
  let windowBase = $state(0)
  // key → path (badge + center open). Filled only from worker messages.
  let bindings = $state<Record<number, string | null>>({})
  let workerGen = 0

  const SLOT_MIN = 240
  const SLOT_MAX = 660
  let stageH = $state(0)
  let stageW = $state(0)
  const S = $derived(
    Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40, stageW - 80)),
  )

  const indices = $derived(
    Array.from({ length: keyHi - keyLo + 1 }, (_, i) => keyLo + i),
  )

  function easeInOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
  }

  function expandKeysFor(from: number, to: number): void {
    const lo = Math.min(from, to) - SPARE
    const hi = Math.max(from, to) + SPARE
    if (lo < keyLo) keyLo = lo
    if (hi > keyHi) keyHi = hi
  }

  function settleKeys(base: number): void {
    windowBase = base
    keyLo = base - SPARE
    keyHi = base - SPARE + LIMIT - 1
    // Keep prior bindings for retained keys (no badge wipe); new keys stay
    // null until the worker's `paths` message.
    const next: Record<number, string | null> = {}
    for (const k of indices) next[k] = bindings[k] ?? null
    bindings = next
  }

  let moveRaf: number | null = null
  let loopTimer: ReturnType<typeof setTimeout> | undefined
  let animating = false
  let cancelled = false
  let worker: Worker | null = null

  function send(msg: MainToWorker, transfer?: Transferable[]): void {
    worker?.postMessage(msg, transfer ?? [])
  }

  function onWorkerMessage(ev: MessageEvent<WorkerToMain>): void {
    if (cancelled) return
    const msg = ev.data
    switch (msg.type) {
      case 'status': {
        if (msg.status === 'ready') {
          status = 'ready'
          settleKeys(Math.round(p))
          scheduleAuto()
        } else if (msg.status === 'empty') {
          status = 'empty'
        } else {
          status = 'error'
          error = msg.error ?? '未知错误'
        }
        break
      }
      case 'paths': {
        if (msg.gen !== workerGen) return
        // Merge worker paths — badges update without touching motion.
        const next: Record<number, string | null> = { ...bindings }
        for (const e of msg.entries) next[e.key] = e.path
        // Drop keys outside the reported window that are no longer relevant
        // only if they're outside current index set (avoid mid-motion wipe).
        bindings = next
        break
      }
      case 'log': {
        console.warn(msg.message)
        break
      }
    }
  }

  function animateTo(to: number, queueDelta?: number): void {
    if (cancelled || animating) return
    clearTimeout(loopTimer)
    const from = windowBase
    if (from === to) {
      settleKeys(to)
      scheduleAuto()
      return
    }
    expandKeysFor(from, to)
    const t0 = performance.now()
    animating = true
    const finish = (): void => {
      p = to
      settleKeys(to)
      animating = false
      if (queueDelta !== undefined && !cancelled) {
        send({
          type: 'rebase',
          displayBase: to,
          queueDelta,
          gen: workerGen,
        })
      }
      scheduleAuto()
    }
    const frame = (now: number): void => {
      if (cancelled) return
      const t = Math.min(1, (now - t0) / MOVE_MS)
      p = from + (to - from) * easeInOutCubic(t)
      if (t < 1) moveRaf = requestAnimationFrame(frame)
      else finish()
    }
    moveRaf = requestAnimationFrame(frame)
  }

  function scheduleAuto(): void {
    if (!cancelled) loopTimer = setTimeout(startMove, PAUSE_MS)
  }

  function startMove(): void {
    if (cancelled || animating || status !== 'ready') return
    animateTo(windowBase + 1, 1)
  }

  function onSlotClick(k: number): void {
    if (cancelled || animating || status !== 'ready') return
    if (k === windowBase) {
      const path = bindings[k]
      if (path === null || path === undefined) return
      const { dir, file } = splitMediaPath(path)
      window.open(new URL(mediaHref(dir, file), location.href).href, '_blank')
      return
    }
    animateTo(k, k - windowBase)
  }

  /** Transfer canvas control to the worker; keyed each remounts get a fresh transfer. */
  function attachCanvas(node: HTMLCanvasElement, key: number) {
    const g = workerGen
    try {
      const off = node.transferControlToOffscreen()
      send(
        {
          type: 'attach',
          key,
          canvas: off,
          width: node.width,
          height: node.height,
          gen: g,
        },
        [off],
      )
    } catch (err: unknown) {
      console.warn('cover canvas attach failed', err)
    }
    return {
      destroy: () => {
        send({ type: 'detach', key, gen: g })
      },
    }
  }

  // Pin the CLIP box to the VISUAL viewport.
  function pinStage(): void {
    if (stageEl === null) return
    const clip = stageEl.parentElement
    if (clip === null) return
    const main = document.querySelector('main')
    const mr = main?.getBoundingClientRect()
    const top = Math.round((mr?.top ?? 73) + 16)
    clip.style.top = `${top}px`
    clip.style.height = `${Math.max(320, window.innerHeight - top - 16)}px`
    clip.style.width = `${document.documentElement.clientWidth}px`
  }

  let stageEl = $state<HTMLDivElement | null>(null)

  function parentSegsOf(path: string | null | undefined): string[] {
    return path === null || path === undefined
      ? []
      : toSegments(splitMediaPath(path).dir)
  }

  // Notify worker of backing-store size when S changes (rest only would be
  // nicer; resize during motion only touches canvas buffers, not motion).
  let lastS = 0
  $effect(() => {
    const s = S
    if (s <= 0 || s === lastS) return
    lastS = s
    if (status !== 'ready') return
    send({ type: 'resize', width: s, height: s, gen: workerGen })
  })

  onMount(() => {
    pinStage()
    const onResize = (): void => pinStage()
    window.addEventListener('resize', onResize)

    worker = new Worker(new URL('../lib/cover.worker.ts', import.meta.url), {
      type: 'module',
    })
    worker.onmessage = onWorkerMessage
    worker.onerror = (err) => {
      console.error('cover worker error', err)
      if (status === 'loading') {
        status = 'error'
        error = 'worker 启动失败'
      }
    }
    const offset0 = Math.floor(Math.random() * 1_000_000)
    workerGen = 1
    send({ type: 'init', offset0, limit: LIMIT, mid: MID, gen: workerGen })

    return () => {
      cancelled = true
      window.removeEventListener('resize', onResize)
      clearTimeout(loopTimer)
      if (moveRaf !== null) cancelAnimationFrame(moveRaf)
      send({ type: 'dispose' })
      worker?.terminate()
      worker = null
    }
  })
</script>

<div
  class="flex min-h-0 flex-1 flex-col justify-center"
  data-testid="coverflow"
>
  {#if status === 'loading'}
    <span class="loading loading-dots"></span>
  {:else if status === 'empty'}
    <span class="text-sm opacity-60">暂无照片</span>
  {:else if status === 'error'}
    <div class="alert alert-error py-2 text-sm" role="alert">
      <span>照片列表加载失败：{error}</span>
    </div>
  {/if}
  <div class="cf-clip fixed left-0">
    <div
      class="cf-stage"
      bind:this={stageEl}
      bind:clientHeight={stageH}
      bind:clientWidth={stageW}
      data-testid="cover-stage"
    >
      {#if status === 'ready'}
        {#each indices as k (k)}
          {@const d = k - p}
          {@const t = slotTransform(d, S)}
          {@const path = bindings[k] ?? null}
          {@const segs = parentSegsOf(path)}
          <button
            type="button"
            class="cf-slot cursor-pointer"
            data-cf-slot
            data-d={d}
            data-k={k}
            data-theta={t.theta}
            aria-label={path ?? ''}
            style:width="{S}px"
            style:height="{S}px"
            style:margin-left="{-S / 2}px"
            style:margin-top="{-S / 2}px"
            style:transform="translate3d({t.tx}px, 0, {t.tz}px) rotateY({t.theta}deg)"
            onclick={() => onSlotClick(k)}
          >
            <div
              class="relative h-full w-full overflow-hidden rounded-lg bg-white p-3 ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]"
              data-testid="cover-ph"
            >
              <div
                class="absolute inset-3 flex items-center justify-center rounded bg-base-200/40 text-sm text-base-content/40"
                data-k-label={k}
              >
                {k}
              </div>
              <canvas
                class="cf-canvas absolute inset-3 h-[calc(100%-1.5rem)] w-[calc(100%-1.5rem)] rounded"
                width={Math.max(1, Math.round(S - 24))}
                height={Math.max(1, Math.round(S - 24))}
                data-testid="cover-canvas"
                data-canvas-k={k}
                use:attachCanvas={k}
              ></canvas>
            </div>
            {#if segs.length > 0}
              <span
                class="badge badge-sm absolute bottom-2 left-2 z-20 ml-2 mb-2 max-w-[90%] bg-base-100/70 text-base-content/90 backdrop-blur"
                data-testid="cover-parent"
              >
                <span class="block truncate whitespace-nowrap">{segs.join(' › ')}</span>
              </span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .cf-canvas {
    display: block;
    pointer-events: none;
  }
</style>