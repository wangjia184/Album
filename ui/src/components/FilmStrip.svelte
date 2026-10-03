<script lang="ts">
  import { onMount } from 'svelte'
  import { fetchQueue, fileUrl } from '../lib/api'
  import { thumbCapacity, windowSlots } from '../lib/pipeline'

  let { oncurrent }: { oncurrent: (path: string) => void } = $props()

  const TICK_MS = 10_000
  const PREFETCH_AHEAD = 16
  const MAX_EMPTY_RETRIES = 60

  let status = $state<'loading' | 'ready' | 'empty' | 'error'>('loading')
  let history = $state<string[]>([])
  let upcoming = $state<string[]>([])
  let nextOffset = $state<number | null>(null)
  let error = $state('')
  let width = $state(0)

  const capacity = $derived(thumbCapacity(width))
  const mid = $derived(Math.floor((capacity - 1) / 2))
  const slots = $derived.by(() => windowSlots(history, upcoming, capacity))

  let cancelled = false
  let started = false
  let inFlight = false
  let timer: ReturnType<typeof setInterval> | undefined
  let retryTimer: ReturnType<typeof setTimeout> | undefined

  function prefetch(): void {
    for (const p of upcoming.slice(0, PREFETCH_AHEAD)) {
      const im = new Image()
      im.src = fileUrl('', p)
    }
  }

  async function advance(): Promise<void> {
    if (cancelled || inFlight || nextOffset === null) return
    inFlight = true
    try {
      const resp = await fetchQueue(nextOffset, capacity)
      if (cancelled) return
      if (resp.images.length > 0) {
        const current = resp.images[0]
        if (current !== undefined) {
          history = [...history, current]
          upcoming = resp.images.slice(1)
          nextOffset = resp.nextOffset
          oncurrent(current)
          prefetch()
        }
      }
      // Empty mid-cycle: keep state; the next tick retries the same nextOffset.
    } catch (err: unknown) {
      console.warn('queue tick failed', err)
    } finally {
      inFlight = false
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
        const first = resp.images[0]
        if (first !== undefined) {
          history = [first]
          upcoming = resp.images.slice(1)
          nextOffset = resp.nextOffset
          status = 'ready'
          oncurrent(first)
          prefetch()
          timer = setInterval(() => {
            void advance()
          }, TICK_MS)
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

  onMount(() => {
    return () => {
      cancelled = true
      if (timer !== undefined) clearInterval(timer)
      if (retryTimer !== undefined) clearTimeout(retryTimer)
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
  {:else if status === 'ready' && capacity > 0}
    {#each slots as p, i (i)}
      {#if p !== null}
        <div
          class="h-16 w-28 shrink-0 overflow-hidden rounded-lg bg-base-300 {i === mid
            ? 'ring-2 ring-primary'
            : ''}"
        >
          <img src={fileUrl('', p)} alt="" class="h-full w-full object-cover" />
        </div>
      {:else}
        <div class="h-16 w-28 shrink-0 rounded-lg bg-base-300/40"></div>
      {/if}
    {/each}
  {/if}
</div>