<script lang="ts">
  import { onMount } from 'svelte'
  import { fetchQueue, fileUrl, type QueueItem } from '../lib/api'
  import { thumbCapacity } from '../lib/pipeline'

  let { oncurrent }: { oncurrent: (path: string) => void } = $props()

  const IDLE_MS = 5_000
  const PREFETCH_AHEAD = 16
  const MAX_EMPTY_RETRIES = 60

  let status = $state<'loading' | 'ready' | 'empty' | 'error'>('loading')
  let slots = $state<QueueItem[]>([])
  let current = $state<QueueItem | null>(null)
  let ringIndex = $state(0)
  let error = $state('')
  let width = $state(0)

  const capacity = $derived(thumbCapacity(width))
  const mid = $derived(Math.floor((capacity - 1) / 2))

  let cancelled = false
  let started = false
  let inFlight = false
  let lastCapacity = 0
  let idleTimer: ReturnType<typeof setTimeout> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined
  let resizeTimer: ReturnType<typeof setTimeout> | undefined

  const warmed = new Set<HTMLImageElement>()

  function prefetch(items: QueueItem[]): void {
    for (const item of items.slice(0, PREFETCH_AHEAD)) {
      const im = new Image()
      warmed.add(im)
      const settle = (): void => {
        warmed.delete(im)
      }
      im.onload = settle
      im.onerror = settle
      im.src = fileUrl('', item.path, true)
    }
  }

  function resetIdle(): void {
    if (cancelled || status !== 'ready') return
    clearTimeout(idleTimer)
    idleTimer = setTimeout(() => {
      void autoAdvance()
    }, IDLE_MS)
  }

  async function autoAdvance(): Promise<void> {
    if (cancelled || status !== 'ready' || current === null) return
    if (inFlight) {
      // Collision with refill (or a prior advance): re-arm so an idle user
      // never ends up with a permanently disarmed timer.
      resetIdle()
      return
    }
    inFlight = true
    try {
      const resp = await fetchQueue(current.index + 1, capacity)
      if (cancelled) return
      if (resp.images.length > 0) {
        slots = resp.images
        current = slots[mid] ?? slots[0] ?? null
        ringIndex = mid
        if (current !== null) {
          oncurrent(current.path)
          prefetch(slots.slice(mid + 1))
        }
      }
    } catch (err: unknown) {
      console.warn('queue advance failed', err)
    } finally {
      inFlight = false
      if (!cancelled) resetIdle()
    }
  }

  function select(i: number): void {
    if (cancelled || status !== 'ready' || inFlight) return
    const item = slots[i]
    if (item === undefined) return
    current = item
    ringIndex = i
    oncurrent(item.path)
    prefetch(slots.slice(i + 1))
    resetIdle()
  }

  async function refill(): Promise<void> {
    if (cancelled || inFlight || status !== 'ready' || current === null) return
    const target = current.index
    inFlight = true
    try {
      const resp = await fetchQueue(target, capacity)
      if (cancelled || resp.images.length === 0) return
      slots = resp.images
      const found = slots.findIndex((it) => it.index === target)
      ringIndex = found >= 0 ? found : mid
      current = slots[ringIndex] ?? current
      prefetch(slots.slice(ringIndex + 1))
    } catch (err: unknown) {
      console.warn('queue refill failed', err)
    } finally {
      inFlight = false
      // Always re-arm: if the idle timer fired (and bailed) while this refill
      // held inFlight, nobody else will restart rotation for an idle user.
      if (!cancelled && status === 'ready') resetIdle()
    }
  }

  async function bootstrap(): Promise<void> {
    const offset0 = Math.floor(Math.random() * 1_000_000)
    let attempts = 0
    for (;;) {
      if (cancelled) return
      try {
        const resp = await fetchQueue(offset0, capacity)
        if (cancelled) return
        if (resp.images.length > 0) {
          slots = resp.images
          current = slots[mid] ?? slots[0] ?? null
          ringIndex = mid
          status = 'ready'
          if (current !== null) {
            oncurrent(current.path)
            prefetch(slots.slice(mid + 1))
          }
          resetIdle()
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

  // Bootstrap once the strip has a real width (never fetch with capacity from 0).
  $effect(() => {
    if (width > 0 && !started) {
      started = true
      void bootstrap()
    }
  })

  // Refill centered on current when capacity changes (debounced 300ms).
  $effect(() => {
    const c = capacity
    if (status !== 'ready' || current === null) {
      lastCapacity = c
      return
    }
    if (c === lastCapacity) return
    lastCapacity = c
    clearTimeout(resizeTimer)
    resizeTimer = setTimeout(() => {
      void refill()
    }, 300)
  })

  onMount(() => {
    const onActivity = (): void => {
      resetIdle()
    }
    document.addEventListener('mousemove', onActivity, { passive: true })
    document.addEventListener('click', onActivity, { passive: true })
    return () => {
      cancelled = true
      document.removeEventListener('mousemove', onActivity)
      document.removeEventListener('click', onActivity)
      clearTimeout(idleTimer)
      clearTimeout(retryTimer)
      clearTimeout(resizeTimer)
    }
  })
</script>

<div
  bind:clientWidth={width}
  data-testid="filmstrip"
  class="flex h-20 shrink-0 items-center justify-center gap-2 overflow-hidden rounded-box bg-base-200/40 p-2 backdrop-blur-md"
>
  {#if status === 'loading'}
    <span class="loading loading-dots"></span>
  {:else if status === 'empty'}
    <span class="text-sm opacity-60">暂无照片</span>
  {:else if status === 'error'}
    <div class="alert alert-error py-2 text-sm" role="alert">
      <span>照片列表加载失败：{error}</span>
    </div>
  {:else if status === 'ready'}
    {#each slots as item, i (i)}
      <button
        type="button"
        class="h-16 w-28 shrink-0 cursor-pointer overflow-hidden rounded-lg bg-base-300 transition {i ===
        ringIndex
          ? 'ring-2 ring-primary'
          : 'hover:ring-1 hover:ring-primary/60'}"
        aria-label={item.path}
        onclick={() => select(i)}
      >
        <img src={fileUrl('', item.path, true)} alt="" class="h-full w-full object-cover" />
      </button>
    {/each}
  {/if}
</div>