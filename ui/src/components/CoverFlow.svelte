<script lang="ts">
  import { onMount } from 'svelte'
  import { fetchQueue, type QueueItem } from '../lib/api'
  import { applyPreset, slotTransform } from '../lib/coverflow'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref, toSegments } from '../lib/path'

  // ── Motion layer (pure index) ─────────────────────────────────────────
  // Knowing one cover's index implies all others: slots are white boards
  // keyed by integer display indices. Animation only mutates `p` (transform
  // writes). The key window is expanded BEFORE a jump to cover the whole
  // trajectory ± SPARE, frozen during motion, and re-based/shrunk at rest —
  // always, whether or not any data fetch succeeds. Missing paths → white.
  //
  // ── Data layer (optional) ─────────────────────────────────────────────
  // At rest only: fetch the queue window for badges / center-open. Failures
  // never touch motion state. No image bytes are loaded in this component.
  applyPreset('tightSeam')

  const SPARE = 4 // spare slots each side beyond the visible ±3
  const LIMIT = 2 * SPARE + 1 // fetch window (center=true → MID at center)
  const MID = SPARE
  const MOVE_MS = 950
  const PAUSE_MS = 4050 // MOVE + PAUSE = 5s cycle
  const MAX_EMPTY_RETRIES = 30

  type Status = 'loading' | 'ready' | 'empty' | 'error'

  let p = $state(0) // playhead — the ONE animated state
  let status = $state<Status>('loading')
  let error = $state('')
  // Display key window [keyLo, keyHi]; expanded before jumps, frozen in motion.
  let keyLo = $state(-SPARE)
  let keyHi = $state(SPARE)
  let windowBase = $state(0) // rest anchor: center display key; motion-owned
  let items = $state<QueueItem[]>([]) // queue window (display-centered on itemsBase)
  let itemsBase: number | null = null // windowBase that `items` describes
  let centerQueueIdx = 0 // queue index of items[MID]
  let queueTotal = 0 // full queue length (0 = unknown/empty)
  let bindings = $state<Record<number, string | null>>({}) // key → path at rest

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

  function wrapQueue(idx: number): number {
    if (queueTotal <= 0) return Math.max(0, idx)
    return ((idx % queueTotal) + queueTotal) % queueTotal
  }

  // Expand the key window to cover [min(from,to)-SPARE, max(from,to)+SPARE]
  // BEFORE motion starts (create slots before/after motion, never during).
  function expandKeysFor(from: number, to: number): void {
    const lo = Math.min(from, to) - SPARE
    const hi = Math.max(from, to) + SPARE
    if (lo < keyLo) keyLo = lo
    if (hi > keyHi) keyHi = hi
  }

  // Rest transaction: snap keys to windowBase ± SPARE, bind paths if the
  // fetched window matches, else leave white boards.
  function settleKeys(base: number): void {
    windowBase = base
    keyLo = base - SPARE
    keyHi = base + SPARE
    rebind()
  }

  // Bind paths for the current key window. When the fetched window does not
  // match yet (just re-based / fetch in flight), KEEP prior paths for keys
  // that still exist — wiping them would unmount every badge (visible flash
  // at each rest). Only newly appeared keys start as white boards.
  function rebind(): void {
    const next: Record<number, string | null> = {}
    const usable = itemsBase === windowBase && items.length > 0
    for (const k of indices) {
      if (usable) {
        const j = k - windowBase + MID
        next[k] = j >= 0 && j < items.length ? items[j].path : null
      } else {
        next[k] = bindings[k] ?? null
      }
    }
    bindings = next
  }

  // Data-only: fetch paths for the rest window. Never gates or mutates motion.
  async function recenter(queueIdx: number, expectBase: number): Promise<void> {
    if (cancelled) return
    try {
      const resp = await fetchQueue(wrapQueue(queueIdx), LIMIT)
      if (cancelled || resp.images.length === 0) return
      if (animating || expectBase !== windowBase) return // stale / mid-motion
      items = resp.images
      itemsBase = expectBase
      centerQueueIdx = items[MID].index
      queueTotal = resp.total
      rebind()
    } catch (err: unknown) {
      console.warn('coverflow recenter failed', err)
    }
  }

  let moveRaf: number | null = null
  let loopTimer: ReturnType<typeof setTimeout> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined
  let animating = false
  let cancelled = false

  function animateTo(to: number, queueDelta?: number): void {
    if (cancelled || animating) return
    clearTimeout(loopTimer)
    const from = windowBase // rest anchor — integer
    if (from === to) {
      settleKeys(to)
      scheduleAuto()
      return
    }
    expandKeysFor(from, to) // create any far-side placeholders BEFORE motion
    const targetQueue =
      queueDelta === undefined ? null : wrapQueue(centerQueueIdx + queueDelta)
    const t0 = performance.now()
    animating = true
    const finish = (): void => {
      p = to
      settleKeys(to) // motion-owned: always re-base keys at rest
      animating = false
      if (targetQueue !== null && !cancelled) {
        void recenter(targetQueue, to)
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
    // Pure index: never gated on data or images.
    animateTo(windowBase + 1, 1)
  }

  function onSlotClick(k: number): void {
    if (cancelled || animating || status !== 'ready') return
    if (k === windowBase) {
      const path = bindings[k]
      if (path === null) return
      const { dir, file } = splitMediaPath(path)
      window.open(new URL(mediaHref(dir, file), location.href).href, '_blank')
      return
    }
    animateTo(k, k - windowBase)
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
          queueTotal = resp.total
          settleKeys(Math.round(p))
          itemsBase = windowBase
          centerQueueIdx = items[MID].index
          rebind()
          status = 'ready'
          scheduleAuto()
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
            <!-- white placeholder: no image loading — motion never waits on bytes -->
            <div
              class="h-full w-full overflow-hidden rounded-lg bg-white p-3 ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]"
              data-testid="cover-ph"
            >
              <div
                class="flex h-full w-full items-center justify-center rounded bg-base-200/40 text-sm text-base-content/40"
                data-k-label={k}
              >
                {k}
              </div>
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