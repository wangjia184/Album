<script lang="ts">
  import { onMount } from 'svelte'
  import { fetchQueue, fileUrl, type QueueItem } from '../lib/api'
  import { applyPreset, slotOpacity, slotTransform } from '../lib/coverflow'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref, toSegments } from '../lib/path'

  // View layer only: animation loop, stage pinning, queue window, rendering.
  // Slot geometry: lib/coverflow.ts (pure slotTransform(d, S), ratio-based,
  // fine-grid collision-free presets). Photos fill slots via object-fit:cover
  // (short edge scales to the slot side, other axis center-cropped) so every
  // cover is exactly S×S regardless of source aspect ratio.
  applyPreset('tightSeam')

  const M = 3 // slots each side → 2M+1 = 7
  const SLOT_COUNT = 2 * M + 1 // visible slots (keys round(p)±3) — always 7
  const LIMIT = 2 * M + 3 // fetch 9 — one spare each side beyond the visible
                          // slots: during the 2nd half of a move the window
                          // keys shift +1 and the new edge slot needs its photo
                          // BEFORE the end-of-move recenter (no pop-in at rest)
  const MID = M + 1 // = 4: center index inside a center=true window of 9
                    // (server: floor((9-1)/2) = 4)
  const MOVE_MS = 950
  const PAUSE_MS = 4050 // pause; MOVE+PAUSE = 5s cycle
  const MAX_EMPTY_RETRIES = 30

  type Status = 'loading' | 'ready' | 'empty' | 'error'

  let p = $state(0) // playhead — the ONE animated state
  let status = $state<Status>('loading')
  let error = $state('')
  let items = $state<QueueItem[]>([])
  // Paths whose <img> has fired load/error → eligible for the fade-in class.
  let loaded = $state<string[]>([])
  // Stable mapping anchor: item index = k - anchorK + MID. Captured at fetch
  // time so images never swap mid-move (anchor only re-bases after a
  // completed move/click recenter, when geometry is at rest).
  let anchorK = 0
  let centerQueueIdx = 0 // normalized queue index of the item at slot MID

  const SLOT_MIN = 240
  const SLOT_MAX = 660
  let stageH = $state(0)
  let stageW = $state(0)
  const S = $derived(
    Math.max(SLOT_MIN, Math.min(SLOT_MAX, stageH - 40, stageW - 80)),
  )

  const indices = $derived(
    Array.from({ length: SLOT_COUNT }, (_, i) => Math.round(p) - M + i),
  )

  function itemFor(k: number): QueueItem | null {
    const j = k - anchorK + MID
    return j >= 0 && j < items.length ? items[j] : null
  }

  function markLoaded(item: QueueItem): void {
    if (!loaded.includes(item.path)) loaded = [...loaded, item.path]
  }

  function easeInOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
  }

  let moveRaf: number | null = null
  let loopTimer: ReturnType<typeof setTimeout> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined
  let animating = false
  let cancelled = false

  async function recenter(queueIdx: number): Promise<void> {
    if (cancelled) return
    try {
      const resp = await fetchQueue(queueIdx, LIMIT)
      if (cancelled || resp.images.length === 0) return
      items = resp.images
      centerQueueIdx = items[MID].index // normalized truth from the server
      anchorK = Math.round(p)
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
      after?.()
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
    animateTo(Math.round(p) + 1, () => {
      void recenter(centerQueueIdx + 1)
    })
  }

  function onSlotClick(k: number): void {
    if (cancelled || animating || status !== 'ready') return
    const item = itemFor(k)
    if (item === null) return
    if (k === Math.round(p)) {
      const { dir, file } = splitMediaPath(item.path)
      // open the album lightbox in a new tab (click is a user gesture —
      // window.open won't be popup-blocked)
      window.open(new URL(mediaHref(dir, file), location.href).href, '_blank')
      return
    }
    const delta = k - Math.round(p) // capture BEFORE the move
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
          anchorK = Math.round(p) // 0
          centerQueueIdx = items[MID].index
          status = 'ready'
          if (!cancelled) loopTimer = setTimeout(startMove, PAUSE_MS)
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

  // Pin the CLIP box to the VISUAL viewport (fixed escapes main's container
  // max-width/overflow); clip owns overflow:hidden + perspective; inner stage
  // owns preserve-3d (overflow on a preserve-3d element forces flattening).
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
          {@const item = itemFor(k)}
          {@const parentSegs =
            item !== null ? toSegments(splitMediaPath(item.path).dir) : []}
          <button
            type="button"
            class="cf-slot cursor-pointer"
            data-cf-slot
            data-d={d}
            data-k={k}
            data-theta={t.theta}
            aria-label={item !== null ? item.path : ''}
            style:width="{S}px"
            style:height="{S}px"
            style:margin-left="{-S / 2}px"
            style:margin-top="{-S / 2}px"
            style:transform="translate3d({t.tx}px, 0, {t.tz}px) rotateY({t.theta}deg)"
            onclick={() => onSlotClick(k)}
          >
            <div
              class="h-full w-full overflow-hidden rounded-lg bg-white p-3 ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]"
              style:opacity={slotOpacity(d)}
            >
              {#if item !== null}
              <img
                src={fileUrl('', item.path)}
                alt=""
                class="cf-photo block h-full w-full object-cover"
                class:is-loaded={loaded.includes(item.path)}
                data-testid="cover-img"
                onload={() => item !== null && markLoaded(item)}
                onerror={() => item !== null && markLoaded(item)}
              />
              {/if}
            </div>
            {#if parentSegs.length > 0}
              <span
                class="badge badge-sm absolute bottom-2 left-2 z-20 ml-2 mb-2 max-w-[90%] bg-base-100/70 text-base-content/90 backdrop-blur"
                data-testid="cover-parent"
              >
                <span class="block truncate whitespace-nowrap">{parentSegs.join(' › ')}</span>
              </span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>