<script lang="ts">
  import { onMount } from 'svelte'
  import { fetchQueue, fileUrl, type QueueItem } from '../lib/api'
  import { createCoverLoader } from '../lib/coverLoader'
  import { applyPreset, slotTransform } from '../lib/coverflow'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref, toSegments } from '../lib/path'

  // ── Two strictly separated layers ──────────────────────────────────────
  //
  // Motion layer (pure index): given the rest anchor `windowBase`, every
  // slot key is `windowBase ± SPARE` and geometry is slotTransform(k − p).
  // Knowing one index implies all others — no data, no image, no fetch is
  // required to drive or complete a glide. Keys freeze for the whole
  // animation; only `p` changes (transform writes only).
  //
  // Data/loader layer: at rest frames only, a queue window is fetched and
  // index → path bindings are swapped in atomically. The loader then warms
  // those paths off to the side (future: Web Worker). A missing/slow image
  // leaves the white placeholder board visible — never blocks motion.
  applyPreset('tightSeam')

  const SPARE = 4 // one spare slot each side beyond the ±3 visible range:
                  // absorbs the ±1 key shift so key re-basing happens at rest
  const SLOT_COUNT = 2 * SPARE + 1 // 9 placeholders (outer pair clips offscreen)
  const LIMIT = SLOT_COUNT // fetch window = placeholder set
  const MID = SPARE // center index inside a center=true window (=4)
  const MOVE_MS = 950
  const PAUSE_MS = 4050 // MOVE + PAUSE = 5s cycle
  const MAX_EMPTY_RETRIES = 30

  type Status = 'loading' | 'ready' | 'empty' | 'error'

  let p = $state(0) // playhead — the ONE animated state
  let status = $state<Status>('loading')
  let error = $state('')
  let items = $state<QueueItem[]>([]) // latest fetched window (position → path)
  let windowBase = $state(0) // integer anchor: keys = windowBase ± SPARE.
                             // Frozen during motion; re-based only at rest.
  let centerQueueIdx = 0 // queue index of items[MID]
  // index → path, swapped only at rest. null = white board (no data yet).
  let bindings = $state<Record<number, string | null>>({})
  const loader = createCoverLoader()

  const SLOT_MIN = 240
  const SLOT_MAX = 660
  let stageH = $state(0)
  let stageW = $state(0)
  const S = $derived(
    Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40, stageW - 80)),
  )

  // Keys are driven by windowBase (rest state), NOT round(p) — frozen in motion.
  const indices = $derived(
    Array.from({ length: SLOT_COUNT }, (_, i) => windowBase - SPARE + i),
  )

  function easeInOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
  }

  let moveRaf: number | null = null
  let loopTimer: ReturnType<typeof setTimeout> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined
  let animating = false
  let cancelled = false

  // Re-derive placeholder → path bindings from the current window.
  // Called ONLY at rest frames — path knowledge never arrives mid-motion.
  // A null binding is legal: the white board shows until bytes settle.
  function rebind(): void {
    const next: Record<number, string | null> = {}
    for (const k of indices) {
      const j = k - windowBase + MID
      next[k] = j >= 0 && j < items.length ? items[j].path : null
    }
    bindings = next
  }

  // Rest transaction: fetch new window, then swap state atomically
  // (items / windowBase / bindings) and kick the loader. Never during motion.
  async function recenter(queueIdx: number): Promise<void> {
    if (cancelled) return
    try {
      const resp = await fetchQueue(queueIdx, LIMIT)
      if (cancelled || resp.images.length === 0) return
      if (animating) {
        // purity: DOM/index state swaps only happen at rest — retry shortly
        retryTimer = setTimeout(() => {
          void recenter(queueIdx)
        }, 100)
        return
      }
      items = resp.images
      windowBase = Math.round(p) // p is integer at rest
      centerQueueIdx = items[MID].index
      rebind()
      const paths = items.map((i) => i.path)
      loader.prune(new Set(paths))
      loader.warm(paths)
    } catch (err: unknown) {
      console.warn('coverflow recenter failed', err)
    }
  }

  function animateTo(to: number, after?: () => void): void {
    if (cancelled || animating) return
    clearTimeout(loopTimer)
    const from = p
    const finish = (): void => {
      p = to
      animating = false
      after?.() // rest transaction (recenter etc.)
      if (!cancelled) loopTimer = setTimeout(startMove, PAUSE_MS)
    }
    if (from === to) {
      finish()
      return
    }
    const t0 = performance.now()
    animating = true
    const frame = (now: number): void => {
      if (cancelled) return
      const t = Math.min(1, (now - t0) / MOVE_MS)
      p = from + (to - from) * easeInOutCubic(t)
      if (t < 1) moveRaf = requestAnimationFrame(frame)
      else finish()
    }
    moveRaf = requestAnimationFrame(frame)
  }

  function startMove(): void {
    if (cancelled || animating || status !== 'ready') return
    // Motion is index-driven: NEVER gated on image readiness or bindings.
    animateTo(windowBase + 1, () => {
      void recenter(centerQueueIdx + 1)
    })
  }

  function onSlotClick(k: number): void {
    if (cancelled || animating || status !== 'ready') return
    if (k === windowBase) {
      const path = bindings[k]
      if (path === null) return // white placeholder — nothing to open
      const { dir, file } = splitMediaPath(path)
      window.open(new URL(mediaHref(dir, file), location.href).href, '_blank')
      return
    }
    const delta = k - windowBase // capture at rest
    animateTo(k, () => {
      void recenter(centerQueueIdx + delta)
    })
  }

  async function bootstrap(): Promise<void> {
    const offset0 = Math.floor(Math.random() * 1_000_000)
    let attempts = 0
    for (;;) {
      if (cancelled) return
      try {
        const resp = await fetchQueue(offset0, LIMIT)
        if (cancelled) return
        if (resp.images.length > 0) {
          items = resp.images
          windowBase = Math.round(p)
          centerQueueIdx = items[MID].index
          status = 'ready'
          rebind()
          loader.warm(items.map((i) => i.path))
          loopTimer = setTimeout(startMove, PAUSE_MS)
          return
        }
        if (resp.done) {
          status = 'empty'
          return
        }
        attempts += 1
        if (attempts > MAX_EMPTY_RETRIES) {
          status = 'error'
          error = '扫描超时'
          return
        }
        await new Promise<void>((resolve) => {
          retryTimer = setTimeout(resolve, 1000)
        })
      } catch (err: unknown) {
        if (cancelled) return
        status = 'error'
        error = err instanceof Error ? err.message : String(err)
        return
      }
    }
  }

  // Pin the CLIP box to the VISUAL viewport (fixed escapes main's container);
  // clip owns overflow:hidden + perspective; stage owns preserve-3d.
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

  function parentSegsOf(path: string | null): string[] {
    return path === null ? [] : toSegments(splitMediaPath(path).dir)
  }

  onMount(() => {
    pinStage()
    const onResize = (): void => pinStage()
    window.addEventListener('resize', onResize)
    void bootstrap()
    return () => {
      cancelled = true
      window.removeEventListener('resize', onResize)
      clearTimeout(loopTimer)
      clearTimeout(retryTimer)
      if (moveRaf !== null) cancelAnimationFrame(moveRaf)
      loader.dispose()
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
            <!-- white board: visible until the bound image decodes (or forever
                 if path is null — motion must never wait on bytes) -->
            <div
              class="h-full w-full overflow-hidden rounded-lg bg-white p-3 ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]"
            >
              {#if path !== null}
                {#key path}
                  <img
                    src={fileUrl('', path)}
                    alt=""
                    class="cf-photo block h-full w-full object-cover"
                    data-testid="cover-img"
                    onload={(e) => {
                      e.currentTarget.classList.add('is-loaded')
                    }}
                    onerror={(e) => {
                      e.currentTarget.classList.add('is-loaded')
                    }}
                  />
                {/key}
              {/if}
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